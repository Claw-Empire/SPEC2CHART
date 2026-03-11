use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecGraph {
    pub specgraph: String,
    pub title: String,
    pub mode: String,
    #[serde(default)]
    pub nodes: Vec<SpecNode>,
    #[serde(default)]
    pub edges: Vec<SpecEdge>,
    #[serde(default)]
    pub metadata: Option<SpecMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecNode {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub shape: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub attributes: Option<Vec<SpecAttribute>>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub position: Option<Vec<f32>>,  // [x, y] or [x, y, z]
    #[serde(default)]
    pub size: Option<[f32; 2]>,
    #[serde(default)]
    pub style: Option<SpecNodeStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecAttribute {
    pub name: String,
    #[serde(default)]
    pub pk: bool,
    #[serde(default)]
    pub fk: bool,
    #[serde(default, rename = "type")]
    pub attr_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecNodeStyle {
    pub fill: Option<[u8; 4]>,
    pub border: Option<[u8; 4]>,
    pub border_width: Option<f32>,
    pub text_color: Option<[u8; 4]>,
    pub font_size: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecEdge {
    pub from: SpecPort,
    pub to: SpecPort,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub source_label: Option<String>,
    #[serde(default)]
    pub target_label: Option<String>,
    #[serde(default)]
    pub source_cardinality: Option<String>,
    #[serde(default)]
    pub target_cardinality: Option<String>,
    #[serde(default)]
    pub style: Option<SpecEdgeStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecPort {
    pub node: String,
    pub side: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecEdgeStyle {
    pub color: Option<[u8; 4]>,
    pub width: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecMetadata {
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub llm_source: Option<String>,
    #[serde(default)]
    pub ab_variant: Option<String>,
}
