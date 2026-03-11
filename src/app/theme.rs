//! Catppuccin Mocha color palette and UI dimension constants.

use egui::Color32;

// Canvas
pub const CANVAS_BG: Color32 = Color32::from_rgb(30, 30, 46);
pub const GRID_COLOR: Color32 = Color32::from_rgba_premultiplied(69, 71, 90, 50);
pub const SELECTION_COLOR: Color32 = Color32::from_rgb(137, 180, 250);
pub const PORT_FILL: Color32 = Color32::from_rgb(49, 50, 68);
pub const BOX_SELECT_FILL: Color32 = Color32::from_rgba_premultiplied(137, 180, 250, 20);
pub const BOX_SELECT_STROKE: Color32 = Color32::from_rgba_premultiplied(137, 180, 250, 100);

// UI chrome
pub const ACCENT: Color32 = Color32::from_rgb(137, 180, 250);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(205, 214, 244);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(166, 173, 200);
pub const TEXT_DIM: Color32 = Color32::from_rgb(108, 112, 134);
pub const SURFACE0: Color32 = Color32::from_rgb(49, 50, 68);
pub const SURFACE1: Color32 = Color32::from_rgb(69, 71, 90);
pub const MANTLE: Color32 = Color32::from_rgb(24, 24, 37);

// Accent alpha variants (used for glows, selections, hover states)
pub const ACCENT_GLOW: Color32 = Color32::from_rgba_premultiplied(137, 180, 250, 30);
pub const ACCENT_HOVER: Color32 = Color32::from_rgba_premultiplied(137, 180, 250, 80);
pub const ACCENT_FAINT: Color32 = Color32::from_rgba_premultiplied(137, 180, 250, 15);
pub const ACCENT_SELECT_BG: Color32 = Color32::from_rgba_premultiplied(137, 180, 250, 40);
pub const ACCENT_SELECT_LIGHT: Color32 = Color32::from_rgba_premultiplied(137, 180, 250, 100);
pub const TEXT_HOVER_BG: Color32 = Color32::from_rgba_premultiplied(205, 214, 244, 18);

// Shadows
pub const SHADOW_LIGHT: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 40);
pub const SHADOW_MEDIUM: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 50);

// Toast
pub const TOAST_SUCCESS: Color32 = Color32::from_rgb(166, 227, 161);

// ER diagram
pub const FK_COLOR: Color32 = Color32::from_rgb(249, 226, 175);

// Minimap
pub const MINIMAP_BG: Color32 = Color32::from_rgba_premultiplied(20, 20, 20, 200);
pub const MINIMAP_BORDER: Color32 = Color32::from_rgba_premultiplied(80, 80, 80, 180);
pub const MINIMAP_NODE: Color32 = Color32::from_rgba_premultiplied(80, 160, 255, 220);
pub const MINIMAP_VP_FILL: Color32 = Color32::from_rgba_premultiplied(80, 160, 255, 30);
pub const MINIMAP_VP_STROKE: Color32 = Color32::from_rgba_premultiplied(80, 160, 255, 150);

// Edge label background
pub const EDGE_LABEL_BG: Color32 = Color32::from_rgba_premultiplied(30, 30, 30, 200);

// Section divider
pub const DIVIDER_COLOR: Color32 = Color32::from_rgba_premultiplied(69, 71, 90, 80);
pub const ROW_DIVIDER: Color32 = Color32::from_rgba_premultiplied(100, 100, 100, 60);

// Node preview (drag from toolbar)
pub const PREVIEW_FILL: Color32 = Color32::from_rgba_premultiplied(100, 160, 255, 80);

// Tooltip
pub const TOOLTIP_BG: Color32 = Color32::from_rgba_premultiplied(18, 18, 30, 230);
pub const TOOLTIP_BORDER: Color32 = Color32::from_rgba_premultiplied(69, 71, 90, 200);

// Dimensions
pub const PORT_RADIUS: f32 = 4.5;
pub const PORT_HIT_RADIUS: f32 = 12.0;
pub const TOOLBAR_WIDTH: f32 = 220.0;
pub const PROPERTIES_WIDTH: f32 = 280.0;

/// Convert [u8; 4] RGBA to egui Color32.
pub fn to_color32(rgba: [u8; 4]) -> Color32 {
    Color32::from_rgba_premultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}
