use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::backup::save_inspector::SaveSummary;
use crate::backup::vault::SnapshotSummary;
use crate::core::updater::UpdateCheckResult;
use crate::geode::dependency::CompatibilityAudit;
use crate::geode::index_api::{IndexMod, LoaderRelease};
use crate::geode::package::InstalledMod;
use crate::launcher::detection::InstallationProbe;
use crate::launcher::integrity::IntegrityReport;
use crate::tools::crash_analyzer::CrashReport;
use crate::tools::songs::{CachedSongMeta, SongCacheReport};

#[derive(Clone, Debug)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug)]
pub enum WorkerEvent {
    Toast {
        kind: ToastKind,
        title: String,
        body: String,
    },
    ProbeCompleted(InstallationProbe),
    IntegrityCompleted(IntegrityReport),
    ModsScanned {
        mods: Vec<InstalledMod>,
        audit: CompatibilityAudit,
    },
    IndexSearchCompleted {
        mods: Vec<IndexMod>,
        total_count: usize,
        page: usize,
    },
    ModUpdatesChecked {
        updates: Vec<ModUpdateAvailable>,
    },
    LoaderReleaseFetched(Result<LoaderRelease, String>),
    BackupsRefreshed {
        snapshots: Vec<SnapshotSummary>,
        save_summary: Option<SaveSummary>,
        legacy_backups: Vec<(String, std::path::PathBuf)>,
    },
    MaxPerfApplied(Result<String, String>),
    CrashLogsScanned(Vec<CrashReport>),
    SongCacheScanned(SongCacheReport),
    SongMetadataBatchResolved {
        entries: Vec<(String, CachedSongMeta)>,
    },
    SongMetadataResolutionDone {
        resolved_count: usize,
        failed_offline: bool,
    },
    AppUpdateChecked(Result<UpdateCheckResult, String>),
    AppUpdateDownloaded(Result<String, String>),
    BackupCompleted(Result<SnapshotSummary, String>),
    RestoreCompleted(Result<String, String>),
    SnapshotSlotCreated(Result<(String, String), String>),
    ModPackExported(Result<String, String>),
    ModPackImported(Result<String, String>),
    ModInstallFinished {
        mod_id: String,
        result: Result<String, String>,
    },
    ModLogoFetched {
        mod_id: String,
        rgba: (u32, u32, Vec<u8>),
    },
    LoaderInstallFinished(Result<String, String>),
    GameExited {
        exit_code: Option<i32>,
        duration_secs: u64,
        new_crashes: usize,
    },
}

#[derive(Clone, Debug)]
pub struct ModUpdateAvailable {
    pub mod_id: String,
    pub mod_name: String,
    pub current_version: String,
    pub target_version: String,
    pub download_link: String,
    pub expected_hash: String,
    pub required_geode: String,
}

pub struct WorkerBus {
    tx: Sender<WorkerEvent>,
    rx: Receiver<WorkerEvent>,
}

impl WorkerBus {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }

    pub fn sender(&self) -> Sender<WorkerEvent> {
        self.tx.clone()
    }

    pub fn poll(&self) -> Vec<WorkerEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            events.push(ev);
        }
        events
    }
}

impl Default for WorkerBus {
    fn default() -> Self {
        Self::new()
    }
}

pub fn spawn_job<F>(tx: Sender<WorkerEvent>, ctx: Option<eframe::egui::Context>, f: F)
where
    F: FnOnce(&Sender<WorkerEvent>) + Send + 'static,
{
    thread::spawn(move || {
        f(&tx);
        if let Some(c) = ctx {
            c.request_repaint();
        }
    });
}
