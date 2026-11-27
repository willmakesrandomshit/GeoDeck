use eframe::egui;

// Crisp Phosphor Regular vector icons (guaranteed present in Proportional, Bold, Display, and Monospace)
pub mod ph {
    pub use super::regular::{
        ARROW_CLOCKWISE, ARROW_COUNTER_CLOCKWISE, ARROW_LEFT, ARROW_RIGHT, ARROWS_CLOCKWISE, BUG,
        CARET_DOWN, CARET_LEFT, CARET_RIGHT, CHECK, CHECK_CIRCLE, CLOUD_ARROW_DOWN, CODE, COMPASS,
        COMPASS as REG_COMPASS, COPY, CPU, DIAMOND, DOTS_THREE, DOTS_THREE_OUTLINE,
        DOWNLOAD_SIMPLE, EXPORT, FILE_ARCHIVE, FIRE, FLOPPY_DISK, FOLDER_OPEN, FUNNEL,
        GAME_CONTROLLER, GAME_CONTROLLER as REG_CONTROLLER, GEAR_SIX, GLOBE, HARD_DRIVES, HOUSE,
        IMAGE, INFO, KEY, KEYBOARD, LIGHTNING as BOLT, LIGHTNING, LIGHTNING as REG_LIGHTNING, LIST,
        LOCK, LOCK_OPEN, MAGNIFYING_GLASS, MINUS, MUSIC_NOTE, PACKAGE, PALETTE, PENCIL_SIMPLE,
        PLAY, PLAY as REG_PLAY, PLUS, PUZZLE_PIECE, SHIELD_CHECK, SHIELD_CHECK as REG_SHIELD,
        SHIELD_WARNING, SLIDERS, SORT_ASCENDING, SPARKLE, SPEAKER_HIGH, SQUARE, SQUARES_FOUR,
        STACK as LAYERS, STAR, STOP, TAG, TRASH, UPLOAD_SIMPLE, USER_CIRCLE, USERS, WARNING,
        WRENCH, X, X_CIRCLE,
    };
}

use egui_phosphor::regular;

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
