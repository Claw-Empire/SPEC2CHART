use egui::Pos2;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub Uuid);

impl EdgeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeShape {
    Rectangle,
    RoundedRect,
    Diamond,
    Circle,
    Parallelogram,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortSide {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStyle {
    pub fill_color: [u8; 4],
    pub border_color: [u8; 4],
    pub border_width: f32,
    pub text_color: [u8; 4],
    pub font_size: f32,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            fill_color: [49, 50, 68, 255],     // surface0
            border_color: [69, 71, 90, 255],    // surface1
            border_width: 1.5,
            text_color: [205, 214, 244, 255],   // text
            font_size: 13.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub shape: NodeShape,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub label: String,
    pub description: String,
    pub style: NodeStyle,
}

impl Node {
    pub fn new(shape: NodeShape, position: Pos2) -> Self {
        let size = match shape {
            NodeShape::Circle => [80.0, 80.0],
            NodeShape::Diamond => [120.0, 100.0],
            _ => [140.0, 60.0],
        };
        Self {
            id: NodeId::new(),
            shape,
            position: [position.x, position.y],
            size,
            label: String::from("New Node"),
            description: String::new(),
            style: NodeStyle::default(),
        }
    }

    pub fn pos(&self) -> Pos2 {
        Pos2::new(self.position[0], self.position[1])
    }

    pub fn set_pos(&mut self, pos: Pos2) {
        self.position = [pos.x, pos.y];
    }

    pub fn size_vec(&self) -> egui::Vec2 {
        egui::Vec2::new(self.size[0], self.size[1])
    }

    pub fn rect(&self) -> egui::Rect {
        egui::Rect::from_min_size(self.pos(), self.size_vec())
    }

    pub fn port_position(&self, side: PortSide) -> Pos2 {
        let rect = self.rect();
        match side {
            PortSide::Top => rect.center_top(),
            PortSide::Bottom => rect.center_bottom(),
            PortSide::Left => rect.left_center(),
            PortSide::Right => rect.right_center(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub node_id: NodeId,
    pub side: PortSide,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeStyle {
    pub color: [u8; 4],
    pub width: f32,
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self {
            color: [100, 100, 100, 255],
            width: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub source: Port,
    pub target: Port,
    pub label: String,
    pub style: EdgeStyle,
}

impl Edge {
    pub fn new(source: Port, target: Port) -> Self {
        Self {
            id: EdgeId::new(),
            source,
            target,
            label: String::new(),
            style: EdgeStyle::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub offset: [f32; 2],
    pub zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            zoom: 1.0,
        }
    }
}

impl Viewport {
    pub fn screen_to_canvas(&self, screen_pos: Pos2) -> Pos2 {
        Pos2::new(
            (screen_pos.x - self.offset[0]) / self.zoom,
            (screen_pos.y - self.offset[1]) / self.zoom,
        )
    }

    pub fn canvas_to_screen(&self, canvas_pos: Pos2) -> Pos2 {
        Pos2::new(
            canvas_pos.x * self.zoom + self.offset[0],
            canvas_pos.y * self.zoom + self.offset[1],
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Selection {
    pub node_ids: Vec<NodeId>,
    pub edge_ids: Vec<EdgeId>,
}

impl Selection {
    pub fn clear(&mut self) {
        self.node_ids.clear();
        self.edge_ids.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.node_ids.is_empty() && self.edge_ids.is_empty()
    }

    pub fn contains_node(&self, id: &NodeId) -> bool {
        self.node_ids.contains(id)
    }

    pub fn contains_edge(&self, id: &EdgeId) -> bool {
        self.edge_ids.contains(id)
    }

    pub fn toggle_node(&mut self, id: NodeId) {
        if let Some(pos) = self.node_ids.iter().position(|n| *n == id) {
            self.node_ids.remove(pos);
        } else {
            self.node_ids.push(id);
        }
    }

    pub fn select_node(&mut self, id: NodeId) {
        self.clear();
        self.node_ids.push(id);
    }

    pub fn select_edge(&mut self, id: EdgeId) {
        self.clear();
        self.edge_ids.push(id);
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlowchartDocument {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl FlowchartDocument {
    pub fn find_node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == *id)
    }

    pub fn find_node_mut(&mut self, id: &NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == *id)
    }

    pub fn find_edge(&self, id: &EdgeId) -> Option<&Edge> {
        self.edges.iter().find(|e| e.id == *id)
    }

    pub fn find_edge_mut(&mut self, id: &EdgeId) -> Option<&mut Edge> {
        self.edges.iter_mut().find(|e| e.id == *id)
    }

    pub fn node_at_pos(&self, pos: Pos2) -> Option<NodeId> {
        for node in self.nodes.iter().rev() {
            if node.rect().contains(pos) {
                return Some(node.id);
            }
        }
        None
    }

    pub fn remove_node(&mut self, id: &NodeId) {
        self.edges
            .retain(|e| e.source.node_id != *id && e.target.node_id != *id);
        self.nodes.retain(|n| n.id != *id);
    }

    pub fn remove_edge(&mut self, id: &EdgeId) {
        self.edges.retain(|e| e.id != *id);
    }
}
