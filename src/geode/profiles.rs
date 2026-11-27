use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::config::{config_dir, ModProfile};
use crate::geode::package::{
    local_gd_data_dir, mod_settings_path, scan_installed_mods_with_data_dir,
    set_mod_enabled_by_id,
};

fn profile_snapshots_dir() -> PathBuf {
    config_dir().join("profile_configs")
}

pub fn capture_mod_profile(
    name: &str,
    description: &str,
    game_dir: &str,
    target_gd_version: &str,
    include_settings_snapshot: bool,
) -> Result<ModProfile, String> {
    capture_mod_profile_with_data_dir(
        name,
        description,
        game_dir,
        target_gd_version,
        include_settings_snapshot,
        &local_gd_data_dir(),
        &profile_snapshots_dir(),
    )
}

pub fn capture_mod_profile_with_data_dir(
    name: &str,
    description: &str,
    game_dir: &str,
    target_gd_version: &str,
    include_settings_snapshot: bool,
    local_data_dir: &Path,
    profiles_dir: &Path,
) -> Result<ModProfile, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Profile name cannot be empty.".into());
    }
    let mods = scan_installed_mods_with_data_dir(game_dir, local_data_dir);
    let mut enabled_mods = BTreeMap::new();
    for m in &mods {
        enabled_mods.insert(m.id.clone(), m.enabled);
    }

    let id = format!(
        "prof-{}-{}",
        trimmed
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>(),
        chrono::Local::now().format("%H%M%S")
    );

    let mut has_settings_snapshot = false;
    if include_settings_snapshot {
        let snap_root = profiles_dir.join(&id);
        fs::create_dir_all(&snap_root).map_err(|e| format!("create profile snapshot dir: {e}"))?;
        for m in &mods {
            let src = mod_settings_path(local_data_dir, &m.id);
            if src.is_file() {
                let dst = snap_root.join(format!("{}.settings.json", m.id));
                let _ = fs::copy(&src, &dst);
                has_settings_snapshot = true;
            }
        }
    }

    Ok(ModProfile {
        id,
        name: trimmed.to_string(),
        target_gd_version: target_gd_version.to_string(),
        created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        description: description.trim().to_string(),
        enabled_mods,
        has_settings_snapshot,
    })
}

pub fn apply_mod_profile(
    profile: &ModProfile,
    game_dir: &str,
    restore_settings: bool,
) -> Result<usize, String> {
    apply_mod_profile_with_data_dir(
        profile,
        game_dir,
        restore_settings,
        &local_gd_data_dir(),
        &profile_snapshots_dir(),
    )
}

pub fn apply_mod_profile_with_data_dir(
    profile: &ModProfile,
    game_dir: &str,
    restore_settings: bool,
    local_data_dir: &Path,
    profiles_dir: &Path,
) -> Result<usize, String> {
    let mut changed = 0usize;
    let current = scan_installed_mods_with_data_dir(game_dir, local_data_dir);
    for m in &current {
        if let Some(&want) = profile.enabled_mods.get(&m.id) {
            if m.enabled != want {
                set_mod_enabled_by_id(game_dir, &m.id, want, local_data_dir)?;
                changed += 1;
            }
        }
    }

    if restore_settings && profile.has_settings_snapshot {
        let snap_root = profiles_dir.join(&profile.id);
        if snap_root.is_dir() {
            if let Ok(rd) = fs::read_dir(&snap_root) {
                for entry in rd.flatten() {
                    let fname = entry.file_name().to_string_lossy().into_owned();
                    if let Some(mod_id) = fname.strip_suffix(".settings.json") {
                        let dst = mod_settings_path(local_data_dir, mod_id);
                        if let Some(parent) = dst.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        let _ = fs::copy(entry.path(), dst);
                    }
                }
            }
        }
    }

    Ok(changed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickModPreset {
    Boosted,
    Vanilla,
    Tournament,
    Globed,
}

impl QuickModPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Boosted => "Boosted",
            Self::Vanilla => "Vanilla",
            Self::Tournament => "Tournament",
            Self::Globed => "Globed",
        }
    }
}

/// Applies one of the quick presets (`Boosted`, `Vanilla`, `Tournament`, `Globed`) using Geode's `saved.json`.
pub fn apply_quick_mod_preset(
    preset: QuickModPreset,
    game_dir: &str,
    local_data_dir: &Path,
) -> Result<String, String> {
    let mods = scan_installed_mods_with_data_dir(game_dir, local_data_dir);
    if mods.is_empty() {
        return Ok(format!(
            "Preset '{}' applied (no mods installed).",
            preset.label()
        ));
    }

    let mut adjusted = 0usize;
    for m in &mods {
        let lower_id = m.id.to_lowercase();
        let should_enable = match preset {
            QuickModPreset::Vanilla => false,
            QuickModPreset::Tournament => {
                !(lower_id.contains("globed") || lower_id.contains("imageplus"))
            }
            QuickModPreset::Globed => {
                if lower_id.contains("globed") || lower_id.contains("node-ids") {
                    true
                } else {
                    m.enabled
                }
            }
            QuickModPreset::Boosted => !(lower_id == "prevter.imageplus" && m.version == "1.1.1"),
        };
        if m.enabled != should_enable {
            set_mod_enabled_by_id(game_dir, &m.id, should_enable, local_data_dir)?;
            adjusted += 1;
        }
    }
    Ok(format!(
        "Preset '{}' applied ({adjusted} mod(s) toggled).",
        preset.label()
    ))
}

pub fn delete_profile_snapshot(profile_id: &str) {
    let dir = profile_snapshots_dir().join(profile_id);
    if dir.is_dir() {
        let _ = fs::remove_dir_all(dir);
    }
}
