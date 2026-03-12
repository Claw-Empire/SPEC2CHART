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

        painter.rect_filled(canvas_rect, CornerRadius::ZERO, CANVAS_BG);

        if self.show_grid {
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
                    // Click on node => select it
                    if cmd_held {
                        self.selection.toggle_node(node_id);
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

        // Double-click to focus label editing
        if response.double_clicked() {
            if let Some(mouse) = pointer_pos {
                let canvas_pos = self.viewport.screen_to_canvas(mouse);
                if let Some(node_id) = self.document.node_at_pos(canvas_pos) {
                    self.selection.select_node(node_id);
                    self.focus_label_edit = true;
                }
            }
        }

        // Set cursor
        let cursor = self.pick_cursor(pointer_pos);
        ui.ctx().set_cursor_icon(cursor);

        // --- Drawing ---
        let node_idx = self.document.node_index();
        let hover_pos = ui.ctx().input(|i| i.pointer.hover_pos());

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
                self.draw_edge(edge, &painter, &node_idx, hover_canvas);
            }
        }

        // Nodes (visible only)
        for node in &self.document.nodes {
            let screen_pos = self.viewport.canvas_to_screen(node.pos());
            let screen_size = node.size_vec() * self.viewport.zoom;
            let screen_rect = Rect::from_min_size(screen_pos, screen_size).expand(20.0);
            if screen_rect.intersects(canvas_rect) {
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

        // --- Previews ---
        self.draw_box_select_preview(&painter, pointer_pos);
        self.draw_edge_creation_preview(&painter, &node_idx);
        self.draw_new_node_preview(&painter, canvas_rect);
        self.draw_node_tooltip(&painter, hover_pos, canvas_rect);
        self.draw_status_toast(&painter, canvas_rect, ui.ctx());
        self.draw_canvas_hud(&painter, canvas_rect);
        self.draw_empty_canvas_hint(&painter, canvas_rect);
        self.draw_search_overlay(ui, canvas_rect);
        self.draw_minimap(&painter, canvas_rect);

        // Minimap click-to-pan
        if let Some(click_pos) = pointer_pos {
            if ui.ctx().input(|i| i.pointer.primary_clicked()) {
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
                DragState::DraggingNode { .. } | DragState::ResizingNode { .. } => {
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

    fn draw_box_select_preview(&self, painter: &egui::Painter, pointer_pos: Option<Pos2>) {
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

        // Highlight target port
        let canvas_dst = self.viewport.screen_to_canvas(*current_screen);
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

    fn draw_canvas_hud(&self, painter: &egui::Painter, canvas_rect: Rect) {
        let zoom_pct = (self.viewport.zoom * 100.0).round() as i32;
        let n_nodes = self.document.nodes.len();
        let n_edges = self.document.edges.len();
        let n_sel = self.selection.node_ids.len();

        let line1 = format!("{zoom_pct}%");
        let line2 = if n_sel > 0 {
            format!("{n_sel} selected  ·  {n_nodes}N {n_edges}E")
        } else {
            format!("{n_nodes} nodes  ·  {n_edges} edges")
        };

        let pad = 8.0;
        let x = canvas_rect.min.x + pad;
        let y = canvas_rect.max.y - 36.0;

        let font_big = egui::FontId::proportional(15.0);
        let font_sm  = egui::FontId::proportional(10.5);

        painter.text(
            Pos2::new(x, y),
            egui::Align2::LEFT_TOP,
            &line1,
            font_big,
            TEXT_SECONDARY,
        );
        painter.text(
            Pos2::new(x, y + 17.0),
            egui::Align2::LEFT_TOP,
            &line2,
            font_sm,
            TEXT_DIM,
        );
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

    // --- Minimap ---

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

        for node in &self.document.nodes {
            let center = node.rect().center();
            let screen_pt = map_point(center.x, center.y);
            if minimap_rect.contains(screen_pt) {
                painter.circle_filled(screen_pt, 2.5, MINIMAP_NODE);
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
