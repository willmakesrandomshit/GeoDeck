use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::{ProcessRefreshKind, System};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessAdvisory {
    pub process_name: String,
    pub category: String,
    pub instance_count: usize,
    pub total_memory_mb: u64,
    pub impact: String,
    pub recommendation: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SystemAdvisorReport {
    pub readiness_score: u32,
    pub free_ram_gb: f32,
    pub total_processes: usize,
    pub advisories: Vec<ProcessAdvisory>,
}

struct ContenderSpec {
    exe_lower: &'static str,
    display: &'static str,
    category: &'static str,
    penalty: u32,
    impact: &'static str,
    recommendation: &'static str,
}

const CONTENDERS: &[ContenderSpec] = &[
    ContenderSpec {
        exe_lower: "gameoverlayui.exe",
        display: "Steam In-Game Overlay",
        category: "Hook Contention",
        penalty: 10,
        impact: "Injects DX/OpenGL overlay hooks that can cause frame-time spikes with Geode.",
        recommendation: "Disable Steam Overlay for Geometry Dash in Steam Properties -> General.",
    },
    ContenderSpec {
        exe_lower: "discord.exe",
        display: "Discord Overlay / Client",
        category: "Overlay & Hardware Accel",
        penalty: 6,
        impact: "Discord Game Overlay adds OpenGL swapchain hooks and GPU contention.",
        recommendation: "Disable Discord In-Game Overlay for Geometry Dash when grinding.",
    },
    ContenderSpec {
        exe_lower: "obs64.exe",
        display: "OBS Studio",
        category: "Capture Hook",
        penalty: 6,
        impact: "Game Capture hook + encoder uses GPU/CPU scheduling slices.",
        recommendation: "Use NVENC/AMF hardware encoding and cap preview framerate.",
    },
    ContenderSpec {
        exe_lower: "wallpaper32.exe",
        display: "Wallpaper Engine",
        category: "GPU Background",
        penalty: 10,
        impact: "Renders animated GPU shaders continuously behind desktop/windowed GD.",
        recommendation: "Set Wallpaper Engine to 'Pause' when another application is focused/fullscreen.",
    },
    ContenderSpec {
        exe_lower: "wallpaper64.exe",
        display: "Wallpaper Engine (x64)",
        category: "GPU Background",
        penalty: 10,
        impact: "Renders animated GPU shaders continuously behind desktop/windowed GD.",
        recommendation: "Set Wallpaper Engine to 'Pause' when another application is focused/fullscreen.",
    },
    ContenderSpec {
        exe_lower: "gamebar.exe",
        display: "Xbox Game Bar",
        category: "Overlay Contention",
        penalty: 7,
        impact: "Windows Game DVR background capture adds DWM presentation latency.",
        recommendation: "Disable background recording in Windows Gaming -> Captures.",
    },
    ContenderSpec {
        exe_lower: "chrome.exe",
        display: "Google Chrome",
        category: "Memory & CPU",
        penalty: 8,
        impact: "Multiple renderer processes compete for L3 cache and memory bandwidth.",
        recommendation: "Close heavy video/tab sessions before demanding 240+ FPS attempts.",
    },
    ContenderSpec {
        exe_lower: "msedge.exe",
        display: "Microsoft Edge",
        category: "Memory & CPU",
        penalty: 6,
        impact: "Background tabs & Sleeping Tabs inactive workers consume RAM.",
        recommendation: "Close unused browser tabs when chasing low-latency consistency.",
    },
    ContenderSpec {
        exe_lower: "nvcontainer.exe",
        display: "NVIDIA Overlay / Share",
        category: "Overlay Contention",
        penalty: 5,
        impact: "Instant Replay hooks OpenGL framebuffers.",
        recommendation: "Turn off Instant Replay if experiencing periodic 1-frame hitches.",
    },
];

pub fn scan_system_advisor(
    high_priority_enabled: bool,
    gpu_pref_enabled: bool,
) -> SystemAdvisorReport {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        ProcessRefreshKind::new().with_memory(),
    );

    let free_ram_mb = sys.available_memory() / (1024 * 1024);
    let free_ram_gb = free_ram_mb as f32 / 1024.0;
    let total_processes = sys.processes().len();

    let mut grouped: HashMap<&'static str, (usize, u64)> = HashMap::new();
    for proc_ in sys.processes().values() {
        let exe = proc_.name().to_string_lossy().to_lowercase();
        for spec in CONTENDERS {
            if exe == spec.exe_lower {
                let entry = grouped.entry(spec.exe_lower).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += proc_.memory() / (1024 * 1024);
            }
        }
    }

    let mut score: i32 = 85;
    if high_priority_enabled {
        score += 8;
    }
    if gpu_pref_enabled {
        score += 7;
    }
    if free_ram_mb < 2048 {
        score -= 12;
    }

    let mut advisories = Vec::new();
    for spec in CONTENDERS {
        if let Some(&(count, mem_mb)) = grouped.get(spec.exe_lower) {
            score -= spec.penalty as i32;
            advisories.push(ProcessAdvisory {
                process_name: spec.display.to_string(),
                category: spec.category.to_string(),
                instance_count: count,
                total_memory_mb: mem_mb,
                impact: spec.impact.to_string(),
                recommendation: spec.recommendation.to_string(),
            });
        }
    }

    advisories.sort_by(|a, b| b.total_memory_mb.cmp(&a.total_memory_mb));

    SystemAdvisorReport {
        readiness_score: score.clamp(15, 100) as u32,
        free_ram_gb,
        total_processes,
        advisories,
    }
}
