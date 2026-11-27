use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepImportance {
    Required,
    Recommended,
    Suggested,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModDependency {
    pub id: String,
    pub version: String,
    pub importance: DepImportance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModIncompatibility {
    pub id: String,
    pub version: String,
    pub importance: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParsedModJson {
    pub id: String,
    pub name: String,
    pub version: String,
    pub developer: String,
    pub description: String,
    pub geode: String,
    pub gd_win: Option<String>,
    pub tags: Vec<String>,
    pub early_load: bool,
    pub api: bool,
    pub repository: Option<String>,
    pub dependencies: Vec<ModDependency>,
    pub incompatibilities: Vec<ModIncompatibility>,
    pub has_settings: bool,
}

#[derive(Clone, Debug)]
pub struct InstalledMod {
    pub id: String,
    pub name: String,
    pub version: String,
    pub developer: String,
    pub description: String,
    pub geode_target: String,
    pub gd_win_target: Option<String>,
    pub tags: Vec<String>,
    pub early_load: bool,
    pub is_api: bool,
    pub enabled: bool,
    pub pinned: bool,
    pub file_disabled_ext: bool,
    pub path: String,
    pub file_size: u64,
    pub sha256: String,
    pub modified: String,
    pub dependencies: Vec<ModDependency>,
    pub incompatibilities: Vec<ModIncompatibility>,
    pub has_settings_schema: bool,
    pub has_user_settings: bool,
    pub logo_rgba: Option<(u32, u32, Vec<u8>)>,
}

pub fn local_gd_data_dir() -> PathBuf {
    crate::core::platform::resolve_gd_save_dir(None)
}

pub fn geode_loader_saved_path(local_data_dir: &Path) -> PathBuf {
    local_data_dir
        .join("geode")
        .join("mods")
        .join("geode.loader")
        .join("saved.json")
}

/// Mod IDs end up in file names and directory paths (settings, unzipped cache, `.geode`
/// packages). They come from untrusted data (`mod.json` inside packages, `.gdpack`
/// manifests, the remote index), so anything that could escape its folder is rejected.
/// Geode IDs only use ASCII letters, digits, `-`, `_` and `.`.
pub fn is_safe_mod_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.starts_with('.')
        && !id.contains("..")
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

pub fn mod_settings_path(local_data_dir: &Path, mod_id: &str) -> PathBuf {
    // Defence in depth: never let an unvalidated ID choose a path outside geode/mods.
    let component = if is_safe_mod_id(mod_id) {
        mod_id
    } else {
        "_invalid-mod-id"
    };
    local_data_dir
        .join("geode")
        .join("mods")
        .join(component)
        .join("settings.json")
}

fn load_loader_saved_json(local_data_dir: &Path) -> serde_json::Map<String, Value> {
    let path = geode_loader_saved_path(local_data_dir);
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn save_loader_saved_json(
    local_data_dir: &Path,
    map: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    let path = geode_loader_saved_path(local_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create geode.loader dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(&Value::Object(map.clone()))
        .map_err(|e| format!("serialize saved.json: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("write saved.json: {e}"))
}

pub fn parse_mod_json_str(raw: &str, fallback_id: &str) -> Result<ParsedModJson, String> {
    let root: Value = serde_json::from_str(raw).map_err(|e| format!("parse mod.json: {e}"))?;
    let obj = root
        .as_object()
        .ok_or_else(|| "mod.json root is not an object".to_string())?;

    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or(fallback_id)
        .to_string();
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();
    let version = obj
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();
    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let geode = obj
        .get("geode")
        .and_then(|v| v.as_str())
        .unwrap_or("*")
        .to_string();

    let developer = if let Some(s) = obj.get("developer").and_then(|v| v.as_str()) {
        s.to_string()
    } else if let Some(arr) = obj.get("developers").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        "Unknown".into()
    };

    let gd_win = match obj.get("gd") {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Object(m)) => m
            .get("win")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    };

    let tags = obj
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let early_load = obj
        .get("early-load")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let api = obj.get("api").is_some_and(|v| !v.is_null() && v != &Value::Bool(false));
    let repository = obj
        .get("repository")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let has_settings = obj
        .get("settings")
        .and_then(|v| v.as_object())
        .is_some_and(|m| !m.is_empty());

    let dependencies = parse_dependencies(obj.get("dependencies"));
    let incompatibilities = parse_incompatibilities(obj.get("incompatibilities"));

    Ok(ParsedModJson {
        id,
        name,
        version,
        developer,
        description,
        geode,
        gd_win,
        tags,
        early_load,
        api,
        repository,
        dependencies,
        incompatibilities,
        has_settings,
    })
}

fn parse_importance(val: Option<&Value>) -> DepImportance {
    match val.and_then(|v| v.as_str()).unwrap_or("required") {
        "recommended" => DepImportance::Recommended,
        "suggested" => DepImportance::Suggested,
        _ => DepImportance::Required,
    }
}

fn parse_dependencies(val: Option<&Value>) -> Vec<ModDependency> {
    let mut deps = Vec::new();
    let Some(v) = val else {
        return deps;
    };
    match v {
        Value::Array(arr) => {
            for item in arr {
                if let Some(obj) = item.as_object() {
                    if let Some(id) = obj.get("id").and_then(|x| x.as_str()) {
                        let version = obj
                            .get("version")
                            .and_then(|x| x.as_str())
                            .unwrap_or("*")
                            .to_string();
                        let importance = if obj.get("required").and_then(|x| x.as_bool()) == Some(false) {
                            DepImportance::Suggested
                        } else {
                            parse_importance(obj.get("importance"))
                        };
                        deps.push(ModDependency {
                            id: id.to_string(),
                            version,
                            importance,
                        });
                    }
                }
            }
        }
        Value::Object(map) => {
            for (id, spec) in map {
                match spec {
                    Value::String(ver) => deps.push(ModDependency {
                        id: id.clone(),
                        version: ver.clone(),
                        importance: DepImportance::Required,
                    }),
                    Value::Object(obj) => {
                        let version = obj
                            .get("version")
                            .and_then(|x| x.as_str())
                            .unwrap_or("*")
                            .to_string();
                        let importance = if obj.get("required").and_then(|x| x.as_bool()) == Some(false) {
                            DepImportance::Suggested
                        } else {
                            parse_importance(obj.get("importance"))
                        };
                        deps.push(ModDependency {
                            id: id.clone(),
                            version,
                            importance,
                        });
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    deps
}

fn parse_incompatibilities(val: Option<&Value>) -> Vec<ModIncompatibility> {
    let mut out = Vec::new();
    let Some(v) = val else {
        return out;
    };
    match v {
        Value::Array(arr) => {
            for item in arr {
                if let Some(obj) = item.as_object() {
                    if let Some(id) = obj.get("id").and_then(|x| x.as_str()) {
                        let version = obj
                            .get("version")
                            .and_then(|x| x.as_str())
                            .unwrap_or("*")
                            .to_string();
                        let importance = obj
                            .get("importance")
                            .and_then(|x| x.as_str())
                            .unwrap_or("breaking")
                            .to_string();
                        out.push(ModIncompatibility {
                            id: id.to_string(),
                            version,
                            importance,
                        });
                    }
                }
            }
        }
        Value::Object(map) => {
            for (id, spec) in map {
                match spec {
                    Value::String(ver) => out.push(ModIncompatibility {
                        id: id.clone(),
                        version: ver.clone(),
                        importance: "breaking".into(),
                    }),
                    Value::Object(obj) => {
                        let version = obj
                            .get("version")
                            .and_then(|x| x.as_str())
                            .unwrap_or("*")
                            .to_string();
                        let importance = obj
                            .get("importance")
                            .and_then(|x| x.as_str())
                            .unwrap_or("breaking")
                            .to_string();
                        out.push(ModIncompatibility {
                            id: id.clone(),
                            version,
                            importance,
                        });
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    out
}

pub fn read_geode_package_metadata(
    path: &Path,
    fallback_id: &str,
) -> Result<(ParsedModJson, Option<(u32, u32, Vec<u8>)>, String), String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let sha256 = format!("{:x}", hasher.finalize());

    let cursor = std::io::Cursor::new(&bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| format!("open .geode zip {}: {e}", path.display()))?;

    let mod_json_str = {
        let mut entry = archive
            .by_name("mod.json")
            .map_err(|e| format!("missing mod.json in {}: {e}", path.display()))?;
        let mut text = String::new();
        entry
            .read_to_string(&mut text)
            .map_err(|e| format!("read mod.json: {e}"))?;
        text
    };
    let parsed = parse_mod_json_str(&mod_json_str, fallback_id)?;

    let logo = if let Ok(mut logo_entry) = archive.by_name("logo.png") {
        let mut png_bytes = Vec::new();
        if logo_entry.read_to_end(&mut png_bytes).is_ok() {
            decode_logo_rgba(&png_bytes)
        } else {
            None
        }
    } else {
        None
    };

    Ok((parsed, logo, sha256))
}

fn decode_logo_rgba(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::load_from_memory(bytes).ok()?;
    let resized = img.thumbnail(128, 128).to_rgba8();
    let (w, h) = resized.dimensions();
    Some((w, h, resized.into_raw()))
}

pub fn scan_installed_mods(game_dir: &str) -> Vec<InstalledMod> {
    scan_installed_mods_with_data_dir(game_dir, &local_gd_data_dir())
}

pub fn scan_installed_mods_with_data_dir(game_dir: &str, local_data_dir: &Path) -> Vec<InstalledMod> {
    let mods_dir = Path::new(game_dir).join("geode").join("mods");
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(&mods_dir) else {
        return out;
    };

    let saved = load_loader_saved_json(local_data_dir);

    for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let fname = entry.file_name().to_string_lossy().into_owned();
        let (stem, file_disabled) = if let Some(s) = fname.strip_suffix(".geode.disabled") {
            (s.to_string(), true)
        } else if let Some(s) = fname.strip_suffix(".geode") {
            (s.to_string(), false)
        } else {
            continue;
        };

        let meta = entry.metadata().ok();
        let file_size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = meta
            .and_then(|m| m.modified().ok())
            .map(|t| {
                chrono::DateTime::<chrono::Local>::from(t)
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            })
            .unwrap_or_default();

        match read_geode_package_metadata(&path, &stem) {
            Ok((parsed, logo_rgba, sha256)) => {
                if !is_safe_mod_id(&parsed.id) {
                    // A mod ID that is not a plain identifier cannot be managed safely.
                    continue;
                }
                let load_key = format!("should-load-{}", parsed.id);
                let saved_enabled = saved
                    .get(&load_key)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let enabled = !file_disabled && saved_enabled;
                let pinned = saved
                    .get(&format!("is-pinned-{}", parsed.id))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let has_user_settings = mod_settings_path(local_data_dir, &parsed.id).is_file();

                out.push(InstalledMod {
                    id: parsed.id,
                    name: parsed.name,
                    version: parsed.version,
                    developer: parsed.developer,
                    description: parsed.description,
                    geode_target: parsed.geode,
                    gd_win_target: parsed.gd_win,
                    tags: parsed.tags,
                    early_load: parsed.early_load,
                    is_api: parsed.api,
                    enabled,
                    pinned,
                    file_disabled_ext: file_disabled,
                    path: path.to_string_lossy().into_owned(),
                    file_size,
                    sha256,
                    modified,
                    dependencies: parsed.dependencies,
                    incompatibilities: parsed.incompatibilities,
                    has_settings_schema: parsed.has_settings,
                    has_user_settings,
                    logo_rgba,
                });
            }
            Err(_) => {
                out.push(InstalledMod {
                    id: stem.clone(),
                    name: stem.clone(),
                    version: "corrupt".into(),
                    developer: "Unknown".into(),
                    description: "Failed to read mod.json inside .geode archive".into(),
                    geode_target: "*".into(),
                    gd_win_target: None,
                    tags: Vec::new(),
                    early_load: false,
                    is_api: false,
                    enabled: false,
                    pinned: false,
                    file_disabled_ext: file_disabled,
                    path: path.to_string_lossy().into_owned(),
                    file_size,
                    sha256: String::new(),
                    modified,
                    dependencies: Vec::new(),
                    incompatibilities: Vec::new(),
                    has_settings_schema: false,
                    has_user_settings: false,
                    logo_rgba: None,
                });
            }
        }
    }

    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Toggle a mod using Geode's native `saved.json` `"should-load-<id>"` flag, normalizing legacy `.geode.disabled` filenames cleanly.
pub fn set_mod_enabled(mod_item: &InstalledMod, enable: bool) -> Result<(), String> {
    set_mod_enabled_with_data_dir(mod_item, enable, &local_gd_data_dir())
}

pub fn set_mod_enabled_with_data_dir(
    mod_item: &InstalledMod,
    enable: bool,
    local_data_dir: &Path,
) -> Result<(), String> {
    let current_path = PathBuf::from(&mod_item.path);
    if !current_path.exists() {
        return Err(format!("Mod package no longer exists: {}", mod_item.path));
    }

    // If enabling a legacy `.geode.disabled` file, restore its `.geode` extension so Geode discovers it
    if enable && mod_item.file_disabled_ext {
        if let Some(fname) = current_path.file_name().and_then(|s| s.to_str()) {
            if let Some(stem) = fname.strip_suffix(".geode.disabled") {
                let target = current_path.with_file_name(format!("{stem}.geode"));
                fs::rename(&current_path, &target)
                    .map_err(|e| format!("rename mod package: {e}"))?;
            }
        }
    }

    let mut saved = load_loader_saved_json(local_data_dir);
    saved.insert(format!("should-load-{}", mod_item.id), Value::Bool(enable));
    save_loader_saved_json(local_data_dir, &saved)
}

pub fn set_mod_enabled_by_id(
    game_dir: &str,
    mod_id: &str,
    enable: bool,
    local_data_dir: &Path,
) -> Result<(), String> {
    let mods = scan_installed_mods_with_data_dir(game_dir, local_data_dir);
    if let Some(m) = mods.into_iter().find(|m| m.id == mod_id) {
        set_mod_enabled_with_data_dir(&m, enable, local_data_dir)
    } else {
        let mut saved = load_loader_saved_json(local_data_dir);
        saved.insert(format!("should-load-{mod_id}"), Value::Bool(enable));
        save_loader_saved_json(local_data_dir, &saved)
    }
}

pub fn remove_installed_mod(mod_item: &InstalledMod) -> Result<(), String> {
    let path = Path::new(&mod_item.path);
    if path.is_file() {
        fs::remove_file(path).map_err(|e| format!("delete {}: {e}", path.display()))?;
    }
    // Also clean up unzipped cache inside <game>/geode/unzipped/<id> if present
    if !is_safe_mod_id(&mod_item.id) {
        return Ok(());
    }
    if let Some(mods_dir) = path.parent() {
        if let Some(geode_dir) = mods_dir.parent() {
            let unzipped = geode_dir.join("unzipped").join(&mod_item.id);
            if unzipped.is_dir() {
                let _ = fs::remove_dir_all(&unzipped);
            }
        }
    }
    Ok(())
}

pub fn read_mod_settings_map(local_data_dir: &Path, mod_id: &str) -> BTreeMap<String, Value> {
    let path = mod_settings_path(local_data_dir, mod_id);
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<BTreeMap<String, Value>>(&s).ok())
        .unwrap_or_default()
}

pub fn write_mod_settings_map(
    local_data_dir: &Path,
    mod_id: &str,
    settings: &BTreeMap<String, Value>,
) -> Result<(), String> {
    let path = mod_settings_path(local_data_dir, mod_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create mod settings dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| format!("write settings.json: {e}"))
}
