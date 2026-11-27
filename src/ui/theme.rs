use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Margin, RichText, Rounding,
    Shadow, Stroke, Vec2,
};

use crate::ui::icons::ph;

// === Machined Concentric Corner-Radius Scale ===
pub const RADIUS_PANEL: f32 = 10.0;
pub const RADIUS_CARD: f32 = 8.0;
pub const RADIUS_BUTTON: f32 = 7.0;
pub const RADIUS_SEARCH: f32 = 7.0;
pub const RADIUS_INPUT: f32 = 6.0;
pub const RADIUS_PILL: f32 = 999.0;
pub const RADIUS_ICON: f32 = 7.0;
pub const RADIUS_DIALOG: f32 = 12.0;

// === Neutral Carbon-Graphite Palette (Restrained Brand Purple + WCAG AA Contrast) ===
pub const BG_VOID: Color32 = Color32::from_rgb(13, 14, 17);
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(17, 19, 23);
pub const BG_SURFACE: Color32 = Color32::from_rgb(22, 24, 29);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(29, 32, 39);
pub const BG_HOVER: Color32 = Color32::from_rgb(37, 41, 50);
pub const BG_INPUT: Color32 = Color32::from_rgb(15, 17, 21);

pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(38, 42, 52);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(56, 62, 77);
pub const BORDER_GLOW: Color32 = Color32::from_rgb(111, 84, 199);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(242, 244, 248);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(180, 187, 200);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(143, 151, 168);

pub const ACCENT_EPIC: Color32 = Color32::from_rgb(143, 107, 255);
pub const ACCENT_EPIC_BRIGHT: Color32 = Color32::from_rgb(179, 155, 255);
pub const ACCENT_EPIC_SOFT: Color32 = Color32::from_rgb(35, 30, 54);

pub const STATUS_OK: Color32 = Color32::from_rgb(56, 211, 159);
pub const STATUS_OK_SOFT: Color32 = Color32::from_rgb(22, 43, 36);
pub const STATUS_WARN: Color32 = Color32::from_rgb(240, 178, 76);
pub const STATUS_WARN_SOFT: Color32 = Color32::from_rgb(47, 37, 22);
pub const STATUS_ERR: Color32 = Color32::from_rgb(244, 107, 107);
pub const STATUS_ERR_SOFT: Color32 = Color32::from_rgb(49, 25, 29);

const FONT_DISPLAY_BOLD: &[u8] = include_bytes!("../../assets/fonts/SpaceGrotesk-Bold.ttf");
const FONT_BODY_REGULAR: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSans-Regular.ttf");
const FONT_BODY_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSans-SemiBold.ttf");
const FONT_BODY_BOLD: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSans-Bold.ttf");
const FONT_MONO_REGULAR: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");

pub fn setup_fonts_and_theme(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "space_grotesk_bold".into(),
        FontData::from_static(FONT_DISPLAY_BOLD),
    );
    fonts.font_data.insert(
        "plex_regular".into(),
        FontData::from_static(FONT_BODY_REGULAR),
    );
    fonts.font_data.insert(
        "plex_semibold".into(),
        FontData::from_static(FONT_BODY_SEMIBOLD),
    );
    fonts
        .font_data
        .insert("plex_bold".into(), FontData::from_static(FONT_BODY_BOLD));
    fonts.font_data.insert(
        "jetbrains_mono".into(),
        FontData::from_static(FONT_MONO_REGULAR),
    );

    if let Some(prop_fam) = fonts.families.get_mut(&FontFamily::Proportional) {
        prop_fam.insert(0, "plex_regular".into());
    }
    if let Some(mono_fam) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono_fam.insert(0, "jetbrains_mono".into());
    }

    let fallback_tail: Vec<String> = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|name| name != "plex_regular")
        .collect();

    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    let mut semibold_fam = vec!["plex_semibold".into(), "plex_regular".into(), "phosphor".into()];
    semibold_fam.extend(fallback_tail.iter().cloned());
    fonts
        .families
        .insert(FontFamily::Name("semibold".into()), semibold_fam);

    let mut bold_fam = vec!["plex_bold".into(), "plex_regular".into(), "phosphor".into()];
    bold_fam.extend(fallback_tail.iter().cloned());
    fonts
        .families
        .insert(FontFamily::Name("bold".into()), bold_fam);

    let mut display_fam = vec![
        "space_grotesk_bold".into(),
        "plex_bold".into(),
        "plex_regular".into(),
        "phosphor".into(),
    ];
    display_fam.extend(fallback_tail);
    fonts
        .families
        .insert(FontFamily::Name("display".into()), display_fam);

    if let Some(mono_fam) = fonts.families.get_mut(&FontFamily::Monospace) {
        if !mono_fam.iter().any(|f| f == "phosphor") {
            mono_fam.push("phosphor".into());
        }
    }

    ctx.set_fonts(fonts);
    apply_workbench_visuals(ctx);
}

pub fn semibold_text(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text.into())
        .font(FontId::new(size, FontFamily::Name("semibold".into())))
        .color(color)
}

pub fn display_text(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text.into())
        .font(FontId::new(size, FontFamily::Name("display".into())))
        .color(color)
}

pub fn bold_text(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text.into())
        .font(FontId::new(size, FontFamily::Name("bold".into())))
        .color(color)
}

pub fn mono_text(text: impl Into<String>, size: f32, color: Color32) -> RichText {
    RichText::new(text.into())
        .font(FontId::new(size, FontFamily::Monospace))
        .color(color)
}

pub fn apply_workbench_visuals(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = Margin::same(16.0);
    style.spacing.indent = 16.0;
    style.spacing.interact_size.y = 30.0;
    style.animation_time = 0.10;

    let mut vis = egui::Visuals::dark();
    vis.panel_fill = BG_VOID;
    vis.window_fill = BG_SURFACE;
    vis.extreme_bg_color = BG_INPUT;
    vis.faint_bg_color = BG_ELEVATED;
    vis.code_bg_color = BG_INPUT;

    vis.window_rounding = Rounding::same(RADIUS_DIALOG);
    vis.window_stroke = Stroke::new(1.0_f32, BORDER_STRONG);
    vis.window_shadow = Shadow {
        offset: egui::vec2(0.0, 10.0),
        blur: 22.0,
        spread: 0.0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 130),
    };
    vis.menu_rounding = Rounding::same(RADIUS_CARD);
    vis.popup_shadow = Shadow {
        offset: egui::vec2(0.0, 6.0),
        blur: 14.0,
        spread: 0.0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 110),
    };

    vis.widgets.noninteractive.bg_fill = BG_SURFACE;
    vis.widgets.noninteractive.weak_bg_fill = BG_SURFACE;
    vis.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    vis.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_SECONDARY);
    vis.widgets.noninteractive.rounding = Rounding::same(RADIUS_INPUT);

    vis.widgets.inactive.bg_fill = BG_ELEVATED;
    vis.widgets.inactive.weak_bg_fill = BG_ELEVATED;
    vis.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    vis.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    vis.widgets.inactive.rounding = Rounding::same(RADIUS_INPUT);

    vis.widgets.hovered.bg_fill = BG_HOVER;
    vis.widgets.hovered.weak_bg_fill = BG_HOVER;
    vis.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, BORDER_STRONG);
    vis.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, TEXT_PRIMARY);
    vis.widgets.hovered.rounding = Rounding::same(RADIUS_INPUT);

    vis.widgets.active.bg_fill = ACCENT_EPIC;
    vis.widgets.active.weak_bg_fill = ACCENT_EPIC;
    vis.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT_EPIC_BRIGHT);
    vis.widgets.active.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    vis.widgets.active.rounding = Rounding::same(RADIUS_INPUT);

    vis.selection.bg_fill = ACCENT_EPIC_SOFT;
    vis.selection.stroke = Stroke::new(1.0_f32, ACCENT_EPIC);

    style.visuals = vis;
    ctx.set_style(style);
}

/// Main flat surface panel frame (10px machined corners, crisp 1px border, zero shadow blur).
pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_PANEL))
        .inner_margin(Margin::same(18.0))
}

/// Elevated secondary card frame (8px rounding, crisp border).
pub fn elevated_card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_ELEVATED)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_CARD))
        .inner_margin(Margin::same(12.0))
}

/// Recessed inset surface (log viewer, code blocks, command preview).
pub fn inset_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_INPUT)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_CARD))
        .inner_margin(Margin::same(12.0))
}

/// Borderless row frame used inside main cards (subtle hover highlight on BG_ELEVATED).
pub fn list_row_frame(hovered: bool) -> egui::Frame {
    egui::Frame::none()
        .fill(if hovered {
            BG_ELEVATED
        } else {
            Color32::TRANSPARENT
        })
        .rounding(Rounding::same(RADIUS_CARD))
        .inner_margin(Margin::symmetric(10.0, 8.0))
}

/// Interactive row wrapper that paints a clean flat hover surface behind its contents.
pub fn hover_row(
    ui: &mut egui::Ui,
    sense: egui::Sense,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let bg_slot = ui.painter().add(egui::Shape::Noop);
    let inner = list_row_frame(false).show(ui, add_contents);
    let resp = inner.response.interact(sense);
    if resp.hovered() {
        ui.painter().set(
            bg_slot,
            egui::Shape::rect_filled(resp.rect, Rounding::same(RADIUS_CARD), BG_ELEVATED),
        );
    }
    resp
}

/// Crisp 1px horizontal hairline divider between flat rows.
pub fn row_divider(ui: &mut egui::Ui) {
    let w = ui.available_width().max(1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 5.0), egui::Sense::hover());
    let y = rect.center().y.round();
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        Stroke::new(1.0_f32, BORDER_SUBTLE),
    );
}

/// Modal dialog frame (12px rounding).
pub fn dialog_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER_STRONG))
        .rounding(Rounding::same(RADIUS_DIALOG))
        .inner_margin(Margin::same(20.0))
        .shadow(Shadow {
            offset: egui::vec2(0.0, 14.0),
            blur: 28.0,
            spread: 0.0,
            color: Color32::from_rgba_premultiplied(0, 0, 0, 165),
        })
}

/// Compact machined status tag (5px radius instead of oversized bubble pill).
pub fn badge_pill(ui: &mut egui::Ui, label: &str, text_color: Color32, bg: Color32) {
    egui::Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, text_color.gamma_multiply(0.18)))
        .rounding(Rounding::same(5.0))
        .inner_margin(Margin::symmetric(7.0, 2.0))
        .show(ui, |ui| {
            ui.label(semibold_text(label, 10.5, text_color));
        });
}

/// Compact status pill with a single sharp solid indicator dot (no halo glow).
pub fn status_dot_pill(ui: &mut egui::Ui, label: &str, color: Color32, bg: Color32) {
    egui::Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.22)))
        .rounding(Rounding::same(RADIUS_PILL))
        .inner_margin(Margin::symmetric(8.0, 2.5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (dot_rect, _) =
                    ui.allocate_exact_size(Vec2::splat(7.0), egui::Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 2.8, color);
                ui.label(semibold_text(label, 10.5, color));
            });
        });
}

/// Flat filter toggle button (7px machined corners, no purple pill bloat).
pub fn filter_chip(ui: &mut egui::Ui, label: &str, selected: bool) -> bool {
    let resp = egui::Frame::none()
        .fill(if selected {
            BG_ELEVATED
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::new(
            1.0_f32,
            if selected {
                BORDER_STRONG
            } else {
                BORDER_SUBTLE
            },
        ))
        .rounding(Rounding::same(RADIUS_BUTTON))
        .inner_margin(Margin::symmetric(10.0, 4.5))
        .show(ui, |ui| {
            ui.label(semibold_text(
                label,
                11.5,
                if selected {
                    TEXT_PRIMARY
                } else {
                    TEXT_SECONDARY
                },
            ));
        })
        .response
        .interact(egui::Sense::click());
    resp.clicked()
}

/// Single-container horizontal telemetry strip with crisp 1px vertical column hairlines.
pub fn telemetry_strip(ui: &mut egui::Ui, items: &[(&str, &str, &str, Color32)]) {
    if items.is_empty() {
        return;
    }
    let mut divider_xs: Vec<f32> = Vec::with_capacity(items.len().saturating_sub(1));
    let frame_resp = egui::Frame::none()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_PANEL))
        .inner_margin(Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.columns(items.len(), |cols| {
                for (idx, (label, value, sub, accent)) in items.iter().enumerate() {
                    let col_ui = &mut cols[idx];
                    if idx > 0 {
                        divider_xs.push((col_ui.max_rect().left() - 4.0).round());
                    }
                    egui::Frame::none()
                        .inner_margin(Margin {
                            left: if idx > 0 { 10.0 } else { 0.0 },
                            right: 6.0,
                            top: 0.0,
                            bottom: 0.0,
                        })
                        .show(col_ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                let (dot_r, _) =
                                    ui.allocate_exact_size(Vec2::splat(6.0), egui::Sense::hover());
                                ui.painter().circle_filled(dot_r.center(), 2.5, *accent);
                                ui.add(
                                    egui::Label::new(mono_text(*label, 10.0, TEXT_MUTED)).truncate(),
                                );
                            });
                            ui.add_space(4.0);
                            let val_col = if *accent == TEXT_MUTED {
                                TEXT_MUTED
                            } else {
                                TEXT_PRIMARY
                            };
                            ui.add(egui::Label::new(bold_text(*value, 17.0, val_col)).truncate());
                            ui.add_space(2.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(*sub).size(11.5).color(TEXT_SECONDARY),
                                )
                                .truncate(),
                            );
                        });
                }
            });
        });

    let r = frame_resp.response.rect;
    for x in divider_xs {
        ui.painter().line_segment(
            [egui::pos2(x, r.top() + 12.0), egui::pos2(x, r.bottom() - 12.0)],
            Stroke::new(1.0_f32, BORDER_SUBTLE),
        );
    }
}

/// Primary CTA button: pure crisp white (#FFFFFF) fill with bold dark text (7px rounded, 32px min height).
pub fn epic_cta_button<'a>(icon: &str, label: &str) -> egui::Button<'a> {
    let text = format_btn_text(icon, label);
    egui::Button::new(bold_text(text, 12.5, Color32::from_rgb(13, 14, 17)))
        .fill(Color32::WHITE)
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(RADIUS_BUTTON))
        .min_size(Vec2::new(0.0, 32.0))
}

/// Brand purple accent CTA button for secondary highlighted actions.
pub fn brand_cta_button<'a>(icon: &str, label: &str) -> egui::Button<'a> {
    let text = format_btn_text(icon, label);
    egui::Button::new(bold_text(text, 12.5, Color32::WHITE))
        .fill(ACCENT_EPIC)
        .stroke(Stroke::new(1.0_f32, ACCENT_EPIC_BRIGHT.gamma_multiply(0.45)))
        .rounding(Rounding::same(RADIUS_BUTTON))
        .min_size(Vec2::new(0.0, 32.0))
}

/// Elevated secondary button (7px rounded, 32px min height).
pub fn epic_secondary_button<'a>(icon: &str, label: &str) -> egui::Button<'a> {
    let text = format_btn_text(icon, label);
    egui::Button::new(semibold_text(text, 12.0, TEXT_PRIMARY))
        .fill(BG_ELEVATED)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_BUTTON))
        .min_size(Vec2::new(0.0, 32.0))
}

/// Ghost icon button for toolbar navigation & chrome actions.
pub fn ghost_icon_button<'a>(icon: &str) -> egui::Button<'a> {
    egui::Button::new(RichText::new(icon).size(14.0).color(TEXT_SECONDARY))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_BUTTON))
        .min_size(Vec2::new(30.0, 30.0))
}

/// Soft destructive action button (7px rounded, 32px min height).
pub fn epic_danger_button<'a>(icon: &str, label: &str) -> egui::Button<'a> {
    let text = format_btn_text(icon, label);
    egui::Button::new(bold_text(text, 12.0, STATUS_ERR))
        .fill(STATUS_ERR_SOFT)
        .stroke(Stroke::new(1.0_f32, STATUS_ERR.gamma_multiply(0.38)))
        .rounding(Rounding::same(RADIUS_BUTTON))
        .min_size(Vec2::new(0.0, 32.0))
}

fn format_btn_text(icon: &str, label: &str) -> String {
    if icon.is_empty() {
        label.to_string()
    } else if label.is_empty() {
        icon.to_string()
    } else {
        format!("{icon}  {label}")
    }
}

/// Compact 38x20 stadium toggle switch.
pub fn epic_toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = Vec2::new(38.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let bg = egui::lerp(
            egui::Rgba::from(Color32::from_rgb(42, 46, 58))..=egui::Rgba::from(ACCENT_EPIC),
            how_on,
        );
        let r = rect.height() * 0.5;
        ui.painter()
            .rect_filled(rect, Rounding::same(r), Color32::from(bg));
        if response.hovered() {
            ui.painter().rect_stroke(
                rect,
                Rounding::same(r),
                Stroke::new(1.0_f32, ACCENT_EPIC_BRIGHT.gamma_multiply(0.55)),
            );
        } else if !*on {
            ui.painter().rect_stroke(
                rect,
                Rounding::same(r),
                Stroke::new(1.0_f32, BORDER_STRONG),
            );
        }
        let circle_x = egui::lerp((rect.left() + r)..=(rect.right() - r), how_on);
        let thumb_col = if *on {
            Color32::WHITE
        } else {
            Color32::from_rgb(205, 210, 224)
        };
        ui.painter()
            .circle_filled(egui::pos2(circle_x, rect.center().y), r - 3.0, thumb_col);
    }
    response
}

/// Search input with leading magnifying glass, focus border, and clear button.
pub fn styled_search_bar(
    ui: &mut egui::Ui,
    query: &mut String,
    hint: &str,
    desired_width: f32,
) -> egui::Response {
    let mut inner_resp: Option<egui::Response> = None;
    let id = ui.make_persistent_id(hint);
    let is_focused = ui.memory(|m| m.has_focus(id));

    egui::Frame::none()
        .fill(BG_INPUT)
        .stroke(Stroke::new(
            1.0_f32,
            if is_focused {
                ACCENT_EPIC
            } else {
                BORDER_SUBTLE
            },
        ))
        .rounding(Rounding::same(RADIUS_SEARCH))
        .inner_margin(Margin::symmetric(10.0, 5.0))
        .show(ui, |ui| {
            ui.set_min_height(20.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 7.0;
                ui.label(
                    RichText::new(ph::MAGNIFYING_GLASS)
                        .size(14.0)
                        .color(if is_focused {
                            ACCENT_EPIC_BRIGHT
                        } else {
                            TEXT_MUTED
                        }),
                );
                let clear_reserve = if query.is_empty() { 4.0 } else { 22.0 };
                let resp = ui.add(
                    egui::TextEdit::singleline(query)
                        .id(id)
                        .frame(false)
                        .desired_width((desired_width - clear_reserve - 24.0).max(90.0))
                        .hint_text(RichText::new(hint).color(TEXT_MUTED)),
                );
                if !query.is_empty()
                    && ui
                        .add(
                            egui::Button::new(
                                RichText::new(ph::X).size(12.0).color(TEXT_SECONDARY),
                            )
                            .frame(false),
                        )
                        .on_hover_text("Clear search")
                        .clicked()
                {
                    query.clear();
                }
                inner_resp = Some(resp);
            });
        });

    inner_resp.expect("styled_search_bar inner response")
}

/// 6px rounded input field with subtle focus ring.
pub fn styled_text_input(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: &str,
    desired_width: f32,
) -> egui::Response {
    let mut inner_resp: Option<egui::Response> = None;
    let id = ui.make_persistent_id(("input", hint, desired_width as i32));
    let is_focused = ui.memory(|m| m.has_focus(id));

    egui::Frame::none()
        .fill(BG_INPUT)
        .stroke(Stroke::new(
            1.0_f32,
            if is_focused {
                ACCENT_EPIC
            } else {
                BORDER_SUBTLE
            },
        ))
        .rounding(Rounding::same(RADIUS_INPUT))
        .inner_margin(Margin::symmetric(10.0, 5.0))
        .show(ui, |ui| {
            ui.set_min_height(20.0);
            let resp = ui.add(
                egui::TextEdit::singleline(value)
                    .id(id)
                    .frame(false)
                    .desired_width((desired_width - 20.0).max(60.0))
                    .hint_text(RichText::new(hint).color(TEXT_MUTED)),
            );
            inner_resp = Some(resp);
        });

    inner_resp.expect("styled_text_input inner response")
}

/// Clean square icon container (7px machined corners).
pub fn icon_tile(ui: &mut egui::Ui, icon: &str, size: f32, fg: Color32, bg: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, Rounding::same(RADIUS_ICON), bg);
    p.rect_stroke(
        rect,
        Rounding::same(RADIUS_ICON),
        Stroke::new(1.0_f32, fg.gamma_multiply(0.18)),
    );
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        FontId::proportional(size * 0.46),
        fg,
    );
}

/// Centered empty-state block.
pub fn empty_state_card(ui: &mut egui::Ui, icon: &str, title: &str, subtitle: &str) {
    inset_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.vertical_centered(|ui| {
            ui.add_space(12.0);
            icon_tile(ui, icon, 40.0, TEXT_SECONDARY, BG_ELEVATED);
            ui.add_space(8.0);
            ui.label(bold_text(title, 14.5, TEXT_PRIMARY));
            if !subtitle.is_empty() {
                ui.add_space(3.0);
                ui.label(RichText::new(subtitle).size(12.0).color(TEXT_SECONDARY));
            }
            ui.add_space(12.0);
        });
    });
}

/// Dimmed fullscreen modal backdrop.
pub fn modal_backdrop(ctx: &egui::Context, id: &str) {
    egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Middle)
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            let screen = ctx.screen_rect();
            ui.allocate_rect(screen, egui::Sense::click());
            ui.painter().rect_filled(
                screen,
                Rounding::ZERO,
                Color32::from_rgba_premultiplied(6, 7, 11, 170),
            );
        });
}

/// Flat bottom-bordered workbench sub-tab strip with crisp 2px active underline rail.
pub fn epic_subtab_bar(ui: &mut egui::Ui, tabs: &[(&str, &str)], active_idx: &mut usize) {
    let strip_resp = ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (idx, (icon, label)) in tabs.iter().enumerate() {
            let selected = *active_idx == idx;
            let fg = if selected {
                TEXT_PRIMARY
            } else {
                TEXT_SECONDARY
            };
            let title = if icon.is_empty() {
                (*label).to_string()
            } else {
                format!("{icon}  {label}")
            };

            let bg_slot = ui.painter().add(egui::Shape::Noop);
            let resp = egui::Frame::none()
                .fill(Color32::TRANSPARENT)
                .rounding(Rounding::same(RADIUS_BUTTON))
                .inner_margin(Margin::symmetric(11.0, 7.0))
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(semibold_text(title, 12.5, fg))
                            .wrap_mode(egui::TextWrapMode::Extend),
                    );
                })
                .response
                .interact(egui::Sense::click());

            if selected {
                let r = resp.rect;
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(r.left() + 6.0, r.bottom() - 2.0),
                        egui::pos2(r.right() - 6.0, r.bottom()),
                    ),
                    Rounding::same(1.0),
                    ACCENT_EPIC,
                );
            } else if resp.hovered() {
                ui.painter().set(
                    bg_slot,
                    egui::Shape::rect_filled(resp.rect, Rounding::same(RADIUS_BUTTON), BG_HOVER),
                );
            }
            if resp.clicked() {
                *active_idx = idx;
            }
        }
    });

    let line_y = strip_resp.response.rect.bottom().round();
    let x0 = strip_resp.response.rect.left();
    let x1 = x0 + ui.available_width().max(strip_resp.response.rect.width());
    ui.painter().line_segment(
        [egui::pos2(x0, line_y), egui::pos2(x1, line_y)],
        Stroke::new(1.0_f32, BORDER_SUBTLE),
    );
    ui.add_space(12.0);
}

pub fn section_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(display_text(title, 22.0, TEXT_PRIMARY));
    if !subtitle.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(subtitle).size(12.5).color(TEXT_SECONDARY));
    }
    ui.add_space(12.0);
}
