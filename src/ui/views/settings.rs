use eframe::egui::{self, RichText, Rounding, Stroke, Vec2};

use crate::core::config::LaunchMode;
use crate::core::logging;
use crate::core::updater::current_app_display_version;
use crate::launcher::process::build_launch_args;
use crate::ui::app::GeodeDeckApp;
use crate::ui::icons::ph;
use crate::ui::theme::*;

pub fn render(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    section_heading(
        ui,
        "Settings & About",
        "Configure launch behavior, vault/mod directories, automatic GitHub updates, and diagnostic logs.",
    );

    // === About GeoDeck Brand Identity Card ===
    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if let Some(ref icon) = app.brand_icon {
                egui::Frame::none()
                    .fill(BG_ELEVATED)
                    .stroke(Stroke::new(1.0_f32, BORDER_STRONG))
                    .rounding(Rounding::same(RADIUS_CARD))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.add(
                            egui::Image::new(icon)
                                .fit_to_exact_size(Vec2::splat(52.0))
                                .rounding(RADIUS_ICON),
                        );
                    });
            }
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - 220.0).max(220.0));
                if let Some(ref lockup) = app.brand_lockup {
                    ui.add(egui::Image::new(lockup).fit_to_exact_size(Vec2::new(160.0, 56.0)));
                } else {
                    ui.label(bold_text("GeoDeck", 18.0, TEXT_PRIMARY));
                }
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "All-in-one Geometry Dash Launcher, Version Slot Manager, Geode Mod Studio, and Encrypted Backup Vault.",
                    )
                    .size(12.5)
                    .color(TEXT_SECONDARY),
                );
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    badge_pill(
                        ui,
                        current_app_display_version(),
                        ACCENT_EPIC_BRIGHT,
                        ACCENT_EPIC_SOFT,
                    );
                    badge_pill(ui, "Max Performance (Auto)", ACCENT_EPIC_BRIGHT, ACCENT_EPIC_SOFT);
                    badge_pill(ui, "AES-256-GCM + DPAPI", STATUS_OK, STATUS_OK_SOFT);
                    badge_pill(ui, "Offline-First NONG Cache", TEXT_PRIMARY, BG_ELEVATED);
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(epic_cta_button(ph::ARROWS_CLOCKWISE, "Check Updates"))
                    .clicked()
                {
                    app.check_app_update_async();
                }
            });
        });
    });

    ui.add_space(14.0);

    // === Launch Flags & Directories ===
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            icon_tile(
                ui,
                ph::GEAR_SIX,
                38.0,
                ACCENT_EPIC_BRIGHT,
                ACCENT_EPIC_SOFT,
            );
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.label(bold_text("Launch Flags & Directories", 15.5, TEXT_PRIMARY));
                ui.label(
                    RichText::new("Customize Geode CLI flags, storage paths, and crash privacy")
                        .size(12.0)
                        .color(TEXT_SECONDARY),
                );
            });
        });
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(10.0);

        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label(semibold_text("Default Launch Mode:", 12.5, TEXT_SECONDARY));
            egui::ComboBox::from_id_salt("settings_launch_mode")
                .selected_text(app.config.launch_mode.label())
                .show_ui(ui, |ui| {
                    for mode in [
                        LaunchMode::DirectExe,
                        LaunchMode::SteamProtocol,
                        LaunchMode::SafeMode,
                    ] {
                        if ui
                            .selectable_value(&mut app.config.launch_mode, mode, mode.label())
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
        });

        ui.add_space(10.0);
        ui.label(bold_text("Native Geode Flags", 13.5, TEXT_PRIMARY));
        ui.add_space(6.0);
        for (lbl, flag) in [
            (
                "Always pass --geode:safe-mode (boots without mods)",
                &mut app.config.geode_flags.safe_mode,
            ),
            (
                "Pass --geode:disable-crash-handler",
                &mut app.config.geode_flags.disable_crash_handler,
            ),
            (
                "Pass --geode:enable-tulip-hook-logs",
                &mut app.config.geode_flags.enable_tulip_hook_logs,
            ),
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(lbl).size(12.5).color(TEXT_PRIMARY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if epic_toggle_switch(ui, flag).changed() {
                        changed = true;
                    }
                });
            });
            ui.add_space(4.0);
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(semibold_text("Extra CLI Args:", 12.5, TEXT_SECONDARY));
            if styled_text_input(
                ui,
                &mut app.config.geode_flags.extra_args,
                "--custom-flag",
                260.0,
            )
            .changed()
            {
                changed = true;
            }
        });

        let preview_args = build_launch_args(
            &app.config.geode_flags,
            app.config
                .active_installation()
                .map(|i| i.custom_args.as_str())
                .unwrap_or(""),
            app.config.launch_mode == LaunchMode::SafeMode,
        );
        ui.add_space(8.0);
        inset_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new(format!(
                    "Command Preview: GeometryDash.exe {}",
                    preview_args.join(" ")
                ))
                .size(11.5)
                .monospace()
                .color(TEXT_SECONDARY),
            );
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(10.0);

        for (label, field) in [
            ("Backups Vault Dir", 0usize),
            ("Version Slots Dir", 1usize),
            ("Mod Exports Dir", 2usize),
        ] {
            ui.horizontal(|ui| {
                ui.set_min_width(ui.available_width());
                ui.label(semibold_text(label, 12.5, TEXT_SECONDARY));
                let path_val = match field {
                    0 => &mut app.config.backup_dir,
                    1 => &mut app.config.versions_dir,
                    _ => &mut app.config.export_dir,
                };
                if styled_text_input(ui, path_val, "Directory path...", 300.0).changed() {
                    changed = true;
                }
                if ui
                    .add(epic_secondary_button(ph::FOLDER_OPEN, ""))
                    .on_hover_text("Browse Folder")
                    .clicked()
                {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        *path_val = dir.to_string_lossy().into_owned();
                        changed = true;
                    }
                }
            });
            ui.add_space(4.0);
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Redact Windows username paths in crash summaries")
                    .size(12.5)
                    .color(TEXT_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if epic_toggle_switch(ui, &mut app.config.redact_crash_usernames).changed() {
                    changed = true;
                }
            });
        });

        if changed {
            app.config.save();
        }
    });

    ui.add_space(14.0);

    // === Updater ===
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            icon_tile(
                ui,
                ph::CLOUD_ARROW_DOWN,
                38.0,
                ACCENT_EPIC_BRIGHT,
                ACCENT_EPIC_SOFT,
            );
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.label(bold_text(
                    &format!("GeoDeck Updater ({})", current_app_display_version()),
                    15.5,
                    TEXT_PRIMARY,
                ));
                ui.label(
                    RichText::new(
                        "Automatically checks GitHub Releases on every launch with SHA-256 verification",
                    )
                    .size(12.0)
                    .color(TEXT_SECONDARY),
                );
            });
        });
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(semibold_text("GitHub Repo:", 12.5, TEXT_SECONDARY));
            if styled_text_input(
                ui,
                &mut app.config.update_repo,
                "willmakesrandomshit/GeoDeck",
                270.0,
            )
            .changed()
            {
                app.config.save();
            }
            if ui
                .add(epic_cta_button(ph::ARROWS_CLOCKWISE, "Check Updates"))
                .clicked()
            {
                app.check_app_update_async();
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Check for verified updates automatically on every launch")
                    .size(12.5)
                    .color(TEXT_PRIMARY),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if epic_toggle_switch(ui, &mut app.config.check_updates_on_startup).changed() {
                    app.config.save();
                }
            });
        });

        if let Some(res) = app.app_update_result.clone() {
            ui.add_space(10.0);
            elevated_card_frame().show(ui, |ui| match res {
                Ok(info) => {
                    ui.label(bold_text(
                        &if info.update_available {
                            format!(
                                "Update available: {} -> {}",
                                info.current_version, info.latest_version
                            )
                        } else {
                            format!("Up to date ({})", info.current_version)
                        },
                        13.5,
                        if info.update_available {
                            STATUS_OK
                        } else {
                            TEXT_PRIMARY
                        },
                    ));
                    if info.update_available {
                        if let (Some(url), Some(name)) = (&info.asset_url, &info.asset_name) {
                            ui.add_space(6.0);
                            if ui
                                .add(epic_cta_button(
                                    ph::DOWNLOAD_SIMPLE,
                                    &format!("Download {name}"),
                                ))
                                .clicked()
                            {
                                app.download_app_update_async(
                                    url.clone(),
                                    name.clone(),
                                    info.expected_sha256.clone(),
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    ui.label(RichText::new(e).size(12.0).color(STATUS_WARN));
                }
            });
        }
    });

    ui.add_space(14.0);

    // === Diagnostic Log Console ===
    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(bold_text("Diagnostic Log Console", 15.5, TEXT_PRIMARY));
            for (idx, lbl) in ["All", "Info", "Success", "Warn", "Error"]
                .into_iter()
                .enumerate()
            {
                let selected = app.ui_state.log_level_filter == idx;
                let resp = egui::Frame::none()
                    .fill(if selected {
                        ACCENT_EPIC_SOFT
                    } else {
                        BG_ELEVATED
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if selected {
                            ACCENT_EPIC
                        } else {
                            BORDER_SUBTLE
                        },
                    ))
                    .rounding(Rounding::same(RADIUS_PILL))
                    .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                    .show(ui, |ui| {
                        ui.label(semibold_text(
                            lbl,
                            11.5,
                            if selected {
                                ACCENT_EPIC_BRIGHT
                            } else {
                                TEXT_SECONDARY
                            },
                        ));
                    })
                    .response
                    .interact(egui::Sense::click());
                if resp.clicked() {
                    app.ui_state.log_level_filter = idx;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if ui.add(epic_secondary_button(ph::TRASH, "Clear")).clicked() {
                    logging::clear_memory_entries();
                }
                if ui
                    .add(epic_secondary_button(ph::FOLDER_OPEN, "Open Log"))
                    .clicked()
                {
                    let p = logging::log_file_path();
                    if let Some(parent) = p.parent() {
                        let _ = std::process::Command::new("explorer").arg(parent).spawn();
                    }
                }
            });
        });
        ui.add_space(8.0);

        let filter_lvl = app.ui_state.log_level_filter;
        let entries: Vec<_> = logging::recent_entries()
            .into_iter()
            .filter(|e| match filter_lvl {
                1 => matches!(e.level, logging::LogLevel::Info),
                2 => matches!(e.level, logging::LogLevel::Success),
                3 => matches!(e.level, logging::LogLevel::Warn),
                4 => matches!(e.level, logging::LogLevel::Error),
                _ => true,
            })
            .collect();

        inset_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            egui::ScrollArea::vertical()
                .max_height(240.0)
                .show(ui, |ui| {
                    if entries.is_empty() {
                        ui.label(
                            RichText::new("No log entries match the selected filter.")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                    } else {
                        for item in entries {
                            let col = match item.level {
                                logging::LogLevel::Success => STATUS_OK,
                                logging::LogLevel::Warn => STATUS_WARN,
                                logging::LogLevel::Error => STATUS_ERR,
                                logging::LogLevel::Info => TEXT_SECONDARY,
                            };
                            ui.label(
                                RichText::new(format!(
                                    "[{}] [{}] [{}] {}",
                                    item.timestamp,
                                    item.level.tag(),
                                    item.category,
                                    item.message
                                ))
                                .size(11.5)
                                .monospace()
                                .color(col),
                            );
                        }
                    }
                });
        });
    });
}
