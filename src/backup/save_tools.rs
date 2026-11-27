use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::backup::save_inspector::{
    decode_gd_dat_to_xml, encode_gd_dat_from_xml, find_matching_dict_end, parse_gs_value_stats,
};
use crate::backup::vault::{create_snapshot_with_paths, SAVE_FILES};
use crate::backup::crypto::default_vault_key_path;
use crate::launcher::process::is_gd_running;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SaveBloatReport {
    pub file_size_bytes: u64,
    pub total_levels: usize,
    pub online_cached: usize,
    pub user_created: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SaveCompactResult {
    pub removed_levels: usize,
    pub old_size_bytes: u64,
    pub new_size_bytes: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SaveSlotInfo {
    pub name: String,
    pub path: PathBuf,
    pub has_saves: bool,
    pub player_name: String,
    pub stars: u64,
    pub demons: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SalvageCandidate {
    pub path: PathBuf,
    pub relative_path: String,
    pub size_bytes: u64,
    pub player_name: String,
    pub stars: u64,
    pub demons: u64,
    pub modified_str: String,
    pub modified_ts: u64,
}

fn entry_is_online_cache(entry_xml: &str) -> bool {
    let has_k1_int = entry_xml.contains("<k>k1</k><i>");
    if !has_k1_int {
        return false;
    }
    entry_xml.contains("<k>k21</k><i>1</i>") || !entry_xml.contains("<k>k21</k>")
}

/// Inspects `CCLocalLevels.dat` for downloaded online level cache bloat.
pub fn analyze_save_bloat(save_dir: &Path) -> Result<SaveBloatReport, String> {
    let ll_path = save_dir.join("CCLocalLevels.dat");
    if !ll_path.is_file() {
        return Err("CCLocalLevels.dat does not exist in save directory.".into());
    }
    let raw_bytes = fs::read(&ll_path).map_err(|e| format!("read CCLocalLevels.dat: {e}"))?;
    let file_size_bytes = raw_bytes.len() as u64;
    let xml = decode_gd_dat_to_xml(&raw_bytes)?;

    let Some(llm_pos) = xml.find("<k>LLM_01</k>") else {
        return Err("Could not locate LLM_01 in CCLocalLevels.dat.".into());
    };
    let rest = &xml[llm_pos + "<k>LLM_01</k>".len()..];
    let Some(dict_rel) = rest.find("<d>") else {
        return Ok(SaveBloatReport {
            file_size_bytes,
            total_levels: 0,
            online_cached: 0,
            user_created: 0,
        });
    };
    let dict_start = llm_pos + "<k>LLM_01</k>".len() + dict_rel;
    let dict_end = find_matching_dict_end(&xml, dict_start)
        .ok_or_else(|| "Malformed LLM_01 dictionary in CCLocalLevels.dat".to_string())?;
    let body = &xml[dict_start + 3..dict_end.saturating_sub(4)];

    let mut cursor = 0usize;
    let mut total_levels = 0usize;
    let mut online_cached = 0usize;
    while let Some(rel) = body[cursor..].find("<k>k_") {
        let key_start = cursor + rel;
        let Some(open_rel) = body[key_start..].find("<d>") else {
            break;
        };
        let entry_dict_start = key_start + open_rel;
        if let Some(entry_dict_end) = find_matching_dict_end(body, entry_dict_start) {
            let entry_xml = &body[key_start..entry_dict_end];
            total_levels += 1;
            if entry_is_online_cache(entry_xml) {
                online_cached += 1;
            }
            cursor = entry_dict_end;
        } else {
            break;
        }
    }

    Ok(SaveBloatReport {
        file_size_bytes,
        total_levels,
        online_cached,
        user_created: total_levels.saturating_sub(online_cached),
    })
}

/// Removes online-cached levels from `CCLocalLevels.dat` while keeping user-created levels intact
/// and writing a timestamped `.bak` copy before modifying the save file.
pub fn clean_save_bloat(save_dir: &Path) -> Result<SaveCompactResult, String> {
    if is_gd_running() {
        return Err("Close Geometry Dash before compacting CCLocalLevels.dat.".into());
    }
    let ll_path = save_dir.join("CCLocalLevels.dat");
    if !ll_path.is_file() {
        return Err("CCLocalLevels.dat does not exist.".into());
    }

    let raw_bytes = fs::read(&ll_path).map_err(|e| format!("read CCLocalLevels.dat: {e}"))?;
    let old_size_bytes = raw_bytes.len() as u64;
    let xml = decode_gd_dat_to_xml(&raw_bytes)?;

    let Some(llm_pos) = xml.find("<k>LLM_01</k>") else {
        return Err("Could not locate LLM_01 in CCLocalLevels.dat.".into());
    };
    let rest = &xml[llm_pos + "<k>LLM_01</k>".len()..];
    let Some(dict_rel) = rest.find("<d>") else {
        return Err("Could not locate LLM_01 dict body.".into());
    };
    let dict_start = llm_pos + "<k>LLM_01</k>".len() + dict_rel;
    let dict_end = find_matching_dict_end(&xml, dict_start)
        .ok_or_else(|| "Malformed LLM_01 dictionary.".to_string())?;
    let body_start = dict_start + 3;
    let body_end = dict_end - 4;
    let body = &xml[body_start..body_end];

    let mut cursor = 0usize;
    let mut kept = Vec::new();
    let mut removed_levels = 0usize;

    while let Some(rel) = body[cursor..].find("<k>k_") {
        let key_start = cursor + rel;
        let Some(k_end_rel) = body[key_start..].find("</k>") else {
            break;
        };
        let key_tag_end = key_start + k_end_rel + 4;
        let Some(open_rel) = body[key_tag_end..].find("<d>") else {
            break;
        };
        let entry_dict_start = key_tag_end + open_rel;
        if let Some(entry_dict_end) = find_matching_dict_end(body, entry_dict_start) {
            let full_entry = &body[key_start..entry_dict_end];
            if entry_is_online_cache(full_entry) {
                removed_levels += 1;
            } else {
                let tail = &body[key_tag_end..entry_dict_end];
                let new_idx = kept.len();
                kept.push(format!("<k>k_{new_idx}</k>{tail}"));
            }
            cursor = entry_dict_end;
        } else {
            break;
        }
    }

    let mut new_body = String::from("<k>_isArr</k><t/>");
    for item in kept {
        new_body.push_str(&item);
    }

    let mut new_xml = String::with_capacity(xml.len() - (body_end - body_start) + new_body.len());
    new_xml.push_str(&xml[..body_start]);
    new_xml.push_str(&new_body);
    new_xml.push_str(&xml[body_end..]);

    let encoded = encode_gd_dat_from_xml(&new_xml)?;
    // Verify roundtrip decode before touching live save file
    decode_gd_dat_to_xml(&encoded)?;

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let bak_path = save_dir.join(format!("CCLocalLevels.dat.bak_{ts}"));
    fs::copy(&ll_path, &bak_path).map_err(|e| format!("create pre-compact backup: {e}"))?;

    let tmp_path = save_dir.join("CCLocalLevels.dat.tmp");
    fs::write(&tmp_path, &encoded).map_err(|e| format!("write compacted save: {e}"))?;
    fs::rename(&tmp_path, &ll_path).map_err(|e| format!("finalize compacted save: {e}"))?;
    let new_size_bytes = encoded.len() as u64;

    Ok(SaveCompactResult {
        removed_levels,
        old_size_bytes,
        new_size_bytes,
    })
}

pub fn save_slots_dir(save_dir: &Path) -> PathBuf {
    save_dir.join("GeodeSync_Save_Slots")
}

fn sanitize_slot_label(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == ' ')
        .collect::<String>()
        .trim()
        .to_string()
}

fn parse_gm_slot_preview(gm_path: &Path) -> (String, u64, u64) {
    let Ok(bytes) = fs::read(gm_path) else {
        return ("Player".into(), 0, 0);
    };
    let Ok(xml) = decode_gd_dat_to_xml(&bytes) else {
        return ("Player".into(), 0, 0);
    };
    let name = extract_quick_tag(&xml, "GJA_001")
        .or_else(|| extract_quick_tag(&xml, "playerName"))
        .unwrap_or_else(|| "Player".into());
    let stats = parse_gs_value_stats(&xml);
    (name, stats.stars, stats.demons)
}

fn extract_quick_tag(xml: &str, key: &str) -> Option<String> {
    let pat = format!("<k>{key}</k>");
    let pos = xml.find(&pat)?;
    let rest = xml[pos + pat.len()..].trim_start();
    if let Some(after_s) = rest.strip_prefix("<s>") {
        let end = after_s.find("</s>")?;
        return Some(after_s[..end].to_string());
    }
    None
}

pub fn list_save_slots(save_dir: &Path) -> Vec<SaveSlotInfo> {
    let root = save_slots_dir(save_dir);
    let Ok(rd) = fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut slots = Vec::new();
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let gm = p.join("CCGameManager.dat");
            let has_saves = gm.is_file();
            let (player_name, stars, demons) = if has_saves {
                parse_gm_slot_preview(&gm)
            } else {
                ("Empty Slot".into(), 0, 0)
            };
            slots.push(SaveSlotInfo {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: p,
                has_saves,
                player_name,
                stars,
                demons,
            });
        }
    }
    slots.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    slots
}

pub fn create_save_slot(save_dir: &Path, slot_name: &str) -> Result<String, String> {
    let clean = sanitize_slot_label(slot_name);
    if clean.is_empty() {
        return Err("Slot name cannot be empty.".into());
    }
    let slots_root = save_slots_dir(save_dir);
    fs::create_dir_all(&slots_root).map_err(|e| format!("create save slots dir: {e}"))?;
    let target = slots_root.join(&clean);
    if target.exists() {
        return Err(format!("Save slot '{clean}' already exists."));
    }
    fs::create_dir_all(&target).map_err(|e| format!("create slot dir: {e}"))?;

    let mut copied = 0usize;
    if let Ok(rd) = fs::read_dir(save_dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_file() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(".dat") || name.ends_with(".dat.bak") {
                    fs::copy(&p, target.join(&name)).map_err(|e| format!("copy {name}: {e}"))?;
                    copied += 1;
                }
            }
        }
    }
    if copied == 0 {
        let _ = fs::remove_dir_all(&target);
        return Err("No .dat save files found to store in slot.".into());
    }
    Ok(format!("Created save slot '{clean}' ({copied} save file(s))."))
}

pub fn switch_save_slot(
    save_dir: &Path,
    slot_name: &str,
    vault_backup_dir: &Path,
    key_path: &Path,
) -> Result<String, String> {
    if is_gd_running() {
        return Err("Close Geometry Dash before switching save slots.".into());
    }
    let slot_dir = save_slots_dir(save_dir).join(slot_name);
    if !slot_dir.is_dir() {
        return Err("Selected save slot directory does not exist.".into());
    }
    // Always protect current live saves first
    let _ = create_snapshot_with_paths(
        save_dir,
        vault_backup_dir,
        key_path,
        &format!("Pre-Slot Switch ({slot_name})"),
        true,
        false,
        false,
        25,
    );

    let mut restored = 0usize;
    let rd = fs::read_dir(&slot_dir).map_err(|e| format!("read slot: {e}"))?;
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".dat") || name.ends_with(".dat.bak") {
                fs::copy(&p, save_dir.join(&name)).map_err(|e| format!("restore {name}: {e}"))?;
                restored += 1;
            }
        }
    }
    Ok(format!(
        "Switched to save slot '{slot_name}' ({restored} file(s) applied)."
    ))
}

pub fn delete_save_slot(save_dir: &Path, slot_name: &str) -> Result<String, String> {
    let clean = sanitize_slot_label(slot_name);
    if clean.is_empty() {
        return Err("Invalid slot name.".into());
    }
    let slot_dir = save_slots_dir(save_dir).join(&clean);
    if slot_dir.is_dir() {
        fs::remove_dir_all(&slot_dir).map_err(|e| format!("delete save slot: {e}"))?;
    }
    Ok(format!("Deleted save slot '{clean}'."))
}

/// Scans `save_dir` (and nested backup folders) for valid `CCGameManager*.dat` / `.bak` candidates.
pub fn find_salvageable_saves(save_dir: &Path) -> Vec<SalvageCandidate> {
    let mut out = Vec::new();
    let mut stack = vec![(save_dir.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 4 {
            continue;
        }
        let Ok(rd) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push((p, depth + 1));
            } else if p.is_file() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("CCGameManager")
                    && (name.ends_with(".dat") || name.contains(".bak"))
                {
                    let Ok(meta) = entry.metadata() else {
                        continue;
                    };
                    if meta.len() < 100 {
                        continue;
                    }
                    let Ok(raw) = fs::read(&p) else {
                        continue;
                    };
                    if decode_gd_dat_to_xml(&raw).is_err() {
                        continue;
                    }
                    let (player_name, stars, demons) = parse_gm_slot_preview(&p);
                    let modified_ts = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let modified_str = meta
                        .modified()
                        .ok()
                        .map(|t| {
                            let dt: chrono::DateTime<chrono::Local> = t.into();
                            dt.format("%Y-%m-%d %H:%M:%S").to_string()
                        })
                        .unwrap_or_else(|| "Unknown".into());
                    let relative_path = p
                        .strip_prefix(save_dir)
                        .map(|r| r.to_string_lossy().replace('\\', "/"))
                        .unwrap_or(name);

                    out.push(SalvageCandidate {
                        path: p,
                        relative_path,
                        size_bytes: meta.len(),
                        player_name,
                        stars,
                        demons,
                        modified_str,
                        modified_ts,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| b.modified_ts.cmp(&a.modified_ts));
    out
}

/// Resurrects live `CCGameManager.dat` (and sibling `CCLocalLevels.dat` if present) from a salvage candidate.
pub fn resurrect_save(
    save_dir: &Path,
    candidate_gm_path: &Path,
    vault_backup_dir: &Path,
) -> Result<String, String> {
    if is_gd_running() {
        return Err("Close Geometry Dash before resurrecting save files.".into());
    }
    if !candidate_gm_path.is_file() {
        return Err("Selected salvage candidate no longer exists.".into());
    }
    let raw = fs::read(candidate_gm_path).map_err(|e| format!("read salvage file: {e}"))?;
    decode_gd_dat_to_xml(&raw).map_err(|e| format!("Salvage file failed XML check: {e}"))?;

    let _ = create_snapshot_with_paths(
        save_dir,
        vault_backup_dir,
        &default_vault_key_path(),
        "Pre-Save Resurrection",
        true,
        false,
        false,
        25,
    );

    fs::copy(candidate_gm_path, save_dir.join("CCGameManager.dat"))
        .map_err(|e| format!("restore CCGameManager.dat: {e}"))?;

    if let Some(parent) = candidate_gm_path.parent() {
        for ll_name in ["CCLocalLevels.dat", "CCLocalLevels.dat.bak"] {
            let sibling = parent.join(ll_name);
            if sibling.is_file() {
                if let Ok(bytes) = fs::read(&sibling) {
                    if decode_gd_dat_to_xml(&bytes).is_ok() {
                        let _ = fs::copy(&sibling, save_dir.join("CCLocalLevels.dat"));
                        break;
                    }
                }
            }
        }
    }

    Ok("Save files resurrected successfully! Pre-resurrection snapshot stored in Vault.".into())
}

/// Exports saves, Geode mods, and mod configs into a portable `.geodesync` ZIP archive.
pub fn export_geodesync_bundle(
    save_dir: &Path,
    game_path: &str,
    dest_path: &Path,
    include_saves: bool,
    include_mods: bool,
    include_configs: bool,
) -> Result<String, String> {
    let gm = save_dir.join("CCGameManager.dat");
    let (player_name, stars, demons) = if gm.is_file() {
        parse_gm_slot_preview(&gm)
    } else {
        ("Player".into(), 0, 0)
    };

    let tmp_path = dest_path.with_extension("geodesync.tmp");
    let file = File::create(&tmp_path).map_err(|e| format!("create bundle: {e}"))?;
    let mut writer = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut contents = Vec::new();

    if include_saves && save_dir.is_dir() {
        for name in SAVE_FILES {
            let p = save_dir.join(name);
            if p.is_file() {
                let data = fs::read(&p).map_err(|e| format!("read {name}: {e}"))?;
                let arc = format!("saves/{name}");
                writer
                    .start_file(&arc, opts)
                    .map_err(|e| format!("zip start {arc}: {e}"))?;
                writer
                    .write_all(&data)
                    .map_err(|e| format!("zip write {arc}: {e}"))?;
                contents.push(arc);
            }
        }
    }

    if !game_path.trim().is_empty() {
        let geode_root = crate::core::platform::gd_geode_dir(Path::new(game_path));
        if include_mods {
            let mods_dir = geode_root.join("mods");
            if let Ok(rd) = fs::read_dir(&mods_dir) {
                for entry in rd.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if name.ends_with(".geode") || name.ends_with(".geode.disabled") {
                            let data = fs::read(&p).map_err(|e| format!("read mod {name}: {e}"))?;
                            let arc = format!("mods/{name}");
                            writer
                                .start_file(&arc, opts)
                                .map_err(|e| format!("zip start {arc}: {e}"))?;
                            writer
                                .write_all(&data)
                                .map_err(|e| format!("zip write {arc}: {e}"))?;
                            contents.push(arc);
                        }
                    }
                }
            }
        }
        if include_configs {
            let cfg_dir = geode_root.join("config");
            if cfg_dir.is_dir() {
                let mut stack = vec![cfg_dir.clone()];
                while let Some(dir) = stack.pop() {
                    let Ok(rd) = fs::read_dir(&dir) else {
                        continue;
                    };
                    for entry in rd.flatten() {
                        let p = entry.path();
                        if p.is_dir() {
                            stack.push(p);
                        } else if p.is_file() {
                            if let Ok(rel) = p.strip_prefix(&cfg_dir) {
                                let rel_str = rel.to_string_lossy().replace('\\', "/");
                                let data = fs::read(&p).map_err(|e| format!("read cfg: {e}"))?;
                                let arc = format!("configs/{rel_str}");
                                writer
                                    .start_file(&arc, opts)
                                    .map_err(|e| format!("zip start {arc}: {e}"))?;
                                writer
                                    .write_all(&data)
                                    .map_err(|e| format!("zip write {arc}: {e}"))?;
                                contents.push(arc);
                            }
                        }
                    }
                }
            }
        }
    }

    let manifest = json!({
        "format_version": "1.0",
        "created_at": chrono::Local::now().to_rfc3339(),
        "geodeck_version": crate::core::updater::current_app_display_version(),
        "player_name": player_name,
        "stars": stars,
        "demons": demons,
        "contents": contents,
    });
    writer
        .start_file("manifest.json", opts)
        .map_err(|e| format!("zip start manifest: {e}"))?;
    writer
        .write_all(
            serde_json::to_string_pretty(&manifest)
                .unwrap_or_default()
                .as_bytes(),
        )
        .map_err(|e| format!("zip write manifest: {e}"))?;
    writer.finish().map_err(|e| format!("finish zip: {e}"))?;

    if dest_path.exists() {
        let _ = fs::remove_file(dest_path);
    }
    fs::rename(&tmp_path, dest_path).map_err(|e| format!("finalize bundle: {e}"))?;
    Ok(format!(
        "Exported portable bundle ({} items) to {}",
        contents.len(),
        dest_path.display()
    ))
}

/// Imports saves, mods, and configs from a portable `.geodesync` bundle with pre-import safety backup.
pub fn import_geodesync_bundle(
    bundle_path: &Path,
    save_dir: &Path,
    game_path: &str,
    overwrite_saves: bool,
    overwrite_mods: bool,
    vault_backup_dir: &Path,
) -> Result<String, String> {
    if is_gd_running() {
        return Err("Close Geometry Dash before importing a .geodesync bundle.".into());
    }
    if !bundle_path.is_file() {
        return Err("Bundle file does not exist.".into());
    }

    let _ = create_snapshot_with_paths(
        save_dir,
        vault_backup_dir,
        &default_vault_key_path(),
        "Pre-Bundle Import",
        true,
        false,
        false,
        25,
    );

    let file = File::open(bundle_path).map_err(|e| format!("open bundle: {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("invalid bundle zip: {e}"))?;

    let mut saves_cnt = 0usize;
    let mut mods_cnt = 0usize;
    let mut cfg_cnt = 0usize;

    let geode_root = (!game_path.trim().is_empty())
        .then(|| crate::core::platform::gd_geode_dir(Path::new(game_path)));

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| format!("zip entry: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        let Some(rel) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        if rel_str.starts_with("saves/") && overwrite_saves {
            if let Some(fname) = rel.file_name() {
                fs::create_dir_all(save_dir).map_err(|e| format!("mkdir save: {e}"))?;
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                fs::write(save_dir.join(fname), &buf).map_err(|e| e.to_string())?;
                saves_cnt += 1;
            }
        } else if rel_str.starts_with("mods/") && overwrite_mods {
            if let (Some(ref gr), Some(fname)) = (&geode_root, rel.file_name()) {
                let mods_dir = gr.join("mods");
                fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                fs::write(mods_dir.join(fname), &buf).map_err(|e| e.to_string())?;
                mods_cnt += 1;
            }
        } else if let Some(cfg_rel) = rel_str.strip_prefix("configs/") {
            if let Some(ref gr) = geode_root {
                let dest = gr.join("config").join(cfg_rel);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                fs::write(&dest, &buf).map_err(|e| e.to_string())?;
                cfg_cnt += 1;
            }
        }
    }

    Ok(format!(
        "Imported bundle: {saves_cnt} save(s), {mods_cnt} mod(s), {cfg_cnt} config file(s)."
    ))
}
