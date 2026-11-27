use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use crate::core::config::{GeodeLaunchFlags, LaunchMode};
use crate::core::worker::{ToastKind, WorkerEvent};
use crate::optimizer::tweaks::{apply_live_process_affinity, apply_live_process_priority};

#[derive(Clone, Debug)]
pub struct ActiveSession {
    pub pid: Option<u32>,
    pub installation_name: String,
    pub game_path: String,
    pub started_at: Instant,
    pub launch_mode: LaunchMode,
}

pub fn build_launch_args(flags: &GeodeLaunchFlags, custom_args: &str, force_safe: bool) -> Vec<String> {
    let mut args = Vec::new();
    if force_safe || flags.safe_mode {
        args.push("--geode:safe-mode".into());
    }
    if flags.disable_crash_handler {
        args.push("--geode:disable-crash-handler".into());
    }
    if flags.enable_tulip_hook_logs {
        args.push("--geode:enable-tulip-hook-logs".into());
    }
    let bin = flags.custom_binary_dir.trim();
    if !bin.is_empty() {
        args.push(format!("--geode:binary-dir={bin}"));
    }
    for raw in [flags.extra_args.as_str(), custom_args] {
        for tok in raw.split_whitespace() {
            if !tok.is_empty() {
                args.push(tok.to_string());
            }
        }
    }
    args
}

fn snapshot_crashlog_names(game_path: &str) -> HashSet<String> {
    let dir = Path::new(game_path).join("geode").join("crashlogs");
    let mut set = HashSet::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            set.insert(e.file_name().to_string_lossy().into_owned());
        }
    }
    set
}

pub fn is_gd_running() -> bool {
    find_running_gd_pid().is_some()
}

pub fn find_running_gd_pid() -> Option<u32> {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        sysinfo::ProcessRefreshKind::new(),
    );
    for (pid, proc_) in sys.processes() {
        let name = proc_.name().to_string_lossy().to_lowercase();
        if name == "geometrydash.exe" {
            return Some(pid.as_u32());
        }
    }
    None
}

pub fn terminate_gd_process() -> Result<(), String> {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        sysinfo::ProcessRefreshKind::new(),
    );
    let mut killed = false;
    for (_pid, proc_) in sys.processes() {
        if proc_.name().to_string_lossy().eq_ignore_ascii_case("GeometryDash.exe") {
            if proc_.kill() {
                killed = true;
            }
        }
    }
    if killed {
        Ok(())
    } else {
        Err("GeometryDash.exe is not currently running.".into())
    }
}

pub fn launch_game(
    installation_name: &str,
    game_path: &str,
    mode: LaunchMode,
    flags: &GeodeLaunchFlags,
    custom_args: &str,
    high_priority: bool,
    perf_affinity: bool,
    tx: Sender<WorkerEvent>,
    ctx: Option<eframe::egui::Context>,
) -> Result<ActiveSession, String> {
    if is_gd_running() {
        return Err("Geometry Dash is already running.".into());
    }

    let exe = PathBuf::from(game_path).join("GeometryDash.exe");
    if !exe.is_file() {
        return Err(format!("GeometryDash.exe not found at {}", exe.display()));
    }

    let pre_crashes = snapshot_crashlog_names(game_path);
    let force_safe = mode == LaunchMode::SafeMode;
    let args = build_launch_args(flags, custom_args, force_safe);

    if mode == LaunchMode::SteamProtocol {
        Command::new("cmd")
            .args(["/C", "start", "", "steam://rungameid/322170"])
            .spawn()
            .map_err(|e| format!("Failed to invoke Steam launch: {e}"))?;

        let game_dir = game_path.to_string();
        thread::spawn(move || {
            let started = Instant::now();
            // Wait up to 20s for Steam to spawn GeometryDash.exe
            let mut pid = None;
            for _ in 0..40 {
                if let Some(found) = find_running_gd_pid() {
                    pid = Some(found);
                    break;
                }
                thread::sleep(Duration::from_millis(500));
            }
            if let Some(p) = pid {
                if high_priority {
                    let _ = apply_live_process_priority(p, true);
                }
                if perf_affinity {
                    let _ = apply_live_process_affinity(p, true);
                }
                while find_running_gd_pid().is_some() {
                    thread::sleep(Duration::from_millis(1200));
                }
            }
            let post_crashes = snapshot_crashlog_names(&game_dir);
            let new_crashes = post_crashes.difference(&pre_crashes).count();
            let _ = tx.send(WorkerEvent::GameExited {
                exit_code: None,
                duration_secs: started.elapsed().as_secs(),
                new_crashes,
            });
            if let Some(c) = ctx {
                c.request_repaint();
            }
        });

        return Ok(ActiveSession {
            pid: None,
            installation_name: installation_name.to_string(),
            game_path: game_path.to_string(),
            started_at: Instant::now(),
            launch_mode: mode,
        });
    }

    let mut child = Command::new(&exe)
        .current_dir(game_path)
        .args(&args)
        .spawn()
        .map_err(|e| format!("Failed to launch GeometryDash.exe: {e}"))?;

    let pid = child.id();
    if high_priority {
        if let Err(e) = apply_live_process_priority(pid, true) {
            let _ = tx.send(WorkerEvent::Toast {
                kind: ToastKind::Warning,
                title: "Priority Adjustment".into(),
                body: format!("Could not set High CPU priority: {e}"),
            });
        }
    }
    if perf_affinity {
        let _ = apply_live_process_affinity(pid, true);
    }

    let game_dir = game_path.to_string();
    thread::spawn(move || {
        let started = Instant::now();
        let status = child.wait().ok();
        let exit_code = status.and_then(|s| s.code());
        let post_crashes = snapshot_crashlog_names(&game_dir);
        let new_crashes = post_crashes.difference(&pre_crashes).count();
        let _ = tx.send(WorkerEvent::GameExited {
            exit_code,
            duration_secs: started.elapsed().as_secs(),
            new_crashes,
        });
        if let Some(c) = ctx {
            c.request_repaint();
        }
    });

    Ok(ActiveSession {
        pid: Some(pid),
        installation_name: installation_name.to_string(),
        game_path: game_path.to_string(),
        started_at: Instant::now(),
        launch_mode: mode,
    })
}
