use std::time::{Duration, Instant};

use crate::backup::vault::{create_snapshot, live_save_fingerprint, SnapshotSummary};
use crate::core::config::BackupPolicy;
use crate::geode::package::local_gd_data_dir;

pub struct BackupScheduler {
    last_tick: Instant,
}

impl BackupScheduler {
    pub fn new() -> Self {
        Self {
            last_tick: Instant::now(),
        }
    }

    pub fn should_run_periodic(&self, policy: &BackupPolicy) -> bool {
        if policy.scheduled_interval_mins == 0 {
            return false;
        }
        let interval = Duration::from_secs(policy.scheduled_interval_mins as u64 * 60);
        self.last_tick.elapsed() >= interval
    }

    pub fn mark_tick(&mut self) {
        self.last_tick = Instant::now();
    }
}

impl Default for BackupScheduler {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run_auto_backup_if_changed(
    backup_dir: &str,
    policy: &mut BackupPolicy,
    reason: &str,
) -> Result<Option<SnapshotSummary>, String> {
    let live_dir = local_gd_data_dir();
    let Some(fp) = live_save_fingerprint(&live_dir) else {
        return Ok(None);
    };
    if !policy.last_backup_hash.is_empty() && policy.last_backup_hash == fp {
        return Ok(None);
    }

    let snap = create_snapshot(
        backup_dir,
        reason,
        policy.encrypt_vault,
        policy.verify_xml_before_backup,
        policy.include_geode_configs,
        policy.retention_count,
    )?;
    policy.last_backup_at = snap.created_at.clone();
    policy.last_backup_hash = fp;
    Ok(Some(snap))
}
