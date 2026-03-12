use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};
use crate::model::*;
use super::{FlowchartApp, DragState, Tool};
use super::interaction::{control_points_for_side, cubic_bezier_point};
use super::theme::*;

impl FlowchartApp {
    pub(crate) fn draw_canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::all());
        let canvas_rect = response.rect;
        self.canvas_rect = canvas_rect;

        let bg = Color32::from_rgba_unmultiplied(
            self.canvas_bg[0], self.canvas_bg[1], self.canvas_bg[2], self.canvas_bg[3],
        );
        painter.rect_filled(canvas_rect, CornerRadius::ZERO, bg);

        if self.show_grid && self.bg_pattern != super::BgPattern::None {
            self.draw_grid(&painter, canvas_rect);
        }

        // --- Input handling ---
        let pointer_pos = response
            .hover_pos()
            .or_else(|| ui.ctx().input(|i| i.pointer.hover_pos()));

        // Scroll => pan (infinite canvas), Pinch/Cmd+scroll => zoom
        let (raw_scroll, smooth_scroll, pinch_zoom, cmd_held) = ui.ctx().input(|i| {
            (
                i.raw_scroll_delta,
                i.smooth_scroll_delta,
                i.zoom_delta(),
                i.modifiers.command,
            )
        });
        let scroll = if raw_scroll.length() > 0.0 { raw_scroll } else { smooth_scroll };

        // Pinch-to-zoom or Cmd+scroll => zoom towards mouse
        let zoom_scroll = if cmd_held { scroll.y } else { 0.0 };
        if pinch_zoom != 1.0 || zoom_scroll != 0.0 {
            if let Some(mouse) = pointer_pos {
                let old_zoom = self.viewport.zoom;
                let factor = if pinch_zoom != 1.0 {
                    pinch_zoom
                } else {
                    (1.0 + zoom_scroll * 0.003).clamp(0.9, 1.1)
                };
                self.viewport.zoom = (self.viewport.zoom * factor).clamp(0.1, 10.0);
                let ratio = self.viewport.zoom / old_zoom;
                self.viewport.offset[0] = mouse.x - ratio * (mouse.x - self.viewport.offset[0]);
                self.viewport.offset[1] = mouse.y - ratio * (mouse.y - self.viewport.offset[1]);
            }
        }

        // Advance animated layout transition each frame
        let dt = ui.ctx().input(|i| i.stable_dt).clamp(0.001, 0.1);
        if !self.layout_targets.is_empty() {
            let ctx_clone = ui.ctx().clone();
            self.step_layout_animation(dt, &ctx_clone);
        }

        // Regular scroll => pan canvas (with inertia accumulation)
        if !cmd_held && scroll.length() > 0.0 {
            // User is scrolling — cancel any in-progress fly-to animation
            self.pan_target = None;
            // Add scroll to velocity (weighted by frame time for consistent feel)
            self.pan_velocity[0] += scroll.x * 0.6;
            self.pan_velocity[1] += scroll.y * 0.6;
            self.viewport.offset[0] += scroll.x;
            self.viewport.offset[1] += scroll.y;
        }

        // Apply pan inertia (decay velocity each frame)
        let friction = 0.82_f32;
        if self.pan_velocity[0].abs() > 0.3 || self.pan_velocity[1].abs() > 0.3 {
            self.pan_velocity[0] *= friction;
            self.pan_velocity[1] *= friction;
            self.viewport.offset[0] += self.pan_velocity[0];
            self.viewport.offset[1] += self.pan_velocity[1];
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        } else {
            self.pan_velocity = [0.0, 0.0];
        }

        // Smooth zoom interpolation toward zoom_target
        let zoom_diff = self.zoom_target - self.viewport.zoom;
        if zoom_diff.abs() > 0.001 {
            let lerp_speed = 1.0 - 0.85_f32.powf(dt * 60.0);
            self.viewport.zoom += zoom_diff * lerp_speed;
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        } else {
            self.zoom_target = self.viewport.zoom;
        }

        // Smooth pan interpolation toward pan_target (used by fit-to-content / minimap fly-to)
        if let Some(target) = self.pan_target {
            let lerp = 1.0 - 0.80_f32.powf(dt * 60.0);
            let dx = target[0] - self.viewport.offset[0];
            let dy = target[1] - self.viewport.offset[1];
            if dx.abs() > 0.5 || dy.abs() > 0.5 {
                self.viewport.offset[0] += dx * lerp;
                self.viewport.offset[1] += dy * lerp;
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
            } else {
                self.viewport.offset[0] = target[0];
                self.viewport.offset[1] = target[1];
                self.pan_target = None;
            }
        }

        self.handle_drag_start(&response, ui, pointer_pos);
        self.handle_dragging(&response, ui, pointer_pos);
        self.handle_drag_end(&response, ui, pointer_pos, canvas_rect);

        // Click (non-drag) for selection / deselection
        if response.clicked() {
            if let Some(mouse) = pointer_pos {
                let canvas_pos = self.viewport.screen_to_canvas(mouse);
                let cmd_held = ui.ctx().input(|i| i.modifiers.command);

                if let Some(node_id) = self.document.node_at_pos(canvas_pos) {
                    // Cmd+click on node with URL => open the URL
                    if cmd_held {
                        let url = self.document.find_node(&node_id).map(|n| n.url.clone()).unwrap_or_default();
                        if !url.is_empty() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(&url));
                        } else {
                            self.selection.toggle_node(node_id);
                        }
                    } else {
                        // Click on chevron (▶) of a collapsed node → expand it
                        let is_collapsed = self.document.find_node(&node_id)
                            .map_or(false, |n| n.collapsed);
                        if is_collapsed {
                            let tl = self.document.find_node(&node_id)
                                .map(|n| self.viewport.canvas_to_screen(n.pos()));
                            let h = self.document.find_node(&node_id)
                                .map(|n| n.size[1] * self.viewport.zoom)
                                .unwrap_or(28.0);
                            if let Some(tl) = tl {
                                let chevron_rect = egui::Rect::from_min_size(
                                    tl + Vec2::new(4.0, 0.0),
                                    Vec2::new(20.0, h),
                                );
                                if let Some(mp) = pointer_pos {
                                    if chevron_rect.contains(mp) {
                                        if let Some(node) = self.document.find_node_mut(&node_id) {
                                            node.toggle_collapsed();
                                        }
                                        self.history.push(&self.document);
                                        return; // don't select after toggling
                                    }
                                }
                            }
                        }
                        self.selection.select_node(node_id);
                    }
                } else if let Some(edge_id) = self.hit_test_edge(canvas_pos) {
                    // Click on edge => select it
                    if cmd_held {
                        self.selection.toggle_edge(edge_id);
                    } else {
                        self.selection.select_edge(edge_id);
                    }
                } else {
                    // Click on empty space => deselect
                    if !cmd_held {
                        self.selection.clear();
                    }
                }
            }
        }

        // Right-click context menu
        response.context_menu(|ui| {
            if let Some(mouse) = pointer_pos {
                let canvas_pos = self.viewport.screen_to_canvas(mouse);
                if let Some(node_id) = self.document.node_at_pos(canvas_pos) {
                    // Node context menu (handled below)
                    self.selection.select_node(node_id);

                    // Quick-color row at top
                    ui.label(egui::RichText::new("Fill").size(9.5).color(TEXT_DIM));
                    let quick_colors: &[([u8;4], &str)] = &[
                        ([30, 30, 46, 255],  "Surface"),
                        ([137, 180, 250, 255], "Blue"),
                        ([166, 227, 161, 255], "Green"),
                        ([243, 139, 168, 255], "Red"),
                        ([249, 226, 175, 255], "Yellow"),
                        ([203, 166, 247, 255], "Purple"),
                        ([245, 194, 231, 255], "Pink"),
                        ([148, 226, 213, 255], "Teal"),
                        ([255, 255, 255, 255], "White"),
                        ([17, 17, 27, 255],   "Black"),
                    ];
                    let mut color_pick: Option<[u8;4]> = None;
                    ui.horizontal_wrapped(|ui| {
                        for (color, name) in quick_colors {
                            let c = to_color32(*color);
                            let is_current = self.document.find_node(&node_id)
                                .map(|n| n.style.fill_color == *color)
                                .unwrap_or(false);
                            let btn = egui::Button::new(if is_current { "✓" } else { "  " })
                                .fill(c)
                                .min_size(egui::Vec2::new(22.0, 22.0));
                            if ui.add(btn).on_hover_text(*name).clicked() {
                                color_pick = Some(*color);
                            }
                        }
                    });
                    if let Some(col) = color_pick {
                        if let Some(n) = self.document.find_node_mut(&node_id) {
                            n.style.fill_color = col;
                        }
                        self.history.push(&self.document);
                        ui.close_menu();
                    }
                    ui.separator();

                    if ui.button("✏ Edit label").clicked() {
                        self.focus_label_edit = true;
                        ui.close_menu();
                    }
                    if ui.button("⎘ Duplicate").clicked() {
                        if let Some(node) = self.document.find_node(&node_id).cloned() {
                            let mut copy = node;
                            copy.id = NodeId::new();
                            copy.set_pos(copy.pos() + Vec2::new(24.0, 24.0));
                            let cid = copy.id;
                            self.document.nodes.push(copy);
                            self.selection.select_node(cid);
                            self.history.push(&self.document);
                        }
                        ui.close_menu();
                    }
                    if ui.button("⬆ Bring to Front").clicked() {
                        if let Some(i) = self.document.nodes.iter().position(|n| n.id == node_id) {
                            let n = self.document.nodes.remove(i);
                            self.document.nodes.push(n);
                            self.history.push(&self.document);
                        }
                        ui.close_menu();
                    }
                    // Quick tag submenu
                    ui.menu_button("🏷 Tag…", |ui| {
                        let tags = [
                            (None, "None"),
                            (Some(crate::model::NodeTag::Critical), "🔴 Critical"),
                            (Some(crate::model::NodeTag::Warning),  "🟡 Warning"),
                            (Some(crate::model::NodeTag::Ok),       "🟢 OK"),
                            (Some(crate::model::NodeTag::Info),     "🔵 Info"),
                        ];
                        for (variant, label) in tags {
                            if ui.button(label).clicked() {
                                if let Some(n) = self.document.find_node_mut(&node_id) {
                                    n.tag = variant;
                                }
                                self.history.push(&self.document);
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("🗑 Delete").clicked() {
                        self.document.remove_node(&node_id);
                        self.selection.clear();
                        self.history.push(&self.document);
                        ui.close_menu();
                    }
                } else if let Some(edge_id) = self.hit_test_edge(canvas_pos) {
                    // Edge context menu
                    self.selection.select_edge(edge_id);
                    ui.label(egui::RichText::new("Edge").size(11.0).color(TEXT_DIM));
                    ui.separator();
                    // Color presets
                    let colors: &[([u8; 4], &str)] = &[
                        ([100, 100, 100, 255], "Gray"),
                        ([137, 180, 250, 255], "Blue"),
                        ([166, 227, 161, 255], "Green"),
                        ([243, 139, 168, 255], "Red"),
                        ([249, 226, 175, 255], "Yellow"),
                        ([203, 166, 247, 255], "Purple"),
                    ];
                    ui.horizontal_wrapped(|ui| {
                        for (color, name) in colors {
                            let c = to_color32(*color);
                            if ui.add(egui::Button::new("  ").fill(c).min_size(egui::Vec2::new(22.0, 22.0)))
                                .on_hover_text(*name).clicked() {
                                if let Some(e) = self.document.find_edge_mut(&edge_id) {
                                    e.style.color = *color;
                                }
                                self.history.push(&self.document);
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("↺ Reset style").clicked() {
                        if let Some(e) = self.document.find_edge_mut(&edge_id) {
                            e.style = EdgeStyle::default();
                        }
                        self.history.push(&self.document);
                        ui.close_menu();
                    }
                    if ui.button("🗑 Delete edge").clicked() {
                        self.document.remove_edge(&edge_id);
                        self.selection.clear();
                        self.history.push(&self.document);
                        ui.close_menu();
                    }
                } else {
                    // Canvas context menu
                    ui.label(egui::RichText::new("Canvas").size(10.0).color(TEXT_DIM));
                    ui.separator();

                    // Add node submenu
                    ui.menu_button("➕ Add Node…", |ui| {
                        for (shape, label) in [
                            (NodeShape::Rectangle,   "□ Rectangle"),
                            (NodeShape::RoundedRect, "▢ Rounded"),
                            (NodeShape::Diamond,     "◇ Diamond"),
                            (NodeShape::Circle,      "○ Circle"),
                        ] {
                            if ui.button(label).clicked() {
                                let w = 140.0_f32; let h = 60.0_f32;
                                let pos = egui::Pos2::new(canvas_pos.x - w/2.0, canvas_pos.y - h/2.0);
                                let mut node = Node::new(shape, pos);
                                node.size = [w, h];
                                let id = node.id;
                                self.document.nodes.push(node);
                                self.selection.select_node(id);
                                self.focus_label_edit = true;
                                self.history.push(&self.document);
                                ui.close_menu();
                            }
                        }
                        ui.separator();
                        if ui.button("📝 Sticky Note").clicked() {
                            let n = Node::new_sticky(crate::model::StickyColor::Yellow,
                                egui::Pos2::new(canvas_pos.x - 75.0, canvas_pos.y - 75.0));
                            self.selection.select_node(n.id);
                            self.document.nodes.push(n);
                            self.history.push(&self.document);
                            ui.close_menu();
                        }
                        if ui.button("⬜ Frame").clicked() {
                            let n = Node::new_frame(egui::Pos2::new(canvas_pos.x - 150.0, canvas_pos.y - 110.0));
                            self.selection.select_node(n.id);
                            self.document.nodes.push(n);
                            self.history.push(&self.document);
                            ui.close_menu();
                        }
                    });

                    if !self.clipboard.is_empty() {
                        if ui.button(format!("📋 Paste ({} node(s))", self.clipboard.len())).clicked() {
                            self.selection.clear();
                            let n = self.clipboard.len() as f32;
                            let centroid = self.clipboard.iter().fold(Vec2::ZERO, |a, nd| a + nd.pos().to_vec2()) / n;
                            let shift = canvas_pos.to_vec2() - centroid;
                            for tmpl in self.clipboard.clone() {
                                let mut nd = tmpl;
                                nd.id = NodeId::new();
                                nd.set_pos(nd.pos() + shift);
                                self.selection.node_ids.insert(nd.id);
                                self.document.nodes.push(nd);
                            }
                            self.history.push(&self.document);
                            ui.close_menu();
                        }
                    }

                    if ui.button("🔍 Select All").clicked() {
                        for n in &self.document.nodes { self.selection.node_ids.insert(n.id); }
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("⊞ Fit to Content").clicked() {
                        self.fit_to_content();
                        ui.close_menu();
                    }
                    if ui.button("1:1 Reset Zoom").clicked() {
                        self.viewport.zoom = 1.0;
                        ui.close_menu();
                    }

                    ui.separator();

                    let grid_label = if self.show_grid { "⊡ Hide Grid" } else { "⊞ Show Grid" };
                    if ui.button(grid_label).clicked() {
                        self.show_grid = !self.show_grid;
                        ui.close_menu();
                    }
                    let snap_label = if self.snap_to_grid { "⊠ Snap Off" } else { "⊟ Snap to Grid" };
                    if ui.button(snap_label).clicked() {
                        self.snap_to_grid = !self.snap_to_grid;
                        ui.close_menu();
                    }
                }
            }
        });

        // Double-click to focus label editing, or create new node on empty space
        if response.double_clicked() {
            if let Some(mouse) = pointer_pos {
                let canvas_pos = self.viewport.screen_to_canvas(mouse);
                if let Some(node_id) = self.document.node_at_pos(canvas_pos) {
                    self.selection.select_node(node_id);
                    // Open inline canvas editor for editable label types
                    let current_label = self.document.find_node(&node_id)
                        .and_then(|n| match &n.kind {
                            NodeKind::Shape { label, .. } => Some(label.clone()),
                            NodeKind::Text { content } => Some(content.clone()),
                            NodeKind::Entity { name, .. } => Some(name.clone()),
                            NodeKind::StickyNote { text, .. } => Some(text.clone()),
                        });
                    if let Some(label) = current_label {
                        self.inline_node_edit = Some((node_id, label));
                    } else {
                        self.focus_label_edit = true;
                    }
                } else if let Some(edge_id) = self.hit_test_edge(canvas_pos) {
                    // Double-click edge => open inline label editor near click position
                    self.selection.select_edge(edge_id);
                    self.inline_edge_edit = Some((edge_id, mouse));
                } else if self.tool == Tool::Select {
                    // Create a new default shape node centered on the click
                    let mut node = Node::new(NodeShape::Rectangle, canvas_pos);
                    let w = node.size[0];
                    let h = node.size[1];
                    node.set_pos(egui::Pos2::new(canvas_pos.x - w / 2.0, canvas_pos.y - h / 2.0));
                    let id = node.id;
                    self.document.nodes.push(node);
                    self.selection.select_node(id);
                    self.inline_node_edit = Some((id, String::new()));
                    self.history.push(&self.document);
                    self.status_message = Some(("Node created".to_string(), std::time::Instant::now()));
                }
            }
        }

        // Set cursor
        let cursor = self.pick_cursor(pointer_pos);
        ui.ctx().set_cursor_icon(cursor);

        // --- Drawing ---
        let node_idx = self.document.node_index();
        let hover_pos = ui.ctx().input(|i| i.pointer.hover_pos());

        // Compute highlighted path edges (when 2 nodes selected)
        let path_edge_ids: std::collections::HashSet<EdgeId> = {
            let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
            if ids.len() == 2 && self.selection.edge_ids.is_empty() {
                self.bfs_path_edges(ids[0], ids[1]).into_iter().collect()
            } else {
                std::collections::HashSet::new()
            }
        };

        // Hovered node for connection highlighting (not dragging)
        let hover_node_id: Option<NodeId> = match &self.drag {
            DragState::None => hover_pos.and_then(|hp| {
                let canvas_hp = self.viewport.screen_to_canvas(hp);
                self.document.node_at_pos(canvas_hp)
            }),
            _ => None,
        };

        // Selection-flash tracking: record when new nodes enter the selection
        {
            let now = ui.ctx().input(|i| i.time);
            // Prune stale entries (older than 0.4s to free memory)
            self.selection_times.retain(|_, t| now - *t < 0.4);
            // Add newly selected nodes
            let newly_selected: Vec<NodeId> = self.selection.node_ids.iter()
                .filter(|id| !self.selection_times.contains_key(id))
                .copied().collect();
            for id in newly_selected {
                self.selection_times.insert(id, now);
            }
        }

        // Progressive tooltip: track how long we've been hovering the same node
        {
            let now = ui.ctx().input(|i| i.time);
            match (hover_node_id, self.hover_node_start) {
                (Some(hid), Some((prev_id, _))) if hid == prev_id => {
                    // Same node — keep the timer running, request repaint for animation
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                }
                (Some(hid), _) => {
                    // New node hovered — reset timer
                    self.hover_node_start = Some((hid, now));
                }
                (None, _) => {
                    self.hover_node_start = None;
                }
            }
        }

        // Focus mode: precompute 1-hop neighbor set for hop-aware dimming
        let focus_neighbors: std::collections::HashSet<NodeId> = if self.focus_mode && !self.selection.is_empty() {
            self.document.edges.iter()
                .flat_map(|e| {
                    let mut v = Vec::new();
                    if self.selection.contains_node(&e.source.node_id) { v.push(e.target.node_id); }
                    if self.selection.contains_node(&e.target.node_id) { v.push(e.source.node_id); }
                    v
                })
                .collect()
        } else {
            std::collections::HashSet::new()
        };

        // Edges (visible only)
        for edge in &self.document.edges {
            let src_visible = node_idx
                .get(&edge.source.node_id)
                .and_then(|&i| self.document.nodes.get(i))
                .map(|n| {
                    let sr = Rect::from_min_size(
                        self.viewport.canvas_to_screen(n.pos()),
                        n.size_vec() * self.viewport.zoom,
                    );
                    sr.expand(100.0).intersects(canvas_rect)
                })
                .unwrap_or(false);
            let tgt_visible = node_idx
                .get(&edge.target.node_id)
                .and_then(|&i| self.document.nodes.get(i))
                .map(|n| {
                    let sr = Rect::from_min_size(
                        self.viewport.canvas_to_screen(n.pos()),
                        n.size_vec() * self.viewport.zoom,
                    );
                    sr.expand(100.0).intersects(canvas_rect)
                })
                .unwrap_or(false);
            if src_visible || tgt_visible {
                let hover_canvas = hover_pos.map(|p| self.viewport.screen_to_canvas(p));

                // Connection highlight: dim edges not connected to hovered node
                let is_connected_to_hover = hover_node_id.map_or(false, |hid| {
                    edge.source.node_id == hid || edge.target.node_id == hid
                });
                let should_dim = hover_node_id.is_some()
                    && !is_connected_to_hover
                    && self.selection.is_empty();
                if should_dim {
                    // Draw a dim overlay line instead of full edge rendering
                    if let (Some(&si), Some(&ti)) = (node_idx.get(&edge.source.node_id), node_idx.get(&edge.target.node_id)) {
                        if let (Some(sn), Some(tn)) = (self.document.nodes.get(si), self.document.nodes.get(ti)) {
                            let s = self.viewport.canvas_to_screen(sn.port_position(edge.source.side));
                            let t = self.viewport.canvas_to_screen(tn.port_position(edge.target.side));
                            let offset = 60.0 * self.viewport.zoom;
                            let (cp1, cp2) = super::interaction::control_points_for_side(s, t, edge.source.side, offset);
                            let dim_color = Color32::from_rgba_unmultiplied(100, 100, 120, 40);
                            let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                                [s, cp1, cp2, t], false, Color32::TRANSPARENT,
                                Stroke::new(edge.style.width, dim_color),
                            );
                            painter.add(bezier);
                        }
                    }
                } else if self.focus_mode && !self.selection.is_empty() {
                    // Focus mode: only draw edges connected to selected or 1-hop neighbors
                    let src_rel = self.selection.contains_node(&edge.source.node_id)
                        || focus_neighbors.contains(&edge.source.node_id);
                    let tgt_rel = self.selection.contains_node(&edge.target.node_id)
                        || focus_neighbors.contains(&edge.target.node_id);
                    if src_rel || tgt_rel {
                        self.draw_edge(edge, &painter, &node_idx, hover_canvas);
                    } else if let (Some(&si), Some(&ti)) = (node_idx.get(&edge.source.node_id), node_idx.get(&edge.target.node_id)) {
                        if let (Some(sn), Some(tn)) = (self.document.nodes.get(si), self.document.nodes.get(ti)) {
                            let s = self.viewport.canvas_to_screen(sn.port_position(edge.source.side));
                            let t = self.viewport.canvas_to_screen(tn.port_position(edge.target.side));
                            let off = 60.0 * self.viewport.zoom;
                            let (cp1, cp2) = super::interaction::control_points_for_side(s, t, edge.source.side, off);
                            let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                                [s, cp1, cp2, t], false, Color32::TRANSPARENT,
                                Stroke::new(edge.style.width, Color32::from_rgba_unmultiplied(80, 80, 100, 20)),
                            );
                            painter.add(bezier);
                        }
                    }
                } else {
                    self.draw_edge(edge, &painter, &node_idx, hover_canvas);
                }
                // Draw path highlight overlay
                if path_edge_ids.contains(&edge.id) {
                    self.draw_path_highlight(edge, &painter, &node_idx);
                }
            }
        }

        // Connected-edge glow pass: highlight edges touching the hovered node
        if let Some(hid) = hover_node_id {
            if self.selection.is_empty() {
                let t_pulse = painter.ctx().input(|i| i.time) as f32;
                self.draw_hover_edge_glow(&painter, &node_idx, hid, t_pulse, canvas_rect);
                painter.ctx().request_repaint_after(std::time::Duration::from_millis(33));
            }
        }

        // Alt-hover distance rulers: show spacing measurement lines to nearby nodes
        let alt_held = ui.ctx().input(|i| i.modifiers.alt);
        if alt_held {
            if let Some(hid) = hover_node_id {
                self.draw_distance_rulers(&painter, hid, canvas_rect);
            } else if let Some(sel_id) = self.selection.node_ids.iter().next().copied() {
                // Also show rulers for the selected node when Alt is held
                self.draw_distance_rulers(&painter, sel_id, canvas_rect);
            }
        }

        // Data-flow animation: dots traveling along edges
        if self.show_flow_animation && !self.document.edges.is_empty() {
            let time = painter.ctx().input(|i| i.time) as f32;
            self.draw_flow_animation(&painter, &node_idx, time, canvas_rect);
            painter.ctx().request_repaint_after(std::time::Duration::from_millis(16));
        }

        // Focus mode overlay: hop-aware neighborhood dimming
        // Selected: no overlay | 1-hop neighbors: light dim | others: heavy dim
        if self.focus_mode && !self.selection.is_empty() {
            for node in &self.document.nodes {
                if self.selection.contains_node(&node.id) { continue; }
                let screen_pos = self.viewport.canvas_to_screen(node.pos());
                let screen_size = node.size_vec() * self.viewport.zoom;
                let screen_rect = Rect::from_min_size(screen_pos, screen_size);
                if !screen_rect.intersects(canvas_rect) { continue; }
                // Neighbors get a lighter veil; distant nodes get heavy dim
                let alpha = if focus_neighbors.contains(&node.id) { 90u8 } else { 190u8 };
                painter.rect_filled(
                    screen_rect,
                    CornerRadius::same(4),
                    Color32::from_rgba_premultiplied(16, 16, 28, alpha),
                );
            }
        }

        // Multi-selection bounding box with dimension labels (shown when ≥2 nodes selected)
        if self.selection.node_ids.len() >= 2 {
            let sel_nodes: Vec<&Node> = self.document.nodes.iter()
                .filter(|n| self.selection.contains_node(&n.id))
                .collect();
            if !sel_nodes.is_empty() {
                let mut min_x = f32::MAX; let mut min_y = f32::MAX;
                let mut max_x = f32::MIN; let mut max_y = f32::MIN;
                for n in &sel_nodes {
                    let p = n.pos();
                    let s = n.size_vec();
                    min_x = min_x.min(p.x); min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x + s.x); max_y = max_y.max(p.y + s.y);
                }
                let tl = self.viewport.canvas_to_screen(Pos2::new(min_x, min_y));
                let br = self.viewport.canvas_to_screen(Pos2::new(max_x, max_y));
                let bbox = Rect::from_min_max(tl, br).expand(8.0);
                // Dashed border segments
                let dash_color = SELECTION_COLOR.gamma_multiply(0.55);
                let dash_len = 6.0_f32;
                let gap_len = 4.0_f32;
                let corners = [bbox.left_top(), bbox.right_top(), bbox.right_bottom(), bbox.left_bottom(), bbox.left_top()];
                for pair in corners.windows(2) {
                    let (a, b) = (pair[0], pair[1]);
                    let total = (b - a).length();
                    if total < 1.0 { continue; }
                    let dir = (b - a) / total;
                    let mut t = 0.0_f32;
                    let mut drawing = true;
                    while t < total {
                        let seg_end = (t + if drawing { dash_len } else { gap_len }).min(total);
                        if drawing {
                            painter.line_segment([a + dir * t, a + dir * seg_end],
                                Stroke::new(1.2, dash_color));
                        }
                        t = seg_end;
                        drawing = !drawing;
                    }
                }
                // Dimension labels
                let w_canvas = max_x - min_x;
                let h_canvas = max_y - min_y;
                let font_sz = (10.0 * self.viewport.zoom.sqrt()).clamp(9.0, 13.0);
                let lbl_color = SELECTION_COLOR.gamma_multiply(0.75);
                let bg = Color32::from_rgba_premultiplied(16, 16, 28, 180);
                // Width label (bottom-center)
                let w_text = format!("{:.0}", w_canvas);
                let w_pos = Pos2::new(bbox.center().x, bbox.max.y + 10.0);
                let w_galley = painter.layout_no_wrap(w_text.clone(), FontId::proportional(font_sz), lbl_color);
                let wr = Rect::from_center_size(w_pos, w_galley.size()).expand2(Vec2::new(4.0, 2.0));
                painter.rect_filled(wr, CornerRadius::same(3), bg);
                painter.text(w_pos, Align2::CENTER_CENTER, &w_text, FontId::proportional(font_sz), lbl_color);
                // Height label (right-center)
                let h_text = format!("{:.0}", h_canvas);
                let h_pos = Pos2::new(bbox.max.x + 10.0, bbox.center().y);
                let h_galley = painter.layout_no_wrap(h_text.clone(), FontId::proportional(font_sz), lbl_color);
                let hr = Rect::from_center_size(h_pos, h_galley.size()).expand2(Vec2::new(4.0, 2.0));
                painter.rect_filled(hr, CornerRadius::same(3), bg);
                painter.text(h_pos, Align2::CENTER_CENTER, &h_text, FontId::proportional(font_sz), lbl_color);
                // Count label (top-left corner)
                let count_text = format!("{}×", sel_nodes.len());
                let count_pos = Pos2::new(bbox.min.x - 1.0, bbox.min.y - 10.0);
                painter.text(count_pos, Align2::LEFT_BOTTOM, &count_text, FontId::proportional(font_sz), lbl_color);
                let _ = (w_galley, h_galley); // suppress unused warnings
            }
        }

        // Compute search matches (for highlight overlay)
        let search_matches: std::collections::HashSet<NodeId> = if self.show_search && !self.search_query.is_empty() {
            let q = self.search_query.to_lowercase();
            self.document.nodes.iter()
                .filter(|n| n.display_label().to_lowercase().contains(&q))
                .map(|n| n.id)
                .collect()
        } else {
            std::collections::HashSet::new()
        };

        // Detect newly created edges → record birth time for draw-in animation
        {
            let now = ui.ctx().input(|i| i.time);
            // Prune old entries
            self.edge_birth_times.retain(|_, t| now - *t < 0.35);
            // Add NEW edge IDs (only ones not seen on previous frame)
            let current_edge_ids: std::collections::HashSet<EdgeId> =
                self.document.edges.iter().map(|e| e.id).collect();
            for edge in &self.document.edges {
                if !self.prev_edge_ids.contains(&edge.id) && !self.prev_edge_ids.is_empty() {
                    self.edge_birth_times.insert(edge.id, now);
                }
            }
            self.prev_edge_ids = current_edge_ids;
            // Request repaint if any animation in progress
            if !self.edge_birth_times.is_empty() {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
            }
        }

        // Detect newly created nodes → spawn creation ripples
        {
            let now = ui.ctx().input(|i| i.time);
            let current_ids: std::collections::HashSet<NodeId> =
                self.document.nodes.iter().map(|n| n.id).collect();
            // Prune stale node birth entries (3s freshness window)
            self.node_birth_times.retain(|_, t| now - *t < 3.0);
            for node in &self.document.nodes {
                if !self.prev_node_ids.contains(&node.id) && !self.prev_node_ids.is_empty() {
                    let center = node.rect().center();
                    self.creation_ripples.push(([center.x, center.y], now));
                    self.node_birth_times.insert(node.id, now);
                }
            }
            self.prev_node_ids = current_ids;

            // Draw and cull ripples (duration = 0.55s)
            let ripple_duration = 0.55_f64;
            self.creation_ripples.retain(|&(_, birth)| now - birth < ripple_duration);
            for &(world_center, birth) in &self.creation_ripples {
                let t = ((now - birth) / ripple_duration) as f32; // 0 → 1
                let center = self.viewport.canvas_to_screen(
                    egui::Pos2::new(world_center[0], world_center[1])
                );
                // Spring: expand fast, ease out; max radius ~80 screen px
                let radius = t.powf(0.45) * 80.0;
                // Alpha: full at t=0, fades by t=1
                let alpha = ((1.0 - t) * 200.0) as u8;
                // Two-ring ripple: outer faint, inner brighter
                painter.circle_stroke(
                    center, radius,
                    Stroke::new(2.0, Color32::from_rgba_premultiplied(137, 180, 250, alpha / 2)),
                );
                painter.circle_stroke(
                    center, radius * 0.55,
                    Stroke::new(1.5, Color32::from_rgba_premultiplied(166, 227, 161, alpha)),
                );
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
            }
        }

        // Nodes: frame nodes first (they render behind), then regular nodes sorted by z_offset
        // This ensures frames appear as translucent containers beneath everything else,
        // and Cmd+]/[ z-ordering works correctly for regular nodes.
        let node_ids: Vec<NodeId> = {
            let mut frames: Vec<(NodeId, f32)> = self.document.nodes.iter()
                .filter(|n| n.is_frame).map(|n| (n.id, n.z_offset)).collect();
            let mut rest: Vec<(NodeId, f32)> = self.document.nodes.iter()
                .filter(|n| !n.is_frame).map(|n| (n.id, n.z_offset)).collect();
            frames.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            rest.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            frames.into_iter().chain(rest).map(|(id, _)| id).collect()
        };
        for node_id in &node_ids {
            let Some(node) = self.document.find_node(node_id) else { continue };
            let screen_pos = self.viewport.canvas_to_screen(node.pos());
            let screen_size = node.size_vec() * self.viewport.zoom;
            let screen_rect = Rect::from_min_size(screen_pos, screen_size).expand(20.0);
            if screen_rect.intersects(canvas_rect) {
                // Draw search highlight ring before rendering the node
                if search_matches.contains(&node.id) {
                    let node_screen_rect = Rect::from_min_size(
                        self.viewport.canvas_to_screen(node.pos()),
                        node.size_vec() * self.viewport.zoom,
                    );
                    let time = ui.ctx().input(|i| i.time);
                    let pulse = ((time * 3.0 * std::f64::consts::PI).sin() as f32) * 0.4 + 0.6;
                    painter.rect_stroke(
                        node_screen_rect.expand(4.0),
                        CornerRadius::same(8),
                        Stroke::new(2.5, Color32::from_rgba_unmultiplied(137, 220, 235, (200.0 * pulse) as u8)),
                        StrokeKind::Outside,
                    );
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
                }
                self.draw_node(node, &painter, hover_pos);
            }
        }

        // Connectivity heatmap overlay
        if self.show_heatmap && !self.document.nodes.is_empty() {
            self.draw_heatmap_overlay(&painter, canvas_rect);
        }

        // Resize handles on single-selected node
        if self.selection.node_ids.len() == 1 {
            let sel_id = *self.selection.node_ids.iter().next().unwrap();
            if let Some(node) = self.document.find_node(&sel_id) {
                let top_left = self.viewport.canvas_to_screen(node.pos());
                let size = node.size_vec() * self.viewport.zoom;
                let screen_rect = Rect::from_min_size(top_left, size);
                self.draw_resize_handles(&painter, screen_rect);
            }
        }

        // Floating quick-action bar above selected node(s)
        self.draw_floating_action_bar(ui, canvas_rect);

        // --- Rulers ---
        if self.show_grid {
            self.draw_rulers(&painter, canvas_rect);
            self.draw_ruler_crosshair(&painter, canvas_rect, pointer_pos);
        }

        // --- Presentation spotlight ---
        if self.presentation_mode {
            self.draw_presentation_spotlight(&painter, canvas_rect, pointer_pos);
        }

        // Multi-node drag ghost: faint outlines at original positions when dragging 2+ nodes
        if let DragState::DraggingNode { start_positions, .. } = &self.drag {
            if start_positions.len() >= 2 {
                let ghost_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(137, 180, 250, 60));
                for (node_id, orig_pos) in start_positions {
                    if let Some(node) = self.document.find_node(node_id) {
                        let screen_tl = self.viewport.canvas_to_screen(*orig_pos);
                        let ghost_rect = Rect::from_min_size(screen_tl, node.size_vec() * self.viewport.zoom);
                        if ghost_rect.intersects(canvas_rect) {
                            painter.rect_stroke(ghost_rect, CornerRadius::same(4), ghost_stroke, StrokeKind::Outside);
                        }
                    }
                }
            }
        }

        // Resize ghost: show original node rect when resizing
        if let DragState::ResizingNode { start_rect, .. } = &self.drag {
            let sr = *start_rect; // [x, y, w, h] in canvas space
            let tl = self.viewport.canvas_to_screen(Pos2::new(sr[0], sr[1]));
            let ghost_rect = Rect::from_min_size(tl, Vec2::new(sr[2], sr[3]) * self.viewport.zoom);
            painter.rect_stroke(
                ghost_rect,
                CornerRadius::same(4),
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(137, 180, 250, 90)),
                StrokeKind::Outside,
            );
            // Draw dashed ghost with 2 offset rects
            painter.rect_stroke(
                ghost_rect.expand(1.5),
                CornerRadius::same(5),
                Stroke::new(0.5, Color32::from_rgba_unmultiplied(137, 180, 250, 40)),
                StrokeKind::Outside,
            );
            // Size label
            painter.text(
                ghost_rect.center_bottom() + Vec2::new(0.0, 6.0),
                Align2::CENTER_TOP,
                &format!("{:.0} × {:.0}", sr[2], sr[3]),
                FontId::proportional(9.0),
                Color32::from_rgba_unmultiplied(137, 180, 250, 160),
            );
        }

        // --- Previews ---
        self.draw_alignment_guides(&painter, canvas_rect);
        self.draw_distance_indicators(&painter);
        // Multi-selection bounding box outline
        if self.selection.node_ids.len() >= 2 {
            let bb = self.selection.node_ids.iter()
                .filter_map(|id| self.document.find_node(id))
                .fold(Option::<Rect>::None, |acc, n| {
                    let sr = Rect::from_min_size(self.viewport.canvas_to_screen(n.pos()), n.size_vec() * self.viewport.zoom);
                    Some(acc.map_or(sr, |r| r.union(sr)))
                });
            if let Some(bb) = bb {
                let expanded = bb.expand(6.0);
                // Dashed bounding box stroke
                painter.rect_stroke(
                    expanded, CornerRadius::same(4),
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(137, 180, 250, 80)),
                    StrokeKind::Outside,
                );
                // Corner handle dots
                for corner in [expanded.left_top(), expanded.right_top(), expanded.left_bottom(), expanded.right_bottom()] {
                    painter.circle_filled(corner, 4.0, Color32::from_rgba_unmultiplied(137, 180, 250, 180));
                    painter.circle_stroke(corner, 4.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(30, 30, 46, 200)));
                }
                // Edge midpoint handles
                for mid in [
                    Pos2::new(expanded.center().x, expanded.min.y),
                    Pos2::new(expanded.center().x, expanded.max.y),
                    Pos2::new(expanded.min.x, expanded.center().y),
                    Pos2::new(expanded.max.x, expanded.center().y),
                ] {
                    painter.circle_filled(mid, 3.0, Color32::from_rgba_unmultiplied(137, 180, 250, 140));
                }
                // Bounding box size label
                let w = (bb.width() / self.viewport.zoom).round() as i32;
                let h = (bb.height() / self.viewport.zoom).round() as i32;
                painter.text(
                    expanded.center_top() - Vec2::new(0.0, 14.0),
                    Align2::CENTER_BOTTOM,
                    &format!("{w} × {h}"),
                    FontId::proportional(9.0),
                    Color32::from_rgba_unmultiplied(137, 180, 250, 160),
                );
            }
        }

        self.draw_inline_node_editor(ui, canvas_rect);
        // Floating edge style bar: quick-toggle edge styles on selected edge
        self.draw_floating_edge_bar(ui, canvas_rect);
        // Quick-connect arrows: show ±4 directional buttons on hovered node
        let drag_idle = matches!(&self.drag, DragState::None);
        let not_editing = self.inline_node_edit.is_none();
        let not_connect = self.tool != Tool::Connect;
        if drag_idle && not_editing && not_connect {
            if let Some(hid) = hover_node_id {
                if self.selection.is_empty() || self.selection.contains_node(&hid) {
                    self.draw_quick_connect_arrows(ui, hid, canvas_rect);
                }
            }
        }
        self.draw_box_select_preview(&painter, pointer_pos);
        self.draw_edge_creation_preview(&painter, &node_idx);
        self.draw_new_node_preview(&painter, canvas_rect);
        self.draw_node_tooltip(&painter, hover_pos, canvas_rect);
        self.draw_edge_tooltip(&painter, hover_pos, canvas_rect, &node_idx);
        self.draw_status_toast(&painter, canvas_rect, ui.ctx());
        self.draw_canvas_hud(&painter, canvas_rect, pointer_pos);
        self.draw_canvas_vignette(&painter, canvas_rect);
        self.draw_project_title(&painter, canvas_rect);
        self.draw_empty_canvas_hint(&painter, canvas_rect);
        self.draw_search_overlay(ui, canvas_rect);
        self.draw_zoom_presets(ui, canvas_rect);
        self.draw_minimap(&painter, canvas_rect);
        if self.show_quick_notes {
            self.draw_quick_notes_panel(ui, canvas_rect);
        }

        // Minimap click-to-pan and drag-to-pan
        if let Some(click_pos) = pointer_pos {
            let (clicked, dragging) = ui.ctx().input(|i| (
                i.pointer.primary_clicked(),
                i.pointer.primary_down() && i.pointer.is_moving(),
            ));
            if clicked || dragging {
                self.handle_minimap_click(click_pos, canvas_rect);
            }
        }
    }

    // --- Input dispatch helpers ---

    fn handle_drag_start(
        &mut self,
        response: &egui::Response,
        ui: &egui::Ui,
        pointer_pos: Option<Pos2>,
    ) {
        if !response.drag_started() {
            return;
        }
        let Some(mouse) = pointer_pos else { return };
        let canvas_pos = self.viewport.screen_to_canvas(mouse);
        let middle_button = ui
            .ctx()
            .input(|i| i.pointer.button_down(egui::PointerButton::Middle));

        if self.space_held || middle_button {
            // Cancel any fly-to animation when user grabs the canvas
            self.pan_target = None;
            self.drag = DragState::Panning {
                start_offset: self.viewport.offset,
                start_mouse: mouse,
            };
        } else if self.tool == Tool::Connect {
            if let Some(port) = self.hit_test_port(canvas_pos) {
                self.drag = DragState::CreatingEdge {
                    source: port,
                    current_screen: mouse,
                };
            }
        } else {
            // Select tool
            if let Some((node_id, handle)) = self.hit_test_resize_handle(mouse) {
                if let Some(node) = self.document.find_node(&node_id) {
                    self.drag = DragState::ResizingNode {
                        node_id,
                        handle,
                        start_rect: [
                            node.position[0],
                            node.position[1],
                            node.size[0],
                            node.size[1],
                        ],
                        start_mouse: canvas_pos,
                    };
                }
            } else if let Some(port) = self.hit_test_port(canvas_pos) {
                self.drag = DragState::CreatingEdge {
                    source: port,
                    current_screen: mouse,
                };
            } else if let Some(node_id) = self.document.node_at_pos(canvas_pos) {
                let cmd_held = ui.ctx().input(|i| i.modifiers.command);
                let alt_held = ui.ctx().input(|i| i.modifiers.alt);
                if cmd_held {
                    self.selection.toggle_node(node_id);
                } else if !self.selection.contains_node(&node_id) {
                    self.selection.select_node(node_id);
                }
                // Don't initiate node drag when canvas is locked
                if !self.canvas_locked {
                    // Alt+drag: clone selected nodes, drag the clones
                    if alt_held && !self.selection.node_ids.is_empty() {
                        let sel_ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                        let mut new_ids = Vec::new();
                        for id in &sel_ids {
                            if let Some(original) = self.document.find_node(id).cloned() {
                                let mut clone = original.clone();
                                clone.id = NodeId::new();
                                new_ids.push(clone.id);
                                self.document.nodes.push(clone);
                            }
                        }
                        // Select only the clones; originals stay unselected
                        self.selection.clear();
                        for id in &new_ids { self.selection.select_node(*id); }
                        self.status_message = Some(("Alt+drag: duplicated".to_string(), std::time::Instant::now()));
                    }
                    let start_positions: Vec<(NodeId, Pos2)> = self
                        .selection
                        .node_ids
                        .iter()
                        .filter_map(|id| self.document.find_node(id).map(|n| (*id, n.pos())))
                        .collect();
                    self.drag = DragState::DraggingNode {
                        start_positions,
                        start_z_offsets: vec![],
                        start_mouse: canvas_pos,
                    };
                }
            } else if let Some(edge_id) = self.hit_test_bend_handle(mouse) {
                // Drag the curve bend handle of a selected edge
                let bend = self.document.find_edge(&edge_id)
                    .map(|e| e.style.curve_bend)
                    .unwrap_or(0.0);
                self.drag = DragState::DraggingEdgeBend {
                    edge_id,
                    start_bend: bend,
                    start_mouse: canvas_pos,
                };
            } else if let Some(edge_id) = self.hit_test_edge(canvas_pos) {
                let cmd_held = ui.ctx().input(|i| i.modifiers.command);
                if cmd_held {
                    self.selection.toggle_edge(edge_id);
                } else {
                    self.selection.select_edge(edge_id);
                }
                self.drag = DragState::None;
            } else {
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

    fn handle_dragging(
        &mut self,
        response: &egui::Response,
        _ui: &egui::Ui,
        pointer_pos: Option<Pos2>,
    ) {
        if !response.dragged() {
            return;
        }
        let Some(mouse) = pointer_pos else { return };

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
                ..
            } => {
                let canvas_mouse = self.viewport.screen_to_canvas(mouse);
                let delta = canvas_mouse - *start_mouse;
                let positions = start_positions.clone();
                let alt_held = _ui.ctx().input(|i| i.modifiers.alt);
                self.alignment_guides.clear();

                // Alignment snap: only for single-node drags (unless alt suppresses)
                if !alt_held && positions.len() == 1 {
                    let (id, start_pos) = positions[0];
                    let mut new_pos = start_pos + delta;
                    if self.snap_to_grid {
                        new_pos = self.snap_pos(new_pos);
                    }
                    let (snapped, guides) = self.compute_alignment_snap(id, new_pos, 8.0 / self.viewport.zoom);
                    self.alignment_guides = guides;
                    if let Some(node) = self.document.find_node_mut(&id) {
                        if !node.pinned && !node.locked {
                            node.set_pos(snapped);
                        }
                    }
                } else {
                    for (id, start_pos) in &positions {
                        let mut new_pos = *start_pos + delta;
                        if self.snap_to_grid {
                            new_pos = self.snap_pos(new_pos);
                        }
                        if let Some(node) = self.document.find_node_mut(id) {
                            if !node.pinned && !node.locked {
                                node.set_pos(new_pos);
                            }
                        }
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
            DragState::DraggingEdgeBend { edge_id, start_bend, start_mouse } => {
                let canvas_mouse = self.viewport.screen_to_canvas(mouse);
                let edge_id = *edge_id;
                let start_bend = *start_bend;
                let start_mouse = *start_mouse;
                // Project drag delta onto perpendicular of the edge direction
                if let Some(edge) = self.document.find_edge(&edge_id) {
                    let src = self.document.find_node(&edge.source.node_id)
                        .map(|n| n.port_position(edge.source.side));
                    let tgt = self.document.find_node(&edge.target.node_id)
                        .map(|n| n.port_position(edge.target.side));
                    if let (Some(s), Some(t)) = (src, tgt) {
                        let dir = if (t - s).length() > 1.0 { (t - s).normalized() } else { Vec2::X };
                        let perp = Vec2::new(-dir.y, dir.x);
                        let delta = canvas_mouse - start_mouse;
                        let bend_delta = delta.dot(perp);
                        let edge_id = edge_id;
                        if let Some(edge) = self.document.find_edge_mut(&edge_id) {
                            edge.style.curve_bend = start_bend + bend_delta;
                        }
                    }
                }
            }
            DragState::BoxSelect { .. } | DragState::None => {}
        }
    }

    fn handle_drag_end(
        &mut self,
        response: &egui::Response,
        _ui: &egui::Ui,
        pointer_pos: Option<Pos2>,
        canvas_rect: Rect,
    ) {
        if !response.drag_stopped() {
            return;
        }
        if let Some(mouse) = pointer_pos {
            match &self.drag {
                DragState::DraggingNode { .. }
                | DragState::ResizingNode { .. }
                | DragState::DraggingEdgeBend { .. } => {
                    self.history.push(&self.document);
                }
                DragState::CreatingEdge { source, .. } => {
                    let canvas_pos = self.viewport.screen_to_canvas(mouse);
                    if let Some(target) = self.hit_test_port(canvas_pos) {
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
                    for edge in &self.document.edges {
                        if self.selection.contains_edge(&edge.id) {
                            continue;
                        }
                        let src_node = self.document.find_node(&edge.source.node_id);
                        let tgt_node = self.document.find_node(&edge.target.node_id);
                        if let (Some(sn), Some(tn)) = (src_node, tgt_node) {
                            let src = sn.port_position(edge.source.side);
                            let tgt = tn.port_position(edge.target.side);
                            let (cp1, cp2) =
                                control_points_for_side(src, tgt, edge.source.side, 60.0);
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
                DragState::DraggingNewNode {
                    kind,
                    current_screen,
                } => {
                    if canvas_rect.contains(*current_screen) {
                        let mut canvas_pos =
                            self.viewport.screen_to_canvas(*current_screen);
                        if self.snap_to_grid {
                            canvas_pos = self.snap_pos(canvas_pos);
                        }
                        let node = match kind {
                            NodeKind::Shape { shape, .. } => Node::new(*shape, canvas_pos),
                            NodeKind::StickyNote { color, .. } => {
                                Node::new_sticky(*color, canvas_pos)
                            }
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

    fn pick_cursor(&self, pointer_pos: Option<Pos2>) -> egui::CursorIcon {
        match &self.drag {
            DragState::Panning { .. } | DragState::DraggingNode { .. } => {
                egui::CursorIcon::Grabbing
            }
            DragState::DraggingNewNode { .. } => egui::CursorIcon::Copy,
            DragState::CreatingEdge { .. } | DragState::BoxSelect { .. } => {
                egui::CursorIcon::Crosshair
            }
            DragState::ResizingNode { handle, .. } => Self::resize_cursor(*handle),
            DragState::DraggingEdgeBend { .. } => egui::CursorIcon::ResizeNeSw,
            DragState::None => {
                if self.space_held {
                    egui::CursorIcon::Grab
                } else if self.tool == Tool::Connect {
                    egui::CursorIcon::Crosshair
                } else if let Some(hover) = pointer_pos {
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
    }

    // --- Preview drawing ---

    fn draw_alignment_guides(&self, painter: &egui::Painter, canvas_rect: Rect) {
        // Only show during node drag
        let DragState::DraggingNode { .. } = &self.drag else { return };
        if self.selection.node_ids.is_empty() { return; }

        let threshold = 4.0 / self.viewport.zoom; // world-space tolerance
        let guide_color = egui::Color32::from_rgba_premultiplied(100, 180, 255, 160);
        let guide_stroke = Stroke::new(1.0, guide_color);

        // Collect dragged node rects
        let drag_rects: Vec<Rect> = self.selection.node_ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.rect())
            .collect();

        // Collect reference node rects (not dragged)
        let ref_rects: Vec<Rect> = self.document.nodes.iter()
            .filter(|n| !self.selection.node_ids.contains(&n.id))
            .map(|n| n.rect())
            .collect();

        for drag_rect in &drag_rects {
            let d_vals = [drag_rect.min.x, drag_rect.center().x, drag_rect.max.x,
                          drag_rect.min.y, drag_rect.center().y, drag_rect.max.y];
            for ref_rect in &ref_rects {
                let r_vals = [ref_rect.min.x, ref_rect.center().x, ref_rect.max.x,
                              ref_rect.min.y, ref_rect.center().y, ref_rect.max.y];
                // Vertical guides (X alignment)
                for dv in &d_vals[0..3] {
                    for rv in &r_vals[0..3] {
                        if (dv - rv).abs() < threshold {
                            let sx = self.viewport.canvas_to_screen(Pos2::new(*rv, 0.0)).x;
                            painter.line_segment(
                                [Pos2::new(sx, canvas_rect.min.y), Pos2::new(sx, canvas_rect.max.y)],
                                guide_stroke,
                            );
                        }
                    }
                }
                // Horizontal guides (Y alignment)
                for dv in &d_vals[3..6] {
                    for rv in &r_vals[3..6] {
                        if (dv - rv).abs() < threshold {
                            let sy = self.viewport.canvas_to_screen(Pos2::new(0.0, *rv)).y;
                            painter.line_segment(
                                [Pos2::new(canvas_rect.min.x, sy), Pos2::new(canvas_rect.max.x, sy)],
                                guide_stroke,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Figma-style distance indicators: red measurement lines when dragging near another node.
    fn draw_distance_indicators(&self, painter: &egui::Painter) {
        let DragState::DraggingNode { .. } = &self.drag else { return };
        let sel = &self.selection.node_ids;
        if sel.is_empty() { return; }

        // Compute bounding rect of all dragged nodes in canvas space
        let drag_rects: Vec<Rect> = sel.iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.rect())
            .collect();
        let Some(drag_union) = drag_rects.iter().copied().reduce(|a, b| a.union(b)) else { return };

        // Show measurements to nodes that overlap in one axis and are within 200 canvas-units on the other
        let threshold = 200.0;
        let dist_color = Color32::from_rgba_unmultiplied(255, 75, 75, 220);
        let line_stroke = Stroke::new(1.0, dist_color);
        let label_bg = Color32::from_rgba_unmultiplied(255, 75, 75, 200);
        let label_fg = Color32::WHITE;

        for node in &self.document.nodes {
            if sel.contains(&node.id) { continue; }
            let r = node.rect();

            // Horizontal gap: node is left or right of drag_union, with vertical overlap
            let v_overlap = drag_union.min.y < r.max.y && drag_union.max.y > r.min.y;
            // Vertical gap: node is above or below drag_union, with horizontal overlap
            let h_overlap = drag_union.min.x < r.max.x && drag_union.max.x > r.min.x;

            // -- Horizontal distance (gap on X axis) --
            if v_overlap {
                let (left_x, right_x) = if r.max.x <= drag_union.min.x {
                    (r.max.x, drag_union.min.x) // node is to the left
                } else if r.min.x >= drag_union.max.x {
                    (drag_union.max.x, r.min.x) // node is to the right
                } else {
                    continue;  // overlapping horizontally, skip
                };
                let gap = right_x - left_x;
                if gap <= 0.0 || gap > threshold { continue; }

                // Midpoint Y = center of vertical overlap
                let mid_y = drag_union.min.y.max(r.min.y) +
                    (drag_union.max.y.min(r.max.y) - drag_union.min.y.max(r.min.y)) * 0.5;
                let sp1 = self.viewport.canvas_to_screen(Pos2::new(left_x, mid_y));
                let sp2 = self.viewport.canvas_to_screen(Pos2::new(right_x, mid_y));
                painter.line_segment([sp1, sp2], line_stroke);
                // Tick marks
                let tick = Vec2::new(0.0, 4.0);
                painter.line_segment([sp1 - tick, sp1 + tick], line_stroke);
                painter.line_segment([sp2 - tick, sp2 + tick], line_stroke);
                // Distance label
                let label = format!("{:.0}", gap);
                let mid = (sp1 + sp2.to_vec2()) * 0.5;
                let glyph_rect = Rect::from_center_size(mid, Vec2::new(label.len() as f32 * 6.5 + 6.0, 15.0));
                painter.rect_filled(glyph_rect, CornerRadius::same(3), label_bg);
                painter.text(mid, Align2::CENTER_CENTER, &label, FontId::proportional(10.0), label_fg);
            }

            // -- Vertical distance (gap on Y axis) --
            if h_overlap {
                let (top_y, bot_y) = if r.max.y <= drag_union.min.y {
                    (r.max.y, drag_union.min.y)
                } else if r.min.y >= drag_union.max.y {
                    (drag_union.max.y, r.min.y)
                } else {
                    continue;
                };
                let gap = bot_y - top_y;
                if gap <= 0.0 || gap > threshold { continue; }

                let mid_x = drag_union.min.x.max(r.min.x) +
                    (drag_union.max.x.min(r.max.x) - drag_union.min.x.max(r.min.x)) * 0.5;
                let sp1 = self.viewport.canvas_to_screen(Pos2::new(mid_x, top_y));
                let sp2 = self.viewport.canvas_to_screen(Pos2::new(mid_x, bot_y));
                painter.line_segment([sp1, sp2], line_stroke);
                let tick = Vec2::new(4.0, 0.0);
                painter.line_segment([sp1 - tick, sp1 + tick], line_stroke);
                painter.line_segment([sp2 - tick, sp2 + tick], line_stroke);
                let label = format!("{:.0}", gap);
                let mid = (sp1 + sp2.to_vec2()) * 0.5;
                let glyph_rect = Rect::from_center_size(mid, Vec2::new(label.len() as f32 * 6.5 + 6.0, 15.0));
                painter.rect_filled(glyph_rect, CornerRadius::same(3), label_bg);
                painter.text(mid, Align2::CENTER_CENTER, &label, FontId::proportional(10.0), label_fg);
            }
        }
    }

    /// Computes snap-to-alignment for a dragged node.
    /// Returns (snapped_pos, guide_specs) where guide_specs are (is_horizontal, canvas_coord).
    fn compute_alignment_snap(
        &self,
        drag_id: NodeId,
        proposed: Pos2,
        threshold: f32,
    ) -> (Pos2, Vec<(bool, f32)>) {
        let drag_node = match self.document.find_node(&drag_id) {
            Some(n) => n,
            None => return (proposed, vec![]),
        };
        let dw = drag_node.size[0];
        let dh = drag_node.size[1];
        let mut snapped = proposed;
        let mut guides: Vec<(bool, f32)> = Vec::new();
        let mut x_snapped = false;
        let mut y_snapped = false;

        for other in &self.document.nodes {
            if other.id == drag_id || self.selection.node_ids.contains(&other.id) {
                continue;
            }
            let nx = other.position[0];
            let ny = other.position[1];
            let nw = other.size[0];
            let nh = other.size[1];

            if !x_snapped {
                let candidates = [nx, nx + nw / 2.0, nx + nw];
                let drag_refs = [(snapped.x, 0.0f32), (snapped.x + dw / 2.0, -dw / 2.0), (snapped.x + dw, -dw)];
                'x_outer: for &cand in &candidates {
                    for &(ref_x, offset) in &drag_refs {
                        if (ref_x - cand).abs() < threshold {
                            snapped.x = cand + offset;
                            guides.push((false, cand));
                            x_snapped = true;
                            break 'x_outer;
                        }
                    }
                }
            }

            if !y_snapped {
                let candidates = [ny, ny + nh / 2.0, ny + nh];
                let drag_refs = [(snapped.y, 0.0f32), (snapped.y + dh / 2.0, -dh / 2.0), (snapped.y + dh, -dh)];
                'y_outer: for &cand in &candidates {
                    for &(ref_y, offset) in &drag_refs {
                        if (ref_y - cand).abs() < threshold {
                            snapped.y = cand + offset;
                            guides.push((true, cand));
                            y_snapped = true;
                            break 'y_outer;
                        }
                    }
                }
            }

            if x_snapped && y_snapped {
                break;
            }
        }
        (snapped, guides)
    }

    fn draw_box_select_preview(&self, painter: &egui::Painter, pointer_pos: Option<Pos2>) {
        if let DragState::BoxSelect { start_canvas } = &self.drag {
            if let Some(mouse) = pointer_pos {
                let end_canvas = self.viewport.screen_to_canvas(mouse);
                let a = self.viewport.canvas_to_screen(*start_canvas);
                let b = self.viewport.canvas_to_screen(end_canvas);
                let sel_rect = Rect::from_two_pos(a, b);
                painter.rect_filled(sel_rect, CornerRadius::ZERO, BOX_SELECT_FILL);

                // Marching ants: animated dashed border
                {
                    let t = painter.ctx().input(|i| i.time);
                    let dash_len = 6.0_f32;
                    let gap_len = 4.0_f32;
                    let period = dash_len + gap_len;
                    let speed = 30.0_f32; // pixels per second
                    let phase = (t as f32 * speed) % period;

                    // Walk around the perimeter: top, right, bottom (reversed), left (reversed)
                    let corners = [
                        sel_rect.left_top(),
                        sel_rect.right_top(),
                        sel_rect.right_bottom(),
                        sel_rect.left_bottom(),
                        sel_rect.left_top(), // close
                    ];

                    let stroke = Stroke::new(1.2, BOX_SELECT_STROKE);
                    let mut dist_offset = period - phase; // start phase
                    for seg in 0..4 {
                        let a = corners[seg];
                        let b = corners[seg + 1];
                        let seg_len = (b - a).length();
                        let dir = (b - a) / seg_len;
                        let mut d = dist_offset % period;
                        // align start: skip to first dash
                        if d < gap_len { d += gap_len; } else { d -= gap_len; }
                        let pos = 0.0_f32;
                        // Draw or skip based on phase
                        let mut drawing = d < dash_len;
                        let mut cursor = -(d % period);
                        while cursor < seg_len {
                            let seg_start = cursor.max(0.0);
                            let seg_end = (cursor + if drawing { dash_len } else { gap_len }).min(seg_len);
                            if drawing && seg_end > seg_start {
                                painter.line_segment(
                                    [a + dir * seg_start, a + dir * seg_end],
                                    stroke,
                                );
                            }
                            cursor += if drawing { dash_len } else { gap_len };
                            drawing = !drawing;
                        }
                        let _ = pos; // suppress warning
                        dist_offset = (dist_offset + seg_len) % period;
                    }

                    painter.ctx().request_repaint_after(std::time::Duration::from_millis(33));
                }

                // Count nodes within box-select rect and show badge
                let start_canvas_local = *start_canvas;
                let end_canvas = self.viewport.screen_to_canvas(mouse);
                let canvas_sel_rect = Rect::from_two_pos(start_canvas_local, end_canvas);
                let count = self.document.nodes.iter().filter(|n| canvas_sel_rect.contains(n.rect().center())).count();
                if count > 0 {
                    let badge_pos = Pos2::new(sel_rect.max.x + 4.0, sel_rect.min.y);
                    let badge_text = format!("{count}");
                    painter.rect_filled(
                        Rect::from_min_size(badge_pos, Vec2::new(22.0, 16.0)),
                        CornerRadius::same(4), ACCENT,
                    );
                    painter.text(badge_pos + Vec2::new(11.0, 8.0), Align2::CENTER_CENTER,
                        &badge_text, FontId::proportional(10.0), Color32::BLACK);
                }
            }
        }
    }

    fn draw_edge_creation_preview(
        &self,
        painter: &egui::Painter,
        node_idx: &std::collections::HashMap<NodeId, usize>,
    ) {
        let DragState::CreatingEdge {
            source,
            current_screen,
        } = &self.drag
        else {
            return;
        };
        let Some(src_node) = node_idx
            .get(&source.node_id)
            .and_then(|&i| self.document.nodes.get(i))
        else {
            return;
        };

        let src_pos = src_node.port_position(source.side);
        let src_screen = self.viewport.canvas_to_screen(src_pos);
        let dst = *current_screen;
        let offset = 60.0 * self.viewport.zoom;
        let (cp1, cp2) = control_points_for_side(src_screen, dst, source.side, offset);
        let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
            [src_screen, cp1, cp2, dst],
            false,
            Color32::TRANSPARENT,
            Stroke::new(2.0, SELECTION_COLOR),
        );
        painter.add(bezier);

        // Draw all port side labels on nearby nodes to guide connection
        let canvas_dst = self.viewport.screen_to_canvas(*current_screen);
        for node in &self.document.nodes {
            if node.id == source.node_id { continue; }
            let node_screen_rect = Rect::from_min_size(
                self.viewport.canvas_to_screen(node.pos()),
                node.size_vec() * self.viewport.zoom,
            );
            // Only label if close enough (within 250px of cursor)
            if (node_screen_rect.center() - *current_screen).length() > 250.0 { continue; }
            let r = PORT_RADIUS * self.viewport.zoom.sqrt();
            for (side, label) in [
                (PortSide::Top, "T"), (PortSide::Bottom, "B"),
                (PortSide::Left, "L"), (PortSide::Right, "R"),
            ] {
                let port_canvas = node.port_position(side);
                let port_screen = self.viewport.canvas_to_screen(port_canvas);
                let near = (port_screen - *current_screen).length() < 20.0;
                let color = if near { ACCENT } else { Color32::from_rgba_unmultiplied(147, 153, 178, 160) };
                painter.circle_filled(port_screen, if near { r * 1.8 } else { r }, color);
                if near || self.viewport.zoom > 0.7 {
                    let offset = match side {
                        PortSide::Top    => Vec2::new(0.0, -12.0),
                        PortSide::Bottom => Vec2::new(0.0,  12.0),
                        PortSide::Left   => Vec2::new(-12.0, 0.0),
                        PortSide::Right  => Vec2::new( 12.0, 0.0),
                    };
                    painter.text(port_screen + offset, Align2::CENTER_CENTER, label,
                        FontId::proportional(9.0), color);
                }
            }
        }

        // Highlight target port with name badge
        if let Some(target_port) = self.hit_test_port(canvas_dst) {
            if target_port.node_id != source.node_id {
                if let Some(tgt_node) = node_idx
                    .get(&target_port.node_id)
                    .and_then(|&i| self.document.nodes.get(i))
                {
                    let port_pos = self
                        .viewport
                        .canvas_to_screen(tgt_node.port_position(target_port.side));
                    let r = PORT_RADIUS * self.viewport.zoom.sqrt() * 2.0;
                    painter.circle_filled(port_pos, r * 1.5, ACCENT_SELECT_BG);
                    painter.circle_filled(port_pos, r, ACCENT);
                    painter.circle_stroke(port_pos, r, Stroke::new(2.0, Color32::WHITE));
                    // Port name badge
                    let side_name = match target_port.side {
                        PortSide::Top => "Top", PortSide::Bottom => "Bottom",
                        PortSide::Left => "Left", PortSide::Right => "Right",
                    };
                    let badge_pos = port_pos + Vec2::new(0.0, r * 2.2);
                    let badge_w = side_name.len() as f32 * 5.5 + 10.0;
                    let badge_rect = Rect::from_center_size(badge_pos, Vec2::new(badge_w, 16.0));
                    painter.rect_filled(badge_rect, CornerRadius::same(4), ACCENT);
                    painter.text(badge_pos, Align2::CENTER_CENTER, side_name,
                        FontId::proportional(9.5), Color32::BLACK);
                }
            }
        }
    }

    fn draw_new_node_preview(&self, painter: &egui::Painter, canvas_rect: Rect) {
        let DragState::DraggingNewNode {
            kind,
            current_screen,
        } = &self.drag
        else {
            return;
        };
        if !canvas_rect.contains(*current_screen) {
            return;
        }

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
        painter.rect_filled(screen_rect, CornerRadius::same(4), PREVIEW_FILL);
        painter.rect_stroke(
            screen_rect,
            CornerRadius::same(4),
            Stroke::new(1.5, SELECTION_COLOR),
            StrokeKind::Outside,
        );
    }

    fn draw_status_toast(
        &self,
        painter: &egui::Painter,
        canvas_rect: Rect,
        ctx: &egui::Context,
    ) {
        if let Some((ref msg, time)) = self.status_message {
            let elapsed = time.elapsed().as_secs_f32();
            if elapsed < 2.0 {
                let alpha = ((2.0 - elapsed) * 255.0).min(255.0) as u8;
                let toast_pos = Pos2::new(canvas_rect.center().x, canvas_rect.max.y - 40.0);
                let font = FontId::proportional(12.0);

                // Measure text to draw pill background
                let galley = painter.layout_no_wrap(msg.clone(), font.clone(), TOAST_SUCCESS);
                let pill_rect = Rect::from_center_size(
                    toast_pos,
                    Vec2::new(galley.size().x + 24.0, galley.size().y + 12.0),
                );
                let bg_alpha = (alpha as f32 * 0.85) as u8;
                painter.rect_filled(
                    pill_rect,
                    CornerRadius::same(16),
                    Color32::from_rgba_premultiplied(24, 24, 37, bg_alpha),
                );
                painter.rect_stroke(
                    pill_rect,
                    CornerRadius::same(16),
                    Stroke::new(
                        1.0,
                        Color32::from_rgba_premultiplied(166, 227, 161, bg_alpha / 2),
                    ),
                    StrokeKind::Outside,
                );

                // Checkmark + text
                let check = "\u{2713} ";
                painter.text(
                    toast_pos,
                    Align2::CENTER_CENTER,
                    &format!("{}{}", check, msg),
                    font,
                    Color32::from_rgba_premultiplied(166, 227, 161, alpha),
                );
                ctx.request_repaint();
            }
        }
    }

    // --- Node tooltip ---

    fn draw_edge_tooltip(
        &self,
        painter: &egui::Painter,
        hover_pos: Option<Pos2>,
        canvas_rect: Rect,
        node_idx: &std::collections::HashMap<NodeId, usize>,
    ) {
        let Some(mouse) = hover_pos else { return };
        if !canvas_rect.contains(mouse) { return }

        // Don't show edge tooltip if we're over a node
        let canvas_pos = self.viewport.screen_to_canvas(mouse);
        if self.document.node_at_pos(canvas_pos).is_some() { return; }

        // Find hovered edge
        let Some(edge_id) = self.hit_test_edge(canvas_pos) else { return };
        let Some(edge) = self.document.find_edge(&edge_id) else { return };

        let src_name = node_idx.get(&edge.source.node_id)
            .and_then(|&i| self.document.nodes.get(i))
            .map(|n| n.display_label())
            .unwrap_or("?");
        let tgt_name = node_idx.get(&edge.target.node_id)
            .and_then(|&i| self.document.nodes.get(i))
            .map(|n| n.display_label())
            .unwrap_or("?");

        let line = if edge.label.is_empty() {
            format!("{} → {}", src_name, tgt_name)
        } else {
            format!("{} →[{}]→ {}", src_name, edge.label, tgt_name)
        };

        let font = egui::FontId::proportional(11.0);
        let pad = 8.0;
        let w = 220.0_f32;
        let h = 26.0_f32;
        let mut tx = mouse.x + 12.0;
        let mut ty = mouse.y - h - 6.0;
        if tx + w > canvas_rect.max.x { tx = mouse.x - w - 12.0; }
        if ty < canvas_rect.min.y { ty = mouse.y + 12.0; }
        let bg_rect = Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(w, h));
        painter.rect_filled(bg_rect, egui::CornerRadius::same(4), TOOLTIP_BG);
        painter.rect_stroke(bg_rect, egui::CornerRadius::same(4), egui::Stroke::new(1.0, TOOLTIP_BORDER), egui::StrokeKind::Outside);
        painter.text(Pos2::new(tx + pad, ty + h / 2.0), Align2::LEFT_CENTER, &line, font, TEXT_SECONDARY);
    }

    fn draw_node_tooltip(&self, painter: &egui::Painter, hover_pos: Option<Pos2>, canvas_rect: Rect) {
        let Some(mouse) = hover_pos else { return };
        if !canvas_rect.contains(mouse) { return }

        // Determine hover duration to decide tooltip richness
        let now = painter.ctx().input(|i| i.time);
        let hover_duration = self.hover_node_start
            .and_then(|(_, start)| Some(now - start))
            .unwrap_or(0.0);

        // Find hovered node
        let hovered_node = self.document.nodes.iter().rev().find(|node| {
            let top_left = self.viewport.canvas_to_screen(node.pos());
            let sr = Rect::from_min_size(top_left, node.size_vec() * self.viewport.zoom);
            sr.contains(mouse)
        });
        let Some(node) = hovered_node else { return };

        // Basic tooltip: show if node has a description (immediate)
        let desc = match &node.kind {
            crate::model::NodeKind::Shape { description, .. } => description.as_str(),
            _ => "",
        };
        let label = node.display_label();

        // Only show if there's something to show
        let rich_mode = hover_duration > 0.8;
        let has_basic = !desc.is_empty();
        if !has_basic && !rich_mode { return; }

        let max_w = 240.0;
        let pad = 10.0;

        // Collect rich data
        let conn_in  = self.document.edges.iter().filter(|e| e.target.node_id == node.id).count();
        let conn_out = self.document.edges.iter().filter(|e| e.source.node_id == node.id).count();
        let has_url     = !node.url.is_empty();
        let has_comment = !node.comment.is_empty();
        let tag_label   = node.tag.map(|t| t.label());

        // Build rows to render
        let mut rows: Vec<(String, Color32)> = Vec::new();
        if !desc.is_empty() {
            rows.push((desc.to_string(), TEXT_DIM));
        }
        if rich_mode {
            if conn_in > 0 || conn_out > 0 {
                rows.push((format!("↑{} in  ↓{} out", conn_in, conn_out), TEXT_DIM));
            }
            if let Some(tag) = tag_label {
                rows.push((format!("Tag: {}", tag), TEXT_DIM));
            }
            if has_url  { rows.push(("🔗 URL attached".to_string(), TEXT_DIM)); }
            if has_comment { rows.push(("💬 Has comment".to_string(), TEXT_DIM)); }
            if node.locked { rows.push(("🔒 Locked".to_string(), TEXT_DIM)); }
        }
        if rows.is_empty() && !rich_mode { return; }

        let line_h = 14.0_f32;
        let header_h = 16.0_f32;
        let total_h = pad * 2.0 + header_h + (rows.len() as f32) * line_h + if !rows.is_empty() { 4.0 } else { 0.0 };

        let mut tx = mouse.x + 14.0;
        let mut ty = mouse.y + 14.0;
        if tx + max_w + pad > canvas_rect.max.x { tx = mouse.x - max_w - 14.0; }
        if ty + total_h > canvas_rect.max.y { ty = mouse.y - total_h - 14.0; }

        // Fade-in effect when switching to rich mode
        let alpha_factor = if rich_mode {
            ((hover_duration - 0.8) / 0.3).clamp(0.0, 1.0) as f32
        } else { 1.0 };

        let bg = TOOLTIP_BG;
        let border_col = if rich_mode {
            Color32::from_rgba_unmultiplied(137, 180, 250, (80.0 * alpha_factor) as u8)
        } else {
            TOOLTIP_BORDER
        };

        let bg_rect = Rect::from_min_size(Pos2::new(tx, ty), egui::Vec2::new(max_w, total_h));
        painter.rect_filled(bg_rect, egui::CornerRadius::same(6), bg);
        painter.rect_stroke(bg_rect, egui::CornerRadius::same(6),
            egui::Stroke::new(1.0, border_col), egui::StrokeKind::Outside);

        // Label header
        painter.text(Pos2::new(tx + pad, ty + pad), egui::Align2::LEFT_TOP, &label,
            egui::FontId::proportional(12.0), TEXT_SECONDARY);

        // Separator line when rich
        if rich_mode && !rows.is_empty() {
            painter.line_segment(
                [Pos2::new(tx + pad, ty + pad + header_h + 1.0),
                 Pos2::new(tx + max_w - pad, ty + pad + header_h + 1.0)],
                egui::Stroke::new(0.5, Color32::from_rgba_unmultiplied(100, 100, 140, (60.0 * alpha_factor) as u8)),
            );
        }

        // Detail rows
        for (i, (text, color)) in rows.iter().enumerate() {
            let row_color = Color32::from_rgba_unmultiplied(
                color.r(), color.g(), color.b(), (color.a() as f32 * alpha_factor) as u8,
            );
            painter.text(
                Pos2::new(tx + pad, ty + pad + header_h + 4.0 + i as f32 * line_h),
                egui::Align2::LEFT_TOP, text,
                egui::FontId::proportional(10.5), row_color,
            );
        }
    }

    // --- Canvas HUD ---

    fn draw_empty_canvas_hint(&self, painter: &egui::Painter, canvas_rect: Rect) {
        if !self.document.nodes.is_empty() { return; }
        let center = canvas_rect.center();
        let t = painter.ctx().input(|i| i.time) as f32;

        // Animated outer ring (breathing)
        let ring_r = 52.0 + ((t * 1.2).sin() * 5.0);
        let ring_alpha = (((t * 1.2).sin() * 0.5 + 0.5) * 50.0 + 15.0) as u8;
        painter.circle_stroke(center, ring_r, Stroke::new(1.0,
            Color32::from_rgba_unmultiplied(137, 180, 250, ring_alpha)));

        // Second ring (offset phase)
        let ring2_r = 38.0 + (((t * 1.2 + 1.0).sin()) * 3.0);
        let ring2_alpha = ((( t * 1.2 + 1.0).sin() * 0.5 + 0.5) * 40.0 + 10.0) as u8;
        painter.circle_stroke(center, ring2_r, Stroke::new(0.7,
            Color32::from_rgba_unmultiplied(137, 180, 250, ring2_alpha)));

        // Central "+" circle button
        let btn_r = 26.0_f32;
        painter.circle_filled(center, btn_r, Color32::from_rgba_unmultiplied(137, 180, 250, 25));
        painter.circle_stroke(center, btn_r, Stroke::new(1.5,
            Color32::from_rgba_unmultiplied(137, 180, 250, 140)));
        painter.text(center, Align2::CENTER_CENTER, "+",
            FontId::proportional(28.0),
            Color32::from_rgba_unmultiplied(137, 180, 250, 200));

        // Title below
        painter.text(center + Vec2::new(0.0, btn_r + 16.0), Align2::CENTER_CENTER,
            "Double-click anywhere to add a node",
            FontId::proportional(11.5),
            TEXT_DIM);

        // Keyboard shortcut hints
        let hints = [
            ("N", "new shape node"),
            ("D", "double-click to create"),
            ("E", "edge connect tool"),
            ("⌘Z", "undo"),
        ];
        for (i, (key, desc)) in hints.iter().enumerate() {
            let y = center.y + btn_r + 40.0 + i as f32 * 15.0;
            painter.text(Pos2::new(center.x - 60.0, y), Align2::LEFT_CENTER,
                *key, FontId::proportional(10.5), ACCENT.gamma_multiply(0.7));
            painter.text(Pos2::new(center.x - 38.0, y), Align2::LEFT_CENTER,
                *desc, FontId::proportional(10.5), TEXT_DIM.gamma_multiply(0.6));
        }

        painter.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }

    fn draw_canvas_hud(&self, painter: &egui::Painter, canvas_rect: Rect, pointer_pos: Option<Pos2>) {
        let zoom_pct = (self.viewport.zoom * 100.0).round() as i32;
        let n_nodes = self.document.nodes.len();
        let n_edges = self.document.edges.len();
        let n_sel_n = self.selection.node_ids.len();
        let n_sel_e = self.selection.edge_ids.len();

        let line1 = format!("{zoom_pct}%");
        let line2 = if n_sel_n > 0 || n_sel_e > 0 {
            let mut parts = Vec::new();
            if n_sel_n > 0 { parts.push(format!("{}N", n_sel_n)); }
            if n_sel_e > 0 { parts.push(format!("{}E", n_sel_e)); }
            // Compute selection bounding box
            let bb = self.selection.node_ids.iter()
                .filter_map(|id| self.document.find_node(id))
                .fold(Option::<egui::Rect>::None, |acc, n| {
                    Some(acc.map_or(n.rect(), |r| r.union(n.rect())))
                });
            let size_str = bb.map(|r| format!("  {:.0}×{:.0}", r.width(), r.height())).unwrap_or_default();
            // Show node-of-total when single node is selected
            let node_idx_str = if n_sel_n == 1 && n_sel_e == 0 {
                let sel_id = *self.selection.node_ids.iter().next().unwrap();
                let idx = self.document.nodes.iter().position(|n| n.id == sel_id).unwrap_or(0);
                format!("  ({}/{})", idx + 1, n_nodes)
            } else { String::new() };
            format!("{} sel{}{}  ·  {}N {}E", parts.join("+"), size_str, node_idx_str, n_nodes, n_edges)
        } else {
            format!("{}N  {}E", n_nodes, n_edges)
        };
        let line3 = {
            let mut parts = Vec::new();
            if self.show_grid { parts.push("grid".to_string()); }
            if self.snap_to_grid { parts.push("snap".to_string()); }
            if self.canvas_locked { parts.push("🔒".to_string()); }
            if self.focus_mode { parts.push("focus".to_string()); }
            let u = self.history.undo_steps();
            let r = self.history.redo_steps();
            if u > 0 || r > 0 {
                parts.push(format!("↩{u}↪{r}"));
            }
            parts.join(" · ")
        };

        // Cursor world coordinates (shown while dragging or when Shift held)
        let cursor_line: Option<String> = pointer_pos.and_then(|sp| {
            let is_dragging = !matches!(self.drag, DragState::None);
            // Only show cursor coords while dragging or during edge creation
            if is_dragging {
                let cp = self.viewport.screen_to_canvas(sp);
                Some(format!("x:{:.0}  y:{:.0}", cp.x, cp.y))
            } else {
                None
            }
        });

        let hud_lines = 2 + (!line3.is_empty() as usize) + (cursor_line.is_some() as usize);
        let line_h = 12.0;
        let pad = 8.0;
        let x = canvas_rect.min.x + pad;
        let y = canvas_rect.max.y - (hud_lines as f32) * line_h - 12.0;

        let font_big = egui::FontId::proportional(15.0);
        let font_sm  = egui::FontId::proportional(10.5);
        let font_xs  = egui::FontId::proportional(9.5);

        painter.text(Pos2::new(x, y), egui::Align2::LEFT_TOP, &line1, font_big, TEXT_SECONDARY);
        painter.text(Pos2::new(x, y + 17.0), egui::Align2::LEFT_TOP, &line2, font_sm, TEXT_DIM);
        let mut next_y = y + 29.0;
        if !line3.is_empty() {
            painter.text(Pos2::new(x, next_y), egui::Align2::LEFT_TOP, &line3, font_xs, TEXT_DIM);
            next_y += 11.0;
        }
        if let Some(ref cl) = cursor_line {
            painter.text(Pos2::new(x, next_y), egui::Align2::LEFT_TOP, cl,
                egui::FontId::proportional(9.5),
                Color32::from_rgba_unmultiplied(137, 220, 235, 160)); // cyan tint
        }
    }

    /// Subtle vignette gradient around the canvas edges — gives a "paper on desk" depth.
    /// Uses a mesh-based gradient on each of the 4 edges.
    fn draw_canvas_vignette(&self, painter: &egui::Painter, canvas_rect: Rect) {
        let depth = 48.0_f32; // pixels of fade
        let alpha = 60_u8;    // max opacity at edge
        let color_edge = Color32::from_rgba_premultiplied(10, 10, 20, alpha);
        let color_mid  = Color32::from_rgba_premultiplied(10, 10, 20, 0);

        // Helper: draw a gradient quad between two points with two color stops
        let grad_quad = |p: &egui::Painter, corners: [Pos2; 4], c0: Color32, c1: Color32| {
            let mut mesh = egui::Mesh::default();
            // corners: [outer0, outer1, inner1, inner0] — outer=dark, inner=transparent
            mesh.vertices.push(egui::epaint::Vertex { pos: corners[0], uv: Pos2::ZERO, color: c0 });
            mesh.vertices.push(egui::epaint::Vertex { pos: corners[1], uv: Pos2::ZERO, color: c0 });
            mesh.vertices.push(egui::epaint::Vertex { pos: corners[2], uv: Pos2::ZERO, color: c1 });
            mesh.vertices.push(egui::epaint::Vertex { pos: corners[3], uv: Pos2::ZERO, color: c1 });
            mesh.indices = vec![0, 1, 2, 0, 2, 3];
            p.add(egui::Shape::mesh(mesh));
        };

        let r = canvas_rect;
        // Top fade
        grad_quad(painter, [
            r.left_top(), r.right_top(),
            Pos2::new(r.max.x, r.min.y + depth),
            Pos2::new(r.min.x, r.min.y + depth),
        ], color_edge, color_mid);
        // Bottom fade
        grad_quad(painter, [
            r.left_bottom(), r.right_bottom(),
            Pos2::new(r.max.x, r.max.y - depth),
            Pos2::new(r.min.x, r.max.y - depth),
        ], color_edge, color_mid);
        // Left fade
        grad_quad(painter, [
            r.left_top(), r.left_bottom(),
            Pos2::new(r.min.x + depth, r.max.y),
            Pos2::new(r.min.x + depth, r.min.y),
        ], color_edge, color_mid);
        // Right fade
        grad_quad(painter, [
            r.right_top(), r.right_bottom(),
            Pos2::new(r.max.x - depth, r.max.y),
            Pos2::new(r.max.x - depth, r.min.y),
        ], color_edge, color_mid);
    }

    /// Draw pulsing glow overlays on all edges connected to `hovered_node`.
    /// Also draws a small in/out degree badge above the node.
    fn draw_hover_edge_glow(
        &self,
        painter: &egui::Painter,
        node_idx: &std::collections::HashMap<NodeId, usize>,
        hovered_node: NodeId,
        time: f32,
        canvas_rect: Rect,
    ) {
        // Pulse: glow width breathes between 0.6–1.0 strength
        let pulse = 0.8 + 0.2 * (time * 3.0).sin();

        let mut in_deg = 0usize;
        let mut out_deg = 0usize;

        for edge in &self.document.edges {
            let is_out = edge.source.node_id == hovered_node;
            let is_in  = edge.target.node_id == hovered_node;
            if !is_out && !is_in { continue; }

            if is_out { out_deg += 1; }
            if is_in  { in_deg  += 1; }

            let src_node = node_idx.get(&edge.source.node_id).and_then(|&i| self.document.nodes.get(i));
            let tgt_node = node_idx.get(&edge.target.node_id).and_then(|&i| self.document.nodes.get(i));
            let (sn, tn) = match (src_node, tgt_node) { (Some(s), Some(t)) => (s, t), _ => continue };

            let src = self.viewport.canvas_to_screen(sn.port_position(edge.source.side));
            let tgt = self.viewport.canvas_to_screen(tn.port_position(edge.target.side));
            let offset = 60.0 * self.viewport.zoom;
            let (mut cp1, mut cp2) = control_points_for_side(src, tgt, edge.source.side, offset);
            if edge.style.curve_bend.abs() > 0.1 {
                let dir = if (tgt - src).length() > 1.0 { (tgt - src).normalized() } else { Vec2::X };
                let perp = Vec2::new(-dir.y, dir.x);
                cp1 = cp1 + perp * edge.style.curve_bend * self.viewport.zoom;
                cp2 = cp2 + perp * edge.style.curve_bend * self.viewport.zoom;
            }

            // Direction-coded color: outgoing = blue accent, incoming = green
            let glow_color = if is_out {
                Color32::from_rgba_premultiplied(137, 180, 250, (80.0 * pulse) as u8)
            } else {
                Color32::from_rgba_premultiplied(166, 227, 161, (80.0 * pulse) as u8)
            };
            let core_color = if is_out {
                Color32::from_rgba_premultiplied(137, 180, 250, (200.0 * pulse) as u8)
            } else {
                Color32::from_rgba_premultiplied(166, 227, 161, (200.0 * pulse) as u8)
            };

            // Outer glow
            painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                [src, cp1, cp2, tgt], false, Color32::TRANSPARENT,
                Stroke::new(10.0 * self.viewport.zoom.sqrt(), glow_color),
            ));
            // Inner bright line
            painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                [src, cp1, cp2, tgt], false, Color32::TRANSPARENT,
                Stroke::new(2.5, core_color),
            ));
        }

        // Draw in/out degree badge near the hovered node
        if let Some(&idx) = node_idx.get(&hovered_node) {
            if let Some(node) = self.document.nodes.get(idx) {
                let node_screen = self.viewport.canvas_to_screen(node.pos());
                let node_size_s = node.size_vec() * self.viewport.zoom;
                let badge_pos = Pos2::new(
                    node_screen.x + node_size_s.x / 2.0,
                    node_screen.y - 22.0,
                );
                if canvas_rect.contains(badge_pos) {
                    let text = format!("↑{out_deg}  ↓{in_deg}");
                    let font = egui::FontId::proportional(10.5);
                    let text_size = painter.ctx().fonts(|f| f.layout_no_wrap(
                        text.clone(), font.clone(), Color32::WHITE,
                    ).size());
                    let bg_rect = Rect::from_center_size(
                        badge_pos,
                        egui::vec2(text_size.x + 12.0, text_size.y + 6.0),
                    );
                    painter.rect_filled(bg_rect, CornerRadius::same(4), Color32::from_rgba_premultiplied(20, 20, 35, 210));
                    painter.rect_stroke(bg_rect, CornerRadius::same(4), Stroke::new(1.0, SURFACE1), StrokeKind::Outside);
                    painter.text(
                        badge_pos,
                        egui::Align2::CENTER_CENTER,
                        &text,
                        font,
                        Color32::from_rgb(205, 214, 244),
                    );
                }
            }
        }
    }

    fn draw_path_highlight(
        &self,
        edge: &Edge,
        painter: &egui::Painter,
        node_idx: &std::collections::HashMap<NodeId, usize>,
    ) {
        let src_node = node_idx.get(&edge.source.node_id).and_then(|&i| self.document.nodes.get(i));
        let tgt_node = node_idx.get(&edge.target.node_id).and_then(|&i| self.document.nodes.get(i));
        let (sn, tn) = match (src_node, tgt_node) { (Some(s), Some(t)) => (s, t), _ => return };
        let src = self.viewport.canvas_to_screen(sn.port_position(edge.source.side));
        let tgt = self.viewport.canvas_to_screen(tn.port_position(edge.target.side));
        let offset = 60.0 * self.viewport.zoom;
        let (mut cp1, mut cp2) = control_points_for_side(src, tgt, edge.source.side, offset);
        if edge.style.curve_bend.abs() > 0.1 {
            let dir = if (tgt - src).length() > 1.0 { (tgt - src).normalized() } else { Vec2::X };
            let perp = Vec2::new(-dir.y, dir.x);
            let bend_screen = edge.style.curve_bend * self.viewport.zoom;
            cp1 = cp1 + perp * bend_screen;
            cp2 = cp2 + perp * bend_screen;
        }
        let highlight_color = Color32::from_rgba_premultiplied(250, 179, 135, 200);
        let glow = egui::epaint::CubicBezierShape::from_points_stroke(
            [src, cp1, cp2, tgt],
            false,
            Color32::TRANSPARENT,
            Stroke::new(8.0 * self.viewport.zoom.sqrt(), highlight_color.gamma_multiply(0.3)),
        );
        painter.add(glow);
        let path_line = egui::epaint::CubicBezierShape::from_points_stroke(
            [src, cp1, cp2, tgt],
            false,
            Color32::TRANSPARENT,
            Stroke::new(2.5 * self.viewport.zoom.sqrt(), highlight_color),
        );
        painter.add(path_line);
    }

    // --- Rulers ---

    fn draw_floating_action_bar(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        // Edge floating action bar (when exactly 1 edge selected, no nodes)
        if self.selection.node_ids.is_empty() && self.selection.edge_ids.len() == 1 {
            let edge_id = *self.selection.edge_ids.iter().next().unwrap();
            if let Some(edge) = self.document.find_edge(&edge_id) {
                // Find midpoint on screen for bar positioning
                let node_idx: std::collections::HashMap<NodeId, usize> = self.document.nodes.iter().enumerate()
                    .map(|(i, n)| (n.id, i)).collect();
                let src_pos = node_idx.get(&edge.source.node_id)
                    .and_then(|&i| self.document.nodes.get(i))
                    .map(|n| self.viewport.canvas_to_screen(n.rect().center()))
                    .unwrap_or(canvas_rect.center());
                let tgt_pos = node_idx.get(&edge.target.node_id)
                    .and_then(|&i| self.document.nodes.get(i))
                    .map(|n| self.viewport.canvas_to_screen(n.rect().center()))
                    .unwrap_or(canvas_rect.center());
                let mid = Pos2::new((src_pos.x + tgt_pos.x) / 2.0, (src_pos.y + tgt_pos.y) / 2.0);

                let edge_actions: &[(&str, &str)] = &[
                    ("⇄", "Reverse direction"),
                    ("✏", "Edit label"),
                    ("⎘", "Duplicate edge"),
                    ("🗑", "Delete"),
                ];
                let btn_w = 28.0_f32;
                let bar_h = 26.0_f32;
                let bar_w = edge_actions.len() as f32 * btn_w + (edge_actions.len() - 1) as f32 * 2.0 + 8.0;
                let bar_x = (mid.x - bar_w / 2.0).clamp(canvas_rect.min.x + 4.0, canvas_rect.max.x - bar_w - 4.0);
                let bar_y = (mid.y - bar_h - 10.0).max(canvas_rect.min.y + 4.0);
                let bar_rect = Rect::from_min_size(Pos2::new(bar_x, bar_y), Vec2::new(bar_w, bar_h));

                let painter2 = ui.painter();
                painter2.rect_filled(bar_rect, CornerRadius::same(6), TOOLTIP_BG);
                painter2.rect_stroke(bar_rect, CornerRadius::same(6), Stroke::new(1.0, ACCENT.gamma_multiply(0.4)), StrokeKind::Outside);

                let mut ex = bar_rect.min.x + 4.0;
                let mut edge_clicked: Option<usize> = None;
                for (i, (icon, tip)) in edge_actions.iter().enumerate() {
                    let btn_r = Rect::from_min_size(Pos2::new(ex, bar_rect.min.y + 2.0), Vec2::new(btn_w, bar_h - 4.0));
                    let resp = ui.put(btn_r, egui::Button::new(
                        egui::RichText::new(*icon).size(12.0)
                    ).frame(false)).on_hover_text(*tip);
                    if resp.clicked() { edge_clicked = Some(i); }
                    ex += btn_w + 2.0;
                }
                let _ = edge; // end borrow before mutable operations below

                if let Some(action) = edge_clicked {
                    match action {
                        0 => { // Reverse
                            if let Some(e) = self.document.find_edge_mut(&edge_id) {
                                let old_src = e.source.clone();
                                let old_tgt = e.target.clone();
                                e.source = old_tgt;
                                e.target = old_src;
                            }
                            self.history.push(&self.document);
                            self.status_message = Some(("Edge reversed".to_string(), std::time::Instant::now()));
                        }
                        1 => { self.focus_label_edit = true; }
                        2 => { // Duplicate edge
                            if let Some(e) = self.document.find_edge(&edge_id).cloned() {
                                let mut copy = e;
                                copy.id = EdgeId::new();
                                copy.style.curve_bend += 30.0;
                                self.document.edges.push(copy);
                                self.history.push(&self.document);
                            }
                        }
                        3 => { // Delete
                            self.document.remove_edge(&edge_id);
                            self.selection.clear();
                            self.history.push(&self.document);
                        }
                        _ => {}
                    }
                }
            }
            return; // don't show node bar when only edge is selected
        }

        if self.selection.node_ids.is_empty() { return; }
        // Compute bounding box of selected nodes in screen space
        let bb = self.selection.node_ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .fold(Option::<Rect>::None, |acc, n| {
                let sr = Rect::from_min_size(self.viewport.canvas_to_screen(n.pos()), n.size_vec() * self.viewport.zoom);
                Some(acc.map_or(sr, |r| r.union(sr)))
            });
        let Some(bb) = bb else { return };

        let bar_h = 28.0;
        let bar_margin = 8.0;
        let bar_y = (bb.min.y - bar_h - bar_margin).max(canvas_rect.min.y + 4.0);
        let bar_center_x = bb.center().x.clamp(canvas_rect.min.x + 80.0, canvas_rect.max.x - 80.0);

        // Build the bar as a horizontal layout
        let actions: &[(&str, &str)] = &[
            ("✏", "Edit label"),
            ("⎘", "Duplicate (⌘D)"),
            ("📋", "Copy style (⌘⇧C)"),
            ("🗑", "Delete"),
        ];
        let btn_w = 28.0;
        let bar_w = actions.len() as f32 * btn_w + (actions.len() - 1) as f32 * 2.0 + 8.0;
        let bar_rect = Rect::from_center_size(
            egui::Pos2::new(bar_center_x, bar_y + bar_h / 2.0),
            egui::Vec2::new(bar_w, bar_h),
        );

        // Draw bar background via painter
        let painter = ui.painter();
        painter.rect_filled(bar_rect, CornerRadius::same(6), TOOLTIP_BG);
        painter.rect_stroke(bar_rect, CornerRadius::same(6), Stroke::new(1.0, SURFACE1), StrokeKind::Outside);

        // Draw buttons using ui.put()
        let pad = 4.0;
        let mut x = bar_rect.min.x + pad;
        let mut clicked_action: Option<usize> = None;
        for (i, (icon, tooltip)) in actions.iter().enumerate() {
            let btn_rect = Rect::from_min_size(
                egui::Pos2::new(x, bar_rect.min.y + 2.0),
                egui::Vec2::new(btn_w, bar_h - 4.0),
            );
            let resp = ui.put(btn_rect, egui::Button::new(
                egui::RichText::new(*icon).size(13.0)
            ).frame(false)).on_hover_text(*tooltip);
            if resp.clicked() { clicked_action = Some(i); }
            x += btn_w + 2.0;
        }

        // Handle clicked actions
        if let Some(action) = clicked_action {
            match action {
                0 => { self.focus_label_edit = true; }
                1 => { // Duplicate
                    let offset = egui::Vec2::new(24.0, 24.0);
                    let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                    let originals: Vec<crate::model::Node> = ids.iter().filter_map(|id| self.document.find_node(id).cloned()).collect();
                    self.selection.clear();
                    for mut node in originals {
                        node.id = NodeId::new();
                        node.set_pos(node.pos() + offset);
                        self.selection.node_ids.insert(node.id);
                        self.document.nodes.push(node);
                    }
                    self.history.push(&self.document);
                }
                2 => { // Copy style
                    if let Some(id) = self.selection.node_ids.iter().next() {
                        if let Some(node) = self.document.find_node(id) {
                            self.style_clipboard = Some(node.style.clone());
                            self.status_message = Some(("Style copied".to_string(), std::time::Instant::now()));
                        }
                    }
                }
                3 => { // Delete
                    let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                    for id in &ids { self.document.remove_node(id); }
                    self.selection.clear();
                    self.history.push(&self.document);
                }
                _ => {}
            }
        }

        // Multi-select alignment bar (only when 2+ nodes selected)
        if self.selection.node_ids.len() >= 2 {
            let align_actions: &[(&str, &str)] = &[
                ("⬛", "Align left edges"),
                ("⬛", "Align centers H"),
                ("⬛", "Align right edges"),
                ("▬", "Align top edges"),
                ("▬", "Align middles V"),
                ("▬", "Align bottom edges"),
                ("↔", "Distribute H"),
                ("↕", "Distribute V"),
            ];
            // Actually use distinct icons
            let align_icons: &[(&str, &str)] = &[
                ("◧", "Align left edges"),
                ("◫", "Align centers H"),
                ("◨", "Align right edges"),
                ("⬒", "Align top edges"),
                ("⬓", "Align middles V"),
                ("⬗", "Align bottom edges"),
                ("↔", "Distribute H (⇧H)"),
                ("↕", "Distribute V (⇧V)"),
            ];
            let _ = align_actions;
            let abtn_w = 24.0_f32;
            let abar_h = 24.0_f32;
            let abar_w = align_icons.len() as f32 * abtn_w + (align_icons.len() - 1) as f32 * 2.0 + 8.0;
            let abar_y = bar_rect.max.y + 4.0;
            let abar_rect = Rect::from_center_size(
                egui::Pos2::new(bar_center_x, abar_y + abar_h / 2.0),
                egui::Vec2::new(abar_w, abar_h),
            );

            let painter2 = ui.painter();
            painter2.rect_filled(abar_rect, CornerRadius::same(6), TOOLTIP_BG);
            painter2.rect_stroke(abar_rect, CornerRadius::same(6), Stroke::new(1.0, ACCENT.gamma_multiply(0.5)), StrokeKind::Outside);

            let mut ax = abar_rect.min.x + 4.0;
            let mut align_clicked: Option<usize> = None;
            for (i, (icon, tip)) in align_icons.iter().enumerate() {
                let btn_r = Rect::from_min_size(
                    egui::Pos2::new(ax, abar_rect.min.y + 2.0),
                    egui::Vec2::new(abtn_w, abar_h - 4.0),
                );
                let resp = ui.put(btn_r, egui::Button::new(
                    egui::RichText::new(*icon).size(11.0)
                ).frame(false)).on_hover_text(*tip);
                if resp.clicked() { align_clicked = Some(i); }
                ax += abtn_w + 2.0;
            }

            if let Some(idx) = align_clicked {
                let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                let positions: Vec<(NodeId, Pos2, Vec2)> = ids.iter()
                    .filter_map(|id| self.document.find_node(id).map(|n| (*id, n.pos(), n.size_vec())))
                    .collect();

                match idx {
                    0 => { // Align left
                        let min_x = positions.iter().map(|(_,p,_)| p.x).fold(f32::MAX, f32::min);
                        for (id, pos, _) in &positions {
                            if let Some(n) = self.document.find_node_mut(id) { n.set_pos(Pos2::new(min_x, pos.y)); }
                        }
                    }
                    1 => { // Align center H
                        let avg_cx = positions.iter().map(|(_,p,s)| p.x + s.x/2.0).sum::<f32>() / positions.len() as f32;
                        for (id, _, s) in &positions {
                            let new_x = avg_cx - s.x/2.0;
                            if let Some(n) = self.document.find_node_mut(id) { let y = n.pos().y; n.set_pos(Pos2::new(new_x, y)); }
                        }
                    }
                    2 => { // Align right
                        let max_right = positions.iter().map(|(_,p,s)| p.x + s.x).fold(f32::MIN, f32::max);
                        for (id, _, s) in &positions {
                            let new_x = max_right - s.x;
                            if let Some(n) = self.document.find_node_mut(id) { let y = n.pos().y; n.set_pos(Pos2::new(new_x, y)); }
                        }
                    }
                    3 => { // Align top
                        let min_y = positions.iter().map(|(_,p,_)| p.y).fold(f32::MAX, f32::min);
                        for (id, pos, _) in &positions {
                            if let Some(n) = self.document.find_node_mut(id) { n.set_pos(Pos2::new(pos.x, min_y)); }
                        }
                    }
                    4 => { // Align middle V
                        let avg_cy = positions.iter().map(|(_,p,s)| p.y + s.y/2.0).sum::<f32>() / positions.len() as f32;
                        for (id, _, s) in &positions {
                            let new_y = avg_cy - s.y/2.0;
                            if let Some(n) = self.document.find_node_mut(id) { let x = n.pos().x; n.set_pos(Pos2::new(x, new_y)); }
                        }
                    }
                    5 => { // Align bottom
                        let max_bottom = positions.iter().map(|(_,p,s)| p.y + s.y).fold(f32::MIN, f32::max);
                        for (id, _, s) in &positions {
                            let new_y = max_bottom - s.y;
                            if let Some(n) = self.document.find_node_mut(id) { let x = n.pos().x; n.set_pos(Pos2::new(x, new_y)); }
                        }
                    }
                    6 => { // Distribute H
                        let mut sorted: Vec<_> = positions.iter().collect();
                        sorted.sort_by(|a,b| a.1.x.partial_cmp(&b.1.x).unwrap_or(std::cmp::Ordering::Equal));
                        if sorted.len() >= 2 {
                            let left = sorted[0].1.x;
                            let right = sorted[sorted.len()-1].1.x + sorted[sorted.len()-1].2.x;
                            let total_w: f32 = sorted.iter().map(|(_,_,s)| s.x).sum();
                            let gap = (right - left - total_w) / (sorted.len() - 1) as f32;
                            let mut cx = left;
                            for (id, _, s) in &sorted {
                                if let Some(n) = self.document.find_node_mut(id) { let y = n.pos().y; n.set_pos(Pos2::new(cx, y)); }
                                cx += s.x + gap;
                            }
                        }
                    }
                    7 => { // Distribute V
                        let mut sorted: Vec<_> = positions.iter().collect();
                        sorted.sort_by(|a,b| a.1.y.partial_cmp(&b.1.y).unwrap_or(std::cmp::Ordering::Equal));
                        if sorted.len() >= 2 {
                            let top = sorted[0].1.y;
                            let bottom = sorted[sorted.len()-1].1.y + sorted[sorted.len()-1].2.y;
                            let total_h: f32 = sorted.iter().map(|(_,_,s)| s.y).sum();
                            let gap = (bottom - top - total_h) / (sorted.len() - 1) as f32;
                            let mut cy = top;
                            for (id, _, s) in &sorted {
                                if let Some(n) = self.document.find_node_mut(id) { let x = n.pos().x; n.set_pos(Pos2::new(x, cy)); }
                                cy += s.y + gap;
                            }
                        }
                    }
                    _ => {}
                }
                if align_clicked.is_some() {
                    self.history.push(&self.document);
                    self.status_message = Some((align_icons[idx].1.to_string(), std::time::Instant::now()));
                }
            }
        }
    }

    fn draw_project_title(&self, painter: &egui::Painter, canvas_rect: Rect) {
        if self.project_title.is_empty() { return; }
        let font = FontId::proportional(13.0);
        let color = Color32::from_rgba_premultiplied(180, 180, 200, 100);
        let pos = Pos2::new(canvas_rect.min.x + 20.0, canvas_rect.min.y + 20.0);
        painter.text(pos, Align2::LEFT_TOP, &self.project_title, font, color);
    }

    fn draw_quick_notes_panel(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        // Floating "sticky" quick-notes panel in top-left area
        let panel_w = 200.0_f32;
        let panel_h = 160.0_f32;
        let margin = 16.0_f32;
        let panel_rect = Rect::from_min_size(
            Pos2::new(canvas_rect.min.x + margin, canvas_rect.min.y + margin + 14.0),
            Vec2::new(panel_w, panel_h),
        );

        // Draw sticky note background (warm yellow)
        let painter2 = ui.painter();
        painter2.rect_filled(
            panel_rect,
            CornerRadius::same(6),
            Color32::from_rgba_unmultiplied(249, 226, 175, 230),
        );
        painter2.rect_stroke(
            panel_rect,
            CornerRadius::same(6),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(200, 170, 90, 180)),
            StrokeKind::Outside,
        );

        // Folded corner effect (top-right triangle)
        let corner = panel_rect.right_top();
        let fold_size = 14.0_f32;
        painter2.add(egui::Shape::convex_polygon(
            vec![
                corner,
                corner + Vec2::new(-fold_size, 0.0),
                corner + Vec2::new(0.0, fold_size),
            ],
            Color32::from_rgba_unmultiplied(200, 160, 80, 200),
            Stroke::NONE,
        ));

        // Title bar
        painter2.text(
            panel_rect.left_top() + Vec2::new(8.0, 8.0),
            Align2::LEFT_TOP,
            "📝 Quick Notes  (⇧P)",
            FontId::proportional(9.5),
            Color32::from_rgba_unmultiplied(100, 70, 20, 200),
        );

        // Text area
        let text_rect = Rect::from_min_max(
            panel_rect.min + Vec2::new(6.0, 24.0),
            panel_rect.max - Vec2::new(6.0, 6.0),
        );
        ui.put(text_rect, egui::TextEdit::multiline(&mut self.quick_notes_text)
            .desired_width(text_rect.width())
            .font(FontId::proportional(10.5))
            .frame(false)
            .text_color(Color32::from_rgba_unmultiplied(80, 55, 15, 230))
        );
    }

    fn draw_heatmap_overlay(&self, painter: &egui::Painter, canvas_rect: Rect) {
        // Count degree (in + out edges) per node
        let mut degree: std::collections::HashMap<NodeId, usize> = std::collections::HashMap::new();
        for node in &self.document.nodes {
            degree.entry(node.id).or_insert(0);
        }
        for edge in &self.document.edges {
            *degree.entry(edge.source.node_id).or_insert(0) += 1;
            *degree.entry(edge.target.node_id).or_insert(0) += 1;
        }
        let max_deg = degree.values().copied().max().unwrap_or(1).max(1) as f32;

        for node in &self.document.nodes {
            if node.is_frame { continue; }
            let deg = *degree.get(&node.id).unwrap_or(&0) as f32;
            let t = (deg / max_deg).clamp(0.0, 1.0); // 0 = cool, 1 = hot

            let screen_pos = self.viewport.canvas_to_screen(node.pos());
            let screen_size = node.size_vec() * self.viewport.zoom;
            let screen_rect = Rect::from_min_size(screen_pos, screen_size);
            if !screen_rect.intersects(canvas_rect) { continue; }

            // Interpolate color: blue(0) → green(0.5) → orange(0.75) → red(1.0)
            let heat_color = if t < 0.5 {
                let s = t * 2.0;
                Color32::from_rgba_unmultiplied(
                    (30.0 + s * 100.0) as u8,
                    (100.0 + s * 127.0) as u8,
                    (220.0 - s * 180.0) as u8,
                    100,
                )
            } else {
                let s = (t - 0.5) * 2.0;
                Color32::from_rgba_unmultiplied(
                    (130.0 + s * 125.0) as u8,
                    (227.0 - s * 180.0) as u8,
                    40,
                    110,
                )
            };

            // Draw a colored ring around the node
            let ring_width = (2.0 + t * 5.0) * self.viewport.zoom.sqrt();
            painter.rect_stroke(
                screen_rect.expand(ring_width * 0.5),
                CornerRadius::same(6),
                Stroke::new(ring_width, heat_color),
                StrokeKind::Outside,
            );

            // Degree label badge in top-left
            if self.viewport.zoom > 0.4 {
                let deg_i = deg as usize;
                let badge = format!("{deg_i}");
                let bp = screen_pos + Vec2::new(-2.0, -2.0);
                painter.rect_filled(
                    Rect::from_min_size(bp - Vec2::new(2.0, 2.0), Vec2::new(16.0, 12.0)),
                    CornerRadius::same(3),
                    heat_color,
                );
                painter.text(bp + Vec2::new(6.0, 4.0), Align2::CENTER_CENTER,
                    &badge, FontId::proportional(8.0), Color32::WHITE);
            }
        }

        // Legend in bottom-right
        let leg_w = 120.0_f32;
        let leg_h = 20.0_f32;
        let leg_pos = Pos2::new(canvas_rect.max.x - leg_w - 20.0, canvas_rect.max.y - leg_h - 16.0);
        painter.text(leg_pos, Align2::LEFT_TOP, "Connectivity:", FontId::proportional(9.0), TEXT_DIM);
        let bar_y = leg_pos.y + 11.0;
        for i in 0..=60 {
            let t = i as f32 / 60.0;
            let color = if t < 0.5 {
                let s = t * 2.0;
                Color32::from_rgba_unmultiplied((30.0 + s*100.0) as u8, (100.0+s*127.0) as u8, (220.0-s*180.0) as u8, 180)
            } else {
                let s = (t - 0.5) * 2.0;
                Color32::from_rgba_unmultiplied((130.0+s*125.0) as u8, (227.0-s*180.0) as u8, 40, 180)
            };
            let x = leg_pos.x + t * leg_w;
            painter.rect_filled(Rect::from_min_size(Pos2::new(x, bar_y), Vec2::new(leg_w/60.0+0.5, 7.0)), CornerRadius::ZERO, color);
        }
        painter.text(leg_pos + Vec2::new(0.0, 12.0), Align2::LEFT_TOP, "low", FontId::proportional(7.0), TEXT_DIM);
        painter.text(leg_pos + Vec2::new(leg_w, 12.0), Align2::RIGHT_TOP, "high", FontId::proportional(7.0), TEXT_DIM);
        painter.text(Pos2::new(canvas_rect.max.x - 12.0, canvas_rect.max.y - 30.0),
            Align2::RIGHT_BOTTOM, "[H] heatmap", FontId::proportional(8.0), TEXT_DIM.gamma_multiply(0.6));
    }

    fn draw_presentation_spotlight(&self, painter: &egui::Painter, canvas_rect: Rect, pointer_pos: Option<Pos2>) {
        // Soft radial vignette centered on cursor; fallback to canvas center
        let center = pointer_pos.unwrap_or(canvas_rect.center());
        let radius = (canvas_rect.width().min(canvas_rect.height()) * 0.38).max(200.0);

        // Draw vignette as a mesh: concentric rings from transparent near center to dark at edges
        // We approximate with painter.add(Shape::mesh) using vertex colors
        use egui::{epaint::{Mesh, Vertex}, Shape};
        let dark = Color32::from_rgba_unmultiplied(0, 0, 0, 160);
        let transparent = Color32::TRANSPARENT;

        // Corners of the canvas get darkened; region near cursor stays clear
        // Approach: draw 4 corner triangulated quads that fade from dark at canvas corners to transparent near spotlight
        // Simpler approach: draw dark overlay rect, then punch a gradient circle on top using blending via many small quads

        // Actually render as a set of thin concentric "ring" polys isn't easy without custom shaders.
        // Use additive approach: draw dark-filled canvas rect, then draw bright circle with gamma_multiply trick.
        // Since egui compositing is additive/alpha, we do:
        //   1. Semi-transparent dark overlay over whole canvas
        //   2. Bright transparent "erase" circle — achieved by drawing a slightly lighter circle on top
        // This isn't true vignette but gives a useful spotlight visual.

        // Layer 1: dark overlay
        painter.rect_filled(canvas_rect, CornerRadius::ZERO, dark);

        // Layer 2: "spotlight" — lighter circle at cursor to lift the darkness
        // Use a mesh with 32 segments, alpha = 0 at center fading to 0 at edge (i.e., a hole in the dark)
        // We can't "erase" in egui, so instead we lighten by drawing a bright overlay with ZERO at edge
        // Strategy: draw gradient circle from white/transparent at center to fully transparent at radius
        // Combined with the dark overlay, this creates a spotlight feel.
        let segments = 48usize;
        let mut mesh = Mesh::default();

        // Center vertex: bright (lifts dark overlay)
        let center_color = Color32::from_rgba_unmultiplied(30, 30, 50, 0); // transparent at center
        mesh.vertices.push(Vertex { pos: center, uv: egui::pos2(0.0, 0.0), color: center_color });

        for i in 0..=segments {
            let angle = i as f32 / segments as f32 * std::f32::consts::TAU;
            let p = center + Vec2::new(angle.cos(), angle.sin()) * radius;
            let t = 1.0_f32; // edge: fully dark (matches overlay, no additional brightening)
            let _ = t;
            mesh.vertices.push(Vertex { pos: p, uv: egui::pos2(0.0, 0.0), color: transparent });
        }

        // Indices: triangle fan from center (vertex 0) to ring vertices
        for i in 0..segments as u32 {
            mesh.indices.extend_from_slice(&[0, i + 1, i + 2]);
        }

        // Paint the "spotlight hole" — this is still dark, but punches through the overlay.
        // Because egui alpha-blends, this transparent region lets the actual canvas show through more.
        // To actually lighten, we need a bright-colored center — use a subtle white with partial alpha:
        for v in mesh.vertices.iter_mut() {
            if (v.pos - center).length() < 1.0 {
                v.color = Color32::from_rgba_unmultiplied(255, 255, 240, 0);
            }
        }

        painter.add(Shape::mesh(mesh));

        // Soft glow ring at cursor
        let glow_color = Color32::from_rgba_unmultiplied(255, 255, 200, 25);
        painter.circle_filled(center, radius * 0.12, glow_color);
        painter.circle_filled(center, radius * 0.06, Color32::from_rgba_unmultiplied(255, 255, 220, 40));

        // Cursor circle indicator
        painter.circle_stroke(center, 18.0, Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 240, 180, 160)));
        painter.circle_stroke(center, 4.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 240, 180, 220)));

        // "PRESENT" badge in top-left
        let badge_pos = Pos2::new(canvas_rect.min.x + 12.0, canvas_rect.min.y + 12.0);
        painter.rect_filled(
            Rect::from_min_size(badge_pos, Vec2::new(58.0, 16.0)),
            CornerRadius::same(4),
            Color32::from_rgba_unmultiplied(200, 60, 60, 200),
        );
        painter.text(
            badge_pos + Vec2::new(29.0, 8.0),
            Align2::CENTER_CENTER,
            "● PRESENT",
            FontId::proportional(9.0),
            Color32::WHITE,
        );
    }

    fn draw_rulers(&self, painter: &egui::Painter, canvas_rect: Rect) {
        let zoom = self.viewport.zoom;
        // Choose ruler tick interval: a nice round number in world space
        let raw_interval = 100.0_f32 / zoom;
        let magnitude = 10_f32.powf(raw_interval.log10().floor());
        let normalized = raw_interval / magnitude;
        let interval = if normalized < 1.5 { magnitude }
            else if normalized < 3.5 { 2.0 * magnitude }
            else { 5.0 * magnitude };
        let interval_screen = interval * zoom;
        if interval_screen < 20.0 { return; }

        let ruler_h = 12.0_f32;
        let ruler_color = SURFACE0;
        let tick_color = TEXT_DIM;
        let label_font = egui::FontId::proportional(8.5);

        // Horizontal ruler along top
        painter.rect_filled(
            Rect::from_min_size(canvas_rect.min, Vec2::new(canvas_rect.width(), ruler_h)),
            egui::CornerRadius::ZERO, ruler_color,
        );

        // Vertical ruler along left
        painter.rect_filled(
            Rect::from_min_size(canvas_rect.min, Vec2::new(ruler_h, canvas_rect.height())),
            egui::CornerRadius::ZERO, ruler_color,
        );

        // Horizontal ticks
        let world_start_x = ((canvas_rect.min.x - self.viewport.offset[0]) / zoom / interval).floor() * interval;
        let mut wx = world_start_x;
        while self.viewport.canvas_to_screen(Pos2::new(wx, 0.0)).x < canvas_rect.max.x {
            let sx = self.viewport.canvas_to_screen(Pos2::new(wx, 0.0)).x;
            if sx > canvas_rect.min.x + ruler_h {
                let is_major = (wx / interval).round() as i32 % 5 == 0;
                let tick_h = if is_major { ruler_h } else { ruler_h * 0.5 };
                painter.line_segment(
                    [Pos2::new(sx, canvas_rect.min.y), Pos2::new(sx, canvas_rect.min.y + tick_h)],
                    egui::Stroke::new(0.5, tick_color),
                );
                if is_major {
                    painter.text(
                        Pos2::new(sx + 2.0, canvas_rect.min.y + 1.0),
                        Align2::LEFT_TOP,
                        &format!("{}", wx as i32),
                        label_font.clone(),
                        tick_color,
                    );
                }
            }
            wx += interval;
        }

        // Vertical ticks
        let world_start_y = ((canvas_rect.min.y - self.viewport.offset[1]) / zoom / interval).floor() * interval;
        let mut wy = world_start_y;
        while self.viewport.canvas_to_screen(Pos2::new(0.0, wy)).y < canvas_rect.max.y {
            let sy = self.viewport.canvas_to_screen(Pos2::new(0.0, wy)).y;
            if sy > canvas_rect.min.y + ruler_h {
                let is_major = (wy / interval).round() as i32 % 5 == 0;
                let tick_w = if is_major { ruler_h } else { ruler_h * 0.5 };
                painter.line_segment(
                    [Pos2::new(canvas_rect.min.x, sy), Pos2::new(canvas_rect.min.x + tick_w, sy)],
                    egui::Stroke::new(0.5, tick_color),
                );
                if is_major {
                    // Rotate text 90° by drawing char by char is complex; just draw a short number
                    painter.text(
                        Pos2::new(canvas_rect.min.x + 1.0, sy - 1.0),
                        Align2::LEFT_BOTTOM,
                        &format!("{}", wy as i32),
                        label_font.clone(),
                        tick_color,
                    );
                }
            }
            wy += interval;
        }

        // Corner box
        painter.rect_filled(
            Rect::from_min_size(canvas_rect.min, Vec2::splat(ruler_h)),
            egui::CornerRadius::ZERO, SURFACE1,
        );
    }

    fn draw_ruler_crosshair(&self, painter: &egui::Painter, canvas_rect: Rect, pointer_pos: Option<Pos2>) {
        let Some(mouse) = pointer_pos else { return };
        if !canvas_rect.contains(mouse) { return; }

        let ruler_h = 12.0_f32;
        let cross_color = Color32::from_rgba_unmultiplied(255, 80, 80, 120);
        let cross_stroke = egui::Stroke::new(1.0, cross_color);

        // Vertical line at cursor X (full canvas height below the ruler)
        if mouse.x > canvas_rect.min.x + ruler_h {
            painter.line_segment(
                [
                    Pos2::new(mouse.x, canvas_rect.min.y + ruler_h),
                    Pos2::new(mouse.x, canvas_rect.max.y),
                ],
                egui::Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 80, 80, 35)),
            );
            // Cursor notch on horizontal ruler
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(mouse.x - 1.5, canvas_rect.min.y),
                    Vec2::new(3.0, ruler_h),
                ),
                egui::CornerRadius::ZERO,
                Color32::from_rgba_unmultiplied(255, 80, 80, 200),
            );
            // World coordinate label on horizontal ruler
            let wx = (mouse.x - self.viewport.offset[0]) / self.viewport.zoom;
            painter.text(
                Pos2::new(mouse.x + 3.0, canvas_rect.min.y + 1.5),
                Align2::LEFT_TOP,
                &format!("{:.0}", wx),
                FontId::proportional(7.5),
                Color32::from_rgba_unmultiplied(255, 160, 160, 230),
            );
        }

        // Horizontal line at cursor Y (full canvas width right of the ruler)
        if mouse.y > canvas_rect.min.y + ruler_h {
            painter.line_segment(
                [
                    Pos2::new(canvas_rect.min.x + ruler_h, mouse.y),
                    Pos2::new(canvas_rect.max.x, mouse.y),
                ],
                egui::Stroke::new(0.5, Color32::from_rgba_unmultiplied(255, 80, 80, 35)),
            );
            // Cursor notch on vertical ruler
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(canvas_rect.min.x, mouse.y - 1.5),
                    Vec2::new(ruler_h, 3.0),
                ),
                egui::CornerRadius::ZERO,
                Color32::from_rgba_unmultiplied(255, 80, 80, 200),
            );
            // World coordinate label on vertical ruler
            let wy = (mouse.y - self.viewport.offset[1]) / self.viewport.zoom;
            painter.text(
                Pos2::new(canvas_rect.min.x + 1.0, mouse.y + 3.0),
                Align2::LEFT_TOP,
                &format!("{:.0}", wy),
                FontId::proportional(7.5),
                Color32::from_rgba_unmultiplied(255, 160, 160, 230),
            );
        }

        let _ = cross_color; let _ = cross_stroke; // satisfy checker
    }

    // --- Grid ---

    fn draw_grid(&self, painter: &egui::Painter, canvas_rect: Rect) {
        let zoom = self.viewport.zoom;
        let grid_screen = self.grid_size * zoom;

        if grid_screen < 8.0 {
            return;
        }

        let max_dots = 5000;
        let cols = (canvas_rect.width() / grid_screen) as usize;
        let rows = (canvas_rect.height() / grid_screen) as usize;
        if cols * rows > max_dots {
            return;
        }

        let offset_x = self.viewport.offset[0] % grid_screen;
        let offset_y = self.viewport.offset[1] % grid_screen;

        let start_x = canvas_rect.min.x + offset_x;
        let start_y = canvas_rect.min.y + offset_y;

        // Major grid every 5 minor cells
        let major_every = 5_i32;
        let major_grid_screen = grid_screen * major_every as f32;
        let major_offset_x = self.viewport.offset[0] % major_grid_screen;
        let major_offset_y = self.viewport.offset[1] % major_grid_screen;
        let _major_start_x = canvas_rect.min.x + major_offset_x;
        let _major_start_y = canvas_rect.min.y + major_offset_y;

        match self.bg_pattern {
            super::BgPattern::None => {}
            super::BgPattern::Dots => {
                // Minor dots
                let mut xi = 0_i32;
                let mut x = start_x;
                while x < canvas_rect.max.x {
                    let mut yi = 0_i32;
                    let mut y = start_y;
                    while y < canvas_rect.max.y {
                        let is_major = (xi % major_every == 0) && (yi % major_every == 0);
                        let (r, c) = if is_major { (1.8, GRID_MAJOR_COLOR) } else { (0.8, GRID_COLOR) };
                        painter.circle_filled(Pos2::new(x, y), r, c);
                        y += grid_screen;
                        yi += 1;
                    }
                    x += grid_screen;
                    xi += 1;
                }
            }
            super::BgPattern::Lines => {
                // Horizontal lines with major emphasis
                let mut y = start_y;
                let mut yi = 0_i32;
                while y < canvas_rect.max.y {
                    let is_major = yi % major_every == 0;
                    let stroke = if is_major {
                        egui::Stroke::new(0.7, GRID_MAJOR_COLOR)
                    } else {
                        egui::Stroke::new(0.4, GRID_COLOR)
                    };
                    painter.line_segment(
                        [Pos2::new(canvas_rect.min.x, y), Pos2::new(canvas_rect.max.x, y)],
                        stroke,
                    );
                    y += grid_screen;
                    yi += 1;
                }
            }
            super::BgPattern::Crosshatch => {
                let mut y = start_y;
                let mut yi = 0_i32;
                while y < canvas_rect.max.y {
                    let is_major = yi % major_every == 0;
                    let stroke = if is_major {
                        egui::Stroke::new(0.7, GRID_MAJOR_COLOR)
                    } else {
                        egui::Stroke::new(0.4, GRID_COLOR)
                    };
                    painter.line_segment(
                        [Pos2::new(canvas_rect.min.x, y), Pos2::new(canvas_rect.max.x, y)],
                        stroke,
                    );
                    y += grid_screen;
                    yi += 1;
                }
                let mut x = start_x;
                let mut xi = 0_i32;
                while x < canvas_rect.max.x {
                    let is_major = xi % major_every == 0;
                    let stroke = if is_major {
                        egui::Stroke::new(0.7, GRID_MAJOR_COLOR)
                    } else {
                        egui::Stroke::new(0.4, GRID_COLOR)
                    };
                    painter.line_segment(
                        [Pos2::new(x, canvas_rect.min.y), Pos2::new(x, canvas_rect.max.y)],
                        stroke,
                    );
                    x += grid_screen;
                    xi += 1;
                }
            }
        }
    }

    // --- Minimap ---

    fn draw_zoom_presets(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        // Small interactive zoom chips at bottom-left, just above HUD text
        let presets: &[(f32, &str)] = &[(0.5, "50%"), (1.0, "100%"), (2.0, "200%")];
        let y = canvas_rect.max.y - 72.0;
        let mut x = canvas_rect.min.x + 8.0;
        for (zoom, label) in presets {
            let chip_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(36.0, 14.0));
            let is_active = (self.viewport.zoom - zoom).abs() < 0.05;
            let color = if is_active { ACCENT } else { TEXT_DIM };
            let resp = ui.interact(chip_rect, egui::Id::new(("zoom_preset", label)), egui::Sense::click());
            if resp.clicked() {
                let center = self.canvas_rect.center();
                let old_zoom = self.viewport.zoom;
                self.viewport.zoom = *zoom;
                let ratio = self.viewport.zoom / old_zoom;
                self.viewport.offset[0] = center.x - ratio * (center.x - self.viewport.offset[0]);
                self.viewport.offset[1] = center.y - ratio * (center.y - self.viewport.offset[1]);
            }
            ui.painter().text(
                chip_rect.center(), Align2::CENTER_CENTER, *label,
                FontId::proportional(9.5),
                if resp.hovered() { TEXT_SECONDARY } else { color },
            );
            x += 40.0;
        }
    }

    fn draw_search_overlay(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        if !self.show_search { return; }

        // Collect matching results
        let q = self.search_query.to_lowercase();
        let max_results = 8_usize;
        let results: Vec<(NodeId, String)> = if q.is_empty() {
            Vec::new()
        } else {
            self.document.nodes.iter()
                .filter(|n| n.display_label().to_lowercase().contains(&q))
                .take(max_results)
                .map(|n| (n.id, n.display_label().to_string()))
                .collect()
        };

        // Clamp cursor
        if !results.is_empty() && self.search_cursor >= results.len() {
            self.search_cursor = results.len() - 1;
        }

        let w = 320.0_f32;
        let input_h = 38.0_f32;
        let row_h = 30.0_f32;
        let results_h = results.len() as f32 * row_h;
        let total_h = input_h + results_h;

        let top = canvas_rect.min.y + 50.0;
        let overlay_rect = Rect::from_min_size(
            Pos2::new(canvas_rect.center().x - w / 2.0, top),
            Vec2::new(w, total_h.max(input_h)),
        );

        // Background panel
        {
            let painter = ui.painter().clone();
            painter.rect_filled(overlay_rect, CornerRadius::same(10), TOOLTIP_BG);
            painter.rect_stroke(overlay_rect, CornerRadius::same(10),
                Stroke::new(1.0, ACCENT.gamma_multiply(0.4)), StrokeKind::Outside);
            // Search icon
            painter.text(
                Pos2::new(overlay_rect.min.x + 12.0, overlay_rect.min.y + input_h / 2.0),
                Align2::LEFT_CENTER, "🔍",
                FontId::proportional(13.0), TEXT_DIM,
            );
            // Divider between input and results
            if !results.is_empty() {
                painter.line_segment(
                    [Pos2::new(overlay_rect.min.x + 12.0, top + input_h),
                     Pos2::new(overlay_rect.max.x - 12.0, top + input_h)],
                    Stroke::new(0.5, SURFACE1),
                );
            }
        }

        // Search input field (offset right of icon)
        let input_rect = Rect::from_min_size(
            Pos2::new(overlay_rect.min.x + 30.0, overlay_rect.min.y + 2.0),
            Vec2::new(w - 50.0, input_h - 4.0),
        );
        let mut ui2 = ui.new_child(
            egui::UiBuilder::new().max_rect(input_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center))
        );
        let resp = ui2.add(
            egui::TextEdit::singleline(&mut self.search_query)
                .hint_text("Search nodes…")
                .desired_width(w - 60.0)
                .font(egui::FontId::proportional(14.0))
                .frame(false),
        );
        resp.request_focus();

        let ctx = ui2.ctx().clone();

        // Keyboard navigation within results
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) && !results.is_empty() {
            self.search_cursor = (self.search_cursor + 1).min(results.len() - 1);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) && self.search_cursor > 0 {
            self.search_cursor -= 1;
        }

        // Close on Escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_search = false;
            self.search_query.clear();
            self.search_cursor = 0;
            return;
        }

        // Jump to result on Enter
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(&(nid, _)) = results.get(self.search_cursor).or_else(|| results.first()) {
                self.selection.select_node(nid);
                self.zoom_to_selection();
            } else if !results.is_empty() {
                // Select all matches
                self.selection.clear();
                for (nid, _) in &results { self.selection.node_ids.insert(*nid); }
                self.zoom_to_selection();
            }
            self.show_search = false;
            self.search_query.clear();
            self.search_cursor = 0;
            return;
        }

        // Result count badge
        {
            let badge = if results.is_empty() && !q.is_empty() { "0".to_string() }
                        else if !results.is_empty() { format!("{}", results.len()) }
                        else { String::new() };
            if !badge.is_empty() {
                ui2.painter().text(
                    Pos2::new(overlay_rect.max.x - 8.0, overlay_rect.min.y + input_h / 2.0),
                    Align2::RIGHT_CENTER, &badge,
                    FontId::proportional(11.0), TEXT_DIM,
                );
            }
        }

        // Render result rows
        for (i, (nid, label)) in results.iter().enumerate() {
            let row_y = top + input_h + i as f32 * row_h;
            let row_rect = Rect::from_min_size(
                Pos2::new(overlay_rect.min.x, row_y),
                Vec2::new(w, row_h),
            );
            let is_highlighted = i == self.search_cursor;
            let resp = ui.allocate_rect(row_rect, egui::Sense::click());
            let is_hov = resp.hovered();

            let bg = if is_highlighted { ACCENT.gamma_multiply(0.18) }
                     else if is_hov    { Color32::from_rgba_unmultiplied(137, 180, 250, 12) }
                     else              { Color32::TRANSPARENT };
            ui.painter().rect_filled(row_rect, CornerRadius::ZERO, bg);

            // Truncate label
            let short: String = label.chars().take(36).collect();
            let trail = if label.chars().count() > 36 { "…" } else { "" };
            let disp = format!("{}{}", short, trail);
            ui.painter().text(
                Pos2::new(row_rect.min.x + 14.0, row_rect.center().y),
                Align2::LEFT_CENTER, &disp,
                FontId::proportional(12.5),
                if is_highlighted { TEXT_PRIMARY } else { TEXT_SECONDARY },
            );

            if resp.clicked() {
                self.selection.select_node(*nid);
                self.zoom_to_selection();
                self.show_search = false;
                self.search_query.clear();
                self.search_cursor = 0;
            }
            if is_hov { self.search_cursor = i; }
        }
    }

    fn handle_minimap_click(&mut self, click_pos: Pos2, canvas_rect: Rect) {
        if self.document.nodes.is_empty() { return; }

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

        if !minimap_rect.contains(click_pos) { return; }

        // Compute world bounding box (same as draw_minimap)
        let mut bb_min = Pos2::new(f32::MAX, f32::MAX);
        let mut bb_max = Pos2::new(f32::MIN, f32::MIN);
        for node in &self.document.nodes {
            let r = node.rect();
            bb_min.x = bb_min.x.min(r.min.x);
            bb_min.y = bb_min.y.min(r.min.y);
            bb_max.x = bb_max.x.max(r.max.x);
            bb_max.y = bb_max.y.max(r.max.y);
        }
        let padding = 50.0;
        bb_min.x -= padding; bb_min.y -= padding;
        bb_max.x += padding; bb_max.y += padding;
        let bb_w = (bb_max.x - bb_min.x).max(1.0);
        let bb_h = (bb_max.y - bb_min.y).max(1.0);

        let inset = 4.0;
        let draw_rect = minimap_rect.shrink(inset);
        let draw_w = draw_rect.width();
        let draw_h = draw_rect.height();
        let scale = (draw_w / bb_w).min(draw_h / bb_h);
        let content_w = bb_w * scale;
        let content_h = bb_h * scale;
        let offset_x = draw_rect.min.x + (draw_w - content_w) / 2.0;
        let offset_y = draw_rect.min.y + (draw_h - content_h) / 2.0;

        // Invert map_point: minimap screen → world
        let world_x = bb_min.x + (click_pos.x - offset_x) / scale;
        let world_y = bb_min.y + (click_pos.y - offset_y) / scale;

        // Smoothly fly viewport so (world_x, world_y) is at canvas center
        let c = canvas_rect.center();
        self.pan_target = Some([
            c.x - world_x * self.viewport.zoom,
            c.y - world_y * self.viewport.zoom,
        ]);
    }

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

        painter.rect_filled(minimap_rect, CornerRadius::same(6), MINIMAP_BG);
        painter.rect_stroke(
            minimap_rect,
            CornerRadius::same(6),
            Stroke::new(1.0, MINIMAP_BORDER),
            StrokeKind::Outside,
        );

        // Minimap label
        painter.text(
            Pos2::new(minimap_rect.min.x + 8.0, minimap_rect.min.y + 10.0),
            Align2::LEFT_CENTER,
            "MINIMAP",
            FontId::proportional(8.0),
            MINIMAP_BORDER,
        );

        let mut bb_min = Pos2::new(f32::MAX, f32::MAX);
        let mut bb_max = Pos2::new(f32::MIN, f32::MIN);
        for node in &self.document.nodes {
            let r = node.rect();
            bb_min.x = bb_min.x.min(r.min.x);
            bb_min.y = bb_min.y.min(r.min.y);
            bb_max.x = bb_max.x.max(r.max.x);
            bb_max.y = bb_max.y.max(r.max.y);
        }

        let padding = 50.0;
        bb_min.x -= padding;
        bb_min.y -= padding;
        bb_max.x += padding;
        bb_max.y += padding;

        let bb_w = (bb_max.x - bb_min.x).max(1.0);
        let bb_h = (bb_max.y - bb_min.y).max(1.0);

        let inset = 4.0;
        let draw_rect = minimap_rect.shrink(inset);
        let draw_w = draw_rect.width();
        let draw_h = draw_rect.height();

        let scale = (draw_w / bb_w).min(draw_h / bb_h);

        let content_w = bb_w * scale;
        let content_h = bb_h * scale;
        let offset_x = draw_rect.min.x + (draw_w - content_w) / 2.0;
        let offset_y = draw_rect.min.y + (draw_h - content_h) / 2.0;

        let map_point = |cx: f32, cy: f32| -> Pos2 {
            Pos2::new(
                offset_x + (cx - bb_min.x) * scale,
                offset_y + (cy - bb_min.y) * scale,
            )
        };

        // Draw edges first (behind nodes)
        let edge_color_mm = Color32::from_rgba_unmultiplied(100, 110, 130, 120);
        let edge_sel_col  = Color32::from_rgba_unmultiplied(137, 180, 250, 200);
        for edge in &self.document.edges {
            let src_node = self.document.find_node(&edge.source.node_id);
            let tgt_node = self.document.find_node(&edge.target.node_id);
            if let (Some(sn), Some(tn)) = (src_node, tgt_node) {
                let sp = sn.port_position(edge.source.side);
                let tp = tn.port_position(edge.target.side);
                let ms = map_point(sp.x, sp.y);
                let mt = map_point(tp.x, tp.y);
                if minimap_rect.contains(ms) || minimap_rect.contains(mt) {
                    let col = if self.selection.contains_edge(&edge.id) { edge_sel_col } else { edge_color_mm };
                    painter.line_segment([ms, mt], Stroke::new(0.8, col));
                }
            }
        }

        for node in &self.document.nodes {
            let r = node.rect();
            let min_pt = map_point(r.min.x, r.min.y);
            let max_pt = map_point(r.max.x, r.max.y);
            let mini_rect = Rect::from_two_pos(min_pt, max_pt);
            let is_selected = self.selection.contains_node(&node.id);
            // Frame nodes: very transparent
            let node_color = if node.is_frame {
                Color32::from_rgba_unmultiplied(89, 91, 118, 50)
            } else if is_selected {
                ACCENT
            } else if let Some(tag) = node.tag {
                to_color32(tag.color())
            } else {
                MINIMAP_NODE
            };
            let cr_val = (mini_rect.width().min(mini_rect.height()) * 0.2) as u8;
            if mini_rect.area() > 2.0 {
                painter.rect_filled(mini_rect, egui::CornerRadius::same(cr_val), node_color);
                // Show abbreviated label if rect is large enough
                if mini_rect.width() > 18.0 && mini_rect.height() > 7.0 {
                    let label = node.display_label();
                    let short: String = label.chars().take(8).collect();
                    let font_size = (mini_rect.height() * 0.55).clamp(5.0, 8.0);
                    let text_color = Color32::from_rgba_unmultiplied(220, 220, 240, 160);
                    painter.text(
                        mini_rect.center(),
                        Align2::CENTER_CENTER,
                        &short,
                        FontId::proportional(font_size),
                        text_color,
                    );
                }
            } else {
                let center = mini_rect.center();
                if minimap_rect.contains(center) {
                    painter.circle_filled(center, if is_selected { 3.5 } else { 2.0 }, node_color);
                }
            }
        }

        let vp_tl = self.viewport.screen_to_canvas(canvas_rect.min);
        let vp_br = self.viewport.screen_to_canvas(canvas_rect.max);
        let vp_min = map_point(vp_tl.x, vp_tl.y);
        let vp_max = map_point(vp_br.x, vp_br.y);
        let vp_rect = Rect::from_two_pos(vp_min, vp_max);

        let clipped = vp_rect.intersect(minimap_rect);
        if clipped.is_positive() {
            painter.rect_filled(clipped, CornerRadius::ZERO, MINIMAP_VP_FILL);
            painter.rect_stroke(
                clipped,
                CornerRadius::ZERO,
                Stroke::new(1.0, MINIMAP_VP_STROKE),
                StrokeKind::Outside,
            );
        }

        // Draw bookmark pins on minimap
        for (slot, bm) in self.bookmarks.iter().enumerate() {
            if let Some(bv) = bm {
                // The bookmark viewport center in canvas space
                let bv_center_canvas = egui::Pos2::new(
                    (canvas_rect.center().x - bv.offset[0]) / bv.zoom,
                    (canvas_rect.center().y - bv.offset[1]) / bv.zoom,
                );
                let pin = map_point(bv_center_canvas.x, bv_center_canvas.y);
                if minimap_rect.contains(pin) {
                    // Diamond pin shape
                    let r = 5.0_f32;
                    let pin_color = Color32::from_rgb(249, 226, 175); // yellow
                    let pts = vec![
                        Pos2::new(pin.x, pin.y - r),
                        Pos2::new(pin.x + r, pin.y),
                        Pos2::new(pin.x, pin.y + r),
                        Pos2::new(pin.x - r, pin.y),
                    ];
                    painter.add(egui::Shape::convex_polygon(pts, pin_color, Stroke::NONE));
                    painter.text(
                        pin,
                        Align2::CENTER_CENTER,
                        &(slot + 1).to_string(),
                        FontId::proportional(6.0),
                        Color32::from_rgb(30, 30, 46),
                    );
                }
            }
        }
    }

    /// Draw animated data-flow dots traveling along each edge from source to target.
    fn draw_flow_animation(
        &self,
        painter: &egui::Painter,
        node_idx: &std::collections::HashMap<NodeId, usize>,
        time: f32,
        canvas_rect: Rect,
    ) {
        // Speed: 0.4 traversals/sec (dot takes 2.5s to travel the full edge)
        let speed = 0.4_f32;
        // How many dots per edge
        const DOTS_PER_EDGE: usize = 3;

        for (edge_i, edge) in self.document.edges.iter().enumerate() {
            let src_node = node_idx.get(&edge.source.node_id)
                .and_then(|&i| self.document.nodes.get(i));
            let tgt_node = node_idx.get(&edge.target.node_id)
                .and_then(|&i| self.document.nodes.get(i));
            let (src_node, tgt_node) = match (src_node, tgt_node) {
                (Some(s), Some(t)) => (s, t),
                _ => continue,
            };

            let src = self.viewport.canvas_to_screen(src_node.port_position(edge.source.side));
            let tgt = self.viewport.canvas_to_screen(tgt_node.port_position(edge.target.side));

            // Quick cull: skip edges fully outside viewport
            let edge_bounds = Rect::from_two_pos(src, tgt).expand(80.0);
            if !edge_bounds.intersects(canvas_rect) {
                continue;
            }

            let offset = 60.0 * self.viewport.zoom;
            let (mut cp1, mut cp2) = control_points_for_side(src, tgt, edge.source.side, offset);
            if edge.style.curve_bend.abs() > 0.1 {
                let dir = if (tgt - src).length() > 1.0 {
                    (tgt - src).normalized()
                } else {
                    Vec2::X
                };
                let perp = Vec2::new(-dir.y, dir.x);
                let bend_screen = edge.style.curve_bend * self.viewport.zoom;
                cp1 = cp1 + perp * bend_screen;
                cp2 = cp2 + perp * bend_screen;
            }

            // Base color from edge, brightened
            let base = to_color32(edge.style.color).gamma_multiply(1.8);
            let dot_r = (3.5 * self.viewport.zoom).clamp(2.0, 6.0);

            // Phase offset per edge so dots aren't all in sync
            let phase_offset = (edge_i as f32 * 0.37) % 1.0;

            for dot in 0..DOTS_PER_EDGE {
                // Each dot is spaced evenly and travels the curve
                let dot_phase = (dot as f32) / DOTS_PER_EDGE as f32;
                let t = ((time * speed + phase_offset + dot_phase) % 1.0).abs();

                // Fade out near start/end so dots don't pop in/out
                let fade = {
                    let ramp = 0.08_f32;
                    let fade_in  = (t / ramp).clamp(0.0, 1.0);
                    let fade_out = ((1.0 - t) / ramp).clamp(0.0, 1.0);
                    fade_in.min(fade_out)
                };
                if fade < 0.01 { continue; }

                let pos = cubic_bezier_point(src, cp1, cp2, tgt, t);

                // Skip if outside canvas rect
                if !canvas_rect.contains(pos) { continue; }

                // Outer glow ring
                let glow_color = Color32::from_rgba_premultiplied(
                    base.r(), base.g(), base.b(),
                    (80.0 * fade) as u8,
                );
                painter.circle_filled(pos, dot_r * 2.0, glow_color);

                // Inner solid dot
                let dot_color = Color32::from_rgba_premultiplied(
                    base.r(), base.g(), base.b(),
                    (220.0 * fade) as u8,
                );
                painter.circle_filled(pos, dot_r, dot_color);
            }
        }
    }

    /// Render the inline canvas label editor overlay when a node is being edited.
    fn draw_inline_node_editor(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        let node_id = match &self.inline_node_edit {
            Some((id, _)) => *id,
            None => return,
        };

        // Get the node's screen rect
        let (screen_rect, is_multiline) = match self.document.find_node(&node_id) {
            Some(node) => {
                let sr = Rect::from_min_size(
                    self.viewport.canvas_to_screen(node.pos()),
                    node.size_vec() * self.viewport.zoom,
                );
                let multi = matches!(&node.kind, NodeKind::StickyNote { .. } | NodeKind::Text { .. });
                (sr, multi)
            }
            None => {
                self.inline_node_edit = None;
                return;
            }
        };

        if !canvas_rect.intersects(screen_rect) {
            return;
        }

        // Clamp editor rect to canvas bounds and add small inset
        let edit_rect = screen_rect.shrink(6.0).intersect(canvas_rect);
        if !edit_rect.is_positive() { return; }

        // Background: semi-transparent dark overlay matching node
        let painter = ui.painter();
        painter.rect_filled(
            screen_rect,
            CornerRadius::same(4),
            Color32::from_rgba_premultiplied(20, 20, 35, 200),
        );
        painter.rect_stroke(
            screen_rect,
            CornerRadius::same(4),
            Stroke::new(2.0, ACCENT),
            StrokeKind::Outside,
        );

        // Place an egui TextEdit directly on the canvas
        let font_size = (13.0 * self.viewport.zoom).clamp(10.0, 22.0);
        let mut area = egui::Area::new(egui::Id::new("inline_node_edit"))
            .fixed_pos(edit_rect.min)
            .order(egui::Order::Foreground);

        // Prevent the area from being dragged
        area = area.interactable(true);

        area.show(ui.ctx(), |ui| {
            ui.set_width(edit_rect.width());
            ui.set_height(edit_rect.height());

            let text = match &mut self.inline_node_edit {
                Some((_, t)) => t,
                None => return,
            };

            let edit_response = if is_multiline {
                ui.add_sized(
                    edit_rect.size(),
                    egui::TextEdit::multiline(text)
                        .font(egui::FontId::proportional(font_size))
                        .frame(false)
                        .desired_width(edit_rect.width()),
                )
            } else {
                ui.add_sized(
                    edit_rect.size(),
                    egui::TextEdit::singleline(text)
                        .font(egui::FontId::proportional(font_size))
                        .frame(false)
                        .desired_width(edit_rect.width()),
                )
            };

            // Auto-focus on first frame
            if !edit_response.has_focus() {
                edit_response.request_focus();
            }

            let committed = ui.ctx().input(|i| {
                i.key_pressed(egui::Key::Enter) && (!is_multiline || !i.modifiers.shift)
            });
            let escaped = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));
            let clicked_away = ui.ctx().input(|i| i.pointer.primary_clicked())
                && !screen_rect.contains(ui.ctx().input(|i| i.pointer.interact_pos().unwrap_or(Pos2::ZERO)));

            if committed || clicked_away {
                // Commit: apply the edited text back to the node
                let final_text = text.clone();
                if let Some(node) = self.document.find_node_mut(&node_id) {
                    match &mut node.kind {
                        NodeKind::Shape { label, .. } => *label = final_text,
                        NodeKind::Text { content } => *content = final_text,
                        NodeKind::Entity { name, .. } => *name = final_text,
                        NodeKind::StickyNote { text: t, .. } => *t = final_text,
                    }
                }
                self.inline_node_edit = None;
                self.history.push(&self.document);
            } else if escaped {
                // Discard
                self.inline_node_edit = None;
            }
        });
    }

    /// Draw 4 directional arrow buttons around a hovered node.
    /// Clicking one instantly creates and connects a new node in that direction.
    fn draw_quick_connect_arrows(&mut self, ui: &mut egui::Ui, node_id: NodeId, canvas_rect: Rect) {
        let (screen_rect, shape, style) = match self.document.find_node(&node_id) {
            Some(n) => (
                Rect::from_min_size(
                    self.viewport.canvas_to_screen(n.pos()),
                    n.size_vec() * self.viewport.zoom,
                ),
                match &n.kind { NodeKind::Shape { shape, .. } => *shape, _ => NodeShape::Rectangle },
                n.style.clone(),
            ),
            None => return,
        };

        // Arrow button dimensions
        let btn_size = 20.0_f32;
        let gap      = 10.0_f32;

        // (label, screen center, port side src, port side tgt, world offset)
        let directions: &[(&str, Pos2, PortSide, PortSide, [f32; 2])] = &[
            ("→", Pos2::new(screen_rect.max.x + gap + btn_size/2.0, screen_rect.center().y),
             PortSide::Right, PortSide::Left,  [1.0,  0.0]),
            ("←", Pos2::new(screen_rect.min.x - gap - btn_size/2.0, screen_rect.center().y),
             PortSide::Left,  PortSide::Right, [-1.0, 0.0]),
            ("↓", Pos2::new(screen_rect.center().x, screen_rect.max.y + gap + btn_size/2.0),
             PortSide::Bottom, PortSide::Top,  [0.0,  1.0]),
            ("↑", Pos2::new(screen_rect.center().x, screen_rect.min.y - gap - btn_size/2.0),
             PortSide::Top,  PortSide::Bottom, [0.0, -1.0]),
        ];

        for (label, center, src_side, tgt_side, world_dir) in directions {
            let btn_rect = Rect::from_center_size(*center, Vec2::splat(btn_size));
            if !canvas_rect.contains(*center) { continue; }

            let hovered = ui.ctx().input(|i| {
                i.pointer.hover_pos().map_or(false, |p| btn_rect.contains(p))
            });
            let clicked = hovered && ui.ctx().input(|i| i.pointer.primary_clicked());

            // Draw button
            let bg = if hovered {
                Color32::from_rgba_premultiplied(137, 180, 250, 200)
            } else {
                Color32::from_rgba_premultiplied(50, 55, 80, 180)
            };
            let painter = ui.painter();
            painter.circle_filled(*center, btn_size / 2.0, bg);
            painter.circle_stroke(*center, btn_size / 2.0,
                Stroke::new(1.5, Color32::from_rgba_premultiplied(137, 180, 250, 160)));
            painter.text(*center, Align2::CENTER_CENTER, *label,
                FontId::proportional(11.0),
                if hovered { Color32::from_rgb(17, 17, 27) } else { Color32::from_rgb(137, 180, 250) });

            if clicked {
                // Compute new node position in world space
                let src_node_rect = match self.document.find_node(&node_id) {
                    Some(n) => n.rect(),
                    None => return,
                };
                let src_size = src_node_rect.size();
                let gap_world = 60.0_f32;
                let new_x = src_node_rect.min.x + world_dir[0] * (src_size.x + gap_world);
                let new_y = src_node_rect.min.y + world_dir[1] * (src_size.y + gap_world);

                let mut new_node = Node::new(shape, Pos2::new(new_x, new_y));
                new_node.size = [src_size.x, src_size.y];
                new_node.style = style.clone();
                let new_id = new_node.id;
                self.document.nodes.push(new_node);

                let edge = Edge::new(
                    Port { node_id, side: *src_side },
                    Port { node_id: new_id, side: *tgt_side },
                );
                self.document.edges.push(edge);

                self.selection.clear();
                self.selection.select_node(new_id);
                self.inline_node_edit = Some((new_id, String::new()));
                self.history.push(&self.document);
                self.status_message = Some(("Quick connect: node added".to_string(), std::time::Instant::now()));
                break; // only one click per frame
            }
        }
    }

    /// Draw Figma-style distance rulers from `source_id` to its nearest neighbours.
    /// Shows dashed red lines with pixel-distance labels on each axis.
    fn draw_distance_rulers(&self, painter: &egui::Painter, source_id: NodeId, canvas_rect: Rect) {
        let source = match self.document.find_node(&source_id) {
            Some(n) => n,
            None => return,
        };
        let src_rect = source.rect();

        // Ruler appearance
        let ruler_color = Color32::from_rgba_premultiplied(243, 139, 168, 200); // soft red
        let label_bg    = Color32::from_rgba_premultiplied(243, 139, 168, 220);
        let label_fg    = Color32::from_rgb(17, 17, 27);
        let font        = egui::FontId::proportional(10.5);
        let stroke      = Stroke::new(1.0, ruler_color);

        // Collect up to 4 nearest nodes per direction (left/right/up/down)
        // by finding the closest node whose rect overlaps on the perpendicular axis.
        let mut closest: [Option<(&Node, f32)>; 4] = [None; 4]; // L, R, U, D

        for node in &self.document.nodes {
            if node.id == source_id { continue; }
            let r = node.rect();

            // Horizontal overlap: their Y ranges intersect?
            let h_overlap = r.min.y < src_rect.max.y && r.max.y > src_rect.min.y;
            // Vertical overlap: their X ranges intersect?
            let v_overlap = r.min.x < src_rect.max.x && r.max.x > src_rect.min.x;

            // Left: node is to the left, and Y-overlaps
            if h_overlap && r.max.x <= src_rect.min.x {
                let gap = src_rect.min.x - r.max.x;
                if closest[0].map_or(true, |(_, g)| gap < g) {
                    closest[0] = Some((node, gap));
                }
            }
            // Right: node is to the right, and Y-overlaps
            if h_overlap && r.min.x >= src_rect.max.x {
                let gap = r.min.x - src_rect.max.x;
                if closest[1].map_or(true, |(_, g)| gap < g) {
                    closest[1] = Some((node, gap));
                }
            }
            // Up: node is above, and X-overlaps
            if v_overlap && r.max.y <= src_rect.min.y {
                let gap = src_rect.min.y - r.max.y;
                if closest[2].map_or(true, |(_, g)| gap < g) {
                    closest[2] = Some((node, gap));
                }
            }
            // Down: node is below, and X-overlaps
            if v_overlap && r.min.y >= src_rect.max.y {
                let gap = r.min.y - src_rect.max.y;
                if closest[3].map_or(true, |(_, g)| gap < g) {
                    closest[3] = Some((node, gap));
                }
            }
        }

        // Helper: draw dashed line + label
        let draw_ruler = |p0: Pos2, p1: Pos2, dist: f32| {
            if !canvas_rect.contains(p0) && !canvas_rect.contains(p1) { return; }
            // Dashed line (8px on, 4px off)
            let total = (p1 - p0).length();
            if total < 1.0 { return; }
            let dir = (p1 - p0) / total;
            let mut t = 0.0_f32;
            let dash = 6.0_f32;
            let gap  = 3.0_f32;
            let mut drawing = true;
            while t < total {
                let seg_end = (t + if drawing { dash } else { gap }).min(total);
                if drawing {
                    painter.line_segment([p0 + dir * t, p0 + dir * seg_end], stroke);
                }
                t = seg_end;
                drawing = !drawing;
            }
            // End tick marks
            let perp = Vec2::new(-dir.y, dir.x) * 4.0;
            painter.line_segment([p0 - perp, p0 + perp], stroke);
            painter.line_segment([p1 - perp, p1 + perp], stroke);

            // Distance label
            let mid = p0 + (p1 - p0) * 0.5;
            let text = format!("{:.0}", dist);
            let galley = painter.ctx().fonts(|f| {
                f.layout_no_wrap(text.clone(), font.clone(), label_fg)
            });
            let text_size = galley.size();
            let bg = Rect::from_center_size(mid, text_size + egui::vec2(6.0, 4.0));
            painter.rect_filled(bg, CornerRadius::same(3), label_bg);
            painter.text(mid, Align2::CENTER_CENTER, &text, font.clone(), label_fg);
        };

        // Convert world distances to screen for drawing, but keep world-unit label
        let s = self.viewport.zoom; // scale factor

        // Left ruler
        if let Some((other, gap)) = closest[0] {
            let other_r = other.rect();
            let mid_y_world = (src_rect.min.y.max(other_r.min.y) + src_rect.max.y.min(other_r.max.y)) / 2.0;
            let p0 = self.viewport.canvas_to_screen(Pos2::new(other_r.max.x, mid_y_world));
            let p1 = self.viewport.canvas_to_screen(Pos2::new(src_rect.min.x, mid_y_world));
            draw_ruler(p0, p1, gap * s);
        }
        // Right ruler
        if let Some((other, gap)) = closest[1] {
            let other_r = other.rect();
            let mid_y_world = (src_rect.min.y.max(other_r.min.y) + src_rect.max.y.min(other_r.max.y)) / 2.0;
            let p0 = self.viewport.canvas_to_screen(Pos2::new(src_rect.max.x, mid_y_world));
            let p1 = self.viewport.canvas_to_screen(Pos2::new(other_r.min.x, mid_y_world));
            draw_ruler(p0, p1, gap * s);
        }
        // Up ruler
        if let Some((other, gap)) = closest[2] {
            let other_r = other.rect();
            let mid_x_world = (src_rect.min.x.max(other_r.min.x) + src_rect.max.x.min(other_r.max.x)) / 2.0;
            let p0 = self.viewport.canvas_to_screen(Pos2::new(mid_x_world, other_r.max.y));
            let p1 = self.viewport.canvas_to_screen(Pos2::new(mid_x_world, src_rect.min.y));
            draw_ruler(p0, p1, gap * s);
        }
        // Down ruler
        if let Some((other, gap)) = closest[3] {
            let other_r = other.rect();
            let mid_x_world = (src_rect.min.x.max(other_r.min.x) + src_rect.max.x.min(other_r.max.x)) / 2.0;
            let p0 = self.viewport.canvas_to_screen(Pos2::new(mid_x_world, src_rect.max.y));
            let p1 = self.viewport.canvas_to_screen(Pos2::new(mid_x_world, other_r.min.y));
            draw_ruler(p0, p1, gap * s);
        }

        // Also draw the source node rect outline in ruler color for reference
        let src_screen = Rect::from_min_size(
            self.viewport.canvas_to_screen(src_rect.min),
            src_rect.size() * self.viewport.zoom,
        );
        painter.rect_stroke(src_screen, CornerRadius::ZERO,
            Stroke::new(1.0, ruler_color.gamma_multiply(0.6)), StrokeKind::Outside);
    }

    /// Floating edge style bar: appears above a single selected edge.
    /// Shows quick-toggle buttons for common edge styles.
    fn draw_floating_edge_bar(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        use super::interaction::{control_points_for_side, cubic_bezier_point};
        use super::theme::*;

        // Only show when exactly one edge is selected and we're not editing its label
        if self.selection.edge_ids.len() != 1 || self.inline_edge_edit.is_some() {
            return;
        }
        let edge_id = *self.selection.edge_ids.iter().next().unwrap();

        // Snapshot edge style values (avoids holding borrow across painting)
        let (is_orthogonal, is_dashed, has_glow, is_animated, has_label, mid_screen) = {
            let edge = match self.document.find_edge(&edge_id) { Some(e) => e, None => return };
            let src_node = self.document.find_node(&edge.source.node_id);
            let tgt_node = self.document.find_node(&edge.target.node_id);
            let mid = match (src_node, tgt_node) {
                (Some(sn), Some(tn)) => {
                    let src = self.viewport.canvas_to_screen(sn.port_position(edge.source.side));
                    let tgt = self.viewport.canvas_to_screen(tn.port_position(edge.target.side));
                    let offset = 60.0 * self.viewport.zoom;
                    let (cp1, cp2) = control_points_for_side(src, tgt, edge.source.side, offset);
                    let dir = if (tgt - src).length() > 1.0 { (tgt - src).normalized() } else { Vec2::X };
                    let perp = Vec2::new(-dir.y, dir.x);
                    let bend = edge.style.curve_bend * self.viewport.zoom;
                    cubic_bezier_point(src + perp * bend, cp1 + perp * bend, cp2 + perp * bend, tgt + perp * bend, 0.5)
                }
                _ => return,
            };
            (edge.style.orthogonal, edge.style.dashed, edge.style.glow, edge.style.animated, !edge.label.is_empty(), mid)
        };

        if !canvas_rect.expand(20.0).contains(mid_screen) { return; }

        let bar_h = 28.0_f32;
        let btn_w = 30.0_f32;
        // (glyph, tooltip, is_active_index: which field it toggles)
        let items: [(&str, &str, bool); 6] = [
            ("╮", "Orthogonal", is_orthogonal),
            ("⌒", "Curved bend",   edge_id.0.as_u128() > 0 && false), // always inactive for bend
            ("╌", "Dashed",        is_dashed),
            ("✦", "Glow",          has_glow),
            ("⟳", "Animated",      is_animated),
            ("✎", "Edit label",    has_label),
        ];
        let bar_w = btn_w * items.len() as f32 + 6.0;

        let bar_pos = Pos2::new(
            (mid_screen.x - bar_w / 2.0).clamp(canvas_rect.min.x + 4.0, canvas_rect.max.x - bar_w - 4.0),
            (mid_screen.y - bar_h - 14.0).clamp(canvas_rect.min.y + 4.0, canvas_rect.max.y - bar_h - 4.0),
        );

        // Collect click events via egui temp storage to avoid borrow-of-self issues inside closure
        let click_key = egui::Id::new("feb_clicks").with(edge_id.0);

        egui::Area::new(egui::Id::new("floating_edge_bar"))
            .fixed_pos(bar_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                let bar_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(bar_w, bar_h));
                ui.set_min_size(bar_rect.size());

                // --- Phase 1: allocate all interactive regions (mutable borrows of ui) ---
                let mut resps = Vec::with_capacity(items.len());
                for (i, _) in items.iter().enumerate() {
                    let bx = 3.0 + i as f32 * btn_w;
                    let btn_rect = Rect::from_min_size(Pos2::new(bx, 2.0), Vec2::new(btn_w - 2.0, bar_h - 4.0));
                    resps.push((btn_rect, ui.allocate_rect(btn_rect, egui::Sense::click())));
                }

                // --- Phase 2: draw (immutable painter borrow, no more ui mutations) ---
                let painter = ui.painter();
                painter.rect_filled(bar_rect, CornerRadius::same(14), SURFACE1);
                painter.rect_stroke(bar_rect, CornerRadius::same(14),
                    Stroke::new(1.0, MINIMAP_BORDER), StrokeKind::Outside);
                // Connector stem to edge midpoint
                let dot = Pos2::new(bar_rect.center().x, bar_rect.max.y);
                painter.line_segment(
                    [dot, mid_screen - bar_pos.to_vec2()],
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 100, 130, 100)),
                );

                let mut clicks: [bool; 6] = [false; 6];
                for (i, ((btn_rect, resp), (glyph, _tip, is_active))) in
                    resps.iter().zip(items.iter()).enumerate()
                {
                    let hov = resp.hovered();
                    let clicked = resp.clicked();
                    let bg = if *is_active {
                        ACCENT.gamma_multiply(0.30)
                    } else if hov {
                        Color32::from_rgba_unmultiplied(137, 180, 250, 22)
                    } else {
                        Color32::TRANSPARENT
                    };
                    painter.rect_filled(*btn_rect, CornerRadius::same(10), bg);
                    painter.text(
                        btn_rect.center(),
                        Align2::CENTER_CENTER,
                        *glyph,
                        FontId::proportional(13.0),
                        if *is_active { ACCENT } else { TEXT_SECONDARY },
                    );
                    if clicked { clicks[i] = true; }
                }

                // Store clicks so we can act on them after the closure
                ui.ctx().data_mut(|d| d.insert_temp(click_key, clicks));
            });

        // Read and clear clicks
        let clicks: [bool; 6] = ui.ctx().data_mut(|d| {
            let v = d.get_temp::<[bool; 6]>(click_key).unwrap_or([false; 6]);
            d.remove::<[bool; 6]>(click_key);
            v
        });

        // Apply style mutations
        if clicks[5] {
            self.inline_edge_edit = Some((edge_id, mid_screen));
            return;
        }
        let mut changed = false;
        if let Some(edge) = self.document.find_edge_mut(&edge_id) {
            if clicks[0] { edge.style.orthogonal = !edge.style.orthogonal; changed = true; }
            if clicks[1] { edge.style.curve_bend = if edge.style.curve_bend.abs() < 5.0 { 40.0 } else { 0.0 }; changed = true; }
            if clicks[2] { edge.style.dashed = !edge.style.dashed; changed = true; }
            if clicks[3] { edge.style.glow = !edge.style.glow; changed = true; }
            if clicks[4] { edge.style.animated = !edge.style.animated; changed = true; }
        }
        if changed { self.history.push(&self.document); }
    }
}
