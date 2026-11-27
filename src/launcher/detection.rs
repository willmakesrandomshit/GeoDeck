use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::UNIX_EPOCH;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatState {
    Compatible,
    ForwardCompatWarning,
    Incompatible,
    VanillaNoGeode,
    NotInstalled,
}

impl CompatState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Compatible => "Compatible",
            Self::ForwardCompatWarning => "Forward-Compat / Version Mismatch",
            Self::Incompatible => "Incompatible ABI",
            Self::VanillaNoGeode => "Vanilla (Geode Not Installed)",
            Self::NotInstalled => "Not Found",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SteamAppManifest {
    pub build_id: String,
    pub depot_322171_manifest: String,
    pub auto_update_behavior: String,
    pub size_on_disk: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstallationProbe {
    pub path: String,
    pub exe_exists: bool,
    pub exe_size_bytes: u64,
    pub exe_sha256: String,
    pub pe_timestamp: Option<u32>,
    pub gd_version: Option<String>,
    pub geode_installed: bool,
    pub geode_version: Option<String>,
    pub geode_proxy_present: bool,
    pub steam_manifest: Option<SteamAppManifest>,
    pub compat_state: CompatState,
    pub compat_message: String,
    pub disk_usage_bytes: u64,
    pub mod_count: usize,
}

impl Default for InstallationProbe {
    fn default() -> Self {
        Self {
            path: String::new(),
            exe_exists: false,
            exe_size_bytes: 0,
            exe_sha256: String::new(),
            pe_timestamp: None,
            gd_version: None,
            geode_installed: false,
            geode_version: None,
            geode_proxy_present: false,
            steam_manifest: None,
            compat_state: CompatState::NotInstalled,
            compat_message: "No Geometry Dash executable found at path.".into(),
            disk_usage_bytes: 0,
            mod_count: 0,
        }
    }
}

/// Official Windows Geometry Dash PE `TimeDateStamp` -> version table from Geode's `gdTimestampMap.hpp`.
pub fn timestamp_to_gd_version(ts: u32) -> Option<&'static str> {
    match ts {
        1419173053 => Some("1.900"),
        1419880840 => Some("1.910"),
        1421745341 => Some("1.920"),
        1440638199 => Some("2.000"),
        1440643927 => Some("2.001"),
        1443053232 => Some("2.010"),
        1443077847 => Some("2.020"),
        1484612867 => Some("2.100"),
        1484626658 => Some("2.101"),
        1484737207 => Some("2.102"),
        1510526914 => Some("2.110"),
        1510538091 => Some("2.111"),
        1510619253 => Some("2.112"),
        1511220108 => Some("2.113"),
        1702921605 => Some("2.200"),
        1704582672 => Some("2.201"),
        1704601266 => Some("2.202"),
        1704948277 => Some("2.203"),
        1705041028 => Some("2.204"),
        1717243515 => Some("2.206"),
        1731098609 => Some("2.207"),
        1731117052 => Some("2.2071"),
        1731130219 => Some("2.2072"),
        1731156923 => Some("2.2073"),
        1731376950 => Some("2.2074"),
        1768823161 => Some("2.208"),
        1768959392 => Some("2.2081"),
        1791899187 => Some("2.209"),
        1792203143 => Some("2.2091"),
        _ => None,
    }
}

pub fn read_pe_timestamp(path: &Path) -> Result<u32, String> {
    let mut file = File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut header = [0u8; 4096];
    let n = file
        .read(&mut header)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    parse_pe_timestamp_bytes(&header[..n])
}

pub fn parse_pe_timestamp_bytes(data: &[u8]) -> Result<u32, String> {
    if data.len() < 0x40 || &data[0..2] != b"MZ" {
        return Err("Missing MZ DOS header".into());
    }
    let e_lfanew = u32::from_le_bytes(data[0x3C..0x40].try_into().unwrap()) as usize;
    if e_lfanew.checked_add(24).is_none_or(|end| end > data.len()) {
        return Err("PE header offset out of bounds".into());
    }
    if &data[e_lfanew..e_lfanew + 4] != b"PE\0\0" {
        return Err("Missing PE signature".into());
    }
    let ts_offset = e_lfanew + 8;
    Ok(u32::from_le_bytes(
        data[ts_offset..ts_offset + 4].try_into().unwrap(),
    ))
}

/// Pure-Rust extraction of `VS_FIXEDFILEINFO` (`dwFileVersionMS`/`LS`) from a Windows PE binary.
/// Falls back gracefully if no version resource exists.
pub fn parse_pe_file_version(path: &Path) -> Option<String> {
    let data = fs::read(path).ok()?;
    parse_pe_file_version_bytes(&data)
}

pub fn parse_pe_file_version_bytes(data: &[u8]) -> Option<String> {
    // VS_FIXEDFILEINFO starts with signature 0xFEEF04BD (little-endian: BD 04 EF FE).
    const SIG: [u8; 4] = [0xBD, 0x04, 0xEF, 0xFE];
    let pos = data.windows(4).position(|w| w == SIG)?;
    if pos + 16 > data.len() {
        return None;
    }
    let ms = u32::from_le_bytes(data[pos + 8..pos + 12].try_into().ok()?);
    let ls = u32::from_le_bytes(data[pos + 12..pos + 16].try_into().ok()?);
    let major = (ms >> 16) & 0xFFFF;
    let minor = ms & 0xFFFF;
    let patch = (ls >> 16) & 0xFFFF;
    let build = ls & 0xFFFF;
    if major == 0 && minor == 0 && patch == 0 && build == 0 {
        return None;
    }
    if build > 0 {
        Some(format!("{major}.{minor}.{patch}.{build}"))
    } else {
        Some(format!("{major}.{minor}.{patch}"))
    }
}

pub fn detect_geode_version_from_logs(game_dir: &Path) -> Option<String> {
    let logs_dir = game_dir.join("geode").join("logs");
    let rd = fs::read_dir(logs_dir).ok()?;
    let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in rd.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("log") {
            if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
                if latest.as_ref().map(|(t, _)| m > *t).unwrap_or(true) {
                    latest = Some((m, p));
                }
            }
        }
    }
    let (_, log_path) = latest?;
    let content = fs::read_to_string(log_path).ok()?;
    for line in content.lines().take(25) {
        if let Some(idx) = line.find("Running Geode v") {
            let rest = &line[idx + "Running Geode v".len()..];
            if let Some(ver) = rest.split_whitespace().next() {
                return Some(ver.trim().to_string());
            }
        }
    }
    None
}

static SHA_CACHE: OnceLock<Mutex<HashMap<(PathBuf, u64, u64), String>>> = OnceLock::new();

pub fn cached_sha256(path: &Path) -> Result<(u64, String), String> {
    let meta = fs::metadata(path).map_err(|e| format!("stat {}: {e}", path.display()))?;
    let size = meta.len();
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let cache = SHA_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (path.to_path_buf(), size, mtime);
    if let Ok(guard) = cache.lock() {
        if let Some(digest) = guard.get(&key) {
            return Ok((size, digest.clone()));
        }
    }

    let mut file = File::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("read {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hex = format!("{:x}", hasher.finalize());
    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, hex.clone());
    }
    Ok((size, hex))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveredInstall {
    pub path: String,
    pub name: String,
    pub source: String,
    pub version: String,
    pub is_steam: bool,
    pub is_onedrive: bool,
}

pub fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    #[cfg(target_os = "windows")]
    {
        for base in [
            std::env::var_os("ProgramFiles(x86)").map(PathBuf::from),
            std::env::var_os("ProgramFiles").map(PathBuf::from),
        ]
        .into_iter()
        .flatten()
        {
            let steam = base.join("Steam");
            if steam.is_dir() && !roots.contains(&steam) {
                roots.push(steam);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mac_steam = crate::core::platform::home_dir()
            .join("Library")
            .join("Application Support")
            .join("Steam");
        if mac_steam.is_dir() && !roots.contains(&mac_steam) {
            roots.push(mac_steam);
        }
    }

    #[cfg(target_os = "linux")]
    {
        for r in crate::core::platform::discover_linux_steam_roots() {
            if !roots.contains(&r) {
                roots.push(r);
            }
        }
    }

    let mut libraries = Vec::new();
    for root in &roots {
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = fs::read_to_string(&vdf) {
            for line in text.lines() {
                let line = line.trim();
                if !line.starts_with("\"path\"") {
                    continue;
                }
                let parts: Vec<&str> = line.split('"').collect();
                if parts.len() >= 4 {
                    let raw_path = if cfg!(windows) {
                        parts[3].replace("\\\\", "\\")
                    } else {
                        parts[3].replace("\\\\", "/")
                    };
                    let pb = PathBuf::from(raw_path);
                    if pb.is_dir() && !libraries.contains(&pb) {
                        libraries.push(pb);
                    }
                }
            }
        }
    }
    for r in roots {
        if !libraries.contains(&r) {
            libraries.push(r);
        }
    }
    libraries
}

pub fn detect_steam_gd_path() -> Option<String> {
    for lib in steam_roots() {
        let candidate = lib.join("steamapps").join("common").join("Geometry Dash");
        if crate::core::platform::game_executable_exists(&candidate) {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    #[cfg(target_os = "windows")]
    for candidate in [
        r"C:\Program Files (x86)\Steam\steamapps\common\Geometry Dash",
        r"C:\Program Files\Steam\steamapps\common\Geometry Dash",
    ] {
        if crate::core::platform::game_executable_exists(Path::new(candidate)) {
            return Some(candidate.to_string());
        }
    }
    #[cfg(target_os = "macos")]
    {
        let candidate = crate::core::platform::home_dir()
            .join("Library")
            .join("Application Support")
            .join("Steam")
            .join("steamapps")
            .join("common")
            .join("Geometry Dash");
        if crate::core::platform::game_executable_exists(&candidate) {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

/// Discovers all Geometry Dash installations across Steam libraries, OneDrive, user folders, and common drives.
pub fn find_all_gd_installations() -> Vec<DiscoveredInstall> {
    let mut results = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut try_add = |folder: &Path, fallback_source: &str| {
        if !folder.is_dir() || !crate::core::platform::game_executable_exists(folder) {
            return;
        }
        let canon = fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
        let path_str = canon.to_string_lossy().into_owned();
        let norm = path_str.to_lowercase();
        if !seen.insert(norm.clone()) {
            return;
        }
        let is_steam = crate::core::platform::is_steam_library_game_path(&path_str);
        let is_onedrive = norm.contains("onedrive");
        let source = if is_steam {
            "Steam".to_string()
        } else if is_onedrive {
            "OneDrive".to_string()
        } else {
            fallback_source.to_string()
        };
        let exe = canon.join("GeometryDash.exe");
        let version = if exe.is_file() {
            read_pe_timestamp(&exe)
                .ok()
                .and_then(timestamp_to_gd_version)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Custom".into())
        } else if let Some(app) = crate::core::platform::find_macos_game_app(&canon) {
            fs::read_to_string(app.join("Contents").join("Info.plist"))
                .ok()
                .and_then(|s| {
                    crate::core::platform::extract_plist_xml_string(&s, "CFBundleShortVersionString")
                })
                .unwrap_or_else(|| "macOS".into())
        } else {
            "Unknown".into()
        };
        let name = canon
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Geometry Dash".into());

        results.push(DiscoveredInstall {
            path: path_str,
            name,
            source,
            version,
            is_steam,
            is_onedrive,
        });
    };

    for lib in steam_roots() {
        let common = lib.join("steamapps").join("common");
        if let Ok(rd) = fs::read_dir(&common) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    try_add(&p, "Steam");
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    for od in crate::core::platform::discover_onedrive_roots() {
        for sub in [
            "",
            "Desktop",
            "Documents",
            "Games",
            "GeometryDash",
            "Geometry Dash",
        ] {
            let target = if sub.is_empty() { od.clone() } else { od.join(sub) };
            if target.is_dir() {
                try_add(&target, "OneDrive");
                if let Ok(rd) = fs::read_dir(&target) {
                    for entry in rd.flatten() {
                        if entry.path().is_dir() {
                            try_add(&entry.path(), "OneDrive");
                        }
                    }
                }
            }
        }
    }

    let home = crate::core::platform::home_dir();
    for folder_name in ["Desktop", "Documents", "Downloads", "Games"] {
        let base = home.join(folder_name);
        if base.is_dir() {
            for name in ["Geometry Dash", "GeometryDash", "GD", "GD 2.2"] {
                try_add(&base.join(name), "Local");
            }
        }
    }

    #[cfg(target_os = "windows")]
    for drive in [r"C:\", r"D:\", r"E:\"] {
        for root_name in ["Games", "GeometryDash", "Geometry Dash", "GD"] {
            try_add(&Path::new(drive).join(root_name), "Local");
        }
    }

    results
}

pub fn parse_steam_appmanifest(game_dir: &Path) -> Option<SteamAppManifest> {
    // Expect <library>\steamapps\common\Geometry Dash -> parent -> parent is <library>\steamapps
    let steamapps = game_dir.parent()?.parent()?;
    let acf = steamapps.join("appmanifest_322170.acf");
    let text = fs::read_to_string(&acf).ok()?;
    let mut out = SteamAppManifest::default();
    let mut in_322171 = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line == "\"322171\"" {
            in_322171 = true;
            continue;
        }
        if in_322171 && line == "}" {
            in_322171 = false;
            continue;
        }
        let tokens: Vec<&str> = line.split('"').collect();
        if tokens.len() >= 4 {
            let key = tokens[1];
            let val = tokens[3];
            match key {
                "buildid" => out.build_id = val.to_string(),
                "AutoUpdateBehavior" => out.auto_update_behavior = val.to_string(),
                "SizeOnDisk" => out.size_on_disk = val.parse().unwrap_or(0),
                "manifest" if in_322171 => out.depot_322171_manifest = val.to_string(),
                _ => {}
            }
        }
    }
    Some(out)
}

pub fn expected_gd_for_geode(geode_ver: &str) -> Option<&'static str> {
    let major = geode_ver
        .trim_start_matches(['v', 'V'])
        .split('.')
        .next()
        .and_then(|s| s.parse::<u32>().ok())?;
    match major {
        6.. => Some("2.2091"),
        5 => Some("2.2081"),
        4 => Some("2.2074"),
        3 => Some("2.206"),
        2 => Some("2.204"),
        _ => None,
    }
}

pub fn evaluate_compatibility(
    gd_ver: Option<&str>,
    geode_installed: bool,
    geode_ver: Option<&str>,
    proxy_present: bool,
) -> (CompatState, String) {
    if gd_ver.is_none() {
        return (
            CompatState::NotInstalled,
            "GeometryDash.exe could not be verified or has an unrecognized PE timestamp.".into(),
        );
    }
    let gd = gd_ver.unwrap();
    if !geode_installed {
        return (
            CompatState::VanillaNoGeode,
            format!("Vanilla Geometry Dash {gd} detected. Install Geode Loader for mod support."),
        );
    }
    if !proxy_present {
        return (
            CompatState::Incompatible,
            "Geode.dll exists, but XInput1_4.dll proxy loader is missing. Repair Geode installation."
                .into(),
        );
    }
    match geode_ver {
        Some(gv) => {
            if let Some(expected_gd) = expected_gd_for_geode(gv) {
                if gd == expected_gd || (expected_gd == "2.2091" && gd == "2.209") {
                    (
                        CompatState::Compatible,
                        format!("Geode v{gv} is natively verified for Geometry Dash {gd}."),
                    )
                } else if gd > expected_gd {
                    (
                        CompatState::ForwardCompatWarning,
                        format!(
                            "GD {gd} is newer than Geode v{gv}'s primary target ({expected_gd}). Update Geode or switch to an isolated {expected_gd} slot."
                        ),
                    )
                } else {
                    (
                        CompatState::Incompatible,
                        format!(
                            "Geode v{gv} targets GD {expected_gd}, which may be incompatible with GD {gd}."
                        ),
                    )
                }
            } else {
                (
                    CompatState::ForwardCompatWarning,
                    format!("Geode v{gv} detected on GD {gd}."),
                )
            }
        }
        None => (
            CompatState::ForwardCompatWarning,
            format!("Geode.dll present on GD {gd}, but loader version could not be read."),
        ),
    }
}

pub fn probe_installation(path_str: &str) -> InstallationProbe {
    let mut probe = InstallationProbe {
        path: path_str.to_string(),
        ..Default::default()
    };
    if path_str.trim().is_empty() {
        return probe;
    }
    let dir = Path::new(path_str);
    let exe = dir.join("GeometryDash.exe");
    let mac_app = if cfg!(target_os = "macos") {
        crate::core::platform::find_macos_game_app(dir)
    } else {
        None
    };

    if exe.is_file() {
        probe.exe_exists = true;
        if let Ok((sz, sha)) = cached_sha256(&exe) {
            probe.exe_size_bytes = sz;
            probe.exe_sha256 = sha;
        }
        if let Ok(ts) = read_pe_timestamp(&exe) {
            probe.pe_timestamp = Some(ts);
            probe.gd_version = timestamp_to_gd_version(ts).map(|s| s.to_string());
        }
    } else if let Some(ref app) = mac_app {
        probe.exe_exists = crate::core::platform::validate_macos_game_app(app).is_ok();
        let plist = app.join("Contents").join("Info.plist");
        if let Ok(raw) = fs::read_to_string(&plist) {
            probe.gd_version =
                crate::core::platform::extract_plist_xml_string(&raw, "CFBundleShortVersionString")
                    .or_else(|| {
                        crate::core::platform::extract_plist_xml_string(&raw, "CFBundleVersion")
                    });
        }
        let geode_dylib = app.join("Contents").join("Frameworks").join("Geode.dylib").is_file()
            || app.join("Contents").join("MacOS").join("Geode.dylib").is_file()
            || app
                .join("Contents")
                .join("Frameworks")
                .join("Geode.framework")
                .join("Geode")
                .is_file();
        probe.geode_installed = geode_dylib || detect_geode_version_from_logs(dir).is_some();
        probe.geode_proxy_present = probe.geode_installed;
        if probe.geode_installed {
            probe.geode_version = detect_geode_version_from_logs(dir)
                .or_else(|| detect_geode_version_from_logs(&crate::geode::package::local_gd_data_dir()));
        }
    } else {
        return probe;
    }

    if mac_app.is_none() {
        let geode_dll = dir.join("Geode.dll");
        probe.geode_installed = geode_dll.is_file();
        probe.geode_proxy_present = dir.join("XInput1_4.dll").is_file();
        if probe.geode_installed {
            probe.geode_version =
                parse_pe_file_version(&geode_dll).or_else(|| detect_geode_version_from_logs(dir));
        }
    }

    probe.steam_manifest = parse_steam_appmanifest(dir);
    let (state, msg) = evaluate_compatibility(
        probe.gd_version.as_deref(),
        probe.geode_installed,
        probe.geode_version.as_deref(),
        probe.geode_proxy_present,
    );
    probe.compat_state = state;
    probe.compat_message = msg;

    probe.mod_count = count_geode_mods(dir);
    probe.disk_usage_bytes = dir_usage_fast(dir);
    probe
}

fn count_geode_mods(dir: &Path) -> usize {
    let mods_dir = dir.join("geode").join("mods");
    let Ok(rd) = fs::read_dir(mods_dir) else {
        return 0;
    };
    rd.flatten()
        .filter(|e| {
            let n = e.file_name();
            let s = n.to_string_lossy();
            s.ends_with(".geode") || s.ends_with(".geode.disabled")
        })
        .count()
}

pub fn dir_usage_fast(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(p) = stack.pop() {
        let Ok(rd) = fs::read_dir(&p) else {
            continue;
        };
        for e in rd.flatten() {
            let fp = e.path();
            if fp.is_dir() {
                stack.push(fp);
            } else if let Ok(m) = e.metadata() {
                total += m.len();
            }
        }
    }
    total
}

pub fn human_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} GB", b / (1024.0 * 1024.0 * 1024.0))
    } else if b >= 1024.0 * 1024.0 {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    } else if b >= 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
