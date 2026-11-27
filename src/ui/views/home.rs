use eframe::egui::{self, Color32, RichText, Rounding, Stroke};

use crate::core::config::LaunchMode;
use crate::core::platform::open_in_file_manager;
use crate::geode::profiles::QuickModPreset;
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
        None => "Geode status unavailable".into(),
    };

    let art = app.game_art(ui.ctx());
    egui::Frame::none()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER_STRONG))
        .rounding(Rounding::same(RADIUS_PANEL))
        .inner_margin(egui::Margin::same(24.0))
        .show(ui, |ui| {
            let width = ui.available_width();
            let art_width = if art.is_some() {
                (width * 0.37).min(330.0)
            } else {
                0.0
            };
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(
                        (width - art_width - ui.spacing().item_spacing.x).max(220.0),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("ACTIVE WORKSPACE")
                                .size(11.0)
                                .strong()
                                .color(ACCENT_EPIC_BRIGHT),
                        );
                        badge_pill(
                            ui,
                            "MAX PERFORMANCE",
                            ACCENT_EPIC_BRIGHT,
                            ACCENT_EPIC_SOFT,
                        );
                    });
                    ui.add_space(6.0);
                    ui.label(display_text("Geometry Dash", 35.0, TEXT_PRIMARY));
                    ui.add_space(10.0);
                    if let Some(install) = &install {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(semibold_text(&install.name, 15.0, TEXT_PRIMARY));
                            status_dot_pill(ui, "Active Slot", STATUS_OK, STATUS_OK_SOFT);
                        });
                        ui.add_space(5.0);
                        let version = install.detected_gd_version.as_deref().unwrap_or("Unknown");
                        ui.label(
                            RichText::new(format!("Game {version}   ·   {geode}"))
                                .size(13.0)
                                .color(TEXT_SECONDARY),
                        );
                    } else {
                        ui.label(RichText::new("No active install").color(TEXT_SECONDARY));
                    }

                    ui.add_space(22.0);
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
                                .add_sized([164.0, 44.0], epic_cta_button(ph::PLAY, play_label))
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
                    // Compact Quick Controls: Mod Preset + Steam Update Lock + Health Check
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
                            "Steam Update Locked"
                        } else {
                            "Steam Lock: Off"
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
                    egui::Frame::none()
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0_f32, BORDER_GLOW.gamma_multiply(0.5)))
                        .rounding(Rounding::same(16.0))
                        .show(ui, |ui| {
                            ui.add(
                                egui::Image::new(art)
                                    .fit_to_exact_size(egui::vec2(art_width, art_width * 0.64))
                                    .rounding(16.0),
                            );
                        });
                }
            });
        });

    ui.add_space(14.0);

    // Live Player Progression Strip from CCGameManager.dat
    if let Some(ref sum) = app.save_summary {
        let st = &sum.player_stats;
        ui.columns(4, |cols| {
            stat_tile(
                &mut cols[0],
                ph::STAR,
                "PLAYER STATS",
                &sum.player_name,
                &format!("{} Stars  ·  {} Moons", st.stars, st.moons),
                ACCENT_EPIC_BRIGHT,
            );
            stat_tile(
                &mut cols[1],
                ph::FIRE,
                "DEMONS",
                &format!("{}", st.demons),
                &format!("{} active grind(s)", sum.active_grinds.len()),
                STATUS_OK,
            );
            stat_tile(
                &mut cols[2],
                ph::DIAMOND,
                "DIAMONDS & ORBS",
                &format!("{}", st.diamonds),
                &format!("{} Orbs  ·  {} Coins", st.orbs, st.user_coins),
                TEXT_PRIMARY,
            );
            stat_tile(
                &mut cols[3],
                ph::SHIELD_CHECK,
                "DISCORD RPC",
                if !app.config.discord_rpc_enabled {
                    "Disabled"
                } else if app.discord_rpc.is_connected() {
                    "Connected"
                } else {
                    "Waiting"
                },
                "Rich Presence IPC",
                if app.discord_rpc.is_connected() {
                    STATUS_OK
                } else {
                    TEXT_MUTED
                },
            );
        });
        ui.add_space(14.0);
    }

    // 3x2 Quick Overview Grid
    ui.spacing_mut().item_spacing.x = 12.0;
    ui.columns(2, |columns| {
        let bkp_latest = app
            .snapshots
            .first()
            .map_or("No backup found".to_string(), |s| s.created_at.clone());
        if overview_tile(
            &mut columns[0],
            ph::SHIELD_CHECK,
            "Latest Vault Backup",
            &bkp_latest,
            STATUS_OK,
            STATUS_OK_SOFT,
            "Open Backups",
        ) {
            app.navigate_to(NavTab::Backups);
        }

        let mod_sub = if app.mod_updates.is_empty() {
            format!("{} mods installed", app.installed_mods.len())
        } else {
            format!("{} update(s) available", app.mod_updates.len())
        };
        if overview_tile(
            &mut columns[1],
            ph::PUZZLE_PIECE,
            "Geode Mods & Updates",
            &mod_sub,
            ACCENT_EPIC_BRIGHT,
            ACCENT_EPIC_SOFT,
            "Open Mods",
        ) {
            if app.mod_updates.is_empty() {
                app.check_mod_updates_async();
            }
            app.ui_state.mods_subtab = ModsSubTab::Installed;
            app.navigate_to(NavTab::Mods);
        }
    });

    ui.add_space(12.0);

    ui.columns(2, |columns| {
        let songs_sub = format!(
            "{} cached  ·  {} resolved",
            app.song_report.total_songs, app.song_report.resolved_count
        );
        if overview_tile(
            &mut columns[0],
            ph::MUSIC_NOTE,
            "Audio & NONG Library",
            &songs_sub,
            ACCENT_EPIC_BRIGHT,
            ACCENT_EPIC_SOFT,
            "Open Audio & NONG Library",
        ) {
            app.ui_state.tools_subtab = 3;
            app.navigate_to(NavTab::Tools);
        }

        let demon_sub = if app.demonlist.is_empty() {
            "Pointercrate Top 150 & Grinds".into()
        } else {
            format!("{} demons loaded · Top 150", app.demonlist.len())
        };
        if overview_tile(
            &mut columns[1],
            ph::STAR,
            "Demonlist & Grinds",
            &demon_sub,
            STATUS_WARN,
            STATUS_WARN_SOFT,
            "Open Pointercrate Demonlist",
        ) {
            app.navigate_to(NavTab::Demonlist);
        }
    });

    ui.add_space(12.0);

    ui.columns(2, |columns| {
        let inst_count = app.config.installations.len();
        let inst_sub = format!(
            "{inst_count} slot(s)  ·  {}",
            if app.probe.geode_installed {
                "Geode Ready"
            } else {
                "Check Loader"
            }
        );
        let inst_fg = if app.probe.geode_installed {
            STATUS_OK
        } else {
            STATUS_WARN
        };
        let inst_bg = if app.probe.geode_installed {
            STATUS_OK_SOFT
        } else {
            STATUS_WARN_SOFT
        };
        if overview_tile(
            &mut columns[0],
            ph::SQUARES_FOUR,
            "Installs & Version Slots",
            &inst_sub,
            inst_fg,
            inst_bg,
            "Open Installs",
        ) {
            app.navigate_to(NavTab::Library);
        }

        let err_cnt = app
            .health_findings
            .iter()
            .filter(|f| f.severity == crate::tools::diagnostics::DiagnosticSeverity::Error)
            .count();
        let health_sub = if app.health_findings.is_empty() {
            "Run 8-point setup check".into()
        } else if err_cnt == 0 {
            format!("All {} checks healthy", app.health_findings.len())
        } else {
            format!("{err_cnt} setup issue(s) flagged")
        };
        if overview_tile(
            &mut columns[1],
            ph::PALETTE,
            "Textures & Setup Health",
            &health_sub,
            if err_cnt > 0 { STATUS_ERR } else { STATUS_OK },
            if err_cnt > 0 {
                STATUS_ERR_SOFT
            } else {
                STATUS_OK_SOFT
            },
            "Open Health Check",
        ) {
            app.ui_state.tools_subtab = 0;
            app.navigate_to(NavTab::Tools);
        }
    });
}

fn overview_tile(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    subtitle: &str,
    fg: Color32,
    bg: Color32,
    tooltip: &str,
) -> bool {
    let col_w = ui.available_width();
    let frame_resp = elevated_card_frame()
        .show(ui, |ui| {
            ui.set_width((col_w - 28.0).max(150.0));
            ui.horizontal(|ui| {
                icon_tile(ui, icon, 40.0, fg, bg);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.set_max_width((col_w - 130.0).max(100.0));
                    ui.label(semibold_text(title, 13.5, TEXT_PRIMARY));
                    ui.add(
                        egui::Label::new(
                            RichText::new(subtitle).size(12.0).color(TEXT_SECONDARY),
                        )
                        .truncate(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(ph::CARET_RIGHT).size(15.0).color(TEXT_MUTED));
                });
            });
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_text(tooltip);

    if frame_resp.hovered() {
        ui.painter().rect_stroke(
            frame_resp.rect,
            Rounding::same(RADIUS_CARD),
            Stroke::new(1.0_f32, BORDER_GLOW.gamma_multiply(0.75)),
        );
    }
    frame_resp.clicked()
}
