use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::backup::crypto::{
    decrypt_vault_payload, default_vault_key_path, encrypt_vault_payload,
    load_or_create_vault_key, sha256_hex,
};
use crate::backup::save_inspector::validate_dat_file;
use crate::geode::package::local_gd_data_dir;
use crate::launcher::process::is_gd_running;

pub const SAVE_FILES: [&str; 4] = [
    "CCGameManager.dat",
    "CCLocalLevels.dat",
    "CCGameManager2.dat",
    "CCLocalLevels2.dat",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultFileRecord {
    pub logical_name: String,
    pub stored_name: String,
    pub plaintext_bytes: u64,
    pub plaintext_sha256: String,
    pub stored_sha256: String,
    pub encrypted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultManifest {
    pub schema_version: u32,
    pub id: String,
    pub created_at: String,
    pub trigger_reason: String,
    pub encrypted: bool,
    pub xml_verified: bool,
    pub includes_geode_configs: bool,
    pub aggregate_sha256: String,
    pub files: Vec<VaultFileRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub path: String,
    pub created_at: String,
    pub trigger_reason: String,
    pub encrypted: bool,
    pub xml_verified: bool,
    pub includes_geode_configs: bool,
    pub file_count: usize,
    pub total_bytes: u64,
    pub aggregate_sha256: String,
    pub integrity_status: String,
    pub source_kind: String,
}

pub fn live_save_fingerprint(save_dir: &Path) -> Option<String> {
    let mut combined = String::new();
    let mut found = false;
    for name in SAVE_FILES {
        let p = save_dir.join(name);
        if p.is_file() {
            if let Ok(bytes) = fs::read(&p) {
                combined.push_str(name);
                combined.push(':');
                combined.push_str(&sha256_hex(&bytes));
                combined.push(';');
                found = true;
            }
        }
    }
    if found {
        Some(sha256_hex(combined.as_bytes()))
    } else {
        None
    }
}

pub fn create_snapshot(
    backup_dir: &str,
    trigger_reason: &str,
    encrypt: bool,
    verify_xml: bool,
    include_geode_configs: bool,
    retention: usize,
) -> Result<SnapshotSummary, String> {
    create_snapshot_with_paths(
        &local_gd_data_dir(),
        Path::new(backup_dir),
        &default_vault_key_path(),
        trigger_reason,
        encrypt,
        verify_xml,
        include_geode_configs,
        retention,
    )
}

pub fn create_snapshot_with_paths(
    save_dir: &Path,
    backup_dir: &Path,
    vault_key_path: &Path,
    trigger_reason: &str,
    encrypt: bool,
    verify_xml: bool,
    include_geode_configs: bool,
    retention: usize,
) -> Result<SnapshotSummary, String> {
    if !save_dir.is_dir() {
        return Err(format!("Geometry Dash save directory not found: {}", save_dir.display()));
    }

    if verify_xml {
        for primary in ["CCGameManager.dat", "CCLocalLevels.dat"] {
            let p = save_dir.join(primary);
            if p.is_file() {
                validate_dat_file(&p).map_err(|e| {
                    format!("Save integrity check blocked backup ({primary} is corrupt): {e}")
                })?;
            }
        }
    }

    let key = if encrypt {
        Some(load_or_create_vault_key(vault_key_path)?)
    } else {
        None
    };

    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let prefix = if encrypt { "vault" } else { "save" };
    let mut id = format!("{prefix}_{stamp}");
    let mut dest = backup_dir.join(&id);
    if dest.exists() {
        let millis = chrono::Local::now().format("%3f").to_string();
        id = format!("{prefix}_{stamp}_{millis}");
        dest = backup_dir.join(&id);
    }
    fs::create_dir_all(&dest).map_err(|e| format!("create snapshot dir: {e}"))?;

    let mut records = Vec::new();
    let mut total_bytes = 0u64;
    let mut agg_input = String::new();

    for name in SAVE_FILES {
        let src = save_dir.join(name);
        if !src.is_file() {
            continue;
        }
        let raw = fs::read(&src).map_err(|e| format!("read {name}: {e}"))?;
        let plain_sha = sha256_hex(&raw);
        let plain_len = raw.len() as u64;

        let (stored_name, out_bytes) = if let Some(ref k) = key {
            (format!("{name}.gdvault"), encrypt_vault_payload(k, &raw)?)
        } else {
            (name.to_string(), raw)
        };
        let stored_sha = sha256_hex(&out_bytes);
        fs::write(dest.join(&stored_name), &out_bytes)
            .map_err(|e| format!("write {stored_name}: {e}"))?;

        total_bytes += out_bytes.len() as u64;
        agg_input.push_str(&format!("{name}:{plain_sha};"));
        records.push(VaultFileRecord {
            logical_name: name.to_string(),
            stored_name,
            plaintext_bytes: plain_len,
            plaintext_sha256: plain_sha,
            stored_sha256: stored_sha,
            encrypted: encrypt,
        });
    }

    if records.is_empty() {
        let _ = fs::remove_dir_all(&dest);
        return Err("No Geometry Dash .dat save files found to back up.".into());
    }

    let mut bundled_configs = false;
    if include_geode_configs {
        let geode_mods = save_dir.join("geode").join("mods");
        if geode_mods.is_dir() {
            if let Ok(bundle) = bundle_geode_configs_zip(&geode_mods) {
                if !bundle.is_empty() {
                    let (stored_name, out_bytes) = if let Some(ref k) = key {
                        (
                            "geode_configs.zip.gdvault".to_string(),
                            encrypt_vault_payload(k, &bundle)?,
                        )
                    } else {
                        ("geode_configs.zip".to_string(), bundle.clone())
                    };
                    let plain_sha = sha256_hex(&bundle);
                    let stored_sha = sha256_hex(&out_bytes);
                    fs::write(dest.join(&stored_name), &out_bytes)
                        .map_err(|e| format!("write geode configs bundle: {e}"))?;
                    total_bytes += out_bytes.len() as u64;
                    records.push(VaultFileRecord {
                        logical_name: "geode_configs.zip".into(),
                        stored_name,
                        plaintext_bytes: bundle.len() as u64,
                        plaintext_sha256: plain_sha,
                        stored_sha256: stored_sha,
                        encrypted: encrypt,
                    });
                    bundled_configs = true;
                }
            }
        }
    }

    let created_human = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let aggregate_sha256 = sha256_hex(agg_input.as_bytes());
    let manifest = VaultManifest {
        schema_version: 2,
        id: id.clone(),
        created_at: created_human.clone(),
        trigger_reason: trigger_reason.to_string(),
        encrypted: encrypt,
        xml_verified: verify_xml,
        includes_geode_configs: bundled_configs,
        aggregate_sha256: aggregate_sha256.clone(),
        files: records,
    };
    let m_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(dest.join("manifest.json"), m_json).map_err(|e| format!("write manifest: {e}"))?;

    if retention > 0 {
        enforce_retention(backup_dir, retention);
    }

    Ok(SnapshotSummary {
        id,
        path: dest.to_string_lossy().into_owned(),
        created_at: created_human,
        trigger_reason: trigger_reason.to_string(),
        encrypted: encrypt,
        xml_verified: verify_xml,
        includes_geode_configs: bundled_configs,
        file_count: manifest.files.len(),
        total_bytes,
        aggregate_sha256,
        integrity_status: "Verified".into(),
        source_kind: if encrypt {
            "GeodeDeck Encrypted Vault".into()
        } else {
            "GeodeDeck Snapshot".into()
        },
    })
}

fn bundle_geode_configs_zip(geode_mods_dir: &Path) -> Result<Vec<u8>, String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(&mut buf);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut count = 0usize;

    let loader_saved = geode_mods_dir.join("geode.loader").join("saved.json");
    if loader_saved.is_file() {
        if let Ok(bytes) = fs::read(&loader_saved) {
            let _ = writer.start_file("geode.loader/saved.json", opts);
            let _ = writer.write_all(&bytes);
            count += 1;
        }
    }

    if let Ok(rd) = fs::read_dir(geode_mods_dir) {
        for entry in rd.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let mod_id = entry.file_name().to_string_lossy().into_owned();
            let settings = dir.join("settings.json");
            if settings.is_file() {
                if let Ok(bytes) = fs::read(&settings) {
                    let _ = writer.start_file(format!("{mod_id}/settings.json"), opts);
                    let _ = writer.write_all(&bytes);
                    count += 1;
                }
            }
        }
    }

    writer.finish().map_err(|e| e.to_string())?;
    if count == 0 {
        return Ok(Vec::new());
    }
    Ok(buf.into_inner())
}

fn enforce_retention(backup_dir: &Path, retention: usize) {
    let mut items = list_snapshots(backup_dir);
    items.sort_by(|a, b| b.id.cmp(&a.id));
    for old in items.into_iter().skip(retention) {
        // Never auto-prune explicit Pre-Restore or Manual safety snapshots first if avoidable; skip by age
        if old.trigger_reason.contains("Pre-Restore") {
            continue;
        }
        let _ = fs::remove_dir_all(&old.path);
    }
}

pub fn list_snapshots(backup_dir: &Path) -> Vec<SnapshotSummary> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(backup_dir) else {
        return out;
    };
    for entry in rd.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let folder = entry.file_name().to_string_lossy().into_owned();
        if !folder.starts_with("vault_")
            && !folder.starts_with("save_")
            && !folder.starts_with("imported_")
        {
            continue;
        }

        let manifest_path = p.join("manifest.json");
        if let Ok(raw) = fs::read_to_string(&manifest_path) {
            if let Ok(mf) = serde_json::from_str::<VaultManifest>(&raw) {
                let total_bytes: u64 = mf
                    .files
                    .iter()
                    .filter_map(|f| fs::metadata(p.join(&f.stored_name)).ok().map(|m| m.len()))
                    .sum();
                out.push(SnapshotSummary {
                    id: mf.id,
                    path: p.to_string_lossy().into_owned(),
                    created_at: mf.created_at,
                    trigger_reason: mf.trigger_reason,
                    encrypted: mf.encrypted,
                    xml_verified: mf.xml_verified,
                    includes_geode_configs: mf.includes_geode_configs,
                    file_count: mf.files.len(),
                    total_bytes,
                    aggregate_sha256: mf.aggregate_sha256,
                    integrity_status: "Ready".into(),
                    source_kind: if mf.encrypted {
                        "AES-256-GCM Vault".into()
                    } else {
                        "Snapshot".into()
                    },
                });
                continue;
            }
        }

        // Legacy v0.1.0 `save_*` snapshot support
        let mut count = 0usize;
        let mut bytes = 0u64;
        for f in SAVE_FILES {
            let fp = p.join(f);
            if fp.is_file() {
                count += 1;
                bytes += fs::metadata(&fp).map(|m| m.len()).unwrap_or(0);
            }
        }
        if count > 0 {
            out.push(SnapshotSummary {
                id: folder.clone(),
                path: p.to_string_lossy().into_owned(),
                created_at: folder
                    .trim_start_matches("save_")
                    .replace('_', " "),
                trigger_reason: "Legacy Snapshot".into(),
                encrypted: false,
                xml_verified: false,
                includes_geode_configs: false,
                file_count: count,
                total_bytes: bytes,
                aggregate_sha256: String::new(),
                integrity_status: "Legacy".into(),
                source_kind: "Legacy v0.1.0".into(),
            });
        }
    }
    out.sort_by(|a, b| b.id.cmp(&a.id));
    out
}

pub fn verify_snapshot_integrity(
    snapshot_dir: &Path,
    vault_key_path: &Path,
) -> Result<String, String> {
    let manifest_path = snapshot_dir.join("manifest.json");
    if manifest_path.is_file() {
        let raw_mf = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("read manifest: {e}"))?;
        if let Ok(mf) = serde_json::from_str::<VaultManifest>(&raw_mf) {
            let key = if mf.encrypted {
                Some(load_or_create_vault_key(vault_key_path)?)
            } else {
                None
            };
            for rec in &mf.files {
                let fp = snapshot_dir.join(&rec.stored_name);
                if !fp.is_file() {
                    return Err(format!("Missing file '{}' in snapshot", rec.stored_name));
                }
                let stored_bytes = fs::read(&fp).map_err(|e| format!("read {}: {e}", rec.stored_name))?;
                let stored_sha = sha256_hex(&stored_bytes);
                if stored_sha != rec.stored_sha256 {
                    return Err(format!("Stored SHA-256 mismatch for {}", rec.stored_name));
                }
                let plain = if let Some(ref k) = key {
                    decrypt_vault_payload(k, &stored_bytes)?
                } else {
                    stored_bytes
                };
                if sha256_hex(&plain) != rec.plaintext_sha256 {
                    return Err(format!("Plaintext SHA-256 mismatch for {}", rec.logical_name));
                }
                if rec.logical_name == "CCGameManager.dat" || rec.logical_name == "CCLocalLevels.dat" {
                    crate::backup::save_inspector::decode_gd_dat_to_xml(&plain).map_err(|e| {
                        format!("XML integrity failed for {}: {e}", rec.logical_name)
                    })?;
                }
            }
            return Ok(format!(
                "Verified {} file(s) ({})",
                mf.files.len(),
                if mf.encrypted { "AES-256-GCM + SHA-256 + XML" } else { "SHA-256 + XML" }
            ));
        }
    }

    // Legacy fallback verification
    let mut verified = 0usize;
    for name in SAVE_FILES {
        let fp = snapshot_dir.join(name);
        if fp.is_file() {
            if name == "CCGameManager.dat" || name == "CCLocalLevels.dat" {
                validate_dat_file(&fp)?;
            }
            verified += 1;
        }
    }
    if verified == 0 {
        return Err("No valid save files in backup folder".into());
    }
    Ok(format!("Verified {verified} legacy save file(s) via XML decode"))
}

pub fn restore_snapshot(
    snapshot_dir: &str,
    backup_dir: &str,
    restore_geode_configs: bool,
) -> Result<String, String> {
    if is_gd_running() {
        return Err("Close Geometry Dash before restoring save files.".into());
    }
    restore_snapshot_with_paths(
        Path::new(snapshot_dir),
        &local_gd_data_dir(),
        Path::new(backup_dir),
        &default_vault_key_path(),
        restore_geode_configs,
    )
}

pub fn restore_snapshot_with_paths(
    snapshot_dir: &Path,
    save_dir: &Path,
    backup_dir: &Path,
    vault_key_path: &Path,
    restore_geode_configs: bool,
) -> Result<String, String> {
    // 1. Verify entire snapshot integrity BEFORE mutating live save files
    verify_snapshot_integrity(snapshot_dir, vault_key_path)?;

    // 2. Create automatic pre-restore safety snapshot of current live state if present
    if save_dir.join("CCGameManager.dat").is_file() || save_dir.join("CCLocalLevels.dat").is_file() {
        let _ = create_snapshot_with_paths(
            save_dir,
            backup_dir,
            vault_key_path,
            "Pre-Restore Safety Point",
            true,
            false,
            true,
            0,
        );
    }

    fs::create_dir_all(save_dir).map_err(|e| format!("create save dir: {e}"))?;

    let mut decoded_files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let manifest_path = snapshot_dir.join("manifest.json");
    if let Ok(raw_mf) = fs::read_to_string(&manifest_path) {
        if let Ok(mf) = serde_json::from_str::<VaultManifest>(&raw_mf) {
            let key = if mf.encrypted {
                Some(load_or_create_vault_key(vault_key_path)?)
            } else {
                None
            };
            for rec in &mf.files {
                let stored = fs::read(snapshot_dir.join(&rec.stored_name))
                    .map_err(|e| format!("read {}: {e}", rec.stored_name))?;
                let plain = if let Some(ref k) = key {
                    decrypt_vault_payload(k, &stored)?
                } else {
                    stored
                };
                decoded_files.insert(rec.logical_name.clone(), plain);
            }
        }
    }

    if decoded_files.is_empty() {
        for name in SAVE_FILES {
            let fp = snapshot_dir.join(name);
            if fp.is_file() {
                decoded_files.insert(name.to_string(), fs::read(&fp).map_err(|e| e.to_string())?);
            }
        }
    }

    let mut restored_count = 0usize;
    for name in SAVE_FILES {
        if let Some(bytes) = decoded_files.get(name) {
            let tmp = save_dir.join(format!("{name}.restore_tmp"));
            let dst = save_dir.join(name);
            fs::write(&tmp, bytes).map_err(|e| format!("write tmp {name}: {e}"))?;
            fs::rename(&tmp, &dst).map_err(|e| format!("commit {name}: {e}"))?;
            restored_count += 1;
        }
    }

    let mut restored_configs = 0usize;
    if restore_geode_configs {
        if let Some(zip_bytes) = decoded_files.get("geode_configs.zip") {
            let geode_mods = save_dir.join("geode").join("mods");
            if let Ok(mut zip) = ZipArchive::new(std::io::Cursor::new(zip_bytes)) {
                for i in 0..zip.len() {
                    if let Ok(mut file) = zip.by_index(i) {
                        if let Some(rel) = file.enclosed_name().map(|p| p.to_path_buf()) {
                            let target = geode_mods.join(rel);
                            if let Some(parent) = target.parent() {
                                let _ = fs::create_dir_all(parent);
                            }
                            let mut buf = Vec::new();
                            if file.read_to_end(&mut buf).is_ok() && fs::write(&target, &buf).is_ok()
                            {
                                restored_configs += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(format!(
        "Restored {restored_count} GD save file(s) and {restored_configs} Geode config(s) after integrity verification."
    ))
}

/// Discovers existing backups created by legacy GeodeSync or GDLauncher in `%LOCALAPPDATA%\GeometryDash`.
pub fn detect_legacy_external_backups() -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    let local = local_gd_data_dir();

    let geodesync_dir = local.join("GeodeSync_Save_Backups");
    if let Ok(rd) = fs::read_dir(&geodesync_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && (p.join("CCGameManager.dat").is_file() || p.join("CCLocalLevels.dat").is_file()) {
                found.push((
                    format!("GeodeSync: {}", e.file_name().to_string_lossy()),
                    p,
                ));
            }
        }
    }

    let gdl_dir = local.join("GDLauncher_Backups");
    if gdl_dir.is_dir() && gdl_dir.join("CCGameManager.dat.1794271158.backup").is_file() {
        // Check generic backup presence
        found.push(("GDLauncher Backups Folder".into(), gdl_dir));
    }

    found
}

pub fn import_backup_folder_into_vault(
    src_dir: &Path,
    backup_dir: &Path,
    label: &str,
) -> Result<SnapshotSummary, String> {
    create_snapshot_with_paths(
        src_dir,
        backup_dir,
        &default_vault_key_path(),
        &format!("Imported ({label})"),
        true,
        true,
        false,
        0,
    )
}

pub fn export_snapshot_portable_zip(
    snapshot_dir: &Path,
    dest_zip: &Path,
    vault_key_path: &Path,
) -> Result<(), String> {
    verify_snapshot_integrity(snapshot_dir, vault_key_path)?;
    let file = File::create(dest_zip).map_err(|e| format!("create zip: {e}"))?;
    let mut writer = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let mf_path = snapshot_dir.join("manifest.json");
    if let Ok(raw_mf) = fs::read_to_string(&mf_path) {
        if let Ok(mf) = serde_json::from_str::<VaultManifest>(&raw_mf) {
            let key = if mf.encrypted {
                Some(load_or_create_vault_key(vault_key_path)?)
            } else {
                None
            };
            for rec in &mf.files {
                let stored = fs::read(snapshot_dir.join(&rec.stored_name))
                    .map_err(|e| e.to_string())?;
                let plain = if let Some(ref k) = key {
                    decrypt_vault_payload(k, &stored)?
                } else {
                    stored
                };
                writer
                    .start_file(&rec.logical_name, opts)
                    .map_err(|e| e.to_string())?;
                writer.write_all(&plain).map_err(|e| e.to_string())?;
            }
            writer.finish().map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    for name in SAVE_FILES {
        let fp = snapshot_dir.join(name);
        if fp.is_file() {
            let bytes = fs::read(&fp).map_err(|e| e.to_string())?;
            writer.start_file(name, opts).map_err(|e| e.to_string())?;
            writer.write_all(&bytes).map_err(|e| e.to_string())?;
        }
    }
    writer.finish().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn import_portable_zip_into_vault(
    zip_path: &Path,
    backup_dir: &Path,
    vault_key_path: &Path,
) -> Result<SnapshotSummary, String> {
    let file = File::open(zip_path).map_err(|e| format!("open zip: {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("invalid zip: {e}"))?;
    let tmp = std::env::temp_dir().join(format!(
        "geodedeck_import_{}",
        chrono::Local::now().format("%Y%m%d%H%M%S")
    ));
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;

    for name in SAVE_FILES {
        if let Ok(mut entry) = archive.by_name(name) {
            let mut buf = Vec::new();
            if entry.read_to_end(&mut buf).is_ok() {
                let _ = fs::write(tmp.join(name), &buf);
            }
        }
    }

    let res = create_snapshot_with_paths(
        &tmp,
        backup_dir,
        vault_key_path,
        "Imported Portable Backup (.zip)",
        true,
        true,
        false,
        0,
    );
    let _ = fs::remove_dir_all(&tmp);
    res
}
