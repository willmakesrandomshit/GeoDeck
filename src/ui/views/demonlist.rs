use eframe::egui::{self, RichText, Rounding, Stroke};

use crate::core::platform::open_url;
use crate::tools::demonlist::match_demon_progress;
use crate::ui::app::GeodeDeckApp;
use crate::ui::icons::ph;
use crate::ui::theme::*;

pub fn render(app: &mut GeodeDeckApp, ui: &mut egui::Ui) {
    section_heading(
        ui,
        "Demonlist & Player Progress",
        "Track progression stats, active grinds, completed demons, and Pointercrate Top 150 standings.",
    );

    let (stats, grinds_cnt, demons_cnt, player_name, progress_map) = match &app.save_summary {
        Some(sum) => (
            sum.player_stats.clone(),
            sum.active_grinds.len(),
            sum.completed_demons.len(),
            sum.player_name.clone(),
            sum.level_progress_by_name.clone(),
        ),
        None => (Default::default(), 0, 0, "Player".into(), Default::default()),
    };

    let matched_demons = app
        .demonlist
        .iter()
        .filter(|d| {
            match_demon_progress(&d.name, &progress_map)
                .is_some_and(|(pct, _)| pct >= 100)
        })
        .count();

    let demons_val = format!("{}", stats.demons.max(demons_cnt as u64));
    let matched_sub = format!("{matched_demons} on Top 150 List");
    let stars_sub = format!("{} Stars  ·  {} Moons", stats.stars, stats.moons);
    let diamonds_val = format!("{}", stats.diamonds);
    let orbs_sub = format!("{} Mana Orbs", stats.orbs);
    let attempts_val = format!("{}", stats.attempts);
    let coins_sub = format!("{} Secret · {} User Coins", stats.secret_coins, stats.user_coins);

    telemetry_strip(
        ui,
        &[
            ("PLAYER PROFILE", &player_name, &stars_sub, ACCENT_EPIC_BRIGHT),
            ("TOTAL DEMONS", &demons_val, &matched_sub, STATUS_OK),
            ("DIAMONDS & ORBS", &diamonds_val, &orbs_sub, TEXT_PRIMARY),
            ("ATTEMPTS & COINS", &attempts_val, &coins_sub, TEXT_SECONDARY),
        ],
    );

    ui.add_space(12.0);

    let grinds_lbl = format!("Active Grinds ({grinds_cnt})");
    let completed_lbl = format!("Completed Demons ({demons_cnt})");
    let top_lbl = format!("Top 150 Pointercrate ({})", app.demonlist.len());

    epic_subtab_bar(
        ui,
        &[
            (ph::STAR, &top_lbl),
            (ph::LIGHTNING, &grinds_lbl),
            (ph::CHECK_CIRCLE, &completed_lbl),
        ],
        &mut app.ui_state.demonlist_subtab,
    );

    card_frame().show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let search_resp = styled_search_bar(
                ui,
                &mut app.ui_state.demon_search,
                "Filter demons by name, verifier, or publisher...",
                (ui.available_width() - 180.0).clamp(200.0, 420.0),
            );
            if app.ui_state.focus_search_requested {
                search_resp.request_focus();
                app.ui_state.focus_search_requested = false;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !app.demonlist_loading,
                        epic_secondary_button(ph::ARROWS_CLOCKWISE, "Refresh List"),
                    )
                    .clicked()
                {
                    app.refresh_demonlist_async();
                }
            });
        });
        ui.add_space(10.0);

        let q = app.ui_state.demon_search.trim().to_lowercase();
        match app.ui_state.demonlist_subtab {
            0 => render_pointercrate(app, ui, &q, &progress_map),
            1 => render_level_entries(
                ui,
                &app.save_summary
                    .as_ref()
                    .map(|s| s.active_grinds.clone())
                    .unwrap_or_default(),
                &q,
                "No Active Grinds Yet",
                "Play levels in Geometry Dash (15%-99% normal progress) to track active grinds here.",
                false,
            ),
            _ => render_level_entries(
                ui,
                &app.save_summary
                    .as_ref()
                    .map(|s| s.completed_demons.clone())
                    .unwrap_or_default(),
                &q,
                "No Completed Demons Found",
                "Beaten demons recorded in CCGameManager.dat will appear here.",
                true,
            ),
        }
    });
}

fn render_pointercrate(
    app: &GeodeDeckApp,
    ui: &mut egui::Ui,
    q: &str,
    progress_map: &std::collections::BTreeMap<String, (u32, u32)>,
) {
    if app.demonlist_loading && app.demonlist.is_empty() {
        empty_state_card(
            ui,
            ph::GLOBE,
            "Fetching Pointercrate Top 150...",
            "Connecting to pointercrate.com and matching your local Geometry Dash progress.",
        );
        return;
    }

    if let Some(ref err) = app.demonlist_error {
        if app.demonlist.is_empty() {
            empty_state_card(ui, ph::WARNING, "Pointercrate Unavailable", err);
            return;
        }
    }

    let items: Vec<_> = app
        .demonlist
        .iter()
        .filter(|d| {
            q.is_empty()
                || d.name.to_lowercase().contains(q)
                || d.verifier.to_lowercase().contains(q)
                || d.publisher.to_lowercase().contains(q)
                || d.position.to_string() == q
        })
        .collect();

    if app.demonlist.is_empty() {
        empty_state_card(
            ui,
            ph::STAR,
            "Pointercrate Top 150 Ready",
            "Click 'Refresh List' above to load the live Pointercrate Demonlist and match your save progress.",
        );
        return;
    }

    if items.is_empty() {
        empty_state_card(
            ui,
            ph::MAGNIFYING_GLASS,
            "No Matching Demons",
            "Clear your filter query or click Refresh List above.",
        );
        return;
    }

    let row_height = 60.0;
    egui::ScrollArea::vertical()
        .max_height(460.0)
        .show_rows(ui, row_height, items.len(), |ui, range| {
            for idx in range {
                let d = items[idx];
                let progress = match_demon_progress(&d.name, progress_map);
                let beaten = progress.is_some_and(|(pct, _)| pct >= 100);
                let rank_col = if d.position <= 10 {
                    ACCENT_EPIC_BRIGHT
                } else if d.position <= 75 {
                    STATUS_WARN
                } else {
                    TEXT_SECONDARY
                };

                hover_row(ui, egui::Sense::hover(), |ui| {
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(44.0, 32.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(mono_text(format!("#{}", d.position), 14.0, rank_col));
                            },
                        );
                        ui.add_space(10.0);

                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 220.0).max(150.0));
                            ui.horizontal_wrapped(|ui| {
                                ui.label(bold_text(&d.name, 14.0, TEXT_PRIMARY));
                                ui.label(mono_text(
                                    format!("List {}%", d.requirement),
                                    11.0,
                                    TEXT_MUTED,
                                ));
                                if let Some((pct, att)) = progress {
                                    if beaten {
                                        status_dot_pill(
                                            ui,
                                            "BEATEN 100%",
                                            STATUS_OK,
                                            STATUS_OK_SOFT,
                                        );
                                    } else if pct > 0 {
                                        badge_pill(
                                            ui,
                                            &format!("{pct}% · {att} att"),
                                            ACCENT_EPIC_BRIGHT,
                                            ACCENT_EPIC_SOFT,
                                        );
                                    }
                                }
                            });
                            ui.add(
                                egui::Label::new(
                                    RichText::new(format!(
                                        "Published by {}  ·  Verified by {}",
                                        d.publisher, d.verifier
                                    ))
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                                )
                                .truncate(),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !d.video.is_empty()
                                && ui
                                    .add(epic_secondary_button(ph::PLAY, "Verification"))
                                    .on_hover_text("Open verification video in browser")
                                    .clicked()
                            {
                                open_url(&d.video);
                            }
                        });
                    });
                });
                if idx + 1 < items.len() {
                    row_divider(ui);
                }
            }
        });
}

fn render_level_entries(
    ui: &mut egui::Ui,
    entries: &[crate::backup::save_inspector::LevelProgressEntry],
    q: &str,
    empty_title: &str,
    empty_sub: &str,
    completed_mode: bool,
) {
    let filtered: Vec<_> = entries
        .iter()
        .filter(|e| {
            q.is_empty()
                || e.name.to_lowercase().contains(q)
                || e.author.to_lowercase().contains(q)
        })
        .collect();

    if filtered.is_empty() {
        empty_state_card(
            ui,
            if completed_mode { ph::CHECK_CIRCLE } else { ph::LIGHTNING },
            empty_title,
            empty_sub,
        );
        return;
    }

    let len = filtered.len();
    egui::ScrollArea::vertical()
        .max_height(460.0)
        .show(ui, |ui| {
            for (idx, e) in filtered.into_iter().enumerate() {
                hover_row(ui, egui::Sense::hover(), |ui| {
                    ui.horizontal(|ui| {
                        let (fg, bg) = if completed_mode {
                            (STATUS_OK, STATUS_OK_SOFT)
                        } else {
                            (ACCENT_EPIC_BRIGHT, ACCENT_EPIC_SOFT)
                        };
                        icon_tile(
                            ui,
                            if completed_mode { ph::CHECK_CIRCLE } else { ph::FIRE },
                            34.0,
                            fg,
                            bg,
                        );
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 150.0).max(150.0));
                            ui.horizontal_wrapped(|ui| {
                                ui.label(bold_text(&e.name, 13.5, TEXT_PRIMARY));
                                ui.label(mono_text(format!("{}★", e.stars), 11.5, STATUS_WARN));
                            });
                            ui.label(
                                RichText::new(format!(
                                    "by {}  ·  Practice {}%  ·  {} attempt(s)",
                                    e.author, e.practice, e.attempts
                                ))
                                .size(11.5)
                                .color(TEXT_MUTED),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let pct_col = if e.percent >= 100 {
                                STATUS_OK
                            } else {
                                ACCENT_EPIC_BRIGHT
                            };
                            ui.label(mono_text(format!("{}%", e.percent), 13.5, pct_col));
                            if !completed_mode {
                                let bar_w = 56.0;
                                let (bar_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(bar_w, 4.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    bar_rect,
                                    Rounding::same(2.0),
                                    BG_ELEVATED,
                                );
                                let fill_w = (bar_w * (e.percent as f32 / 100.0)).clamp(3.0, bar_w);
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(
                                        bar_rect.min,
                                        egui::vec2(fill_w, 4.0),
                                    ),
                                    Rounding::same(2.0),
                                    ACCENT_EPIC,
                                );
                                let _ = Stroke::NONE;
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
