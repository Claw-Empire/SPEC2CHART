mod theme;
mod shortcuts;
mod toolbar;
mod properties;
mod canvas;
mod render;
mod render3d;
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
    pub(crate) view_mode: ViewMode,
    pub(crate) camera3d: camera::Camera3D,
    pub(crate) view_transition: f32,
    pub(crate) view_transition_target: f32,
    pub(crate) pending_fit: bool,
    pub(crate) llm_config: specgraph::LlmConfig,
    pub(crate) show_llm_settings: bool,
    pub(crate) style_clipboard: Option<crate::model::NodeStyle>,
    pub(crate) show_search: bool,
    pub(crate) search_query: String,
    pub(crate) show_shortcuts_panel: bool,
    pub(crate) bg_pattern: BgPattern,
    /// When Some, show a floating shape picker at this screen position
    pub(crate) shape_picker: Option<Pos2>,
    /// Saved viewport for overview mode toggle
    pub(crate) saved_viewport: Option<Viewport>,
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
            view_mode: ViewMode::TwoD,
            camera3d: camera::Camera3D::default(),
            view_transition: 0.0,
            view_transition_target: 0.0,
            pending_fit: false,
            llm_config: specgraph::LlmConfig::default(),
            show_llm_settings: false,
            style_clipboard: None,
            show_search: false,
            search_query: String::new(),
            show_shortcuts_panel: false,
            bg_pattern: BgPattern::Dots,
            shape_picker: None,
            saved_viewport: None,
        }
    }

    pub(crate) fn draw_section_header(ui: &mut egui::Ui, label: &str) {
        ui.label(egui::RichText::new(label).size(10.0).color(TEXT_DIM).strong());
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

        self.draw_toolbar(ctx);

        // Properties panel works in both 2D and 3D (selection is shared)
        self.draw_properties_panel(ctx);

        CentralPanel::default()
            .frame(egui::Frame::NONE.fill(CANVAS_BG))
            .show(ctx, |ui| {
                match self.view_mode {
                    ViewMode::TwoD => self.draw_canvas(ui),
                    ViewMode::ThreeD => self.draw_canvas_3d(ui),
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
                    if ui.ctx().pointer_latest_pos().map_or(false, |p| {
                        !ui.ctx().is_pointer_over_area()
                    }) { close = true; }
                });
            if let Some(kind) = chosen {
                let w = 120.0_f32;
                let h = 70.0_f32;
                let pos = egui::Pos2::new(canvas_pos.x - w / 2.0, canvas_pos.y - h / 2.0);
                let mut node = crate::model::Node {
                    id: NodeId::new(),
                    kind,
                    position: [pos.x, pos.y],
                    size: [w, h],
                    z_offset: 0.0,
                    style: crate::model::NodeStyle::default(),
                    pinned: false,
                    tag: None,
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

        // Keyboard shortcuts panel
        if self.show_shortcuts_panel {
            let mut open = self.show_shortcuts_panel;
            egui::Window::new("Keyboard Shortcuts")
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    let shortcuts: &[(&str, &str)] = &[
                        ("V", "Select tool"),
                        ("E", "Connect tool"),
                        ("G", "Toggle grid"),
                        ("S", "Toggle snap"),
                        ("F", "Fit / zoom to selection"),
                        ("⌘F", "Search nodes"),
                        ("⌘Z", "Undo"),
                        ("⌘⇧Z", "Redo"),
                        ("⌘C / ⌘V", "Copy / Paste"),
                        ("⌘D", "Duplicate"),
                        ("⌘A", "Select all"),
                        ("O", "Toggle bird's eye overview"),
                        ("N", "Insert shape picker"),
                        ("R / C / D", "Quick-create Rect / Circle / Diamond"),
                        ("⌘L", "Auto-layout (hierarchical)"),
                        ("⌘1", "Fit to content"),
                        ("⌘2", "Zoom to selection"),
                        ("⌘= / ⌘-", "Zoom in / out"),
                        ("⌘0", "Reset zoom"),
                        ("Arrow keys", "Nudge 1px (⇧ = 10px)"),
                        ("Del / Backspace", "Delete selected"),
                        ("Escape", "Deselect"),
                        ("Double-click", "Edit label / Create node"),
                        ("Right-click", "Context menu"),
                        ("F1 / ?", "This panel"),
                    ];
                    egui::Grid::new("shortcuts_grid")
                        .striped(true)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            for (key, desc) in shortcuts {
                                ui.label(egui::RichText::new(*key).monospace().color(ACCENT).size(12.0));
                                ui.label(egui::RichText::new(*desc).size(12.0).color(TEXT_SECONDARY));
                                ui.end_row();
                            }
                        });
                });
            self.show_shortcuts_panel = open;
        }
    }
}
