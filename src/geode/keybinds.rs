use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeybindBinding {
    pub mod_id: String,
    pub action: String,
    pub key_combo: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KeybindReport {
    pub total_bindings: usize,
    pub conflicts: BTreeMap<String, Vec<KeybindBinding>>,
    pub all_bindings: BTreeMap<String, Vec<KeybindBinding>>,
}

pub fn glfw_key_name(code: i64) -> String {
    match code {
        32 => "Space".into(),
        39 => "'".into(),
        44 => ",".into(),
        45 => "-".into(),
        46 => ".".into(),
        47 => "/".into(),
        48..=57 => ((code as u8) as char).to_string(),
        59 => ";".into(),
        61 => "=".into(),
        65..=90 => ((code as u8) as char).to_string(),
        256 => "Escape".into(),
        257 => "Enter".into(),
        258 => "Tab".into(),
        259 => "Backspace".into(),
        260 => "Insert".into(),
        261 => "Delete".into(),
        262 => "Right".into(),
        263 => "Left".into(),
        264 => "Down".into(),
        265 => "Up".into(),
        266 => "PageUp".into(),
        267 => "PageDown".into(),
        268 => "Home".into(),
        269 => "End".into(),
        290..=301 => format!("F{}", code - 289),
        other => format!("Key {other}"),
    }
}

fn extract_keybinds_recursive(
    val: &Value,
    mod_id: &str,
    prefix: &str,
    out: &mut BTreeMap<String, Vec<KeybindBinding>>,
) {
    match val {
        Value::Object(map) => {
            if let Some(k_code) = map.get("key").and_then(|v| v.as_i64()) {
                if k_code > 0 {
                    let mut combo = glfw_key_name(k_code);
                    let mods = map.get("modifiers").and_then(|v| v.as_i64()).unwrap_or(0);
                    if mods & 1 != 0 {
                        combo.push_str("+Shift");
                    }
                    if mods & 2 != 0 {
                        combo.push_str("+Ctrl");
                    }
                    if mods & 4 != 0 {
                        combo.push_str("+Alt");
                    }
                    let action = if prefix.is_empty() {
                        "hotkey".to_string()
                    } else {
                        prefix.to_string()
                    };
                    out.entry(combo.clone()).or_default().push(KeybindBinding {
                        mod_id: mod_id.to_string(),
                        action,
                        key_combo: combo,
                    });
                    return;
                }
            }
            for (k, v) in map {
                let next = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                extract_keybinds_recursive(v, mod_id, &next, out);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                extract_keybinds_recursive(item, mod_id, prefix, out);
            }
        }
        _ => {}
    }
}

pub fn scan_keybind_conflicts_in_dirs(dirs: &[&Path]) -> KeybindReport {
    let mut all_bindings: BTreeMap<String, Vec<KeybindBinding>> = BTreeMap::new();
    let mut stack: Vec<std::path::PathBuf> = dirs
        .iter()
        .filter(|d| d.is_dir())
        .map(|d| d.to_path_buf())
        .collect();

    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.is_file() {
                let fname = entry.file_name().to_string_lossy().to_lowercase();
                if matches!(fname.as_str(), "settings.json" | "saved.json" | "config.json") {
                    let mod_id = p
                        .parent()
                        .and_then(|par| par.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "unknown".into());
                    if let Ok(raw) = fs::read_to_string(&p) {
                        if let Ok(json) = serde_json::from_str::<Value>(&raw) {
                            extract_keybinds_recursive(&json, &mod_id, "", &mut all_bindings);
                        }
                    }
                }
            }
        }
    }

    let mut conflicts = BTreeMap::new();
    let mut total_bindings = 0usize;
    for (combo, bindings) in &all_bindings {
        total_bindings += bindings.len();
        let distinct_mods: HashSet<&str> = bindings.iter().map(|b| b.mod_id.as_str()).collect();
        if distinct_mods.len() > 1 {
            conflicts.insert(combo.clone(), bindings.clone());
        }
    }

    KeybindReport {
        total_bindings,
        conflicts,
        all_bindings,
    }
}

pub fn scan_keybind_conflicts(game_path: &str, local_data_dir: &Path) -> KeybindReport {
    let local_mods = local_data_dir.join("geode").join("mods");
    let game_cfg = crate::core::platform::gd_geode_dir(Path::new(game_path)).join("config");
    let game_mods_cfg = crate::core::platform::gd_geode_dir(Path::new(game_path)).join("mods");
    scan_keybind_conflicts_in_dirs(&[
        local_mods.as_path(),
        game_cfg.as_path(),
        game_mods_cfg.as_path(),
    ])
}
