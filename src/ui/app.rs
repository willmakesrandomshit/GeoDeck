use eframe::egui::{self, Color32, Margin, RichText, Rounding, Stroke, TextureHandle, Vec2};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::backup::save_inspector::{inspect_save_dir, SaveSummary};
use crate::backup::save_tools::{
    analyze_save_bloat, clean_save_bloat, create_save_slot, export_geodesync_bundle,
    find_salvageable_saves, import_geodesync_bundle, list_save_slots, resurrect_save,
    switch_save_slot, SalvageCandidate, SaveBloatReport, SaveSlotInfo,
};
use crate::backup::scheduler::{run_auto_backup_if_changed, BackupScheduler};
use crate::backup::vault::{
    create_snapshot, detect_legacy_external_backups, list_snapshots, restore_snapshot,
    SnapshotSummary,
};
use crate::core::config::{Config, GameInstallation, LaunchMode, OptimizationPreset};
use crate::core::logging;
use crate::core::platform::open_url;
use crate::core::presence::DiscordRpcHandle;
use crate::core::updater::{
    check_github_updates, current_app_display_version, download_and_verify_update,
    UpdateCheckResult,
};
use crate::core::worker::{
    spawn_job, ModUpdateAvailable, RefreshKind, ToastKind, WorkerBus, WorkerEvent,
};
use crate::geode::dependency::{audit_mods, CompatibilityAudit};
use crate::geode::exporter::{export_gdpack, import_gdpack};
use crate::geode::index_api::{
    check_updates_for_installed, fetch_latest_loader_release,
    install_mod_from_index, install_mod_version, install_or_update_geode_loader,
    install_specific_mod_version, search_index_mods, IndexMod, IndexModVersion, LoaderRelease,
};
use crate::geode::keybinds::{scan_keybind_conflicts, KeybindReport};
use crate::geode::package::{local_gd_data_dir, scan_installed_mods, InstalledMod};
use crate::geode::profiles::{apply_mod_profile, apply_quick_mod_preset, QuickModPreset};
use crate::launcher::detection::{
    find_all_gd_installations, probe_installation, DiscoveredInstall, InstallationProbe,
};
use crate::launcher::integrity::{verify_installation, IntegrityReport};
use crate::launcher::process::{launch_game, terminate_gd_process, ActiveSession};
use crate::launcher::versions::{
    create_isolated_snapshot, find_downloaded_steam_depots, import_depot_as_slot,
    is_manifest_locked, set_manifest_locked,
};
use crate::optimizer::tweaks::{
    apply_preset, clean_geode_caches, measure_geode_caches, CacheSizes,
};
use crate::tools::assets_manager::{
    backup_vanilla_textures, get_assets_status, install_texture_pack, restore_vanilla_death_sfx,
    restore_vanilla_practice_music, restore_vanilla_textures, set_custom_death_sfx,
    set_custom_practice_music, TexturePackStatus,
};
use crate::tools::crash_analyzer::{scan_crash_logs, CrashReport};
use crate::tools::demonlist::{fetch_pointercrate_demons, PointercrateDemon};
use crate::tools::diagnostics::{
    ensure_local_onedrive_files, scan_installation_issues, DiagnosticFinding,
};
use crate::tools::songs::{
    apply_metadata_to_report, default_song_metadata_cache_path, merge_save_metadata_into_cache,
    resolve_missing_song_metadata, scan_song_cache, SongCacheReport, SongFormatFilter,
    SongSortMode, SongStatusFilter,
};
use crate::ui::assets::{load_brand_icon_texture, load_brand_lockup_texture};
use crate::ui::icons::ph;
use crate::ui::theme::*;
use crate::ui::views;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavTab {
    Home,
    Library,
    Mods,
    Demonlist,
    Backups,
    Tools,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModsSubTab {
    Installed,
    BrowseIndex,
    DependencyAudit,
    ProfilesAndExport,
}

#[derive(Clone, Debug)]
pub struct ToastItem {
    pub kind: ToastKind,
    pub title: String,
    pub body: String,
    pub created_at: Instant,
}

pub struct UiTransientState {
    pub library_subtab: usize,
    pub tools_subtab: usize,
    pub backups_subtab: usize,
    pub demonlist_subtab: usize,
    pub new_slot_name: String,
    pub clone_include_mods: bool,
    pub selected_depot_idx: usize,
    pub selected_found_depot_idx: usize,
    pub mods_subtab: ModsSubTab,
    pub mod_search: String,
    pub mod_filter_mode: usize,
    pub index_query: String,
    pub index_sort: String,
    pub index_match_gd: bool,
    pub index_featured_only: bool,
    pub index_category: usize,
    pub index_page: usize,
    pub demon_search: String,
    pub new_profile_name: String,
    pub profile_snapshot_settings: bool,
    pub export_pack_name: String,
    pub export_only_enabled: bool,
    pub export_include_configs: bool,
    pub new_save_slot_name: String,
    pub bundle_include_saves: bool,
    pub bundle_include_mods: bool,
    pub bundle_include_configs: bool,
    pub error_explainer_input: String,
    pub error_explainer_result: Option<(String, String)>,
    pub pending_restore_snapshot: Option<SnapshotSummary>,
    pub pending_delete_snapshot: Option<SnapshotSummary>,
    pub restore_include_configs: bool,
    pub pending_delete_install: Option<GameInstallation>,
    pub pending_remove_mod: Option<InstalledMod>,
    pub nong_song_id: String,
    pub song_search: String,
    pub song_sort: SongSortMode,
    pub song_format_filter: SongFormatFilter,
    pub song_status_filter: SongStatusFilter,
    pub rename_song_modal: Option<(String, String, String, bool)>,
    pub log_level_filter: usize,
    pub focus_search_requested: bool,
}

impl Default for UiTransientState {
    fn default() -> Self {
        Self {
            library_subtab: 0,
            tools_subtab: 0,
            backups_subtab: 0,
            demonlist_subtab: 0,
            new_slot_name: format!("GD_Slot_{}", chrono::Local::now().format("%Y%m%d")),
            clone_include_mods: true,
            selected_depot_idx: 0,
            selected_found_depot_idx: 0,
            mods_subtab: ModsSubTab::Installed,
            mod_search: String::new(),
            mod_filter_mode: 0,
            index_query: String::new(),
            index_sort: "downloads".into(),
            index_match_gd: true,
            index_featured_only: false,
            index_category: 0,
            index_page: 1,
            demon_search: String::new(),
            new_profile_name: String::new(),
            profile_snapshot_settings: true,
            export_pack_name: "MyGeodePack".into(),
            export_only_enabled: false,
            export_include_configs: true,
            new_save_slot_name: String::new(),
            bundle_include_saves: true,
            bundle_include_mods: true,
            bundle_include_configs: true,
            error_explainer_input: String::new(),
            error_explainer_result: None,
            pending_restore_snapshot: None,
            pending_delete_snapshot: None,
            restore_include_configs: true,
            pending_delete_install: None,
            pending_remove_mod: None,
            nong_song_id: String::new(),
            song_search: String::new(),
            song_sort: SongSortMode::FileSizeDesc,
            song_format_filter: SongFormatFilter::All,
            song_status_filter: SongStatusFilter::All,
            rename_song_modal: None,
            log_level_filter: 0,
            focus_search_requested: false,
        }
    }
}

pub struct GeodeDeckApp {
    pub config: Config,
    pub active_tab: NavTab,
    pub nav_back_stack: Vec<NavTab>,
    pub nav_forward_stack: Vec<NavTab>,
    pub bus: WorkerBus,
    pub egui_ctx: Option<egui::Context>,
    pub discord_rpc: DiscordRpcHandle,

    pub probe: InstallationProbe,
    pub integrity_report: IntegrityReport,
    pub downloaded_depots: Vec<PathBuf>,
    pub discovered_installs: Vec<DiscoveredInstall>,
    pub latest_loader_release: Option<LoaderRelease>,
    pub manifest_locked: bool,

    pub installed_mods: Vec<InstalledMod>,
    pub mod_audit: CompatibilityAudit,
    pub mod_updates: Vec<ModUpdateAvailable>,
    pub index_results: Vec<IndexMod>,
    pub index_total: usize,
    pub keybind_report: KeybindReport,

    // Texture caches
    pub brand_lockup: Option<TextureHandle>,
    pub brand_icon: Option<TextureHandle>,
    pub logo_textures: HashMap<String, TextureHandle>,
    game_art: Option<(String, Option<TextureHandle>)>,

    pub max_perf_applied: bool,
    pub last_backup_tick: Instant,

    pub snapshots: Vec<SnapshotSummary>,
    pub save_summary: Option<SaveSummary>,
    pub legacy_backups: Vec<(String, PathBuf)>,
    pub backup_scheduler: BackupScheduler,
    pub save_bloat: Option<SaveBloatReport>,
    pub save_slots: Vec<SaveSlotInfo>,
    pub salvage_candidates: Vec<SalvageCandidate>,

    pub crash_reports: Vec<CrashReport>,
    pub song_report: SongCacheReport,
    pub song_metadata_resolving: bool,
    pub in_flight_song_ids: HashSet<String>,
    pub demonlist: Vec<PointercrateDemon>,
    pub demonlist_loading: bool,
    pub demonlist_error: Option<String>,
    pub health_findings: Vec<DiagnosticFinding>,
    pub health_checking: bool,
    pub assets_status: TexturePackStatus,
    pub cache_sizes: CacheSizes,
    pub app_update_result: Option<Result<UpdateCheckResult, String>>,
    pub update_banner_dismissed: bool,

    pub active_session: Option<ActiveSession>,
    pub busy_label: Option<String>,
    pub toasts: Vec<ToastItem>,
    pub ui_state: UiTransientState,
    pub initialized: bool,
    pub startup_time: Instant,
    pub initial_sync_complete: bool,

    // Headless screenshot capture helper (--capture <path.png>)
    pub capture_output: Option<PathBuf>,
    pub capture_started_at: Instant,
    pub capture_requested: bool,
}

impl GeodeDeckApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_with_options(cc, None, None, None)
    }

    pub fn new_with_capture(
        cc: &eframe::CreationContext<'_>,
        capture_output: Option<PathBuf>,
    ) -> Self {
        Self::new_with_options(cc, capture_output, None, None)
    }

    pub fn new_with_options(
        cc: &eframe::CreationContext<'_>,
        capture_output: Option<PathBuf>,
        initial_tab: Option<NavTab>,
        initial_tools_subtab: Option<usize>,
    ) -> Self {
        setup_fonts_and_theme(&cc.egui_ctx);
        let config = Config::load();
        config.save();
        logging::info(
            "Startup",
            format!(
                "GeoDeck {} initialized (Active: {})",
                current_app_display_version(),
                config.active_version
            ),
        );

        let brand_lockup = Some(load_brand_lockup_texture(&cc.egui_ctx));
        let brand_icon = Some(load_brand_icon_texture(&cc.egui_ctx));

        let mut ui_state = UiTransientState::default();
        if let Some(sub) = initial_tools_subtab {
            ui_state.tools_subtab = sub;
        }

        let discord_rpc = DiscordRpcHandle::new(config.discord_rpc_enabled);
        let manifest_locked = is_manifest_locked(&config.game_path);
        let assets_status = get_assets_status(&config.game_path);

        let mut app = Self {
            config,
            active_tab: initial_tab.unwrap_or(NavTab::Home),
            nav_back_stack: Vec::new(),
            nav_forward_stack: Vec::new(),
            bus: WorkerBus::new(),
            egui_ctx: Some(cc.egui_ctx.clone()),
            discord_rpc,

            probe: InstallationProbe::default(),
            integrity_report: IntegrityReport::default(),
            downloaded_depots: find_downloaded_steam_depots(),
            discovered_installs: Vec::new(),
            latest_loader_release: None,
            manifest_locked,

            installed_mods: Vec::new(),
            mod_audit: CompatibilityAudit::default(),
            mod_updates: Vec::new(),
            index_results: Vec::new(),
            index_total: 0,
            keybind_report: KeybindReport::default(),

            brand_lockup,
            brand_icon,
            logo_textures: HashMap::new(),
            game_art: None,

            max_perf_applied: false,
            last_backup_tick: Instant::now() - Duration::from_secs(5),

            snapshots: Vec::new(),
            save_summary: None,
            legacy_backups: Vec::new(),
            backup_scheduler: BackupScheduler::new(),
            save_bloat: None,
            save_slots: Vec::new(),
            salvage_candidates: Vec::new(),

            crash_reports: Vec::new(),
            song_report: SongCacheReport::default(),
            song_metadata_resolving: false,
            in_flight_song_ids: HashSet::new(),
            demonlist: Vec::new(),
            demonlist_loading: false,
            demonlist_error: None,
            health_findings: Vec::new(),
            health_checking: false,
            assets_status,
            cache_sizes: CacheSizes::default(),
            app_update_result: None,
            update_banner_dismissed: false,

            active_session: None,
            busy_label: Some("Syncing installation, mods & vault...".into()),
            toasts: Vec::new(),
            ui_state,
            initialized: false,
            startup_time: Instant::now(),
            initial_sync_complete: false,

            capture_output,
            capture_started_at: Instant::now(),
            capture_requested: false,
        };

        app.apply_max_performance_async();
        app.refresh_all_async();
        app.search_geode_index_async();
        app.run_health_check_async();
        if app.capture_output.is_none() {
            app.refresh_demonlist_async();
            if app.config.check_updates_on_startup {
                app.check_app_update_silent_async();
            }
        } else if initial_tab == Some(NavTab::Demonlist) {
            app.refresh_demonlist_async();
        }
        app.initialized = true;
        app
    }

    pub fn game_art(&mut self, ctx: &egui::Context) -> Option<TextureHandle> {
        let path = &self.config.game_path;
        if let Some((cached_path, texture)) = &self.game_art {
            if cached_path == path {
                return texture.clone();
            }
        }
        let texture = Some(crate::ui::assets::load_game_background(ctx, Path::new(path)))
            .flatten();
        self.game_art = Some((path.clone(), texture.clone()));
        texture
    }

    pub fn navigate_to(&mut self, target: NavTab) {
        if self.active_tab == target {
            return;
        }
        self.nav_back_stack.push(self.active_tab);
        if self.nav_back_stack.len() > 24 {
            self.nav_back_stack.remove(0);
        }
        self.nav_forward_stack.clear();
        self.active_tab = target;
    }

    pub fn navigate_back(&mut self) {
        if let Some(prev) = self.nav_back_stack.pop() {
            self.nav_forward_stack.push(self.active_tab);
            self.active_tab = prev;
        }
    }

    pub fn navigate_forward(&mut self) {
        if let Some(next) = self.nav_forward_stack.pop() {
            self.nav_back_stack.push(self.active_tab);
            self.active_tab = next;
        }
    }

    pub fn push_toast(
        &mut self,
        kind: ToastKind,
        title: impl Into<String>,
        body: impl Into<String>,
    ) {
        let title = title.into();
        let body = body.into();
        match kind {
            ToastKind::Info => logging::info("UI", format!("{title}: {body}")),
            ToastKind::Success => logging::ok("UI", format!("{title}: {body}")),
            ToastKind::Warning => logging::warn("UI", format!("{title}: {body}")),
            ToastKind::Error => logging::err("UI", format!("{title}: {body}")),
        }
        self.toasts.push(ToastItem {
            kind,
            title,
            body,
            created_at: Instant::now(),
        });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub fn mod_logo_texture(
        &mut self,
        mod_id: &str,
        rgba: Option<&(u32, u32, Vec<u8>)>,
        ctx: &egui::Context,
    ) -> Option<TextureHandle> {
        if let Some(tex) = self.logo_textures.get(mod_id) {
            return Some(tex.clone());
        }
        let (w, h, pixels) = rgba?;
        let img = egui::ColorImage::from_rgba_unmultiplied([*w as usize, *h as usize], pixels);
        let handle = ctx.load_texture(
            format!("mod_logo_{mod_id}"),
            img,
            egui::TextureOptions::LINEAR,
        );
        self.logo_textures.insert(mod_id.to_string(), handle.clone());
        Some(handle)
    }

    pub fn apply_max_performance_async(&mut self) {
        self.config.optimization_profile = OptimizationPreset::MaxPerformance;
        self.config.applied_tweaks = crate::core::config::AppliedTweaksState::default();
        self.config.high_priority = true;
        self.config.gpu_high_perf = true;
        self.config.save();

        let game_path = self.config.game_path.clone();
        if game_path.is_empty() {
            self.max_perf_applied = true;
            return;
        }
        let active_exe = Path::new(&game_path)
            .join("GeometryDash.exe")
            .to_string_lossy()
            .into_owned();
        let mut tweaks = self.config.applied_tweaks.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = apply_preset(
                OptimizationPreset::MaxPerformance,
                &active_exe,
                &mut tweaks,
            );
            let _ = tx.send(WorkerEvent::MaxPerfApplied(res));
        });
    }

    pub fn refresh_all_async(&mut self) {
        self.refresh_probe_async();
        self.refresh_mods_async();
        self.refresh_backups_async();
        self.refresh_tools_async();
    }

    pub fn refresh_probe_async(&mut self) {
        let path = self.config.game_path.clone();
        self.manifest_locked = is_manifest_locked(&path);
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let probe = probe_installation(&path);
            let _ = tx.send(WorkerEvent::ProbeCompleted(probe));
        });
    }

    pub fn scan_discovered_installs_async(&mut self) {
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let found = find_all_gd_installations();
            let _ = tx.send(WorkerEvent::InstallsScanned(found));
        });
    }

    pub fn toggle_manifest_lock_action(&mut self) {
        let target = !self.manifest_locked;
        match set_manifest_locked(&self.config.game_path, target) {
            Ok(msg) => {
                self.manifest_locked = is_manifest_locked(&self.config.game_path);
                self.push_toast(ToastKind::Success, "Steam Update Lock", msg);
            }
            Err(e) => self.push_toast(ToastKind::Error, "Manifest Lock Failed", e),
        }
    }

    pub fn apply_quick_preset_async(&mut self, preset: QuickModPreset) {
        self.config.active_mod_preset = preset.label().into();
        self.config.save();
        let game = self.config.game_path.clone();
        self.busy_label = Some(format!("Applying {} preset...", preset.label()));
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = apply_quick_mod_preset(preset, &game, &local_gd_data_dir());
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Mod Preset".into(),
                result: res,
                refresh: RefreshKind::Mods,
            });
        });
    }

    pub fn downgrade_mod_version_async(&mut self, mod_id: String, target_ver: String) {
        self.busy_label = Some(format!("Installing {mod_id} v{target_ver}..."));
        let game = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = install_specific_mod_version(&game, &mod_id, &target_ver);
            let _ = tx.send(WorkerEvent::ModInstallFinished {
                mod_id,
                result: res,
            });
        });
    }

    pub fn refresh_demonlist_async(&mut self) {
        if self.demonlist_loading {
            return;
        }
        self.demonlist_loading = true;
        self.demonlist_error = None;
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = fetch_pointercrate_demons(150);
            let _ = tx.send(WorkerEvent::DemonlistFetched(res));
        });
    }

    pub fn run_health_check_async(&mut self) {
        if self.health_checking {
            return;
        }
        self.health_checking = true;
        let game = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let save_dir = local_gd_data_dir();
            let findings = scan_installation_issues(&game, &save_dir);
            let _ = tx.send(WorkerEvent::HealthCheckCompleted(findings));
            let cache = measure_geode_caches(&game);
            let _ = tx.send(WorkerEvent::CacheSizesMeasured(cache));
        });
    }

    pub fn scan_keybinds_async(&mut self) {
        let game = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let rep = scan_keybind_conflicts(&game, &local_gd_data_dir());
            let _ = tx.send(WorkerEvent::KeybindsScanned(rep));
        });
    }

    pub fn clean_geode_caches_async(&mut self) {
        let game = self.config.game_path.clone();
        self.busy_label = Some("Cleaning Geode temp & old logs...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = clean_geode_caches(&game).map(|bytes| {
                format!(
                    "Reclaimed {} of Geode temporary cache and old logs.",
                    crate::launcher::detection::human_bytes(bytes)
                )
            });
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Cache Cleaner".into(),
                result: res,
                refresh: RefreshKind::Tools,
            });
        });
    }

    pub fn pin_onedrive_local_async(&mut self) {
        let game = self.config.game_path.clone();
        self.busy_label = Some("Pinning OneDrive files locally...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let mut msg = ensure_local_onedrive_files(Path::new(&game));
            let save = local_gd_data_dir();
            if save.to_string_lossy().to_lowercase().contains("onedrive") {
                msg = ensure_local_onedrive_files(&save);
            }
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "OneDrive Pin".into(),
                result: msg,
                refresh: RefreshKind::Tools,
            });
        });
    }

    pub fn install_texture_pack_async(&mut self, src: PathBuf) {
        let game = self.config.game_path.clone();
        self.busy_label = Some("Installing texture pack...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = install_texture_pack(&game, &src, None);
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Texture Pack".into(),
                result: res,
                refresh: RefreshKind::Tools,
            });
        });
    }

    pub fn backup_vanilla_textures_async(&mut self) {
        let game = self.config.game_path.clone();
        self.busy_label = Some("Backing up vanilla textures...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = backup_vanilla_textures(&game);
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Vanilla Texture Backup".into(),
                result: res,
                refresh: RefreshKind::Tools,
            });
        });
    }

    pub fn restore_vanilla_textures_async(&mut self) {
        let game = self.config.game_path.clone();
        self.busy_label = Some("Validating & restoring vanilla textures...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = restore_vanilla_textures(&game);
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Restore Vanilla Textures".into(),
                result: res,
                refresh: RefreshKind::Tools,
            });
        });
    }

    pub fn replace_practice_music_async(&mut self, audio: PathBuf) {
        let game = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = set_custom_practice_music(&game, &audio);
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Practice Music".into(),
                result: res,
                refresh: RefreshKind::Tools,
            });
        });
    }

    pub fn restore_practice_music_async(&mut self) {
        let game = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = restore_vanilla_practice_music(&game);
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Practice Music".into(),
                result: res,
                refresh: RefreshKind::Tools,
            });
        });
    }

    pub fn replace_death_sfx_async(&mut self, audio: PathBuf) {
        let game = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = set_custom_death_sfx(&game, &audio);
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Custom Death SFX".into(),
                result: res,
                refresh: RefreshKind::Tools,
            });
        });
    }

    pub fn restore_death_sfx_async(&mut self) {
        let game = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = restore_vanilla_death_sfx(&game);
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Custom Death SFX".into(),
                result: res,
                refresh: RefreshKind::Tools,
            });
        });
    }

    pub fn compact_save_bloat_async(&mut self) {
        self.busy_label = Some("Compacting CCLocalLevels.dat...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = clean_save_bloat(&local_gd_data_dir()).map(|r| {
                format!(
                    "Removed {} cached online level(s). Size: {} -> {}",
                    r.removed_levels,
                    crate::launcher::detection::human_bytes(r.old_size_bytes),
                    crate::launcher::detection::human_bytes(r.new_size_bytes)
                )
            });
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Save Compactor".into(),
                result: res,
                refresh: RefreshKind::Backups,
            });
        });
    }

    pub fn create_save_slot_action(&mut self) {
        let name = self.ui_state.new_save_slot_name.trim().to_string();
        match create_save_slot(&local_gd_data_dir(), &name) {
            Ok(msg) => {
                self.ui_state.new_save_slot_name.clear();
                self.push_toast(ToastKind::Success, "Save Slot Created", msg);
                self.refresh_backups_async();
            }
            Err(e) => self.push_toast(ToastKind::Error, "Save Slot Failed", e),
        }
    }

    pub fn switch_save_slot_async(&mut self, slot_name: String) {
        let bdir = self.config.backup_dir.clone();
        self.busy_label = Some(format!("Switching to save slot '{slot_name}'..."));
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = switch_save_slot(
                &local_gd_data_dir(),
                &slot_name,
                Path::new(&bdir),
                &crate::backup::crypto::default_vault_key_path(),
            );
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Save Slot Switched".into(),
                result: res,
                refresh: RefreshKind::Backups,
            });
        });
    }

    pub fn resurrect_save_async(&mut self, candidate: PathBuf) {
        let bdir = self.config.backup_dir.clone();
        self.busy_label = Some("Resurrecting save file...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = resurrect_save(&local_gd_data_dir(), &candidate, Path::new(&bdir));
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Save Doctor".into(),
                result: res,
                refresh: RefreshKind::Backups,
            });
        });
    }

    pub fn export_bundle_async(&mut self, dest: PathBuf) {
        let game = self.config.game_path.clone();
        let inc_s = self.ui_state.bundle_include_saves;
        let inc_m = self.ui_state.bundle_include_mods;
        let inc_c = self.ui_state.bundle_include_configs;
        self.busy_label = Some("Creating portable .geodesync bundle...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res =
                export_geodesync_bundle(&local_gd_data_dir(), &game, &dest, inc_s, inc_m, inc_c);
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Bundle Export".into(),
                result: res,
                refresh: RefreshKind::None,
            });
        });
    }

    pub fn import_bundle_async(&mut self, src: PathBuf) {
        let game = self.config.game_path.clone();
        let bdir = self.config.backup_dir.clone();
        let inc_s = self.ui_state.bundle_include_saves;
        let inc_m = self.ui_state.bundle_include_mods;
        self.busy_label = Some("Importing .geodesync bundle...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = import_geodesync_bundle(
                &src,
                &local_gd_data_dir(),
                &game,
                inc_s,
                inc_m,
                Path::new(&bdir),
            );
            let _ = tx.send(WorkerEvent::GenericOpCompleted {
                title: "Bundle Import".into(),
                result: res,
                refresh: RefreshKind::All,
            });
        });
    }

    fn sync_discord_presence(&self) {
        let ver = self.probe.gd_version.as_deref().unwrap_or("2.2");
        let mods_cnt = self.installed_mods.iter().filter(|m| m.enabled).count();
        if self.active_session.is_some() {
            let stats_tail = self
                .save_summary
                .as_ref()
                .map(|s| {
                    format!(
                        "{}★ · {} Demons · {mods_cnt} Mods",
                        s.player_stats.stars, s.player_stats.demons
                    )
                })
                .unwrap_or_else(|| format!("GD {ver} · {mods_cnt} Active Mods"));
            self.discord_rpc
                .update("Playing Geometry Dash via GeoDeck", stats_tail, false);
        } else {
            let state = format!("GD {ver} · {mods_cnt} Mods · {} Snapshots", self.snapshots.len());
            self.discord_rpc
                .update("GeoDeck · Geometry Dash Companion", state, false);
        }
    }

    pub fn run_integrity_check_async(&mut self) {
        self.busy_label = Some("Verifying GD & Geode files...".into());
        let path = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let report = verify_installation(&path);
            let _ = tx.send(WorkerEvent::IntegrityCompleted(report));
        });
    }

    pub fn refresh_mods_async(&mut self) {
        let game_path = self.config.game_path.clone();
        let gd_ver = self.probe.gd_version.clone();
        let geode_ver = self.probe.geode_version.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let probe = if gd_ver.is_none() {
                Some(probe_installation(&game_path))
            } else {
                None
            };
            let gd = gd_ver.or_else(|| probe.as_ref().and_then(|p| p.gd_version.clone()));
            let geode = geode_ver.or_else(|| probe.as_ref().and_then(|p| p.geode_version.clone()));
            let mods = scan_installed_mods(&game_path);
            let audit = audit_mods(&mods, gd.as_deref(), geode.as_deref());
            let _ = tx.send(WorkerEvent::ModsScanned { mods, audit });
            let kb = scan_keybind_conflicts(&game_path, &local_gd_data_dir());
            let _ = tx.send(WorkerEvent::KeybindsScanned(kb));
        });
    }

    pub fn search_geode_index_async(&mut self) {
        let q = self.ui_state.index_query.clone();
        let sort = self.ui_state.index_sort.clone();
        let gd = if self.ui_state.index_match_gd {
            self.probe.gd_version.clone()
        } else {
            None
        };
        let featured = self.ui_state.index_featured_only;
        let page = self.ui_state.index_page;
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            match search_index_mods(&q, gd.as_deref(), &sort, featured, page, 20) {
                Ok((mods, total_count)) => {
                    let _ = tx.send(WorkerEvent::IndexSearchCompleted {
                        mods,
                        total_count,
                        page,
                    });
                }
                Err(e) => {
                    let _ = tx.send(WorkerEvent::Toast {
                        kind: ToastKind::Error,
                        title: "Geode Index Error".into(),
                        body: e,
                    });
                }
            }
        });
    }

    pub fn check_mod_updates_async(&mut self) {
        self.busy_label = Some("Checking compatible mod updates...".into());
        let mods = self.installed_mods.clone();
        let gd = self.probe.gd_version.clone();
        let geode = self.probe.geode_version.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let updates = check_updates_for_installed(&mods, gd.as_deref(), geode.as_deref());
            let count = updates.len();
            let _ = tx.send(WorkerEvent::ModUpdatesChecked { updates });
            let _ = tx.send(WorkerEvent::Toast {
                kind: if count > 0 {
                    ToastKind::Success
                } else {
                    ToastKind::Info
                },
                title: "Mod Updates".into(),
                body: if count > 0 {
                    format!("Found {count} compatible mod update(s).")
                } else {
                    "All installed mods are up to date for this GD version.".into()
                },
            });
        });
    }

    pub fn install_mod_by_id_async(&mut self, mod_id: String) {
        self.busy_label = Some(format!("Installing {mod_id}..."));
        let game = self.config.game_path.clone();
        let gd = self.probe.gd_version.clone();
        let geode = self.probe.geode_version.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let result = install_mod_from_index(&game, &mod_id, gd.as_deref(), geode.as_deref());
            let _ = tx.send(WorkerEvent::ModInstallFinished { mod_id, result });
        });
    }

    pub fn install_specific_mod_version_async(
        &mut self,
        mod_id: String,
        mod_name: String,
        version: String,
        download_link: String,
        hash: String,
    ) {
        self.busy_label = Some(format!("Updating {mod_name} -> {version}..."));
        let game = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let ver = IndexModVersion {
                name: mod_name,
                description: String::new(),
                version,
                download_link,
                hash,
                geode: String::new(),
                download_count: 0,
                gd_win: None,
                status: "accepted".into(),
            };
            let result = install_mod_version(&game, &mod_id, &ver);
            let _ = tx.send(WorkerEvent::ModInstallFinished { mod_id, result });
        });
    }

    pub fn update_all_compatible_mods_async(&mut self) {
        let updates = self.mod_updates.clone();
        if updates.is_empty() {
            return;
        }
        self.busy_label = Some(format!("Updating {} mod(s)...", updates.len()));
        let game = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let mut ok_count = 0usize;
            for upd in updates {
                let ver = IndexModVersion {
                    name: upd.mod_name,
                    description: String::new(),
                    version: upd.target_version,
                    download_link: upd.download_link,
                    hash: upd.expected_hash,
                    geode: upd.required_geode,
                    download_count: 0,
                    gd_win: None,
                    status: "accepted".into(),
                };
                if install_mod_version(&game, &upd.mod_id, &ver).is_ok() {
                    ok_count += 1;
                }
            }
            let _ = tx.send(WorkerEvent::ModInstallFinished {
                mod_id: "batch".into(),
                result: Ok(format!("Updated {ok_count} mod(s) with SHA-256 verification.")),
            });
        });
    }

    pub fn fetch_latest_loader(&mut self) {
        self.busy_label = Some("Checking latest Geode Loader release...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = fetch_latest_loader_release();
            let _ = tx.send(WorkerEvent::LoaderReleaseFetched(res));
        });
    }

    pub fn install_loader_release_async(&mut self, release: LoaderRelease) {
        self.busy_label = Some(format!("Installing Geode v{}...", release.version));
        let game = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = install_or_update_geode_loader(&game, &release);
            let _ = tx.send(WorkerEvent::LoaderInstallFinished(res));
        });
    }

    pub fn refresh_backups_async(&mut self) {
        let bdir = self.config.backup_dir.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let save_dir = local_gd_data_dir();
            let snaps = list_snapshots(Path::new(&bdir));
            let sum = inspect_save_dir(&save_dir);
            merge_save_metadata_into_cache(&default_song_metadata_cache_path(), &sum);
            let legacy_backups = detect_legacy_external_backups();
            let _ = tx.send(WorkerEvent::BackupsRefreshed {
                snapshots: snaps,
                save_summary: Some(sum),
                legacy_backups,
            });
            let bloat = analyze_save_bloat(&save_dir).ok();
            let slots = list_save_slots(&save_dir);
            let salvage = find_salvageable_saves(&save_dir);
            let _ = tx.send(WorkerEvent::SaveToolsRefreshed {
                bloat,
                slots,
                salvage,
            });
            let songs = scan_song_cache();
            let _ = tx.send(WorkerEvent::SongCacheScanned(songs));
        });
    }

    pub fn trigger_manual_backup(&mut self, reason: &str) {
        self.busy_label = Some("Creating encrypted vault snapshot...".into());
        let bdir = self.config.backup_dir.clone();
        let pol = self.config.backup_policy.clone();
        let rsn = reason.to_string();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = create_snapshot(
                &bdir,
                &rsn,
                pol.encrypt_vault,
                pol.verify_xml_before_backup,
                pol.include_geode_configs,
                pol.retention_count,
            );
            let _ = tx.send(WorkerEvent::BackupCompleted(res));
        });
    }

    pub fn restore_snapshot_async(&mut self, snap_path: String, restore_configs: bool) {
        self.busy_label = Some("Verifying & restoring save vault...".into());
        let bdir = self.config.backup_dir.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = restore_snapshot(&snap_path, &bdir, restore_configs);
            let _ = tx.send(WorkerEvent::RestoreCompleted(res));
        });
    }

    pub fn refresh_tools_async(&mut self) {
        let game = self.config.game_path.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let crashes = scan_crash_logs(&game);
            let _ = tx.send(WorkerEvent::CrashLogsScanned(crashes));
            let songs = scan_song_cache();
            let _ = tx.send(WorkerEvent::SongCacheScanned(songs));
            let assets = get_assets_status(&game);
            let _ = tx.send(WorkerEvent::AssetsStatusUpdated(assets));
            let cache = measure_geode_caches(&game);
            let _ = tx.send(WorkerEvent::CacheSizesMeasured(cache));
        });
    }

    pub fn resolve_missing_song_metadata_async(&mut self, force_retry_unresolved: bool) {
        if self.song_metadata_resolving {
            return;
        }
        let pending_ids: Vec<String> = self
            .song_report
            .songs
            .iter()
            .filter(|s| {
                (s.needs_remote_lookup
                    || (force_retry_unresolved && !s.is_resolved() && s.size_bytes > 0))
                    && !self.in_flight_song_ids.contains(&s.id)
            })
            .map(|s| s.id.clone())
            .collect();

        if pending_ids.is_empty() {
            return;
        }

        self.song_metadata_resolving = true;
        for id in &pending_ids {
            self.in_flight_song_ids.insert(id.clone());
        }
        let ctx_opt = self.egui_ctx.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let (resolved_count, failed_offline) =
                resolve_missing_song_metadata(&pending_ids, force_retry_unresolved, |batch| {
                    let _ = tx.send(WorkerEvent::SongMetadataBatchResolved { entries: batch });
                    if let Some(ref c) = ctx_opt {
                        c.request_repaint();
                    }
                });
            let _ = tx.send(WorkerEvent::SongMetadataResolutionDone {
                resolved_count,
                failed_offline,
            });
        });
    }

    pub fn create_snapshot_slot_async(&mut self) {
        let name = self.ui_state.new_slot_name.trim().to_string();
        if name.is_empty() {
            self.push_toast(
                ToastKind::Warning,
                "Slot Name Required",
                "Enter a name for the new Version Slot.",
            );
            return;
        }
        self.busy_label = Some(format!("Cloning installation into '{name}'..."));
        let src = self.config.game_path.clone();
        let vdir = self.config.versions_dir.clone();
        let inc = self.ui_state.clone_include_mods;
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            match create_isolated_snapshot(&src, &vdir, &name, inc) {
                Ok(inst) => {
                    let json = serde_json::to_string(&inst).unwrap_or_default();
                    let _ = tx.send(WorkerEvent::SnapshotSlotCreated(Ok((inst.name, json))));
                }
                Err(e) => {
                    let _ = tx.send(WorkerEvent::SnapshotSlotCreated(Err(e)));
                }
            }
        });
    }

    pub fn import_depot_slot_async(&mut self, depot_dir: PathBuf, slot_name: String) {
        self.busy_label = Some(format!("Importing Steam depot into '{slot_name}'..."));
        let vdir = self.config.versions_dir.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            match import_depot_as_slot(&depot_dir, &vdir, &slot_name) {
                Ok(inst) => {
                    let json = serde_json::to_string(&inst).unwrap_or_default();
                    let _ = tx.send(WorkerEvent::SnapshotSlotCreated(Ok((inst.name, json))));
                }
                Err(e) => {
                    let _ = tx.send(WorkerEvent::SnapshotSlotCreated(Err(e)));
                }
            }
        });
    }

    pub fn export_modpack_async(&mut self) {
        let pack_name = self.ui_state.export_pack_name.clone();
        let game = self.config.game_path.clone();
        let dest = self.config.export_dir.clone();
        let only_enabled = self.ui_state.export_only_enabled;
        let inc_cfg = self.ui_state.export_include_configs;
        let gd = self.probe.gd_version.clone().unwrap_or_else(|| "unknown".into());
        let geode = self
            .probe
            .geode_version
            .clone()
            .unwrap_or_else(|| "unknown".into());
        let indices: Vec<usize> = self
            .installed_mods
            .iter()
            .enumerate()
            .filter(|(_, m)| !only_enabled || m.enabled)
            .map(|(i, _)| i)
            .collect();

        self.busy_label = Some("Packaging portable .gdpack.zip...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = export_gdpack(&pack_name, &game, &dest, &indices, inc_cfg, &gd, &geode)
                .map(|p| p.to_string_lossy().into_owned());
            let _ = tx.send(WorkerEvent::ModPackExported(res));
        });
    }

    pub fn import_modpack_async(&mut self, archive: PathBuf) {
        let game = self.config.game_path.clone();
        self.busy_label = Some("Verifying & importing .gdpack.zip...".into());
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = import_gdpack(&archive, &game, true).map(|r| {
                format!(
                    "Imported '{}' ({} mods, {} configs)",
                    r.pack_name, r.imported_mods, r.restored_configs
                )
            });
            let _ = tx.send(WorkerEvent::ModPackImported(res));
        });
    }

    pub fn check_app_update_async(&mut self) {
        let repo = self.config.update_repo.clone();
        self.busy_label = Some("Checking GitHub releases...".into());
        self.update_banner_dismissed = false;
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = check_github_updates(&repo);
            let _ = tx.send(WorkerEvent::AppUpdateChecked(res));
        });
    }

    pub fn check_app_update_silent_async(&mut self) {
        let repo = self.config.update_repo.clone();
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = check_github_updates(&repo);
            let _ = tx.send(WorkerEvent::AppUpdateChecked(res));
        });
    }

    pub fn download_app_update_async(
        &mut self,
        url: String,
        name: String,
        expected_sha: Option<String>,
    ) {
        self.busy_label = Some(format!("Downloading & verifying {name}..."));
        spawn_job(self.bus.sender(), self.egui_ctx.clone(), move |tx| {
            let res = download_and_verify_update(&url, &name, expected_sha.as_deref())
                .map(|p| p.to_string_lossy().into_owned());
            let _ = tx.send(WorkerEvent::AppUpdateDownloaded(res));
        });
    }

    pub fn switch_active_installation(&mut self, id: &str) {
        if self.config.backup_policy.auto_pre_launch {
            let _ = run_auto_backup_if_changed(
                &self.config.backup_dir,
                &mut self.config.backup_policy,
                "Pre-Switch Safety Point",
            );
        }
        self.config.set_active_installation(id);
        let pinned_profile = self
            .config
            .active_installation()
            .and_then(|i| i.pinned_mod_profile.clone());
        if let Some(prof_id) = pinned_profile {
            if let Some(prof) = self
                .config
                .mod_profiles
                .iter()
                .find(|p| p.id == prof_id)
                .cloned()
            {
                let _ = apply_mod_profile(&prof, &self.config.game_path, true);
            }
        }
        self.config.save();
        self.logo_textures.clear();
        self.integrity_report = IntegrityReport::default();
        self.apply_max_performance_async();
        self.refresh_probe_async();
        self.refresh_mods_async();
        self.refresh_tools_async();
        self.push_toast(
            ToastKind::Success,
            "Installation Switched",
            format!("Active slot: {}", self.config.active_version),
        );
    }

    pub fn trigger_launch(&mut self, mode: LaunchMode) {
        if self.config.backup_policy.auto_pre_launch {
            if let Ok(Some(snap)) = run_auto_backup_if_changed(
                &self.config.backup_dir,
                &mut self.config.backup_policy,
                "Auto Pre-Launch Backup",
            ) {
                logging::ok("Backup", format!("Pre-launch snapshot created: {}", snap.id));
            }
        }

        let inst_name = self.config.active_version.clone();
        let game_path = self.config.game_path.clone();
        let custom_args = self
            .config
            .active_installation()
            .map(|i| i.custom_args.clone())
            .unwrap_or_default();

        match launch_game(
            &inst_name,
            &game_path,
            mode,
            &self.config.geode_flags,
            &custom_args,
            self.config.applied_tweaks.cpu_priority_high,
            self.config.applied_tweaks.cpu_affinity_perf,
            self.bus.sender(),
            self.egui_ctx.clone(),
        ) {
            Ok(session) => {
                self.config.last_played = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
                self.config.save();
                self.active_session = Some(session);
                self.sync_discord_presence();
                self.push_toast(
                    ToastKind::Success,
                    "Geometry Dash Launched",
                    format!("Running {} ({})", inst_name, mode.label()),
                );
            }
            Err(e) => {
                self.push_toast(ToastKind::Error, "Launch Failed", e);
            }
        }
    }

    pub fn stop_running_game(&mut self) {
        match terminate_gd_process() {
            Ok(_) => {
                self.active_session = None;
                self.sync_discord_presence();
                self.push_toast(
                    ToastKind::Info,
                    "Process Terminated",
                    "Stopped Geometry Dash.",
                );
            }
            Err(e) => self.push_toast(ToastKind::Warning, "Stop Failed", e),
        }
    }

    fn process_worker_events(&mut self) {
        for ev in self.bus.poll() {
            match ev {
                WorkerEvent::Toast { kind, title, body } => {
                    self.push_toast(kind, title, body);
                }
                WorkerEvent::ProbeCompleted(probe) => {
                    if let Some(inst) = self.config.active_installation_mut() {
                        inst.detected_gd_version = probe.gd_version.clone();
                        inst.detected_geode_version = probe.geode_version.clone();
                    }
                    self.config.save();
                    self.probe = probe;
                    self.busy_label = None;
                    self.initial_sync_complete = true;
                }
                WorkerEvent::IntegrityCompleted(rep) => {
                    self.busy_label = None;
                    let ok = rep.overall_ok;
                    self.integrity_report = rep;
                    self.push_toast(
                        if ok {
                            ToastKind::Success
                        } else {
                            ToastKind::Warning
                        },
                        "Integrity Check Complete",
                        if ok {
                            "All core binaries and resources verified."
                        } else {
                            "Missing or corrupted files detected."
                        },
                    );
                }
                WorkerEvent::ModsScanned { mods, audit } => {
                    self.installed_mods = mods;
                    self.mod_audit = audit;
                    self.busy_label = None;
                    self.sync_discord_presence();
                }
                WorkerEvent::IndexSearchCompleted {
                    mods,
                    total_count,
                    page,
                } => {
                    self.index_results = mods;
                    self.index_total = total_count;
                    self.ui_state.index_page = page;
                    self.busy_label = None;
                }
                WorkerEvent::ModUpdatesChecked { updates } => {
                    self.mod_updates = updates;
                    self.busy_label = None;
                }
                WorkerEvent::LoaderReleaseFetched(res) => {
                    self.busy_label = None;
                    match res {
                        Ok(rel) => self.latest_loader_release = Some(rel),
                        Err(e) => self.push_toast(ToastKind::Error, "Loader Check Failed", e),
                    }
                }
                WorkerEvent::BackupsRefreshed {
                    snapshots,
                    save_summary,
                    legacy_backups,
                } => {
                    self.snapshots = snapshots;
                    self.save_summary = save_summary;
                    self.legacy_backups = legacy_backups;
                    self.busy_label = None;
                    self.initial_sync_complete = true;
                    self.sync_discord_presence();
                }
                WorkerEvent::SaveToolsRefreshed {
                    bloat,
                    slots,
                    salvage,
                } => {
                    self.save_bloat = bloat;
                    self.save_slots = slots;
                    self.salvage_candidates = salvage;
                }
                WorkerEvent::DemonlistFetched(res) => {
                    self.demonlist_loading = false;
                    match res {
                        Ok(list) => {
                            self.demonlist = list;
                            self.demonlist_error = None;
                        }
                        Err(e) => {
                            self.demonlist_error = Some(e);
                        }
                    }
                }
                WorkerEvent::HealthCheckCompleted(findings) => {
                    self.health_checking = false;
                    self.health_findings = findings;
                }
                WorkerEvent::KeybindsScanned(rep) => {
                    self.keybind_report = rep;
                }
                WorkerEvent::AssetsStatusUpdated(st) => {
                    self.assets_status = st;
                }
                WorkerEvent::CacheSizesMeasured(sz) => {
                    self.cache_sizes = sz;
                }
                WorkerEvent::InstallsScanned(found) => {
                    self.discovered_installs = found;
                }
                WorkerEvent::GenericOpCompleted {
                    title,
                    result,
                    refresh,
                } => {
                    self.busy_label = None;
                    match result {
                        Ok(msg) => self.push_toast(ToastKind::Success, title, msg),
                        Err(e) => self.push_toast(ToastKind::Error, title, e),
                    }
                    match refresh {
                        RefreshKind::None => {}
                        RefreshKind::Backups => self.refresh_backups_async(),
                        RefreshKind::Mods => self.refresh_mods_async(),
                        RefreshKind::Tools => {
                            self.refresh_tools_async();
                            self.run_health_check_async();
                        }
                        RefreshKind::Probe => self.refresh_probe_async(),
                        RefreshKind::All => self.refresh_all_async(),
                    }
                }
                WorkerEvent::MaxPerfApplied(res) => {
                    self.max_perf_applied = true;
                    match res {
                        Ok(msg) => logging::ok("Performance", msg),
                        Err(e) => logging::warn("Performance", format!("Auto-tuning skipped: {e}")),
                    }
                }
                WorkerEvent::CrashLogsScanned(list) => {
                    self.crash_reports = list;
                }
                WorkerEvent::SongCacheScanned(rep) => {
                    self.song_report = rep;
                    if self.save_summary.is_some() {
                        self.resolve_missing_song_metadata_async(false);
                    }
                }
                WorkerEvent::SongMetadataBatchResolved { entries } => {
                    apply_metadata_to_report(&mut self.song_report, &entries);
                }
                WorkerEvent::SongMetadataResolutionDone {
                    resolved_count,
                    failed_offline,
                } => {
                    self.song_metadata_resolving = false;
                    self.in_flight_song_ids.clear();
                    if resolved_count > 0 {
                        logging::ok(
                            "Songs",
                            format!("Resolved and cached metadata for {resolved_count} track(s)."),
                        );
                    } else if failed_offline {
                        logging::warn(
                            "Songs",
                            "Song metadata API unreachable; showing cached local names.",
                        );
                    }
                }
                WorkerEvent::AppUpdateChecked(res) => {
                    self.busy_label = None;
                    if let Ok(ref info) = res {
                        if info.update_available {
                            self.push_toast(
                                ToastKind::Info,
                                "GeoDeck Update Available",
                                format!(
                                    "New release {} is available (current: {}).",
                                    info.latest_version, info.current_version
                                ),
                            );
                        }
                    }
                    self.app_update_result = Some(res);
                }
                WorkerEvent::AppUpdateDownloaded(res) => {
                    self.busy_label = None;
                    match res {
                        Ok(path) => self.push_toast(
                            ToastKind::Success,
                            "Verified Update Staged",
                            format!("Downloaded SHA-256 verified package to {path}"),
                        ),
                        Err(e) => self.push_toast(ToastKind::Error, "Update Download Failed", e),
                    }
                }
                WorkerEvent::BackupCompleted(res) => {
                    self.busy_label = None;
                    match res {
                        Ok(s) => {
                            self.config.backup_policy.last_backup_at = s.created_at.clone();
                            self.config.save();
                            self.push_toast(
                                ToastKind::Success,
                                "Backup Created",
                                format!("Snapshot {} ({} files)", s.id, s.file_count),
                            );
                            self.refresh_backups_async();
                        }
                        Err(e) => self.push_toast(ToastKind::Error, "Backup Failed", e),
                    }
                }
                WorkerEvent::RestoreCompleted(res) => {
                    self.busy_label = None;
                    match res {
                        Ok(msg) => {
                            self.push_toast(ToastKind::Success, "Vault Restored", msg);
                            self.refresh_backups_async();
                            self.refresh_mods_async();
                        }
                        Err(e) => self.push_toast(ToastKind::Error, "Restore Failed", e),
                    }
                }
                WorkerEvent::SnapshotSlotCreated(res) => {
                    self.busy_label = None;
                    match res {
                        Ok((name, json)) => {
                            if let Ok(inst) = serde_json::from_str(&json) {
                                self.config.installations.push(inst);
                                self.config.save();
                                self.push_toast(
                                    ToastKind::Success,
                                    "Version Slot Created",
                                    format!("Isolated slot '{name}' is ready."),
                                );
                            }
                        }
                        Err(e) => self.push_toast(ToastKind::Error, "Slot Creation Failed", e),
                    }
                }
                WorkerEvent::ModPackExported(res) => {
                    self.busy_label = None;
                    match res {
                        Ok(path) => self.push_toast(
                            ToastKind::Success,
                            "Mod Pack Exported",
                            format!("Saved portable .gdpack.zip to {path}"),
                        ),
                        Err(e) => self.push_toast(ToastKind::Error, "Export Failed", e),
                    }
                }
                WorkerEvent::ModPackImported(res) => {
                    self.busy_label = None;
                    match res {
                        Ok(msg) => {
                            self.push_toast(ToastKind::Success, "Mod Pack Imported", msg);
                            self.refresh_mods_async();
                        }
                        Err(e) => self.push_toast(ToastKind::Error, "Import Failed", e),
                    }
                }
                WorkerEvent::ModInstallFinished { mod_id, result } => {
                    self.busy_label = None;
                    match result {
                        Ok(label) => {
                            self.push_toast(
                                ToastKind::Success,
                                "Mod Installed",
                                format!("Verified & installed {label}"),
                            );
                            self.mod_updates.retain(|u| u.mod_id != mod_id);
                            self.refresh_mods_async();
                        }
                        Err(e) => self.push_toast(ToastKind::Error, "Mod Install Failed", e),
                    }
                }
                WorkerEvent::LoaderInstallFinished(res) => {
                    self.busy_label = None;
                    match res {
                        Ok(msg) => {
                            self.push_toast(ToastKind::Success, "Geode Loader Installed", msg);
                            self.refresh_probe_async();
                            self.refresh_mods_async();
                        }
                        Err(e) => self.push_toast(ToastKind::Error, "Loader Install Failed", e),
                    }
                }
                WorkerEvent::ModLogoFetched { .. } => {}
                WorkerEvent::GameExited {
                    exit_code,
                    duration_secs,
                    new_crashes,
                } => {
                    self.active_session = None;
                    self.config.playtime_secs += duration_secs;
                    self.config.save();
                    self.sync_discord_presence();
                    logging::info(
                        "Session",
                        format!(
                            "Geometry Dash exited after {}s (code: {:?}, new crashes: {})",
                            duration_secs, exit_code, new_crashes
                        ),
                    );
                    if new_crashes > 0 {
                        self.push_toast(
                            ToastKind::Warning,
                            "Crash Detected",
                            format!(
                                "{new_crashes} new Geode crash log(s) created. Inspect in Tools."
                            ),
                        );
                        self.refresh_tools_async();
                    }
                    if self.config.backup_policy.auto_on_exit {
                        if let Ok(Some(snap)) = run_auto_backup_if_changed(
                            &self.config.backup_dir,
                            &mut self.config.backup_policy,
                            "Auto Post-Exit Backup",
                        ) {
                            self.config.save();
                            self.push_toast(
                                ToastKind::Success,
                                "Auto-Backup Saved",
                                format!("Protected session progress in {}", snap.id),
                            );
                            self.refresh_backups_async();
                        }
                    }
                }
            }
        }
    }
}

impl eframe::App for GeodeDeckApp {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.discord_rpc.shutdown();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_worker_events();

        // === Universal Keyboard Shortcuts (Ctrl+1..7, Ctrl+F, Ctrl+H, F5) ===
        let (nav_shortcut, do_focus_search, do_health, do_refresh) = ctx.input(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.command;
            let nav = if ctrl && i.key_pressed(egui::Key::Num1) {
                Some(NavTab::Home)
            } else if ctrl && i.key_pressed(egui::Key::Num2) {
                Some(NavTab::Library)
            } else if ctrl && i.key_pressed(egui::Key::Num3) {
                Some(NavTab::Mods)
            } else if ctrl && i.key_pressed(egui::Key::Num4) {
                Some(NavTab::Demonlist)
            } else if ctrl && i.key_pressed(egui::Key::Num5) {
                Some(NavTab::Backups)
            } else if ctrl && i.key_pressed(egui::Key::Num6) {
                Some(NavTab::Tools)
            } else if ctrl && i.key_pressed(egui::Key::Num7) {
                Some(NavTab::Settings)
            } else {
                None
            };
            (
                nav,
                ctrl && i.key_pressed(egui::Key::F),
                ctrl && i.key_pressed(egui::Key::H),
                i.key_pressed(egui::Key::F5),
            )
        });
        if let Some(target) = nav_shortcut {
            self.navigate_to(target);
        }
        if do_focus_search {
            self.ui_state.focus_search_requested = true;
        }
        if do_health {
            self.ui_state.tools_subtab = 0;
            self.navigate_to(NavTab::Tools);
            self.run_health_check_async();
        }
        if do_refresh {
            self.refresh_all_async();
        }

        // Check for screenshot capture mode completion
        if let Some(ref out_path) = self.capture_output.clone() {
            let ready_delay = if self.initial_sync_complete
                && (!self.installed_mods.is_empty() || self.startup_time.elapsed() >= Duration::from_millis(2400))
            {
                Duration::from_millis(1200)
            } else {
                Duration::from_millis(3200)
            };
            let demon_ready = self.active_tab != NavTab::Demonlist
                || (!self.demonlist_loading
                    && (!self.demonlist.is_empty() || self.demonlist_error.is_some()))
                || self.capture_started_at.elapsed() >= Duration::from_secs(6);
            if !self.capture_requested
                && demon_ready
                && self.capture_started_at.elapsed() >= ready_delay
            {
                self.capture_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            let screenshot_img = ctx.input(|i| {
                i.raw.events.iter().find_map(|ev| {
                    if let egui::Event::Screenshot { image, .. } = ev {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(img) = screenshot_img {
                if let Some(parent) = out_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let raw_rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Some(buf) = image::RgbaImage::from_raw(
                    img.size[0] as u32,
                    img.size[1] as u32,
                    raw_rgba,
                ) {
                    let _ = buf.save(out_path);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if self.capture_started_at.elapsed() >= Duration::from_secs(8) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(Duration::from_millis(80));
        }

        if self.last_backup_tick.elapsed() >= Duration::from_secs(5) {
            self.last_backup_tick = Instant::now();
            if self
                .backup_scheduler
                .should_run_periodic(&self.config.backup_policy)
            {
                self.backup_scheduler.mark_tick();
                if let Ok(Some(snap)) = run_auto_backup_if_changed(
                    &self.config.backup_dir,
                    &mut self.config.backup_policy,
                    "Scheduled Auto-Backup",
                ) {
                    self.config.save();
                    self.push_toast(
                        ToastKind::Success,
                        "Scheduled Backup",
                        format!("Saved snapshot {}", snap.id),
                    );
                    self.refresh_backups_async();
                }
            }
        }

        self.toasts
            .retain(|t| t.created_at.elapsed() < Duration::from_secs(5));

        let show_loading = !self.initial_sync_complete
            && self.startup_time.elapsed() < Duration::from_millis(900)
            && self.capture_output.is_none();

        if self.active_session.is_some()
            || self.busy_label.is_some()
            || self.song_metadata_resolving
            || show_loading
        {
            ctx.request_repaint_after(Duration::from_millis(180));
        } else {
            ctx.request_repaint_after(Duration::from_secs(2));
        }

        // === Custom Frameless Title Bar with Brand Icon ===
        egui::TopBottomPanel::top("epic_window_chrome")
            .exact_height(34.0)
            .frame(
                egui::Frame::none()
                    .fill(BG_SIDEBAR)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .inner_margin(Margin::symmetric(12.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(ref icon_tex) = self.brand_icon {
                        ui.add(
                            egui::Image::new(icon_tex)
                                .fit_to_exact_size(Vec2::splat(20.0))
                                .rounding(4.0),
                        );
                    }
                    ui.label(semibold_text("GeoDeck", 12.5, TEXT_PRIMARY));
                    let ver_meta = if let Some(ref gd) = self.probe.gd_version {
                        format!("{}  ·  GD {gd}", current_app_display_version())
                    } else {
                        current_app_display_version().to_string()
                    };
                    ui.label(mono_text(ver_meta, 11.0, TEXT_MUTED));

                    let drag_w = (ui.available_width() - 108.0).max(80.0);
                    let (_, drag_resp) =
                        ui.allocate_exact_size(Vec2::new(drag_w, 24.0), egui::Sense::drag());
                    if drag_resp.drag_started_by(egui::PointerButton::Primary) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    if drag_resp.double_clicked() {
                        let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let close_btn = egui::Button::new(
                            RichText::new(ph::X).size(12.5).color(TEXT_SECONDARY),
                        )
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .rounding(Rounding::same(6.0))
                        .min_size(Vec2::new(28.0, 24.0));
                        let close_resp = ui.add(close_btn).on_hover_text("Close");
                        if close_resp.hovered() {
                            ui.painter().rect_filled(
                                close_resp.rect,
                                Rounding::same(6.0),
                                STATUS_ERR_SOFT,
                            );
                            ui.painter().text(
                                close_resp.rect.center(),
                                egui::Align2::CENTER_CENTER,
                                ph::X,
                                egui::FontId::proportional(12.5),
                                STATUS_ERR,
                            );
                        }
                        if close_resp.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(ph::SQUARE).size(11.5).color(TEXT_SECONDARY),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(28.0, 24.0)),
                            )
                            .on_hover_text(if is_max { "Restore" } else { "Maximize" })
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                        }

                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(ph::MINUS).size(12.5).color(TEXT_SECONDARY),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .rounding(Rounding::same(6.0))
                                .min_size(Vec2::new(28.0, 24.0)),
                            )
                            .on_hover_text("Minimize")
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });

        // === Sidebar Navigation with Custom GeoDeck Orbital Purple Logo ===
        egui::SidePanel::left("epic_sidebar")
            .resizable(false)
            .exact_width(204.0)
            .frame(
                egui::Frame::none()
                    .fill(BG_SIDEBAR)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .inner_margin(Margin {
                        left: 12.0,
                        right: 12.0,
                        top: 10.0,
                        bottom: 12.0,
                    }),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if let Some(ref lockup) = self.brand_lockup {
                        let resp = ui
                            .add(
                                egui::Image::new(lockup)
                                    .fit_to_exact_size(Vec2::new(146.0, 51.0))
                                    .sense(egui::Sense::click()),
                            )
                            .on_hover_text("GeoDeck Home");
                        if resp.clicked() {
                            self.navigate_to(NavTab::Home);
                        }
                    } else {
                        ui.label(bold_text("GeoDeck", 20.0, TEXT_PRIMARY));
                    }
                });
                ui.add_space(8.0);
                row_divider(ui);
                ui.add_space(6.0);

                let nav_items = [
                    (NavTab::Home, ph::HOUSE, "Home", 0usize),
                    (NavTab::Library, ph::SQUARES_FOUR, "Installs", 0usize),
                    (NavTab::Mods, ph::PUZZLE_PIECE, "Mods", self.mod_updates.len()),
                    (NavTab::Demonlist, ph::STAR, "Demonlist", 0usize),
                    (NavTab::Backups, ph::SHIELD_CHECK, "Backups", 0usize),
                    (NavTab::Tools, ph::WRENCH, "Tools", self.crash_reports.len()),
                ];

                let mut pending_nav: Option<NavTab> = None;
                for (tab, icon, label, count_badge) in nav_items {
                    let selected = self.active_tab == tab;
                    let fill = if selected {
                        BG_ELEVATED
                    } else {
                        Color32::TRANSPARENT
                    };
                    let fg = if selected {
                        TEXT_PRIMARY
                    } else {
                        TEXT_SECONDARY
                    };
                    let icon_col = if selected { ACCENT_EPIC_BRIGHT } else { fg };

                    let bg_slot = ui.painter().add(egui::Shape::Noop);
                    let resp = egui::Frame::none()
                        .fill(fill)
                        .rounding(Rounding::same(RADIUS_BUTTON))
                        .inner_margin(Margin::symmetric(11.0, 7.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(icon).size(15.0).color(icon_col));
                                ui.add_space(6.0);
                                ui.label(semibold_text(label, 13.0, fg));
                                if count_badge > 0 {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(mono_text(
                                                count_badge.to_string(),
                                                11.0,
                                                ACCENT_EPIC_BRIGHT,
                                            ));
                                        },
                                    );
                                }
                            });
                        })
                        .response
                        .interact(egui::Sense::click());

                    if selected {
                        let bar = egui::Rect::from_min_size(
                            egui::pos2(resp.rect.left() + 1.0, resp.rect.top() + 6.0),
                            Vec2::new(2.0, resp.rect.height() - 12.0),
                        );
                        ui.painter()
                            .rect_filled(bar, Rounding::same(1.0), ACCENT_EPIC);
                    } else if resp.hovered() {
                        ui.painter().set(
                            bg_slot,
                            egui::Shape::rect_filled(
                                resp.rect,
                                Rounding::same(RADIUS_BUTTON),
                                BG_HOVER,
                            ),
                        );
                    }
                    if resp.clicked() {
                        pending_nav = Some(tab);
                    }
                    ui.add_space(2.0);
                }
                if let Some(t) = pending_nav {
                    self.navigate_to(t);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(mono_text(
                            format!("GeoDeck {}", current_app_display_version()),
                            10.5,
                            TEXT_MUTED,
                        ));
                    });
                    ui.add_space(4.0);

                    let set_sel = self.active_tab == NavTab::Settings;
                    let set_fg = if set_sel { TEXT_PRIMARY } else { TEXT_SECONDARY };
                    let set_bg_slot = ui.painter().add(egui::Shape::Noop);
                    let set_resp = egui::Frame::none()
                        .fill(if set_sel {
                            BG_ELEVATED
                        } else {
                            Color32::TRANSPARENT
                        })
                        .rounding(Rounding::same(RADIUS_BUTTON))
                        .inner_margin(Margin::symmetric(11.0, 7.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(ph::GEAR_SIX).size(15.0).color(if set_sel {
                                    ACCENT_EPIC_BRIGHT
                                } else {
                                    set_fg
                                }));
                                ui.add_space(6.0);
                                ui.label(semibold_text("Settings & About", 13.0, set_fg));
                            });
                        })
                        .response
                        .interact(egui::Sense::click());
                    if set_sel {
                        let bar = egui::Rect::from_min_size(
                            egui::pos2(set_resp.rect.left() + 1.0, set_resp.rect.top() + 6.0),
                            Vec2::new(2.0, set_resp.rect.height() - 12.0),
                        );
                        ui.painter()
                            .rect_filled(bar, Rounding::same(1.0), ACCENT_EPIC);
                    } else if set_resp.hovered() {
                        ui.painter().set(
                            set_bg_slot,
                            egui::Shape::rect_filled(
                                set_resp.rect,
                                Rounding::same(RADIUS_BUTTON),
                                BG_HOVER,
                            ),
                        );
                    }
                    if set_resp.clicked() {
                        self.navigate_to(NavTab::Settings);
                    }

                    if let Some(busy) = self
                        .busy_label
                        .clone()
                        .or_else(|| self.song_metadata_resolving.then(|| "Resolving song metadata...".into()))
                    {
                        ui.add_space(6.0);
                        inset_frame().show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(busy).size(11.5).color(TEXT_SECONDARY),
                                    )
                                    .truncate(),
                                );
                            });
                        });
                    }
                });
            });

        // === Top Content Toolbar ===
        egui::TopBottomPanel::top("content_toolbar")
            .exact_height(46.0)
            .frame(
                egui::Frame::none()
                    .fill(BG_VOID)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .inner_margin(Margin {
                        left: 22.0,
                        right: 22.0,
                        top: 7.0,
                        bottom: 7.0,
                    }),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let can_back = !self.nav_back_stack.is_empty();
                    let can_fwd = !self.nav_forward_stack.is_empty();
                    if ui
                        .add_enabled(can_back, ghost_icon_button(ph::ARROW_LEFT))
                        .on_hover_text("Back")
                        .clicked()
                    {
                        self.navigate_back();
                    }
                    if ui
                        .add_enabled(can_fwd, ghost_icon_button(ph::ARROW_RIGHT))
                        .on_hover_text("Forward")
                        .clicked()
                    {
                        self.navigate_forward();
                    }
                    ui.add_space(6.0);
                    ui.label(display_text(
                        match self.active_tab {
                            NavTab::Home => "Home",
                            NavTab::Library => "Installs",
                            NavTab::Mods => "Mods",
                            NavTab::Demonlist => "Demonlist & Progress",
                            NavTab::Backups => "Backups",
                            NavTab::Tools => "Tools",
                            NavTab::Settings => "Settings",
                        },
                        16.0,
                        TEXT_PRIMARY,
                    ));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        if ui
                            .add(ghost_icon_button(ph::ARROWS_CLOCKWISE))
                            .on_hover_text("Refresh workspace (F5)")
                            .clicked()
                        {
                            self.refresh_all_async();
                        }
                        let slot_name = self
                            .config
                            .active_installation()
                            .map(|i| i.name.as_str())
                            .unwrap_or("No Slot");
                        let geode_tag = if self.probe.geode_installed {
                            self.probe
                                .geode_version
                                .as_deref()
                                .map(|v| format!("Geode {v}"))
                                .unwrap_or_else(|| "Geode Ready".into())
                        } else {
                            "No Geode".into()
                        };
                        ui.label(mono_text(
                            format!("{slot_name}  ·  {geode_tag}"),
                            11.0,
                            TEXT_MUTED,
                        ));
                    });
                });
            });

        // === Central Scrollable Content ===
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(BG_VOID)
                    .inner_margin(Margin {
                        left: 22.0,
                        right: 22.0,
                        top: 16.0,
                        bottom: 18.0,
                    }),
            )
            .show(ctx, |ui| {
                if !self.update_banner_dismissed {
                    if let Some(Ok(info)) = self.app_update_result.clone() {
                        if info.update_available {
                            let banner = egui::Frame::none()
                                .fill(BG_ELEVATED)
                                .stroke(Stroke::new(1.0_f32, BORDER_STRONG))
                                .rounding(Rounding::same(RADIUS_CARD))
                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new(ph::CLOUD_ARROW_DOWN)
                                                .size(16.0)
                                                .color(ACCENT_EPIC_BRIGHT),
                                        );
                                        ui.label(bold_text(
                                            &format!(
                                                "GeoDeck {} is available",
                                                info.latest_version
                                            ),
                                            13.0,
                                            TEXT_PRIMARY,
                                        ));
                                        ui.label(mono_text(
                                            format!("current: {}", info.current_version),
                                            11.5,
                                            TEXT_MUTED,
                                        ));
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .add(ghost_icon_button(ph::X))
                                                    .on_hover_text("Dismiss")
                                                    .clicked()
                                                {
                                                    self.update_banner_dismissed = true;
                                                }
                                                if let (Some(url), Some(name)) =
                                                    (&info.asset_url, &info.asset_name)
                                                {
                                                    if ui
                                                        .add(brand_cta_button(
                                                            ph::DOWNLOAD_SIMPLE,
                                                            "Download Update",
                                                        ))
                                                        .clicked()
                                                    {
                                                        self.download_app_update_async(
                                                            url.clone(),
                                                            name.clone(),
                                                            info.expected_sha256.clone(),
                                                        );
                                                    }
                                                } else if ui
                                                    .add(epic_secondary_button(
                                                        ph::GLOBE,
                                                        "View Release",
                                                    ))
                                                    .clicked()
                                                {
                                                    open_url(&info.release_url);
                                                }
                                            },
                                        );
                                    });
                                });
                            let rail = egui::Rect::from_min_size(
                                egui::pos2(banner.response.rect.left() + 1.0, banner.response.rect.top() + 6.0),
                                Vec2::new(2.0, banner.response.rect.height() - 12.0),
                            );
                            ui.painter()
                                .rect_filled(rail, Rounding::same(1.0), ACCENT_EPIC);
                            ui.add_space(12.0);
                        }
                    }
                }

                egui::ScrollArea::vertical()
                    .id_salt(("tab_scroll", self.active_tab as u8))
                    .show(ui, |ui| match self.active_tab {
                    NavTab::Home => views::home::render(self, ui),
                    NavTab::Library => views::library::render(self, ui),
                    NavTab::Mods => views::mods::render(self, ui),
                    NavTab::Demonlist => views::demonlist::render(self, ui),
                    NavTab::Backups => views::backups::render(self, ui),
                    NavTab::Tools => views::tools::render(self, ui),
                    NavTab::Settings => views::settings::render(self, ui),
                });
            });

        // === Startup Branded Loading Overlay ===
        if show_loading {
            egui::Area::new(egui::Id::new("geodeck_startup_splash"))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::Pos2::ZERO)
                .show(ctx, |ui| {
                    let screen = ctx.screen_rect();
                    ui.allocate_rect(screen, egui::Sense::click());
                    ui.painter().rect_filled(screen, Rounding::ZERO, BG_VOID);

                    egui::Area::new(egui::Id::new("geodeck_startup_splash_card"))
                        .order(egui::Order::Tooltip)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            dialog_frame().show(ui, |ui| {
                                ui.set_min_width(340.0);
                                ui.vertical_centered(|ui| {
                                    if let Some(ref lockup) = self.brand_lockup {
                                        ui.add(
                                            egui::Image::new(lockup)
                                                .fit_to_exact_size(Vec2::new(220.0, 77.0)),
                                        );
                                    }
                                    ui.add_space(12.0);
                                    ui.horizontal(|ui| {
                                        ui.add_space(48.0);
                                        ui.spinner();
                                        ui.label(semibold_text(
                                            "Preparing your Geometry Dash workspace...",
                                            13.0,
                                            TEXT_SECONDARY,
                                        ));
                                    });
                                });
                            });
                        });
                });
        }

        // Bottom-right resize corner for frameless window
        egui::Area::new(egui::Id::new("frameless_resize_grip"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [0.0, 0.0])
            .show(ctx, |ui| {
                let (_, resp) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::drag());
                if resp.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(
                        egui::ResizeDirection::SouthEast,
                    ));
                }
            });

        // === FLOATING TOAST STACK ===
        if !self.toasts.is_empty() {
            let mut dismiss_idx: Option<usize> = None;
            egui::Area::new(egui::Id::new("geodedeck_toasts"))
                .order(egui::Order::Tooltip)
                .anchor(egui::Align2::RIGHT_BOTTOM, [-20.0, -20.0])
                .show(ctx, |ui| {
                    ui.set_max_width(370.0);
                    for (idx, toast) in self.toasts.iter().enumerate().rev() {
                        let (accent, icon) = match toast.kind {
                            ToastKind::Success => (STATUS_OK, ph::CHECK_CIRCLE),
                            ToastKind::Warning => (STATUS_WARN, ph::WARNING),
                            ToastKind::Error => (STATUS_ERR, ph::X_CIRCLE),
                            ToastKind::Info => (ACCENT_EPIC_BRIGHT, ph::INFO),
                        };
                        let frame_resp = egui::Frame::none()
                            .fill(BG_ELEVATED)
                            .stroke(Stroke::new(1.0_f32, BORDER_STRONG))
                            .rounding(Rounding::same(RADIUS_CARD))
                            .inner_margin(Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(icon).size(16.0).color(accent));
                                    ui.vertical(|ui| {
                                        ui.set_max_width(280.0);
                                        ui.label(bold_text(&toast.title, 13.0, TEXT_PRIMARY));
                                        ui.label(
                                            RichText::new(&toast.body)
                                                .size(12.0)
                                                .color(TEXT_SECONDARY),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::TOP),
                                        |ui| {
                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new(ph::X)
                                                            .size(12.0)
                                                            .color(TEXT_MUTED),
                                                    )
                                                    .frame(false),
                                                )
                                                .clicked()
                                            {
                                                dismiss_idx = Some(idx);
                                            }
                                        },
                                    );
                                });
                            });
                        let stripe = egui::Rect::from_min_size(
                            egui::pos2(
                                frame_resp.response.rect.left() + 1.0,
                                frame_resp.response.rect.top() + 8.0,
                            ),
                            Vec2::new(2.0, frame_resp.response.rect.height() - 16.0),
                        );
                        ui.painter()
                            .rect_filled(stripe, Rounding::same(1.0), accent);
                        ui.add_space(6.0);
                    }
                });
            if let Some(i) = dismiss_idx {
                if i < self.toasts.len() {
                    self.toasts.remove(i);
                }
            }
        }
    }
}
