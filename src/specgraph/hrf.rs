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
/// - [id] Label text {circle} {z:120}       ← 3D layer offset
/// - [id] Label text {critical}              ← tag badge
/// - [id] Label text {pinned}                ← pinned to canvas
///
/// ## Flow
/// id "label" --> id
/// id --> id
/// id "label" --> id {dashed}                ← dashed edge
/// id --> id {glow}                          ← glowing edge
/// id --> id {arrow:open}                    ← arrow head style
///
/// ## Notes
/// - Note text {yellow}
/// ```
///
/// ### Supported node tags:
///   `{diamond}` `{circle}` `{rectangle}` `{parallelogram}` `{hexagon}` `{connector}` `{text}` `{entity}`
///   `{z:N}` — 3D layer offset (positive = closer to camera)
///   `{critical}` `{warning}` `{ok}` `{info}` — status tag badge
///   `{pinned}` — pin node to canvas position
///
/// ### Supported node style tags:
///   `{fill:blue}` — fill color (blue/green/red/yellow/purple/pink/teal/white/black)
///   `{w:200}` — explicit width in canvas units
///   `{h:100}` — explicit height in canvas units
///   `{icon:🔒}` — icon badge
///   `{shadow}` — drop shadow effect
///   `{bold}` — bold text
///   `{italic}` — italic text
///   `{dashed-border}` — dashed border line
///   `{r:N}` — corner radius override
///   `{border:N}` — border width (e.g. `{border:2.5}`)
///   `{align:left}` `{align:right}` `{align:center}` — horizontal text alignment
///   `{valign:top}` `{valign:bottom}` `{valign:middle}` — vertical text alignment
///
/// ### Supported edge tags:
///   `{dashed}` — dashed line style
///   `{glow}` — neon glow effect
///   `{animated}` — animated flow dots
///   `{thick}` — wider stroke (5px)
///   `{ortho}` — orthogonal (right-angle) routing
///   `{bend:0.3}` — curve bend amount (-1.0 to 1.0)
///   `{color:red}` — edge color (red/blue/green/yellow/purple/gray)
///   `{arrow:open}` `{arrow:circle}` `{arrow:none}` — arrow head variant
///   `{from:label}` — source endpoint label
///   `{to:label}` — target endpoint label
///   `{c-src:1}` — source cardinality (1 / 0..1 / 1..N / 0..N)
///   `{c-tgt:0..N}` — target cardinality (1 / 0..1 / 1..N / 0..N)
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
                    // Indented continuation — entity attribute or description
                    if let Some(nid) = last_node_id {
                        if let Some(node) = doc.find_node_mut(&nid) {
                            if matches!(node.kind, NodeKind::Entity { .. }) {
                                // Parse as entity attribute: `name (type) [PK, FK]`
                                let attr = parse_entity_attribute(trimmed);
                                if let NodeKind::Entity { attributes, .. } = &mut node.kind {
                                    attributes.push(attr);
                                }
                            } else {
                                append_description(node, trimmed);
                            }
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
                    let tag_tag = match node.tag {
                        Some(NodeTag::Critical) => " {critical}",
                        Some(NodeTag::Warning) => " {warning}",
                        Some(NodeTag::Ok) => " {ok}",
                        Some(NodeTag::Info) => " {info}",
                        None => "",
                    };
                    let pin_tag = if node.pinned { " {pinned}" } else { "" };
                    let fill_tag = fill_color_name(node.style.fill_color)
                        .map(|n| format!(" {{fill:{}}}", n))
                        .unwrap_or_default();
                    let w_tag = if node.size[0] != 160.0 {
                        format!(" {{w:{}}}", node.size[0])
                    } else { String::new() };
                    let h_tag = if node.size[1] != 80.0 {
                        format!(" {{h:{}}}", node.size[1])
                    } else { String::new() };
                    let icon_tag = if !node.icon.is_empty() {
                        format!(" {{icon:{}}}", node.icon)
                    } else { String::new() };
                    let shadow_tag = if node.style.shadow { " {shadow}" } else { "" };
                    let bold_tag = if node.style.bold { " {bold}" } else { "" };
                    let italic_tag = if node.style.italic { " {italic}" } else { "" };
                    let dashed_border_tag = if node.style.border_dashed { " {dashed-border}" } else { "" };
                    let radius_tag = if (node.style.corner_radius - 6.0).abs() > 0.1 {
                        format!(" {{r:{}}}", node.style.corner_radius)
                    } else { String::new() };
                    let border_tag = if (node.style.border_width - 1.5).abs() > 0.1 {
                        format!(" {{border:{}}}", node.style.border_width)
                    } else { String::new() };
                    let align_tag = match node.style.text_align {
                        crate::model::TextAlign::Left => " {align:left}",
                        crate::model::TextAlign::Right => " {align:right}",
                        crate::model::TextAlign::Center => "",
                    };
                    let valign_tag = match node.style.text_valign {
                        crate::model::TextVAlign::Top => " {valign:top}",
                        crate::model::TextVAlign::Bottom => " {valign:bottom}",
                        crate::model::TextVAlign::Middle => "",
                    };
                    out.push_str(&format!("- [{}] {}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}\n",
                        id, label, shape_tag, z_tag, tag_tag, pin_tag, fill_tag, icon_tag,
                        shadow_tag, bold_tag, italic_tag, dashed_border_tag, radius_tag,
                        border_tag, align_tag, valign_tag, w_tag, h_tag));
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
            // Collect edge style tags
            let mut style_tags: Vec<String> = Vec::new();
            if edge.style.dashed { style_tags.push("dashed".to_string()); }
            if edge.style.glow { style_tags.push("glow".to_string()); }
            if edge.style.animated { style_tags.push("animated".to_string()); }
            if edge.style.width > 4.0 { style_tags.push("thick".to_string()); }
            if edge.style.orthogonal { style_tags.push("ortho".to_string()); }
            if edge.style.curve_bend.abs() > 0.01 {
                style_tags.push(format!("bend:{:.1}", edge.style.curve_bend));
            }
            if let Some(name) = edge_color_name(edge.style.color) {
                // Only export non-default colors (gray is default)
                if name != "gray" {
                    style_tags.push(format!("color:{}", name));
                }
            }
            match edge.style.arrow_head {
                ArrowHead::Open => style_tags.push("arrow:open".to_string()),
                ArrowHead::Circle => style_tags.push("arrow:circle".to_string()),
                ArrowHead::None => style_tags.push("arrow:none".to_string()),
                ArrowHead::Filled => {} // default, don't export
            }
            if !edge.source_label.is_empty() {
                style_tags.push(format!("from:{}", edge.source_label));
            }
            if !edge.target_label.is_empty() {
                style_tags.push(format!("to:{}", edge.target_label));
            }
            if let Some(s) = cardinality_str(&edge.source_cardinality) {
                style_tags.push(format!("c-src:{}", s));
            }
            if let Some(s) = cardinality_str(&edge.target_cardinality) {
                style_tags.push(format!("c-tgt:{}", s));
            }
            let tag_str = if style_tags.is_empty() {
                String::new()
            } else {
                format!(" {{{}}}", style_tags.join("} {"))
            };
            if edge.label.is_empty() {
                out.push_str(&format!("{} --> {}{}\n", from, to, tag_str));
            } else {
                out.push_str(&format!("{} \"{}\" --> {}{}\n", from, edge.label, to, tag_str));
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
                let z_tag = if node.z_offset != 0.0 {
                    format!(" {{z:{}}}", node.z_offset)
                } else { String::new() };
                out.push_str(&format!("- {}{}{}\n", text, color_tag, z_tag));
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

        // right: node id (first word), then optional {tags}
        let (to_id_raw, edge_tags) = extract_tags(right);
        let to_id = to_id_raw.split_whitespace().next().unwrap_or(&to_id_raw).to_string();

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
        // Apply edge style tags
        for etag in &edge_tags {
            if etag.starts_with("color:") {
                if let Some(c) = tag_to_edge_color(etag[6..].trim()) {
                    edge.style.color = c;
                }
            } else if etag.starts_with("bend:") {
                if let Ok(b) = etag[5..].trim().parse::<f32>() {
                    edge.style.curve_bend = b.clamp(-1.0, 1.0);
                }
            } else if etag.starts_with("from:") {
                edge.source_label = etag[5..].trim().to_string();
            } else if etag.starts_with("to:") {
                edge.target_label = etag[3..].trim().to_string();
            } else if etag.starts_with("c-src:") {
                edge.source_cardinality = parse_cardinality(etag[6..].trim());
            } else if etag.starts_with("c-tgt:") {
                edge.target_cardinality = parse_cardinality(etag[6..].trim());
            } else {
                match etag.as_str() {
                    "dashed" | "dash" => edge.style.dashed = true,
                    "glow" | "neon" => edge.style.glow = true,
                    "animated" | "animate" | "flow" => edge.style.animated = true,
                    "thick" | "bold" => edge.style.width = 5.0,
                    "thin" => edge.style.width = 1.5,
                    "ortho" | "orthogonal" => edge.style.orthogonal = true,
                    "arrow:open" => edge.style.arrow_head = ArrowHead::Open,
                    "arrow:circle" => edge.style.arrow_head = ArrowHead::Circle,
                    "arrow:none" => edge.style.arrow_head = ArrowHead::None,
                    _ => {}
                }
            }
        }
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
    let mut node_tag: Option<NodeTag> = None;
    let mut pinned = false;
    let mut fill_color: Option<[u8; 4]> = None;
    let mut width_override: Option<f32> = None;
    let mut height_override: Option<f32> = None;
    let mut icon: Option<String> = None;
    let mut shadow = false;
    let mut bold = false;
    let mut italic = false;
    let mut dashed_border = false;
    let mut corner_radius: Option<f32> = None;
    let mut border_width: Option<f32> = None;
    let mut text_align: Option<crate::model::TextAlign> = None;
    let mut text_valign: Option<crate::model::TextVAlign> = None;
    for tag in &tags {
        if tag.starts_with("z:") {
            if let Ok(z) = tag[2..].trim().parse::<f32>() {
                z_offset = z;
            }
        } else if tag.starts_with("fill:") {
            fill_color = tag_to_fill_color(tag[5..].trim());
        } else if tag.starts_with("w:") {
            width_override = tag[2..].trim().parse::<f32>().ok();
        } else if tag.starts_with("h:") {
            height_override = tag[2..].trim().parse::<f32>().ok();
        } else if tag.starts_with("r:") {
            corner_radius = tag[2..].trim().parse::<f32>().ok();
        } else if tag.starts_with("icon:") {
            icon = Some(tag[5..].trim().to_string());
        } else if let Some(nt) = tag_to_node_tag(tag) {
            node_tag = Some(nt);
        } else if tag == "pinned" || tag == "pin" {
            pinned = true;
        } else if tag.starts_with("border:") {
            border_width = tag[7..].trim().parse::<f32>().ok();
        } else if tag.starts_with("align:") {
            text_align = match tag[6..].trim() {
                "left" => Some(crate::model::TextAlign::Left),
                "right" => Some(crate::model::TextAlign::Right),
                _ => Some(crate::model::TextAlign::Center),
            };
        } else if tag.starts_with("valign:") {
            text_valign = match tag[7..].trim() {
                "top" => Some(crate::model::TextVAlign::Top),
                "bottom" => Some(crate::model::TextVAlign::Bottom),
                _ => Some(crate::model::TextVAlign::Middle),
            };
        } else if tag == "shadow" || tag == "drop-shadow" {
            shadow = true;
        } else if tag == "bold" || tag == "strong" {
            bold = true;
        } else if tag == "italic" || tag == "em" {
            italic = true;
        } else if tag == "dashed-border" || tag == "dashed_border" || tag == "border-dashed" {
            dashed_border = true;
        } else {
            shape = tag_to_shape(tag);
        }
    }

    // Detect special kinds from tags
    let mut is_entity = false;
    let mut is_text = false;
    for tag in &tags {
        match tag.as_str() {
            "entity" | "table" | "er" => { is_entity = true; }
            "text" | "label" => { is_text = true; }
            _ => {}
        }
    }

    let mut node = if is_entity {
        let mut n = Node::new_entity(Pos2::ZERO);
        if let NodeKind::Entity { name, .. } = &mut n.kind {
            *name = label;
        }
        n
    } else if is_text {
        let mut n = Node::new_text(Pos2::ZERO);
        if let NodeKind::Text { content } = &mut n.kind {
            *content = label;
        }
        n
    } else {
        let mut n = Node::new(shape, Pos2::ZERO);
        if let NodeKind::Shape { label: ref mut l, .. } = n.kind {
            *l = label;
        }
        n
    };
    node.z_offset = z_offset;
    node.tag = node_tag;
    node.pinned = pinned;
    if let Some(fc) = fill_color {
        node.style.fill_color = fc;
        // Auto-contrast: pick light or dark text based on fill luminance
        let luma = 0.299 * fc[0] as f32 + 0.587 * fc[1] as f32 + 0.114 * fc[2] as f32;
        node.style.text_color = if luma > 140.0 { [15, 15, 20, 255] } else { [220, 220, 230, 255] };
    }
    if let Some(w) = width_override {
        node.size[0] = w;
    }
    if let Some(h) = height_override {
        node.size[1] = h;
    }
    if let Some(ic) = icon {
        node.icon = ic;
    }
    if shadow { node.style.shadow = true; }
    if bold { node.style.bold = true; }
    if italic { node.style.italic = true; }
    if dashed_border { node.style.border_dashed = true; }
    if let Some(cr) = corner_radius { node.style.corner_radius = cr; }
    if let Some(bw) = border_width { node.style.border_width = bw; }
    if let Some(ta) = text_align { node.style.text_align = ta; }
    if let Some(tv) = text_valign { node.style.text_valign = tv; }

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
                    let raw = tag_buf.trim().to_string();
                    // For key:value tags, lowercase the key; preserve value case for
                    // from:/to:/icon: but lowercase for fill:/color:/arrow:/z:/w:/h:/bend:
                    let tag = if let Some(colon) = raw.find(':') {
                        let key = raw[..colon].to_lowercase();
                        let val = raw[colon + 1..].trim();
                        match key.as_str() {
                            "from" | "to" | "icon" => format!("{}:{}", key, val),
                            _ => format!("{}:{}", key, val.to_lowercase()),
                        }
                    } else {
                        raw.to_lowercase()
                    };
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
    let (text, tags) = extract_tags(line);
    let mut color = StickyColor::Yellow;
    let mut z_offset = 0.0f32;

    for tag in &tags {
        match tag.as_str() {
            "pink" => color = StickyColor::Pink,
            "green" => color = StickyColor::Green,
            "blue" => color = StickyColor::Blue,
            "purple" => color = StickyColor::Purple,
            "yellow" => color = StickyColor::Yellow,
            _ if tag.starts_with("z:") => {
                if let Ok(v) = tag[2..].trim().parse::<f32>() {
                    z_offset = v;
                }
            }
            _ => {}
        }
    }

    let mut node = Node::new_sticky(color, Pos2::ZERO);
    node.z_offset = z_offset;
    if let NodeKind::StickyNote { text: ref mut t, .. } = node.kind {
        *t = text;
    }
    Ok(node)
}

/// Parse an entity attribute line: `name (type) [PK, FK]`
fn parse_entity_attribute(line: &str) -> EntityAttribute {
    let mut name = line.to_string();
    let mut attr_type = String::new();
    let mut is_pk = false;
    let mut is_fk = false;

    // Extract [PK, FK] suffix
    if let Some(bracket_start) = line.rfind('[') {
        if let Some(bracket_end) = line.rfind(']') {
            let tags_str = &line[bracket_start + 1..bracket_end];
            for part in tags_str.split(',') {
                match part.trim().to_uppercase().as_str() {
                    "PK" | "PRIMARY" | "PRIMARY KEY" => is_pk = true,
                    "FK" | "FOREIGN" | "FOREIGN KEY" => is_fk = true,
                    _ => {}
                }
            }
            name = line[..bracket_start].trim().to_string();
        }
    }

    // Extract (type) — after removing bracket tags
    if let Some(paren_start) = name.rfind('(') {
        if let Some(paren_end) = name.rfind(')') {
            attr_type = name[paren_start + 1..paren_end].trim().to_string();
            name = name[..paren_start].trim().to_string();
        }
    }

    EntityAttribute {
        name,
        attr_type,
        is_primary_key: is_pk,
        is_foreign_key: is_fk,
    }
}

fn tag_to_node_tag(tag: &str) -> Option<NodeTag> {
    match tag {
        "critical" | "crit" | "error" | "danger" => Some(NodeTag::Critical),
        "warning" | "warn" | "caution" => Some(NodeTag::Warning),
        "ok" | "success" | "good" | "done" => Some(NodeTag::Ok),
        "info" | "note" | "information" => Some(NodeTag::Info),
        _ => None,
    }
}

fn tag_to_fill_color(name: &str) -> Option<[u8; 4]> {
    match name {
        "blue"   => Some([137, 180, 250, 255]),
        "green"  => Some([166, 227, 161, 255]),
        "red"    => Some([243, 139, 168, 255]),
        "yellow" => Some([249, 226, 175, 255]),
        "purple" => Some([203, 166, 247, 255]),
        "pink"   => Some([245, 194, 231, 255]),
        "teal"   => Some([148, 226, 213, 255]),
        "white"  => Some([255, 255, 255, 255]),
        "black"  => Some([17, 17, 27, 255]),
        "surface" | "default" => Some([30, 30, 46, 255]),
        _ => None,
    }
}

fn tag_to_edge_color(name: &str) -> Option<[u8; 4]> {
    match name {
        "gray" | "grey"  => Some([100, 100, 100, 255]),
        "blue"           => Some([137, 180, 250, 255]),
        "green"          => Some([166, 227, 161, 255]),
        "red"            => Some([243, 139, 168, 255]),
        "yellow"         => Some([249, 226, 175, 255]),
        "purple"         => Some([203, 166, 247, 255]),
        _ => None,
    }
}

fn fill_color_name(fill: [u8; 4]) -> Option<&'static str> {
    match fill {
        [137, 180, 250, 255] => Some("blue"),
        [166, 227, 161, 255] => Some("green"),
        [243, 139, 168, 255] => Some("red"),
        [249, 226, 175, 255] => Some("yellow"),
        [203, 166, 247, 255] => Some("purple"),
        [245, 194, 231, 255] => Some("pink"),
        [148, 226, 213, 255] => Some("teal"),
        [255, 255, 255, 255] => Some("white"),
        [17, 17, 27, 255]    => Some("black"),
        _ => None,
    }
}

fn edge_color_name(color: [u8; 4]) -> Option<&'static str> {
    match color {
        [100, 100, 100, 255] => Some("gray"),
        [137, 180, 250, 255] => Some("blue"),
        [166, 227, 161, 255] => Some("green"),
        [243, 139, 168, 255] => Some("red"),
        [249, 226, 175, 255] => Some("yellow"),
        [203, 166, 247, 255] => Some("purple"),
        _ => None,
    }
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

fn parse_cardinality(s: &str) -> Cardinality {
    match s {
        "1" => Cardinality::ExactlyOne,
        "0..1" => Cardinality::ZeroOrOne,
        "1..N" | "1..n" | "1..*" => Cardinality::OneOrMany,
        "0..N" | "0..n" | "0..*" => Cardinality::ZeroOrMany,
        _ => Cardinality::None,
    }
}

fn cardinality_str(c: &Cardinality) -> Option<&'static str> {
    match c {
        Cardinality::None => None,
        Cardinality::ExactlyOne => Some("1"),
        Cardinality::ZeroOrOne => Some("0..1"),
        Cardinality::OneOrMany => Some("1..N"),
        Cardinality::ZeroOrMany => Some("0..N"),
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
    fn test_parse_tags_and_3d() {
        let input = r#"
# Tagged Flow

## Nodes
- [api] API Gateway {connector} {z:120} {critical}
- [db] Database {circle} {z:0} {ok} {pinned}
- [cache] Cache Layer {hexagon} {warning}

## Flow
api --> db {dashed}
api --> cache {glow} {thick}
db "sync" --> cache {animated} {arrow:open}
"#;
        let doc = parse_hrf(input).unwrap();
        assert_eq!(doc.title, "Tagged Flow");
        assert_eq!(doc.nodes.len(), 3);

        // Check z-offset and tag on first node
        let api = &doc.nodes[0];
        assert_eq!(api.z_offset, 120.0);
        assert_eq!(api.tag, Some(NodeTag::Critical));
        if let NodeKind::Shape { shape, .. } = &api.kind {
            assert_eq!(*shape, NodeShape::Connector);
        }

        // Check pinned and ok tag on second node
        let db = &doc.nodes[1];
        assert!(db.pinned);
        assert_eq!(db.tag, Some(NodeTag::Ok));

        // Check warning tag
        let cache = &doc.nodes[2];
        assert_eq!(cache.tag, Some(NodeTag::Warning));

        // Check edge styles
        assert!(doc.edges[0].style.dashed);
        assert!(doc.edges[1].style.glow);
        assert!(doc.edges[1].style.width > 4.0); // thick
        assert!(doc.edges[2].style.animated);
        assert_eq!(doc.edges[2].style.arrow_head, ArrowHead::Open);
    }

    #[test]
    fn test_export_preserves_tags() {
        let input = r#"
# Export Test

## Nodes
- [a] Server {connector} {z:50} {critical}
- [b] Client {circle}

## Flow
a "serves" --> b {dashed}
"#;
        let doc = parse_hrf(input).unwrap();
        let exported = export_hrf(&doc, "Export Test");
        assert!(exported.contains("{z:50}"));
        assert!(exported.contains("{critical}"));
        assert!(exported.contains("{connector}"));
        assert!(exported.contains("{dashed}"));
    }

    #[test]
    fn test_node_fill_and_edge_color_tags() {
        let input = r#"
# Color Test

## Nodes
- [a] Server {fill:blue} {w:200} {h:120}
- [b] Client {fill:red} {icon:🖥}

## Flow
a --> b {color:green} {ortho} {bend:0.5}
"#;
        let doc = parse_hrf(input).unwrap();
        assert_eq!(doc.nodes.len(), 2);

        // Node A: blue fill, custom size
        let a = &doc.nodes[0];
        assert_eq!(a.style.fill_color, [137, 180, 250, 255]);
        assert_eq!(a.size[0], 200.0);
        assert_eq!(a.size[1], 120.0);
        // Text should be dark on blue background
        assert_eq!(a.style.text_color[0], 15);

        // Node B: red fill, icon
        let b = &doc.nodes[1];
        assert_eq!(b.style.fill_color, [243, 139, 168, 255]);
        assert_eq!(b.icon, "🖥");

        // Edge: green, ortho, bend
        let e = &doc.edges[0];
        assert_eq!(e.style.color, [166, 227, 161, 255]);
        assert!(e.style.orthogonal);
        assert!((e.style.curve_bend - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_node_style_tags() {
        let input = r#"
# Style Tags

## Nodes
- [a] Bold Node {bold} {shadow} {fill:purple}
- [b] Dashed Border {dashed-border} {italic} {r:12}
"#;
        let doc = parse_hrf(input).unwrap();
        let a = &doc.nodes[0];
        assert!(a.style.bold);
        assert!(a.style.shadow);
        assert_eq!(a.style.fill_color, [203, 166, 247, 255]); // purple

        let b = &doc.nodes[1];
        assert!(b.style.border_dashed);
        assert!(b.style.italic);
        assert!((b.style.corner_radius - 12.0).abs() < 0.1);

        // Round-trip
        let exported = export_hrf(&doc, "Style Tags");
        assert!(exported.contains("{bold}"));
        assert!(exported.contains("{shadow}"));
        assert!(exported.contains("{dashed-border}"));
        assert!(exported.contains("{italic}"));
        assert!(exported.contains("{r:12}"));
    }

    #[test]
    fn test_edge_endpoint_labels_and_styles() {
        let input = r#"
# Endpoint Labels

## Nodes
- [a] Server
- [b] Client

## Flow
a "request" --> b {from:HTTP} {to:REST} {color:blue} {ortho}
"#;
        let doc = parse_hrf(input).unwrap();
        let edge = &doc.edges[0];
        assert_eq!(edge.source_label, "HTTP");
        assert_eq!(edge.target_label, "REST");
        assert_eq!(edge.style.color, [137, 180, 250, 255]); // blue
        assert!(edge.style.orthogonal);

        // Round-trip export
        let exported = export_hrf(&doc, "Endpoint Labels");
        assert!(exported.contains("{from:HTTP}"));
        assert!(exported.contains("{to:REST}"));
        assert!(exported.contains("{color:blue}"));
        assert!(exported.contains("{ortho}"));
    }

    #[test]
    fn test_entity_and_text_nodes() {
        let input = r#"
# ER Diagram

## Nodes
- [users] Users {entity}
  id (uuid) [PK]
  name (varchar)
  email (varchar)
  team_id (uuid) [FK]
- [note] This is a label {text}
- [teams] Teams {entity} {z:100}
  id (uuid) [PK]
  name (varchar)

## Flow
users --> teams
"#;
        let doc = parse_hrf(input).unwrap();
        // 2 entities + 1 text node
        assert_eq!(doc.nodes.len(), 3);

        // Check entity with attributes
        let users = &doc.nodes[0];
        if let NodeKind::Entity { name, attributes } = &users.kind {
            assert_eq!(name, "Users");
            assert_eq!(attributes.len(), 4);
            assert_eq!(attributes[0].name, "id");
            assert_eq!(attributes[0].attr_type, "uuid");
            assert!(attributes[0].is_primary_key);
            assert_eq!(attributes[3].name, "team_id");
            assert!(attributes[3].is_foreign_key);
        } else {
            panic!("Expected entity node");
        }

        // Check text node
        let note = &doc.nodes[1];
        if let NodeKind::Text { content } = &note.kind {
            assert_eq!(content, "This is a label");
        } else {
            panic!("Expected text node");
        }

        // Check z-offset on second entity
        assert_eq!(doc.nodes[2].z_offset, 100.0);
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

    #[test]
    fn test_edge_cardinality_tags() {
        let input = r#"
# ER Test

## Nodes
- [users] Users {entity}
- [orders] Orders {entity}

## Flow
users --> orders {c-src:1} {c-tgt:0..N}
"#;
        let doc = parse_hrf(input).unwrap();
        let edge = &doc.edges[0];
        assert_eq!(edge.source_cardinality, Cardinality::ExactlyOne);
        assert_eq!(edge.target_cardinality, Cardinality::ZeroOrMany);

        // Round-trip
        let exported = export_hrf(&doc, "ER Test");
        assert!(exported.contains("{c-src:1}"), "expected c-src in: {}", exported);
        assert!(exported.contains("{c-tgt:0..N}"), "expected c-tgt in: {}", exported);
    }

    #[test]
    fn test_alignment_and_border_tags() {
        let input = r#"
# Alignment Test

## Nodes
- [a] Left Aligned {align:left} {valign:top}
- [b] Right Aligned {align:right} {valign:bottom}
- [c] Thick Border {border:3}
"#;
        let doc = parse_hrf(input).unwrap();

        let a = &doc.nodes[0];
        assert_eq!(a.style.text_align, crate::model::TextAlign::Left);
        assert_eq!(a.style.text_valign, crate::model::TextVAlign::Top);

        let b = &doc.nodes[1];
        assert_eq!(b.style.text_align, crate::model::TextAlign::Right);
        assert_eq!(b.style.text_valign, crate::model::TextVAlign::Bottom);

        let c = &doc.nodes[2];
        assert!((c.style.border_width - 3.0).abs() < 0.1);

        // Round-trip: non-default alignments and border width should export back
        let exported = export_hrf(&doc, "Alignment Test");
        assert!(exported.contains("{align:left}"), "expected {{align:left}} in: {}", exported);
        assert!(exported.contains("{valign:top}"), "expected {{valign:top}} in: {}", exported);
        assert!(exported.contains("{align:right}"), "expected {{align:right}} in: {}", exported);
        assert!(exported.contains("{valign:bottom}"), "expected {{valign:bottom}} in: {}", exported);
        assert!(exported.contains("{border:3}"), "expected {{border:3}} in: {}", exported);
    }
}
