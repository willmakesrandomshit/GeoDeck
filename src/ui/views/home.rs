use eframe::egui::{self, Color32, RichText, Rounding, Stroke, Vec2};

use crate::core::config::LaunchMode;
use crate::core::platform::open_in_file_manager;
use crate::geode::profiles::QuickModPreset;
use crate::launcher::detection::human_bytes;
use crate::ui::app::{GeodeDeckApp, ModsSubTab, NavTab};
use crate::ui::icons::ph;
use crate::ui::theme::*;

pub fn render(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    let install = app.config.active_installation().cloned();
    let probe = &app.probe;
    let geode = match &install {
        Some(install) if probe.path == install.path && probe.geode_installed => probe
            .geode_version
            .as_ref()
            .map_or_else(|| "Geode installed".into(), |v| format!("Geode {v}")),
        Some(install) if probe.path == install.path => "Geode not detected".into(),
        Some(install) => install.detected_geode_version.as_ref().map_or_else(
            || "Checking Geode".into(),
            |v| format!("Geode {v}"),
        ),
        None => "Geode unavailable".into(),
    };

    let art = app.game_art(ui.ctx());
    egui::Frame::none()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_PANEL))
        .inner_margin(egui::Margin::same(22.0))
        .show(ui, |ui| {
            let width = ui.available_width();
            let art_width = if art.is_some() {
                (width * 0.34).clamp(220.0, 324.0)
            } else {
                0.0
            };
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(
                        (width - art_width - ui.spacing().item_spacing.x).max(220.0),
                    );
                    ui.set_max_width(
                        (width - art_width - ui.spacing().item_spacing.x).max(220.0),
                    );
                    if let Some(install) = &install {
                        let version = install.detected_gd_version.as_deref().unwrap_or("Unknown");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            let (dot_r, _) =
                                ui.allocate_exact_size(Vec2::splat(7.0), egui::Sense::hover());
                            ui.painter().circle_filled(dot_r.center(), 2.8, STATUS_OK);
                            ui.label(mono_text(
                                format!("ACTIVE SLOT  ·  GD {version}  ·  {geode}"),
                                11.0,
                                TEXT_MUTED,
                            ));
                        });
                        ui.add_space(6.0);
                        ui.label(display_text("Geometry Dash", 32.0, TEXT_PRIMARY));
                        ui.add_space(4.0);
                        ui.label(RichText::new(&install.name).size(13.5).color(TEXT_SECONDARY));
                    } else {
                        ui.label(display_text("Geometry Dash", 32.0, TEXT_PRIMARY));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("No active installation configured")
                                .size(13.5)
                                .color(TEXT_SECONDARY),
                        );
                    }

                    ui.add_space(18.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if app.active_session.is_some() {
                            if ui.add(epic_danger_button(ph::STOP, "Stop Game")).clicked() {
                                app.stop_running_game();
                            }
                        } else if install.is_none() {
                            if ui
                                .add(epic_cta_button(ph::PLUS, "Manage Installs"))
                                .clicked()
                            {
                                app.navigate_to(NavTab::Library);
                            }
                        } else {
                            let mode = app.config.launch_mode;
                            let play_label = if mode == LaunchMode::SafeMode {
                                "Play Safe Mode"
                            } else {
                                "Play"
                            };
                            if ui
                                .add_sized([152.0, 40.0], epic_cta_button(ph::PLAY, play_label))
                                .clicked()
                            {
                                app.trigger_launch(mode);
                            }
                            ui.menu_button(
                                RichText::new(ph::CARET_DOWN)
                                    .size(14.0)
                                    .color(TEXT_PRIMARY),
                                |ui| {
                                    if ui.button("Play (Default Mode)").clicked() {
                                        ui.close_menu();
                                        app.trigger_launch(app.config.launch_mode);
                                    }
                                    if ui.button("Play in Geode Safe Mode").clicked() {
                                        ui.close_menu();
                                        app.trigger_launch(LaunchMode::SafeMode);
                                    }
                                    if let Some(ref inst) = install {
                                        if ui.button("Open Game Folder").clicked() {
                                            ui.close_menu();
                                            open_in_file_manager(std::path::Path::new(&inst.path));
                                        }
                                    }
                                },
                            )
                            .response
                            .on_hover_text("Additional launch options");
                        }
                    });

                    ui.add_space(12.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.menu_button(
                            semibold_text(
                                &format!("{} Preset: {}", ph::SLIDERS, app.config.active_mod_preset),
                                12.0,
                                TEXT_PRIMARY,
                            ),
                            |ui| {
                                for preset in [
                                    QuickModPreset::Boosted,
                                    QuickModPreset::Vanilla,
                                    QuickModPreset::Tournament,
                                    QuickModPreset::Globed,
                                ] {
                                    if ui.button(preset.label()).clicked() {
                                        ui.close_menu();
                                        app.apply_quick_preset_async(preset);
                                    }
                                }
                            },
                        )
                        .response
                        .on_hover_text("One-click Geode mod preset (Boosted, Vanilla, Tournament, Globed)");

                        let lock_icon = if app.manifest_locked {
                            ph::LOCK
                        } else {
                            ph::LOCK_OPEN
                        };
                        let lock_lbl = if app.manifest_locked {
                            "Steam Lock: ON"
                        } else {
                            "Steam Lock: OFF"
                        };
                        if ui
                            .add(epic_secondary_button(lock_icon, lock_lbl))
                            .on_hover_text("Protect appmanifest_322170.acf from unwanted Steam auto-updates")
                            .clicked()
                        {
                            app.toggle_manifest_lock_action();
                        }

                        if ui
                            .add(epic_secondary_button(ph::SHIELD_CHECK, "Health Check"))
                            .on_hover_text("Run 8-point diagnostic check (Ctrl+H)")
                            .clicked()
                        {
                            app.ui_state.tools_subtab = 0;
                            app.navigate_to(NavTab::Tools);
                            app.run_health_check_async();
                        }
                    });
                });

                if let Some(art) = &art {
                    let tex_sz = art.size_vec2();
                    let aspect = (tex_sz.y / tex_sz.x.max(1.0)).clamp(0.42, 0.70);
                    let draw_w = art_width;
                    let draw_h = (draw_w * aspect).min(182.0);
                    let img_resp = ui.add(
                        egui::Image::new(art)
                            .fit_to_exact_size(egui::vec2(draw_w, draw_h))
                            .rounding(RADIUS_CARD),
                    );
                    ui.painter().rect_stroke(
                        img_resp.rect,
                        Rounding::same(RADIUS_CARD),
                        Stroke::new(1.0_f32, BORDER_STRONG),
                    );
                }
            });
        });

    ui.add_space(12.0);

    // Live Player Progression Telemetry Strip
    if let Some(ref sum) = app.save_summary {
        let st = &sum.player_stats;
        let stars_sub = format!("{} Stars  ·  {} Moons", st.stars, st.moons);
        let demons_val = format!("{}", st.demons);
        let grinds_sub = format!("{} active grind(s)", sum.active_grinds.len());
        let diamonds_val = format!("{}", st.diamonds);
        let orbs_sub = format!("{} Orbs  ·  {} Coins", st.orbs, st.user_coins);
        let rpc_val = if !app.config.discord_rpc_enabled {
            "Disabled"
        } else if app.discord_rpc.is_connected() {
            "Connected"
        } else {
            "Waiting"
        };
        let rpc_col = if app.discord_rpc.is_connected() {
            STATUS_OK
        } else {
            TEXT_MUTED
        };

        telemetry_strip(
            ui,
            &[
                ("PLAYER", &sum.player_name, &stars_sub, ACCENT_EPIC_BRIGHT),
                ("DEMONS", &demons_val, &grinds_sub, STATUS_OK),
                ("DIAMONDS", &diamonds_val, &orbs_sub, TEXT_PRIMARY),
                ("DISCORD RPC", rpc_val, "Rich Presence IPC", rpc_col),
            ],
        );
        ui.add_space(12.0);
    }

    // Two-Column Workbench Overview: Readiness Links + Live Activity / Grinds
    ui.spacing_mut().item_spacing.x = 12.0;
    ui.columns(2, |cols| {
        let col0_w = (cols[0].available_width() - 38.0).max(180.0);
        card_frame().show(&mut cols[0], |ui| {
            ui.set_width(col0_w);
            ui.set_max_width(col0_w);
            ui.set_min_height(248.0);
            ui.label(bold_text("Workspace Readiness", 14.5, TEXT_PRIMARY));
            ui.label(
                RichText::new("Live status across backups, mods, audio cache, and setup diagnostics")
                    .size(11.5)
                    .color(TEXT_MUTED),
            );
            ui.add_space(8.0);

            let bkp_meta = app.snapshots.first().map_or_else(
                || "No snapshots created".to_string(),
                |s| format!("{} snapshot(s)  ·  Latest {}", app.snapshots.len(), s.created_at),
            );
            let bkp_col = if app.snapshots.is_empty() {
                STATUS_WARN
            } else {
                STATUS_OK
            };
            if readiness_row(
                ui,
                "Encrypted Vault",
                &bkp_meta,
                if app.snapshots.is_empty() { "Empty" } else { "Protected" },
                bkp_col,
            ) {
                app.navigate_to(NavTab::Backups);
            }
            row_divider(ui);

            let mods_meta = if app.mod_updates.is_empty() {
                format!("{} installed · {} conflict(s)", app.installed_mods.len(), app.mod_audit.errors)
            } else {
                format!("{} installed · {} update(s) ready", app.installed_mods.len(), app.mod_updates.len())
            };
            let mods_status = if !app.mod_updates.is_empty() {
                format!("{} update(s)", app.mod_updates.len())
            } else if app.mod_audit.errors > 0 {
                format!("{} issue(s)", app.mod_audit.errors)
            } else {
                "Synced".into()
            };
            let mods_col = if app.mod_audit.errors > 0 {
                STATUS_WARN
            } else if !app.mod_updates.is_empty() {
                ACCENT_EPIC_BRIGHT
            } else {
                STATUS_OK
            };
            if readiness_row(ui, "Geode Mods", &mods_meta, &mods_status, mods_col) {
                app.ui_state.mods_subtab = ModsSubTab::Installed;
                app.navigate_to(NavTab::Mods);
            }
            row_divider(ui);

            let songs_meta = format!(
                "{}/{} resolved  ·  {}",
                app.song_report.resolved_count,
                app.song_report.total_songs,
                human_bytes(app.song_report.total_bytes)
            );
            let nong_status = if app.song_report.nong_count > 0 {
                format!("{} NONG", app.song_report.nong_count)
            } else {
                "Cached".into()
            };
            if readiness_row(
                ui,
                "NONG Audio Cache",
                &songs_meta,
                &nong_status,
                ACCENT_EPIC_BRIGHT,
            ) {
                app.ui_state.tools_subtab = 3;
                app.navigate_to(NavTab::Tools);
            }
            row_divider(ui);

            let err_cnt = app
                .health_findings
                .iter()
                .filter(|f| f.severity == crate::tools::diagnostics::DiagnosticSeverity::Error)
                .count();
            let health_meta = format!(
                "{} slot(s)  ·  {}",
                app.config.installations.len(),
                if app.health_findings.is_empty() {
                    "8-point check ready".into()
                } else if err_cnt == 0 {
                    format!("All {} checks passed", app.health_findings.len())
                } else {
                    format!("{err_cnt} setup issue(s) flagged")
                }
            );
            let health_col = if err_cnt > 0 { STATUS_ERR } else { STATUS_OK };
            if readiness_row(
                ui,
                "Setup Health",
                &health_meta,
                if err_cnt > 0 { "Needs Review" } else { "Healthy" },
                health_col,
            ) {
                app.ui_state.tools_subtab = 0;
                app.navigate_to(NavTab::Tools);
            }
        });

        let col1_w = (cols[1].available_width() - 38.0).max(180.0);
        card_frame().show(&mut cols[1], |ui| {
            ui.set_width(col1_w);
            ui.set_max_width(col1_w);
            ui.set_min_height(248.0);

            let grinds: Vec<_> = app
                .save_summary
                .as_ref()
                .map(|s| s.active_grinds.iter().take(4).cloned().collect())
                .unwrap_or_default();

            if !grinds.is_empty() {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_max_width((col1_w - 135.0).max(100.0));
                        ui.label(bold_text("Active Demon Grinds", 14.5, TEXT_PRIMARY));
                        ui.label(
                            RichText::new("Normal-mode runs recorded in CCGameManager.dat")
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(epic_secondary_button(ph::CARET_RIGHT, "Demonlist"))
                            .clicked()
                        {
                            app.ui_state.demonlist_subtab = 1;
                            app.navigate_to(NavTab::Demonlist);
                        }
                    });
                });
                ui.add_space(10.0);

                let count = grinds.len();
                for (idx, g) in grinds.iter().enumerate() {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.set_max_width((col1_w - 75.0).max(100.0));
                                ui.add(
                                    egui::Label::new(semibold_text(&g.name, 13.0, TEXT_PRIMARY))
                                        .truncate(),
                                );
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "by {}  ·  {} attempt(s)",
                                            g.author, g.attempts
                                        ))
                                        .size(11.0)
                                        .color(TEXT_MUTED),
                                    )
                                    .truncate(),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(mono_text(
                                        format!("{}%", g.percent),
                                        13.0,
                                        ACCENT_EPIC_BRIGHT,
                                    ));
                                },
                            );
                        });
                        ui.add_space(5.0);
                        let bar_w = ui.available_width().max(60.0);
                        let (bar_rect, _) =
                            ui.allocate_exact_size(Vec2::new(bar_w, 3.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(bar_rect, Rounding::same(1.5), BG_ELEVATED);
                        let fill_w = (bar_w * (g.percent as f32 / 100.0)).clamp(3.0, bar_w);
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, 3.0)),
                            Rounding::same(1.5),
                            ACCENT_EPIC,
                        );
                    });
                    if idx + 1 < count {
                        ui.add_space(6.0);
                        row_divider(ui);
                        ui.add_space(4.0);
                    }
                }
            } else {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_max_width((col1_w - 150.0).max(100.0));
                        ui.label(bold_text("Recent Vault Snapshots", 14.5, TEXT_PRIMARY));
                        ui.label(
                            RichText::new("Latest DPAPI-encrypted Geometry Dash save archives")
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(epic_secondary_button(ph::FLOPPY_DISK, "Create Backup"))
                            .clicked()
                        {
                            app.trigger_manual_backup("Home Quick Backup");
                        }
                    });
                });
                ui.add_space(10.0);

                if app.snapshots.is_empty() {
                    empty_state_card(
                        ui,
                        ph::SHIELD_CHECK,
                        "No Snapshots Yet",
                        "Create your first encrypted vault snapshot to safeguard progress.",
                    );
                } else {
                    let snaps: Vec<_> = app.snapshots.iter().take(4).cloned().collect();
                    let count = snaps.len();
                    for (idx, snap) in snaps.into_iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.set_max_width((col1_w - 95.0).max(100.0));
                                ui.add(
                                    egui::Label::new(semibold_text(
                                        &snap.created_at,
                                        13.0,
                                        TEXT_PRIMARY,
                                    ))
                                    .truncate(),
                                );
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "{}  ·  {} file(s)",
                                            snap.trigger_reason, snap.file_count
                                        ))
                                        .size(11.0)
                                        .color(TEXT_MUTED),
                                    )
                                    .truncate(),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(mono_text(
                                        human_bytes(snap.total_bytes),
                                        11.5,
                                        TEXT_SECONDARY,
                                    ));
                                },
                            );
                        });
                        if idx + 1 < count {
                            row_divider(ui);
                        }
                    }
                }
            }
        });
    });
}

fn readiness_row(
    ui: &mut egui::Ui,
    title: &str,
    meta: &str,
    status: &str,
    status_col: Color32,
) -> bool {
    let resp = hover_row(ui, egui::Sense::click(), |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - 110.0).max(100.0));
                ui.label(semibold_text(title, 13.0, TEXT_PRIMARY));
                ui.add(
                    egui::Label::new(RichText::new(meta).size(11.5).color(TEXT_MUTED)).truncate(),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(RichText::new(ph::CARET_RIGHT).size(13.0).color(TEXT_MUTED));
                ui.label(mono_text(status, 11.0, status_col));
            });
        });
    });
    resp.clicked()
}
