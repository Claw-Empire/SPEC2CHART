use egui::{Pos2, Rect, Vec2};
use crate::model::*;
use super::{FlowchartApp, ResizeHandle};
use super::theme::PORT_HIT_RADIUS;

// ---------------------------------------------------------------------------
// Hit testing
// ---------------------------------------------------------------------------

impl FlowchartApp {
    pub(crate) fn hit_test_port(&self, canvas_pos: Pos2) -> Option<Port> {
        let threshold = PORT_HIT_RADIUS / self.viewport.zoom;
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

    pub(crate) fn hit_test_edge(&self, canvas_pos: Pos2) -> Option<EdgeId> {
        let threshold = 14.0 / self.viewport.zoom;
        for edge in self.document.edges.iter().rev() {
            let src_node = self.document.find_node(&edge.source.node_id);
            let tgt_node = self.document.find_node(&edge.target.node_id);
            if let (Some(sn), Some(tn)) = (src_node, tgt_node) {
                let src = sn.port_position(edge.source.side);
                let tgt = tn.port_position(edge.target.side);
                let (cp1, cp2) = control_points_for_side(src, tgt, edge.source.side, 60.0);
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

    /// Returns the EdgeId if `screen_pos` is near the bend handle of a selected, non-orthogonal edge.
    pub(crate) fn hit_test_bend_handle(&self, screen_pos: Pos2) -> Option<EdgeId> {
        if self.selection.edge_ids.len() != 1 {
            return None;
        }
        let edge_id = *self.selection.edge_ids.iter().next()?;
        let edge = self.document.find_edge(&edge_id)?;
        if edge.style.orthogonal {
            return None;
        }
        let src_node = self.document.find_node(&edge.source.node_id)?;
        let tgt_node = self.document.find_node(&edge.target.node_id)?;
        let src = self.viewport.canvas_to_screen(src_node.port_position(edge.source.side));
        let tgt = self.viewport.canvas_to_screen(tgt_node.port_position(edge.target.side));
        let offset = 60.0 * self.viewport.zoom;
        let (mut cp1, mut cp2) = control_points_for_side(src, tgt, edge.source.side, offset);
        if edge.style.curve_bend.abs() > 0.1 {
            let dir = if (tgt - src).length() > 1.0 { (tgt - src).normalized() } else { Vec2::X };
            let perp = Vec2::new(-dir.y, dir.x);
            let bend_screen = edge.style.curve_bend * self.viewport.zoom;
            cp1 = cp1 + perp * bend_screen;
            cp2 = cp2 + perp * bend_screen;
        }
        let handle_pos = super::interaction::cubic_bezier_point(src, cp1, cp2, tgt, 0.5);
        if (screen_pos - handle_pos).length() < 12.0 {
            Some(edge_id)
        } else {
            None
        }
    }

    pub(crate) fn hit_test_resize_handle(
        &self,
        screen_pos: Pos2,
    ) -> Option<(NodeId, ResizeHandle)> {
        if self.selection.node_ids.len() != 1 {
            return None;
        }
        let node_id = *self.selection.node_ids.iter().next().unwrap();
        let node = self.document.find_node(&node_id)?;
        let top_left = self.viewport.canvas_to_screen(node.pos());
        let size = node.size_vec() * self.viewport.zoom;
        let screen_rect = Rect::from_min_size(top_left, size);
        let handles = Self::resize_handle_positions(screen_rect);
        let threshold = 12.0;
        for (handle, pos) in &handles {
            if (screen_pos - *pos).length() < threshold {
                return Some((node_id, *handle));
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Resize handles
// ---------------------------------------------------------------------------

impl FlowchartApp {
    pub(crate) fn resize_handle_positions(screen_rect: Rect) -> [(ResizeHandle, Pos2); 8] {
        [
            (ResizeHandle::TopLeft, screen_rect.left_top()),
            (
                ResizeHandle::Top,
                Pos2::new(screen_rect.center().x, screen_rect.min.y),
            ),
            (ResizeHandle::TopRight, screen_rect.right_top()),
            (
                ResizeHandle::Left,
                Pos2::new(screen_rect.min.x, screen_rect.center().y),
            ),
            (
                ResizeHandle::Right,
                Pos2::new(screen_rect.max.x, screen_rect.center().y),
            ),
            (ResizeHandle::BottomLeft, screen_rect.left_bottom()),
            (
                ResizeHandle::Bottom,
                Pos2::new(screen_rect.center().x, screen_rect.max.y),
            ),
            (ResizeHandle::BottomRight, screen_rect.right_bottom()),
        ]
    }

    pub(crate) fn resize_cursor(handle: ResizeHandle) -> egui::CursorIcon {
        match handle {
            ResizeHandle::TopLeft | ResizeHandle::BottomRight => egui::CursorIcon::ResizeNwSe,
            ResizeHandle::TopRight | ResizeHandle::BottomLeft => egui::CursorIcon::ResizeNeSw,
            ResizeHandle::Left | ResizeHandle::Right => egui::CursorIcon::ResizeHorizontal,
            ResizeHandle::Top | ResizeHandle::Bottom => egui::CursorIcon::ResizeVertical,
        }
    }

    pub(crate) fn compute_resize(
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
}

// ---------------------------------------------------------------------------
// Zoom helpers
// ---------------------------------------------------------------------------

impl FlowchartApp {
    pub(crate) fn fit_to_content(&mut self) {
        if self.document.nodes.is_empty() {
            return;
        }
        self.fit_to_rects(self.document.nodes.iter().map(|n| n.rect()).collect());
    }

    pub(crate) fn zoom_to_selection(&mut self) {
        let rects: Vec<Rect> = self
            .selection
            .node_ids
            .iter()
            .filter_map(|id| self.document.find_node(id))
            .map(|n| n.rect())
            .collect();
        if rects.is_empty() {
            return;
        }
        self.fit_to_rects(rects);
    }

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
        let zoom = (canvas_w / bb.width())
            .min(canvas_h / bb.height())
            .clamp(0.1, 10.0);

        self.viewport.zoom = zoom;
        self.viewport.offset[0] =
            self.canvas_rect.min.x + canvas_w / 2.0 - bb.center().x * zoom;
        self.viewport.offset[1] =
            self.canvas_rect.min.y + canvas_h / 2.0 - bb.center().y * zoom;
    }

    pub(crate) fn step_zoom(&mut self, factor: f32) {
        let center = self.canvas_rect.center();
        let old_zoom = self.viewport.zoom;
        // Set zoom_target for smooth interpolation; immediately update offset for pivot stability
        let new_zoom = (old_zoom * factor).clamp(0.1, 10.0);
        self.zoom_target = new_zoom;
        // Pan offset so that the canvas center stays in place
        let ratio = new_zoom / old_zoom;
        self.viewport.offset[0] = center.x - ratio * (center.x - self.viewport.offset[0]);
        self.viewport.offset[1] = center.y - ratio * (center.y - self.viewport.offset[1]);
    }

    /// BFS shortest path edges between two nodes (undirected). Returns edge IDs on path or empty if unreachable.
    pub(crate) fn bfs_path_edges(&self, from: NodeId, to: NodeId) -> Vec<EdgeId> {
        if from == to { return vec![]; }
        use std::collections::{HashMap, HashSet, VecDeque};
        let mut visited: HashSet<NodeId> = HashSet::new();
        // (current_node, parent_node, edge_id_used)
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        let mut prev: HashMap<NodeId, (NodeId, EdgeId)> = HashMap::new();
        queue.push_back(from);
        visited.insert(from);
        let mut found = false;
        'outer: while let Some(curr) = queue.pop_front() {
            for edge in &self.document.edges {
                let neighbor_and_eid = if edge.source.node_id == curr {
                    Some((edge.target.node_id, edge.id))
                } else if edge.target.node_id == curr {
                    Some((edge.source.node_id, edge.id))
                } else { None };
                if let Some((nb, eid)) = neighbor_and_eid {
                    if !visited.contains(&nb) {
                        visited.insert(nb);
                        prev.insert(nb, (curr, eid));
                        if nb == to { found = true; break 'outer; }
                        queue.push_back(nb);
                    }
                }
            }
        }
        if !found { return vec![]; }
        let mut path = Vec::new();
        let mut cur = to;
        while cur != from {
            let (p, eid) = prev[&cur];
            path.push(eid);
            cur = p;
        }
        path
    }

    /// BFS shortest path in hops between two nodes (undirected). Returns None if unreachable.
    pub(crate) fn bfs_path_length(&self, from: NodeId, to: NodeId) -> Option<usize> {
        if from == to { return Some(0); }
        use std::collections::{HashSet, VecDeque};
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<(NodeId, usize)> = VecDeque::new();
        queue.push_back((from, 0));
        visited.insert(from);
        while let Some((curr, dist)) = queue.pop_front() {
            // Find all neighbors via edges (undirected)
            for edge in &self.document.edges {
                let neighbor = if edge.source.node_id == curr {
                    Some(edge.target.node_id)
                } else if edge.target.node_id == curr {
                    Some(edge.source.node_id)
                } else {
                    None
                };
                if let Some(nb) = neighbor {
                    if nb == to { return Some(dist + 1); }
                    if !visited.contains(&nb) {
                        visited.insert(nb);
                        queue.push_back((nb, dist + 1));
                    }
                }
            }
        }
        None
    }

    pub(crate) fn snap_pos(&self, pos: Pos2) -> Pos2 {
        Pos2::new(
            (pos.x / self.grid_size).round() * self.grid_size,
            (pos.y / self.grid_size).round() * self.grid_size,
        )
    }
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

pub fn control_points_for_side(
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
    let dx = src.x - tgt.x;
    let dy = src.y - tgt.y;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let cp2 = Pos2::new(tgt.x + dx / len * offset, tgt.y + dy / len * offset);
    (cp1, cp2)
}

pub fn cubic_bezier_point(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, t: f32) -> Pos2 {
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

    #[test]
    fn resize_bottom_right_grows() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(30.0, 20.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::BottomRight, start, delta, min);
        assert_eq!(x, 100.0);
        assert_eq!(y, 100.0);
        assert_eq!(w, 170.0);
        assert_eq!(h, 80.0);
    }

    #[test]
    fn resize_top_left_grows() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(-20.0, -10.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::TopLeft, start, delta, min);
        assert_eq!(x, 80.0);
        assert_eq!(y, 90.0);
        assert_eq!(w, 160.0);
        assert_eq!(h, 70.0);
    }

    #[test]
    fn resize_right_only_changes_width() {
        let start = [100.0, 200.0, 140.0, 60.0];
        let delta = Vec2::new(50.0, 999.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::Right, start, delta, min);
        assert_eq!(x, 100.0);
        assert_eq!(y, 200.0);
        assert_eq!(w, 190.0);
        assert_eq!(h, 60.0);
    }

    #[test]
    fn resize_bottom_only_changes_height() {
        let start = [100.0, 200.0, 140.0, 60.0];
        let delta = Vec2::new(999.0, 40.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::Bottom, start, delta, min);
        assert_eq!(x, 100.0);
        assert_eq!(y, 200.0);
        assert_eq!(w, 140.0);
        assert_eq!(h, 100.0);
    }

    #[test]
    fn resize_clamps_to_min_size_shape() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(-200.0, -200.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::BottomRight, start, delta, min);
        assert_eq!(x, 100.0);
        assert_eq!(y, 100.0);
        assert_eq!(w, 40.0);
        assert_eq!(h, 30.0);
    }

    #[test]
    fn resize_clamps_to_min_size_sticky() {
        let start = [50.0, 50.0, 150.0, 150.0];
        let delta = Vec2::new(-200.0, -200.0);
        let min = MIN_SIZE_STICKY;
        let [_x, _y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::BottomRight, start, delta, min);
        assert_eq!(w, 60.0);
        assert_eq!(h, 60.0);
    }

    #[test]
    fn resize_top_left_clamps_adjusts_position() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(200.0, 200.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::TopLeft, start, delta, min);
        assert_eq!(w, 40.0);
        assert_eq!(h, 30.0);
        assert_eq!(x, 200.0);
        assert_eq!(y, 130.0);
    }

    #[test]
    fn resize_left_moves_x_keeps_right_edge() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(-30.0, 0.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::Left, start, delta, min);
        assert_eq!(x, 70.0);
        assert_eq!(w, 170.0);
        assert_eq!(y, 100.0);
        assert_eq!(h, 60.0);
    }

    #[test]
    fn resize_top_moves_y_keeps_bottom_edge() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(0.0, -25.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::Top, start, delta, min);
        assert_eq!(x, 100.0);
        assert_eq!(w, 140.0);
        assert_eq!(y, 75.0);
        assert_eq!(h, 85.0);
    }

    #[test]
    fn handle_positions_are_correct() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 200.0), Vec2::new(200.0, 100.0));
        let handles = FlowchartApp::resize_handle_positions(rect);

        assert_eq!(handles[0].0, ResizeHandle::TopLeft);
        assert_eq!(handles[0].1, Pos2::new(100.0, 200.0));
        assert_eq!(handles[1].0, ResizeHandle::Top);
        assert_eq!(handles[1].1, Pos2::new(200.0, 200.0));
        assert_eq!(handles[2].0, ResizeHandle::TopRight);
        assert_eq!(handles[2].1, Pos2::new(300.0, 200.0));
        assert_eq!(handles[3].0, ResizeHandle::Left);
        assert_eq!(handles[3].1, Pos2::new(100.0, 250.0));
        assert_eq!(handles[4].0, ResizeHandle::Right);
        assert_eq!(handles[4].1, Pos2::new(300.0, 250.0));
        assert_eq!(handles[5].0, ResizeHandle::BottomLeft);
        assert_eq!(handles[5].1, Pos2::new(100.0, 300.0));
        assert_eq!(handles[6].0, ResizeHandle::Bottom);
        assert_eq!(handles[6].1, Pos2::new(200.0, 300.0));
        assert_eq!(handles[7].0, ResizeHandle::BottomRight);
        assert_eq!(handles[7].1, Pos2::new(300.0, 300.0));
    }

    #[test]
    fn resize_cursors_are_correct() {
        assert_eq!(
            FlowchartApp::resize_cursor(ResizeHandle::TopLeft),
            egui::CursorIcon::ResizeNwSe
        );
        assert_eq!(
            FlowchartApp::resize_cursor(ResizeHandle::BottomRight),
            egui::CursorIcon::ResizeNwSe
        );
        assert_eq!(
            FlowchartApp::resize_cursor(ResizeHandle::TopRight),
            egui::CursorIcon::ResizeNeSw
        );
        assert_eq!(
            FlowchartApp::resize_cursor(ResizeHandle::BottomLeft),
            egui::CursorIcon::ResizeNeSw
        );
        assert_eq!(
            FlowchartApp::resize_cursor(ResizeHandle::Left),
            egui::CursorIcon::ResizeHorizontal
        );
        assert_eq!(
            FlowchartApp::resize_cursor(ResizeHandle::Right),
            egui::CursorIcon::ResizeHorizontal
        );
        assert_eq!(
            FlowchartApp::resize_cursor(ResizeHandle::Top),
            egui::CursorIcon::ResizeVertical
        );
        assert_eq!(
            FlowchartApp::resize_cursor(ResizeHandle::Bottom),
            egui::CursorIcon::ResizeVertical
        );
    }

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

    #[test]
    fn resize_top_right_grows() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(20.0, -15.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::TopRight, start, delta, min);
        assert_eq!(x, 100.0);
        assert_eq!(y, 85.0);
        assert_eq!(w, 160.0);
        assert_eq!(h, 75.0);
    }

    #[test]
    fn resize_bottom_left_grows() {
        let start = [100.0, 100.0, 140.0, 60.0];
        let delta = Vec2::new(-20.0, 15.0);
        let min = MIN_SIZE_SHAPE;
        let [x, y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::BottomLeft, start, delta, min);
        assert_eq!(x, 80.0);
        assert_eq!(y, 100.0);
        assert_eq!(w, 160.0);
        assert_eq!(h, 75.0);
    }

    #[test]
    fn resize_entity_respects_min() {
        let start = [0.0, 0.0, 200.0, 100.0];
        let delta = Vec2::new(-300.0, -300.0);
        let min = MIN_SIZE_ENTITY;
        let [_x, _y, w, h] =
            FlowchartApp::compute_resize(ResizeHandle::BottomRight, start, delta, min);
        assert_eq!(w, 160.0);
        assert_eq!(h, MIN_SIZE_ENTITY[1]);
    }
}
