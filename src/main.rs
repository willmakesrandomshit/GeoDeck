#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

use geodedeck::backup::save_inspector::inspect_save_dir;
use geodedeck::backup::vault::list_snapshots;
use geodedeck::core::config::Config;
use geodedeck::geode::dependency::audit_mods;
use geodedeck::geode::package::{local_gd_data_dir, scan_installed_mods};
use geodedeck::launcher::detection::probe_installation;
use geodedeck::launcher::integrity::verify_installation;
use geodedeck::tools::crash_analyzer::scan_crash_logs;
use geodedeck::tools::songs::scan_song_cache;
use geodedeck::ui::app::{GeodeDeckApp, NavTab};
use geodedeck::ui::icons::generate_window_icon;
use std::path::PathBuf;

fn run_self_check() -> i32 {
    let cfg = Config::load();
    let probe = probe_installation(&cfg.game_path);
    let integrity = verify_installation(&cfg.game_path);
    let mods = scan_installed_mods(&cfg.game_path);
    let audit = audit_mods(
        &mods,
        probe.gd_version.as_deref(),
        probe.geode_version.as_deref(),
    );
    let snaps = list_snapshots(std::path::Path::new(&cfg.backup_dir));
    let save_sum = inspect_save_dir(&local_gd_data_dir());
    let crashes = scan_crash_logs(&cfg.game_path);
    let songs = scan_song_cache();

    println!("=== GeoDeck Alpha-1.0.0 ({}) Self-Check ===", env!("CARGO_PKG_VERSION"));
    println!("Active Slot: {} ({})", cfg.active_version, cfg.game_path);
    println!(
        "GD PE Timestamp: {:?} -> Version: {:?}",
        probe.pe_timestamp, probe.gd_version
    );
    println!(
        "Geode Loader: {:?} (Proxy XInput1_4: {}) | Compat: {:?}",
        probe.geode_version, probe.geode_proxy_present, probe.compat_state
    );
    println!(
        "Integrity: ok={} (verified={}, missing={}, corrupted={})",
        integrity.overall_ok,
        integrity.healthy_count,
        integrity.missing_count,
        integrity.corrupted_count
    );
    println!(
        "Geode Mods: {} installed ({} enabled) | Audit: {} errors, {} warnings",
        audit.total_mods, audit.enabled_mods, audit.errors, audit.warnings
    );
    println!(
        "Optimization Profile: {:?} | Auto-Update Repo: {} (on_startup={})",
        cfg.optimization_profile, cfg.update_repo, cfg.check_updates_on_startup
    );
    println!(
        "Save Inspector: player='{}' GM_valid={} LL_valid={} ({} levels) | Vault Snapshots={}",
        save_sum.player_name,
        save_sum.game_manager_valid,
        save_sum.local_levels_valid,
        save_sum.level_count,
        snaps.len()
    );
    println!(
        "Diagnostics: {} crash logs | {} cached songs",
        crashes.len(),
        songs.total_songs
    );
    0
}

struct CliLaunchOptions {
    capture_path: Option<PathBuf>,
    initial_tab: Option<NavTab>,
    initial_tools_subtab: Option<usize>,
    window_size: [f32; 2],
}

fn parse_cli_options() -> CliLaunchOptions {
    let mut capture_path = None;
    let mut initial_tab = None;
    let mut initial_tools_subtab = None;
    let mut window_size = [1140.0_f32, 740.0_f32];

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--capture" {
            capture_path = args.next().map(PathBuf::from);
        } else if arg == "--tab" {
            if let Some(val) = args.next() {
                match val.to_lowercase().as_str() {
                    "home" => initial_tab = Some(NavTab::Home),
                    "installs" | "library" => initial_tab = Some(NavTab::Library),
                    "mods" => initial_tab = Some(NavTab::Mods),
                    "demonlist" | "demons" => initial_tab = Some(NavTab::Demonlist),
                    "backups" => initial_tab = Some(NavTab::Backups),
                    "tools" | "health" => initial_tab = Some(NavTab::Tools),
                    "textures" => {
                        initial_tab = Some(NavTab::Tools);
                        initial_tools_subtab = Some(2);
                    }
                    "songs" | "nong" => {
                        initial_tab = Some(NavTab::Tools);
                        initial_tools_subtab = Some(3);
                    }
                    "settings" | "about" => initial_tab = Some(NavTab::Settings),
                    _ => {}
                }
            }
        } else if arg == "--size" {
            if let Some(val) = args.next() {
                if let Some((w, h)) = val.split_once('x').or_else(|| val.split_once('X')) {
                    if let (Ok(wf), Ok(hf)) = (w.parse::<f32>(), h.parse::<f32>()) {
                        window_size = [wf.max(900.0), hf.max(580.0)];
                    }
                }
            }
        }
    }

    CliLaunchOptions {
        capture_path,
        initial_tab,
        initial_tools_subtab,
        window_size,
    }
}

fn main() -> eframe::Result<()> {
    if std::env::args().any(|a| a == "--self-check") {
        let code = run_self_check();
        std::process::exit(code);
    }

    let cli = parse_cli_options();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("GeoDeck — Geometry Dash Manager")
            .with_icon(generate_window_icon())
            .with_decorations(false)
            .with_inner_size(cli.window_size)
            .with_min_inner_size([920.0, 600.0]),
        persist_window: false,
        persistence_path: Some(geodedeck::core::config::config_dir().join("compact-ui.ron")),
        ..Default::default()
    };

    eframe::run_native(
        "GeoDeck",
        options,
        Box::new(move |cc| {
            Ok(Box::new(GeodeDeckApp::new_with_options(
                cc,
                cli.capture_path,
                cli.initial_tab,
                cli.initial_tools_subtab,
            )))
        }),
    )
}
