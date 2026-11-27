use eframe::egui::{self, RichText, Rounding, Stroke};

use crate::backup::crypto::{dpapi_key_ready, vault_key_protection_label};
use crate::backup::save_tools::delete_save_slot;
use crate::backup::vault::{
    export_snapshot_portable_zip, import_backup_folder_into_vault, import_portable_zip_into_vault,
    verify_snapshot_integrity,
};
use crate::core::platform::open_in_file_manager;
use crate::geode::package::local_gd_data_dir;
use crate::launcher::detection::human_bytes;
use crate::ui::app::GeodeDeckApp;
use crate::ui::icons::ph;
use crate::ui::theme::*;

pub fn render(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    // Top Page Header + Primary Actions
    section_heading(
        ui,
        "Encrypted Save Vault & Backups",
        "DPAPI-protected AES-256-GCM snapshots, Save Slots, Save Doctor salvage, Save Compactor, and portable .geodesync bundles.",
    );

    epic_subtab_bar(
        ui,
        &[
            (ph::SHIELD_CHECK, "Encrypted Vault"),
            (ph::HARD_DRIVES, "Save Slots & Salvage"),
            (ph::PACKAGE, "Compactor & .geodesync"),
        ],
        &mut app.ui_state.backups_subtab,
    );

    if app.ui_state.backups_subtab == 1 {
        render_slots_and_salvage(app, ui);
        return;
    }
    if app.ui_state.backups_subtab == 2 {
        render_compactor_and_bundles(app, ui);
        return;
    }

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if ui
            .add(epic_cta_button(ph::FLOPPY_DISK, "Create Backup Now"))
            .on_hover_text("Validate live save files and create an encrypted vault snapshot")
            .clicked()
        {
            app.trigger_manual_backup("Manual Vault Backup");
        }
        if ui
            .add(epic_secondary_button(ph::UPLOAD_SIMPLE, "Import .zip"))
            .on_hover_text("Import a portable Geometry Dash save .zip into the encrypted vault")
            .clicked()
        {
            if let Some(zip) = rfd::FileDialog::new()
                .add_filter("Backup Zip", &["zip"])
                .pick_file()
            {
                match import_portable_zip_into_vault(
                    &zip,
                    std::path::Path::new(&app.config.backup_dir),
                    &crate::backup::crypto::default_vault_key_path(),
                ) {
                    Ok(s) => {
                        app.push_toast(
                            crate::core::worker::ToastKind::Success,
                            "Backup Imported",
                            format!("Imported {} into encrypted vault.", s.id),
                        );
                        app.refresh_backups_async();
                    }
                    Err(e) => app.push_toast(
                        crate::core::worker::ToastKind::Error,
                        "Import Failed",
                        e,
                    ),
                }
            }
        }
        if ui
            .add(epic_secondary_button(ph::FOLDER_OPEN, "Open Vault"))
            .on_hover_text("Open the vault directory")
            .clicked()
        {
            let _ = std::fs::create_dir_all(&app.config.backup_dir);
            open_in_file_manager(std::path::Path::new(&app.config.backup_dir));
        }
    });

    ui.add_space(14.0);

    // 4-Tile KPI Summary Strip
    let total_vault_bytes: u64 = app.snapshots.iter().map(|s| s.total_bytes).sum();
    let dpapi_active = dpapi_key_ready();
    let save_ok = app
        .save_summary
        .as_ref()
        .is_some_and(|s| s.game_manager_valid && s.local_levels_valid);
    let levels_cnt = app.save_summary.as_ref().map_or(0, |s| s.level_count);
    let sched_label = if app.config.backup_policy.scheduled_interval_mins == 0 {
        "Manual Only".to_string()
    } else {
        format!("Every {} min", app.config.backup_policy.scheduled_interval_mins)
    };
    let last_backup_sub = if app.config.backup_policy.last_backup_at.is_empty() {
        app.snapshots
            .first()
            .map_or("No backup recorded".into(), |s| s.created_at.clone())
    } else {
        app.config.backup_policy.last_backup_at.clone()
    };

    ui.columns(4, |cols| {
        stat_tile(
            &mut cols[0],
            ph::LOCK,
            "ENCRYPTION",
            if app.config.backup_policy.encrypt_vault {
                "AES-256-GCM"
            } else {
                "Plaintext"
            },
            vault_key_protection_label(),
            STATUS_OK,
        );
        stat_tile(
            &mut cols[1],
            ph::FILE_ARCHIVE,
            "SNAPSHOTS",
            &format!("{} Stored", app.snapshots.len()),
            &human_bytes(total_vault_bytes),
            ACCENT_EPIC_BRIGHT,
        );
        stat_tile(
            &mut cols[2],
            ph::SHIELD_CHECK,
            "LIVE SAVE HEALTH",
            if app.save_summary.is_none() {
                "Inspecting..."
            } else if save_ok {
                "Valid XML"
            } else {
                "Attention"
            },
            &format!("{levels_cnt} local level(s)"),
            if save_ok { STATUS_OK } else { STATUS_WARN },
        );
        stat_tile(
            &mut cols[3],
            ph::ARROWS_CLOCKWISE,
            "AUTO-SCHEDULE",
            &sched_label,
            &last_backup_sub,
            TEXT_PRIMARY,
        );
    });

    ui.add_space(14.0);

    // Balanced Two-Column Policy + Live Save Integrity Row (strictly width-bounded to prevent overlap)
    ui.spacing_mut().item_spacing.x = 14.0;
    ui.columns(2, |cols| {
        let col0_inner_w = (cols[0].available_width() - 42.0).max(180.0);
        card_frame().show(&mut cols[0], |ui| {
            ui.set_width(col0_inner_w);
            ui.set_max_width(col0_inner_w);
            ui.set_min_height(392.0);

            ui.horizontal(|ui| {
                icon_tile(ui, ph::SHIELD_CHECK, 38.0, STATUS_OK, STATUS_OK_SOFT);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.set_max_width((col0_inner_w - 170.0).max(100.0));
                    ui.add(
                        egui::Label::new(bold_text("Vault Policy & Scheduler", 15.0, TEXT_PRIMARY))
                            .truncate(),
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new("Automated triggers & retention")
                                .size(11.5)
                                .color(TEXT_SECONDARY),
                        )
                        .truncate(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_dot_pill(
                        ui,
                        if dpapi_active {
                            "DPAPI Active"
                        } else {
                            "DPAPI Ready"
                        },
                        STATUS_OK,
                        STATUS_OK_SOFT,
                    );
                });
            });
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(10.0);

            let pol = &mut app.config.backup_policy;
            let mut changed = false;

            let switches = [
                (
                    "Encrypt Snapshots",
                    "AES-256-GCM cipher locked with Windows DPAPI master key",
                    &mut pol.encrypt_vault,
                ),
                (
                    "Pre-Backup XML Validation",
                    "Blocks snapshots if .dat files fail plist XML inflation",
                    &mut pol.verify_xml_before_backup,
                ),
                (
                    "Include Geode Mod Configs",
                    "Bundles settings.json and geode.loader/saved.json",
                    &mut pol.include_geode_configs,
                ),
                (
                    "Pre-Launch Auto-Backup",
                    "Creates a safety snapshot before launching Geometry Dash",
                    &mut pol.auto_pre_launch,
                ),
                (
                    "Post-Exit Auto-Backup",
                    "Protects session progress automatically on game exit",
                    &mut pol.auto_on_exit,
                ),
            ];
            for (title, desc, flag) in switches {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_max_width((col0_inner_w - 58.0).max(110.0));
                        ui.label(semibold_text(title, 13.0, TEXT_PRIMARY));
                        ui.label(RichText::new(desc).size(11.5).color(TEXT_MUTED));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if epic_toggle_switch(ui, flag).changed() {
                            changed = true;
                        }
                    });
                });
                ui.add_space(7.0);
            }

            ui.separator();
            ui.add_space(10.0);

            // Stacked Scheduler Controls (never overflows horizontally)
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((col0_inner_w - 150.0).max(90.0));
                    ui.label(semibold_text("Scheduled Interval", 12.5, TEXT_PRIMARY));
                    ui.label(
                        RichText::new("Periodic background save check")
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("vault_interval")
                        .width(132.0)
                        .selected_text(if pol.scheduled_interval_mins == 0 {
                            "Disabled".into()
                        } else {
                            format!("Every {} min", pol.scheduled_interval_mins)
                        })
                        .show_ui(ui, |ui| {
                            for mins in [0u32, 15, 30, 60, 120] {
                                let lbl = if mins == 0 {
                                    "Disabled".into()
                                } else {
                                    format!("Every {mins} min")
                                };
                                if ui
                                    .selectable_value(&mut pol.scheduled_interval_mins, mins, lbl)
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });
                });
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((col0_inner_w - 150.0).max(90.0));
                    ui.label(semibold_text("Snapshot Retention", 12.5, TEXT_PRIMARY));
                    ui.label(
                        RichText::new("Max automatic backups kept")
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("vault_retention")
                        .width(132.0)
                        .selected_text(if pol.retention_count == 0 {
                            "Unlimited".into()
                        } else {
                            format!("Keep {}", pol.retention_count)
                        })
                        .show_ui(ui, |ui| {
                            for count in [10usize, 25, 50, 100, 0] {
                                let lbl = if count == 0 {
                                    "Unlimited".into()
                                } else {
                                    format!("Keep {count}")
                                };
                                if ui
                                    .selectable_value(&mut pol.retention_count, count, lbl)
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });
                });
            });

            if changed {
                app.config.auto_backup = app.config.backup_policy.auto_on_exit;
                app.config.retention = app.config.backup_policy.retention_count;
                app.config.save();
            }
        });

        let col1_inner_w = (cols[1].available_width() - 42.0).max(180.0);
        card_frame().show(&mut cols[1], |ui| {
            ui.set_width(col1_inner_w);
            ui.set_max_width(col1_inner_w);
            ui.set_min_height(392.0);

            ui.horizontal(|ui| {
                icon_tile(
                    ui,
                    ph::KEY,
                    38.0,
                    ACCENT_EPIC_BRIGHT,
                    ACCENT_EPIC_SOFT,
                );
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.set_max_width((col1_inner_w - 150.0).max(100.0));
                    ui.add(
                        egui::Label::new(bold_text(
                            "Live Save Integrity & Privacy",
                            15.0,
                            TEXT_PRIMARY,
                        ))
                        .truncate(),
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new("Real-time XML save verification")
                                .size(11.5)
                                .color(TEXT_SECONDARY),
                        )
                        .truncate(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(epic_secondary_button(ph::ARROWS_CLOCKWISE, "Inspect"))
                        .on_hover_text("Re-validate CCGameManager.dat & CCLocalLevels.dat")
                        .clicked()
                    {
                        app.refresh_backups_async();
                    }
                });
            });
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(10.0);

            if let Some(ref sum) = app.save_summary {
                elevated_card_frame().show(ui, |ui| {
                    ui.set_width((col1_inner_w - 28.0).max(140.0));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Player Profile").size(12.5).color(TEXT_MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(bold_text(&sum.player_name, 13.5, TEXT_PRIMARY));
                        });
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("CCGameManager.dat")
                                .size(12.5)
                                .color(TEXT_MUTED),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            status_dot_pill(
                                ui,
                                if sum.game_manager_valid {
                                    "Valid XML Structure"
                                } else {
                                    "Missing or Invalid"
                                },
                                if sum.game_manager_valid {
                                    STATUS_OK
                                } else {
                                    STATUS_ERR
                                },
                                if sum.game_manager_valid {
                                    STATUS_OK_SOFT
                                } else {
                                    STATUS_ERR_SOFT
                                },
                            );
                        });
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("CCLocalLevels.dat")
                                .size(12.5)
                                .color(TEXT_MUTED),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            status_dot_pill(
                                ui,
                                &if sum.local_levels_valid {
                                    format!("Valid XML ({} levels)", sum.level_count)
                                } else {
                                    "Missing or Invalid".into()
                                },
                                if sum.local_levels_valid {
                                    STATUS_OK
                                } else {
                                    STATUS_ERR
                                },
                                if sum.local_levels_valid {
                                    STATUS_OK_SOFT
                                } else {
                                    STATUS_ERR_SOFT
                                },
                            );
                        });
                    });
                });

                if let Some(ref err) = sum.validation_error {
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(STATUS_ERR_SOFT)
                        .stroke(Stroke::new(1.0_f32, STATUS_ERR.gamma_multiply(0.45)))
                        .rounding(Rounding::same(RADIUS_CARD))
                        .inner_margin(egui::Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(ph::WARNING).size(15.0).color(STATUS_ERR));
                                ui.label(RichText::new(err).size(12.0).color(STATUS_ERR));
                            });
                        });
                }
            } else {
                empty_state_card(
                    ui,
                    ph::SHIELD_CHECK,
                    "Inspecting Live Save Files",
                    "Decoding Geometry Dash XML structure in memory...",
                );
            }

            ui.add_space(10.0);
            inset_frame().show(ui, |ui| {
                ui.set_width((col1_inner_w - 28.0).max(140.0));
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(ph::LOCK)
                            .size(16.0)
                            .color(ACCENT_EPIC_BRIGHT),
                    );
                    ui.vertical(|ui| {
                        ui.set_max_width((col1_inner_w - 64.0).max(110.0));
                        ui.label(semibold_text("Strict Privacy Guarantee", 12.5, TEXT_PRIMARY));
                        ui.label(
                            RichText::new(
                                "GeoDeck never reads, logs, or caches GJP/GJP2 password tokens during XML inspection.",
                            )
                            .size(11.5)
                            .color(TEXT_SECONDARY),
                        );
                    });
                });
            });

            if !app.legacy_backups.is_empty() {
                ui.add_space(10.0);
                elevated_card_frame().show(ui, |ui| {
                    ui.set_width((col1_inner_w - 28.0).max(140.0));
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_max_width((col1_inner_w - 175.0).max(100.0));
                            ui.label(bold_text(
                                &format!(
                                    "{} External Backup(s) Found",
                                    app.legacy_backups.len()
                                ),
                                13.0,
                                ACCENT_EPIC_BRIGHT,
                            ));
                            ui.label(
                                RichText::new("Detected GeodeSync / GDLauncher save archives")
                                    .size(11.5)
                                    .color(TEXT_SECONDARY),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(epic_secondary_button(
                                    ph::DOWNLOAD_SIMPLE,
                                    "Migrate Latest",
                                ))
                                .on_hover_text("Encrypt and import the newest external backup")
                                .clicked()
                            {
                                if let Some((lbl, path)) = app.legacy_backups.first().cloned() {
                                    match import_backup_folder_into_vault(
                                        &path,
                                        std::path::Path::new(&app.config.backup_dir),
                                        &lbl,
                                    ) {
                                        Ok(s) => {
                                            app.push_toast(
                                                crate::core::worker::ToastKind::Success,
                                                "Legacy Backup Migrated",
                                                format!("Encrypted {} into {}", lbl, s.id),
                                            );
                                            app.refresh_backups_async();
                                        }
                                        Err(e) => app.push_toast(
                                            crate::core::worker::ToastKind::Error,
                                            "Migration Failed",
                                            e,
                                        ),
                                    }
                                }
                            }
                        });
                    });
                });
            }
        });
    });

    ui.add_space(14.0);

    // Vault Snapshots Panel
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(bold_text("Vault Snapshots", 16.0, TEXT_PRIMARY));
            badge_pill(
                ui,
                &format!("{} total", app.snapshots.len()),
                ACCENT_EPIC_BRIGHT,
                ACCENT_EPIC_SOFT,
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(epic_secondary_button(ph::ARROWS_CLOCKWISE, "Refresh"))
                    .clicked()
                {
                    app.refresh_backups_async();
                }
            });
        });
        ui.add_space(10.0);

        if app.snapshots.is_empty() {
            empty_state_card(
                ui,
                ph::FILE_ARCHIVE,
                "No Vault Snapshots Yet",
                "Click 'Create Backup Now' above to store your first encrypted Geometry Dash save snapshot.",
            );
            return;
        }

        let snapshots = app.snapshots.clone();
        egui::ScrollArea::vertical()
            .max_height(340.0)
            .show(ui, |ui| {
                for snap in snapshots {
                    let frame_resp = elevated_card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            icon_tile(
                                ui,
                                if snap.encrypted {
                                    ph::LOCK
                                } else {
                                    ph::FILE_ARCHIVE
                                },
                                40.0,
                                if snap.encrypted {
                                    STATUS_OK
                                } else {
                                    STATUS_WARN
                                },
                                if snap.encrypted {
                                    STATUS_OK_SOFT
                                } else {
                                    STATUS_WARN_SOFT
                                },
                            );
                            ui.add_space(6.0);

                            ui.vertical(|ui| {
                                ui.set_width((ui.available_width() - 320.0).max(160.0));
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(bold_text(&snap.created_at, 14.0, TEXT_PRIMARY));
                                    badge_pill(
                                        ui,
                                        if snap.encrypted { "AES-256-GCM" } else { "PLAIN" },
                                        if snap.encrypted { STATUS_OK } else { STATUS_WARN },
                                        if snap.encrypted {
                                            STATUS_OK_SOFT
                                        } else {
                                            STATUS_WARN_SOFT
                                        },
                                    );
                                    if snap.xml_verified {
                                        badge_pill(
                                            ui,
                                            "XML Verified",
                                            ACCENT_EPIC_BRIGHT,
                                            ACCENT_EPIC_SOFT,
                                        );
                                    }
                                    if snap.includes_geode_configs {
                                        badge_pill(ui, "+Configs", TEXT_SECONDARY, BG_SURFACE);
                                    }
                                });
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "{}  ·  {} file(s)  ·  {}  ·  {}",
                                            snap.trigger_reason,
                                            snap.file_count,
                                            human_bytes(snap.total_bytes),
                                            snap.id
                                        ))
                                        .size(11.5)
                                        .color(TEXT_MUTED),
                                    )
                                    .truncate(),
                                );
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    if ui
                                        .add(epic_danger_button(ph::TRASH, ""))
                                        .on_hover_text("Delete Snapshot")
                                        .clicked()
                                    {
                                        app.ui_state.pending_delete_snapshot = Some(snap.clone());
                                    }
                                    if ui
                                        .add(epic_cta_button(
                                            ph::ARROW_COUNTER_CLOCKWISE,
                                            "Restore",
                                        ))
                                        .on_hover_text("Verify integrity & restore this snapshot")
                                        .clicked()
                                    {
                                        app.ui_state.pending_restore_snapshot = Some(snap.clone());
                                    }
                                    if ui
                                        .add(epic_secondary_button(ph::CHECK_CIRCLE, "Verify"))
                                        .on_hover_text("Verify AES-GCM, SHA-256, and XML integrity")
                                        .clicked()
                                    {
                                        match verify_snapshot_integrity(
                                            std::path::Path::new(&snap.path),
                                            &crate::backup::crypto::default_vault_key_path(),
                                        ) {
                                            Ok(msg) => app.push_toast(
                                                crate::core::worker::ToastKind::Success,
                                                "Integrity Passed",
                                                msg,
                                            ),
                                            Err(e) => app.push_toast(
                                                crate::core::worker::ToastKind::Error,
                                                "Integrity Error",
                                                e,
                                            ),
                                        }
                                    }
                                    if ui
                                        .add(epic_secondary_button(ph::EXPORT, ".zip"))
                                        .on_hover_text("Export decrypted portable .zip archive")
                                        .clicked()
                                    {
                                        if let Some(dst) = rfd::FileDialog::new()
                                            .set_file_name(format!("{}.zip", snap.id))
                                            .save_file()
                                        {
                                            match export_snapshot_portable_zip(
                                                std::path::Path::new(&snap.path),
                                                &dst,
                                                &crate::backup::crypto::default_vault_key_path(),
                                            ) {
                                                Ok(_) => app.push_toast(
                                                    crate::core::worker::ToastKind::Success,
                                                    "Exported",
                                                    format!("Saved to {}", dst.display()),
                                                ),
                                                Err(e) => app.push_toast(
                                                    crate::core::worker::ToastKind::Error,
                                                    "Export Failed",
                                                    e,
                                                ),
                                            }
                                        }
                                    }
                                },
                            );
                        });
                    });

                    if frame_resp.response.hovered() {
                        ui.painter().rect_stroke(
                            frame_resp.response.rect,
                            Rounding::same(RADIUS_CARD),
                            Stroke::new(1.0_f32, BORDER_STRONG),
                        );
                    }
                    ui.add_space(6.0);
                }
            });
    });

    render_backup_modals(app, ui);
}

fn render_backup_modals(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    if let Some(snap) = app.ui_state.pending_restore_snapshot.clone() {
        modal_backdrop(ui.ctx(), "restore_modal_bg");
        egui::Area::new(egui::Id::new("restore_modal_fg"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                dialog_frame().show(ui, |ui| {
                    ui.set_max_width(440.0);
                    ui.horizontal(|ui| {
                        icon_tile(
                            ui,
                            ph::ARROW_COUNTER_CLOCKWISE,
                            38.0,
                            ACCENT_EPIC_BRIGHT,
                            ACCENT_EPIC_SOFT,
                        );
                        ui.vertical(|ui| {
                            ui.label(bold_text("Confirm Safe Save Restoration", 16.0, TEXT_PRIMARY));
                            ui.label(
                                RichText::new(format!("{}  ·  {}", snap.created_at, snap.id))
                                    .size(12.0)
                                    .color(TEXT_MUTED),
                            );
                        });
                    });
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("GeoDeck will verify SHA-256 + XML integrity first and automatically create a Pre-Restore Safety Backup of your current save files.")
                            .size(12.5)
                            .color(TEXT_SECONDARY),
                    );
                    if snap.includes_geode_configs {
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            epic_toggle_switch(ui, &mut app.ui_state.restore_include_configs);
                            ui.label(
                                RichText::new("Restore bundled Geode mod configs & saved.json")
                                    .size(12.5)
                                    .color(TEXT_PRIMARY),
                            );
                        });
                    }
                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(epic_cta_button(ph::CHECK_CIRCLE, "Verify & Restore"))
                            .clicked()
                        {
                            let inc = snap.includes_geode_configs && app.ui_state.restore_include_configs;
                            app.ui_state.pending_restore_snapshot = None;
                            app.restore_snapshot_async(snap.path, inc);
                        }
                        if ui.add(epic_secondary_button("", "Cancel")).clicked() {
                            app.ui_state.pending_restore_snapshot = None;
                        }
                    });
                });
            });
    }

    // === Delete Snapshot Confirmation Modal Dialog ===
    if let Some(snap) = app.ui_state.pending_delete_snapshot.clone() {
        modal_backdrop(ui.ctx(), "delete_snap_modal_bg");
        egui::Area::new(egui::Id::new("delete_snap_modal_fg"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                dialog_frame().show(ui, |ui| {
                    ui.set_max_width(420.0);
                    ui.horizontal(|ui| {
                        icon_tile(ui, ph::TRASH, 38.0, STATUS_ERR, STATUS_ERR_SOFT);
                        ui.vertical(|ui| {
                            ui.label(bold_text("Delete Vault Snapshot?", 16.0, TEXT_PRIMARY));
                            ui.label(
                                RichText::new(format!("{}  ·  {}", snap.created_at, snap.id))
                                    .size(12.0)
                                    .color(TEXT_MUTED),
                            );
                        });
                    });
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("This permanently removes the snapshot directory from disk and cannot be undone.")
                            .size(12.5)
                            .color(TEXT_SECONDARY),
                    );
                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(epic_danger_button(ph::TRASH, "Delete Permanently")).clicked() {
                            match std::fs::remove_dir_all(&snap.path) {
                                Ok(_) => app.push_toast(
                                    crate::core::worker::ToastKind::Info,
                                    "Snapshot Deleted",
                                    format!("Removed {}", snap.id),
                                ),
                                Err(e) => app.push_toast(
                                    crate::core::worker::ToastKind::Error,
                                    "Delete Failed",
                                    e.to_string(),
                                ),
                            }
                            app.ui_state.pending_delete_snapshot = None;
                            app.refresh_backups_async();
                        }
                        if ui.add(epic_secondary_button("", "Cancel")).clicked() {
                            app.ui_state.pending_delete_snapshot = None;
                        }
                    });
                });
            });
    }
}

fn render_slots_and_salvage(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing.x = 14.0;
    ui.columns(2, |cols| {
        let col0_w = (cols[0].available_width() - 42.0).max(180.0);
        card_frame().show(&mut cols[0], |ui| {
            ui.set_width(col0_w);
            ui.set_max_width(col0_w);
            ui.set_min_height(380.0);
            ui.label(bold_text("Save Slots (Multi-Profile)", 15.5, TEXT_PRIMARY));
            ui.label(
                RichText::new("Switch cleanly between main, challenge, or creator save states.")
                    .size(12.0)
                    .color(TEXT_SECONDARY),
            );
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                styled_text_input(
                    ui,
                    &mut app.ui_state.new_save_slot_name,
                    "New Slot Name...",
                    (col0_w - 150.0).max(120.0),
                );
                if ui
                    .add(epic_cta_button(ph::PLUS, "Create Slot"))
                    .clicked()
                {
                    app.create_save_slot_action();
                }
            });
            ui.add_space(10.0);

            if app.save_slots.is_empty() {
                empty_state_card(
                    ui,
                    ph::HARD_DRIVES,
                    "No Save Slots Created",
                    "Enter a slot name above to snapshot your current save into a switchable profile.",
                );
            } else {
                for slot in app.save_slots.clone() {
                    elevated_card_frame().show(ui, |ui| {
                        ui.set_width((col0_w - 28.0).max(140.0));
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.set_max_width((col0_w - 190.0).max(100.0));
                                ui.label(bold_text(&slot.name, 13.5, TEXT_PRIMARY));
                                ui.label(
                                    RichText::new(format!(
                                        "{}  ·  {}★  ·  {} Demons",
                                        slot.player_name, slot.stars, slot.demons
                                    ))
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                                );
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(epic_danger_button(ph::TRASH, "")).clicked() {
                                    match delete_save_slot(&local_gd_data_dir(), &slot.name) {
                                        Ok(msg) => {
                                            app.push_toast(
                                                crate::core::worker::ToastKind::Info,
                                                "Slot Deleted",
                                                msg,
                                            );
                                            app.refresh_backups_async();
                                        }
                                        Err(e) => app.push_toast(
                                            crate::core::worker::ToastKind::Error,
                                            "Delete Failed",
                                            e,
                                        ),
                                    }
                                }
                                if ui
                                    .add(epic_cta_button(ph::ARROW_CLOCKWISE, "Switch"))
                                    .clicked()
                                {
                                    app.switch_save_slot_async(slot.name.clone());
                                }
                            });
                        });
                    });
                    ui.add_space(6.0);
                }
            }
        });

        let col1_w = (cols[1].available_width() - 42.0).max(180.0);
        card_frame().show(&mut cols[1], |ui| {
            ui.set_width(col1_w);
            ui.set_max_width(col1_w);
            ui.set_min_height(380.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((col1_w - 130.0).max(110.0));
                    ui.label(bold_text("Save Doctor & Salvage", 15.5, TEXT_PRIMARY));
                    ui.label(
                        RichText::new("Auto-discover valid CCGameManager backups & resurrect corrupted saves.")
                            .size(11.5)
                            .color(TEXT_SECONDARY),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(epic_secondary_button(ph::ARROWS_CLOCKWISE, "Scan"))
                        .clicked()
                    {
                        app.refresh_backups_async();
                    }
                });
            });
            ui.add_space(10.0);

            if app.salvage_candidates.is_empty() {
                empty_state_card(
                    ui,
                    ph::SHIELD_CHECK,
                    "No Salvage Files Found",
                    "Decoded CCGameManager*.dat and .bak candidates will appear here.",
                );
            } else {
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for cand in app.salvage_candidates.clone() {
                            elevated_card_frame().show(ui, |ui| {
                                ui.set_width((col1_w - 34.0).max(140.0));
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.set_max_width((col1_w - 160.0).max(100.0));
                                        ui.add(
                                            egui::Label::new(bold_text(
                                                &cand.relative_path,
                                                12.5,
                                                TEXT_PRIMARY,
                                            ))
                                            .truncate(),
                                        );
                                        ui.label(
                                            RichText::new(format!(
                                                "{} · {}★ · {} · {}",
                                                cand.player_name,
                                                cand.stars,
                                                human_bytes(cand.size_bytes),
                                                cand.modified_str
                                            ))
                                            .size(11.0)
                                            .color(TEXT_MUTED),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add(epic_cta_button(
                                                    ph::ARROW_COUNTER_CLOCKWISE,
                                                    "Resurrect",
                                                ))
                                                .clicked()
                                            {
                                                app.resurrect_save_async(cand.path.clone());
                                            }
                                        },
                                    );
                                });
                            });
                            ui.add_space(5.0);
                        }
                    });
            }
        });
    });
}

fn render_compactor_and_bundles(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing.x = 14.0;
    ui.columns(2, |cols| {
        let col0_w = (cols[0].available_width() - 42.0).max(180.0);
        card_frame().show(&mut cols[0], |ui| {
            ui.set_width(col0_w);
            ui.set_max_width(col0_w);
            ui.set_min_height(310.0);
            ui.label(bold_text("Save Bloat Compactor", 15.5, TEXT_PRIMARY));
            ui.label(
                RichText::new("Purge cached online level data from CCLocalLevels.dat while keeping every editor level you built.")
                    .size(12.0)
                    .color(TEXT_SECONDARY),
            );
            ui.add_space(12.0);
            if let Some(ref b) = app.save_bloat {
                elevated_card_frame().show(ui, |ui| {
                    ui.set_width((col0_w - 28.0).max(140.0));
                    ui.label(bold_text(
                        &format!("CCLocalLevels.dat: {}", human_bytes(b.file_size_bytes)),
                        14.0,
                        TEXT_PRIMARY,
                    ));
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        badge_pill(
                            ui,
                            &format!("{} Total Entries", b.total_levels),
                            TEXT_PRIMARY,
                            BG_SURFACE,
                        );
                        badge_pill(
                            ui,
                            &format!("{} User Created", b.user_created),
                            STATUS_OK,
                            STATUS_OK_SOFT,
                        );
                        badge_pill(
                            ui,
                            &format!("{} Cached Online Bloat", b.online_cached),
                            if b.online_cached > 0 {
                                STATUS_WARN
                            } else {
                                TEXT_MUTED
                            },
                            if b.online_cached > 0 {
                                STATUS_WARN_SOFT
                            } else {
                                BG_SURFACE
                            },
                        );
                    });
                });
            } else {
                empty_state_card(
                    ui,
                    ph::LAYERS,
                    "Save Bloat Report Unavailable",
                    "CCLocalLevels.dat was not found or could not be decoded.",
                );
            }
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(epic_cta_button(ph::WRENCH, "Compact Save Now"))
                    .on_hover_text("Creates timestamped .bak first, then strips cached online levels")
                    .clicked()
                {
                    app.compact_save_bloat_async();
                }
                if ui
                    .add(epic_secondary_button(ph::ARROWS_CLOCKWISE, "Re-Analyze"))
                    .clicked()
                {
                    app.refresh_backups_async();
                }
            });
        });

        let col1_w = (cols[1].available_width() - 42.0).max(180.0);
        card_frame().show(&mut cols[1], |ui| {
            ui.set_width(col1_w);
            ui.set_max_width(col1_w);
            ui.set_min_height(310.0);
            ui.label(bold_text("Portable .geodesync Bundles", 15.5, TEXT_PRIMARY));
            ui.label(
                RichText::new("Transfer or restore complete setups (.dat saves, .geode mods, and mod configs) in one file.")
                    .size(12.0)
                    .color(TEXT_SECONDARY),
            );
            ui.add_space(10.0);
            for (lbl, flag) in [
                ("Include GD Save Files (.dat)", &mut app.ui_state.bundle_include_saves),
                ("Include Geode Mods (.geode)", &mut app.ui_state.bundle_include_mods),
                ("Include Mod Configurations", &mut app.ui_state.bundle_include_configs),
            ] {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(lbl).size(12.5).color(TEXT_PRIMARY));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        epic_toggle_switch(ui, flag);
                    });
                });
                ui.add_space(6.0);
            }
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(epic_cta_button(ph::EXPORT, "Export .geodesync"))
                    .clicked()
                {
                    if let Some(dst) = rfd::FileDialog::new()
                        .set_file_name("GeoDeck_Setup.geodesync")
                        .add_filter("GeodeSync Bundle", &["geodesync", "zip"])
                        .save_file()
                    {
                        app.export_bundle_async(dst);
                    }
                }
                if ui
                    .add(epic_secondary_button(ph::UPLOAD_SIMPLE, "Import .geodesync"))
                    .clicked()
                {
                    if let Some(src) = rfd::FileDialog::new()
                        .add_filter("GeodeSync Bundle", &["geodesync", "zip"])
                        .pick_file()
                    {
                        app.import_bundle_async(src);
                    }
                }
            });
        });
    });
}
