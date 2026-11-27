use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

const GD_XOR_KEY: u8 = 11;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GmSongEntry {
    pub song_id: String,
    pub title: String,
    pub artist: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlayerStats {
    pub stars: u64,
    pub demons: u64,
    pub moons: u64,
    pub diamonds: u64,
    pub secret_coins: u64,
    pub user_coins: u64,
    pub orbs: u64,
    pub attempts: u64,
    pub jumps: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LevelProgressEntry {
    pub name: String,
    pub author: String,
    pub percent: u32,
    pub practice: u32,
    pub attempts: u32,
    pub is_demon: bool,
    pub stars: u32,
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
    #[serde(default)]
    pub player_stats: PlayerStats,
    #[serde(default)]
    pub active_grinds: Vec<LevelProgressEntry>,
    #[serde(default)]
    pub completed_demons: Vec<LevelProgressEntry>,
    #[serde(default)]
    pub level_progress_by_name: BTreeMap<String, (u32, u32)>,
}

/// Decodes a Geometry Dash `.dat` file (`XOR 0x0B` -> `URL-safe Base64` -> `Gzip` or `Zlib`) into XML in memory.
pub fn decode_gd_dat_to_xml(raw: &[u8]) -> Result<String, String> {
    if raw.is_empty() {
        return Err("Save file is 0 bytes".into());
    }
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

    let mut xml_bytes = Vec::new();
    let inflate_res = if compressed.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(&compressed[..]);
        decoder.read_to_end(&mut xml_bytes)
    } else {
        let mut zdec = ZlibDecoder::new(&compressed[..]);
        match zdec.read_to_end(&mut xml_bytes) {
            Ok(n) => Ok(n),
            Err(_) => {
                xml_bytes.clear();
                let mut gdec = GzDecoder::new(&compressed[..]);
                gdec.read_to_end(&mut xml_bytes)
            }
        }
    };
    inflate_res.map_err(|e| format!("Inflate failed: {e}"))?;
    let xml = String::from_utf8_lossy(&xml_bytes).into_owned();

    validate_gd_plist_xml(&xml)?;
    Ok(xml)
}

/// Encodes XML back into Geometry Dash's `.dat` wire format (`Gzip` -> `URL-safe Base64` -> `XOR 0x0B`).
pub fn encode_gd_dat_from_xml(xml: &str) -> Result<Vec<u8>, String> {
    validate_gd_plist_xml(xml)?;
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(xml.as_bytes())
        .map_err(|e| format!("Gzip compress failed: {e}"))?;
    let compressed = gz.finish().map_err(|e| format!("Gzip finish failed: {e}"))?;
    let b64 = URL_SAFE.encode(compressed);
    Ok(b64.bytes().map(|b| b ^ GD_XOR_KEY).collect())
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
    let name = extract_plist_string_value(xml, "GJA_001")
        .or_else(|| extract_plist_string_value(xml, "playerName"))
        .unwrap_or_else(|| "Player".into());
    let acct = extract_plist_string_value(xml, "GJA_003").unwrap_or_default();
    (name, acct)
}

/// Extracts `GS_value` progression counters (stars, demons, moons, diamonds, coins, orbs, attempts, jumps).
pub fn parse_gs_value_stats(xml: &str) -> PlayerStats {
    let mut stats = PlayerStats::default();
    let Some(gs_pos) = xml.find("<k>GS_value</k>") else {
        return stats;
    };
    let rest = &xml[gs_pos + "<k>GS_value</k>".len()..];
    let Some(dict_rel) = rest.find("<d>") else {
        return stats;
    };
    let dict_start = gs_pos + "<k>GS_value</k>".len() + dict_rel;
    let Some(dict_end) = find_matching_dict_end(xml, dict_start) else {
        return stats;
    };
    let section = &xml[dict_start..dict_end];

    let read_stat = |k: &str| -> u64 {
        extract_plist_string_value(section, k)
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0)
    };
    stats.jumps = read_stat("1");
    stats.attempts = read_stat("2");
    stats.demons = read_stat("5");
    stats.stars = read_stat("6");
    stats.secret_coins = read_stat("8");
    stats.user_coins = read_stat("12");
    stats.diamonds = read_stat("13");
    stats.orbs = read_stat("14");
    stats.moons = read_stat("28");
    stats
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

struct GlmParsedData {
    song_to_levels: BTreeMap<String, Vec<String>>,
    active_grinds: Vec<LevelProgressEntry>,
    completed_demons: Vec<LevelProgressEntry>,
    level_progress_by_name: BTreeMap<String, (u32, u32)>,
}

fn has_plist_true_tag(block: &str, key: &str) -> bool {
    let pat = format!("<k>{key}</k>");
    let Some(pos) = block.find(&pat) else {
        return false;
    };
    let trimmed = block[pos + pat.len()..].trim_start();
    trimmed.starts_with("<t/>") || trimmed.starts_with("<t />")
}

/// Efficiently extracts `song_id -> Vec<level_name>`, completed demons, active grinds,
/// and level progress map from `GLM_03` in `CCGameManager.dat`, skipping massive `<k>k4</k><s>...</s>` level object strings.
fn parse_glm_data_from_xml(xml: &str) -> GlmParsedData {
    let mut out = GlmParsedData {
        song_to_levels: BTreeMap::new(),
        active_grinds: Vec::new(),
        completed_demons: Vec::new(),
        level_progress_by_name: BTreeMap::new(),
    };
    let Some(glm_pos) = xml.find("<k>GLM_03</k>") else {
        return out;
    };
    let after_key = &xml[glm_pos + "<k>GLM_03</k>".len()..];
    let Some(dict_rel) = after_key.find("<d>") else {
        return out;
    };
    let glm_start = glm_pos + "<k>GLM_03</k>".len() + dict_rel;
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
                let clean_name = extract_plist_string_value_fast_skip_k4(block, "k2")
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                if !clean_name.is_empty() {
                    if let Some(song_id) = extract_plist_string_value_fast_skip_k4(block, "k45") {
                        let sid = song_id.trim().to_string();
                        if !sid.is_empty() && sid != "0" {
                            let list = out.song_to_levels.entry(sid).or_default();
                            if !list.contains(&clean_name) && list.len() < 8 {
                                list.push(clean_name.clone());
                            }
                        }
                    }

                    if let Some(pct_s) = extract_plist_string_value_fast_skip_k4(block, "k19") {
                        if let Ok(percent) = pct_s.trim().parse::<u32>() {
                            let author = extract_plist_string_value_fast_skip_k4(block, "k5")
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| "Unknown".into());
                            let practice = extract_plist_string_value_fast_skip_k4(block, "k21")
                                .and_then(|s| s.trim().parse::<u32>().ok())
                                .unwrap_or(0);
                            let attempts = extract_plist_string_value_fast_skip_k4(block, "k18")
                                .and_then(|s| s.trim().parse::<u32>().ok())
                                .unwrap_or(0);
                            let is_demon = has_plist_true_tag(block, "k25");
                            let stars = extract_plist_string_value_fast_skip_k4(block, "k26")
                                .and_then(|s| s.trim().parse::<u32>().ok())
                                .unwrap_or(if is_demon { 10 } else { 0 });

                            let key = clean_name.to_lowercase();
                            let prev = out.level_progress_by_name.entry(key).or_insert((0, 0));
                            prev.0 = prev.0.max(percent);
                            prev.1 = prev.1.max(attempts);

                            let entry = LevelProgressEntry {
                                name: clean_name,
                                author,
                                percent,
                                practice,
                                attempts,
                                is_demon,
                                stars,
                            };
                            if percent >= 100 && (is_demon || stars >= 10) {
                                out.completed_demons.push(entry);
                            } else if (15..100).contains(&percent) {
                                out.active_grinds.push(entry);
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

    out.active_grinds
        .sort_by(|a, b| b.percent.cmp(&a.percent).then(b.practice.cmp(&a.practice)));
    out.active_grinds.truncate(30);
    out.completed_demons
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out.completed_demons.truncate(100);
    out
}

pub fn parse_glm_level_songs_from_xml(xml: &str) -> BTreeMap<String, Vec<String>> {
    parse_glm_data_from_xml(xml).song_to_levels
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

pub(crate) fn find_matching_dict_end(xml: &str, start_of_open_d: usize) -> Option<usize> {
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
                summary.player_stats = parse_gs_value_stats(&xml);
                let glm = parse_glm_data_from_xml(&xml);
                summary.song_to_levels = glm.song_to_levels;
                summary.active_grinds = glm.active_grinds;
                summary.completed_demons = glm.completed_demons;
                summary.level_progress_by_name = glm.level_progress_by_name;
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
