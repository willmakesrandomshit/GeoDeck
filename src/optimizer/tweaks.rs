use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::core::config::{config_dir, AppliedTweaksState, OptimizationPreset};
use crate::geode::package::{
    local_gd_data_dir, read_mod_settings_map, write_mod_settings_map,
};
use crate::launcher::process::find_running_gd_pid;

const DIRECTX_PREF_KEY: &str = r"HKCU\Software\Microsoft\DirectX\UserGpuPreferences";
const APPCOMPAT_KEY: &str =
    r"HKCU\Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OptimizationRollback {
    pub captured_at: String,
    pub exe_path: String,
    pub prior_gpu_pref: Option<String>,
    pub prior_appcompat: Option<String>,
    pub prior_cbf_settings: Option<BTreeMap<String, Value>>,
}

#[derive(Clone, Debug, Default)]
pub struct CbfSettingsState {
    pub installed: bool,
    pub soft_toggle_disabled: bool,
    pub precision_fix: bool,
    pub mouse_fix: bool,
    pub click_on_steps: bool,
    pub physics_bypass: bool,
    pub safe_mode: bool,
}

#[derive(Clone, Debug, Default)]
pub struct CacheSizes {
    pub geode_temp_bytes: u64,
    pub old_logs_bytes: u64,
    pub old_logs_count: usize,
}

fn rollback_path() -> PathBuf {
    config_dir().join("optimization_rollback.json")
}

pub fn load_rollback() -> Option<OptimizationRollback> {
    fs::read_to_string(rollback_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

pub fn save_rollback(rb: &OptimizationRollback) -> Result<(), String> {
    let path = rollback_path();
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(rb).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn ensure_rollback_captured(exe_path: &str) -> OptimizationRollback {
    if let Some(existing) = load_rollback() {
        if existing.exe_path.eq_ignore_ascii_case(exe_path) {
            return existing;
        }
    }
    let cbf_map = {
        let local = local_gd_data_dir();
        let m = read_mod_settings_map(&local, "syzzi.click_between_frames");
        if m.is_empty() {
            None
        } else {
            Some(m)
        }
    };
    let rb = OptimizationRollback {
        captured_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        exe_path: exe_path.to_string(),
        prior_gpu_pref: query_reg_value(DIRECTX_PREF_KEY, exe_path),
        prior_appcompat: query_reg_value(APPCOMPAT_KEY, exe_path),
        prior_cbf_settings: cbf_map,
    };
    let _ = save_rollback(&rb);
    rb
}

fn query_reg_value(key: &str, value_name: &str) -> Option<String> {
    let out = Command::new("reg")
        .args(["query", key, "/v", value_name])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if line.contains("REG_SZ") {
            if let Some((_, rhs)) = line.split_once("REG_SZ") {
                return Some(rhs.trim().to_string());
            }
        }
    }
    None
}

fn set_reg_sz(key: &str, value_name: &str, data: &str) -> Result<(), String> {
    let out = Command::new("reg")
        .args(["add", key, "/v", value_name, "/t", "REG_SZ", "/d", data, "/f"])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn delete_reg_value(key: &str, value_name: &str) -> Result<(), String> {
    let out = Command::new("reg")
        .args(["delete", key, "/v", value_name, "/f"])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        // Not existing is also acceptable on revert
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("unable to find") {
            Ok(())
        } else {
            Err(stderr.trim().to_string())
        }
    }
}

pub fn read_gpu_high_perf(exe_path: &str) -> bool {
    query_reg_value(DIRECTX_PREF_KEY, exe_path)
        .map(|v| v.contains("GpuPreference=2"))
        .unwrap_or(false)
}

pub fn set_gpu_high_perf(exe_path: &str, enable: bool) -> Result<(), String> {
    ensure_rollback_captured(exe_path);
    let val = if enable {
        "GpuPreference=2;"
    } else {
        "GpuPreference=0;"
    };
    set_reg_sz(DIRECTX_PREF_KEY, exe_path, val)
}

pub fn read_disable_fullscreen_opt(exe_path: &str) -> bool {
    query_reg_value(APPCOMPAT_KEY, exe_path)
        .map(|v| v.contains("DISABLEDXMAXIMIZEDWINDOWEDMODE"))
        .unwrap_or(false)
}

pub fn set_disable_fullscreen_opt(exe_path: &str, enable: bool) -> Result<(), String> {
    ensure_rollback_captured(exe_path);
    if enable {
        set_reg_sz(
            APPCOMPAT_KEY,
            exe_path,
            "~ DISABLEDXMAXIMIZEDWINDOWEDMODE HIGHDPIAWARE",
        )
    } else {
        delete_reg_value(APPCOMPAT_KEY, exe_path)
    }
}

#[cfg(windows)]
pub fn apply_live_process_priority(pid: u32, high: bool) -> Result<(), String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, SetPriorityClass, HIGH_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
        PROCESS_SET_INFORMATION,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_SET_INFORMATION, 0, pid);
        if handle.is_null() {
            return Err("OpenProcess failed for priority adjustment".into());
        }
        let class = if high {
            HIGH_PRIORITY_CLASS
        } else {
            NORMAL_PRIORITY_CLASS
        };
        let ok = SetPriorityClass(handle, class);
        CloseHandle(handle);
        if ok != 0 {
            Ok(())
        } else {
            Err("SetPriorityClass returned failure".into())
        }
    }
}

#[cfg(not(windows))]
pub fn apply_live_process_priority(_pid: u32, _high: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub fn apply_live_process_affinity(pid: u32, pin_primary_cores: bool) -> Result<(), String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetProcessAffinityMask, OpenProcess, SetProcessAffinityMask, PROCESS_QUERY_INFORMATION,
        PROCESS_SET_INFORMATION,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_SET_INFORMATION | PROCESS_QUERY_INFORMATION, 0, pid);
        if handle.is_null() {
            return Err("OpenProcess failed for affinity adjustment".into());
        }
        let mut proc_mask: usize = 0;
        let mut sys_mask: usize = 0;
        if GetProcessAffinityMask(handle, &mut proc_mask, &mut sys_mask) == 0 {
            CloseHandle(handle);
            return Err("GetProcessAffinityMask failed".into());
        }
        let target_mask = if pin_primary_cores {
            // Avoid CPU0 (OS/DPC interrupt core) when >= 4 cores exist; use first 4 performance threads
            let bits = sys_mask.count_ones();
            if bits >= 6 {
                0b0001_1110 & sys_mask
            } else if bits >= 4 {
                0b0000_1110 & sys_mask
            } else {
                sys_mask
            }
        } else {
            sys_mask
        };
        let safe_mask = if target_mask == 0 { sys_mask } else { target_mask };
        let ok = SetProcessAffinityMask(handle, safe_mask);
        CloseHandle(handle);
        if ok != 0 {
            Ok(())
        } else {
            Err("SetProcessAffinityMask failed".into())
        }
    }
}

#[cfg(not(windows))]
pub fn apply_live_process_affinity(_pid: u32, _pin_primary_cores: bool) -> Result<(), String> {
    Ok(())
}

pub fn read_cbf_settings(game_dir: &str) -> CbfSettingsState {
    let mod_exists = Path::new(game_dir)
        .join("geode")
        .join("mods")
        .join("syzzi.click_between_frames.geode")
        .is_file();
    let local = local_gd_data_dir();
    let map = read_mod_settings_map(&local, "syzzi.click_between_frames");
    let get_b = |k: &str, default: bool| -> bool {
        map.get(k).and_then(|v| v.as_bool()).unwrap_or(default)
    };
    CbfSettingsState {
        installed: mod_exists,
        soft_toggle_disabled: get_b("soft-toggle", false),
        precision_fix: get_b("precision-fix", false),
        mouse_fix: get_b("mouse-fix", false),
        click_on_steps: get_b("click-on-steps", false),
        physics_bypass: get_b("physics-bypass", false),
        safe_mode: get_b("safe-mode", false),
    }
}

pub fn write_cbf_settings(
    exe_path: &str,
    precision_fix: bool,
    mouse_fix: bool,
) -> Result<(), String> {
    ensure_rollback_captured(exe_path);
    let local = local_gd_data_dir();
    let mut map = read_mod_settings_map(&local, "syzzi.click_between_frames");
    map.insert("precision-fix".into(), Value::Bool(precision_fix));
    map.insert("mouse-fix".into(), Value::Bool(mouse_fix));
    write_mod_settings_map(&local, "syzzi.click_between_frames", &map)
}

pub fn apply_preset(
    preset: OptimizationPreset,
    exe_path: &str,
    state: &mut AppliedTweaksState,
) -> Result<String, String> {
    match preset {
        OptimizationPreset::MaxPerformance => {
            state.cpu_priority_high = true;
            state.cpu_affinity_perf = true;
            state.gpu_high_perf = true;
            state.disable_fullscreen_opt = true;
            state.close_steam_overlay_warn = true;
            state.cbf_precision_fix = true;
            state.cbf_mouse_fix = true;
        }
        OptimizationPreset::Balanced => {
            state.cpu_priority_high = true;
            state.cpu_affinity_perf = false;
            state.gpu_high_perf = true;
            state.disable_fullscreen_opt = false;
            state.close_steam_overlay_warn = true;
            state.cbf_precision_fix = true;
            state.cbf_mouse_fix = false;
        }
        OptimizationPreset::Compatibility => {
            state.cpu_priority_high = false;
            state.cpu_affinity_perf = false;
            state.gpu_high_perf = false;
            state.disable_fullscreen_opt = false;
            state.close_steam_overlay_warn = false;
            state.cbf_precision_fix = false;
            state.cbf_mouse_fix = false;
        }
        OptimizationPreset::Custom => {}
    }

    apply_all_tweaks(exe_path, state)?;
    Ok(format!("Applied {} preset safely.", preset.label()))
}

pub fn apply_all_tweaks(exe_path: &str, state: &AppliedTweaksState) -> Result<(), String> {
    if !exe_path.is_empty() {
        ensure_rollback_captured(exe_path);
        set_gpu_high_perf(exe_path, state.gpu_high_perf)?;
        set_disable_fullscreen_opt(exe_path, state.disable_fullscreen_opt)?;
        if Path::new(&local_gd_data_dir())
            .join("geode/mods/syzzi.click_between_frames/settings.json")
            .is_file()
        {
            let _ = write_cbf_settings(exe_path, state.cbf_precision_fix, state.cbf_mouse_fix);
        }
    }

    if let Some(pid) = find_running_gd_pid() {
        let _ = apply_live_process_priority(pid, state.cpu_priority_high);
        let _ = apply_live_process_affinity(pid, state.cpu_affinity_perf);
    }

    Ok(())
}

pub fn revert_all_tweaks(exe_path: &str, state: &mut AppliedTweaksState) -> Result<String, String> {
    if let Some(rb) = load_rollback() {
        let target_exe = if exe_path.is_empty() {
            rb.exe_path.as_str()
        } else {
            exe_path
        };
        if !target_exe.is_empty() {
            match rb.prior_gpu_pref {
                Some(ref val) => set_reg_sz(DIRECTX_PREF_KEY, target_exe, val)?,
                None => delete_reg_value(DIRECTX_PREF_KEY, target_exe)?,
            }
            match rb.prior_appcompat {
                Some(ref val) => set_reg_sz(APPCOMPAT_KEY, target_exe, val)?,
                None => delete_reg_value(APPCOMPAT_KEY, target_exe)?,
            }
        }
        if let Some(ref cbf) = rb.prior_cbf_settings {
            let _ = write_mod_settings_map(&local_gd_data_dir(), "syzzi.click_between_frames", cbf);
        }
        let _ = fs::remove_file(rollback_path());
    } else if !exe_path.is_empty() {
        let _ = delete_reg_value(DIRECTX_PREF_KEY, exe_path);
        let _ = delete_reg_value(APPCOMPAT_KEY, exe_path);
    }

    if let Some(pid) = find_running_gd_pid() {
        let _ = apply_live_process_priority(pid, false);
        let _ = apply_live_process_affinity(pid, false);
    }

    *state = AppliedTweaksState {
        cpu_priority_high: false,
        cpu_affinity_perf: false,
        gpu_high_perf: false,
        disable_fullscreen_opt: false,
        close_steam_overlay_warn: false,
        cbf_precision_fix: false,
        cbf_mouse_fix: false,
    };

    Ok("All registry, process, and mod optimizations restored to baseline.".into())
}

pub fn measure_geode_caches(game_dir: &str) -> CacheSizes {
    let mut sizes = CacheSizes::default();
    if game_dir.is_empty() {
        return sizes;
    }
    let temp = Path::new(game_dir).join("geode").join("temp");
    if temp.is_dir() {
        sizes.geode_temp_bytes = crate::launcher::detection::dir_usage_fast(&temp);
    }

    let logs = Path::new(game_dir).join("geode").join("logs");
    if let Ok(rd) = fs::read_dir(&logs) {
        let mut entries: Vec<_> = rd
            .flatten()
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                let mtime = meta.modified().ok()?;
                Some((mtime, meta.len()))
            })
            .collect();
        entries.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, len) in entries.into_iter().skip(20) {
            sizes.old_logs_count += 1;
            sizes.old_logs_bytes += len;
        }
    }
    sizes
}

pub fn clean_geode_caches(game_dir: &str) -> Result<u64, String> {
    let mut reclaimed = 0u64;
    let temp = Path::new(game_dir).join("geode").join("temp");
    if temp.is_dir() {
        if let Ok(rd) = fs::read_dir(&temp) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    reclaimed += crate::launcher::detection::dir_usage_fast(&p);
                    let _ = fs::remove_dir_all(&p);
                } else {
                    reclaimed += e.metadata().map(|m| m.len()).unwrap_or(0);
                    let _ = fs::remove_file(&p);
                }
            }
        }
    }

    let logs = Path::new(game_dir).join("geode").join("logs");
    if let Ok(rd) = fs::read_dir(&logs) {
        let mut entries: Vec<_> = rd
            .flatten()
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                let mtime = meta.modified().ok()?;
                Some((mtime, meta.len(), e.path()))
            })
            .collect();
        entries.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, len, path) in entries.into_iter().skip(20) {
            if fs::remove_file(&path).is_ok() {
                reclaimed += len;
            }
        }
    }
    Ok(reclaimed)
}
