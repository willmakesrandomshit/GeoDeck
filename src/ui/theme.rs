use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Margin, RichText, Rounding,
    Shadow, Stroke, Vec2,
};
use std::path::Path;

use crate::ui::icons::ph;

// === Consistent Rounded Design System Tokens ===
pub const RADIUS_PANEL: f32 = 18.0; // Main content panels (16-20px)
pub const RADIUS_CARD: f32 = 14.0; // Mod cards, snapshot cards, song rows (12-16px)
pub const RADIUS_BUTTON: f32 = 11.0; // Buttons (10-12px)
pub const RADIUS_SEARCH: f32 = 12.0; // Search bars (12px)
pub const RADIUS_INPUT: f32 = 10.0; // Input fields & dropdowns (10px)
pub const RADIUS_PILL: f32 = 999.0; // Fully rounded badges & status pills
pub const RADIUS_ICON: f32 = 13.0; // Mod icons & song tiles (12-14px)
pub const RADIUS_DIALOG: f32 = 20.0; // Modal dialogs (18-22px)

// === Elevated Obsidian Dark Palette Harmonized with GeoDeck Orbital Purple ===
pub const BG_VOID: Color32 = Color32::from_rgb(11, 12, 18);
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(14, 16, 24);
pub const BG_SURFACE: Color32 = Color32::from_rgb(20, 22, 34);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(27, 30, 46);
pub const BG_HOVER: Color32 = Color32::from_rgb(35, 39, 59);
pub const BG_INPUT: Color32 = Color32::from_rgb(14, 16, 25);

pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(34, 38, 56);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(54, 60, 87);
pub const BORDER_GLOW: Color32 = Color32::from_rgb(122, 86, 214);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(246, 247, 251);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(165, 170, 186);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(112, 118, 136);

pub const ACCENT_EPIC: Color32 = Color32::from_rgb(155, 107, 255);
pub const ACCENT_EPIC_BRIGHT: Color32 = Color32::from_rgb(180, 140, 255);
pub const ACCENT_EPIC_SOFT: Color32 = Color32::from_rgb(38, 29, 62);

pub const STATUS_OK: Color32 = Color32::from_rgb(52, 211, 153);
pub const STATUS_OK_SOFT: Color32 = Color32::from_rgb(20, 48, 39);
pub const STATUS_WARN: Color32 = Color32::from_rgb(245, 176, 65);
pub const STATUS_WARN_SOFT: Color32 = Color32::from_rgb(52, 39, 19);
pub const STATUS_ERR: Color32 = Color32::from_rgb(248, 113, 113);
pub const STATUS_ERR_SOFT: Color32 = Color32::from_rgb(54, 24, 28);

fn read_first_existing_font(candidates: &[&str]) -> Option<Vec<u8>> {
    for p in candidates {
        if let Ok(bytes) = std::fs::read(Path::new(p)) {
            return Some(bytes);
        }
    }
    None
}

pub fn setup_fonts_and_theme(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let regular_bytes = read_first_existing_font(&[
        r"C:\Windows\Fonts\segoeui.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/HelveticaNeue.ttc",
        "/Library/Fonts/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ]);
    if let Some(bytes) = regular_bytes {
        fonts
            .font_data
            .insert("ui_regular".into(), FontData::from_owned(bytes));
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
            fam.insert(0, "ui_regular".into());
        }
    }

    let semibold_bytes = read_first_existing_font(&[
        r"C:\Windows\Fonts\seguisb.ttf",
        r"C:\Windows\Fonts\segoeuib.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    ]);
    if let Some(bytes) = semibold_bytes {
        fonts
            .font_data
            .insert("ui_semibold".into(), FontData::from_owned(bytes));
        fonts.families.insert(
            FontFamily::Name("semibold".into()),
            vec!["ui_semibold".into()],
        );
    } else {
        fonts.families.insert(
            FontFamily::Name("semibold".into()),
            fonts
                .families
                .get(&FontFamily::Proportional)
                .cloned()
                .unwrap_or_default(),
        );
    }

    let bold_bytes = read_first_existing_font(&[
        r"C:\Windows\Fonts\segoeuib.ttf",
        r"C:\Windows\Fonts\seguisb.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    ]);
    if let Some(bytes) = bold_bytes {
        fonts
            .font_data
            .insert("ui_bold".into(), FontData::from_owned(bytes));
        fonts.families.insert(
            FontFamily::Name("bold".into()),
            vec!["ui_bold".into()],
        );
    } else {
        fonts.families.insert(
            FontFamily::Name("bold".into()),
            fonts
                .families
                .get(&FontFamily::Proportional)
                .cloned()
                .unwrap_or_default(),
        );
    }

    let display_bytes = read_first_existing_font(&[
        r"C:\Windows\Fonts\seguibl.ttf",
        r"C:\Windows\Fonts\segoeuib.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
    ]);
    if let Some(bytes) = display_bytes {
        fonts
            .font_data
            .insert("ui_black".into(), FontData::from_owned(bytes));
        fonts.families.insert(
            FontFamily::Name("display".into()),
            vec!["ui_black".into()],
        );
    } else {
        fonts.families.insert(
            FontFamily::Name("display".into()),
            fonts
                .families
                .get(&FontFamily::Name("bold".into()))
                .cloned()
                .unwrap_or_default(),
        );
    }

    let mono_bytes = read_first_existing_font(&[
        r"C:\Windows\Fonts\consola.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/SFNSMono.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
    ]);
    if let Some(bytes) = mono_bytes {
        fonts
            .font_data
            .insert("ui_mono".into(), FontData::from_owned(bytes));
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Monospace) {
            fam.insert(0, "ui_mono".into());
        }
    }

    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    for fam_name in ["semibold", "bold", "display"] {
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Name(fam_name.into())) {
            fam.push("phosphor".into());
        }
    }
    if let Some(mono_fam) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono_fam.push("phosphor".into());
    }

    ctx.set_fonts(fonts);
    apply_epic_visuals(ctx);
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

/// Smooth GPU-interpolated 4-corner gradient rectangle via `egui::Mesh`.
pub fn paint_gradient_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    top_left: Color32,
    top_right: Color32,
    bottom_left: Color32,
    bottom_right: Color32,
) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top_left);
    mesh.colored_vertex(rect.right_top(), top_right);
    mesh.colored_vertex(rect.left_bottom(), bottom_left);
    mesh.colored_vertex(rect.right_bottom(), bottom_right);
    mesh.add_triangle(0, 1, 3);
    mesh.add_triangle(0, 3, 2);
    painter.add(egui::Shape::mesh(mesh));
}

pub fn apply_epic_visuals(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = Margin::same(18.0);
    style.spacing.indent = 16.0;
    style.spacing.interact_size.y = 32.0;
    style.animation_time = 0.14;

    let mut vis = egui::Visuals::dark();
    vis.panel_fill = BG_VOID;
    vis.window_fill = BG_SURFACE;
    vis.extreme_bg_color = BG_INPUT;
    vis.faint_bg_color = BG_ELEVATED;
    vis.code_bg_color = BG_INPUT;

    vis.window_rounding = Rounding::same(RADIUS_DIALOG);
    vis.window_stroke = Stroke::new(1.0_f32, BORDER_STRONG);
    vis.window_shadow = Shadow {
        offset: egui::vec2(0.0, 12.0),
        blur: 28.0,
        spread: 0.0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 140),
    };
    vis.menu_rounding = Rounding::same(RADIUS_CARD);
    vis.popup_shadow = Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 18.0,
        spread: 0.0,
        color: Color32::from_rgba_premultiplied(0, 0, 0, 120),
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
    vis.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
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

/// Subtle Orbital Purple ambient atmospheric wash along the top of the workspace.
pub fn paint_ambient_header_glow(painter: &egui::Painter, rect: egui::Rect) {
    let glow_rect = egui::Rect::from_min_max(
        rect.left_top(),
        egui::pos2(rect.right(), (rect.top() + 180.0).min(rect.bottom())),
    );
    let top_l = Color32::from_rgba_unmultiplied(128, 86, 224, 24);
    let top_r = Color32::from_rgba_unmultiplied(84, 110, 230, 14);
    let bot = Color32::from_rgba_unmultiplied(11, 12, 18, 0);
    paint_gradient_rect(painter, glow_rect, top_l, top_r, bot, bot);
}

/// Main content panel frame (18px rounding, subtle border & elevated surface).
pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_PANEL))
        .inner_margin(Margin::same(20.0))
        .shadow(Shadow {
            offset: egui::vec2(0.0, 6.0),
            blur: 18.0,
            spread: 0.0,
            color: Color32::from_rgba_premultiplied(0, 0, 0, 55),
        })
}

/// Mod card / snapshot card / list item frame (14px rounding).
pub fn elevated_card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_ELEVATED)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_CARD))
        .inner_margin(Margin::same(14.0))
}

/// Recessed inset surface (privacy banner, CLI preview, log viewer).
pub fn inset_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_INPUT)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_CARD))
        .inner_margin(Margin::same(14.0))
}

/// Modal dialog frame (20px rounding).
pub fn dialog_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER_STRONG))
        .rounding(Rounding::same(RADIUS_DIALOG))
        .inner_margin(Margin::same(22.0))
        .shadow(Shadow {
            offset: egui::vec2(0.0, 16.0),
            blur: 32.0,
            spread: 0.0,
            color: Color32::from_rgba_premultiplied(0, 0, 0, 175),
        })
}

/// Fully rounded pill badge.
pub fn badge_pill(ui: &mut egui::Ui, label: &str, text_color: Color32, bg: Color32) {
    egui::Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, text_color.gamma_multiply(0.28)))
        .rounding(Rounding::same(RADIUS_PILL))
        .inner_margin(Margin::symmetric(9.0, 3.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(label)
                    .size(11.0)
                    .strong()
                    .color(text_color),
            );
        });
}

/// Fully rounded pill badge with leading status indicator dot (always left-to-right).
pub fn status_dot_pill(ui: &mut egui::Ui, label: &str, color: Color32, bg: Color32) {
    egui::Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.32)))
        .rounding(Rounding::same(RADIUS_PILL))
        .inner_margin(Margin::symmetric(10.0, 4.0))
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let (dot_rect, _) =
                    ui.allocate_exact_size(Vec2::splat(9.0), egui::Sense::hover());
                ui.painter()
                    .circle_filled(dot_rect.center(), 4.5, color.gamma_multiply(0.28));
                ui.painter().circle_filled(dot_rect.center(), 2.6, color);
                ui.label(
                    RichText::new(label)
                        .size(11.0)
                        .strong()
                        .color(color),
                );
            });
        });
}

/// Primary CTA button: pure crisp white (#FFFFFF) fill with bold dark text (11px rounded, 34px min height).
pub fn epic_cta_button<'a>(icon: &str, label: &str) -> egui::Button<'a> {
    let text = format_btn_text(icon, label);
    egui::Button::new(bold_text(text, 13.0, Color32::from_rgb(13, 14, 19)))
        .fill(Color32::WHITE)
        .stroke(Stroke::NONE)
        .rounding(Rounding::same(RADIUS_BUTTON))
        .min_size(Vec2::new(0.0, 34.0))
}

/// Purple brand-accent CTA button for highlighted secondary or signature actions.
pub fn brand_cta_button<'a>(icon: &str, label: &str) -> egui::Button<'a> {
    let text = format_btn_text(icon, label);
    egui::Button::new(bold_text(text, 13.0, Color32::WHITE))
        .fill(ACCENT_EPIC)
        .stroke(Stroke::new(1.0_f32, ACCENT_EPIC_BRIGHT.gamma_multiply(0.5)))
        .rounding(Rounding::same(RADIUS_BUTTON))
        .min_size(Vec2::new(0.0, 34.0))
}

/// Elevated glass secondary button (11px rounded, 34px min height).
pub fn epic_secondary_button<'a>(icon: &str, label: &str) -> egui::Button<'a> {
    let text = format_btn_text(icon, label);
    egui::Button::new(semibold_text(text, 12.5, TEXT_PRIMARY))
        .fill(BG_ELEVATED)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_BUTTON))
        .min_size(Vec2::new(0.0, 34.0))
}

/// Sleek ghost icon button for toolbar navigation & chrome actions.
pub fn ghost_icon_button<'a>(icon: &str) -> egui::Button<'a> {
    egui::Button::new(RichText::new(icon).size(14.0).color(TEXT_SECONDARY))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_BUTTON))
        .min_size(Vec2::new(32.0, 32.0))
}

/// Soft destructive action button (11px rounded, 34px min height).
pub fn epic_danger_button<'a>(icon: &str, label: &str) -> egui::Button<'a> {
    let text = format_btn_text(icon, label);
    egui::Button::new(bold_text(text, 12.5, STATUS_ERR))
        .fill(STATUS_ERR_SOFT)
        .stroke(Stroke::new(1.0_f32, STATUS_ERR.gamma_multiply(0.42)))
        .rounding(Rounding::same(RADIUS_BUTTON))
        .min_size(Vec2::new(0.0, 34.0))
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

pub fn primary_button<'a>(label: impl Into<String>) -> egui::Button<'a> {
    epic_cta_button("", &label.into())
}

pub fn secondary_button<'a>(label: impl Into<String>) -> egui::Button<'a> {
    epic_secondary_button("", &label.into())
}

pub fn danger_button<'a>(label: impl Into<String>) -> egui::Button<'a> {
    epic_danger_button("", &label.into())
}

/// Animated 44x24 stadium toggle switch (Orbital Purple active track with crisp white thumb).
pub fn epic_toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = Vec2::new(44.0, 24.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let bg = egui::lerp(
            egui::Rgba::from(Color32::from_rgb(44, 48, 66))..=egui::Rgba::from(ACCENT_EPIC),
            how_on,
        );
        let r = rect.height() * 0.5;
        ui.painter()
            .rect_filled(rect, Rounding::same(r), Color32::from(bg));
        if response.hovered() {
            ui.painter().rect_stroke(
                rect,
                Rounding::same(r),
                Stroke::new(1.0_f32, ACCENT_EPIC_BRIGHT.gamma_multiply(0.6)),
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
            .circle_filled(egui::pos2(circle_x, rect.center().y), r - 3.5, thumb_col);
    }
    response
}

/// 12px rounded search input with leading magnifying glass, focus border, and quick clear button.
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
        .inner_margin(Margin::symmetric(12.0, 6.0))
        .show(ui, |ui| {
            ui.set_min_height(22.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.label(
                    RichText::new(ph::MAGNIFYING_GLASS)
                        .size(14.5)
                        .color(if is_focused {
                            ACCENT_EPIC_BRIGHT
                        } else {
                            TEXT_MUTED
                        }),
                );
                let clear_reserve = if query.is_empty() { 4.0 } else { 24.0 };
                let resp = ui.add(
                    egui::TextEdit::singleline(query)
                        .id(id)
                        .frame(false)
                        .desired_width((desired_width - clear_reserve - 26.0).max(90.0))
                        .hint_text(RichText::new(hint).color(TEXT_MUTED)),
                );
                if !query.is_empty()
                    && ui
                        .add(
                            egui::Button::new(
                                RichText::new(ph::X).size(12.5).color(TEXT_SECONDARY),
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

/// 10px rounded input field with subtle focus ring.
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
        .inner_margin(Margin::symmetric(11.0, 6.0))
        .show(ui, |ui| {
            ui.set_min_height(22.0);
            let resp = ui.add(
                egui::TextEdit::singleline(value)
                    .id(id)
                    .frame(false)
                    .desired_width((desired_width - 22.0).max(60.0))
                    .hint_text(RichText::new(hint).color(TEXT_MUTED)),
            );
            inner_resp = Some(resp);
        });

    inner_resp.expect("styled_text_input inner response")
}

/// Rounded icon container tile (13px radius).
pub fn icon_tile(ui: &mut egui::Ui, icon: &str, size: f32, fg: Color32, bg: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, Rounding::same(RADIUS_ICON), bg);
    p.rect_stroke(
        rect,
        Rounding::same(RADIUS_ICON),
        Stroke::new(1.0_f32, fg.gamma_multiply(0.22)),
    );
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        FontId::proportional(size * 0.46),
        fg,
    );
}

/// Centered rounded empty-state panel.
pub fn empty_state_card(ui: &mut egui::Ui, icon: &str, title: &str, subtitle: &str) {
    inset_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.vertical_centered(|ui| {
            ui.add_space(14.0);
            icon_tile(ui, icon, 48.0, ACCENT_EPIC_BRIGHT, ACCENT_EPIC_SOFT);
            ui.add_space(10.0);
            ui.label(bold_text(title, 15.0, TEXT_PRIMARY));
            if !subtitle.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new(subtitle).size(12.5).color(TEXT_SECONDARY));
            }
            ui.add_space(14.0);
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

/// Sleek segmented pill sub-tab bar.
pub fn epic_subtab_bar(ui: &mut egui::Ui, tabs: &[(&str, &str)], active_idx: &mut usize) {
    egui::Frame::none()
        .fill(BG_SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .rounding(Rounding::same(RADIUS_CARD))
        .inner_margin(Margin::symmetric(6.0, 5.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for (idx, (icon, label)) in tabs.iter().enumerate() {
                    let selected = *active_idx == idx;
                    let fill = if selected {
                        BG_ELEVATED
                    } else {
                        Color32::TRANSPARENT
                    };
                    let fg = if selected {
                        Color32::WHITE
                    } else {
                        TEXT_SECONDARY
                    };
                    let title = if icon.is_empty() {
                        (*label).to_string()
                    } else {
                        format!("{icon}  {label}")
                    };

                    let resp = egui::Frame::none()
                        .fill(fill)
                        .stroke(if selected {
                            Stroke::new(1.0_f32, BORDER_STRONG)
                        } else {
                            Stroke::NONE
                        })
                        .rounding(Rounding::same(RADIUS_BUTTON))
                        .inner_margin(Margin::symmetric(12.0, 6.0))
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(semibold_text(title, 12.5, fg))
                                    .wrap_mode(egui::TextWrapMode::Extend),
                            );
                        })
                        .response
                        .interact(egui::Sense::click());

                    if !selected && resp.hovered() {
                        ui.painter().rect_filled(
                            resp.rect,
                            Rounding::same(RADIUS_BUTTON),
                            Color32::from_rgba_premultiplied(255, 255, 255, 8),
                        );
                    }
                    if resp.clicked() {
                        *active_idx = idx;
                    }
                }
            });
        });
    ui.add_space(12.0);
}

pub fn section_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(bold_text(title, 24.0, TEXT_PRIMARY));
    if !subtitle.is_empty() {
        ui.add_space(2.0);
        ui.label(RichText::new(subtitle).size(13.0).color(TEXT_SECONDARY));
    }
    ui.add_space(12.0);
}

pub fn stat_tile(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    value: &str,
    sub: &str,
    accent: Color32,
) {
    elevated_card_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon).size(13.5).color(accent));
            ui.add(
                egui::Label::new(RichText::new(label).size(10.5).strong().color(TEXT_MUTED))
                    .truncate(),
            );
        });
        ui.add_space(6.0);
        ui.add(egui::Label::new(bold_text(value, 17.5, TEXT_PRIMARY)).truncate());
        ui.add_space(2.0);
        ui.add(egui::Label::new(RichText::new(sub).size(11.5).color(TEXT_SECONDARY)).truncate());
    });
}
