use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VersionSlot {
    pub name: String,
    pub path: String,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DepotPreset {
    pub label: String,
    pub manifest: String,
    pub note: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InstallationKind {
    #[default]
    Steam,
    Snapshot,
    DepotSlot,
    Custom,
}

impl InstallationKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Steam => "Steam Primary",
            Self::Snapshot => "Isolated Snapshot",
            Self::DepotSlot => "Depot Version Slot",
            Self::Custom => "Custom Install",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GameInstallation {
    pub id: String,
    pub name: String,
    pub path: String,
    pub kind: InstallationKind,
    pub custom_args: String,
    pub geode_safe_mode: bool,
    pub notes: String,
    pub pinned_mod_profile: Option<String>,
    pub detected_gd_version: Option<String>,
    pub detected_geode_version: Option<String>,
}

impl Default for GameInstallation {
    fn default() -> Self {
        Self {
            id: "steam-primary".into(),
            name: "Steam Primary".into(),
            path: String::new(),
            kind: InstallationKind::Steam,
            custom_args: String::new(),
            geode_safe_mode: false,
            notes: "Default Steam Geometry Dash installation".into(),
            pinned_mod_profile: None,
            detected_gd_version: None,
            detected_geode_version: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LaunchMode {
    #[default]
    DirectExe,
    SteamProtocol,
    SafeMode,
}

impl LaunchMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::DirectExe => "Direct Executable",
            Self::SteamProtocol => "Steam Protocol",
            Self::SafeMode => "Geode Safe Mode",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GeodeLaunchFlags {
    pub safe_mode: bool,
    pub disable_crash_handler: bool,
    pub enable_tulip_hook_logs: bool,
    pub custom_binary_dir: String,
    pub extra_args: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BackupPolicy {
    pub auto_on_exit: bool,
    pub auto_pre_launch: bool,
    pub scheduled_interval_mins: u32,
    pub encrypt_vault: bool,
    pub verify_xml_before_backup: bool,
    pub include_geode_configs: bool,
    pub retention_count: usize,
    pub last_backup_at: String,
    pub last_backup_hash: String,
}

impl Default for BackupPolicy {
    fn default() -> Self {
        Self {
            auto_on_exit: true,
            auto_pre_launch: true,
            scheduled_interval_mins: 30,
            encrypt_vault: true,
            verify_xml_before_backup: true,
            include_geode_configs: true,
            retention_count: 25,
            last_backup_at: String::new(),
            last_backup_hash: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OptimizationPreset {
    Balanced,
    #[default]
    MaxPerformance,
    Compatibility,
    Custom,
}

impl OptimizationPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Balanced => "Balanced Performance",
            Self::MaxPerformance => "Maximum Performance",
            Self::Compatibility => "Compatibility & Stability",
            Self::Custom => "Custom Configuration",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppliedTweaksState {
    pub cpu_priority_high: bool,
    pub cpu_affinity_perf: bool,
    pub gpu_high_perf: bool,
    pub disable_fullscreen_opt: bool,
    pub close_steam_overlay_warn: bool,
    pub cbf_precision_fix: bool,
    pub cbf_mouse_fix: bool,
}

impl Default for AppliedTweaksState {
    fn default() -> Self {
        Self {
            cpu_priority_high: true,
            cpu_affinity_perf: true,
            gpu_high_perf: true,
            disable_fullscreen_opt: true,
            close_steam_overlay_warn: true,
            cbf_precision_fix: true,
            cbf_mouse_fix: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ModProfile {
    pub id: String,
    pub name: String,
    pub target_gd_version: String,
    pub created_at: String,
    pub description: String,
    pub enabled_mods: BTreeMap<String, bool>,
    pub has_settings_snapshot: bool,
}

impl Default for ModProfile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            target_gd_version: String::new(),
            created_at: String::new(),
            description: String::new(),
            enabled_mods: BTreeMap::new(),
            has_settings_snapshot: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    // Legacy fields preserved for full backward compatibility with v0.1.0 configs
    pub game_path: String,
    pub backup_dir: String,
    pub export_dir: String,
    pub auto_backup: bool,
    pub high_priority: bool,
    pub gpu_high_perf: bool,
    pub close_steam_overlay: bool,
    pub playtime_secs: u64,
    pub last_played: String,
    pub active_version: String,
    pub versions: Vec<VersionSlot>,
    pub depots: Vec<DepotPreset>,
    pub retention: usize,

    // Platform fields (v0.2.0+)
    pub versions_dir: String,
    pub installations: Vec<GameInstallation>,
    pub active_installation_id: String,
    pub launch_mode: LaunchMode,
    pub geode_flags: GeodeLaunchFlags,
    pub backup_policy: BackupPolicy,
    pub optimization_profile: OptimizationPreset,
    pub applied_tweaks: AppliedTweaksState,
    pub mod_profiles: Vec<ModProfile>,
    pub update_repo: String,
    pub check_updates_on_startup: bool,
    pub redact_crash_usernames: bool,
    pub discord_rpc_enabled: bool,
    pub active_mod_preset: String,
    #[serde(default)]
    pub config_schema_version: u32,
}

impl Default for Config {
    fn default() -> Self {
        let base = config_dir();
        let detected = crate::launcher::detection::detect_steam_gd_path().unwrap_or_default();
        let primary = GameInstallation {
            id: "steam-primary".into(),
            name: "Steam Primary".into(),
            path: detected.clone(),
            kind: InstallationKind::Steam,
            custom_args: String::new(),
            geode_safe_mode: false,
            notes: "Detected primary Steam installation".into(),
            pinned_mod_profile: None,
            detected_gd_version: None,
            detected_geode_version: None,
        };

        Self {
            game_path: detected,
            backup_dir: base.join("backups").to_string_lossy().into_owned(),
            export_dir: base.join("exports").to_string_lossy().into_owned(),
            auto_backup: true,
            high_priority: true,
            gpu_high_perf: true,
            close_steam_overlay: true,
            playtime_secs: 0,
            last_played: String::new(),
            active_version: "Steam Primary".into(),
            versions: Vec::new(),
            depots: default_depots(),
            retention: 25,

            versions_dir: base.join("versions").to_string_lossy().into_owned(),
            installations: vec![primary],
            active_installation_id: "steam-primary".into(),
            launch_mode: LaunchMode::DirectExe,
            geode_flags: GeodeLaunchFlags::default(),
            backup_policy: BackupPolicy::default(),
            optimization_profile: OptimizationPreset::MaxPerformance,
            applied_tweaks: AppliedTweaksState::default(),
            mod_profiles: Vec::new(),
            update_repo: "willmakesrandomshit/GeoDeck".into(),
            check_updates_on_startup: true,
            redact_crash_usernames: true,
            discord_rpc_enabled: true,
            active_mod_preset: "Boosted".into(),
            config_schema_version: 2,
        }
    }
}

pub fn default_depots() -> Vec<DepotPreset> {
    vec![
        DepotPreset {
            label: "Geometry Dash 2.2091 (Windows Latest)".into(),
            manifest: "latest".into(),
            note: "Current Steam release. Supported natively by Geode v6.0.0+".into(),
        },
        DepotPreset {
            label: "Geometry Dash 2.2081 (Windows)".into(),
            manifest: "3816559102876907245".into(),
            note: "Geode v5.x era build (pinned by legacy GeodeSync). Download side-by-side: download_depot 322170 322171 3816559102876907245".into(),
        },
        DepotPreset {
            label: "Geometry Dash 2.2074 (Windows)".into(),
            manifest: "1731376950".into(),
            note: "Classic Geode v4.x build. Use an isolated Version Slot so Steam updates never overwrite it.".into(),
        },
        DepotPreset {
            label: "Geometry Dash 2.207 (Windows)".into(),
            manifest: "7678373534998244044".into(),
            note: "Legacy Geode v4.0 build. Steam console: download_depot 322170 322171 7678373534998244044".into(),
        },
    ]
}

pub fn config_dir() -> PathBuf {
    crate::core::platform::geodedeck_config_dir()
}

impl Config {
    pub fn load() -> Self {
        let path = config_dir().join("config.json");
        let mut cfg = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Self>(&s).ok())
            .unwrap_or_default();
        cfg.migrate_and_reconcile();
        cfg
    }

    pub fn migrate_and_reconcile(&mut self) {
        if self.backup_dir.is_empty() {
            self.backup_dir = config_dir().join("backups").to_string_lossy().into_owned();
        }
        if self.export_dir.is_empty() {
            self.export_dir = config_dir().join("exports").to_string_lossy().into_owned();
        }
        if self.versions_dir.is_empty() {
            self.versions_dir = config_dir().join("versions").to_string_lossy().into_owned();
        }
        if self.depots.is_empty() {
            self.depots = default_depots();
        }
        if self.update_repo.trim().is_empty()
            || self.update_repo.trim().eq_ignore_ascii_case("GeodeDeck/GeodeDeck")
        {
            self.update_repo = "willmakesrandomshit/GeoDeck".into();
        }
        if self.config_schema_version < 2 {
            self.check_updates_on_startup = true;
            self.optimization_profile = OptimizationPreset::MaxPerformance;
            self.applied_tweaks = AppliedTweaksState::default();
            self.high_priority = true;
            self.gpu_high_perf = true;
            self.config_schema_version = 2;
        }
        // Migrate legacy VersionSlot entries into installations if not present
        for slot in &self.versions {
            if !slot.path.is_empty()
                && !self
                    .installations
                    .iter()
                    .any(|inst| inst.path.eq_ignore_ascii_case(&slot.path))
            {
                let id = format!(
                    "slot-{}",
                    slot.name
                        .to_lowercase()
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                        .collect::<String>()
                );
                self.installations.push(GameInstallation {
                    id,
                    name: slot.name.clone(),
                    path: slot.path.clone(),
                    kind: InstallationKind::Snapshot,
                    custom_args: String::new(),
                    geode_safe_mode: false,
                    notes: slot.note.clone(),
                    pinned_mod_profile: None,
                    detected_gd_version: None,
                    detected_geode_version: None,
                });
            }
        }
        if self.installations.is_empty() {
            let detected = if !self.game_path.is_empty() {
                self.game_path.clone()
            } else {
                crate::launcher::detection::detect_steam_gd_path().unwrap_or_default()
            };
            self.installations.push(GameInstallation {
                id: "steam-primary".into(),
                name: "Steam Primary".into(),
                path: detected,
                kind: InstallationKind::Steam,
                ..Default::default()
            });
            self.active_installation_id = "steam-primary".into();
        }
        // Ensure active installation path stays synchronized with legacy game_path
        if let Some(active) = self.active_installation().cloned() {
            if active.path.is_empty() && !self.game_path.is_empty() {
                if let Some(inst) = self
                    .installations
                    .iter_mut()
                    .find(|i| i.id == self.active_installation_id)
                {
                    inst.path = self.game_path.clone();
                }
            } else if !active.path.is_empty() {
                self.game_path = active.path;
                self.active_version = active.name;
            }
        } else if let Some(first) = self.installations.first().cloned() {
            self.active_installation_id = first.id;
            self.game_path = first.path;
            self.active_version = first.name;
        }
    }

    pub fn active_installation(&self) -> Option<&GameInstallation> {
        self.installations
            .iter()
            .find(|i| i.id == self.active_installation_id)
    }

    pub fn active_installation_mut(&mut self) -> Option<&mut GameInstallation> {
        let id = self.active_installation_id.clone();
        self.installations.iter_mut().find(|i| i.id == id)
    }

    pub fn set_active_installation(&mut self, id: &str) {
        if let Some(inst) = self.installations.iter().find(|i| i.id == id).cloned() {
            self.active_installation_id = inst.id;
            self.game_path = inst.path;
            self.active_version = inst.name;
        }
    }

    pub fn save(&self) {
        let dir = config_dir();
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(dir.join("config.json"), json);
        }
    }
}
