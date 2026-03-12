use crate::model::*;
use egui::Pos2;
use std::collections::HashMap;

/// Parse a Human-Readable Format (.spec) string into a FlowchartDocument.
///
/// Format:
/// ```text
/// # Title
///
/// Overall description paragraph that explains the whole diagram.
/// Can span multiple lines.
///
/// ## Nodes
/// - [id] Label text
///   Description of this node. Can span multiple indented lines.
///   More detail here.
/// - [id] Label text {diamond}
///   Another description.
///
/// ## Flow
/// id "label" --> id
/// id --> id
///
/// ## Notes
/// - Note text {yellow}
/// ```
pub fn parse_hrf(input: &str) -> Result<FlowchartDocument, String> {
    let mut doc = FlowchartDocument::default();
    let mut id_map: HashMap<String, NodeId> = HashMap::new();

    let mut section = Section::None;
    let mut preamble_lines: Vec<String> = Vec::new();
    let mut seen_section = false;

    // Track the last node added in Nodes section for multi-line descriptions
    let mut last_node_id: Option<NodeId> = None;

    for (line_num, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim_end();

        // Title: # Something
        if line.trim().starts_with("# ") && !line.trim().starts_with("## ") {
            doc.title = line.trim()[2..].trim().to_string();
            continue;
        }

        // Section headers
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            seen_section = true;
            last_node_id = None;
            let header = trimmed[3..].trim().to_lowercase();
            section = match header.as_str() {
                "nodes" | "node" | "components" => Section::Nodes,
                "flow" | "flows" | "edges" | "connections" => Section::Flow,
                "notes" | "note" | "stickies" => Section::Notes,
                _ => Section::None,
            };
            continue;
        }

        // Before first ## section: collect as diagram description
        if !seen_section {
            if !trimmed.is_empty() && !doc.title.is_empty() {
                preamble_lines.push(trimmed.to_string());
            }
            continue;
        }

        // Empty lines reset the last_node context in Nodes section
        if trimmed.is_empty() {
            continue;
        }

        match section {
            Section::Nodes => {
                if trimmed.starts_with("- ") {
                    // New node definition
                    let stripped = &trimmed[2..];
                    let (id, node) = parse_node_line(stripped, line_num)?;
                    last_node_id = Some(node.id);
                    id_map.insert(id, node.id);
                    doc.nodes.push(node);
                } else if line.starts_with("  ") || line.starts_with("\t") {
                    // Indented continuation = description for last node
                    if let Some(nid) = last_node_id {
                        if let Some(node) = doc.find_node_mut(&nid) {
                            append_description(node, trimmed);
                        }
                    }
                }
            }
            Section::Flow => {
                if !trimmed.is_empty() {
                    let edges = parse_flow_line_chain(trimmed, &id_map, line_num)?;
                    for edge in edges {
                        doc.edges.push(edge);
                    }
                }
            }
            Section::Notes => {
                if trimmed.starts_with("- ") {
                    let stripped = &trimmed[2..];
                    let node = parse_note_line(stripped)?;
                    doc.nodes.push(node);
                }
            }
            Section::None => {}
        }
    }

    doc.description = preamble_lines.join("\n");

    // Auto-layout: topological / hierarchical placement
    super::layout::hierarchical_layout(&mut doc);

    Ok(doc)
}

/// Export a FlowchartDocument to Human-Readable Format.
pub fn export_hrf(doc: &FlowchartDocument, title: &str) -> String {
    let mut out = String::new();
    let display_title = if doc.title.is_empty() { title } else { &doc.title };
    out.push_str(&format!("# {}\n\n", display_title));

    // Diagram description
    if !doc.description.is_empty() {
        out.push_str(&doc.description);
        out.push_str("\n\n");
    }

    // Build reverse ID map
    let id_map: HashMap<NodeId, String> = doc
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let label = n.display_label();
            let id = slugify(label, i);
            (n.id, id)
        })
        .collect();

    // Nodes section (shapes + entities + text)
    let shape_nodes: Vec<&Node> = doc
        .nodes
        .iter()
        .filter(|n| !matches!(n.kind, NodeKind::StickyNote { .. }))
        .collect();

    if !shape_nodes.is_empty() {
        out.push_str("## Nodes\n");
        for node in &shape_nodes {
            let id = id_map.get(&node.id).cloned().unwrap_or_default();
            match &node.kind {
                NodeKind::Shape { shape, label, description } => {
                    let shape_tag = match shape {
                        NodeShape::Rectangle => "",
                        NodeShape::RoundedRect => "",
                        NodeShape::Diamond => " {diamond}",
                        NodeShape::Circle => " {circle}",
                        NodeShape::Parallelogram => " {parallelogram}",
                        NodeShape::Hexagon => " {hexagon}",
                        NodeShape::Connector => " {connector}",
                    };
                    let z_tag = if node.z_offset != 0.0 {
                        format!(" {{z:{}}}", node.z_offset)
                    } else { String::new() };
                    out.push_str(&format!("- [{}] {}{}{}\n", id, label, shape_tag, z_tag));
                    if !description.is_empty() {
                        for desc_line in description.lines() {
                            out.push_str(&format!("  {}\n", desc_line));
                        }
                    }
                }
                NodeKind::Entity { name, attributes } => {
                    let z_tag = if node.z_offset != 0.0 {
                        format!(" {{z:{}}}", node.z_offset)
                    } else { String::new() };
                    out.push_str(&format!("- [{}] {} {{entity}}{}\n", id, name, z_tag));
                    for attr in attributes {
                        let mut tags = Vec::new();
                        if attr.is_primary_key { tags.push("PK"); }
                        if attr.is_foreign_key { tags.push("FK"); }
                        let tag_str = if tags.is_empty() {
                            String::new()
                        } else {
                            format!(" [{}]", tags.join(", "))
                        };
                        if attr.attr_type.is_empty() {
                            out.push_str(&format!("  {}{}\n", attr.name, tag_str));
                        } else {
                            out.push_str(&format!("  {} ({}){}\n", attr.name, attr.attr_type, tag_str));
                        }
                    }
                }
                NodeKind::Text { content } => {
                    let z_tag = if node.z_offset != 0.0 {
                        format!(" {{z:{}}}", node.z_offset)
                    } else { String::new() };
                    out.push_str(&format!("- [{}] {} {{text}}{}\n", id, content, z_tag));
                }
                _ => {}
            }
        }
        out.push('\n');
    }

    // Flow section
    if !doc.edges.is_empty() {
        out.push_str("## Flow\n");
        for edge in &doc.edges {
            let from = id_map.get(&edge.source.node_id).cloned().unwrap_or_default();
            let to = id_map.get(&edge.target.node_id).cloned().unwrap_or_default();
            if edge.label.is_empty() {
                out.push_str(&format!("{} --> {}\n", from, to));
            } else {
                out.push_str(&format!("{} \"{}\" --> {}\n", from, edge.label, to));
            }
        }
        out.push('\n');
    }

    // Notes section
    let sticky_nodes: Vec<&Node> = doc
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::StickyNote { .. }))
        .collect();

    if !sticky_nodes.is_empty() {
        out.push_str("## Notes\n");
        for node in &sticky_nodes {
            if let NodeKind::StickyNote { text, color } = &node.kind {
                let color_tag = match color {
                    StickyColor::Yellow => " {yellow}",
                    StickyColor::Pink => " {pink}",
                    StickyColor::Green => " {green}",
                    StickyColor::Blue => " {blue}",
                    StickyColor::Purple => " {purple}",
                };
                out.push_str(&format!("- {}{}\n", text, color_tag));
            }
        }
        out.push('\n');
    }

    out
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Section {
    None,
    Nodes,
    Flow,
    Notes,
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

/// Append description text to a node's description field.
fn append_description(node: &mut Node, text: &str) {
    match &mut node.kind {
        NodeKind::Shape { description, .. } => {
            if !description.is_empty() {
                description.push('\n');
            }
            description.push_str(text);
        }
        NodeKind::StickyNote { text: t, .. } => {
            if !t.is_empty() {
                t.push('\n');
            }
            t.push_str(text);
        }
        NodeKind::Text { content } => {
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(text);
        }
        NodeKind::Entity { .. } => {
            // Entity descriptions could be attributes — skip for now
        }
    }
}

/// Parse a flow line that may be a chain: `a --> b --> c` or `a "label" --> b --> c`.
/// Splits into individual edges.
fn parse_flow_line_chain(
    line: &str,
    id_map: &HashMap<String, NodeId>,
    line_num: usize,
) -> Result<Vec<Edge>, String> {
    // Split on "-->" but preserve quoted labels that precede arrows
    // Strategy: tokenise by splitting on "-->" then pair up segments
    let segments: Vec<&str> = line.split("-->").collect();
    if segments.len() < 2 {
        return Err(format!("Line {}: expected '-->' in flow definition", line_num + 1));
    }

    let mut edges = Vec::new();
    for i in 0..segments.len() - 1 {
        let left = segments[i].trim();
        let right = segments[i + 1].trim();

        // The "right" side may have a quoted label at the end before the next segment
        // but since we've already split, right is just the node id (possibly with more
        // content if it's the last segment — ignore anything after the id).
        // Extract node id from left (strip any trailing label in quotes)
        let (from_id, label) = if let Some(q_start) = left.find('"') {
            let before = left[..q_start].trim();
            let q_end = left.rfind('"').unwrap_or(left.len());
            let lbl = if q_end > q_start + 1 {
                left[q_start + 1..q_end].to_string()
            } else {
                String::new()
            };
            (before.to_string(), lbl)
        } else {
            (left.to_string(), String::new())
        };

        // right: just the node id (first word)
        let to_id = right.split_whitespace().next().unwrap_or(right).to_string();

        let source_node_id = id_map.get(&from_id).ok_or_else(|| {
            format!("Line {}: unknown node '{}' (not defined in ## Nodes)", line_num + 1, from_id)
        })?;
        let target_node_id = id_map.get(&to_id).ok_or_else(|| {
            format!("Line {}: unknown node '{}' (not defined in ## Nodes)", line_num + 1, to_id)
        })?;

        let source = Port { node_id: *source_node_id, side: PortSide::Bottom };
        let target = Port { node_id: *target_node_id, side: PortSide::Top };
        let mut edge = Edge::new(source, target);
        edge.label = label;
        edges.push(edge);
    }

    Ok(edges)
}

/// Parse: `[id] Label text {shape} {z:50}`
fn parse_node_line(line: &str, line_num: usize) -> Result<(String, Node), String> {
    let id_start = line.find('[').ok_or_else(|| {
        format!("Line {}: expected [id] in node definition", line_num + 1)
    })?;
    let id_end = line.find(']').ok_or_else(|| {
        format!("Line {}: missing closing ] in node id", line_num + 1)
    })?;
    let id = line[id_start + 1..id_end].trim().to_string();
    let rest = line[id_end + 1..].trim();

    let (label, tags) = extract_tags(rest);

    let mut shape = NodeShape::RoundedRect;
    let mut z_offset = 0.0f32;
    for tag in &tags {
        if tag.starts_with("z:") {
            if let Ok(z) = tag[2..].trim().parse::<f32>() {
                z_offset = z;
            }
        } else {
            shape = tag_to_shape(tag);
        }
    }

    let mut node = Node::new(shape, Pos2::ZERO);
    node.z_offset = z_offset;
    if let NodeKind::Shape { label: ref mut l, .. } = node.kind {
        *l = label;
    }

    Ok((id, node))
}

/// Extract `{tag}` blocks from a string, returning the cleaned label and list of tags.
fn extract_tags(s: &str) -> (String, Vec<String>) {
    let mut label = String::new();
    let mut tags = Vec::new();
    let mut in_tag = false;
    let mut tag_buf = String::new();

    for c in s.chars() {
        match c {
            '{' => { in_tag = true; tag_buf.clear(); }
            '}' => {
                if in_tag {
                    let tag = tag_buf.trim().to_lowercase();
                    if !tag.is_empty() { tags.push(tag); }
                    in_tag = false;
                }
            }
            _ => {
                if in_tag { tag_buf.push(c); }
                else { label.push(c); }
            }
        }
    }

    (label.trim().to_string(), tags)
}

/// Parse: `id "label" --> id` or `id --> id`

/// Parse: `Note text {color}`
fn parse_note_line(line: &str) -> Result<Node, String> {
    let (text, color) = if let Some(brace_start) = line.rfind('{') {
        if let Some(brace_end) = line.rfind('}') {
            let tag = line[brace_start + 1..brace_end].trim().to_lowercase();
            let text = line[..brace_start].trim();
            let color = match tag.as_str() {
                "pink" => StickyColor::Pink,
                "green" => StickyColor::Green,
                "blue" => StickyColor::Blue,
                "purple" => StickyColor::Purple,
                _ => StickyColor::Yellow,
            };
            (text.to_string(), color)
        } else {
            (line.to_string(), StickyColor::Yellow)
        }
    } else {
        (line.to_string(), StickyColor::Yellow)
    };

    let mut node = Node::new_sticky(color, Pos2::ZERO);
    if let NodeKind::StickyNote { text: ref mut t, .. } = node.kind {
        *t = text;
    }
    Ok(node)
}

fn tag_to_shape(tag: &str) -> NodeShape {
    match tag {
        "rectangle" | "rect" => NodeShape::Rectangle,
        "diamond" | "decision" => NodeShape::Diamond,
        "circle" => NodeShape::Circle,
        "parallelogram" | "parallel" | "io" => NodeShape::Parallelogram,
        "hexagon" | "hex" | "process" => NodeShape::Hexagon,
        "connector" | "api" | "interface" | "protocol" | "gateway" => NodeShape::Connector,
        _ => NodeShape::RoundedRect,
    }
}

fn slugify(label: &str, index: usize) -> String {
    let slug: String = label
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == ' ' || c == '-' || c == '_' {
                Some('_')
            } else {
                None
            }
        })
        .collect();

    if slug.is_empty() {
        format!("n{}", index + 1)
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_descriptions() {
        let input = r#"
# Test Flow

This is a diagram that shows how testing works.
It has multiple steps.

## Nodes
- [start] Start here
  This is where everything begins.
  Users land on this page first.
- [check] Is it valid? {diamond}
  A decision point that validates user input.
- [done] All done {circle}

## Flow
start --> check
check "yes" --> done

## Notes
- Remember to test {yellow}
"#;
        let doc = parse_hrf(input).unwrap();
        assert_eq!(doc.title, "Test Flow");
        assert_eq!(doc.description, "This is a diagram that shows how testing works.\nIt has multiple steps.");
        assert_eq!(doc.nodes.len(), 4); // 3 shape + 1 sticky

        // Check first node has description
        if let NodeKind::Shape { description, .. } = &doc.nodes[0].kind {
            assert!(description.contains("everything begins"));
            assert!(description.contains("Users land"));
        } else {
            panic!("Expected shape node");
        }
    }

    #[test]
    fn test_roundtrip_hrf() {
        let input = r#"
# My Flow

Overall description here.

## Nodes
- [a] Step A
  First step description.
- [b] Step B {diamond}
- [c] Step C

## Flow
a --> b
b "next" --> c
"#;
        let doc = parse_hrf(input).unwrap();
        let exported = export_hrf(&doc, "My Flow");
        assert!(exported.contains("# My Flow"));
        assert!(exported.contains("Overall description here."));
        assert!(exported.contains("First step description."));
        assert!(exported.contains("## Nodes"));
        assert!(exported.contains("## Flow"));
        assert!(exported.contains("-->"));
    }
}
