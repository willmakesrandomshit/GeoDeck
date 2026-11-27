use eframe::egui::{self, Color32, FontFamily, FontId, Pos2, Rect, RichText, Rounding, Stroke, Vec2};
use egui_phosphor::regular;

// Crisp Phosphor Regular vector icons (guaranteed present in Proportional, Bold, and Monospace)
pub mod ph {
    pub use super::regular::{
        ARROW_CLOCKWISE, ARROW_COUNTER_CLOCKWISE, ARROW_LEFT, ARROW_RIGHT, ARROWS_CLOCKWISE, BUG,
        CARET_DOWN, CARET_LEFT, CARET_RIGHT, CHECK, CHECK_CIRCLE, CLOUD_ARROW_DOWN, CODE, COMPASS,
        COMPASS as REG_COMPASS, COPY, CPU, DOTS_THREE, DOTS_THREE_OUTLINE, DOWNLOAD_SIMPLE, EXPORT,
        FILE_ARCHIVE, FLOPPY_DISK, FOLDER_OPEN, FUNNEL, GAME_CONTROLLER,
        GAME_CONTROLLER as REG_CONTROLLER, GEAR_SIX, GLOBE, HARD_DRIVES, HOUSE, INFO, KEY,
        LIGHTNING as BOLT, LIGHTNING, LIGHTNING as REG_LIGHTNING, LIST, LOCK, LOCK_OPEN,
        MAGNIFYING_GLASS, MINUS, MUSIC_NOTE, PACKAGE, PENCIL_SIMPLE, PLAY, PLAY as REG_PLAY, PLUS,
        PUZZLE_PIECE, SHIELD_CHECK, SHIELD_CHECK as REG_SHIELD, SHIELD_WARNING, SLIDERS,
        SORT_ASCENDING, SPARKLE, SPEAKER_HIGH, SQUARE, SQUARES_FOUR, STACK as LAYERS, STAR, STOP,
        TAG, TRASH, UPLOAD_SIMPLE, USER_CIRCLE, USERS, WARNING, WRENCH, X, X_CIRCLE,
    };
}

pub fn icon_rich(glyph: &str, size: f32, color: Color32) -> RichText {
    RichText::new(glyph).size(size).color(color)
}

/// Pure white shield monolith emblem at top-left of sidebar (matching the reference Epic Games shield).
pub fn paint_brand_crest(painter: &egui::Painter, rect: Rect) {
    let c = rect.center();
    let w = rect.width() * 0.44;
    let h = rect.height() * 0.48;

    let shield = vec![
        Pos2::new(c.x - w, c.y - h),
        Pos2::new(c.x + w, c.y - h),
        Pos2::new(c.x + w, c.y + h * 0.56),
        Pos2::new(c.x, c.y + h),
        Pos2::new(c.x - w, c.y + h * 0.56),
    ];
    painter.add(egui::Shape::convex_polygon(
        shield,
        Color32::from_rgb(245, 245, 248),
        Stroke::NONE,
    ));

    painter.text(
        Pos2::new(c.x, c.y - h * 0.28),
        egui::Align2::CENTER_CENTER,
        "GEODE",
        FontId::new(rect.height() * 0.22, FontFamily::Name("bold".into())),
        Color32::from_rgb(13, 13, 15),
    );
    painter.text(
        Pos2::new(c.x, c.y + h * 0.16),
        egui::Align2::CENTER_CENTER,
        "DECK",
        FontId::new(rect.height() * 0.18, FontFamily::Name("bold".into())),
        Color32::from_rgb(13, 13, 15),
    );
    painter.line_segment(
        [
            Pos2::new(c.x - w * 0.62, c.y + h * 0.44),
            Pos2::new(c.x + w * 0.62, c.y + h * 0.44),
        ],
        Stroke::new(1.5_f32, Color32::from_rgb(13, 13, 15)),
    );
}

/// Stylized Geometry Dash cube thumbnail used for square thumbnails & cover cards.
pub fn paint_gd_cube_badge(painter: &egui::Painter, rect: Rect, accent: Color32) {
    let p = painter.with_clip_rect(rect);
    let dark_corner = Color32::from_rgb(18, 20, 26);
    crate::ui::theme::paint_gradient_rect(
        &p,
        rect,
        accent.gamma_multiply(0.55),
        accent.gamma_multiply(0.92),
        dark_corner,
        accent.gamma_multiply(0.35),
    );

    let pad = rect.width() * 0.22;
    let inner = rect.shrink(pad);
    p.rect_filled(inner, Rounding::same(4.0), accent);

    let eye_w = inner.width() * 0.22;
    let eye_y = inner.top() + inner.height() * 0.27;
    let left_eye = Rect::from_min_size(
        Pos2::new(inner.left() + inner.width() * 0.18, eye_y),
        Vec2::splat(eye_w),
    );
    let right_eye = Rect::from_min_size(
        Pos2::new(inner.right() - inner.width() * 0.18 - eye_w, eye_y),
        Vec2::splat(eye_w),
    );
    let dark = Color32::from_rgb(14, 15, 19);
    p.rect_filled(left_eye, Rounding::same(1.5), dark);
    p.rect_filled(right_eye, Rounding::same(1.5), dark);

    let mouth = Rect::from_min_size(
        Pos2::new(
            inner.left() + inner.width() * 0.2,
            inner.top() + inner.height() * 0.64,
        ),
        Vec2::new(inner.width() * 0.6, inner.height() * 0.16),
    );
    p.rect_filled(mouth, Rounding::same(1.5), dark);
}

/// Renders rich cinematic Epic Store Hero Banner artwork with smooth GPU gradients & left dark scrim.
pub fn paint_hero_artwork(painter: &egui::Painter, rect: Rect, spotlight_idx: usize) {
    let (tl, tr, bl, br, cube_accent, orb_col) = match spotlight_idx {
        0 => (
            Color32::from_rgb(14, 15, 19),
            Color32::from_rgb(38, 44, 62),
            Color32::from_rgb(12, 12, 16),
            Color32::from_rgb(24, 28, 42),
            Color32::from_rgb(56, 189, 248),
            Color32::from_rgba_premultiplied(120, 175, 255, 34),
        ),
        1 => (
            Color32::from_rgb(15, 14, 20),
            Color32::from_rgb(58, 34, 88),
            Color32::from_rgb(12, 12, 16),
            Color32::from_rgb(28, 22, 46),
            Color32::from_rgb(168, 85, 247),
            Color32::from_rgba_premultiplied(168, 85, 247, 36),
        ),
        2 => (
            Color32::from_rgb(13, 18, 18),
            Color32::from_rgb(20, 66, 56),
            Color32::from_rgb(12, 14, 16),
            Color32::from_rgb(16, 36, 34),
            Color32::from_rgb(46, 204, 113),
            Color32::from_rgba_premultiplied(46, 204, 113, 32),
        ),
        3 => (
            Color32::from_rgb(18, 16, 14),
            Color32::from_rgb(76, 52, 22),
            Color32::from_rgb(13, 12, 12),
            Color32::from_rgb(36, 26, 18),
            Color32::from_rgb(245, 158, 11),
            Color32::from_rgba_premultiplied(245, 158, 11, 32),
        ),
        _ => (
            Color32::from_rgb(20, 14, 16),
            Color32::from_rgb(78, 26, 38),
            Color32::from_rgb(13, 12, 14),
            Color32::from_rgb(38, 18, 26),
            Color32::from_rgb(244, 63, 94),
            Color32::from_rgba_premultiplied(244, 63, 94, 34),
        ),
    };

    painter.rect_filled(rect, Rounding::same(14.0), tl);
    let p = painter.with_clip_rect(rect);
    crate::ui::theme::paint_gradient_rect(&p, rect, tl, tr, bl, br);

    p.circle_filled(
        Pos2::new(rect.right() - rect.width() * 0.24, rect.top() + rect.height() * 0.36),
        rect.height() * 0.62,
        orb_col,
    );

    let beam_col = Color32::from_rgba_premultiplied(255, 255, 255, 11);
    for i in 0..7 {
        let off = i as f32 * 48.0;
        p.line_segment(
            [
                Pos2::new(rect.center().x + off - 30.0, rect.top()),
                Pos2::new(rect.center().x + off + 140.0, rect.bottom()),
            ],
            Stroke::new(1.0_f32, beam_col),
        );
    }

    let cube_size = (rect.height() * 0.46).min(142.0);
    let cube_rect = Rect::from_center_size(
        Pos2::new(rect.right() - cube_size * 1.1, rect.center().y),
        Vec2::splat(cube_size),
    );
    paint_gd_cube_badge(&p, cube_rect, cube_accent);

    let scrim = Rect::from_min_max(
        rect.left_top(),
        Pos2::new(rect.left() + rect.width() * 0.56, rect.bottom()),
    );
    crate::ui::theme::paint_gradient_rect(
        &p,
        scrim,
        Color32::from_rgba_premultiplied(12, 12, 15, 210),
        Color32::from_rgba_premultiplied(12, 12, 15, 0),
        Color32::from_rgba_premultiplied(12, 12, 15, 225),
        Color32::from_rgba_premultiplied(12, 12, 15, 0),
    );
}

/// Renders square/vertical cover art tiles for Discover More & Library cards.
pub fn paint_poster_artwork(
    painter: &egui::Painter,
    rect: Rect,
    seed: usize,
    is_active: bool,
) {
    let palettes = [
        (
            Color32::from_rgb(24, 42, 74),
            Color32::from_rgb(42, 118, 210),
            Color32::from_rgb(16, 20, 32),
            Color32::from_rgb(56, 189, 248),
        ),
        (
            Color32::from_rgb(30, 56, 36),
            Color32::from_rgb(76, 148, 62),
            Color32::from_rgb(18, 26, 20),
            Color32::from_rgb(110, 214, 92),
        ),
        (
            Color32::from_rgb(42, 24, 72),
            Color32::from_rgb(122, 62, 214),
            Color32::from_rgb(20, 16, 34),
            Color32::from_rgb(168, 85, 247),
        ),
        (
            Color32::from_rgb(74, 24, 62),
            Color32::from_rgb(218, 62, 142),
            Color32::from_rgb(28, 16, 28),
            Color32::from_rgb(244, 114, 182),
        ),
        (
            Color32::from_rgb(28, 36, 78),
            Color32::from_rgb(92, 120, 232),
            Color32::from_rgb(18, 20, 36),
            Color32::from_rgb(129, 140, 248),
        ),
        (
            Color32::from_rgb(86, 22, 32),
            Color32::from_rgb(214, 52, 68),
            Color32::from_rgb(28, 14, 18),
            Color32::from_rgb(251, 113, 133),
        ),
    ];
    let (tl, tr, bottom, accent) = palettes[seed % palettes.len()];

    let p = painter.with_clip_rect(rect);
    crate::ui::theme::paint_gradient_rect(&p, rect, tl, tr, bottom, bottom);

    let stripe = vec![
        Pos2::new(rect.left(), rect.bottom() - 22.0),
        Pos2::new(rect.right(), rect.top() + 26.0),
        Pos2::new(rect.right(), rect.top() + 48.0),
        Pos2::new(rect.left(), rect.bottom()),
    ];
    p.add(egui::Shape::convex_polygon(
        stripe,
        Color32::from_rgba_premultiplied(255, 255, 255, 18),
        Stroke::NONE,
    ));

    let badge_sz = (rect.width().min(rect.height()) * 0.44).clamp(32.0, 62.0);
    let badge_rect = Rect::from_center_size(rect.center(), Vec2::splat(badge_sz));
    paint_gd_cube_badge(&p, badge_rect, accent);

    if is_active {
        p.circle_filled(
            Pos2::new(rect.right() - 12.0, rect.top() + 12.0),
            4.5,
            Color32::from_rgb(46, 204, 113),
        );
    }
}

/// Anti-aliased 270-degree radial gauge for Performance Readiness.
pub fn paint_radial_gauge(ui: &mut egui::Ui, score: u32, subtitle: &str, diameter: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(diameter), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let c = rect.center();
    let radius = diameter * 0.41;

    let accent = if score >= 85 {
        Color32::from_rgb(46, 204, 113)
    } else if score >= 65 {
        Color32::from_rgb(245, 166, 35)
    } else {
        Color32::from_rgb(239, 68, 68)
    };

    let start_angle = std::f32::consts::PI * 0.75;
    let total_sweep = std::f32::consts::PI * 1.5;
    let steps = 48usize;

    let track_pts: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let a = start_angle + total_sweep * t;
            Pos2::new(c.x + radius * a.cos(), c.y + radius * a.sin())
        })
        .collect();
    painter.add(egui::Shape::line(
        track_pts,
        Stroke::new(7.5_f32, Color32::from_rgb(38, 38, 44)),
    ));

    let frac = (score as f32 / 100.0).clamp(0.02, 1.0);
    let fill_steps = ((steps as f32) * frac).round().max(2.0) as usize;
    let fill_pts: Vec<Pos2> = (0..=fill_steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let a = start_angle + total_sweep * t;
            Pos2::new(c.x + radius * a.cos(), c.y + radius * a.sin())
        })
        .collect();
    painter.add(egui::Shape::line(fill_pts, Stroke::new(7.5_f32, accent)));

    painter.text(
        Pos2::new(c.x, c.y - 4.0),
        egui::Align2::CENTER_CENTER,
        format!("{score}"),
        FontId::new(diameter * 0.28, FontFamily::Name("bold".into())),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(c.x, c.y + diameter * 0.19),
        egui::Align2::CENTER_CENTER,
        subtitle,
        FontId::proportional(10.5),
        Color32::from_rgb(158, 158, 168),
    );
}

/// Decodes the embedded square GeoDeck Saturn icon for the OS window & taskbar.
pub fn generate_window_icon() -> egui::IconData {
    let rgba = image::load_from_memory(crate::ui::assets::BRAND_ICON_BYTES)
        .expect("embedded geodeck_icon.png")
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    }
}
