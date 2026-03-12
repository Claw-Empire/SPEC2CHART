//! Bottom status bar — shows tool mode, zoom, selection count, and canvas cursor position.

use egui::{Align, Color32, Frame, Layout, Margin, RichText, TopBottomPanel};

use super::{DiagramMode, FlowchartApp, Tool};
use crate::app::theme::{ACCENT, MANTLE, SURFACE1, TEXT_DIM, TEXT_PRIMARY, TEXT_SECONDARY};

impl FlowchartApp {
    pub(crate) fn draw_status_bar(&mut self, ctx: &egui::Context) {
        if self.presentation_mode {
            return;
        }

        let zoom_pct = (self.viewport.zoom * 100.0).round() as i32;
        let sel_count = self.selection.node_ids.len();
        let edge_sel = !self.selection.edge_ids.is_empty();

        // Compute canvas cursor position from raw pointer
        let cursor_canvas = ctx.input(|i| i.pointer.hover_pos()).map(|sp| {
            let cp = self.viewport.screen_to_canvas(sp);
            (cp.x.round() as i32, cp.y.round() as i32)
        });

        let tool_label = match self.tool {
            Tool::Select  => "Select",
            Tool::Connect => "Connect",
        };

        let mode_label = match self.diagram_mode {
            DiagramMode::Flowchart => "Flowchart",
            DiagramMode::ER        => "ER",
            DiagramMode::FigJam    => "FigJam",
        };

        TopBottomPanel::bottom("status_bar")
            .exact_height(22.0)
            .frame(
                Frame::NONE
                    .fill(MANTLE)
                    .inner_margin(Margin { left: 12, right: 12, top: 0, bottom: 0 })
                    .stroke(egui::Stroke::new(0.5, SURFACE1)),
            )
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    // Tool
                    pill(ui, tool_label, ACCENT);
                    ui.add_space(8.0);
                    // Mode
                    separator(ui);
                    ui.add_space(8.0);
                    label(ui, mode_label, TEXT_SECONDARY);

                    // Selection info
                    if sel_count > 0 || edge_sel {
                        ui.add_space(8.0);
                        separator(ui);
                        ui.add_space(8.0);
                        let sel_text = if sel_count > 0 && edge_sel {
                            format!("{} node{} + 1 edge", sel_count, if sel_count == 1 { "" } else { "s" })
                        } else if sel_count > 0 {
                            format!("{} node{}", sel_count, if sel_count == 1 { "" } else { "s" })
                        } else {
                            "1 edge".to_string()
                        };
                        label(ui, &sel_text, TEXT_PRIMARY);
                    }

                    // Right side — zoom + cursor
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Cursor coords
                        if let Some((cx, cy)) = cursor_canvas {
                            label(ui, &format!("{cx}, {cy}"), TEXT_DIM);
                            ui.add_space(4.0);
                            label(ui, "↗", TEXT_DIM);
                            ui.add_space(8.0);
                        }
                        separator(ui);
                        ui.add_space(8.0);
                        // Zoom
                        let zoom_text = format!("{zoom_pct}%");
                        label(ui, &zoom_text, if zoom_pct == 100 { TEXT_SECONDARY } else { ACCENT });
                    });
                });
            });
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn label(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).size(11.0).color(color));
}

fn separator(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 12.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, SURFACE1);
}

fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    let galley = ui.fonts(|f| {
        f.layout_no_wrap(
            text.to_string(),
            egui::FontId::proportional(10.0),
            color,
        )
    });
    let size = galley.size() + egui::vec2(8.0, 2.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(3), Color32::from_rgba_premultiplied(137, 180, 250, 18));
    ui.painter().galley(
        egui::pos2(rect.min.x + 4.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
}
