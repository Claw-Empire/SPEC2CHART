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
        // Catppuccin Mocha — deepened for stronger panel/canvas contrast
        Self {
            canvas_bg: Color32::from_rgb(26, 27, 42),          // deeper canvas
            grid_color: Color32::from_rgba_premultiplied(69, 71, 90, 55),
            grid_major_color: Color32::from_rgba_premultiplied(88, 91, 112, 100),
            selection_color: Color32::from_rgb(137, 180, 250),
            port_fill: Color32::from_rgb(55, 56, 76),
            box_select_fill: Color32::from_rgba_premultiplied(137, 180, 250, 22),
            box_select_stroke: Color32::from_rgba_premultiplied(137, 180, 250, 120),

            accent: Color32::from_rgb(137, 180, 250),
            text_primary: Color32::from_rgb(215, 222, 248),    // brighter — clearer reading
            text_secondary: Color32::from_rgb(172, 180, 208),
            text_dim: Color32::from_rgb(135, 140, 168),        // WCAG AA compliant (~5.1:1)
            surface0: Color32::from_rgb(45, 46, 64),           // panel items
            surface1: Color32::from_rgb(65, 67, 87),           // borders/dividers
            mantle: Color32::from_rgb(18, 18, 30),             // sidebar — clearly darker than canvas

            accent_glow: Color32::from_rgba_premultiplied(137, 180, 250, 35),
            accent_hover: Color32::from_rgba_premultiplied(137, 180, 250, 85),
            accent_faint: Color32::from_rgba_premultiplied(137, 180, 250, 18),
            accent_select_bg: Color32::from_rgba_premultiplied(137, 180, 250, 45),
            accent_select_light: Color32::from_rgba_premultiplied(137, 180, 250, 110),
            text_hover_bg: Color32::from_rgba_premultiplied(215, 222, 248, 20),

            shadow_light: Color32::from_rgba_premultiplied(0, 0, 0, 50),
            shadow_medium: Color32::from_rgba_premultiplied(0, 0, 0, 70),

            toast_success: Color32::from_rgb(166, 227, 161),

            fk_color: Color32::from_rgb(249, 226, 175),

            minimap_bg: Color32::from_rgba_premultiplied(14, 14, 24, 220),
            minimap_border: Color32::from_rgba_premultiplied(80, 82, 104, 200),
            minimap_node: Color32::from_rgba_premultiplied(100, 170, 255, 230),
            minimap_vp_fill: Color32::from_rgba_premultiplied(100, 170, 255, 35),
            minimap_vp_stroke: Color32::from_rgba_premultiplied(100, 170, 255, 160),

            edge_label_bg: Color32::from_rgba_premultiplied(18, 18, 30, 220),

            divider_color: Color32::from_rgba_premultiplied(65, 67, 87, 90),
            row_divider: Color32::from_rgba_premultiplied(90, 92, 115, 65),

            preview_fill: Color32::from_rgba_premultiplied(100, 160, 255, 90),

            tooltip_bg: Color32::from_rgba_premultiplied(14, 14, 26, 240),
            tooltip_border: Color32::from_rgba_premultiplied(65, 67, 87, 220),

            ghost_stroke: Color32::from_rgba_premultiplied(137, 180, 250, 70),
            dim_overlay: Color32::from_rgba_premultiplied(12, 12, 22, 185),
            dim_overlay_heavy: Color32::from_rgba_premultiplied(8, 8, 18, 180),
            focus_dim_near: Color32::from_rgba_premultiplied(12, 12, 22, 95),
            focus_dim_far: Color32::from_rgba_premultiplied(12, 12, 22, 195),

            crust: Color32::from_rgb(12, 12, 22),
            surface2: Color32::from_rgb(88, 91, 112),
            lavender: Color32::from_rgb(180, 190, 254),
        }
    }

    pub fn light() -> Self {
        // Catppuccin Latte — cleaner canvas, stronger panel separation
        Self {
            canvas_bg: Color32::from_rgb(253, 253, 255),        // near-white clean canvas
            grid_color: Color32::from_rgba_premultiplied(180, 184, 200, 55),
            grid_major_color: Color32::from_rgba_premultiplied(150, 154, 175, 110),
            selection_color: Color32::from_rgb(22, 96, 232),
            port_fill: Color32::from_rgb(195, 200, 215),
            box_select_fill: Color32::from_rgba_premultiplied(22, 96, 232, 22),
            box_select_stroke: Color32::from_rgba_premultiplied(22, 96, 232, 130),

            accent: Color32::from_rgb(22, 96, 232),             // crisp blue accent
            text_primary: Color32::from_rgb(40, 44, 68),        // deep, high-contrast text
            text_secondary: Color32::from_rgb(76, 80, 108),
            text_dim: Color32::from_rgb(105, 110, 135),        // WCAG AA compliant (~5.3:1)
            surface0: Color32::from_rgb(218, 222, 234),         // panel items (clearly distinct from canvas)
            surface1: Color32::from_rgb(196, 200, 218),         // borders — visible against white canvas
            mantle: Color32::from_rgb(210, 215, 228),           // sidebar — noticeably different from canvas

            accent_glow: Color32::from_rgba_premultiplied(22, 96, 232, 30),
            accent_hover: Color32::from_rgba_premultiplied(22, 96, 232, 75),
            accent_faint: Color32::from_rgba_premultiplied(22, 96, 232, 14),
            accent_select_bg: Color32::from_rgba_premultiplied(22, 96, 232, 38),
            accent_select_light: Color32::from_rgba_premultiplied(22, 96, 232, 115),
            text_hover_bg: Color32::from_rgba_premultiplied(40, 44, 68, 14),

            shadow_light: Color32::from_rgba_premultiplied(0, 0, 0, 18),
            shadow_medium: Color32::from_rgba_premultiplied(0, 0, 0, 28),

            toast_success: Color32::from_rgb(52, 148, 36),

            fk_color: Color32::from_rgb(200, 118, 18),

            minimap_bg: Color32::from_rgba_premultiplied(218, 220, 232, 215),
            minimap_border: Color32::from_rgba_premultiplied(155, 160, 185, 190),
            minimap_node: Color32::from_rgba_premultiplied(22, 96, 232, 220),
            minimap_vp_fill: Color32::from_rgba_premultiplied(22, 96, 232, 28),
            minimap_vp_stroke: Color32::from_rgba_premultiplied(22, 96, 232, 155),

            edge_label_bg: Color32::from_rgba_premultiplied(252, 252, 255, 240),

            divider_color: Color32::from_rgba_premultiplied(170, 175, 195, 85),
            row_divider: Color32::from_rgba_premultiplied(155, 160, 182, 65),

            preview_fill: Color32::from_rgba_premultiplied(22, 96, 232, 75),

            tooltip_bg: Color32::from_rgba_premultiplied(248, 249, 254, 245),
            tooltip_border: Color32::from_rgba_premultiplied(165, 170, 192, 210),

            ghost_stroke: Color32::from_rgba_premultiplied(22, 96, 232, 65),
            dim_overlay: Color32::from_rgba_premultiplied(210, 213, 228, 135),
            dim_overlay_heavy: Color32::from_rgba_premultiplied(200, 204, 220, 155),
            focus_dim_near: Color32::from_rgba_premultiplied(195, 198, 215, 65),
            focus_dim_far: Color32::from_rgba_premultiplied(195, 198, 215, 155),

            crust: Color32::from_rgb(200, 205, 220),
            surface2: Color32::from_rgb(165, 170, 190),
            lavender: Color32::from_rgb(100, 124, 242),
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

