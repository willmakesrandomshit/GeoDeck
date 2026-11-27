//! Regression tests for untrusted-input handling: mod IDs and downloads.

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use geodedeck::core::updater::download_and_verify_update;
use geodedeck::geode::exporter::import_gdpack_with_data_dir;
use geodedeck::geode::index_api::download_verified_bytes;
use geodedeck::geode::package::{
    is_safe_mod_id, mod_settings_path, scan_installed_mods_with_data_dir,
};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "geodedeck_sec_{tag}_{}_{}",
        std::process::id(),
        chrono::Local::now().format("%H%M%S%3f")
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    let mut w = ZipWriter::new(File::create(path).unwrap());
    for (name, data) in files {
        w.start_file(*name, SimpleFileOptions::default()).unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap();
}

#[test]
fn safe_mod_id_accepts_geode_ids_and_rejects_paths() {
    for ok in ["geode.loader", "author.core-api", "dev.my_mod", "a.b", "x1.y2-z3"] {
        assert!(is_safe_mod_id(ok), "{ok} should be accepted");
    }
    for bad in [
        "",
        ".",
        "..",
        "../escape",
        "..\\escape",
        "a/b",
        "a\\b",
        "/abs",
        "C:\\win",
        ".hidden",
        "a..b",
        "has space",
        "nul\0byte",
        "emoji\u{1F600}",
    ] {
        assert!(!is_safe_mod_id(bad), "{bad:?} should be rejected");
    }
    assert!(!is_safe_mod_id(&"a".repeat(129)));
}

#[test]
fn mod_settings_path_never_escapes_the_mods_folder() {
    let data = Path::new("/data/local");
    for id in ["../../etc", "a/b", "..", ""] {
        let p = mod_settings_path(data, id);
        assert!(p.starts_with(data.join("geode").join("mods")), "{id:?} escaped: {p:?}");
        assert!(!p.components().any(|c| c.as_os_str() == ".."), "{id:?} left a '..': {p:?}");
    }
    assert_eq!(
        mod_settings_path(data, "author.mod"),
        data.join("geode/mods/author.mod/settings.json")
    );
}

#[test]
fn scan_skips_mods_whose_mod_json_has_a_path_like_id() {
    let root = temp_dir("scan");
    let game = root.join("GD");
    let mods = game.join("geode/mods");
    let good = br#"{"geode":"5.0.0","gd":"2.2081","id":"author.good","name":"Good","version":"1.0.0","developer":"A"}"#;
    let evil = br#"{"geode":"5.0.0","gd":"2.2081","id":"../../../..","name":"Evil","version":"1.0.0","developer":"A"}"#;
    write_zip(&mods.join("author.good.geode"), &[("mod.json", good)]);
    write_zip(&mods.join("evil.geode"), &[("mod.json", evil)]);

    let found = scan_installed_mods_with_data_dir(&game.to_string_lossy(), &root.join("Local"));
    let ids: Vec<_> = found.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, vec!["author.good"], "unsafe id must be skipped, not managed");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn gdpack_import_rejects_path_traversal_ids_and_writes_nothing() {
    let root = temp_dir("gdpack");
    let game = root.join("a/b/GD");
    fs::create_dir_all(game.join("geode/mods")).unwrap();
    let payload = b"not really a mod";
    let sha = format!("{:x}", Sha256::digest(payload));
    let manifest = serde_json::json!({
        "format_version": 1,
        "pack_name": "evil",
        "created_at": "2026-01-01",
        "target_gd_version": "2.2081",
        "target_geode_version": "5.0.0",
        "mods": [{
            "id": "../../../escaped",
            "name": "Evil",
            "version": "1.0.0",
            "developer": "x",
            "archive_path": "mods/evil.geode",
            "sha256": sha,
            "enabled": true,
            "has_settings": false
        }]
    })
    .to_string();
    let pack = root.join("evil.gdpack.zip");
    write_zip(
        &pack,
        &[("geodedeck-pack.json", manifest.as_bytes()), ("mods/evil.geode", payload)],
    );

    let err = import_gdpack_with_data_dir(&pack, &game.to_string_lossy(), true, &root.join("Local"))
        .expect_err("a pack with a path-like mod id must be refused");
    assert!(err.contains("unsafe mod id"), "unexpected error: {err}");

    // Nothing may have been written anywhere under the temp root except our own files.
    let escaped: Vec<_> = walk(&root)
        .into_iter()
        .filter(|p| p.to_string_lossy().contains("escaped"))
        .collect();
    assert!(escaped.is_empty(), "files were written: {escaped:?}");
    assert!(fs::read_dir(game.join("geode/mods")).unwrap().next().is_none());
    let _ = fs::remove_dir_all(&root);
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            out.push(p.clone());
            if p.is_dir() {
                out.extend(walk(&p));
            }
        }
    }
    out
}

#[test]
fn downloads_fail_closed_without_a_checksum() {
    // Both must refuse before any network access happens.
    let err = download_verified_bytes("http://127.0.0.1:9/never-contacted", "  ").unwrap_err();
    assert!(err.contains("No SHA-256"), "unexpected error: {err}");

    let err = download_and_verify_update("http://127.0.0.1:9/never-contacted", "GeoDeck.exe", None)
        .unwrap_err();
    assert!(err.contains("no SHA-256"), "unexpected error: {err}");
    let err = download_and_verify_update("http://127.0.0.1:9/never-contacted", "GeoDeck.exe", Some(" "))
        .unwrap_err();
    assert!(err.contains("no SHA-256"), "unexpected error: {err}");
}

#[test]
fn vault_snapshot_manifest_cannot_point_outside_its_folder() {
    use geodedeck::backup::vault::{
        is_plain_file_name, restore_snapshot_with_paths, verify_snapshot_integrity,
    };

    for ok in ["CCGameManager.dat.gdvault", "CCLocalLevels.dat", "a.b"] {
        assert!(is_plain_file_name(ok), "{ok} should be accepted");
    }
    for bad in ["", ".", "..", "../x", "a/b", "a\\b", "nul\0"] {
        assert!(!is_plain_file_name(bad), "{bad:?} should be rejected");
    }

    let root = temp_dir("vault");
    let snap = root.join("vault_evil");
    let save = root.join("Save");
    fs::create_dir_all(&snap).unwrap();
    fs::create_dir_all(&save).unwrap();
    // A file outside the snapshot folder that the manifest tries to pull in.
    let secret = b"top secret contents";
    fs::write(root.join("secret.txt"), secret).unwrap();
    let sha = format!("{:x}", Sha256::digest(secret));
    let manifest = serde_json::json!({
        "schema_version": 1, "id": "evil", "created_at": "2026-01-01",
        "trigger_reason": "x", "encrypted": false, "xml_verified": false,
        "includes_geode_configs": false, "aggregate_sha256": "x",
        "files": [{
            "logical_name": "CCGameManager.dat",
            "stored_name": "../secret.txt",
            "plaintext_bytes": secret.len(),
            "plaintext_sha256": sha, "stored_sha256": sha, "encrypted": false
        }]
    });
    fs::write(snap.join("manifest.json"), manifest.to_string()).unwrap();

    let key = root.join("vault.key");
    let err = verify_snapshot_integrity(&snap, &key).unwrap_err();
    assert!(err.contains("unsafe file name"), "unexpected error: {err}");

    let err = restore_snapshot_with_paths(&snap, &save, &root.join("Vault"), &key, false).unwrap_err();
    assert!(err.contains("unsafe file name"), "unexpected error: {err}");
    assert!(!save.join("CCGameManager.dat").exists(), "outside file leaked into the save folder");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn pe_header_parsing_rejects_hostile_offsets_without_panicking() {
    use geodedeck::launcher::detection::parse_pe_timestamp_bytes;

    // e_lfanew near u32::MAX must be a clean error (and must not overflow on 32-bit targets).
    let mut buf = vec![0u8; 0x80];
    buf[0] = b'M';
    buf[1] = b'Z';
    buf[0x3C..0x40].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(parse_pe_timestamp_bytes(&buf).is_err());

    // Truncated and empty inputs.
    assert!(parse_pe_timestamp_bytes(&[]).is_err());
    assert!(parse_pe_timestamp_bytes(&buf[..0x3F]).is_err());

    // Offset pointing just past the end.
    buf[0x3C..0x40].copy_from_slice(&0x70u32.to_le_bytes());
    assert!(parse_pe_timestamp_bytes(&buf).is_err());
}

#[cfg(unix)]
#[test]
fn vault_key_file_is_created_owner_only() {
    use geodedeck::backup::crypto::load_or_create_vault_key;
    use std::os::unix::fs::PermissionsExt;

    let root = temp_dir("vaultkey");
    let key_path = root.join("nested/vault.key.dpapi");
    let key = load_or_create_vault_key(&key_path).unwrap();
    assert_eq!(key.len(), 32);
    let mode = fs::metadata(&key_path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "vault key must be owner read/write only, got {mode:o}");
    // Loading again returns the same key.
    assert_eq!(load_or_create_vault_key(&key_path).unwrap(), key);
    let _ = fs::remove_dir_all(&root);
}
