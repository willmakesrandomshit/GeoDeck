use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use crate::core::platform::gd_resources_dir;
use crate::launcher::process::is_gd_running;
use crate::tools::diagnostics::inspect_png_resource;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TexturePackStatus {
    pub active_pack: String,
    pub has_vanilla_backup: bool,
    pub vanilla_file_count: usize,
    pub practice_music_custom: bool,
    pub practice_music_status: String,
    pub death_sfx_custom: bool,
    pub resources_exists: bool,
}

pub fn get_assets_status(game_path: &str) -> TexturePackStatus {
    if game_path.trim().is_empty() {
        return TexturePackStatus {
            active_pack: "Unknown".into(),
            practice_music_status: "No game folder set".into(),
            ..Default::default()
        };
    }

    let root = Path::new(game_path);
    let res_dir = gd_resources_dir(root);
    let resources_exists = res_dir.is_dir();

    let bak_dir = root.join("Resources_Vanilla_Backup");
    let vanilla_file_count = fs::read_dir(&bak_dir)
        .ok()
        .map(|rd| rd.flatten().filter(|e| e.path().is_file()).count())
        .unwrap_or(0);
    let has_vanilla_backup = vanilla_file_count > 0;

    let info_file = root.join("geode").join("active_texture_pack.json");
    let mut active_pack = "Vanilla (Default)".to_string();
    if let Ok(raw) = fs::read_to_string(&info_file) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(name) = v.get("name").and_then(|n| n.as_str()) {
                if !name.trim().is_empty() {
                    active_pack = name.trim().to_string();
                }
            }
        }
    }

    let practice_curr = res_dir.join("StayInsideMe.mp3");
    let practice_bak = res_dir.join("StayInsideMe.mp3.bak");
    let (practice_music_custom, practice_music_status) = if !practice_curr.is_file() {
        (false, "StayInsideMe.mp3 not found".into())
    } else if practice_bak.is_file() {
        (true, "Custom Practice Music Active".into())
    } else {
        (false, "Vanilla StayInsideMe.mp3".into())
    };

    let death_sfx_custom = res_dir.join("explode_11.ogg.bak").is_file();

    TexturePackStatus {
        active_pack,
        has_vanilla_backup,
        vanilla_file_count,
        practice_music_custom,
        practice_music_status,
        death_sfx_custom,
        resources_exists,
    }
}

pub fn backup_vanilla_textures(game_path: &str) -> Result<String, String> {
    if game_path.trim().is_empty() {
        return Err("Geometry Dash path not set.".into());
    }
    let root = Path::new(game_path);
    let res_dir = gd_resources_dir(root);
    if !res_dir.is_dir() {
        return Err("Resources directory not found.".into());
    }
    let bak_dir = root.join("Resources_Vanilla_Backup");
    if bak_dir.is_dir()
        && fs::read_dir(&bak_dir)
            .ok()
            .is_some_and(|mut rd| rd.next().is_some())
    {
        return Ok("Vanilla backup is already safe and up to date.".into());
    }

    fs::create_dir_all(&bak_dir).map_err(|e| format!("create backup dir: {e}"))?;
    let mut copied = 0usize;
    let rd = fs::read_dir(&res_dir).map_err(|e| format!("read Resources: {e}"))?;
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_file() {
            let fname = entry.file_name();
            fs::copy(&p, bak_dir.join(&fname))
                .map_err(|e| format!("copy {}: {e}", fname.to_string_lossy()))?;
            copied += 1;
        }
    }
    Ok(format!("Backed up {copied} vanilla texture/resource files."))
}

fn is_valid_texture_ext(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".plist")
        || lower.ends_with(".fnt")
        || lower.ends_with(".ogg")
        || lower.ends_with(".mp3")
        || lower.ends_with(".wav")
}

pub fn install_texture_pack(
    game_path: &str,
    source: &Path,
    pack_name: Option<&str>,
) -> Result<String, String> {
    if game_path.trim().is_empty() {
        return Err("Geometry Dash path not set.".into());
    }
    if is_gd_running() {
        return Err("Close Geometry Dash before installing a texture pack.".into());
    }
    if !source.exists() {
        return Err("Texture pack source does not exist.".into());
    }

    backup_vanilla_textures(game_path)?;

    let root = Path::new(game_path);
    let res_dir = gd_resources_dir(root);
    fs::create_dir_all(&res_dir).map_err(|e| format!("create Resources: {e}"))?;

    let display_name = pack_name
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            source
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "Custom Pack".into());

    let mut installed = 0usize;
    if source.is_file() {
        let f = File::open(source).map_err(|e| format!("open zip: {e}"))?;
        let mut archive = ZipArchive::new(f).map_err(|e| format!("invalid zip archive: {e}"))?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| format!("zip entry: {e}"))?;
            if entry.is_dir() {
                continue;
            }
            let Some(enclosed) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
                continue;
            };
            let Some(file_name) = enclosed.file_name().map(|n| n.to_os_string()) else {
                continue;
            };
            let fname_str = file_name.to_string_lossy();
            if !is_valid_texture_ext(&fname_str) {
                continue;
            }
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| format!("read {fname_str}: {e}"))?;
            fs::write(res_dir.join(&file_name), &buf)
                .map_err(|e| format!("write {fname_str}: {e}"))?;
            installed += 1;
        }
    } else if source.is_dir() {
        let mut stack = vec![source.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.is_file() {
                    let fname = entry.file_name();
                    let fname_str = fname.to_string_lossy();
                    if is_valid_texture_ext(&fname_str) {
                        fs::copy(&p, res_dir.join(&fname))
                            .map_err(|e| format!("copy {fname_str}: {e}"))?;
                        installed += 1;
                    }
                }
            }
        }
    } else {
        return Err("Texture pack source must be a .zip file or folder.".into());
    }

    let meta_dir = root.join("geode");
    fs::create_dir_all(&meta_dir).map_err(|e| format!("create geode dir: {e}"))?;
    let meta = json!({
        "name": display_name,
        "files": installed,
        "installed_at": chrono::Local::now().to_rfc3339(),
    });
    fs::write(
        meta_dir.join("active_texture_pack.json"),
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    )
    .map_err(|e| format!("write pack metadata: {e}"))?;

    Ok(format!(
        "Installed texture pack '{display_name}' ({installed} asset file(s) applied)."
    ))
}

pub fn restore_vanilla_textures(game_path: &str) -> Result<String, String> {
    if game_path.trim().is_empty() {
        return Err("Geometry Dash path not set.".into());
    }
    if is_gd_running() {
        return Err("Close Geometry Dash before restoring vanilla textures. No files were changed.".into());
    }

    let root = Path::new(game_path);
    let bak_dir = root.join("Resources_Vanilla_Backup");
    if !bak_dir.is_dir() {
        return Err("Vanilla backup does not exist yet. Backup vanilla textures before installing packs.".into());
    }

    let mut files = Vec::new();
    let rd = fs::read_dir(&bak_dir).map_err(|e| format!("read backup dir: {e}"))?;
    for entry in rd.flatten() {
        let p = entry.path();
        if let Ok(meta) = fs::symlink_metadata(&p) {
            if !meta.file_type().is_symlink() && p.is_file() {
                files.push(p);
            }
        }
    }
    if files.is_empty() {
        return Err("Vanilla backup contains no usable files. No files were changed.".into());
    }

    // Validate EVERY backup PNG structurally (signature + chunk CRC-32) before modifying Resources/
    for p in &files {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name.to_lowercase().ends_with(".png") {
            if let Some(problem) = inspect_png_resource(p) {
                return Err(format!(
                    "Vanilla backup texture '{name}' failed integrity check ({problem}). No files were changed."
                ));
            }
        }
    }

    let res_dir = gd_resources_dir(root);
    fs::create_dir_all(&res_dir).map_err(|e| format!("create Resources: {e}"))?;
    let total = files.len();
    for p in &files {
        if let Some(fname) = p.file_name() {
            fs::copy(p, res_dir.join(fname))
                .map_err(|e| format!("restore {}: {e}", fname.to_string_lossy()))?;
        }
    }

    let info_file = root.join("geode").join("active_texture_pack.json");
    if info_file.is_file() {
        let _ = fs::remove_file(&info_file);
    }

    Ok(format!("Restored {total} verified vanilla resource file(s)!"))
}

pub fn set_custom_practice_music(game_path: &str, source_audio: &Path) -> Result<String, String> {
    if game_path.trim().is_empty() {
        return Err("Geometry Dash path not set.".into());
    }
    if !source_audio.is_file() {
        return Err("Selected audio file does not exist.".into());
    }
    let res_dir = gd_resources_dir(Path::new(game_path));
    if !res_dir.is_dir() {
        return Err("Geometry Dash Resources directory not found.".into());
    }
    let orig = res_dir.join("StayInsideMe.mp3");
    let bak = res_dir.join("StayInsideMe.mp3.bak");
    if orig.is_file() && !bak.is_file() {
        fs::copy(&orig, &bak).map_err(|e| format!("backup vanilla practice music: {e}"))?;
    }
    fs::copy(source_audio, &orig).map_err(|e| format!("replace StayInsideMe.mp3: {e}"))?;
    Ok("Applied custom Practice Mode music (vanilla backed up as .bak).".into())
}

pub fn restore_vanilla_practice_music(game_path: &str) -> Result<String, String> {
    if game_path.trim().is_empty() {
        return Err("Geometry Dash path not set.".into());
    }
    let res_dir = gd_resources_dir(Path::new(game_path));
    let orig = res_dir.join("StayInsideMe.mp3");
    let bak = res_dir.join("StayInsideMe.mp3.bak");
    if !bak.is_file() {
        return Ok("Already using vanilla Practice Mode music.".into());
    }
    fs::copy(&bak, &orig).map_err(|e| format!("restore StayInsideMe.mp3: {e}"))?;
    let _ = fs::remove_file(&bak);
    Ok("Restored original vanilla Practice Mode music (StayInsideMe.mp3).".into())
}

pub fn set_custom_death_sfx(game_path: &str, source_audio: &Path) -> Result<String, String> {
    if game_path.trim().is_empty() {
        return Err("Geometry Dash path not set.".into());
    }
    if !source_audio.is_file() {
        return Err("Selected audio file does not exist.".into());
    }
    let res_dir = gd_resources_dir(Path::new(game_path));
    if !res_dir.is_dir() {
        return Err("Geometry Dash Resources directory not found.".into());
    }
    let orig = res_dir.join("explode_11.ogg");
    let bak = res_dir.join("explode_11.ogg.bak");
    if orig.is_file() && !bak.is_file() {
        fs::copy(&orig, &bak).map_err(|e| format!("backup vanilla death SFX: {e}"))?;
    }
    fs::copy(source_audio, &orig).map_err(|e| format!("replace explode_11.ogg: {e}"))?;
    Ok("Applied custom Death SFX (vanilla backed up as .bak).".into())
}

pub fn restore_vanilla_death_sfx(game_path: &str) -> Result<String, String> {
    if game_path.trim().is_empty() {
        return Err("Geometry Dash path not set.".into());
    }
    let res_dir = gd_resources_dir(Path::new(game_path));
    let orig = res_dir.join("explode_11.ogg");
    let bak = res_dir.join("explode_11.ogg.bak");
    if !bak.is_file() {
        return Ok("Already using vanilla Death SFX.".into());
    }
    fs::copy(&bak, &orig).map_err(|e| format!("restore explode_11.ogg: {e}"))?;
    let _ = fs::remove_file(&bak);
    Ok("Restored original vanilla Death SFX (explode_11.ogg).".into())
}
