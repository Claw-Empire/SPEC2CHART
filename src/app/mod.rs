mod theme;
mod shortcuts;
mod toolbar;
mod properties;
mod canvas;
mod render;
mod render3d;
mod statusbar;
mod command_palette;
pub(crate) mod camera;
pub(crate) mod interaction;

pub(crate) use theme::*;

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
    pub(crate) focus_mode: bool,
    pub(crate) canvas_locked: bool,
    /// Alignment guide lines computed during node drag: (is_horizontal, canvas_coord)
    pub(crate) alignment_guides: Vec<(bool, f32)>,
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
    /// Command palette open/closed
    pub(crate) show_command_palette: bool,
    /// Command palette search text
    pub(crate) command_palette_query: String,
    /// Command palette selected row
    pub(crate) command_palette_cursor: usize,
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
        let mut visuals = egui::Visuals::dark();

        // Colors only used in theme setup (not global constants)
        let crust = Color32::from_rgb(17, 17, 27);
        let surface2 = Color32::from_rgb(88, 91, 112);
        let lavender = Color32::from_rgb(180, 190, 254);

        visuals.panel_fill = MANTLE;
        visuals.window_fill = CANVAS_BG;
        visuals.extreme_bg_color = crust;
        visuals.faint_bg_color = SURFACE0;

        visuals.widgets.noninteractive.bg_fill = SURFACE0;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, SURFACE1);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

        visuals.widgets.inactive.bg_fill = SURFACE0;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
        visuals.widgets.inactive.bg_stroke = Stroke::new(0.5, SURFACE1);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

        visuals.widgets.hovered.bg_fill = SURFACE1;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

        visuals.widgets.active.bg_fill = surface2;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, lavender);
        visuals.widgets.active.corner_radius = CornerRadius::same(6);

        visuals.widgets.open.bg_fill = SURFACE1;
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT);
        visuals.widgets.open.corner_radius = CornerRadius::same(6);

        visuals.selection.bg_fill = ACCENT_SELECT_BG;
        visuals.selection.stroke = Stroke::new(1.0, ACCENT);

        visuals.window_corner_radius = CornerRadius::same(8);
        visuals.window_shadow = egui::Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_rgba_premultiplied(0, 0, 0, 60),
        };
        visuals.window_stroke = Stroke::new(1.0, SURFACE1);

        visuals.override_text_color = Some(TEXT_PRIMARY);

        cc.egui_ctx.set_visuals(visuals);

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
            focus_mode: false,
            canvas_locked: false,
            alignment_guides: Vec::new(),
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
            show_command_palette: false,
            command_palette_query: String::new(),
            command_palette_cursor: 0,
        }
    }

    pub(crate) fn draw_section_header(ui: &mut egui::Ui, label: &str) {
        ui.label(egui::RichText::new(label).size(11.0).color(TEXT_SECONDARY).strong());
    }

    pub(crate) fn draw_divider(ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let y = rect.min.y;
        ui.painter().line_segment(
            [
                Pos2::new(rect.min.x, y),
                Pos2::new(rect.max.x, y),
            ],
            Stroke::new(0.5, DIVIDER_COLOR),
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
        if !self.presentation_mode {
            self.draw_toolbar(ctx);
            // Properties panel works in both 2D and 3D (selection is shared)
            self.draw_properties_panel(ctx);
        }

        CentralPanel::default()
            .frame(egui::Frame::NONE.fill(CANVAS_BG))
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

        // Find & Replace dialog (Cmd+H)
        if self.show_find_replace {
            let mut open = self.show_find_replace;
            let _do_replace = false;
            let mut do_replace_all = false;
            egui::Window::new("Find & Replace")
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Find:");
                        ui.text_edit_singleline(&mut self.find_query);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Replace:");
                        ui.text_edit_singleline(&mut self.replace_query);
                    });
                    ui.add_space(4.0);
                    // Match count
                    let count = self.document.nodes.iter().filter(|n| {
                        !self.find_query.is_empty()
                            && n.display_label().to_lowercase().contains(&self.find_query.to_lowercase())
                    }).count();
                    if !self.find_query.is_empty() {
                        ui.label(egui::RichText::new(format!("{count} match(es)")).size(10.5).color(TEXT_DIM));
                    }
                    ui.add_space(4.0);
                    if ui.button("Replace All").clicked() { do_replace_all = true; }
                });
            if do_replace_all && !self.find_query.is_empty() {
                let find = self.find_query.to_lowercase();
                let replace = self.replace_query.clone();
                let mut changed = 0usize;
                for node in self.document.nodes.iter_mut() {
                    match &mut node.kind {
                        NodeKind::Shape { label, .. } => {
                            if label.to_lowercase().contains(&find) {
                                *label = label.to_lowercase().replace(&find, &replace);
                                changed += 1;
                            }
                        }
                        NodeKind::StickyNote { text, .. } => {
                            if text.to_lowercase().contains(&find) {
                                *text = text.to_lowercase().replace(&find, &replace);
                                changed += 1;
                            }
                        }
                        NodeKind::Entity { name, .. } => {
                            if name.to_lowercase().contains(&find) {
                                *name = name.to_lowercase().replace(&find, &replace);
                                changed += 1;
                            }
                        }
                        NodeKind::Text { content } => {
                            if content.to_lowercase().contains(&find) {
                                *content = content.to_lowercase().replace(&find, &replace);
                                changed += 1;
                            }
                        }
                    }
                }
                if changed > 0 {
                    self.history.push(&self.document);
                    self.status_message = Some((format!("Replaced {changed} node(s)"), std::time::Instant::now()));
                }
            }
            self.show_find_replace = open;
        }

        // Shape picker floating palette (N key)
        if let Some(picker_pos) = self.shape_picker {
            let shapes: &[(&str, NodeKind)] = &[
                ("■ Rect",   NodeKind::Shape { shape: crate::model::NodeShape::Rectangle, label: String::new(), description: String::new() }),
                ("⬮ Round",  NodeKind::Shape { shape: crate::model::NodeShape::RoundedRect, label: String::new(), description: String::new() }),
                ("◆ Diamond",NodeKind::Shape { shape: crate::model::NodeShape::Diamond, label: String::new(), description: String::new() }),
                ("● Circle", NodeKind::Shape { shape: crate::model::NodeShape::Circle, label: String::new(), description: String::new() }),
                ("▱ Parallel",NodeKind::Shape { shape: crate::model::NodeShape::Parallelogram, label: String::new(), description: String::new() }),
                ("📝 Sticky", NodeKind::StickyNote { text: String::new(), color: crate::model::StickyColor::Yellow }),
                ("T Text",   NodeKind::Text { content: String::new() }),
            ];
            let canvas_pos = self.viewport.screen_to_canvas(picker_pos);
            let mut chosen: Option<NodeKind> = None;
            let mut close = false;
            egui::Window::new("##shape_picker")
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .fixed_pos(picker_pos)
                .frame(egui::Frame {
                    fill: SURFACE0,
                    inner_margin: egui::Margin::same(8),
                    stroke: egui::Stroke::new(1.0, SURFACE1),
                    corner_radius: egui::CornerRadius::same(8),
                    ..Default::default()
                })
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("Insert node").size(10.0).color(TEXT_DIM));
                    ui.add_space(4.0);
                    for (label, kind) in shapes {
                        if ui.add(
                            egui::Button::new(egui::RichText::new(*label).size(12.0))
                                .min_size(egui::vec2(110.0, 22.0))
                        ).clicked() {
                            chosen = Some(kind.clone());
                            close = true;
                        }
                    }
                    if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) { close = true; }
                    if ui.ctx().pointer_latest_pos().map_or(false, |_p| {
                        !ui.ctx().is_pointer_over_area()
                    }) { close = true; }
                });
            if let Some(kind) = chosen {
                let w = 120.0_f32;
                let h = 70.0_f32;
                let pos = egui::Pos2::new(canvas_pos.x - w / 2.0, canvas_pos.y - h / 2.0);
                let node = crate::model::Node {
                    id: NodeId::new(),
                    kind,
                    position: [pos.x, pos.y],
                    size: [w, h],
                    z_offset: 0.0,
                    style: crate::model::NodeStyle::default(),
                    pinned: false,
                    tag: None,
                    collapsed: false,
                    uncollapsed_size: None,
                    url: String::new(),
                    locked: false,
                    comment: String::new(),
                    is_frame: false,
                    frame_color: crate::model::default_frame_color(),
                };
                let id = node.id;
                self.document.nodes.push(node);
                self.selection.select_node(id);
                self.focus_label_edit = true;
                self.history.push(&self.document);
                self.status_message = Some(("Node inserted".to_string(), std::time::Instant::now()));
            }
            if close { self.shape_picker = None; }
        }

        // Inline edge label editor (opens when double-clicking an edge)
        if let Some((edge_id, pos)) = self.inline_edge_edit {
            let mut close_editor = false;
            egui::Window::new("##edge_label_editor")
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .fixed_pos(pos)
                .frame(egui::Frame {
                    fill: SURFACE0,
                    inner_margin: egui::Margin::same(6),
                    stroke: egui::Stroke::new(1.0, ACCENT),
                    corner_radius: egui::CornerRadius::same(6),
                    ..Default::default()
                })
                .show(ctx, |ui| {
                    // Title row with char count
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Edge label").size(10.0).color(TEXT_DIM));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let char_count = self.document.find_edge(&edge_id)
                                .map(|e| e.label.chars().count())
                                .unwrap_or(0);
                            let count_color = if char_count > 45 { Color32::from_rgb(243, 139, 168) } else { TEXT_DIM };
                            ui.label(egui::RichText::new(format!("{}/50", char_count)).size(9.5).color(count_color));
                        });
                    });
                    if let Some(edge) = self.document.find_edge_mut(&edge_id) {
                        // Cap at 50 chars
                        if edge.label.chars().count() > 50 {
                            let trimmed: String = edge.label.chars().take(50).collect();
                            edge.label = trimmed;
                        }
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut edge.label)
                                .desired_width(180.0)
                                .hint_text("e.g. depends on, owns, sends to…")
                                .font(egui::FontId::proportional(13.0)),
                        );
                        resp.request_focus();
                        if ui.ctx().input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)) {
                            close_editor = true;
                            self.history.push(&self.document);
                        }
                    } else {
                        close_editor = true;
                    }
                    // Hint row
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Enter").monospace().size(9.5).color(ACCENT.gamma_multiply(0.7)));
                        ui.label(egui::RichText::new("save").size(9.5).color(TEXT_DIM));
                        ui.label(egui::RichText::new("·").size(9.5).color(TEXT_DIM));
                        ui.label(egui::RichText::new("Esc").monospace().size(9.5).color(ACCENT.gamma_multiply(0.7)));
                        ui.label(egui::RichText::new("cancel").size(9.5).color(TEXT_DIM));
                    });
                });
            if close_editor { self.inline_edge_edit = None; }
        }

        // Comment editor (Cmd+M to open for selected node)
        if let Some(node_id) = self.comment_editing {
            let mut close_comment = false;
            // Position near top-right of node
            let node_screen_pos = self.document.find_node(&node_id)
                .map(|n| {
                    let p = self.viewport.canvas_to_screen(n.pos());
                    let s = n.size_vec() * self.viewport.zoom;
                    egui::Pos2::new(p.x + s.x + 8.0, p.y)
                })
                .unwrap_or(egui::Pos2::new(200.0, 200.0));
            egui::Window::new("##comment_editor")
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .fixed_pos(node_screen_pos)
                .frame(egui::Frame {
                    fill: egui::Color32::from_rgba_unmultiplied(249, 226, 175, 240), // yellow note color
                    inner_margin: egui::Margin::same(8),
                    stroke: egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(200, 175, 100, 255)),
                    corner_radius: egui::CornerRadius::same(8),
                    ..Default::default()
                })
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("💬 Comment").size(10.0)
                        .color(egui::Color32::from_rgba_unmultiplied(80, 60, 20, 255)));
                    if let Some(node) = self.document.find_node_mut(&node_id) {
                        let resp = ui.add(
                            egui::TextEdit::multiline(&mut node.comment)
                                .desired_width(200.0)
                                .desired_rows(3)
                                .font(egui::FontId::proportional(12.0))
                                .text_color(egui::Color32::from_rgba_unmultiplied(60, 40, 10, 255)),
                        );
                        resp.request_focus();
                        if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                            close_comment = true;
                            self.history.push(&self.document);
                        }
                    } else {
                        close_comment = true;
                    }
                    ui.horizontal(|ui| {
                        if ui.small_button("✓ Done").clicked() {
                            close_comment = true;
                            self.history.push(&self.document);
                        }
                        if ui.small_button("🗑 Clear").clicked() {
                            if let Some(node) = self.document.find_node_mut(&node_id) {
                                node.comment.clear();
                            }
                            close_comment = true;
                            self.history.push(&self.document);
                        }
                    });
                });
            if close_comment { self.comment_editing = None; }
        }

        // Keyboard shortcuts panel
        if self.show_shortcuts_panel {
            let mut open = self.show_shortcuts_panel;
            egui::Window::new("Keyboard Shortcuts")
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .min_width(460.0)
                .show(ctx, |ui| {
                    type Section = (&'static str, &'static [(&'static str, &'static str)]);
                    let sections: &[Section] = &[
                        ("Tools", &[
                            ("V", "Select tool"),
                            ("E", "Connect / edge tool"),
                            ("N", "Insert shape picker"),
                            ("R / C / D", "Quick-create Rect / Circle / Diamond"),
                            ("Double-click canvas", "Create node"),
                            ("Double-click node", "Edit label"),
                            ("Right-click", "Context menu"),
                        ]),
                        ("Selection", &[
                            ("⌘A", "Select all"),
                            ("Escape", "Deselect"),
                            ("Del / Backspace", "Delete selected"),
                            ("Arrow keys", "Nudge 1 px  (⇧ = 10 px)"),
                            ("⇧H / ⇧V", "Distribute selected horizontally / vertically"),
                            ("⌘G", "Group into frame"),
                        ]),
                        ("Edit", &[
                            ("⌘Z", "Undo"),
                            ("⌘⇧Z", "Redo"),
                            ("⌘C / ⌘V", "Copy / Paste (nodes + edges)"),
                            ("⌘D", "Duplicate"),
                            ("⌘⇧H", "Collapse / expand selected nodes"),
                            ("⌘L", "Auto-layout (hierarchical)"),
                        ]),
                        ("View", &[
                            ("⌘1", "Fit all to view"),
                            ("⌘2", "Zoom to selection"),
                            ("⌘= / ⌘-", "Zoom in / out"),
                            ("⌘0", "Reset zoom to 100%"),
                            ("F", "Focus mode — dim unconnected nodes"),
                            ("G", "Toggle grid"),
                            ("S", "Toggle snap · S with edge selected = cycle edge style"),
                            ("O", "Bird's-eye overview"),
                            ("Alt+hover", "Show distance rulers"),
                        ]),
                        ("Search & Navigate", &[
                            ("⌘F", "Search nodes (spotlight)"),
                            ("↑ / ↓", "Navigate search results"),
                            ("Enter", "Jump to search result"),
                            ("⌘⇧1–5", "Save viewport bookmark"),
                            ("⇧1–5", "Jump to bookmark"),
                        ]),
                        ("Help", &[
                            ("F1 / ?", "This shortcuts panel"),
                            ("⌘K", "Command palette"),
                            ("[", "Collapse / expand left toolbar"),
                            ("]", "Collapse / expand right panel"),
                            ("⇧R", "Toggle coordinate rulers"),
                        ]),
                    ];
                    egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                        for (section, items) in sections {
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new(*section)
                                .size(10.0)
                                .color(TEXT_DIM)
                                .strong());
                            egui::Grid::new(format!("sc_{}", section))
                                .striped(true)
                                .num_columns(2)
                                .spacing([16.0, 3.0])
                                .show(ui, |ui| {
                                    for (key, desc) in *items {
                                        ui.label(egui::RichText::new(*key)
                                            .monospace()
                                            .color(ACCENT)
                                            .size(11.5));
                                        ui.label(egui::RichText::new(*desc)
                                            .size(11.5)
                                            .color(TEXT_SECONDARY));
                                        ui.end_row();
                                    }
                                });
                        }
                    });
                });
            self.show_shortcuts_panel = open;
        }
    }
}
