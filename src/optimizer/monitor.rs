use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use sysinfo::{ProcessRefreshKind, System};

use crate::core::config::AppliedTweaksState;
use crate::optimizer::advisor::scan_system_advisor;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PerfSnapshot {
    pub captured_at: String,
    pub readiness_score: u32,
    pub available_ram_mb: u64,
    pub contention_processes: usize,
    pub gd_running: bool,
    pub gd_memory_mb: u64,
    pub active_tweaks_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct TelemetrySample {
    pub gd_running: bool,
    pub gd_cpu_pct: f32,
    pub gd_ram_mb: u64,
    pub sys_cpu_pct: f32,
    pub sys_ram_used_pct: f32,
}

pub struct TelemetryHistory {
    sys: System,
    pub samples: VecDeque<TelemetrySample>,
}

impl TelemetryHistory {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        Self {
            sys,
            samples: VecDeque::with_capacity(120),
        }
    }

    pub fn sample_tick(&mut self) -> TelemetrySample {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            ProcessRefreshKind::new().with_cpu().with_memory(),
        );

        let mut gd_running = false;
        let mut gd_cpu = 0.0f32;
        let mut gd_ram = 0u64;

        for proc_ in self.sys.processes().values() {
            if proc_
                .name()
                .to_string_lossy()
                .eq_ignore_ascii_case("GeometryDash.exe")
            {
                gd_running = true;
                gd_cpu = proc_.cpu_usage() / self.sys.cpus().len().max(1) as f32;
                gd_ram = proc_.memory() / (1024 * 1024);
                break;
            }
        }

        let total_mem = self.sys.total_memory().max(1) as f32;
        let used_mem = self.sys.used_memory() as f32;
        let sample = TelemetrySample {
            gd_running,
            gd_cpu_pct: gd_cpu.clamp(0.0, 100.0),
            gd_ram_mb: gd_ram,
            sys_cpu_pct: self.sys.global_cpu_usage().clamp(0.0, 100.0),
            sys_ram_used_pct: (used_mem / total_mem * 100.0).clamp(0.0, 100.0),
        };

        if self.samples.len() >= 120 {
            self.samples.pop_front();
        }
        self.samples.push_back(sample.clone());
        sample
    }
}

impl Default for TelemetryHistory {
    fn default() -> Self {
        Self::new()
    }
}

pub fn capture_perf_snapshot(tweaks: &AppliedTweaksState) -> PerfSnapshot {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        ProcessRefreshKind::new().with_memory(),
    );

    let mut gd_running = false;
    let mut gd_memory_mb = 0u64;
    for proc_ in sys.processes().values() {
        if proc_
            .name()
            .to_string_lossy()
            .eq_ignore_ascii_case("GeometryDash.exe")
        {
            gd_running = true;
            gd_memory_mb = proc_.memory() / (1024 * 1024);
            break;
        }
    }

    let adv = scan_system_advisor(tweaks.cpu_priority_high, tweaks.gpu_high_perf);
    let active_tweaks_count = [
        tweaks.cpu_priority_high,
        tweaks.cpu_affinity_perf,
        tweaks.gpu_high_perf,
        tweaks.disable_fullscreen_opt,
        tweaks.cbf_precision_fix,
        tweaks.cbf_mouse_fix,
    ]
    .into_iter()
    .filter(|&b| b)
    .count();

    PerfSnapshot {
        captured_at: chrono::Local::now().format("%H:%M:%S").to_string(),
        readiness_score: adv.readiness_score,
        available_ram_mb: sys.available_memory() / (1024 * 1024),
        contention_processes: adv.advisories.len(),
        gd_running,
        gd_memory_mb,
        active_tweaks_count,
    }
}
