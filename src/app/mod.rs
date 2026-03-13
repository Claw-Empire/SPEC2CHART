mod theme;
mod shortcuts;
mod toolbar;
mod properties;
mod canvas;
mod render;
mod render3d;
mod statusbar;
mod command_palette;
mod context_menu;
mod overlays;
pub(crate) mod export_mermaid;
pub(crate) mod camera;
pub(crate) mod interaction;

use egui::{CentralPanel, Color32, CornerRadius, Pos2, Rect, Stroke, Vec2};
use crate::history::UndoStack;
use crate::model::*;
use crate::specgraph;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Connect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    TwoD,
    ThreeD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramMode {
    Flowchart,
    ER,
    FigJam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

#[derive(Debug, Clone)]
pub enum DragState {
    None,
    Panning {
        start_offset: [f32; 2],
        start_mouse: Pos2,
    },
    DraggingNode {
        start_positions: Vec<(NodeId, Pos2)>,
        start_z_offsets: Vec<(NodeId, f32)>,
        start_mouse: Pos2,
    },
    BoxSelect {
        start_canvas: Pos2,
    },
    CreatingEdge {
        source: Port,
        current_screen: Pos2,
    },
    DraggingNewNode {
        kind: NodeKind,
        current_screen: Pos2,
    },
    ResizingNode {
        node_id: NodeId,
        handle: ResizeHandle,
        start_rect: [f32; 4],
        start_mouse: Pos2,
    },
    DraggingEdgeBend {
        edge_id: EdgeId,
        start_bend: f32,
        start_mouse: Pos2,
    },
}

// ---------------------------------------------------------------------------
// FlowchartApp
// ---------------------------------------------------------------------------

pub struct FlowchartApp {
    pub(crate) document: FlowchartDocument,
    pub(crate) viewport: Viewport,
    pub(crate) selection: Selection,
    pub(crate) history: UndoStack,
    pub(crate) clipboard: Vec<Node>,
    /// Edges connecting copied nodes — pasted alongside the copied nodes
    pub(crate) edge_clipboard: Vec<Edge>,
    pub(crate) tool: Tool,
    pub(crate) drag: DragState,
    pub(crate) show_grid: bool,
    pub(crate) snap_to_grid: bool,
    pub(crate) grid_size: f32,
    pub(crate) diagram_mode: DiagramMode,
    pub(crate) selected_sticky_color: StickyColor,
    pub(crate) space_held: bool,
    pub(crate) canvas_rect: Rect,
    pub(crate) status_message: Option<(String, std::time::Instant)>,
    pub(crate) focus_label_edit: bool,
    /// When Some, show a floating inline edge label editor at this screen position
    pub(crate) inline_edge_edit: Option<(EdgeId, Pos2)>,
    /// When Some, show a floating comment editor for this node
    pub(crate) comment_editing: Option<NodeId>,
    pub(crate) view_mode: ViewMode,
    pub(crate) camera3d: camera::Camera3D,
    pub(crate) view_transition: f32,
    pub(crate) view_transition_target: f32,
    pub(crate) pending_fit: bool,
    pub(crate) llm_config: specgraph::LlmConfig,
    pub(crate) show_llm_settings: bool,
    pub(crate) show_spec_cheatsheet: bool,
    pub(crate) style_clipboard: Option<crate::model::NodeStyle>,
    /// Recently used fill colors (most recent first, max 10)
    pub(crate) recent_colors: Vec<[u8; 4]>,
    pub(crate) show_search: bool,
    pub(crate) search_query: String,
    pub(crate) show_shortcuts_panel: bool,
    pub(crate) bg_pattern: BgPattern,
    /// When Some, show a floating shape picker at this screen position
    pub(crate) shape_picker: Option<Pos2>,
    /// Saved viewport for overview mode toggle
    pub(crate) saved_viewport: Option<Viewport>,
    pub(crate) show_find_replace: bool,
    pub(crate) find_query: String,
    pub(crate) replace_query: String,
    /// "Go to XY" overlay — type canvas coordinates to pan there
    pub(crate) show_goto: bool,
    pub(crate) goto_query: String,
    pub(crate) focus_mode: bool,
    pub(crate) canvas_locked: bool,
    /// Alignment guide lines computed during node drag: (is_horizontal, canvas_coord)
    pub(crate) alignment_guides: Vec<(bool, f32)>,
    /// User-placed ruler guides: (is_vertical, canvas_coordinate). Cleared with Cmd+Shift+G.
    #[allow(dead_code)]
    pub(crate) ruler_guides: Vec<(bool, f32)>,
    /// When true, hide all panels for a clean presentation view (toggle with F)
    pub(crate) presentation_mode: bool,
    /// Custom canvas background color (overrides default CANVAS_BG)
    pub(crate) canvas_bg: [u8; 4],
    /// Optional project title shown as a watermark in the canvas top-left
    pub(crate) project_title: String,
    /// When true, overlay connectivity heatmap on nodes (toggle with H)
    pub(crate) show_heatmap: bool,
    /// Accumulated pan velocity for inertial scroll (pixels/frame)
    pub(crate) pan_velocity: [f32; 2],
    /// Target zoom for smooth keyboard zoom interpolation
    pub(crate) zoom_target: f32,
    /// Target pan offset for smooth fly-to animation (None = no animation)
    pub(crate) pan_target: Option<[f32; 2]>,
    /// Floating quick-notes panel visible/hidden (Shift+P)
    pub(crate) show_quick_notes: bool,
    /// Contents of the floating quick-notes panel
    pub(crate) quick_notes_text: String,
    /// How many times paste has been invoked since last copy (for progressive offset)
    pub(crate) paste_count: usize,
    /// When true, animate data-flow dots along edges (toggle with Shift+A)
    pub(crate) show_flow_animation: bool,
    /// Target positions for animated layout transitions: node_id → [target_x, target_y]
    pub(crate) layout_targets: std::collections::HashMap<NodeId, [f32; 2]>,
    /// Node IDs seen on the previous frame — used to detect newly created nodes
    pub(crate) prev_node_ids: std::collections::HashSet<NodeId>,
    /// Active creation ripples: (world_center, birth_time_secs)
    pub(crate) creation_ripples: Vec<([f32; 2], f64)>,
    /// Inline canvas label editor: (node_id, editing_text)
    pub(crate) inline_node_edit: Option<(NodeId, String)>,
    /// Tracks when a node hover started: (node_id, egui_time_secs)
    /// Used for progressive tooltip delay.
    pub(crate) hover_node_start: Option<(NodeId, f64)>,
    /// Records when each node was first selected (egui time in seconds).
    /// Used for selection-confirmation flash micro-animation.
    pub(crate) selection_times: std::collections::HashMap<NodeId, f64>,
    /// Currently highlighted result index in the search overlay
    pub(crate) search_cursor: usize,
    /// New-edge draw-in animations: (edge_id, birth_time_secs)
    pub(crate) edge_birth_times: std::collections::HashMap<EdgeId, f64>,
    /// Edge IDs seen on the previous frame — used to detect newly added edges
    pub(crate) prev_edge_ids: std::collections::HashSet<EdgeId>,
    /// Canvas location bookmarks: Cmd+Shift+1..5 saves, Shift+1..5 jumps
    pub(crate) bookmarks: [Option<crate::model::Viewport>; 5],
    /// Node freshness: (node_id → birth_time_secs) for "just created" ring animation
    pub(crate) node_birth_times: std::collections::HashMap<NodeId, f64>,
    /// Active tag filter: when set, non-matching nodes are dimmed
    pub(crate) tag_filter: Option<crate::model::NodeTag>,
    /// Deletion ghost animations: (canvas_center, canvas_size, fill_color, death_time)
    pub(crate) deletion_ghosts: Vec<([f32; 2], [f32; 2], [u8; 4], f64)>,
    /// Toolbar (left panel) collapse state
    pub(crate) toolbar_collapsed: bool,
    /// Properties panel (right panel) collapse state
    pub(crate) properties_collapsed: bool,
    /// Show canvas coordinate rulers (toggle with Shift+R)
    pub(crate) show_rulers: bool,
    /// Zoom level just before this frame (for change detection)
    pub(crate) last_zoom: f32,
    /// When zoom changed: birth time for fade-out indicator (egui time)
    pub(crate) zoom_indicator_time: Option<f64>,
    /// Command palette open/closed
    pub(crate) show_command_palette: bool,
    /// Command palette search text
    pub(crate) command_palette_query: String,
    /// Command palette selected row
    pub(crate) command_palette_cursor: usize,
    /// Current color theme (dark or light)
    pub(crate) theme: theme::Theme,
    /// Whether dark mode is active (true = dark, false = light)
    pub(crate) dark_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgPattern {
    Dots,
    Lines,
    Crosshatch,
    None,
}

impl FlowchartApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let t = theme::Theme::dark();
        Self::apply_visuals(&cc.egui_ctx, &t, true);

        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.window_margin = egui::Margin::same(16);
        cc.egui_ctx.set_style(style);

        let doc = FlowchartDocument::default();
        let mut history = UndoStack::new(100);
        history.push(&doc);
        Self {
            document: doc,
            viewport: Viewport::default(),
            selection: Selection::default(),
            history,
            clipboard: Vec::new(),
            edge_clipboard: Vec::new(),
            tool: Tool::Select,
            drag: DragState::None,
            show_grid: true,
            snap_to_grid: true,
            grid_size: 20.0,
            diagram_mode: DiagramMode::Flowchart,
            selected_sticky_color: StickyColor::Yellow,
            space_held: false,
            canvas_rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)),
            status_message: None,
            focus_label_edit: false,
            inline_edge_edit: None,
            comment_editing: None,
            view_mode: ViewMode::TwoD,
            camera3d: camera::Camera3D::default(),
            view_transition: 0.0,
            view_transition_target: 0.0,
            pending_fit: false,
            llm_config: specgraph::LlmConfig::default(),
            show_llm_settings: false,
            show_spec_cheatsheet: false,
            style_clipboard: None,
            recent_colors: Vec::new(),
            show_search: false,
            search_query: String::new(),
            show_shortcuts_panel: false,
            bg_pattern: BgPattern::Dots,
            shape_picker: None,
            saved_viewport: None,
            show_find_replace: false,
            find_query: String::new(),
            replace_query: String::new(),
            show_goto: false,
            goto_query: String::new(),
            focus_mode: false,
            canvas_locked: false,
            alignment_guides: Vec::new(),
            ruler_guides: Vec::new(),
            presentation_mode: false,
            canvas_bg: [30, 30, 46, 255], // default = CANVAS_BG
            project_title: String::new(),
            show_heatmap: false,
            pan_velocity: [0.0, 0.0],
            zoom_target: 1.0,
            pan_target: None,
            show_quick_notes: false,
            quick_notes_text: String::new(),
            paste_count: 0,
            show_flow_animation: false,
            layout_targets: std::collections::HashMap::new(),
            prev_node_ids: std::collections::HashSet::new(),
            creation_ripples: Vec::new(),
            inline_node_edit: None,
            hover_node_start: None,
            selection_times: std::collections::HashMap::new(),
            search_cursor: 0,
            edge_birth_times: std::collections::HashMap::new(),
            prev_edge_ids: std::collections::HashSet::new(),
            bookmarks: [None, None, None, None, None],
            node_birth_times: std::collections::HashMap::new(),
            tag_filter: None,
            deletion_ghosts: Vec::new(),
            toolbar_collapsed: false,
            properties_collapsed: false,
            show_rulers: false,
            last_zoom: 1.0,
            zoom_indicator_time: None,
            show_command_palette: false,
            command_palette_query: String::new(),
            command_palette_cursor: 0,
            theme: t,
            dark_mode: true,
        }
    }

    /// Toggle between dark and light mode, re-applying egui visuals.
    pub(crate) fn toggle_dark_mode(&mut self, ctx: &egui::Context) {
        self.dark_mode = !self.dark_mode;
        self.theme = if self.dark_mode {
            theme::Theme::dark()
        } else {
            theme::Theme::light()
        };
        Self::apply_visuals(ctx, &self.theme, self.dark_mode);
        // Update canvas background to match the theme
        let bg = self.theme.canvas_bg;
        self.canvas_bg = [bg.r(), bg.g(), bg.b(), bg.a()];
        let label = if self.dark_mode { "Dark mode" } else { "Light mode" };
        self.status_message = Some((label.to_string(), std::time::Instant::now()));
    }

    /// Apply theme colors to egui visuals.
    fn apply_visuals(ctx: &egui::Context, t: &theme::Theme, dark: bool) {
        let mut visuals = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };

        visuals.panel_fill = t.mantle;
        visuals.window_fill = t.canvas_bg;
        visuals.extreme_bg_color = t.crust;
        visuals.faint_bg_color = t.surface0;

        visuals.widgets.noninteractive.bg_fill = t.surface0;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text_secondary);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, t.surface1);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

        visuals.widgets.inactive.bg_fill = t.surface0;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text_primary);
        visuals.widgets.inactive.bg_stroke = Stroke::new(0.5, t.surface1);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

        visuals.widgets.hovered.bg_fill = t.surface1;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, t.text_primary);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, t.accent);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

        visuals.widgets.active.bg_fill = t.surface2;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, t.text_primary);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, t.lavender);
        visuals.widgets.active.corner_radius = CornerRadius::same(6);

        visuals.widgets.open.bg_fill = t.surface1;
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, t.text_primary);
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, t.accent);
        visuals.widgets.open.corner_radius = CornerRadius::same(6);

        visuals.selection.bg_fill = t.accent_select_bg;
        visuals.selection.stroke = Stroke::new(1.0, t.accent);

        visuals.window_corner_radius = CornerRadius::same(8);
        visuals.window_shadow = egui::Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_rgba_premultiplied(0, 0, 0, if dark { 60 } else { 25 }),
        };
        visuals.window_stroke = Stroke::new(1.0, t.surface1);
        visuals.override_text_color = Some(t.text_primary);

        ctx.set_visuals(visuals);
    }

    pub(crate) fn draw_section_header(&self, ui: &mut egui::Ui, label: &str) {
        ui.label(egui::RichText::new(label).size(11.0).color(self.theme.text_secondary).strong());
    }

    pub(crate) fn draw_divider(&self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let y = rect.min.y;
        ui.painter().line_segment(
            [
                Pos2::new(rect.min.x, y),
                Pos2::new(rect.max.x, y),
            ],
            Stroke::new(0.5, self.theme.divider_color),
        );
        ui.add_space(1.0);
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for FlowchartApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some((_, time)) = &self.status_message {
            if time.elapsed().as_secs_f32() > 2.5 {
                self.status_message = None;
            }
        }

        self.handle_shortcuts(ctx);

        // Animate view transition
        let transitioning = self.animate_view_transition();

        let has_active_toast = self
            .status_message
            .as_ref()
            .map_or(false, |(_, t)| t.elapsed().as_secs_f32() < 2.5);
        match self.drag {
            DragState::None if !has_active_toast && !transitioning => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            _ => {
                ctx.request_repaint();
            }
        }

        self.draw_status_bar(ctx);
        self.draw_command_palette(ctx);
        self.draw_goto_overlay(ctx);

        self.draw_zoom_indicator(ctx);

        if !self.presentation_mode {
            self.draw_toolbar(ctx);
            // Properties panel works in both 2D and 3D (selection is shared)
            self.draw_properties_panel(ctx);
        }

        CentralPanel::default()
            .frame(egui::Frame::NONE.fill(self.theme.canvas_bg))
            .show(ctx, |ui| {
                match self.view_mode {
                    ViewMode::TwoD => self.draw_canvas(ui),
                    ViewMode::ThreeD => self.draw_canvas_3d(ui),
                }
                // Presentation mode badge
                if self.presentation_mode {
                    let painter = ui.painter();
                    let screen_rect = ui.max_rect();
                    let label = "Presentation  [F to exit]";
                    let font = egui::FontId::proportional(11.0);
                    let text_color = Color32::from_rgba_premultiplied(200, 200, 220, 180);
                    let galley = ui.fonts(|f| f.layout_no_wrap(label.to_string(), font, text_color));
                    let pos = Pos2::new(
                        screen_rect.center().x - galley.size().x / 2.0,
                        screen_rect.max.y - galley.size().y - 12.0,
                    );
                    let bg_rect = Rect::from_min_size(pos - Vec2::new(8.0, 4.0), galley.size() + Vec2::new(16.0, 8.0));
                    painter.rect_filled(bg_rect, egui::CornerRadius::same(4), Color32::from_rgba_premultiplied(30, 30, 46, 200));
                    painter.galley(pos, galley, text_color);
                }
            });

        if self.pending_fit {
            self.pending_fit = false;
            self.fit_to_content();
            ctx.request_repaint();
        }

        // Dynamic window title: show node/edge count
        let n = self.document.nodes.len();
        let e = self.document.edges.len();
        let title = if n == 0 {
            "Light Figma".to_string()
        } else {
            format!("Light Figma — {n}N {e}E")
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        self.draw_find_replace(ctx);
        self.draw_shape_picker(ctx);
        self.draw_edge_label_editor(ctx);
        self.draw_comment_editor(ctx);
        self.draw_shortcuts_panel(ctx);
    }
}
