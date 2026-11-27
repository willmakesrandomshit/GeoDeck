use eframe::egui::{self, RichText};

use crate::core::config::{GameInstallation, InstallationKind};
use crate::core::platform::open_in_file_manager;
use crate::launcher::detection::probe_installation;
use crate::launcher::versions::{
    delete_slot_directory, find_downloaded_steam_depots, open_steam_console_for_manifest,
    trigger_steam_validate,
};
use crate::ui::app::GeodeDeckApp;
use crate::ui::icons::ph;
use crate::ui::theme::*;

pub fn render(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.vertical(|ui| {
            ui.label(display_text("Installations & Version Slots", 22.0, TEXT_PRIMARY));
            ui.label(
                RichText::new(
                    "Manage isolated Geometry Dash versions, pin mod profiles, and verify game binaries.",
                )
                .size(12.5)
                .color(TEXT_SECONDARY),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(epic_secondary_button(
                    if app.manifest_locked { ph::LOCK } else { ph::LOCK_OPEN },
                    if app.manifest_locked { "Steam Lock: ON" } else { "Steam Lock: OFF" },
                ))
                .on_hover_text("Protect appmanifest_322170.acf from Steam auto-updates")
                .clicked()
            {
                app.toggle_manifest_lock_action();
            }
            if ui
                .add(epic_secondary_button(ph::MAGNIFYING_GLASS, "Scan Drives"))
                .on_hover_text("Scan Steam libraries, OneDrive, and user folders for GD installs")
                .clicked()
            {
                app.scan_discovered_installs_async();
            }
            if ui
                .add(epic_cta_button(ph::PLUS, "Add Install Folder"))
                .on_hover_text("Register an existing Geometry Dash folder")
                .clicked()
            {
                if let Some(folder) = rfd::FileDialog::new()
                    .set_title("Select Geometry Dash Installation Folder")
                    .pick_folder()
                {
                    let path = folder.to_string_lossy().into_owned();
                    let probe = probe_installation(&path);
                    if !probe.exe_exists {
                        app.push_toast(
                            crate::core::worker::ToastKind::Error,
                            "Invalid Directory",
                            "GeometryDash.exe was not found in the selected folder.",
                        );
                    } else {
                        let name = folder
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "Custom GD".into());
                        let id = format!("custom-{}", chrono::Local::now().format("%H%M%S"));
                        app.config.installations.push(GameInstallation {
                            id: id.clone(),
                            name,
                            path,
                            kind: InstallationKind::Custom,
                            custom_args: String::new(),
                            geode_safe_mode: false,
                            notes: "User-added Geometry Dash installation".into(),
                            pinned_mod_profile: None,
                            detected_gd_version: probe.gd_version,
                            detected_geode_version: probe.geode_version,
                        });
                        app.switch_active_installation(&id);
                    }
                }
            }
        });
    });

    ui.add_space(12.0);

    let active_id = app.config.active_installation_id.clone();
    let mut installs = app.config.installations.clone();
    installs.sort_by_key(|install| install.id != active_id);
    let mut activate = None;
    let mut profile_updates = Vec::new();

    card_frame().show(ui, |ui| {
        if installs.is_empty() {
            empty_state_card(
                ui,
                ph::SQUARES_FOUR,
                "No Geometry Dash Installs Found",
                "Click 'Add Install Folder' above to point GeoDeck to your Geometry Dash directory.",
            );
        } else {
            let len = installs.len();
            for (idx, install) in installs.iter().enumerate() {
                let active = install.id == active_id;
                hover_row(ui, egui::Sense::hover(), |ui| {
                    ui.horizontal(|ui| {
                        icon_tile(
                            ui,
                            ph::GAME_CONTROLLER,
                            38.0,
                            if active { STATUS_OK } else { TEXT_SECONDARY },
                            if active {
                                STATUS_OK_SOFT
                            } else {
                                BG_ELEVATED
                            },
                        );
                        ui.add_space(6.0);

                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 220.0).max(160.0));
                            ui.horizontal_wrapped(|ui| {
                                ui.add(
                                    egui::Label::new(bold_text(&install.name, 14.0, TEXT_PRIMARY))
                                        .truncate(),
                                );
                                if active {
                                    status_dot_pill(ui, "Active", STATUS_OK, STATUS_OK_SOFT);
                                }
                                ui.label(mono_text(install.kind.label(), 11.0, TEXT_MUTED));
                            });
                            let version = install.detected_gd_version.as_deref().unwrap_or("Unknown");
                            let geode = install
                                .detected_geode_version
                                .as_ref()
                                .map_or_else(|| "Geode not detected".into(), |v| format!("Geode {v}"));
                            ui.add(
                                egui::Label::new(
                                    mono_text(
                                        format!("GD {version}  ·  {geode}  ·  {}", install.path),
                                        11.0,
                                        TEXT_MUTED,
                                    ),
                                )
                                .truncate(),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            ui.menu_button(
                                RichText::new("Options").size(12.0).color(TEXT_PRIMARY),
                                |ui| {
                                    if ui.button("Open Folder").clicked() {
                                        open_in_file_manager(std::path::Path::new(&install.path));
                                        ui.close_menu();
                                    }
                                    let mut profile = install.pinned_mod_profile.clone();
                                    let selected = app
                                        .config
                                        .mod_profiles
                                        .iter()
                                        .find(|p| Some(&p.id) == profile.as_ref())
                                        .map_or("Default", |p| p.name.as_str());
                                    egui::ComboBox::from_id_salt(("install_profile", &install.id))
                                        .selected_text(format!("Mod profile: {selected}"))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut profile, None, "Default");
                                            for item in &app.config.mod_profiles {
                                                ui.selectable_value(
                                                    &mut profile,
                                                    Some(item.id.clone()),
                                                    &item.name,
                                                );
                                            }
                                        });
                                    if profile != install.pinned_mod_profile {
                                        profile_updates.push((install.id.clone(), profile));
                                    }
                                    if install.kind != InstallationKind::Steam
                                        && !active
                                        && ui
                                            .add(epic_danger_button(
                                                ph::TRASH,
                                                "Delete Install Folder",
                                            ))
                                            .clicked()
                                    {
                                        app.ui_state.pending_delete_install = Some(install.clone());
                                        ui.close_menu();
                                    }
                                },
                            );
                            if active {
                                if ui
                                    .add_enabled(
                                        app.active_session.is_none(),
                                        epic_cta_button(ph::PLAY, "Play"),
                                    )
                                    .clicked()
                                {
                                    app.trigger_launch(app.config.launch_mode);
                                }
                            } else if ui
                                .add(epic_secondary_button(ph::CHECK_CIRCLE, "Set Active"))
                                .clicked()
                            {
                                activate = Some(install.id.clone());
                            }
                        });
                    });
                });
                if idx + 1 < len {
                    row_divider(ui);
                }
            }
        }
    });

    for (id, profile) in profile_updates {
        if let Some(install) = app.config.installations.iter_mut().find(|i| i.id == id) {
            install.pinned_mod_profile = profile;
        }
        app.config.save();
    }
    if let Some(id) = activate {
        app.switch_active_installation(&id);
    }

    if !app.discovered_installs.is_empty() {
        ui.add_space(12.0);
        card_frame().show(ui, |ui| {
            ui.label(bold_text(
                &format!("Discovered GD Installations ({})", app.discovered_installs.len()),
                15.0,
                TEXT_PRIMARY,
            ));
            ui.add_space(6.0);
            let disc_list = app.discovered_installs.clone();
            let len = disc_list.len();
            for (idx, disc) in disc_list.into_iter().enumerate() {
                let already = app
                    .config
                    .installations
                    .iter()
                    .any(|i| i.path.eq_ignore_ascii_case(&disc.path));
                hover_row(ui, egui::Sense::hover(), |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 150.0).max(160.0));
                            ui.horizontal_wrapped(|ui| {
                                ui.label(bold_text(&disc.name, 13.5, TEXT_PRIMARY));
                                ui.label(mono_text(
                                    format!("{}  ·  GD {}", disc.source, disc.version),
                                    11.0,
                                    TEXT_MUTED,
                                ));
                            });
                            ui.label(mono_text(&disc.path, 11.0, TEXT_MUTED));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if already {
                                badge_pill(ui, "Registered", STATUS_OK, STATUS_OK_SOFT);
                            } else if ui.add(epic_cta_button(ph::PLUS, "Register")).clicked() {
                                let probe = probe_installation(&disc.path);
                                let id = format!("disc-{}", chrono::Local::now().format("%H%M%S"));
                                app.config.installations.push(GameInstallation {
                                    id: id.clone(),
                                    name: format!("{} ({})", disc.name, disc.source),
                                    path: disc.path,
                                    kind: InstallationKind::Custom,
                                    custom_args: String::new(),
                                    geode_safe_mode: false,
                                    notes: "Auto-discovered installation".into(),
                                    pinned_mod_profile: None,
                                    detected_gd_version: probe.gd_version,
                                    detected_geode_version: probe.geode_version,
                                });
                                app.switch_active_installation(&id);
                            }
                        });
                    });
                });
                if idx + 1 < len {
                    row_divider(ui);
                }
            }
        });
    }

    ui.add_space(14.0);
    ui.label(bold_text("Advanced Install Tools", 15.5, TEXT_PRIMARY));
    ui.add_space(6.0);

    epic_subtab_bar(
        ui,
        &[
            (ph::SHIELD_CHECK, "Integrity & Loader"),
            (ph::COPY, "Clone Version Slot"),
            (ph::DOWNLOAD_SIMPLE, "Steam Depot"),
        ],
        &mut app.ui_state.library_subtab,
    );

    card_frame().show(ui, |ui| match app.ui_state.library_subtab {
        0 => {
            ui.label(
                RichText::new(
                    "Check Geometry Dash and Geode binaries for missing or modified files.",
                )
                .size(12.5)
                .color(TEXT_SECONDARY),
            );
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(epic_cta_button(ph::SHIELD_CHECK, "Verify Game Files"))
                    .clicked()
                {
                    app.run_integrity_check_async();
                }
                if ui
                    .add(epic_secondary_button(
                        ph::ARROW_CLOCKWISE,
                        "Validate with Steam",
                    ))
                    .clicked()
                {
                    trigger_steam_validate();
                }
                if ui
                    .add(epic_secondary_button(
                        ph::DOWNLOAD_SIMPLE,
                        "Check Geode Loader",
                    ))
                    .clicked()
                {
                    app.fetch_latest_loader();
                }
            });
            if !app.integrity_report.checked_at.is_empty() {
                ui.add_space(10.0);
                inset_frame().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        mono_text(
                            format!(
                                "Checked {}  ·  {} healthy  ·  {} missing  ·  {} changed",
                                app.integrity_report.checked_at,
                                app.integrity_report.healthy_count,
                                app.integrity_report.missing_count,
                                app.integrity_report.corrupted_count
                            ),
                            12.0,
                            if app.integrity_report.overall_ok {
                                STATUS_OK
                            } else {
                                STATUS_WARN
                            },
                        ),
                    );
                });
            }
            if let Some(release) = app.latest_loader_release.clone() {
                ui.add_space(10.0);
                row_divider(ui);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(semibold_text(
                        &format!(
                            "Geode {} available for GD {}",
                            release.version, release.gd_win
                        ),
                        13.0,
                        TEXT_PRIMARY,
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(epic_cta_button(ph::DOWNLOAD_SIMPLE, "Install Loader"))
                            .clicked()
                        {
                            app.install_loader_release_async(release);
                        }
                    });
                });
            }
        }
        1 => {
            ui.label(
                RichText::new(
                    "Create an isolated copy of the active installation so you can keep another game version.",
                )
                .size(12.5)
                .color(TEXT_SECONDARY),
            );
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(semibold_text("Slot Name:", 12.5, TEXT_SECONDARY));
                styled_text_input(ui, &mut app.ui_state.new_slot_name, "GD_Slot_Name", 220.0);
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                epic_toggle_switch(ui, &mut app.ui_state.clone_include_mods);
                ui.label(
                    RichText::new("Include Geode and installed mods")
                        .size(12.5)
                        .color(TEXT_PRIMARY),
                );
            });
            ui.add_space(12.0);
            if ui
                .add(epic_cta_button(ph::COPY, "Create Isolated Slot"))
                .clicked()
            {
                app.create_snapshot_slot_async();
            }
        }
        _ => {
            ui.label(
                RichText::new("Download an older Steam build into a separate version slot.")
                    .size(12.5)
                    .color(TEXT_SECONDARY),
            );
            ui.add_space(10.0);
            let depots = app.config.depots.clone();
            let selected = app
                .ui_state
                .selected_depot_idx
                .min(depots.len().saturating_sub(1));
            if let Some(depot) = depots.get(selected) {
                egui::ComboBox::from_id_salt("install_depot_choice")
                    .selected_text(&depot.label)
                    .show_ui(ui, |ui| {
                        for (index, item) in depots.iter().enumerate() {
                            ui.selectable_value(
                                &mut app.ui_state.selected_depot_idx,
                                index,
                                &item.label,
                            );
                        }
                    });
                ui.add_space(6.0);
                ui.label(RichText::new(&depot.note).size(12.0).color(TEXT_SECONDARY));
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add(epic_secondary_button(
                            ph::GAME_CONTROLLER,
                            "Open Steam Console",
                        ))
                        .clicked()
                    {
                        let command = open_steam_console_for_manifest(&depot.manifest);
                        ui.output_mut(|out| out.copied_text = command.clone());
                        app.push_toast(
                            crate::core::worker::ToastKind::Info,
                            "Steam Command Copied",
                            command,
                        );
                    }
                    if ui
                        .add(epic_secondary_button(
                            ph::ARROWS_CLOCKWISE,
                            "Scan Depots",
                        ))
                        .clicked()
                    {
                        app.downloaded_depots = find_downloaded_steam_depots();
                    }
                });
            }

            ui.add_space(10.0);
            if app.downloaded_depots.is_empty() {
                ui.label(
                    RichText::new(
                        "No downloaded depots found yet. Run the command in Steam, then click Scan Depots.",
                    )
                    .size(12.0)
                    .color(TEXT_MUTED),
                );
            } else {
                let depots_found = app.downloaded_depots.clone();
                let found_idx = app
                    .ui_state
                    .selected_found_depot_idx
                    .min(depots_found.len().saturating_sub(1));
                ui.horizontal_wrapped(|ui| {
                    egui::ComboBox::from_id_salt("found_depot_folder")
                        .selected_text(depots_found[found_idx].display().to_string())
                        .show_ui(ui, |ui| {
                            for (index, path) in depots_found.iter().enumerate() {
                                ui.selectable_value(
                                    &mut app.ui_state.selected_found_depot_idx,
                                    index,
                                    path.display().to_string(),
                                );
                            }
                        });
                    if ui
                        .add(epic_cta_button(ph::DOWNLOAD_SIMPLE, "Import Depot as Slot"))
                        .clicked()
                    {
                        let chosen = depots_found[found_idx].clone();
                        let slot_name = app.ui_state.new_slot_name.trim().to_string();
                        let final_name = if slot_name.is_empty() {
                            format!("Depot_{}", chrono::Local::now().format("%Y%m%d_%H%M"))
                        } else {
                            slot_name
                        };
                        app.import_depot_slot_async(chosen, final_name);
                    }
                });
            }
        }
    });

    // === Delete Install Slot Confirmation Modal ===
    if let Some(install) = app.ui_state.pending_delete_install.clone() {
        modal_backdrop(ui.ctx(), "delete_install_bg");
        egui::Area::new(egui::Id::new("delete_install_fg"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                dialog_frame().show(ui, |ui| {
                    ui.set_max_width(420.0);
                    ui.horizontal(|ui| {
                        icon_tile(ui, ph::TRASH, 36.0, STATUS_ERR, STATUS_ERR_SOFT);
                        ui.vertical(|ui| {
                            ui.label(bold_text("Delete Isolated Install?", 15.5, TEXT_PRIMARY));
                            ui.label(mono_text(&install.name, 11.5, TEXT_MUTED));
                        });
                    });
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!("This will delete '{}' from disk.", install.path))
                            .size(12.5)
                            .color(TEXT_SECONDARY),
                    );
                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(epic_danger_button(ph::TRASH, "Delete Folder"))
                            .clicked()
                        {
                            match delete_slot_directory(&install) {
                                Ok(()) => {
                                    app.config.installations.retain(|i| i.id != install.id);
                                    app.config.save();
                                    app.push_toast(
                                        crate::core::worker::ToastKind::Success,
                                        "Slot Removed",
                                        format!("Deleted isolated slot '{}'.", install.name),
                                    );
                                }
                                Err(error) => app.push_toast(
                                    crate::core::worker::ToastKind::Error,
                                    "Delete Failed",
                                    error,
                                ),
                            }
                            app.ui_state.pending_delete_install = None;
                        }
                        if ui.add(epic_secondary_button("", "Cancel")).clicked() {
                            app.ui_state.pending_delete_install = None;
                        }
                    });
                });
            });
    }
}
