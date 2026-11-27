use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CrashReport {
    pub filename: String,
    pub path: String,
    pub timestamp: String,
    pub loader_version: String,
    pub faulty_mod_name: String,
    pub faulty_mod_id: Option<String>,
    pub exception_code: String,
    pub exception_details: String,
    pub stack_trace: Vec<String>,
    pub installed_mods_count: usize,
    pub raw_text: String,
}

pub fn scan_crash_logs(game_dir: &str) -> Vec<CrashReport> {
    let dir = Path::new(game_dir).join("geode").join("crashlogs");
    let Ok(rd) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut items: Vec<(std::time::SystemTime, CrashReport)> = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("log") {
            continue;
        }
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let filename = entry.file_name().to_string_lossy().into_owned();
        let report = parse_crash_log(&filename, &path.to_string_lossy(), &raw);
        let mtime = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        items.push((mtime, report));
    }

    items.sort_by(|a, b| b.0.cmp(&a.0));
    items.into_iter().map(|(_, r)| r).collect()
}

pub fn parse_crash_log(filename: &str, full_path: &str, raw: &str) -> CrashReport {
    let mut lines = raw.lines();
    let timestamp = lines.next().unwrap_or("").trim().to_string();

    let mut loader_version = String::new();
    let mut faulty_mod_name = String::new();
    let mut faulty_mod_id = None;
    let mut exception_code = String::new();
    let mut exception_details = String::new();
    let mut stack_trace = Vec::new();
    let mut installed_mods_count = 0usize;

    let mut in_stack = false;
    let mut in_mods = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("== Stack Trace") {
            in_stack = true;
            in_mods = false;
            continue;
        }
        if trimmed.starts_with("== Register States") {
            in_stack = false;
            continue;
        }
        if trimmed.starts_with("== Installed Mods") {
            in_stack = false;
            in_mods = true;
            continue;
        }
        if trimmed.starts_with("==") {
            in_stack = false;
            in_mods = false;
            continue;
        }

        if in_stack {
            if trimmed.starts_with('-') {
                stack_trace.push(trimmed.trim_start_matches('-').trim().to_string());
            }
            continue;
        }
        if in_mods {
            if !trimmed.is_empty() {
                installed_mods_count += 1;
            }
            continue;
        }

        if let Some(v) = trimmed.strip_prefix("Loader Version:") {
            loader_version = v.trim().to_string();
        } else if let Some(v) = trimmed.strip_prefix("Faulty Mod:") {
            let val = v.trim();
            faulty_mod_name = val.to_string();
            if let (Some(l), Some(r)) = (val.rfind('('), val.rfind(')')) {
                if r > l + 1 {
                    faulty_mod_id = Some(val[l + 1..r].trim().to_string());
                }
            }
        } else if let Some(v) = trimmed.strip_prefix("Exception Code:") {
            exception_code = v.trim().to_string();
        } else if let Some(v) = trimmed.strip_prefix("Exception Details:") {
            exception_details = v.trim().to_string();
        }
    }

    if faulty_mod_name.is_empty() {
        faulty_mod_name = "Unknown / Core Binary".into();
    }

    CrashReport {
        filename: filename.to_string(),
        path: full_path.to_string(),
        timestamp,
        loader_version,
        faulty_mod_name,
        faulty_mod_id,
        exception_code,
        exception_details,
        stack_trace,
        installed_mods_count,
        raw_text: raw.to_string(),
    }
}

pub fn format_redacted_summary(report: &CrashReport, redact_user: bool) -> String {
    let mut text = format!(
        "Geode Crash Summary ({})\nTimestamp: {}\nLoader: {}\nFaulty Mod: {}\nException: {} — {}\nStack Top:\n",
        report.filename,
        report.timestamp,
        report.loader_version,
        report.faulty_mod_name,
        report.exception_code,
        report.exception_details
    );
    for frame in report.stack_trace.iter().take(6) {
        text.push_str("  - ");
        text.push_str(frame);
        text.push('\n');
    }

    if redact_user {
        if let Ok(user) = std::env::var("USERNAME") {
            if !user.is_empty() {
                text = text.replace(&format!("Users\\{user}"), "Users\\<REDACTED>");
            }
        }
    }
    text
}

pub fn delete_crash_log(path: &str) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| format!("delete crash log: {e}"))
}
