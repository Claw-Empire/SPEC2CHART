use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use crate::model::*;
use super::FlowchartApp;
use super::camera::compute_z_layers;
use super::interaction::{control_points_for_side, cubic_bezier_point};
use super::theme::PORT_RADIUS;
use super::{DragState, ResizeHandle, Tool};

const Z_SPACING: f32 = 120.0;
const CUBE_THICKNESS: f32 = 40.0;

/// Projected node info for hit-testing and rendering.
struct NodeProjection {
    node_id: NodeId,
    node_idx: usize,
    screen_pos: Pos2,
    screen_rect: Rect,
    depth_scale: f32,
    z_layer: i32,
    cam_depth: f32,
    z: f32,
}

/// Darken or lighten a color by a multiplicative factor.
fn shade_color(c: Color32, factor: f32) -> Color32 {
    Color32::from_rgba_premultiplied(
        (c.r() as f32 * factor).clamp(0.0, 255.0) as u8,
        (c.g() as f32 * factor).clamp(0.0, 255.0) as u8,
        (c.b() as f32 * factor).clamp(0.0, 255.0) as u8,
        c.a(),
    )
}

impl FlowchartApp {
    /// Sync 3D camera to match the current 2D viewport center and zoom.
    pub(crate) fn sync_camera_to_viewport(&mut self) {
        let screen_center = self.canvas_rect.center();
        let world_center = self.viewport.screen_to_canvas(screen_center);
        self.camera3d.target = [world_center.x, world_center.y, 0.0];
        let visible_width = self.canvas_rect.width() / self.viewport.zoom;
        let fov_tan = (self.camera3d.fov / 2.0).tan();
        self.camera3d.distance = (visible_width / (2.0 * fov_tan)).max(100.0);
    }

    /// Sync the 2D viewport to match the current 3D camera target and distance.
    pub(crate) fn sync_viewport_to_camera(&mut self) {
        let fov_tan = (self.camera3d.fov / 2.0).tan();
        let visible_width = 2.0 * self.camera3d.distance * fov_tan;
        let zoom = (self.canvas_rect.width() / visible_width).clamp(0.05, 10.0);
        let screen_center = self.canvas_rect.center();
        self.viewport.offset = [
            screen_center.x - self.camera3d.target[0] * zoom,
            screen_center.y - self.camera3d.target[1] * zoom,
        ];
        self.viewport.zoom = zoom;
    }

    /// Animate the view transition value towards its target.
    pub(crate) fn animate_view_transition(&mut self) -> bool {
        let speed = 5.0;
        let dt = 1.0 / 60.0;
        let diff = self.view_transition_target - self.view_transition;
        if diff.abs() < 0.005 {
            self.view_transition = self.view_transition_target;
            return false;
        }
        self.view_transition += diff * speed * dt;
        self.view_transition = self.view_transition.clamp(0.0, 1.0);
        true
    }

    // -----------------------------------------------------------------------
    // Hit testing helpers
    // -----------------------------------------------------------------------

    fn hit_test_3d_node(mouse: Pos2, projections: &[NodeProjection]) -> Option<NodeId> {
        for proj in projections.iter().rev() {
            if proj.screen_rect.contains(mouse) {
                return Some(proj.node_id);
            }
        }
        None
    }

    fn hit_test_3d_port(
        &self,
        mouse: Pos2,
        projections: &[NodeProjection],
        screen_center: Pos2,
        screen_size: Vec2,
    ) -> Option<Port> {
        let threshold = 15.0;
        for proj in projections.iter().rev() {
            let node = &self.document.nodes[proj.node_idx];
            for side in &ALL_SIDES {
                let port_pos = node.port_position(*side);
                if let Some((screen_port, _)) = self.camera3d.project(
                    [port_pos.x, port_pos.y, proj.z],
                    screen_center,
                    screen_size,
                ) {
                    if (mouse - screen_port).length() < threshold {
                        return Some(Port {
                            node_id: proj.node_id,
                            side: *side,
                        });
                    }
                }
            }
        }
        None
    }

    fn hit_test_3d_resize_handle(
        &self,
        mouse: Pos2,
        projections: &[NodeProjection],
    ) -> Option<(NodeId, ResizeHandle)> {
        if self.selection.node_ids.len() != 1 {
            return None;
        }
        let selected_id = *self.selection.node_ids.iter().next().unwrap();
        let proj = projections.iter().find(|p| p.node_id == selected_id)?;
        let handles = Self::resize_handle_positions(proj.screen_rect);
        let threshold = 14.0;
        for (handle, pos) in &handles {
            if (mouse - *pos).length() < threshold {
                return Some((selected_id, *handle));
            }
        }
        None
    }

    // -----------------------------------------------------------------------
    // 3D helpers
    // -----------------------------------------------------------------------

    /// Compute screen-space extrusion vector for 3D depth effect.
    fn compute_extrude(
        &self,
        world_center: [f32; 3],
        thickness: f32,
        screen_center: Pos2,
        screen_size: Vec2,
    ) -> Vec2 {
        let top = [world_center[0], world_center[1], world_center[2] + thickness];
        match (
            self.camera3d.project(world_center, screen_center, screen_size),
            self.camera3d.project(top, screen_center, screen_size),
        ) {
            (Some((b, _)), Some((t, _))) => Vec2::new(t.x - b.x, t.y - b.y),
            _ => Vec2::new(0.0, -8.0),
        }
    }

    /// Unproject a screen-space delta to world-space movement on the node's z-plane.
    #[allow(dead_code)]
    fn unproject_drag_delta(
        &self,
        screen_delta: Vec2,
        world_origin: Pos2,
        z: f32,
        screen_center: Pos2,
        screen_size: Vec2,
    ) -> Vec2 {
        let eps = 1.0;
        let origin = [world_origin.x, world_origin.y, z];

        let Some((s0, _)) = self.camera3d.project(origin, screen_center, screen_size) else {
            return Vec2::ZERO;
        };
        let dx_world = [origin[0] + eps, origin[1], origin[2]];
        let Some((sx, _)) = self.camera3d.project(dx_world, screen_center, screen_size) else {
            return Vec2::ZERO;
        };
        let dy_world = [origin[0], origin[1] + eps, origin[2]];
        let Some((sy, _)) = self.camera3d.project(dy_world, screen_center, screen_size) else {
            return Vec2::ZERO;
        };

        let a = (sx.x - s0.x) / eps;
        let b = (sy.x - s0.x) / eps;
        let c = (sx.y - s0.y) / eps;
        let d = (sy.y - s0.y) / eps;

        let det = a * d - b * c;
        if det.abs() < 1e-6 {
            return Vec2::ZERO;
        }

        let inv_det = 1.0 / det;
        let world_dx = inv_det * (d * screen_delta.x - b * screen_delta.y);
        let world_dy = inv_det * (-c * screen_delta.x + a * screen_delta.y);

        Vec2::new(world_dx, world_dy)
    }

    /// Unproject screen delta to true 3D movement on the camera view plane.
    /// Solves a 3x3 system: 2 Jacobian rows (screen ↔ world) + 1 view-plane constraint.
    fn unproject_drag_3d(
        &self,
        screen_delta: Vec2,
        world_origin: [f32; 3],
        screen_center: Pos2,
        screen_size: Vec2,
    ) -> [f32; 3] {
        let eps = 1.0;

        let Some((s0, _)) =
            self.camera3d.project(world_origin, screen_center, screen_size)
        else {
            return [0.0, 0.0, 0.0];
        };

        // Jacobian via finite differences along X, Y, Z world axes
        let Some((sx, _)) = self.camera3d.project(
            [world_origin[0] + eps, world_origin[1], world_origin[2]],
            screen_center,
            screen_size,
        ) else {
            return [0.0, 0.0, 0.0];
        };
        let Some((sy, _)) = self.camera3d.project(
            [world_origin[0], world_origin[1] + eps, world_origin[2]],
            screen_center,
            screen_size,
        ) else {
            return [0.0, 0.0, 0.0];
        };
        let Some((sz, _)) = self.camera3d.project(
            [world_origin[0], world_origin[1], world_origin[2] + eps],
            screen_center,
            screen_size,
        ) else {
            return [0.0, 0.0, 0.0];
        };

        // J rows: d(screen_x)/d(world), d(screen_y)/d(world)
        let j00 = (sx.x - s0.x) / eps;
        let j01 = (sy.x - s0.x) / eps;
        let j02 = (sz.x - s0.x) / eps;
        let j10 = (sx.y - s0.y) / eps;
        let j11 = (sy.y - s0.y) / eps;
        let j12 = (sz.y - s0.y) / eps;

        // Camera forward direction (view-plane normal constraint)
        let cam_pos = self.camera3d.position();
        let fwd = [
            self.camera3d.target[0] - cam_pos[0],
            self.camera3d.target[1] - cam_pos[1],
            self.camera3d.target[2] - cam_pos[2],
        ];
        let fwd_len =
            (fwd[0] * fwd[0] + fwd[1] * fwd[1] + fwd[2] * fwd[2]).sqrt();
        if fwd_len < 0.001 {
            return [0.0, 0.0, 0.0];
        }
        let f0 = fwd[0] / fwd_len;
        let f1 = fwd[1] / fwd_len;
        let f2 = fwd[2] / fwd_len;

        // 3x3 system: [[j00,j01,j02],[j10,j11,j12],[f0,f1,f2]] * [dx,dy,dz] = [sdx,sdy,0]
        let det = j00 * (j11 * f2 - j12 * f1)
            - j01 * (j10 * f2 - j12 * f0)
            + j02 * (j10 * f1 - j11 * f0);

        if det.abs() < 1e-6 {
            return [0.0, 0.0, 0.0];
        }
        let inv = 1.0 / det;

        let sdx = screen_delta.x;
        let sdy = screen_delta.y;

        // Cramer's rule (rhs = [sdx, sdy, 0])
        let dx = inv
            * (sdx * (j11 * f2 - j12 * f1) - j01 * (sdy * f2)
                + j02 * (sdy * f1));
        let dy = inv
            * (j00 * (sdy * f2) - sdx * (j10 * f2 - j12 * f0)
                + j02 * (-(sdy * f0)));
        let dz = inv
            * (j00 * (-(sdy * f1)) - j01 * (-(sdy * f0))
                + sdx * (j10 * f1 - j11 * f0));

        [dx, dy, dz]
    }

    /// Convert vertical screen drag to Z-axis world movement.
    #[allow(dead_code)]
    fn unproject_z_delta(
        &self,
        screen_delta_y: f32,
        world_pos: [f32; 3],
        screen_center: Pos2,
        screen_size: Vec2,
    ) -> f32 {
        let eps = 1.0;
        let Some((s0, _)) =
            self.camera3d.project(world_pos, screen_center, screen_size)
        else {
            return 0.0;
        };
        let elevated = [world_pos[0], world_pos[1], world_pos[2] + eps];
        let Some((sz, _)) =
            self.camera3d.project(elevated, screen_center, screen_size)
        else {
            return 0.0;
        };
        let dsy = sz.y - s0.y; // screen Y change per Z unit
        if dsy.abs() < 1e-6 {
            return 0.0;
        }
        screen_delta_y / dsy
    }

    // -----------------------------------------------------------------------
    // 3D cube face helpers
    // -----------------------------------------------------------------------

    /// Draw depth faces (top strip + side strip) behind a rectangular front face.
    fn draw_depth_faces(
        painter: &egui::Painter,
        screen_rect: Rect,
        extrude: Vec2,
        fill: Color32,
        stroke_width: f32,
        stroke_color: Color32,
    ) {
        let thin = Stroke::new(stroke_width * 0.5, stroke_color);
        let elevated_tl = screen_rect.left_top() + extrude;
        let elevated_tr = screen_rect.right_top() + extrude;
        let elevated_br = screen_rect.right_bottom() + extrude;
        let elevated_bl = screen_rect.left_bottom() + extrude;

        // Top strip: connects front-top edge to elevated-top edge
        painter.add(egui::Shape::convex_polygon(
            vec![
                screen_rect.left_top(),
                screen_rect.right_top(),
                elevated_tr,
                elevated_tl,
            ],
            shade_color(fill, 0.8),
            thin,
        ));

        // Side strip
        if extrude.x > 0.0 {
            // Elevated face is to the right → right side visible
            painter.add(egui::Shape::convex_polygon(
                vec![
                    screen_rect.right_top(),
                    screen_rect.right_bottom(),
                    elevated_br,
                    elevated_tr,
                ],
                shade_color(fill, 0.65),
                thin,
            ));
        } else {
            // Elevated face is to the left → left side visible
            painter.add(egui::Shape::convex_polygon(
                vec![
                    elevated_tl,
                    elevated_bl,
                    screen_rect.left_bottom(),
                    screen_rect.left_top(),
                ],
                shade_color(fill, 0.65),
                thin,
            ));
        }
    }

    // -----------------------------------------------------------------------
    // Main 3D canvas
    // -----------------------------------------------------------------------

    pub(crate) fn draw_canvas_3d(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::all());
        let canvas_rect = response.rect;
        self.canvas_rect = canvas_rect;

        let canvas_bg = Color32::from_rgba_unmultiplied(
            self.canvas_bg[0], self.canvas_bg[1], self.canvas_bg[2], self.canvas_bg[3],
        );
        painter.rect_filled(canvas_rect, CornerRadius::ZERO, canvas_bg);

        let screen_center = canvas_rect.center();
        let screen_size = canvas_rect.size();

        let pointer_pos = response.hover_pos().or_else(|| {
            ui.ctx().input(|i| i.pointer.hover_pos())
        });

        // Scroll => zoom (only when pointer is over the 3D canvas, not the sidebar).
        // Consume both scroll fields via input_mut so they cannot leak to the sidebar.
        if response.hovered() {
            let scroll_delta = ui.ctx().input_mut(|i| {
                let d = if i.raw_scroll_delta.y != 0.0 {
                    i.raw_scroll_delta.y
                } else {
                    i.smooth_scroll_delta.y
                };
                i.raw_scroll_delta    = egui::Vec2::ZERO;
                i.smooth_scroll_delta = egui::Vec2::ZERO;
                d
            });
            if scroll_delta != 0.0 {
                let factor = (1.0 - scroll_delta * 0.003).clamp(0.9, 1.1);
                self.camera3d.distance = (self.camera3d.distance * factor).clamp(100.0, 10000.0);
            }
        }

        // Compute z-layers
        let z_layers = compute_z_layers(&self.document);

        // Build projections
        let mut projections: Vec<NodeProjection> = Vec::new();
        for (i, node) in self.document.nodes.iter().enumerate() {
            let z = z_layers.get(&node.id).copied().unwrap_or(0) as f32 * Z_SPACING
                + node.z_offset;
            let center = node.rect().center();
            let world_pos = [center.x, center.y, z];
            if let Some((screen_pos, depth_scale)) = self.camera3d.project(world_pos, screen_center, screen_size) {
                if canvas_rect.expand(100.0).contains(screen_pos) {
                    let cam_pos = self.camera3d.position();
                    let dx = world_pos[0] - cam_pos[0];
                    let dy = world_pos[1] - cam_pos[1];
                    let dz = world_pos[2] - cam_pos[2];
                    let cam_depth = (dx * dx + dy * dy + dz * dz).sqrt();
                    let scale = depth_scale.clamp(0.15, 3.0);
                    let w = node.size[0] * scale;
                    let h = node.size[1] * scale;
                    let screen_rect = Rect::from_center_size(screen_pos, Vec2::new(w, h));
                    projections.push(NodeProjection {
                        node_id: node.id,
                        node_idx: i,
                        screen_pos,
                        screen_rect,
                        depth_scale,
                        z_layer: z_layers.get(&node.id).copied().unwrap_or(0),
                        cam_depth,
                        z,
                    });
                }
            }
        }

        // Sort back-to-front (painter's algorithm)
        projections.sort_by(|a, b| {
            b.cam_depth
                .partial_cmp(&a.cam_depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // --- Input handling ---
        let middle_button = ui.ctx().input(|i| i.pointer.button_down(egui::PointerButton::Middle));
        let secondary_button = ui
            .ctx()
            .input(|i| i.pointer.button_down(egui::PointerButton::Secondary));
        let is_orbit_modifier = self.space_held || middle_button || secondary_button;

        if response.drag_started() {
            if let Some(mouse) = pointer_pos {
                if is_orbit_modifier {
                    self.drag = DragState::Panning {
                        start_offset: [self.camera3d.yaw, self.camera3d.pitch],
                        start_mouse: mouse,
                    };
                } else if self.tool == Tool::Connect {
                    if let Some(port) = self.hit_test_3d_port(
                        mouse,
                        &projections,
                        screen_center,
                        screen_size,
                    ) {
                        self.drag = DragState::CreatingEdge {
                            source: port,
                            current_screen: mouse,
                        };
                    }
                } else {
                    // Select tool: resize > port > node > orbit
                    if let Some((node_id, handle)) =
                        self.hit_test_3d_resize_handle(mouse, &projections)
                    {
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
                                start_mouse: mouse,
                            };
                        }
                    } else if let Some(port) = self.hit_test_3d_port(
                        mouse,
                        &projections,
                        screen_center,
                        screen_size,
                    ) {
                        self.drag = DragState::CreatingEdge {
                            source: port,
                            current_screen: mouse,
                        };
                    } else if let Some(node_id) =
                        Self::hit_test_3d_node(mouse, &projections)
                    {
                        let cmd_held = ui.ctx().input(|i| {
                            if cfg!(target_os = "macos") {
                                i.modifiers.mac_cmd
                            } else {
                                i.modifiers.ctrl
                            }
                        });
                        if !cmd_held && !self.selection.contains_node(&node_id) {
                            self.selection.clear();
                        }
                        self.selection.node_ids.insert(node_id);

                        let start_positions: Vec<(NodeId, Pos2)> = self
                            .selection
                            .node_ids
                            .iter()
                            .filter_map(|id| {
                                self.document.find_node(id).map(|n| (*id, n.pos()))
                            })
                            .collect();
                        let start_z_offsets: Vec<(NodeId, f32)> = self
                            .selection
                            .node_ids
                            .iter()
                            .filter_map(|id| {
                                self.document.find_node(id).map(|n| (*id, n.z_offset))
                            })
                            .collect();
                        self.drag = DragState::DraggingNode {
                            start_positions,
                            start_z_offsets,
                            start_mouse: mouse,
                        };
                    } else {
                        self.drag = DragState::Panning {
                            start_offset: [self.camera3d.yaw, self.camera3d.pitch],
                            start_mouse: mouse,
                        };
                    }
                }
            }
        }

        if response.dragged() {
            if let Some(mouse) = pointer_pos {
                match &self.drag {
                    DragState::Panning {
                        start_offset,
                        start_mouse,
                    } => {
                        let delta = mouse - *start_mouse;
                        let sensitivity = 0.005;
                        self.camera3d.yaw = start_offset[0] + delta.x * sensitivity;
                        self.camera3d.pitch = (start_offset[1] - delta.y * sensitivity)
                            .clamp(0.1, std::f32::consts::FRAC_PI_2 - 0.05);
                    }
                    DragState::DraggingNode {
                        start_positions,
                        start_z_offsets,
                        start_mouse,
                    } => {
                        let screen_delta = mouse - *start_mouse;

                        // True 3D movement on the camera view plane
                        let (first_id, first_pos) = start_positions
                            .first()
                            .map(|(id, pos)| (*id, *pos))
                            .unwrap_or((NodeId::new(), Pos2::ZERO));
                        let base_z = z_layers
                            .get(&first_id)
                            .map(|&l| l as f32 * Z_SPACING)
                            .unwrap_or(0.0);
                        let start_z_off = start_z_offsets
                            .iter()
                            .find(|(id, _)| *id == first_id)
                            .map(|(_, z)| *z)
                            .unwrap_or(0.0);
                        let world_origin =
                            [first_pos.x, first_pos.y, base_z + start_z_off];
                        let [dx, dy, dz] = self.unproject_drag_3d(
                            screen_delta,
                            world_origin,
                            screen_center,
                            screen_size,
                        );

                        for (id, start_pos) in start_positions {
                            if let Some(node) = self.document.find_node_mut(id) {
                                node.set_pos(Pos2::new(
                                    start_pos.x + dx,
                                    start_pos.y + dy,
                                ));
                            }
                        }
                        for (id, sz) in start_z_offsets {
                            if let Some(node) = self.document.find_node_mut(id) {
                                node.z_offset = *sz + dz;
                            }
                        }
                    }
                    DragState::ResizingNode {
                        node_id,
                        handle,
                        start_rect,
                        start_mouse,
                    } => {
                        let screen_delta = mouse - *start_mouse;
                        let nid = *node_id;
                        let h = *handle;
                        let sr = *start_rect;
                        // Convert screen pixels → world units using the node's
                        // projected scale (screen_size = world_size * depth_scale).
                        // This preserves the correct sign: drag right → delta.x > 0
                        // → right handle grows, left handle contracts — matching
                        // what compute_resize expects. unproject_drag_delta cannot
                        // be used here because it maps screen motion to a world
                        // translation, which has an inverted sign relationship for
                        // the 3D camera (screen-right = world-left in this setup).
                        let world_delta = if let Some(proj) =
                            projections.iter().find(|p| p.node_id == nid)
                        {
                            let screen_w = proj.screen_rect.width().max(1.0);
                            let world_w = sr[2].max(1.0);
                            let scale = screen_w / world_w;
                            Vec2::new(screen_delta.x / scale, screen_delta.y / scale)
                        } else {
                            screen_delta
                        };
                        if let Some(node) = self.document.find_node(&nid) {
                            let min = node.min_size();
                            let [nx, ny, nw, nh] =
                                Self::compute_resize(h, sr, world_delta, min);
                            if let Some(node) = self.document.find_node_mut(&nid) {
                                node.position = [nx, ny];
                                node.size = [nw, nh];
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
                    _ => {}
                }
            }
        }

        if response.drag_stopped() {
            match &self.drag {
                DragState::DraggingNode { .. } | DragState::ResizingNode { .. } => {
                    self.history.push(&self.document);
                }
                DragState::CreatingEdge { source, .. } => {
                    let src = *source;
                    if let Some(mouse) = pointer_pos {
                        if let Some(target) = self.hit_test_3d_port(
                            mouse,
                            &projections,
                            screen_center,
                            screen_size,
                        ) {
                            if src.node_id != target.node_id {
                                let edge = Edge::new(src, target);
                                self.document.edges.push(edge);
                                self.history.push(&self.document);
                            }
                        }
                    }
                }
                _ => {}
            }
            self.drag = DragState::None;
        }

        // Click without drag = select/deselect
        if response.clicked() {
            if let Some(mouse) = pointer_pos {
                let hit = Self::hit_test_3d_node(mouse, &projections);
                let cmd_held = ui.ctx().input(|i| {
                    if cfg!(target_os = "macos") {
                        i.modifiers.mac_cmd
                    } else {
                        i.modifiers.ctrl
                    }
                });
                if let Some(node_id) = hit {
                    if cmd_held {
                        if self.selection.contains_node(&node_id) {
                            self.selection.node_ids.remove(&node_id);
                        } else {
                            self.selection.node_ids.insert(node_id);
                        }
                    } else {
                        self.selection.clear();
                        self.selection.node_ids.insert(node_id);
                    }
                } else if !cmd_held {
                    self.selection.clear();
                }
            }
        }

        // --- Cursor ---
        let hovered_node_id =
            pointer_pos.and_then(|m| Self::hit_test_3d_node(m, &projections));
        let hovering_resize = pointer_pos
            .and_then(|m| self.hit_test_3d_resize_handle(m, &projections))
            .map(|(_, h)| h);
        let hovering_port = pointer_pos
            .and_then(|m| {
                self.hit_test_3d_port(m, &projections, screen_center, screen_size)
            })
            .is_some();

        ui.ctx().set_cursor_icon(match &self.drag {
            DragState::DraggingNode { .. } | DragState::Panning { .. } => {
                egui::CursorIcon::Grabbing
            }
            DragState::ResizingNode { handle, .. } => Self::resize_cursor(*handle),
            DragState::CreatingEdge { .. } => egui::CursorIcon::Crosshair,
            DragState::None => {
                if self.space_held {
                    egui::CursorIcon::Grab
                } else if let Some(handle) = hovering_resize {
                    Self::resize_cursor(handle)
                } else if hovering_port {
                    egui::CursorIcon::Crosshair
                } else if self.tool == Tool::Connect {
                    egui::CursorIcon::Crosshair
                } else if hovered_node_id.is_some() {
                    egui::CursorIcon::PointingHand
                } else if response.hovered() {
                    egui::CursorIcon::Grab
                } else {
                    egui::CursorIcon::Default
                }
            }
            _ => egui::CursorIcon::Default,
        });

        // --- Rendering ---

        // Ground plane grid
        self.draw_ground_plane(&painter, screen_center, screen_size, &z_layers);

        // Edges
        let node_idx_map = self.document.node_index();
        for edge in &self.document.edges {
            self.draw_edge_3d(
                edge,
                &painter,
                &node_idx_map,
                &z_layers,
                screen_center,
                screen_size,
            );
        }

        // Creating edge preview
        if let DragState::CreatingEdge {
            source,
            current_screen,
        } = &self.drag
        {
            if let Some(src_node) = self.document.find_node(&source.node_id) {
                let src_z = z_layers
                    .get(&source.node_id)
                    .copied()
                    .unwrap_or(0) as f32
                    * Z_SPACING
                    + src_node.z_offset;
                let src_port = src_node.port_position(source.side);
                if let Some((src_screen, _)) = self.camera3d.project(
                    [src_port.x, src_port.y, src_z],
                    screen_center,
                    screen_size,
                ) {
                    let offset = 60.0;
                    let (cp1, cp2) = control_points_for_side(
                        src_screen,
                        *current_screen,
                        source.side,
                        offset,
                    );
                    let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                        [src_screen, cp1, cp2, *current_screen],
                        false,
                        Color32::TRANSPARENT,
                        Stroke::new(2.5, self.theme.accent),
                    );
                    painter.add(bezier);
                    painter.circle_filled(*current_screen, 5.0, self.theme.accent);
                }
            }
        }

        // Nodes (back to front) with 3D shapes
        for proj in &projections {
            let node = &self.document.nodes[proj.node_idx];
            let extrude = self.compute_extrude(
                [node.rect().center().x, node.rect().center().y, proj.z],
                CUBE_THICKNESS,
                screen_center,
                screen_size,
            );
            self.draw_node_3d(
                node,
                &painter,
                proj.screen_pos,
                proj.depth_scale,
                proj.z_layer,
                extrude,
            );
        }

        // Ports
        let show_all_ports = self.tool == Tool::Connect
            || matches!(self.drag, DragState::CreatingEdge { .. });
        for proj in &projections {
            let show = show_all_ports
                || self.selection.contains_node(&proj.node_id)
                || hovered_node_id == Some(proj.node_id);
            if !show {
                continue;
            }
            let node = &self.document.nodes[proj.node_idx];
            for side in &ALL_SIDES {
                let port_pos = node.port_position(*side);
                if let Some((screen_port, _)) = self.camera3d.project(
                    [port_pos.x, port_pos.y, proj.z],
                    screen_center,
                    screen_size,
                ) {
                    let r =
                        PORT_RADIUS * proj.depth_scale.sqrt().clamp(0.5, 2.0);
                    let hovered = pointer_pos
                        .map_or(false, |m| (m - screen_port).length() < r * 3.0);
                    if hovered {
                        painter.circle_filled(screen_port, r * 1.3, self.theme.accent);
                        painter.circle_stroke(
                            screen_port,
                            r * 1.3,
                            Stroke::new(2.0, self.theme.text_primary),
                        );
                    } else {
                        painter.circle_filled(screen_port, r, self.theme.port_fill);
                        painter.circle_stroke(
                            screen_port,
                            r,
                            Stroke::new(1.5, self.theme.selection_color),
                        );
                    }
                }
            }
        }

        // Resize handles on selected node
        if self.tool == Tool::Select && self.selection.node_ids.len() == 1 {
            let selected_id = *self.selection.node_ids.iter().next().unwrap();
            if let Some(proj) =
                projections.iter().find(|p| p.node_id == selected_id)
            {
                self.draw_resize_handles(&painter, proj.screen_rect);
            }
        }

        // Instructions
        let instructions = match (&self.tool, &self.drag) {
            (Tool::Connect, _) => {
                "Click port to connect  |  Right-drag orbit  |  Scroll zoom  |  2 for 2D"
            }
            _ if !self.selection.is_empty() => {
                "Drag move  |  Shift+drag Z-axis  |  Handles resize  |  Ports connect  |  Right-drag orbit"
            }
            _ => {
                "Click select  |  Drag move  |  Shift+drag Z-axis  |  Right-drag orbit  |  Scroll zoom"
            }
        };
        painter.text(
            Pos2::new(canvas_rect.center().x, canvas_rect.max.y - 20.0),
            Align2::CENTER_CENTER,
            instructions,
            FontId::proportional(11.0),
            self.theme.text_dim,
        );

        // Status toast
        if let Some((ref msg, time)) = self.status_message {
            let elapsed = time.elapsed().as_secs_f32();
            if elapsed < 2.0 {
                let alpha = ((2.0 - elapsed) * 255.0).min(255.0) as u8;
                let toast_pos =
                    Pos2::new(canvas_rect.center().x, canvas_rect.max.y - 40.0);
                painter.text(
                    toast_pos,
                    Align2::CENTER_CENTER,
                    msg,
                    FontId::proportional(12.0),
                    Color32::from_rgba_premultiplied(
                        self.theme.toast_success.r(),
                        self.theme.toast_success.g(),
                        self.theme.toast_success.b(),
                        alpha,
                    ),
                );
                ui.ctx().request_repaint();
            }
        }
    }

    // -----------------------------------------------------------------------
    // 3D node rendering
    // -----------------------------------------------------------------------

    fn draw_node_3d(
        &self,
        node: &Node,
        painter: &egui::Painter,
        screen_pos: Pos2,
        depth_scale: f32,
        z_layer: i32,
        extrude: Vec2,
    ) {
        let scale = depth_scale.clamp(0.15, 3.0);
        let w = node.size[0] * scale;
        let h = node.size[1] * scale;
        let screen_rect = Rect::from_center_size(screen_pos, Vec2::new(w, h));

        let is_selected = self.selection.contains_node(&node.id);
        let opacity = (scale * 0.8).clamp(0.3, 1.0);
        let alpha = (opacity * 255.0) as u8;

        let layer_tint = match z_layer % 4 {
            0 => [0_i16, 0, 0],
            1 => [0, 0, 15],
            2 => [0, 10, 0],
            3 => [10, 0, 10],
            _ => [0, 0, 0],
        };

        let fill = {
            let c = node.style.fill_color;
            Color32::from_rgba_premultiplied(
                (c[0] as i16 + layer_tint[0]).clamp(0, 255) as u8,
                (c[1] as i16 + layer_tint[1]).clamp(0, 255) as u8,
                (c[2] as i16 + layer_tint[2]).clamp(0, 255) as u8,
                alpha,
            )
        };

        let border_color = if is_selected {
            self.theme.selection_color
        } else {
            let c = node.style.border_color;
            Color32::from_rgba_premultiplied(c[0], c[1], c[2], alpha)
        };
        let border_width = if is_selected {
            (node.style.border_width * scale.sqrt()).max(1.5)
        } else {
            (node.style.border_width * scale.sqrt()).max(0.5)
        };
        let stroke = Stroke::new(border_width, border_color);

        // Selection glow
        if is_selected {
            let glow_rect = screen_rect.expand(4.0 * scale);
            let sc = self.theme.selection_color;
            painter.rect_filled(
                glow_rect,
                CornerRadius::same(6),
                Color32::from_rgba_premultiplied(
                    sc.r(),
                    sc.g(),
                    sc.b(),
                    (50.0 * opacity) as u8,
                ),
            );
        }

        // Shadow
        let shadow_offset = Vec2::new(2.0, 3.0) * scale;
        let shadow_rect = screen_rect.translate(shadow_offset);
        painter.rect_filled(
            shadow_rect,
            CornerRadius::same(4),
            Color32::from_rgba_premultiplied(
                self.theme.shadow_light.r(),
                self.theme.shadow_light.g(),
                self.theme.shadow_light.b(),
                (self.theme.shadow_light.a() as f32 * opacity) as u8,
            ),
        );

        match &node.kind {
            NodeKind::Shape { shape, .. } => match shape {
                NodeShape::Rectangle => {
                    // --- CUBE ---
                    Self::draw_depth_faces(
                        painter,
                        screen_rect,
                        extrude,
                        fill,
                        border_width,
                        border_color,
                    );
                    painter.rect_filled(screen_rect, CornerRadius::ZERO, fill);
                    painter.rect_stroke(
                        screen_rect,
                        CornerRadius::ZERO,
                        stroke,
                        StrokeKind::Outside,
                    );
                }
                NodeShape::RoundedRect => {
                    // --- ROUNDED CUBE ---
                    Self::draw_depth_faces(
                        painter,
                        screen_rect,
                        extrude,
                        fill,
                        border_width,
                        border_color,
                    );
                    let r = (10.0 * scale) as u8;
                    painter.rect_filled(
                        screen_rect,
                        CornerRadius::same(r),
                        fill,
                    );
                    painter.rect_stroke(
                        screen_rect,
                        CornerRadius::same(r),
                        stroke,
                        StrokeKind::Outside,
                    );
                }
                NodeShape::Diamond => {
                    // --- 3D DIAMOND / GEM ---
                    let center = screen_rect.center();
                    let hw = screen_rect.width() / 2.0;
                    let hh = screen_rect.height() / 2.0;
                    let top = Pos2::new(center.x, center.y - hh);
                    let right = Pos2::new(center.x + hw, center.y);
                    let bottom = Pos2::new(center.x, center.y + hh);
                    let left = Pos2::new(center.x - hw, center.y);

                    // Back facets
                    let top_e = top + extrude;
                    let right_e = right + extrude;
                    let bottom_e = bottom + extrude;
                    let left_e = left + extrude;

                    painter.add(egui::Shape::convex_polygon(
                        vec![top, right, right_e, top_e],
                        shade_color(fill, 0.7),
                        Stroke::new(border_width * 0.5, border_color),
                    ));
                    painter.add(egui::Shape::convex_polygon(
                        vec![right, bottom, bottom_e, right_e],
                        shade_color(fill, 0.6),
                        Stroke::new(border_width * 0.5, border_color),
                    ));
                    painter.add(egui::Shape::convex_polygon(
                        vec![bottom, left, left_e, bottom_e],
                        shade_color(fill, 0.75),
                        Stroke::new(border_width * 0.5, border_color),
                    ));
                    painter.add(egui::Shape::convex_polygon(
                        vec![left, top, top_e, left_e],
                        shade_color(fill, 0.8),
                        Stroke::new(border_width * 0.5, border_color),
                    ));

                    // Front diamond face
                    painter.add(egui::Shape::convex_polygon(
                        vec![top, right, bottom, left],
                        fill,
                        stroke,
                    ));
                }
                NodeShape::Circle => {
                    // --- SPHERE ---
                    let radius =
                        screen_rect.width().min(screen_rect.height()) / 2.0;
                    let center = screen_rect.center();

                    // Base sphere
                    painter.circle_filled(
                        center,
                        radius,
                        shade_color(fill, 0.8),
                    );

                    // Lighting gradient simulation
                    let light_dir = Vec2::new(-0.35, -0.55).normalized();
                    for i in 1..=6 {
                        let t = i as f32 / 6.0;
                        let r = radius * (1.0 - t * 0.6);
                        let offset = light_dir * t * radius * 0.35;
                        let highlight = shade_color(fill, 0.85 + t * 0.35);
                        painter.circle_filled(center + offset, r, highlight);
                    }

                    // Specular highlight
                    let spec_offset = light_dir * radius * 0.4;
                    painter.circle_filled(
                        center + spec_offset,
                        radius * 0.1,
                        Color32::from_rgba_premultiplied(
                            255,
                            255,
                            255,
                            (100.0 * opacity) as u8,
                        ),
                    );

                    // Border
                    painter.circle_stroke(center, radius, stroke);
                }
                NodeShape::Parallelogram => {
                    // --- 3D PARALLELOGRAM ---
                    let skew = screen_rect.width() * 0.15;
                    let tl =
                        Pos2::new(screen_rect.min.x + skew, screen_rect.min.y);
                    let tr = Pos2::new(screen_rect.max.x, screen_rect.min.y);
                    let br =
                        Pos2::new(screen_rect.max.x - skew, screen_rect.max.y);
                    let bl = Pos2::new(screen_rect.min.x, screen_rect.max.y);

                    // Top strip
                    painter.add(egui::Shape::convex_polygon(
                        vec![tl, tr, tr + extrude, tl + extrude],
                        shade_color(fill, 0.8),
                        Stroke::new(border_width * 0.5, border_color),
                    ));
                    // Side strip
                    if extrude.x > 0.0 {
                        painter.add(egui::Shape::convex_polygon(
                            vec![tr, br, br + extrude, tr + extrude],
                            shade_color(fill, 0.65),
                            Stroke::new(border_width * 0.5, border_color),
                        ));
                    } else {
                        painter.add(egui::Shape::convex_polygon(
                            vec![tl + extrude, bl + extrude, bl, tl],
                            shade_color(fill, 0.65),
                            Stroke::new(border_width * 0.5, border_color),
                        ));
                    }

                    // Front face
                    painter.add(egui::Shape::convex_polygon(
                        vec![tl, tr, br, bl],
                        fill,
                        stroke,
                    ));
                }
                NodeShape::Connector => {
                    // --- 3D CONNECTOR (extruded tube / capsule) ---
                    let radius = screen_rect.height() / 2.0;
                    let cr = CornerRadius::same(radius as u8);
                    let thin = Stroke::new(border_width * 0.5, border_color);

                    // Back pill (depth face — the "far" end of the tube)
                    let back_rect = screen_rect.translate(extrude);
                    painter.rect_filled(
                        back_rect,
                        cr,
                        shade_color(fill, 0.45),
                    );

                    // Top strip (connects front-top to back-top edge)
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            screen_rect.left_top(),
                            screen_rect.right_top(),
                            back_rect.right_top(),
                            back_rect.left_top(),
                        ],
                        shade_color(fill, 0.75),
                        thin,
                    ));

                    // Side strip (right or left depending on camera angle)
                    if extrude.x > 0.0 {
                        painter.add(egui::Shape::convex_polygon(
                            vec![
                                screen_rect.right_top(),
                                screen_rect.right_bottom(),
                                back_rect.right_bottom(),
                                back_rect.right_top(),
                            ],
                            shade_color(fill, 0.55),
                            thin,
                        ));
                    } else {
                        painter.add(egui::Shape::convex_polygon(
                            vec![
                                back_rect.left_top(),
                                back_rect.left_bottom(),
                                screen_rect.left_bottom(),
                                screen_rect.left_top(),
                            ],
                            shade_color(fill, 0.55),
                            thin,
                        ));
                    }

                    // Bottom strip
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            screen_rect.left_bottom(),
                            screen_rect.right_bottom(),
                            back_rect.right_bottom(),
                            back_rect.left_bottom(),
                        ],
                        shade_color(fill, 0.60),
                        thin,
                    ));

                    // Front pill face (semi-transparent)
                    let connector_fill = Color32::from_rgba_unmultiplied(
                        fill.r(), fill.g(), fill.b(),
                        (fill.a() as f32 * 0.85) as u8,
                    );
                    painter.rect_filled(screen_rect, cr, connector_fill);
                    painter.rect_stroke(screen_rect, cr, stroke, StrokeKind::Outside);

                    // Small diamond accent on the left cap
                    let diamond_size = 5.0 * scale;
                    let left_center = Pos2::new(
                        screen_rect.min.x - diamond_size * 0.5,
                        screen_rect.center().y,
                    );
                    let diamond_pts = vec![
                        Pos2::new(left_center.x, left_center.y - diamond_size),
                        Pos2::new(left_center.x + diamond_size, left_center.y),
                        Pos2::new(left_center.x, left_center.y + diamond_size),
                        Pos2::new(left_center.x - diamond_size, left_center.y),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        diamond_pts,
                        border_color,
                        Stroke::NONE,
                    ));
                }
                NodeShape::Hexagon => {
                    // 3D Hexagon — extruded flat-top hex
                    let cx = screen_rect.center().x;
                    let cy = screen_rect.center().y;
                    let hw = screen_rect.width() / 2.0;
                    let hh = screen_rect.height() / 2.0;
                    let inset = hw * 0.3;
                    let hex_pts = |dx: f32, dy: f32| vec![
                        Pos2::new(cx - hw + dx,    cy + dy),
                        Pos2::new(cx - inset + dx, cy - hh + dy),
                        Pos2::new(cx + inset + dx, cy - hh + dy),
                        Pos2::new(cx + hw + dx,    cy + dy),
                        Pos2::new(cx + inset + dx, cy + hh + dy),
                        Pos2::new(cx - inset + dx, cy + hh + dy),
                    ];
                    // Back face
                    painter.add(egui::Shape::convex_polygon(hex_pts(extrude.x, extrude.y), shade_color(fill, 0.55), Stroke::NONE));
                    // Front face
                    painter.add(egui::Shape::convex_polygon(hex_pts(0.0, 0.0), fill, stroke));
                }
            },
            NodeKind::StickyNote { .. } => {
                // --- 3D STICKY NOTE ---
                let corner = CornerRadius::same((8.0 * scale) as u8);
                Self::draw_depth_faces(
                    painter,
                    screen_rect,
                    extrude,
                    shade_color(fill, 0.9),
                    0.5,
                    shade_color(fill, 0.5),
                );
                painter.rect_filled(screen_rect, corner, fill);
                if is_selected {
                    painter.rect_stroke(
                        screen_rect,
                        corner,
                        Stroke::new(2.0 * scale.sqrt(), self.theme.selection_color),
                        StrokeKind::Outside,
                    );
                }
            }
            NodeKind::Entity { .. } => {
                // --- 3D ENTITY BOX ---
                Self::draw_depth_faces(
                    painter,
                    screen_rect,
                    extrude,
                    fill,
                    border_width,
                    border_color,
                );
                painter.rect_filled(
                    screen_rect,
                    CornerRadius::same(3),
                    fill,
                );
                painter.rect_stroke(
                    screen_rect,
                    CornerRadius::same(3),
                    stroke,
                    StrokeKind::Outside,
                );
                let header_h = ENTITY_HEADER_HEIGHT * scale;
                let header_rect = Rect::from_min_size(
                    screen_rect.min,
                    Vec2::new(
                        screen_rect.width(),
                        header_h.min(screen_rect.height()),
                    ),
                );
                painter.rect_filled(
                    header_rect,
                    CornerRadius {
                        nw: 3,
                        ne: 3,
                        sw: 0,
                        se: 0,
                    },
                    border_color,
                );
            }
            NodeKind::Text { .. } => {
                if is_selected {
                    let sc = self.theme.selection_color;
                    painter.rect_stroke(
                        screen_rect,
                        CornerRadius::same(2),
                        Stroke::new(
                            1.0,
                            Color32::from_rgba_premultiplied(
                                sc.r(),
                                sc.g(),
                                sc.b(),
                                (100.0 * opacity) as u8,
                            ),
                        ),
                        StrokeKind::Outside,
                    );
                }
            }
        }

        // Label (billboard)
        let label_text = node.display_label();
        let font_size = (node.style.font_size * scale).max(7.0).min(24.0);
        if font_size >= 7.0 && !label_text.is_empty() {
            let text_color = {
                let c = node.style.text_color;
                Color32::from_rgba_premultiplied(c[0], c[1], c[2], alpha)
            };
            let display_text = if label_text.len() > 20 && font_size < 10.0 {
                &label_text[..18]
            } else {
                label_text
            };
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                display_text,
                FontId::proportional(font_size),
                text_color,
            );
        }

        // z-layer badge
        if z_layer > 0 && scale > 0.3 {
            let badge_pos = Pos2::new(
                screen_rect.max.x - 4.0 * scale,
                screen_rect.min.y + 4.0 * scale,
            );
            let badge_size = (8.0 * scale).max(6.0);
            let ac = self.theme.accent;
            painter.text(
                badge_pos,
                Align2::RIGHT_TOP,
                format!("z{}", z_layer),
                FontId::monospace(badge_size),
                Color32::from_rgba_premultiplied(
                    ac.r(),
                    ac.g(),
                    ac.b(),
                    (180.0 * opacity) as u8,
                ),
            );
        }
    }

    // -----------------------------------------------------------------------
    // Edge drawing (unchanged)
    // -----------------------------------------------------------------------

    fn draw_edge_3d(
        &self,
        edge: &Edge,
        painter: &egui::Painter,
        node_idx: &std::collections::HashMap<NodeId, usize>,
        z_layers: &std::collections::HashMap<NodeId, i32>,
        screen_center: Pos2,
        screen_size: Vec2,
    ) {
        let src_node = node_idx
            .get(&edge.source.node_id)
            .and_then(|&i| self.document.nodes.get(i));
        let tgt_node = node_idx
            .get(&edge.target.node_id)
            .and_then(|&i| self.document.nodes.get(i));
        let (src_node, tgt_node) = match (src_node, tgt_node) {
            (Some(s), Some(t)) => (s, t),
            _ => return,
        };

        let src_z = z_layers
            .get(&edge.source.node_id)
            .copied()
            .unwrap_or(0) as f32
            * Z_SPACING
            + src_node.z_offset;
        let tgt_z = z_layers
            .get(&edge.target.node_id)
            .copied()
            .unwrap_or(0) as f32
            * Z_SPACING
            + tgt_node.z_offset;

        let src_port = src_node.port_position(edge.source.side);
        let tgt_port = tgt_node.port_position(edge.target.side);

        let src_proj = self.camera3d.project(
            [src_port.x, src_port.y, src_z],
            screen_center,
            screen_size,
        );
        let tgt_proj = self.camera3d.project(
            [tgt_port.x, tgt_port.y, tgt_z],
            screen_center,
            screen_size,
        );

        let (src_screen, src_scale) = match src_proj {
            Some(v) => v,
            None => return,
        };
        let (tgt_screen, tgt_scale) = match tgt_proj {
            Some(v) => v,
            None => return,
        };

        let is_selected = self.selection.contains_edge(&edge.id);
        let avg_scale = (src_scale + tgt_scale) / 2.0;
        let opacity = (avg_scale * 0.8).clamp(0.2, 1.0);
        let alpha = (opacity * 255.0) as u8;

        let edge_color = if is_selected {
            self.theme.selection_color
        } else {
            let c = edge.style.color;
            Color32::from_rgba_premultiplied(c[0], c[1], c[2], alpha)
        };
        let width = (edge.style.width * avg_scale.sqrt()).max(0.5).min(5.0);

        let offset = 60.0 * avg_scale;
        let (cp1, cp2) = control_points_for_side(
            src_screen,
            tgt_screen,
            edge.source.side,
            offset,
        );

        // Glow halo (drawn first, behind edge)
        if edge.style.glow && !is_selected {
            let glow_color = Color32::from_rgba_premultiplied(
                edge.style.color[0], edge.style.color[1], edge.style.color[2],
                (alpha as f32 * 0.24) as u8,
            );
            let glow = egui::epaint::CubicBezierShape::from_points_stroke(
                [src_screen, cp1, cp2, tgt_screen], false, Color32::TRANSPARENT,
                Stroke::new(width + 8.0, glow_color),
            );
            painter.add(glow);
        }

        // Main edge stroke
        if edge.style.dashed || edge.style.animated {
            // Dashed/animated: sample bezier and draw alternating segments
            let dash = 8.0 * avg_scale.sqrt();
            let gap = 5.0 * avg_scale.sqrt();
            let steps = 60usize;
            let pts: Vec<Pos2> = (0..=steps)
                .map(|i| {
                    let t = i as f32 / steps as f32;
                    cubic_bezier_point(src_screen, cp1, cp2, tgt_screen, t)
                })
                .collect();
            // Animated offset
            let time_offset = if edge.style.animated {
                let t = painter.ctx().input(|i| i.time) as f32;
                painter.ctx().request_repaint_after(std::time::Duration::from_millis(33));
                (t * 40.0) % (dash + gap)
            } else { 0.0 };

            let mut dist = -time_offset;
            let mut drawing = true;
            let mut seg_start = pts[0];
            for i in 1..pts.len() {
                let d = (pts[i] - pts[i - 1]).length();
                dist += d;
                let threshold = if drawing { dash } else { gap };
                if dist >= threshold {
                    if drawing {
                        painter.line_segment([seg_start, pts[i]], Stroke::new(width, edge_color));
                    }
                    seg_start = pts[i];
                    dist = 0.0;
                    drawing = !drawing;
                }
            }
            if drawing {
                painter.line_segment([seg_start, *pts.last().unwrap()], Stroke::new(width, edge_color));
            }
        } else {
            let bezier = egui::epaint::CubicBezierShape::from_points_stroke(
                [src_screen, cp1, cp2, tgt_screen],
                false,
                Color32::TRANSPARENT,
                Stroke::new(width, edge_color),
            );
            painter.add(bezier);
        }

        // Arrow head (respects ArrowHead variant)
        let dir = (tgt_screen - cp2).normalized();
        if dir.length() > 0.01 {
            let arrow_len = 8.0 * avg_scale.sqrt();
            let arrow_width = 5.0 * avg_scale.sqrt();
            let perp = Vec2::new(-dir.y, dir.x);
            let tip = tgt_screen;
            let left = tip - dir * arrow_len + perp * arrow_width;
            let right = tip - dir * arrow_len - perp * arrow_width;
            match edge.style.arrow_head {
                ArrowHead::Filled => {
                    painter.add(egui::Shape::convex_polygon(
                        vec![tip, left, right],
                        edge_color,
                        Stroke::new(width * 0.5, edge_color),
                    ));
                }
                ArrowHead::Open => {
                    painter.line_segment([left, tip], Stroke::new(width, edge_color));
                    painter.line_segment([right, tip], Stroke::new(width, edge_color));
                }
                ArrowHead::Circle => {
                    let center = tip - dir * arrow_len * 0.5;
                    painter.circle_filled(center, arrow_width * 0.8, edge_color);
                }
                ArrowHead::None => {} // no arrow
            }
        }

        // Edge label
        if !edge.label.is_empty() {
            let mid =
                cubic_bezier_point(src_screen, cp1, cp2, tgt_screen, 0.5);
            let font_size = (12.0 * avg_scale).clamp(7.0, 16.0);
            if font_size >= 7.0 {
                let text_color = self.theme.text_primary.gamma_multiply(opacity);
                // Label background pill
                let galley = painter.layout_no_wrap(
                    edge.label.clone(),
                    FontId::proportional(font_size),
                    text_color,
                );
                let text_rect = egui::Rect::from_center_size(mid, galley.size()).expand2(Vec2::new(4.0, 2.0));
                painter.rect_filled(text_rect, egui::CornerRadius::same(4), self.theme.edge_label_bg);
                painter.text(
                    mid,
                    Align2::CENTER_CENTER,
                    &edge.label,
                    FontId::proportional(font_size),
                    text_color,
                );
            }
        }

        // Source/target endpoint labels (3D)
        let ep_font_size = (10.0 * avg_scale).clamp(6.0, 13.0);
        if ep_font_size >= 6.0 {
            let text_col = self.theme.text_secondary.gamma_multiply(opacity);
            if !edge.source_label.is_empty() {
                let near_src = cubic_bezier_point(src_screen, cp1, cp2, tgt_screen, 0.08);
                painter.text(near_src + Vec2::new(0.0, -8.0 * avg_scale), Align2::CENTER_BOTTOM,
                    &edge.source_label, FontId::proportional(ep_font_size), text_col);
            }
            if !edge.target_label.is_empty() {
                let near_tgt = cubic_bezier_point(src_screen, cp1, cp2, tgt_screen, 0.92);
                painter.text(near_tgt + Vec2::new(0.0, -8.0 * avg_scale), Align2::CENTER_BOTTOM,
                    &edge.target_label, FontId::proportional(ep_font_size), text_col);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Ground plane (unchanged)
    // -----------------------------------------------------------------------

    fn draw_ground_plane(
        &self,
        painter: &egui::Painter,
        screen_center: Pos2,
        screen_size: Vec2,
        z_layers: &std::collections::HashMap<NodeId, i32>,
    ) {
        let max_layer = z_layers.values().copied().max().unwrap_or(0);

        let grid_step = 100.0;
        let grid_range = 800.0;
        let target = self.camera3d.target;

        let gx_start =
            ((target[0] - grid_range) / grid_step).floor() as i32;
        let gx_end = ((target[0] + grid_range) / grid_step).ceil() as i32;
        let gy_start =
            ((target[1] - grid_range) / grid_step).floor() as i32;
        let gy_end = ((target[1] + grid_range) / grid_step).ceil() as i32;

        for i in gy_start..=gy_end {
            let y = i as f32 * grid_step;
            let p1 = [gx_start as f32 * grid_step, y, 0.0];
            let p2 = [gx_end as f32 * grid_step, y, 0.0];
            if let (Some((s1, _)), Some((s2, _))) = (
                self.camera3d
                    .project(p1, screen_center, screen_size),
                self.camera3d
                    .project(p2, screen_center, screen_size),
            ) {
                painter.line_segment(
                    [s1, s2],
                    Stroke::new(0.5, self.theme.grid_color),
                );
            }
        }

        for i in gx_start..=gx_end {
            let x = i as f32 * grid_step;
            let p1 = [x, gy_start as f32 * grid_step, 0.0];
            let p2 = [x, gy_end as f32 * grid_step, 0.0];
            if let (Some((s1, _)), Some((s2, _))) = (
                self.camera3d
                    .project(p1, screen_center, screen_size),
                self.camera3d
                    .project(p2, screen_center, screen_size),
            ) {
                painter.line_segment(
                    [s1, s2],
                    Stroke::new(0.5, self.theme.grid_color),
                );
            }
        }

        for layer in 1..=max_layer {
            let z = layer as f32 * Z_SPACING;
            let node_count = z_layers.values().filter(|&&l| l == layer).count();

            // Draw a subtle transparent grid plane for each non-zero layer
            // to help visualize the 3D separation.
            if node_count > 0 {
                let plane_alpha = 8u8; // very transparent
                let plane_color = self.theme.accent.gamma_multiply(plane_alpha as f32 / 255.0);
                let edge_alpha = 20u8;
                let edge_color = self.theme.accent.gamma_multiply(edge_alpha as f32 / 255.0);

                // Draw 4 grid lines as a bounding frame on this plane
                let plane_corners = [
                    [target[0] - grid_range * 0.7, target[1] - grid_range * 0.7, z],
                    [target[0] + grid_range * 0.7, target[1] - grid_range * 0.7, z],
                    [target[0] + grid_range * 0.7, target[1] + grid_range * 0.7, z],
                    [target[0] - grid_range * 0.7, target[1] + grid_range * 0.7, z],
                ];
                let projected_corners: Vec<Option<Pos2>> = plane_corners.iter()
                    .map(|&p| self.camera3d.project(p, screen_center, screen_size).map(|(s, _)| s))
                    .collect();
                // Draw frame lines
                for i in 0..4 {
                    let next = (i + 1) % 4;
                    if let (Some(a), Some(b)) = (projected_corners[i], projected_corners[next]) {
                        painter.line_segment([a, b], Stroke::new(0.5, edge_color));
                    }
                }
                let _ = plane_color; // used conceptually, polygon fill not shown for performance
            }

            let label_pos = [target[0] - grid_range, target[1] - grid_range, z];
            if let Some((screen_pos, _)) = self
                .camera3d
                .project(label_pos, screen_center, screen_size)
            {
                let layer_name = self.document.layer_names.get(&layer)
                    .map(|s| format!("  {}", s))
                    .unwrap_or_default();
                let lbl = if node_count > 0 {
                    format!("z={:.0}  ×{}{}", z, node_count, layer_name)
                } else {
                    format!("z={:.0}{}", z, layer_name)
                };
                painter.text(
                    screen_pos,
                    Align2::LEFT_CENTER,
                    lbl,
                    FontId::proportional(9.0),
                    self.theme.accent.gamma_multiply(0.28),
                );
            }
        }

        // Origin axis indicators — short colored lines at the world origin
        let origin = [target[0], target[1], 0.0];
        let axis_len = 60.0;
        let x_end = [origin[0] + axis_len, origin[1], 0.0];
        let y_end = [origin[0], origin[1] + axis_len, 0.0];
        let z_end = [origin[0], origin[1], axis_len];

        if let Some((o_s, _)) = self.camera3d.project(origin, screen_center, screen_size) {
            // X axis — red
            if let Some((x_s, _)) = self.camera3d.project(x_end, screen_center, screen_size) {
                painter.line_segment([o_s, x_s], Stroke::new(1.5, Color32::from_rgba_premultiplied(220, 80, 80, 120)));
                painter.text(x_s, Align2::LEFT_CENTER, "X", FontId::proportional(8.0), Color32::from_rgba_premultiplied(220, 80, 80, 100));
            }
            // Y axis — green
            if let Some((y_s, _)) = self.camera3d.project(y_end, screen_center, screen_size) {
                painter.line_segment([o_s, y_s], Stroke::new(1.5, Color32::from_rgba_premultiplied(80, 200, 80, 120)));
                painter.text(y_s, Align2::LEFT_CENTER, "Y", FontId::proportional(8.0), Color32::from_rgba_premultiplied(80, 200, 80, 100));
            }
            // Z axis — blue (accent)
            if let Some((z_s, _)) = self.camera3d.project(z_end, screen_center, screen_size) {
                painter.line_segment([o_s, z_s], Stroke::new(1.5, self.theme.accent.gamma_multiply(0.47)));
                painter.text(z_s, Align2::LEFT_CENTER, "Z", FontId::proportional(8.0), self.theme.accent.gamma_multiply(0.39));
            }
        }
    }
}
