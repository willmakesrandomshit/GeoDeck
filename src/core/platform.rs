use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const GD_STEAM_APP_ID: &str = "322170";

/// Returns the current OS user's login name across Windows, macOS, and Linux.
pub fn current_username() -> Option<String> {
    std::env::var("USERNAME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("USER")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
        .or_else(|| {
            std::env::var("LOGNAME")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
}

/// Returns the user's home directory across Windows, macOS, and Linux.
pub fn home_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(p) = std::env::var_os("USERPROFILE") {
            return PathBuf::from(p);
        }
        if let (Some(drive), Some(path)) =
            (std::env::var_os("HOMEDRIVE"), std::env::var_os("HOMEPATH"))
        {
            return PathBuf::from(format!(
                "{}{}",
                drive.to_string_lossy(),
                path.to_string_lossy()
            ));
        }
    }
    if let Some(p) = std::env::var_os("HOME") {
        return PathBuf::from(p);
    }
    std::env::temp_dir()
}

/// Persistent application configuration directory:
/// - Windows: `%APPDATA%\GeodeDeck` (preserved for backward compatibility)
/// - macOS: `~/Library/Application Support/GeodeDeck`
/// - Linux: `$XDG_CONFIG_HOME/geodedeck` or `~/.config/geodedeck`
pub fn geodedeck_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("GeodeDeck");
    }
    #[cfg(target_os = "macos")]
    {
        return home_dir()
            .join("Library")
            .join("Application Support")
            .join("GeodeDeck");
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("geodedeck");
            }
        }
        home_dir().join(".config").join("geodedeck")
    }
}

/// Opens a folder or path in the native file manager.
pub fn open_in_file_manager(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("explorer").arg(path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(path).spawn();
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}

/// Highlights a specific file in the OS file manager where supported.
pub fn reveal_file_in_file_manager(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").args(["-R"]).arg(path).spawn();
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let target = path.parent().unwrap_or(path);
        let _ = Command::new("xdg-open").arg(target).spawn();
    }
}

/// Opens an external HTTP(S) or `steam://` URL using the OS handler.
pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(url).spawn();
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

/// Resolves either a Steam game folder or a selected `.app` bundle on macOS.
pub fn find_macos_game_app(game_dir: &Path) -> Option<PathBuf> {
    if game_dir
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("app"))
    {
        return game_dir.is_dir().then(|| game_dir.to_path_buf());
    }
    for name in ["Geometry Dash.app", "GeometryDash.app"] {
        let candidate = game_dir.join(name);
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

/// Extracts a simple `<key>KEY</key><string>VALUE</string>` entry from XML `Info.plist`.
pub fn extract_plist_xml_string(plist_xml: &str, key: &str) -> Option<String> {
    let tag = format!("<key>{key}</key>");
    let idx = plist_xml.find(&tag)?;
    let rest = plist_xml[idx + tag.len()..].trim_start();
    let after_str = rest.strip_prefix("<string>")?;
    let end = after_str.find("</string>")?;
    Some(after_str[..end].trim().to_string())
}

/// Validates a macOS `Geometry Dash.app` bundle's `Info.plist` and main executable.
pub fn validate_macos_game_app(app_path: &Path) -> Result<(), String> {
    if !app_path.is_dir() {
        return Err("Geometry Dash.app was not found in the selected directory.".into());
    }
    let plist_path = app_path.join("Contents").join("Info.plist");
    let raw = fs::read_to_string(&plist_path)
        .map_err(|e| format!("Geometry Dash.app has a missing or unreadable Info.plist ({e})."))?;

    let exec_name = extract_plist_xml_string(&raw, "CFBundleExecutable").ok_or_else(|| {
        "Geometry Dash.app has a missing or invalid CFBundleExecutable in Info.plist.".to_string()
    })?;
    if exec_name.is_empty() || exec_name.contains('/') || exec_name.contains('\\') {
        return Err("Geometry Dash.app has an invalid CFBundleExecutable path.".into());
    }
    let executable = app_path.join("Contents").join("MacOS").join(&exec_name);
    if !executable.is_file() {
        return Err(format!(
            "Geometry Dash.app is incomplete: executable '{}' is missing.",
            exec_name
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&executable) {
            if meta.permissions().mode() & 0o111 == 0 {
                return Err(
                    "Geometry Dash.app's executable bit is missing. Verify integrity in Steam."
                        .into(),
                );
            }
        }
    }
    Ok(())
}

/// Returns true if a valid Geometry Dash executable exists at `game_dir` on the current OS.
pub fn game_executable_exists(game_dir: &Path) -> bool {
    if cfg!(target_os = "macos") {
        find_macos_game_app(game_dir).is_some() || game_dir.join("GeometryDash.exe").is_file()
    } else {
        game_dir.join("GeometryDash.exe").is_file()
    }
}

/// Returns the `Resources` directory inside a Geometry Dash folder (handling macOS `.app` bundles).
pub fn gd_resources_dir(game_dir: &Path) -> PathBuf {
    if let Some(app) = find_macos_game_app(game_dir) {
        let bundle_res = app.join("Contents").join("Resources");
        if bundle_res.is_dir() {
            return bundle_res;
        }
    }
    game_dir.join("Resources")
}

/// Returns the directory holding `geode/mods`, `geode/config`, and `geode/crashlogs`.
pub fn gd_geode_dir(game_dir: &Path) -> PathBuf {
    let in_game = game_dir.join("geode");
    if in_game.is_dir() || !cfg!(target_os = "macos") {
        return in_game;
    }
    home_dir()
        .join("Library")
        .join("Application Support")
        .join("GeometryDash")
        .join("geode")
}

/// Recognizes a Geometry Dash folder inside any Steam library (`.../steamapps/common/Geometry Dash`).
pub fn is_steam_library_game_path(path: &str) -> bool {
    let parts: Vec<String> = path
        .split(['/', '\\'])
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.trim().to_lowercase())
        .collect();
    for window in parts.windows(3) {
        if window[0] == "steamapps" && window[1] == "common" {
            let folder = &window[2];
            if folder == "gd"
                || folder.starts_with("geometry dash")
                || folder.starts_with("geometrydash")
            {
                return true;
            }
        }
    }
    false
}

/// Discovers native & Flatpak Steam installation roots on Linux (and SteamOS).
pub fn discover_linux_steam_roots() -> Vec<PathBuf> {
    let home = home_dir();
    let xdg_data = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("share"));

    let candidates = [
        std::env::var_os("STEAM_COMPAT_CLIENT_INSTALL_PATH").map(PathBuf::from),
        std::env::var_os("STEAM_DIR").map(PathBuf::from),
        Some(xdg_data.join("Steam")),
        Some(home.join(".steam").join("steam")),
        Some(home.join(".steam").join("root")),
        Some(
            home.join(".var")
                .join("app")
                .join("com.valvesoftware.Steam")
                .join("data")
                .join("Steam"),
        ),
    ];

    let mut roots = Vec::new();
    for cand in candidates.into_iter().flatten() {
        if cand.is_dir() {
            let canonical = fs::canonicalize(&cand).unwrap_or(cand);
            if !roots.contains(&canonical) {
                roots.push(canonical);
            }
        }
    }
    roots
}

/// Discovers common OneDrive root directories on Windows.
pub fn discover_onedrive_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for var in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        if let Some(val) = std::env::var_os(var) {
            let pb = PathBuf::from(val);
            if pb.is_dir() && !roots.contains(&pb) {
                roots.push(pb);
            }
        }
    }
    let home = home_dir();
    for name in ["OneDrive", "OneDrive - Personal"] {
        let pb = home.join(name);
        if pb.is_dir() && !roots.contains(&pb) {
            roots.push(pb);
        }
    }
    roots
}

fn dir_contains_gd_saves(dir: &Path) -> bool {
    let Ok(rd) = fs::read_dir(dir) else {
        return false;
    };
    rd.flatten().any(|e| {
        let name = e.file_name().to_string_lossy().to_lowercase();
        name.starts_with("cc") && name.ends_with(".dat")
    })
}

/// Resolves Geometry Dash's local save directory across Windows, macOS, and Linux/Proton.
pub fn resolve_gd_save_dir(game_path: Option<&str>) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let _ = game_path;
        return home_dir()
            .join("Library")
            .join("Application Support")
            .join("GeometryDash");
    }

    #[cfg(target_os = "linux")]
    {
        let mut candidates = Vec::new();
        if let Some(gp) = game_path {
            if let Some(steamapps) = Path::new(gp).parent().and_then(|p| p.parent()) {
                candidates.push(
                    steamapps
                        .join("compatdata")
                        .join(GD_STEAM_APP_ID)
                        .join("pfx/drive_c/users/steamuser/AppData/Local/GeometryDash"),
                );
            }
        }
        for root in discover_linux_steam_roots() {
            candidates.push(
                root.join("steamapps")
                    .join("compatdata")
                    .join(GD_STEAM_APP_ID)
                    .join("pfx/drive_c/users/steamuser/AppData/Local/GeometryDash"),
            );
        }
        for c in &candidates {
            if c.is_dir() && dir_contains_gd_saves(c) {
                return c.clone();
            }
        }
        for c in &candidates {
            if c.is_dir() {
                return c.clone();
            }
        }
        return candidates
            .into_iter()
            .next()
            .unwrap_or_else(|| home_dir().join(".local/share/GeometryDash"));
    }

    #[cfg(target_os = "windows")]
    {
        let local_default = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("GeometryDash");

        if local_default.is_dir() && dir_contains_gd_saves(&local_default) {
            return local_default;
        }

        let mut candidates = vec![local_default.clone()];
        if let Some(gp) = game_path {
            if !gp.trim().is_empty() {
                let p = PathBuf::from(gp);
                candidates.push(p.join("save"));
                candidates.push(p.join("saves"));
                candidates.push(p.join("AppData").join("Local").join("GeometryDash"));
                if !is_steam_library_game_path(gp) {
                    candidates.push(p);
                }
            }
        }
        for od in discover_onedrive_roots() {
            candidates.push(od.join("GeometryDash"));
            candidates.push(od.join("Documents").join("GeometryDash"));
            candidates.push(od.join("AppData").join("Local").join("GeometryDash"));
            candidates.push(od.join("Desktop").join("GeometryDash"));
        }
        if let Some(roaming) = std::env::var_os("APPDATA") {
            candidates.push(PathBuf::from(roaming).join("GeometryDash"));
        }
        let home = home_dir();
        candidates.push(home.join("Saved Games").join("GeometryDash"));
        candidates.push(home.join("Documents").join("GeometryDash"));

        for c in &candidates {
            if c.is_dir() && dir_contains_gd_saves(c) {
                return c.clone();
            }
        }
        return local_default;
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = game_path;
        std::env::temp_dir().join("GeometryDash")
    }
}
