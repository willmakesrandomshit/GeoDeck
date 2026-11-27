use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;

const GD_XOR_KEY: u8 = 11;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GmSongEntry {
    pub song_id: String,
    pub title: String,
    pub artist: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LocalLevelInfo {
    pub index: usize,
    pub name: String,
    pub description: String,
    pub revision: String,
    pub song_id: String,
    pub raw_dict_xml: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SaveSummary {
    pub inspected_at: String,
    pub game_manager_valid: bool,
    pub local_levels_valid: bool,
    pub validation_error: Option<String>,
    pub player_name: String,
    pub account_id: String,
    pub level_count: usize,
    pub levels: Vec<LocalLevelInfo>,
    #[serde(default)]
    pub known_songs: Vec<GmSongEntry>,
    #[serde(default)]
    pub song_to_levels: BTreeMap<String, Vec<String>>,
}

/// Decodes a Geometry Dash Windows `.dat` file (`XOR 0x0B` -> `URL-safe Base64` -> `Gzip`) into XML in memory.
pub fn decode_gd_dat_to_xml(raw: &[u8]) -> Result<String, String> {
    if raw.is_empty() {
        return Err("Save file is 0 bytes".into());
    }
    // Check if unencoded plaintext XML already
    if raw.starts_with(b"<?xml") || raw.starts_with(b"<plist") {
        let s = String::from_utf8(raw.to_vec()).map_err(|e| format!("UTF-8 error: {e}"))?;
        validate_gd_plist_xml(&s)?;
        return Ok(s);
    }

    let xored: Vec<u8> = raw
        .iter()
        .map(|&b| b ^ GD_XOR_KEY)
        .take_while(|&b| b != 0)
        .filter(|&b| !b.is_ascii_whitespace())
        .collect();

    let mut b64_bytes = xored;
    while b64_bytes.len() % 4 != 0 {
        b64_bytes.push(b'=');
    }

    let compressed = URL_SAFE
        .decode(&b64_bytes)
        .map_err(|e| format!("Base64 decode failed: {e}"))?;

    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut xml_bytes = Vec::new();
    decoder
        .read_to_end(&mut xml_bytes)
        .map_err(|e| format!("Gzip inflate failed: {e}"))?;
    let xml = String::from_utf8_lossy(&xml_bytes).into_owned();

    validate_gd_plist_xml(&xml)?;
    Ok(xml)
}

pub fn validate_gd_plist_xml(xml: &str) -> Result<(), String> {
    let trimmed = xml.trim();
    if !trimmed.contains("<plist") || !trimmed.contains("</plist>") {
        return Err("Missing <plist>...</plist> root XML structure".into());
    }
    Ok(())
}

pub fn validate_dat_file(path: &Path) -> Result<(), String> {
    let raw = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    decode_gd_dat_to_xml(&raw).map(|_| ())
}

/// Extracts strictly non-sensitive summary fields from CCGameManager XML.
/// NEVER reads or returns GJA_002 (password/GJP), GJA_005 (GJP2), or token keys.
fn extract_safe_player_meta(xml: &str) -> (String, String) {
    let name = extract_plist_string_value(xml, "GJA_001").unwrap_or_else(|| "Player".into());
    let acct = extract_plist_string_value(xml, "GJA_003").unwrap_or_default();
    (name, acct)
}

fn xml_unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn extract_plist_string_value(xml: &str, key: &str) -> Option<String> {
    let pat = format!("<k>{key}</k>");
    let idx = xml.find(&pat)?;
    let rest = &xml[idx + pat.len()..];
    // Immediate next tag could be <s>...</s> or <i>...</i>
    let trimmed = rest.trim_start();
    if let Some(after_s) = trimmed.strip_prefix("<s>") {
        let end = after_s.find("</s>")?;
        Some(xml_unescape(&after_s[..end]))
    } else if let Some(after_i) = trimmed.strip_prefix("<i>") {
        let end = after_i.find("</i>")?;
        Some(after_i[..end].to_string())
    } else {
        None
    }
}

/// Parses `MDLM_001` (Music Download Manager) song dictionary from `CCGameManager.dat` XML.
pub fn parse_mdlm_songs_from_xml(xml: &str) -> Vec<GmSongEntry> {
    let mut out = Vec::new();
    let Some(mdlm_pos) = xml.find("<k>MDLM_001</k>") else {
        return out;
    };
    let rest = &xml[mdlm_pos + "<k>MDLM_001</k>".len()..];
    let Some(dict_rel) = rest.find("<d>") else {
        return out;
    };
    let dict_start = mdlm_pos + "<k>MDLM_001</k>".len() + dict_rel;
    let Some(dict_end) = find_matching_dict_end(xml, dict_start) else {
        return out;
    };
    let section = &xml[dict_start + 3..dict_end.saturating_sub(4)];

    let mut cursor = 0usize;
    while let Some(k_rel) = section[cursor..].find("<k>") {
        let k_start = cursor + k_rel + 3;
        let Some(k_end_rel) = section[k_start..].find("</k>") else {
            break;
        };
        let key_id = section[k_start..k_start + k_end_rel].trim();
        let after_k = k_start + k_end_rel + 4;
        let trimmed = section[after_k..].trim_start();
        let ws_len = section[after_k..].len() - trimmed.len();
        let entry_start = after_k + ws_len;
        if trimmed.starts_with("<d>") {
            if let Some(entry_end) = find_matching_dict_end(section, entry_start) {
                let block = &section[entry_start..entry_end];
                let song_id = extract_plist_string_value(block, "1")
                    .unwrap_or_else(|| key_id.to_string());
                let title = extract_plist_string_value(block, "2")
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                let artist = extract_plist_string_value(block, "4")
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                if !song_id.is_empty() && !title.is_empty() {
                    out.push(GmSongEntry {
                        song_id,
                        title,
                        artist,
                    });
                }
                cursor = entry_end;
                continue;
            }
        }
        cursor = after_k;
    }
    out
}

/// Efficiently extracts `song_id -> Vec<level_name>` from `GLM_03` (saved online levels)
/// in `CCGameManager.dat`, skipping massive `<k>k4</k><s>...</s>` level object strings.
pub fn parse_glm_level_songs_from_xml(xml: &str) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let Some(glm_pos) = xml.find("<k>GLM_03</k>") else {
        return map;
    };
    let after_key = &xml[glm_pos + "<k>GLM_03</k>".len()..];
    let Some(dict_rel) = after_key.find("<d>") else {
        return map;
    };
    let glm_start = glm_pos + "<k>GLM_03</k>".len() + dict_rel;
    // Scan each `<d><k>kCEK</k>` level block inside GLM_03 without scanning inside k4 level strings
    let mut cursor = glm_start + 3;
    let bytes = xml.as_bytes();
    while cursor < bytes.len() {
        let rem = &xml[cursor..];
        if rem.starts_with("</d>") {
            break;
        }
        let Some(k_rel) = rem.find("<k>") else {
            break;
        };
        let tag_pos = cursor + k_rel;
        let Some(close_k) = xml[tag_pos + 3..].find("</k>") else {
            break;
        };
        let after_k = tag_pos + 3 + close_k + 4;
        let trimmed = xml[after_k..].trim_start();
        let entry_start = after_k + (xml[after_k..].len() - trimmed.len());
        if trimmed.starts_with("<d>") {
            if let Some(entry_end) = find_matching_dict_end_skip_k4(xml, entry_start) {
                let block = &xml[entry_start..entry_end];
                if let Some(song_id) = extract_plist_string_value_fast_skip_k4(block, "k45") {
                    let sid = song_id.trim().to_string();
                    if !sid.is_empty() && sid != "0" {
                        if let Some(name) = extract_plist_string_value_fast_skip_k4(block, "k2") {
                            let clean_name = name.trim().to_string();
                            if !clean_name.is_empty() {
                                let list = map.entry(sid).or_default();
                                if !list.contains(&clean_name) && list.len() < 8 {
                                    list.push(clean_name);
                                }
                            }
                        }
                    }
                }
                cursor = entry_end;
                continue;
            }
        }
        cursor = after_k;
    }
    map
}

fn find_matching_dict_end_skip_k4(xml: &str, start_of_open_d: usize) -> Option<usize> {
    let bytes = xml.as_bytes();
    let mut i = start_of_open_d;
    let mut depth = 0i32;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let rem = &bytes[i..];
            if rem.starts_with(b"<k>k4</k><s>") {
                let s_start = i + 11;
                if let Some(s_end) = xml[s_start..].find("</s>") {
                    i = s_start + s_end + 4;
                    continue;
                }
            }
            if rem.starts_with(b"<d>") {
                depth += 1;
                i += 3;
                continue;
            } else if rem.starts_with(b"<d />") {
                if depth == 0 {
                    return Some(i + 5);
                }
                i += 5;
                continue;
            } else if rem.starts_with(b"</d>") {
                depth -= 1;
                i += 4;
                if depth <= 0 {
                    return Some(i);
                }
                continue;
            }
        }
        i += 1;
    }
    None
}

fn extract_plist_string_value_fast_skip_k4(block: &str, key: &str) -> Option<String> {
    let pat = format!("<k>{key}</k>");
    let mut pos = 0usize;
    while pos < block.len() {
        // Avoid scanning inside giant k4 strings
        let k4_pos = block[pos..].find("<k>k4</k><s>");
        let pat_pos = block[pos..].find(&pat);
        match (pat_pos, k4_pos) {
            (Some(pp), Some(kp)) if kp < pp => {
                let s_start = pos + kp + "<k>k4</k><s>".len();
                if let Some(s_end) = block[s_start..].find("</s>") {
                    pos = s_start + s_end + 4;
                    continue;
                } else {
                    return None;
                }
            }
            (Some(pp), _) => {
                let after = &block[pos + pp + pat.len()..];
                let trimmed = after.trim_start();
                if let Some(after_s) = trimmed.strip_prefix("<s>") {
                    let end = after_s.find("</s>")?;
                    return Some(xml_unescape(&after_s[..end]));
                } else if let Some(after_i) = trimmed.strip_prefix("<i>") {
                    let end = after_i.find("</i>")?;
                    return Some(after_i[..end].to_string());
                } else {
                    return None;
                }
            }
            (None, _) => return None,
        }
    }
    None
}

/// Parses `CCLocalLevels.dat` XML to list user-created editor levels safely.
pub fn parse_local_levels_from_xml(xml: &str) -> Vec<LocalLevelInfo> {
    let mut levels = Vec::new();
    let Some(llm_pos) = xml.find("<k>LLM_01</k>") else {
        return levels;
    };
    let section = &xml[llm_pos..];
    // Find each `<k>k_0</k><d>...` or `<k>k_N</k><d>` entry inside LLM_01
    let mut cursor = 0usize;
    let mut idx = 0usize;
    while let Some(rel) = section[cursor..].find("<k>k_") {
        let key_start = cursor + rel;
        let Some(dict_rel) = section[key_start..].find("<d>") else {
            break;
        };
        let dict_start = key_start + dict_rel;
        if let Some(dict_end) = find_matching_dict_end(section, dict_start) {
            let block = &section[dict_start..dict_end];
            let name = extract_plist_string_value(block, "k2")
                .unwrap_or_else(|| format!("Unnamed Level {}", idx + 1));
            let desc_b64 = extract_plist_string_value(block, "k3").unwrap_or_default();
            let description = decode_gd_description(&desc_b64);
            let revision = extract_plist_string_value(block, "k46").unwrap_or_else(|| "0".into());
            let song_id = extract_plist_string_value(block, "k45").unwrap_or_else(|| "0".into());

            levels.push(LocalLevelInfo {
                index: idx,
                name,
                description,
                revision,
                song_id,
                raw_dict_xml: block.to_string(),
            });
            idx += 1;
            cursor = dict_end;
        } else {
            break;
        }
    }
    levels
}

fn find_matching_dict_end(xml: &str, start_of_open_d: usize) -> Option<usize> {
    let bytes = xml.as_bytes();
    let mut i = start_of_open_d;
    let mut depth = 0i32;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let rem = &bytes[i..];
            if rem.starts_with(b"<d>") {
                depth += 1;
                i += 3;
                continue;
            } else if rem.starts_with(b"<d />") {
                if depth == 0 {
                    return Some(i + 5);
                }
                i += 5;
                continue;
            } else if rem.starts_with(b"</d>") {
                depth -= 1;
                i += 4;
                if depth <= 0 {
                    return Some(i);
                }
                continue;
            }
        }
        i += 1;
    }
    None
}

fn decode_gd_description(raw: &str) -> String {
    if raw.trim().is_empty() {
        return String::new();
    }
    let mut padded = raw.trim().to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    if let Ok(bytes) = URL_SAFE.decode(&padded) {
        if let Ok(text) = String::from_utf8(bytes) {
            return text;
        }
    }
    raw.to_string()
}

pub fn export_level_to_gmd(level: &LocalLevelInfo, dest_path: &Path) -> Result<(), String> {
    if level.raw_dict_xml.trim().is_empty() {
        return Err("Level XML payload is empty".into());
    }
    fs::write(dest_path, &level.raw_dict_xml)
        .map_err(|e| format!("write .gmd file {}: {e}", dest_path.display()))
}

pub fn inspect_save_dir(save_dir: &Path) -> SaveSummary {
    let mut summary = SaveSummary {
        inspected_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        ..Default::default()
    };

    let gm_path = save_dir.join("CCGameManager.dat");
    if gm_path.is_file() {
        match fs::read(&gm_path).map_err(|e| e.to_string()).and_then(|b| decode_gd_dat_to_xml(&b)) {
            Ok(xml) => {
                summary.game_manager_valid = true;
                let (name, acct) = extract_safe_player_meta(&xml);
                summary.player_name = name;
                summary.account_id = acct;
                summary.known_songs = parse_mdlm_songs_from_xml(&xml);
                summary.song_to_levels = parse_glm_level_songs_from_xml(&xml);
            }
            Err(e) => {
                summary.validation_error = Some(format!("CCGameManager.dat: {e}"));
            }
        }
    }

    let ll_path = save_dir.join("CCLocalLevels.dat");
    if ll_path.is_file() {
        match fs::read(&ll_path).map_err(|e| e.to_string()).and_then(|b| decode_gd_dat_to_xml(&b)) {
            Ok(xml) => {
                summary.local_levels_valid = true;
                let lvls = parse_local_levels_from_xml(&xml);
                for lvl in &lvls {
                    let sid = lvl.song_id.trim();
                    if !sid.is_empty() && sid != "0" && !lvl.name.trim().is_empty() {
                        let list = summary.song_to_levels.entry(sid.to_string()).or_default();
                        if !list.contains(&lvl.name) && list.len() < 8 {
                            list.push(lvl.name.clone());
                        }
                    }
                }
                summary.level_count = lvls.len();
                summary.levels = lvls;
            }
            Err(e) => {
                let msg = format!("CCLocalLevels.dat: {e}");
                summary.validation_error = Some(match summary.validation_error {
                    Some(prev) => format!("{prev}; {msg}"),
                    None => msg,
                });
            }
        }
    }

    summary
}
