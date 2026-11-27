use eframe::egui::{self, RichText, Rounding, Stroke, Vec2};

use crate::backup::save_inspector::export_level_to_gmd;
use crate::core::config::LaunchMode;
use crate::geode::package::{local_gd_data_dir, set_mod_enabled_by_id};
use crate::launcher::detection::human_bytes;
use crate::tools::crash_analyzer::{delete_crash_log, format_redacted_summary};
use crate::tools::songs::{
    clear_manual_song_metadata, filter_and_sort_songs, install_nong_replacement,
    remove_broken_zero_byte_songs, restore_original_song, set_manual_song_metadata,
    SongCacheStatus, SongFormatFilter, SongSortMode, SongStatusFilter,
};
use crate::ui::app::GeodeDeckApp;
use crate::ui::icons::ph;
use crate::ui::theme::*;

pub fn render(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    section_heading(
        ui,
        "Diagnostics & Creator Tools",
        "Inspect Geode crashlogs, export local editor levels, and manage your NONG Song Library.",
    );

    let crash_lbl = format!("Crash Diagnostics ({})", app.crash_reports.len());
    let levels_lbl = format!(
        "Local Editor Levels ({})",
        app.save_summary.as_ref().map(|s| s.level_count).unwrap_or(0)
    );
    let songs_lbl = format!("NONG Song Library ({})", app.song_report.total_songs);

    epic_subtab_bar(
        ui,
        &[
            (ph::BUG, &crash_lbl),
            (ph::LAYERS, &levels_lbl),
            (ph::MUSIC_NOTE, &songs_lbl),
        ],
        &mut app.ui_state.tools_subtab,
    );

    match app.ui_state.tools_subtab {
        0 => render_crashes(app, ui),
        1 => render_levels(app, ui),
        _ => render_songs(app, ui),
    }
}

fn render_crashes(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            icon_tile(ui, ph::BUG, 38.0, STATUS_WARN, STATUS_WARN_SOFT);
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.label(bold_text("Geode Crash Log Analyzer", 16.0, TEXT_PRIMARY));
                ui.label(
                    RichText::new("Scans <game>/geode/crashlogs and identifies faulty modules")
                        .size(12.0)
                        .color(TEXT_SECONDARY),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(epic_secondary_button(ph::ARROWS_CLOCKWISE, "Rescan"))
                    .clicked()
                {
                    app.refresh_tools_async();
                }
            });
        });
        ui.add_space(12.0);

        if app.crash_reports.is_empty() {
            empty_state_card(
                ui,
                ph::CHECK_CIRCLE,
                "No Crash Logs Found",
                "Your Geometry Dash installation has no crash reports in <game>/geode/crashlogs.",
            );
            return;
        }

        for rep in app.crash_reports.clone() {
            elevated_card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(bold_text(&rep.faulty_mod_name, 14.5, STATUS_ERR));
                    badge_pill(ui, &rep.exception_code, STATUS_WARN, STATUS_WARN_SOFT);
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!("{}  ·  Loader {}", rep.timestamp, rep.loader_version))
                        .size(11.5)
                        .color(TEXT_MUTED),
                );
                if !rep.exception_details.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(&rep.exception_details)
                            .size(12.5)
                            .color(TEXT_SECONDARY),
                    );
                }
                if !rep.stack_trace.is_empty() {
                    ui.add_space(8.0);
                    inset_frame().show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        for frame in rep.stack_trace.iter().take(5) {
                            ui.label(
                                RichText::new(frame)
                                    .size(11.5)
                                    .monospace()
                                    .color(TEXT_SECONDARY),
                            );
                        }
                    });
                }
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    if let Some(ref mod_id) = rep.faulty_mod_id {
                        if ui
                            .add(epic_cta_button(ph::STOP, "Disable Faulty Mod"))
                            .clicked()
                        {
                            match set_mod_enabled_by_id(
                                &app.config.game_path,
                                mod_id,
                                false,
                                &local_gd_data_dir(),
                            ) {
                                Ok(_) => {
                                    app.push_toast(
                                        crate::core::worker::ToastKind::Success,
                                        "Faulty Mod Disabled",
                                        format!("Disabled {mod_id} in saved.json."),
                                    );
                                    app.refresh_mods_async();
                                }
                                Err(e) => app.push_toast(
                                    crate::core::worker::ToastKind::Error,
                                    "Disable Failed",
                                    e,
                                ),
                            }
                        }
                    }
                    if ui
                        .add(epic_secondary_button(
                            ph::SHIELD_WARNING,
                            "Launch Safe Mode",
                        ))
                        .clicked()
                    {
                        app.trigger_launch(LaunchMode::SafeMode);
                    }
                    if ui
                        .add(epic_secondary_button(ph::COPY, "Copy Redacted Summary"))
                        .clicked()
                    {
                        let txt =
                            format_redacted_summary(&rep, app.config.redact_crash_usernames);
                        ui.ctx().copy_text(txt);
                        app.push_toast(
                            crate::core::worker::ToastKind::Info,
                            "Copied",
                            "Crash summary copied to clipboard.",
                        );
                    }
                    if ui.add(epic_danger_button(ph::TRASH, "Delete")).clicked() {
                        let _ = delete_crash_log(&rep.path);
                        app.refresh_tools_async();
                    }
                });
            });
            ui.add_space(8.0);
        }
    });
}

fn render_levels(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            icon_tile(ui, ph::LAYERS, 38.0, ACCENT_EPIC_BRIGHT, ACCENT_EPIC_SOFT);
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.label(bold_text(
                    "Created Editor Levels (CCLocalLevels.dat)",
                    16.0,
                    TEXT_PRIMARY,
                ));
                ui.label(
                    RichText::new("Inspect local editor projects and export standalone .gmd files")
                        .size(12.0)
                        .color(TEXT_SECONDARY),
                );
            });
        });
        ui.add_space(12.0);

        if let Some(sum) = app.save_summary.clone() {
            if sum.levels.is_empty() {
                empty_state_card(
                    ui,
                    ph::LAYERS,
                    "No Local Editor Levels",
                    "Create a level in Geometry Dash's editor to inspect or export it here.",
                );
            } else {
                for lvl in &sum.levels {
                    let song_label = app
                        .song_report
                        .songs
                        .iter()
                        .find(|s| s.id == lvl.song_id)
                        .and_then(|s| {
                            s.title.as_ref().map(|t| {
                                if let Some(ref a) = s.artist {
                                    format!("{t} — {a}")
                                } else {
                                    t.clone()
                                }
                            })
                        })
                        .unwrap_or_else(|| format!("Song ID {}", lvl.song_id));

                    elevated_card_frame().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width((ui.available_width() - 160.0).max(160.0));
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(bold_text(&lvl.name, 14.5, TEXT_PRIMARY));
                                    badge_pill(
                                        ui,
                                        &format!("Rev {}", lvl.revision),
                                        TEXT_SECONDARY,
                                        BG_SURFACE,
                                    );
                                    badge_pill(
                                        ui,
                                        &song_label,
                                        ACCENT_EPIC_BRIGHT,
                                        ACCENT_EPIC_SOFT,
                                    );
                                });
                                ui.label(
                                    RichText::new(if lvl.description.is_empty() {
                                        "No description provided".to_string()
                                    } else {
                                        lvl.description.clone()
                                    })
                                    .size(12.0)
                                    .color(TEXT_SECONDARY),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(epic_cta_button(ph::EXPORT, "Export .gmd"))
                                        .clicked()
                                    {
                                        let safe_name =
                                            crate::launcher::versions::sanitize_slot_name(
                                                &lvl.name,
                                            );
                                        if let Some(dst) = rfd::FileDialog::new()
                                            .set_file_name(format!("{safe_name}.gmd"))
                                            .save_file()
                                        {
                                            match export_level_to_gmd(lvl, &dst) {
                                                Ok(_) => app.push_toast(
                                                    crate::core::worker::ToastKind::Success,
                                                    "Level Exported",
                                                    format!(
                                                        "Exported '{}' to {}",
                                                        lvl.name,
                                                        dst.display()
                                                    ),
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
                    ui.add_space(6.0);
                }
            }
        } else {
            empty_state_card(
                ui,
                ph::LAYERS,
                "Inspecting Local Levels",
                "Reading CCLocalLevels.dat...",
            );
        }
    });
}

fn render_songs(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    card_frame().show(ui, |ui| {
        // Header Row (Title & Badges on left, Action buttons below or right without overlap)
        ui.horizontal_wrapped(|ui| {
            ui.label(bold_text("NONG Song Library", 17.0, TEXT_PRIMARY));
            status_dot_pill(
                ui,
                &format!("{} songs cached", app.song_report.total_songs),
                STATUS_OK,
                STATUS_OK_SOFT,
            );
            badge_pill(
                ui,
                &human_bytes(app.song_report.total_bytes),
                TEXT_SECONDARY,
                BG_ELEVATED,
            );
            if app.song_report.nong_count > 0 {
                badge_pill(
                    ui,
                    &format!("{} NONG active", app.song_report.nong_count),
                    ACCENT_EPIC_BRIGHT,
                    ACCENT_EPIC_SOFT,
                );
            }
            if app.song_metadata_resolving {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new("Resolving metadata...")
                            .size(11.5)
                            .color(ACCENT_EPIC_BRIGHT),
                    );
                });
            }
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if ui
                .add_enabled(
                    !app.song_metadata_resolving,
                    epic_secondary_button(ph::GLOBE, "Resolve Metadata"),
                )
                .on_hover_text("Lookup missing song titles and artists from official GD servers")
                .clicked()
            {
                app.resolve_missing_song_metadata_async(true);
            }
            if ui
                .add(epic_secondary_button(ph::ARROWS_CLOCKWISE, "Rescan"))
                .on_hover_text("Rescan Geometry Dash local audio cache")
                .clicked()
            {
                app.refresh_tools_async();
            }
            if app.song_report.zero_byte_count > 0
                && ui
                    .add(epic_danger_button(
                        ph::TRASH,
                        &format!("Clean {} Broken", app.song_report.zero_byte_count),
                    ))
                    .on_hover_text("Remove 0-byte corrupted audio files")
                    .clicked()
            {
                let _ = remove_broken_zero_byte_songs();
                app.refresh_tools_async();
            }
        });

        ui.add_space(12.0);

        // Quick NONG Replacement Inset Strip
        inset_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(semibold_text("Quick NONG Replace:", 12.5, TEXT_SECONDARY));
                styled_text_input(ui, &mut app.ui_state.nong_song_id, "Song ID (e.g. 1036782)", 175.0);
                if ui
                    .add(brand_cta_button(
                        ph::MUSIC_NOTE,
                        "Replace with Custom Audio",
                    ))
                    .on_hover_text("Backs up original track before copying custom .mp3/.ogg")
                    .clicked()
                {
                    if let Some(audio) = rfd::FileDialog::new()
                        .add_filter("Audio", &["mp3", "ogg"])
                        .pick_file()
                    {
                        match install_nong_replacement(&app.ui_state.nong_song_id, &audio) {
                            Ok(msg) => {
                                app.push_toast(
                                    crate::core::worker::ToastKind::Success,
                                    "NONG Applied",
                                    msg,
                                );
                                app.refresh_tools_async();
                            }
                            Err(e) => {
                                app.push_toast(
                                    crate::core::worker::ToastKind::Error,
                                    "NONG Failed",
                                    e,
                                );
                            }
                        }
                    }
                }
            });
        });

        ui.add_space(10.0);

        // Search, Sort & Filter Toolbar
        ui.horizontal_wrapped(|ui| {
            let search_w = (ui.available_width() - 410.0).clamp(210.0, 420.0);
            styled_search_bar(
                ui,
                &mut app.ui_state.song_search,
                "Search songs, artists, levels, or IDs...",
                search_w,
            );

            egui::ComboBox::from_id_salt("song_sort_combo")
                .selected_text(format!("{} {}", ph::SORT_ASCENDING, app.ui_state.song_sort.label()))
                .show_ui(ui, |ui| {
                    for mode in [
                        SongSortMode::FileSizeDesc,
                        SongSortMode::TitleAsc,
                        SongSortMode::TitleDesc,
                        SongSortMode::ArtistAsc,
                        SongSortMode::SongIdAsc,
                    ] {
                        ui.selectable_value(&mut app.ui_state.song_sort, mode, mode.label());
                    }
                });

            egui::ComboBox::from_id_salt("song_fmt_combo")
                .selected_text(app.ui_state.song_format_filter.label())
                .show_ui(ui, |ui| {
                    for fmt in [
                        SongFormatFilter::All,
                        SongFormatFilter::Mp3,
                        SongFormatFilter::Ogg,
                    ] {
                        ui.selectable_value(&mut app.ui_state.song_format_filter, fmt, fmt.label());
                    }
                });

            egui::ComboBox::from_id_salt("song_status_combo")
                .selected_text(format!("{} {}", ph::FUNNEL, app.ui_state.song_status_filter.label()))
                .show_ui(ui, |ui| {
                    for st in [
                        SongStatusFilter::All,
                        SongStatusFilter::MusicOnly,
                        SongStatusFilter::Resolved,
                        SongStatusFilter::NongActive,
                        SongStatusFilter::ManualRenamed,
                        SongStatusFilter::SfxOnly,
                        SongStatusFilter::Unknown,
                        SongStatusFilter::Broken,
                    ] {
                        ui.selectable_value(&mut app.ui_state.song_status_filter, st, st.label());
                    }
                });
        });

        ui.add_space(8.0);

        let songs = filter_and_sort_songs(
            &app.song_report.songs,
            &app.ui_state.song_search,
            app.ui_state.song_format_filter,
            app.ui_state.song_status_filter,
            app.ui_state.song_sort,
        );

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "Showing {} of {} cached entries  ·  {} resolved",
                    songs.len(),
                    app.song_report.total_songs,
                    app.song_report.resolved_count
                ))
                .size(11.5)
                .color(TEXT_MUTED),
            );
        });
        ui.add_space(8.0);

        if app.song_report.songs.is_empty() {
            empty_state_card(
                ui,
                ph::MUSIC_NOTE,
                "No Cached Songs Found",
                "Play levels in Geometry Dash or apply a NONG replacement above to populate the cache.",
            );
            return;
        }

        if songs.is_empty() {
            empty_state_card(
                ui,
                ph::MAGNIFYING_GLASS,
                "No Matching Songs",
                "Try clearing your search query or adjusting the format and status filters.",
            );
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                if ui
                    .add(epic_secondary_button(ph::X, "Clear Search & Filters"))
                    .clicked()
                {
                    app.ui_state.song_search.clear();
                    app.ui_state.song_format_filter = SongFormatFilter::All;
                    app.ui_state.song_status_filter = SongStatusFilter::All;
                }
            });
            return;
        }

        let mut action_restore: Option<String> = None;
        let mut action_nong_replace: Option<String> = None;
        let mut action_open_rename: Option<(String, String, String, bool)> = None;
        let mut action_reset_manual: Option<String> = None;
        let mut action_copy_id: Option<String> = None;

        // Virtualized row rendering supports thousands of cached songs smoothly
        let row_height = 74.0;
        egui::ScrollArea::vertical()
            .max_height(430.0)
            .show_rows(ui, row_height, songs.len(), |ui, row_range| {
                for idx in row_range {
                    let song = &songs[idx];
                    let st = song.status();
                    let (icon_glyph, icon_fg, icon_bg) = match st {
                        SongCacheStatus::NongActive => {
                            (ph::MUSIC_NOTE, ACCENT_EPIC_BRIGHT, ACCENT_EPIC_SOFT)
                        }
                        SongCacheStatus::ManualRenamed => {
                            (ph::PENCIL_SIMPLE, ACCENT_EPIC_BRIGHT, ACCENT_EPIC_SOFT)
                        }
                        SongCacheStatus::Sfx => (ph::SPEAKER_HIGH, TEXT_SECONDARY, BG_SURFACE),
                        SongCacheStatus::Broken => (ph::WARNING, STATUS_ERR, STATUS_ERR_SOFT),
                        SongCacheStatus::Unknown => (ph::MUSIC_NOTE, STATUS_WARN, STATUS_WARN_SOFT),
                        SongCacheStatus::Cached => (ph::MUSIC_NOTE, TEXT_PRIMARY, BG_SURFACE),
                    };

                    let frame_resp = egui::Frame::none()
                        .fill(BG_ELEVATED)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .rounding(Rounding::same(RADIUS_CARD))
                        .inner_margin(egui::Margin::symmetric(14.0, 11.0))
                        .show(ui, |ui| {
                            ui.set_min_height(46.0);
                            ui.horizontal(|ui| {
                                icon_tile(ui, icon_glyph, 42.0, icon_fg, icon_bg);
                                ui.add_space(6.0);

                                ui.vertical(|ui| {
                                    ui.set_width((ui.available_width() - 180.0).max(160.0));
                                    ui.horizontal(|ui| {
                                        let primary_line = if song.is_resolved() {
                                            format!(
                                                "{} — {}",
                                                song.display_title(),
                                                song.display_artist()
                                            )
                                        } else {
                                            song.display_title()
                                        };
                                        ui.add(
                                            egui::Label::new(bold_text(
                                                primary_line,
                                                13.5,
                                                if st == SongCacheStatus::Broken {
                                                    STATUS_ERR
                                                } else {
                                                    TEXT_PRIMARY
                                                },
                                            ))
                                            .truncate(),
                                        );

                                        match st {
                                            SongCacheStatus::NongActive => badge_pill(
                                                ui,
                                                "NONG",
                                                ACCENT_EPIC_BRIGHT,
                                                ACCENT_EPIC_SOFT,
                                            ),
                                            SongCacheStatus::ManualRenamed => badge_pill(
                                                ui,
                                                "CUSTOM",
                                                ACCENT_EPIC_BRIGHT,
                                                ACCENT_EPIC_SOFT,
                                            ),
                                            SongCacheStatus::Sfx => {
                                                badge_pill(ui, "SFX", TEXT_SECONDARY, BG_SURFACE)
                                            }
                                            SongCacheStatus::Unknown => badge_pill(
                                                ui,
                                                "UNKNOWN",
                                                STATUS_WARN,
                                                STATUS_WARN_SOFT,
                                            ),
                                            SongCacheStatus::Broken => badge_pill(
                                                ui,
                                                "BROKEN",
                                                STATUS_ERR,
                                                STATUS_ERR_SOFT,
                                            ),
                                            SongCacheStatus::Cached => badge_pill(
                                                ui,
                                                "CACHED",
                                                STATUS_OK,
                                                STATUS_OK_SOFT,
                                            ),
                                        }
                                        badge_pill(ui, &song.format, TEXT_MUTED, BG_INPUT);
                                    });

                                    let mut sub_parts = vec![format!("ID: {}", song.id)];
                                    if !song.associated_levels.is_empty() {
                                        let lvls = song
                                            .associated_levels
                                            .iter()
                                            .take(2)
                                            .cloned()
                                            .collect::<Vec<_>>()
                                            .join(", ");
                                        sub_parts.push(format!("Level: {lvls}"));
                                    }
                                    if song.is_resolved() {
                                        sub_parts.push(song.source.label().to_string());
                                    } else {
                                        sub_parts.push(
                                            "Metadata unavailable · Use Rename to assign".into(),
                                        );
                                    }
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(sub_parts.join("  ·  "))
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
                                        ui.menu_button(
                                            RichText::new(ph::DOTS_THREE)
                                                .size(16.0)
                                                .color(TEXT_SECONDARY),
                                            |ui| {
                                                if ui.button("Rename / Edit Metadata").clicked() {
                                                    action_open_rename = Some((
                                                        song.id.clone(),
                                                        song.title.clone().unwrap_or_default(),
                                                        song.artist.clone().unwrap_or_default(),
                                                        song.manual_override,
                                                    ));
                                                    ui.close_menu();
                                                }
                                                if ui.button("Replace with Custom Audio (NONG)").clicked() {
                                                    action_nong_replace = Some(song.id.clone());
                                                    ui.close_menu();
                                                }
                                                if song.has_nong_backup
                                                    && ui.button("Restore Original Audio").clicked()
                                                {
                                                    action_restore = Some(song.filename.clone());
                                                    ui.close_menu();
                                                }
                                                if song.manual_override
                                                    && ui.button("Reset to Auto Metadata").clicked()
                                                {
                                                    action_reset_manual = Some(song.id.clone());
                                                    ui.close_menu();
                                                }
                                                if ui.button("Copy Song ID").clicked() {
                                                    action_copy_id = Some(song.id.clone());
                                                    ui.close_menu();
                                                }
                                                if ui.button("Show in File Explorer").clicked() {
                                                    let _ = std::process::Command::new("explorer")
                                                        .arg(format!("/select,{}", song.path))
                                                        .spawn();
                                                    ui.close_menu();
                                                }
                                            },
                                        )
                                        .response
                                        .on_hover_text("Song actions");

                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new(ph::PENCIL_SIMPLE)
                                                        .size(13.5)
                                                        .color(TEXT_SECONDARY),
                                                )
                                                .fill(BG_SURFACE)
                                                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                                .rounding(Rounding::same(9.0))
                                                .min_size(Vec2::new(28.0, 28.0)),
                                            )
                                            .on_hover_text("Rename song title & artist")
                                            .clicked()
                                        {
                                            action_open_rename = Some((
                                                song.id.clone(),
                                                song.title.clone().unwrap_or_default(),
                                                song.artist.clone().unwrap_or_default(),
                                                song.manual_override,
                                            ));
                                        }

                                        ui.label(
                                            RichText::new(human_bytes(song.size_bytes))
                                                .size(12.0)
                                                .color(TEXT_SECONDARY),
                                        );
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

        if let Some(fname) = action_restore {
            match restore_original_song(&fname) {
                Ok(msg) => {
                    app.push_toast(
                        crate::core::worker::ToastKind::Success,
                        "Original Restored",
                        msg,
                    );
                    app.refresh_tools_async();
                }
                Err(e) => app.push_toast(
                    crate::core::worker::ToastKind::Error,
                    "Restore Failed",
                    e,
                ),
            }
        }
        if let Some(sid) = action_nong_replace {
            if let Some(audio) = rfd::FileDialog::new()
                .add_filter("Audio", &["mp3", "ogg"])
                .pick_file()
            {
                match install_nong_replacement(&sid, &audio) {
                    Ok(msg) => {
                        app.push_toast(
                            crate::core::worker::ToastKind::Success,
                            "NONG Applied",
                            msg,
                        );
                        app.refresh_tools_async();
                    }
                    Err(e) => app.push_toast(
                        crate::core::worker::ToastKind::Error,
                        "NONG Failed",
                        e,
                    ),
                }
            }
        }
        if let Some(id) = action_copy_id {
            ui.ctx().copy_text(id.clone());
            app.push_toast(
                crate::core::worker::ToastKind::Info,
                "Song ID Copied",
                format!("Copied {id} to clipboard."),
            );
        }
        if let Some(id) = action_reset_manual {
            let _ = clear_manual_song_metadata(&id);
            app.refresh_backups_async();
            app.push_toast(
                crate::core::worker::ToastKind::Info,
                "Metadata Reset",
                format!("Cleared custom override for ID {id}."),
            );
        }
        if let Some(modal) = action_open_rename {
            app.ui_state.rename_song_modal = Some(modal);
        }
    });

    // === Manual Song Rename Modal Dialog ===
    if let Some((song_id, mut title_buf, mut artist_buf, was_manual)) =
        app.ui_state.rename_song_modal.clone()
    {
        modal_backdrop(ui.ctx(), "rename_song_bg");
        let mut keep_open = true;
        egui::Area::new(egui::Id::new("rename_song_fg"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                dialog_frame().show(ui, |ui| {
                    ui.set_max_width(430.0);
                    ui.horizontal(|ui| {
                        icon_tile(
                            ui,
                            ph::PENCIL_SIMPLE,
                            38.0,
                            ACCENT_EPIC_BRIGHT,
                            ACCENT_EPIC_SOFT,
                        );
                        ui.vertical(|ui| {
                            ui.label(bold_text("Edit Song Metadata", 16.0, TEXT_PRIMARY));
                            ui.label(
                                RichText::new(format!("Song ID: {song_id}"))
                                    .size(12.0)
                                    .color(TEXT_MUTED),
                            );
                        });
                    });
                    ui.add_space(12.0);
                    ui.label(semibold_text("Song Title", 12.5, TEXT_SECONDARY));
                    ui.add_space(4.0);
                    styled_text_input(ui, &mut title_buf, "Enter song title...", 380.0);

                    ui.add_space(10.0);
                    ui.label(semibold_text("Artist Name", 12.5, TEXT_SECONDARY));
                    ui.add_space(4.0);
                    styled_text_input(ui, &mut artist_buf, "Enter artist name...", 380.0);

                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(epic_cta_button(ph::CHECK, "Save")).clicked() {
                            match set_manual_song_metadata(&song_id, &title_buf, &artist_buf) {
                                Ok(_) => {
                                    app.push_toast(
                                        crate::core::worker::ToastKind::Success,
                                        "Song Metadata Saved",
                                        format!("Updated ID {song_id}."),
                                    );
                                    keep_open = false;
                                    app.refresh_tools_async();
                                }
                                Err(e) => app.push_toast(
                                    crate::core::worker::ToastKind::Error,
                                    "Invalid Metadata",
                                    e,
                                ),
                            }
                        }
                        if ui.add(epic_secondary_button("", "Cancel")).clicked() {
                            keep_open = false;
                        }
                        if was_manual
                            && ui
                                .add(epic_secondary_button(
                                    ph::ARROW_COUNTER_CLOCKWISE,
                                    "Reset Auto",
                                ))
                                .clicked()
                        {
                            let _ = clear_manual_song_metadata(&song_id);
                            keep_open = false;
                            app.refresh_backups_async();
                        }
                    });
                });
            });
        if keep_open {
            app.ui_state.rename_song_modal = Some((song_id, title_buf, artist_buf, was_manual));
        } else {
            app.ui_state.rename_song_modal = None;
        }
    }
}
