use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileHealth {
    Healthy,
    Missing,
    Corrupted,
    OptionalMissing,
}

impl FileHealth {
    pub fn label(self) -> &'static str {
        match self {
            Self::Healthy => "Verified",
            Self::Missing => "Missing",
            Self::Corrupted => "Corrupted",
            Self::OptionalMissing => "Not Installed",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrityItem {
    pub relative_path: String,
    pub category: String,
    pub required: bool,
    pub health: FileHealth,
    pub size_bytes: u64,
    pub detail: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub game_path: String,
    pub checked_at: String,
    pub healthy_count: usize,
    pub missing_count: usize,
    pub corrupted_count: usize,
    pub overall_ok: bool,
    pub geode_ok: bool,
    pub items: Vec<IntegrityItem>,
}

const CORE_BINARIES: &[&str] = &[
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
];

const CORE_RESOURCES: &[&str] = &[
    "Resources/GJ_LaunchSheet.plist",
    "Resources/GJ_GameSheet.plist",
    "Resources/GJ_GameSheet02.plist",
    "Resources/GJ_GameSheet03.plist",
    "Resources/GJ_GameSheet04.plist",
    "Resources/bigFont.fnt",
    "Resources/chatFont.fnt",
    "Resources/goldFont.fnt",
];

const GEODE_FILES: &[&str] = &[
    "Geode.dll",
    "XInput1_4.dll",
    "geode/resources/geode.loader",
];

fn validate_pe_magic(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buf = [0u8; 4096];
    let n = file.read(&mut buf).map_err(|e| e.to_string())?;
    if n < 0x40 || &buf[0..2] != b"MZ" {
        return Err("Invalid MZ executable signature".into());
    }
    let e_lfanew = u32::from_le_bytes(buf[0x3C..0x40].try_into().unwrap()) as usize;
    if e_lfanew + 4 > n || &buf[e_lfanew..e_lfanew + 4] != b"PE\0\0" {
        return Err("Corrupt PE header".into());
    }
    Ok(())
}

pub fn verify_installation(game_path: &str) -> IntegrityReport {
    let root = Path::new(game_path);
    let mut items = Vec::new();

    if cfg!(target_os = "macos") && !root.join("GeometryDash.exe").is_file() {
        if let Some(app) = crate::core::platform::find_macos_game_app(root) {
            let app_valid = crate::core::platform::validate_macos_game_app(&app);
            items.push(IntegrityItem {
                relative_path: app
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Geometry Dash.app".into()),
                category: "Core Engine Binary".into(),
                required: true,
                health: if app_valid.is_ok() {
                    FileHealth::Healthy
                } else {
                    FileHealth::Corrupted
                },
                size_bytes: 0,
                detail: app_valid
                    .err()
                    .unwrap_or_else(|| "macOS Info.plist & executable verified".into()),
            });
            let res_root = crate::core::platform::gd_resources_dir(root);
            for rel in CORE_RESOURCES {
                let short = rel.strip_prefix("Resources/").unwrap_or(rel);
                items.push(check_file(&res_root, short, "Game Resource Asset", true, false));
            }
            let healthy_count = items.iter().filter(|i| i.health == FileHealth::Healthy).count();
            let missing_count = items
                .iter()
                .filter(|i| i.required && i.health == FileHealth::Missing)
                .count();
            let corrupted_count = items
                .iter()
                .filter(|i| i.health == FileHealth::Corrupted)
                .count();
            return IntegrityReport {
                game_path: game_path.to_string(),
                checked_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                healthy_count,
                missing_count,
                corrupted_count,
                overall_ok: missing_count == 0 && corrupted_count == 0,
                geode_ok: true,
                items,
            };
        }
    }

    for rel in CORE_BINARIES {
        items.push(check_file(root, rel, "Core Engine Binary", true, true));
    }
    for rel in CORE_RESOURCES {
        items.push(check_file(root, rel, "Game Resource Asset", true, false));
    }
    let geode_present = root.join("Geode.dll").exists() || root.join("XInput1_4.dll").exists();
    for rel in GEODE_FILES {
        let is_dir = *rel == "geode/resources/geode.loader";
        let is_pe = rel.ends_with(".dll");
        if is_dir {
            let full = root.join(rel);
            let exists = full.is_dir();
            items.push(IntegrityItem {
                relative_path: (*rel).into(),
                category: "Geode Framework".into(),
                required: geode_present,
                health: if exists {
                    FileHealth::Healthy
                } else if geode_present {
                    FileHealth::Missing
                } else {
                    FileHealth::OptionalMissing
                },
                size_bytes: 0,
                detail: if exists {
                    "Directory present".into()
                } else {
                    "Geode loader resources missing".into()
                },
            });
        } else {
            items.push(check_file(root, rel, "Geode Framework", geode_present, is_pe));
        }
    }

    let healthy_count = items
        .iter()
        .filter(|i| i.health == FileHealth::Healthy)
        .count();
    let missing_count = items
        .iter()
        .filter(|i| i.required && i.health == FileHealth::Missing)
        .count();
    let corrupted_count = items
        .iter()
        .filter(|i| i.health == FileHealth::Corrupted)
        .count();

    let geode_ok = !geode_present
        || items
            .iter()
            .filter(|i| i.category == "Geode Framework")
            .all(|i| i.health == FileHealth::Healthy);

    IntegrityReport {
        game_path: game_path.to_string(),
        checked_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        healthy_count,
        missing_count,
        corrupted_count,
        overall_ok: missing_count == 0 && corrupted_count == 0,
        geode_ok,
        items,
    }
}

fn check_file(
    root: &Path,
    rel: &str,
    category: &str,
    required: bool,
    check_pe: bool,
) -> IntegrityItem {
    let full = root.join(rel);
    if !full.is_file() {
        return IntegrityItem {
            relative_path: rel.into(),
            category: category.into(),
            required,
            health: if required {
                FileHealth::Missing
            } else {
                FileHealth::OptionalMissing
            },
            size_bytes: 0,
            detail: "File missing from installation directory".into(),
        };
    }

    let size = fs::metadata(&full).map(|m| m.len()).unwrap_or(0);
    if size == 0 {
        return IntegrityItem {
            relative_path: rel.into(),
            category: category.into(),
            required,
            health: FileHealth::Corrupted,
            size_bytes: 0,
            detail: "File is 0 bytes (truncated)".into(),
        };
    }

    if check_pe {
        if let Err(err) = validate_pe_magic(&full) {
            return IntegrityItem {
                relative_path: rel.into(),
                category: category.into(),
                required,
                health: FileHealth::Corrupted,
                size_bytes: size,
                detail: err,
            };
        }
    }

    IntegrityItem {
        relative_path: rel.into(),
        category: category.into(),
        required,
        health: FileHealth::Healthy,
        size_bytes: size,
        detail: "Header & size verified".into(),
    }
}
