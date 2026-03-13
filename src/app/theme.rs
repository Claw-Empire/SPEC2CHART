//! Color theme system with dark/light mode support.
//! Dark theme: Catppuccin Mocha. Light theme: Catppuccin Latte.

use egui::Color32;

/// All theme colors, resolved for the current dark/light mode.
#[derive(Clone)]
pub struct Theme {
    // Canvas
    pub canvas_bg: Color32,
    pub grid_color: Color32,
    pub grid_major_color: Color32,
    pub selection_color: Color32,
    pub port_fill: Color32,
    pub box_select_fill: Color32,
    pub box_select_stroke: Color32,

    // UI chrome
    pub accent: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_dim: Color32,
    pub surface0: Color32,
    pub surface1: Color32,
    pub mantle: Color32,

    // Accent alpha variants
    pub accent_glow: Color32,
    pub accent_hover: Color32,
    pub accent_faint: Color32,
    pub accent_select_bg: Color32,
    pub accent_select_light: Color32,
    pub text_hover_bg: Color32,

    // Shadows
    pub shadow_light: Color32,
    pub shadow_medium: Color32,

    // Toast
    pub toast_success: Color32,

    // ER diagram
    pub fk_color: Color32,

    // Minimap
    pub minimap_bg: Color32,
    pub minimap_border: Color32,
    pub minimap_node: Color32,
    pub minimap_vp_fill: Color32,
    pub minimap_vp_stroke: Color32,

    // Edge label background
    pub edge_label_bg: Color32,

    // Section divider
    pub divider_color: Color32,
    pub row_divider: Color32,

    // Node preview (drag from toolbar)
    pub preview_fill: Color32,

    // Tooltip
    pub tooltip_bg: Color32,
    pub tooltip_border: Color32,

    // Ghost / overlay colors
    pub ghost_stroke: Color32,
    pub dim_overlay: Color32,
    pub dim_overlay_heavy: Color32,
    pub focus_dim_near: Color32,
    pub focus_dim_far: Color32,

    // Visuals setup helpers
    pub crust: Color32,
    pub surface2: Color32,
    pub lavender: Color32,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            canvas_bg: Color32::from_rgb(30, 30, 46),
            grid_color: Color32::from_rgba_premultiplied(69, 71, 90, 50),
            grid_major_color: Color32::from_rgba_premultiplied(88, 91, 112, 90),
            selection_color: Color32::from_rgb(137, 180, 250),
            port_fill: Color32::from_rgb(49, 50, 68),
            box_select_fill: Color32::from_rgba_premultiplied(137, 180, 250, 20),
            box_select_stroke: Color32::from_rgba_premultiplied(137, 180, 250, 100),

            accent: Color32::from_rgb(137, 180, 250),
            text_primary: Color32::from_rgb(205, 214, 244),
            text_secondary: Color32::from_rgb(166, 173, 200),
            text_dim: Color32::from_rgb(108, 112, 134),
            surface0: Color32::from_rgb(49, 50, 68),
            surface1: Color32::from_rgb(69, 71, 90),
            mantle: Color32::from_rgb(24, 24, 37),

            accent_glow: Color32::from_rgba_premultiplied(137, 180, 250, 30),
            accent_hover: Color32::from_rgba_premultiplied(137, 180, 250, 80),
            accent_faint: Color32::from_rgba_premultiplied(137, 180, 250, 15),
            accent_select_bg: Color32::from_rgba_premultiplied(137, 180, 250, 40),
            accent_select_light: Color32::from_rgba_premultiplied(137, 180, 250, 100),
            text_hover_bg: Color32::from_rgba_premultiplied(205, 214, 244, 18),

            shadow_light: Color32::from_rgba_premultiplied(0, 0, 0, 40),
            shadow_medium: Color32::from_rgba_premultiplied(0, 0, 0, 50),

            toast_success: Color32::from_rgb(166, 227, 161),

            fk_color: Color32::from_rgb(249, 226, 175),

            minimap_bg: Color32::from_rgba_premultiplied(20, 20, 20, 200),
            minimap_border: Color32::from_rgba_premultiplied(80, 80, 80, 180),
            minimap_node: Color32::from_rgba_premultiplied(80, 160, 255, 220),
            minimap_vp_fill: Color32::from_rgba_premultiplied(80, 160, 255, 30),
            minimap_vp_stroke: Color32::from_rgba_premultiplied(80, 160, 255, 150),

            edge_label_bg: Color32::from_rgba_premultiplied(30, 30, 30, 200),

            divider_color: Color32::from_rgba_premultiplied(69, 71, 90, 80),
            row_divider: Color32::from_rgba_premultiplied(100, 100, 100, 60),

            preview_fill: Color32::from_rgba_premultiplied(100, 160, 255, 80),

            tooltip_bg: Color32::from_rgba_premultiplied(18, 18, 30, 230),
            tooltip_border: Color32::from_rgba_premultiplied(69, 71, 90, 200),

            ghost_stroke: Color32::from_rgba_premultiplied(137, 180, 250, 60),
            dim_overlay: Color32::from_rgba_premultiplied(16, 16, 28, 180),
            dim_overlay_heavy: Color32::from_rgba_premultiplied(12, 12, 22, 175),
            focus_dim_near: Color32::from_rgba_premultiplied(16, 16, 28, 90),
            focus_dim_far: Color32::from_rgba_premultiplied(16, 16, 28, 190),

            crust: Color32::from_rgb(17, 17, 27),
            surface2: Color32::from_rgb(88, 91, 112),
            lavender: Color32::from_rgb(180, 190, 254),
        }
    }

    pub fn light() -> Self {
        Self {
            canvas_bg: Color32::from_rgb(239, 241, 245),
            grid_color: Color32::from_rgba_premultiplied(172, 176, 190, 60),
            grid_major_color: Color32::from_rgba_premultiplied(140, 143, 161, 100),
            selection_color: Color32::from_rgb(30, 102, 245),
            port_fill: Color32::from_rgb(204, 208, 218),
            box_select_fill: Color32::from_rgba_premultiplied(30, 102, 245, 25),
            box_select_stroke: Color32::from_rgba_premultiplied(30, 102, 245, 120),

            accent: Color32::from_rgb(30, 102, 245),
            text_primary: Color32::from_rgb(76, 79, 105),
            text_secondary: Color32::from_rgb(92, 95, 119),
            text_dim: Color32::from_rgb(140, 143, 161),
            surface0: Color32::from_rgb(204, 208, 218),
            surface1: Color32::from_rgb(188, 192, 204),
            mantle: Color32::from_rgb(230, 233, 239),

            accent_glow: Color32::from_rgba_premultiplied(30, 102, 245, 30),
            accent_hover: Color32::from_rgba_premultiplied(30, 102, 245, 80),
            accent_faint: Color32::from_rgba_premultiplied(30, 102, 245, 15),
            accent_select_bg: Color32::from_rgba_premultiplied(30, 102, 245, 40),
            accent_select_light: Color32::from_rgba_premultiplied(30, 102, 245, 120),
            text_hover_bg: Color32::from_rgba_premultiplied(76, 79, 105, 18),

            shadow_light: Color32::from_rgba_premultiplied(0, 0, 0, 20),
            shadow_medium: Color32::from_rgba_premultiplied(0, 0, 0, 30),

            toast_success: Color32::from_rgb(64, 160, 43),

            fk_color: Color32::from_rgb(223, 142, 29),

            minimap_bg: Color32::from_rgba_premultiplied(230, 230, 235, 200),
            minimap_border: Color32::from_rgba_premultiplied(170, 170, 180, 180),
            minimap_node: Color32::from_rgba_premultiplied(30, 102, 245, 220),
            minimap_vp_fill: Color32::from_rgba_premultiplied(30, 102, 245, 30),
            minimap_vp_stroke: Color32::from_rgba_premultiplied(30, 102, 245, 150),

            edge_label_bg: Color32::from_rgba_premultiplied(240, 240, 245, 220),

            divider_color: Color32::from_rgba_premultiplied(172, 176, 190, 80),
            row_divider: Color32::from_rgba_premultiplied(160, 160, 170, 60),

            preview_fill: Color32::from_rgba_premultiplied(30, 102, 245, 80),

            tooltip_bg: Color32::from_rgba_premultiplied(245, 245, 250, 235),
            tooltip_border: Color32::from_rgba_premultiplied(172, 176, 190, 200),

            ghost_stroke: Color32::from_rgba_premultiplied(30, 102, 245, 60),
            dim_overlay: Color32::from_rgba_premultiplied(220, 220, 230, 140),
            dim_overlay_heavy: Color32::from_rgba_premultiplied(210, 210, 220, 160),
            focus_dim_near: Color32::from_rgba_premultiplied(200, 200, 210, 70),
            focus_dim_far: Color32::from_rgba_premultiplied(200, 200, 210, 160),

            crust: Color32::from_rgb(220, 224, 232),
            surface2: Color32::from_rgb(172, 176, 190),
            lavender: Color32::from_rgb(114, 135, 253),
        }
    }
}

// Dimensions (mode-independent)
pub const PORT_RADIUS: f32 = 4.5;
pub const PORT_HIT_RADIUS: f32 = 12.0;
pub const TOOLBAR_WIDTH: f32 = 220.0;
pub const PROPERTIES_WIDTH: f32 = 280.0;

/// Convert [u8; 4] RGBA to egui Color32.
pub fn to_color32(rgba: [u8; 4]) -> Color32 {
    Color32::from_rgba_premultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

// ---------------------------------------------------------------------------
// Shared color palettes — used by context menus, toolbar, and properties panel
// ---------------------------------------------------------------------------

/// A named color swatch: (RGBA bytes, display name).
pub type ColorSwatch = ([u8; 4], &'static str);

/// Full node fill color palette (10 colors, used in node context menus).
pub const NODE_COLORS: &[ColorSwatch] = &[
    ([30, 30, 46, 255],    "Surface"),
    ([137, 180, 250, 255], "Blue"),
    ([166, 227, 161, 255], "Green"),
    ([243, 139, 168, 255], "Red"),
    ([249, 226, 175, 255], "Yellow"),
    ([203, 166, 247, 255], "Purple"),
    ([245, 194, 231, 255], "Pink"),
    ([148, 226, 213, 255], "Teal"),
    ([255, 255, 255, 255], "White"),
    ([17, 17, 27, 255],    "Black"),
];

/// Compact bulk-color palette (6 colors, used in multi-select context menus).
pub const BULK_COLORS: &[ColorSwatch] = &[
    ([137, 180, 250, 255], "Blue"),
    ([166, 227, 161, 255], "Green"),
    ([243, 139, 168, 255], "Red"),
    ([249, 226, 175, 255], "Yellow"),
    ([203, 166, 247, 255], "Purple"),
    ([148, 226, 213, 255], "Teal"),
];

/// Edge color palette (6 colors).
pub const EDGE_COLORS: &[ColorSwatch] = &[
    ([100, 100, 100, 255], "Gray"),
    ([137, 180, 250, 255], "Blue"),
    ([166, 227, 161, 255], "Green"),
    ([243, 139, 168, 255], "Red"),
    ([249, 226, 175, 255], "Yellow"),
    ([203, 166, 247, 255], "Purple"),
];

/// Canvas background presets: (RGBA, display name).
pub const CANVAS_BG_PRESETS: &[ColorSwatch] = &[
    ([30, 30, 46, 255],    "Dark"),
    ([245, 244, 240, 255], "Light"),
    ([10, 20, 60, 255],    "Blueprint"),
    ([8, 8, 8, 255],       "Midnight"),
];

/// Pick a legible text color (light or dark) for the given background fill.
pub fn auto_contrast_text(fill: [u8; 4]) -> [u8; 4] {
    let [r, g, b, _] = fill;
    let luma = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
    if luma > 140.0 {
        [15, 15, 20, 255]
    } else {
        [220, 220, 230, 255]
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible constants — alias the dark theme so existing code
// that uses `CANVAS_BG`, `ACCENT`, etc. continues to compile unchanged.
// New code should prefer `self.theme.xxx` for dark/light awareness.
// ---------------------------------------------------------------------------

// Canvas
pub const CANVAS_BG: Color32 = Color32::from_rgb(30, 30, 46);
pub const GRID_COLOR: Color32 = Color32::from_rgba_premultiplied(69, 71, 90, 50);
pub const GRID_MAJOR_COLOR: Color32 = Color32::from_rgba_premultiplied(88, 91, 112, 90);
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

// Accent alpha variants
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
pub const MINIMAP_VP_FILL: Color32 = Color32::from_rgba_premultiplied(80, 160, 255, 30);
pub const MINIMAP_VP_STROKE: Color32 = Color32::from_rgba_premultiplied(80, 160, 255, 150);

// Edge label background
pub const EDGE_LABEL_BG: Color32 = Color32::from_rgba_premultiplied(30, 30, 30, 200);

// Section divider
pub const DIVIDER_COLOR: Color32 = Color32::from_rgba_premultiplied(69, 71, 90, 80);
pub const ROW_DIVIDER: Color32 = Color32::from_rgba_premultiplied(100, 100, 100, 60);

// Node preview
pub const PREVIEW_FILL: Color32 = Color32::from_rgba_premultiplied(100, 160, 255, 80);

// Tooltip
pub const TOOLTIP_BG: Color32 = Color32::from_rgba_premultiplied(18, 18, 30, 230);
pub const TOOLTIP_BORDER: Color32 = Color32::from_rgba_premultiplied(69, 71, 90, 200);

// Ghost / overlay
pub const GHOST_STROKE: Color32 = Color32::from_rgba_premultiplied(137, 180, 250, 60);
pub const DIM_OVERLAY: Color32 = Color32::from_rgba_premultiplied(16, 16, 28, 180);
pub const DIM_OVERLAY_HEAVY: Color32 = Color32::from_rgba_premultiplied(12, 12, 22, 175);
pub const FOCUS_DIM_NEAR: Color32 = Color32::from_rgba_premultiplied(16, 16, 28, 90);
pub const FOCUS_DIM_FAR: Color32 = Color32::from_rgba_premultiplied(16, 16, 28, 190);
