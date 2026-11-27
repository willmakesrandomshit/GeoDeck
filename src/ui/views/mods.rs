use eframe::egui::{self, RichText, Rounding, Stroke};

use crate::geode::dependency::{IssueAction, IssueSeverity};
use crate::geode::package::{local_gd_data_dir, remove_installed_mod, set_mod_enabled};
use crate::launcher::detection::human_bytes;
use crate::ui::app::{GeodeDeckApp, ModsSubTab};
use crate::ui::icons::ph;
use crate::ui::theme::*;

pub fn render(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    section_heading(
        ui,
        "Geode Mods",
        "Manage installed .geode packages, browse the Geode Index, audit dependencies, and export .gdpack bundles.",
    );

    let mut sub_idx = match app.ui_state.mods_subtab {
        ModsSubTab::Installed => 0,
        ModsSubTab::BrowseIndex => 1,
        ModsSubTab::DependencyAudit => 2,
        ModsSubTab::ProfilesAndExport => 3,
    };
    let audit_total = app.mod_audit.errors + app.mod_audit.warnings;
    let audit_lbl = format!("Dependency Audit ({audit_total})");
    let inst_lbl = format!("Installed ({})", app.installed_mods.len());
    epic_subtab_bar(
        ui,
        &[
            (ph::PUZZLE_PIECE, &inst_lbl),
            (ph::GLOBE, "Browse Geode"),
            (ph::SHIELD_WARNING, &audit_lbl),
            (ph::PACKAGE, "Profiles & .gdpack"),
        ],
        &mut sub_idx,
    );
    app.ui_state.mods_subtab = match sub_idx {
        0 => ModsSubTab::Installed,
        1 => {
            if app.index_results.is_empty() {
                app.search_geode_index_async();
            }
            ModsSubTab::BrowseIndex
        }
        2 => ModsSubTab::DependencyAudit,
        _ => ModsSubTab::ProfilesAndExport,
    };

    match app.ui_state.mods_subtab {
        ModsSubTab::Installed => render_installed(app, ui),
        ModsSubTab::BrowseIndex => render_index_browser(app, ui),
        ModsSubTab::DependencyAudit => render_audit(app, ui),
        ModsSubTab::ProfilesAndExport => render_profiles_export(app, ui),
    }

    // === Mod Removal Confirmation Modal ===
    if let Some(mod_item) = app.ui_state.pending_remove_mod.clone() {
        modal_backdrop(ui.ctx(), "remove_mod_modal_bg");
        egui::Area::new(egui::Id::new("remove_mod_modal_fg"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                dialog_frame().show(ui, |ui| {
                    ui.set_max_width(420.0);
                    ui.horizontal(|ui| {
                        icon_tile(ui, ph::TRASH, 38.0, STATUS_ERR, STATUS_ERR_SOFT);
                        ui.vertical(|ui| {
                            ui.label(bold_text("Uninstall Geode Mod?", 16.0, TEXT_PRIMARY));
                            ui.label(
                                RichText::new(format!("{}  ·  {}", mod_item.name, mod_item.version))
                                    .size(12.0)
                                    .color(TEXT_MUTED),
                            );
                        });
                    });
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!(
                            "This deletes '{}' from your active installation's geode/mods directory.",
                            mod_item.id
                        ))
                        .size(12.5)
                        .color(TEXT_SECONDARY),
                    );
                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(epic_danger_button(ph::TRASH, "Uninstall Mod"))
                            .clicked()
                        {
                            if crate::launcher::process::is_gd_running() {
                                app.push_toast(
                                    crate::core::worker::ToastKind::Warning,
                                    "Game Running",
                                    "Close Geometry Dash before removing .geode packages.",
                                );
                            } else {
                                match remove_installed_mod(&mod_item) {
                                    Ok(_) => {
                                        app.push_toast(
                                            crate::core::worker::ToastKind::Success,
                                            "Mod Removed",
                                            format!("Removed {}.", mod_item.name),
                                        );
                                        app.refresh_mods_async();
                                    }
                                    Err(e) => app.push_toast(
                                        crate::core::worker::ToastKind::Error,
                                        "Remove Failed",
                                        e,
                                    ),
                                }
                            }
                            app.ui_state.pending_remove_mod = None;
                        }
                        if ui.add(epic_secondary_button("", "Cancel")).clicked() {
                            app.ui_state.pending_remove_mod = None;
                        }
                    });
                });
            });
    }
}

fn render_installed(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    card_frame().show(ui, |ui| {
        let total_mods = app.installed_mods.len();
        let enabled_mods = app.installed_mods.iter().filter(|m| m.enabled).count();
        let disabled_mods = total_mods.saturating_sub(enabled_mods);

        ui.horizontal_wrapped(|ui| {
            let search_w = (ui.available_width() - 360.0).clamp(160.0, 420.0);
            styled_search_bar(
                ui,
                &mut app.ui_state.mod_search,
                "Search installed mods, IDs, or developers...",
                search_w,
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if ui
                    .add(epic_cta_button(ph::CLOUD_ARROW_DOWN, "Check Updates"))
                    .clicked()
                {
                    app.check_mod_updates_async();
                }
                if ui
                    .add(epic_secondary_button(ph::ARROWS_CLOCKWISE, "Refresh"))
                    .clicked()
                {
                    app.refresh_mods_async();
                }
                if ui
                    .add(epic_secondary_button(ph::FOLDER_OPEN, "Folder"))
                    .on_hover_text("Open <game>/geode/mods")
                    .clicked()
                {
                    let p = std::path::Path::new(&app.config.game_path)
                        .join("geode")
                        .join("mods");
                    let _ = std::process::Command::new("explorer").arg(p).spawn();
                }
            });
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            for (idx, label) in [
                format!("All ({total_mods})"),
                format!("Enabled ({enabled_mods})"),
                format!("Disabled ({disabled_mods})"),
            ]
            .into_iter()
            .enumerate()
            {
                let selected = app.ui_state.mod_filter_mode == idx;
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
                    .inner_margin(egui::Margin::symmetric(12.0, 5.0))
                    .show(ui, |ui| {
                        ui.label(semibold_text(
                            label,
                            12.0,
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
                    app.ui_state.mod_filter_mode = idx;
                }
            }
        });

        if !app.mod_updates.is_empty() {
            ui.add_space(12.0);
            elevated_card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(bold_text(
                        &format!(
                            "{} Compatible Update(s) Available",
                            app.mod_updates.len()
                        ),
                        14.0,
                        ACCENT_EPIC_BRIGHT,
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(epic_cta_button(ph::DOWNLOAD_SIMPLE, "Update All"))
                            .clicked()
                        {
                            app.update_all_compatible_mods_async();
                        }
                    });
                });
                ui.add_space(6.0);
                for upd in app.mod_updates.clone() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "{}  ({} -> {})",
                                upd.mod_name, upd.current_version, upd.target_version
                            ))
                            .size(12.5)
                            .color(TEXT_PRIMARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(epic_secondary_button(ph::DOWNLOAD_SIMPLE, "Update"))
                                .clicked()
                            {
                                app.install_specific_mod_version_async(
                                    upd.mod_id,
                                    upd.mod_name,
                                    upd.target_version,
                                    upd.download_link,
                                    upd.expected_hash,
                                );
                            }
                        });
                    });
                    ui.add_space(3.0);
                }
            });
        }

        ui.add_space(12.0);
        let q = app.ui_state.mod_search.trim().to_lowercase();
        let filter_mode = app.ui_state.mod_filter_mode;
        let mods = app.installed_mods.clone();
        let gd_running = crate::launcher::process::is_gd_running();

        if mods.is_empty() {
            empty_state_card(
                ui,
                ph::PUZZLE_PIECE,
                "No Geode Mods Installed",
                "Install mods from the 'Browse Geode' tab or drop .geode packages into <game>/geode/mods.",
            );
            return;
        }

        let mut toggled: Option<(crate::geode::package::InstalledMod, bool)> = None;

        let filtered: Vec<_> = mods
            .into_iter()
            .filter(|m| {
                if filter_mode == 1 && !m.enabled {
                    return false;
                }
                if filter_mode == 2 && m.enabled {
                    return false;
                }
                if !q.is_empty()
                    && !m.name.to_lowercase().contains(&q)
                    && !m.id.to_lowercase().contains(&q)
                    && !m.developer.to_lowercase().contains(&q)
                {
                    return false;
                }
                true
            })
            .collect();

        if filtered.is_empty() {
            empty_state_card(
                ui,
                ph::MAGNIFYING_GLASS,
                "No Matching Mods",
                "No installed mods match your search query or enabled/disabled filter.",
            );
            return;
        }

        egui::ScrollArea::vertical()
            .max_height(460.0)
            .show(ui, |ui| {
                for m in &filtered {
                    let frame_resp = elevated_card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let logo = app.mod_logo_texture(&m.id, m.logo_rgba.as_ref(), ui.ctx());
                            if let Some(logo) = logo {
                                ui.add(
                                    egui::Image::new(&logo)
                                        .fit_to_exact_size(egui::vec2(46.0, 46.0))
                                        .rounding(RADIUS_ICON),
                                );
                            } else {
                                icon_tile(
                                    ui,
                                    ph::PUZZLE_PIECE,
                                    46.0,
                                    ACCENT_EPIC_BRIGHT,
                                    ACCENT_EPIC_SOFT,
                                );
                            }

                            ui.add_space(6.0);
                            ui.vertical(|ui| {
                                ui.set_width((ui.available_width() - 190.0).max(140.0));
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(bold_text(&m.name, 14.5, TEXT_PRIMARY));
                                    badge_pill(ui, &m.version, TEXT_PRIMARY, BG_SURFACE);
                                    if let Some(ref gd) = m.gd_win_target {
                                        let matches = app
                                            .probe
                                            .gd_version
                                            .as_deref()
                                            .map(|a| gd == "*" || gd == a)
                                            .unwrap_or(true);
                                        badge_pill(
                                            ui,
                                            &format!("GD {gd}"),
                                            if matches { TEXT_SECONDARY } else { STATUS_ERR },
                                            if matches { BG_SURFACE } else { STATUS_ERR_SOFT },
                                        );
                                    }
                                    if m.is_api {
                                        badge_pill(
                                            ui,
                                            "API",
                                            ACCENT_EPIC_BRIGHT,
                                            ACCENT_EPIC_SOFT,
                                        );
                                    }
                                });
                                ui.label(
                                    RichText::new(format!(
                                        "{}  ·  by {}  ·  {}",
                                        m.id,
                                        m.developer,
                                        human_bytes(m.file_size)
                                    ))
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                                );
                                if !m.description.is_empty() {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&m.description)
                                                .size(12.0)
                                                .color(TEXT_SECONDARY),
                                        )
                                        .truncate(),
                                    );
                                }
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;
                                    if ui
                                        .add(epic_danger_button(ph::TRASH, ""))
                                        .on_hover_text("Uninstall Mod")
                                        .clicked()
                                    {
                                        app.ui_state.pending_remove_mod = Some(m.clone());
                                    }
                                    let mut enabled_val = m.enabled;
                                    if epic_toggle_switch(ui, &mut enabled_val)
                                        .on_hover_text(if m.enabled {
                                            "Disable mod in saved.json"
                                        } else {
                                            "Enable mod in saved.json"
                                        })
                                        .changed()
                                    {
                                        toggled = Some((m.clone(), enabled_val));
                                    }
                                    if m.has_user_settings
                                        && ui
                                            .add(epic_secondary_button(ph::SLIDERS, "Config"))
                                            .on_hover_text("Open mod config directory")
                                            .clicked()
                                    {
                                        let cfg_dir =
                                            local_gd_data_dir().join("geode/mods").join(&m.id);
                                        let _ = std::process::Command::new("explorer")
                                            .arg(cfg_dir)
                                            .spawn();
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

        if let Some((mod_item, want)) = toggled {
            if gd_running {
                app.push_toast(
                    crate::core::worker::ToastKind::Warning,
                    "Game Running",
                    "Close Geometry Dash before toggling mods so Geode does not overwrite saved.json.",
                );
            } else {
                match set_mod_enabled(&mod_item, want) {
                    Ok(_) => {
                        app.push_toast(
                            crate::core::worker::ToastKind::Success,
                            if want { "Mod Enabled" } else { "Mod Disabled" },
                            format!("{} updated in geode.loader/saved.json.", mod_item.name),
                        );
                        app.refresh_mods_async();
                    }
                    Err(e) => {
                        app.push_toast(crate::core::worker::ToastKind::Error, "Toggle Failed", e)
                    }
                }
            }
        }
    });
}

fn render_index_browser(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            styled_search_bar(
                ui,
                &mut app.ui_state.index_query,
                "Search Geode Index...",
                250.0,
            );
            egui::ComboBox::from_id_salt("epic_index_sort")
                .selected_text(&app.ui_state.index_sort)
                .show_ui(ui, |ui| {
                    for s in ["downloads", "recently_updated", "recently_published", "name"] {
                        ui.selectable_value(&mut app.ui_state.index_sort, s.into(), s);
                    }
                });
            ui.checkbox(&mut app.ui_state.index_match_gd, "Match active GD");
            ui.checkbox(&mut app.ui_state.index_featured_only, "Featured");
            if ui
                .add(epic_cta_button(ph::MAGNIFYING_GLASS, "Search"))
                .clicked()
            {
                app.ui_state.index_page = 1;
                app.search_geode_index_async();
            }
        });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{} mods in catalog  ·  Page {}",
                    app.index_total, app.ui_state.index_page
                ))
                .size(12.0)
                .color(TEXT_MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if ui
                    .add(epic_secondary_button(ph::CARET_RIGHT, "Next"))
                    .clicked()
                {
                    app.ui_state.index_page += 1;
                    app.search_geode_index_async();
                }
                if app.ui_state.index_page > 1
                    && ui
                        .add(epic_secondary_button(ph::CARET_LEFT, "Prev"))
                        .clicked()
                {
                    app.ui_state.index_page -= 1;
                    app.search_geode_index_async();
                }
            });
        });
        ui.add_space(10.0);

        let installed_ids: std::collections::HashSet<String> =
            app.installed_mods.iter().map(|m| m.id.clone()).collect();
        let results = app.index_results.clone();

        if results.is_empty() {
            empty_state_card(
                ui,
                ph::GLOBE,
                "No Index Results Loaded",
                "Press Search to query the live Geode Index v1 API.",
            );
            return;
        }

        egui::ScrollArea::vertical()
            .max_height(430.0)
            .show(ui, |ui| {
                for item in results {
                    let frame_resp = elevated_card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width((ui.available_width() - 140.0).max(160.0));
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(bold_text(&item.name, 14.5, TEXT_PRIMARY));
                                    if let Some(latest) = item.versions.first() {
                                        badge_pill(
                                            ui,
                                            &format!("v{}", latest.version),
                                            TEXT_PRIMARY,
                                            BG_SURFACE,
                                        );
                                        if let Some(ref gd) = latest.gd_win {
                                            badge_pill(
                                                ui,
                                                &format!("GD {gd}"),
                                                ACCENT_EPIC_BRIGHT,
                                                ACCENT_EPIC_SOFT,
                                            );
                                        }
                                    }
                                    if item.featured {
                                        status_dot_pill(
                                            ui,
                                            "Featured",
                                            STATUS_OK,
                                            STATUS_OK_SOFT,
                                        );
                                    }
                                });
                                ui.label(
                                    RichText::new(format!(
                                        "{}  ·  by {}  ·  {} downloads",
                                        item.id, item.developer, item.download_count
                                    ))
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                                );
                                if !item.description.is_empty() {
                                    ui.label(
                                        RichText::new(&item.description)
                                            .size(12.0)
                                            .color(TEXT_SECONDARY),
                                    );
                                }
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let is_inst = installed_ids.contains(&item.id);
                                    if is_inst {
                                        if ui
                                            .add(epic_secondary_button(
                                                ph::ARROWS_CLOCKWISE,
                                                "Reinstall",
                                            ))
                                            .clicked()
                                        {
                                            app.install_mod_by_id_async(item.id.clone());
                                        }
                                    } else if ui
                                        .add(epic_cta_button(ph::DOWNLOAD_SIMPLE, "Install"))
                                        .clicked()
                                    {
                                        app.install_mod_by_id_async(item.id.clone());
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
}

fn render_audit(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    let audit = app.mod_audit.clone();
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            icon_tile(
                ui,
                if audit.errors > 0 {
                    ph::WARNING
                } else {
                    ph::CHECK_CIRCLE
                },
                38.0,
                if audit.errors > 0 {
                    STATUS_ERR
                } else {
                    STATUS_OK
                },
                if audit.errors > 0 {
                    STATUS_ERR_SOFT
                } else {
                    STATUS_OK_SOFT
                },
            );
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.label(bold_text(
                    "Compatibility & Dependency Audit",
                    15.5,
                    TEXT_PRIMARY,
                ));
                ui.label(
                    RichText::new(format!(
                        "{} error(s)  ·  {} warning(s)",
                        audit.errors, audit.warnings
                    ))
                    .size(12.0)
                    .color(TEXT_SECONDARY),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(epic_secondary_button(ph::ARROWS_CLOCKWISE, "Re-Scan"))
                    .clicked()
                {
                    app.refresh_mods_async();
                }
            });
        });
        ui.add_space(12.0);

        if audit.issues.is_empty() {
            empty_state_card(
                ui,
                ph::CHECK_CIRCLE,
                "All Dependencies Satisfied",
                "All enabled Geode mods match your active GD binary, Geode ABI, and dependencies.",
            );
            return;
        }

        for issue in audit.issues {
            let (col, bg) = match issue.severity {
                IssueSeverity::Error => (STATUS_ERR, STATUS_ERR_SOFT),
                IssueSeverity::Warning => (STATUS_WARN, STATUS_WARN_SOFT),
            };
            elevated_card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 180.0).max(160.0));
                        ui.horizontal_wrapped(|ui| {
                            badge_pill(
                                ui,
                                if issue.severity == IssueSeverity::Error {
                                    "ERROR"
                                } else {
                                    "WARN"
                                },
                                col,
                                bg,
                            );
                            ui.label(bold_text(&issue.mod_name, 13.5, TEXT_PRIMARY));
                            ui.label(RichText::new(&issue.summary).color(col));
                        });
                        ui.label(RichText::new(&issue.detail).size(12.0).color(TEXT_SECONDARY));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match issue.action {
                            IssueAction::InstallMod(ref dep_id) => {
                                if ui
                                    .add(epic_cta_button(
                                        ph::DOWNLOAD_SIMPLE,
                                        &format!("Install {dep_id}"),
                                    ))
                                    .clicked()
                                {
                                    app.install_mod_by_id_async(dep_id.clone());
                                }
                            }
                            IssueAction::EnableMod(ref dep_id) => {
                                if ui
                                    .add(epic_cta_button(
                                        ph::CHECK_CIRCLE,
                                        &format!("Enable {dep_id}"),
                                    ))
                                    .clicked()
                                {
                                    let _ = crate::geode::package::set_mod_enabled_by_id(
                                        &app.config.game_path,
                                        dep_id,
                                        true,
                                        &local_gd_data_dir(),
                                    );
                                    app.refresh_mods_async();
                                }
                            }
                            IssueAction::SmartRepairMod(ref mod_id) => {
                                if ui
                                    .add(epic_cta_button(ph::WRENCH, "Smart Repair"))
                                    .clicked()
                                {
                                    app.install_mod_by_id_async(mod_id.clone());
                                }
                            }
                            IssueAction::DisableMod(ref mod_id) => {
                                if ui.add(epic_danger_button(ph::STOP, "Disable")).clicked() {
                                    let _ = crate::geode::package::set_mod_enabled_by_id(
                                        &app.config.game_path,
                                        mod_id,
                                        false,
                                        &local_gd_data_dir(),
                                    );
                                    app.refresh_mods_async();
                                }
                            }
                            IssueAction::None => {}
                        }
                    });
                });
            });
            ui.add_space(6.0);
        }
    });
}

fn render_profiles_export(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    ui.columns(2, |cols| {
        card_frame().show(&mut cols[0], |ui| {
            ui.set_min_height(320.0);
            ui.label(bold_text("Mod Profiles", 15.5, TEXT_PRIMARY));
            ui.label(
                RichText::new(
                    "Capture enabled mod sets and per-mod settings.json files into switchable presets.",
                )
                .size(12.0)
                .color(TEXT_SECONDARY),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(semibold_text("Name:", 12.5, TEXT_SECONDARY));
                styled_text_input(
                    ui,
                    &mut app.ui_state.new_profile_name,
                    "e.g. Demon Grinding",
                    220.0,
                );
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                epic_toggle_switch(ui, &mut app.ui_state.profile_snapshot_settings);
                ui.label(
                    RichText::new("Snapshot per-mod settings.json files")
                        .size(12.5)
                        .color(TEXT_PRIMARY),
                );
            });
            ui.add_space(10.0);
            if ui
                .add(epic_cta_button(ph::FLOPPY_DISK, "Capture Mod Profile"))
                .clicked()
            {
                let gd = app
                    .probe
                    .gd_version
                    .clone()
                    .unwrap_or_else(|| "unknown".into());
                match crate::geode::profiles::capture_mod_profile(
                    &app.ui_state.new_profile_name,
                    "Captured from GeoDeck",
                    &app.config.game_path,
                    &gd,
                    app.ui_state.profile_snapshot_settings,
                ) {
                    Ok(prof) => {
                        app.push_toast(
                            crate::core::worker::ToastKind::Success,
                            "Profile Captured",
                            format!("Saved '{}' ({} mods).", prof.name, prof.enabled_mods.len()),
                        );
                        app.config.mod_profiles.push(prof);
                        app.config.save();
                        app.ui_state.new_profile_name.clear();
                    }
                    Err(e) => app.push_toast(
                        crate::core::worker::ToastKind::Error,
                        "Capture Failed",
                        e,
                    ),
                }
            }

            ui.add_space(12.0);
            let profiles = app.config.mod_profiles.clone();
            let mut delete_idx: Option<usize> = None;
            if profiles.is_empty() {
                ui.label(
                    RichText::new("No saved profiles yet.")
                        .size(12.0)
                        .color(TEXT_MUTED),
                );
            }
            for (idx, prof) in profiles.iter().enumerate() {
                elevated_card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(bold_text(&prof.name, 13.5, TEXT_PRIMARY));
                            let enabled_cnt = prof.enabled_mods.values().filter(|&&b| b).count();
                            ui.label(
                                RichText::new(format!(
                                    "GD {}  ·  {} enabled / {} total",
                                    prof.target_gd_version,
                                    enabled_cnt,
                                    prof.enabled_mods.len()
                                ))
                                .size(11.5)
                                .color(TEXT_MUTED),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            if ui.add(epic_danger_button(ph::TRASH, "")).clicked() {
                                delete_idx = Some(idx);
                            }
                            if ui
                                .add(epic_cta_button(ph::CHECK_CIRCLE, "Apply"))
                                .clicked()
                            {
                                match crate::geode::profiles::apply_mod_profile(
                                    prof,
                                    &app.config.game_path,
                                    true,
                                ) {
                                    Ok(n) => {
                                        app.push_toast(
                                            crate::core::worker::ToastKind::Success,
                                            "Profile Applied",
                                            format!("Applied '{}': updated {n} mod(s).", prof.name),
                                        );
                                        app.refresh_mods_async();
                                    }
                                    Err(e) => app.push_toast(
                                        crate::core::worker::ToastKind::Error,
                                        "Apply Failed",
                                        e,
                                    ),
                                }
                            }
                        });
                    });
                });
                ui.add_space(5.0);
            }
            if let Some(i) = delete_idx {
                let p = app.config.mod_profiles.remove(i);
                crate::geode::profiles::delete_profile_snapshot(&p.id);
                app.config.save();
            }
        });

        card_frame().show(&mut cols[1], |ui| {
            ui.set_min_height(320.0);
            ui.label(bold_text("Portable .gdpack Studio", 15.5, TEXT_PRIMARY));
            ui.label(
                RichText::new(
                    "Bundle .geode packages + SHA-256 manifest + optional mod settings into one portable archive.",
                )
                .size(12.0)
                .color(TEXT_SECONDARY),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(semibold_text("Pack Name:", 12.5, TEXT_SECONDARY));
                styled_text_input(
                    ui,
                    &mut app.ui_state.export_pack_name,
                    "MyGeodePack",
                    210.0,
                );
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                epic_toggle_switch(ui, &mut app.ui_state.export_only_enabled);
                ui.label(
                    RichText::new("Export enabled mods only")
                        .size(12.5)
                        .color(TEXT_PRIMARY),
                );
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                epic_toggle_switch(ui, &mut app.ui_state.export_include_configs);
                ui.label(
                    RichText::new("Include mod settings.json files")
                        .size(12.5)
                        .color(TEXT_PRIMARY),
                );
            });
            ui.add_space(14.0);

            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(epic_cta_button(ph::EXPORT, "Export .gdpack.zip"))
                    .clicked()
                {
                    app.export_modpack_async();
                }
                if ui
                    .add(epic_secondary_button(
                        ph::UPLOAD_SIMPLE,
                        "Import .gdpack.zip",
                    ))
                    .clicked()
                {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("GeoDeck Mod Pack", &["zip"])
                        .pick_file()
                    {
                        app.import_modpack_async(file);
                    }
                }
                if ui
                    .add(epic_secondary_button(ph::PLUS, "Install .geode"))
                    .clicked()
                {
                    if let Some(file) = rfd::FileDialog::new()
                        .add_filter("Geode Mod Package", &["geode"])
                        .pick_file()
                    {
                        let dest_dir = std::path::Path::new(&app.config.game_path)
                            .join("geode")
                            .join("mods");
                        let _ = std::fs::create_dir_all(&dest_dir);
                        if let Some(fname) = file.file_name() {
                            match std::fs::copy(&file, dest_dir.join(fname)) {
                                Ok(_) => {
                                    app.push_toast(
                                        crate::core::worker::ToastKind::Success,
                                        "Local Mod Installed",
                                        format!("Installed {}", fname.to_string_lossy()),
                                    );
                                    app.refresh_mods_async();
                                }
                                Err(e) => app.push_toast(
                                    crate::core::worker::ToastKind::Error,
                                    "Install Failed",
                                    e.to_string(),
                                ),
                            }
                        }
                    }
                }
                if ui
                    .add(epic_secondary_button(ph::FOLDER_OPEN, "Open Exports"))
                    .clicked()
                {
                    let _ = std::fs::create_dir_all(&app.config.export_dir);
                    let _ = std::process::Command::new("explorer")
                        .arg(&app.config.export_dir)
                        .spawn();
                }
            });
        });
    });
}
