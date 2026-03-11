use std::collections::HashMap;
use egui::Pos2;
use crate::model::*;
use super::schema::*;

/// Convert a FlowchartDocument into a SpecGraph YAML structure.
pub fn document_to_specgraph(doc: &FlowchartDocument, title: &str) -> SpecGraph {
    let mode = infer_mode(doc);
    let id_map: HashMap<NodeId, String> = doc
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id, format!("n{}", i + 1)))
        .collect();

    let nodes: Vec<SpecNode> = doc
        .nodes
        .iter()
        .map(|n| node_to_spec(n, &id_map))
        .collect();

    let edges: Vec<SpecEdge> = doc
        .edges
        .iter()
        .filter_map(|e| edge_to_spec(e, &id_map))
        .collect();

    SpecGraph {
        specgraph: "1.0".to_string(),
        title: title.to_string(),
        mode,
        nodes,
        edges,
        metadata: None,
    }
}

/// Convert a SpecGraph YAML structure into a FlowchartDocument.
pub fn specgraph_to_document(sg: &SpecGraph) -> Result<FlowchartDocument, String> {
    let mut doc = FlowchartDocument::default();
    let mut id_map: HashMap<String, NodeId> = HashMap::new();

    for (i, sn) in sg.nodes.iter().enumerate() {
        let node = spec_to_node(sn, i)?;
        id_map.insert(sn.id.clone(), node.id);
        doc.nodes.push(node);
    }

    for se in &sg.edges {
        let edge = spec_to_edge(se, &id_map)?;
        doc.edges.push(edge);
    }

    // Auto-layout: assign positions to nodes that don't have them
    super::layout::hierarchical_layout(&mut doc);

    Ok(doc)
}

// ---------------------------------------------------------------------------
// Node conversion
// ---------------------------------------------------------------------------

fn node_to_spec(node: &Node, id_map: &HashMap<NodeId, String>) -> SpecNode {
    let id = id_map.get(&node.id).cloned().unwrap_or_default();
    let style = Some(SpecNodeStyle {
        fill: Some(node.style.fill_color),
        border: Some(node.style.border_color),
        border_width: Some(node.style.border_width),
        text_color: Some(node.style.text_color),
        font_size: Some(node.style.font_size),
    });

    match &node.kind {
        NodeKind::Shape { shape, label, description } => SpecNode {
            id,
            kind: "shape".to_string(),
            shape: Some(shape_to_str(shape).to_string()),
            label: Some(label.clone()),
            description: if description.is_empty() { None } else { Some(description.clone()) },
            name: None,
            attributes: None,
            text: None,
            color: None,
            content: None,
            position: Some(node_position_3d(node)),
            size: Some(node.size),
            style,
        },
        NodeKind::Entity { name, attributes } => SpecNode {
            id,
            kind: "entity".to_string(),
            shape: None,
            label: None,
            description: None,
            name: Some(name.clone()),
            attributes: Some(
                attributes.iter().map(|a| SpecAttribute {
                    name: a.name.clone(),
                    pk: a.is_primary_key,
                    fk: a.is_foreign_key,
                    attr_type: if a.attr_type.is_empty() { None } else { Some(a.attr_type.clone()) },
                }).collect(),
            ),
            text: None,
            color: None,
            content: None,
            position: Some(node_position_3d(node)),
            size: Some(node.size),
            style,
        },
        NodeKind::StickyNote { text, color } => SpecNode {
            id,
            kind: "sticky".to_string(),
            shape: None,
            label: None,
            description: None,
            name: None,
            attributes: None,
            text: Some(text.clone()),
            color: Some(sticky_color_to_str(color).to_string()),
            content: None,
            position: Some(node_position_3d(node)),
            size: Some(node.size),
            style,
        },
        NodeKind::Text { content } => SpecNode {
            id,
            kind: "text".to_string(),
            shape: None,
            label: None,
            description: None,
            name: None,
            attributes: None,
            text: None,
            color: None,
            content: Some(content.clone()),
            position: Some(node_position_3d(node)),
            size: Some(node.size),
            style,
        },
    }
}

fn spec_to_node(sn: &SpecNode, _index: usize) -> Result<Node, String> {
    let position = sn.position.as_deref().unwrap_or(&[]);
    let pos = Pos2::new(
        position.get(0).copied().unwrap_or(0.0),
        position.get(1).copied().unwrap_or(0.0),
    );
    let z_offset = position.get(2).copied().unwrap_or(0.0);

    let mut node = match sn.kind.as_str() {
        "shape" => {
            let shape = sn.shape.as_deref().map(str_to_shape).transpose()?
                .unwrap_or(NodeShape::RoundedRect);
            let mut node = Node::new(shape, pos);
            if let Some(label) = &sn.label {
                if let NodeKind::Shape { label: ref mut l, .. } = node.kind {
                    *l = label.clone();
                }
            }
            if let Some(desc) = &sn.description {
                if let NodeKind::Shape { description: ref mut d, .. } = node.kind {
                    *d = desc.clone();
                }
            }
            if let Some(size) = sn.size {
                node.size = size;
            }
            apply_style(&mut node, &sn.style);
            node
        }
        "entity" => {
            let mut node = Node::new_entity(pos);
            if let Some(name) = &sn.name {
                if let NodeKind::Entity { name: ref mut n, .. } = node.kind {
                    *n = name.clone();
                }
            }
            if let Some(attrs) = &sn.attributes {
                if let NodeKind::Entity { attributes: ref mut a, .. } = node.kind {
                    *a = attrs.iter().map(|sa| EntityAttribute {
                        name: sa.name.clone(),
                        attr_type: sa.attr_type.clone().unwrap_or_default(),
                        is_primary_key: sa.pk,
                        is_foreign_key: sa.fk,
                    }).collect();
                }
            }
            node.auto_size_entity();
            if let Some(size) = sn.size {
                node.size = size;
            }
            apply_style(&mut node, &sn.style);
            node
        }
        "sticky" => {
            let color = sn.color.as_deref().map(str_to_sticky_color).transpose()?
                .unwrap_or(StickyColor::Yellow);
            let mut node = Node::new_sticky(color, pos);
            if let Some(text) = &sn.text {
                if let NodeKind::StickyNote { text: ref mut t, .. } = node.kind {
                    *t = text.clone();
                }
            }
            if let Some(size) = sn.size {
                node.size = size;
            }
            apply_style(&mut node, &sn.style);
            node
        }
        "text" => {
            let mut node = Node::new_text(pos);
            if let Some(content) = &sn.content {
                if let NodeKind::Text { content: ref mut c } = node.kind {
                    *c = content.clone();
                }
            }
            if let Some(size) = sn.size {
                node.size = size;
            }
            apply_style(&mut node, &sn.style);
            node
        }
        other => return Err(format!("Unknown node kind: '{}'", other)),
    };

    node.z_offset = z_offset;
    Ok(node)
}

fn apply_style(node: &mut Node, style: &Option<SpecNodeStyle>) {
    if let Some(s) = style {
        if let Some(fill) = s.fill { node.style.fill_color = fill; }
        if let Some(border) = s.border { node.style.border_color = border; }
        if let Some(bw) = s.border_width { node.style.border_width = bw; }
        if let Some(tc) = s.text_color { node.style.text_color = tc; }
        if let Some(fs) = s.font_size { node.style.font_size = fs; }
    }
}

// ---------------------------------------------------------------------------
// Edge conversion
// ---------------------------------------------------------------------------

fn edge_to_spec(edge: &Edge, id_map: &HashMap<NodeId, String>) -> Option<SpecEdge> {
    let from_id = id_map.get(&edge.source.node_id)?;
    let to_id = id_map.get(&edge.target.node_id)?;

    Some(SpecEdge {
        from: SpecPort {
            node: from_id.clone(),
            side: side_to_str(edge.source.side).to_string(),
        },
        to: SpecPort {
            node: to_id.clone(),
            side: side_to_str(edge.target.side).to_string(),
        },
        label: if edge.label.is_empty() { None } else { Some(edge.label.clone()) },
        source_label: if edge.source_label.is_empty() { None } else { Some(edge.source_label.clone()) },
        target_label: if edge.target_label.is_empty() { None } else { Some(edge.target_label.clone()) },
        source_cardinality: Some(cardinality_to_str(edge.source_cardinality).to_string()),
        target_cardinality: Some(cardinality_to_str(edge.target_cardinality).to_string()),
        style: Some(SpecEdgeStyle {
            color: Some(edge.style.color),
            width: Some(edge.style.width),
        }),
    })
}

fn spec_to_edge(se: &SpecEdge, id_map: &HashMap<String, NodeId>) -> Result<Edge, String> {
    let source_id = id_map.get(&se.from.node)
        .ok_or_else(|| format!("Unknown node id in edge 'from': '{}'", se.from.node))?;
    let target_id = id_map.get(&se.to.node)
        .ok_or_else(|| format!("Unknown node id in edge 'to': '{}'", se.to.node))?;

    let source = Port {
        node_id: *source_id,
        side: str_to_side(&se.from.side)?,
    };
    let target = Port {
        node_id: *target_id,
        side: str_to_side(&se.to.side)?,
    };

    let mut edge = Edge::new(source, target);
    if let Some(label) = &se.label { edge.label = label.clone(); }
    if let Some(sl) = &se.source_label { edge.source_label = sl.clone(); }
    if let Some(tl) = &se.target_label { edge.target_label = tl.clone(); }
    if let Some(sc) = &se.source_cardinality {
        edge.source_cardinality = str_to_cardinality(sc);
    }
    if let Some(tc) = &se.target_cardinality {
        edge.target_cardinality = str_to_cardinality(tc);
    }
    if let Some(s) = &se.style {
        if let Some(color) = s.color { edge.style.color = color; }
        if let Some(width) = s.width { edge.style.width = width; }
    }
    Ok(edge)
}

// ---------------------------------------------------------------------------
// Position helpers
// ---------------------------------------------------------------------------

fn node_position_3d(node: &Node) -> Vec<f32> {
    if node.z_offset != 0.0 {
        vec![node.position[0], node.position[1], node.z_offset]
    } else {
        vec![node.position[0], node.position[1]]
    }
}

// ---------------------------------------------------------------------------
// String helpers
// ---------------------------------------------------------------------------

fn shape_to_str(s: &NodeShape) -> &'static str {
    match s {
        NodeShape::Rectangle => "rectangle",
        NodeShape::RoundedRect => "rounded_rect",
        NodeShape::Diamond => "diamond",
        NodeShape::Circle => "circle",
        NodeShape::Parallelogram => "parallelogram",
        NodeShape::Connector => "connector",
    }
}

fn str_to_shape(s: &str) -> Result<NodeShape, String> {
    match s {
        "rectangle" => Ok(NodeShape::Rectangle),
        "rounded_rect" => Ok(NodeShape::RoundedRect),
        "diamond" => Ok(NodeShape::Diamond),
        "circle" => Ok(NodeShape::Circle),
        "parallelogram" => Ok(NodeShape::Parallelogram),
        "connector" => Ok(NodeShape::Connector),
        other => Err(format!("Unknown shape: '{}'", other)),
    }
}

fn sticky_color_to_str(c: &StickyColor) -> &'static str {
    match c {
        StickyColor::Yellow => "yellow",
        StickyColor::Pink => "pink",
        StickyColor::Green => "green",
        StickyColor::Blue => "blue",
        StickyColor::Purple => "purple",
    }
}

fn str_to_sticky_color(s: &str) -> Result<StickyColor, String> {
    match s {
        "yellow" => Ok(StickyColor::Yellow),
        "pink" => Ok(StickyColor::Pink),
        "green" => Ok(StickyColor::Green),
        "blue" => Ok(StickyColor::Blue),
        "purple" => Ok(StickyColor::Purple),
        other => Err(format!("Unknown sticky color: '{}'", other)),
    }
}

fn side_to_str(s: PortSide) -> &'static str {
    match s {
        PortSide::Top => "top",
        PortSide::Bottom => "bottom",
        PortSide::Left => "left",
        PortSide::Right => "right",
    }
}

fn str_to_side(s: &str) -> Result<PortSide, String> {
    match s {
        "top" => Ok(PortSide::Top),
        "bottom" => Ok(PortSide::Bottom),
        "left" => Ok(PortSide::Left),
        "right" => Ok(PortSide::Right),
        other => Err(format!("Unknown port side: '{}'", other)),
    }
}

fn cardinality_to_str(c: Cardinality) -> &'static str {
    match c {
        Cardinality::None => "none",
        Cardinality::ExactlyOne => "exactly_one",
        Cardinality::ZeroOrOne => "zero_or_one",
        Cardinality::OneOrMany => "one_or_many",
        Cardinality::ZeroOrMany => "zero_or_many",
    }
}

fn str_to_cardinality(s: &str) -> Cardinality {
    match s {
        "exactly_one" => Cardinality::ExactlyOne,
        "zero_or_one" => Cardinality::ZeroOrOne,
        "one_or_many" => Cardinality::OneOrMany,
        "zero_or_many" => Cardinality::ZeroOrMany,
        _ => Cardinality::None,
    }
}

fn infer_mode(doc: &FlowchartDocument) -> String {
    let has_entity = doc.nodes.iter().any(|n| matches!(n.kind, NodeKind::Entity { .. }));
    let has_sticky = doc.nodes.iter().any(|n| matches!(n.kind, NodeKind::StickyNote { .. }));
    if has_entity {
        "er".to_string()
    } else if has_sticky {
        "figjam".to_string()
    } else {
        "flowchart".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specgraph::import_yaml;

    #[test]
    fn test_import_login_flow() {
        let yaml = std::fs::read_to_string("examples/login-flow.yaml").unwrap();
        match import_yaml(&yaml) {
            Ok(doc) => {
                println!("Nodes: {}", doc.nodes.len());
                println!("Edges: {}", doc.edges.len());
                for n in &doc.nodes {
                    println!("  node at {:?} size {:?}", n.position, n.size);
                }
                assert_eq!(doc.nodes.len(), 6);
                assert_eq!(doc.edges.len(), 6);
            }
            Err(e) => panic!("Import failed: {}", e),
        }
    }
}
