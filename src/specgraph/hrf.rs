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
/// - [id] Label text {circle} {z:120}       ← 3D layer offset (explicit)
/// - [id] Label text {critical}              ← tag badge
/// - [id] Label text {pinned} {x:100} {y:200} ← pinned to canvas position
///
/// ## Layer 0: Database                     ← named 3D layer section (z = 0)
/// - [db] Database {circle}                 ← all nodes get z=0
///   Stores all user data.                  ← indented description → tooltip
///
/// ## Layer 1: Backend                      ← z = 1 × 120 = 120
/// - [api] API Service {layer:1}            ← {layer:N} = z × 120 (same as section)
///
/// // This is a comment — ignored           ← // line comments supported
///
/// ## Layer 120                             ← explicit z value (> 10 = raw z)
/// - [frontend] Web App {z:240}             ← {z:N} explicit raw z
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
///   Semantic presets (set shape + fill color):
///   `{server}` `{database}` `{cloud}` `{user}` `{service}` `{queue}` `{cache}` `{internet}`
///   `{decision}` `{start}` `{end}` `{process}` `{task}` `{load-balancer}`
///   `{z:N}` — 3D layer offset (positive = closer to camera)
///   `{critical}` `{warning}` `{ok}` `{info}` — status tag badge
///   `{pinned}` — pin node to canvas position
///   `{x:N}` `{y:N}` — explicit canvas position (auto-included when pinned)
///   `{frame}` — group frame container (large translucent background box)
///
/// ### Supported node style tags:
///   `{fill:blue}` — fill color (blue/green/red/yellow/purple/pink/teal/white/black)
///   `{fill:#rrggbb}` — fill color as CSS hex (e.g. `{fill:#1e6f5c}`)
///   `{border-color:red}` or `{stroke:red}` — border/stroke color
///   `{text-color:white}` or `{color:white}` — text color override
///   `{tooltip:text}` or `{tip:text}` or `{desc:text}` — inline description/tooltip text
///   `{size:200x80}` — shorthand for `{w:200} {h:80}`
///   `{pos:X,Y}` — shorthand for `{x:X} {y:Y}` (also pins the node)
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
///   `{weight:N}` — edge weight/importance (1=thin, 2=normal, 3=thick, 4+=very thick)
///
/// ### `## Groups` section
/// ```text
/// ## Groups
/// - [grp_id] Group Label {fill:blue}
///   node_id1, node_id2, node_id3
/// ```
/// Creates a frame node bounding all listed member nodes (after auto-layout).
pub fn parse_hrf(input: &str) -> Result<FlowchartDocument, String> {
    let mut doc = FlowchartDocument::default();
    let mut id_map: HashMap<String, NodeId> = HashMap::new();

    let mut section = Section::None;
    let mut preamble_lines: Vec<String> = Vec::new();
    let mut seen_section = false;

    // Track the last node added in Nodes section for multi-line descriptions
    let mut last_node_id: Option<NodeId> = None;

    // ## Groups section: (group_id, label, fill_color, member_ids)
    let mut groups: Vec<(String, String, Option<[u8;4]>, Vec<String>)> = Vec::new();

    // ## Config section: key = value pairs
    let mut config_map: HashMap<String, String> = HashMap::new();

    for (line_num, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim_end();
        let trimmed = line.trim();

        // `//` line comments — skip entirely
        if trimmed.starts_with("//") {
            continue;
        }

        // Title: # Something
        if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            doc.title = trimmed[2..].trim().to_string();
            continue;
        }

        // Section headers
        if trimmed.starts_with("## ") {
            seen_section = true;
            last_node_id = None;
            // Preserve original case for layer names, lowercase only for matching
            let header_raw = trimmed[3..].trim();
            let header = header_raw.to_lowercase();
            section = match header.as_str() {
                "nodes" | "node" | "components" => Section::Nodes { default_z: 0.0 },
                "flow" | "flows" | "edges" | "connections" => Section::Flow,
                "notes" | "note" | "stickies" => Section::Notes,
                "groups" | "group" | "clusters" => Section::Groups,
                "config" | "settings" | "meta" => Section::Config,
                _ => {
                    // Check for "Layer N" or "Layer N: Name" patterns
                    // "layer 0", "layer 1", "layer 2", ... → z = N * Z_SPACING (120)
                    // "layer z:120", "layer z=120" → z = 120 (explicit)
                    if header.starts_with("layer") {
                        let after_lower = header[5..].trim();
                        let after_raw = header_raw[5..].trim();
                        let z = if after_lower.is_empty() {
                            0.0_f32
                        } else {
                            // Strip ": Name" or "— description" and parse the number
                            let num_part = after_lower.split(':').next()
                                .and_then(|s| { let s = s.trim(); if s.is_empty() { None } else { Some(s) } })
                                .unwrap_or(after_lower.split('—').next().unwrap_or(after_lower).trim());
                            // Explicit "z=N" or "z:N" → use raw value
                            if num_part.starts_with("z=") || num_part.starts_with("z:") {
                                let num_str = &num_part[2..];
                                num_str.parse::<f32>().unwrap_or(0.0)
                            } else {
                                // Plain number: ≤ 10 → layer index (×120), > 10 → raw z
                                if let Ok(v) = num_part.parse::<f32>() {
                                    if v <= 10.0 { v * 120.0 } else { v }
                                } else {
                                    0.0
                                }
                            }
                        };
                        // Store optional layer name: "Layer 1: Frontend" → "Frontend"
                        if let Some(colon_pos) = after_raw.find(':') {
                            let name_part = after_raw[colon_pos+1..].trim();
                            if !name_part.is_empty() {
                                let layer_idx = (z / 120.0).round() as i32;
                                doc.layer_names.insert(layer_idx, name_part.to_string());
                            }
                        }
                        Section::Nodes { default_z: z }
                    } else {
                        Section::None
                    }
                }
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
            Section::Nodes { default_z } => {
                if trimmed.starts_with("- ") {
                    // New node definition
                    let stripped = &trimmed[2..];
                    let (id, mut node) = parse_node_line(stripped, line_num)?;
                    // Apply section default z if node doesn't have an explicit {z:N} tag
                    if node.z_offset == 0.0 && default_z != 0.0 {
                        node.z_offset = default_z;
                    }
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
            Section::Groups => {
                // Format: - [group_id] Group Name {fill:blue}
                //           member1, member2, member3
                if trimmed.starts_with("- ") || trimmed.starts_with("- [") {
                    let stripped = if trimmed.starts_with("- ") { &trimmed[2..] } else { trimmed };
                    if stripped.contains('[') {
                        let id_start = stripped.find('[').unwrap();
                        let id_end = stripped.find(']').unwrap_or(stripped.len());
                        let gid = stripped[id_start+1..id_end].trim().to_string();
                        let rest = stripped[id_end+1..].trim();
                        let (label, tags) = extract_tags(rest);
                        let fill = tags.iter()
                            .find(|t| t.starts_with("fill:"))
                            .and_then(|t| tag_to_fill_color(t[5..].trim()));
                        groups.push((gid, label, fill, Vec::new()));
                    }
                } else if !trimmed.is_empty() && !groups.is_empty() {
                    // Continuation: comma-separated member IDs
                    let last = groups.last_mut().unwrap();
                    for part in trimmed.split(',') {
                        let id = part.trim().to_string();
                        if !id.is_empty() { last.3.push(id); }
                    }
                }
            }
            Section::Config => {
                // Format: key = value  or  key: value
                if !trimmed.is_empty() {
                    let sep = if trimmed.contains('=') { '=' }
                        else if trimmed.contains(':') { ':' }
                        else { continue; };
                    if let Some(pos) = trimmed.find(sep) {
                        let key = trimmed[..pos].trim().to_lowercase();
                        let val = trimmed[pos+1..].trim().to_string();
                        config_map.insert(key, val);
                    }
                }
            }
            Section::None => {}
        }
    }

    doc.description = preamble_lines.join("\n");

    // Apply ## Config values
    for (key, val) in &config_map {
        match key.as_str() {
            "title" => { doc.title = val.clone(); }
            "description" | "desc" => { doc.description = val.clone(); }
            // layer names: layer0 = Data Tier, layer 1 = Backend
            _ if key.starts_with("layer") => {
                let num_part = key.trim_start_matches("layer").trim();
                if let Ok(idx) = num_part.trim_matches(|c: char| !c.is_ascii_digit())
                    .parse::<i32>()
                {
                    doc.layer_names.insert(idx, val.clone());
                }
            }
            _ => {}
        }
    }

    // Auto-layout: topological / hierarchical placement
    super::layout::hierarchical_layout(&mut doc);

    // Create frame nodes for each group (after layout so positions are known)
    for (gid, label, fill_color, member_ids) in groups {
        if member_ids.is_empty() { continue; }
        let pad = 24.0_f32;
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for mid in &member_ids {
            if let Some(&nid) = id_map.get(mid) {
                if let Some(node) = doc.nodes.iter().find(|n| n.id == nid) {
                    let x1 = node.position[0];
                    let y1 = node.position[1];
                    let x2 = x1 + node.size[0];
                    let y2 = y1 + node.size[1];
                    min_x = min_x.min(x1);
                    min_y = min_y.min(y1);
                    max_x = max_x.max(x2);
                    max_y = max_y.max(y2);
                }
            }
        }
        if min_x == f32::INFINITY { continue; }
        let mut frame = Node::new_frame(egui::Pos2::new(min_x - pad, min_y - pad));
        frame.size = [max_x - min_x + pad * 2.0, max_y - min_y + pad * 2.0];
        if let NodeKind::Shape { label: ref mut l, .. } = frame.kind {
            *l = label;
        }
        if let Some(fc) = fill_color {
            frame.style.fill_color = fc;
        }
        // Insert at the beginning so frames appear behind other nodes
        doc.nodes.insert(0, frame);
        let _ = gid; // frame has a new id; group-id is not tracked after creation
    }

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
    // Group by z_offset: if multiple layers exist, emit ## Layer z=N sections.
    let shape_nodes: Vec<&Node> = doc
        .nodes
        .iter()
        .filter(|n| !matches!(n.kind, NodeKind::StickyNote { .. }))
        .collect();

    if !shape_nodes.is_empty() {
        // Collect distinct z-offsets (preserve insertion order via Vec dedup)
        let mut z_groups: Vec<f32> = Vec::new();
        for n in &shape_nodes {
            let z = n.z_offset;
            if !z_groups.iter().any(|&g| (g - z).abs() < 0.5) {
                z_groups.push(z);
            }
        }
        z_groups.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let use_layers = z_groups.len() > 1;

        for section_z in &z_groups {
            let group: Vec<&Node> = shape_nodes
                .iter()
                .copied()
                .filter(|n| (n.z_offset - section_z).abs() < 0.5)
                .collect();

            if use_layers {
                // Use natural index (0,1,2...) when z is a multiple of Z_SPACING (120),
                // otherwise use explicit "z=N" notation to avoid the index heuristic.
                let z_spacing = 120.0_f32;
                let idx = (section_z / z_spacing).round();
                let is_multiple = (section_z - idx * z_spacing).abs() < 0.5;
                let layer_key = idx as i32;
                let name_suffix = doc.layer_names.get(&layer_key)
                    .map(|n| format!(": {}", n))
                    .unwrap_or_default();
                if is_multiple {
                    out.push_str(&format!("## Layer {}{}\n", idx as i32, name_suffix));
                } else {
                    out.push_str(&format!("## Layer z={}{}\n", section_z, name_suffix));
                }
            } else {
                out.push_str("## Nodes\n");
            }

            for node in group {
                let id = id_map.get(&node.id).cloned().unwrap_or_default();
                // Only emit z_tag if the node's z differs from the section default
                let z_tag = if (node.z_offset - section_z).abs() > 0.5 {
                    format!(" {{z:{}}}", node.z_offset)
                } else { String::new() };
                export_node_to_hrf(node, &id, &z_tag, &mut out);
            }
            out.push('\n');
        }
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
            } else {
                let ec = edge.style.color;
                let default_ec = [150_u8, 150, 170, 255];
                if ec != default_ec {
                    style_tags.push(format!("color:#{:02x}{:02x}{:02x}", ec[0], ec[1], ec[2]));
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
    /// `default_z` is applied to any node in this section that doesn't have an explicit {z:N} tag.
    Nodes { default_z: f32 },
    Flow,
    Notes,
    Groups,
    Config,
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
/// Also supports `->` as a shorter alias and `<--` / `<->` for reverse/bidirectional.
/// Splits into individual edges.
fn parse_flow_line_chain(
    line: &str,
    id_map: &HashMap<String, NodeId>,
    line_num: usize,
) -> Result<Vec<Edge>, String> {
    // Normalize arrow variants to "-->":
    //   "->"  → "-->"   (shorter alias)
    //   "<--" → split into reversed edge
    //   "<->" → bidirectional (forward + backward)
    // We handle these by normalizing the input first.
    // Strategy: replace "-->" variants then split.
    //
    // Order matters: replace "<->" first (bidirectional), then "<--" (reverse), then "->"
    // We convert "<->" to "-->" but set a flag; convert "<--" by reversing.
    // Simplest approach: expand bidirectional into two lines by replacing `<->` with `-->`.
    // Then for `<--` lines, reverse direction.

    // Detect dominant arrow type in this line
    let is_reverse = !line.contains("-->") && !line.contains("<->") && line.contains("<--");
    let is_bidir = line.contains("<->");

    // Normalize to use "-->" for all splits
    let normalized = if is_bidir {
        line.replace("<->", "-->").replace("<-->", "-->")
    } else if is_reverse {
        line.replace("<--", "-->")
    } else {
        // Support "->" as alias for "-->" (replace only bare "->", not "-->")
        // Do this carefully: replace "-->" temporarily, sub "->", restore
        line.replace("-->", "\x00ARROW\x00")
            .replace("->", "-->")
            .replace("\x00ARROW\x00", "-->")
    };

    // Split on "-->" but preserve quoted labels that precede arrows
    // Strategy: tokenise by splitting on "-->" then pair up segments
    let segments: Vec<&str> = normalized.split("-->").collect();
    if segments.len() < 2 {
        return Err(format!("Line {}: expected '-->' or '->' in flow definition", line_num + 1));
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
            let hint = suggest_id(&from_id, id_map.keys().map(|s| s.as_str()));
            format!("Line {}: unknown node id '{}'{}", line_num + 1, from_id, hint)
        })?;
        let target_node_id = id_map.get(&to_id).ok_or_else(|| {
            let hint = suggest_id(&to_id, id_map.keys().map(|s| s.as_str()));
            format!("Line {}: unknown node id '{}'{}", line_num + 1, to_id, hint)
        })?;

        // For reverse arrows (<--), swap source and target
        let (actual_source_id, actual_target_id) = if is_reverse {
            (target_node_id, source_node_id)
        } else {
            (source_node_id, target_node_id)
        };
        let mut src_side = PortSide::Bottom;
        let mut tgt_side = PortSide::Top;
        // Pre-scan for port overrides before creating port structs
        for etag in &edge_tags {
            if etag.starts_with("src-port:") || etag.starts_with("sport:") {
                let key_len = if etag.starts_with("src-port:") { 9 } else { 6 };
                if let Some(ps) = tag_to_port_side(&etag[key_len..]) {
                    src_side = ps;
                }
            } else if etag.starts_with("tgt-port:") || etag.starts_with("tport:") {
                let key_len = if etag.starts_with("tgt-port:") { 9 } else { 6 };
                if let Some(ps) = tag_to_port_side(&etag[key_len..]) {
                    tgt_side = ps;
                }
            }
        }
        let source = Port { node_id: *actual_source_id, side: src_side };
        let target = Port { node_id: *actual_target_id, side: tgt_side };
        let mut edge = Edge::new(source, target);
        edge.label = label.clone();
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
            } else if etag.starts_with("weight:") || etag.starts_with("w:") {
                let v = if etag.starts_with("weight:") { &etag[7..] } else { &etag[2..] };
                if let Ok(w) = v.trim().parse::<f32>() {
                    // weight 1=1.5px, 2=3px, 3=5px, 4+=7px
                    edge.style.width = (w * 1.8).clamp(1.0, 9.0);
                }
            } else if etag.starts_with("from:") {
                edge.source_label = etag[5..].trim().to_string();
            } else if etag.starts_with("to:") {
                edge.target_label = etag[3..].trim().to_string();
            } else if etag.starts_with("c-src:") {
                edge.source_cardinality = parse_cardinality(etag[6..].trim());
            } else if etag.starts_with("c-tgt:") {
                edge.target_cardinality = parse_cardinality(etag[6..].trim());
            } else if etag.starts_with("src-port:") || etag.starts_with("sport:")
                    || etag.starts_with("tgt-port:") || etag.starts_with("tport:") {
                // Already handled above
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

        // Bidirectional: also add a reversed edge
        if is_bidir {
            let rev_source = Port { node_id: *actual_target_id, side: PortSide::Bottom };
            let rev_target = Port { node_id: *actual_source_id, side: PortSide::Top };
            let mut rev_edge = Edge::new(rev_source, rev_target);
            // Share the same style as the forward edge (use last edge pushed)
            if let Some(fwd) = edges.last() {
                rev_edge.style = fwd.style.clone();
            }
            edges.push(rev_edge);
        }
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
    let mut is_frame = false;
    let mut pos_x: Option<f32> = None;
    let mut pos_y: Option<f32> = None;
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
    let mut opacity_override: Option<f32> = None;
    let mut gradient = false;
    let mut locked = false;
    let mut url_override: Option<String> = None;
    let mut border_color: Option<[u8; 4]> = None;
    let mut text_color: Option<[u8; 4]> = None;
    let mut tooltip_text: Option<String> = None;
    for tag in &tags {
        if tag.starts_with("z:") {
            if let Ok(z) = tag[2..].trim().parse::<f32>() {
                z_offset = z;
            }
        } else if tag.starts_with("layer:") {
            // {layer:N} is a human-friendly alias for z = N * 120
            if let Ok(v) = tag[6..].trim().parse::<f32>() {
                z_offset = v * 120.0;
            }
        } else if tag.starts_with("fill:") {
            fill_color = tag_to_fill_color(tag[5..].trim());
        } else if tag.starts_with("size:") {
            // {size:200x80} shorthand for {w:200} {h:80}
            let dims = tag[5..].trim();
            if let Some(x_pos) = dims.find('x') {
                width_override  = dims[..x_pos].parse::<f32>().ok();
                height_override = dims[x_pos+1..].parse::<f32>().ok();
            }
        } else if tag.starts_with("pos:") {
            // {pos:100,200} shorthand for {x:100} {y:200} + pinned
            let coords = tag[4..].trim();
            if let Some(comma) = coords.find(',') {
                pos_x = coords[..comma].trim().parse::<f32>().ok();
                pos_y = coords[comma+1..].trim().parse::<f32>().ok();
                if pos_x.is_some() && pos_y.is_some() { pinned = true; }
            }
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
        } else if tag == "frame" || tag == "group" || tag == "container" {
            is_frame = true;
        } else if tag.starts_with("x:") {
            pos_x = tag[2..].trim().parse::<f32>().ok();
        } else if tag.starts_with("y:") {
            pos_y = tag[2..].trim().parse::<f32>().ok();
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
        } else if tag.starts_with("opacity:") || tag.starts_with("alpha:") {
            let val_str = if tag.starts_with("opacity:") { &tag[8..] } else { &tag[6..] };
            if let Ok(v) = val_str.trim().parse::<f32>() {
                // Accept 0-100 percentage or 0.0-1.0 float
                opacity_override = Some(if v > 1.0 { v / 100.0 } else { v });
            }
        } else if tag == "gradient" || tag == "grad" {
            gradient = true;
        } else if tag == "locked" || tag == "lock" {
            locked = true;
        } else if tag.starts_with("url:") || tag.starts_with("link:") {
            let prefix_len = if tag.starts_with("url:") { 4 } else { 5 };
            url_override = Some(tag[prefix_len..].trim().to_string());
        } else if tag.starts_with("tooltip:") || tag.starts_with("tip:") || tag.starts_with("desc:") {
            let prefix = if tag.starts_with("tooltip:") { 8 } else if tag.starts_with("tip:") { 4 } else { 5 };
            tooltip_text = Some(tag[prefix..].trim().to_string());
        } else if tag.starts_with("border-color:") || tag.starts_with("stroke:") {
            let v = if tag.starts_with("border-color:") { &tag[13..] } else { &tag[7..] };
            border_color = tag_to_fill_color(v.trim());
        } else if tag.starts_with("text-color:") || tag.starts_with("color:") {
            let v = if tag.starts_with("text-color:") { &tag[11..] } else { &tag[6..] };
            text_color = tag_to_fill_color(v.trim());
        } else if tag == "shadow" || tag == "drop-shadow" {
            shadow = true;
        } else if tag == "bold" || tag == "strong" {
            bold = true;
        } else if tag == "italic" || tag == "em" {
            italic = true;
        } else if tag == "dashed-border" || tag == "dashed_border" || tag == "border-dashed" {
            dashed_border = true;
        } else if let Some((preset_shape, preset_color)) = tag_to_preset(tag) {
            // Semantic preset: sets shape AND fill color at once
            shape = preset_shape;
            if fill_color.is_none() {
                fill_color = Some(preset_color);
            }
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
    node.is_frame = is_frame;
    // Apply explicit position (used when {pinned} {x:N} {y:N} are present)
    if let Some(x) = pos_x { node.position[0] = x; }
    if let Some(y) = pos_y { node.position[1] = y; }
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
    if let Some(op) = opacity_override { node.style.opacity = op.clamp(0.0, 1.0); }
    if gradient { node.style.gradient = true; }
    if locked { node.locked = true; }
    if let Some(u) = url_override { node.url = u; }
    if let Some(bc) = border_color { node.style.border_color = bc; }
    if let Some(tc) = text_color { node.style.text_color = tc; }
    if let Some(tt) = tooltip_text {
        if let NodeKind::Shape { description, .. } = &mut node.kind {
            if description.is_empty() { *description = tt; }
        }
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
                    let raw = tag_buf.trim().to_string();
                    // For key:value tags, lowercase the key; preserve value case for
                    // from:/to:/icon: but lowercase for fill:/color:/arrow:/z:/w:/h:/bend:
                    let tag = if let Some(colon) = raw.find(':') {
                        let key = raw[..colon].to_lowercase();
                        let val = raw[colon + 1..].trim();
                        match key.as_str() {
                            // Preserve original case for these values
                            "from" | "to" | "icon" | "url" | "link"
                            | "tooltip" | "tip" | "desc" => {
                                format!("{}:{}", key, val)
                            }
                            // Preserve fill/color values that start with '#' (hex colors)
                            "fill" | "color" if val.starts_with('#') => {
                                format!("{}:{}", key, val)
                            }
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
        _ => parse_hex_color(name),
    }
}

/// Parse a CSS-style hex color: `#rgb`, `#rrggbb`, or `#rrggbbaa`.
fn parse_hex_color(s: &str) -> Option<[u8; 4]> {
    let s = s.trim().strip_prefix('#')?;
    match s.len() {
        3 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            Some([r, g, b, 255])
        }
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some([r, g, b, 255])
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some([r, g, b, a])
        }
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
        _ => parse_hex_color(name),
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

/// Semantic preset → (shape, fill_color). Returns None if tag is not a preset.
fn tag_to_preset(tag: &str) -> Option<(NodeShape, [u8; 4])> {
    match tag {
        // Infrastructure
        "server"    => Some((NodeShape::Rectangle,   [243, 139, 168, 255])), // red
        "database"  | "db" | "storage"
                    => Some((NodeShape::Circle,      [137, 180, 250, 255])), // blue
        "cloud"     => Some((NodeShape::Hexagon,     [148, 226, 213, 255])), // teal
        "user"      | "actor" | "person"
                    => Some((NodeShape::Circle,      [203, 166, 247, 255])), // purple
        "service"   | "microservice"
                    => Some((NodeShape::RoundedRect, [166, 227, 161, 255])), // green
        "queue"     | "mq" | "broker"
                    => Some((NodeShape::Parallelogram, [249, 226, 175, 255])), // yellow
        "load-balancer" | "lb" | "proxy"
                    => Some((NodeShape::Hexagon,     [245, 194, 231, 255])), // pink
        "cache"     | "redis"
                    => Some((NodeShape::RoundedRect, [249, 226, 175, 255])), // yellow
        "internet"  | "external"
                    => Some((NodeShape::Parallelogram, [137, 180, 250, 255])), // blue
        "decision"  | "branch"
                    => Some((NodeShape::Diamond,     [249, 226, 175, 255])), // yellow
        "start"     | "end" | "terminal"
                    => Some((NodeShape::Circle,      [166, 227, 161, 255])), // green
        "process"   | "task" | "step"
                    => Some((NodeShape::RoundedRect, [137, 180, 250, 255])), // blue
        _ => None,
    }
}

/// Format a single non-sticky node into HRF text and append to `out`.
/// `z_tag` is pre-computed so callers can suppress it when a section already implies the z.
fn export_node_to_hrf(node: &Node, id: &str, z_tag: &str, out: &mut String) {
    match &node.kind {
        NodeKind::Shape { shape, label, description } => {
            let shape_tag = if node.is_frame {
                " {frame}"
            } else {
                match shape {
                    NodeShape::Rectangle => "",
                    NodeShape::RoundedRect => "",
                    NodeShape::Diamond => " {diamond}",
                    NodeShape::Circle => " {circle}",
                    NodeShape::Parallelogram => " {parallelogram}",
                    NodeShape::Hexagon => " {hexagon}",
                    NodeShape::Connector => " {connector}",
                }
            };
            let tag_tag = match node.tag {
                Some(NodeTag::Critical) => " {critical}",
                Some(NodeTag::Warning) => " {warning}",
                Some(NodeTag::Ok) => " {ok}",
                Some(NodeTag::Info) => " {info}",
                None => "",
            };
            let pin_tag = if node.pinned {
                format!(" {{pos:{:.0},{:.0}}}", node.position[0], node.position[1])
            } else { String::new() };
            let fill_tag = if let Some(name) = fill_color_name(node.style.fill_color) {
                format!(" {{fill:{}}}", name)
            } else {
                let fc = node.style.fill_color;
                let default_fill = [30_u8, 30, 46, 255];
                if fc != default_fill {
                    format!(" {{fill:#{:02x}{:02x}{:02x}}}", fc[0], fc[1], fc[2])
                } else { String::new() }
            };
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
            let opacity_tag = if (node.style.opacity - 1.0).abs() > 0.01 {
                format!(" {{opacity:{:.0}}}", node.style.opacity * 100.0)
            } else { String::new() };
            let gradient_tag = if node.style.gradient { " {gradient}" } else { "" };
            let locked_tag = if node.locked { " {locked}" } else { "" };
            let url_tag = if !node.url.is_empty() {
                format!(" {{url:{}}}", node.url)
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
            // Border color and text color (only if non-default)
            let default_border = [100_u8, 100, 140, 255];
            let default_text   = [220_u8, 220, 230, 255];
            let border_color_tag = if node.style.border_color != default_border {
                if let Some(name) = fill_color_name(node.style.border_color) {
                    format!(" {{border-color:{}}}", name)
                } else {
                    let bc = node.style.border_color;
                    format!(" {{border-color:#{:02x}{:02x}{:02x}}}", bc[0], bc[1], bc[2])
                }
            } else { String::new() };
            let text_color_tag = if node.style.text_color != default_text {
                if let Some(name) = fill_color_name(node.style.text_color) {
                    format!(" {{text-color:{}}}", name)
                } else {
                    let tc = node.style.text_color;
                    format!(" {{text-color:#{:02x}{:02x}{:02x}}}", tc[0], tc[1], tc[2])
                }
            } else { String::new() };
            out.push_str(&format!("- [{}] {}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}\n",
                id, label, shape_tag, z_tag, tag_tag, pin_tag, fill_tag, icon_tag,
                gradient_tag, shadow_tag, bold_tag, italic_tag, dashed_border_tag, radius_tag,
                border_tag, opacity_tag, locked_tag, url_tag, align_tag, valign_tag,
                border_color_tag, text_color_tag, w_tag, h_tag));
            if !description.is_empty() {
                for desc_line in description.lines() {
                    out.push_str(&format!("  {}\n", desc_line));
                }
            }
        }
        NodeKind::Entity { name, attributes } => {
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
            out.push_str(&format!("- [{}] {} {{text}}{}\n", id, content, z_tag));
        }
        _ => {}
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

/// Parse a port side name into a PortSide enum value.
fn tag_to_port_side(s: &str) -> Option<PortSide> {
    match s.trim().to_lowercase().as_str() {
        "top" | "t" => Some(PortSide::Top),
        "bottom" | "bot" | "b" => Some(PortSide::Bottom),
        "left" | "l" => Some(PortSide::Left),
        "right" | "r" => Some(PortSide::Right),
        _ => None,
    }
}

/// Suggest similar IDs for better error messages using simple prefix/substring matching.
fn suggest_id<'a>(bad_id: &str, candidates: impl Iterator<Item = &'a str>) -> String {
    let bad_lower = bad_id.to_lowercase();
    let matches: Vec<&str> = candidates
        .filter(|c| {
            let cl = c.to_lowercase();
            cl.contains(&bad_lower[..bad_lower.len().min(3)]) || bad_lower.contains(&cl[..cl.len().min(3)])
        })
        .take(3)
        .collect();
    if matches.is_empty() {
        " — define it in ## Nodes section".to_string()
    } else {
        format!(" — did you mean: {}?", matches.join(", "))
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
        // With multi-layer export, z:50 is expressed as a ## Layer 50 section
        // (rather than inline {z:50} tags on each node).
        assert!(exported.contains("## Layer 50") || exported.contains("## Layer z=50") || exported.contains("{z:50}"),
            "expected z:50 info in: {}", exported);
        assert!(exported.contains("{critical}"));
        assert!(exported.contains("{connector}"));
        assert!(exported.contains("{dashed}"));
        // Verify the layer section round-trips correctly
        let doc2 = parse_hrf(&exported).unwrap();
        let server = doc2.nodes.iter().find(|n| n.display_label() == "Server").expect("Server node");
        assert!((server.z_offset - 50.0).abs() < 1.0,
            "Server z_offset should be 50 after round-trip, got {}", server.z_offset);
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
    fn test_layer_sections_3d() {
        // ## Layer N sections should assign z-offset to all nodes in that section
        let input = r#"
# Layered Architecture

## Layer 0
- [db] Database {circle}
- [cache] Redis Cache

## Layer 1
- [api] API Service
- [auth] Auth Service

## Layer 2
- [web] Web Frontend {parallelogram}

## Flow
api --> db
auth --> db
web --> api
"#;
        let doc = parse_hrf(input).unwrap();
        assert_eq!(doc.nodes.len(), 5);

        let db = doc.nodes.iter().find(|n| n.display_label() == "Database").unwrap();
        let api = doc.nodes.iter().find(|n| n.display_label() == "API Service").unwrap();
        let web = doc.nodes.iter().find(|n| n.display_label() == "Web Frontend").unwrap();

        assert!((db.z_offset - 0.0).abs() < 1.0, "Layer 0 should give z=0, got {}", db.z_offset);
        assert!((api.z_offset - 120.0).abs() < 1.0, "Layer 1 should give z=120, got {}", api.z_offset);
        assert!((web.z_offset - 240.0).abs() < 1.0, "Layer 2 should give z=240, got {}", web.z_offset);

        // Round-trip: multi-layer export should use ## Layer sections
        let exported = export_hrf(&doc, "Layered Architecture");
        assert!(exported.contains("## Layer"), "expected layer sections in: {}", exported);

        let doc2 = parse_hrf(&exported).unwrap();
        let api2 = doc2.nodes.iter().find(|n| n.display_label() == "API Service").unwrap();
        assert!((api2.z_offset - 120.0).abs() < 1.0,
            "API z_offset after round-trip: {}", api2.z_offset);
    }

    #[test]
    fn test_pinned_position_roundtrip() {
        // When a node is pinned its position should survive export/import
        let input = r#"
# Pin Test

## Nodes
- [a] Fixed Node {pinned} {x:250} {y:180}
- [b] Free Node
"#;
        let doc = parse_hrf(input).unwrap();
        let a = &doc.nodes[0];
        assert!(a.pinned);
        assert!((a.position[0] - 250.0).abs() < 1.0, "x mismatch: {}", a.position[0]);
        assert!((a.position[1] - 180.0).abs() < 1.0, "y mismatch: {}", a.position[1]);

        let b = &doc.nodes[1];
        assert!(!b.pinned);

        // Round-trip: exported spec must contain position tags (now as {pos:X,Y})
        let exported = export_hrf(&doc, "Pin Test");
        assert!(
            exported.contains("{pos:250,180}") || exported.contains("{pinned}"),
            "missing position in: {}", exported
        );

        // Re-import should preserve position
        let doc2 = parse_hrf(&exported).unwrap();
        let a2 = &doc2.nodes[0];
        assert!(a2.pinned);
        assert!((a2.position[0] - 250.0).abs() < 1.0);
        assert!((a2.position[1] - 180.0).abs() < 1.0);
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

    #[test]
    fn test_named_layers_and_comments() {
        let input = r#"
# Named Layer Test
// This is a comment and should be ignored
## Layer 0: Database
// Another comment
- [db] Main DB {circle}

## Layer 1: Backend
- [api] API Service

## Layer 2: Frontend
- [web] Web App {parallelogram}

## Flow
api --> db
web --> api
"#;
        let doc = parse_hrf(input).unwrap();
        assert_eq!(doc.nodes.len(), 3);

        // Layer names should be stored
        assert_eq!(doc.layer_names.get(&0), Some(&"Database".to_string()));
        assert_eq!(doc.layer_names.get(&1), Some(&"Backend".to_string()));
        assert_eq!(doc.layer_names.get(&2), Some(&"Frontend".to_string()));

        // z-offsets should be correct (0=0, 1=120, 2=240)
        let db = doc.nodes.iter().find(|n| n.display_label() == "Main DB").unwrap();
        let api = doc.nodes.iter().find(|n| n.display_label() == "API Service").unwrap();
        let web = doc.nodes.iter().find(|n| n.display_label() == "Web App").unwrap();
        assert!((db.z_offset - 0.0).abs() < 1.0);
        assert!((api.z_offset - 120.0).abs() < 1.0);
        assert!((web.z_offset - 240.0).abs() < 1.0);

        // Export should include layer names
        let exported = export_hrf(&doc, "Named Layer Test");
        assert!(exported.contains("## Layer 0: Database") || exported.contains("Database"),
            "expected Database name in: {}", exported);
        assert!(exported.contains("## Layer 1: Backend") || exported.contains("Backend"),
            "expected Backend name in: {}", exported);
    }

    #[test]
    fn test_arrow_aliases() {
        // -> and <-- and <-> arrow variants
        let input = r#"
# Arrow Test

## Nodes
- [a] Node A
- [b] Node B
- [c] Node C
- [d] Node D

## Flow
a -> b
c <-- d
a <-> c
"#;
        let doc = parse_hrf(input).unwrap();
        // a->b: 1 edge from a to b
        // c<--d: 1 edge from d to c (reversed)
        // a<->c: 2 edges (a->c and c->a)
        assert_eq!(doc.edges.len(), 4, "expected 4 edges, got {}", doc.edges.len());

        let a = doc.nodes.iter().find(|n| n.display_label() == "Node A").unwrap().id;
        let b = doc.nodes.iter().find(|n| n.display_label() == "Node B").unwrap().id;
        let c = doc.nodes.iter().find(|n| n.display_label() == "Node C").unwrap().id;
        let d = doc.nodes.iter().find(|n| n.display_label() == "Node D").unwrap().id;

        // a->b
        assert!(doc.edges.iter().any(|e| e.source.node_id == a && e.target.node_id == b),
            "expected a->b edge");
        // c<--d means d->c
        assert!(doc.edges.iter().any(|e| e.source.node_id == d && e.target.node_id == c),
            "expected d->c edge (from c<--d)");
        // a<->c creates a->c and c->a
        assert!(doc.edges.iter().any(|e| e.source.node_id == a && e.target.node_id == c),
            "expected a->c edge (from a<->c)");
        assert!(doc.edges.iter().any(|e| e.source.node_id == c && e.target.node_id == a),
            "expected c->a edge (from a<->c)");
    }

    #[test]
    fn test_hex_color_and_size_pos_shorthands() {
        let input = r#"
# Hex Test

## Nodes
- [a] Node A {fill:#ff6600} {size:200x90}
- [b] Node B {fill:#abc} {pos:100,200}
- [c] Node C {fill:#1a2b3c4d}

## Flow
a --> b
"#;
        let doc = parse_hrf(input).unwrap();

        let a = doc.nodes.iter().find(|n| n.display_label() == "Node A").unwrap();
        assert_eq!(a.style.fill_color, [0xff, 0x66, 0x00, 0xff], "hex 6-digit fill");
        assert!((a.size[0] - 200.0).abs() < 1.0, "size width: {}", a.size[0]);
        assert!((a.size[1] - 90.0).abs() < 1.0, "size height: {}", a.size[1]);

        let b = doc.nodes.iter().find(|n| n.display_label() == "Node B").unwrap();
        assert_eq!(b.style.fill_color, [0xaa, 0xbb, 0xcc, 0xff], "hex 3-digit fill");
        assert!(b.pinned, "pos: shorthand should pin");
        assert!((b.position[0] - 100.0).abs() < 1.0, "pos x: {}", b.position[0]);
        assert!((b.position[1] - 200.0).abs() < 1.0, "pos y: {}", b.position[1]);

        let c = doc.nodes.iter().find(|n| n.display_label() == "Node C").unwrap();
        assert_eq!(c.style.fill_color, [0x1a, 0x2b, 0x3c, 0x4d], "hex 8-digit fill with alpha");

        // Export should round-trip hex colors
        let exported = export_hrf(&doc, "Hex Test");
        assert!(exported.contains("{fill:#ff6600}"), "hex in export: {}", exported);
    }

    #[test]
    fn test_groups_section_creates_frame() {
        let input = r#"
# Group Test

## Nodes
- [a] Alpha
- [b] Beta
- [c] Gamma

## Flow
a --> b
b --> c

## Groups
- [g1] Backend Cluster {fill:blue}
  a, b
"#;
        let doc = parse_hrf(input).unwrap();

        // Should have 4 nodes: a, b, c + 1 frame
        assert_eq!(doc.nodes.len(), 4, "expected 4 nodes (3 + 1 frame): {:?}",
            doc.nodes.iter().map(|n| n.display_label()).collect::<Vec<_>>());

        // The frame should be first (inserted at index 0)
        assert!(doc.nodes[0].is_frame, "first node should be a frame");
        assert_eq!(doc.nodes[0].display_label(), "Backend Cluster");

        // Frame should bound the two member nodes
        let frame = &doc.nodes[0];
        let a = doc.nodes.iter().find(|n| n.display_label() == "Alpha").unwrap();
        let b = doc.nodes.iter().find(|n| n.display_label() == "Beta").unwrap();
        assert!(frame.position[0] <= a.position[0].min(b.position[0]),
            "frame left should be <= member left");
        assert!(frame.position[1] <= a.position[1].min(b.position[1]),
            "frame top should be <= member top");
    }
}
