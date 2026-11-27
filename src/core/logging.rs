use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Success,
    Warn,
    Error,
}

impl LogLevel {
    pub fn tag(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Success => "OK",
            Self::Warn => "WARN",
            Self::Error => "ERR",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub category: String,
    pub message: String,
}

struct LoggerState {
    entries: VecDeque<LogEntry>,
    log_path: PathBuf,
}

const MAX_MEMORY_ENTRIES: usize = 400;
const MAX_LOG_FILE_BYTES: u64 = 2 * 1024 * 1024;

static LOGGER: OnceLock<Mutex<LoggerState>> = OnceLock::new();

fn default_log_path() -> PathBuf {
    crate::core::config::config_dir()
        .join("logs")
        .join("geodedeck.log")
}

fn state() -> &'static Mutex<LoggerState> {
    LOGGER.get_or_init(|| {
        let log_path = default_log_path();
        if let Some(parent) = log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        Mutex::new(LoggerState {
            entries: VecDeque::with_capacity(MAX_MEMORY_ENTRIES),
            log_path,
        })
    })
}

pub fn log(level: LogLevel, category: impl Into<String>, message: impl Into<String>) {
    let category = category.into();
    let message = message.into();
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    if let Ok(mut guard) = state().lock() {
        let entry = LogEntry {
            timestamp: timestamp.clone(),
            level,
            category: category.clone(),
            message: message.clone(),
        };
        if guard.entries.len() >= MAX_MEMORY_ENTRIES {
            guard.entries.pop_front();
        }
        guard.entries.push_back(entry);

        if let Ok(meta) = fs::metadata(&guard.log_path) {
            if meta.len() > MAX_LOG_FILE_BYTES {
                let rotated = guard.log_path.with_extension("log.1");
                let _ = fs::rename(&guard.log_path, rotated);
            }
        }
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&guard.log_path)
        {
            let _ = writeln!(file, "[{timestamp}] [{}] [{category}] {message}", level.tag());
        }
    }
}

pub fn info(category: &str, msg: impl Into<String>) {
    log(LogLevel::Info, category, msg);
}

pub fn ok(category: &str, msg: impl Into<String>) {
    log(LogLevel::Success, category, msg);
}

pub fn warn(category: &str, msg: impl Into<String>) {
    log(LogLevel::Warn, category, msg);
}

pub fn err(category: &str, msg: impl Into<String>) {
    log(LogLevel::Error, category, msg);
}

pub fn recent_entries() -> Vec<LogEntry> {
    state()
        .lock()
        .map(|g| g.entries.iter().rev().cloned().collect())
        .unwrap_or_default()
}

pub fn log_file_path() -> PathBuf {
    state()
        .lock()
        .map(|g| g.log_path.clone())
        .unwrap_or_else(|_| default_log_path())
}

pub fn clear_memory_entries() {
    if let Ok(mut g) = state().lock() {
        g.entries.clear();
    }
}
