use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use geodedeck::backup::crypto::{decrypt_vault_payload, encrypt_vault_payload, load_or_create_vault_key};
use geodedeck::backup::save_inspector::{
    decode_gd_dat_to_xml, export_level_to_gmd, inspect_save_dir, validate_dat_file,
};
use geodedeck::tools::songs::{
    clear_manual_song_metadata_at_path, filter_and_sort_songs, load_metadata_cache_at_path,
    merge_save_metadata_into_cache, parse_boomlings_song_info, parse_gd_music_library,
    parse_gd_sfx_library, parse_id3v1_bytes, parse_id3v2_bytes, parse_jukebox_manifest_json,
    parse_ogg_vorbis_comment_bytes, scan_song_cache_with_paths, set_manual_song_metadata_at_path,
    MetadataSource, SongCacheStatus, SongFormatFilter, SongSortMode, SongStatusFilter,
};
use geodedeck::backup::vault::{
    create_snapshot_with_paths, list_snapshots, restore_snapshot_with_paths,
    verify_snapshot_integrity,
};
use geodedeck::core::config::{Config, OptimizationPreset};
use geodedeck::core::updater::{normalize_repo_slug, parse_release_version};
use geodedeck::geode::dependency::audit_mods;
use geodedeck::geode::exporter::{export_gdpack_with_data_dir, import_gdpack_with_data_dir};
use geodedeck::geode::package::{
    mod_settings_path, scan_installed_mods_with_data_dir, set_mod_enabled_with_data_dir,
    write_mod_settings_map,
};
use geodedeck::geode::profiles::{
    apply_mod_profile_with_data_dir, capture_mod_profile_with_data_dir,
};
use geodedeck::launcher::detection::{
    evaluate_compatibility, parse_pe_file_version_bytes, parse_pe_timestamp_bytes,
    timestamp_to_gd_version, CompatState,
};
use geodedeck::launcher::integrity::verify_installation;
use geodedeck::launcher::versions::create_isolated_snapshot;
use geodedeck::optimizer::hardware::compute_recommended_fps;
use geodedeck::tools::crash_analyzer::{format_redacted_summary, parse_crash_log};

fn temp_test_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "geodedeck_test_{tag}_{}_{}",
        std::process::id(),
        chrono::Local::now().format("%H%M%S%3f")
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn synthetic_pe_with_timestamp(ts: u32) -> Vec<u8> {
    let mut buf = vec![0u8; 512];
    buf[0] = b'M';
    buf[1] = b'Z';
    let pe_offset: u32 = 0x80;
    buf[0x3C..0x40].copy_from_slice(&pe_offset.to_le_bytes());
    buf[0x80..0x84].copy_from_slice(b"PE\0\0");
    buf[0x88..0x8C].copy_from_slice(&ts.to_le_bytes());
    buf
}

fn encode_gd_dat(xml: &str) -> Vec<u8> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(xml.as_bytes()).unwrap();
    let compressed = gz.finish().unwrap();
    let b64 = URL_SAFE.encode(compressed);
    b64.bytes().map(|b| b ^ 11).collect()
}

fn create_synthetic_geode_zip(path: &Path, mod_json: &str) {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).unwrap();
    }
    let file = File::create(path).unwrap();
    let mut writer = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    writer.start_file("mod.json", opts).unwrap();
    writer.write_all(mod_json.as_bytes()).unwrap();
    writer.finish().unwrap();
}

#[test]
fn test_pe_timestamp_and_compatibility_matrix() {
    let pe_22081 = synthetic_pe_with_timestamp(0x69702da0);
    let ts = parse_pe_timestamp_bytes(&pe_22081).unwrap();
    assert_eq!(ts, 1768959392);
    assert_eq!(timestamp_to_gd_version(ts), Some("2.2081"));

    let pe_22091 = synthetic_pe_with_timestamp(1792203143);
    let ts2 = parse_pe_timestamp_bytes(&pe_22091).unwrap();
    assert_eq!(timestamp_to_gd_version(ts2), Some("2.2091"));

    // Synthetic VS_FIXEDFILEINFO for 5.11.2.0
    let mut ver_buf = vec![0u8; 64];
    ver_buf[16..20].copy_from_slice(&[0xBD, 0x04, 0xEF, 0xFE]);
    let ms = (5u32 << 16) | 11u32;
    let ls = (2u32 << 16) | 0u32;
    ver_buf[24..28].copy_from_slice(&ms.to_le_bytes());
    ver_buf[28..32].copy_from_slice(&ls.to_le_bytes());
    assert_eq!(parse_pe_file_version_bytes(&ver_buf).as_deref(), Some("5.11.2"));

    assert_eq!(
        evaluate_compatibility(Some("2.2081"), true, Some("5.11.2"), true).0,
        CompatState::Compatible
    );
    assert_eq!(
        evaluate_compatibility(Some("2.2091"), true, Some("6.0.0"), true).0,
        CompatState::Compatible
    );
    assert_eq!(
        evaluate_compatibility(Some("2.2091"), true, Some("5.11.2"), true).0,
        CompatState::ForwardCompatWarning
    );
    assert_eq!(
        evaluate_compatibility(Some("2.2081"), false, None, false).0,
        CompatState::VanillaNoGeode
    );
}

#[test]
fn test_integrity_and_isolated_snapshot() {
    let root = temp_test_dir("integrity");
    let game = root.join("GD");
    let slots = root.join("Slots");
    fs::create_dir_all(game.join("Resources")).unwrap();
    fs::create_dir_all(game.join("geode/resources/geode.loader")).unwrap();

    let pe = synthetic_pe_with_timestamp(1792203143);
    for bin in [
        "GeometryDash.exe",
        "libcocos2d.dll",
        "libExtensions.dll",
        "fmod.dll",
        "glew32.dll",
        "steam_api64.dll",
        "sqlite3.dll",
        "zlib1.dll",
        "libcurl.dll",
        "libpng16.dll",
        "libssl-3-x64.dll",
        "libcrypto-3-x64.dll",
        "websockets.dll",
        "uv.dll",
        "pthreadVC3.dll",
        "Geode.dll",
        "XInput1_4.dll",
    ] {
        fs::write(game.join(bin), &pe).unwrap();
    }
    for res in [
        "Resources/GJ_LaunchSheet.plist",
        "Resources/GJ_GameSheet.plist",
        "Resources/GJ_GameSheet02.plist",
        "Resources/GJ_GameSheet03.plist",
        "Resources/GJ_GameSheet04.plist",
        "Resources/bigFont.fnt",
        "Resources/chatFont.fnt",
        "Resources/goldFont.fnt",
    ] {
        fs::write(game.join(res), b"plist_data").unwrap();
    }

    let ok_rep = verify_installation(&game.to_string_lossy());
    assert!(ok_rep.overall_ok, "expected healthy install: {ok_rep:?}");
    assert!(ok_rep.geode_ok);

    // Corrupt a DLL by writing 0 bytes
    fs::write(game.join("fmod.dll"), b"").unwrap();
    let bad_rep = verify_installation(&game.to_string_lossy());
    assert!(!bad_rep.overall_ok);
    assert_eq!(bad_rep.corrupted_count, 1);

    // Snapshot into isolated slot
    let inst = create_isolated_snapshot(
        &game.to_string_lossy(),
        &slots.to_string_lossy(),
        "Test_2_2091",
        true,
    )
    .unwrap();
    assert!(Path::new(&inst.path).join("steam_appid.txt").is_file());
    assert_eq!(inst.detected_gd_version.as_deref(), Some("2.2091"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_geode_mods_toggle_dependency_profiles_and_gdpack_roundtrip() {
    let root = temp_test_dir("geode");
    let game = root.join("GD");
    let local_data = root.join("LocalGD");
    let exports = root.join("Exports");
    let profiles_dir = root.join("Profiles");
    fs::create_dir_all(game.join("geode/mods")).unwrap();
    fs::create_dir_all(&local_data).unwrap();

    create_synthetic_geode_zip(
        &game.join("geode/mods/author.core-api.geode"),
        r#"{
            "geode": "5.0.0",
            "gd": { "win": "2.2081" },
            "id": "author.core-api",
            "name": "Core API",
            "version": "1.2.0",
            "developer": "Author",
            "api": true
        }"#,
    );
    create_synthetic_geode_zip(
        &game.join("geode/mods/author.gameplay-mod.geode"),
        r#"{
            "geode": "5.3.0",
            "gd": "2.2081",
            "id": "author.gameplay-mod",
            "name": "Gameplay Mod",
            "version": "2.0.0",
            "developers": ["Author", "Helper"],
            "dependencies": {
                "author.core-api": ">=1.0.0"
            },
            "incompatibilities": {
                "author.conflict-mod": ">=1.0.0"
            }
        }"#,
    );

    let mut mods = scan_installed_mods_with_data_dir(&game.to_string_lossy(), &local_data);
    assert_eq!(mods.len(), 2);
    assert!(mods.iter().all(|m| m.enabled));

    let clean_audit = audit_mods(&mods, Some("2.2081"), Some("5.11.2"));
    assert_eq!(clean_audit.errors, 0);

    // Disable required dependency via saved.json and verify audit catches it!
    let core = mods.iter().find(|m| m.id == "author.core-api").unwrap().clone();
    set_mod_enabled_with_data_dir(&core, false, &local_data).unwrap();

    mods = scan_installed_mods_with_data_dir(&game.to_string_lossy(), &local_data);
    let core_after = mods.iter().find(|m| m.id == "author.core-api").unwrap();
    assert!(!core_after.enabled);

    let err_audit = audit_mods(&mods, Some("2.2081"), Some("5.11.2"));
    assert_eq!(err_audit.errors, 1);

    // Re-enable core API and write custom settings.json
    set_mod_enabled_with_data_dir(core_after, true, &local_data).unwrap();
    let mut cfg_map = BTreeMap::new();
    cfg_map.insert("precision-fix".into(), serde_json::Value::Bool(true));
    write_mod_settings_map(&local_data, "author.gameplay-mod", &cfg_map).unwrap();

    // Capture and apply Mod Profile
    let profile = capture_mod_profile_with_data_dir(
        "Competitive",
        "Test profile",
        &game.to_string_lossy(),
        "2.2081",
        true,
        &local_data,
        &profiles_dir,
    )
    .unwrap();
    assert_eq!(profile.enabled_mods.get("author.core-api"), Some(&true));

    // Export .gdpack.zip -> wipe mods & settings -> import back!
    let pack_path = export_gdpack_with_data_dir(
        "MyPack",
        &game.to_string_lossy(),
        &exports.to_string_lossy(),
        &[0, 1],
        true,
        "2.2081",
        "5.11.2",
        &local_data,
    )
    .unwrap();
    assert!(pack_path.is_file());

    fs::remove_dir_all(game.join("geode/mods")).unwrap();
    fs::remove_file(mod_settings_path(&local_data, "author.gameplay-mod")).unwrap();

    let imported = import_gdpack_with_data_dir(
        &pack_path,
        &game.to_string_lossy(),
        true,
        &local_data,
    )
    .unwrap();
    assert_eq!(imported.imported_mods, 2);
    assert_eq!(imported.restored_configs, 1);
    assert!(mod_settings_path(&local_data, "author.gameplay-mod").is_file());

    // Apply saved profile back
    let _ = apply_mod_profile_with_data_dir(
        &profile,
        &game.to_string_lossy(),
        true,
        &local_data,
        &profiles_dir,
    )
    .unwrap();

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_save_inspector_privacy_and_encrypted_vault_roundtrip() {
    let root = temp_test_dir("vault");
    let live = root.join("LocalGD");
    let vault_dir = root.join("Vault");
    let key_path = root.join("vault.key.dpapi");
    fs::create_dir_all(&live).unwrap();

    let gm_xml = r#"<?xml version="1.0"?><plist version="1.0"><dict><k>GJA_001</k><s>TestDasher</s><k>GJA_002</k><s>SUPER_SECRET_PASSWORD_123</s><k>GJA_005</k><s>SECRET_GJP2_HASH</s><k>GJA_003</k><i>424242</i></dict></plist>"#;
    let ll_xml = r#"<?xml version="1.0"?><plist version="1.0"><dict><k>LLM_01</k><d><k>k_0</k><d><k>kCEK</k><i>4</i><k>k2</k><s>Crystal Pulse</s><k>k3</k><s>U29tZSBUZXN0IERlc2M=</s><k>k45</k><i>1036782</i><k>k46</k><i>3</i></d></d></dict></plist>"#;

    let gm_bytes = encode_gd_dat(gm_xml);
    let mut ll_bytes = encode_gd_dat(ll_xml);
    // Simulate GD C++ save writer trailing NUL + uninitialized byte after XOR 11
    ll_bytes.push(0x00 ^ 11);
    ll_bytes.push(b'H' ^ 11);
    fs::write(live.join("CCGameManager.dat"), &gm_bytes).unwrap();
    fs::write(live.join("CCLocalLevels.dat"), &ll_bytes).unwrap();

    // 1. Validate XML decoding + Privacy (never expose GJA_002 or GJA_005!)
    let summary = inspect_save_dir(&live);
    assert!(summary.game_manager_valid);
    assert!(summary.local_levels_valid);
    assert_eq!(summary.player_name, "TestDasher");
    assert_eq!(summary.level_count, 1);
    assert_eq!(summary.levels[0].name, "Crystal Pulse");
    assert_eq!(summary.levels[0].description, "Some Test Desc");

    let serialized_sum = serde_json::to_string(&summary).unwrap();
    assert!(!serialized_sum.contains("SUPER_SECRET_PASSWORD_123"));
    assert!(!serialized_sum.contains("SECRET_GJP2_HASH"));

    let gmd_out = root.join("crystal.gmd");
    export_level_to_gmd(&summary.levels[0], &gmd_out).unwrap();
    assert!(fs::read_to_string(&gmd_out).unwrap().contains("Crystal Pulse"));

    // Corrupt .dat must fail validation
    assert!(decode_gd_dat_to_xml(b"").is_err());
    let bad_dat = root.join("broken.dat");
    fs::write(&bad_dat, b"not_valid_gd_save").unwrap();
    assert!(validate_dat_file(&bad_dat).is_err());

    // 2. Encrypted Vault Snapshot -> Corrupt Live -> Restore Roundtrip
    let snap = create_snapshot_with_paths(
        &live,
        &vault_dir,
        &key_path,
        "UnitTest",
        true,
        true,
        false,
        10,
    )
    .unwrap();
    assert!(snap.encrypted);
    assert!(verify_snapshot_integrity(Path::new(&snap.path), &key_path).is_ok());

    // Tamper check on AES-GCM cipher
    let key = load_or_create_vault_key(&key_path).unwrap();
    let mut enc = encrypt_vault_payload(&key, b"hello").unwrap();
    assert_eq!(decrypt_vault_payload(&key, &enc).unwrap(), b"hello");
    let last = enc.len() - 1;
    enc[last] ^= 0xFF;
    assert!(decrypt_vault_payload(&key, &enc).is_err());

    // Overwrite live save and restore!
    fs::write(live.join("CCGameManager.dat"), b"CORRUPTED").unwrap();
    let msg = restore_snapshot_with_paths(
        Path::new(&snap.path),
        &live,
        &vault_dir,
        &key_path,
        false,
    )
    .unwrap();
    assert!(msg.contains("Restored 2 GD save file(s)"));
    assert_eq!(fs::read(live.join("CCGameManager.dat")).unwrap(), gm_bytes);

    // Pre-restore safety backup should also now exist in vault_dir
    let all_snaps = list_snapshots(&vault_dir);
    assert!(all_snaps.len() >= 2);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_crash_parser_and_fps_calc() {
    let raw = r#"2026-11-23T03:19:22.7785823+0000
Whoopsies! An unhandled exception has occurred.
== Geode Information ==
Loader Version: v5.11.2
== Exception Information ==
Faulty Mod: Swag Route Larry v0.3.0 (com.dizzy.neural-dash)
Exception Code: c0000005 (EXCEPTION_ACCESS_VIOLATION)
Exception Details: Failed to read from memory at 0xFFFFFFFFFFFFFFFF
== Stack Trace (the most important part) ==
- com.dizzy.neural-dash.dll + 0x16211a (nd::NeuralDash::removeOverlays + 0x1a | C:\Users\alice\proj\src.cpp:346)
== Register States ==
RAX: 0x0
== Installed Mods ==
x | [v5.11.2] geode.loader
"#;
    let parsed = parse_crash_log("test.log", "C:\\test.log", raw);
    assert_eq!(parsed.loader_version, "v5.11.2");
    assert_eq!(
        parsed.faulty_mod_id.as_deref(),
        Some("com.dizzy.neural-dash")
    );
    assert!(parsed.exception_code.contains("c0000005"));
    assert_eq!(parsed.stack_trace.len(), 1);
    assert_eq!(parsed.installed_mods_count, 1);

    let summary = format_redacted_summary(&parsed, false);
    assert!(summary.contains("com.dizzy.neural-dash"));

    assert_eq!(compute_recommended_fps(60).0, 240);
    assert_eq!(compute_recommended_fps(144).0, 288);
    assert_eq!(compute_recommended_fps(240).0, 240);
}

#[test]
fn test_nong_song_metadata_resolution_cache_manual_rename_and_filters() {
    let root = temp_test_dir("nong_songs");
    let gd_data = root.join("LocalGD");
    let backup_dir = root.join("nong_backups");
    let cache_path = root.join("song_metadata_cache.json");
    let jukebox_mf = gd_data.join("geode/mods/fleym.nongd/manifest");
    fs::create_dir_all(&jukebox_mf).unwrap();
    fs::create_dir_all(&backup_dir).unwrap();

    // 1. Synthetic CCGameManager.dat with MDLM_001 + GLM_03 (and secret GJA keys that must never leak)
    let gm_xml = r#"<?xml version="1.0"?><plist version="1.0"><dict>
        <k>GJA_001</k><s>DashPlayer</s>
        <k>GJA_002</k><s>SECRET_PWD_SHOULD_NEVER_APPEAR</s>
        <k>GJA_005</k><s>SECRET_GJP2_SHOULD_NEVER_APPEAR</s>
        <k>GLM_03</k><d>
            <k>27690100</k><d><k>kCEK</k><i>4</i><k>k2</k><s>Slaughterhouse</s><k>k4</k><s>H4sIAAAAAAA=</s><k>k45</k><i>1036782</i></d>
        </d>
        <k>MDLM_001</k><d>
            <k>1036782</k><d><k>kCEK</k><i>6</i><k>1</k><i>1036782</i><k>2</k><s>Warm Seas &amp; Sky</s><k>4</k><s>AuSk</s></d>
        </d>
    </dict></plist>"#;
    fs::write(gd_data.join("CCGameManager.dat"), encode_gd_dat(gm_xml)).unwrap();

    let summary = inspect_save_dir(&gd_data);
    assert_eq!(summary.known_songs.len(), 1);
    assert_eq!(summary.known_songs[0].title, "Warm Seas & Sky");
    assert_eq!(summary.known_songs[0].artist, "AuSk");
    assert_eq!(
        summary.song_to_levels.get("1036782").unwrap(),
        &vec!["Slaughterhouse".to_string()]
    );

    let sum_json = serde_json::to_string(&summary).unwrap();
    assert!(!sum_json.contains("SECRET_PWD_SHOULD_NEVER_APPEAR"));
    assert!(!sum_json.contains("SECRET_GJP2_SHOULD_NEVER_APPEAR"));

    merge_save_metadata_into_cache(&cache_path, &summary);
    let cache_json = fs::read_to_string(&cache_path).unwrap();
    assert!(!cache_json.contains("SECRET_PWD_SHOULD_NEVER_APPEAR"));
    assert!(!cache_json.contains("SECRET_GJP2_SHOULD_NEVER_APPEAR"));

    // 2. Create cached audio files:
    // - 1036782.mp3 (resolved via GameManager + linked to Slaughterhouse)
    // - 1266014.mp3 (resolved via Jukebox active NONG)
    // - 999888.mp3 (resolved via ID3v2 embedded tags)
    // - 11223344.ogg (unknown, no metadata)
    // - 777.mp3 (0-byte broken)
    fs::write(gd_data.join("1036782.mp3"), vec![0xAAu8; 4096]).unwrap();
    fs::write(gd_data.join("1266014.mp3"), vec![0xBBu8; 8192]).unwrap();
    fs::write(backup_dir.join("1266014.mp3.orig"), b"orig").unwrap();

    // Minimal ID3v2.3 with TIT2="Tagged Track" and TPE1="Tag Artist"
    let mut id3 = Vec::new();
    id3.extend_from_slice(b"ID3\x03\x00\x00");
    let mut frames = Vec::new();
    for (fid, val) in [(b"TIT2", "Tagged Track"), (b"TPE1", "Tag Artist")] {
        frames.extend_from_slice(fid);
        let payload_len = (1 + val.len()) as u32;
        frames.extend_from_slice(&payload_len.to_be_bytes());
        frames.extend_from_slice(&[0x00, 0x00]); // flags
        frames.push(0x03); // UTF-8
        frames.extend_from_slice(val.as_bytes());
    }
    let sz = frames.len() as u32;
    id3.push(((sz >> 21) & 0x7F) as u8);
    id3.push(((sz >> 14) & 0x7F) as u8);
    id3.push(((sz >> 7) & 0x7F) as u8);
    id3.push((sz & 0x7F) as u8);
    id3.extend_from_slice(&frames);
    id3.extend_from_slice(&[0u8; 64]);
    fs::write(gd_data.join("999888.mp3"), &id3).unwrap();

    fs::write(gd_data.join("11223344.ogg"), vec![0xCCu8; 2048]).unwrap();
    fs::write(gd_data.join("777.mp3"), b"").unwrap();

    // Jukebox manifest with active NONG replacement
    let jb_raw = r#"{
        "default": {"name": "Tidal Wave Default", "unique_id": "def1", "artist": "RunoNG"},
        "active": "nong1",
        "hosted": [{"name": "Shiawase (VIP)", "unique_id": "nong1", "artist": "Dion Timmer"}]
    }"#;
    let jb_parsed = parse_jukebox_manifest_json(jb_raw).unwrap();
    assert!(jb_parsed.nong_active);
    assert_eq!(jb_parsed.nong_title.as_deref(), Some("Shiawase (VIP)"));
    fs::write(jukebox_mf.join("1266014.json"), jb_raw).unwrap();

    let report = scan_song_cache_with_paths(&gd_data, &backup_dir, &cache_path);
    assert_eq!(report.total_songs, 5);
    assert_eq!(report.zero_byte_count, 1);

    let s1 = report.songs.iter().find(|s| s.id == "1036782").unwrap();
    assert_eq!(s1.display_title(), "Warm Seas & Sky");
    assert_eq!(s1.display_artist(), "AuSk");
    assert_eq!(s1.associated_levels, vec!["Slaughterhouse".to_string()]);

    let s2 = report.songs.iter().find(|s| s.id == "1266014").unwrap();
    assert_eq!(s2.display_title(), "Shiawase (VIP)");
    assert_eq!(s2.display_artist(), "Dion Timmer");
    assert_eq!(s2.status(), SongCacheStatus::NongActive);
    assert_eq!(s2.source, MetadataSource::JukeboxNong);

    let s3 = report.songs.iter().find(|s| s.id == "999888").unwrap();
    assert_eq!(s3.display_title(), "Tagged Track");
    assert_eq!(s3.display_artist(), "Tag Artist");

    let s_unk = report.songs.iter().find(|s| s.id == "11223344").unwrap();
    assert_eq!(s_unk.display_title(), "Unknown Song — ID: 11223344");
    assert_eq!(s_unk.status(), SongCacheStatus::Unknown);

    // 3. Manual rename persistence & reset
    set_manual_song_metadata_at_path(&cache_path, "11223344", "Custom Track", "Custom Artist")
        .unwrap();
    let cache_loaded = load_metadata_cache_at_path(&cache_path);
    assert!(cache_loaded.songs["11223344"].manual_override);

    let report2 = scan_song_cache_with_paths(&gd_data, &backup_dir, &cache_path);
    let s_ren = report2.songs.iter().find(|s| s.id == "11223344").unwrap();
    assert_eq!(s_ren.display_title(), "Custom Track");
    assert_eq!(s_ren.display_artist(), "Custom Artist");
    assert_eq!(s_ren.status(), SongCacheStatus::ManualRenamed);

    // Search, sort & filter checks
    let by_level = filter_and_sort_songs(
        &report2.songs,
        "Slaughterhouse",
        SongFormatFilter::All,
        SongStatusFilter::All,
        SongSortMode::TitleAsc,
    );
    assert_eq!(by_level.len(), 1);
    assert_eq!(by_level[0].id, "1036782");

    let ogg_only = filter_and_sort_songs(
        &report2.songs,
        "",
        SongFormatFilter::Ogg,
        SongStatusFilter::All,
        SongSortMode::TitleAsc,
    );
    assert_eq!(ogg_only.len(), 1);
    assert_eq!(ogg_only[0].id, "11223344");

    let alpha = filter_and_sort_songs(
        &report2.songs,
        "",
        SongFormatFilter::All,
        SongStatusFilter::Resolved,
        SongSortMode::TitleAsc,
    );
    assert_eq!(alpha[0].display_title(), "Custom Track");

    clear_manual_song_metadata_at_path(&cache_path, "11223344").unwrap();
    let report3 = scan_song_cache_with_paths(&gd_data, &backup_dir, &cache_path);
    assert_eq!(
        report3
            .songs
            .iter()
            .find(|s| s.id == "11223344")
            .unwrap()
            .display_title(),
        "Unknown Song — ID: 11223344"
    );

    // 4. API & Tag parser unit checks
    assert_eq!(
        parse_boomlings_song_info("1~|~1036782~|~2~|~Warm%20Seas~|~4~|~AuSk~|~10~|~url"),
        Some(("Warm Seas".into(), "AuSk".into()))
    );
    assert_eq!(parse_boomlings_song_info("-1"), None);

    let mlib = parse_gd_music_library("149|30,Waterflame,url;|10012626,Running Away,30,1000,1;");
    assert_eq!(
        mlib.get("10012626"),
        Some(&("Running Away".into(), "Waterflame".into()))
    );

    let slib = parse_gd_sfx_library("10,Aquatic,1,0,0,0;10728,Splash,0,10,100,200;");
    assert_eq!(
        slib.get("s10728"),
        Some(&("Splash".into(), "GD SFX · Aquatic".into()))
    );

    // ID3v1 + Ogg Vorbis Comment parsers
    let mut id3v1 = [0u8; 128];
    id3v1[0..3].copy_from_slice(b"TAG");
    id3v1[3..13].copy_from_slice(b"V1 Song   ");
    id3v1[33..43].copy_from_slice(b"V1 Artist ");
    assert_eq!(
        parse_id3v1_bytes(&id3v1),
        Some((Some("V1 Song".into()), Some("V1 Artist".into())))
    );
    assert_eq!(
        parse_id3v2_bytes(&id3),
        Some((Some("Tagged Track".into()), Some("Tag Artist".into())))
    );

    let mut ogg = Vec::new();
    ogg.extend_from_slice(b"OggS\x00\x02\x00\x00");
    ogg.extend_from_slice(b"\x03vorbis");
    ogg.extend_from_slice(&4u32.to_le_bytes());
    ogg.extend_from_slice(b"Xiph");
    ogg.extend_from_slice(&2u32.to_le_bytes());
    for c in ["TITLE=Vorbis Song", "ARTIST=Vorbis Artist"] {
        ogg.extend_from_slice(&(c.len() as u32).to_le_bytes());
        ogg.extend_from_slice(c.as_bytes());
    }
    assert_eq!(
        parse_ogg_vorbis_comment_bytes(&ogg),
        Some((Some("Vorbis Song".into()), Some("Vorbis Artist".into())))
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_updater_tag_repo_normalization_and_config_migration() {
    assert_eq!(
        normalize_repo_slug("https://github.com/willmakesrandomshit/GeoDeck").unwrap(),
        "willmakesrandomshit/GeoDeck"
    );
    assert_eq!(
        normalize_repo_slug("https://github.com/willmakesrandomshit/GeoDeck.git").unwrap(),
        "willmakesrandomshit/GeoDeck"
    );
    assert_eq!(
        normalize_repo_slug("willmakesrandomshit/GeoDeck").unwrap(),
        "willmakesrandomshit/GeoDeck"
    );

    let a100 = parse_release_version("Alpha-1.0.0").unwrap();
    let a101 = parse_release_version("Alpha-1.0.1").unwrap();
    let b100 = parse_release_version("Beta-1.0.0").unwrap();
    let v100 = parse_release_version("v1.0.0").unwrap();
    let rel100 = parse_release_version("1.0.0").unwrap();
    assert!(a101 > a100);
    assert!(b100 > a100);
    assert!(v100 > b100);
    assert_eq!(rel100, v100);
    assert!(rel100 > a100);

    // Legacy v0.2.0 config migrates automatically to Alpha-1.0.0 defaults
    let legacy_json = r#"{
        "update_repo": "GeodeDeck/GeodeDeck",
        "check_updates_on_startup": false,
        "optimization_profile": "Balanced",
        "high_priority": false,
        "gpu_high_perf": false
    }"#;
    let mut cfg: Config = serde_json::from_str(legacy_json).unwrap();
    cfg.migrate_and_reconcile();
    assert_eq!(cfg.update_repo, "willmakesrandomshit/GeoDeck");
    assert!(cfg.check_updates_on_startup);
    assert_eq!(cfg.optimization_profile, OptimizationPreset::MaxPerformance);
    assert!(cfg.applied_tweaks.cpu_priority_high);
    assert!(cfg.applied_tweaks.cpu_affinity_perf);
    assert!(cfg.applied_tweaks.gpu_high_perf);
    assert!(cfg.applied_tweaks.disable_fullscreen_opt);
    assert_eq!(cfg.config_schema_version, 2);
}

fn minimal_valid_png_bytes() -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut c = !0u32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                let mask = (c & 1).wrapping_neg();
                c = (c >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !c
    }
    fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        out.extend_from_slice(tag);
        out.extend_from_slice(payload);
        let mut crc_in = Vec::with_capacity(4 + payload.len());
        crc_in.extend_from_slice(tag);
        crc_in.extend_from_slice(payload);
        out.extend_from_slice(&crc32(&crc_in).to_be_bytes());
        out
    }
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    // 1x1 RGBA IHDR
    png.extend_from_slice(&chunk(
        b"IHDR",
        &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0],
    ));
    // Valid zlib-compressed scanline [0, 255, 0, 255, 255]
    png.extend_from_slice(&chunk(
        b"IDAT",
        &[0x78, 0x9c, 0x63, 0xf8, 0xcf, 0xf0, 0x9f, 0x01, 0x05, 0x00, 0x04, 0xfc, 0x01, 0xff],
    ));
    png.extend_from_slice(&chunk(b"IEND", &[]));
    png
}

#[test]
fn test_player_stats_grinds_demons_and_pointercrate_crossref() {
    let root = temp_test_dir("demonlist");
    let gm_xml = r#"<?xml version="1.0"?><plist version="1.0"><dict>
        <k>GJA_001</k><s>ProDasher</s>
        <k>GJA_002</k><s>HIDDEN_SECRET_PASS</s>
        <k>GJA_005</k><s>HIDDEN_GJP2_SECRET</s>
        <k>GS_value</k><d>
            <k>1</k><s>50000</s><k>2</k><s>12345</s><k>5</k><s>42</s>
            <k>6</k><s>3500</s><k>8</k><s>149</s><k>12</k><s>250</s>
            <k>13</k><s>4800</s><k>14</k><s>90000</s><k>28</k><s>65</s>
        </d>
        <k>GLM_03</k><d>
            <k>1001</k><d><k>k2</k><s>Avernus</s><k>k5</k><s>PockeWindfish</s><k>k18</k><s>15400</s><k>k19</k><i>78</i><k>k21</k><i>100</i><k>k25</k><t/><k>k26</k><i>10</i><k>k4</k><s>H4sIAAAAAAA=</s></d>
            <k>1002</k><d><k>k2</k><s>Tidal Wave</s><k>k5</k><s>OniLink</s><k>k18</k><s>42000</s><k>k19</k><i>100</i><k>k21</k><i>100</i><k>k25</k><t/><k>k26</k><i>10</i><k>k4</k><s>H4sIAAAAAAA=</s></d>
        </d>
    </dict></plist>"#;
    fs::write(root.join("CCGameManager.dat"), encode_gd_dat(gm_xml)).unwrap();

    let sum = inspect_save_dir(&root);
    assert_eq!(sum.player_name, "ProDasher");
    assert_eq!(sum.player_stats.stars, 3500);
    assert_eq!(sum.player_stats.demons, 42);
    assert_eq!(sum.player_stats.moons, 65);
    assert_eq!(sum.player_stats.diamonds, 4800);
    assert_eq!(sum.active_grinds.len(), 1);
    assert_eq!(sum.active_grinds[0].name, "Avernus");
    assert_eq!(sum.completed_demons.len(), 1);
    assert_eq!(sum.completed_demons[0].name, "Tidal Wave");

    let json = serde_json::to_string(&sum).unwrap();
    assert!(!json.contains("HIDDEN_SECRET_PASS"));
    assert!(!json.contains("HIDDEN_GJP2_SECRET"));

    assert_eq!(
        geodedeck::tools::demonlist::pointercrate_page_url(150, None),
        "https://pointercrate.com/api/v2/demons/listed/?limit=100"
    );
    assert_eq!(
        geodedeck::tools::demonlist::pointercrate_page_url(50, Some(100)),
        "https://pointercrate.com/api/v2/demons/listed/?limit=50&after=100"
    );

    let pc_json = r#"[
        {"position": 1, "name": "Tidal Wave", "requirement": 100, "video": "https://youtu.be/tw", "publisher": {"name": "OniLink"}, "verifier": {"name": "Zoink"}},
        {"position": 2, "name": "Avernus", "requirement": 63, "video": "https://youtu.be/av", "publisher": {"name": "PockeWindfish"}, "verifier": {"name": "Zoink"}}
    ]"#;
    let list = geodedeck::tools::demonlist::parse_pointercrate_json(pc_json).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(
        geodedeck::tools::demonlist::match_demon_progress(&list[0].name, &sum.level_progress_by_name),
        Some((100, 42000))
    );
    assert_eq!(
        geodedeck::tools::demonlist::match_demon_progress(&list[1].name, &sum.level_progress_by_name),
        Some((78, 15400))
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_texture_packs_png_crc_validation_and_audio_replacers() {
    let root = temp_test_dir("assets_mgr");
    let game = root.join("GD");
    let res = game.join("Resources");
    fs::create_dir_all(&res).unwrap();

    let good_png = minimal_valid_png_bytes();
    fs::write(res.join("GJ_GameSheet-hd.png"), &good_png).unwrap();
    fs::write(res.join("StayInsideMe.mp3"), b"VANILLA_PRACTICE").unwrap();
    fs::write(res.join("explode_11.ogg"), b"VANILLA_DEATH").unwrap();
    assert!(geodedeck::tools::diagnostics::inspect_png_resource(&res.join("GJ_GameSheet-hd.png")).is_none());

    // Package a texture pack zip
    let pack_zip = root.join("CoolPack.zip");
    {
        let f = File::create(&pack_zip).unwrap();
        let mut zw = ZipWriter::new(f);
        zw.start_file("sub/GJ_GameSheet-hd.png", SimpleFileOptions::default())
            .unwrap();
        zw.write_all(&good_png).unwrap();
        zw.finish().unwrap();
    }

    geodedeck::tools::assets_manager::install_texture_pack(
        &game.to_string_lossy(),
        &pack_zip,
        Some("CoolPack"),
    )
    .unwrap();
    assert!(game.join("Resources_Vanilla_Backup/GJ_GameSheet-hd.png").is_file());
    let st = geodedeck::tools::assets_manager::get_assets_status(&game.to_string_lossy());
    assert_eq!(st.active_pack, "CoolPack");
    assert!(st.has_vanilla_backup);

    // Corrupt backup PNG CRC -> restore MUST fail without changing Resources/
    let mut bad_png = good_png.clone();
    let len = bad_png.len();
    bad_png[len - 15] ^= 0xFF;
    assert!(geodedeck::tools::diagnostics::inspect_png_resource(&res.join("GJ_GameSheet-hd.png")).is_none());
    fs::write(game.join("Resources_Vanilla_Backup/GJ_GameSheet-hd.png"), &bad_png).unwrap();
    assert!(geodedeck::tools::diagnostics::inspect_png_resource(
        &game.join("Resources_Vanilla_Backup/GJ_GameSheet-hd.png")
    )
    .is_some());
    assert!(
        geodedeck::tools::assets_manager::restore_vanilla_textures(&game.to_string_lossy()).is_err()
    );

    // Repair backup PNG -> restore succeeds
    fs::write(game.join("Resources_Vanilla_Backup/GJ_GameSheet-hd.png"), &good_png).unwrap();
    geodedeck::tools::assets_manager::restore_vanilla_textures(&game.to_string_lossy()).unwrap();

    // Practice music & death SFX replace + rollback
    let custom_mp3 = root.join("custom_practice.mp3");
    let custom_ogg = root.join("custom_death.ogg");
    fs::write(&custom_mp3, b"CUSTOM_PRACTICE").unwrap();
    fs::write(&custom_ogg, b"CUSTOM_DEATH").unwrap();

    geodedeck::tools::assets_manager::set_custom_practice_music(&game.to_string_lossy(), &custom_mp3)
        .unwrap();
    geodedeck::tools::assets_manager::set_custom_death_sfx(&game.to_string_lossy(), &custom_ogg)
        .unwrap();
    assert_eq!(fs::read(res.join("StayInsideMe.mp3")).unwrap(), b"CUSTOM_PRACTICE");
    assert_eq!(fs::read(res.join("explode_11.ogg")).unwrap(), b"CUSTOM_DEATH");

    geodedeck::tools::assets_manager::restore_vanilla_practice_music(&game.to_string_lossy())
        .unwrap();
    geodedeck::tools::assets_manager::restore_vanilla_death_sfx(&game.to_string_lossy()).unwrap();
    assert_eq!(fs::read(res.join("StayInsideMe.mp3")).unwrap(), b"VANILLA_PRACTICE");
    assert_eq!(fs::read(res.join("explode_11.ogg")).unwrap(), b"VANILLA_DEATH");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_save_compactor_slots_salvage_and_geodesync_bundle() {
    let root = temp_test_dir("save_tools");
    let save_dir = root.join("Save");
    let game_dir = root.join("GD");
    let vault_dir = root.join("Vault");
    let key_path = root.join("vault.key.dpapi");
    fs::create_dir_all(&save_dir).unwrap();
    fs::create_dir_all(game_dir.join("geode/mods")).unwrap();
    fs::create_dir_all(game_dir.join("geode/config/demo.mod")).unwrap();

    let gm_xml = r#"<?xml version="1.0"?><plist version="1.0"><dict>
        <k>GJA_001</k><s>Builder</s>
        <k>GS_value</k><d><k>5</k><s>12</s><k>6</k><s>750</s></d>
    </dict></plist>"#;
    let ll_xml = r#"<?xml version="1.0"?><plist version="1.0"><dict><k>LLM_01</k><d><k>_isArr</k><t/><k>k_0</k><d><k>kCEK</k><i>4</i><k>k2</k><s>MyLevel</s></d><k>k_1</k><d><k>kCEK</k><i>4</i><k>k1</k><i>999888</i><k>k2</k><s>CachedOnline</s><k>k21</k><i>1</i></d></d><k>LLM_02</k><i>42</i></dict></plist>"#;
    fs::write(save_dir.join("CCGameManager.dat"), encode_gd_dat(gm_xml)).unwrap();
    fs::write(save_dir.join("CCLocalLevels.dat"), encode_gd_dat(ll_xml)).unwrap();

    let bloat = geodedeck::backup::save_tools::analyze_save_bloat(&save_dir).unwrap();
    assert_eq!(bloat.total_levels, 2);
    assert_eq!(bloat.online_cached, 1);
    assert_eq!(bloat.user_created, 1);

    let compact = geodedeck::backup::save_tools::clean_save_bloat(&save_dir).unwrap();
    assert_eq!(compact.removed_levels, 1);
    let after_bloat = geodedeck::backup::save_tools::analyze_save_bloat(&save_dir).unwrap();
    assert_eq!(after_bloat.total_levels, 1);
    assert_eq!(after_bloat.user_created, 1);

    // Save slots + salvage
    geodedeck::backup::save_tools::create_save_slot(&save_dir, "MainProfile").unwrap();
    let slots = geodedeck::backup::save_tools::list_save_slots(&save_dir);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].stars, 750);

    let salvage = geodedeck::backup::save_tools::find_salvageable_saves(&save_dir);
    assert!(!salvage.is_empty());

    // Modify live saves -> switch slot back restores original
    fs::write(save_dir.join("CCGameManager.dat"), b"broken").unwrap();
    geodedeck::backup::save_tools::switch_save_slot(
        &save_dir,
        "MainProfile",
        &vault_dir,
        &key_path,
    )
    .unwrap();
    assert!(validate_dat_file(&save_dir.join("CCGameManager.dat")).is_ok());

    // .geodesync bundle export -> clear -> import
    fs::write(game_dir.join("geode/mods/demo.mod.geode"), b"modbytes").unwrap();
    fs::write(game_dir.join("geode/config/demo.mod/cfg.json"), b"{}").unwrap();
    let bundle = root.join("setup.geodesync");
    geodedeck::backup::save_tools::export_geodesync_bundle(
        &save_dir,
        &game_dir.to_string_lossy(),
        &bundle,
        true,
        true,
        true,
    )
    .unwrap();
    fs::remove_file(save_dir.join("CCGameManager.dat")).unwrap();
    fs::remove_file(game_dir.join("geode/mods/demo.mod.geode")).unwrap();

    geodedeck::backup::save_tools::import_geodesync_bundle(
        &bundle,
        &save_dir,
        &game_dir.to_string_lossy(),
        true,
        true,
        &vault_dir,
    )
    .unwrap();
    assert!(save_dir.join("CCGameManager.dat").is_file());
    assert!(game_dir.join("geode/mods/demo.mod.geode").is_file());
    assert!(game_dir.join("geode/config/demo.mod/cfg.json").is_file());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_diagnostics_pe_arch_keybinds_and_error_explainer() {
    let root = temp_test_dir("diag_kb");
    let game = root.join("GD");
    let local = root.join("LocalGD");
    fs::create_dir_all(local.join("geode/mods/mod.alpha")).unwrap();
    fs::create_dir_all(local.join("geode/mods/mod.beta")).unwrap();
    fs::create_dir_all(&game).unwrap();

    // Valid x64 EXE + mismatched x86 DLL
    fn make_pe(machine: u16) -> Vec<u8> {
        let mut buf = vec![0u8; 512];
        buf[0..2].copy_from_slice(b"MZ");
        buf[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        buf[0x80..0x84].copy_from_slice(b"PE\0\0");
        buf[0x84..0x86].copy_from_slice(&machine.to_le_bytes()); // Machine
        buf[0x86..0x88].copy_from_slice(&1u16.to_le_bytes()); // SectionCount = 1
        buf[0x94..0x96].copy_from_slice(&4u16.to_le_bytes()); // OptionalHeader size
        let magic: u16 = if machine == 0x014C { 0x10B } else { 0x20B };
        buf[0x98..0x9A].copy_from_slice(&magic.to_le_bytes());
        buf
    }
    fs::write(game.join("GeometryDash.exe"), make_pe(0x8664)).unwrap();
    fs::write(game.join("good.dll"), make_pe(0x8664)).unwrap();
    assert!(geodedeck::tools::diagnostics::validate_windows_game_images(&game).is_none());

    fs::write(game.join("bad_x86.dll"), make_pe(0x014C)).unwrap();
    let mismatch = geodedeck::tools::diagnostics::validate_windows_game_images(&game).unwrap();
    assert!(mismatch.contains("x86"));

    // Keybind collision detection
    fs::write(
        local.join("geode/mods/mod.alpha/settings.json"),
        r#"{"noclip":{"key":78,"modifiers":2}}"#,
    )
    .unwrap();
    fs::write(
        local.join("geode/mods/mod.beta/saved.json"),
        r#"{"speedhack":{"key":78,"modifiers":2}}"#,
    )
    .unwrap();
    let kb = geodedeck::geode::keybinds::scan_keybind_conflicts(&game.to_string_lossy(), &local);
    assert!(kb.conflicts.contains_key("N+Ctrl"));

    // Error Explainer signatures
    assert!(geodedeck::tools::diagnostics::explain_error_text("0xc000007b")
        .0
        .contains("0xc000007b"));
    assert!(geodedeck::tools::diagnostics::explain_error_text("WinError 392 OneDrive")
        .0
        .contains("OneDrive"));
    assert!(
        geodedeck::tools::diagnostics::explain_error_text("prevter.imageplus entry point not found")
            .0
            .contains("Incompatible Mod")
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_platform_updater_asset_selection() {
    use geodedeck::core::updater::{select_platform_asset_for, GithubAsset};
    let assets = vec![
        GithubAsset {
            name: "GeoDeck-Alpha-1.0.0-windows-x64.exe".into(),
            browser_download_url: "https://example.com/win.exe".into(),
            digest: None,
        },
        GithubAsset {
            name: "GeoDeck-Alpha-1.0.0-linux-x64.tar.gz".into(),
            browser_download_url: "https://example.com/linux.tar.gz".into(),
            digest: None,
        },
        GithubAsset {
            name: "GeoDeck-Alpha-1.0.0-macos-arm64.zip".into(),
            browser_download_url: "https://example.com/mac-arm64.zip".into(),
            digest: None,
        },
        GithubAsset {
            name: "GeoDeck-Alpha-1.0.0-macos-x64.zip".into(),
            browser_download_url: "https://example.com/mac-x64.zip".into(),
            digest: None,
        },
    ];
    assert_eq!(
        select_platform_asset_for(&assets, "windows", "x86_64").unwrap().name,
        "GeoDeck-Alpha-1.0.0-windows-x64.exe"
    );
    assert_eq!(
        select_platform_asset_for(&assets, "linux", "x86_64").unwrap().name,
        "GeoDeck-Alpha-1.0.0-linux-x64.tar.gz"
    );
    assert_eq!(
        select_platform_asset_for(&assets, "macos", "aarch64").unwrap().name,
        "GeoDeck-Alpha-1.0.0-macos-arm64.zip"
    );
    assert_eq!(
        select_platform_asset_for(&assets, "macos", "x86_64").unwrap().name,
        "GeoDeck-Alpha-1.0.0-macos-x64.zip"
    );
}
