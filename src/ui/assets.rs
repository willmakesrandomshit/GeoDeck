use eframe::egui::{self, ColorImage, TextureHandle};
use image::{imageops, Rgba, RgbaImage};

pub const BRAND_LOCKUP_BYTES: &[u8] = include_bytes!("../../assets/geodeck_logo_lockup.png");
pub const BRAND_ICON_BYTES: &[u8] = include_bytes!("../../assets/geodeck_icon.png");

pub const UTIL_BYTES: [&[u8]; 6] = [
    include_bytes!("../../assets/util_0.jpg"),
    include_bytes!("../../assets/util_1.jpg"),
    include_bytes!("../../assets/util_2.jpg"),
    include_bytes!("../../assets/util_3.jpg"),
    include_bytes!("../../assets/util_4.jpg"),
    include_bytes!("../../assets/util_5.jpg"),
];

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
            // Smooth 1.2px anti-aliased edge
            let alpha_mul = (0.5 - dist / 1.2).clamp(0.0, 1.0);
            if alpha_mul < 1.0 {
                let px = img.get_pixel_mut(x, y);
                px[3] = (px[3] as f32 * alpha_mul).round() as u8;
            }
        }
    }
}

pub fn load_game_background(ctx: &egui::Context, game_dir: &std::path::Path) -> Option<TextureHandle> {
    let resources = game_dir.join("Resources");
    for name in ["game_bg_01_001-hd.png", "game_bg_01_001.png", "game_bg_01_001-uhd.png"] {
        let Ok(image) = image::open(resources.join(name)) else {
            continue;
        };
        let mut rgba = imageops::resize(&image.to_rgba8(), 420, 270, imageops::FilterType::Triangle);
        let (w, h) = rgba.dimensions();
        for y in 0..h {
            let ny = y as f32 / h as f32;
            for x in 0..w {
                let nx = x as f32 / w as f32;
                let px = rgba.get_pixel_mut(x, y);
                let luma = (0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32) / 255.0;
                // Rich Orbital Purple -> Midnight Indigo gradient tint with soft vignette
                let dx = nx - 0.52;
                let dy = ny - 0.45;
                let vig = (1.0 - (dx * dx + dy * dy).sqrt() * 0.72).clamp(0.45, 1.0);
                let grad_r = 104.0 * (1.0 - nx * 0.35) + 52.0 * nx;
                let grad_g = 56.0 * (1.0 - ny * 0.3) + 64.0 * nx;
                let grad_b = 208.0 * (1.0 - ny * 0.15) + 38.0;
                let base_r = 18.0 + (grad_r * (0.28 + luma * 0.72)) * vig;
                let base_g = 19.0 + (grad_g * (0.24 + luma * 0.68)) * vig;
                let base_b = 32.0 + (grad_b * (0.34 + luma * 0.68)) * vig;
                px[0] = base_r.clamp(0.0, 255.0) as u8;
                px[1] = base_g.clamp(0.0, 255.0) as u8;
                px[2] = base_b.clamp(0.0, 255.0) as u8;
                px[3] = 255;
            }
        }
        round_corners_rgba(&mut rgba, 16.0, false);
        let pixels = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        return Some(ctx.load_texture("geometry_dash_background_tinted", pixels, egui::TextureOptions::LINEAR));
    }
    None
}

pub fn load_util_cover_texture(ctx: &egui::Context, idx: usize) -> TextureHandle {
    let bytes = UTIL_BYTES[idx % UTIL_BYTES.len()];
    let mut rgba = image::load_from_memory(bytes)
        .expect("embedded util jpeg")
        .to_rgba8();
    round_corners_rgba(&mut rgba, 14.0, false);
    let (w, h) = rgba.dimensions();
    let cimg = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    ctx.load_texture(format!("epic_util_{idx}"), cimg, egui::TextureOptions::LINEAR)
}

/// Extracts dominant vivid accent color from a mod logo, or falls back to curated palette.
fn dominant_logo_color(logo: &RgbaImage, fallback_seed: usize) -> [f32; 3] {
    let mut sum_r = 0.0_f32;
    let mut sum_g = 0.0_f32;
    let mut sum_b = 0.0_f32;
    let mut weight_sum = 0.0_f32;

    for px in logo.pixels() {
        let a = px[3] as f32 / 255.0;
        if a < 0.2 {
            continue;
        }
        let r = px[0] as f32;
        let g = px[1] as f32;
        let b = px[2] as f32;
        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let sat = if max_c > 1.0 { (max_c - min_c) / max_c } else { 0.0 };
        let w = a * (0.25 + sat * 2.0);
        sum_r += r * w;
        sum_g += g * w;
        sum_b += b * w;
        weight_sum += w;
    }

    if weight_sum > 10.0 {
        return [sum_r / weight_sum, sum_g / weight_sum, sum_b / weight_sum];
    }

    const FALLBACKS: [[f32; 3]; 6] = [
        [56.0, 152.0, 245.0],
        [152.0, 92.0, 246.0],
        [46.0, 204.0, 113.0],
        [245.0, 158.0, 11.0],
        [244.0, 63.0, 94.0],
        [38.0, 187.0, 255.0],
    ];
    FALLBACKS[fallback_seed % FALLBACKS.len()]
}

/// Composites a crisp 240x240 Epic Store-style square mod cover card:
/// rich radial studio spotlight matching the mod's accent color + subtle drop shadow + centered mod logo + smooth rounded top corners.
pub fn compose_mod_cover_texture(
    ctx: &egui::Context,
    mod_id: &str,
    logo_rgba: Option<&(u32, u32, Vec<u8>)>,
    seed_idx: usize,
) -> TextureHandle {
    let size = 240u32;
    let mut canvas = RgbaImage::new(size, size);

    let logo_img = logo_rgba.and_then(|(w, h, bytes)| {
        RgbaImage::from_raw(*w, *h, bytes.clone())
    });

    let accent = if let Some(ref li) = logo_img {
        dominant_logo_color(li, seed_idx)
    } else {
        // Fallback: use one of our 6 high-res utility covers as the base artwork if logo isn't loaded yet
        return load_util_cover_texture(ctx, seed_idx);
    };

    // Render smooth radial studio backdrop with subtle diagonal sheen
    for y in 0..size {
        let ny = y as f32 / size as f32;
        for x in 0..size {
            let nx = x as f32 / size as f32;
            let dx = nx - 0.5;
            let dy = ny - 0.44;
            let radial = (1.0 - (dx * dx + dy * dy).sqrt() * 1.45).clamp(0.0, 1.0);
            let sheen = ((nx * 0.7 + ny * 0.3 - 0.3).abs() < 0.08) as u8 as f32 * 0.04;
            let base_dark = 18.0 + (1.0 - ny) * 14.0;

            let r = (base_dark + accent[0] * (0.20 + radial * 0.48) + sheen * 255.0).clamp(0.0, 255.0) as u8;
            let g = (base_dark + accent[1] * (0.20 + radial * 0.48) + sheen * 255.0).clamp(0.0, 255.0) as u8;
            let b = (base_dark + accent[2] * (0.22 + radial * 0.50) + sheen * 255.0).clamp(0.0, 255.0) as u8;
            canvas.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    if let Some(li) = logo_img {
        // Fit crisply at 104x104 inside 240x240 card with balanced breathing room
        let target_dim = 104u32;
        let scaled = if li.width() == target_dim && li.height() == target_dim {
            li
        } else {
            imageops::resize(&li, target_dim, target_dim, imageops::FilterType::Lanczos3)
        };
        let ox = (size - scaled.width()) / 2;
        let oy = (size - scaled.height()) / 2;

        // Soft shadow underneath logo
        for y in 0..scaled.height() {
            for x in 0..scaled.width() {
                let sa = scaled.get_pixel(x, y)[3] as f32 / 255.0;
                if sa > 0.05 {
                    let sx = ox + x;
                    let sy = (oy + y + 6).min(size - 1);
                    let dst = canvas.get_pixel_mut(sx, sy);
                    let dark_mul = 1.0 - sa * 0.42;
                    dst[0] = (dst[0] as f32 * dark_mul) as u8;
                    dst[1] = (dst[1] as f32 * dark_mul) as u8;
                    dst[2] = (dst[2] as f32 * dark_mul) as u8;
                }
            }
        }

        // Alpha-blend crisp logo
        for y in 0..scaled.height() {
            for x in 0..scaled.width() {
                let sp = scaled.get_pixel(x, y);
                let sa = sp[3] as f32 / 255.0;
                if sa <= 0.0 {
                    continue;
                }
                let dst = canvas.get_pixel_mut(ox + x, oy + y);
                let inv = 1.0 - sa;
                dst[0] = (sp[0] as f32 * sa + dst[0] as f32 * inv).round() as u8;
                dst[1] = (sp[1] as f32 * sa + dst[1] as f32 * inv).round() as u8;
                dst[2] = (sp[2] as f32 * sa + dst[2] as f32 * inv).round() as u8;
            }
        }
    }

    round_corners_rgba(&mut canvas, 14.0, false);
    let cimg = ColorImage::from_rgba_unmultiplied([size as usize, size as usize], &canvas);
    ctx.load_texture(
        format!("mod_cover_{mod_id}_{}", logo_rgba.is_some()),
        cimg,
        egui::TextureOptions::LINEAR,
    )
}

/// Supersampled crisp pure-white Epic-style brand shield emblem with clean geometric GD crest.
pub fn load_brand_shield_texture(ctx: &egui::Context) -> TextureHandle {
    let w = 80u32;
    let h = 92u32;
    let mut img = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));

    // Draw smooth SDF white shield silhouette (rounded top corners, angled bottom point)
    for y in 0..h {
        let fy = y as f32 + 0.5;
        for x in 0..w {
            let fx = x as f32 + 0.5;
            // Shield width bounds
            let margin_x = 6.0_f32;
            let margin_top = 4.0_f32;
            let shoulder_y = 66.0_f32;
            let tip_y = 88.0_f32;

            let in_x = fx >= margin_x && fx <= (w as f32 - margin_x);
            let in_top = fy >= margin_top;
            if !in_x || !in_top || fy > tip_y {
                continue;
            }

            // Bottom tapered V check
            let mut alpha = 1.0_f32;
            if fy > shoulder_y {
                let t = (fy - shoulder_y) / (tip_y - shoulder_y);
                let half_w = ((w as f32 * 0.5) - margin_x) * (1.0 - t);
                let dist_x = (fx - w as f32 * 0.5).abs() - half_w;
                alpha = (0.5 - dist_x / 1.2).clamp(0.0, 1.0);
            }

            // Rounded top-left / top-right corners (r = 5.0)
            let r = 5.0_f32;
            if fy < margin_top + r {
                if fx < margin_x + r {
                    let d = ((fx - (margin_x + r)).powi(2) + (fy - (margin_top + r)).powi(2)).sqrt() - r;
                    alpha = alpha.min((0.5 - d / 1.2).clamp(0.0, 1.0));
                } else if fx > (w as f32 - margin_x - r) {
                    let d = ((fx - (w as f32 - margin_x - r)).powi(2) + (fy - (margin_top + r)).powi(2)).sqrt() - r;
                    alpha = alpha.min((0.5 - d / 1.2).clamp(0.0, 1.0));
                }
            }

            if alpha > 0.01 {
                // Carve crisp dark inner geometric GD cube icon + horizontal underline inside white shield
                let cx = w as f32 * 0.5;
                let cy = 33.0_f32;
                let in_cube_outer = (fx - cx).abs() < 16.0 && (fy - cy).abs() < 16.0;
                let in_cube_ring = in_cube_outer && ((fx - cx).abs() > 12.0 || (fy - cy).abs() > 12.0);
                let in_eye_l = (fx - (cx - 5.5)).abs() < 2.6 && (fy - (cy - 3.5)).abs() < 2.6;
                let in_eye_r = (fx - (cx + 5.5)).abs() < 2.6 && (fy - (cy - 3.5)).abs() < 2.6;
                let in_mouth = (fx - cx).abs() < 7.5 && (fy - (cy + 5.5)).abs() < 2.1;
                let in_bar = (fx - cx).abs() < 17.0 && (fy - 58.0).abs() < 1.8;

                if in_cube_ring || in_eye_l || in_eye_r || in_mouth || in_bar {
                    img.put_pixel(x, y, Rgba([13, 13, 15, (255.0 * alpha) as u8]));
                } else {
                    img.put_pixel(x, y, Rgba([244, 244, 248, (255.0 * alpha) as u8]));
                }
            }
        }
    }

    let cimg = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &img);
    ctx.load_texture("epic_brand_shield", cimg, egui::TextureOptions::LINEAR)
}
