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
        start_rect: [f32; 4], // [x, y, w, h]
        start_mouse: Pos2,    // canvas pos at drag start
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
const TOOLBAR_WIDTH: f32 = 220.0;
const PROPERTIES_WIDTH: f32 = 280.0;

// Extra colors for UI
const ACCENT: Color32 = Color32::from_rgb(137, 180, 250);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(205, 214, 244);
const TEXT_SECONDARY: Color32 = Color32::from_rgb(166, 173, 200);
const TEXT_DIM: Color32 = Color32::from_rgb(108, 112, 134);
const SURFACE0: Color32 = Color32::from_rgb(49, 50, 68);
const SURFACE1: Color32 = Color32::from_rgb(69, 71, 90);
const MANTLE: Color32 = Color32::from_rgb(24, 24, 37);

/// Convert an [u8; 4] RGBA array to an egui Color32, avoiding repeated verbose calls.
fn to_color32(rgba: [u8; 4]) -> Color32 {
    Color32::from_rgba_premultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

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
    pub diagram_mode: DiagramMode,
    pub selected_sticky_color: StickyColor,
    /// Track whether space key is held (for pan mode)
    space_held: bool,
    /// Cached canvas rect from last frame (for toolbar new-node placement)
    canvas_rect: Rect,
    /// Status toast message with creation time
    status_message: Option<(String, std::time::Instant)>,
    /// Flag to request focus on label text edit (e.g., on double-click)
    focus_label_edit: bool,
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
        }
    }

    // -----------------------------------------------------------------------
    // UI Helpers
    // -----------------------------------------------------------------------

    fn draw_section_header(ui: &mut egui::Ui, label: &str) {
        ui.label(egui::RichText::new(label).size(10.0).color(TEXT_DIM).strong());
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
            let node_ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            let edge_ids: Vec<EdgeId> = self.selection.edge_ids.iter().copied().collect();
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
                self.selection.node_ids.insert(node.id);
                self.document.nodes.push(node);
            }
            self.history.push(&self.document);
        }

        // V = select tool (when no modifier held)
        if ctx.input(|i| i.key_pressed(Key::V) && i.modifiers.is_none()) {
            self.tool = Tool::Select;
        }
        // E = connect tool (when no modifier held)
        if ctx.input(|i| i.key_pressed(Key::E) && i.modifiers.is_none()) {
            self.tool = Tool::Connect;
        }

        // Cmd+A = select all
        if ctx.input(|i| i.key_pressed(Key::A) && i.modifiers.matches_exact(cmd)) {
            self.selection.clear();
            for node in &self.document.nodes {
                self.selection.node_ids.insert(node.id);
            }
            for edge in &self.document.edges {
                self.selection.edge_ids.insert(edge.id);
            }
        }

        // Cmd+1 = fit to content
        if ctx.input(|i| i.key_pressed(Key::Num1) && i.modifiers.matches_exact(cmd)) {
            self.fit_to_content();
        }

        // Cmd+2 = zoom to selection
        if ctx.input(|i| i.key_pressed(Key::Num2) && i.modifiers.matches_exact(cmd)) {
            self.zoom_to_selection();
        }

        // Cmd+= zoom in (25%)
        if ctx.input(|i| (i.key_pressed(Key::Equals) || i.key_pressed(Key::Plus)) && i.modifiers.matches_exact(cmd)) {
            self.step_zoom(1.25);
        }

        // Cmd+- zoom out (25%)
        if ctx.input(|i| i.key_pressed(Key::Minus) && i.modifiers.matches_exact(cmd)) {
            self.step_zoom(0.8);
        }

        // Cmd+0 = reset zoom to 100%
        if ctx.input(|i| i.key_pressed(Key::Num0) && i.modifiers.matches_exact(cmd)) {
            self.viewport.zoom = 1.0;
            self.viewport.offset = [0.0, 0.0];
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
                inner_margin: egui::Margin::same(16),
                stroke: Stroke::new(1.0, SURFACE1),
                ..Default::default()
            })
            .show(ctx, |ui| {
                // App title
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Light Figma").size(18.0).strong().color(TEXT_PRIMARY));
                });
                ui.add_space(16.0);

                // File actions
                Self::draw_section_header(ui, "FILE");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let btn_size = egui::vec2(84.0, 32.0);
                    if ui.add_sized(btn_size, egui::Button::new(
                        egui::RichText::new("Save").size(12.0)
                    )).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Flowchart", &["flow"])
                            .set_file_name("untitled.flow")
                            .save_file()
                        {
                            match io::save_document(&self.document, &path) {
                                Ok(()) => {
                                    self.status_message = Some(("Saved!".to_string(), std::time::Instant::now()));
                                }
                                Err(e) => eprintln!("Save error: {}", e),
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
                ui.add_space(12.0);

                // Export
                Self::draw_section_header(ui, "EXPORT");
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    let btn_size = egui::vec2(54.0, 30.0);
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
                                .set_file_name(format!("flowchart.{}", ext))
                                .save_file()
                            {
                                let result = match export_fn {
                                    "png" => export::export_png(&self.document, &path),
                                    "svg" => export::export_svg(&self.document, &path),
                                    "pdf" => export::export_pdf(&self.document, &path),
                                    _ => Ok(()),
                                };
                                match result {
                                    Ok(()) => {
                                        self.status_message = Some((format!("Exported {}!", label), std::time::Instant::now()));
                                    }
                                    Err(e) => eprintln!("Export error: {}", e),
                                }
                            }
                        }
                    }
                });
                ui.add_space(12.0);

                // Tools
                Self::draw_section_header(ui, "TOOLS");
                ui.add_space(4.0);
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

                    let btn_size = egui::vec2(84.0, 32.0);
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
                ui.label(egui::RichText::new("V Select  \u{00b7}  E Connect").size(9.0).color(TEXT_DIM));
                ui.add_space(12.0);

                // Mode tabs
                Self::draw_section_header(ui, "MODE");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let modes = [
                        (DiagramMode::Flowchart, "Flow"),
                        (DiagramMode::ER, "ER"),
                        (DiagramMode::FigJam, "FigJam"),
                    ];
                    for (mode, label) in modes {
                        let is_active = self.diagram_mode == mode;
                        let text = if is_active {
                            egui::RichText::new(label).size(11.0).strong().color(ACCENT)
                        } else {
                            egui::RichText::new(label).size(11.0).color(TEXT_SECONDARY)
                        };
                        if ui.add(egui::Button::new(text)
                            .fill(if is_active { SURFACE1 } else { SURFACE0 })
                        ).clicked() {
                            self.diagram_mode = mode;
                        }
                    }
                });
                ui.add_space(12.0);

                // Shapes (mode-dependent)
                Self::draw_section_header(ui, "SHAPES");
                ui.add_space(6.0);

                match self.diagram_mode {
                    DiagramMode::Flowchart => {
                        let shapes = [
                            (NodeShape::Rectangle, "Rectangle"),
                            (NodeShape::RoundedRect, "Rounded"),
                            (NodeShape::Diamond, "Diamond"),
                            (NodeShape::Circle, "Circle"),
                            (NodeShape::Parallelogram, "Parallel"),
                        ];

                        let available_width = ui.available_width();
                        let btn_width = (available_width - 10.0) / 2.0;
                        let btn_height = 56.0;

                        let mut i = 0;
                        while i < shapes.len() {
                            ui.horizontal(|ui| {
                                for j in 0..2 {
                                    if i + j < shapes.len() {
                                        let (shape, name) = shapes[i + j];
                                        let response = self.draw_shape_button(ui, shape, name, btn_width, btn_height);
                                        if response.clicked() {
                                            let center_screen = self.canvas_rect.center();
                                            let center_canvas = self.viewport.screen_to_canvas(center_screen);
                                            let node = Node::new(shape, center_canvas);
                                            self.selection.clear();
                                            self.selection.node_ids.insert(node.id);
                                            self.document.nodes.push(node);
                                            self.history.push(&self.document);
                                        }
                                        if response.drag_started() {
                                            if let Some(pos) = response.interact_pointer_pos() {
                                                self.drag = DragState::DraggingNewNode {
                                                    kind: NodeKind::Shape { shape, label: "New Node".into(), description: String::new() },
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
                    }
                    DiagramMode::ER => {
                        let available_width = ui.available_width();
                        if ui.add_sized(
                            egui::vec2(available_width, 40.0),
                            egui::Button::new(egui::RichText::new("+ Entity").size(13.0).color(TEXT_PRIMARY))
                                .fill(SURFACE0),
                        ).clicked() {
                            let center_screen = self.canvas_rect.center();
                            let center_canvas = self.viewport.screen_to_canvas(center_screen);
                            let node = Node::new_entity(center_canvas);
                            self.selection.clear();
                            self.selection.node_ids.insert(node.id);
                            self.document.nodes.push(node);
                            self.history.push(&self.document);
                        }
                    }
                    DiagramMode::FigJam => {
                        let available_width = ui.available_width();
                        // Sticky note button
                        if ui.add_sized(
                            egui::vec2(available_width, 40.0),
                            egui::Button::new(egui::RichText::new("+ Sticky Note").size(13.0).color(TEXT_PRIMARY))
                                .fill(SURFACE0),
                        ).clicked() {
                            let center_screen = self.canvas_rect.center();
                            let center_canvas = self.viewport.screen_to_canvas(center_screen);
                            let node = Node::new_sticky(self.selected_sticky_color, center_canvas);
                            self.selection.clear();
                            self.selection.node_ids.insert(node.id);
                            self.document.nodes.push(node);
                            self.history.push(&self.document);
                        }
                        ui.add_space(4.0);

                        // Sticky color picker
                        ui.horizontal(|ui| {
                            for color in &StickyColor::ALL {
                                let fill = to_color32(color.fill_rgba());
                                let is_selected = self.selected_sticky_color == *color;
                                let size = if is_selected { 22.0 } else { 18.0 };
                                let (response, painter) = ui.allocate_painter(egui::vec2(size, size), Sense::click());
                                let r = response.rect;
                                painter.circle_filled(r.center(), size / 2.0, fill);
                                if is_selected {
                                    painter.circle_stroke(r.center(), size / 2.0, Stroke::new(2.0, Color32::WHITE));
                                }
                                if response.clicked() {
                                    self.selected_sticky_color = *color;
                                }
                            }
                        });
                        ui.add_space(8.0);

                        // Text node button
                        if ui.add_sized(
                            egui::vec2(available_width, 36.0),
                            egui::Button::new(egui::RichText::new("+ Text").size(13.0).color(TEXT_PRIMARY))
                                .fill(SURFACE0),
                        ).clicked() {
                            let center_screen = self.canvas_rect.center();
                            let center_canvas = self.viewport.screen_to_canvas(center_screen);
                            let node = Node::new_text(center_canvas);
                            self.selection.clear();
                            self.selection.node_ids.insert(node.id);
                            self.document.nodes.push(node);
                            self.history.push(&self.document);
                        }
                    }
                }

                ui.add_space(12.0);

                // View
                Self::draw_section_header(ui, "VIEW");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_grid, "");
                    ui.label(egui::RichText::new("Grid").size(12.0).color(TEXT_SECONDARY));
                    ui.add_space(12.0);
                    ui.checkbox(&mut self.snap_to_grid, "");
                    ui.label(egui::RichText::new("Snap").size(12.0).color(TEXT_SECONDARY));
                });
                ui.add_space(8.0);

                // Zoom
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
                inner_margin: egui::Margin::same(16),
                stroke: Stroke::new(1.0, SURFACE1),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("PROPERTIES").size(10.0).color(TEXT_DIM).strong());
                ui.add_space(12.0);

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
                    let node_id = *self.selection.node_ids.iter().next().unwrap();
                    if let Some(node) = self.document.find_node_mut(&node_id) {
                        // Node type badge
                        let kind_name = match &node.kind {
                            NodeKind::Shape { shape, .. } => match shape {
                                NodeShape::Rectangle => "Rectangle",
                                NodeShape::RoundedRect => "Rounded Rect",
                                NodeShape::Diamond => "Diamond",
                                NodeShape::Circle => "Circle",
                                NodeShape::Parallelogram => "Parallelogram",
                            },
                            NodeKind::StickyNote { .. } => "Sticky Note",
                            NodeKind::Entity { .. } => "Entity",
                            NodeKind::Text { .. } => "Text",
                        };
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(kind_name).size(13.0).strong().color(ACCENT));
                        });
                        ui.add_space(12.0);

                        // Content section — varies by node kind
                        let mut needs_entity_resize = false;
                        match &mut node.kind {
                            NodeKind::Shape { label, description, .. } => {
                                Self::draw_section_header(ui, "CONTENT");
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("Label").size(11.0).color(TEXT_DIM));
                                ui.add_space(2.0);
                                let label_response = ui.add(egui::TextEdit::singleline(label)
                                    .desired_width(f32::INFINITY)
                                    .font(FontId::proportional(13.0)));
                                if self.focus_label_edit {
                                    label_response.request_focus();
                                    self.focus_label_edit = false;
                                }
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new("Description").size(11.0).color(TEXT_DIM));
                                ui.add_space(2.0);
                                ui.add(egui::TextEdit::multiline(description)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(3)
                                    .font(FontId::proportional(12.0)));
                            }
                            NodeKind::StickyNote { text, color } => {
                                Self::draw_section_header(ui, "CONTENT");
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("Text").size(11.0).color(TEXT_DIM));
                                ui.add_space(2.0);
                                let text_response = ui.add(egui::TextEdit::multiline(text)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(4)
                                    .font(FontId::proportional(13.0)));
                                if self.focus_label_edit {
                                    text_response.request_focus();
                                    self.focus_label_edit = false;
                                }
                                ui.add_space(12.0);

                                Self::draw_section_header(ui, "COLOR");
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    for sc in &StickyColor::ALL {
                                        let fill = to_color32(sc.fill_rgba());
                                        let is_active = *color == *sc;
                                        let size = if is_active { 24.0 } else { 20.0 };
                                        let (response, painter) = ui.allocate_painter(egui::vec2(size, size), Sense::click());
                                        let r = response.rect;
                                        painter.circle_filled(r.center(), size / 2.0, fill);
                                        if is_active {
                                            painter.circle_stroke(r.center(), size / 2.0, Stroke::new(2.0, Color32::WHITE));
                                        }
                                        if response.clicked() {
                                            *color = *sc;
                                            node.style.fill_color = sc.fill_rgba();
                                            node.style.text_color = sc.text_rgba();
                                        }
                                    }
                                });
                            }
                            NodeKind::Entity { name, attributes } => {
                                Self::draw_section_header(ui, "CONTENT");
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("Name").size(11.0).color(TEXT_DIM));
                                ui.add_space(2.0);
                                let name_response = ui.add(egui::TextEdit::singleline(name)
                                    .desired_width(f32::INFINITY)
                                    .font(FontId::proportional(13.0)));
                                if self.focus_label_edit {
                                    name_response.request_focus();
                                    self.focus_label_edit = false;
                                }
                                ui.add_space(12.0);

                                Self::draw_section_header(ui, "ATTRIBUTES");
                                ui.add_space(4.0);

                                let mut to_remove: Option<usize> = None;
                                for (i, attr) in attributes.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        let pk_text = if attr.is_primary_key {
                                            egui::RichText::new("PK").size(9.0).strong().color(ACCENT)
                                        } else {
                                            egui::RichText::new("PK").size(9.0).color(TEXT_DIM)
                                        };
                                        if ui.add(egui::Button::new(pk_text).min_size(egui::vec2(24.0, 18.0)))
                                            .on_hover_text("Primary Key — uniquely identifies each row in this table")
                                            .clicked()
                                        {
                                            attr.is_primary_key = !attr.is_primary_key;
                                        }
                                        let fk_text = if attr.is_foreign_key {
                                            egui::RichText::new("FK").size(9.0).strong().color(
                                                Color32::from_rgb(249, 226, 175))
                                        } else {
                                            egui::RichText::new("FK").size(9.0).color(TEXT_DIM)
                                        };
                                        if ui.add(egui::Button::new(fk_text).min_size(egui::vec2(24.0, 18.0)))
                                            .on_hover_text("Foreign Key — references a primary key in another table")
                                            .clicked()
                                        {
                                            attr.is_foreign_key = !attr.is_foreign_key;
                                        }
                                        ui.add(egui::TextEdit::singleline(&mut attr.name)
                                            .desired_width(60.0)
                                            .font(FontId::proportional(11.0)))
                                            .on_hover_text("Attribute name (e.g. id, name, email)");
                                        ui.add(egui::TextEdit::singleline(&mut attr.attr_type)
                                            .desired_width(50.0)
                                            .font(FontId::monospace(10.0)))
                                            .on_hover_text("Data type (e.g. INT, VARCHAR, TIMESTAMP)");
                                        if ui.add(egui::Button::new(
                                            egui::RichText::new("x").size(10.0).color(TEXT_DIM)
                                        ).min_size(egui::vec2(18.0, 18.0)))
                                            .on_hover_text("Remove this attribute")
                                            .clicked()
                                        {
                                            to_remove = Some(i);
                                        }
                                    });
                                }
                                if let Some(i) = to_remove {
                                    attributes.remove(i);
                                    needs_entity_resize = true;
                                }
                                ui.add_space(4.0);
                                if ui.add(egui::Button::new(
                                    egui::RichText::new("+ Add Attribute").size(11.0).color(ACCENT)
                                )).clicked() {
                                    attributes.push(EntityAttribute {
                                        name: String::from("field"),
                                        attr_type: String::from("INT"),
                                        is_primary_key: false,
                                        is_foreign_key: false,
                                    });
                                    needs_entity_resize = true;
                                }
                            }
                            NodeKind::Text { content } => {
                                Self::draw_section_header(ui, "CONTENT");
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("Content").size(11.0).color(TEXT_DIM));
                                ui.add_space(2.0);
                                let text_response = ui.add(egui::TextEdit::multiline(content)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(3)
                                    .font(FontId::proportional(13.0)));
                                if self.focus_label_edit {
                                    text_response.request_focus();
                                    self.focus_label_edit = false;
                                }
                            }
                        }
                        ui.add_space(16.0);

                        if needs_entity_resize {
                            node.auto_size_entity();
                        }

                        // Style section
                        Self::draw_section_header(ui, "STYLE");
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let mut c = to_color32(node.style.fill_color);
                            ui.label(egui::RichText::new("Fill").size(11.0).color(TEXT_DIM));
                            if ui.color_edit_button_srgba(&mut c).changed() {
                                node.style.fill_color = c.to_array();
                            }
                            ui.add_space(16.0);
                            let mut b = to_color32(node.style.border_color);
                            ui.label(egui::RichText::new("Border").size(11.0).color(TEXT_DIM));
                            if ui.color_edit_button_srgba(&mut b).changed() {
                                node.style.border_color = b.to_array();
                            }
                        });
                        ui.add_space(8.0);
                        ui.add(egui::Slider::new(&mut node.style.border_width, 0.0..=10.0)
                            .text("Border"));
                        ui.add_space(4.0);
                        ui.add(egui::Slider::new(&mut node.style.font_size, 8.0..=48.0)
                            .text("Font"));
                        ui.add_space(16.0);

                        // Size section
                        Self::draw_section_header(ui, "DIMENSIONS");
                        ui.add_space(4.0);
                        ui.add(egui::Slider::new(&mut node.size[0], 40.0..=400.0).text("W"));
                        ui.add_space(4.0);
                        ui.add(egui::Slider::new(&mut node.size[1], 30.0..=400.0).text("H"));
                    }
                } else if sel_edges == 1 {
                    let edge_id = *self.selection.edge_ids.iter().next().unwrap();
                    if let Some(edge) = self.document.find_edge_mut(&edge_id) {
                        ui.label(egui::RichText::new("Edge").size(13.0).strong().color(ACCENT));
                        ui.add_space(12.0);

                        Self::draw_section_header(ui, "CONTENT");
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Label").size(11.0).color(TEXT_DIM));
                        ui.add_space(2.0);
                        ui.add(egui::TextEdit::singleline(&mut edge.label)
                            .desired_width(f32::INFINITY)
                            .font(FontId::proportional(13.0)));
                        ui.add_space(12.0);

                        Self::draw_section_header(ui, "RELATIONSHIP");
                        ui.add_space(4.0);

                        let rel_presets: &[(&str, &str, Cardinality, Cardinality, &str)] = &[
                            ("None", "──▶", Cardinality::None, Cardinality::None,
                             "No cardinality. A plain arrow."),
                            ("1 : 1", "||──||", Cardinality::ExactlyOne, Cardinality::ExactlyOne,
                             "One to One\nEach record relates to exactly one on the other side.\nExample: User ↔ Profile"),
                            ("1 : N", "||──o<", Cardinality::ExactlyOne, Cardinality::ZeroOrMany,
                             "One to Many\nOne source record relates to many targets.\nExample: User → many Orders"),
                            ("N : 1", "o<──||", Cardinality::ZeroOrMany, Cardinality::ExactlyOne,
                             "Many to One\nMany source records relate to one target.\nExample: many Orders → one User"),
                            ("M : N", "o<──o<", Cardinality::ZeroOrMany, Cardinality::ZeroOrMany,
                             "Many to Many\nMany on both sides. Needs a junction table.\nExample: Students ↔ Courses"),
                            ("1 : 0..1", "||──o|", Cardinality::ExactlyOne, Cardinality::ZeroOrOne,
                             "One to Optional\nOne source relates to zero or one target.\nExample: User → optional Address"),
                            ("1 : 1..N", "||──|<", Cardinality::ExactlyOne, Cardinality::OneOrMany,
                             "One to One-or-Many\nOne source relates to at least one target.\nExample: Order → one or more Items"),
                        ];

                        for (label, symbol, src, tgt, tooltip) in rel_presets {
                            let is_selected = edge.source_cardinality == *src && edge.target_cardinality == *tgt;
                            let text_color = if is_selected { ACCENT } else { Color32::from_rgb(205, 214, 244) };
                            let bg = if is_selected {
                                Color32::from_rgba_premultiplied(137, 180, 250, 30)
                            } else {
                                Color32::TRANSPARENT
                            };

                            let btn = egui::Button::new(
                                egui::RichText::new(format!("{:<8} {}", label, symbol))
                                    .size(11.0)
                                    .family(egui::FontFamily::Monospace)
                                    .color(text_color)
                            )
                            .fill(bg)
                            .stroke(egui::Stroke::NONE)
                            .min_size(egui::vec2(ui.available_width(), 24.0))
                            .corner_radius(4.0);

                            let resp = ui.add(btn);

                            // Paint hover highlight behind
                            if resp.hovered() && !is_selected {
                                let hover_rect = resp.rect;
                                ui.painter().rect_filled(
                                    hover_rect,
                                    4.0,
                                    Color32::from_rgba_premultiplied(205, 214, 244, 18),
                                );
                            }

                            let clicked = resp.clicked();
                            resp.on_hover_text(*tooltip);
                            if clicked {
                                edge.source_cardinality = *src;
                                edge.target_cardinality = *tgt;
                            }
                        }
                        ui.add_space(8.0);

                        // Text labels (optional, for custom annotations)
                        Self::draw_section_header(ui, "TEXT LABELS");
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Source").size(11.0).color(TEXT_DIM));
                            ui.add(egui::TextEdit::singleline(&mut edge.source_label)
                                .desired_width(60.0)
                                .font(FontId::proportional(11.0)));
                        });
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Target").size(11.0).color(TEXT_DIM));
                            ui.add(egui::TextEdit::singleline(&mut edge.target_label)
                                .desired_width(60.0)
                                .font(FontId::proportional(11.0)));
                        });
                        ui.add_space(12.0);

                        Self::draw_section_header(ui, "STYLE");
                        ui.horizontal(|ui| {
                            let mut c = to_color32(edge.style.color);
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
        self.canvas_rect = canvas_rect;

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
                let factor = (1.0 + scroll_delta * 0.003).clamp(0.9, 1.1);
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
                    // Check resize handles first (highest priority on selected node)
                    if let Some((node_id, handle)) = self.hit_test_resize_handle(mouse) {
                        if let Some(node) = self.document.find_node(&node_id) {
                            self.drag = DragState::ResizingNode {
                                node_id,
                                handle,
                                start_rect: [node.position[0], node.position[1], node.size[0], node.size[1]],
                                start_mouse: canvas_pos,
                            };
                        }
                    }
                    // Check if clicked on a port (for connect mode)
                    else if let Some(port) = self.hit_test_port(canvas_pos) {
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
                                self.selection.toggle_edge(edge_id);
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
                    DragState::ResizingNode {
                        node_id,
                        handle,
                        start_rect,
                        start_mouse,
                    } => {
                        let canvas_mouse = self.viewport.screen_to_canvas(mouse);
                        let delta = canvas_mouse - *start_mouse;
                        let nid = *node_id;
                        let h = *handle;
                        let sr = *start_rect;
                        if let Some(node) = self.document.find_node(&nid) {
                            let min = node.min_size();
                            let [nx, ny, nw, nh] = Self::compute_resize(h, sr, delta, min);
                            if let Some(node) = self.document.find_node_mut(&nid) {
                                node.position = [nx, ny];
                                node.size = [nw, nh];
                            }
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
                    DragState::DraggingNode { .. } | DragState::ResizingNode { .. } => {
                        self.history.push(&self.document);
                    }
                    DragState::CreatingEdge { source, .. } => {
                        let canvas_pos = self.viewport.screen_to_canvas(mouse);
                        if let Some(target) = self.hit_test_port(canvas_pos) {
                            // Don't connect a node to itself on the same port
                            if source.node_id != target.node_id {
                                let edge = Edge::new(*source, target);
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
                                self.selection.node_ids.insert(node.id);
                            }
                        }
                        // Also select edges whose curve passes through the box
                        for edge in &self.document.edges {
                            if self.selection.contains_edge(&edge.id) {
                                continue;
                            }
                            let src_node = self.document.find_node(&edge.source.node_id);
                            let tgt_node = self.document.find_node(&edge.target.node_id);
                            if let (Some(sn), Some(tn)) = (src_node, tgt_node) {
                                let src = sn.port_position(edge.source.side);
                                let tgt = tn.port_position(edge.target.side);
                                let (cp1, cp2) = control_points_for_side(src, tgt, edge.source.side, 60.0);
                                for i in 0..=20 {
                                    let t = i as f32 / 20.0;
                                    let p = cubic_bezier_point(src, cp1, cp2, tgt, t);
                                    if sel_rect.contains(p) {
                                        self.selection.edge_ids.insert(edge.id);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    DragState::DraggingNewNode { kind, current_screen } => {
                        // Drop new node on canvas
                        if canvas_rect.contains(*current_screen) {
                            let mut canvas_pos =
                                self.viewport.screen_to_canvas(*current_screen);
                            if self.snap_to_grid {
                                canvas_pos = self.snap_pos(canvas_pos);
                            }
                            let node = match kind {
                                NodeKind::Shape { shape, .. } => Node::new(*shape, canvas_pos),
                                NodeKind::StickyNote { color, .. } => Node::new_sticky(*color, canvas_pos),
                                NodeKind::Entity { .. } => Node::new_entity(canvas_pos),
                                NodeKind::Text { .. } => Node::new_text(canvas_pos),
                            };
                            self.selection.clear();
                            self.selection.node_ids.insert(node.id);
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

        // Handle double-click on node to focus label editing
        if response.double_clicked() {
            if let Some(mouse) = pointer_pos {
                let canvas_pos = self.viewport.screen_to_canvas(mouse);
                if let Some(node_id) = self.document.node_at_pos(canvas_pos) {
                    self.selection.select_node(node_id);
                    self.focus_label_edit = true;
                }
            }
        }

        // Set cursor based on current state
        let cursor = match &self.drag {
            DragState::Panning { .. } => egui::CursorIcon::Grabbing,
            DragState::DraggingNode { .. } => egui::CursorIcon::Grabbing,
            DragState::DraggingNewNode { .. } => egui::CursorIcon::Copy,
            DragState::CreatingEdge { .. } => egui::CursorIcon::Crosshair,
            DragState::BoxSelect { .. } => egui::CursorIcon::Crosshair,
            DragState::ResizingNode { handle, .. } => Self::resize_cursor(*handle),
            DragState::None => {
                if self.space_held {
                    egui::CursorIcon::Grab
                } else if self.tool == Tool::Connect {
                    egui::CursorIcon::Crosshair
                } else {
                    // Check what's under cursor
                    if let Some(hover) = pointer_pos {
                        // Check resize handles first
                        if let Some((_nid, handle)) = self.hit_test_resize_handle(hover) {
                            Self::resize_cursor(handle)
                        } else {
                            let canvas_pos = self.viewport.screen_to_canvas(hover);
                            if self.hit_test_port(canvas_pos).is_some() {
                                egui::CursorIcon::Crosshair
                            } else if self.document.node_at_pos(canvas_pos).is_some() {
                                egui::CursorIcon::Grab
                            } else if self.hit_test_edge(canvas_pos).is_some() {
                                egui::CursorIcon::PointingHand
                            } else {
                                egui::CursorIcon::Default
                            }
                        }
                    } else {
                        egui::CursorIcon::Default
                    }
                }
            }
        };
        ui.ctx().set_cursor_icon(cursor);

        // Build index once per frame for O(1) node lookups
        let node_idx = self.document.node_index();

        // Get hover position for port visibility optimization
        let hover_pos = ui.ctx().input(|i| i.pointer.hover_pos());

        // ---- Draw edges (only visible ones) ----
        for edge in &self.document.edges {
            // Quick visibility check for edges
            let src_visible = node_idx.get(&edge.source.node_id).and_then(|&i| self.document.nodes.get(i))
                .map(|n| {
                    let sr = Rect::from_min_size(self.viewport.canvas_to_screen(n.pos()), n.size_vec() * self.viewport.zoom);
                    sr.expand(100.0).intersects(canvas_rect)
                }).unwrap_or(false);
            let tgt_visible = node_idx.get(&edge.target.node_id).and_then(|&i| self.document.nodes.get(i))
                .map(|n| {
                    let sr = Rect::from_min_size(self.viewport.canvas_to_screen(n.pos()), n.size_vec() * self.viewport.zoom);
                    sr.expand(100.0).intersects(canvas_rect)
                }).unwrap_or(false);
            if src_visible || tgt_visible {
                self.draw_edge(edge, &painter, &node_idx);
            }
        }

        // ---- Draw nodes (only visible ones) ----
        for node in &self.document.nodes {
            let screen_pos = self.viewport.canvas_to_screen(node.pos());
            let screen_size = node.size_vec() * self.viewport.zoom;
            let screen_rect = Rect::from_min_size(screen_pos, screen_size).expand(20.0);
            if screen_rect.intersects(canvas_rect) {
                self.draw_node(node, &painter, hover_pos);
            }
        }

        // ---- Draw resize handles on single-selected node ----
        if self.selection.node_ids.len() == 1 {
            let sel_id = *self.selection.node_ids.iter().next().unwrap();
            if let Some(node) = self.document.find_node(&sel_id) {
                let top_left = self.viewport.canvas_to_screen(node.pos());
                let size = node.size_vec() * self.viewport.zoom;
                let screen_rect = Rect::from_min_size(top_left, size);
                self.draw_resize_handles(&painter, screen_rect);
            }
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
            if let Some(src_node) = node_idx.get(&source.node_id).and_then(|&i| self.document.nodes.get(i)) {
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

                // Highlight target port if hovering over one
                let canvas_dst = self.viewport.screen_to_canvas(*current_screen);
                if let Some(target_port) = self.hit_test_port(canvas_dst) {
                    if target_port.node_id != source.node_id {
                        // Valid target -- draw highlighted port
                        if let Some(tgt_node) = node_idx.get(&target_port.node_id).and_then(|&i| self.document.nodes.get(i)) {
                            let port_pos = self.viewport.canvas_to_screen(tgt_node.port_position(target_port.side));
                            let r = PORT_RADIUS * self.viewport.zoom.sqrt() * 2.0;
                            painter.circle_filled(port_pos, r * 1.5, Color32::from_rgba_premultiplied(137, 180, 250, 40));
                            painter.circle_filled(port_pos, r, ACCENT);
                            painter.circle_stroke(port_pos, r, Stroke::new(2.0, Color32::WHITE));
                        }
                    }
                }
            }
        }

        // New node preview while dragging from toolbar
        if let DragState::DraggingNewNode {
            kind,
            current_screen,
        } = &self.drag
        {
            if canvas_rect.contains(*current_screen) {
                let preview_size = match kind {
                    NodeKind::Shape { shape, .. } => {
                        let n = Node::new(*shape, Pos2::ZERO);
                        Vec2::new(n.size[0], n.size[1])
                    }
                    NodeKind::StickyNote { .. } => Vec2::new(150.0, 150.0),
                    NodeKind::Entity { .. } => Vec2::new(160.0, 34.0),
                    NodeKind::Text { .. } => Vec2::new(120.0, 40.0),
                };
                let half_w = preview_size.x * 0.5 * self.viewport.zoom;
                let half_h = preview_size.y * 0.5 * self.viewport.zoom;
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

        // ---- Draw status toast ----
        if let Some((ref msg, time)) = self.status_message {
            let elapsed = time.elapsed().as_secs_f32();
            if elapsed < 2.0 {
                let alpha = ((2.0 - elapsed) * 255.0).min(255.0) as u8;
                let toast_pos = Pos2::new(canvas_rect.center().x, canvas_rect.max.y - 40.0);
                painter.text(
                    toast_pos,
                    Align2::CENTER_CENTER,
                    msg,
                    FontId::proportional(12.0),
                    Color32::from_rgba_premultiplied(166, 227, 161, alpha),
                );
                ui.ctx().request_repaint(); // Keep repainting for animation
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
        let clipped = vp_rect.intersect(minimap_rect);
        if clipped.is_positive() {
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

        // Limit grid drawing to prevent jank at low zoom
        let max_dots = 5000;
        let cols = (canvas_rect.width() / grid_screen) as usize;
        let rows = (canvas_rect.height() / grid_screen) as usize;
        if cols * rows > max_dots {
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

    fn draw_node(&self, node: &Node, painter: &egui::Painter, hover_pos: Option<Pos2>) {
        let top_left = self.viewport.canvas_to_screen(node.pos());
        let size = node.size_vec() * self.viewport.zoom;
        let screen_rect = Rect::from_min_size(top_left, size);

        let is_selected = self.selection.contains_node(&node.id);
        let is_hovered = hover_pos.map_or(false, |hp| screen_rect.expand(6.0).contains(hp));

        // Selection glow behind node
        if is_selected {
            let glow_rect = screen_rect.expand(5.0);
            painter.rect_filled(
                glow_rect,
                CornerRadius::same(6),
                Color32::from_rgba_premultiplied(137, 180, 250, 30),
            );
        } else if is_hovered {
            // Hover outline for all node types (drawn here so individual draw methods don't need it)
            painter.rect_stroke(
                screen_rect.expand(2.0),
                CornerRadius::same(4),
                Stroke::new(1.5, Color32::from_rgba_premultiplied(137, 180, 250, 80)),
                StrokeKind::Outside,
            );
        }

        // Dispatch drawing based on node kind
        match &node.kind {
            NodeKind::Shape { shape, label, .. } => {
                self.draw_shape_node(painter, screen_rect, *shape, label, &node.style, is_selected, is_hovered);
            }
            NodeKind::StickyNote { text, .. } => {
                self.draw_sticky_node(painter, screen_rect, text, &node.style, is_selected, is_hovered);
            }
            NodeKind::Entity { name, attributes } => {
                self.draw_entity_node(painter, screen_rect, name, attributes, &node.style, is_selected, is_hovered);
            }
            NodeKind::Text { content } => {
                self.draw_text_node(painter, screen_rect, content, &node.style, is_selected, is_hovered);
            }
        }

        // Draw port circles - only when mouse nearby or in Connect mode
        let show_ports = self.tool == Tool::Connect || {
            if let Some(hover) = hover_pos {
                let expanded = screen_rect.expand(30.0);
                expanded.contains(hover)
            } else {
                false
            }
        };
        if show_ports {
            for side in &ALL_SIDES {
                let canvas_port = node.port_position(*side);
                let screen_port = self.viewport.canvas_to_screen(canvas_port);
                let r = PORT_RADIUS * self.viewport.zoom.sqrt();

                // Check if this specific port is being hovered
                let port_hovered = hover_pos.map_or(false, |hp| (hp - screen_port).length() < r * 3.0);

                if port_hovered {
                    // Highlighted port -- larger, brighter
                    let glow_r = r * 2.5;
                    painter.circle_filled(screen_port, glow_r, Color32::from_rgba_premultiplied(137, 180, 250, 30));
                    painter.circle_filled(screen_port, r * 1.3, ACCENT);
                    painter.circle_stroke(screen_port, r * 1.3, Stroke::new(2.0, Color32::WHITE));
                } else {
                    // Normal port
                    painter.circle_filled(screen_port, r, PORT_FILL);
                    painter.circle_stroke(screen_port, r, Stroke::new(1.5, SELECTION_COLOR));
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Draw Shape Node (flowchart)
    // -----------------------------------------------------------------------

    fn draw_shape_node(
        &self,
        painter: &egui::Painter,
        screen_rect: Rect,
        shape: NodeShape,
        label: &str,
        style: &NodeStyle,
        is_selected: bool,
        _is_hovered: bool,
    ) {
        // Drop shadow
        let shadow_offset = Vec2::new(2.0, 3.0) * self.viewport.zoom;
        let shadow_rect = screen_rect.translate(shadow_offset);
        painter.rect_filled(shadow_rect, CornerRadius::same(4), Color32::from_rgba_premultiplied(0, 0, 0, 40));

        let fill = to_color32(style.fill_color);
        let border_color = if is_selected { SELECTION_COLOR } else { to_color32(style.border_color) };
        let border_width = if is_selected { style.border_width.max(2.5) } else { style.border_width };
        let stroke = Stroke::new(border_width * self.viewport.zoom.sqrt(), border_color);

        match shape {
            NodeShape::Rectangle => {
                painter.rect_filled(screen_rect, CornerRadius::ZERO, fill);
                painter.rect_stroke(screen_rect, CornerRadius::ZERO, stroke, StrokeKind::Outside);
            }
            NodeShape::RoundedRect => {
                let r = (10.0 * self.viewport.zoom) as u8;
                painter.rect_filled(screen_rect, CornerRadius::same(r), fill);
                painter.rect_stroke(screen_rect, CornerRadius::same(r), stroke, StrokeKind::Outside);
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

        // Label
        let text_color = to_color32(style.text_color);
        let font_size = style.font_size * self.viewport.zoom;
        if font_size > 4.0 {
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(font_size),
                text_color,
            );
        }
    }

    // -----------------------------------------------------------------------
    // Draw Sticky Note
    // -----------------------------------------------------------------------

    fn draw_sticky_node(
        &self,
        painter: &egui::Painter,
        screen_rect: Rect,
        text: &str,
        style: &NodeStyle,
        is_selected: bool,
        _is_hovered: bool,
    ) {
        // Shadow (slightly more prominent for sticky notes)
        let shadow_offset = Vec2::new(3.0, 4.0) * self.viewport.zoom;
        let shadow_rect = screen_rect.translate(shadow_offset);
        painter.rect_filled(
            shadow_rect,
            CornerRadius::same(4),
            Color32::from_rgba_premultiplied(0, 0, 0, 50),
        );

        let fill = to_color32(style.fill_color);
        let corner = CornerRadius::same((8.0 * self.viewport.zoom) as u8);
        painter.rect_filled(screen_rect, corner, fill);

        if is_selected {
            painter.rect_stroke(
                screen_rect,
                corner,
                Stroke::new(2.5 * self.viewport.zoom.sqrt(), SELECTION_COLOR),
                StrokeKind::Outside,
            );
        }

        // Multi-line text
        let text_color = to_color32(style.text_color);
        let font_size = style.font_size * self.viewport.zoom;
        if font_size > 4.0 && !text.is_empty() {
            let padding = 10.0 * self.viewport.zoom;
            let text_rect = screen_rect.shrink(padding);
            let galley = painter.layout(
                text.to_string(),
                FontId::proportional(font_size),
                text_color,
                text_rect.width(),
            );
            let text_pos = Pos2::new(text_rect.min.x, text_rect.min.y);
            painter.galley(text_pos, galley, Color32::TRANSPARENT);
        }
    }

    // -----------------------------------------------------------------------
    // Draw Entity Node (ER diagram)
    // -----------------------------------------------------------------------

    fn draw_entity_node(
        &self,
        painter: &egui::Painter,
        screen_rect: Rect,
        name: &str,
        attributes: &[EntityAttribute],
        style: &NodeStyle,
        is_selected: bool,
        _is_hovered: bool,
    ) {
        // Shadow
        let shadow_offset = Vec2::new(2.0, 3.0) * self.viewport.zoom;
        let shadow_rect = screen_rect.translate(shadow_offset);
        painter.rect_filled(shadow_rect, CornerRadius::same(4), Color32::from_rgba_premultiplied(0, 0, 0, 40));

        let fill = to_color32(style.fill_color);
        let border_color = if is_selected { SELECTION_COLOR } else { to_color32(style.border_color) };
        let border_width = if is_selected { style.border_width.max(2.5) } else { style.border_width };
        let stroke = Stroke::new(border_width * self.viewport.zoom.sqrt(), border_color);
        let zoom = self.viewport.zoom;

        // Main body
        painter.rect_filled(screen_rect, CornerRadius::same(3), fill);
        painter.rect_stroke(screen_rect, CornerRadius::same(3), stroke, StrokeKind::Outside);

        // Header background
        let header_h = ENTITY_HEADER_HEIGHT * zoom;
        let header_rect = Rect::from_min_size(
            screen_rect.min,
            Vec2::new(screen_rect.width(), header_h),
        );
        let header_color = to_color32(style.border_color);
        painter.rect_filled(
            header_rect,
            CornerRadius { nw: 3, ne: 3, sw: 0, se: 0 },
            header_color,
        );

        // Header divider line
        let divider_y = screen_rect.min.y + header_h;
        painter.line_segment(
            [Pos2::new(screen_rect.min.x, divider_y), Pos2::new(screen_rect.max.x, divider_y)],
            Stroke::new(1.0, border_color),
        );

        // Entity name
        let font_size = (style.font_size + 1.0) * zoom;
        if font_size > 4.0 {
            painter.text(
                header_rect.center(),
                Align2::CENTER_CENTER,
                name,
                FontId::proportional(font_size),
                Color32::WHITE,
            );
        }

        // Attributes
        let row_h = ENTITY_ROW_HEIGHT * zoom;
        let attr_font = style.font_size * zoom * 0.9;
        let text_color = to_color32(style.text_color);
        let pk_color = ACCENT;
        let fk_color = Color32::from_rgb(249, 226, 175);

        if attr_font > 3.0 {
            for (i, attr) in attributes.iter().enumerate() {
                let row_y = divider_y + (i as f32) * row_h;
                let row_center_y = row_y + row_h / 2.0;

                // Row separator
                if i > 0 {
                    painter.line_segment(
                        [Pos2::new(screen_rect.min.x + 4.0, row_y), Pos2::new(screen_rect.max.x - 4.0, row_y)],
                        Stroke::new(0.5, Color32::from_rgba_premultiplied(100, 100, 100, 60)),
                    );
                }

                let left_x = screen_rect.min.x + 6.0 * zoom;

                // PK/FK indicators
                if attr.is_primary_key {
                    painter.text(
                        Pos2::new(left_x, row_center_y),
                        Align2::LEFT_CENTER,
                        "PK",
                        FontId::monospace(attr_font * 0.7),
                        pk_color,
                    );
                } else if attr.is_foreign_key {
                    painter.text(
                        Pos2::new(left_x, row_center_y),
                        Align2::LEFT_CENTER,
                        "FK",
                        FontId::monospace(attr_font * 0.7),
                        fk_color,
                    );
                }

                // Attribute name
                let name_x = left_x + 22.0 * zoom;
                painter.text(
                    Pos2::new(name_x, row_center_y),
                    Align2::LEFT_CENTER,
                    &attr.name,
                    FontId::proportional(attr_font),
                    text_color,
                );

                // Attribute type (right-aligned)
                let type_x = screen_rect.max.x - 6.0 * zoom;
                painter.text(
                    Pos2::new(type_x, row_center_y),
                    Align2::RIGHT_CENTER,
                    &attr.attr_type,
                    FontId::monospace(attr_font * 0.85),
                    TEXT_DIM,
                );
            }

            // Show placeholder if no attributes
            if attributes.is_empty() {
                let row_center_y = divider_y + row_h / 2.0;
                painter.text(
                    Pos2::new(screen_rect.center().x, row_center_y),
                    Align2::CENTER_CENTER,
                    "no attributes",
                    FontId::proportional(attr_font * 0.85),
                    TEXT_DIM,
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Draw Text Node (freeform)
    // -----------------------------------------------------------------------

    fn draw_text_node(
        &self,
        painter: &egui::Painter,
        screen_rect: Rect,
        content: &str,
        style: &NodeStyle,
        is_selected: bool,
        _is_hovered: bool,
    ) {
        if is_selected {
            painter.rect_stroke(
                screen_rect,
                CornerRadius::same(2),
                Stroke::new(1.5, Color32::from_rgba_premultiplied(137, 180, 250, 100)),
                StrokeKind::Outside,
            );
        }

        // Render text
        let text_color = to_color32(style.text_color);
        let font_size = style.font_size * self.viewport.zoom;
        if font_size > 4.0 && !content.is_empty() {
            let galley = painter.layout(
                content.to_string(),
                FontId::proportional(font_size),
                text_color,
                screen_rect.width(),
            );
            let text_pos = Pos2::new(screen_rect.min.x, screen_rect.min.y);
            painter.galley(text_pos, galley, Color32::TRANSPARENT);
        }
    }

    // -----------------------------------------------------------------------
    // Draw Edge
    // -----------------------------------------------------------------------

    fn draw_edge(&self, edge: &Edge, painter: &egui::Painter, node_idx: &std::collections::HashMap<NodeId, usize>) {
        let src_node = node_idx.get(&edge.source.node_id).and_then(|&i| self.document.nodes.get(i));
        let tgt_node = node_idx.get(&edge.target.node_id).and_then(|&i| self.document.nodes.get(i));
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
            to_color32(edge.style.color)
        };
        let base_width = edge.style.width * self.viewport.zoom.sqrt();
        let width = if is_selected { base_width.max(3.0) } else { base_width };

        let offset = 60.0 * self.viewport.zoom;
        let (cp1, cp2) = control_points_for_side(src, tgt, edge.source.side, offset);

        // Draw selection glow behind the edge
        if is_selected {
            let glow = egui::epaint::CubicBezierShape::from_points_stroke(
                [src, cp1, cp2, tgt],
                false,
                Color32::TRANSPARENT,
                Stroke::new(width + 6.0, Color32::from_rgba_premultiplied(137, 180, 250, 40)),
            );
            painter.add(glow);
        }

        let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
            [src, cp1, cp2, tgt],
            false,
            Color32::TRANSPARENT,
            Stroke::new(width, edge_color),
        );
        painter.add(bezier);

        // Draw endpoints: crow's foot symbols or arrow head
        let has_source_card = edge.source_cardinality != Cardinality::None;
        let has_target_card = edge.target_cardinality != Cardinality::None;

        if has_source_card {
            self.draw_crow_foot(painter, cp1, src, edge.source_cardinality, edge_color, width);
        }
        if has_target_card {
            self.draw_crow_foot(painter, cp2, tgt, edge.target_cardinality, edge_color, width);
        } else {
            // Default arrow head when no cardinality set on target
            self.draw_arrow_head(painter, cp2, tgt, edge_color, width);
        }

        // Edge label at midpoint
        if !edge.label.is_empty() {
            let mid = cubic_bezier_point(src, cp1, cp2, tgt, 0.5);
            let font_size = 12.0 * self.viewport.zoom;
            if font_size > 4.0 {
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

        // Source text label (near source endpoint)
        let card_font_size = 11.0 * self.viewport.zoom;
        if !edge.source_label.is_empty() && card_font_size > 3.0 {
            let near_src = cubic_bezier_point(src, cp1, cp2, tgt, 0.08);
            let lbl_offset = Vec2::new(0.0, -10.0 * self.viewport.zoom);
            painter.text(
                near_src + lbl_offset,
                Align2::CENTER_BOTTOM,
                &edge.source_label,
                FontId::proportional(card_font_size),
                edge_color,
            );
        }

        // Target text label (near target endpoint)
        if !edge.target_label.is_empty() && card_font_size > 3.0 {
            let near_tgt = cubic_bezier_point(src, cp1, cp2, tgt, 0.92);
            let lbl_offset = Vec2::new(0.0, -10.0 * self.viewport.zoom);
            painter.text(
                near_tgt + lbl_offset,
                Align2::CENTER_BOTTOM,
                &edge.target_label,
                FontId::proportional(card_font_size),
                edge_color,
            );
        }
    }

    /// Draw crow's foot cardinality symbol at an edge endpoint.
    /// `from` is the approach direction (control point), `to` is the endpoint on the entity.
    fn draw_crow_foot(
        &self,
        painter: &egui::Painter,
        from: Pos2,
        to: Pos2,
        cardinality: Cardinality,
        color: Color32,
        line_width: f32,
    ) {
        let dir = (to - from).normalized();
        if dir.length() < 0.01 {
            return;
        }
        let perp = Vec2::new(-dir.y, dir.x);
        let zoom = self.viewport.zoom.sqrt();

        let bar_half = 8.0 * zoom;      // half-length of perpendicular bar
        let circle_r = 5.0 * zoom;      // circle radius
        let foot_spread = 8.0 * zoom;   // crow's foot prong spread
        let foot_len = 12.0 * zoom;     // crow's foot prong length

        // Outer = closest to entity (at `to`), Inner = further from entity
        let outer_dist = 3.0 * zoom;
        let inner_dist = 15.0 * zoom;
        let stroke = Stroke::new(line_width.max(1.5 * zoom), color);

        match cardinality {
            Cardinality::None => {}
            Cardinality::ExactlyOne => {
                // || two perpendicular bars
                let outer_pt = to - dir * outer_dist;
                let inner_pt = to - dir * inner_dist;
                painter.line_segment(
                    [outer_pt + perp * bar_half, outer_pt - perp * bar_half],
                    stroke,
                );
                painter.line_segment(
                    [inner_pt + perp * bar_half, inner_pt - perp * bar_half],
                    stroke,
                );
            }
            Cardinality::ZeroOrOne => {
                // o| circle (inner) + bar (outer)
                let outer_pt = to - dir * outer_dist;
                let circle_center = to - dir * (inner_dist + circle_r);
                painter.line_segment(
                    [outer_pt + perp * bar_half, outer_pt - perp * bar_half],
                    stroke,
                );
                painter.circle_stroke(circle_center, circle_r, stroke);
            }
            Cardinality::OneOrMany => {
                // |< bar (inner) + crow's foot (outer)
                let inner_pt = to - dir * inner_dist;
                let convergence = to - dir * foot_len;
                // Crow's foot: three prongs from convergence to entity
                painter.line_segment([convergence, to + perp * foot_spread], stroke);
                painter.line_segment([convergence, to], stroke);
                painter.line_segment([convergence, to - perp * foot_spread], stroke);
                // Inner perpendicular bar
                painter.line_segment(
                    [inner_pt + perp * bar_half, inner_pt - perp * bar_half],
                    stroke,
                );
            }
            Cardinality::ZeroOrMany => {
                // o< circle (inner) + crow's foot (outer)
                let convergence = to - dir * foot_len;
                let circle_center = to - dir * (inner_dist + circle_r);
                // Crow's foot
                painter.line_segment([convergence, to + perp * foot_spread], stroke);
                painter.line_segment([convergence, to], stroke);
                painter.line_segment([convergence, to - perp * foot_spread], stroke);
                // Inner circle
                painter.circle_stroke(circle_center, circle_r, stroke);
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
        // Iterate in reverse so topmost node wins
        for node in self.document.nodes.iter().rev() {
            for side in &ALL_SIDES {
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
        let threshold = 14.0 / self.viewport.zoom;
        for edge in self.document.edges.iter().rev() {
            let src_node = self.document.find_node(&edge.source.node_id);
            let tgt_node = self.document.find_node(&edge.target.node_id);
            if let (Some(sn), Some(tn)) = (src_node, tgt_node) {
                let src = sn.port_position(edge.source.side);
                let tgt = tn.port_position(edge.target.side);
                let offset = 60.0;
                let (cp1, cp2) = control_points_for_side(src, tgt, edge.source.side, offset);
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
    // Zoom Helpers
    // -----------------------------------------------------------------------

    /// Fit viewport to show all content with padding.
    fn fit_to_content(&mut self) {
        if self.document.nodes.is_empty() {
            return;
        }
        self.fit_to_rects(
            self.document.nodes.iter().map(|n| n.rect()).collect(),
        );
    }

    /// Zoom viewport to fit selected nodes.
    fn zoom_to_selection(&mut self) {
        let rects: Vec<Rect> = self.selection.node_ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.rect())
            .collect();
        if rects.is_empty() {
            return;
        }
        self.fit_to_rects(rects);
    }

    /// Fit viewport to show a set of rects with padding.
    fn fit_to_rects(&mut self, rects: Vec<Rect>) {
        if rects.is_empty() {
            return;
        }
        let mut bb = rects[0];
        for r in &rects[1..] {
            bb = bb.union(*r);
        }
        let padding = 40.0;
        bb = bb.expand(padding);

        let canvas_w = self.canvas_rect.width();
        let canvas_h = self.canvas_rect.height();
        let zoom = (canvas_w / bb.width()).min(canvas_h / bb.height()).clamp(0.1, 10.0);

        self.viewport.zoom = zoom;
        self.viewport.offset[0] = self.canvas_rect.min.x + canvas_w / 2.0 - bb.center().x * zoom;
        self.viewport.offset[1] = self.canvas_rect.min.y + canvas_h / 2.0 - bb.center().y * zoom;
    }

    /// Step zoom by a factor, centered on the canvas center.
    fn step_zoom(&mut self, factor: f32) {
        let center = self.canvas_rect.center();
        let old_zoom = self.viewport.zoom;
        self.viewport.zoom = (old_zoom * factor).clamp(0.1, 10.0);
        let ratio = self.viewport.zoom / old_zoom;
        self.viewport.offset[0] = center.x - ratio * (center.x - self.viewport.offset[0]);
        self.viewport.offset[1] = center.y - ratio * (center.y - self.viewport.offset[1]);
    }

    // -----------------------------------------------------------------------
    // Resize Handles
    // -----------------------------------------------------------------------

    /// Returns the 8 resize handle positions (in screen space) for a given screen rect.
    fn resize_handle_positions(screen_rect: Rect) -> [(ResizeHandle, Pos2); 8] {
        [
            (ResizeHandle::TopLeft, screen_rect.left_top()),
            (ResizeHandle::Top, Pos2::new(screen_rect.center().x, screen_rect.min.y)),
            (ResizeHandle::TopRight, screen_rect.right_top()),
            (ResizeHandle::Left, Pos2::new(screen_rect.min.x, screen_rect.center().y)),
            (ResizeHandle::Right, Pos2::new(screen_rect.max.x, screen_rect.center().y)),
            (ResizeHandle::BottomLeft, screen_rect.left_bottom()),
            (ResizeHandle::Bottom, Pos2::new(screen_rect.center().x, screen_rect.max.y)),
            (ResizeHandle::BottomRight, screen_rect.right_bottom()),
        ]
    }

    /// Draw 8 small square resize handles around the selected node's screen rect.
    fn draw_resize_handles(&self, painter: &egui::Painter, screen_rect: Rect) {
        let handle_half = 4.0;
        let handles = Self::resize_handle_positions(screen_rect);
        for (_handle, pos) in &handles {
            let r = Rect::from_center_size(*pos, Vec2::splat(handle_half * 2.0));
            painter.rect_filled(r, CornerRadius::ZERO, SELECTION_COLOR);
            painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, Color32::WHITE), StrokeKind::Outside);
        }
    }

    /// Hit test resize handles: checks if cursor (screen pos) is within ~6px of any handle.
    /// Only checks the single selected node.
    fn hit_test_resize_handle(&self, screen_pos: Pos2) -> Option<(NodeId, ResizeHandle)> {
        if self.selection.node_ids.len() != 1 {
            return None;
        }
        let node_id = *self.selection.node_ids.iter().next().unwrap();
        let node = self.document.find_node(&node_id)?;
        let top_left = self.viewport.canvas_to_screen(node.pos());
        let size = node.size_vec() * self.viewport.zoom;
        let screen_rect = Rect::from_min_size(top_left, size);
        let handles = Self::resize_handle_positions(screen_rect);
        let threshold = 6.0;
        for (handle, pos) in &handles {
            if (screen_pos - *pos).length() < threshold {
                return Some((node_id, *handle));
            }
        }
        None
    }

    /// Returns the appropriate resize cursor for a handle.
    fn resize_cursor(handle: ResizeHandle) -> egui::CursorIcon {
        match handle {
            ResizeHandle::TopLeft | ResizeHandle::BottomRight => egui::CursorIcon::ResizeNwSe,
            ResizeHandle::TopRight | ResizeHandle::BottomLeft => egui::CursorIcon::ResizeNeSw,
            ResizeHandle::Left | ResizeHandle::Right => egui::CursorIcon::ResizeHorizontal,
            ResizeHandle::Top | ResizeHandle::Bottom => egui::CursorIcon::ResizeVertical,
        }
    }

    /// Apply resize logic: given the handle, original rect, and mouse delta, compute new [x, y, w, h].
    fn compute_resize(
        handle: ResizeHandle,
        start_rect: [f32; 4],
        delta: Vec2,
        min_size: [f32; 2],
    ) -> [f32; 4] {
        let [sx, sy, sw, sh] = start_rect;
        let [min_w, min_h] = min_size;
        let (mut x, mut y, mut w, mut h) = (sx, sy, sw, sh);

        match handle {
            ResizeHandle::Right | ResizeHandle::TopRight | ResizeHandle::BottomRight => {
                w = (sw + delta.x).max(min_w);
            }
            ResizeHandle::Left | ResizeHandle::TopLeft | ResizeHandle::BottomLeft => {
                let new_w = (sw - delta.x).max(min_w);
                x = sx + sw - new_w;
                w = new_w;
            }
            _ => {}
        }

        match handle {
            ResizeHandle::Bottom | ResizeHandle::BottomLeft | ResizeHandle::BottomRight => {
                h = (sh + delta.y).max(min_h);
            }
            ResizeHandle::Top | ResizeHandle::TopLeft | ResizeHandle::TopRight => {
                let new_h = (sh - delta.y).max(min_h);
                y = sy + sh - new_h;
                h = new_h;
            }
            _ => {}
        }

        // Edge-only handles: don't change the other axis
        match handle {
            ResizeHandle::Left | ResizeHandle::Right => {
                y = sy;
                h = sh;
            }
            ResizeHandle::Top | ResizeHandle::Bottom => {
                x = sx;
                w = sw;
            }
            _ => {}
        }

        [x, y, w, h]
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
        // Cleanup expired status messages
        if let Some((_, time)) = &self.status_message {
            if time.elapsed().as_secs_f32() > 2.5 {
                self.status_message = None;
            }
        }

        self.handle_shortcuts(ctx);

        // Only repaint continuously during drag operations or active toasts
        let has_active_toast = self.status_message.as_ref().map_or(false, |(_, t)| t.elapsed().as_secs_f32() < 2.5);
        match self.drag {
            DragState::None if !has_active_toast => {
                // Idle: repaint at low rate for cursor changes
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
            _ => {
                // Active drag or toast animation: repaint continuously
                ctx.request_repaint();
            }
        }

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    // -- compute_resize tests --

    #[test]
    fn resize_bottom_right_grows() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(30.0, 20.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] = FlowchartApp::compute_resize(ResizeHandle::BottomRight, start, delta, min);
        assert_eq!(x, 100.0); // position unchanged
        assert_eq!(y, 100.0);
        assert_eq!(w, 170.0); // 140 + 30
        assert_eq!(h, 80.0);  // 60 + 20
    }

    #[test]
    fn resize_top_left_grows() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(-20.0, -10.0); // drag up-left
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] = FlowchartApp::compute_resize(ResizeHandle::TopLeft, start, delta, min);
        assert_eq!(x, 80.0);  // moved left by 20
        assert_eq!(y, 90.0);  // moved up by 10
        assert_eq!(w, 160.0); // 140 + 20
        assert_eq!(h, 70.0);  // 60 + 10
    }

    #[test]
    fn resize_right_only_changes_width() {
        let start = [100.0, 200.0, 140.0, 60.0];
        let delta = Vec2::new(50.0, 999.0); // y should be ignored
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] = FlowchartApp::compute_resize(ResizeHandle::Right, start, delta, min);
        assert_eq!(x, 100.0);
        assert_eq!(y, 200.0);
        assert_eq!(w, 190.0);
        assert_eq!(h, 60.0); // unchanged
    }

    #[test]
    fn resize_bottom_only_changes_height() {
        let start = [100.0, 200.0, 140.0, 60.0];
        let delta = Vec2::new(999.0, 40.0); // x should be ignored
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] = FlowchartApp::compute_resize(ResizeHandle::Bottom, start, delta, min);
        assert_eq!(x, 100.0);
        assert_eq!(y, 200.0);
        assert_eq!(w, 140.0); // unchanged
        assert_eq!(h, 100.0);
    }

    #[test]
    fn resize_clamps_to_min_size_shape() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(-200.0, -200.0); // try to shrink way below min
        let min = MIN_SIZE_SHAPE; // [40, 30]
        let [x, y, w, h] = FlowchartApp::compute_resize(ResizeHandle::BottomRight, start, delta, min);
        assert_eq!(x, 100.0);
        assert_eq!(y, 100.0);
        assert_eq!(w, 40.0);  // clamped to min
        assert_eq!(h, 30.0);  // clamped to min
    }

    #[test]
    fn resize_clamps_to_min_size_sticky() {
        let start = [50.0, 50.0, 150.0, 150.0];
        let delta = Vec2::new(-200.0, -200.0);
        let min = MIN_SIZE_STICKY; // [60, 60]
        let [_x, _y, w, h] = FlowchartApp::compute_resize(ResizeHandle::BottomRight, start, delta, min);
        assert_eq!(w, 60.0);
        assert_eq!(h, 60.0);
    }

    #[test]
    fn resize_top_left_clamps_adjusts_position() {
        // When dragging top-left to shrink, position should move to maintain bottom-right corner
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(200.0, 200.0); // drag far right/down => shrink
        let min = MIN_SIZE_SHAPE; // [40, 30]
        let [x, y, w, h] = FlowchartApp::compute_resize(ResizeHandle::TopLeft, start, delta, min);
        assert_eq!(w, 40.0);  // clamped
        assert_eq!(h, 30.0);  // clamped
        assert_eq!(x, 200.0); // 100 + 140 - 40
        assert_eq!(y, 130.0); // 100 + 60 - 30
    }

    #[test]
    fn resize_left_moves_x_keeps_right_edge() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(-30.0, 0.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] = FlowchartApp::compute_resize(ResizeHandle::Left, start, delta, min);
        assert_eq!(x, 70.0);   // moved left
        assert_eq!(w, 170.0);  // grew
        assert_eq!(y, 100.0);  // unchanged
        assert_eq!(h, 60.0);   // unchanged
    }

    #[test]
    fn resize_top_moves_y_keeps_bottom_edge() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(0.0, -25.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] = FlowchartApp::compute_resize(ResizeHandle::Top, start, delta, min);
        assert_eq!(x, 100.0);  // unchanged
        assert_eq!(w, 140.0);  // unchanged
        assert_eq!(y, 75.0);   // moved up
        assert_eq!(h, 85.0);   // grew
    }

    // -- resize_handle_positions tests --

    #[test]
    fn handle_positions_are_correct() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 200.0), Vec2::new(200.0, 100.0));
        let handles = FlowchartApp::resize_handle_positions(rect);

        // TopLeft
        assert_eq!(handles[0].0, ResizeHandle::TopLeft);
        assert_eq!(handles[0].1, Pos2::new(100.0, 200.0));

        // Top (center top)
        assert_eq!(handles[1].0, ResizeHandle::Top);
        assert_eq!(handles[1].1, Pos2::new(200.0, 200.0));

        // TopRight
        assert_eq!(handles[2].0, ResizeHandle::TopRight);
        assert_eq!(handles[2].1, Pos2::new(300.0, 200.0));

        // Left (center left)
        assert_eq!(handles[3].0, ResizeHandle::Left);
        assert_eq!(handles[3].1, Pos2::new(100.0, 250.0));

        // Right (center right)
        assert_eq!(handles[4].0, ResizeHandle::Right);
        assert_eq!(handles[4].1, Pos2::new(300.0, 250.0));

        // BottomLeft
        assert_eq!(handles[5].0, ResizeHandle::BottomLeft);
        assert_eq!(handles[5].1, Pos2::new(100.0, 300.0));

        // Bottom (center bottom)
        assert_eq!(handles[6].0, ResizeHandle::Bottom);
        assert_eq!(handles[6].1, Pos2::new(200.0, 300.0));

        // BottomRight
        assert_eq!(handles[7].0, ResizeHandle::BottomRight);
        assert_eq!(handles[7].1, Pos2::new(300.0, 300.0));
    }

    // -- resize cursor tests --

    #[test]
    fn resize_cursors_are_correct() {
        assert_eq!(FlowchartApp::resize_cursor(ResizeHandle::TopLeft), egui::CursorIcon::ResizeNwSe);
        assert_eq!(FlowchartApp::resize_cursor(ResizeHandle::BottomRight), egui::CursorIcon::ResizeNwSe);
        assert_eq!(FlowchartApp::resize_cursor(ResizeHandle::TopRight), egui::CursorIcon::ResizeNeSw);
        assert_eq!(FlowchartApp::resize_cursor(ResizeHandle::BottomLeft), egui::CursorIcon::ResizeNeSw);
        assert_eq!(FlowchartApp::resize_cursor(ResizeHandle::Left), egui::CursorIcon::ResizeHorizontal);
        assert_eq!(FlowchartApp::resize_cursor(ResizeHandle::Right), egui::CursorIcon::ResizeHorizontal);
        assert_eq!(FlowchartApp::resize_cursor(ResizeHandle::Top), egui::CursorIcon::ResizeVertical);
        assert_eq!(FlowchartApp::resize_cursor(ResizeHandle::Bottom), egui::CursorIcon::ResizeVertical);
    }

    // -- Node min_size tests --

    #[test]
    fn node_min_sizes_are_correct() {
        let shape_node = Node::new(NodeShape::Rectangle, Pos2::new(0.0, 0.0));
        assert_eq!(shape_node.min_size(), MIN_SIZE_SHAPE);

        let sticky_node = Node::new_sticky(StickyColor::Yellow, Pos2::new(0.0, 0.0));
        assert_eq!(sticky_node.min_size(), MIN_SIZE_STICKY);

        let entity_node = Node::new_entity(Pos2::new(0.0, 0.0));
        assert_eq!(entity_node.min_size(), MIN_SIZE_ENTITY);

        let text_node = Node::new_text(Pos2::new(0.0, 0.0));
        assert_eq!(text_node.min_size(), MIN_SIZE_TEXT);
    }

    // -- All 8 handles resize correctly (corner + edge) --

    #[test]
    fn resize_top_right_grows() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(20.0, -15.0); // right + up
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] = FlowchartApp::compute_resize(ResizeHandle::TopRight, start, delta, min);
        assert_eq!(x, 100.0);  // unchanged (right edge moves)
        assert_eq!(y, 85.0);   // moved up
        assert_eq!(w, 160.0);  // grew right
        assert_eq!(h, 75.0);   // grew up
    }

    #[test]
    fn resize_bottom_left_grows() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(-20.0, 15.0); // left + down
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] = FlowchartApp::compute_resize(ResizeHandle::BottomLeft, start, delta, min);
        assert_eq!(x, 80.0);   // moved left
        assert_eq!(y, 100.0);  // unchanged (bottom edge moves)
        assert_eq!(w, 160.0);  // grew left
        assert_eq!(h, 75.0);   // grew down
    }

    // -- Entity min size enforced --

    #[test]
    fn resize_entity_respects_min() {
        let start = [0.0, 0.0, 200.0, 100.0];
        let delta = Vec2::new(-300.0, -300.0);
        let min = MIN_SIZE_ENTITY; // [160, 52]
        let [_x, _y, w, h] = FlowchartApp::compute_resize(ResizeHandle::BottomRight, start, delta, min);
        assert_eq!(w, 160.0);
        assert_eq!(h, MIN_SIZE_ENTITY[1]);
    }
}
