use std::fs;
use std::path::{Path, PathBuf};

use crate::core::config::{GameInstallation, InstallationKind};
use crate::core::platform::{game_executable_exists, open_url};
use crate::launcher::detection::{probe_installation, steam_roots};

pub fn sanitize_slot_name(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        format!("slot_{}", chrono::Local::now().format("%Y%m%d_%H%M%S"))
    } else {
        cleaned
    }
}

/// Snapshot an existing GD installation into an isolated Version Slot so Steam auto-updates cannot clobber it.
pub fn create_isolated_snapshot(
    source_dir: &str,
    versions_dir: &str,
    slot_name: &str,
    include_mods: bool,
) -> Result<GameInstallation, String> {
    let src = Path::new(source_dir);
    if !game_executable_exists(src) {
        return Err("Source path does not contain a Geometry Dash executable".into());
    }

    let folder_name = sanitize_slot_name(slot_name);
    let dst = PathBuf::from(versions_dir).join(&folder_name);
    if dst.exists() {
        return Err(format!("Version slot folder already exists: {}", dst.display()));
    }

    copy_dir_recursive(src, &dst, include_mods)?;

    // Ensure steam_appid.txt exists so launching GeometryDash.exe directly from a non-Steam slot integrates with Steam cleanly.
    let appid = dst.join("steam_appid.txt");
    if !appid.exists() {
        let _ = fs::write(&appid, "322170");
    }

    let probe = probe_installation(&dst.to_string_lossy());
    Ok(GameInstallation {
        id: format!("slot-{}", folder_name.to_lowercase()),
        name: slot_name.trim().to_string(),
        path: dst.to_string_lossy().into_owned(),
        kind: InstallationKind::Snapshot,
        custom_args: String::new(),
        geode_safe_mode: false,
        notes: format!(
            "Isolated snapshot cloned on {} (GD {})",
            chrono::Local::now().format("%Y-%m-%d %H:%M"),
            probe.gd_version.clone().unwrap_or_else(|| "Unknown".into())
        ),
        pinned_mod_profile: None,
        detected_gd_version: probe.gd_version,
        detected_geode_version: probe.geode_version,
    })
}

pub fn copy_dir_recursive(src: &Path, dst: &Path, include_geode_mods: bool) -> Result<(), String> {
    let mut stack = vec![(src.to_path_buf(), dst.to_path_buf())];
    while let Some((from, to)) = stack.pop() {
        fs::create_dir_all(&to).map_err(|e| format!("create dir {}: {e}", to.display()))?;
        let Ok(rd) = fs::read_dir(&from) else {
            continue;
        };
        for entry in rd.flatten() {
            let fp = entry.path();
            let fname = entry.file_name();
            let name_str = fname.to_string_lossy();
            // Never copy transient temp/unzipped/crash folders when cloning a clean slot
            if name_str == "temp" || name_str == "unzipped" {
                continue;
            }
            if !include_geode_mods && name_str == "geode" {
                continue;
            }
            let tp = to.join(&fname);
            if fp.is_dir() {
                stack.push((fp, tp));
            } else {
                fs::copy(&fp, &tp).map_err(|e| format!("copy {}: {e}", fp.display()))?;
            }
        }
    }
    Ok(())
}

/// Searches standard Steam depot download folders (`Steam\steamapps\content\app_322170\depot_322171`)
/// for downloaded version builds that can be safely imported side-by-side without overwriting Steam Primary.
pub fn find_downloaded_steam_depots() -> Vec<PathBuf> {
    let mut found = Vec::new();
    let depot_ids = if cfg!(target_os = "macos") {
        ["depot_322172", "depot_322171"]
    } else {
        ["depot_322171", "depot_322172"]
    };
    for root in steam_roots() {
        for dep in depot_ids {
            let depot = root
                .join("steamapps")
                .join("content")
                .join("app_322170")
                .join(dep);
            if game_executable_exists(&depot) && !found.contains(&depot) {
                found.push(depot);
            }
        }
    }
    found
}

pub fn import_depot_as_slot(
    depot_dir: &Path,
    versions_dir: &str,
    slot_name: &str,
) -> Result<GameInstallation, String> {
    let mut inst = create_isolated_snapshot(
        &depot_dir.to_string_lossy(),
        versions_dir,
        slot_name,
        false,
    )?;
    inst.kind = InstallationKind::DepotSlot;
    inst.notes = format!(
        "Imported from Steam depot content ({}) on {}",
        inst.detected_gd_version
            .clone()
            .unwrap_or_else(|| "Unknown".into()),
        chrono::Local::now().format("%Y-%m-%d %H:%M")
    );
    Ok(inst)
}

pub fn open_steam_console_for_manifest(manifest_id: &str) -> String {
    let depot_id = if cfg!(target_os = "macos") { "322172" } else { "322171" };
    let cmd_str = if manifest_id == "latest" {
        "app_update 322170 validate".to_string()
    } else {
        format!("download_depot 322170 {depot_id} {manifest_id}")
    };
    open_url("steam://open/console");
    cmd_str
}

pub fn trigger_steam_validate() {
    open_url("steam://validate/322170");
}

/// Locates Steam's `appmanifest_322170.acf` for a Geometry Dash installation.
pub fn find_appmanifest_path(game_path: &str) -> Option<PathBuf> {
    if !game_path.trim().is_empty() {
        if let Some(steamapps) = Path::new(game_path).parent().and_then(|p| p.parent()) {
            let acf = steamapps.join("appmanifest_322170.acf");
            if acf.is_file() {
                return Some(acf);
            }
        }
    }
    for root in steam_roots() {
        let acf = root.join("steamapps").join("appmanifest_322170.acf");
        if acf.is_file() {
            return Some(acf);
        }
    }
    None
}

/// Returns whether `appmanifest_322170.acf` is write-protected (read-only / immutable) to block unwanted Steam updates.
pub fn is_manifest_locked(game_path: &str) -> bool {
    let Some(acf) = find_appmanifest_path(game_path) else {
        return false;
    };
    let Ok(meta) = fs::metadata(&acf) else {
        return false;
    };
    if meta.permissions().readonly() {
        return true;
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("ls")
            .args(["-lO"])
            .arg(&acf)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if text.contains("uchg") {
                return true;
            }
        }
    }
    false
}

/// Toggles Steam's `appmanifest_322170.acf` update protection (read-only lock + `chflags uchg` on macOS).
pub fn set_manifest_locked(game_path: &str, lock: bool) -> Result<String, String> {
    let acf = find_appmanifest_path(game_path)
        .ok_or_else(|| "appmanifest_322170.acf not found for this Steam library.".to_string())?;

    let mut perms = fs::metadata(&acf)
        .map_err(|e| format!("stat manifest: {e}"))?
        .permissions();

    #[cfg(target_os = "macos")]
    {
        if lock {
            perms.set_readonly(true);
            fs::set_permissions(&acf, perms.clone()).map_err(|e| format!("chmod manifest: {e}"))?;
            let status = std::process::Command::new("chflags")
                .arg("uchg")
                .arg(&acf)
                .status()
                .map_err(|e| format!("chflags uchg failed: {e}"))?;
            if !status.success() {
                perms.set_readonly(false);
                let _ = fs::set_permissions(&acf, perms);
                return Err("Could not apply macOS uchg immutable lock on manifest.".into());
            }
        } else {
            let _ = std::process::Command::new("chflags")
                .arg("nouchg")
                .arg(&acf)
                .status();
            perms.set_readonly(false);
            fs::set_permissions(&acf, perms).map_err(|e| format!("unlock manifest: {e}"))?;
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        perms.set_readonly(lock);
        fs::set_permissions(&acf, perms).map_err(|e| format!("set manifest lock: {e}"))?;
    }

    Ok(if lock {
        "Steam Update Lock enabled: appmanifest_322170.acf is protected.".into()
    } else {
        "Steam Update Lock disabled: appmanifest_322170.acf is writable.".into()
    })
}

pub fn delete_slot_directory(inst: &GameInstallation) -> Result<(), String> {
    if inst.kind == InstallationKind::Steam {
        return Err("Refusing to delete primary Steam installation directory.".into());
    }
    let p = Path::new(&inst.path);
    if p.is_dir() {
        fs::remove_dir_all(p).map_err(|e| format!("remove slot dir: {e}"))?;
    }
    Ok(())
}
