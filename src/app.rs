use egui::{
    Align2, CentralPanel, Color32, CornerRadius, FontId, Key, Modifiers, Pos2, Rect, Sense,
    SidePanel, Stroke, StrokeKind, Vec2,
};

use crate::export;
use crate::history::UndoStack;
use crate::io;
use crate::model::*;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Connect,
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
        shape: NodeShape,
        current_screen: Pos2,
    },
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// Catppuccin Mocha palette
const CANVAS_BG: Color32 = Color32::from_rgb(30, 30, 46);
const GRID_COLOR: Color32 = Color32::from_rgba_premultiplied(69, 71, 90, 50);
const SELECTION_COLOR: Color32 = Color32::from_rgb(137, 180, 250);
const PORT_FILL: Color32 = Color32::from_rgb(49, 50, 68);
const PORT_RADIUS: f32 = 4.5;
const PORT_HIT_RADIUS: f32 = 12.0;
const BOX_SELECT_FILL: Color32 = Color32::from_rgba_premultiplied(137, 180, 250, 20);
const BOX_SELECT_STROKE: Color32 = Color32::from_rgba_premultiplied(137, 180, 250, 100);
const TOOLBAR_WIDTH: f32 = 200.0;
const PROPERTIES_WIDTH: f32 = 260.0;

// Extra colors for UI
const ACCENT: Color32 = Color32::from_rgb(137, 180, 250);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(205, 214, 244);
const TEXT_SECONDARY: Color32 = Color32::from_rgb(166, 173, 200);
const TEXT_DIM: Color32 = Color32::from_rgb(108, 112, 134);
const SURFACE0: Color32 = Color32::from_rgb(49, 50, 68);
const SURFACE1: Color32 = Color32::from_rgb(69, 71, 90);
const MANTLE: Color32 = Color32::from_rgb(24, 24, 37);
#[allow(dead_code)]
const GREEN: Color32 = Color32::from_rgb(166, 227, 161);
#[allow(dead_code)]
const PEACH: Color32 = Color32::from_rgb(250, 179, 135);
#[allow(dead_code)]
const RED: Color32 = Color32::from_rgb(243, 139, 168);

// ---------------------------------------------------------------------------
// FlowchartApp
// ---------------------------------------------------------------------------

pub struct FlowchartApp {
    pub document: FlowchartDocument,
    pub viewport: Viewport,
    pub selection: Selection,
    pub history: UndoStack,
    pub clipboard: Vec<Node>,
    pub tool: Tool,
    pub drag: DragState,
    pub show_grid: bool,
    pub snap_to_grid: bool,
    pub grid_size: f32,
    /// Track whether space key is held (for pan mode)
    space_held: bool,
}

impl FlowchartApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set custom dark theme
        let mut visuals = egui::Visuals::dark();

        // Catppuccin Mocha-inspired palette
        let base = Color32::from_rgb(30, 30, 46);        // #1e1e2e
        let mantle = Color32::from_rgb(24, 24, 37);      // #181825
        let crust = Color32::from_rgb(17, 17, 27);       // #11111b
        let surface0 = Color32::from_rgb(49, 50, 68);    // #313244
        let surface1 = Color32::from_rgb(69, 71, 90);    // #45475a
        let surface2 = Color32::from_rgb(88, 91, 112);   // #585b70
        let text = Color32::from_rgb(205, 214, 244);      // #cdd6f4
        let subtext = Color32::from_rgb(166, 173, 200);   // #a6adc8
        let blue = Color32::from_rgb(137, 180, 250);      // #89b4fa
        let lavender = Color32::from_rgb(180, 190, 254);  // #b4befe

        visuals.panel_fill = mantle;
        visuals.window_fill = base;
        visuals.extreme_bg_color = crust;
        visuals.faint_bg_color = surface0;

        // Widget styling
        visuals.widgets.noninteractive.bg_fill = surface0;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, subtext);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, surface1);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

        visuals.widgets.inactive.bg_fill = surface0;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text);
        visuals.widgets.inactive.bg_stroke = Stroke::new(0.5, surface1);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

        visuals.widgets.hovered.bg_fill = surface1;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, text);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, blue);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

        visuals.widgets.active.bg_fill = surface2;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, text);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, lavender);
        visuals.widgets.active.corner_radius = CornerRadius::same(6);

        visuals.widgets.open.bg_fill = surface1;
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, text);
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, blue);
        visuals.widgets.open.corner_radius = CornerRadius::same(6);

        visuals.selection.bg_fill = Color32::from_rgba_premultiplied(137, 180, 250, 40);
        visuals.selection.stroke = Stroke::new(1.0, blue);

        visuals.window_corner_radius = CornerRadius::same(8);
        visuals.window_shadow = egui::Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_rgba_premultiplied(0, 0, 0, 60),
        };
        visuals.window_stroke = Stroke::new(1.0, surface1);

        visuals.override_text_color = Some(text);

        cc.egui_ctx.set_visuals(visuals);

        // Set style with more spacing
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.window_margin = egui::Margin::same(12);
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
            space_held: false,
        }
    }

    // -----------------------------------------------------------------------
    // UI Helpers
    // -----------------------------------------------------------------------

    fn draw_section_header(ui: &mut egui::Ui, label: &str) {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(label).size(10.0).color(TEXT_DIM).strong());
        ui.add_space(2.0);
    }

    fn draw_shape_button(&self, ui: &mut egui::Ui, shape: NodeShape, name: &str, width: f32, height: f32) -> egui::Response {
        let (response, painter) = ui.allocate_painter(egui::vec2(width, height), Sense::click_and_drag());
        let rect = response.rect;

        // Background
        let bg = if response.hovered() { SURFACE1 } else { SURFACE0 };
        painter.rect_filled(rect, CornerRadius::same(6), bg);
        if response.hovered() {
            painter.rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0, ACCENT), StrokeKind::Inside);
        }

        // Draw shape preview in the upper portion
        let preview_center = Pos2::new(rect.center().x, rect.min.y + height * 0.38);
        let pw = width * 0.35;
        let ph = height * 0.32;
        let shape_stroke = Stroke::new(1.5, ACCENT);
        let shape_fill = Color32::from_rgba_premultiplied(137, 180, 250, 15);

        match shape {
            NodeShape::Rectangle => {
                let r = Rect::from_center_size(preview_center, egui::vec2(pw, ph));
                painter.rect_filled(r, CornerRadius::ZERO, shape_fill);
                painter.rect_stroke(r, CornerRadius::ZERO, shape_stroke, StrokeKind::Outside);
            }
            NodeShape::RoundedRect => {
                let r = Rect::from_center_size(preview_center, egui::vec2(pw, ph));
                painter.rect_filled(r, CornerRadius::same(4), shape_fill);
                painter.rect_stroke(r, CornerRadius::same(4), shape_stroke, StrokeKind::Outside);
            }
            NodeShape::Diamond => {
                let c = preview_center;
                let hw = pw * 0.5;
                let hh = ph * 0.5;
                let pts = vec![
                    Pos2::new(c.x, c.y - hh),
                    Pos2::new(c.x + hw, c.y),
                    Pos2::new(c.x, c.y + hh),
                    Pos2::new(c.x - hw, c.y),
                ];
                painter.add(egui::Shape::convex_polygon(pts, shape_fill, shape_stroke));
            }
            NodeShape::Circle => {
                let r = pw.min(ph) * 0.5;
                painter.circle_filled(preview_center, r, shape_fill);
                painter.circle_stroke(preview_center, r, shape_stroke);
            }
            NodeShape::Parallelogram => {
                let skew = pw * 0.2;
                let half_w = pw * 0.5;
                let half_h = ph * 0.5;
                let pts = vec![
                    Pos2::new(preview_center.x - half_w + skew, preview_center.y - half_h),
                    Pos2::new(preview_center.x + half_w, preview_center.y - half_h),
                    Pos2::new(preview_center.x + half_w - skew, preview_center.y + half_h),
                    Pos2::new(preview_center.x - half_w, preview_center.y + half_h),
                ];
                painter.add(egui::Shape::convex_polygon(pts, shape_fill, shape_stroke));
            }
        }

        // Label below shape
        painter.text(
            Pos2::new(rect.center().x, rect.max.y - 10.0),
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(10.0),
            if response.hovered() { TEXT_PRIMARY } else { TEXT_SECONDARY },
        );

        response
    }

    // -----------------------------------------------------------------------
    // Keyboard Shortcuts
    // -----------------------------------------------------------------------

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Track space key for panning
        ctx.input(|i| {
            self.space_held = i.key_down(Key::Space);
        });

        let cmd = if cfg!(target_os = "macos") {
            Modifiers::MAC_CMD
        } else {
            Modifiers::CTRL
        };

        // Cmd+Z = undo
        if ctx.input(|i| i.key_pressed(Key::Z) && i.modifiers.matches_exact(cmd)) {
            if let Some(doc) = self.history.undo() {
                self.document = doc.clone();
                self.selection.clear();
            }
        }

        // Cmd+Shift+Z = redo
        let cmd_shift = Modifiers {
            shift: true,
            ..cmd
        };
        if ctx.input(|i| i.key_pressed(Key::Z) && i.modifiers.matches_exact(cmd_shift)) {
            if let Some(doc) = self.history.redo() {
                self.document = doc.clone();
                self.selection.clear();
            }
        }

        // Delete/Backspace = remove selected
        if ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
            && !self.selection.is_empty()
        {
            let node_ids: Vec<NodeId> = self.selection.node_ids.clone();
            let edge_ids: Vec<EdgeId> = self.selection.edge_ids.clone();
            for id in &node_ids {
                self.document.remove_node(id);
            }
            for id in &edge_ids {
                self.document.remove_edge(id);
            }
            self.selection.clear();
            self.history.push(&self.document);
        }

        // Cmd+C = copy selected nodes
        if ctx.input(|i| i.key_pressed(Key::C) && i.modifiers.matches_exact(cmd)) {
            self.clipboard.clear();
            for id in &self.selection.node_ids {
                if let Some(node) = self.document.find_node(id) {
                    self.clipboard.push(node.clone());
                }
            }
        }

        // Cmd+V = paste
        if ctx.input(|i| i.key_pressed(Key::V) && i.modifiers.matches_exact(cmd))
            && !self.clipboard.is_empty()
        {
            self.selection.clear();
            let offset = Vec2::new(30.0, 30.0);
            for template in self.clipboard.clone() {
                let mut node = template;
                node.id = NodeId::new();
                let pos = node.pos() + offset;
                node.set_pos(pos);
                self.selection.node_ids.push(node.id);
                self.document.nodes.push(node);
            }
            self.history.push(&self.document);
        }

        // Cmd+A = select all
        if ctx.input(|i| i.key_pressed(Key::A) && i.modifiers.matches_exact(cmd)) {
            self.selection.clear();
            for node in &self.document.nodes {
                self.selection.node_ids.push(node.id);
            }
            for edge in &self.document.edges {
                self.selection.edge_ids.push(edge.id);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Toolbar (left sidebar)
    // -----------------------------------------------------------------------

    fn draw_toolbar(&mut self, ctx: &egui::Context) {
        SidePanel::left("toolbar")
            .resizable(false)
            .exact_width(TOOLBAR_WIDTH)
            .frame(egui::Frame {
                fill: MANTLE,
                inner_margin: egui::Margin::same(12),
                stroke: Stroke::new(1.0, SURFACE1),
                ..Default::default()
            })
            .show(ctx, |ui| {
                // App title
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Light Figma").size(16.0).strong().color(TEXT_PRIMARY));
                });
                ui.add_space(8.0);

                // File actions as icon-like compact row
                Self::draw_section_header(ui, "FILE");
                ui.horizontal(|ui| {
                    let btn_size = egui::vec2(76.0, 28.0);
                    if ui.add_sized(btn_size, egui::Button::new(
                        egui::RichText::new("Save").size(12.0)
                    )).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Flowchart", &["flow"])
                            .set_file_name("untitled.flow")
                            .save_file()
                        {
                            if let Err(e) = io::save_document(&self.document, &path) {
                                eprintln!("Save error: {}", e);
                            }
                        }
                    }
                    if ui.add_sized(btn_size, egui::Button::new(
                        egui::RichText::new("Open").size(12.0)
                    )).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Flowchart", &["flow"])
                            .pick_file()
                        {
                            match io::load_document(&path) {
                                Ok(doc) => {
                                    self.document = doc;
                                    self.selection.clear();
                                    self.history.push(&self.document);
                                }
                                Err(e) => eprintln!("Load error: {}", e),
                            }
                        }
                    }
                });
                ui.add_space(4.0);

                // Export as compact row
                Self::draw_section_header(ui, "EXPORT");
                ui.horizontal_wrapped(|ui| {
                    let btn_size = egui::vec2(50.0, 26.0);
                    for (label, ext, export_fn) in [
                        ("PNG", "png", "png" as &str),
                        ("SVG", "svg", "svg"),
                        ("PDF", "pdf", "pdf"),
                    ] {
                        if ui.add_sized(btn_size, egui::Button::new(
                            egui::RichText::new(label).size(11.0)
                        )).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(label, &[ext])
                                .set_file_name(&format!("flowchart.{}", ext))
                                .save_file()
                            {
                                let result = match export_fn {
                                    "png" => export::export_png(&self.document, &path),
                                    "svg" => export::export_svg(&self.document, &path),
                                    "pdf" => export::export_pdf(&self.document, &path),
                                    _ => Ok(()),
                                };
                                if let Err(e) = result {
                                    eprintln!("Export error: {}", e);
                                }
                            }
                        }
                    }
                });
                ui.add_space(4.0);

                // Tools section
                Self::draw_section_header(ui, "TOOLS");
                ui.horizontal(|ui| {
                    let select_text = if self.tool == Tool::Select {
                        egui::RichText::new("Select").size(12.0).strong().color(ACCENT)
                    } else {
                        egui::RichText::new("Select").size(12.0).color(TEXT_SECONDARY)
                    };
                    let connect_text = if self.tool == Tool::Connect {
                        egui::RichText::new("Connect").size(12.0).strong().color(ACCENT)
                    } else {
                        egui::RichText::new("Connect").size(12.0).color(TEXT_SECONDARY)
                    };

                    let btn_size = egui::vec2(76.0, 28.0);
                    if ui.add_sized(btn_size, egui::Button::new(select_text)
                        .fill(if self.tool == Tool::Select { SURFACE1 } else { SURFACE0 })
                    ).clicked() {
                        self.tool = Tool::Select;
                    }
                    if ui.add_sized(btn_size, egui::Button::new(connect_text)
                        .fill(if self.tool == Tool::Connect { SURFACE1 } else { SURFACE0 })
                    ).clicked() {
                        self.tool = Tool::Connect;
                    }
                });
                ui.add_space(4.0);

                // Shapes section with visual preview buttons
                Self::draw_section_header(ui, "SHAPES");
                ui.add_space(2.0);

                let shapes = [
                    (NodeShape::Rectangle, "Rectangle"),
                    (NodeShape::RoundedRect, "Rounded"),
                    (NodeShape::Diamond, "Diamond"),
                    (NodeShape::Circle, "Circle"),
                    (NodeShape::Parallelogram, "Parallel"),
                ];

                // Draw shapes in a 2-column grid with visual previews
                let available_width = ui.available_width();
                let btn_width = (available_width - 8.0) / 2.0;
                let btn_height = 48.0;

                let mut i = 0;
                while i < shapes.len() {
                    ui.horizontal(|ui| {
                        for j in 0..2 {
                            if i + j < shapes.len() {
                                let (shape, name) = shapes[i + j];
                                let response = self.draw_shape_button(ui, shape, name, btn_width, btn_height);
                                if response.clicked() {
                                    let center_screen = Pos2::new(640.0, 400.0);
                                    let center_canvas = self.viewport.screen_to_canvas(center_screen);
                                    let node = Node::new(shape, center_canvas);
                                    self.selection.clear();
                                    self.selection.node_ids.push(node.id);
                                    self.document.nodes.push(node);
                                    self.history.push(&self.document);
                                }
                                if response.drag_started() {
                                    if let Some(pos) = response.interact_pointer_pos() {
                                        self.drag = DragState::DraggingNewNode {
                                            shape,
                                            current_screen: pos,
                                        };
                                    }
                                }
                                if response.dragged() {
                                    if let DragState::DraggingNewNode { ref mut current_screen, .. } = self.drag {
                                        if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                                            *current_screen = pos;
                                        }
                                    }
                                }
                            }
                        }
                    });
                    i += 2;
                }

                ui.add_space(8.0);

                // View section
                Self::draw_section_header(ui, "VIEW");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_grid, "");
                    ui.label(egui::RichText::new("Grid").size(12.0).color(TEXT_SECONDARY));
                    ui.add_space(8.0);
                    ui.checkbox(&mut self.snap_to_grid, "");
                    ui.label(egui::RichText::new("Snap").size(12.0).color(TEXT_SECONDARY));
                });
                ui.add_space(4.0);

                // Zoom control
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{:.0}%", self.viewport.zoom * 100.0))
                        .size(12.0).color(TEXT_DIM).monospace());
                    if ui.small_button("Reset").clicked() {
                        self.viewport.zoom = 1.0;
                        self.viewport.offset = [0.0, 0.0];
                    }
                });

                // Bottom spacer + node count
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!(
                        "{} nodes  {}  edges",
                        self.document.nodes.len(),
                        self.document.edges.len()
                    )).size(11.0).color(TEXT_DIM));
                });
            });
    }

    // -----------------------------------------------------------------------
    // Properties Panel (right sidebar)
    // -----------------------------------------------------------------------

    fn draw_properties_panel(&mut self, ctx: &egui::Context) {
        SidePanel::right("properties")
            .resizable(false)
            .exact_width(PROPERTIES_WIDTH)
            .frame(egui::Frame {
                fill: MANTLE,
                inner_margin: egui::Margin::same(12),
                stroke: Stroke::new(1.0, SURFACE1),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("PROPERTIES").size(10.0).color(TEXT_DIM).strong());
                ui.add_space(8.0);

                let sel_nodes = self.selection.node_ids.len();
                let sel_edges = self.selection.edge_ids.len();
                let total = sel_nodes + sel_edges;

                if total == 0 {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("No selection").size(13.0).color(TEXT_DIM));
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Click a node or edge\nto edit properties").size(11.0).color(TEXT_DIM));
                    });
                } else if total > 1 {
                    ui.label(egui::RichText::new(format!("{} items selected", total)).size(13.0).color(TEXT_SECONDARY));
                } else if sel_nodes == 1 {
                    let node_id = self.selection.node_ids[0];
                    if let Some(node) = self.document.find_node_mut(&node_id) {
                        // Node type badge
                        ui.horizontal(|ui| {
                            let shape_name = match node.shape {
                                NodeShape::Rectangle => "Rectangle",
                                NodeShape::RoundedRect => "Rounded Rect",
                                NodeShape::Diamond => "Diamond",
                                NodeShape::Circle => "Circle",
                                NodeShape::Parallelogram => "Parallelogram",
                            };
                            ui.label(egui::RichText::new(shape_name).size(13.0).strong().color(ACCENT));
                        });
                        ui.add_space(8.0);

                        // Content section
                        Self::draw_section_header(ui, "CONTENT");
                        ui.label(egui::RichText::new("Label").size(11.0).color(TEXT_DIM));
                        ui.add(egui::TextEdit::singleline(&mut node.label)
                            .desired_width(f32::INFINITY)
                            .font(FontId::proportional(13.0)));
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Description").size(11.0).color(TEXT_DIM));
                        ui.add(egui::TextEdit::multiline(&mut node.description)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .font(FontId::proportional(12.0)));
                        ui.add_space(8.0);

                        // Style section
                        Self::draw_section_header(ui, "STYLE");
                        ui.horizontal(|ui| {
                            let mut c = Color32::from_rgba_premultiplied(
                                node.style.fill_color[0], node.style.fill_color[1],
                                node.style.fill_color[2], node.style.fill_color[3],
                            );
                            ui.label(egui::RichText::new("Fill").size(11.0).color(TEXT_DIM));
                            if ui.color_edit_button_srgba(&mut c).changed() {
                                node.style.fill_color = c.to_array();
                            }
                            ui.add_space(12.0);
                            let mut b = Color32::from_rgba_premultiplied(
                                node.style.border_color[0], node.style.border_color[1],
                                node.style.border_color[2], node.style.border_color[3],
                            );
                            ui.label(egui::RichText::new("Border").size(11.0).color(TEXT_DIM));
                            if ui.color_edit_button_srgba(&mut b).changed() {
                                node.style.border_color = b.to_array();
                            }
                        });
                        ui.add_space(4.0);
                        ui.add(egui::Slider::new(&mut node.style.border_width, 0.0..=10.0)
                            .text("Border"));
                        ui.add(egui::Slider::new(&mut node.style.font_size, 8.0..=48.0)
                            .text("Font"));
                        ui.add_space(8.0);

                        // Size section
                        Self::draw_section_header(ui, "DIMENSIONS");
                        ui.add(egui::Slider::new(&mut node.size[0], 40.0..=400.0).text("W"));
                        ui.add(egui::Slider::new(&mut node.size[1], 30.0..=400.0).text("H"));
                    }
                } else if sel_edges == 1 {
                    let edge_id = self.selection.edge_ids[0];
                    if let Some(edge) = self.document.find_edge_mut(&edge_id) {
                        ui.label(egui::RichText::new("Edge").size(13.0).strong().color(ACCENT));
                        ui.add_space(8.0);

                        Self::draw_section_header(ui, "CONTENT");
                        ui.label(egui::RichText::new("Label").size(11.0).color(TEXT_DIM));
                        ui.add(egui::TextEdit::singleline(&mut edge.label)
                            .desired_width(f32::INFINITY)
                            .font(FontId::proportional(13.0)));
                        ui.add_space(8.0);

                        Self::draw_section_header(ui, "STYLE");
                        ui.horizontal(|ui| {
                            let mut c = Color32::from_rgba_premultiplied(
                                edge.style.color[0], edge.style.color[1],
                                edge.style.color[2], edge.style.color[3],
                            );
                            ui.label(egui::RichText::new("Color").size(11.0).color(TEXT_DIM));
                            if ui.color_edit_button_srgba(&mut c).changed() {
                                edge.style.color = c.to_array();
                            }
                        });
                        ui.add(egui::Slider::new(&mut edge.style.width, 1.0..=10.0).text("Width"));
                    }
                }
            });
    }

    // -----------------------------------------------------------------------
    // Canvas (central panel)
    // -----------------------------------------------------------------------

    fn draw_canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::all());
        let canvas_rect = response.rect;

        // Fill background
        painter.rect_filled(canvas_rect, CornerRadius::ZERO, CANVAS_BG);

        // Draw grid
        if self.show_grid {
            self.draw_grid(&painter, canvas_rect);
        }

        // ---- Handle input ----
        let pointer_pos = response.hover_pos().or_else(|| {
            ui.ctx().input(|i| i.pointer.hover_pos())
        });

        // Scroll => zoom towards mouse
        let scroll_delta = ui.ctx().input(|i| i.raw_scroll_delta.y);
        if scroll_delta != 0.0 {
            if let Some(mouse) = pointer_pos {
                let old_zoom = self.viewport.zoom;
                let factor = if scroll_delta > 0.0 { 1.1 } else { 1.0 / 1.1 };
                self.viewport.zoom = (self.viewport.zoom * factor).clamp(0.1, 10.0);
                // Adjust offset so that the point under the mouse stays fixed
                let ratio = self.viewport.zoom / old_zoom;
                self.viewport.offset[0] = mouse.x - ratio * (mouse.x - self.viewport.offset[0]);
                self.viewport.offset[1] = mouse.y - ratio * (mouse.y - self.viewport.offset[1]);
            }
        }

        // Handle drag start
        if response.drag_started() {
            if let Some(mouse) = pointer_pos {
                let canvas_pos = self.viewport.screen_to_canvas(mouse);
                let middle_button =
                    ui.ctx().input(|i| i.pointer.button_down(egui::PointerButton::Middle));

                if self.space_held || middle_button {
                    // Pan
                    self.drag = DragState::Panning {
                        start_offset: self.viewport.offset,
                        start_mouse: mouse,
                    };
                } else if self.tool == Tool::Connect {
                    // Try to start edge from a port
                    if let Some(port) = self.hit_test_port(canvas_pos) {
                        self.drag = DragState::CreatingEdge {
                            source: port,
                            current_screen: mouse,
                        };
                    }
                } else {
                    // Select tool
                    // Check if clicked on a port first (for connect mode)
                    if let Some(port) = self.hit_test_port(canvas_pos) {
                        // If we clicked a port even in Select mode, start edge creation
                        self.drag = DragState::CreatingEdge {
                            source: port,
                            current_screen: mouse,
                        };
                    } else if let Some(node_id) = self.document.node_at_pos(canvas_pos) {
                        // Clicked on a node
                        let cmd_held = ui.ctx().input(|i| i.modifiers.command);
                        if cmd_held {
                            self.selection.toggle_node(node_id);
                        } else if !self.selection.contains_node(&node_id) {
                            self.selection.select_node(node_id);
                        }
                        // Start dragging selected nodes
                        let start_positions: Vec<(NodeId, Pos2)> = self
                            .selection
                            .node_ids
                            .iter()
                            .filter_map(|id| {
                                self.document.find_node(id).map(|n| (*id, n.pos()))
                            })
                            .collect();
                        self.drag = DragState::DraggingNode {
                            start_positions,
                            start_mouse: canvas_pos,
                        };
                    } else {
                        // Check if clicked on an edge
                        if let Some(edge_id) = self.hit_test_edge(canvas_pos) {
                            let cmd_held = ui.ctx().input(|i| i.modifiers.command);
                            if cmd_held {
                                self.selection.toggle_node(NodeId(edge_id.0)); // edge toggle not in API so handle manually
                                // Actually toggle edge:
                                if self.selection.contains_edge(&edge_id) {
                                    self.selection
                                        .edge_ids
                                        .retain(|e| *e != edge_id);
                                } else {
                                    self.selection.edge_ids.push(edge_id);
                                }
                            } else {
                                self.selection.select_edge(edge_id);
                            }
                            self.drag = DragState::None;
                        } else {
                            // Empty canvas => box select
                            let cmd_held = ui.ctx().input(|i| i.modifiers.command);
                            if !cmd_held {
                                self.selection.clear();
                            }
                            self.drag = DragState::BoxSelect {
                                start_canvas: canvas_pos,
                            };
                        }
                    }
                }
            }
        }

        // Handle dragging
        if response.dragged() {
            if let Some(mouse) = pointer_pos {
                match &self.drag {
                    DragState::Panning {
                        start_offset,
                        start_mouse,
                    } => {
                        let delta = mouse - *start_mouse;
                        self.viewport.offset[0] = start_offset[0] + delta.x;
                        self.viewport.offset[1] = start_offset[1] + delta.y;
                    }
                    DragState::DraggingNode {
                        start_positions,
                        start_mouse,
                    } => {
                        let canvas_mouse = self.viewport.screen_to_canvas(mouse);
                        let delta = canvas_mouse - *start_mouse;
                        let positions = start_positions.clone();
                        for (id, start_pos) in &positions {
                            let mut new_pos = *start_pos + delta;
                            if self.snap_to_grid {
                                new_pos = self.snap_pos(new_pos);
                            }
                            if let Some(node) = self.document.find_node_mut(id) {
                                node.set_pos(new_pos);
                            }
                        }
                    }
                    DragState::CreatingEdge { .. } => {
                        if let DragState::CreatingEdge {
                            ref mut current_screen,
                            ..
                        } = self.drag
                        {
                            *current_screen = mouse;
                        }
                    }
                    DragState::DraggingNewNode { .. } => {
                        if let DragState::DraggingNewNode {
                            ref mut current_screen,
                            ..
                        } = self.drag
                        {
                            *current_screen = mouse;
                        }
                    }
                    DragState::BoxSelect { .. } | DragState::None => {}
                }
            }
        }

        // Handle drag end
        if response.drag_stopped() {
            if let Some(mouse) = pointer_pos {
                match &self.drag {
                    DragState::DraggingNode { .. } => {
                        self.history.push(&self.document);
                    }
                    DragState::CreatingEdge { source, .. } => {
                        let canvas_pos = self.viewport.screen_to_canvas(mouse);
                        if let Some(target) = self.hit_test_port(canvas_pos) {
                            // Don't connect a node to itself on the same port
                            if source.node_id != target.node_id {
                                let edge = Edge::new(source.clone(), target);
                                self.document.edges.push(edge);
                                self.history.push(&self.document);
                            }
                        }
                    }
                    DragState::BoxSelect { start_canvas } => {
                        let end_canvas = self.viewport.screen_to_canvas(mouse);
                        let sel_rect = Rect::from_two_pos(*start_canvas, end_canvas);
                        for node in &self.document.nodes {
                            if sel_rect.intersects(node.rect())
                                && !self.selection.contains_node(&node.id)
                            {
                                self.selection.node_ids.push(node.id);
                            }
                        }
                    }
                    DragState::DraggingNewNode { shape, current_screen } => {
                        // Drop new node on canvas
                        if canvas_rect.contains(*current_screen) {
                            let mut canvas_pos =
                                self.viewport.screen_to_canvas(*current_screen);
                            if self.snap_to_grid {
                                canvas_pos = self.snap_pos(canvas_pos);
                            }
                            let node = Node::new(*shape, canvas_pos);
                            self.selection.clear();
                            self.selection.node_ids.push(node.id);
                            self.document.nodes.push(node);
                            self.history.push(&self.document);
                        }
                    }
                    _ => {}
                }
            }
            self.drag = DragState::None;
        }

        // Handle click (non-drag) for edge selection / deselection
        if response.clicked() {
            if let Some(mouse) = pointer_pos {
                let canvas_pos = self.viewport.screen_to_canvas(mouse);
                // If nothing under cursor, clear selection
                if self.document.node_at_pos(canvas_pos).is_none()
                    && self.hit_test_port(canvas_pos).is_none()
                    && self.hit_test_edge(canvas_pos).is_none()
                {
                    let cmd_held = ui.ctx().input(|i| i.modifiers.command);
                    if !cmd_held {
                        self.selection.clear();
                    }
                }
            }
        }

        // ---- Draw edges ----
        for edge in &self.document.edges {
            self.draw_edge(edge, &painter);
        }

        // ---- Draw nodes ----
        for node in &self.document.nodes {
            self.draw_node(node, &painter);
        }

        // ---- Draw previews ----

        // Box select preview
        if let DragState::BoxSelect { start_canvas } = &self.drag {
            if let Some(mouse) = pointer_pos {
                let end_canvas = self.viewport.screen_to_canvas(mouse);
                let a = self.viewport.canvas_to_screen(*start_canvas);
                let b = self.viewport.canvas_to_screen(end_canvas);
                let sel_rect = Rect::from_two_pos(a, b);
                painter.rect_filled(sel_rect, CornerRadius::ZERO, BOX_SELECT_FILL);
                painter.rect_stroke(
                    sel_rect,
                    CornerRadius::ZERO,
                    Stroke::new(1.0, BOX_SELECT_STROKE),
                    StrokeKind::Outside,
                );
            }
        }

        // Edge creation preview
        if let DragState::CreatingEdge {
            source,
            current_screen,
        } = &self.drag
        {
            if let Some(src_node) = self.document.find_node(&source.node_id) {
                let src_pos = src_node.port_position(source.side);
                let src_screen = self.viewport.canvas_to_screen(src_pos);
                let dst = *current_screen;
                // Draw a simple bezier preview
                let offset = 60.0 * self.viewport.zoom;
                let (cp1, cp2) = control_points_for_side(src_screen, dst, source.side, offset);
                let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                    [src_screen, cp1, cp2, dst],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(2.0, SELECTION_COLOR),
                );
                painter.add(bezier);
            }
        }

        // New node preview while dragging from toolbar
        if let DragState::DraggingNewNode {
            shape,
            current_screen,
        } = &self.drag
        {
            if canvas_rect.contains(*current_screen) {
                let preview_node = Node::new(*shape, Pos2::ZERO);
                let half_w = preview_node.size[0] * 0.5 * self.viewport.zoom;
                let half_h = preview_node.size[1] * 0.5 * self.viewport.zoom;
                let screen_rect = Rect::from_center_size(
                    *current_screen,
                    Vec2::new(half_w * 2.0, half_h * 2.0),
                );
                painter.rect_filled(
                    screen_rect,
                    CornerRadius::same(4),
                    Color32::from_rgba_premultiplied(100, 160, 255, 80),
                );
                painter.rect_stroke(
                    screen_rect,
                    CornerRadius::same(4),
                    Stroke::new(1.5, SELECTION_COLOR),
                    StrokeKind::Outside,
                );
            }
        }

        // ---- Draw mini-map ----
        self.draw_minimap(&painter, canvas_rect);
    }

    // -----------------------------------------------------------------------
    // Mini-map
    // -----------------------------------------------------------------------

    fn draw_minimap(&self, painter: &egui::Painter, canvas_rect: Rect) {
        if self.document.nodes.is_empty() {
            return;
        }

        let minimap_w: f32 = 180.0;
        let minimap_h: f32 = 120.0;
        let margin: f32 = 12.0;

        let minimap_rect = Rect::from_min_size(
            Pos2::new(
                canvas_rect.max.x - minimap_w - margin,
                canvas_rect.max.y - minimap_h - margin,
            ),
            Vec2::new(minimap_w, minimap_h),
        );

        // Semi-transparent dark background
        painter.rect_filled(
            minimap_rect,
            CornerRadius::same(4),
            Color32::from_rgba_premultiplied(20, 20, 20, 200),
        );
        painter.rect_stroke(
            minimap_rect,
            CornerRadius::same(4),
            Stroke::new(1.0, Color32::from_rgba_premultiplied(80, 80, 80, 180)),
            StrokeKind::Outside,
        );

        // Compute bounding box of all nodes in canvas space
        let mut bb_min = Pos2::new(f32::MAX, f32::MAX);
        let mut bb_max = Pos2::new(f32::MIN, f32::MIN);
        for node in &self.document.nodes {
            let r = node.rect();
            bb_min.x = bb_min.x.min(r.min.x);
            bb_min.y = bb_min.y.min(r.min.y);
            bb_max.x = bb_max.x.max(r.max.x);
            bb_max.y = bb_max.y.max(r.max.y);
        }

        // Add some padding around the bounding box
        let padding = 50.0;
        bb_min.x -= padding;
        bb_min.y -= padding;
        bb_max.x += padding;
        bb_max.y += padding;

        let bb_w = (bb_max.x - bb_min.x).max(1.0);
        let bb_h = (bb_max.y - bb_min.y).max(1.0);

        // Inset the minimap rect slightly for drawing content
        let inset = 4.0;
        let draw_rect = minimap_rect.shrink(inset);
        let draw_w = draw_rect.width();
        let draw_h = draw_rect.height();

        // Scale to fit the bounding box into the minimap draw area
        let scale = (draw_w / bb_w).min(draw_h / bb_h);

        // Center the content within the draw area
        let content_w = bb_w * scale;
        let content_h = bb_h * scale;
        let offset_x = draw_rect.min.x + (draw_w - content_w) / 2.0;
        let offset_y = draw_rect.min.y + (draw_h - content_h) / 2.0;

        // Map a canvas-space point to minimap screen-space
        let map_point = |cx: f32, cy: f32| -> Pos2 {
            Pos2::new(
                offset_x + (cx - bb_min.x) * scale,
                offset_y + (cy - bb_min.y) * scale,
            )
        };

        // Draw each node as a small dot
        for node in &self.document.nodes {
            let center = node.rect().center();
            let screen_pt = map_point(center.x, center.y);
            if minimap_rect.contains(screen_pt) {
                painter.circle_filled(
                    screen_pt,
                    2.5,
                    Color32::from_rgba_premultiplied(80, 160, 255, 220),
                );
            }
        }

        // Draw current viewport rectangle
        // The viewport shows the region of canvas space visible on screen.
        // screen_to_canvas maps the canvas_rect corners to canvas space.
        let vp_tl = self.viewport.screen_to_canvas(canvas_rect.min);
        let vp_br = self.viewport.screen_to_canvas(canvas_rect.max);

        let vp_min = map_point(vp_tl.x, vp_tl.y);
        let vp_max = map_point(vp_br.x, vp_br.y);
        let vp_rect = Rect::from_two_pos(vp_min, vp_max);

        // Clip the viewport rect to the minimap bounds
        if let Some(clipped) = vp_rect.intersect(minimap_rect).is_positive().then(|| vp_rect.intersect(minimap_rect)) {
            painter.rect_filled(
                clipped,
                CornerRadius::ZERO,
                Color32::from_rgba_premultiplied(80, 160, 255, 30),
            );
            painter.rect_stroke(
                clipped,
                CornerRadius::ZERO,
                Stroke::new(1.0, Color32::from_rgba_premultiplied(80, 160, 255, 150)),
                StrokeKind::Outside,
            );
        }
    }

    // -----------------------------------------------------------------------
    // Draw grid
    // -----------------------------------------------------------------------

    fn draw_grid(&self, painter: &egui::Painter, canvas_rect: Rect) {
        let zoom = self.viewport.zoom;
        let grid_screen = self.grid_size * zoom;

        // Don't draw grid if too small
        if grid_screen < 8.0 {
            return;
        }

        let offset_x = self.viewport.offset[0] % grid_screen;
        let offset_y = self.viewport.offset[1] % grid_screen;

        // Draw as dots for a cleaner look
        let start_x = canvas_rect.min.x + offset_x;
        let start_y = canvas_rect.min.y + offset_y;

        let mut x = start_x;
        while x < canvas_rect.max.x {
            let mut y = start_y;
            while y < canvas_rect.max.y {
                painter.circle_filled(Pos2::new(x, y), 0.8, GRID_COLOR);
                y += grid_screen;
            }
            x += grid_screen;
        }
    }

    // -----------------------------------------------------------------------
    // Draw Node
    // -----------------------------------------------------------------------

    fn draw_node(&self, node: &Node, painter: &egui::Painter) {
        let top_left = self.viewport.canvas_to_screen(node.pos());
        let size = node.size_vec() * self.viewport.zoom;
        let screen_rect = Rect::from_min_size(top_left, size);

        // Drop shadow (draw before the shape)
        let shadow_offset = Vec2::new(2.0, 3.0) * self.viewport.zoom;
        let shadow_rect = screen_rect.translate(shadow_offset);
        let shadow_color = Color32::from_rgba_premultiplied(0, 0, 0, 40);
        painter.rect_filled(shadow_rect, CornerRadius::same(4), shadow_color);

        let fill = Color32::from_rgba_premultiplied(
            node.style.fill_color[0],
            node.style.fill_color[1],
            node.style.fill_color[2],
            node.style.fill_color[3],
        );

        let is_selected = self.selection.contains_node(&node.id);
        let border_color = if is_selected {
            SELECTION_COLOR
        } else {
            Color32::from_rgba_premultiplied(
                node.style.border_color[0],
                node.style.border_color[1],
                node.style.border_color[2],
                node.style.border_color[3],
            )
        };
        let border_width = if is_selected {
            node.style.border_width.max(2.5)
        } else {
            node.style.border_width
        };
        let stroke = Stroke::new(border_width * self.viewport.zoom.sqrt(), border_color);

        match node.shape {
            NodeShape::Rectangle => {
                painter.rect_filled(screen_rect, CornerRadius::ZERO, fill);
                painter.rect_stroke(screen_rect, CornerRadius::ZERO, stroke, StrokeKind::Outside);
            }
            NodeShape::RoundedRect => {
                let r = (10.0 * self.viewport.zoom) as u8;
                painter.rect_filled(screen_rect, CornerRadius::same(r), fill);
                painter.rect_stroke(
                    screen_rect,
                    CornerRadius::same(r),
                    stroke,
                    StrokeKind::Outside,
                );
            }
            NodeShape::Diamond => {
                let center = screen_rect.center();
                let hw = screen_rect.width() / 2.0;
                let hh = screen_rect.height() / 2.0;
                let points = vec![
                    Pos2::new(center.x, center.y - hh),
                    Pos2::new(center.x + hw, center.y),
                    Pos2::new(center.x, center.y + hh),
                    Pos2::new(center.x - hw, center.y),
                ];
                painter.add(egui::Shape::convex_polygon(points, fill, stroke));
            }
            NodeShape::Circle => {
                let center = screen_rect.center();
                let radius = screen_rect.width().min(screen_rect.height()) / 2.0;
                painter.circle_filled(center, radius, fill);
                painter.circle_stroke(center, radius, stroke);
            }
            NodeShape::Parallelogram => {
                let skew = screen_rect.width() * 0.15;
                let points = vec![
                    Pos2::new(screen_rect.min.x + skew, screen_rect.min.y),
                    Pos2::new(screen_rect.max.x, screen_rect.min.y),
                    Pos2::new(screen_rect.max.x - skew, screen_rect.max.y),
                    Pos2::new(screen_rect.min.x, screen_rect.max.y),
                ];
                painter.add(egui::Shape::convex_polygon(points, fill, stroke));
            }
        }

        // Draw label text centered
        let text_color = Color32::from_rgba_premultiplied(
            node.style.text_color[0],
            node.style.text_color[1],
            node.style.text_color[2],
            node.style.text_color[3],
        );
        let font_size = node.style.font_size * self.viewport.zoom;
        if font_size > 4.0 {
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                &node.label,
                FontId::proportional(font_size),
                text_color,
            );
        }

        // Draw port circles on all 4 sides
        let port_sides = [PortSide::Top, PortSide::Bottom, PortSide::Left, PortSide::Right];
        for side in &port_sides {
            let canvas_port = node.port_position(*side);
            let screen_port = self.viewport.canvas_to_screen(canvas_port);
            let r = PORT_RADIUS * self.viewport.zoom.sqrt();
            painter.circle_filled(screen_port, r, PORT_FILL);
            painter.circle_stroke(screen_port, r, Stroke::new(1.5, SELECTION_COLOR));
        }
    }

    // -----------------------------------------------------------------------
    // Draw Edge
    // -----------------------------------------------------------------------

    fn draw_edge(&self, edge: &Edge, painter: &egui::Painter) {
        let src_node = self.document.find_node(&edge.source.node_id);
        let tgt_node = self.document.find_node(&edge.target.node_id);
        let (src_node, tgt_node) = match (src_node, tgt_node) {
            (Some(s), Some(t)) => (s, t),
            _ => return,
        };

        let src_canvas = src_node.port_position(edge.source.side);
        let tgt_canvas = tgt_node.port_position(edge.target.side);

        let src = self.viewport.canvas_to_screen(src_canvas);
        let tgt = self.viewport.canvas_to_screen(tgt_canvas);

        let is_selected = self.selection.contains_edge(&edge.id);
        let edge_color = if is_selected {
            SELECTION_COLOR
        } else {
            Color32::from_rgba_premultiplied(
                edge.style.color[0],
                edge.style.color[1],
                edge.style.color[2],
                edge.style.color[3],
            )
        };
        let width = edge.style.width * self.viewport.zoom.sqrt();

        let offset = 60.0 * self.viewport.zoom;
        let (cp1, cp2) = control_points_for_side(src, tgt, edge.source.side, offset);

        let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
            [src, cp1, cp2, tgt],
            false,
            Color32::TRANSPARENT,
            Stroke::new(width, edge_color),
        );
        painter.add(bezier);

        // Arrow head at target
        self.draw_arrow_head(painter, cp2, tgt, edge_color, width);

        // Edge label at midpoint
        if !edge.label.is_empty() {
            // Approximate midpoint of cubic bezier at t=0.5
            let mid = cubic_bezier_point(src, cp1, cp2, tgt, 0.5);
            let font_size = 12.0 * self.viewport.zoom;
            if font_size > 4.0 {
                // Background for readability
                let galley = painter.layout_no_wrap(
                    edge.label.clone(),
                    FontId::proportional(font_size),
                    Color32::WHITE,
                );
                let text_rect = Rect::from_min_size(
                    Pos2::new(
                        mid.x - galley.size().x / 2.0,
                        mid.y - galley.size().y / 2.0,
                    ),
                    galley.size(),
                )
                .expand(3.0);
                painter.rect_filled(
                    text_rect,
                    CornerRadius::same(3),
                    Color32::from_rgba_premultiplied(30, 30, 30, 200),
                );
                painter.text(
                    mid,
                    Align2::CENTER_CENTER,
                    &edge.label,
                    FontId::proportional(font_size),
                    edge_color,
                );
            }
        }
    }

    fn draw_arrow_head(
        &self,
        painter: &egui::Painter,
        from: Pos2,
        to: Pos2,
        color: Color32,
        width: f32,
    ) {
        let dir = (to - from).normalized();
        if dir.length() < 0.01 {
            return;
        }
        let arrow_len = 10.0 * self.viewport.zoom.sqrt();
        let arrow_width = 6.0 * self.viewport.zoom.sqrt();
        let perp = Vec2::new(-dir.y, dir.x);

        let tip = to;
        let left = tip - dir * arrow_len + perp * arrow_width;
        let right = tip - dir * arrow_len - perp * arrow_width;

        painter.add(egui::Shape::convex_polygon(
            vec![tip, left, right],
            color,
            Stroke::new(width * 0.5, color),
        ));
    }

    // -----------------------------------------------------------------------
    // Hit testing
    // -----------------------------------------------------------------------

    fn hit_test_port(&self, canvas_pos: Pos2) -> Option<Port> {
        let threshold = PORT_HIT_RADIUS / self.viewport.zoom;
        let sides = [PortSide::Top, PortSide::Bottom, PortSide::Left, PortSide::Right];
        // Iterate in reverse so topmost node wins
        for node in self.document.nodes.iter().rev() {
            for side in &sides {
                let port_pos = node.port_position(*side);
                if (canvas_pos - port_pos).length() < threshold {
                    return Some(Port {
                        node_id: node.id,
                        side: *side,
                    });
                }
            }
        }
        None
    }

    fn hit_test_edge(&self, canvas_pos: Pos2) -> Option<EdgeId> {
        let threshold = 8.0 / self.viewport.zoom;
        for edge in self.document.edges.iter().rev() {
            let src_node = self.document.find_node(&edge.source.node_id);
            let tgt_node = self.document.find_node(&edge.target.node_id);
            if let (Some(sn), Some(tn)) = (src_node, tgt_node) {
                let src = sn.port_position(edge.source.side);
                let tgt = tn.port_position(edge.target.side);
                let offset = 60.0;
                let (cp1, cp2) = control_points_for_side_canvas(src, tgt, edge.source.side, offset);
                // Sample the bezier at several points and check distance
                for i in 0..=20 {
                    let t = i as f32 / 20.0;
                    let p = cubic_bezier_point(src, cp1, cp2, tgt, t);
                    if (canvas_pos - p).length() < threshold {
                        return Some(edge.id);
                    }
                }
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // Snap
    // -----------------------------------------------------------------------

    fn snap_pos(&self, pos: Pos2) -> Pos2 {
        Pos2::new(
            (pos.x / self.grid_size).round() * self.grid_size,
            (pos.y / self.grid_size).round() * self.grid_size,
        )
    }
}

// ---------------------------------------------------------------------------
// eframe::App implementation
// ---------------------------------------------------------------------------

impl eframe::App for FlowchartApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.draw_toolbar(ctx);
        self.draw_properties_panel(ctx);

        CentralPanel::default()
            .frame(egui::Frame::NONE.fill(CANVAS_BG))
            .show(ctx, |ui| {
                self.draw_canvas(ui);
            });
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Compute control points for a cubic bezier between two screen-space points,
/// offsetting perpendicular to the source port side.
fn control_points_for_side(
    src: Pos2,
    tgt: Pos2,
    source_side: PortSide,
    offset: f32,
) -> (Pos2, Pos2) {
    let cp1 = match source_side {
        PortSide::Top => Pos2::new(src.x, src.y - offset),
        PortSide::Bottom => Pos2::new(src.x, src.y + offset),
        PortSide::Left => Pos2::new(src.x - offset, src.y),
        PortSide::Right => Pos2::new(src.x + offset, src.y),
    };
    // Target control point: towards the source from target
    let dx = src.x - tgt.x;
    let dy = src.y - tgt.y;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let cp2 = Pos2::new(tgt.x + dx / len * offset, tgt.y + dy / len * offset);
    (cp1, cp2)
}

/// Same as control_points_for_side but for canvas-space coords (used in edge hit testing).
fn control_points_for_side_canvas(
    src: Pos2,
    tgt: Pos2,
    source_side: PortSide,
    offset: f32,
) -> (Pos2, Pos2) {
    control_points_for_side(src, tgt, source_side, offset)
}

/// Evaluate a cubic bezier at parameter t in [0,1].
fn cubic_bezier_point(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let uuu = uu * u;
    let ttt = tt * t;
    Pos2::new(
        uuu * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + ttt * p3.x,
        uuu * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + ttt * p3.y,
    )
}
