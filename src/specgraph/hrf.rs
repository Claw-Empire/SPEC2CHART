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
/// ## Nodes  (or: Components / Services / Modules / Architecture / Actors / …)
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
/// ## Flow  (or: Edges / Connections / Dependencies / Links / Interactions / …)
/// id "label" --> id
/// id --> id
/// id "label" --> id {dashed}                ← dashed edge
/// id --> id {glow}                          ← glowing edge
/// id --> id {arrow:open}                    ← arrow head style
/// [a, b, c] -> target                       ← multi-source fan-in (same as a->target, b->target, c->target)
/// source -> [a, b, c] {dashed}              ← multi-target fan-out (tags applied to all edges)
/// a -> b  // inline comment                 ← // after space is stripped before parsing
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
///   `{layer:N}` / `{level:N}` / `{tier:N}` — 3D layer as index × 120 (N=0,1,2…)
///   `{layer:db}` `{layer:api}` `{layer:frontend}` `{layer:edge}` `{layer:infra}` — semantic tier names
///   `{back}` `{far}` `{mid}` `{near}` `{front}` — 3D depth shortcuts (z=0/120/240/360/480)
///   `{3d-depth:N}` / `{depth:N}` — custom extrusion thickness in 3D view (world units; default 40)
///   `{critical}` `{warning}` `{ok}` `{info}` — status tag badge
///   `{badge:text}` / `{v:text}` — text/version badge (alias for icon)
///   `{pinned}` — pin node to canvas position
///   `{x:N}` `{y:N}` — explicit canvas position (auto-included when pinned)
///   `{frame}` — group frame container (large translucent background box)
///   `{collapsed}` / `{compact}` — render node as a compact pill (collapsed state)
///
/// ### Supported node style tags:
///   `{fill:blue}` — fill color (blue/green/red/yellow/purple/pink/teal/orange/sky/lavender/gray/mauve/white/black/none)
///   `{fill:#rrggbb}` — fill color as CSS hex (e.g. `{fill:#1e6f5c}`)
///   `{fill:none}` — transparent fill (border only)
///   `{border-color:red}` or `{stroke:red}` — border/stroke color
///   `{text-color:white}` or `{color:white}` — text color override
///   `{tooltip:text}` or `{tip:text}` or `{desc:text}` — inline description/tooltip text
///   `{size:200x80}` — shorthand for `{w:200} {h:80}`
///   `{tiny}` / `{small}` / `{medium}` / `{large}` / `{xlarge}` — size presets (70–300 × 36–140)
///   `{wide}` — wide-and-short preset (240×60); `{tall}` — narrow-and-tall preset (80×180)
///   `{pos:X,Y}` — shorthand for `{x:X} {y:Y}` (also pins the node)
///   `{w:200}` — explicit width in canvas units
///   `{h:100}` — explicit height in canvas units
///   `{icon:🔒}` — icon badge (or bare `{🔒}` — bare emoji shorthand, no prefix needed)
///   `{shadow}` — drop shadow effect
///   `{highlight}` / `{pulse}` / `{starred}` / `{important}` — slow amber pulsing ring (important node marker)
///   `{glow}` / `{neon}` — neon glow effect on node border
///   `{shape:circle}` / `{type:diamond}` / `{kind:hexagon}` — explicit property-style shape (same as shorthand)
///   `{status:done}` / `{status:wip}` / `{status:blocked}` — property-style status (same as shorthand)
///   `{done}` / `{complete}` — progress=100%, Ok badge (green)
///   `{wip}` / `{in-progress}` / `{doing}` — progress=50%, Info badge (blue)
///   `{review}` / `{in-review}` — progress=75%, Warning badge (yellow)
///   `{blocked}` / `{stuck}` / `{failed}` — Critical badge (red), no progress implied
///   `{todo}` / `{pending}` / `{backlog}` — Warning badge (yellow), no progress
///   `{note:text}` / `{annotation:text}` / `{comment:text}` — 💬 annotation tooltip shown on hover
///   `{section:Name}` / `{stage:Name}` / `{col:Name}` — assign to a kanban section inline (alias: column:/board:)
///   `{group:name}` / `{cluster:name}` / `{in:name}` — assign to inline group frame (auto-creates bounding frame)
///   `{bold}` — bold text
///   `{italic}` — italic text
///   `{dashed-border}` — dashed border line
///   `{r:N}` / `{radius:N}` / `{corner:N}` — corner radius override
///   `{rounded}` — corner radius 12 (shorthand)
///   `{pill-shape}` — fully rounded corners (radius 50)
///   `{sharp}` / `{square}` — zero corner radius
///   `{border:N}` — border width (e.g. `{border:2.5}`)
///   `{text-size:N}` / `{font-size:N}` / `{fs:N}` — font size override
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
///   `{note:text}` / `{comment:text}` — annotation shown as tooltip when hovering the edge
///   Indented line after an edge → sets the edge comment (multi-word description):
///   ```
///   api -> db
///     Reads and writes user records. Uses connection pooling.
///   ```
///
/// ### `## Style` section
/// ```text
/// ## Style
/// primary = {fill:blue} {highlight}
/// danger  = {fill:red} {bold}
/// muted   = fill:teal opacity:0.7
/// ```
/// Named style templates expanded into full tag sets. Use `{style_name}` in any node line.
/// Value can use `{}` tags explicitly or bare `key:value` pairs (auto-wrapped in `{}`).
///
/// ### `## Palette` section
/// ```text
/// ## Palette
/// primary = #1e6f5c
/// warning = #f28b30
/// accent  = blue
/// ```
/// Named color aliases for use in `{fill:primary}`, `{color:warning}`, `{border-color:accent}`.
///
/// ### `## Steps` section
/// ```text
/// ## Steps
/// 1. User submits form {diamond}
/// 2. Validate data
/// 3. Save to database {fill:green}
/// ```
/// Creates nodes with auto-generated IDs (step1, step2, …) and sequential edges between them.
/// Supports optional `{tags}` for shape, fill, etc. Bullet (`-`) or number (`1.`) prefix is stripped.
///
/// ### `## Grid [cols=N]` section
/// ```text
/// ## Grid cols=4
/// - [a] Feature A {fill:blue}
/// - [b] Feature B {fill:green}
/// - [c] Feature C {fill:red}
/// - [d] Feature D {fill:yellow}
/// ```
/// Nodes are laid out in a grid (default 3 columns). Aliases: `## Matrix`, `## Table`.
/// Accepts `cols=N` or just `## Grid 4` (bare number = column count).
/// Nodes are pinned in place so they won't be moved by auto-layout.
///
/// ### Edge label syntax
/// ```text
/// a "label" --> b         ← prefix quoted label (original)
/// a ->|label| b           ← Mermaid-style pipe label
/// a -> b: label text      ← suffix colon label
/// a → b                   ← Unicode arrow (same as -->)
/// a ↔ b                   ← bidirectional Unicode arrow (<->)
/// ```
///
/// ### `## Groups` section
/// ```text
/// ## Groups
/// - [grp_id] Group Label {fill:blue}
///   node_id1, node_id2, node_id3
/// ```
/// Creates a frame node bounding all listed member nodes (after auto-layout).
///
/// ### `## Config` section
/// ```text
/// ## Config
/// title    = My Diagram     (project title watermark on canvas)
/// bg       = dots           (dots / lines / crosshatch / none)
/// bg-color = #1e1e2e        (canvas background color: hex or name)
/// snap   = true
/// grid-size = 20
/// zoom   = 1.5           (or "fit"/"auto" to auto-fit on load)
/// flow   = LR            (layout direction: LR / TB / RL / BT)
/// view   = 3d            (open in 3D view on import)
/// camera = iso           (preset: iso / top / front / side)
/// auto-z = true          (auto-assign z-offsets from topological layers)
/// camera_yaw   = -0.6    (raw yaw in radians, overridden by camera preset)
/// camera_pitch =  0.5    (raw pitch in radians)
/// layer0 = Database      (display name for 3D layer 0)
/// layer1 = API
/// ```
/// Import-time viewport hints applied when the spec is loaded into the app.
/// `view = 3d` opens in 3D view on import.
/// `camera = iso/top/front/side` sets a named 3D camera preset.
/// `camera_yaw/pitch` set raw angles (ignored when a named preset is used).
/// `zoom = fit` (or `auto`) auto-fits the diagram to the viewport on load.
///
/// Pre-scan `## Palette` sections and return a palette map + pre-expanded input string
/// where `{fill:name}` and `{color:name}` are replaced with `{fill:#hex}` / `{color:#hex}`.
fn expand_palette(input: &str) -> (String, HashMap<String, [u8; 4]>) {
    let mut palette: HashMap<String, [u8; 4]> = HashMap::new();
    let mut in_palette = false;
    // First pass: collect palette entries
    for line in input.lines() {
        let t = line.trim();
        if t.starts_with("## ") {
            let h = t[3..].trim().to_lowercase();
            in_palette = matches!(h.as_str(), "palette" | "colors" | "colour" | "colours" | "theme");
            continue;
        }
        if in_palette && !t.is_empty() {
            let sep = if t.contains('=') { '=' } else if t.contains(':') { ':' } else { continue; };
            if let Some(pos) = t.find(sep) {
                let name = t[..pos].trim().to_lowercase();
                let val = t[pos+1..].trim();
                if let Some(color) = tag_to_fill_color(val) {
                    palette.insert(name, color);
                }
            }
        }
    }
    if palette.is_empty() {
        return (input.to_string(), palette);
    }
    // Second pass: expand {fill:name} / {color:name} / {border-color:name} in non-palette lines
    let mut out = String::with_capacity(input.len() + 128);
    let mut in_pal = false;
    for line in input.lines() {
        let t = line.trim();
        if t.starts_with("## ") {
            let h = t[3..].trim().to_lowercase();
            in_pal = matches!(h.as_str(), "palette" | "colors" | "colour" | "colours" | "theme");
            out.push_str(line); out.push('\n');
            continue;
        }
        if in_pal { out.push_str(line); out.push('\n'); continue; }
        // Replace palette references inside {fill:name} tags
        let mut expanded = line.to_string();
        for (name, color) in &palette {
            let hex = format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2]);
            // Replace all occurrences of {fill:name}, {color:name}, {border-color:name}
            for prefix in &["fill:", "color:", "border-color:", "stroke:"] {
                let search = format!("{{{}{}}}", prefix, name);
                let replace = format!("{{{}{}}}", prefix, hex);
                expanded = expanded.replace(&search, &replace);
            }
        }
        out.push_str(&expanded); out.push('\n');
    }
    (out, palette)
}

/// Pre-scan `## Style` sections and return an expanded string where
/// `{style_name}` references in node lines are replaced with the full tag set.
///
/// Example:
/// ```text
/// ## Style
/// primary = {fill:blue} {highlight}
/// danger  = {fill:red} {bold}
/// ```
/// A node `- [api] API {primary}` becomes `- [api] API {fill:blue} {highlight}`.
fn expand_styles(input: &str) -> String {
    let mut styles: HashMap<String, String> = HashMap::new();
    let mut in_style = false;

    // First pass: collect style definitions
    for line in input.lines() {
        let t = line.trim();
        if t.starts_with("## ") {
            let h = t[3..].trim().to_lowercase();
            in_style = matches!(h.as_str(), "style" | "styles" | "template" | "templates" | "vars" | "macros");
            continue;
        }
        if !in_style || t.is_empty() || t.starts_with("//") { continue; }
        // Parse: name = {tag1} {tag2} ...  or  name = tag1 tag2 ...
        let sep = if t.contains('=') { '=' } else if t.contains(':') { ':' } else { continue; };
        if let Some(pos) = t.find(sep) {
            let name = t[..pos].trim().to_lowercase();
            if name.is_empty() { continue; }
            let val = t[pos+1..].trim();
            // Normalize: if val doesn't contain {}, wrap each space-separated token in {}
            let expansion = if val.contains('{') {
                val.to_string()
            } else {
                val.split_whitespace()
                    .map(|tok| format!("{{{}}}", tok))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            styles.insert(name, expansion);
        }
    }
    if styles.is_empty() { return input.to_string(); }

    // Second pass: expand {style_name} on non-style-section lines
    let mut out = String::with_capacity(input.len() + 256);
    let mut skip = false;
    for line in input.lines() {
        let t = line.trim();
        if t.starts_with("## ") {
            let h = t[3..].trim().to_lowercase();
            skip = matches!(h.as_str(), "style" | "styles" | "template" | "templates" | "vars" | "macros");
            out.push_str(line); out.push('\n');
            continue;
        }
        if skip { out.push_str(line); out.push('\n'); continue; }

        // Expand any {style_name} references
        let mut expanded = line.to_string();
        for (name, tags) in &styles {
            let pattern = format!("{{{}}}", name);
            expanded = expanded.replace(&pattern, tags);
        }
        out.push_str(&expanded); out.push('\n');
    }
    out
}

/// Pre-scan `## Layers` sections and expand `{layer:name}` tokens to `{z:N}`.
///
/// Format:
/// ```text
/// ## Layers
/// frontend = 240   // or z:240 or z=240
/// backend  = 120
/// data     = 0
/// ```
/// After this pass, `{layer:frontend}` becomes `{z:240}` for downstream parsing.
fn expand_layers(input: &str) -> String {
    use std::collections::HashMap;
    let mut layer_map: HashMap<String, f32> = HashMap::new();
    let mut in_layers = false;

    // First pass: collect layer definitions
    for line in input.lines() {
        let t = line.trim();
        if t.starts_with("## ") {
            let h = t[3..].trim().to_lowercase();
            in_layers = matches!(h.as_str(), "layers" | "layer-map" | "layer-names" | "z-layers");
            continue;
        }
        if !in_layers || t.is_empty() || t.starts_with("//") { continue; }
        let sep = if t.contains('=') { '=' } else if t.contains(':') { ':' } else { continue; };
        if let Some(pos) = t.find(sep) {
            let name = t[..pos].trim().to_lowercase();
            if name.is_empty() { continue; }
            let val = t[pos+1..].trim();
            // Parse value as z offset: "240", "z:240", "z=240", or named tier
            let z: Option<f32> = if let Ok(n) = val.parse::<f32>() {
                Some(n)
            } else if let Some(rest) = val.strip_prefix("z:").or_else(|| val.strip_prefix("z=")) {
                rest.trim().parse::<f32>().ok()
            } else {
                // Named tier fallback
                match val.to_lowercase().as_str() {
                    "db" | "data" | "database" | "storage" | "cache" | "queue" | "persistence" => Some(0.0),
                    "app" | "api" | "service" | "backend" | "server" | "logic" | "worker" => Some(120.0),
                    "ui" | "frontend" | "client" | "web" | "browser" | "view" | "spa" => Some(240.0),
                    "edge" | "gateway" | "lb" | "proxy" | "cdn" | "ingress" => Some(360.0),
                    "infra" | "platform" | "ops" | "host" | "cloud" => Some(480.0),
                    _ => None,
                }
            };
            if let Some(z_val) = z {
                layer_map.insert(name, z_val);
            }
        }
    }
    if layer_map.is_empty() { return input.to_string(); }

    // Second pass: replace {layer:name} with {z:N}
    let mut out = String::with_capacity(input.len() + 64);
    let mut in_layers_skip = false;
    for line in input.lines() {
        let t = line.trim();
        if t.starts_with("## ") {
            let h = t[3..].trim().to_lowercase();
            in_layers_skip = matches!(h.as_str(), "layers" | "layer-map" | "layer-names" | "z-layers");
            out.push_str(line); out.push('\n');
            continue;
        }
        if in_layers_skip { out.push_str(line); out.push('\n'); continue; }

        let mut expanded = line.to_string();
        for (name, z_val) in &layer_map {
            // Replace {layer:name} and {tier:name} with {z:N}
            let z_int = z_val.round() as i32;
            for prefix in &["layer:", "tier:", "level:"] {
                let search = format!("{{{}{}}}", prefix, name);
                let replace = format!("{{z:{}}}", z_int);
                expanded = expanded.replace(&search, &replace);
            }
        }
        out.push_str(&expanded); out.push('\n');
    }
    out
}

/// Parse a `.spec` Human-Readable Format string into a `FlowchartDocument`.
///
/// See the full format reference in the doc-comment on `expand_palette` above.
/// Supports: `## Nodes`, `## Layer N`, `## Flow`, `## Notes`, `## Config`,
/// `## Style`, `## Palette`, `## Steps`, `## Groups`.
/// Multi-source `[a,b]->c` and multi-target `a->[b,c]` shorthands are expanded
/// before edge parsing.
pub fn parse_hrf(input: &str) -> Result<FlowchartDocument, String> {
    // Pre-expand style templates, then palette color names
    let input_with_layers = expand_layers(input);
    let input_with_styles = expand_styles(&input_with_layers);
    let (expanded_input, _palette_map) = expand_palette(&input_with_styles);
    let input = expanded_input.as_str();

    let mut doc = FlowchartDocument::default();
    let mut id_map: HashMap<String, NodeId> = HashMap::new();
    // label_map: slugified display label → NodeId for natural-language flow references
    let mut label_map: HashMap<String, NodeId> = HashMap::new();

    let mut section = Section::None;
    let mut preamble_lines: Vec<String> = Vec::new();
    let mut seen_section = false;

    // Track the last node added in Nodes section for multi-line descriptions
    let mut last_node_id: Option<NodeId> = None;
    // Track the last edges added in Flow section for indented description continuation
    let mut last_flow_edge_ids: Vec<crate::model::EdgeId> = Vec::new();

    // ## Groups section: (group_id, label, fill_color, member_ids)
    let mut groups: Vec<(String, String, Option<[u8;4]>, Vec<String>)> = Vec::new();

    // Inline group assignments from {group:name} tags in ## Nodes section
    // Format: (node_str_id, group_name)
    let mut inline_group_assignments: Vec<(String, String)> = Vec::new();

    // ## Config section: key = value pairs
    let mut config_map: HashMap<String, String> = HashMap::new();

    // ## Grid sections: collect (NodeIds, cols) for post-parse grid layout
    let mut pending_grid_groups: Vec<(Vec<NodeId>, usize)> = Vec::new();

    // ## Period sections: current period label for nodes parsed below
    let mut current_period: Option<String> = None;

    // Current section display label (original case, e.g. "Hypotheses", "Evidence")
    let mut current_section_label: String = String::new();

    // ## Palette section: handled via pre-expand pass (expand_palette above)

    // Deferred inline edges: (source_str_id, target_str_id, edge_tags)
    // Created when "- [id] Label → target1, target2" is found in ## Nodes sections.
    let mut deferred_inline_edges: Vec<(String, String, Vec<String>)> = Vec::new();

    // {dep:target} decorators: (from_node_id, target_str_id)
    // Resolved in a post-pass after all nodes are parsed.
    let mut node_deps: Vec<(NodeId, String)> = Vec::new();

    for (line_num, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim_end();
        // Strip inline `//` comments: only strip when `//` appears OUTSIDE of {} tags
        // and is not part of a URL (avoid stripping `https://`).
        // Simple heuristic: strip from the first `//` that is preceded by whitespace or `}`.
        let line_stripped: String = {
            let bytes = line.as_bytes();
            let mut result_end = line.len();
            let mut i = 0;
            while i + 1 < bytes.len() {
                if bytes[i] == b'/' && bytes[i + 1] == b'/' {
                    // Ensure it's not inside a URL (preceded by ':')
                    let preceded_by_colon = i > 0 && bytes[i - 1] == b':';
                    if !preceded_by_colon {
                        result_end = i;
                        break;
                    }
                }
                i += 1;
            }
            line[..result_end].trim_end().to_string()
        };
        let line = line_stripped.as_str();
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
            last_flow_edge_ids.clear();
            // Finalise Grid section before switching away
            if let Section::Grid { cols, nodes, .. } = &section {
                if !nodes.is_empty() {
                    pending_grid_groups.push((nodes.clone(), *cols));
                }
            }
            // Preserve original case for layer names, lowercase only for matching
            let header_raw = trimmed[3..].trim();
            let header = header_raw.to_lowercase();
            section = match header.as_str() {
                // Node / component section aliases
                "nodes" | "node" | "components" | "component"
                | "services" | "service" | "modules" | "module"
                | "systems" | "system" | "resources" | "resource"
                | "items" | "item" | "objects" | "object"
                | "elements" | "element" | "actors" | "actor"
                | "entities" | "entity" | "architecture"
                | "parts" | "part" | "blocks" | "block" => {
                    current_section_label = header_raw.to_string();
                    Section::Nodes { default_z: 0.0 }
                }
                // Hypothesis / design-thinking section aliases
                | "hypotheses" | "hypothesis" | "theories" | "theory"
                | "assumptions" | "assumption" | "premises"
                | "evidence" | "findings" | "facts"
                | "questions" | "unknowns"
                | "ideas" | "concepts" | "brainstorm"
                | "causes" | "root-causes"
                | "effects" | "impacts"
                | "risks" | "threats"
                | "goals" | "objectives" | "outcomes"
                | "experiments" | "tests"
                | "metrics" | "kpis"
                | "strengths" | "weaknesses" | "opportunities"
                | "how-might-we" | "hmw"
                // Customer journey / UX research section aliases
                | "touchpoints" | "touchpoint"
                | "pain points" | "painpoints" | "pain-points"
                | "emotions" | "emotion" | "feelings"
                | "channels" | "channel"
                // Decision / ADR section aliases
                | "decisions" | "decision"
                | "background"
                | "alternatives" | "alternative"
                | "consequences" | "consequence" | "tradeoffs" | "trade-offs"
                | "constraints" | "constraint"
                | "stakeholders" | "stakeholder" | "personas" | "persona"
                // Empathy map section aliases
                | "says" | "thinks" | "does" | "feels"
                | "pains" | "pain" | "gains" | "gain"
                | "user" | "users" | "customers" | "customer"
                // Value proposition / jobs-to-be-done section aliases
                | "jobs" | "jobs to be done" | "jtbd"
                | "features" | "feature"
                | "pain relievers" | "gain creators"
                | "product" | "products"
                // Fishbone / Ishikawa section aliases
                | "problem" | "effect" | "defect"
                | "people" | "technology" | "environment"
                | "materials" | "material" | "measurement" | "measurements"
                | "machine" | "machines" | "method" | "methods"
                | "management"
                // PESTLE section aliases
                | "political" | "economic" | "social" | "technological" | "legal" | "environmental"
                | "focus" | "strategic focus"
                // Mind map section aliases
                | "central" | "central idea" | "core" | "root"
                | "branches" | "branch" | "subtopics" | "subtopic"
                | "sub-themes" | "sub-theme"
                // Retrospective / premortem section aliases
                | "roses" | "rose" | "wins" | "what went well"
                | "buds" | "bud"
                | "thorns" | "thorn" | "blockers" | "what went wrong" | "problems"
                | "action items" | "next steps" | "follow-ups" | "follow-up"
                | "scenario" | "prevention" | "preventions"
                | "product failures" | "team failures" | "market failures" | "customer failures"
                // Strategic / OKR section aliases
                | "initiatives" | "initiative"
                | "countermeasures" | "countermeasure" | "mitigations" | "mitigation"
                | "driving forces" | "restraining forces"
                => {
                    current_section_label = header_raw.to_string();
                    Section::Nodes { default_z: 0.0 }
                }
                // Edge / flow section aliases
                "flow" | "flows" | "edges" | "connections" | "connection"
                | "links" | "link" | "relations" | "relation"
                | "relationships" | "relationship"
                | "dependencies" | "dependency"
                | "interactions" | "interaction"
                | "data flow" | "dataflow" | "api" | "events" => Section::Flow,
                "notes" | "note" | "stickies" | "sticky" | "annotations" | "annotation" => Section::Notes,
                "groups" | "group" | "clusters" | "cluster"
                | "frames" | "frame" | "containers" | "container"
                | "packages" | "package" | "sections" => Section::Groups,
                "config" | "settings" | "meta" | "diagram" | "options" | "configuration" | "setup" => Section::Config,
                "summary" | "about" | "overview" | "readme" | "description"
                | "intro" | "introduction" | "context" => Section::Summary,
                "palette" | "colors" | "colour" | "colours" | "theme" | "themes" => Section::Palette,
                "style" | "styles" | "template" | "templates" | "vars" | "macros" | "variables" => Section::Palette, // skip: handled by expand_styles
                "layers" | "layer-map" | "layer-names" | "z-layers" => Section::Palette, // skip: handled by expand_layers pre-pass
                // Ordered / sequential section aliases
                "steps" | "step" | "process" | "procedure" | "workflow"
                | "pipeline" | "sequence" | "tasks" | "task"
                | "actions" | "action" | "instructions" | "instruction"
                | "phases" | "phase" | "stages" | "stage"
                | "roadmap" | "milestones" | "milestone"
                | "journey" | "checklist" | "todo" | "plan" => Section::Steps { default_z: 0.0, last_step_id: None, step_count: 0 },
                // "## Timeline" is now a reserved section type — emit warning, treat as None.
                "timeline" => {
                    // Emit parse warning via setting a note in doc (non-fatal)
                    Section::None
                }
                _ if header.starts_with("grid") || header.starts_with("matrix") || header.starts_with("table") => {
                    // ## Grid [cols=N] / ## Grid N / ## Matrix [cols=N]
                    // Parse optional cols parameter from header
                    let rest = if header.starts_with("grid") { header_raw[4..].trim() }
                               else if header.starts_with("matrix") { header_raw[6..].trim() }
                               else { header_raw[5..].trim() }; // table
                    let cols = rest
                        .split_whitespace()
                        .find_map(|tok| {
                            let v = tok.trim_start_matches("cols=")
                                       .trim_start_matches("columns=")
                                       .trim_start_matches("col=")
                                       .trim_start_matches("n=")
                                       .trim_start_matches("c=");
                            v.parse::<usize>().ok()
                        })
                        .or_else(|| rest.parse::<usize>().ok())
                        .unwrap_or(3); // default 3 columns
                    Section::Grid { cols, default_z: 0.0, nodes: Vec::new() }
                }
                _ => {
                    // Check for "Period N" or "Period N: Label" patterns
                    if header.starts_with("period") {
                        let after_raw = header_raw[6..].trim();
                        // Parse "Period N: Label" — extract index and label
                        let (idx, label) = if let Some(colon_pos) = after_raw.find(':') {
                            let num_str = after_raw[..colon_pos].trim();
                            let idx = num_str.parse::<usize>().unwrap_or(0);
                            let label = after_raw[colon_pos+1..].trim().to_string();
                            (idx, if label.is_empty() { format!("Period {}", idx) } else { label })
                        } else {
                            let idx = after_raw.trim().parse::<usize>().unwrap_or(0);
                            (idx, format!("Period {}", idx))
                        };
                        // Register in doc.timeline_periods at the right index position
                        // Grow the vec if needed
                        while doc.timeline_periods.len() < idx {
                            doc.timeline_periods.push(format!("Period {}", doc.timeline_periods.len() + 1));
                        }
                        if idx > 0 {
                            if idx - 1 < doc.timeline_periods.len() {
                                doc.timeline_periods[idx - 1] = label.clone();
                            } else {
                                doc.timeline_periods.push(label.clone());
                            }
                        } else {
                            doc.timeline_periods.push(label.clone());
                        }
                        current_period = Some(label.clone());
                        Section::Period { label }
                    // Check for "Lane N" or "Lane N: Name" patterns
                    } else if header.starts_with("lane") {
                        let after_raw = header_raw[4..].trim();
                        let (idx, label) = if let Some(colon_pos) = after_raw.find(':') {
                            let num_str = after_raw[..colon_pos].trim();
                            let idx = num_str.parse::<usize>().unwrap_or(0);
                            let label = after_raw[colon_pos+1..].trim().to_string();
                            (idx, if label.is_empty() { format!("Lane {}", idx) } else { label })
                        } else {
                            let idx = after_raw.trim().parse::<usize>().unwrap_or(0);
                            (idx, format!("Lane {}", idx))
                        };
                        // Register in doc.timeline_lanes preserving order
                        while doc.timeline_lanes.len() < idx.saturating_sub(1) {
                            doc.timeline_lanes.push(format!("Lane {}", doc.timeline_lanes.len() + 1));
                        }
                        if !doc.timeline_lanes.contains(&label) {
                            if idx > 0 && idx - 1 <= doc.timeline_lanes.len() {
                                if idx - 1 == doc.timeline_lanes.len() {
                                    doc.timeline_lanes.push(label);
                                } else {
                                    doc.timeline_lanes[idx - 1] = label;
                                }
                            } else {
                                doc.timeline_lanes.push(label);
                            }
                        }
                        Section::Lane
                    // Check for "Layer N" or "Layer N: Name" patterns
                    // "layer 0", "layer 1", "layer 2", ... → z = N * Z_SPACING (120)
                    // "layer z:120", "layer z=120" → z = 120 (explicit)
                    } else if header.starts_with("layer") {
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
                        // Unknown section name — treat as a Nodes section so user-defined
                        // sections like "## Strengths", "## Quick Wins", "## Phase 1: Detect"
                        // all produce visible nodes instead of being silently dropped.
                        current_section_label = header_raw.to_string();
                        Section::Nodes { default_z: 0.0 }
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
                    // New node definition — may have inline edges: "- [id] Label → target1, target2 {tags}"
                    let stripped = &trimmed[2..];
                    // Split on → or -> (outside braces) for inline edges
                    let (node_part, inline_targets) = split_inline_edges(stripped);
                    let (id, mut node, deps) = parse_node_line(node_part, line_num)?;
                    // Apply section default z if node doesn't have an explicit {z:N} tag
                    if node.z_offset == 0.0 && default_z != 0.0 {
                        node.z_offset = default_z;
                    }
                    last_node_id = Some(node.id);
                    for dep in deps { node_deps.push((node.id, dep)); }
                    id_map.insert(id.clone(), node.id);
                    label_map.insert(slugify(node.display_label(), 0), node.id);
                    // Detect {group:name} tag for inline group assignment
                    {
                        let (_, tags) = extract_tags(node_part);
                        for tag in &tags {
                            if tag.starts_with("group:") || tag.starts_with("cluster:") || tag.starts_with("in:") {
                                let prefix = if tag.starts_with("group:") { 6 }
                                    else if tag.starts_with("cluster:") { 8 }
                                    else { 3 }; // in:
                                let gname = tag[prefix..].trim().to_string();
                                if !gname.is_empty() {
                                    inline_group_assignments.push((id.clone(), gname));
                                }
                                break;
                            }
                        }
                    }
                    // Inherit current timeline period context
                    if node.timeline_period.is_none() {
                        node.timeline_period = current_period.clone();
                    }
                    // Record the section this node belongs to
                    if node.section_name.is_empty() && !current_section_label.is_empty() {
                        node.section_name = current_section_label.clone();
                    }
                    // Auto-discover lane in doc.timeline_lanes
                    if let Some(ref lane) = node.timeline_lane.clone() {
                        if !doc.timeline_lanes.contains(lane) {
                            doc.timeline_lanes.push(lane.clone());
                        }
                    }
                    // Defer inline edge creation
                    for (target_id, edge_tags) in inline_targets {
                        deferred_inline_edges.push((id.clone(), target_id, edge_tags));
                    }
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
                    // Indented continuation → set description/comment on last added edge(s)
                    if (line.starts_with("  ") || line.starts_with('\t')) && !last_flow_edge_ids.is_empty() {
                        let desc = trimmed.to_string();
                        for eid in &last_flow_edge_ids {
                            if let Some(edge) = doc.edges.iter_mut().find(|e| e.id == *eid) {
                                if edge.comment.is_empty() {
                                    edge.comment = desc.clone();
                                } else {
                                    edge.comment.push(' ');
                                    edge.comment.push_str(&desc);
                                }
                            }
                        }
                    } else {
                        // Expand multi-source: `[a, b] -> target {tags}` → multiple lines
                        // Expand multi-target: `source -> [a, b, c] {tags}` → multiple lines
                        let lines_to_parse: Vec<String> = if let Some(expanded) = expand_multi_source(trimmed) {
                            expanded
                        } else if let Some(expanded) = expand_multi_target(trimmed) {
                            expanded
                        } else {
                            vec![trimmed.to_string()]
                        };
                        last_flow_edge_ids.clear();
                        for expanded_line in &lines_to_parse {
                            let edges = parse_flow_line_chain(expanded_line.trim(), &id_map, &label_map, line_num)?;
                            for edge in edges {
                                last_flow_edge_ids.push(edge.id);
                                doc.edges.push(edge);
                            }
                        }
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
            Section::Summary => {
                // ## Summary: collect prose lines into doc.description (overrides preamble)
                if !trimmed.is_empty() {
                    if doc.description.is_empty() {
                        doc.description = trimmed.to_string();
                    } else {
                        doc.description.push('\n');
                        doc.description.push_str(trimmed);
                    }
                }
            }
            Section::Palette => {
                // Colors pre-expanded by expand_palette(); skip these lines
            }
            Section::Steps { .. } => {
                // Handled below to allow mutation of section state
                if trimmed.is_empty() { continue; }
                // Extract step state with &mut for count update
                let current_step = if let Section::Steps { step_count, .. } = &mut section {
                    *step_count += 1;
                    *step_count
                } else { continue; };
                let (s_default_z, s_last_id) = if let Section::Steps { default_z, last_step_id, .. } = section {
                    (default_z, last_step_id)
                } else { continue; };
                // Strip leading numbering or bullet
                let stripped = {
                    let t = trimmed;
                    if t.starts_with(|c: char| c.is_ascii_digit()) {
                        let end = t.find(|c: char| c == '.' || c == ')').map(|i| i + 1).unwrap_or(0);
                        if end > 0 && t[end..].starts_with(' ') { t[end..].trim_start() } else { t }
                    } else if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") {
                        &t[2..]
                    } else { t }
                };
                let auto_id = format!("step{}", current_step);
                let (raw_label_and_tags, tags) = extract_tags(stripped);
                // Support "Title: description" colon syntax — text before ':' is the step title,
                // text after ':' becomes the sublabel (step number auto-sublabel is suppressed).
                let (label, step_colon_sublabel) = {
                    // Avoid splitting on colons inside URLs (http://)
                    let first_word_end = raw_label_and_tags.find(|c: char| c.is_whitespace())
                        .unwrap_or(raw_label_and_tags.len());
                    let after_first = raw_label_and_tags[first_word_end..].trim();
                    if let Some(colon_pos) = after_first.find(':') {
                        if colon_pos > 0 && !after_first[..colon_pos].contains(|c: char| c == '/' || c == '{') {
                            let title = format!("{} {}", &raw_label_and_tags[..first_word_end], &after_first[..colon_pos]).trim().to_string();
                            let desc  = after_first[colon_pos + 1..].trim().to_string();
                            (title, if desc.is_empty() { None } else { Some(desc) })
                        } else {
                            (raw_label_and_tags.to_string(), None)
                        }
                    } else {
                        (raw_label_and_tags.to_string(), None)
                    }
                };
                // Find shape tag — look for known shape names in tags
                let shape = tags.iter()
                    .find(|t| matches!(t.as_str(),
                        "diamond" | "circle" | "rectangle" | "rounded_rect" | "parallelogram"
                        | "hexagon" | "connector" | "text" | "entity" | "decision" | "start" | "end"))
                    .map(|t| tag_to_shape(t))
                    .unwrap_or(NodeShape::RoundedRect);
                let fill = tags.iter()
                    .find(|t| t.starts_with("fill:"))
                    .and_then(|t| tag_to_fill_color(t[5..].trim()));
                let mut node = Node::new(shape, egui::Pos2::ZERO);
                if let NodeKind::Shape { label: lbl, .. } = &mut node.kind {
                    *lbl = label.trim().to_string();
                }
                // Add step number as sublabel if not already set (colon desc overrides step number)
                if let Some(desc) = step_colon_sublabel {
                    node.sublabel = desc;
                } else if node.sublabel.is_empty() {
                    node.sublabel = format!("Step {}", current_step);
                }
                node.z_offset = s_default_z;
                if let Some(fc) = fill { node.style.fill_color = fc; }
                let node_id = node.id;
                id_map.insert(auto_id, node_id);
                label_map.insert(slugify(node.display_label(), 0), node_id);
                if let Some(prev_id) = s_last_id {
                    let e = Edge::new(
                        Port { node_id: prev_id, side: PortSide::Right },
                        Port { node_id: node_id, side: PortSide::Left },
                    );
                    doc.edges.push(e);
                }
                // Update the section's last_step_id
                if let Section::Steps { last_step_id, .. } = &mut section {
                    *last_step_id = Some(node_id);
                }
                // Note: step_count was already incremented at the start of this block
                doc.nodes.push(node);
            }
            Section::Grid { ref mut nodes, default_z, .. } => {
                if trimmed.starts_with("- ") {
                    let stripped = &trimmed[2..];
                    let (node_part, inline_targets) = split_inline_edges(stripped);
                    let (id, mut node, deps) = parse_node_line(node_part, line_num)?;
                    if node.z_offset == 0.0 && default_z != 0.0 {
                        node.z_offset = default_z;
                    }
                    last_node_id = Some(node.id);
                    let nid = node.id;
                    for dep in deps { node_deps.push((nid, dep)); }
                    id_map.insert(id.clone(), nid);
                    label_map.insert(slugify(node.display_label(), 0), nid);
                    for (target_id, edge_tags) in inline_targets {
                        deferred_inline_edges.push((id.clone(), target_id, edge_tags));
                    }
                    nodes.push(nid);
                    doc.nodes.push(node);
                }
            }
            Section::Period { .. } => {
                // Nodes parsed under a Period section are handled by Section::Nodes logic
                // after the section header sets current_period. Here we parse them the
                // same way — route through the Nodes arm by converting on the fly.
                if trimmed.starts_with("- ") {
                    let stripped = &trimmed[2..];
                    let (node_part, inline_targets) = split_inline_edges(stripped);
                    let (id, mut node, deps) = parse_node_line(node_part, line_num)?;
                    node.timeline_period = current_period.clone();
                    if let Some(ref lane) = node.timeline_lane.clone() {
                        if !doc.timeline_lanes.contains(lane) {
                            doc.timeline_lanes.push(lane.clone());
                        }
                    }
                    last_node_id = Some(node.id);
                    for dep in deps { node_deps.push((node.id, dep)); }
                    id_map.insert(id.clone(), node.id);
                    label_map.insert(slugify(node.display_label(), 0), node.id);
                    for (target_id, edge_tags) in inline_targets {
                        deferred_inline_edges.push((id.clone(), target_id, edge_tags));
                    }
                    doc.nodes.push(node);
                } else if line.starts_with("  ") || line.starts_with("\t") {
                    if let Some(nid) = last_node_id {
                        if let Some(node) = doc.find_node_mut(&nid) {
                            append_description(node, trimmed);
                        }
                    }
                }
            }
            Section::Lane => {
                // Lane declarations are header-only — body lines are ignored
            }
            Section::None => {}
        }
    }

    // Finalise last section if it was a Grid
    if let Section::Grid { cols, nodes, .. } = &section {
        if !nodes.is_empty() {
            pending_grid_groups.push((nodes.clone(), *cols));
        }
    }

    doc.description = preamble_lines.join("\n");

    // Resolve deferred inline edges (from "- [id] Label → target1, target2" syntax)
    for (src_id, tgt_id, edge_tags) in &deferred_inline_edges {
        if let (Some(&src_node_id), Some(&tgt_node_id)) = (id_map.get(src_id), id_map.get(tgt_id)) {
            let mut edge = Edge::new(
                Port { node_id: src_node_id, side: PortSide::Right },
                Port { node_id: tgt_node_id, side: PortSide::Left },
            );
            for etag in edge_tags {
                if etag.starts_with("color:") {
                    if let Some(c) = tag_to_edge_color(etag[6..].trim()) { edge.style.color = c; }
                } else if etag.starts_with("note:") {
                    edge.comment = etag[5..].trim().to_string();
                } else {
                    match etag.as_str() {
                        "dashed" | "dash" => edge.style.dashed = true,
                        "glow" | "neon" => edge.style.glow = true,
                        "animated" | "flow" => edge.style.animated = true,
                        "thick" | "bold" => edge.style.width = 5.0,
                        "ortho" | "orthogonal" => edge.style.orthogonal = true,
                        "escalate" | "escalation" | "escalated" => {
                            edge.style.color = [243, 139, 168, 255];
                            edge.style.width = 3.5;
                            edge.style.glow = true;
                        }
                        "resolves" | "resolved-by" | "fixes" | "closes" => {
                            edge.style.color = [166, 227, 161, 255];
                            edge.style.dashed = true;
                        }
                        "blocks" | "blocked-by" | "blocking" => {
                            edge.style.color = [250, 179, 135, 255];
                            edge.style.width = 3.0;
                        }
                        _ => {}
                    }
                }
            }
            doc.edges.push(edge);
        }
    }

    // Apply ## Config values
    for (key, val) in &config_map {
        match key.as_str() {
            "title" => {
                doc.title = val.clone();
                doc.import_hints.project_title = Some(val.clone());
            }
            "description" | "desc" => { doc.description = val.clone(); }
            // import hints — applied by the toolbar after import
            "bg" | "bg-pattern" => {
                doc.import_hints.bg_pattern = Some(val.to_lowercase());
            }
            "snap" | "snap-to-grid" | "snap_to_grid" => {
                doc.import_hints.snap = match val.to_lowercase().as_str() {
                    "true" | "on" | "yes" | "1" => Some(true),
                    "false" | "off" | "no" | "0" => Some(false),
                    _ => None,
                };
            }
            "grid-size" | "grid_size" | "grid" => {
                if let Ok(sz) = val.trim().parse::<f32>() {
                    doc.import_hints.grid_size = Some(sz.clamp(5.0, 200.0));
                }
            }
            "zoom" | "initial-zoom" | "scale" => {
                match val.trim().to_lowercase().as_str() {
                    "fit" | "auto" | "auto-fit" | "autofit" => {
                        doc.import_hints.auto_fit = true;
                    }
                    _ => {
                        if let Ok(z) = val.trim().parse::<f32>() {
                            doc.import_hints.zoom = Some(z.clamp(0.1, 4.0));
                        }
                    }
                }
            }
            // 3D camera: camera_yaw = -0.4, camera_pitch = 0.6, view = 3d
            "camera_yaw" | "camera-yaw" | "yaw" => {
                if let Ok(v) = val.trim().parse::<f32>() {
                    doc.import_hints.camera_yaw = Some(v);
                }
            }
            "camera_pitch" | "camera-pitch" | "pitch" => {
                if let Ok(v) = val.trim().parse::<f32>() {
                    doc.import_hints.camera_pitch = Some(v);
                }
            }
            "view" | "view-mode" | "mode" => {
                match val.to_lowercase().as_str() {
                    "3d" | "three-d" | "threed" => { doc.import_hints.view_3d = Some(true); }
                    "2d" | "flat" => { doc.import_hints.view_3d = Some(false); }
                    _ => {}
                }
            }
            // camera = iso | top | front | side  (named presets)
            "camera" | "camera-preset" | "cam" => {
                // Match preset names to (yaw, pitch) — same values as toolbar buttons
                let maybe_preset: Option<(f32, f32)> = match val.to_lowercase().trim() {
                    "iso" | "isometric" | "default"    => Some((-0.6, 0.5)),
                    "top" | "overhead" | "bird"        => Some((0.0,  1.55)),
                    "front" | "elevation"              => Some((0.0,  0.05)),
                    "side" | "right" | "left"          => Some((1.57, 0.05)),
                    _ => None,
                };
                if let Some((yaw, pitch)) = maybe_preset {
                    doc.import_hints.camera_yaw   = Some(yaw);
                    doc.import_hints.camera_pitch = Some(pitch);
                    // Named camera preset implies 3D view
                    doc.import_hints.view_3d = Some(true);
                }
            }
            "timeline" => {
                match val.to_lowercase().as_str() {
                    "true" | "yes" | "on" | "1" => { doc.timeline_mode = true; }
                    _ => {}
                }
            }
            "timeline-dir" | "timeline_dir" => {
                doc.timeline_dir = match val.to_uppercase().as_str() {
                    "TB" | "TOP-BOTTOM" | "VERTICAL" => "TB".to_string(),
                    _ => "LR".to_string(),
                };
            }
            "flow" | "layout" | "direction" | "layout-dir" | "layout_dir" => {
                let dir = match val.to_uppercase().as_str() {
                    "LR" | "LEFT-RIGHT" | "LEFT_RIGHT" | "HORIZONTAL" => "LR",
                    "RL" | "RIGHT-LEFT" | "RIGHT_LEFT" => "RL",
                    "BT" | "BOTTOM-TOP" | "BOTTOM_TOP" | "UP" => "BT",
                    _ => "TB", // default top-to-bottom
                };
                doc.layout_dir = dir.to_string();
            }
            // auto-z: automatically assign z offsets from topological layer ordering
            "auto-z" | "auto_z" | "z-auto" | "auto-layers" | "3d-auto" => {
                match val.to_lowercase().as_str() {
                    "true" | "yes" | "on" | "1" => { doc.import_hints.auto_z = true; }
                    _ => {}
                }
            }
            "auto-tier-color" | "tier-color" | "auto-color" | "tier-tint" => {
                match val.to_lowercase().as_str() {
                    "true" | "yes" | "on" | "1" => { doc.import_hints.auto_tier_color = true; }
                    _ => {}
                }
            }
            // canvas background color: bg-color = #1e1e2e  or  bg-color = dark
            "bg-color" | "background-color" | "background" | "canvas-bg" | "canvas-color" => {
                let v = val.trim();
                let rgba = parse_hex_color(v).or_else(|| tag_to_fill_color(v).filter(|c| c[3] > 0));
                if let Some(c) = rgba {
                    doc.import_hints.canvas_bg = Some(c);
                }
            }
            // project title watermark (title is handled above; these are aliases)
            "project-title" | "watermark" => {
                doc.import_hints.project_title = Some(val.clone());
            }
            // layer names: layer0 = Data Tier, layer 1 = Backend
            // layout spacing: spacing=120 or gap=80 or gap-main=80 gap-cross=50
            "spacing" | "gap" | "node-spacing" | "node_spacing" => {
                if let Ok(v) = val.parse::<f32>() {
                    doc.layout_gap_main = v;
                    doc.layout_gap_cross = v * 0.75; // cross is 3/4 of main by default
                }
            }
            "gap-main" | "gap_main" | "layer-spacing" | "layer_spacing" | "main-gap" => {
                if let Ok(v) = val.parse::<f32>() { doc.layout_gap_main = v; }
            }
            "gap-cross" | "gap_cross" | "cross-gap" | "node-gap" | "node_gap" => {
                if let Ok(v) = val.parse::<f32>() { doc.layout_gap_cross = v; }
            }
            _ if key.starts_with("layer") => {
                let num_part = key.trim_start_matches("layer").trim();
                if let Ok(idx) = num_part.trim_matches(|c: char| !c.is_ascii_digit())
                    .parse::<i32>()
                {
                    doc.layer_names.insert(idx, val.clone());
                }
            }
            // SLA target days by priority: sla-p1 = 1, sla-p2 = 3, etc.
            "sla-p1" | "sla_p1" => {
                if let Ok(v) = val.parse::<u32>() { doc.sla_days[0] = v; }
            }
            "sla-p2" | "sla_p2" => {
                if let Ok(v) = val.parse::<u32>() { doc.sla_days[1] = v; }
            }
            "sla-p3" | "sla_p3" => {
                if let Ok(v) = val.parse::<u32>() { doc.sla_days[2] = v; }
            }
            "sla-p4" | "sla_p4" => {
                if let Ok(v) = val.parse::<u32>() { doc.sla_days[3] = v; }
            }
            _ => {}
        }
    }

    // Apply ## Grid section layouts — positions assigned BEFORE hierarchical_layout
    // so that layout skips them (hierarchical_layout only moves nodes at origin [0,0]).
    if !pending_grid_groups.is_empty() {
        let cell_w = 220.0_f32; // cell width including gap
        let cell_h = 140.0_f32; // cell height including gap
        let mut start_y: f32 = 100.0; // non-zero so hierarchical_layout skips these nodes
        for (grid_nodes, cols) in &pending_grid_groups {
            let cols = (*cols).max(1);
            let num_nodes = grid_nodes.len();
            for (idx, nid) in grid_nodes.iter().enumerate() {
                let col = (idx % cols) as f32;
                let row = (idx / cols) as f32;
                if let Some(node) = doc.nodes.iter_mut().find(|n| n.id == *nid) {
                    node.position = [col * cell_w + 100.0, row * cell_h + start_y];
                    node.pinned = true; // prevent hierarchical_layout from overriding
                }
            }
            let rows_count = ((num_nodes as f32) / (cols as f32)).ceil();
            start_y += rows_count * cell_h + 80.0;
        }
    }

    // Auto-layout: timeline or hierarchical placement
    if doc.timeline_mode {
        super::layout::timeline_layout(&mut doc);
    } else {
        super::layout::hierarchical_layout(&mut doc);
    }

    // auto-z: assign z-offsets from topological layer ordering (only for nodes at z=0)
    if doc.import_hints.auto_z {
        use std::collections::{HashMap as HM, VecDeque};
        let n = doc.nodes.len();
        let ni: HM<NodeId, usize> = doc.nodes.iter().enumerate().map(|(i, nd)| (nd.id, i)).collect();
        let mut in_deg: Vec<i32> = vec![0; n];
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for e in &doc.edges {
            if let (Some(&f), Some(&t)) = (ni.get(&e.source.node_id), ni.get(&e.target.node_id)) {
                if f != t { adj[f].push(t); in_deg[t] += 1; }
            }
        }
        let mut topo_layer: Vec<i32> = vec![0; n];
        let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_deg[i] == 0).collect();
        let mut rem = in_deg.clone();
        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                let c = topo_layer[u] + 1;
                if c > topo_layer[v] { topo_layer[v] = c; }
                rem[v] -= 1;
                if rem[v] == 0 { queue.push_back(v); }
            }
        }
        const Z_SPACING: f32 = 120.0;
        for (i, node) in doc.nodes.iter_mut().enumerate() {
            if node.z_offset == 0.0 && !node.is_frame {
                node.z_offset = topo_layer[i] as f32 * Z_SPACING;
            }
        }
    }

    // auto-tier-color: tint nodes with default fill based on their z-tier
    if doc.import_hints.auto_tier_color {
        let default_fill: [u8; 4] = [49, 50, 68, 255];
        for node in doc.nodes.iter_mut() {
            if !node.is_frame && node.z_offset != 0.0 && node.style.fill_color == default_fill {
                node.style.fill_color = z_tier_fill_color(node.z_offset);
            }
        }
    }

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

    // Process inline group assignments: {group:name} tags from ## Nodes section
    if !inline_group_assignments.is_empty() {
        // Collect all unique group names and their member node ids
        let mut inline_group_map: std::collections::HashMap<String, Vec<NodeId>> = std::collections::HashMap::new();
        for (str_id, group_name) in &inline_group_assignments {
            if let Some(&nid) = id_map.get(str_id) {
                inline_group_map.entry(group_name.clone()).or_default().push(nid);
            }
        }
        let pad = 24.0_f32;
        // Sort for deterministic frame creation order
        let mut group_names: Vec<&String> = inline_group_map.keys().collect();
        group_names.sort();
        for group_name in group_names {
            let member_ids = &inline_group_map[group_name];
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            for &nid in member_ids {
                if let Some(node) = doc.nodes.iter().find(|n| n.id == nid) {
                    let x1 = node.position[0];
                    let y1 = node.position[1];
                    min_x = min_x.min(x1);
                    min_y = min_y.min(y1);
                    max_x = max_x.max(x1 + node.size[0]);
                    max_y = max_y.max(y1 + node.size[1]);
                }
            }
            if min_x == f32::INFINITY { continue; }
            let mut frame = Node::new_frame(egui::Pos2::new(min_x - pad, min_y - pad));
            frame.size = [max_x - min_x + pad * 2.0, max_y - min_y + pad * 2.0];
            // Capitalize first letter of group name for the frame label
            let label = {
                let mut s = group_name.clone();
                if let Some(c) = s.get_mut(0..1) { c.make_ascii_uppercase(); }
                s.replace('-', " ").replace('_', " ")
            };
            if let NodeKind::Shape { label: ref mut l, .. } = frame.kind {
                *l = label;
            }
            doc.nodes.insert(0, frame);
        }
    }

    // Position sticky notes in a horizontal strip below the main diagram.
    // Find the lowest y-coordinate of all non-note nodes, then place notes
    // starting from there + a gap, arranged in a row.
    {
        let sticky_ids: Vec<NodeId> = doc.nodes.iter()
            .filter(|n| matches!(n.kind, NodeKind::StickyNote { .. }) && n.position == [0.0, 0.0])
            .map(|n| n.id)
            .collect();
        if !sticky_ids.is_empty() {
            // Find bounding box of non-note nodes
            let max_y_of_diagram = doc.nodes.iter()
                .filter(|n| !matches!(n.kind, NodeKind::StickyNote { .. }))
                .map(|n| n.position[1] + n.size[1])
                .fold(0.0_f32, f32::max);
            let min_x_of_diagram = doc.nodes.iter()
                .filter(|n| !matches!(n.kind, NodeKind::StickyNote { .. }))
                .map(|n| n.position[0])
                .fold(f32::INFINITY, f32::min);
            let note_y = max_y_of_diagram + 60.0;
            let mut note_x = if min_x_of_diagram.is_finite() { min_x_of_diagram } else { 100.0 };
            for nid in sticky_ids {
                if let Some(node) = doc.nodes.iter_mut().find(|n| n.id == nid) {
                    node.position = [note_x, note_y];
                    note_x += node.size[0] + 16.0;
                }
            }
        }
    }

    // Auto-suggest 3D view when the spec has nodes at 2+ distinct z-levels
    // and the user has not explicitly set `view = 2d` in Config.
    if doc.import_hints.view_3d.is_none() {
        let mut distinct_z: std::collections::HashSet<i32> = std::collections::HashSet::new();
        for node in &doc.nodes {
            if !node.is_frame {
                distinct_z.insert(node.z_offset.round() as i32);
            }
        }
        if distinct_z.len() >= 2 {
            doc.import_hints.view_3d = Some(true);
            doc.import_hints.auto_fit = true;
        }
    }

    // Resolve {dep:target} decorators into dashed dependency edges.
    // id_map and label_map are fully populated by this point.
    for (from_id, dep_target) in node_deps {
        let target_node_id = id_map.get(dep_target.as_str())
            .or_else(|| {
                let slug = slugify(&dep_target, 0);
                label_map.get(&slug)
            })
            .copied();
        if let Some(to_id) = target_node_id {
            let edge = Edge::new(
                Port { node_id: from_id, side: PortSide::Bottom },
                Port { node_id: to_id, side: PortSide::Top },
            );
            // Dep edges are dashed by convention: visually distinct from data-flow edges
            let mut edge = edge;
            edge.style.dashed = true;
            doc.edges.push(edge);
        }
    }

    Ok(doc)
}

/// Viewport settings serialized into `## Config` during HRF export.
pub struct ViewportExportConfig<'a> {
    /// "dots", "lines", "crosshatch", or "none"
    pub bg_pattern: &'a str,
    pub snap: bool,
    pub grid_size: f32,
    /// Current zoom level (1.0 = 100%). Emitted as `zoom = N.N` when != 1.0.
    pub zoom: f32,
    /// 3D camera yaw (radians). None = omit from config.
    pub camera_yaw: Option<f32>,
    /// 3D camera pitch (radians). None = omit from config.
    pub camera_pitch: Option<f32>,
    /// Whether the diagram should open in 3D view.
    pub view_3d: bool,
}

/// Export a FlowchartDocument to Human-Readable Format.
/// Pass `viewport` to include a `## Config` section with viewport settings.
pub fn export_hrf(doc: &FlowchartDocument, title: &str) -> String {
    export_hrf_ex(doc, title, None)
}

/// Export with an optional viewport config section.
pub fn export_hrf_ex(doc: &FlowchartDocument, title: &str, viewport: Option<&ViewportExportConfig<'_>>) -> String {
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
            // Prefer the original HRF ID if set (stable round-trip), else generate from label
            let id = if !n.hrf_id.is_empty() {
                n.hrf_id.clone()
            } else {
                let label = n.display_label();
                slugify(label, i)
            };
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

    if !shape_nodes.is_empty() && doc.timeline_mode {
        // Timeline export: group nodes by period, emit ## Period N: sections
        // First emit ## Lane declarations (if any)
        for (i, lane) in doc.timeline_lanes.iter().enumerate() {
            out.push_str(&format!("## Lane {}: {}\n", i + 1, lane));
        }
        if !doc.timeline_lanes.is_empty() { out.push('\n'); }

        let periods = &doc.timeline_periods;
        for (p_idx, period_label) in periods.iter().enumerate() {
            let period_nodes: Vec<&Node> = shape_nodes.iter().copied()
                .filter(|n| n.timeline_period.as_deref() == Some(period_label.as_str()))
                .collect();
            out.push_str(&format!("## Period {}: {}\n", p_idx + 1, period_label));
            for node in period_nodes {
                let id = id_map.get(&node.id).cloned().unwrap_or_default();
                let lane_suffix = node.timeline_lane.as_ref()
                    .map(|l| format!(" {{lane:{}}}", l))
                    .unwrap_or_default();
                // Emit lane tag inline with the node line
                // We use a temporary approach: export normally then append lane tag
                let mut node_line = String::new();
                export_node_to_hrf(node, &id, "", &mut node_line);
                // Insert lane tag before the trailing newline
                let node_line = if !lane_suffix.is_empty() {
                    node_line.trim_end_matches('\n').to_string() + &lane_suffix + "\n"
                } else { node_line };
                out.push_str(&node_line);
            }
            out.push('\n');
        }
        // Emit nodes with no period assignment in a trailing ## Nodes section
        let unperioded: Vec<&Node> = shape_nodes.iter().copied()
            .filter(|n| n.timeline_period.is_none())
            .collect();
        if !unperioded.is_empty() {
            out.push_str("## Nodes\n");
            for node in unperioded {
                let id = id_map.get(&node.id).cloned().unwrap_or_default();
                export_node_to_hrf(node, &id, "", &mut out);
            }
            out.push('\n');
        }
    } else if !shape_nodes.is_empty() {
        // Collect distinct z-offsets (preserve insertion order via Vec dedup)
        let mut z_groups: Vec<f32> = Vec::new();
        for n in &shape_nodes {
            let z = n.z_offset;
            if !z_groups.iter().any(|&g| (g - z).abs() < 0.5) {
                z_groups.push(z);
            }
        }
        z_groups.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // If all nodes are on z=0 and have section_name set, group by section_name
        let all_z0 = z_groups.len() == 1 && z_groups[0].abs() < 0.5;
        let any_section = shape_nodes.iter().any(|n| !n.section_name.is_empty());
        if all_z0 && any_section {
            // Collect section names in document order (preserve first occurrence order)
            let mut section_order: Vec<String> = Vec::new();
            for n in &shape_nodes {
                if !n.section_name.is_empty() && !section_order.contains(&n.section_name) {
                    section_order.push(n.section_name.clone());
                }
            }
            let mut used_ids: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
            for section in &section_order {
                let group: Vec<&Node> = shape_nodes.iter().copied()
                    .filter(|n| &n.section_name == section)
                    .collect();
                out.push_str(&format!("## {}\n", section));
                for node in group {
                    let id = id_map.get(&node.id).cloned().unwrap_or_default();
                    export_node_to_hrf(node, &id, "", &mut out);
                    used_ids.insert(node.id);
                }
                out.push('\n');
            }
            let unsectioned: Vec<&Node> = shape_nodes.iter().copied()
                .filter(|n| !used_ids.contains(&n.id))
                .collect();
            if !unsectioned.is_empty() {
                out.push_str("## Nodes\n");
                for node in unsectioned {
                    let id = id_map.get(&node.id).cloned().unwrap_or_default();
                    export_node_to_hrf(node, &id, "", &mut out);
                }
                out.push('\n');
            }
        } else {

        let use_layers = z_groups.len() > 1;

        for section_z in &z_groups {
            let mut group: Vec<&Node> = shape_nodes
                .iter()
                .copied()
                .filter(|n| (n.z_offset - section_z).abs() < 0.5)
                .collect();
            // Sort nodes top-to-bottom, left-to-right for human-readable spec output.
            // Frames (is_frame=true) are placed last since they're background containers.
            group.sort_by(|a, b| {
                let a_frame = a.is_frame as i32;
                let b_frame = b.is_frame as i32;
                if a_frame != b_frame { return a_frame.cmp(&b_frame); }
                // Primary sort: Y (top to bottom), quantized to 80px buckets
                let ay = (a.position[1] / 80.0).floor() as i64;
                let by = (b.position[1] / 80.0).floor() as i64;
                if ay != by { return ay.cmp(&by); }
                // Secondary sort: X (left to right)
                a.position[0].partial_cmp(&b.position[0]).unwrap_or(std::cmp::Ordering::Equal)
            });

            if use_layers {
                // Use natural index (0,1,2...) when z is a multiple of Z_SPACING (120),
                // otherwise use explicit "z=N" notation to avoid the index heuristic.
                let z_spacing = 120.0_f32;
                let idx = (section_z / z_spacing).round();
                let is_multiple = (section_z - idx * z_spacing).abs() < 0.5;
                let layer_key = idx as i32;
                // Use explicit layer name, then semantic tier name, then nothing
                let name_suffix = doc.layer_names.get(&layer_key)
                    .map(|n| format!(": {}", n))
                    .or_else(|| {
                        // Fallback: well-known z values get a human-readable tier name
                        let semantic = match *section_z as i32 {
                            0   => Some("db"),
                            120 => Some("api"),
                            240 => Some("frontend"),
                            360 => Some("edge"),
                            480 => Some("infra"),
                            _   => None,
                        };
                        semantic.map(|n| format!(": {}", n))
                    })
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
        } // close `else` of `if all_z0 && any_section`
    }

    // Flow section — edges grouped by source node for human readability
    if !doc.edges.is_empty() {
        // Build a helper closure to collect style tags for an edge
        let edge_style_tags = |edge: &Edge| -> (String, Vec<String>) {
            // Returns (to_id, style_tags_vec)
            let to = id_map.get(&edge.target.node_id).cloned().unwrap_or_default();
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
                ArrowHead::Filled => {}
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
            // Note: edge.comment is emitted as indented continuation below (not here)
            (to, style_tags)
        };

        // Build a helper closure to format a single edge line
        let fmt_edge_line = |edge: &Edge| -> String {
            let from = id_map.get(&edge.source.node_id).cloned().unwrap_or_default();
            let (to, style_tags) = edge_style_tags(edge);
            let tag_str = if style_tags.is_empty() {
                String::new()
            } else {
                format!(" {{{}}}", style_tags.join("} {"))
            };
            // Emit the edge declaration line, then an indented comment if present
            let edge_line = if edge.label.is_empty() {
                format!("{} --> {}{}\n", from, to, tag_str)
            } else {
                format!("{} \"{}\" --> {}{}\n", from, edge.label, to, tag_str)
            };
            if !edge.comment.is_empty() {
                format!("{}  {}\n", edge_line, edge.comment)
            } else {
                edge_line
            }
        };

        // Group edges by source node id, preserving first-seen order
        let mut source_order: Vec<NodeId> = Vec::new();
        let mut groups: std::collections::HashMap<NodeId, Vec<&Edge>> = std::collections::HashMap::new();
        for edge in &doc.edges {
            let sid = edge.source.node_id;
            if !groups.contains_key(&sid) {
                source_order.push(sid);
            }
            groups.entry(sid).or_default().push(edge);
        }

        // Sort source groups by canvas Y then X so top-left nodes appear first
        source_order.sort_by(|a, b| {
            let pos_a = doc.nodes.iter().find(|n| n.id == *a).map(|n| (n.position[1] as i32, n.position[0] as i32)).unwrap_or((0, 0));
            let pos_b = doc.nodes.iter().find(|n| n.id == *b).map(|n| (n.position[1] as i32, n.position[0] as i32)).unwrap_or((0, 0));
            pos_a.cmp(&pos_b)
        });

        out.push_str("## Flow\n");
        // Only add group headers when there are multiple source nodes
        let use_headers = source_order.len() > 1;
        for sid in &source_order {
            if use_headers {
                // Find a human-readable label for this source node
                let header = doc.nodes.iter().find(|n| n.id == *sid)
                    .map(|n| n.display_label().to_string())
                    .unwrap_or_else(|| id_map.get(sid).cloned().unwrap_or_default());
                if !header.is_empty() {
                    out.push_str(&format!("// {}\n", header));
                }
            }
            if let Some(edges) = groups.get(sid) {
                let from = id_map.get(sid).cloned().unwrap_or_default();
                // Collect (to_id, tag_str, has_label) for each edge
                let edge_infos: Vec<(String, String, bool)> = edges.iter().map(|edge| {
                    let (to, style_tags) = edge_style_tags(edge);
                    let tag_str = if style_tags.is_empty() {
                        String::new()
                    } else {
                        format!(" {{{}}}", style_tags.join("} {"))
                    };
                    (to, tag_str, !edge.label.is_empty())
                }).collect();

                // Try to collapse consecutive edges with same tag_str and no label
                // into multi-target: `from -> [t1, t2, t3] {tag}`
                let mut i = 0;
                while i < edge_infos.len() {
                    let (ref _to0, ref tag0, has_label0) = edge_infos[i];
                    // Can only collapse unlabelled edges
                    if has_label0 {
                        out.push_str(&fmt_edge_line(edges[i]));
                        i += 1;
                        continue;
                    }
                    // Look ahead for consecutive edges with same tag_str
                    let mut j = i + 1;
                    while j < edge_infos.len() {
                        let (_, ref tag_j, has_label_j) = edge_infos[j];
                        if !has_label_j && tag_j == tag0 { j += 1; } else { break; }
                    }
                    if j - i >= 2 {
                        // Collapse i..j into multi-target
                        let targets: Vec<&str> = edge_infos[i..j].iter().map(|(t, _, _)| t.as_str()).collect();
                        out.push_str(&format!("{} --> [{}]{}\n", from, targets.join(", "), tag0));
                    } else {
                        out.push_str(&fmt_edge_line(edges[i]));
                    }
                    i = j;
                }
            }
            if use_headers {
                out.push('\n');
            }
        }
        if !use_headers {
            out.push('\n');
        }
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

    // ## Config section — include layer names and viewport hints
    let has_layer_names = !doc.layer_names.is_empty();
    let has_viewport = viewport.is_some();
    let has_layout_dir = !doc.layout_dir.is_empty() && doc.layout_dir != "TB";
    let has_title = !doc.title.is_empty();
    let has_timeline = doc.timeline_mode;
    if has_layer_names || has_viewport || has_layout_dir || has_timeline {
        out.push_str("## Config\n");
        // Project title (if set and not already the document heading)
        if has_title {
            out.push_str(&format!("title = {}\n", doc.title));
        }
        // Timeline mode
        if has_timeline {
            out.push_str("timeline = true\n");
            let tdir = if doc.timeline_dir.is_empty() { "LR" } else { &doc.timeline_dir };
            if tdir != "LR" {
                out.push_str(&format!("timeline-dir = {}\n", tdir));
            }
        }
        // Layout direction (non-default TB is always exported)
        if has_layout_dir {
            out.push_str(&format!("flow = {}\n", doc.layout_dir));
        }
        if let Some(vp) = viewport {
            // Only emit non-default values to keep the config section clean
            if vp.bg_pattern != "dots" {
                out.push_str(&format!("bg = {}\n", vp.bg_pattern));
            }
            if vp.snap {
                out.push_str("snap = true\n");
            }
            if (vp.grid_size - 20.0).abs() > 0.5 {
                out.push_str(&format!("grid-size = {}\n", vp.grid_size));
            }
            if (vp.zoom - 1.0).abs() > 0.01 {
                // Round to 2 decimal places for readability
                let z = (vp.zoom * 100.0).round() / 100.0;
                out.push_str(&format!("zoom = {}\n", z));
            }
            if vp.view_3d {
                // Check if (yaw, pitch) matches a named preset — prefer compact `camera = iso`
                let preset_name = match (vp.camera_yaw, vp.camera_pitch) {
                    (Some(y), Some(p)) => {
                        let presets: &[(&str, f32, f32)] = &[
                            ("iso",   -0.6, 0.5),
                            ("top",    0.0, 1.55),
                            ("front",  0.0, 0.05),
                            ("side",   1.57, 0.05),
                        ];
                        presets.iter().find(|(_, py, pp)| (y - py).abs() < 0.08 && (p - pp).abs() < 0.08)
                            .map(|(name, _, _)| *name)
                    }
                    _ => None,
                };
                if let Some(name) = preset_name {
                    out.push_str(&format!("camera = {}\n", name));
                } else {
                    out.push_str("view = 3d\n");
                    if let Some(yaw) = vp.camera_yaw {
                        out.push_str(&format!("camera_yaw = {:.4}\n", yaw));
                    }
                    if let Some(pitch) = vp.camera_pitch {
                        out.push_str(&format!("camera_pitch = {:.4}\n", pitch));
                    }
                }
            }
        }
        if has_layer_names {
            let mut sorted_layers: Vec<(&i32, &String)> = doc.layer_names.iter().collect();
            sorted_layers.sort_by_key(|&(k, _)| k);
            for (idx, name) in sorted_layers {
                out.push_str(&format!("layer{} = {}\n", *idx, name));
            }
        }
        out.push('\n');
    }

    out
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Section {
    None,
    /// `default_z` is applied to any node in this section that doesn't have an explicit {z:N} tag.
    Nodes { default_z: f32 },
    Flow,
    Notes,
    Groups,
    Config,
    Palette,
    /// `## Summary` / `## About` — prose paragraph stored as doc.description.
    Summary,
    /// `## Steps` — numbered list creates sequential flowchart nodes with auto edges.
    /// Tracks (last_step_id, section_z, step_count).
    Steps { default_z: f32, last_step_id: Option<NodeId>, step_count: u32 },
    /// `## Grid [cols=N]` — nodes are laid out in a grid (cols wide).
    /// Nodes are collected in order; positions are assigned after the section ends.
    Grid { cols: usize, default_z: f32, nodes: Vec<NodeId> },
    /// `## Period N: Label` — nodes parsed below belong to this timeline period.
    Period { label: String },
    /// `## Lane N: Name` — declares a swim-lane (order preserved in doc.timeline_lanes).
    Lane,
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

/// Expand a multi-target shorthand line into individual lines.
///
/// `api -> [pg, redis, worker] {dashed}` expands to:
/// ```text
/// api -> pg {dashed}
/// api -> redis {dashed}
/// api -> worker {dashed}
/// ```
///
/// Returns `None` if the line does not use multi-target syntax.
fn expand_multi_target(line: &str) -> Option<Vec<String>> {
    // Find the arrow and the opening bracket that follows
    let arrow = if line.contains("-->") { "-->" }
        else if line.contains("<->") { "<->" }
        else if line.contains("<--") { "<--" }
        else if line.contains("->")  { "->"  }
        else { return None; };

    let arrow_pos = line.find(arrow)?;
    let after_arrow = line[arrow_pos + arrow.len()..].trim_start();
    if !after_arrow.starts_with('[') { return None; }

    let source_part = line[..arrow_pos].trim();
    let close = after_arrow.find(']')?;
    let ids_str = &after_arrow[1..close];
    let tail = after_arrow[close + 1..].trim(); // any tags after ]

    let targets: Vec<&str> = ids_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if targets.is_empty() { return None; }

    let expanded: Vec<String> = targets
        .into_iter()
        .map(|t| {
            if tail.is_empty() {
                format!("{} {} {}", source_part, arrow, t)
            } else {
                format!("{} {} {} {}", source_part, arrow, t, tail)
            }
        })
        .collect();
    Some(expanded)
}

/// Expand a multi-source shorthand line into individual lines.
///
/// `[web, mobile] -> api {thick}` expands to:
/// ```text
/// web -> api {thick}
/// mobile -> api {thick}
/// ```
///
/// Returns `None` if the line does not use multi-source syntax.
fn expand_multi_source(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('[') { return None; }

    let close = trimmed.find(']')?;
    let ids_str = &trimmed[1..close];
    let after_bracket = trimmed[close + 1..].trim_start();

    // Must be followed by an arrow
    let arrow = if after_bracket.starts_with("-->") { "-->" }
        else if after_bracket.starts_with("<->")  { "<->" }
        else if after_bracket.starts_with("<--")  { "<--" }
        else if after_bracket.starts_with("->")   { "->"  }
        else { return None; };

    let rest = after_bracket[arrow.len()..].trim_start(); // target + any tags

    let sources: Vec<&str> = ids_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if sources.is_empty() { return None; }

    // Preserve any leading whitespace from original line (for indented lines)
    let indent_len = line.len() - trimmed.len();
    let indent = &line[..indent_len];

    let expanded: Vec<String> = sources
        .into_iter()
        .map(|src| format!("{}{} {} {}", indent, src, arrow, rest))
        .collect();
    Some(expanded)
}

/// Parse a flow line that may be a chain: `a --> b --> c` or `a "label" --> b --> c`.
/// Also supports `->` as a shorter alias and `<--` / `<->` for reverse/bidirectional.
/// Splits into individual edges.
fn parse_flow_line_chain(
    line: &str,
    id_map: &HashMap<String, NodeId>,
    label_map: &HashMap<String, NodeId>,
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

    // Detect style-shorthand arrows before Unicode normalization:
    // -.->  or  -.--> = dashed edge
    // ==>   or  ====> = thick edge
    // ~~>   or  ~->   = animated edge
    let implicit_dashed   = line.contains("-.->");
    let implicit_thick    = line.contains("==>") || line.contains("====>");
    let implicit_animated = line.contains("~~>") || line.contains("~->");

    // Normalize Unicode arrows to ASCII equivalents before further processing
    let line_unicode_normalized: String = line
        // Style shorthand arrows → standard -->
        .replace("-.--->", "-->")
        .replace("-.-->", "-->")
        .replace("-.->" , "-->")
        .replace("====>", "-->")
        .replace("==>",   "-->")
        .replace("~~>",   "-->")
        .replace("~->",   "-->")
        // Reverse style shorthands
        .replace("<-.-", "<--")
        .replace("<===", "<--")
        .replace("<~~",  "<--")
        // Unicode arrows
        .replace('→', "-->")   // U+2192 RIGHTWARDS ARROW
        .replace('⇒', "-->")   // U+21D2 RIGHTWARDS DOUBLE ARROW
        .replace('⟶', "-->")   // U+27F6 LONG RIGHTWARDS ARROW
        .replace('←', "<--")   // U+2190 LEFTWARDS ARROW
        .replace('⟵', "<--")   // U+27F5 LONG LEFTWARDS ARROW
        .replace('↔', "<->")   // U+2194 LEFT RIGHT ARROW
        .replace('⇔', "<->")   // U+21D4 LEFT RIGHT DOUBLE ARROW
        .replace('⟷', "<->")   // U+27F7 LONG LEFT RIGHT ARROW
        ;
    let line = line_unicode_normalized.as_str();

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
            if before.is_empty() {
                // `"Display Name" -->` — whole left is a quoted label reference, no edge label
                (lbl, String::new())
            } else {
                // `node_id "edge label" -->` — before is node ID, lbl is edge label
                (before.to_string(), lbl)
            }
        } else {
            (left.to_string(), String::new())
        };

        // right may start with a Mermaid-style pipe label: |label text| node_id {tags}
        // Detect and extract it BEFORE the extract_tags call.
        let (right, pipe_label) = if right.starts_with('|') {
            if let Some(close_pipe) = right[1..].find('|') {
                let lbl = right[1..close_pipe + 1].trim().to_string();
                let after_pipe = right[close_pipe + 2..].trim();
                (after_pipe, lbl)
            } else {
                (right, String::new())
            }
        } else {
            (right, String::new())
        };

        // right: node id (first word), optional ": colon label", then optional {tags}
        // Supports: `b: performs auth {dashed}` or `b {dashed}` or `b`
        // Also: `"Display Name" {tags}` — quoted label reference (node lookup by label).
        // We must extract {tags} first to avoid a colon inside a tag confusing us.
        let (to_id_raw, edge_tags) = extract_tags(right);
        // Now to_id_raw may be: "b" or "b: auth flow" or "b : auth flow" or `"Display Name"`
        let (to_id, colon_label) = {
            let raw = to_id_raw.trim();
            if raw.starts_with('"') {
                // Quoted label reference: `"Display Name"` → node id = Display Name (label lookup)
                let q_end = raw[1..].find('"').map(|i| i + 1).unwrap_or(raw.len());
                let quoted = raw[1..q_end].to_string();
                (quoted, String::new())
            } else {
                let first_token_end = raw.find(|c: char| c.is_whitespace() || c == ':')
                    .unwrap_or(raw.len());
                let first_token = &raw[..first_token_end];
                let rest = raw[first_token_end..].trim();
                if let Some(rest_after_colon) = rest.strip_prefix(':') {
                    let lbl = rest_after_colon.trim().to_string();
                    (first_token.to_string(), if lbl.is_empty() { String::new() } else { lbl })
                } else {
                    (first_token.to_string(), String::new())
                }
            }
        };
        // Prefix quoted label > pipe label > colon label (priority order)
        let label = if !label.is_empty() {
            label.clone()
        } else if !pipe_label.is_empty() {
            pipe_label
        } else {
            colon_label
        };

        let source_node_id = id_map.get(&from_id)
            .or_else(|| label_map.get(&slugify(&from_id, 0)))
            .ok_or_else(|| {
                let hint = suggest_id(&from_id, id_map.keys().map(|s| s.as_str()));
                format!("Line {}: unknown node id '{}'{}", line_num + 1, from_id, hint)
            })?;
        let target_node_id = id_map.get(&to_id)
            .or_else(|| label_map.get(&slugify(&to_id, 0)))
            .or_else(|| label_map.get(&slugify(to_id_raw.trim(), 0)))
            .ok_or_else(|| {
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
            } else if etag.starts_with("note:") || etag.starts_with("comment:") || etag.starts_with("annotation:") {
                let val = if etag.starts_with("note:") { &etag[5..] }
                    else if etag.starts_with("comment:") { &etag[8..] }
                    else { &etag[11..] };
                edge.comment = val.trim().to_string();
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
                    "arrow:open" | "open" => edge.style.arrow_head = ArrowHead::Open,
                    "arrow:circle" | "circle-end" => edge.style.arrow_head = ArrowHead::Circle,
                    "arrow:none" | "no-arrow" | "line" => edge.style.arrow_head = ArrowHead::None,
                    // Support escalation shorthand: thick red glow (urgent escalation path)
                    "escalate" | "escalation" | "escalated" => {
                        edge.style.color = [243, 139, 168, 255]; // red
                        edge.style.width = 3.5;
                        edge.style.glow = true;
                    }
                    // Resolution dependency: green dashed (this edge resolves that ticket)
                    "resolves" | "resolved-by" | "fixes" | "closes" => {
                        edge.style.color = [166, 227, 161, 255]; // green
                        edge.style.dashed = true;
                    }
                    // Blocks/blocks dependency: orange thick
                    "blocks" | "blocked-by" | "blocking" => {
                        edge.style.color = [250, 179, 135, 255]; // orange
                        edge.style.width = 3.0;
                    }
                    _ => {}
                }
            }
        }
        // Apply line-level implicit style flags from shorthand arrows
        if implicit_dashed   && !edge.style.dashed    { edge.style.dashed    = true; }
        if implicit_thick    && edge.style.width < 4.0 { edge.style.width    = 5.0; }
        if implicit_animated && !edge.style.animated   { edge.style.animated = true; }
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
fn parse_node_line(line: &str, line_num: usize) -> Result<(String, Node, Vec<String>), String> {
    let id_start = line.find('[').ok_or_else(|| {
        format!("Line {}: expected [id] in node definition", line_num + 1)
    })?;
    let id_end = line.find(']').ok_or_else(|| {
        format!("Line {}: missing closing ] in node id", line_num + 1)
    })?;
    let id = line[id_start + 1..id_end].trim().to_string();
    let rest = line[id_end + 1..].trim();

    let (raw_label, tags) = extract_tags(rest);
    // Process \n escape sequences in labels for multi-line node text
    let label = raw_label.replace("\\n", "\n");

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
    let mut font_size_override: Option<f32> = None;
    let mut gradient = false;
    let mut locked = false;
    let mut url_override: Option<String> = None;
    let mut border_color: Option<[u8; 4]> = None;
    let mut text_color: Option<[u8; 4]> = None;
    let mut tooltip_text: Option<String> = None;
    let mut sublabel_text: Option<String> = None;
    let mut node_note_text: Option<String> = None;
    let mut depth_3d: f32 = 0.0;
    let mut highlight = false;
    let mut progress: f32 = 0.0;
    let mut node_glow = false;
    let mut tier_color_tag = false;
    let mut gradient_angle: Option<u8> = None;
    let mut frame_color_override: Option<[u8; 4]> = None;
    let mut collapsed = false;
    let mut lane_tag: Option<String> = None;
    let mut section_override: Option<String> = None;
    let mut created_date_tag: Option<String> = None;
    let mut priority_tag: u8 = 0;
    let mut metric_value: Option<String> = None;
    let mut owner_value: Option<String> = None;
    let mut dep_targets: Vec<String> = Vec::new();
    for tag in &tags {
        if tag.starts_with("z:") {
            if let Ok(z) = tag[2..].trim().parse::<f32>() {
                z_offset = z;
            }
        } else if tag.starts_with("3d-depth:") || (tag.starts_with("depth:") && !tag.starts_with("depth-scale:")) {
            // {3d-depth:N} — custom extrusion depth in 3D view (world units)
            let colon = tag.find(':').unwrap();
            if let Ok(d) = tag[colon+1..].trim().parse::<f32>() {
                depth_3d = d.clamp(0.0, 400.0);
            }
        } else if tag == "back" || tag == "background" || tag == "ground" {
            // Depth shortcuts — snap to standard 3D tiers without knowing numbers
            z_offset = 0.0;
        } else if tag == "far" {
            z_offset = 120.0;
        } else if tag == "mid" || tag == "middle" || tag == "center-z" {
            z_offset = 240.0;
        } else if tag == "near" || tag == "close" {
            z_offset = 360.0;
        } else if tag == "front" || tag == "foreground" || tag == "top-z" {
            z_offset = 480.0;
        } else if tag.starts_with("layer:") || tag.starts_with("level:") || tag.starts_with("tier:") {
            // {layer:N} / {level:N} / {tier:N} — numeric: z = N * 120
            // {layer:name} — named semantic tier (db=0, api=120, frontend=240, edge=360, infra=480)
            let colon = tag.find(':').unwrap();
            let val = tag[colon+1..].trim();
            if let Ok(v) = val.parse::<f32>() {
                z_offset = v * 120.0;
            } else {
                z_offset = match val.to_lowercase().as_str() {
                    "db" | "data" | "database" | "storage" | "store"
                    | "cache" | "queue" | "mq" | "persistence" => 0.0,
                    "app" | "api" | "service" | "server" | "backend"
                    | "biz" | "logic" | "worker" | "handler" | "core" => 120.0,
                    "ui" | "frontend" | "client" | "web" | "browser"
                    | "view" | "spa" | "mobile" | "app-ui" => 240.0,
                    "edge" | "gateway" | "lb" | "proxy" | "cdn"
                    | "ingress" | "router" | "balancer" => 360.0,
                    "infra" | "platform" | "ops" | "host"
                    | "k8s" | "kubernetes" | "cloud" | "network" => 480.0,
                    _ => z_offset, // unknown name — leave z unchanged
                };
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
        } else if tag == "tiny" || tag == "xs" {
            // Size shorthands — set both width and height
            width_override  = Some(70.0);
            height_override = Some(36.0);
        } else if tag == "small" || tag == "sm" {
            width_override  = Some(110.0);
            height_override = Some(50.0);
        } else if tag == "medium" || tag == "md" {
            width_override  = Some(160.0);
            height_override = Some(70.0);
        } else if tag == "large" || tag == "lg" {
            width_override  = Some(220.0);
            height_override = Some(100.0);
        } else if tag == "xlarge" || tag == "xl" {
            width_override  = Some(300.0);
            height_override = Some(140.0);
        } else if tag == "wide" {
            width_override  = Some(240.0);
            height_override = Some(60.0);
        } else if tag == "tall" {
            width_override  = Some(80.0);
            height_override = Some(180.0);
        } else if tag.starts_with("r:") || tag.starts_with("radius:") || tag.starts_with("corner:") {
            let colon = tag.find(':').unwrap();
            corner_radius = tag[colon+1..].trim().parse::<f32>().ok();
        } else if tag == "rounded" || tag == "round" {
            corner_radius = Some(12.0);
        } else if tag == "pill-shape" {
            corner_radius = Some(50.0);
        } else if tag == "sharp" || tag == "square" {
            corner_radius = Some(0.0);
        } else if tag.starts_with("icon:") || tag.starts_with("badge:") || tag.starts_with("v:") {
            let colon = tag.find(':').unwrap();
            icon = Some(tag[colon+1..].trim().to_string());
        } else if tag == "done" || tag == "complete" || tag == "completed" || tag == "finished" {
            // Status shorthand: done → full progress + Ok badge
            progress = 1.0;
            if node_tag.is_none() { node_tag = Some(NodeTag::Ok); }
        } else if tag == "wip" || tag == "in-progress" || tag == "doing" || tag == "active" {
            // Status shorthand: wip → half progress + Info badge
            if progress < 0.01 { progress = 0.5; }
            if node_tag.is_none() { node_tag = Some(NodeTag::Info); }
        } else if tag == "review" || tag == "in-review" || tag == "reviewing" {
            // Status shorthand: review → 75% progress + Warning badge
            if progress < 0.01 { progress = 0.75; }
            if node_tag.is_none() { node_tag = Some(NodeTag::Warning); }
        } else if tag == "blocked" || tag == "stuck" || tag == "failed" || tag == "error" {
            // Status shorthand: blocked → Critical badge (no progress implied)
            if node_tag.is_none() { node_tag = Some(NodeTag::Critical); }
        } else if tag == "todo" || tag == "pending" || tag == "backlog" || tag == "queued" {
            // Status shorthand: todo → Warning badge (no progress)
            if node_tag.is_none() { node_tag = Some(NodeTag::Warning); }
        } else if tag == "p1" || tag == "priority-1" || tag == "sev1" || tag == "sev-1" {
            // Support priority/severity shorthand: P1/SEV-1 → Critical badge + red fill
            if node_tag.is_none() { node_tag = Some(NodeTag::Critical); }
            if fill_color.is_none() { fill_color = Some([243, 139, 168, 255]); } // red
            if priority_tag == 0 { priority_tag = 1; }
        } else if tag == "p2" || tag == "priority-2" || tag == "sev2" || tag == "sev-2" {
            // Support priority: P2/SEV-2 → Warning badge + orange fill
            if node_tag.is_none() { node_tag = Some(NodeTag::Warning); }
            if fill_color.is_none() { fill_color = Some([250, 179, 135, 255]); } // orange
            if priority_tag == 0 { priority_tag = 2; }
        } else if tag == "p3" || tag == "priority-3" || tag == "sev3" || tag == "sev-3" {
            // Support priority: P3/SEV-3 → Info badge + blue fill
            if node_tag.is_none() { node_tag = Some(NodeTag::Info); }
            if fill_color.is_none() { fill_color = Some([137, 180, 250, 255]); } // blue
            if priority_tag == 0 { priority_tag = 3; }
        } else if tag == "p4" || tag == "priority-4" {
            // Support priority: P4 → no badge (low), subtle green fill
            if fill_color.is_none() { fill_color = Some([166, 227, 161, 255]); } // green
            if priority_tag == 0 { priority_tag = 4; }
        } else if tag == "escalated" || tag == "escalate" {
            // Escalation shorthand: escalated → Critical badge + glow (needs immediate attention)
            if node_tag.is_none() { node_tag = Some(NodeTag::Critical); }
            node_glow = true;
        } else if tag == "urgent" || tag == "critical-now" || tag == "hot" {
            // Urgent shorthand: P1 + glow + red fill (combines priority + escalation)
            if node_tag.is_none() { node_tag = Some(NodeTag::Critical); }
            if fill_color.is_none() { fill_color = Some([243, 139, 168, 255]); }
            node_glow = true;
        } else if tag == "wontfix" || tag == "won't-fix" || tag == "wont-fix" || tag == "no-fix" || tag == "closed" {
            // Won't fix / closed: gray fill, no tag (neutral closure state)
            if fill_color.is_none() { fill_color = Some([88, 91, 112, 255]); } // gray
            if opacity_override.is_none() { opacity_override = Some(0.7); }
        } else if tag == "pending" || tag == "waiting" || tag == "on-hold" {
            // Pending/waiting: blue-gray fill, Warning tag (needs attention but blocked)
            if node_tag.is_none() { node_tag = Some(NodeTag::Warning); }
            if fill_color.is_none() { fill_color = Some([108, 112, 134, 255]); } // blue-gray
        } else if tag == "in-progress" || tag == "active" || tag == "doing" {
            // Explicit in-progress: maps to WIP (Info tag + progress)
            if node_tag.is_none() { node_tag = Some(NodeTag::Info); }
            if progress < 0.01 { progress = 0.5; }
        } else if tag == "glow" || tag == "neon" || tag == "glow-node" {
            node_glow = true;
        } else if tag.starts_with("shape:") || tag.starts_with("type:") || tag.starts_with("kind:") {
            // {shape:circle} / {type:diamond} / {kind:hexagon} — explicit property-style shape
            let colon = tag.find(':').unwrap();
            let val = tag[colon+1..].trim();
            shape = tag_to_shape(val);
        } else if tag.starts_with("status:") {
            // {status:done} / {status:wip} / {status:blocked} etc — property-style status
            let val = tag[7..].trim();
            // Re-dispatch to status shorthands
            match val {
                "done" | "complete" | "completed" | "finished" => {
                    progress = 1.0;
                    if node_tag.is_none() { node_tag = Some(NodeTag::Ok); }
                }
                "wip" | "in-progress" | "doing" | "active" => {
                    if progress < 0.01 { progress = 0.5; }
                    if node_tag.is_none() { node_tag = Some(NodeTag::Info); }
                }
                "review" | "in-review" | "reviewing" => {
                    if progress < 0.01 { progress = 0.75; }
                    if node_tag.is_none() { node_tag = Some(NodeTag::Warning); }
                }
                "blocked" | "stuck" | "failed" | "error" => {
                    if node_tag.is_none() { node_tag = Some(NodeTag::Critical); }
                }
                "todo" | "pending" | "backlog" | "queued" => {
                    if node_tag.is_none() { node_tag = Some(NodeTag::Warning); }
                }
                other => {
                    if let Some(nt) = tag_to_node_tag(other) {
                        node_tag = Some(nt);
                    }
                }
            }
        } else if tag == "tier-color" || tag == "auto-color" || tag == "tint" || tag == "tier-tint" {
            // {tier-color}: auto-assign fill based on z-tier (applied after all tags)
            tier_color_tag = true;
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
        } else if tag.starts_with("font-size:") || tag.starts_with("fs:") || tag.starts_with("fontsize:") || tag.starts_with("text-size:") || tag.starts_with("textsize:") {
            let colon = tag.find(':').unwrap();
            font_size_override = tag[colon+1..].trim().parse::<f32>().ok();
        } else if tag.starts_with("opacity:") || tag.starts_with("alpha:") {
            let val_str = if tag.starts_with("opacity:") { &tag[8..] } else { &tag[6..] };
            if let Ok(v) = val_str.trim().parse::<f32>() {
                // Accept 0-100 percentage or 0.0-1.0 float
                opacity_override = Some(if v > 1.0 { v / 100.0 } else { v });
            }
        } else if tag == "hidden" || tag == "invisible" {
            opacity_override = Some(0.0);
        } else if tag == "dim" || tag == "dimmed" {
            opacity_override = Some(0.35);
        } else if tag == "ghost" || tag == "faded" {
            opacity_override = Some(0.18);
        } else if tag == "muted" {
            if opacity_override.is_none() { opacity_override = Some(0.6); }
        } else if tag == "gradient" || tag == "grad" {
            gradient = true;
        } else if tag.starts_with("gradient-angle:") || tag.starts_with("grad-angle:") || tag.starts_with("gradient:") && tag.len() > 9 && tag[9..].trim().parse::<u8>().is_ok() {
            // {gradient-angle:45} or {gradient:45} — gradient direction in degrees
            let colon = tag.find(':').unwrap();
            if let Ok(a) = tag[colon+1..].trim().parse::<u8>() {
                gradient = true; // gradient-angle implies gradient
                gradient_angle = Some(a);
            }
        } else if tag.starts_with("frame-color:") || tag.starts_with("frame-fill:") || tag.starts_with("bg-color:") {
            // {frame-color:#rrggbb} — override group frame background color
            let colon = tag.find(':').unwrap();
            let v = tag[colon+1..].trim();
            if let Some(c) = parse_hex_color(v).or_else(|| tag_to_fill_color(v)) {
                frame_color_override = Some(c);
            }
        } else if tag == "collapsed" || tag == "collapse" || tag == "compact" || tag == "pill" {
            collapsed = true;
        } else if tag == "locked" || tag == "lock" {
            locked = true;
        } else if tag.starts_with("url:") || tag.starts_with("link:") {
            let prefix_len = if tag.starts_with("url:") { 4 } else { 5 };
            url_override = Some(tag[prefix_len..].trim().to_string());
        } else if tag.starts_with("lane:") {
            // {lane:Name} — assign node to a timeline swim-lane
            let lane_name = tag[5..].trim().to_string();
            if !lane_name.is_empty() {
                lane_tag = Some(lane_name);
            }
        } else if tag.starts_with("tooltip:") || tag.starts_with("tip:") || tag.starts_with("desc:") {
            let prefix = if tag.starts_with("tooltip:") { 8 } else if tag.starts_with("tip:") { 4 } else { 5 };
            tooltip_text = Some(tag[prefix..].trim().to_string());
        } else if tag.starts_with("due:") || tag.starts_with("deadline:") || tag.starts_with("by:") {
            // {due:2026-03-20} / {deadline:Q2} → sublabel "📅 date" (compose with existing assignee)
            let prefix = if tag.starts_with("due:") { 4 }
                else if tag.starts_with("deadline:") { 9 }
                else { 3 }; // by:
            let date = tag[prefix..].trim();
            if !date.is_empty() {
                let due_part = format!("📅 {date}");
                sublabel_text = Some(match &sublabel_text {
                    Some(existing) if existing.starts_with("👤") => format!("{}\n{}", existing, due_part),
                    _ => due_part,
                });
            }
        } else if tag.starts_with("assigned:") || tag.starts_with("owner:") || tag.starts_with("assignee:") {
            // {assigned:Alice} / {owner:Bob} → sublabel "👤 name" (compose with existing due date)
            // Also stored in node.owner for programmatic access
            let prefix = if tag.starts_with("assigned:") { 9 }
                else if tag.starts_with("owner:") { 6 }
                else { 9 }; // assignee:
            let name = tag[prefix..].trim();
            if !name.is_empty() {
                owner_value = Some(name.to_string());
                let person_part = format!("👤 {name}");
                sublabel_text = Some(match &sublabel_text {
                    Some(existing) if existing.starts_with("📅") => format!("{}\n{}", person_part, existing),
                    _ => person_part,
                });
            }
        } else if tag.starts_with("created:") || tag.starts_with("opened:") || tag.starts_with("started:") {
            // {created:YYYY-MM-DD} — ticket creation date for age badge
            let prefix = if tag.starts_with("created:") { 8 }
                else if tag.starts_with("opened:") { 7 }
                else { 8 }; // started:
            let date = tag[prefix..].trim();
            if !date.is_empty() { created_date_tag = Some(date.to_string()); }
        } else if tag.starts_with("section:") || tag.starts_with("stage:") || tag.starts_with("board:") || tag.starts_with("col:") || tag.starts_with("column:") {
            // {section:Intake} — assign node to a kanban section inline (overrides header-based section)
            let prefix = if tag.starts_with("section:") { 8 }
                else if tag.starts_with("stage:") { 6 }
                else if tag.starts_with("board:") { 6 }
                else if tag.starts_with("col:") { 4 }
                else { 7 }; // column:
            let sec = tag[prefix..].trim().to_string();
            if !sec.is_empty() { section_override = Some(sec); }
        } else if tag.starts_with("sublabel:") || tag.starts_with("sub:") || tag.starts_with("subtitle:") || tag.starts_with("caption:") {
            let prefix = if tag.starts_with("sublabel:") { 9 }
                else if tag.starts_with("sub:") { 4 }
                else if tag.starts_with("subtitle:") { 9 }
                else { 8 }; // caption:
            sublabel_text = Some(tag[prefix..].trim().to_string());
        } else if tag.starts_with("note:") || tag.starts_with("annotation:") || tag.starts_with("comment:") {
            // {note:text} on a node → shown as a 💬 annotation in the tooltip
            let prefix = if tag.starts_with("note:") { 5 }
                else if tag.starts_with("annotation:") { 11 }
                else { 8 }; // comment:
            node_note_text = Some(tag[prefix..].trim().to_string());
        } else if tag.starts_with("border-color:") || tag.starts_with("stroke:") {
            let v = if tag.starts_with("border-color:") { &tag[13..] } else { &tag[7..] };
            border_color = tag_to_fill_color(v.trim());
        } else if tag.starts_with("text-color:") || tag.starts_with("color:") {
            let v = if tag.starts_with("text-color:") { &tag[11..] } else { &tag[6..] };
            text_color = tag_to_fill_color(v.trim());
        } else if tag == "shadow" || tag == "drop-shadow" {
            shadow = true;
        } else if tag == "highlight" || tag == "pulse" || tag == "starred" || tag == "important" || tag == "star" {
            highlight = true;
        } else if tag.starts_with("progress:") || tag.starts_with("pct:") || tag.starts_with("percent:") {
            let prefix = if tag.starts_with("progress:") { 9 } else if tag.starts_with("pct:") { 4 } else { 8 };
            let val_str = tag[prefix..].trim().trim_end_matches('%');
            if let Ok(v) = val_str.parse::<f32>() {
                // Accept 0–100 range or 0.0–1.0 range
                progress = if v > 1.0 { (v / 100.0).clamp(0.0, 1.0) } else { v.clamp(0.0, 1.0) };
            }
        } else if tag == "bold" || tag == "strong" {
            bold = true;
        } else if tag == "italic" || tag == "em" {
            italic = true;
        } else if tag == "dashed-border" || tag == "dashed_border" || tag == "border-dashed" {
            dashed_border = true;
        } else if tag == "milestone" {
            // {milestone} — marks a key milestone; renders as a Diamond shape
            shape = NodeShape::Diamond;
        } else if tag == "revenue" {
            // {revenue} — green fill to highlight revenue-generating nodes
            if fill_color.is_none() { fill_color = Some([166, 227, 161, 255]); } // green
        } else if tag == "cost" {
            // {cost} — red fill to highlight cost / spend nodes
            if fill_color.is_none() { fill_color = Some([243, 139, 168, 255]); } // red
        } else if tag == "growth" {
            // {growth} — yellow fill + upward arrow sublabel for growth metrics
            if fill_color.is_none() { fill_color = Some([249, 226, 175, 255]); } // yellow
            if sublabel_text.is_none() { sublabel_text = Some("↑".to_string()); }
        } else if tag == "opportunity" {
            // {opportunity} — blue fill + star sublabel for market / product opportunities
            if fill_color.is_none() { fill_color = Some([137, 180, 250, 255]); } // blue
            if sublabel_text.is_none() { sublabel_text = Some("★".to_string()); }
        } else if tag == "risk" {
            // {risk} — Warning node tag to surface risk items in the status bar
            if node_tag.is_none() { node_tag = Some(NodeTag::Warning); }
        } else if let Some((preset_shape, preset_color)) = tag_to_preset(tag) {
            // Semantic preset: sets shape AND fill color at once
            shape = preset_shape;
            if fill_color.is_none() {
                fill_color = Some(preset_color);
            }
        } else if let Some(val) = tag.strip_prefix("metric:") {
            // {metric:$2.4M ARR} — business metric badge shown on the node
            metric_value = Some(val.to_string());
        } else if let Some(val) = tag.strip_prefix("dep:") {
            // {dep:target} — declares a dependency edge from this node to target
            // Targets can be hrf_ids or natural-language labels; resolved in a post-pass
            let dep = val.trim().to_string();
            if !dep.is_empty() { dep_targets.push(dep); }
        } else if is_emoji_only(tag) {
            // {🔒} / {⚡} / {🗄️} — bare emoji shorthand: treated as {icon:emoji}
            if icon.is_none() {
                icon = Some(tag.to_string());
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
    // Apply tier-color: auto-assign fill based on z_offset tier when {tier-color} is used
    // and no explicit fill color was given.
    if tier_color_tag && fill_color.is_none() {
        fill_color = Some(z_tier_fill_color(z_offset));
    }
    if let Some(fc) = fill_color {
        node.style.fill_color = fc;
        // Auto-contrast: pick light or dark text based on fill luminance
        let luma = 0.299 * fc[0] as f32 + 0.587 * fc[1] as f32 + 0.114 * fc[2] as f32;
        node.style.text_color = if luma > 140.0 { [15, 15, 20, 255] } else { [220, 220, 230, 255] };
    }
    // Auto-size: expand width to fit label if no explicit {w:N} given.
    // Uses an approximation: ~7.5px per character at the default font size.
    // Only expands (never shrinks) and caps at 320px to stay readable.
    if width_override.is_none() && !node.is_frame {
        let label_chars = node.display_label().chars().count() as f32;
        let sublabel_chars = node.sublabel.chars().count() as f32;
        let longest = label_chars.max(sublabel_chars);
        let auto_w = (longest * 7.5 + 44.0).clamp(100.0, 320.0);
        if auto_w > node.size[0] {
            node.size[0] = auto_w;
        }
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
    if let Some(fs) = font_size_override { node.style.font_size = fs.clamp(6.0, 72.0); }
    if gradient { node.style.gradient = true; }
    if let Some(ga) = gradient_angle { node.style.gradient_angle = ga; }
    if let Some(fc) = frame_color_override { node.frame_color = fc; }
    if locked { node.locked = true; }
    if collapsed { node.collapsed = true; }
    if let Some(u) = url_override { node.url = u; }
    if let Some(bc) = border_color { node.style.border_color = bc; }
    if let Some(tc) = text_color { node.style.text_color = tc; }
    if let Some(tt) = tooltip_text {
        if let NodeKind::Shape { description, .. } = &mut node.kind {
            if description.is_empty() { *description = tt; }
        }
    }
    if let Some(sl) = sublabel_text {
        node.sublabel = sl;
    }
    if depth_3d > 0.0 {
        node.depth_3d = depth_3d;
    }
    if highlight { node.highlight = true; }
    if node_glow { node.style.glow = true; }
    if progress > 0.0 { node.progress = progress; }
    if let Some(nn) = node_note_text {
        node.comment = nn;
    }
    if let Some(lane) = lane_tag {
        node.timeline_lane = Some(lane);
    }
    if let Some(sec) = section_override {
        node.section_name = sec;
    }
    if let Some(date) = created_date_tag {
        node.created_date = date;
    }
    if priority_tag > 0 {
        node.priority = priority_tag;
    }
    // Store the user-assigned HRF ID for stable export and display
    if !id.is_empty() {
        node.hrf_id = id.clone();
    }
    if let Some(mv) = metric_value {
        node.metric = Some(mv);
    }
    if let Some(ov) = owner_value {
        node.owner = Some(ov);
    }

    Ok((id, node, dep_targets))
}

/// Extract `{tag}` blocks from a string, returning the cleaned label and list of tags.
/// Split a node line on `→` or ` -> ` (outside `{...}` tag braces).
/// Returns `(node_part, Vec<(target_id, edge_tags)>)`.
/// e.g. `"[api] API → db, redis {dashed}"` →
///   `("[api] API", [("db", []), ("redis", ["dashed"])])`
fn split_inline_edges(line: &str) -> (&str, Vec<(String, Vec<String>)>) {
    // Find → or -> outside braces
    let bytes = line.as_bytes();
    let mut depth = 0usize;
    let mut arrow_pos: Option<usize> = None;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => { if depth > 0 { depth -= 1; } }
            b'\xe2' if depth == 0 => {
                // UTF-8 for → is [0xe2, 0x86, 0x92]
                if bytes.get(i+1) == Some(&0x86) && bytes.get(i+2) == Some(&0x92) {
                    arrow_pos = Some(i);
                    break;
                }
            }
            b'-' if depth == 0 => {
                // Check for " -> " or "-> " at start
                if bytes.get(i+1) == Some(&b'>') {
                    // Make sure it's not inside [id] brackets
                    arrow_pos = Some(i);
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let Some(pos) = arrow_pos else {
        return (line, Vec::new());
    };
    let node_part = line[..pos].trim_end();
    // Arrow length: 3 bytes for →, 2 for ->
    let arrow_len = if bytes[pos] == b'\xe2' { 3 } else { 2 };
    let targets_str = line[pos + arrow_len..].trim();
    // Parse comma-separated targets, each optionally with {tags}
    let targets = targets_str.split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() { return None; }
            let (tgt_raw, edge_tags) = extract_tags(part);
            let tgt = tgt_raw.trim().to_string();
            if tgt.is_empty() { return None; }
            Some((tgt, edge_tags))
        })
        .collect();
    (node_part, targets)
}

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
                            | "tooltip" | "tip" | "desc"
                            | "lane" | "sublabel" | "sub" | "subtitle" | "caption"
                            | "note" | "annotation" | "comment"
                            | "section" | "stage" | "col" | "column" | "board"
                            | "assigned" | "owner" | "assignee"
                            | "created" | "opened" | "started" | "due" | "deadline" | "by"
                            | "metric" | "dep" => {
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
            "pink" | "red" | "critical" | "error" => color = StickyColor::Pink,
            "green" | "ok" | "success" | "done" => color = StickyColor::Green,
            "blue" | "info" | "note" | "sky" | "teal" => color = StickyColor::Blue,
            "purple" | "mauve" | "violet" | "lavender" => color = StickyColor::Purple,
            "yellow" | "warning" | "warn" | "caution" | "orange" | "peach" => color = StickyColor::Yellow,
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
        "blue"     => Some([137, 180, 250, 255]),
        "green"    => Some([166, 227, 161, 255]),
        "red"      => Some([243, 139, 168, 255]),
        "yellow"   => Some([249, 226, 175, 255]),
        "purple"   => Some([203, 166, 247, 255]),
        "mauve"    => Some([203, 166, 247, 255]),
        "pink"     => Some([245, 194, 231, 255]),
        "teal"     => Some([148, 226, 213, 255]),
        "white"    => Some([255, 255, 255, 255]),
        "black"    => Some([17, 17, 27, 255]),
        "orange"   => Some([250, 179, 135, 255]),
        "peach"    => Some([250, 179, 135, 255]),
        "sky"      => Some([137, 220, 235, 255]),
        "lavender" => Some([180, 190, 254, 255]),
        "gray" | "grey" => Some([108, 112, 134, 255]),
        "surface" | "default" => Some([30, 30, 46, 255]),
        "none" | "transparent" | "clear" => Some([0, 0, 0, 0]),
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
        [250, 179, 135, 255] => Some("orange"),
        [137, 220, 235, 255] => Some("sky"),
        [180, 190, 254, 255] => Some("lavender"),
        [108, 112, 134, 255] => Some("gray"),
        [0, 0, 0, 0]         => Some("none"),
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

/// Returns true if the tag consists entirely of emoji/pictograph characters
/// with no ASCII letters or digits — used to detect bare emoji icon shorthands.
fn is_emoji_only(tag: &str) -> bool {
    if tag.is_empty() { return false; }
    // Must have at least one non-ASCII char (emoji are > U+007F)
    if tag.is_ascii() { return false; }
    // Must not contain alphanumeric ASCII (letters or digits) or '-'/'_'
    if tag.bytes().any(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
        return false;
    }
    // Check that all scalar values are in recognized emoji/symbol ranges
    tag.chars().all(|c| {
        let cp = c as u32;
        cp > 0x7F && (
            (0x1F300..=0x1FAFF).contains(&cp)  // emoji: misc symbols, transport, nature, food, etc.
            || (0x2600..=0x27FF).contains(&cp)  // misc symbols, dingbats
            || (0x1F900..=0x1F9FF).contains(&cp)// supplemental symbols
            || (0x231A..=0x27BF).contains(&cp)  // watchface, arrows, dingbats
            || c == '\u{FE0F}'                   // variation selector-16 (emoji modifier)
            || c == '\u{20E3}'                   // combining enclosing keycap
        )
    })
}

fn tag_to_shape(tag: &str) -> NodeShape {
    match tag {
        // Rectangle variants
        "rectangle" | "rect" | "box" | "square-shape" => NodeShape::Rectangle,
        // Diamond / decision
        "diamond" | "decision" | "rhombus" | "lozenge" | "branch" | "if" | "choice" => NodeShape::Diamond,
        // Circle / oval
        "circle" | "oval" | "ellipse" | "dot" | "bubble" | "round" => NodeShape::Circle,
        // Parallelogram / IO
        "parallelogram" | "parallel" | "io" | "skew" | "input" | "output" | "data" => NodeShape::Parallelogram,
        // Hexagon
        "hexagon" | "hex" | "process" | "cluster" | "hive" | "cell" => NodeShape::Hexagon,
        // Connector / small circle
        "connector" | "api" | "interface" | "protocol" | "gateway" | "port" | "endpoint" => NodeShape::Connector,
        // Triangle / pyramid
        "triangle" | "pyramid" | "hierarchy" | "peak" | "apex" => NodeShape::Triangle,
        // Callout / speech bubble
        "callout" | "speech" | "speech-bubble" | "balloon" => NodeShape::Callout,
        // Person / actor — human silhouette for user-facing diagrams
        "person" | "user" | "actor" | "human" | "stick-figure" => NodeShape::Person,
        // Screen / UI — rounded rect with top chrome bar for mockups
        "screen" | "ui" | "mockup" | "wireframe" | "page" | "view" => NodeShape::Screen,
        // Cylinder / database drum
        "cylinder" | "db" | "database" | "storage" | "drum" => NodeShape::Cylinder,
        // Cloud — blob outline for SaaS / cloud infrastructure
        "cloud" | "saas" | "aws" | "gcp" | "azure" | "infra" => NodeShape::Cloud,
        // Document — rectangle with folded corner
        "document" | "doc" | "file" | "report" | "spec" => NodeShape::Document,
        // Channel / funnel — pipeline or funnel shapes
        "channel" | "funnel" | "pipeline" | "flow-channel" => NodeShape::Channel,
        // Segment / group — person-group shape for cohorts or teams
        "segment" | "group" | "audience" | "cohort" | "team" => NodeShape::Segment,
        // Default
        _ => NodeShape::RoundedRect,
    }
}

/// Returns a muted tier-specific fill color for a given z-offset.
/// Used by the {tier-color} tag and auto-tier-color config option.
fn z_tier_fill_color(z: f32) -> [u8; 4] {
    match z.round() as i32 {
        0   => [28, 55, 100, 255],   // db — deep navy (complements blue accent)
        120 => [25, 80, 68, 255],    // api — deep teal
        240 => [68, 40, 105, 255],   // frontend — deep purple
        360 => [105, 65, 20, 255],   // edge — deep amber
        480 => [48, 50, 70, 255],    // infra — deep slate
        _   => [49, 50, 68, 255],    // unknown — default fill
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
        // Pyramid / hierarchy presets
        "pyramid"   | "hierarchy" | "priority-level" | "tier"
                    => Some((NodeShape::Triangle,    [203, 166, 247, 255])), // purple
        // Hypothesis / design-thinking presets
        "hypothesis" | "guess" | "theory"
                    => Some((NodeShape::Diamond,     [250, 179, 135, 255])), // peach
        "assumption" | "premise" | "given"
                    => Some((NodeShape::Parallelogram, [137, 180, 250, 255])), // blue
        "evidence"  | "fact" | "data" | "proof" | "finding"
                    => Some((NodeShape::Rectangle,   [166, 227, 161, 255])), // green
        "conclusion" | "result" | "outcome" | "insight"
                    => Some((NodeShape::Hexagon,     [203, 166, 247, 255])), // purple
        "question"  | "ask" | "unknown" | "query"
                    => Some((NodeShape::Circle,      [249, 226, 175, 255])), // yellow
        "cause"     | "root-cause" | "reason" | "why"
                    => Some((NodeShape::Diamond,     [243, 139, 168, 255])), // red
        "effect"    | "impact" | "consequence"
                    => Some((NodeShape::RoundedRect, [148, 226, 213, 255])), // teal
        "idea"      | "concept" | "brainstorm" | "thought"
                    => Some((NodeShape::Circle,      [245, 194, 231, 255])), // pink
        "risk"      | "threat" | "issue" | "blocker" | "concern"
                    => Some((NodeShape::Diamond,     [250, 179, 135, 255])), // orange
        "goal"      | "objective" | "target" | "aim"
                    => Some((NodeShape::RoundedRect, [166, 227, 161, 255])), // green
        "strength"  => Some((NodeShape::RoundedRect, [166, 227, 161, 255])), // green SWOT
        "weakness"  => Some((NodeShape::RoundedRect, [243, 139, 168, 255])), // red SWOT
        "opportunity" => Some((NodeShape::RoundedRect, [137, 180, 250, 255])), // blue SWOT
        "threat-swot" => Some((NodeShape::RoundedRect, [249, 226, 175, 255])), // yellow SWOT
        "how-might-we" | "hmw"
                    => Some((NodeShape::RoundedRect, [245, 194, 231, 255])), // pink HMW
        "experiment" | "test" | "trial"
                    => Some((NodeShape::Hexagon,     [249, 226, 175, 255])), // yellow
        "metric"    | "kpi" | "measure"
                    => Some((NodeShape::Rectangle,   [148, 226, 213, 255])), // teal
        // Empathy map presets
        "quote"     | "verbatim" | "observation"
                    => Some((NodeShape::Callout, [245, 194, 231, 255])), // callout, pink
        "pain"      | "frustration" | "blocker-ux"
                    => Some((NodeShape::Diamond,     [243, 139, 168, 255])), // red/pink
        "gain"      | "delight" | "win"
                    => Some((NodeShape::RoundedRect, [166, 227, 161, 255])), // green
        // Value proposition presets
        "job"       | "jtbd"
                    => Some((NodeShape::Rectangle,   [137, 180, 250, 255])), // blue
        _ => None,
    }
}

/// Format a single non-sticky node into HRF text and append to `out`.
/// `z_tag` is pre-computed so callers can suppress it when a section already implies the z.
fn export_node_to_hrf(node: &Node, id: &str, z_tag: &str, out: &mut String) {
    match &node.kind {
        NodeKind::Shape { shape, label: raw_label, description } => {
            // Escape actual newlines in label back to \n for HRF text format
            let label_owned;
            let label: &str = if raw_label.contains('\n') {
                label_owned = raw_label.replace('\n', "\\n");
                &label_owned
            } else {
                raw_label.as_str()
            };
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
                    NodeShape::Triangle => " {triangle}",
                    NodeShape::Callout => " {callout}",
                    NodeShape::Person => " {person}",
                    NodeShape::Screen => " {screen}",
                    NodeShape::Cylinder => " {cylinder}",
                    NodeShape::Cloud => " {cloud}",
                    NodeShape::Document => " {document}",
                    NodeShape::Channel => " {channel}",
                    NodeShape::Segment => " {segment}",
                }
            };
            // Prefer {p1}/{p2}/{p3}/{p4} over generic tag names when priority is set
            let tag_tag_owned: String;
            let tag_tag = if node.priority > 0 {
                tag_tag_owned = format!(" {{p{}}}", node.priority);
                &tag_tag_owned
            } else {
                match node.tag {
                    Some(NodeTag::Critical) => " {critical}",
                    Some(NodeTag::Warning) => " {warning}",
                    Some(NodeTag::Ok) => " {ok}",
                    Some(NodeTag::Info) => " {info}",
                    None => "",
                }
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
            let font_size_tag = if (node.style.font_size - 13.0).abs() > 0.5 {
                format!(" {{font-size:{}}}", node.style.font_size)
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
                // Use friendly shorthand names for well-known opacity levels
                let pct = (node.style.opacity * 100.0).round() as u32;
                let friendly = match pct {
                    0  => " {hidden}".to_string(),
                    18 => " {ghost}".to_string(),
                    35 => " {dim}".to_string(),
                    60 => " {muted}".to_string(),
                    _  => format!(" {{opacity:{}}}", pct),
                };
                friendly
            } else { String::new() };
            let gradient_tag = if node.style.gradient {
                if node.style.gradient_angle > 0 {
                    format!(" {{gradient}} {{gradient-angle:{}}}", node.style.gradient_angle)
                } else {
                    " {gradient}".to_string()
                }
            } else { String::new() };
            let locked_tag = if node.locked { " {locked}" } else { "" };
            let collapsed_tag = if node.collapsed { " {collapsed}" } else { "" };
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
            // Multi-line sublabel: split "👤 Alice\n📅 date" into separate {assigned:}/{due:} tags
            let sublabel_tag = if !node.sublabel.is_empty() {
                let mut parts = Vec::new();
                for line in node.sublabel.split('\n') {
                    if let Some(name) = line.strip_prefix("👤 ") {
                        parts.push(format!(" {{assigned:{}}}", name.trim()));
                    } else if let Some(date) = line.strip_prefix("📅 ") {
                        parts.push(format!(" {{due:{}}}", date.trim()));
                    } else if !line.is_empty() {
                        parts.push(format!(" {{sublabel:{}}}", line));
                    }
                }
                parts.join("")
            } else { String::new() };
            let depth_3d_tag = if node.depth_3d > 0.0 {
                format!(" {{3d-depth:{:.0}}}", node.depth_3d)
            } else { String::new() };
            let highlight_tag = if node.highlight { " {highlight}" } else { "" };
            let progress_tag = if node.progress > 0.0 {
                format!(" {{progress:{}}}", (node.progress * 100.0).round() as u32)
            } else { String::new() };
            let note_tag = if !node.comment.is_empty() {
                format!(" {{note:{}}}", node.comment)
            } else { String::new() };
            let glow_tag = if node.style.glow { " {glow}" } else { "" };
            let frame_color_tag = if node.is_frame {
                let default_fc = crate::model::default_frame_color();
                if node.frame_color != default_fc {
                    let fc = node.frame_color;
                    format!(" {{frame-color:#{:02x}{:02x}{:02x}}}", fc[0], fc[1], fc[2])
                } else { String::new() }
            } else { String::new() };
            let created_date_tag = if !node.created_date.is_empty() {
                format!(" {{created:{}}}", node.created_date)
            } else { String::new() };
            out.push_str(&format!("- [{}] {}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}\n",
                id, label, shape_tag, z_tag, tag_tag, pin_tag, fill_tag, icon_tag,
                gradient_tag, shadow_tag, bold_tag, italic_tag, dashed_border_tag, radius_tag,
                border_tag, opacity_tag, locked_tag, collapsed_tag, url_tag, align_tag, valign_tag,
                border_color_tag, text_color_tag, font_size_tag, w_tag, h_tag, sublabel_tag, depth_3d_tag, highlight_tag, progress_tag, note_tag, glow_tag, frame_color_tag, created_date_tag));
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

        // Re-import should preserve position (find by label, not index)
        let doc2 = parse_hrf(&exported).unwrap();
        let a2 = doc2.nodes.iter().find(|n| n.display_label() == "Fixed Node")
            .expect("Fixed Node not found in re-imported doc");
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

    #[test]
    fn test_steps_section_creates_sequential_flow() {
        let input = r#"
# Order Process

## Steps
1. Customer places order
2. Payment validation {diamond}
3. Warehouse picks items
4. Shipped to customer {fill:green}
"#;
        let doc = parse_hrf(input).unwrap();
        // Should have 4 nodes
        assert_eq!(doc.nodes.len(), 4, "expected 4 step nodes");
        // Should have 3 edges connecting them in sequence
        assert_eq!(doc.edges.len(), 3, "expected 3 sequential edges");
        // Labels should be set correctly
        let labels: Vec<&str> = doc.nodes.iter().map(|n| n.display_label()).collect();
        assert!(labels.iter().any(|l| l.contains("Customer places order")), "label mismatch: {:?}", labels);
        assert!(labels.iter().any(|l| l.contains("Payment validation")), "label mismatch: {:?}", labels);
        // Step 2 should be diamond shape
        let step2 = doc.nodes.iter().find(|n| n.display_label().contains("Payment validation")).unwrap();
        if let NodeKind::Shape { shape, .. } = step2.kind {
            assert_eq!(shape, NodeShape::Diamond);
        }
        // Step 4 should have green fill
        let step4 = doc.nodes.iter().find(|n| n.display_label().contains("Shipped")).unwrap();
        assert_ne!(step4.style.fill_color, NodeStyle::default().fill_color, "step4 fill should be green");
    }

    #[test]
    fn test_inline_edges_in_nodes_section() {
        let input = "## Nodes\n- [api] API Service -> db, cache\n- [db] Database\n- [cache] Redis Cache\n";
        let doc = parse_hrf(input).unwrap();
        assert_eq!(doc.nodes.len(), 3, "should have 3 nodes");
        assert_eq!(doc.edges.len(), 2, "should have 2 inline edges");
        let api_id = doc.nodes.iter().find(|n| n.display_label() == "API Service").unwrap().id;
        let db_id  = doc.nodes.iter().find(|n| n.display_label() == "Database").unwrap().id;
        assert!(doc.edges.iter().any(|e| e.source.node_id == api_id && e.target.node_id == db_id),
            "api→db edge missing");
    }

    #[test]
    fn test_palette_section_expands_colors() {
        let input = r#"
## Palette
brand   = #1e3a5f
danger  = red
success = #166534

## Nodes
- [a] Service {fill:brand}
- [b] Error State {fill:danger}
- [c] Healthy {fill:success}
"#;
        let doc = parse_hrf(input).unwrap();
        assert_eq!(doc.nodes.len(), 3);
        let a = doc.nodes.iter().find(|n| n.display_label() == "Service").unwrap();
        // brand = #1e3a5f → [30, 58, 95, 255]
        assert_eq!(a.style.fill_color, [30, 58, 95, 255], "brand color mismatch: {:?}", a.style.fill_color);
        // danger = red → should be non-default red color
        let b = doc.nodes.iter().find(|n| n.display_label() == "Error State").unwrap();
        assert_ne!(b.style.fill_color, NodeStyle::default().fill_color, "danger fill should be red");
        // success = #166534 → [22, 101, 52, 255]
        let c = doc.nodes.iter().find(|n| n.display_label() == "Healthy").unwrap();
        assert_eq!(c.style.fill_color, [22, 101, 52, 255], "success color mismatch: {:?}", c.style.fill_color);
    }

    #[test]
    fn test_inline_group_assignment_creates_frame() {
        let input = r#"
## Nodes
- [db] Database {circle} {group:backend}
- [api] API {group:backend}
- [ui] Frontend {group:frontend}
"#;
        let doc = parse_hrf(input).unwrap();
        // 3 real nodes + 2 frames (backend, frontend)
        assert_eq!(doc.nodes.len(), 5, "expected 3 nodes + 2 frames, got: {}", doc.nodes.len());
        let frames: Vec<_> = doc.nodes.iter().filter(|n| n.is_frame).collect();
        assert_eq!(frames.len(), 2, "expected 2 frame nodes");
        let labels: Vec<&str> = frames.iter()
            .map(|n| n.display_label())
            .collect();
        assert!(labels.iter().any(|&l| l.to_lowercase().contains("backend")),
            "expected a Backend frame, got: {:?}", labels);
        assert!(labels.iter().any(|&l| l.to_lowercase().contains("frontend")),
            "expected a Frontend frame, got: {:?}", labels);
    }

    #[test]
    fn test_style_section_expands_templates() {
        let input = r#"
## Style
primary = {fill:blue} {highlight}
danger   = {fill:red} {bold}
muted    = fill:teal opacity:0.7

## Nodes
- [a] API {primary}
- [b] Error {danger}
- [c] Helper {muted}
"#;
        let doc = parse_hrf(input).unwrap();
        assert_eq!(doc.nodes.len(), 3);
        let a = doc.nodes.iter().find(|n| n.display_label() == "API").unwrap();
        // {primary} = {fill:blue} {highlight}
        assert!(a.highlight, "API node should be highlighted via primary style");
        assert_eq!(a.style.fill_color, [137, 180, 250, 255], "primary fill should be blue");
        let b = doc.nodes.iter().find(|n| n.display_label() == "Error").unwrap();
        // {danger} = {fill:red} {bold}
        assert!(b.style.bold, "Error node should be bold via danger style");
        let c = doc.nodes.iter().find(|n| n.display_label() == "Helper").unwrap();
        // {muted} = fill:teal opacity:0.7 → wrapped as {fill:teal} {opacity:0.7}
        assert_eq!(c.style.fill_color, [148, 226, 213, 255], "muted fill should be teal");
    }

    #[test]
    fn test_multi_target_shorthand_parse_and_export() {
        let input = r#"
# Multi Target

## Nodes
- [api] API Service {connector}
- [pg] PostgreSQL {circle}
- [redis] Redis {circle}
- [worker] Worker

## Flow
api -> [pg, redis] {dashed}
api -> worker
"#;
        let doc = parse_hrf(input).unwrap();
        assert_eq!(doc.nodes.len(), 4);
        assert_eq!(doc.edges.len(), 3, "expected 3 edges (api->[pg,redis], api->worker), got {}", doc.edges.len());

        let api = doc.nodes.iter().find(|n| n.display_label() == "API Service").unwrap().id;
        let pg = doc.nodes.iter().find(|n| n.display_label() == "PostgreSQL").unwrap().id;
        let redis = doc.nodes.iter().find(|n| n.display_label() == "Redis").unwrap().id;
        let worker = doc.nodes.iter().find(|n| n.display_label() == "Worker").unwrap().id;

        let api_pg = doc.edges.iter().find(|e| e.source.node_id == api && e.target.node_id == pg);
        assert!(api_pg.is_some(), "expected api->pg edge");
        assert!(api_pg.unwrap().style.dashed, "api->pg should be dashed");

        let api_redis = doc.edges.iter().find(|e| e.source.node_id == api && e.target.node_id == redis);
        assert!(api_redis.is_some(), "expected api->redis edge");
        assert!(api_redis.unwrap().style.dashed, "api->redis should be dashed");

        assert!(doc.edges.iter().any(|e| e.source.node_id == api && e.target.node_id == worker),
            "expected api->worker edge");
        assert!(!doc.edges.iter().find(|e| e.source.node_id == api && e.target.node_id == worker)
            .unwrap().style.dashed, "api->worker should NOT be dashed");

        // Export should collapse the two dashed edges back to multi-target bracket syntax
        let exported = export_hrf(&doc, "Multi Target");
        // Nodes have explicit IDs [pg] and [redis], so those are preserved in export
        assert!(exported.contains("[pg, redis]") || exported.contains("[redis, pg]"),
            "expected multi-target in export:\n{}", exported);
    }

    #[test]
    fn test_multi_source_shorthand_parse() {
        let input = r#"
# Multi Source

## Nodes
- [web] Web App
- [mobile] Mobile App
- [api] API Service

## Flow
[web, mobile] -> api {thick}
"#;
        let doc = parse_hrf(input).unwrap();
        assert_eq!(doc.nodes.len(), 3);
        // [web, mobile] -> api expands to 2 edges: web->api and mobile->api
        assert_eq!(doc.edges.len(), 2, "expected 2 edges, got {}", doc.edges.len());

        let web    = doc.nodes.iter().find(|n| n.display_label() == "Web App").unwrap().id;
        let mobile = doc.nodes.iter().find(|n| n.display_label() == "Mobile App").unwrap().id;
        let api    = doc.nodes.iter().find(|n| n.display_label() == "API Service").unwrap().id;

        assert!(doc.edges.iter().any(|e| e.source.node_id == web && e.target.node_id == api),
            "expected web->api edge");
        assert!(doc.edges.iter().any(|e| e.source.node_id == mobile && e.target.node_id == api),
            "expected mobile->api edge");

        // Both edges should be thick
        for edge in &doc.edges {
            assert!(edge.style.width > 4.0, "expected thick edge, width={}", edge.style.width);
        }
    }

    #[test]
    fn test_progress_tag_parse_and_export() {
        let input = r#"
# Progress Test

## Nodes
- [task_a] Task A {progress:75}
- [task_b] Task B {progress:100%}
- [task_c] Task C {progress:0.5}

## Flow
task_a -> task_b -> task_c
"#;
        let doc = parse_hrf(input).expect("should parse");
        let a = doc.nodes.iter().find(|n| n.display_label() == "Task A").unwrap();
        let b = doc.nodes.iter().find(|n| n.display_label() == "Task B").unwrap();
        let c = doc.nodes.iter().find(|n| n.display_label() == "Task C").unwrap();
        assert!((a.progress - 0.75).abs() < 0.01, "task_a progress should be 0.75, got {}", a.progress);
        assert!((b.progress - 1.00).abs() < 0.01, "task_b progress should be 1.0, got {}", b.progress);
        assert!((c.progress - 0.5).abs()  < 0.01, "task_c progress should be 0.5, got {}", c.progress);

        // Export round-trip
        let exported = export_hrf(&doc, "Progress Test");
        assert!(exported.contains("{progress:75}"), "should export progress:75, got:\n{}", exported);
        assert!(exported.contains("{progress:100}"), "should export progress:100, got:\n{}", exported);
        assert!(exported.contains("{progress:50}"), "should export progress:50, got:\n{}", exported);
    }

    #[test]
    fn test_inline_comments_stripped() {
        let input = r#"
# Inline Comment Test

## Nodes
- [a] Alpha  // this is a node comment
- [b] Beta   // another comment

## Flow
a -> b  // external traffic
b -> a  // return path; https://example.com is not a comment stop
"#;
        let doc = parse_hrf(input).expect("should parse with inline comments");
        assert_eq!(doc.nodes.len(), 2, "should have 2 nodes");
        assert_eq!(doc.edges.len(), 2, "should have 2 edges");
        // Node labels should not contain the comment
        let a = doc.nodes.iter().find(|n| n.display_label() == "Alpha").expect("Alpha node");
        let b = doc.nodes.iter().find(|n| n.display_label() == "Beta").expect("Beta node");
        let ab = doc.edges.iter().any(|e| e.source.node_id == a.id && e.target.node_id == b.id);
        let ba = doc.edges.iter().any(|e| e.source.node_id == b.id && e.target.node_id == a.id);
        assert!(ab, "expected a->b edge");
        assert!(ba, "expected b->a edge (URL in inline comment not swallowed)");
    }

    #[test]
    fn test_auto_z_assigns_z_offsets() {
        let input = r#"
# Auto-Z Test

## Config
auto-z = true
view   = 3d

## Nodes
- [client] Client {user}
- [api]    API Service
- [db]     Database {database}

## Flow
client -> api
api    -> db
"#;
        let doc = parse_hrf(input).expect("should parse auto-z spec");
        assert!(doc.import_hints.auto_z, "auto_z hint should be set");
        let client = doc.nodes.iter().find(|n| n.display_label() == "Client").unwrap();
        let api    = doc.nodes.iter().find(|n| n.display_label() == "API Service").unwrap();
        let db     = doc.nodes.iter().find(|n| n.display_label() == "Database").unwrap();
        // client has no incoming edges → layer 0 → z=0
        assert_eq!(client.z_offset, 0.0, "client should be at z=0");
        // api has 1 predecessor → layer 1 → z=120
        assert_eq!(api.z_offset, 120.0, "api should be at z=120");
        // db has 2 predecessors → layer 2 → z=240
        assert_eq!(db.z_offset, 240.0, "db should be at z=240");
    }

    #[test]
    fn test_unicode_arrows_in_flow() {
        // → ⇒ ⟶ should all create forward edges like -->
        // ← ⟵ should create reverse edges like <--
        // ↔ ⇔ ⟷ should create bidirectional edges like <->
        let input = r#"
# Unicode Arrow Test

## Nodes
- [a] Alpha
- [b] Beta
- [c] Gamma
- [d] Delta
- [e] Epsilon

## Flow
a → b
c ⇒ d
e ⟷ a
"#;
        let doc = parse_hrf(input).expect("should parse unicode arrows");
        assert_eq!(doc.nodes.len(), 5, "5 nodes");
        // a→b  and  c⇒d should each produce one directed edge
        let a = doc.nodes.iter().find(|n| n.display_label() == "Alpha").unwrap();
        let b = doc.nodes.iter().find(|n| n.display_label() == "Beta").unwrap();
        let c = doc.nodes.iter().find(|n| n.display_label() == "Gamma").unwrap();
        let d = doc.nodes.iter().find(|n| n.display_label() == "Delta").unwrap();
        let e = doc.nodes.iter().find(|n| n.display_label() == "Epsilon").unwrap();
        let ab = doc.edges.iter().any(|e| e.source.node_id == a.id && e.target.node_id == b.id);
        let cd = doc.edges.iter().any(|e| e.source.node_id == c.id && e.target.node_id == d.id);
        assert!(ab, "a → b should create a→b edge");
        assert!(cd, "c ⇒ d should create c→d edge");
        // e⟷a should create two edges (bidirectional)
        let ea = doc.edges.iter().any(|edge| edge.source.node_id == e.id && edge.target.node_id == a.id);
        let ae = doc.edges.iter().any(|edge| edge.source.node_id == a.id && edge.target.node_id == e.id);
        assert!(ea || ae, "e ⟷ a should create at least one e↔a edge");
    }

    #[test]
    fn test_colon_label_on_edge() {
        let input = r#"
# Colon Label Test

## Nodes
- [a] Service A
- [b] Service B
- [c] Service C

## Flow
a -> b: authenticates user
b -> c: stores session {dashed}
"#;
        let doc = parse_hrf(input).expect("colon label parse");
        let a = doc.nodes.iter().find(|n| n.display_label() == "Service A").unwrap();
        let b = doc.nodes.iter().find(|n| n.display_label() == "Service B").unwrap();
        let c = doc.nodes.iter().find(|n| n.display_label() == "Service C").unwrap();
        let ab = doc.edges.iter().find(|e| e.source.node_id == a.id && e.target.node_id == b.id)
            .expect("a->b edge");
        let bc = doc.edges.iter().find(|e| e.source.node_id == b.id && e.target.node_id == c.id)
            .expect("b->c edge");
        assert_eq!(ab.label, "authenticates user", "colon label on plain edge");
        assert_eq!(bc.label, "stores session", "colon label with {{dashed}} tag");
        assert!(bc.style.dashed, "dashed tag should apply even with colon label");
    }

    #[test]
    fn test_pipe_label_on_edge() {
        // Mermaid-style: a ->|label| b
        let input = r#"
# Pipe Label Test

## Nodes
- [x] Node X
- [y] Node Y
- [z] Node Z

## Flow
x ->|sends request| y
y ->|returns data| z {dashed}
"#;
        let doc = parse_hrf(input).expect("pipe label parse");
        let x = doc.nodes.iter().find(|n| n.display_label() == "Node X").unwrap();
        let y = doc.nodes.iter().find(|n| n.display_label() == "Node Y").unwrap();
        let z_node = doc.nodes.iter().find(|n| n.display_label() == "Node Z").unwrap();
        let xy = doc.edges.iter().find(|e| e.source.node_id == x.id && e.target.node_id == y.id)
            .expect("x->y edge");
        let yz = doc.edges.iter().find(|e| e.source.node_id == y.id && e.target.node_id == z_node.id)
            .expect("y->z edge");
        assert_eq!(xy.label, "sends request", "pipe label on plain edge");
        assert_eq!(yz.label, "returns data", "pipe label with {{dashed}} tag");
        assert!(yz.style.dashed, "dashed tag applies with pipe label");
    }

    #[test]
    fn test_grid_section_layout() {
        let input = r#"
# Grid Test

## Grid cols=3
- [a] Alpha
- [b] Beta
- [c] Gamma
- [d] Delta
- [e] Epsilon
"#;
        let doc = parse_hrf(input).expect("grid parse");
        assert_eq!(doc.nodes.len(), 5);
        let a = doc.nodes.iter().find(|n| n.display_label() == "Alpha").unwrap();
        let b = doc.nodes.iter().find(|n| n.display_label() == "Beta").unwrap();
        let d = doc.nodes.iter().find(|n| n.display_label() == "Delta").unwrap();
        // a is at col=0, row=0; b at col=1, row=0; d at col=0, row=1
        assert!(a.position[0] < b.position[0], "a should be left of b (same row)");
        assert_eq!(a.position[1], b.position[1], "a and b should be in same row");
        assert_eq!(a.position[0], d.position[0], "a and d should be in same column");
        assert!(a.position[1] < d.position[1], "d should be below a");
    }

    #[test]
    fn test_named_tier_z_offsets() {
        // {layer:db/api/frontend/edge/infra} should map to canonical z values
        let input = r#"
# Named Tier Test

## Nodes
- [db]       Database    {database} {layer:db}
- [api]      API         {service}  {layer:api}
- [ui]       Frontend    {user}     {layer:frontend}
- [gw]       Gateway     {service}  {layer:edge}
- [host]     Host        {server}   {layer:infra}

## Flow
db -> api
api -> ui
gw -> api
"#;
        let doc = parse_hrf(input).expect("named tier parse");
        let find = |label: &str| doc.nodes.iter().find(|n| n.display_label() == label)
            .expect("node not found").z_offset;
        assert_eq!(find("Database"),   0.0,   "db → z=0");
        assert_eq!(find("API"),        120.0, "api → z=120");
        assert_eq!(find("Frontend"),   240.0, "frontend → z=240");
        assert_eq!(find("Gateway"),    360.0, "edge → z=360");
        assert_eq!(find("Host"),       480.0, "infra → z=480");
    }

    #[test]
    fn test_layers_section_defines_custom_z() {
        // ## Layers section allows arbitrary name → z mapping
        let input = r#"
# Layers Section Test

## Layers
frontend = 240
middletier = 120
persistence = 0
edge = 360

## Nodes
- [a] Browser    {layer:frontend}
- [b] Service    {layer:middletier}
- [c] Database   {layer:persistence}
- [d] Gateway    {layer:edge}

## Flow
d -> a
a -> b
b -> c
"#;
        let doc = parse_hrf(input).expect("layers section parse");
        let find_z = |label: &str| doc.nodes.iter().find(|n| n.display_label() == label)
            .unwrap().z_offset;
        assert_eq!(find_z("Browser"),   240.0, "frontend → 240");
        assert_eq!(find_z("Service"),   120.0, "middletier → 120");
        assert_eq!(find_z("Database"),    0.0, "persistence → 0");
        assert_eq!(find_z("Gateway"),   360.0, "edge → 360");
    }

    #[test]
    fn test_feature_showcase_spec_parses() {
        // Smoke test: the feature_showcase.spec bundled example should parse without errors
        let spec = r#"
# Feature Showcase

## Config
view    = 3d
camera  = iso
bg      = dots
auto-z  = false

## Style
frontend = {fill:sky}    {layer:frontend}
backend  = {fill:blue}   {layer:api}
storage  = {fill:purple} {layer:db}

## Nodes
- [a] Alpha  {frontend}
- [b] Beta   {backend}
- [c] Gamma  {storage}

## Flow
a →|request| b
b -> c: stores data {dashed}

## Grid cols=2
- [g1] Item One   {fill:blue}
- [g2] Item Two   {fill:green}
- [g3] Item Three {fill:red}
- [g4] Item Four  {fill:yellow}
"#;
        let doc = parse_hrf(spec).expect("showcase parse");
        assert_eq!(doc.nodes.len(), 7, "3 flow nodes + 4 grid nodes");
        assert!(!doc.edges.is_empty(), "should have edges");
        // a→b edge should have pipe label
        let a = doc.nodes.iter().find(|n| n.display_label() == "Alpha").unwrap();
        let b = doc.nodes.iter().find(|n| n.display_label() == "Beta").unwrap();
        let ab = doc.edges.iter().find(|e| e.source.node_id == a.id && e.target.node_id == b.id)
            .expect("a→b edge");
        assert_eq!(ab.label, "request");
        // b→c should have colon label
        let c = doc.nodes.iter().find(|n| n.display_label() == "Gamma").unwrap();
        let bc = doc.edges.iter().find(|e| e.source.node_id == b.id && e.target.node_id == c.id)
            .expect("b→c edge");
        assert_eq!(bc.label, "stores data");
        assert!(bc.style.dashed);
        // z offsets from named tiers
        assert_eq!(a.z_offset, 240.0, "frontend tier");
        assert_eq!(b.z_offset, 120.0, "backend tier");
        assert_eq!(c.z_offset, 0.0,   "storage tier");
    }

    #[test]
    fn test_style_shorthand_arrows() {
        let input = r#"
# Style Arrow Test

## Nodes
- [a] Alpha
- [b] Beta
- [c] Gamma
- [d] Delta

## Flow
a -.-> b
c ==> d
"#;
        let doc = parse_hrf(input).expect("should parse");
        let a = doc.nodes.iter().find(|n| n.display_label() == "Alpha").unwrap();
        let b = doc.nodes.iter().find(|n| n.display_label() == "Beta").unwrap();
        let c = doc.nodes.iter().find(|n| n.display_label() == "Gamma").unwrap();
        let d = doc.nodes.iter().find(|n| n.display_label() == "Delta").unwrap();

        let ab = doc.edges.iter().find(|e| e.source.node_id == a.id && e.target.node_id == b.id)
            .expect("a->b edge via -.-> should exist");
        assert!(ab.style.dashed, "-.-> should create dashed edge");

        let cd = doc.edges.iter().find(|e| e.source.node_id == c.id && e.target.node_id == d.id)
            .expect("c->d edge via ==> should exist");
        assert!(cd.style.width > 4.0, "==> should create thick edge, width={}", cd.style.width);
    }

    #[test]
    fn test_multiline_label_escape() {
        let input = r#"
# Multiline Label Test

## Nodes
- [a] First Line\nSecond Line
- [b] Single Line
"#;
        let doc = parse_hrf(input).expect("should parse");
        let a = doc.nodes.iter().find(|n| n.id != doc.nodes[1].id && n.display_label().contains('\n')).unwrap();
        assert!(a.display_label().contains('\n'), "label should have actual newline");
        assert_eq!(a.display_label(), "First Line\nSecond Line");

        // Export and verify escape roundtrip
        let exported = export_hrf(&doc, "Multiline Label Test");
        assert!(exported.contains("First Line\\nSecond Line"),
            "exported spec should escape newlines back to \\n: {}", exported);

        // Re-import the exported spec and verify label is preserved
        let doc2 = parse_hrf(&exported).expect("re-import should work");
        let a2 = doc2.nodes.iter().find(|n| n.display_label().contains('\n')).unwrap();
        assert_eq!(a2.display_label(), "First Line\nSecond Line",
            "label should survive roundtrip");
    }

    #[test]
    fn test_tier_color_tag_and_config() {
        let input = r#"
# Tier Color Test

## Config
auto-tier-color = true
view = 3d

## Nodes
- [db_node] Database {layer:db}
- [api_node] REST API {layer:api}
- [ui_node] UI App {layer:frontend}
- [custom] Custom {layer:api} {fill:red}
"#;
        let doc = parse_hrf(input).expect("should parse");
        let find = |label: &str| doc.nodes.iter().find(|n| n.display_label() == label).unwrap();

        // auto-tier-color should tint nodes with default fill
        let db_node = find("Database");
        assert_eq!(db_node.z_offset, 0.0, "db tier → z=0");
        // db is z=0, auto-tier-color only tints z!=0, so db stays default
        // (z=0 nodes are the base layer and auto-tier-color skips them)

        let api_node = find("REST API");
        assert_eq!(api_node.z_offset, 120.0, "api tier → z=120");
        let api_fill = api_node.style.fill_color;
        assert_ne!(api_fill, [49, 50, 68, 255], "api should be tinted, not default fill");

        let ui_node = find("UI App");
        assert_eq!(ui_node.z_offset, 240.0, "frontend tier → z=240");
        let ui_fill = ui_node.style.fill_color;
        assert_ne!(ui_fill, [49, 50, 68, 255], "frontend should be tinted, not default fill");
        // UI tier should be different from API tier
        assert_ne!(ui_fill, api_fill, "different tiers should have different tints");

        // explicit {fill:red} should override tier-color
        let custom = find("Custom");
        let red_fill = [243_u8, 139, 168, 255];
        assert_eq!(custom.style.fill_color, red_fill, "explicit fill should override tier-color");

        // Import hint should be set
        assert!(doc.import_hints.auto_tier_color, "auto_tier_color hint should be set");
        assert_eq!(doc.import_hints.view_3d, Some(true), "view=3d hint should be set");
    }

    #[test]
    fn test_status_shorthand_tags() {
        let input = r#"
# Status Tags Test

## Nodes
- [a] Alpha {done}
- [b] Beta {wip}
- [c] Gamma {review}
- [d] Delta {blocked}
- [e] Epsilon {todo}
- [f] Zeta {glow}
- [g] Eta {in-progress}
"#;
        let doc = parse_hrf(input).expect("should parse");
        let find = |label: &str| doc.nodes.iter().find(|n| n.display_label() == label).unwrap();

        let a = find("Alpha");
        assert!((a.progress - 1.0).abs() < 0.01, "done -> progress=1.0, got {}", a.progress);
        assert_eq!(a.tag, Some(crate::model::NodeTag::Ok), "done -> Ok badge");

        let b = find("Beta");
        assert!((b.progress - 0.5).abs() < 0.01, "wip -> progress=0.5, got {}", b.progress);
        assert_eq!(b.tag, Some(crate::model::NodeTag::Info), "wip -> Info badge");

        let c = find("Gamma");
        assert!((c.progress - 0.75).abs() < 0.01, "review -> progress=0.75, got {}", c.progress);
        assert_eq!(c.tag, Some(crate::model::NodeTag::Warning), "review -> Warning badge");

        let d = find("Delta");
        assert_eq!(d.tag, Some(crate::model::NodeTag::Critical), "blocked -> Critical badge");

        let e = find("Epsilon");
        assert_eq!(e.tag, Some(crate::model::NodeTag::Warning), "todo -> Warning badge");
        assert!(e.progress < 0.01, "todo -> no progress, got {}", e.progress);

        let f = find("Zeta");
        assert!(f.style.glow, "glow -> node.style.glow should be true");

        let g = find("Eta");
        assert_eq!(g.tag, Some(crate::model::NodeTag::Info), "in-progress -> Info badge");
        assert!((g.progress - 0.5).abs() < 0.01, "in-progress -> progress=0.5, got {}", g.progress);
    }

    #[test]
    fn test_natural_language_flow_references() {
        // Nodes can be referenced in the Flow section by their display label
        // (slugified and matched as fallback when the direct id lookup fails).
        let input = r#"
# Label Reference Test

## Nodes
- [auth] Authentication Service
- [db] PostgreSQL Database
- [cache] Redis Cache

## Flow
Authentication Service --> PostgreSQL Database
authentication_service --> redis_cache
auth --> db
"#;
        let doc = parse_hrf(input).expect("should parse without errors");
        // There should be 3 edges (one per flow line)
        assert_eq!(doc.edges.len(), 3, "expected 3 edges from label-based lookup, got {}", doc.edges.len());

        let auth_id = doc.nodes.iter().find(|n| n.display_label() == "Authentication Service").unwrap().id;
        let db_id   = doc.nodes.iter().find(|n| n.display_label() == "PostgreSQL Database").unwrap().id;
        let cache_id = doc.nodes.iter().find(|n| n.display_label() == "Redis Cache").unwrap().id;

        // First edge: label ref -> label ref
        assert_eq!(doc.edges[0].source.node_id, auth_id);
        assert_eq!(doc.edges[0].target.node_id, db_id);
        // Second edge: slug ref -> slug ref
        assert_eq!(doc.edges[1].source.node_id, auth_id);
        assert_eq!(doc.edges[1].target.node_id, cache_id);
        // Third edge: explicit id -> explicit id (still works)
        assert_eq!(doc.edges[2].source.node_id, auth_id);
        assert_eq!(doc.edges[2].target.node_id, db_id);
    }

    #[test]
    fn test_quoted_label_flow_references() {
        // "Display Name" --> "Other Node" — both source and target as quoted label refs
        // The existing `auth "edge label" --> db` syntax must still work.
        let input = r#"
# Quoted Label Flow Test

## Nodes
- [auth] Auth Service
- [db] Main Database

## Flow
"Auth Service" --> "Main Database"
auth "calls" --> db
"#;
        let doc = parse_hrf(input).expect("should parse");
        assert_eq!(doc.edges.len(), 2);
        let auth_id = doc.nodes.iter().find(|n| n.display_label() == "Auth Service").unwrap().id;
        let db_id   = doc.nodes.iter().find(|n| n.display_label() == "Main Database").unwrap().id;

        // First edge: quoted label refs, no edge label
        assert_eq!(doc.edges[0].source.node_id, auth_id);
        assert_eq!(doc.edges[0].target.node_id, db_id);
        assert!(doc.edges[0].label.is_empty(), "no edge label on quoted node ref");

        // Second edge: explicit id with quoted edge label
        assert_eq!(doc.edges[1].source.node_id, auth_id);
        assert_eq!(doc.edges[1].target.node_id, db_id);
        assert_eq!(doc.edges[1].label, "calls", "edge label from quoted syntax");
    }

    #[test]
    fn test_opacity_shorthand_tags() {
        let input = r#"
# Opacity Test

## Nodes
- [a] Active
- [b] Dimmed {dim}
- [c] Ghost {ghost}
- [d] Muted {muted}
- [e] Hidden {hidden}
- [f] Custom {opacity:50}
"#;
        let doc = parse_hrf(input).expect("should parse");
        let find = |label: &str| doc.nodes.iter().find(|n| n.display_label() == label).unwrap();

        assert!((find("Active").style.opacity - 1.0).abs() < 0.01, "default opacity=1");
        assert!((find("Dimmed").style.opacity - 0.35).abs() < 0.01, "dim=0.35");
        assert!((find("Ghost").style.opacity - 0.18).abs() < 0.01, "ghost=0.18");
        assert!((find("Muted").style.opacity - 0.6).abs() < 0.01, "muted=0.6");
        assert!(find("Hidden").style.opacity < 0.01, "hidden=0.0");
        assert!((find("Custom").style.opacity - 0.5).abs() < 0.01, "opacity:50 = 0.5");

        // Export roundtrip — friendly names should be used for known values
        let exported = export_hrf(&doc, "Opacity Test");
        assert!(exported.contains("{dim}"),    "dim should export as {{dim}}");
        assert!(exported.contains("{ghost}"),  "ghost should export as {{ghost}}");
        assert!(exported.contains("{muted}"),  "muted should export as {{muted}}");
        assert!(exported.contains("{hidden}"), "hidden should export as {{hidden}}");
        assert!(exported.contains("{opacity:50}"), "50% should export as {{opacity:50}}");
    }

    #[test]
    fn test_timeline_period_lane_parse() {
        let input = r#"
# Product Roadmap

## Config
timeline = true
timeline-dir = LR

## Period 1: Q1 — Foundation
- [mvp] MVP Launch {done} {lane:Product}
- [auth] Auth System {wip} {lane:Backend}

## Period 2: Q2 — Growth
- [api] Public API {lane:Backend}
- [onboard] Onboarding {lane:Product}

## Lane 1: Product
## Lane 2: Backend

## Flow
auth --> api: builds on
"#;
        let doc = parse_hrf(input).expect("should parse");

        // timeline_mode should be set
        assert!(doc.timeline_mode, "timeline_mode should be true");

        // periods should be detected
        assert_eq!(doc.timeline_periods.len(), 2);
        assert!(doc.timeline_periods[0].contains("Q1"));
        assert!(doc.timeline_periods[1].contains("Q2"));

        // lanes should be detected
        assert!(doc.timeline_lanes.contains(&"Product".to_string()));
        assert!(doc.timeline_lanes.contains(&"Backend".to_string()));

        // nodes should have period and lane assigned
        let mvp = doc.nodes.iter().find(|n| n.display_label() == "MVP Launch").unwrap();
        assert!(mvp.timeline_period.as_deref().map_or(false, |p| p.contains("Q1")));
        assert_eq!(mvp.timeline_lane.as_deref(), Some("Product"));

        let api = doc.nodes.iter().find(|n| n.display_label() == "Public API").unwrap();
        assert!(api.timeline_period.as_deref().map_or(false, |p| p.contains("Q2")));
        assert_eq!(api.timeline_lane.as_deref(), Some("Backend"));

        // edges should be parsed
        assert_eq!(doc.edges.len(), 1);

        // export roundtrip should include Period sections
        let exported = export_hrf(&doc, "Product Roadmap");
        assert!(exported.contains("timeline = true"), "should export timeline = true");
        assert!(exported.contains("## Period 1:"), "should export period sections");
        assert!(exported.contains("## Period 2:"), "should export period 2");
    }

    #[test]
    fn test_section_name_preserved_in_export() {
        let input = r#"
# Hypothesis Map

## Hypotheses
- [h1] Users churn due to bad onboarding {hypothesis}

## Evidence
- [e1] 68% drop-off at step 3 {evidence} {done}

## Flow
e1 --> h1: supports
"#;
        let doc = parse_hrf(input).expect("should parse");

        // Nodes should have section_name set
        let h1 = doc.nodes.iter().find(|n| n.display_label() == "Users churn due to bad onboarding")
            .expect("h1 node");
        assert_eq!(h1.section_name, "Hypotheses");

        let e1 = doc.nodes.iter().find(|n| n.display_label() == "68% drop-off at step 3")
            .expect("e1 node");
        assert_eq!(e1.section_name, "Evidence");

        // Export should use section names as section headers
        let exported = export_hrf(&doc, "Hypothesis Map");
        assert!(exported.contains("## Hypotheses"), "export should have Hypotheses section");
        assert!(exported.contains("## Evidence"), "export should have Evidence section");
    }

    #[test]
    fn test_inline_section_tag() {
        // {section:Name} should assign node to that section without a header
        let input = r#"
# Support Board

## Nodes
- [t1] Bug Report {section:Intake} {p1}
- [t2] Feature Request {col:Backlog} {p3}
- [t3] Crash on login {stage:In Progress} {p2}
"#;
        let doc = parse_hrf(input).expect("should parse");
        assert_eq!(doc.nodes.len(), 3);

        let t1 = doc.nodes.iter().find(|n| n.display_label() == "Bug Report").expect("t1");
        assert_eq!(t1.section_name, "Intake", "section: tag should set section_name");

        let t2 = doc.nodes.iter().find(|n| n.display_label() == "Feature Request").expect("t2");
        assert_eq!(t2.section_name, "Backlog", "col: tag should set section_name");

        let t3 = doc.nodes.iter().find(|n| n.display_label() == "Crash on login").expect("t3");
        assert_eq!(t3.section_name, "In Progress", "stage: tag should set section_name");
    }

    #[test]
    fn test_metric_decorator() {
        let spec = "## Nodes\n- [a] Alpha {metric:$2.4M ARR}\n";
        let doc = parse_hrf(spec).unwrap();
        assert_eq!(doc.nodes[0].metric, Some("$2.4M ARR".to_string()));
    }

    #[test]
    fn test_owner_decorator() {
        let spec = "## Nodes\n- [a] Alpha {owner:@alice}\n";
        let doc = parse_hrf(spec).unwrap();
        assert_eq!(doc.nodes[0].owner, Some("@alice".to_string()));
    }

    #[test]
    fn test_dep_generates_edge() {
        let spec = "## Nodes\n- [a] Alpha\n- [b] Beta {dep:a}\n";
        let doc = parse_hrf(spec).unwrap();
        assert_eq!(doc.edges.len(), 1);
        // edge goes FROM b TO a (dep means "depends on")
        let edge = &doc.edges[0];
        assert_eq!(doc.nodes.iter().find(|n| n.id == edge.source.node_id).unwrap().hrf_id, "b");
        assert_eq!(doc.nodes.iter().find(|n| n.id == edge.target.node_id).unwrap().hrf_id, "a");
    }

    #[test]
    fn test_shape_person() {
        let spec = "## Nodes\n- [alice] Alice {shape:person}\n";
        let doc = parse_hrf(spec).unwrap();
        let crate::model::NodeKind::Shape { shape, .. } = &doc.nodes[0].kind else { panic!("wrong kind") };
        assert_eq!(*shape, crate::model::NodeShape::Person);
    }

    #[test]
    fn test_shape_screen() {
        let spec = "## Nodes\n- [ls] Login Screen {shape:screen}\n";
        let doc = parse_hrf(spec).unwrap();
        let crate::model::NodeKind::Shape { shape, .. } = &doc.nodes[0].kind else { panic!("wrong kind") };
        assert_eq!(*shape, crate::model::NodeShape::Screen);
    }

    #[test]
    fn test_business_tag_revenue() {
        let spec = "## Nodes\n- [pp] Pro Plan {revenue}\n";
        let doc = parse_hrf(spec).unwrap();
        // {revenue} applies green fill preset — green channel (index 1) should be high
        assert!(doc.nodes[0].style.fill_color[1] > 150, "revenue should set green fill");
    }

    #[test]
    fn test_milestone_tag() {
        let spec = "## Nodes\n- [launch] Launch {milestone}\n";
        let doc = parse_hrf(spec).unwrap();
        let crate::model::NodeKind::Shape { shape, .. } = &doc.nodes[0].kind else { panic!("wrong kind") };
        assert_eq!(*shape, crate::model::NodeShape::Diamond);
    }
}
