use egui::{Pos2, Vec2};
use crate::model::*;
use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Camera3D — perspective projection for 3D overview
// ---------------------------------------------------------------------------

pub(crate) struct Camera3D {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: [f32; 3],
    pub fov: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            yaw: -0.4,
            pitch: 0.6,
            distance: 1200.0,
            target: [0.0, 0.0, 0.0],
            fov: std::f32::consts::FRAC_PI_4,
        }
    }
}

impl Camera3D {
    /// Camera position derived from orbit angles and distance.
    pub fn position(&self) -> [f32; 3] {
        let cos_pitch = self.pitch.cos();
        [
            self.target[0] + self.distance * cos_pitch * self.yaw.sin(),
            self.target[1] - self.distance * self.pitch.sin(),
            self.target[2] + self.distance * cos_pitch * self.yaw.cos(),
        ]
    }

    /// Project a 3D world point to 2D screen coordinates.
    /// Returns None if the point is behind the camera.
    pub fn project(&self, point: [f32; 3], screen_center: Pos2, screen_size: Vec2) -> Option<(Pos2, f32)> {
        let pos = self.position();

        let fwd = [
            self.target[0] - pos[0],
            self.target[1] - pos[1],
            self.target[2] - pos[2],
        ];
        let fwd_len = (fwd[0] * fwd[0] + fwd[1] * fwd[1] + fwd[2] * fwd[2]).sqrt();
        if fwd_len < 0.001 {
            return None;
        }
        let fwd = [fwd[0] / fwd_len, fwd[1] / fwd_len, fwd[2] / fwd_len];

        let world_up = [0.0_f32, -1.0, 0.0];

        let right = [
            fwd[1] * world_up[2] - fwd[2] * world_up[1],
            fwd[2] * world_up[0] - fwd[0] * world_up[2],
            fwd[0] * world_up[1] - fwd[1] * world_up[0],
        ];
        let right_len = (right[0] * right[0] + right[1] * right[1] + right[2] * right[2]).sqrt();
        if right_len < 0.001 {
            return None;
        }
        let right = [right[0] / right_len, right[1] / right_len, right[2] / right_len];

        let up = [
            right[1] * fwd[2] - right[2] * fwd[1],
            right[2] * fwd[0] - right[0] * fwd[2],
            right[0] * fwd[1] - right[1] * fwd[0],
        ];

        let d = [point[0] - pos[0], point[1] - pos[1], point[2] - pos[2]];

        let cam_x = d[0] * right[0] + d[1] * right[1] + d[2] * right[2];
        let cam_y = d[0] * up[0] + d[1] * up[1] + d[2] * up[2];
        let cam_z = d[0] * fwd[0] + d[1] * fwd[1] + d[2] * fwd[2];

        if cam_z < 1.0 {
            return None;
        }

        let aspect = screen_size.x / screen_size.y;
        let half_fov_tan = (self.fov / 2.0).tan();

        let ndc_x = cam_x / (cam_z * half_fov_tan * aspect);
        let ndc_y = cam_y / (cam_z * half_fov_tan);

        let screen_x = screen_center.x + ndc_x * screen_size.x / 2.0;
        let screen_y = screen_center.y + ndc_y * screen_size.y / 2.0;

        let depth_scale = self.distance / cam_z;

        Some((Pos2::new(screen_x, screen_y), depth_scale))
    }
}

/// Compute z-layer depths for nodes using BFS from root nodes.
pub(crate) fn compute_z_layers(doc: &FlowchartDocument) -> HashMap<NodeId, i32> {
    let mut has_incoming: HashSet<NodeId> = HashSet::new();
    let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for node in &doc.nodes {
        adjacency.entry(node.id).or_default();
    }
    for edge in &doc.edges {
        has_incoming.insert(edge.target.node_id);
        adjacency.entry(edge.source.node_id).or_default().push(edge.target.node_id);
    }

    let mut depths: HashMap<NodeId, i32> = HashMap::new();
    let mut queue: VecDeque<NodeId> = VecDeque::new();

    for node in &doc.nodes {
        if !has_incoming.contains(&node.id) {
            depths.insert(node.id, 0);
            queue.push_back(node.id);
        }
    }

    if queue.is_empty() {
        if let Some(node) = doc.nodes.first() {
            depths.insert(node.id, 0);
            queue.push_back(node.id);
        }
    }

    while let Some(nid) = queue.pop_front() {
        let current_depth = depths[&nid];
        if let Some(neighbors) = adjacency.get(&nid) {
            for &neighbor in neighbors {
                if !depths.contains_key(&neighbor) {
                    depths.insert(neighbor, current_depth + 1);
                    queue.push_back(neighbor);
                }
            }
        }
    }

    for node in &doc.nodes {
        depths.entry(node.id).or_insert(0);
    }

    depths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera3d_default_position() {
        let cam = Camera3D::default();
        let pos = cam.position();
        assert!(pos[1] < 0.0, "camera should be above target (y is down)");
        assert!(pos[2] > 0.0, "camera should be in front of target");
    }

    #[test]
    fn camera3d_projects_origin_to_center() {
        let cam = Camera3D {
            yaw: 0.0,
            pitch: 0.0,
            distance: 500.0,
            target: [0.0, 0.0, 0.0],
            fov: std::f32::consts::FRAC_PI_4,
        };
        let screen_center = Pos2::new(400.0, 300.0);
        let screen_size = Vec2::new(800.0, 600.0);
        let result = cam.project([0.0, 0.0, 0.0], screen_center, screen_size);
        assert!(result.is_some(), "target should be visible");
        let (pos, _scale) = result.unwrap();
        assert!((pos.x - 400.0).abs() < 5.0, "x should be near center: {}", pos.x);
        assert!((pos.y - 300.0).abs() < 5.0, "y should be near center: {}", pos.y);
    }

    #[test]
    fn camera3d_farther_objects_are_smaller() {
        let cam = Camera3D {
            yaw: 0.0,
            pitch: 0.3,
            distance: 500.0,
            target: [0.0, 0.0, 0.0],
            fov: std::f32::consts::FRAC_PI_4,
        };
        let screen_center = Pos2::new(400.0, 300.0);
        let screen_size = Vec2::new(800.0, 600.0);
        let near = cam.project([0.0, 0.0, 0.0], screen_center, screen_size);
        let far = cam.project([0.0, 0.0, -300.0], screen_center, screen_size);
        assert!(near.is_some() && far.is_some());
        let (_, near_scale) = near.unwrap();
        let (_, far_scale) = far.unwrap();
        assert!(near_scale > far_scale, "near objects should have larger scale: near={} far={}", near_scale, far_scale);
    }

    #[test]
    fn camera3d_behind_camera_returns_none() {
        let cam = Camera3D {
            yaw: 0.0,
            pitch: 0.0,
            distance: 500.0,
            target: [0.0, 0.0, 0.0],
            fov: std::f32::consts::FRAC_PI_4,
        };
        let screen_center = Pos2::new(400.0, 300.0);
        let screen_size = Vec2::new(800.0, 600.0);
        let result = cam.project([0.0, 0.0, 600.0], screen_center, screen_size);
        assert!(result.is_none(), "point behind camera should return None");
    }

    #[test]
    fn z_layers_empty_doc() {
        let doc = FlowchartDocument::default();
        let layers = compute_z_layers(&doc);
        assert!(layers.is_empty());
    }

    #[test]
    fn z_layers_single_node() {
        let mut doc = FlowchartDocument::default();
        let node = Node::new(NodeShape::Rectangle, Pos2::new(0.0, 0.0));
        let nid = node.id;
        doc.nodes.push(node);
        let layers = compute_z_layers(&doc);
        assert_eq!(layers[&nid], 0);
    }

    #[test]
    fn z_layers_linear_chain() {
        let mut doc = FlowchartDocument::default();
        let a = Node::new(NodeShape::Rectangle, Pos2::new(0.0, 0.0));
        let b = Node::new(NodeShape::Rectangle, Pos2::new(200.0, 0.0));
        let c = Node::new(NodeShape::Rectangle, Pos2::new(400.0, 0.0));
        let a_id = a.id;
        let b_id = b.id;
        let c_id = c.id;
        let edge_ab = Edge::new(
            Port { node_id: a_id, side: PortSide::Right },
            Port { node_id: b_id, side: PortSide::Left },
        );
        let edge_bc = Edge::new(
            Port { node_id: b_id, side: PortSide::Right },
            Port { node_id: c_id, side: PortSide::Left },
        );
        doc.nodes.push(a);
        doc.nodes.push(b);
        doc.nodes.push(c);
        doc.edges.push(edge_ab);
        doc.edges.push(edge_bc);
        let layers = compute_z_layers(&doc);
        assert_eq!(layers[&a_id], 0);
        assert_eq!(layers[&b_id], 1);
        assert_eq!(layers[&c_id], 2);
    }

    #[test]
    fn z_layers_disconnected_nodes() {
        let mut doc = FlowchartDocument::default();
        let a = Node::new(NodeShape::Rectangle, Pos2::new(0.0, 0.0));
        let b = Node::new(NodeShape::Rectangle, Pos2::new(200.0, 0.0));
        let a_id = a.id;
        let b_id = b.id;
        doc.nodes.push(a);
        doc.nodes.push(b);
        let layers = compute_z_layers(&doc);
        assert_eq!(layers[&a_id], 0);
        assert_eq!(layers[&b_id], 0);
    }
}
