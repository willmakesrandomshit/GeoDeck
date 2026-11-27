use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::geode::package::{
    local_gd_data_dir, mod_settings_path, scan_installed_mods_with_data_dir,
    set_mod_enabled_by_id,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackModEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub developer: String,
    pub archive_path: String,
    pub sha256: String,
    pub enabled: bool,
    pub has_settings: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModPackManifest {
    pub format_version: u32,
    pub pack_name: String,
    pub created_at: String,
    pub target_gd_version: String,
    pub target_geode_version: String,
    pub mods: Vec<PackModEntry>,
}

#[derive(Clone, Debug)]
pub struct ImportPackResult {
    pub pack_name: String,
    pub imported_mods: usize,
    pub restored_configs: usize,
    pub target_gd_version: String,
}

pub fn export_gdpack(
    pack_name: &str,
    game_dir: &str,
    dest_dir: &str,
    selected_indices: &[usize],
    include_configs: bool,
    target_gd_version: &str,
    target_geode_version: &str,
) -> Result<PathBuf, String> {
    export_gdpack_with_data_dir(
        pack_name,
        game_dir,
        dest_dir,
        selected_indices,
        include_configs,
        target_gd_version,
        target_geode_version,
        &local_gd_data_dir(),
    )
}

pub fn export_gdpack_with_data_dir(
    pack_name: &str,
    game_dir: &str,
    dest_dir: &str,
    selected_indices: &[usize],
    include_configs: bool,
    target_gd_version: &str,
    target_geode_version: &str,
    local_data_dir: &Path,
) -> Result<PathBuf, String> {
    let mods = scan_installed_mods_with_data_dir(game_dir, local_data_dir);
    if mods.is_empty() {
        return Err("No installed Geode mods found to export.".into());
    }

    fs::create_dir_all(dest_dir).map_err(|e| format!("create export dir: {e}"))?;
    let safe_label: String = pack_name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let label = if safe_label.is_empty() {
        "GeodeMods".to_string()
    } else {
        safe_label
    };
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let out_path = Path::new(dest_dir).join(format!("{label}_{stamp}.gdpack.zip"));

    let file = File::create(&out_path).map_err(|e| format!("create export archive: {e}"))?;
    let mut writer = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let mut entries = Vec::new();
    for &idx in selected_indices {
        let Some(m) = mods.get(idx) else { continue };
        let raw = fs::read(&m.path).map_err(|e| format!("read {}: {e}", m.id))?;
        let mut hasher = Sha256::new();
        hasher.update(&raw);
        let sha256 = format!("{:x}", hasher.finalize());

        let archive_path = format!("mods/{}.geode", m.id);
        writer
            .start_file(&archive_path, opts)
            .map_err(|e| format!("zip start {}: {e}", archive_path))?;
        writer
            .write_all(&raw)
            .map_err(|e| format!("zip write {}: {e}", archive_path))?;

        let mut has_settings = false;
        if include_configs {
            let settings_src = mod_settings_path(local_data_dir, &m.id);
            if settings_src.is_file() {
                if let Ok(cfg_bytes) = fs::read(&settings_src) {
                    let cfg_rel = format!("configs/{}/settings.json", m.id);
                    writer
                        .start_file(&cfg_rel, opts)
                        .map_err(|e| format!("zip start {cfg_rel}: {e}"))?;
                    writer
                        .write_all(&cfg_bytes)
                        .map_err(|e| format!("zip write {cfg_rel}: {e}"))?;
                    has_settings = true;
                }
            }
        }

        entries.push(PackModEntry {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version.clone(),
            developer: m.developer.clone(),
            archive_path,
            sha256,
            enabled: m.enabled,
            has_settings,
        });
    }

    if entries.is_empty() {
        return Err("No mods selected for export.".into());
    }

    let manifest = ModPackManifest {
        format_version: 2,
        pack_name: pack_name.trim().to_string(),
        created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        target_gd_version: target_gd_version.to_string(),
        target_geode_version: target_geode_version.to_string(),
        mods: entries,
    };

    let manifest_json =
        serde_json::to_string_pretty(&manifest).map_err(|e| format!("serialize manifest: {e}"))?;
    writer
        .start_file("geodedeck-pack.json", opts)
        .map_err(|e| format!("zip manifest: {e}"))?;
    writer
        .write_all(manifest_json.as_bytes())
        .map_err(|e| format!("write manifest: {e}"))?;

    writer.finish().map_err(|e| format!("finish zip: {e}"))?;
    Ok(out_path)
}

pub fn import_gdpack(
    archive_path: &Path,
    game_dir: &str,
    restore_configs: bool,
) -> Result<ImportPackResult, String> {
    import_gdpack_with_data_dir(archive_path, game_dir, restore_configs, &local_gd_data_dir())
}

pub fn import_gdpack_with_data_dir(
    archive_path: &Path,
    game_dir: &str,
    restore_configs: bool,
    local_data_dir: &Path,
) -> Result<ImportPackResult, String> {
    let file = File::open(archive_path).map_err(|e| format!("open pack: {e}"))?;
    let mut zip = ZipArchive::new(file).map_err(|e| format!("invalid zip archive: {e}"))?;

    let manifest: ModPackManifest = {
        let mut mf = zip
            .by_name("geodedeck-pack.json")
            .map_err(|_| "Archive is missing geodedeck-pack.json manifest".to_string())?;
        let mut s = String::new();
        mf.read_to_string(&mut s)
            .map_err(|e| format!("read manifest: {e}"))?;
        serde_json::from_str(&s).map_err(|e| format!("parse manifest: {e}"))?
    };

    let mods_dir = Path::new(game_dir).join("geode").join("mods");
    fs::create_dir_all(&mods_dir).map_err(|e| format!("create mods dir: {e}"))?;

    let mut imported_mods = 0usize;
    let mut restored_configs = 0usize;

    for entry in &manifest.mods {
        let bytes = {
            let mut mod_file = zip
                .by_name(&entry.archive_path)
                .map_err(|_| format!("Pack missing mod payload '{}'", entry.archive_path))?;
            let mut bytes = Vec::new();
            mod_file
                .read_to_end(&mut bytes)
                .map_err(|e| format!("read {}: {e}", entry.id))?;
            bytes
        };

        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = format!("{:x}", hasher.finalize());
        if !actual.eq_ignore_ascii_case(&entry.sha256) {
            return Err(format!(
                "Checksum mismatch for '{}': expected {}, got {}",
                entry.id, entry.sha256, actual
            ));
        }

        let dst = mods_dir.join(format!("{}.geode", entry.id));
        fs::write(&dst, &bytes).map_err(|e| format!("write {}: {e}", entry.id))?;
        let _ = set_mod_enabled_by_id(game_dir, &entry.id, entry.enabled, local_data_dir);
        imported_mods += 1;

        if restore_configs && entry.has_settings {
            let cfg_zip_path = format!("configs/{}/settings.json", entry.id);
            if let Ok(mut cfg_file) = zip.by_name(&cfg_zip_path) {
                let mut cfg_bytes = Vec::new();
                if cfg_file.read_to_end(&mut cfg_bytes).is_ok() {
                    let dst_cfg = mod_settings_path(local_data_dir, &entry.id);
                    if let Some(parent) = dst_cfg.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if fs::write(&dst_cfg, &cfg_bytes).is_ok() {
                        restored_configs += 1;
                    }
                }
            }
        }
    }

    Ok(ImportPackResult {
        pack_name: manifest.pack_name,
        imported_mods,
        restored_configs,
        target_gd_version: manifest.target_gd_version,
    })
}
