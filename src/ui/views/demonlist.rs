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
        "Track your Geometry Dash progression stats, active grinds, completed demons, and live Pointercrate Top 150 standings.",
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

    ui.columns(4, |cols| {
        stat_tile(
            &mut cols[0],
            ph::USER_CIRCLE,
            "PLAYER PROFILE",
            &player_name,
            &format!("{} Stars  ·  {} Moons", stats.stars, stats.moons),
            ACCENT_EPIC_BRIGHT,
        );
        stat_tile(
            &mut cols[1],
            ph::FIRE,
            "TOTAL DEMONS",
            &format!("{}", stats.demons.max(demons_cnt as u64)),
            &format!("{matched_demons} on Top 150 List"),
            STATUS_OK,
        );
        stat_tile(
            &mut cols[2],
            ph::DIAMOND,
            "DIAMONDS & ORBS",
            &format!("{}", stats.diamonds),
            &format!("{} Mana Orbs", stats.orbs),
            TEXT_PRIMARY,
        );
        stat_tile(
            &mut cols[3],
            ph::GAME_CONTROLLER,
            "ATTEMPTS & COINS",
            &format!("{}", stats.attempts),
            &format!("{} Secret · {} User Coins", stats.secret_coins, stats.user_coins),
            TEXT_SECONDARY,
        );
    });

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

    let row_height = 70.0;
    egui::ScrollArea::vertical()
        .max_height(460.0)
        .show_rows(ui, row_height, items.len(), |ui, range| {
            for idx in range {
                let d = items[idx];
                let progress = match_demon_progress(&d.name, progress_map);
                let beaten = progress.is_some_and(|(pct, _)| pct >= 100);
                let frame_resp = elevated_card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (badge_fg, badge_bg) = if d.position <= 10 {
                            (ACCENT_EPIC_BRIGHT, ACCENT_EPIC_SOFT)
                        } else if d.position <= 75 {
                            (STATUS_WARN, STATUS_WARN_SOFT)
                        } else {
                            (TEXT_PRIMARY, BG_SURFACE)
                        };
                        egui::Frame::none()
                            .fill(badge_bg)
                            .stroke(Stroke::new(1.0_f32, badge_fg.gamma_multiply(0.3)))
                            .rounding(Rounding::same(RADIUS_ICON))
                            .inner_margin(egui::Margin::symmetric(10.0, 9.0))
                            .show(ui, |ui| {
                                ui.set_min_width(38.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(bold_text(&format!("#{}", d.position), 13.5, badge_fg));
                                });
                            });
                        ui.add_space(6.0);

                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 190.0).max(150.0));
                            ui.horizontal_wrapped(|ui| {
                                ui.label(bold_text(&d.name, 14.5, TEXT_PRIMARY));
                                badge_pill(
                                    ui,
                                    &format!("List {}%", d.requirement),
                                    TEXT_SECONDARY,
                                    BG_SURFACE,
                                );
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
                                            &format!("{pct}% ({att} att)"),
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
                if frame_resp.response.hovered() {
                    ui.painter().rect_stroke(
                        frame_resp.response.rect,
                        Rounding::same(RADIUS_CARD),
                        Stroke::new(1.0_f32, BORDER_STRONG),
                    );
                }
                ui.add_space(5.0);
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

    egui::ScrollArea::vertical()
        .max_height(460.0)
        .show(ui, |ui| {
            for e in filtered {
                elevated_card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        icon_tile(
                            ui,
                            if completed_mode { ph::CHECK_CIRCLE } else { ph::FIRE },
                            40.0,
                            if completed_mode { STATUS_OK } else { ACCENT_EPIC_BRIGHT },
                            if completed_mode { STATUS_OK_SOFT } else { ACCENT_EPIC_SOFT },
                        );
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 180.0).max(150.0));
                            ui.horizontal_wrapped(|ui| {
                                ui.label(bold_text(&e.name, 14.0, TEXT_PRIMARY));
                                badge_pill(
                                    ui,
                                    &format!("{}★", e.stars),
                                    STATUS_WARN,
                                    STATUS_WARN_SOFT,
                                );
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
                            status_dot_pill(
                                ui,
                                &format!("{}%", e.percent),
                                if e.percent >= 100 { STATUS_OK } else { ACCENT_EPIC_BRIGHT },
                                if e.percent >= 100 {
                                    STATUS_OK_SOFT
                                } else {
                                    ACCENT_EPIC_SOFT
                                },
                            );
                        });
                    });
                });
                ui.add_space(5.0);
            }
        });
}
