use serde::{Deserialize, Serialize};
#[cfg(windows)]
use std::process::Command;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub cpu_name: String,
    pub logical_cores: usize,
    pub physical_cores: usize,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub gpus: Vec<String>,
    pub monitor_hz: u32,
    pub monitor_width: u32,
    pub monitor_height: u32,
    pub recommended_fps: u32,
    pub fps_recommendation_note: String,
}

pub fn detect_hardware_profile() -> HardwareProfile {
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    let cpu_name = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown CPU".into());
    let logical_cores = sys.cpus().len().max(1);
    let physical_cores = sys.physical_core_count().unwrap_or(logical_cores);
    let total_ram_mb = sys.total_memory() / (1024 * 1024);
    let available_ram_mb = sys.available_memory() / (1024 * 1024);

    let gpus = detect_gpus();
    let (monitor_width, monitor_height, monitor_hz) = detect_display_mode();
    let (recommended_fps, fps_recommendation_note) = compute_recommended_fps(monitor_hz);

    HardwareProfile {
        cpu_name,
        logical_cores,
        physical_cores,
        total_ram_mb,
        available_ram_mb,
        gpus,
        monitor_width,
        monitor_height,
        monitor_hz,
        recommended_fps,
        fps_recommendation_note,
    }
}

pub fn compute_recommended_fps(hz: u32) -> (u32, String) {
    if hz == 0 {
        return (
            240,
            "Default 240 FPS matches Geometry Dash 2.2's 240 TPS physics tick rate.".into(),
        );
    }
    if hz >= 240 {
        (
            hz,
            format!(
                "Display runs at {hz} Hz. Matching {hz} FPS or a 240/480 divisor minimizes frame pacing jitter."
            ),
        )
    } else if 240 % hz == 0 {
        (
            240,
            format!(
                "Display runs at {hz} Hz (exact divisor of 240 TPS). Enable 240 FPS + V-Sync off or CBF for crispest input pacing."
            ),
        )
    } else {
        (
            hz * 2,
            format!(
                "Display runs at {hz} Hz. Running {} FPS (2x display) with CBF minimizes micro-stutter and input delay.",
                hz * 2
            ),
        )
    }
}

#[cfg(windows)]
fn detect_display_mode() -> (u32, u32, u32) {
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS,
    };

    unsafe {
        let mut dm: DEVMODEW = std::mem::zeroed();
        dm.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        if EnumDisplaySettingsW(std::ptr::null(), ENUM_CURRENT_SETTINGS, &mut dm) != 0 {
            return (dm.dmPelsWidth, dm.dmPelsHeight, dm.dmDisplayFrequency);
        }
    }
    (1920, 1080, 60)
}

#[cfg(not(windows))]
fn detect_display_mode() -> (u32, u32, u32) {
    (1920, 1080, 60)
}

#[cfg(windows)]
fn detect_gpus() -> Vec<String> {
    let mut gpus = Vec::new();
    let out = Command::new("reg")
        .args([
            "query",
            r"HKLM\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}",
            "/s",
            "/v",
            "DriverDesc",
        ])
        .output();

    if let Ok(output) = out {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("DriverDesc") {
                if let Some((_, val)) = trimmed.split_once("REG_SZ") {
                    let name = val.trim().to_string();
                    if !name.is_empty() && !gpus.contains(&name) {
                        gpus.push(name);
                    }
                }
            }
        }
    }

    if gpus.is_empty() {
        gpus.push("Default DirectX / OpenGL Display Adapter".into());
    }
    gpus
}

#[cfg(not(windows))]
fn detect_gpus() -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        vec!["Apple Metal / OpenGL Display Adapter".into()]
    }
    #[cfg(not(target_os = "macos"))]
    {
        vec!["Linux OpenGL / Vulkan / Proton Display Adapter".into()]
    }
}
