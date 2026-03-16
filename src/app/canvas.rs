use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};
use crate::model::*;
use super::{FlowchartApp, DragState, Tool, ResizeHandle};
use super::interaction::{control_points_for_side, cubic_bezier_point};
use super::theme::{PORT_RADIUS, to_color32};

/// Apply a section-appropriate default fill color (and optionally shape) to a new node.
/// Only called for plain Shape nodes created via double-click or drag into a section area.
fn apply_section_style(node: &mut Node, section: &str) {
    let s = section.to_lowercase();
    let (fill, shape): ([u8; 4], Option<NodeShape>) = if s.contains("hypothes") {
        ([137, 180, 250, 255], Some(NodeShape::RoundedRect)) // blue
    } else if s.contains("evidence") || s.contains("data") || s.contains("fact") {
        ([166, 227, 161, 255], Some(NodeShape::Rectangle))   // green
    } else if s.contains("assumption") {
        ([249, 226, 175, 255], None)                          // yellow
    } else if s.contains("conclusion") || s.contains("decision") || s.contains("finding") {
        ([203, 166, 247, 255], Some(NodeShape::RoundedRect)) // purple
    } else if s.contains("risk") || s.contains("issue") || s.contains("block") || s.contains("threat") {
        ([243, 139, 168, 255], Some(NodeShape::Diamond))      // red/diamond
    } else if s.contains("observation") || s.contains("insight") || s.contains("quote") {
        ([245, 194, 231, 255], Some(NodeShape::Callout))      // pink/callout
    } else if s.contains("strength") || s.contains("opportunit") {
        ([166, 227, 161, 255], None)                          // green
    } else if s.contains("weakness") {
        ([249, 226, 175, 255], None)                          // yellow
    } else {
        return; // no override for unknown sections
    };
    node.style.fill_color = fill;
    node.style.text_color = crate::app::theme::auto_contrast_text(fill);
    if let Some(sh) = shape {
        if let NodeKind::Shape { ref mut shape, .. } = node.kind {
            *shape = sh;
        }
    }
}

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

        // Kanban column backgrounds: faint vertical bands per section (LR layout only)
        self.draw_kanban_column_bands(&painter, canvas_rect);

        // --- Input handling ---
        let pointer_pos = response
            .hover_pos()
            .or_else(|| ui.ctx().input(|i| i.pointer.hover_pos()));

        // Scroll => pan (infinite canvas), Pinch/Cmd+scroll => zoom.
        // Only act on scroll events when the canvas is hovered. Use input_mut to
        // *consume* both raw and smooth scroll deltas so they cannot leak to the
        // sidebar ScrollArea (which only consumes smooth_scroll_delta itself).
        let canvas_hovered = response.hovered()
            || matches!(self.drag, DragState::Panning { .. } | DragState::DraggingNode { .. });
        let (raw_scroll, smooth_scroll, pinch_zoom, cmd_held) = if canvas_hovered {
            ui.ctx().input_mut(|i| {
                let raw   = i.raw_scroll_delta;
                let smooth = i.smooth_scroll_delta;
                let pinch  = i.zoom_delta();
                let cmd    = i.modifiers.command;
                // Consume both scroll fields so the sidebar never sees them.
                i.raw_scroll_delta    = Vec2::ZERO;
                i.smooth_scroll_delta = Vec2::ZERO;
                (raw, smooth, pinch, cmd)
            })
        } else {
            (Vec2::ZERO, Vec2::ZERO, 1.0, false)
        };
        // Prefer smooth_scroll for Cmd+scroll zoom (raw is unbuffered and causes jitter).
        // For regular pan, use raw if available (more responsive).
        let pan_scroll = if raw_scroll.length() > 0.0 { raw_scroll } else { smooth_scroll };
        let zoom_scroll_raw = smooth_scroll.y;

        // Pinch-to-zoom or Cmd+scroll => zoom towards mouse
        let zoom_input = if pinch_zoom != 1.0 {
            Some(pinch_zoom)
        } else if cmd_held && zoom_scroll_raw != 0.0 {
            Some((1.0 + zoom_scroll_raw * 0.004).clamp(0.85, 1.15))
        } else {
            None
        };
        if let Some(factor) = zoom_input {
            if let Some(mouse) = pointer_pos {
                let old_zoom = self.viewport.zoom;
                let new_zoom = (old_zoom * factor).clamp(0.1, 10.0);
                // Update both zoom and zoom_target so the smooth interpolator
                // doesn't fight against the scroll-driven change.
                self.viewport.zoom = new_zoom;
                self.zoom_target = new_zoom;
                let ratio = new_zoom / old_zoom;
                self.viewport.offset[0] = mouse.x - ratio * (mouse.x - self.viewport.offset[0]);
                self.viewport.offset[1] = mouse.y - ratio * (mouse.y - self.viewport.offset[1]);
            }
        }
        let scroll = if cmd_held { Vec2::ZERO } else { pan_scroll };

        // Advance animated layout transition each frame
        let dt = ui.ctx().input(|i| i.stable_dt).clamp(0.001, 0.1);
        if !self.layout_targets.is_empty() {
            let ctx_clone = ui.ctx().clone();
            self.step_layout_animation(dt, &ctx_clone);
        }

        // Regular scroll => pan canvas (with inertia accumulation)
        // scroll is already Vec2::ZERO when cmd_held, so no extra guard needed.
        if scroll.length() > 0.0 {
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

        // Global handler for DraggingNewNode dragged in from the toolbar sidebar.
        // handle_drag_end only fires on canvas response.drag_stopped(), which won't
        // trigger when the drag originated in a different panel (the toolbar).
        if matches!(self.drag, DragState::DraggingNewNode { .. }) {
            let (hover, primary_down, primary_released) = ui.ctx().input(|i| {
                (i.pointer.hover_pos(), i.pointer.primary_down(), i.pointer.primary_released())
            });
            // Track pointer position globally
            if let (Some(pos), DragState::DraggingNewNode { ref mut current_screen, .. }) =
                (hover, &mut self.drag)
            {
                *current_screen = pos;
            }
            // On release, place node if pointer is over the canvas
            if primary_released || !primary_down {
                let node_to_place = if let DragState::DraggingNewNode { ref kind, ref current_screen } =
                    self.drag
                {
                    if canvas_rect.contains(*current_screen) {
                        let mut canvas_pos = self.viewport.screen_to_canvas(*current_screen);
                        if self.snap_to_grid {
                            canvas_pos = self.snap_pos(canvas_pos);
                        }
                        Some(match kind {
                            NodeKind::Shape { shape, .. } => Node::new(*shape, canvas_pos),
                            NodeKind::StickyNote { color, .. } => Node::new_sticky(*color, canvas_pos),
                            NodeKind::Entity { .. } => Node::new_entity(canvas_pos),
                            NodeKind::Text { .. } => Node::new_text(canvas_pos),
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(node) = node_to_place {
                    self.selection.clear();
                    self.selection.node_ids.insert(node.id);
                    self.document.nodes.push(node);
                    self.history.push(&self.document);
                }
                self.drag = DragState::None;
            }
        }

        // Live position tooltip — show X,Y while dragging a node
        if let DragState::DraggingNode { start_positions, .. } = &self.drag {
            if start_positions.len() == 1 {
                let node_id = start_positions[0].0;
                if let (Some(node), Some(mp)) = (self.document.find_node(&node_id), pointer_pos) {
                    let pos = node.pos();
                    let text = format!("{},{}", pos.x.round() as i32, pos.y.round() as i32);
                    let tp = mp + Vec2::new(12.0, -22.0);
                    let pad = Vec2::new(6.0, 3.0);
                    let font = FontId::proportional(10.5);
                    let galley = ui.ctx().fonts(|f| f.layout_no_wrap(text.clone(), font.clone(), self.theme.text_dim));
                    let bg_rect = Rect::from_min_size(tp - pad, galley.size() + pad * 2.0);
                    painter.rect_filled(bg_rect, CornerRadius::same(3), self.theme.tooltip_bg);
                    painter.text(tp, Align2::LEFT_TOP, &text, font, self.theme.text_dim);
                }
            }
        }

        // Live resize tooltip — show W×H while ResizingNode
        if let DragState::ResizingNode { node_id, .. } = &self.drag {
            if let (Some(node), Some(mp)) = (self.document.find_node(node_id), pointer_pos) {
                let nw = node.size[0].round() as i32;
                let nh = node.size[1].round() as i32;
                let text = format!("{}×{}", nw, nh);
                let tp = mp + Vec2::new(12.0, -22.0);
                let pad = Vec2::new(6.0, 3.0);
                let font = FontId::proportional(10.5);
                let galley = ui.ctx().fonts(|f| f.layout_no_wrap(text.clone(), font.clone(), self.theme.accent));
                let bg_rect = Rect::from_min_size(tp - pad, galley.size() + pad * 2.0);
                painter.rect_filled(bg_rect, CornerRadius::same(3), self.theme.tooltip_bg);
                painter.text(tp, Align2::LEFT_TOP, &text, font, self.theme.accent);
            }
        }

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
                    // Click on empty space: check if inside a section background → select all in section
                    if let Some(section_name) = self.section_at_canvas_pos(canvas_pos) {
                        if !cmd_held {
                            self.selection.clear();
                        }
                        let section_ids: Vec<_> = self.document.nodes.iter()
                            .filter(|n| n.section_name == section_name)
                            .map(|n| n.id)
                            .collect();
                        let count = section_ids.len();
                        for id in section_ids {
                            self.selection.node_ids.insert(id);
                        }
                        if count > 0 {
                            self.status_message = Some((
                                format!("Selected {count} nodes in \"{section_name}\""),
                                std::time::Instant::now(),
                            ));
                        }
                    } else if !cmd_held {
                        self.selection.clear();
                    }
                }
            }
        }

        // Right-click context menu.
        // Capture the pointer position at the moment of secondary click so that
        // hit-testing inside draw_context_menu stays correct — once the popup
        // opens, response.hover_pos() returns None (cursor is over the popup).
        if response.secondary_clicked() {
            self.context_menu_origin = pointer_pos;
        }
        response.context_menu(|ui| {
            self.draw_context_menu(ui, self.context_menu_origin);
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
                    // First: check if double-click is on a section label → rename section
                    let hit_section = self.section_label_hit(mouse);
                    if let Some((sec_name, label_pos)) = hit_section {
                        self.section_rename = Some((sec_name.clone(), sec_name, label_pos));
                    } else {
                        // Create a new default shape node centered on the click
                        let mut node = Node::new(NodeShape::Rectangle, canvas_pos);
                        let w = node.size[0];
                        let h = node.size[1];
                        node.set_pos(egui::Pos2::new(canvas_pos.x - w / 2.0, canvas_pos.y - h / 2.0));
                        // Auto-assign section and style if dropped inside a section background
                        if let Some(sec) = self.section_at_canvas_pos(canvas_pos) {
                            apply_section_style(&mut node, &sec);
                            node.section_name = sec;
                        }
                        let id = node.id;
                        self.document.nodes.push(node);
                        self.selection.select_node(id);
                        self.inline_node_edit = Some((id, String::new()));
                        self.history.push(&self.document);
                        self.status_message = Some(("Node created".to_string(), std::time::Instant::now()));
                    }
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

        // Pre-compute parallel-edge bends: when 2+ edges share the same node pair,
        // spread them apart with a perpendicular offset so they're individually readable.
        let parallel_bends: std::collections::HashMap<EdgeId, f32> = {
            let mut group: std::collections::HashMap<(NodeId, NodeId), Vec<EdgeId>> = std::collections::HashMap::new();
            for edge in &self.document.edges {
                // Use (src, tgt) order as-is so A→B and B→A are treated independently.
                group.entry((edge.source.node_id, edge.target.node_id))
                    .or_default()
                    .push(edge.id);
            }
            let mut bends = std::collections::HashMap::new();
            for edge_ids in group.values() {
                if edge_ids.len() > 1 {
                    let count = edge_ids.len() as f32;
                    // Spread parallel edges apart with a perpendicular canvas-unit offset.
                    // step=30: two edges → -15 and +15; three → -30, 0, +30 (clearly separated).
                    let step = 30.0_f32;
                    for (i, &eid) in edge_ids.iter().enumerate() {
                        let offset = (i as f32 - (count - 1.0) / 2.0) * step;
                        bends.insert(eid, offset);
                    }
                }
            }
            bends
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

        // Store current hover node id for statusbar / other panels to read
        self.hover_node_id = hover_node_id;

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

        // Timeline grid overlay (drawn before edges/nodes so it's underneath)
        if self.document.timeline_mode && !matches!(self.view_mode, super::ViewMode::ThreeD) {
            self.draw_timeline_grid(&painter, canvas_rect);
        }

        // Section backgrounds + labels (non-timeline, 2D only)
        if !self.document.timeline_mode && !matches!(self.view_mode, super::ViewMode::ThreeD)
            && self.viewport.zoom > 0.2
        {
            self.draw_section_backgrounds(&painter, canvas_rect);
            if self.viewport.zoom > 0.3 {
                self.draw_section_labels(&painter, canvas_rect);
            }
        }

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
                            let dim_color = self.theme.text_dim.gamma_multiply(0.15);
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
                        let pb = parallel_bends.get(&edge.id).copied().unwrap_or(0.0);
                        self.draw_edge(edge, &painter, &node_idx, hover_canvas, pb);
                    } else if let (Some(&si), Some(&ti)) = (node_idx.get(&edge.source.node_id), node_idx.get(&edge.target.node_id)) {
                        if let (Some(sn), Some(tn)) = (self.document.nodes.get(si), self.document.nodes.get(ti)) {
                            let s = self.viewport.canvas_to_screen(sn.port_position(edge.source.side));
                            let t = self.viewport.canvas_to_screen(tn.port_position(edge.target.side));
                            let off = 60.0 * self.viewport.zoom;
                            let (cp1, cp2) = super::interaction::control_points_for_side(s, t, edge.source.side, off);
                            let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                                [s, cp1, cp2, t], false, Color32::TRANSPARENT,
                                Stroke::new(edge.style.width, self.theme.text_dim.gamma_multiply(0.08)),
                            );
                            painter.add(bezier);
                        }
                    }
                } else {
                    let pb = parallel_bends.get(&edge.id).copied().unwrap_or(0.0);
                    self.draw_edge(edge, &painter, &node_idx, hover_canvas, pb);
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

        self.draw_focus_and_filter_overlays(&painter, canvas_rect, &focus_neighbors);

        self.draw_multi_selection_dimensions(&painter);

        // Compute search matches (for highlight overlay)
        let search_matches: std::collections::HashSet<NodeId> = if (self.show_search || self.persist_search_filter) && !self.search_query.is_empty() {
            let q = self.search_query.trim().to_lowercase();
            let today_str = super::render::today_iso();
            self.document.nodes.iter()
                .filter(|n| {
                    // Smart filter prefixes
                    if let Some(status_q) = q.strip_prefix("status:") {
                        let tag_match = match status_q.trim() {
                            "done"    | "✅" => matches!(n.tag, Some(crate::model::NodeTag::Ok)),
                            "wip"     | "🔄" | "in progress" => matches!(n.tag, Some(crate::model::NodeTag::Info)),
                            "todo"    | "📋" | "pending" => matches!(n.tag, Some(crate::model::NodeTag::Warning)) && n.progress < 0.5,
                            "review"  | "👁"             => matches!(n.tag, Some(crate::model::NodeTag::Warning)) && n.progress >= 0.5,
                            "blocked" | "⛔"             => matches!(n.tag, Some(crate::model::NodeTag::Critical)),
                            "tagged"                    => n.tag.is_some(),
                            "none" | "untagged"         => n.tag.is_none(),
                            _                           => false,
                        };
                        return tag_match;
                    }
                    if let Some(sec_q) = q.strip_prefix("section:").or_else(|| q.strip_prefix("§")) {
                        return n.section_name.to_lowercase().contains(sec_q.trim());
                    }
                    if let Some(icon_q) = q.strip_prefix("icon:") {
                        return n.icon.to_lowercase().contains(icon_q.trim());
                    }
                    if q == "glow" || q == "highlighted" {
                        return n.style.glow || n.highlight;
                    }
                    if let Some(pri_q) = q.strip_prefix("priority:").or_else(|| q.strip_prefix("p:")) {
                        return match pri_q.trim() {
                            "p1" | "1" | "critical" => matches!(n.tag, Some(crate::model::NodeTag::Critical)),
                            "p2" | "2" | "high"     => matches!(n.tag, Some(crate::model::NodeTag::Warning)),
                            "p3" | "3" | "medium"   => matches!(n.tag, Some(crate::model::NodeTag::Info)),
                            "p4" | "4" | "low"      => n.tag.is_none(),
                            _                       => false,
                        };
                    }
                    if q == "escalated" {
                        return matches!(n.tag, Some(crate::model::NodeTag::Critical)) && n.style.glow;
                    }
                    if q == "overdue" || q == "due:overdue" || q == "past-due" {
                        // Match nodes with a 📅 sublabel whose date is today or in the past
                        if let Some(date_str) = n.sublabel.split('\n').find_map(|l| l.strip_prefix("📅 ")) {
                            let d = date_str.trim();
                            return d <= today_str.as_str() && d.len() >= 8;
                        }
                        return false;
                    }
                    if q == "upcoming" || q == "due:upcoming" || q == "due:future" {
                        if let Some(date_str) = n.sublabel.split('\n').find_map(|l| l.strip_prefix("📅 ")) {
                            let d = date_str.trim();
                            return d > today_str.as_str();
                        }
                        return false;
                    }
                    if q == "has-due" || q == "has-deadline" {
                        return n.sublabel.starts_with("📅");
                    }
                    if let Some(due_q) = q.strip_prefix("due:") {
                        let tgt = due_q.trim().to_lowercase();
                        if let Some(date_str) = n.sublabel.strip_prefix("📅 ") {
                            return date_str.trim().to_lowercase().contains(&tgt);
                        }
                        return false;
                    }
                    if let Some(assignee_q) = q.strip_prefix("assigned:").or_else(|| q.strip_prefix("owner:")).or_else(|| q.strip_prefix("assignee:")) {
                        // {assigned:Alice} → sublabel starts with "👤 "
                        let tgt = assignee_q.trim().to_lowercase();
                        return n.sublabel.to_lowercase().contains(&tgt);
                    }
                    if q == "assigned" || q == "has-owner" {
                        return n.sublabel.starts_with("👤");
                    }
                    if q == "unassigned" || q == "no-owner" {
                        return !n.sublabel.starts_with("👤");
                    }
                    if q == "linked" {
                        return self.document.edges.iter().any(|e| e.source.node_id == n.id || e.target.node_id == n.id);
                    }
                    if q == "unlinked" || q == "orphan" {
                        return !self.document.edges.iter().any(|e| e.source.node_id == n.id || e.target.node_id == n.id);
                    }
                    if let Some(url_q) = q.strip_prefix("url:").or_else(|| q.strip_prefix("link:")) {
                        return n.url.to_lowercase().contains(url_q.trim());
                    }
                    if q == "has-url" || q == "has-link" {
                        return !n.url.is_empty();
                    }
                    if q == "no-url" || q == "no-link" {
                        return n.url.is_empty();
                    }
                    if q == "commented" || q == "has-comment" || q == "has-note" {
                        return !n.comment.is_empty();
                    }
                    if q == "overdue" || q == "sla-breach" {
                        if let Some(date_str) = n.sublabel.split('\n').find_map(|l| l.strip_prefix("📅 ")) {
                            return date_str.trim() <= today_str.as_str();
                        }
                        return false;
                    }
                    // Default: label / description / section name / URL / sublabel
                    let label = n.display_label().to_lowercase();
                    let desc = match &n.kind { crate::model::NodeKind::Shape { description, .. } => description.to_lowercase(), _ => String::new() };
                    label.contains(&q) || desc.contains(&q) || n.section_name.to_lowercase().contains(&q)
                        || n.url.to_lowercase().contains(&q) || n.sublabel.to_lowercase().contains(&q)
                })
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

        self.draw_deletion_ghosts(&painter);

        // Search dim: when search is active with a query, dim non-matching nodes
        if (self.show_search || self.persist_search_filter) && !self.search_query.is_empty() {
            for node in &self.document.nodes {
                if search_matches.contains(&node.id) { continue; }
                let sp = self.viewport.canvas_to_screen(node.pos());
                let ss = node.size_vec() * self.viewport.zoom;
                let sr = Rect::from_min_size(sp, ss);
                if sr.expand(20.0).intersects(canvas_rect) {
                    let cr = CornerRadius::same(node.style.corner_radius as u8);
                    painter.rect_filled(sr, cr, Color32::from_rgba_unmultiplied(0, 0, 0, 160));
                }
            }
        }

        // Hover dim: when hovering a node with no selection, dim non-neighbor nodes
        if let Some(hid) = hover_node_id {
            if self.selection.is_empty() {
                let neighbors: std::collections::HashSet<NodeId> = self.document.edges.iter()
                    .flat_map(|e| {
                        if e.source.node_id == hid { Some(e.target.node_id) }
                        else if e.target.node_id == hid { Some(e.source.node_id) }
                        else { None }
                    })
                    .collect();
                for node in &self.document.nodes {
                    if node.id == hid || neighbors.contains(&node.id) { continue; }
                    let sp = self.viewport.canvas_to_screen(node.pos());
                    let ss = node.size_vec() * self.viewport.zoom;
                    let sr = Rect::from_min_size(sp, ss);
                    if sr.expand(20.0).intersects(canvas_rect) {
                        let cr = CornerRadius::same(node.style.corner_radius as u8);
                        // Dim overlay using canvas bg with ~50% alpha
                        let dim = self.theme.canvas_bg.gamma_multiply(0.55);
                        painter.rect_filled(sr.expand(2.0), cr, dim);
                    }
                }
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

        self.draw_drag_ghosts(&painter, canvas_rect);

        self.draw_resize_feedback(&painter, pointer_pos);

        // --- Previews ---
        self.draw_alignment_guides(&painter, canvas_rect);
        self.draw_distance_indicators(&painter);
        self.draw_multi_selection_handles(&painter);

        self.draw_inline_node_editor(ui, canvas_rect);
        self.draw_section_rename_editor(ui);
        self.draw_quick_assign_popup(ui, canvas_rect);
        self.draw_quick_comment_popup(ui, canvas_rect);
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
        self.handle_section_summary_click(ui, canvas_rect);
        self.draw_section_progress_summary(&painter, canvas_rect);
        self.draw_canvas_vignette(&painter, canvas_rect);

        self.draw_back_to_content(&painter, canvas_rect, ui);

        self.draw_tag_filter_pills(&painter, canvas_rect, ui);
        self.draw_persistent_filter_chip(ui, canvas_rect);
        self.draw_kanban_column_headers(ui, canvas_rect);

        self.draw_project_title(&painter, canvas_rect);
        self.draw_empty_canvas_hint(&painter, canvas_rect);
        let sm_clone = search_matches.clone();
        self.draw_search_overlay(ui, canvas_rect, &sm_clone);
        self.draw_section_navigator(ui, canvas_rect);
        self.draw_zoom_presets(ui, canvas_rect);
        if self.show_minimap {
            self.draw_minimap(&painter, canvas_rect);
        }
        if self.show_rulers {
            self.draw_side_rulers(&painter, canvas_rect, pointer_pos);
        }
        if self.show_quick_notes {
            self.draw_quick_notes_panel(ui, canvas_rect);
        }
        if self.show_workload_panel {
            self.draw_workload_panel(ui, canvas_rect);
        }

        // Minimap click-to-pan and drag-to-pan (only when minimap is visible)
        if self.show_minimap {
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
                let raw_delta = canvas_mouse - *start_mouse;
                let (alt_held, shift_held) = _ui.ctx().input(|i| (i.modifiers.alt, i.modifiers.shift));
                // Shift+drag: constrain to dominant axis
                let delta = if shift_held {
                    if raw_delta.x.abs() >= raw_delta.y.abs() {
                        egui::Vec2::new(raw_delta.x, 0.0)
                    } else {
                        egui::Vec2::new(0.0, raw_delta.y)
                    }
                } else {
                    raw_delta
                };
                let positions = start_positions.clone();
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
                let (shift_held, alt_held) = _ui.ctx().input(|i| (i.modifiers.shift, i.modifiers.alt));
                if let Some(node) = self.document.find_node(&nid) {
                    let min = node.min_size();
                    let [mut nx, mut ny, mut nw, mut nh] = Self::compute_resize(h, sr, delta, min);
                    // Shift = proportional resize: lock aspect ratio
                    if shift_held && sr[2] > 0.0 && sr[3] > 0.0 {
                        let aspect = sr[2] / sr[3]; // original w/h
                        // Determine which axis drove the resize
                        let w_changed = (nw - sr[2]).abs() > 0.001;
                        let h_changed = (nh - sr[3]).abs() > 0.001;
                        match h {
                            ResizeHandle::Left | ResizeHandle::Right => {
                                nh = (nw / aspect).max(min[1]);
                            }
                            ResizeHandle::Top | ResizeHandle::Bottom => {
                                nw = (nh * aspect).max(min[0]);
                            }
                            _ => {
                                // Corner: use whichever dimension changed more
                                if w_changed && h_changed {
                                    let wf = (nw / sr[2]).max(0.0);
                                    let hf = (nh / sr[3]).max(0.0);
                                    if wf > hf {
                                        nh = (nw / aspect).max(min[1]);
                                    } else {
                                        nw = (nh * aspect).max(min[0]);
                                    }
                                }
                            }
                        }
                    }
                    // Alt = resize from center: expand equally on both sides
                    if alt_held {
                        let dw = nw - sr[2]; // width delta
                        let dh = nh - sr[3]; // height delta
                        // Center stays fixed: start_rect center
                        let cx = sr[0] + sr[2] / 2.0;
                        let cy = sr[1] + sr[3] / 2.0;
                        let new_w = (sr[2] + dw.abs() * 2.0).max(min[0]);
                        let new_h = (sr[3] + dh.abs() * 2.0).max(min[1]);
                        nx = cx - new_w / 2.0;
                        ny = cy - new_h / 2.0;
                        nw = new_w;
                        nh = new_h;
                    }
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
                    // After drag: auto-reassign section for moved nodes
                    if matches!(self.drag, DragState::DraggingNode { .. }) {
                        let moved_ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
                        for node_id in moved_ids {
                            if let Some(pos) = self.document.find_node(&node_id).map(|n| n.rect().center()) {
                                let new_section = self.section_at_canvas_pos_excluding(pos, node_id);
                                if let Some(node) = self.document.find_node_mut(&node_id) {
                                    // Only reassign if: moved into a section, or moved out of its current section
                                    let was_in_section = !node.section_name.is_empty();
                                    match &new_section {
                                        Some(sec) if sec != &node.section_name => {
                                            node.section_name = sec.clone();
                                        }
                                        None if was_in_section => {
                                            // Moved outside all sections — clear section
                                            // (only if it's now clearly outside: use a stricter check)
                                            // We'll keep the section assignment to avoid accidental clearing
                                            // near edges. User can clear via context menu if desired.
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
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
                DragState::Panning { .. } => {
                    // Transfer drag velocity to pan_velocity for mouse-drag inertia
                    let vel = _ui.ctx().input(|i| i.pointer.velocity());
                    let speed = vel.length();
                    if speed > 50.0 {
                        // Scale down and cap so fast flicks feel natural, not runaway
                        let scale = (speed / 800.0).min(1.0) * 0.5;
                        self.pan_velocity[0] += vel.x * scale;
                        self.pan_velocity[1] += vel.y * scale;
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
                        let mut node = match kind {
                            NodeKind::Shape { shape, .. } => Node::new(*shape, canvas_pos),
                            NodeKind::StickyNote { color, .. } => {
                                Node::new_sticky(*color, canvas_pos)
                            }
                            NodeKind::Entity { .. } => Node::new_entity(canvas_pos),
                            NodeKind::Text { .. } => Node::new_text(canvas_pos),
                        };
                        // Auto-assign section and style if dropped inside a section background
                        if let Some(sec) = self.section_at_canvas_pos(canvas_pos) {
                            // Only apply section style for plain Shape nodes (not stickies, entities, text)
                            if matches!(node.kind, NodeKind::Shape { .. }) {
                                apply_section_style(&mut node, &sec);
                            }
                            node.section_name = sec;
                        }
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
                        } else if self.section_label_hit(hover).is_some() {
                            // Section labels are double-clickable for rename
                            egui::CursorIcon::Text
                        } else {
                            // Check if over section progress summary panel
                            let panel_w = 170.0_f32;
                            let panel_x = self.canvas_rect.max.x - panel_w - 12.0;
                            let panel_y = self.canvas_rect.min.y + 12.0;
                            let summary_panel = Rect::from_min_size(
                                Pos2::new(panel_x, panel_y),
                                egui::Vec2::new(panel_w, 150.0), // conservative height
                            );
                            if summary_panel.contains(hover) {
                                egui::CursorIcon::PointingHand
                            } else {
                                egui::CursorIcon::Default
                            }
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
        let guide_color = self.theme.accent.gamma_multiply(0.63);
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
        let dist_color = Color32::from_rgb(255, 75, 75).gamma_multiply(0.86);
        let line_stroke = Stroke::new(1.0, dist_color);
        let label_bg = Color32::from_rgb(255, 75, 75).gamma_multiply(0.78);
        let label_fg = self.theme.text_primary;

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
                painter.rect_filled(sel_rect, CornerRadius::ZERO, self.theme.box_select_fill);

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

                    let stroke = Stroke::new(1.2, self.theme.box_select_stroke);
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
                        CornerRadius::same(4), self.theme.accent,
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
            Stroke::new(2.0, self.theme.selection_color),
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
                let color = if near { self.theme.accent } else { self.theme.text_dim.gamma_multiply(0.63) };
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
                    painter.circle_filled(port_pos, r * 1.5, self.theme.accent_select_bg);
                    painter.circle_filled(port_pos, r, self.theme.accent);
                    painter.circle_stroke(port_pos, r, Stroke::new(2.0, self.theme.text_primary));
                    // Port name badge
                    let side_name = match target_port.side {
                        PortSide::Top => "Top", PortSide::Bottom => "Bottom",
                        PortSide::Left => "Left", PortSide::Right => "Right",
                    };
                    let badge_pos = port_pos + Vec2::new(0.0, r * 2.2);
                    let badge_w = side_name.len() as f32 * 5.5 + 10.0;
                    let badge_rect = Rect::from_center_size(badge_pos, Vec2::new(badge_w, 16.0));
                    painter.rect_filled(badge_rect, CornerRadius::same(4), self.theme.accent);
                    let badge_text_col = crate::app::theme::auto_contrast_text(
                        [self.theme.accent.r(), self.theme.accent.g(), self.theme.accent.b(), 255]);
                    painter.text(badge_pos, Align2::CENTER_CENTER, side_name,
                        FontId::proportional(9.5), crate::app::theme::to_color32(badge_text_col));
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
        painter.rect_filled(screen_rect, CornerRadius::same(4), self.theme.preview_fill);
        painter.rect_stroke(
            screen_rect,
            CornerRadius::same(4),
            Stroke::new(1.5, self.theme.selection_color),
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
                let galley = painter.layout_no_wrap(msg.clone(), font.clone(), self.theme.toast_success);
                let pill_rect = Rect::from_center_size(
                    toast_pos,
                    Vec2::new(galley.size().x + 24.0, galley.size().y + 12.0),
                );
                let bg_alpha = (alpha as f32 * 0.85) as u8;
                painter.rect_filled(
                    pill_rect,
                    CornerRadius::same(16),
                    self.theme.mantle.gamma_multiply(bg_alpha as f32 / 255.0),
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
        let note_font = egui::FontId::proportional(10.5);
        let pad = 8.0;
        let w = 240.0_f32;
        let has_note = !edge.comment.is_empty();
        let h = if has_note { 44.0_f32 } else { 26.0_f32 };
        let mut tx = mouse.x + 12.0;
        let mut ty = mouse.y - h - 6.0;
        if tx + w > canvas_rect.max.x { tx = mouse.x - w - 12.0; }
        if ty < canvas_rect.min.y { ty = mouse.y + 12.0; }
        let bg_rect = Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(w, h));
        painter.rect_filled(bg_rect, egui::CornerRadius::same(4), self.theme.tooltip_bg);
        painter.rect_stroke(bg_rect, egui::CornerRadius::same(4), egui::Stroke::new(1.0, self.theme.tooltip_border), egui::StrokeKind::Outside);
        painter.text(Pos2::new(tx + pad, ty + 13.0), Align2::LEFT_CENTER, &line, font, self.theme.text_secondary);
        if has_note {
            let note_y = ty + 30.0;
            painter.text(
                Pos2::new(tx + pad, note_y),
                Align2::LEFT_CENTER,
                &format!("💬 {}", edge.comment),
                note_font,
                self.theme.text_dim,
            );
        }
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
            rows.push((desc.to_string(), self.theme.text_dim));
        }
        if rich_mode {
            // Sublabel (if set and different from main label)
            if !node.sublabel.is_empty() {
                rows.push((node.sublabel.clone(), self.theme.text_dim.gamma_multiply(0.75)));
            }
            // Progress bar percentage
            if node.progress > 0.0 {
                let pct = (node.progress * 100.0).round() as u32;
                let bar = "█".repeat((node.progress * 10.0) as usize);
                let empty = "░".repeat(10 - (node.progress * 10.0) as usize);
                rows.push((format!("{}% {}{}", pct, bar, empty),
                    if node.progress >= 1.0 { Color32::from_rgb(166, 227, 161) }
                    else if node.progress >= 0.6 { Color32::from_rgb(249, 226, 175) }
                    else { Color32::from_rgb(243, 139, 168) }
                ));
            }
            if conn_in > 0 || conn_out > 0 {
                rows.push((format!("↑{} in  ↓{} out", conn_in, conn_out), self.theme.text_dim));
            }
            if !node.section_name.is_empty() {
                rows.push((format!("§ {}", node.section_name), self.theme.accent.gamma_multiply(0.7)));
            }
            if let Some(tag) = tag_label {
                rows.push((format!("Tag: {}", tag), self.theme.text_dim));
            }
            // SLA info: show created + due date analysis
            {
                let today = super::render::today_iso();
                let due_str_opt: Option<&str> = node.sublabel.split('\n')
                    .find_map(|line| line.strip_prefix("📅 "));
                if let Some(due_str) = due_str_opt {
                    let days_rem = super::render::iso_days_remaining_pub(due_str.trim(), &today);
                    let (sla_text, sla_col) = if days_rem < 0 {
                        (format!("⏰ OVERDUE by {} day{}", -days_rem, if days_rem == -1 { "" } else { "s" }),
                         Color32::from_rgb(243, 139, 168))
                    } else if days_rem == 0 {
                        ("⏰ Due TODAY".to_string(), Color32::from_rgb(250, 179, 135))
                    } else if days_rem == 1 {
                        ("⏰ Due TOMORROW".to_string(), Color32::from_rgb(249, 226, 175))
                    } else {
                        (format!("⏰ Due in {} days", days_rem), self.theme.text_dim)
                    };
                    rows.push((sla_text, sla_col));
                    // SLA % if created date is set
                    if !node.created_date.is_empty() {
                        let total = -super::render::iso_days_remaining_pub(&node.created_date, due_str.trim());
                        let elapsed = -super::render::iso_days_remaining_pub(&node.created_date, &today);
                        if total > 0 && elapsed >= 0 {
                            let pct = ((elapsed as f32 / total as f32) * 100.0).clamp(0.0, 999.0) as u32;
                            let bar_filled = ((elapsed as f32 / total as f32).clamp(0.0, 1.0) * 10.0) as usize;
                            let bar = format!("{}{}", "█".repeat(bar_filled), "░".repeat(10usize.saturating_sub(bar_filled)));
                            let sla_pct_col = if pct < 50 { Color32::from_rgb(100, 220, 60) }
                                else if pct < 80 { Color32::from_rgb(220, 165, 30) }
                                else { Color32::from_rgb(220, 60, 50) };
                            rows.push((format!("SLA {}% {}", pct, bar), sla_pct_col));
                        }
                    }
                }
                if !node.created_date.is_empty() {
                    let age = -super::render::iso_days_remaining_pub(&node.created_date, &today);
                    if age >= 0 {
                        rows.push((format!("📅 opened {} ({}d ago)", node.created_date, age), self.theme.text_dim.gamma_multiply(0.75)));
                    }
                }
            }
            if !node.comment.is_empty() {
                rows.push((format!("💬 {}", node.comment.chars().take(80).collect::<String>()), self.theme.text_dim.gamma_multiply(0.8)));
            }
            if node.z_offset != 0.0 {
                rows.push((format!("3D layer z={:.0}", node.z_offset), self.theme.accent.gamma_multiply(0.7)));
            }
            // Quick spec hint
            let spec_hint = {
                use crate::model::NodeKind;
                let shape_hint = match &node.kind {
                    NodeKind::Shape { shape, .. } => match shape {
                        crate::model::NodeShape::Diamond => " {diamond}",
                        crate::model::NodeShape::Circle => " {circle}",
                        crate::model::NodeShape::Parallelogram => " {parallelogram}",
                        crate::model::NodeShape::Hexagon => " {hexagon}",
                        crate::model::NodeShape::Connector => " {connector}",
                        _ => "",
                    },
                    NodeKind::Entity { .. } => " {entity}",
                    NodeKind::Text { .. } => " {text}",
                    NodeKind::StickyNote { .. } => "",
                };
                // Use semantic tier name when z matches a standard tier
                let z_hint = if node.z_offset != 0.0 {
                    let tier_name = match node.z_offset as i32 {
                        0   => Some("db"),
                        120 => Some("api"),
                        240 => Some("frontend"),
                        360 => Some("edge"),
                        480 => Some("infra"),
                        _   => None,
                    };
                    if let Some(name) = tier_name {
                        format!(" {{layer:{}}}", name)
                    } else {
                        format!(" {{z:{:.0}}}", node.z_offset)
                    }
                } else { String::new() };
                // 3D per-node extrusion depth (only when non-default)
                let depth_hint = if node.depth_3d > 0.0 {
                    format!(" {{3d-depth:{:.0}}}", node.depth_3d)
                } else { String::new() };
                // Status shorthands: highlight/tag/progress
                let status_hint = match (node.highlight, node.tag) {
                    (true, _) => " {highlight}",
                    (_, Some(crate::model::NodeTag::Critical)) => " {blocked}",
                    (_, Some(crate::model::NodeTag::Warning))  => " {warning}",
                    (_, Some(crate::model::NodeTag::Ok))       => " {done}",
                    (_, Some(crate::model::NodeTag::Info))     => " {info}",
                    _ => "",
                };
                if !shape_hint.is_empty() || !z_hint.is_empty() || !depth_hint.is_empty() || !status_hint.is_empty() {
                    Some(format!("spec: [id] label{}{}{}{}", shape_hint, z_hint, depth_hint, status_hint))
                } else { None }
            };
            if let Some(hint) = spec_hint {
                rows.push((hint, self.theme.text_dim.gamma_multiply(0.6)));
            }
            if has_url  { rows.push(("🔗 ⌘+click to open URL".to_string(), self.theme.text_dim)); }
            if has_comment {
                let preview = if node.comment.len() > 60 {
                    format!("💬 {}…", &node.comment[..60])
                } else {
                    format!("💬 {}", &node.comment)
                };
                rows.push((preview, Color32::from_rgba_unmultiplied(166, 227, 161, 200)));
            }
            if node.locked { rows.push(("🔒 Locked".to_string(), self.theme.text_dim)); }
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

        let bg = self.theme.tooltip_bg;
        let border_col = if rich_mode {
            Color32::from_rgba_unmultiplied(137, 180, 250, (80.0 * alpha_factor) as u8)
        } else {
            self.theme.tooltip_border
        };

        let bg_rect = Rect::from_min_size(Pos2::new(tx, ty), egui::Vec2::new(max_w, total_h));
        painter.rect_filled(bg_rect, egui::CornerRadius::same(6), bg);
        painter.rect_stroke(bg_rect, egui::CornerRadius::same(6),
            egui::Stroke::new(1.0, border_col), egui::StrokeKind::Outside);

        // Label header
        painter.text(Pos2::new(tx + pad, ty + pad), egui::Align2::LEFT_TOP, &label,
            egui::FontId::proportional(12.0), self.theme.text_secondary);

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

        // Rotating contextual message (cycles every 5 seconds, diagram-mode aware)
        let messages: &[&str] = match self.diagram_mode {
            super::DiagramMode::ER => &[
                "Double-click to add your first entity",
                "Cmd+click to add entity fields",
                "Build an entity-relationship diagram",
                "Drag to connect entities with relationships",
                "Press ? for all keyboard shortcuts",
            ],
            super::DiagramMode::FigJam => &[
                "Double-click to add a sticky note",
                "Press N to pick shapes · drag from toolbar",
                "Collaborate with sticky notes and frames",
                "Group ideas, then connect them",
                "Press ? for all keyboard shortcuts",
            ],
            super::DiagramMode::Flowchart => &[
                "Double-click anywhere to add your first node",
                "⌘K → Templates  (40 diagrams: arch · design thinking · support ops)",
                "ICE Scoring · Causal Loop · Theory of Change · Experiment Board — new in ⌘K",
                "Try {hypothesis} {assumption} {evidence} {conclusion}",
                "H = hypothesis · Y = assumption · W = evidence (quick-create)",
                "## Hypotheses / ## Evidence sections get colored backgrounds",
                "S key: cycle status (Todo → WIP → Review → Done → Blocked)",
                "Right-click a node → Move to Section… to organize ideas",
                "SWOT: ## Strengths / ## Weaknesses / ## Opportunities",
                "Lean Canvas · Fishbone · PESTLE · Empathy Map · Premortem — ⌘K",
                "ICE Score = Impact × Confidence × Ease → run highest first",
                "Double Diamond · Hypothesis Canvas · Assumption Map — ⌘K",
                "⌘⇧E = insert Experiment Card (Hypothesis → Test → Result → Learning)",
                "⌘F search: status:done · section:Triage · priority:p1 · orphan · linked",
                "SPEC → Import to load a diagram instantly",
                "Every great theory starts with a single hypothesis",
                "Press ? for all keyboard shortcuts",
            ],
        };
        let slot = ((t / 5.0) as usize) % messages.len();
        // Cross-fade between messages
        let fade_t = (t % 5.0) / 5.0;
        let alpha = if fade_t < 0.1 {
            ((fade_t / 0.1) * 160.0) as u8
        } else if fade_t > 0.85 {
            (((1.0 - fade_t) / 0.15) * 160.0) as u8
        } else {
            160u8
        };
        painter.text(
            center + Vec2::new(0.0, btn_r + 18.0),
            Align2::CENTER_CENTER,
            messages[slot],
            FontId::proportional(12.0),
            self.theme.text_dim.gamma_multiply(alpha as f32 / 160.0),
        );

        // Quick-start shortcut row
        let hints = [("H", "hypothesis"), ("N", "shape"), ("E", "connect"), ("⌘K", "commands"), ("?", "help")];
        let total_w = hints.len() as f32 * 50.0;
        let row_y = center.y + btn_r + 40.0;
        let start_x = center.x - total_w / 2.0 + 4.0;
        for (i, (key, desc)) in hints.iter().enumerate() {
            let x = start_x + i as f32 * 50.0;
            painter.text(Pos2::new(x, row_y), Align2::LEFT_CENTER,
                *key, FontId::proportional(10.0), self.theme.accent.gamma_multiply(0.7));
            painter.text(Pos2::new(x, row_y + 12.0), Align2::LEFT_CENTER,
                *desc, FontId::proportional(8.5), self.theme.text_dim.gamma_multiply(0.5));
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
            if let Some(tf) = self.tag_filter { parts.push(format!("filter:{}", tf.label())); }
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

        painter.text(Pos2::new(x, y), egui::Align2::LEFT_TOP, &line1, font_big, self.theme.text_secondary);
        painter.text(Pos2::new(x, y + 17.0), egui::Align2::LEFT_TOP, &line2, font_sm, self.theme.text_dim);
        let mut next_y = y + 29.0;
        if !line3.is_empty() {
            painter.text(Pos2::new(x, next_y), egui::Align2::LEFT_TOP, &line3, font_xs, self.theme.text_dim);
            next_y += 11.0;
        }
        if let Some(ref cl) = cursor_line {
            painter.text(Pos2::new(x, next_y), egui::Align2::LEFT_TOP, cl,
                egui::FontId::proportional(9.5),
                self.theme.accent.gamma_multiply(0.7)); // theme accent tint
        }
    }

    /// Top-right overlay: per-section hypothesis validation progress.
    /// Shows Done/WIP/Blocked counts per section when the doc has sections with status tags.
    /// Handle click on the section progress summary panel (top-right).
    /// Clicking a section row selects all nodes in that section.
    fn handle_section_summary_click(&mut self, ui: &egui::Ui, canvas_rect: Rect) {
        use std::collections::HashMap;

        let pointer = ui.ctx().input(|i| i.pointer.press_origin());
        let Some(click_pos) = pointer else { return };
        if !ui.ctx().input(|i| i.pointer.any_released()) { return }

        // Compute section list (same logic as draw function)
        let mut sections: HashMap<String, [u32; 5]> = HashMap::new();
        let mut has_any_tag = false;
        for node in &self.document.nodes {
            if node.section_name.is_empty() { continue; }
            let counts = sections.entry(node.section_name.clone()).or_insert([0u32; 5]);
            counts[4] += 1;
            match node.tag {
                Some(crate::model::NodeTag::Ok)       => { counts[0] += 1; has_any_tag = true; }
                Some(crate::model::NodeTag::Info)     => { counts[1] += 1; has_any_tag = true; }
                Some(crate::model::NodeTag::Warning) if node.progress >= 0.5 => { counts[2] += 1; has_any_tag = true; }
                Some(crate::model::NodeTag::Critical) => { counts[3] += 1; has_any_tag = true; }
                _ => {}
            }
        }
        if sections.is_empty() || !has_any_tag { return; }

        let mut sorted: Vec<String> = sections.keys().cloned().collect();
        sorted.sort();

        let panel_w = 170.0_f32;
        let panel_x = canvas_rect.max.x - panel_w - 12.0;
        let panel_y = canvas_rect.min.y + 12.0;
        let pad_y   = 8.0;
        let row_h   = 15.0;
        let header_h = 14.0;
        let rows_start_y = panel_y + pad_y + header_h;

        // Check if click is on any section row
        for (i, section_name) in sorted.iter().enumerate() {
            let row_y = rows_start_y + i as f32 * row_h;
            let row_rect = Rect::from_min_size(
                Pos2::new(panel_x, row_y),
                egui::Vec2::new(panel_w, row_h),
            );
            if row_rect.contains(click_pos) {
                // Select all nodes in this section
                self.selection.clear();
                let count = self.document.nodes.iter()
                    .filter(|n| &n.section_name == section_name)
                    .count();
                for node in &self.document.nodes {
                    if &node.section_name == section_name {
                        self.selection.node_ids.insert(node.id);
                    }
                }
                if count > 0 {
                    self.zoom_to_selection();
                    self.status_message = Some((
                        format!("Selected {} nodes in \"{}\"", count, section_name),
                        std::time::Instant::now(),
                    ));
                }
                break;
            }
        }
    }

    /// Horizontal pill strip at bottom-center: one pill per section, click to pan there.
    fn draw_section_navigator(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        // Collect unique sections with counts, dominant tag color, and overdue count
        use std::collections::BTreeMap;
        #[derive(Default)]
        struct SecInfo { total: u32, critical: u32, warning: u32, ok: u32, info: u32, overdue: u32, critical_overdue: u32 }
        let today = super::render::today_iso();
        let mut sec_map: BTreeMap<String, SecInfo> = BTreeMap::new();
        for node in &self.document.nodes {
            if node.section_name.is_empty() { continue; }
            let e = sec_map.entry(node.section_name.clone()).or_default();
            e.total += 1;
            match node.tag {
                Some(crate::model::NodeTag::Critical) => e.critical += 1,
                Some(crate::model::NodeTag::Warning)  => e.warning += 1,
                Some(crate::model::NodeTag::Ok)        => e.ok += 1,
                Some(crate::model::NodeTag::Info)      => e.info += 1,
                None => {}
            }
            // Count overdue (📅 date in past)
            let is_overdue = node.sublabel.split('\n').any(|line| {
                if let Some(ds) = line.strip_prefix("📅 ") {
                    let d = ds.trim();
                    d.len() >= 8 && d < today.as_str()
                } else { false }
            });
            if is_overdue {
                e.overdue += 1;
                if matches!(node.tag, Some(crate::model::NodeTag::Critical)) {
                    e.critical_overdue += 1;
                }
            }
        }
        if sec_map.is_empty() { return; }

        let pill_h = 22.0_f32;
        let pill_pad_x = 10.0_f32;
        let pill_gap = 5.0_f32;
        let bottom_margin = 36.0_f32; // above status bar

        // Compute pill widths
        let sections: Vec<(String, SecInfo)> = sec_map.into_iter().collect();
        let font = FontId::proportional(10.5);
        // Estimate pill widths (approx 6.5px/char + padding); extra width for overdue badge
        let pill_widths: Vec<f32> = sections.iter().map(|(name, info)| {
            let chars = name.chars().count().min(14);
            let count_chars = format!("{}", info.total).len();
            let overdue_extra = if info.overdue > 0 { 26.0 } else { 0.0 };
            let fire_extra = if info.critical_overdue > 0 { 18.0 } else { 0.0 };
            (chars + count_chars + 2) as f32 * 6.5 + pill_pad_x * 2.0 + 14.0 + overdue_extra + fire_extra
        }).collect();
        let total_w: f32 = pill_widths.iter().sum::<f32>() + pill_gap * (sections.len().saturating_sub(1)) as f32;

        // Center horizontally, bottom-anchored
        let start_x = canvas_rect.center().x - total_w / 2.0;
        let pill_y = canvas_rect.max.y - bottom_margin - pill_h;

        let hover_pos = ui.ctx().input(|i| i.pointer.hover_pos());
        let clicked   = ui.ctx().input(|i| i.pointer.primary_clicked());

        let mut x = start_x;
        for ((name, info), pw) in sections.iter().zip(pill_widths.iter()) {
            let pill_rect = Rect::from_min_size(
                Pos2::new(x, pill_y),
                Vec2::new(*pw, pill_h),
            );
            let hovered = hover_pos.map_or(false, |p| pill_rect.contains(p));

            // Dominant status color for pill border
            let border_col = if info.critical > 0 {
                Color32::from_rgb(243, 139, 168)
            } else if info.warning > 0 {
                Color32::from_rgb(250, 179, 135)
            } else if info.ok > 0 {
                Color32::from_rgb(166, 227, 161)
            } else if info.info > 0 {
                Color32::from_rgb(137, 180, 250)
            } else {
                self.theme.accent.gamma_multiply(0.4)
            };

            let bg = if hovered {
                Color32::from_rgba_unmultiplied(30, 30, 50, 230)
            } else {
                Color32::from_rgba_unmultiplied(18, 18, 28, 200)
            };
            let painter = ui.painter();

            // 🔥 on-fire halo: pulsing orange glow for pills with P1 (Critical) overdue tickets
            if info.critical_overdue > 0 {
                let t = ui.ctx().input(|i| i.time) as f32;
                // Pulse at ~1.2 Hz between alpha 60 and 160
                let pulse = ((t * 1.2 * std::f32::consts::TAU).sin() * 0.5 + 0.5) as f32;
                let glow_alpha = (60.0 + pulse * 100.0) as u8;
                let glow_expand = 3.0 + pulse * 3.0;
                let glow_rect = pill_rect.expand(glow_expand);
                painter.rect(glow_rect, CornerRadius::same((11.0 + glow_expand) as u8),
                    Color32::TRANSPARENT,
                    Stroke::new(2.5, Color32::from_rgba_unmultiplied(255, 120, 30, glow_alpha)),
                    StrokeKind::Outside);
                // Second tighter ring
                let inner_rect = pill_rect.expand(1.0);
                painter.rect(inner_rect, CornerRadius::same(12_u8),
                    Color32::TRANSPARENT,
                    Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 80, 20, (glow_alpha / 2).saturating_add(40))),
                    StrokeKind::Outside);
                ui.ctx().request_repaint();
            }

            painter.rect(pill_rect, CornerRadius::same(11),
                bg, Stroke::new(1.0, border_col), StrokeKind::Inside);

            // Label: truncate name + total count; 🔥 prefix if P1 overdue
            let short: String = name.chars().take(14).collect();
            let trail = if name.chars().count() > 14 { "…" } else { "" };
            let fire_prefix = if info.critical_overdue > 0 { "🔥 " } else { "" };
            let disp = format!("{}{}{} {}", fire_prefix, short, trail, info.total);
            let label_col = if hovered { self.theme.text_primary } else { self.theme.text_secondary };

            // If there are overdue items, shift label left and draw a red ⚠N badge on right
            if info.overdue > 0 {
                let badge_w = 22.0_f32;
                let badge_h = 14.0_f32;
                let badge_x = pill_rect.max.x - badge_w - 4.0;
                let badge_y = pill_rect.center().y - badge_h / 2.0;
                let badge_rect = Rect::from_min_size(Pos2::new(badge_x, badge_y), Vec2::new(badge_w, badge_h));
                painter.rect_filled(badge_rect, CornerRadius::same(7),
                    Color32::from_rgb(185, 50, 70));
                let badge_text = format!("⚠{}", info.overdue);
                painter.text(badge_rect.center(), Align2::CENTER_CENTER, &badge_text,
                    FontId::proportional(8.5), Color32::WHITE);
                // Label shifted left of badge
                let label_x = pill_rect.min.x + pill_pad_x + (pill_rect.width() - badge_w - 6.0) / 2.0;
                painter.text(
                    Pos2::new(label_x, pill_rect.center().y),
                    Align2::CENTER_CENTER, &disp,
                    font.clone(), label_col,
                );
            } else {
                painter.text(
                    pill_rect.center(),
                    Align2::CENTER_CENTER, &disp,
                    font.clone(), label_col,
                );
            }

            // Pan on click
            if hovered && clicked {
                let sec_rects: Vec<Rect> = self.document.nodes.iter()
                    .filter(|n| &n.section_name == name)
                    .map(|n| n.rect())
                    .collect();
                if let Some(first) = sec_rects.first() {
                    let mut bb = *first;
                    for r in &sec_rects[1..] { bb = bb.union(*r); }
                    let zoom = self.viewport.zoom;
                    let cx = canvas_rect.center().x;
                    let cy = canvas_rect.center().y;
                    let target_ox = cx - bb.center().x * zoom;
                    let target_oy = cy - bb.center().y * zoom;
                    self.pan_target = Some([target_ox, target_oy]);
                    self.status_message = Some((
                        format!("→ \"{}\" ({} nodes)", name, info.total),
                        std::time::Instant::now(),
                    ));
                }
            }

            x += pw + pill_gap;
        }
    }

    fn draw_section_progress_summary(&self, painter: &egui::Painter, canvas_rect: Rect) {
        use std::collections::HashMap;

        // Collect per-section status counts
        let mut sections: HashMap<String, [u32; 5]> = HashMap::new(); // [done, wip, review, blocked, total]
        let mut has_any_tag = false;

        for node in &self.document.nodes {
            if node.section_name.is_empty() { continue; }
            let counts = sections.entry(node.section_name.clone()).or_insert([0u32; 5]);
            counts[4] += 1; // total
            match node.tag {
                Some(crate::model::NodeTag::Ok) => { counts[0] += 1; has_any_tag = true; }
                Some(crate::model::NodeTag::Info) => { counts[1] += 1; has_any_tag = true; }
                Some(crate::model::NodeTag::Warning) if node.progress >= 0.5 => { counts[2] += 1; has_any_tag = true; }
                Some(crate::model::NodeTag::Critical) => { counts[3] += 1; has_any_tag = true; }
                _ => {}
            }
        }

        if sections.is_empty() || !has_any_tag { return; }

        // Sort sections by name for stable ordering
        let mut sorted_sections: Vec<(String, [u32; 5])> = sections.into_iter().collect();
        sorted_sections.sort_by(|a, b| a.0.cmp(&b.0));

        let font = egui::FontId::proportional(10.5);
        let font_hd = egui::FontId::proportional(9.5);
        let row_h = 15.0;
        let pad_x = 10.0;
        let pad_y = 8.0;
        let panel_w = 170.0_f32;
        let panel_h = pad_y * 2.0 + 14.0 + row_h * sorted_sections.len() as f32;

        let panel_x = canvas_rect.max.x - panel_w - 12.0;
        let panel_y = canvas_rect.min.y + 12.0;
        let panel_rect = Rect::from_min_size(
            Pos2::new(panel_x, panel_y),
            egui::Vec2::new(panel_w, panel_h),
        );

        // Panel background
        let bg = egui::Color32::from_rgba_unmultiplied(18, 18, 28, 210);
        let border = egui::Color32::from_rgba_unmultiplied(120, 120, 150, 60);
        painter.rect(panel_rect, egui::CornerRadius::same(8),
            bg, egui::Stroke::new(1.0, border), egui::StrokeKind::Inside);

        // Header
        let header_color = egui::Color32::from_rgba_unmultiplied(180, 180, 220, 170);
        painter.text(
            Pos2::new(panel_x + pad_x, panel_y + pad_y),
            egui::Align2::LEFT_TOP,
            "Section Progress",
            font_hd.clone(),
            header_color,
        );

        // Legend dots (right-aligned header row)
        let legend_x = panel_x + panel_w - pad_x;
        let legend_y = panel_y + pad_y + 1.0;
        for (i, (sym, col)) in [("✅", egui::Color32::from_rgb(166, 227, 161)),
                                  ("🔄", egui::Color32::from_rgb(137, 180, 250)),
                                  ("⛔", egui::Color32::from_rgb(243, 139, 168))].iter().enumerate().rev() {
            let lx = legend_x - i as f32 * 22.0;
            painter.text(Pos2::new(lx, legend_y), egui::Align2::RIGHT_TOP,
                sym, egui::FontId::proportional(9.0), *col);
        }

        // Section rows
        let color_done    = egui::Color32::from_rgb(166, 227, 161);
        let color_wip     = egui::Color32::from_rgb(137, 180, 250);
        let color_blocked = egui::Color32::from_rgb(243, 139, 168);
        let color_text    = egui::Color32::from_rgba_unmultiplied(210, 210, 230, 200);

        // Hover detection for row highlight
        let hover_pos = painter.ctx().input(|i| i.pointer.hover_pos());

        for (i, (sec_name, counts)) in sorted_sections.iter().enumerate() {
            let ry = panel_y + pad_y + 14.0 + row_h * i as f32;
            let row_rect = Rect::from_min_size(
                Pos2::new(panel_x + 2.0, ry - 1.0),
                egui::Vec2::new(panel_w - 4.0, row_h),
            );

            // Hover highlight
            if let Some(hp) = hover_pos {
                if row_rect.contains(hp) {
                    painter.rect_filled(row_rect, egui::CornerRadius::same(3),
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12));
                }
            }

            let [done, wip, _review, blocked, total] = *counts;

            // Completion indicator: ✅ prefix when all tagged nodes are done
            let tagged = done + wip + blocked;
            let all_done = tagged > 0 && tagged == done;
            let name_prefix = if all_done { "✅ " } else { "" };

            // Truncate section name to fit (accounting for prefix)
            let max_chars = if all_done { 11 } else { 14 };
            let display = if sec_name.chars().count() > max_chars {
                format!("{name_prefix}{}…", sec_name.chars().take(max_chars - 1).collect::<String>())
            } else {
                format!("{name_prefix}{sec_name}")
            };

            let name_col = if all_done { color_done } else { color_text };
            painter.text(Pos2::new(panel_x + pad_x, ry), egui::Align2::LEFT_TOP,
                &display, font.clone(), name_col);

            // Count columns (aligned right)
            let rx = panel_x + panel_w - pad_x;

            painter.text(Pos2::new(rx - 0.0, ry), egui::Align2::RIGHT_TOP,
                &format!("{blocked}"), font.clone(), if blocked > 0 { color_blocked } else { egui::Color32::from_rgba_unmultiplied(100, 100, 120, 100) });
            painter.text(Pos2::new(rx - 22.0, ry), egui::Align2::RIGHT_TOP,
                &format!("{wip}"), font.clone(), if wip > 0 { color_wip } else { egui::Color32::from_rgba_unmultiplied(100, 100, 120, 100) });
            painter.text(Pos2::new(rx - 44.0, ry), egui::Align2::RIGHT_TOP,
                &format!("{done}"), font.clone(), if done > 0 { color_done } else { egui::Color32::from_rgba_unmultiplied(100, 100, 120, 100) });
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
                    painter.rect_stroke(bg_rect, CornerRadius::same(4), Stroke::new(1.0, self.theme.surface1), StrokeKind::Outside);
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
                painter2.rect_filled(bar_rect, CornerRadius::same(6), self.theme.tooltip_bg);
                painter2.rect_stroke(bar_rect, CornerRadius::same(6), Stroke::new(1.0, self.theme.accent.gamma_multiply(0.4)), StrokeKind::Outside);

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
            ("→", "Spawn connected child node"),
            ("🗑", "Delete"),
        ];
        let btn_w = 28.0;
        // For single-node selection, include status badge + section pill at the end
        let show_status_badge = self.selection.node_ids.len() == 1;
        let status_badge_w = if show_status_badge { 38.0 } else { 0.0 };
        let status_sep_w   = if show_status_badge { 6.0  } else { 0.0 };
        // Section pill width: depends on section name length (capped at 12 chars)
        let section_pill_w: f32 = if show_status_badge {
            let sec = self.selection.node_ids.iter().next()
                .and_then(|id| self.document.find_node(id))
                .map(|n| n.section_name.clone())
                .unwrap_or_default();
            let chars = sec.chars().count().min(12);
            if chars == 0 { 32.0 } else { chars as f32 * 6.5 + 18.0 }
        } else { 0.0 };
        let section_sep_w = if show_status_badge { 4.0 } else { 0.0 };
        // Assignee chip: show "👤 Name" for single-node if node has an assignee
        let assignee_str: Option<String> = if show_status_badge {
            self.selection.node_ids.iter().next()
                .and_then(|id| self.document.find_node(id))
                .and_then(|n| n.sublabel.lines().find(|l| l.starts_with("👤 ")).map(|l| l.to_string()))
        } else { None };
        let assignee_chip_w: f32 = if let Some(ref a) = assignee_str {
            let chars = a.chars().count().min(14);
            chars as f32 * 6.2 + 14.0
        } else { 0.0 };
        let assignee_sep_w = if assignee_str.is_some() { 4.0 } else { 0.0 };
        let bar_w = actions.len() as f32 * btn_w + (actions.len() - 1) as f32 * 2.0 + 8.0
            + status_badge_w + status_sep_w + section_pill_w + section_sep_w
            + assignee_chip_w + assignee_sep_w;
        let bar_rect = Rect::from_center_size(
            egui::Pos2::new(bar_center_x, bar_y + bar_h / 2.0),
            egui::Vec2::new(bar_w, bar_h),
        );

        // Draw bar background via painter
        let painter = ui.painter();
        painter.rect_filled(bar_rect, CornerRadius::same(6), self.theme.tooltip_bg);
        painter.rect_stroke(bar_rect, CornerRadius::same(6), Stroke::new(1.0, self.theme.surface1), StrokeKind::Outside);

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

        // Status badge for single-node selection
        let mut status_clicked = false;
        if show_status_badge {
            if let Some(&sel_id) = self.selection.node_ids.iter().next() {
                let (badge_label, badge_color) = if let Some(node) = self.document.find_node(&sel_id) {
                    match (node.tag, node.progress) {
                        (Some(crate::model::NodeTag::Ok), p) if p >= 0.99 => ("✅", egui::Color32::from_rgb(166, 227, 161)),
                        (Some(crate::model::NodeTag::Info), _)             => ("🔄", egui::Color32::from_rgb(137, 180, 250)),
                        (Some(crate::model::NodeTag::Warning), p) if p > 0.5 => ("👁", egui::Color32::from_rgb(249, 226, 175)),
                        (Some(crate::model::NodeTag::Critical), _)         => ("⛔", egui::Color32::from_rgb(243, 139, 168)),
                        (Some(crate::model::NodeTag::Warning), _)          => ("📋", egui::Color32::from_rgb(203, 166, 247)),
                        _                                                  => ("○", self.theme.text_dim.gamma_multiply(0.5)),
                    }
                } else { ("○", self.theme.text_dim.gamma_multiply(0.5)) };

                // Separator line
                let sep_x = x + 2.0;
                let sep_rect = Rect::from_min_size(
                    egui::Pos2::new(sep_x, bar_rect.min.y + 4.0),
                    egui::Vec2::new(1.0, bar_h - 8.0),
                );
                ui.painter().rect_filled(sep_rect, egui::CornerRadius::ZERO, self.theme.surface1);
                x += status_sep_w;

                let badge_rect = Rect::from_min_size(
                    egui::Pos2::new(x, bar_rect.min.y + 2.0),
                    egui::Vec2::new(status_badge_w - 2.0, bar_h - 4.0),
                );
                let resp = ui.put(badge_rect, egui::Button::new(
                    egui::RichText::new(badge_label).size(12.0).color(badge_color)
                ).frame(false)).on_hover_text("Click to cycle status: None → Todo → WIP → Done → Blocked");
                if resp.clicked() { status_clicked = true; }
                x += status_badge_w;
            }
        }

        // Section pill: shows current section, click cycles support workflow stages
        let mut section_clicked = false;
        if show_status_badge {
            if let Some(&sel_id) = self.selection.node_ids.iter().next() {
                let cur_section = self.document.find_node(&sel_id)
                    .map(|n| n.section_name.clone())
                    .unwrap_or_default();

                // Separator
                let sep_x = x + 1.0;
                ui.painter().rect_filled(
                    Rect::from_min_size(egui::Pos2::new(sep_x, bar_rect.min.y + 4.0), egui::Vec2::new(1.0, bar_h - 8.0)),
                    egui::CornerRadius::ZERO, self.theme.surface1,
                );
                x += section_sep_w;

                let sec_label = if cur_section.is_empty() {
                    "§".to_string()
                } else {
                    let short: String = cur_section.chars().take(12).collect();
                    format!("§ {}", short)
                };
                let sec_col = if cur_section.is_empty() { self.theme.text_dim } else { self.theme.accent.gamma_multiply(0.8) };
                let pill_rect = Rect::from_min_size(
                    egui::Pos2::new(x, bar_rect.min.y + 2.0),
                    egui::Vec2::new(section_pill_w - 2.0, bar_h - 4.0),
                );
                let resp = ui.put(pill_rect, egui::Button::new(
                    egui::RichText::new(&sec_label).size(11.0).color(sec_col)
                ).frame(false)).on_hover_text("Click to cycle section: Intake → Triage → In Progress → Resolved → Escalated → Closed → none");
                if resp.clicked() { section_clicked = true; }
                x += section_pill_w; // advance x for assignee chip
            }
        }

        // Assignee chip: "👤 Name" — clicking activates assignee search filter
        let mut assignee_filter_clicked = false;
        if let Some(ref a_str) = assignee_str {
            // Separator
            ui.painter().rect_filled(
                Rect::from_min_size(egui::Pos2::new(x + 1.0, bar_rect.min.y + 4.0), egui::Vec2::new(1.0, bar_h - 8.0)),
                egui::CornerRadius::ZERO, self.theme.surface1,
            );
            x += assignee_sep_w;
            let short: String = a_str.chars().take(14).collect();
            let chip_rect = Rect::from_min_size(
                egui::Pos2::new(x, bar_rect.min.y + 2.0),
                egui::Vec2::new(assignee_chip_w - 2.0, bar_h - 4.0),
            );
            let resp = ui.put(chip_rect, egui::Button::new(
                egui::RichText::new(&short).size(10.5).color(Color32::from_rgb(180, 210, 255))
            ).frame(false)).on_hover_text("Click to filter by this assignee");
            if resp.clicked() { assignee_filter_clicked = true; }
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
                3 => { // Spawn connected child node
                    if let Some(&src_id) = self.selection.node_ids.iter().next() {
                        if let Some(src) = self.document.find_node(&src_id).cloned() {
                            // Offset direction based on document layout (LR = right, TB = down)
                            let (dx, dy) = if self.document.layout_dir == "TB" || self.document.layout_dir == "BT" {
                                (0.0_f32, src.size[1] + 80.0)
                            } else {
                                (src.size[0] + 100.0, 0.0_f32)
                            };
                            let new_pos = egui::Pos2::new(src.position[0] + dx, src.position[1] + dy);
                            let mut child = Node::new(
                                match &src.kind {
                                    NodeKind::Shape { shape, .. } => *shape,
                                    _ => NodeShape::RoundedRect,
                                },
                                new_pos,
                            );
                            child.size = src.size;
                            child.style = src.style.clone();
                            child.section_name = src.section_name.clone();
                            child.z_offset = src.z_offset;
                            // Clear status so new node starts fresh
                            child.tag = None;
                            child.progress = 0.0;
                            let child_id = child.id;
                            self.document.nodes.push(child);
                            // Create edge src → child, respecting layout direction
                            let is_tb = matches!(self.document.layout_dir.as_str(), "TB" | "BT");
                            let (src_side, tgt_side) = if is_tb {
                                (crate::model::PortSide::Bottom, crate::model::PortSide::Top)
                            } else {
                                (crate::model::PortSide::Right, crate::model::PortSide::Left)
                            };
                            let edge = Edge::new(
                                Port { node_id: src_id, side: src_side },
                                Port { node_id: child_id, side: tgt_side },
                            );
                            self.document.edges.push(edge);
                            self.selection.clear();
                            self.selection.select_node(child_id);
                            self.inline_node_edit = Some((child_id, String::new()));
                            self.history.push(&self.document);
                            self.status_message = Some(("Child node spawned — type to label".to_string(), std::time::Instant::now()));
                        }
                    }
                }
                4 => { // Delete
                    let ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
                    for id in &ids { self.document.remove_node(id); }
                    self.selection.clear();
                    self.history.push(&self.document);
                }
                _ => {}
            }
        }

        // Cycle status on status badge click
        if status_clicked {
            if let Some(&sel_id) = self.selection.node_ids.iter().next() {
                if let Some(node) = self.document.find_node_mut(&sel_id) {
                    let (new_tag, new_progress, label) = match node.tag {
                        None => (Some(crate::model::NodeTag::Warning), 0.0, "Todo"),
                        Some(crate::model::NodeTag::Warning) if node.progress < 0.5 => {
                            (Some(crate::model::NodeTag::Info), 0.5, "WIP")
                        }
                        Some(crate::model::NodeTag::Info) => {
                            (Some(crate::model::NodeTag::Warning), 0.75, "Review")
                        }
                        Some(crate::model::NodeTag::Warning) => {
                            (Some(crate::model::NodeTag::Ok), 1.0, "Done")
                        }
                        Some(crate::model::NodeTag::Ok) => {
                            (Some(crate::model::NodeTag::Critical), 0.0, "Blocked")
                        }
                        Some(crate::model::NodeTag::Critical) => (None, 0.0, "None"),
                    };
                    node.tag = new_tag;
                    node.progress = new_progress;
                    // (label is used below for fallback status message)
                    let _ = label; // suppress "not used" warning if no celebration
                }
                // Check if all nodes in node's section are now Done
                let celebration = self.document.find_node(&sel_id)
                    .filter(|n| !n.section_name.is_empty())
                    .map(|n| n.section_name.clone())
                    .and_then(|sec| {
                        let nodes_in_sec: Vec<_> = self.document.nodes.iter()
                            .filter(|n| n.section_name == sec)
                            .collect();
                        if !nodes_in_sec.is_empty()
                            && nodes_in_sec.iter().all(|n| matches!(n.tag, Some(crate::model::NodeTag::Ok)))
                        {
                            Some(format!("🎉 All done in \"{sec}\"!"))
                        } else {
                            None
                        }
                    });
                let msg = celebration.unwrap_or_else(|| "Status updated".to_string());
                self.status_message = Some((msg, std::time::Instant::now()));
                self.history.push(&self.document);
            }
        }

        // Cycle section on section pill click
        if section_clicked {
            if let Some(&sel_id) = self.selection.node_ids.iter().next() {
                let support_stages = ["Intake", "Triage", "In Progress", "Resolved", "Escalated", "Closed"];
                if let Some(node) = self.document.find_node_mut(&sel_id) {
                    let cur = &node.section_name;
                    let next = if cur.is_empty() {
                        "Intake".to_string()
                    } else if let Some(idx) = support_stages.iter().position(|&s| s == cur.as_str()) {
                        let next_idx = (idx + 1) % (support_stages.len() + 1);
                        if next_idx >= support_stages.len() { String::new() } else { support_stages[next_idx].to_string() }
                    } else {
                        String::new() // unknown section → clear
                    };
                    let msg = if next.is_empty() {
                        "Section cleared".to_string()
                    } else {
                        format!("→ {}", next)
                    };
                    node.section_name = next;
                    self.status_message = Some((msg, std::time::Instant::now()));
                    self.history.push(&self.document);
                }
            }
        }

        // Assignee chip click → activate assignee search filter
        if assignee_filter_clicked {
            if let Some(ref a_str) = assignee_str {
                // Extract the name part after "👤 "
                let name = a_str.strip_prefix("👤 ").unwrap_or(a_str.as_str()).trim().to_string();
                self.search_query = format!("assigned:{}", name);
                self.persist_search_filter = true;
                self.show_search = false;
                self.status_message = Some((
                    format!("Filtering: {}", a_str),
                    std::time::Instant::now(),
                ));
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
            painter2.rect_filled(abar_rect, CornerRadius::same(6), self.theme.tooltip_bg);
            painter2.rect_stroke(abar_rect, CornerRadius::same(6), Stroke::new(1.0, self.theme.accent.gamma_multiply(0.5)), StrokeKind::Outside);

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

        // Status summary below title (only when tagged nodes exist)
        let done = self.document.nodes.iter().filter(|n| matches!(n.tag, Some(crate::model::NodeTag::Ok))).count();
        let total = self.document.nodes.iter().filter(|n| n.tag.is_some()).count();
        if total > 0 {
            let summary = format!("{done}/{total} done");
            let summary_color = if done == total {
                Color32::from_rgba_unmultiplied(166, 227, 161, 160) // green when all done
            } else {
                Color32::from_rgba_premultiplied(150, 150, 180, 80)
            };
            let summary_pos = Pos2::new(canvas_rect.min.x + 20.0, canvas_rect.min.y + 36.0);
            painter.text(summary_pos, Align2::LEFT_TOP, &summary,
                FontId::proportional(10.5), summary_color);
        }
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

    /// Workload summary panel (Cmd+Shift+W): assignee × section ticket counts
    fn draw_workload_panel(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        use std::collections::{BTreeMap, BTreeSet};

        // Collect sections in order of first occurrence
        let mut section_order: Vec<String> = Vec::new();
        let mut seen_sections: BTreeSet<String> = BTreeSet::new();
        for n in &self.document.nodes {
            if !n.section_name.is_empty() && !seen_sections.contains(&n.section_name) {
                section_order.push(n.section_name.clone());
                seen_sections.insert(n.section_name.clone());
            }
        }

        // Build: assignee -> (section -> count)
        let mut workload: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();
        for n in &self.document.nodes {
            let assignee = n.sublabel.lines()
                .find(|l| l.starts_with("👤 "))
                .and_then(|l| l.strip_prefix("👤 "))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Unassigned".to_string());
            let section = if n.section_name.is_empty() { "—".to_string() } else { n.section_name.clone() };
            *workload.entry(assignee).or_default().entry(section).or_default() += 1;
        }
        if workload.is_empty() { self.show_workload_panel = false; return; }

        let row_h = 18.0_f32;
        let col_w = 56.0_f32;
        let label_w = 90.0_f32;
        let header_h = 32.0_f32;
        let panel_w = label_w + col_w * section_order.len() as f32 + col_w + 16.0;
        let panel_h = header_h + row_h * workload.len() as f32 + 24.0;
        let panel_w = panel_w.clamp(240.0, canvas_rect.width() - 32.0);
        let margin = 16.0_f32;
        let panel_rect = Rect::from_min_size(
            egui::pos2(
                canvas_rect.max.x - panel_w - margin - 200.0, // offset left of properties panel
                canvas_rect.min.y + margin + 14.0,
            ),
            egui::vec2(panel_w, panel_h),
        );

        {
            let painter = ui.painter();
            painter.rect_filled(panel_rect, CornerRadius::same(8), self.theme.tooltip_bg);
            painter.rect_stroke(panel_rect, CornerRadius::same(8),
                Stroke::new(1.2, self.theme.surface1), StrokeKind::Outside);
            // Title
            painter.text(
                panel_rect.min + egui::vec2(10.0, 8.0), Align2::LEFT_TOP,
                "👥 Workload  ⌘⇧W",
                egui::FontId::proportional(11.0), self.theme.text_secondary,
            );
        }

        // Pass 1: compute row data and process interactions
        struct RowData {
            row_y: f32, short_name: String, name_rect: Rect, is_filtered: bool,
            is_hovered: bool, clicked: bool,
            section_cells: Vec<(Rect, Color32, String)>, // (rect, color, text)
            total: u32, total_x: f32,
        }
        let mut rows_data: Vec<RowData> = Vec::new();
        let mut row_y = panel_rect.min.y + header_h;
        for (assignee, section_counts) in &workload {
            let name_rect = Rect::from_min_size(
                egui::pos2(panel_rect.min.x + 2.0, row_y),
                egui::vec2(label_w - 4.0, row_h),
            );
            let name_resp = ui.allocate_rect(name_rect, egui::Sense::click());
            let is_filtered = self.search_query == format!("assignee:{}", assignee);
            if name_resp.clicked() {
                let filter = format!("assignee:{}", assignee);
                if self.search_query == filter {
                    self.search_query.clear();
                    self.persist_search_filter = false;
                    self.status_message = Some(("Filter cleared".to_string(), std::time::Instant::now()));
                } else {
                    self.search_query = filter;
                    self.persist_search_filter = true;
                    self.status_message = Some((format!("Showing: {}", assignee), std::time::Instant::now()));
                }
            }
            let mut section_cells: Vec<(Rect, Color32, String)> = Vec::new();
            let mut col_x2 = panel_rect.min.x + label_w;
            let mut total = 0u32;
            for sec in &section_order {
                let count = section_counts.get(sec).copied().unwrap_or(0);
                total += count;
                if count > 0 {
                    let sec_idx = section_order.iter().position(|s| s == sec).unwrap_or(0);
                    let frac = sec_idx as f32 / section_order.len().max(1) as f32;
                    let cell_color = if frac >= 0.75 {
                        Color32::from_rgba_unmultiplied(166, 227, 161, 200)
                    } else if frac >= 0.5 {
                        Color32::from_rgba_unmultiplied(137, 180, 250, 200)
                    } else if frac >= 0.25 {
                        Color32::from_rgba_unmultiplied(250, 179, 135, 200)
                    } else {
                        Color32::from_rgba_unmultiplied(203, 166, 247, 200)
                    };
                    let cell_rect = Rect::from_center_size(
                        egui::pos2(col_x2 + col_w * 0.5, row_y + row_h * 0.5),
                        egui::vec2(col_w - 8.0, row_h - 4.0),
                    );
                    section_cells.push((cell_rect, cell_color, count.to_string()));
                }
                col_x2 += col_w;
            }
            let short_name = if assignee.len() > 11 { format!("{}…", &assignee[..10]) } else { assignee.clone() };
            rows_data.push(RowData {
                row_y, short_name, name_rect, is_filtered,
                is_hovered: name_resp.hovered(), clicked: name_resp.clicked(),
                section_cells, total, total_x: col_x2,
            });
            row_y += row_h;
        }

        // Pass 2: draw everything
        {
            let painter = ui.painter();
            // Column headers
            let header_y = panel_rect.min.y + header_h - row_h;
            let mut col_x = panel_rect.min.x + label_w;
            for sec in &section_order {
                let short = if sec.len() > 6 { format!("{}…", &sec[..5]) } else { sec.clone() };
                painter.text(
                    egui::pos2(col_x + col_w * 0.5, header_y), Align2::CENTER_BOTTOM,
                    &short, egui::FontId::proportional(9.5), self.theme.text_secondary,
                );
                col_x += col_w;
            }
            painter.text(
                egui::pos2(col_x + col_w * 0.5, header_y), Align2::CENTER_BOTTOM,
                "Total", egui::FontId::proportional(9.5), self.theme.text_secondary,
            );
            // Rows
            for rd in &rows_data {
                let row_rect = Rect::from_min_size(
                    egui::pos2(panel_rect.min.x, rd.row_y),
                    egui::vec2(panel_w, row_h),
                );
                if (rd.row_y as i32 / row_h as i32) % 2 == 0 {
                    painter.rect_filled(row_rect, CornerRadius::ZERO, self.theme.surface0.linear_multiply(0.5));
                }
                if rd.is_hovered {
                    painter.rect_filled(rd.name_rect, CornerRadius::same(3),
                        self.theme.accent.linear_multiply(0.15));
                }
                let name_col = if rd.is_filtered { self.theme.accent } else { self.theme.text_primary };
                painter.text(
                    egui::pos2(panel_rect.min.x + 8.0, rd.row_y + row_h * 0.5), Align2::LEFT_CENTER,
                    &rd.short_name, egui::FontId::proportional(11.0), name_col,
                );
                for (cell_rect, cell_color, count_str) in &rd.section_cells {
                    painter.rect_filled(*cell_rect, CornerRadius::same(4), cell_color.linear_multiply(0.3));
                    painter.text(cell_rect.center(), Align2::CENTER_CENTER,
                        count_str, egui::FontId::proportional(11.0), *cell_color);
                }
                painter.text(
                    egui::pos2(rd.total_x + col_w * 0.5, rd.row_y + row_h * 0.5), Align2::CENTER_CENTER,
                    &rd.total.to_string(), egui::FontId::proportional(11.0), self.theme.text_secondary,
                );
            }
        }

        // Close on Escape
        if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_workload_panel = false;
        }
    }

    /// Sticky kanban column headers: pinned to top of canvas, show section name + ticket count.
    /// Only shown in LR layout with multiple sections when zoomed in enough.
    /// Clicking a header activates a `section:Name` filter (click again to clear).
    fn draw_kanban_column_headers(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        if self.document.layout_dir != "LR" { return; }
        if matches!(self.view_mode, super::ViewMode::ThreeD) { return; }
        if self.viewport.zoom < 0.3 { return; }

        // Build ordered sections + x-span
        let mut section_order: Vec<String> = Vec::new();
        for n in &self.document.nodes {
            if !n.section_name.is_empty() && !section_order.contains(&n.section_name) {
                section_order.push(n.section_name.clone());
            }
        }
        if section_order.len() < 2 { return; }

        let mut section_x_min: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        let mut section_x_max: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        let mut section_count: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let mut section_done: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let mut section_overdue: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let today_hdr = super::render::today_iso();
        for n in &self.document.nodes {
            if n.section_name.is_empty() || n.is_frame { continue; }
            let sr = Rect::from_min_size(
                self.viewport.canvas_to_screen(n.pos()),
                n.size_vec() * self.viewport.zoom,
            );
            let entry_min = section_x_min.entry(n.section_name.clone()).or_insert(f32::MAX);
            *entry_min = entry_min.min(sr.min.x);
            let entry_max = section_x_max.entry(n.section_name.clone()).or_insert(f32::MIN);
            *entry_max = entry_max.max(sr.max.x);
            *section_count.entry(n.section_name.clone()).or_default() += 1;
            if matches!(n.tag, Some(crate::model::NodeTag::Ok)) || n.progress >= 1.0 {
                *section_done.entry(n.section_name.clone()).or_default() += 1;
            }
            let is_over = n.sublabel.split('\n').any(|line| {
                if let Some(ds) = line.strip_prefix("📅 ") {
                    let d = ds.trim();
                    d.len() >= 8 && d < today_hdr.as_str()
                } else { false }
            });
            if is_over { *section_overdue.entry(n.section_name.clone()).or_default() += 1; }
        }

        let header_h = 22.0_f32;
        let top_y = canvas_rect.min.y + 2.0;
        let pad = 12.0_f32 * self.viewport.zoom.sqrt().max(0.5);

        let column_fills: &[[u8; 4]] = &[
            [203, 166, 247, 25], [250, 179, 135, 25], [137, 180, 250, 25],
            [166, 227, 161, 25], [249, 226, 175, 25], [243, 139, 168, 25],
        ];

        // Determine active section filter
        let active_section = self.search_query.strip_prefix("section:")
            .map(|s| s.to_string());

        let mut clicked_section: Option<String> = None;
        for (i, sec) in section_order.iter().enumerate() {
            let x_min = match section_x_min.get(sec) { Some(&v) => v, None => continue };
            let x_max = match section_x_max.get(sec) { Some(&v) => v, None => continue };
            let cx = (x_min + x_max) * 0.5;
            if cx < canvas_rect.min.x || cx > canvas_rect.max.x { continue; }
            let count = section_count.get(sec).copied().unwrap_or(0);
            let col_w = (x_max - x_min + pad * 2.0).max(80.0);
            let header_rect = Rect::from_center_size(
                egui::pos2(cx, top_y + header_h * 0.5),
                egui::vec2(col_w.min(canvas_rect.width() - 20.0), header_h),
            );
            let is_active = active_section.as_deref() == Some(sec.as_str());
            let fill = column_fills[i % column_fills.len()];
            let bg_alpha = if is_active { 160u8 } else { 80u8 };
            {
                let painter = ui.painter();
                painter.rect_filled(header_rect, CornerRadius::same(5),
                    Color32::from_rgba_unmultiplied(fill[0], fill[1], fill[2], bg_alpha));
                let stroke_alpha = if is_active { 220u8 } else { 120u8 };
                painter.rect_stroke(header_rect, CornerRadius::same(5),
                    egui::Stroke::new(if is_active { 1.5 } else { 0.8 }, Color32::from_rgba_unmultiplied(fill[0], fill[1], fill[2], stroke_alpha)),
                    StrokeKind::Outside);
                let done = section_done.get(sec).copied().unwrap_or(0);
                let overdue = section_overdue.get(sec).copied().unwrap_or(0);
                let done_part = if done > 0 { format!("  ✓{}/{}", done, count) } else { format!("  {}", count) };
                let overdue_part = if overdue > 0 { format!("  ⚠{}", overdue) } else { String::new() };
                let dismiss = if is_active { "✕ " } else { "" };
                let label = format!("{}{}{}{}", dismiss, sec, done_part, overdue_part);
                let txt_alpha = 230u8;
                let txt_col = if overdue > 0 {
                    Color32::from_rgba_unmultiplied(243, 139, 168, txt_alpha)
                } else {
                    Color32::from_rgba_unmultiplied(fill[0].saturating_add(60), fill[1].saturating_add(60), fill[2].saturating_add(60), txt_alpha)
                };
                painter.text(header_rect.center(), Align2::CENTER_CENTER,
                    &label, egui::FontId::proportional(11.0), txt_col);
            }
            let resp = ui.allocate_rect(header_rect, egui::Sense::click());
            if resp.clicked() {
                clicked_section = Some(sec.clone());
            }
            // Tooltip hint
            if resp.hovered() {
                let painter = ui.painter();
                painter.rect_stroke(header_rect, CornerRadius::same(5),
                    egui::Stroke::new(1.5, Color32::from_rgba_unmultiplied(fill[0], fill[1], fill[2], 200)),
                    StrokeKind::Outside);
            }
        }

        // Apply click
        if let Some(sec) = clicked_section {
            let filter = format!("section:{}", sec);
            if self.search_query == filter && self.persist_search_filter {
                // Toggle off
                self.search_query.clear();
                self.persist_search_filter = false;
                self.status_message = Some(("Filter cleared".to_string(), std::time::Instant::now()));
            } else {
                self.search_query = filter;
                self.persist_search_filter = true;
                self.status_message = Some((format!("Showing: {}", sec), std::time::Instant::now()));
            }
        }
    }

    /// Kanban column bands: draw faint vertical column backgrounds when in LR layout
    /// with multiple sections. Makes the kanban board look and feel like a real board.
    fn draw_kanban_column_bands(&self, painter: &egui::Painter, canvas_rect: Rect) {
        // Only applicable in LR layout (left-to-right kanban) and 2D view
        if self.document.layout_dir != "LR" { return; }
        if matches!(self.view_mode, super::ViewMode::ThreeD) { return; }
        // Only when there are nodes with distinct sections
        let mut section_order: Vec<String> = Vec::new();
        for n in &self.document.nodes {
            if !n.section_name.is_empty() && !section_order.contains(&n.section_name) {
                section_order.push(n.section_name.clone());
            }
        }
        if section_order.len() < 2 { return; }

        // Compute x-span per section
        let mut section_x_min: std::collections::HashMap<&str, f32> = std::collections::HashMap::new();
        let mut section_x_max: std::collections::HashMap<&str, f32> = std::collections::HashMap::new();
        for n in &self.document.nodes {
            if n.section_name.is_empty() { continue; }
            let sr = Rect::from_min_size(
                self.viewport.canvas_to_screen(n.pos()),
                n.size_vec() * self.viewport.zoom,
            );
            let entry_min = section_x_min.entry(&n.section_name).or_insert(f32::MAX);
            *entry_min = entry_min.min(sr.min.x);
            let entry_max = section_x_max.entry(&n.section_name).or_insert(f32::MIN);
            *entry_max = entry_max.max(sr.max.x);
        }

        // Section column palette (soft, subtle fills) — cycling through 6 distinct hues
        let column_fills: &[[u8; 4]] = &[
            [203, 166, 247, 12], // mauve
            [250, 179, 135, 12], // peach
            [137, 180, 250, 12], // blue
            [166, 227, 161, 12], // green
            [249, 226, 175, 12], // yellow
            [243, 139, 168, 12], // pink
        ];

        let pad = 12.0_f32 * self.viewport.zoom.sqrt().max(0.5);
        for (i, sec) in section_order.iter().enumerate() {
            let x_min = match section_x_min.get(sec.as_str()) { Some(&v) => v, None => continue };
            let x_max = match section_x_max.get(sec.as_str()) { Some(&v) => v, None => continue };
            if x_min >= x_max { continue; }
            let col_rect = Rect::from_x_y_ranges(
                (x_min - pad)..=(x_max + pad),
                canvas_rect.min.y..=canvas_rect.max.y,
            );
            if !col_rect.intersects(canvas_rect) { continue; }
            let fill = column_fills[i % column_fills.len()];
            painter.rect_filled(col_rect, CornerRadius::ZERO,
                Color32::from_rgba_unmultiplied(fill[0], fill[1], fill[2], fill[3]));
            // Faint separator line on right edge (except last column)
            if i + 1 < section_order.len() {
                let mid_x = x_max + pad;
                painter.line_segment(
                    [egui::pos2(mid_x, canvas_rect.min.y), egui::pos2(mid_x, canvas_rect.max.y)],
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(fill[0], fill[1], fill[2], 30)),
                );
            }
        }
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
        painter.text(leg_pos, Align2::LEFT_TOP, "Connectivity:", FontId::proportional(9.0), self.theme.text_dim);
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
        painter.text(leg_pos + Vec2::new(0.0, 12.0), Align2::LEFT_TOP, "low", FontId::proportional(7.0), self.theme.text_dim);
        painter.text(leg_pos + Vec2::new(leg_w, 12.0), Align2::RIGHT_TOP, "high", FontId::proportional(7.0), self.theme.text_dim);
        painter.text(Pos2::new(canvas_rect.max.x - 12.0, canvas_rect.max.y - 30.0),
            Align2::RIGHT_BOTTOM, "[H] heatmap", FontId::proportional(8.0), self.theme.text_dim.gamma_multiply(0.6));
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
        let ruler_color = self.theme.surface0;
        let tick_color = self.theme.text_dim;
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
        let major_every = if zoom < 0.5 { 10i32 } else if zoom > 2.0 { 2i32 } else { 5i32 };
        let world_start_x = ((canvas_rect.min.x - self.viewport.offset[0]) / zoom / interval).floor() * interval;
        let mut wx = world_start_x;
        while self.viewport.canvas_to_screen(Pos2::new(wx, 0.0)).x < canvas_rect.max.x {
            let sx = self.viewport.canvas_to_screen(Pos2::new(wx, 0.0)).x;
            if sx > canvas_rect.min.x + ruler_h {
                let is_major = (wx / interval).round() as i32 % major_every == 0;
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
                let is_major = (wy / interval).round() as i32 % major_every == 0;
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
            egui::CornerRadius::ZERO, self.theme.surface1,
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

    // --- Rulers ---

    fn draw_side_rulers(&self, painter: &egui::Painter, canvas_rect: Rect, pointer_pos: Option<Pos2>) {
        let ruler_w = 18.0_f32; // ruler thickness in screen pixels
        let bg = Color32::from_rgba_premultiplied(24, 24, 37, 220);
        let tick_color = Color32::from_rgba_premultiplied(100, 104, 130, 200);
        let label_color = Color32::from_rgba_premultiplied(130, 135, 160, 240);
        let hairline_color = self.theme.accent.gamma_multiply(0.7);

        // Horizontal ruler (top strip)
        let h_ruler = Rect::from_min_max(
            Pos2::new(canvas_rect.min.x + ruler_w, canvas_rect.min.y),
            Pos2::new(canvas_rect.max.x, canvas_rect.min.y + ruler_w),
        );
        // Vertical ruler (left strip)
        let v_ruler = Rect::from_min_max(
            Pos2::new(canvas_rect.min.x, canvas_rect.min.y + ruler_w),
            Pos2::new(canvas_rect.min.x + ruler_w, canvas_rect.max.y),
        );
        // Corner square
        let corner = Rect::from_min_max(canvas_rect.min, canvas_rect.min + Vec2::splat(ruler_w));

        painter.rect_filled(h_ruler, CornerRadius::ZERO, bg);
        painter.rect_filled(v_ruler, CornerRadius::ZERO, bg);
        painter.rect_filled(corner, CornerRadius::ZERO, bg);

        // Determine a nice step: target ~60 screen-px per major tick
        let zoom = self.viewport.zoom;
        let raw_step = 60.0 / zoom; // canvas units per desired tick
        let magnitude = (10.0_f32).powf(raw_step.log10().floor());
        let step = if raw_step / magnitude < 2.5 { magnitude }
                   else if raw_step / magnitude < 6.0 { magnitude * 2.5 }
                   else { magnitude * 5.0 };
        let step = step.max(1.0);

        let font = egui::FontId::proportional(8.5);

        // ── Horizontal ruler ticks ────────────────────────────────────────
        {
            let canvas_left  = self.viewport.screen_to_canvas(Pos2::new(h_ruler.min.x, 0.0)).x;
            let canvas_right = self.viewport.screen_to_canvas(Pos2::new(h_ruler.max.x, 0.0)).x;
            let first = (canvas_left / step).floor() as i64;
            let last  = (canvas_right / step).ceil() as i64;
            for i in first..=last {
                let c = i as f32 * step;
                let sx = self.viewport.canvas_to_screen(Pos2::new(c, 0.0)).x;
                if sx < h_ruler.min.x || sx > h_ruler.max.x { continue; }
                // major tick
                painter.line_segment(
                    [Pos2::new(sx, h_ruler.max.y - 6.0), Pos2::new(sx, h_ruler.max.y)],
                    Stroke::new(0.8, tick_color),
                );
                // label (omit if step very fine)
                if step * zoom > 20.0 {
                    let label = format_coord(c);
                    painter.text(
                        Pos2::new(sx + 2.0, h_ruler.min.y + 2.0),
                        Align2::LEFT_TOP,
                        &label,
                        font.clone(),
                        label_color,
                    );
                }
                // minor ticks (÷ 5)
                for sub in 1..5i64 {
                    let sc = c + sub as f32 * step / 5.0;
                    let ssx = self.viewport.canvas_to_screen(Pos2::new(sc, 0.0)).x;
                    if ssx < h_ruler.min.x || ssx > h_ruler.max.x { continue; }
                    painter.line_segment(
                        [Pos2::new(ssx, h_ruler.max.y - 3.0), Pos2::new(ssx, h_ruler.max.y)],
                        Stroke::new(0.5, tick_color.gamma_multiply(0.5)),
                    );
                }
            }
        }

        // ── Vertical ruler ticks ─────────────────────────────────────────
        {
            let canvas_top    = self.viewport.screen_to_canvas(Pos2::new(0.0, v_ruler.min.y)).y;
            let canvas_bottom = self.viewport.screen_to_canvas(Pos2::new(0.0, v_ruler.max.y)).y;
            let first = (canvas_top / step).floor() as i64;
            let last  = (canvas_bottom / step).ceil() as i64;
            for i in first..=last {
                let c = i as f32 * step;
                let sy = self.viewport.canvas_to_screen(Pos2::new(0.0, c)).y;
                if sy < v_ruler.min.y || sy > v_ruler.max.y { continue; }
                painter.line_segment(
                    [Pos2::new(v_ruler.max.x - 6.0, sy), Pos2::new(v_ruler.max.x, sy)],
                    Stroke::new(0.8, tick_color),
                );
                if step * zoom > 20.0 {
                    let label = format_coord(c);
                    // Rotated label via clipped transform: just draw upright for simplicity
                    painter.text(
                        Pos2::new(v_ruler.min.x + 1.0, sy - 1.0),
                        Align2::LEFT_BOTTOM,
                        &label,
                        font.clone(),
                        label_color,
                    );
                }
                for sub in 1..5i64 {
                    let sc = c + sub as f32 * step / 5.0;
                    let ssy = self.viewport.canvas_to_screen(Pos2::new(0.0, sc)).y;
                    if ssy < v_ruler.min.y || ssy > v_ruler.max.y { continue; }
                    painter.line_segment(
                        [Pos2::new(v_ruler.max.x - 3.0, ssy), Pos2::new(v_ruler.max.x, ssy)],
                        Stroke::new(0.5, tick_color.gamma_multiply(0.5)),
                    );
                }
            }
        }

        // ── Hairline cursor indicator ─────────────────────────────────────
        if let Some(mouse) = pointer_pos {
            if mouse.x >= h_ruler.min.x {
                painter.line_segment(
                    [Pos2::new(mouse.x, h_ruler.min.y), Pos2::new(mouse.x, h_ruler.max.y)],
                    Stroke::new(1.0, hairline_color),
                );
            }
            if mouse.y >= v_ruler.min.y {
                painter.line_segment(
                    [Pos2::new(v_ruler.min.x, mouse.y), Pos2::new(v_ruler.max.x, mouse.y)],
                    Stroke::new(1.0, hairline_color),
                );
            }
        }

        // Ruler border lines
        painter.line_segment(
            [Pos2::new(h_ruler.min.x, h_ruler.max.y), Pos2::new(h_ruler.max.x, h_ruler.max.y)],
            Stroke::new(0.5, self.theme.surface1),
        );
        painter.line_segment(
            [Pos2::new(v_ruler.max.x, v_ruler.min.y), Pos2::new(v_ruler.max.x, v_ruler.max.y)],
            Stroke::new(0.5, self.theme.surface1),
        );
    }

    // --- Grid ---

    fn draw_grid(&self, painter: &egui::Painter, canvas_rect: Rect) {
        let zoom = self.viewport.zoom;
        // Multi-level fallback: if the grid is too dense, step up to 5x or 25x the grid interval
        // so that some reference lines are always visible at any zoom level.
        let raw_grid_screen = self.grid_size * zoom;
        let (grid_screen, effective_major_every) = if raw_grid_screen >= 8.0 {
            (raw_grid_screen, 5_i32)
        } else if raw_grid_screen * 5.0 >= 8.0 {
            (raw_grid_screen * 5.0, 5_i32)    // minor cells are 5× grid_size
        } else if raw_grid_screen * 25.0 >= 8.0 {
            (raw_grid_screen * 25.0, 5_i32)   // minor cells are 25× grid_size
        } else {
            return; // truly too small to show anything useful
        };

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

        // Major grid every N minor cells (adaptive based on zoom level)
        let major_every = effective_major_every;
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
                        // Scale dot radius with zoom for crisp appearance at any zoom level
                        let zoom_scale = zoom.sqrt().clamp(0.5, 2.0);
                        let (r, c) = if is_major { (1.8 * zoom_scale, self.theme.grid_major_color) } else { (0.8 * zoom_scale, self.theme.grid_color) };
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
                        egui::Stroke::new(0.7, self.theme.grid_major_color)
                    } else {
                        egui::Stroke::new(0.4, self.theme.grid_color)
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
                        egui::Stroke::new(0.7, self.theme.grid_major_color)
                    } else {
                        egui::Stroke::new(0.4, self.theme.grid_color)
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
                        egui::Stroke::new(0.7, self.theme.grid_major_color)
                    } else {
                        egui::Stroke::new(0.4, self.theme.grid_color)
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
            let color = if is_active { self.theme.accent } else { self.theme.text_dim };
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
                if resp.hovered() { self.theme.text_secondary } else { color },
            );
            x += 40.0;
        }
    }

    fn draw_search_overlay(&mut self, ui: &mut egui::Ui, canvas_rect: Rect, search_matches: &std::collections::HashSet<NodeId>) {
        if !self.show_search { return; }

        // Collect matching results — use the same smart-filter set as canvas highlights
        let q = self.search_query.to_lowercase();
        let max_results = 8_usize;
        let total_count = search_matches.len();
        let results: Vec<(NodeId, String)> = if q.is_empty() {
            Vec::new()
        } else {
            self.document.nodes.iter()
                .filter(|n| search_matches.contains(&n.id))
                .take(max_results)
                .map(|n| (n.id, n.display_label().to_string()))
                .collect()
        };

        // Show a "create" row when query is non-empty and is not a smart-filter prefix
        let is_filter_query = !q.is_empty() && (
            q.starts_with("status:") || q.starts_with("priority:") || q.starts_with("p:") ||
            q.starts_with("section:") || q.starts_with("§") || q.starts_with("assigned:") ||
            q.starts_with("owner:") || q.starts_with("url:") || q.starts_with("due:") ||
            q.starts_with("icon:") || q.starts_with("assigned:") ||
            matches!(q.as_str(), "overdue" | "upcoming" | "has-url" | "no-url" | "has-due"
                | "has-comment" | "glow" | "sla-breach" | "past-due" | "escalated" | "isolated")
        );
        let show_create_row = !q.is_empty() && results.is_empty() && !is_filter_query;

        // Clamp cursor
        if !results.is_empty() && self.search_cursor >= results.len() {
            self.search_cursor = results.len() - 1;
        }

        let w = 320.0_f32;
        let input_h = 38.0_f32;
        let row_h = 30.0_f32;
        let create_row_h = if show_create_row { 32.0 } else { 0.0 };
        let results_h = results.len() as f32 * row_h + create_row_h;
        let total_h = input_h + results_h;

        let top = canvas_rect.min.y + 50.0;
        let overlay_rect = Rect::from_min_size(
            Pos2::new(canvas_rect.center().x - w / 2.0, top),
            Vec2::new(w, total_h.max(input_h)),
        );

        // Background panel
        {
            let painter = ui.painter().clone();
            painter.rect_filled(overlay_rect, CornerRadius::same(10), self.theme.tooltip_bg);
            painter.rect_stroke(overlay_rect, CornerRadius::same(10),
                Stroke::new(1.0, self.theme.accent.gamma_multiply(0.4)), StrokeKind::Outside);
            // Search icon
            painter.text(
                Pos2::new(overlay_rect.min.x + 12.0, overlay_rect.min.y + input_h / 2.0),
                Align2::LEFT_CENTER, "🔍",
                FontId::proportional(13.0), self.theme.text_dim,
            );
            // Divider between input and results
            if !results.is_empty() {
                painter.line_segment(
                    [Pos2::new(overlay_rect.min.x + 12.0, top + input_h),
                     Pos2::new(overlay_rect.max.x - 12.0, top + input_h)],
                    Stroke::new(0.5, self.theme.surface1),
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
                .hint_text("Search… status:done · priority:p1 · assigned:Alice · url:jira · overdue")
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

        // Close on Escape (also clears any pinned filter)
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_search = false;
            self.search_query.clear();
            self.persist_search_filter = false;
            self.search_cursor = 0;
            return;
        }

        // Jump to result on Enter; Shift+Enter = select ALL + pin filter
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            let shift = ctx.input(|i| i.modifiers.shift);
            if shift || total_count == 1 {
                // Select ALL matching nodes (not just the visible 8) and zoom to fit
                self.selection.clear();
                for nid in search_matches { self.selection.node_ids.insert(*nid); }
                if !search_matches.is_empty() { self.zoom_to_selection(); }
                if total_count > 1 {
                    self.status_message = Some((format!("Selected {} nodes — filter pinned (⌘F to clear)", total_count), std::time::Instant::now()));
                }
                // Pin filter so dimming stays active after closing search
                if shift && total_count > 0 {
                    self.persist_search_filter = true;
                    // Keep query for persistent filtering; close overlay only
                    self.show_search = false;
                    self.search_cursor = 0;
                    return;
                }
            } else if let Some(&(nid, _)) = results.get(self.search_cursor).or_else(|| results.first()) {
                self.selection.select_node(nid);
                self.zoom_to_selection();
            }
            self.show_search = false;
            self.search_query.clear();
            self.persist_search_filter = false;
            self.search_cursor = 0;
            return;
        }

        // Result count badge — "N of M" when results are capped, "0" when no match
        {
            let badge = if q.is_empty() { String::new() }
                        else if total_count == 0 { "0".to_string() }
                        else if total_count > max_results { format!("{} of {}", max_results, total_count) }
                        else { format!("{}", total_count) };
            if !badge.is_empty() {
                let badge_color = if total_count == 0 { self.theme.text_dim }
                                  else if total_count > max_results { self.theme.accent }
                                  else { self.theme.text_dim };
                ui2.painter().text(
                    Pos2::new(overlay_rect.max.x - 8.0, overlay_rect.min.y + input_h / 2.0),
                    Align2::RIGHT_CENTER, &badge,
                    FontId::proportional(11.0), badge_color,
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

            let bg = if is_highlighted { self.theme.accent.gamma_multiply(0.18) }
                     else if is_hov    { Color32::from_rgba_unmultiplied(137, 180, 250, 12) }
                     else              { Color32::TRANSPARENT };
            ui.painter().rect_filled(row_rect, CornerRadius::ZERO, bg);

            // Look up node for context (tag + section)
            let (node_tag, node_section) = self.document.find_node(nid)
                .map(|n| (n.tag, n.section_name.clone()))
                .unwrap_or((None, String::new()));

            // Tag dot on the right
            let tag_color = match node_tag {
                Some(crate::model::NodeTag::Critical) => Color32::from_rgb(243, 139, 168),
                Some(crate::model::NodeTag::Warning)  => Color32::from_rgb(250, 179, 135),
                Some(crate::model::NodeTag::Ok)        => Color32::from_rgb(166, 227, 161),
                Some(crate::model::NodeTag::Info)      => Color32::from_rgb(137, 180, 250),
                None => Color32::TRANSPARENT,
            };
            if node_tag.is_some() {
                ui.painter().circle_filled(
                    Pos2::new(overlay_rect.max.x - 10.0, row_rect.center().y),
                    4.0, tag_color,
                );
            }

            // Section label dim (right-aligned, before the dot)
            let right_x = if node_tag.is_some() { overlay_rect.max.x - 20.0 } else { overlay_rect.max.x - 8.0 };
            if !node_section.is_empty() {
                let sec_short: String = node_section.chars().take(12).collect();
                ui.painter().text(
                    Pos2::new(right_x, row_rect.center().y),
                    Align2::RIGHT_CENTER, &sec_short,
                    FontId::proportional(9.5), self.theme.text_dim.gamma_multiply(0.6),
                );
            }

            // Truncate label (leave room for right-side context)
            let max_label_chars = if node_section.is_empty() && node_tag.is_none() { 36 } else { 26 };
            let short: String = label.chars().take(max_label_chars).collect();
            let trail = if label.chars().count() > max_label_chars { "…" } else { "" };
            let disp = format!("{}{}", short, trail);
            ui.painter().text(
                Pos2::new(row_rect.min.x + 14.0, row_rect.center().y),
                Align2::LEFT_CENTER, &disp,
                FontId::proportional(12.5),
                if is_highlighted { self.theme.text_primary } else { self.theme.text_secondary },
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

        // "Create" row — shown when no results and query is a plain string
        if show_create_row {
            let create_y = top + input_h; // immediately below input
            let create_rect = Rect::from_min_size(
                Pos2::new(overlay_rect.min.x, create_y),
                Vec2::new(w, create_row_h),
            );
            let create_resp = ui.allocate_rect(create_rect, egui::Sense::click());
            let hov = create_resp.hovered();
            let bg = if hov { Color32::from_rgba_unmultiplied(30, 160, 60, 30) }
                     else   { Color32::from_rgba_unmultiplied(20, 100, 40, 18) };
            ui.painter().rect_filled(create_rect, CornerRadius::same(0), bg);
            // Divider at top
            ui.painter().line_segment(
                [Pos2::new(overlay_rect.min.x + 12.0, create_y),
                 Pos2::new(overlay_rect.max.x - 12.0, create_y)],
                Stroke::new(0.5, self.theme.surface1),
            );
            let raw_query = &self.search_query;
            let short_q: String = raw_query.chars().take(30).collect();
            let trail = if raw_query.chars().count() > 30 { "…" } else { "" };
            let create_label = format!("➕ Create \"{}{}\"", short_q, trail);
            ui.painter().text(
                Pos2::new(create_rect.min.x + 14.0, create_rect.center().y),
                Align2::LEFT_CENTER, &create_label,
                FontId::proportional(12.0),
                if hov { Color32::from_rgb(120, 210, 130) } else { Color32::from_rgba_unmultiplied(100, 180, 110, 180) },
            );
            if hov {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            if create_resp.clicked() {
                // Create a new node at canvas center with the search text as label
                use crate::model::{Node, NodeKind, NodeShape};
                let canvas_center = self.viewport.screen_to_canvas(canvas_rect.center());
                let raw_q = self.search_query.clone();
                let canvas_pos = egui::Pos2::new(canvas_center[0], canvas_center[1]);
                let mut new_node = Node::new(NodeShape::Rectangle, canvas_pos);
                match &mut new_node.kind {
                    NodeKind::Shape { label, .. } => { *label = raw_q.clone(); }
                    _ => {}
                }
                let new_id = new_node.id;
                self.document.nodes.push(new_node);
                self.selection.clear();
                self.selection.select_node(new_id);
                self.zoom_to_selection();
                self.history.push(&self.document);
                self.show_search = false;
                self.search_query.clear();
                self.search_cursor = 0;
                self.status_message = Some((
                    format!("Created \"{}\"", raw_q.chars().take(24).collect::<String>()),
                    std::time::Instant::now(),
                ));
            }
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

        painter.rect_filled(minimap_rect, CornerRadius::same(6), self.theme.minimap_bg);
        painter.rect_stroke(
            minimap_rect,
            CornerRadius::same(6),
            Stroke::new(1.0, self.theme.minimap_border),
            StrokeKind::Outside,
        );

        // Minimap label
        painter.text(
            Pos2::new(minimap_rect.min.x + 8.0, minimap_rect.min.y + 10.0),
            Align2::LEFT_CENTER,
            "MINIMAP",
            FontId::proportional(8.0),
            self.theme.minimap_border,
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

        // Draw section backgrounds in minimap
        {
            use std::collections::HashMap;
            let mut section_bounds: HashMap<&str, egui::Rect> = HashMap::new();
            for node in &self.document.nodes {
                if node.section_name.is_empty() { continue; }
                let r = node.rect();
                let entry = section_bounds.entry(node.section_name.as_str()).or_insert(r);
                *entry = entry.union(r);
            }
            for (sec_name, bounds) in &section_bounds {
                let min_pt = map_point(bounds.min.x, bounds.min.y);
                let max_pt = map_point(bounds.max.x, bounds.max.y);
                let mini_rect = egui::Rect::from_two_pos(min_pt, max_pt);
                let bg = Self::section_bg_color_pub(sec_name);
                let fill = egui::Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), 60);
                painter.rect_filled(mini_rect.expand(2.0), egui::CornerRadius::same(2), fill);
            }
        }

        // Draw edges first (behind nodes)
        let edge_color_mm = self.theme.text_dim.gamma_multiply(0.5);
        let edge_sel_col  = self.theme.accent.gamma_multiply(0.8);
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
            // Use actual node fill color in minimap for visual accuracy
            let node_color = if node.is_frame {
                self.theme.surface1.gamma_multiply(0.3)
            } else if is_selected {
                self.theme.accent
            } else {
                let [r, g, b, a] = node.style.fill_color;
                if a < 30 {
                    // Transparent/nearly-transparent nodes: use theme minimap color
                    self.theme.minimap_node
                } else {
                    Color32::from_rgba_unmultiplied(r, g, b, 200)
                }
            };
            let cr_val = (mini_rect.width().min(mini_rect.height()) * 0.2) as u8;
            if mini_rect.area() > 2.0 {
                painter.rect_filled(mini_rect, egui::CornerRadius::same(cr_val), node_color);
                // Show abbreviated label if rect is large enough
                if mini_rect.width() > 18.0 && mini_rect.height() > 7.0 {
                    let label = node.display_label();
                    let short: String = label.chars().take(8).collect();
                    let font_size = (mini_rect.height() * 0.55).clamp(5.0, 8.0);
                    let text_color = self.theme.text_primary.gamma_multiply(0.65);
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
            painter.rect_filled(clipped, CornerRadius::ZERO, self.theme.minimap_vp_fill);
            painter.rect_stroke(
                clipped,
                CornerRadius::ZERO,
                Stroke::new(1.0, self.theme.minimap_vp_stroke),
                StrokeKind::Outside,
            );
            // Zoom % label inside the viewport rect if large enough
            if clipped.width() > 22.0 && clipped.height() > 9.0 {
                let zoom_pct = (self.viewport.zoom * 100.0).round() as i32;
                painter.text(
                    clipped.center(),
                    Align2::CENTER_CENTER,
                    &format!("{}%", zoom_pct),
                    FontId::proportional(6.5),
                    self.theme.accent.gamma_multiply(0.7),
                );
            }
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

    /// Render the inline section rename editor overlay.
    fn draw_section_rename_editor(&mut self, ui: &mut egui::Ui) {
        if self.section_rename.is_none() { return; }

        let (old_name, label_pos) = {
            let (ref old, _, ref pos) = *self.section_rename.as_ref().unwrap();
            (old.clone(), *pos)
        };

        // Background pill behind the text input
        let edit_w = 180.0_f32;
        let edit_h = 22.0_f32;
        let bg_rect = egui::Rect::from_min_size(
            egui::Pos2::new(label_pos.x - 4.0, label_pos.y - 2.0),
            egui::Vec2::new(edit_w, edit_h),
        );
        ui.painter().rect_filled(bg_rect, egui::CornerRadius::same(4), self.theme.surface1);
        ui.painter().rect_stroke(bg_rect, egui::CornerRadius::same(4),
            egui::Stroke::new(1.5, self.theme.accent.gamma_multiply(0.7)), egui::StrokeKind::Outside);

        // Floating text edit area
        let area = egui::Area::new(egui::Id::new("section_rename_area"))
            .fixed_pos(egui::Pos2::new(label_pos.x, label_pos.y - 1.0))
            .order(egui::Order::Foreground);
        let mut committed = false;
        let mut cancelled = false;
        area.show(ui.ctx(), |ui| {
            ui.set_max_width(edit_w);
            let edit_text = &mut self.section_rename.as_mut().unwrap().1;
            let re = ui.add(
                egui::TextEdit::singleline(edit_text)
                    .desired_width(edit_w - 8.0)
                    .frame(false)
                    .font(egui::TextStyle::Body)
            );
            re.request_focus();
            if re.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Tab)) {
                committed = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
        });

        if cancelled {
            self.section_rename = None;
            return;
        }
        if committed {
            let new_name = self.section_rename.as_ref().unwrap().1.trim().to_string();
            if !new_name.is_empty() && new_name != old_name {
                for node in &mut self.document.nodes {
                    if node.section_name == old_name {
                        node.section_name = new_name.clone();
                    }
                }
                self.history.push(&self.document);
                self.status_message = Some((
                    format!("Section renamed to \"{new_name}\""),
                    std::time::Instant::now(),
                ));
            }
            self.section_rename = None;
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
            self.theme.tooltip_bg,
        );
        painter.rect_stroke(
            screen_rect,
            CornerRadius::same(4),
            Stroke::new(2.0, self.theme.accent),
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

    /// Quick-assign popup: shown when `quick_assign_buf` is Some.
    /// A text field appears above the selection centroid; Enter applies, Escape cancels.
    fn draw_quick_assign_popup(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        if self.quick_assign_buf.is_none() { return; }

        // Compute selection bounding box in screen space
        let sel_ids: Vec<_> = self.selection.node_ids.iter().copied().collect();
        if sel_ids.is_empty() { self.quick_assign_buf = None; return; }

        let mut top = f32::MAX;
        let mut left = f32::MAX;
        let mut right = f32::MIN;
        for id in &sel_ids {
            if let Some(n) = self.document.find_node(id) {
                let sr = Rect::from_min_size(
                    self.viewport.canvas_to_screen(n.pos()),
                    n.size_vec() * self.viewport.zoom,
                );
                top   = top.min(sr.min.y);
                left  = left.min(sr.min.x);
                right = right.max(sr.max.x);
            }
        }
        if top == f32::MAX { self.quick_assign_buf = None; return; }

        // Collect known assignees from the whole document for autocomplete
        let mut known_assignees: Vec<String> = Vec::new();
        for n in &self.document.nodes {
            for line in n.sublabel.lines() {
                if let Some(name) = line.strip_prefix("👤 ") {
                    let name = name.trim().to_string();
                    if !name.is_empty() && !known_assignees.contains(&name) {
                        known_assignees.push(name);
                    }
                }
            }
        }
        known_assignees.sort();

        let popup_w = (right - left).clamp(160.0, 280.0);
        let popup_h = 36.0;
        let cx = (left + right) * 0.5;
        let popup_rect = Rect::from_min_size(
            egui::pos2((cx - popup_w * 0.5).clamp(canvas_rect.min.x + 8.0, canvas_rect.max.x - popup_w - 8.0),
                       (top - popup_h - 10.0).max(canvas_rect.min.y + 8.0)),
            egui::vec2(popup_w, popup_h),
        );

        // Draw popup background
        {
            let painter = ui.painter();
            painter.rect_filled(popup_rect.expand(2.0), CornerRadius::same(8), self.theme.tooltip_bg);
            painter.rect_stroke(popup_rect.expand(2.0), CornerRadius::same(8),
                Stroke::new(1.5, self.theme.accent), StrokeKind::Outside);
        }

        // Autocomplete suggestions above the text box
        let q = self.quick_assign_buf.as_deref().unwrap_or("").to_lowercase();
        let suggestions: Vec<String> = known_assignees.iter()
            .filter(|a| q.is_empty() || a.to_lowercase().starts_with(&q))
            .take(4)
            .map(|s| s.clone())
            .collect();
        let sug_h = 22.0;
        let mut clicked_suggestion: Option<String> = None;
        if !suggestions.is_empty() {
            let sug_total_h = suggestions.len() as f32 * sug_h;
            let sug_rect = Rect::from_min_size(
                egui::pos2(popup_rect.min.x, popup_rect.min.y - sug_total_h - 4.0),
                egui::vec2(popup_w, sug_total_h),
            );
            {
                let painter = ui.painter();
                painter.rect_filled(sug_rect.expand(2.0), CornerRadius::same(6), self.theme.tooltip_bg);
                painter.rect_stroke(sug_rect.expand(2.0), CornerRadius::same(6),
                    Stroke::new(1.0, self.theme.surface1), StrokeKind::Outside);
            }
            for (i, sug) in suggestions.iter().enumerate() {
                let row_rect = Rect::from_min_size(
                    egui::pos2(sug_rect.min.x + 2.0, sug_rect.min.y + i as f32 * sug_h),
                    egui::vec2(popup_w - 4.0, sug_h),
                );
                let resp = ui.allocate_rect(row_rect, egui::Sense::click());
                let hovered = resp.hovered();
                let clicked = resp.clicked();
                let painter = ui.painter();
                if hovered {
                    painter.rect_filled(row_rect, CornerRadius::same(4), self.theme.accent.linear_multiply(0.25));
                }
                painter.text(row_rect.left_center() + egui::vec2(6.0, 0.0), Align2::LEFT_CENTER,
                    format!("👤 {}", sug), egui::FontId::proportional(12.0), self.theme.text_primary);
                if clicked {
                    clicked_suggestion = Some(sug.clone());
                }
            }
            if let Some(s) = clicked_suggestion {
                self.quick_assign_buf = Some(s);
            }
        }

        egui::Area::new(egui::Id::new("quick_assign_area"))
            .fixed_pos(popup_rect.min)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                ui.set_width(popup_w);
                ui.set_height(popup_h);
                let text = match &mut self.quick_assign_buf {
                    Some(t) => t,
                    None => return,
                };
                let resp = ui.add_sized(
                    egui::vec2(popup_w, popup_h),
                    egui::TextEdit::singleline(text)
                        .hint_text("Assign to…")
                        .font(egui::FontId::proportional(13.0))
                        .desired_width(popup_w),
                );
                if !resp.has_focus() { resp.request_focus(); }

                let commit = ui.ctx().input(|i| i.key_pressed(egui::Key::Enter));
                let cancel = ui.ctx().input(|i| i.key_pressed(egui::Key::Escape));
                let clicked_away = ui.ctx().input(|i| i.pointer.primary_clicked())
                    && !popup_rect.expand(50.0).contains(ui.ctx().input(|i| i.pointer.interact_pos().unwrap_or(egui::Pos2::ZERO)));

                if commit || clicked_away {
                    let assignee = text.trim().to_string();
                    let sel_ids2: Vec<_> = self.selection.node_ids.iter().copied().collect();
                    for id in &sel_ids2 {
                        if let Some(n) = self.document.find_node_mut(id) {
                            // Compose sublabel: replace/insert "👤 " line
                            let other_lines: Vec<String> = n.sublabel.lines()
                                .filter(|l| !l.starts_with("👤 "))
                                .map(|l| l.to_string())
                                .collect();
                            let mut parts: Vec<String> = Vec::new();
                            if !assignee.is_empty() { parts.push(format!("👤 {}", assignee)); }
                            parts.extend(other_lines);
                            parts.retain(|l| !l.is_empty());
                            n.sublabel = parts.join("\n");
                        }
                    }
                    let count = sel_ids2.len();
                    if !assignee.is_empty() {
                        self.history.push(&self.document);
                        self.status_message = Some((
                            if count == 1 { format!("Assigned: {}", assignee) }
                            else { format!("Assigned: {} ({} nodes)", assignee, count) },
                            std::time::Instant::now(),
                        ));
                    }
                    self.quick_assign_buf = None;
                } else if cancel {
                    self.quick_assign_buf = None;
                }
            });
    }

    /// Quick-comment popup: shown when `quick_comment_buf` is Some (C key with selection).
    fn draw_quick_comment_popup(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        if self.quick_comment_buf.is_none() { return; }
        let sel_ids: Vec<NodeId> = self.selection.node_ids.iter().copied().collect();
        if sel_ids.is_empty() { self.quick_comment_buf = None; return; }

        // Position above the first selected node (topmost)
        let mut top = f32::MAX;
        let mut cx = canvas_rect.center().x;
        for id in &sel_ids {
            if let Some(n) = self.document.find_node(id) {
                let sr = self.viewport.canvas_to_screen(n.pos());
                if sr.y < top { top = sr.y; cx = sr.x + n.size[0] * self.viewport.zoom * 0.5; }
            }
        }
        if top == f32::MAX { self.quick_comment_buf = None; return; }

        let popup_w = 280.0_f32;
        let popup_h = 86.0_f32;
        let popup_x = (cx - popup_w / 2.0).clamp(canvas_rect.min.x + 4.0, canvas_rect.max.x - popup_w - 4.0);
        let popup_y = (top - popup_h - 8.0).max(canvas_rect.min.y + 4.0);
        let popup_rect = Rect::from_min_size(egui::pos2(popup_x, popup_y), egui::vec2(popup_w, popup_h));

        {
            let painter = ui.painter();
            painter.rect_filled(popup_rect, CornerRadius::same(8), Color32::from_rgba_unmultiplied(20, 20, 32, 240));
            painter.rect_stroke(popup_rect, CornerRadius::same(8),
                egui::Stroke::new(1.0, self.theme.surface1), StrokeKind::Outside);
            painter.text(egui::pos2(popup_rect.min.x + 10.0, popup_rect.min.y + 8.0),
                Align2::LEFT_TOP, "💬 Comment  (Enter = save · Esc = cancel)",
                egui::FontId::proportional(10.0), self.theme.text_dim);
        }

        let text_rect = Rect::from_min_size(
            egui::pos2(popup_rect.min.x + 8.0, popup_rect.min.y + 24.0),
            egui::vec2(popup_w - 16.0, popup_h - 32.0),
        );

        let mut submit = false;
        let mut cancel = false;
        let sel_ids2 = sel_ids.clone();

        ui.allocate_ui_at_rect(text_rect, |ui| {
            let resp = ui.add(
                egui::TextEdit::multiline(self.quick_comment_buf.as_mut().unwrap())
                    .font(egui::FontId::proportional(11.0))
                    .desired_width(text_rect.width())
                    .desired_rows(3)
                    .frame(false)
                    .text_color(self.theme.text_primary)
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command) {
                submit = true;
            }
            if !resp.has_focus() && !resp.gained_focus() {
                resp.request_focus();
            }
        });

        if submit {
            let comment = self.quick_comment_buf.clone().unwrap_or_default();
            for id in &sel_ids2 {
                if let Some(n) = self.document.find_node_mut(id) {
                    n.comment = comment.clone();
                }
            }
            let count = sel_ids2.len();
            if !comment.is_empty() {
                self.history.push(&self.document);
                self.status_message = Some((
                    if count == 1 { "💬 Comment saved".to_string() }
                    else { format!("💬 Comment saved ({} nodes)", count) },
                    std::time::Instant::now(),
                ));
            }
            self.quick_comment_buf = None;
        } else if cancel {
            self.quick_comment_buf = None;
        }
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
                self.theme.accent.gamma_multiply(0.78)
            } else {
                self.theme.surface0.gamma_multiply(0.71)
            };
            let painter = ui.painter();
            painter.circle_filled(*center, btn_size / 2.0, bg);
            painter.circle_stroke(*center, btn_size / 2.0,
                Stroke::new(1.5, self.theme.accent.gamma_multiply(0.63)));
            painter.text(*center, Align2::CENTER_CENTER, *label,
                FontId::proportional(11.0),
                if hovered { self.theme.crust } else { self.theme.accent });

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
                painter.rect_filled(bar_rect, CornerRadius::same(14), self.theme.surface1);
                painter.rect_stroke(bar_rect, CornerRadius::same(14),
                    Stroke::new(1.0, self.theme.minimap_border), StrokeKind::Outside);
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
                        self.theme.accent.gamma_multiply(0.30)
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
                        if *is_active { self.theme.accent } else { self.theme.text_secondary },
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

    /// "Go to XY" overlay — Shift+G pops up a small input: type "x, y" to pan there.
    pub(crate) fn draw_goto_overlay(&mut self, ctx: &egui::Context) {
        if !self.show_goto { return; }
        // Close on Escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_goto = false;
            self.goto_query.clear();
            return;
        }
        let w = 220.0_f32;
        let h = 44.0_f32;
        let screen_center = ctx.screen_rect().center();
        egui::Area::new(egui::Id::new("goto_overlay"))
            .fixed_pos(egui::pos2(screen_center.x - w / 2.0, screen_center.y - h / 2.0 - 60.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(self.theme.tooltip_bg)
                    .corner_radius(egui::CornerRadius::same(8))
                    .stroke(egui::Stroke::new(1.0, self.theme.accent.gamma_multiply(0.5)))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(w);
                        ui.label(egui::RichText::new("Go to position (x, y)").size(11.0).color(self.theme.text_dim));
                        ui.add_space(4.0);
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.goto_query)
                                .desired_width(w - 24.0)
                                .hint_text("e.g. 200, -150")
                                .font(egui::FontId::proportional(13.0)),
                        );
                        resp.request_focus();
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            // Parse "x, y" or "x y"
                            let q = self.goto_query.replace(',', " ");
                            let nums: Vec<f32> = q.split_whitespace()
                                .filter_map(|s| s.parse::<f32>().ok())
                                .collect();
                            if nums.len() >= 2 {
                                let (cx, cy) = (nums[0], nums[1]);
                                let c = self.canvas_rect.center();
                                self.pan_target = Some([
                                    c.x - cx * self.viewport.zoom,
                                    c.y - cy * self.viewport.zoom,
                                ]);
                                self.status_message = Some((
                                    format!("Jumped to ({:.0}, {:.0})", cx, cy),
                                    std::time::Instant::now(),
                                ));
                            }
                            self.show_goto = false;
                            self.goto_query.clear();
                        }
                    });
            });
    }

    // -----------------------------------------------------------------------
    // Extracted sub-methods (previously inline in draw_canvas)
    // -----------------------------------------------------------------------

    /// Tag filter pills — show at top-right when any tagged nodes exist.
    /// Shows a dismissible "Filter: <query>" chip at bottom-left when persist_search_filter is active.
    fn draw_persistent_filter_chip(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        if !self.persist_search_filter || self.search_query.is_empty() { return; }

        let q = &self.search_query;
        let short: String = q.chars().take(24).collect();
        let trail = if q.chars().count() > 24 { "…" } else { "" };
        let label = format!("🔍 Filter: {}{}  ✕", short, trail);

        let chip_h = 22.0_f32;
        let approx_w = label.chars().count() as f32 * 7.0 + 12.0;
        let chip_rect = Rect::from_min_size(
            Pos2::new(canvas_rect.min.x + 12.0, canvas_rect.max.y - 60.0 - chip_h),
            Vec2::new(approx_w, chip_h),
        );
        let painter = ui.painter();
        let bg = Color32::from_rgba_unmultiplied(30, 80, 160, 200);
        let border = self.theme.accent.gamma_multiply(0.7);
        painter.rect(chip_rect, CornerRadius::same(11), bg, Stroke::new(1.0, border), StrokeKind::Inside);
        painter.text(chip_rect.center(), Align2::CENTER_CENTER, &label,
            FontId::proportional(10.5), Color32::from_rgb(200, 220, 255));

        // Click to clear
        let resp = ui.allocate_rect(chip_rect, egui::Sense::click());
        if resp.clicked() {
            self.persist_search_filter = false;
            self.search_query.clear();
            self.status_message = Some(("Filter cleared".to_string(), std::time::Instant::now()));
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    fn draw_tag_filter_pills(&mut self, painter: &egui::Painter, canvas_rect: Rect, ui: &mut egui::Ui) {
        use crate::model::NodeTag;
        let mut tags_used: Vec<NodeTag> = self.document.nodes.iter()
            .filter_map(|n| n.tag)
            .collect();
        tags_used.dedup();
        if tags_used.is_empty() { return; }
        let all_tags = [NodeTag::Critical, NodeTag::Warning, NodeTag::Ok, NodeTag::Info];
        let pill_h = 20.0_f32;
        let pad = 8.0_f32;
        let gap = 4.0_f32;
        let visible: Vec<NodeTag> = all_tags.iter().copied()
            .filter(|t| tags_used.iter().any(|u| u == t))
            .collect();
        let total_w: f32 = visible.iter().map(|t| {
            t.label().len() as f32 * 6.5 + pad * 2.0
        }).sum::<f32>() + gap * (visible.len().saturating_sub(1)) as f32 + 24.0;
        let origin = Pos2::new(canvas_rect.max.x - total_w - 8.0, canvas_rect.min.y + 8.0);
        let mut x = origin.x;
        painter.text(Pos2::new(x, origin.y + pill_h / 2.0), Align2::LEFT_CENTER,
            "Filter:", FontId::proportional(10.0), self.theme.text_dim);
        x += 38.0;
        let click_pos = ui.ctx().input(|i| {
            if i.pointer.any_click() { i.pointer.latest_pos() } else { None }
        });
        for tag in &visible {
            let tag_c = tag.color();
            let fill_c = Color32::from_rgba_unmultiplied(tag_c[0], tag_c[1], tag_c[2], tag_c[3]);
            let label = tag.label();
            let pw = label.len() as f32 * 6.5 + pad * 2.0;
            let pill = Rect::from_min_size(Pos2::new(x, origin.y), Vec2::new(pw, pill_h));
            let is_active = self.tag_filter == Some(*tag);
            let bg_alpha = if is_active { 240u8 } else { 80u8 };
            let bg = Color32::from_rgba_unmultiplied(fill_c.r(), fill_c.g(), fill_c.b(), bg_alpha);
            painter.rect_filled(pill, CornerRadius::same(10), bg);
            if is_active {
                painter.rect_stroke(pill, CornerRadius::same(10),
                    Stroke::new(1.5, fill_c), StrokeKind::Outside);
            }
            let txt_col = if is_active { Color32::from_rgb(20, 20, 30) } else { fill_c };
            painter.text(pill.center(), Align2::CENTER_CENTER, label,
                FontId::proportional(10.5), txt_col);
            if let Some(cp) = click_pos {
                if pill.expand(2.0).contains(cp) {
                    self.tag_filter = if is_active { None } else { Some(*tag) };
                }
            }
            x += pw + gap;
        }
    }

    /// "Back to content" button — shown when user has panned away from all nodes.
    fn draw_back_to_content(&mut self, painter: &egui::Painter, canvas_rect: Rect, ui: &mut egui::Ui) {
        if self.document.nodes.is_empty() { return; }
        let any_visible = self.document.nodes.iter().any(|n| {
            let sr = Rect::from_min_size(
                self.viewport.canvas_to_screen(n.pos()),
                n.size_vec() * self.viewport.zoom,
            );
            sr.expand(40.0).intersects(canvas_rect)
        });
        if any_visible { return; }
        let btn_center = Pos2::new(canvas_rect.center().x, canvas_rect.max.y - 48.0);
        let btn_size = Vec2::new(160.0, 30.0);
        let btn_rect = Rect::from_center_size(btn_center, btn_size);
        painter.rect_filled(btn_rect, CornerRadius::same(15),
            Color32::from_rgba_premultiplied(35, 35, 55, 220));
        painter.rect_stroke(btn_rect, CornerRadius::same(15),
            Stroke::new(1.0, self.theme.accent.gamma_multiply(0.6)), StrokeKind::Outside);
        painter.text(btn_center, Align2::CENTER_CENTER, "↩  Back to content",
            FontId::proportional(12.5), self.theme.text_secondary.gamma_multiply(1.2));
        if ui.ctx().input(|i| i.pointer.any_click()) {
            if let Some(mp) = ui.ctx().input(|i| i.pointer.latest_pos()) {
                if btn_rect.contains(mp) {
                    self.fit_to_content();
                }
            }
        }
    }

    /// Multi-selection dashed bounding box with dimension labels.
    fn draw_multi_selection_dimensions(&self, painter: &egui::Painter) {
        if self.selection.node_ids.len() < 2 { return; }
        let sel_nodes: Vec<&Node> = self.document.nodes.iter()
            .filter(|n| self.selection.contains_node(&n.id))
            .collect();
        if sel_nodes.is_empty() { return; }
        let mut min_x = f32::MAX; let mut min_y = f32::MAX;
        let mut max_x = f32::MIN; let mut max_y = f32::MIN;
        for n in &sel_nodes {
            let p = n.pos(); let s = n.size_vec();
            min_x = min_x.min(p.x); min_y = min_y.min(p.y);
            max_x = max_x.max(p.x + s.x); max_y = max_y.max(p.y + s.y);
        }
        let tl = self.viewport.canvas_to_screen(Pos2::new(min_x, min_y));
        let br = self.viewport.canvas_to_screen(Pos2::new(max_x, max_y));
        let bbox = Rect::from_min_max(tl, br).expand(8.0);
        // Dashed border
        let dash_color = self.theme.selection_color.gamma_multiply(0.55);
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
        let lbl_color = self.theme.selection_color.gamma_multiply(0.75);
        let bg = self.theme.dim_overlay;
        let w_text = format!("{:.0}", w_canvas);
        let w_pos = Pos2::new(bbox.center().x, bbox.max.y + 10.0);
        let w_galley = painter.layout_no_wrap(w_text.clone(), FontId::proportional(font_sz), lbl_color);
        let wr = Rect::from_center_size(w_pos, w_galley.size()).expand2(Vec2::new(4.0, 2.0));
        painter.rect_filled(wr, CornerRadius::same(3), bg);
        painter.text(w_pos, Align2::CENTER_CENTER, &w_text, FontId::proportional(font_sz), lbl_color);
        let h_text = format!("{:.0}", h_canvas);
        let h_pos = Pos2::new(bbox.max.x + 10.0, bbox.center().y);
        let h_galley = painter.layout_no_wrap(h_text.clone(), FontId::proportional(font_sz), lbl_color);
        let hr = Rect::from_center_size(h_pos, h_galley.size()).expand2(Vec2::new(4.0, 2.0));
        painter.rect_filled(hr, CornerRadius::same(3), bg);
        painter.text(h_pos, Align2::CENTER_CENTER, &h_text, FontId::proportional(font_sz), lbl_color);
        let count_text = format!("{}×", sel_nodes.len());
        let count_pos = Pos2::new(bbox.min.x - 1.0, bbox.min.y - 10.0);
        painter.text(count_pos, Align2::LEFT_BOTTOM, &count_text, FontId::proportional(font_sz), lbl_color);
        let _ = (w_galley, h_galley);
    }

    /// Deletion ghost animations: shrink-fade over 0.25s.
    fn draw_deletion_ghosts(&mut self, painter: &egui::Painter) {
        let now = painter.ctx().input(|i| i.time);
        let duration = 0.25_f64;
        self.deletion_ghosts.retain(|g| now - g.3 < duration);
        if self.deletion_ghosts.is_empty() { return; }
        for &(center, size, fill, death_time) in &self.deletion_ghosts {
            let t = ((now - death_time) / duration) as f32;
            let scale = 1.0 - t * 0.5;
            let alpha = ((1.0 - t).powf(1.5) * 180.0) as u8;
            let screen_center = self.viewport.canvas_to_screen(Pos2::new(center[0], center[1]));
            let sw = size[0] * self.viewport.zoom * scale;
            let sh = size[1] * self.viewport.zoom * scale;
            let ghost_rect = Rect::from_center_size(screen_center, Vec2::new(sw, sh));
            let fill_col = Color32::from_rgba_unmultiplied(fill[0], fill[1], fill[2], alpha);
            painter.rect_filled(ghost_rect, CornerRadius::same(6), fill_col);
            let stroke_col = Color32::from_rgba_unmultiplied(200, 80, 80, (alpha as f32 * 0.8) as u8);
            painter.rect_stroke(ghost_rect, CornerRadius::same(6),
                Stroke::new(1.5, stroke_col), StrokeKind::Outside);
        }
        painter.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }

    /// Resize feedback: ghost outline of original rect + live size readout.
    fn draw_resize_feedback(&self, painter: &egui::Painter, pointer_pos: Option<Pos2>) {
        if let DragState::ResizingNode { node_id, start_rect, .. } = &self.drag {
            // Live size readout near cursor
            if let (Some(node), Some(mouse)) = (self.document.find_node(node_id), pointer_pos) {
                let [w, h] = node.size;
                let label = format!("{w:.0} × {h:.0}");
                let lpos = mouse + Vec2::new(12.0, 12.0);
                let font = FontId::proportional(11.0);
                let galley = painter.layout_no_wrap(label.clone(), font.clone(), Color32::WHITE);
                let bg = Rect::from_min_size(lpos - Vec2::new(4.0, 2.0), galley.size() + Vec2::new(8.0, 4.0));
                painter.rect_filled(bg, CornerRadius::same(4),
                    Color32::from_rgba_premultiplied(20, 20, 36, 220));
                painter.rect_stroke(bg, CornerRadius::same(4),
                    Stroke::new(0.5, self.theme.accent.gamma_multiply(0.5)), StrokeKind::Outside);
                painter.text(lpos, Align2::LEFT_TOP, &label, font,
                    Color32::from_rgba_unmultiplied(205, 214, 244, 240));
            }
            // Ghost outline of original rect
            let sr = *start_rect;
            let tl = self.viewport.canvas_to_screen(Pos2::new(sr[0], sr[1]));
            let ghost_rect = Rect::from_min_size(tl, Vec2::new(sr[2], sr[3]) * self.viewport.zoom);
            painter.rect_stroke(ghost_rect, CornerRadius::same(4),
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(137, 180, 250, 90)),
                StrokeKind::Outside);
            painter.rect_stroke(ghost_rect.expand(1.5), CornerRadius::same(5),
                Stroke::new(0.5, Color32::from_rgba_unmultiplied(137, 180, 250, 40)),
                StrokeKind::Outside);
            painter.text(ghost_rect.center_bottom() + Vec2::new(0.0, 6.0), Align2::CENTER_TOP,
                &format!("{:.0} × {:.0}", sr[2], sr[3]), FontId::proportional(9.0),
                Color32::from_rgba_unmultiplied(137, 180, 250, 160));
        }
    }

    /// Focus mode + tag filter dim overlays.
    fn draw_focus_and_filter_overlays(
        &self,
        painter: &egui::Painter,
        canvas_rect: Rect,
        focus_neighbors: &std::collections::HashSet<NodeId>,
    ) {
        if self.focus_mode && !self.selection.is_empty() {
            for node in &self.document.nodes {
                if self.selection.contains_node(&node.id) { continue; }
                let screen_pos = self.viewport.canvas_to_screen(node.pos());
                let screen_size = node.size_vec() * self.viewport.zoom;
                let screen_rect = Rect::from_min_size(screen_pos, screen_size);
                if !screen_rect.intersects(canvas_rect) { continue; }
                let dim = if focus_neighbors.contains(&node.id) { self.theme.focus_dim_near } else { self.theme.focus_dim_far };
                painter.rect_filled(screen_rect, CornerRadius::same(4), dim);
            }
        }
        if let Some(filter_tag) = self.tag_filter {
            for node in &self.document.nodes {
                if node.tag == Some(filter_tag) { continue; }
                let screen_pos = self.viewport.canvas_to_screen(node.pos());
                let screen_size = node.size_vec() * self.viewport.zoom;
                let screen_rect = Rect::from_min_size(screen_pos, screen_size);
                if !screen_rect.intersects(canvas_rect) { continue; }
                painter.rect_filled(screen_rect, CornerRadius::same(4), self.theme.dim_overlay_heavy);
            }
        }
    }

    /// Multi-node drag ghost: faint outlines at original positions.
    fn draw_drag_ghosts(&self, painter: &egui::Painter, canvas_rect: Rect) {
        if let DragState::DraggingNode { start_positions, .. } = &self.drag {
            if start_positions.len() >= 2 {
                let ghost_stroke = Stroke::new(1.0, self.theme.ghost_stroke);
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
    }

    /// Multi-selection bounding box with corner + midpoint handles and size label.
    fn draw_multi_selection_handles(&self, painter: &egui::Painter) {
        if self.selection.node_ids.len() < 2 { return; }
        let bb = self.selection.node_ids.iter()
            .filter_map(|id| self.document.find_node(id))
            .fold(Option::<Rect>::None, |acc, n| {
                let sr = Rect::from_min_size(
                    self.viewport.canvas_to_screen(n.pos()),
                    n.size_vec() * self.viewport.zoom,
                );
                Some(acc.map_or(sr, |r| r.union(sr)))
            });
        let Some(bb) = bb else { return };
        let expanded = bb.expand(6.0);
        painter.rect_stroke(expanded, CornerRadius::same(4),
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(137, 180, 250, 80)),
            StrokeKind::Outside);
        for corner in [expanded.left_top(), expanded.right_top(), expanded.left_bottom(), expanded.right_bottom()] {
            painter.circle_filled(corner, 4.0, Color32::from_rgba_unmultiplied(137, 180, 250, 180));
            painter.circle_stroke(corner, 4.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(30, 30, 46, 200)));
        }
        for mid in [
            Pos2::new(expanded.center().x, expanded.min.y),
            Pos2::new(expanded.center().x, expanded.max.y),
            Pos2::new(expanded.min.x, expanded.center().y),
            Pos2::new(expanded.max.x, expanded.center().y),
        ] {
            painter.circle_filled(mid, 3.0, Color32::from_rgba_unmultiplied(137, 180, 250, 140));
        }
        let w = (bb.width() / self.viewport.zoom).round() as i32;
        let h = (bb.height() / self.viewport.zoom).round() as i32;
        painter.text(expanded.center_top() - Vec2::new(0.0, 14.0), Align2::CENTER_BOTTOM,
            &format!("{w} × {h}"), FontId::proportional(9.0),
            Color32::from_rgba_unmultiplied(137, 180, 250, 160));
    }
}


/// Format a canvas coordinate for ruler labels: suppress ".0" for integers.
fn format_coord(v: f32) -> String {
    if v.fract().abs() < 0.05 {
        format!("{}", v as i64)
    } else {
        format!("{:.1}", v)
    }
}
