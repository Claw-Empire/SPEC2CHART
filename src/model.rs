use std::collections::{HashMap, HashSet};

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
    Connector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StickyColor {
    Yellow,
    Pink,
    Green,
    Blue,
    Purple,
}

impl StickyColor {
    pub const ALL: [StickyColor; 5] = [
        StickyColor::Yellow,
        StickyColor::Pink,
        StickyColor::Green,
        StickyColor::Blue,
        StickyColor::Purple,
    ];

    pub fn fill_rgba(&self) -> [u8; 4] {
        match self {
            StickyColor::Yellow => [249, 226, 175, 255],
            StickyColor::Pink => [243, 139, 168, 255],
            StickyColor::Green => [166, 227, 161, 255],
            StickyColor::Blue => [137, 180, 250, 255],
            StickyColor::Purple => [203, 166, 247, 255],
        }
    }

    pub fn text_rgba(&self) -> [u8; 4] {
        [30, 30, 46, 255]
    }

    pub fn name(&self) -> &'static str {
        match self {
            StickyColor::Yellow => "Yellow",
            StickyColor::Pink => "Pink",
            StickyColor::Green => "Green",
            StickyColor::Blue => "Blue",
            StickyColor::Purple => "Purple",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cardinality {
    None,
    ExactlyOne,
    ZeroOrOne,
    OneOrMany,
    ZeroOrMany,
}

impl Cardinality {
    pub const ALL: [Cardinality; 5] = [
        Cardinality::None,
        Cardinality::ExactlyOne,
        Cardinality::ZeroOrOne,
        Cardinality::OneOrMany,
        Cardinality::ZeroOrMany,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Cardinality::None => "None",
            Cardinality::ExactlyOne => "1 (exactly one)",
            Cardinality::ZeroOrOne => "0..1 (zero or one)",
            Cardinality::OneOrMany => "1..N (one or many)",
            Cardinality::ZeroOrMany => "0..N (zero or many)",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Cardinality::None => "",
            Cardinality::ExactlyOne => "1",
            Cardinality::ZeroOrOne => "0..1",
            Cardinality::OneOrMany => "1..N",
            Cardinality::ZeroOrMany => "0..N",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Cardinality::None => "No cardinality constraint. A plain arrow is drawn.",
            Cardinality::ExactlyOne => "Exactly one related record must exist.\nDrawn as two perpendicular bars: ||",
            Cardinality::ZeroOrOne => "Zero or one related record may exist (optional).\nDrawn as a circle and bar: o|",
            Cardinality::OneOrMany => "One or more related records must exist.\nDrawn as a bar and crow's foot fork: |<",
            Cardinality::ZeroOrMany => "Zero or more related records may exist.\nDrawn as a circle and crow's foot fork: o<",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAttribute {
    pub name: String,
    pub attr_type: String,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    Shape {
        shape: NodeShape,
        label: String,
        description: String,
    },
    StickyNote {
        text: String,
        color: StickyColor,
    },
    Entity {
        name: String,
        attributes: Vec<EntityAttribute>,
    },
    Text {
        content: String,
    },
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
    pub corner_radius: f32,
    pub border_dashed: bool,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            fill_color: [49, 50, 68, 255],
            border_color: [69, 71, 90, 255],
            border_width: 1.5,
            text_color: [205, 214, 244, 255],
            font_size: 13.0,
            corner_radius: 8.0,
            border_dashed: false,
        }
    }
}

/// Entity node sizing constants.
pub const ENTITY_HEADER_HEIGHT: f32 = 30.0;
pub const ENTITY_ROW_HEIGHT: f32 = 22.0;
pub const ENTITY_MIN_WIDTH: f32 = 160.0;

/// Minimum sizes per node kind (width, height).
pub const MIN_SIZE_SHAPE: [f32; 2] = [40.0, 30.0];
pub const MIN_SIZE_ENTITY: [f32; 2] = [ENTITY_MIN_WIDTH, ENTITY_HEADER_HEIGHT + ENTITY_ROW_HEIGHT];
pub const MIN_SIZE_STICKY: [f32; 2] = [60.0, 60.0];
pub const MIN_SIZE_TEXT: [f32; 2] = [40.0, 20.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeTag {
    Critical,
    Warning,
    Ok,
    Info,
}

impl NodeTag {
    pub fn color(&self) -> [u8; 4] {
        match self {
            NodeTag::Critical => [243, 139, 168, 220], // red
            NodeTag::Warning  => [249, 226, 175, 220], // yellow
            NodeTag::Ok       => [166, 227, 161, 220], // green
            NodeTag::Info     => [137, 180, 250, 220], // blue
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            NodeTag::Critical => "Critical",
            NodeTag::Warning  => "Warning",
            NodeTag::Ok       => "OK",
            NodeTag::Info     => "Info",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub z_offset: f32,
    pub style: NodeStyle,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub tag: Option<NodeTag>,
}

impl Node {
    /// Create a flowchart shape node (backward-compatible constructor).
    pub fn new(shape: NodeShape, position: Pos2) -> Self {
        let size = match shape {
            NodeShape::Circle => [80.0, 80.0],
            NodeShape::Diamond => [120.0, 100.0],
            NodeShape::Connector => [110.0, 34.0],
            _ => [140.0, 60.0],
        };
        Self {
            id: NodeId::new(),
            kind: NodeKind::Shape {
                shape,
                label: String::from("New Node"),
                description: String::new(),
            },
            position: [position.x, position.y],
            size,
            z_offset: 0.0, pinned: false, tag: None,
            style: NodeStyle::default(),
        }
    }

    pub fn new_sticky(color: StickyColor, position: Pos2) -> Self {
        Self {
            id: NodeId::new(),
            kind: NodeKind::StickyNote {
                text: String::new(),
                color,
            },
            position: [position.x, position.y],
            size: [150.0, 150.0],
            z_offset: 0.0, pinned: false, tag: None,
            style: NodeStyle {
                fill_color: color.fill_rgba(),
                border_color: [0, 0, 0, 30],
                border_width: 0.0,
                text_color: color.text_rgba(),
                font_size: 14.0,
                corner_radius: 8.0, border_dashed: false,
            },
        }
    }

    pub fn new_entity(position: Pos2) -> Self {
        Self {
            id: NodeId::new(),
            kind: NodeKind::Entity {
                name: String::from("Entity"),
                attributes: vec![],
            },
            position: [position.x, position.y],
            size: [ENTITY_MIN_WIDTH, ENTITY_HEADER_HEIGHT + 4.0],
            z_offset: 0.0, pinned: false, tag: None,
            style: NodeStyle {
                fill_color: [49, 50, 68, 255],
                border_color: [137, 180, 250, 255],
                border_width: 1.5,
                text_color: [205, 214, 244, 255],
                font_size: 12.0,
                corner_radius: 4.0, border_dashed: false,
            },
        }
    }

    pub fn new_text(position: Pos2) -> Self {
        Self {
            id: NodeId::new(),
            kind: NodeKind::Text {
                content: String::from("Text"),
            },
            position: [position.x, position.y],
            size: [120.0, 40.0],
            z_offset: 0.0, pinned: false, tag: None,
            style: NodeStyle {
                fill_color: [0, 0, 0, 0],
                border_color: [0, 0, 0, 0],
                border_width: 0.0,
                text_color: [205, 214, 244, 255],
                font_size: 16.0,
                corner_radius: 0.0, border_dashed: false,
            },
        }
    }

    /// Returns the display label for any node kind.
    pub fn display_label(&self) -> &str {
        match &self.kind {
            NodeKind::Shape { label, .. } => label,
            NodeKind::StickyNote { text, .. } => text,
            NodeKind::Entity { name, .. } => name,
            NodeKind::Text { content, .. } => content,
        }
    }

    /// Recalculate size for Entity nodes based on attribute count.
    pub fn auto_size_entity(&mut self) {
        if let NodeKind::Entity { attributes, .. } = &self.kind {
            let rows = attributes.len().max(1) as f32;
            self.size[1] = ENTITY_HEADER_HEIGHT + rows * ENTITY_ROW_HEIGHT + 4.0;
            self.size[0] = self.size[0].max(ENTITY_MIN_WIDTH);
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

    /// Returns the minimum allowed [width, height] for this node kind.
    pub fn min_size(&self) -> [f32; 2] {
        match &self.kind {
            NodeKind::Shape { .. } => MIN_SIZE_SHAPE,
            NodeKind::Entity { .. } => MIN_SIZE_ENTITY,
            NodeKind::StickyNote { .. } => MIN_SIZE_STICKY,
            NodeKind::Text { .. } => MIN_SIZE_TEXT,
        }
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Port {
    pub node_id: NodeId,
    pub side: PortSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArrowHead {
    Filled,
    Open,
    Circle,
    None,
}

impl Default for ArrowHead {
    fn default() -> Self { Self::Filled }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeStyle {
    pub color: [u8; 4],
    pub width: f32,
    pub dashed: bool,
    pub orthogonal: bool,
    #[serde(default)]
    pub arrow_head: ArrowHead,
    #[serde(default)]
    pub curve_bend: f32,
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self {
            color: [100, 100, 100, 255],
            width: 2.5,
            dashed: false,
            orthogonal: false,
            arrow_head: ArrowHead::Filled,
            curve_bend: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub source: Port,
    pub target: Port,
    pub label: String,
    pub source_label: String,
    pub target_label: String,
    pub source_cardinality: Cardinality,
    pub target_cardinality: Cardinality,
    pub style: EdgeStyle,
}

impl Edge {
    pub fn new(source: Port, target: Port) -> Self {
        Self {
            id: EdgeId::new(),
            source,
            target,
            label: String::new(),
            source_label: String::new(),
            target_label: String::new(),
            source_cardinality: Cardinality::None,
            target_cardinality: Cardinality::None,
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
    pub node_ids: HashSet<NodeId>,
    pub edge_ids: HashSet<EdgeId>,
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
        if !self.node_ids.remove(&id) {
            self.node_ids.insert(id);
        }
    }

    pub fn toggle_edge(&mut self, id: EdgeId) {
        if !self.edge_ids.remove(&id) {
            self.edge_ids.insert(id);
        }
    }

    pub fn select_node(&mut self, id: NodeId) {
        self.clear();
        self.node_ids.insert(id);
    }

    pub fn select_edge(&mut self, id: EdgeId) {
        self.clear();
        self.edge_ids.insert(id);
    }
}

/// All four port sides, useful for iterating over every port on a node.
pub const ALL_SIDES: [PortSide; 4] = [PortSide::Top, PortSide::Bottom, PortSide::Left, PortSide::Right];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlowchartDocument {
    pub title: String,
    pub description: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl FlowchartDocument {
    /// Build a HashMap from NodeId to index for O(1) lookups.
    /// Call once per frame and reuse for edge drawing and hit testing.
    pub fn node_index(&self) -> HashMap<NodeId, usize> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id, i))
            .collect()
    }

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
