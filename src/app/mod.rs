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
    }
}
