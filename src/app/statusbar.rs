//! Bottom status bar — shows tool mode, zoom, selection count, and canvas cursor position.

use egui::{Align, Color32, Frame, Layout, Margin, RichText, TopBottomPanel};

use super::{DiagramMode, FlowchartApp, Tool, ViewMode};

impl FlowchartApp {
    pub(crate) fn draw_status_bar(&mut self, ctx: &egui::Context) {
        if self.presentation_mode {
            return;
        }

        let accent = self.theme.accent;
        let mantle = self.theme.mantle;
        let surface1 = self.theme.surface1;
        let text_dim = self.theme.text_dim;
        let text_primary = self.theme.text_primary;
        let text_secondary = self.theme.text_secondary;
        let accent_glow = self.theme.accent_glow;

        let zoom_pct = (self.effective_zoom() * 100.0).round() as i32;
        let sel_count = self.selection.node_ids.len();
        let edge_sel = !self.selection.edge_ids.is_empty();
        let total_nodes = self.document.nodes.len();
        let total_edges = self.document.edges.len();
        let tag_active = self.tag_filter.is_some();
        let undo_steps = self.history.undo_steps();
        let redo_steps = self.history.redo_steps();

        // Kanban metrics: computed when document has sections
        let has_sections = self.document.nodes.iter().any(|n| !n.section_name.is_empty());
        let kanban_metrics = if has_sections {
            let today = super::render::today_iso();
            let mut open = 0u32;
            let mut overdue = 0u32;
            let mut done = 0u32;
            let mut fire = 0u32; // P1 (priority==1 or Critical tag) — not done
            for n in &self.document.nodes {
                if n.is_frame { continue; }
                let is_done = matches!(n.tag, Some(crate::model::NodeTag::Ok)) || n.progress >= 1.0;
                if is_done { done += 1; continue; }
                open += 1;
                let is_overdue = n.sublabel.split('\n').any(|line| {
                    if let Some(ds) = line.strip_prefix("📅 ") {
                        let d = ds.trim();
                        d.len() >= 8 && d < today.as_str()
                    } else { false }
                });
                if is_overdue { overdue += 1; }
                // P1 = priority field or Critical tag
                if n.priority == 1 || matches!(n.tag, Some(crate::model::NodeTag::Critical)) {
                    fire += 1;
                }
            }
            Some((open, overdue, done, fire))
        } else {
            None
        };

        // Compute canvas cursor position from raw pointer
        let cursor_canvas = ctx.input(|i| i.pointer.hover_pos()).map(|sp| {
            let cp = self.viewport.screen_to_canvas(sp);
            (cp.x.round() as i32, cp.y.round() as i32)
        });

        let tool_label = match self.tool {
            Tool::Select  => "Select",
            Tool::Connect => "Connect",
        };
        let tool_hint: Option<&str> = match self.tool {
            Tool::Connect => Some("drag port → port to connect"),
            Tool::Select if sel_count == 0 => Some("click node or box-select"),
            _ => None,
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
                    .fill(mantle)
                    .inner_margin(Margin { left: 12, right: 12, top: 0, bottom: 0 })
                    .stroke(egui::Stroke::new(0.5, surface1)),
            )
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    // Tool + hint
                    pill(ui, tool_label, accent, accent_glow);
                    if let Some(hint) = tool_hint {
                        ui.add_space(4.0);
                        label(ui, hint, text_dim);
                    }
                    ui.add_space(8.0);
                    // Mode
                    separator(ui, surface1);
                    ui.add_space(8.0);
                    label(ui, mode_label, text_secondary);
                    // Focus mode indicator
                    if self.focus_mode {
                        ui.add_space(4.0);
                        pill(ui, "FOCUS", Color32::from_rgb(203, 166, 247), Color32::from_rgba_unmultiplied(203, 166, 247, 40));
                    }
                    // 3D indicator
                    if matches!(self.view_mode, ViewMode::ThreeD) {
                        ui.add_space(6.0);
                        let cam_yaw = self.camera3d.yaw.to_degrees().round() as i32;
                        let cam_pitch = self.camera3d.pitch.to_degrees().round() as i32;
                        pill(ui, &format!("3D  {}° {}°", cam_yaw, cam_pitch),
                            self.theme.accent, self.theme.accent_glow);
                    }

                    // Selection info
                    if sel_count > 0 || edge_sel {
                        ui.add_space(8.0);
                        separator(ui, surface1);
                        ui.add_space(8.0);
                        let sel_text = if sel_count > 0 && edge_sel {
                            format!("{} node{} + 1 edge", sel_count, if sel_count == 1 { "" } else { "s" })
                        } else if sel_count > 0 {
                            format!("{} node{}", sel_count, if sel_count == 1 { "" } else { "s" })
                        } else {
                            "1 edge".to_string()
                        };
                        label(ui, &sel_text, text_primary);
                        // Show endpoints for selected edge
                        if sel_count == 0 && edge_sel {
                            if let Some(eid) = self.selection.edge_ids.iter().next() {
                                if let Some(edge) = self.document.find_edge(eid) {
                                    let src_name = self.document.find_node(&edge.source.node_id)
                                        .map(|n| n.display_label().to_string())
                                        .unwrap_or_default();
                                    let tgt_name = self.document.find_node(&edge.target.node_id)
                                        .map(|n| n.display_label().to_string())
                                        .unwrap_or_default();
                                    if !src_name.is_empty() || !tgt_name.is_empty() {
                                        ui.add_space(6.0);
                                        label(ui, &format!("{} → {}", src_name, tgt_name), text_dim);
                                    }
                                }
                            }
                        }

                        // Geometry info for selected node(s)
                        if sel_count == 1 {
                            if let Some(node) = self.selection.node_ids.iter().next()
                                .and_then(|id| self.document.find_node(id))
                            {
                                let pos = node.pos();
                                ui.add_space(6.0);
                                let pos_text = format!("({},{}) {}×{}", pos.x.round() as i32, pos.y.round() as i32, node.size[0].round() as i32, node.size[1].round() as i32);
                                label(ui, &pos_text, text_dim);
                                // 3D layer info
                                if node.z_offset != 0.0 {
                                    ui.add_space(4.0);
                                    label(ui, &format!("z:{}", node.z_offset.round() as i32), accent.gamma_multiply(0.7));
                                }
                                // Section info
                                if !node.section_name.is_empty() {
                                    ui.add_space(6.0);
                                    label(ui, &format!("§ {}", node.section_name), accent.gamma_multiply(0.6));
                                }
                                // Progress info
                                if node.progress > 0.0 {
                                    ui.add_space(4.0);
                                    let pct = (node.progress * 100.0).round() as u32;
                                    label(ui, &format!("▓ {}%", pct), text_dim);
                                }
                            }
                        } else if sel_count > 1 {
                            // Bounding box of selection
                            let mut bb_min = egui::pos2(f32::MAX, f32::MAX);
                            let mut bb_max = egui::pos2(f32::MIN, f32::MIN);
                            for id in &self.selection.node_ids {
                                if let Some(n) = self.document.find_node(id) {
                                    let r = n.rect();
                                    bb_min.x = bb_min.x.min(r.min.x);
                                    bb_min.y = bb_min.y.min(r.min.y);
                                    bb_max.x = bb_max.x.max(r.max.x);
                                    bb_max.y = bb_max.y.max(r.max.y);
                                }
                            }
                            if bb_min.x < f32::MAX {
                                ui.add_space(6.0);
                                let bw = (bb_max.x - bb_min.x).round() as i32;
                                let bh = (bb_max.y - bb_min.y).round() as i32;
                                label(ui, &format!("bbox {}×{}", bw, bh), text_dim);
                            }
                            // Distance between exactly 2 selected nodes
                            if sel_count == 2 {
                                let ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
                                if let (Some(n1), Some(n2)) = (
                                    self.document.find_node(&ids[0]),
                                    self.document.find_node(&ids[1]),
                                ) {
                                    let c1 = n1.rect().center();
                                    let c2 = n2.rect().center();
                                    let dist = ((c2.x - c1.x).powi(2) + (c2.y - c1.y).powi(2)).sqrt();
                                    ui.add_space(6.0);
                                    label(ui, &format!("↔ {:.0}", dist), accent.gamma_multiply(0.8));
                                }
                            }
                        }
                    }

                    // Hovered node info (when nothing selected)
                    if sel_count == 0 && !edge_sel {
                        if let Some(hid) = self.hover_node_id {
                            if let Some(node) = self.document.find_node(&hid) {
                                let ins  = self.document.edges.iter().filter(|e| e.target.node_id == hid).count();
                                let outs = self.document.edges.iter().filter(|e| e.source.node_id == hid).count();
                                ui.add_space(8.0);
                                separator(ui, surface1);
                                ui.add_space(8.0);
                                // Truncate long labels
                                let lbl = node.display_label();
                                let short = if lbl.len() > 28 { format!("{}…", &lbl[..27]) } else { lbl.to_string() };
                                label(ui, &short, text_primary);
                                if ins > 0 || outs > 0 {
                                    ui.add_space(5.0);
                                    label(ui, &format!("↑{} ↓{}", ins, outs), text_dim);
                                }
                                if node.highlight {
                                    ui.add_space(4.0);
                                    label(ui, "⭐", Color32::from_rgb(255, 200, 50));
                                }
                            }
                        }
                    }

                    // Right side — graph stats, zoom, cursor
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(ref msg) = self.autosave_status {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(msg)
                                    .size(11.0)
                                    .color(Color32::from_gray(120)),
                            );
                            ui.add_space(4.0);
                            separator(ui, surface1);
                        }
                        // Cursor coords
                        if let Some((cx, cy)) = cursor_canvas {
                            label(ui, &format!("{cx}, {cy}"), text_dim);
                            ui.add_space(4.0);
                            label(ui, "↗", text_dim);
                            ui.add_space(8.0);
                        }
                        separator(ui, surface1);
                        ui.add_space(8.0);
                        // Zoom — clickable button: single click resets to 100%
                        let zoom_text = format!("{zoom_pct}%");
                        let zoom_color = if zoom_pct == 100 { text_secondary } else { accent };
                        let zoom_resp = ui.add(
                            egui::Button::new(RichText::new(&zoom_text).size(11.0).color(zoom_color))
                                .frame(false)
                                .min_size(egui::vec2(0.0, 0.0))
                        ).on_hover_text("Click to reset zoom to 100%");
                        if zoom_resp.clicked() {
                            self.viewport.zoom = 1.0;
                        }
                        ui.add_space(8.0);
                        separator(ui, surface1);
                        ui.add_space(8.0);
                        // Kanban metrics (right-to-left order)
                        if let Some((open, overdue, done, fire)) = kanban_metrics {
                            ui.add_space(8.0);
                            separator(ui, surface1);
                            ui.add_space(8.0);
                            // Show from right: done | overdue | P1 | open
                            if done > 0 {
                                label(ui, &format!("✓{done}"), Color32::from_rgb(166, 227, 161));
                                ui.add_space(5.0);
                            }
                            if fire > 0 {
                                // Pulsing red for P1 tickets
                                let t = ctx.input(|i| i.time) as f32;
                                let pulse = (t * std::f32::consts::PI / 1.5).sin() * 0.5 + 0.5;
                                let r = (220.0 + pulse * 35.0) as u8;
                                label(ui, &format!("P1:{fire}"), Color32::from_rgb(r, 70, 70));
                                ui.add_space(4.0);
                            }
                            if overdue > 0 {
                                label(ui, &format!("⚠{overdue}"), Color32::from_rgb(243, 139, 168));
                                ui.add_space(4.0);
                            }
                            label(ui, &format!("{open} open"), text_secondary);
                        }

                        // Graph totals (right-to-left, so reversed order)
                        label(ui, &format!("{total_edges}e  {total_nodes}n"), text_dim);
                        if tag_active {
                            ui.add_space(4.0);
                            label(ui, "🏷", accent);
                        }
                        // Undo/redo depth indicator
                        if undo_steps > 0 || redo_steps > 0 {
                            ui.add_space(8.0);
                            separator(ui, surface1);
                            ui.add_space(8.0);
                            let has_history = undo_steps > 0 || redo_steps > 0;
                            let hist_col = if has_history { text_dim } else { text_dim.gamma_multiply(0.4) };
                            label(ui, &format!("↺{} ↻{}", undo_steps, redo_steps), hist_col);
                        }
                    });
                });
            });
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn label(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).size(11.0).color(color));
}

fn separator(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 12.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, color);
}

fn pill(ui: &mut egui::Ui, text: &str, color: Color32, bg: Color32) {
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
        .rect_filled(rect, egui::CornerRadius::same(3), bg);
    ui.painter().galley(
        egui::pos2(rect.min.x + 4.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
}
