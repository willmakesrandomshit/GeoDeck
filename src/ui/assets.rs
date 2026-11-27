use eframe::egui::{self, ColorImage, TextureHandle};
use image::RgbaImage;

pub const BRAND_LOCKUP_BYTES: &[u8] = include_bytes!("../../assets/geodeck_logo_lockup.png");
pub const BRAND_ICON_BYTES: &[u8] = include_bytes!("../../assets/geodeck_icon.png");
pub const GD_HERO_BYTES: &[u8] = include_bytes!("../../assets/gd_hero.jpg");

pub fn load_brand_lockup_texture(ctx: &egui::Context) -> TextureHandle {
    let rgba = image::load_from_memory(BRAND_LOCKUP_BYTES)
        .expect("embedded geodeck_logo_lockup.png")
        .to_rgba8();
    let (w, h) = rgba.dimensions();
    let cimg = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    ctx.load_texture("geodeck_brand_lockup", cimg, egui::TextureOptions::LINEAR)
}

pub fn load_brand_icon_texture(ctx: &egui::Context) -> TextureHandle {
    let rgba = image::load_from_memory(BRAND_ICON_BYTES)
        .expect("embedded geodeck_icon.png")
        .to_rgba8();
    let (w, h) = rgba.dimensions();
    let cimg = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    ctx.load_texture("geodeck_brand_icon", cimg, egui::TextureOptions::LINEAR)
}

/// Smooth SDF anti-aliased corner rounding for RGBA images (radius in pixels).
pub fn round_corners_rgba(img: &mut RgbaImage, radius: f32, top_only: bool) {
    let w = img.width() as f32;
    let h = img.height() as f32;
    if radius <= 0.5 || w < 2.0 || h < 2.0 {
        return;
    }

    for y in 0..img.height() {
        let fy = y as f32 + 0.5;
        let in_top = fy < radius;
        let in_bot = !top_only && fy > (h - radius);
        if !in_top && !in_bot {
            continue;
        }
        for x in 0..img.width() {
            let fx = x as f32 + 0.5;
            let in_left = fx < radius;
            let in_right = fx > (w - radius);
            if !in_left && !in_right {
                continue;
            }
            let cx = if in_left { radius } else { w - radius };
            let cy = if in_top { radius } else { h - radius };
            let dist = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt() - radius;
            let alpha_mul = (0.5 - dist / 1.2).clamp(0.0, 1.0);
            if alpha_mul < 1.0 {
                let px = img.get_pixel_mut(x, y);
                px[3] = (px[3] as f32 * alpha_mul).round() as u8;
            }
        }
    }
}

/// Loads official Geometry Dash capsule hero artwork with crisp anti-aliased rounded corners.
pub fn load_game_background(ctx: &egui::Context, _game_dir: &std::path::Path) -> Option<TextureHandle> {
    let mut rgba = image::load_from_memory(GD_HERO_BYTES).ok()?.to_rgba8();
    let (w, h) = rgba.dimensions();
    round_corners_rgba(&mut rgba, 14.0, false);
    let pixels = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    Some(ctx.load_texture(
        "geometry_dash_hero_art",
        pixels,
        egui::TextureOptions::LINEAR,
    ))
}
