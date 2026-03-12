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

        // Regular scroll => pan canvas
        if !cmd_held && scroll.length() > 0.0 {
            self.viewport.offset[0] += scroll.x;
            self.viewport.offset[1] += scroll.y;
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
                    self.focus_label_edit = true;
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
                    self.focus_label_edit = true;
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
                } else {
                    self.draw_edge(edge, &painter, &node_idx, hover_canvas);
                }
                // Draw path highlight overlay
                if path_edge_ids.contains(&edge.id) {
                    self.draw_path_highlight(edge, &painter, &node_idx);
                }
            }
        }

        // Focus mode overlay: draw dim rect over all non-selected nodes
        if self.focus_mode && !self.selection.is_empty() {
            for node in &self.document.nodes {
                if !self.selection.contains_node(&node.id) {
                    let screen_pos = self.viewport.canvas_to_screen(node.pos());
                    let screen_size = node.size_vec() * self.viewport.zoom;
                    let screen_rect = Rect::from_min_size(screen_pos, screen_size);
                    if screen_rect.intersects(canvas_rect) {
                        painter.rect_filled(
                            screen_rect,
                            CornerRadius::same(4),
                            Color32::from_rgba_premultiplied(30, 30, 46, 160),
                        );
                    }
                }
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

        // --- Previews ---
        self.draw_alignment_guides(&painter, canvas_rect);
        self.draw_distance_indicators(&painter);
        self.draw_box_select_preview(&painter, pointer_pos);
        self.draw_edge_creation_preview(&painter, &node_idx);
        self.draw_new_node_preview(&painter, canvas_rect);
        self.draw_node_tooltip(&painter, hover_pos, canvas_rect);
        self.draw_edge_tooltip(&painter, hover_pos, canvas_rect, &node_idx);
        self.draw_status_toast(&painter, canvas_rect, ui.ctx());
        self.draw_canvas_hud(&painter, canvas_rect, pointer_pos);
        self.draw_project_title(&painter, canvas_rect);
        self.draw_empty_canvas_hint(&painter, canvas_rect);
        self.draw_search_overlay(ui, canvas_rect);
        self.draw_zoom_presets(ui, canvas_rect);
        self.draw_minimap(&painter, canvas_rect);

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
                if cmd_held {
                    self.selection.toggle_node(node_id);
                } else if !self.selection.contains_node(&node_id) {
                    self.selection.select_node(node_id);
                }
                // Don't initiate node drag when canvas is locked
                if !self.canvas_locked {
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
                        let mut pos = 0.0_f32;
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

        // Find hovered node and extract description
        let description = self.document.nodes.iter().rev().find_map(|node| {
            let top_left = self.viewport.canvas_to_screen(node.pos());
            let sr = Rect::from_min_size(top_left, node.size_vec() * self.viewport.zoom);
            if !sr.contains(mouse) { return None; }
            match &node.kind {
                crate::model::NodeKind::Shape { description, label, .. } if !description.is_empty() => {
                    Some((label.clone(), description.clone()))
                }
                _ => None,
            }
        });

        let Some((label, desc)) = description else { return };

        let max_w = 240.0;
        let pad = 10.0;
        let font_label = egui::FontId::proportional(12.0);
        let font_desc  = egui::FontId::proportional(11.0);

        // Measure text height (approximate: 14px per line)
        let desc_lines = desc.lines().count().max(1);
        let total_h = 14.0 + (desc_lines as f32) * 14.0 + pad * 2.0 + 4.0;

        let mut tx = mouse.x + 14.0;
        let mut ty = mouse.y + 14.0;
        // Keep on screen
        if tx + max_w + pad > canvas_rect.max.x { tx = mouse.x - max_w - 14.0; }
        if ty + total_h > canvas_rect.max.y { ty = mouse.y - total_h - 14.0; }

        let bg_rect = Rect::from_min_size(Pos2::new(tx, ty), egui::Vec2::new(max_w, total_h));
        painter.rect_filled(bg_rect, egui::CornerRadius::same(6), TOOLTIP_BG);
        painter.rect_stroke(bg_rect, egui::CornerRadius::same(6), egui::Stroke::new(1.0, TOOLTIP_BORDER), egui::StrokeKind::Outside);

        painter.text(Pos2::new(tx + pad, ty + pad), egui::Align2::LEFT_TOP, &label, font_label, TEXT_SECONDARY);
        painter.text(Pos2::new(tx + pad, ty + pad + 16.0), egui::Align2::LEFT_TOP, &desc, font_desc, TEXT_DIM);
    }

    // --- Canvas HUD ---

    fn draw_empty_canvas_hint(&self, painter: &egui::Painter, canvas_rect: Rect) {
        if !self.document.nodes.is_empty() { return; }
        let cx = canvas_rect.center().x;
        let cy = canvas_rect.center().y;
        let title_font = FontId::proportional(18.0);
        let hint_font  = FontId::proportional(11.5);
        painter.text(Pos2::new(cx, cy - 48.0), Align2::CENTER_CENTER,
            "Empty canvas", title_font, TEXT_DIM);
        let hints = [
            ("N", "new node (toolbar)"),
            ("E", "connect tool"),
            ("V", "select tool"),
            ("F", "fit to content"),
            ("G", "toggle grid"),
            ("⌘Z", "undo"),
        ];
        for (i, (key, desc)) in hints.iter().enumerate() {
            let y = cy - 16.0 + i as f32 * 16.0;
            painter.text(Pos2::new(cx - 60.0, y), Align2::LEFT_CENTER,
                *key, hint_font.clone(), ACCENT);
            painter.text(Pos2::new(cx - 40.0, y), Align2::LEFT_CENTER,
                *desc, hint_font.clone(), TEXT_DIM);
        }
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
    }

    fn draw_project_title(&self, painter: &egui::Painter, canvas_rect: Rect) {
        if self.project_title.is_empty() { return; }
        let font = FontId::proportional(13.0);
        let color = Color32::from_rgba_premultiplied(180, 180, 200, 100);
        let pos = Pos2::new(canvas_rect.min.x + 20.0, canvas_rect.min.y + 20.0);
        painter.text(pos, Align2::LEFT_TOP, &self.project_title, font, color);
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
        let major_start_x = canvas_rect.min.x + major_offset_x;
        let major_start_y = canvas_rect.min.y + major_offset_y;

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

        let w = 320.0_f32;
        let overlay_rect = Rect::from_center_size(
            Pos2::new(canvas_rect.center().x, canvas_rect.min.y + 60.0),
            Vec2::new(w, 36.0),
        );

        // Draw background first (painter doesn't alias with child ui since it's a clone)
        {
            let painter = ui.painter().clone();
            painter.rect_filled(overlay_rect.expand(4.0), CornerRadius::same(8), TOOLTIP_BG);
            painter.rect_stroke(overlay_rect.expand(4.0), CornerRadius::same(8),
                Stroke::new(1.0, SURFACE1), StrokeKind::Outside);
        }

        // Search text edit
        let mut ui2 = ui.new_child(
            egui::UiBuilder::new().max_rect(overlay_rect).layout(egui::Layout::left_to_right(egui::Align::Center))
        );
        let resp = ui2.add(
            egui::TextEdit::singleline(&mut self.search_query)
                .hint_text("Search nodes…")
                .desired_width(w)
                .font(egui::FontId::proportional(14.0))
                .frame(false),
        );
        resp.request_focus();

        let ctx = ui2.ctx().clone();

        // Close on Escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_search = false;
            return;
        }

        // Select matching nodes on Enter
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            let q = self.search_query.to_lowercase();
            self.selection.clear();
            for node in &self.document.nodes {
                if let crate::model::NodeKind::Shape { label, .. } = &node.kind {
                    if label.to_lowercase().contains(&q) {
                        self.selection.node_ids.insert(node.id);
                    }
                }
            }
            if !self.selection.is_empty() {
                self.zoom_to_selection();
            }
            self.show_search = false;
            return;
        }

        // Live "N found" hint
        if !self.search_query.is_empty() {
            let q = self.search_query.to_lowercase();
            let count = self.document.nodes.iter().filter(|n| {
                if let crate::model::NodeKind::Shape { label, .. } = &n.kind {
                    label.to_lowercase().contains(&q)
                } else { false }
            }).count();
            ui2.painter().text(
                Pos2::new(overlay_rect.max.x - 4.0, overlay_rect.center().y),
                Align2::RIGHT_CENTER,
                format!("{count}"),
                FontId::proportional(11.0),
                TEXT_DIM,
            );
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

        // Pan viewport so (world_x, world_y) is at canvas center
        let c = canvas_rect.center();
        self.viewport.offset[0] = c.x - world_x * self.viewport.zoom;
        self.viewport.offset[1] = c.y - world_y * self.viewport.zoom;
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
    }
}
