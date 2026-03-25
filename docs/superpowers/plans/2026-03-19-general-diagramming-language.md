# General Diagramming Language Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend light-figma's HRF language and layout engine to support all user roles (engineers, PMs, businessmen, AI agents) through new diagram types, shape vocabulary, template gallery, and a headless CLI.

**Architecture:** New HRF syntax (`## Timeline`, `## Swimlane:`, `## OrgTree`, `## Kanban:`) maps to new layout functions in `layout.rs`. New `Node` fields (`metric`, `owner`) support visual badges. New `NodeShape` variants (Person, Screen, Cylinder, Cloud, Document, Channel, Segment) add business/UX shapes. A `template_gallery.rs` panel provides role-specific starting points. A restructured `main.rs` supports headless CLI subcommands using the existing `export_svg()` function (already takes `&FlowchartDocument` — no eframe needed headless).

**Tech Stack:** Rust, egui/eframe, existing `hrf.rs` parser, `layout.rs`, `render.rs`, `export.rs`. New deps: `clap` (CLI), `petgraph` (orgtree layout), `tiny_http` (embed server), `notify` (watch mode).

**Codebase notes before starting:**
- `NodeShape` already has: Rectangle, RoundedRect, Diamond, Circle, Parallelogram, Connector, Hexagon, Triangle, Callout
- `{star}` already sets `highlight = true` — not a shape
- `{person}`, `{cloud}` already handled by `tag_to_preset()` as semantic presets (Circle/Hexagon + color)
- `Node` already has `timeline_period: Option<String>` and `timeline_lane: Option<String>`
- `FlowchartDocument` already has `timeline_mode: bool`, `timeline_periods`, `timeline_lanes`
- `## Period N: Name` and `## Lane N: Name` sections already parse timeline/swimlane grids
- `export_svg(&FlowchartDocument, &Path)` already works without eframe

---

## Phase 1: HRF Language Extensions

### Task 1.1: Add `metric` and `owner` fields to `Node`

**Files:**
- Modify: `src/model.rs`

- [ ] **Step 1: Add fields to Node struct**

In `src/model.rs`, find the `Node` struct (around line 288). Add after `priority`:
```rust
#[serde(default)]
pub metric: Option<String>,
#[serde(default)]
pub owner: Option<String>,
```

- [ ] **Step 2: Add NodeShape variants for new shapes**

In `src/model.rs`, extend the `NodeShape` enum (line 26):
```rust
pub enum NodeShape {
    Rectangle,
    RoundedRect,
    Diamond,
    Circle,
    Parallelogram,
    Connector,
    Hexagon,
    Triangle,
    Callout,
    // New shapes for multi-role diagrams
    Person,       // circle head + body silhouette
    Screen,       // rounded rect + top chrome bar
    Cylinder,     // database drum with top ellipse
    Cloud,        // cloud blob outline (replaces hexagon preset)
    Document,     // rectangle with folded corner
    Channel,      // funnel shape
    Segment,      // person-group shape
}
```

- [ ] **Step 3: Fix Node::new() default size match for new shapes**

In the `Node::new()` impl (around line 360), add match arms for each new shape:
```rust
NodeShape::Person    => [70.0,  90.0],
NodeShape::Screen    => [140.0, 100.0],
NodeShape::Cylinder  => [100.0, 80.0],
NodeShape::Cloud     => [140.0, 80.0],
NodeShape::Document  => [130.0, 90.0],
NodeShape::Channel   => [90.0,  80.0],
NodeShape::Segment   => [110.0, 80.0],
```

- [ ] **Step 4: Add `LayoutMode` enum to model.rs**

After the `NodeShape` enum, add:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LayoutMode {
    #[default]
    Hierarchical,
    Timeline,
    OrgTree,
    Kanban,
    Swimlane,
}
```

- [ ] **Step 5: Add `layout_mode` and `kanban_columns` to FlowchartDocument**

In `FlowchartDocument` struct (around line 751), add:
```rust
#[serde(default)]
pub layout_mode: LayoutMode,
#[serde(default)]
pub kanban_columns: Vec<String>,
```

- [ ] **Step 6: Build and check for compile errors**

```bash
cargo build 2>&1 | head -40
```
Expected: errors about non-exhaustive match arms on `NodeShape`. Fix each one by adding `_ => {}` or appropriate handling.

- [ ] **Step 7: Run existing tests**

```bash
cargo test 2>&1 | tail -20
```
Expected: all existing ~64 tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/model.rs
git commit -m "feat: add metric/owner fields, new NodeShape variants, LayoutMode enum"
```

---

### Task 1.2: HRF parser — new decorators `{metric:}`, `{owner:}`, `{dep:}`

**Files:**
- Modify: `src/specgraph/hrf.rs`

Context: Tag parsing happens in `apply_tag_to_node()` (around line 2569). The function signature is approximately:
```rust
fn apply_tag_to_node(tag: &str, node: &mut Node, doc: &mut FlowchartDocument, ...) -> bool
```

- [ ] **Step 1: Write tests first**

In `src/specgraph/hrf.rs`, find the `#[cfg(test)]` mod and add:
```rust
#[test]
fn test_metric_decorator() {
    let spec = "## Nodes\n- Alpha {metric:$2.4M ARR}\n";
    let doc = parse_hrf(spec).unwrap();
    assert_eq!(doc.nodes[0].metric, Some("$2.4M ARR".to_string()));
}

#[test]
fn test_owner_decorator() {
    let spec = "## Nodes\n- Alpha {owner:@alice}\n";
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
    assert_eq!(doc.nodes.iter().find(|n| n.id == edge.source).unwrap().hrf_id, "b");
    assert_eq!(doc.nodes.iter().find(|n| n.id == edge.target).unwrap().hrf_id, "a");
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test test_metric_decorator test_owner_decorator test_dep_generates_edge 2>&1 | tail -20
```
Expected: FAIL (fields not parsed yet).

- [ ] **Step 3: Parse `{metric:}` and `{owner:}` in `apply_tag_to_node()`**

Find the block starting `} else if tag.starts_with("shape:") ||` and add before it:
```rust
} else if let Some(val) = tag.strip_prefix("metric:") {
    node.metric = Some(val.to_string());
} else if let Some(val) = tag.strip_prefix("owner:") {
    node.owner = Some(val.to_string());
} else if let Some(val) = tag.strip_prefix("dep:") {
    // Stored temporarily; resolved in post-pass after all nodes are parsed.
    // Reuse hrf_id field on Edge to store the dep target for resolution.
    // Simplest: store in a side-channel Vec<(NodeId, String)> passed through parsing.
    // See Task 1.3 for {dep:} post-pass.
    node_deps.push((node.id, val.to_string()));
```

Note: `node_deps` is a `Vec<(NodeId, String)>` threaded through the parse call. Add it to the parse context and resolve after the main loop.

- [ ] **Step 4: Implement `{dep:}` post-pass in `parse_hrf()`**

After the main parse loop in `parse_hrf()`, before returning `Ok(doc)`:
```rust
// Resolve {dep:} decorators into edges
for (from_id, dep_target) in node_deps {
    // Try id_map first, then label_map slug
    let target_id = id_map.get(dep_target.as_str())
        .or_else(|| {
            let slug = dep_target.to_lowercase().replace(' ', "-");
            label_map.get(slug.as_str())
        })
        .copied();
    if let Some(to_id) = target_id {
        let edge = Edge {
            id: EdgeId(uuid::Uuid::new_v4()),
            source: from_id,
            target: to_id,
            label: String::new(),
            style: EdgeStyle { dashed: true, ..Default::default() },
            ..Default::default()
        };
        doc.edges.push(edge);
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test test_metric_decorator test_owner_decorator test_dep_generates_edge 2>&1 | tail -20
```
Expected: all 3 PASS.

- [ ] **Step 6: Run full test suite**

```bash
cargo test 2>&1 | tail -10
```
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/specgraph/hrf.rs
git commit -m "feat: parse {metric:}, {owner:}, {dep:} decorators in HRF"
```

---

### Task 1.3: HRF parser — new shape tags and business tags

**Files:**
- Modify: `src/specgraph/hrf.rs`

- [ ] **Step 1: Write tests**

```rust
#[test]
fn test_shape_person() {
    let spec = "## Nodes\n- Alice {shape:person}\n";
    let doc = parse_hrf(spec).unwrap();
    let NodeKind::Shape { shape, .. } = &doc.nodes[0].kind else { panic!() };
    assert_eq!(*shape, NodeShape::Person);
}

#[test]
fn test_shape_screen() {
    let spec = "## Nodes\n- Login Screen {shape:screen}\n";
    let doc = parse_hrf(spec).unwrap();
    let NodeKind::Shape { shape, .. } = &doc.nodes[0].kind else { panic!() };
    assert_eq!(*shape, NodeShape::Screen);
}

#[test]
fn test_business_tag_revenue() {
    let spec = "## Nodes\n- Pro Plan {revenue}\n";
    let doc = parse_hrf(spec).unwrap();
    // {revenue} applies green fill preset
    assert_eq!(doc.nodes[0].style.fill_color[1], 227); // green channel high
}

#[test]
fn test_milestone_tag() {
    let spec = "## Nodes\n- Launch {milestone}\n";
    let doc = parse_hrf(spec).unwrap();
    let NodeKind::Shape { shape, .. } = &doc.nodes[0].kind else { panic!() };
    assert_eq!(*shape, NodeShape::Diamond);
}
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cargo test test_shape_person test_shape_screen test_business_tag_revenue test_milestone_tag 2>&1 | tail -20
```

- [ ] **Step 3: Extend `tag_to_shape()` with new variants**

In the `tag_to_shape()` function (around line 3200):
```rust
"person" | "user" | "actor" | "human" | "stick-figure" => NodeShape::Person,
"screen" | "ui" | "mockup" | "wireframe" | "page" | "view" => NodeShape::Screen,
"cylinder" | "db" | "database" | "storage" | "drum" => NodeShape::Cylinder,
"cloud" | "saas" | "aws" | "gcp" | "azure" | "infra" => NodeShape::Cloud,
"document" | "doc" | "file" | "report" | "spec" => NodeShape::Document,
"channel" | "funnel" | "pipeline" | "flow-channel" => NodeShape::Channel,
"segment" | "group" | "audience" | "cohort" | "team" => NodeShape::Segment,
```

Also add `{milestone}` tag handling in `apply_tag_to_node()`:
```rust
} else if tag == "milestone" {
    if let NodeKind::Shape { ref mut shape, .. } = node.kind {
        *shape = NodeShape::Diamond;
    }
```

- [ ] **Step 4: Add business tags in `apply_tag_to_node()`**

```rust
} else if tag == "revenue" {
    node.style.fill_color = [166, 227, 161, 255]; // green
} else if tag == "cost" {
    node.style.fill_color = [243, 139, 168, 255]; // red
} else if tag == "growth" {
    node.style.fill_color = [249, 226, 175, 255]; // yellow
    // {growth} also sets a badge — stored in sublabel for now
    node.sublabel = "↑".to_string();
} else if tag == "opportunity" {
    node.style.fill_color = [137, 180, 250, 255]; // blue
    node.sublabel = "★".to_string();
} else if tag == "risk" {
    node.tag = Some(NodeTag::Warning);
```

- [ ] **Step 5: Run tests**

```bash
cargo test test_shape_person test_shape_screen test_business_tag_revenue test_milestone_tag 2>&1 | tail -20
```
Expected: all PASS.

- [ ] **Step 6: Run full suite**

```bash
cargo test 2>&1 | tail -10
```

- [ ] **Step 7: Commit**

```bash
git add src/specgraph/hrf.rs
git commit -m "feat: new shape tags (person, screen, cylinder, cloud, document) and business tags"
```

---

### Task 1.4: HRF parser — new layout section headers

**Files:**
- Modify: `src/specgraph/hrf.rs`

Background: The existing `Section` enum has `Period { label }` and `Lane`. The new `## Timeline`, `## Swimlane: Name`, `## OrgTree`, `## Kanban: Name` sections either alias the existing mechanism or set new doc-level flags.

- [ ] **Step 1: Write tests**

```rust
#[test]
fn test_timeline_section_sets_timeline_mode() {
    let spec = "## Config\nflow = LR\n\n## Timeline\n- Q1 2026 {phase:Q1}\n";
    let doc = parse_hrf(spec).unwrap();
    assert!(doc.timeline_mode);
}

#[test]
fn test_swimlane_section_adds_lane() {
    let spec = "## Swimlane: Awareness\n- HN Launch {done}\n";
    let doc = parse_hrf(spec).unwrap();
    assert!(doc.timeline_lanes.contains(&"Awareness".to_string()));
    assert_eq!(doc.nodes[0].timeline_lane, Some("Awareness".to_string()));
}

#[test]
fn test_orgtree_sets_layout_mode() {
    let spec = "## OrgTree\n- CEO\n  - CTO\n  - COO\n";
    let doc = parse_hrf(spec).unwrap();
    assert_eq!(doc.layout_mode, LayoutMode::OrgTree);
}

#[test]
fn test_kanban_section_adds_column() {
    let spec = "## Kanban: Todo\n- Task A {todo}\n";
    let doc = parse_hrf(spec).unwrap();
    assert!(doc.kanban_columns.contains(&"Todo".to_string()));
    assert_eq!(doc.layout_mode, LayoutMode::Kanban);
}
```

- [ ] **Step 2: Run to confirm fail**

```bash
cargo test test_timeline_section test_swimlane_section test_orgtree_sets test_kanban_section 2>&1 | tail -20
```

- [ ] **Step 3: Add section parsing in the header-detection block**

Find where `## Period N: Name` is parsed (around line 651). In the same section-header-detection block, add before or after it:

```rust
} else if trimmed.eq_ignore_ascii_case("## timeline") {
    // Sets timeline_mode = true; nodes declared after get placed on time axis
    doc.timeline_mode = true;
    current_section = Section::Nodes { default_z: current_z };

} else if let Some(lane_name) = trimmed
    .strip_prefix("## Swimlane:")
    .or_else(|| trimmed.strip_prefix("## swimlane:"))
    .map(str::trim)
{
    // Alias for ## Lane: adds a named swimlane
    let label = lane_name.to_string();
    if !doc.timeline_lanes.contains(&label) {
        doc.timeline_lanes.push(label.clone());
    }
    current_lane = Some(label);
    current_section = Section::Lane;

} else if trimmed.eq_ignore_ascii_case("## orgtree") {
    doc.layout_mode = LayoutMode::OrgTree;
    current_section = Section::Nodes { default_z: current_z };

} else if let Some(col_name) = trimmed
    .strip_prefix("## Kanban:")
    .or_else(|| trimmed.strip_prefix("## kanban:"))
    .map(str::trim)
{
    let label = col_name.to_string();
    if !doc.kanban_columns.contains(&label) {
        doc.kanban_columns.push(label.clone());
    }
    doc.layout_mode = LayoutMode::Kanban;
    current_kanban_col = Some(label);
    current_section = Section::Nodes { default_z: current_z };
```

Note: `current_lane` and `current_kanban_col` are new parse-state variables of type `Option<String>`. Add them to the parse state initialization. When parsing nodes in `Section::Lane` and `Section::Nodes` with a current_kanban_col active, set `node.timeline_lane = current_lane.clone()` (reusing the existing field for kanban column too).

- [ ] **Step 4: Run tests**

```bash
cargo test test_timeline_section test_swimlane_section test_orgtree_sets test_kanban_section 2>&1 | tail -20
```

- [ ] **Step 5: Parse `{phase:}` and `{lane:}` decorators**

In `apply_tag_to_node()`:
```rust
} else if let Some(val) = tag.strip_prefix("phase:") {
    node.timeline_period = Some(val.to_string());
} else if let Some(val) = tag.strip_prefix("lane:") {
    node.timeline_lane = Some(val.to_string());
```

- [ ] **Step 6: Add tests for {phase:} and {lane:}**

```rust
#[test]
fn test_phase_decorator() {
    let spec = "## Nodes\n- Alpha {phase:Q1}\n";
    let doc = parse_hrf(spec).unwrap();
    assert_eq!(doc.nodes[0].timeline_period, Some("Q1".to_string()));
}

#[test]
fn test_lane_decorator() {
    let spec = "## Nodes\n- Alpha {lane:Sales}\n";
    let doc = parse_hrf(spec).unwrap();
    assert_eq!(doc.nodes[0].timeline_lane, Some("Sales".to_string()));
}
```

- [ ] **Step 7: Run all tests**

```bash
cargo test 2>&1 | tail -10
```
Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src/specgraph/hrf.rs
git commit -m "feat: parse ## Timeline, ## Swimlane, ## OrgTree, ## Kanban sections; {phase:} {lane:} decorators"
```

---

## Phase 2: Layout Engine

### Task 2.1: Add petgraph dependency and swimlane_layout()

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/specgraph/layout.rs`

- [ ] **Step 1: Add petgraph to Cargo.toml**

```toml
petgraph = "0.6"
```

- [ ] **Step 2: Verify build**

```bash
cargo build 2>&1 | tail -10
```

- [ ] **Step 3: `swimlane_layout()` is already covered by existing `timeline_layout()`**

Run the GTM example spec to confirm:
```bash
# Create a test file /tmp/gtm_test.spec:
cat > /tmp/gtm_test.spec << 'EOF'
## Config
flow = LR

## Swimlane: Awareness
- [hn] HN Launch {done}
- [blog] Dev Blog {wip}

## Swimlane: Revenue
- [pro] Pro Plan {metric:$12/mo} {todo}

## Flow
hn --> pro
EOF
```

Write a test in layout.rs or hrf.rs that parses this and verifies nodes get distinct `timeline_lane` values:
```rust
#[test]
fn test_swimlane_layout_positions_nodes_in_rows() {
    let spec = "## Swimlane: Awareness\n- Alpha\n\n## Swimlane: Revenue\n- Beta\n";
    let mut doc = parse_hrf(spec).unwrap();
    crate::specgraph::layout::auto_layout(&mut doc);
    let alpha_y = doc.nodes[0].position[1];
    let beta_y = doc.nodes[1].position[1];
    assert!((alpha_y - beta_y).abs() > 50.0, "lanes should have different Y positions");
}
```

- [ ] **Step 4: Verify existing timeline_layout covers swimlane case**

```bash
cargo test test_swimlane_layout_positions_nodes 2>&1 | tail -10
```
Expected: PASS (existing timeline_layout already separates by lane).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/specgraph/layout.rs
git commit -m "feat: add petgraph dep, verify swimlane layout via existing timeline_layout"
```

---

### Task 2.2: `orgtree_layout()` using petgraph

**Files:**
- Modify: `src/specgraph/layout.rs`

- [ ] **Step 1: Write test**

```rust
#[test]
fn test_orgtree_layout_root_at_top() {
    use crate::parse_hrf;
    let spec = "## OrgTree\n- [ceo] CEO\n  - [cto] CTO\n  - [coo] COO\n";
    let mut doc = parse_hrf(spec).unwrap();
    auto_layout(&mut doc);
    let ceo = doc.nodes.iter().find(|n| n.hrf_id == "ceo").unwrap();
    let cto = doc.nodes.iter().find(|n| n.hrf_id == "cto").unwrap();
    // CEO (root) should be above CTO (child)
    assert!(ceo.position[1] < cto.position[1], "root should have smaller Y than children");
    // CTO and COO should have the same Y (same depth)
    let coo = doc.nodes.iter().find(|n| n.hrf_id == "coo").unwrap();
    assert!((cto.position[1] - coo.position[1]).abs() < 5.0, "siblings should share Y");
}
```

- [ ] **Step 2: Run to confirm fail**

```bash
cargo test test_orgtree_layout_root_at_top 2>&1 | tail -10
```

- [ ] **Step 3: Implement `orgtree_layout()`**

In `src/specgraph/layout.rs`, add:
```rust
use petgraph::graph::DiGraph;
use petgraph::algo::toposort;

pub fn orgtree_layout(doc: &mut FlowchartDocument) {
    // Build a directed graph from edges
    let mut graph: DiGraph<usize, ()> = DiGraph::new();
    let node_count = doc.nodes.len();
    let mut pg_indices = vec![graph.add_node(0usize); node_count];
    for (i, _) in doc.nodes.iter().enumerate() {
        pg_indices[i] = graph.add_node(i);
    }
    // Remove the dummy initial nodes
    // Actually, simpler: map NodeId -> petgraph index
    let id_to_idx: std::collections::HashMap<_, _> = doc.nodes.iter()
        .enumerate()
        .map(|(i, n)| (n.id, i))
        .collect();
    let mut g: DiGraph<usize, ()> = DiGraph::new();
    let petgraph_nodes: Vec<_> = (0..node_count).map(|i| g.add_node(i)).collect();

    for edge in &doc.edges {
        if let (Some(&src_i), Some(&tgt_i)) = (id_to_idx.get(&edge.source), id_to_idx.get(&edge.target)) {
            g.add_edge(petgraph_nodes[src_i], petgraph_nodes[tgt_i], ());
        }
    }

    // Compute depth of each node (BFS from roots)
    let mut depth = vec![0usize; node_count];
    let mut in_degree = vec![0u32; node_count];
    for edge in &doc.edges {
        if let Some(&tgt_i) = id_to_idx.get(&edge.target) {
            in_degree[tgt_i] += 1;
        }
    }
    let roots: Vec<usize> = (0..node_count).filter(|&i| in_degree[i] == 0).collect();
    let mut queue = std::collections::VecDeque::from(roots);
    while let Some(i) = queue.pop_front() {
        let node_id = doc.nodes[i].id;
        for edge in &doc.edges {
            if edge.source == node_id {
                if let Some(&tgt_i) = id_to_idx.get(&edge.target) {
                    depth[tgt_i] = depth[i] + 1;
                    queue.push_back(tgt_i);
                }
            }
        }
    }

    // Group nodes by depth level
    let max_depth = depth.iter().copied().max().unwrap_or(0);
    let gap_y = doc.layout_gap_main.max(120.0);
    let gap_x = doc.layout_gap_cross.max(160.0);

    for d in 0..=max_depth {
        let at_depth: Vec<usize> = (0..node_count).filter(|&i| depth[i] == d).collect();
        let total_width = at_depth.len() as f32 * gap_x;
        let start_x = -total_width / 2.0;
        for (j, &i) in at_depth.iter().enumerate() {
            doc.nodes[i].position = [
                start_x + j as f32 * gap_x,
                d as f32 * gap_y,
            ];
        }
    }
}
```

- [ ] **Step 4: Wire into `auto_layout()`**

In `auto_layout()`, add:
```rust
if doc.layout_mode == LayoutMode::OrgTree {
    orgtree_layout(doc);
    return;
}
```
Add the `use crate::model::LayoutMode;` import if needed.

- [ ] **Step 5: Run test**

```bash
cargo test test_orgtree_layout_root_at_top 2>&1 | tail -10
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/specgraph/layout.rs
git commit -m "feat: orgtree_layout() using petgraph for top-down tree positioning"
```

---

### Task 2.3: `kanban_layout()`

**Files:**
- Modify: `src/specgraph/layout.rs`

- [ ] **Step 1: Write test**

```rust
#[test]
fn test_kanban_layout_columns_side_by_side() {
    let spec = "## Kanban: Todo\n- Task A\n- Task B\n\n## Kanban: Done\n- Task C\n";
    let mut doc = parse_hrf(spec).unwrap();
    auto_layout(&mut doc);
    let a = doc.nodes.iter().find(|n| { if let NodeKind::Shape { label, .. } = &n.kind { label == "Task A" } else { false } }).unwrap();
    let c = doc.nodes.iter().find(|n| { if let NodeKind::Shape { label, .. } = &n.kind { label == "Task C" } else { false } }).unwrap();
    // Todo column left of Done column
    assert!(a.position[0] < c.position[0], "Todo column should be left of Done");
    // Task A and Task B should have different Y (stacked in column)
    let b = doc.nodes.iter().find(|n| { if let NodeKind::Shape { label, .. } = &n.kind { label == "Task B" } else { false } }).unwrap();
    assert!((a.position[1] - b.position[1]).abs() > 40.0, "cards in same column should stack");
}
```

- [ ] **Step 2: Run to confirm fail**

```bash
cargo test test_kanban_layout_columns_side_by_side 2>&1 | tail -10
```

- [ ] **Step 3: Implement `kanban_layout()`**

```rust
pub fn kanban_layout(doc: &mut FlowchartDocument) {
    let col_width = 200.0f32;
    let card_height = 80.0f32;
    let gap_y = 20.0f32;
    let padding_top = 60.0f32; // space for column header

    for (col_idx, col_name) in doc.kanban_columns.iter().enumerate() {
        let col_x = col_idx as f32 * (col_width + 40.0);
        let mut row = 0usize;
        for node in doc.nodes.iter_mut() {
            let in_col = node.timeline_lane.as_deref() == Some(col_name.as_str());
            if in_col {
                node.position = [
                    col_x,
                    padding_top + row as f32 * (card_height + gap_y),
                ];
                node.size = [col_width - 20.0, card_height];
                row += 1;
            }
        }
    }
    // Nodes without a column: place below all columns
    let unassigned_y = doc.kanban_columns.len() as f32 * 200.0 + 40.0;
    let mut unassigned_x = 0.0f32;
    for node in doc.nodes.iter_mut() {
        if node.timeline_lane.is_none() {
            node.position = [unassigned_x, unassigned_y];
            unassigned_x += 200.0;
        }
    }
}
```

Also note: when parsing `## Kanban: Col`, nodes in that section should have `node.timeline_lane = Some(col_name.clone())`. Verify this was implemented in Task 1.4 (`current_kanban_col`).

- [ ] **Step 4: Wire into `auto_layout()`**

```rust
if doc.layout_mode == LayoutMode::Kanban {
    kanban_layout(doc);
    return;
}
```

- [ ] **Step 5: Run test**

```bash
cargo test test_kanban_layout_columns_side_by_side 2>&1 | tail -10
```

- [ ] **Step 6: Run all tests**

```bash
cargo test 2>&1 | tail -10
```

- [ ] **Step 7: Commit**

```bash
git add src/specgraph/layout.rs
git commit -m "feat: kanban_layout() for vertical column card placement"
```

---

## Phase 3: Renderer

### Task 3.1: New shape renderers in `render.rs`

**Files:**
- Modify: `src/app/render.rs`

Background: `draw_node()` uses a match on `node.kind` → `NodeKind::Shape { shape, .. }` → match on shape. Add new arms for each new `NodeShape` variant.

- [ ] **Step 1: Find shape drawing code in render.rs**

```bash
grep -n "NodeShape::\|draw_node\|match shape" src/app/render.rs | head -30
```

- [ ] **Step 2: Add Person shape renderer**

Person = circle head (top 1/3) + rounded rectangle body (bottom 2/3), same fill/border color.

Find the shape match and add:
```rust
NodeShape::Person => {
    let head_r = size.x * 0.22;
    let head_center = pos + vec2(0.0, -size.y * 0.22);
    painter.circle(head_center.to_pos2(), head_r, fill, stroke);
    let body_rect = Rect::from_center_size(
        (pos + vec2(0.0, size.y * 0.15)).to_pos2(),
        vec2(size.x * 0.55, size.y * 0.55),
    );
    painter.rect(body_rect, 14.0, fill, stroke);
},
```

- [ ] **Step 3: Add Screen shape renderer**

Screen = rounded rect with a thin top bar in a slightly different shade.

```rust
NodeShape::Screen => {
    // Main body
    painter.rect(rect, corner_r, fill, stroke);
    // Top chrome bar
    let bar_rect = Rect::from_min_max(
        rect.min,
        pos2(rect.max.x, rect.min.y + 14.0),
    );
    let bar_fill = Color32::from_rgba_unmultiplied(
        fill.r().saturating_add(20),
        fill.g().saturating_add(20),
        fill.b().saturating_add(20),
        fill.a(),
    );
    painter.rect(bar_rect, Rounding { nw: corner_r, ne: corner_r, sw: 0.0, se: 0.0 }, bar_fill, Stroke::NONE);
    // Three tiny dots in bar (traffic lights)
    for (i, dot_color) in [(0, Color32::RED), (1, Color32::YELLOW), (2, Color32::GREEN)].iter() {
        let cx = rect.min.x + 8.0 + *i as f32 * 12.0;
        let cy = rect.min.y + 7.0;
        painter.circle_filled(pos2(cx, cy), 3.0, *dot_color);
    }
},
```

- [ ] **Step 4: Add Cylinder shape renderer**

Cylinder = rectangle body + ellipse top cap + ellipse bottom cap (outline only on bottom).

```rust
NodeShape::Cylinder => {
    let cap_h = size.y * 0.18;
    let body_rect = Rect::from_min_max(
        pos2(rect.min.x, rect.min.y + cap_h * 0.5),
        pos2(rect.max.x, rect.max.y - cap_h * 0.5),
    );
    painter.rect(body_rect, 0.0, fill, Stroke::NONE);
    // Draw side borders
    painter.line_segment([body_rect.left_top(), body_rect.left_bottom()], stroke);
    painter.line_segment([body_rect.right_top(), body_rect.right_bottom()], stroke);
    // Top ellipse (filled — the "lid")
    let top_center = pos2(rect.center().x, rect.min.y + cap_h * 0.5);
    painter.add(egui::Shape::ellipse_filled(top_center, vec2(size.x * 0.5, cap_h * 0.5), fill));
    painter.add(egui::Shape::ellipse_stroke(top_center, vec2(size.x * 0.5, cap_h * 0.5), stroke));
    // Bottom ellipse (outline only — shows depth)
    let bot_center = pos2(rect.center().x, rect.max.y - cap_h * 0.5);
    painter.add(egui::Shape::ellipse_stroke(bot_center, vec2(size.x * 0.5, cap_h * 0.5), stroke));
},
```

- [ ] **Step 5: Add Cloud, Document, Channel, Segment shape renderers**

Cloud: approximate with 4–5 overlapping circles.
```rust
NodeShape::Cloud => {
    // Approximate cloud with overlapping circles
    let cx = rect.center().x;
    let cy = rect.center().y;
    let w = size.x;
    let h = size.y;
    for (dx, dy, r) in [
        (0.0f32,   0.0f32,  h * 0.38),
        (-w*0.25, h*0.08,  h * 0.28),
        ( w*0.25, h*0.08,  h * 0.28),
        (-w*0.12, -h*0.05, h * 0.32),
        ( w*0.12, -h*0.05, h * 0.30),
    ] {
        painter.circle_filled(pos2(cx + dx, cy + dy), r, fill);
    }
    // Bottom flat base
    let base = Rect::from_min_max(
        pos2(rect.min.x + 4.0, cy + h * 0.15),
        pos2(rect.max.x - 4.0, rect.max.y),
    );
    painter.rect_filled(base, 0.0, fill);
    // Outline stroke on whole rect area
    painter.rect(rect, 0.0, Color32::TRANSPARENT, stroke);
},
```

Document: rectangle with folded top-right corner.
```rust
NodeShape::Document => {
    let fold = size.x.min(size.y) * 0.18;
    let points = vec![
        rect.min,
        pos2(rect.max.x - fold, rect.min.y),
        pos2(rect.max.x, rect.min.y + fold),
        rect.max,
        pos2(rect.min.x, rect.max.y),
    ];
    painter.add(egui::Shape::convex_polygon(points.clone(), fill, stroke));
    // Fold crease
    painter.line_segment([
        pos2(rect.max.x - fold, rect.min.y),
        pos2(rect.max.x - fold, rect.min.y + fold),
    ], stroke);
    painter.line_segment([
        pos2(rect.max.x - fold, rect.min.y + fold),
        pos2(rect.max.x, rect.min.y + fold),
    ], stroke);
},
```

Channel (funnel): trapezoid wide at top, narrow at bottom.
```rust
NodeShape::Channel => {
    let points = vec![
        rect.min,
        rect.right_top(),
        pos2(rect.center().x + size.x * 0.15, rect.max.y),
        pos2(rect.center().x - size.x * 0.15, rect.max.y),
    ];
    painter.add(egui::Shape::convex_polygon(points, fill, stroke));
},
```

Segment (person group): two overlapping Person silhouettes.
```rust
NodeShape::Segment => {
    // Draw two slightly offset Person shapes to imply a group
    let offsets = [-size.x * 0.15, size.x * 0.15];
    for dx in offsets {
        let shifted_rect = Rect::from_center_size(
            pos2(rect.center().x + dx, rect.center().y),
            vec2(size.x * 0.6, size.y),
        );
        // Inline person rendering at shifted_rect
        let head_r = shifted_rect.size().x * 0.22;
        let head_center = shifted_rect.center() + vec2(0.0, -shifted_rect.size().y * 0.22);
        painter.circle(head_center.to_pos2(), head_r, fill, stroke);
        let body_rect = Rect::from_center_size(
            (shifted_rect.center() + vec2(0.0, shifted_rect.size().y * 0.15)).to_pos2(),
            vec2(shifted_rect.size().x * 0.55, shifted_rect.size().y * 0.55),
        );
        painter.rect(body_rect, 12.0, fill, stroke);
    }
},
```

- [ ] **Step 6: Build and verify no compile errors**

```bash
cargo build 2>&1 | head -30
```

- [ ] **Step 7: Commit**

```bash
git add src/app/render.rs
git commit -m "feat: render new shapes — person, screen, cylinder, cloud, document, channel, segment"
```

---

### Task 3.2: Metric and owner badge overlays

**Files:**
- Modify: `src/app/render.rs`

- [ ] **Step 1: Add metric badge in `draw_node()` after shape is drawn**

Find where node text is rendered (after shape drawing). Add at the end of `draw_node()`:

```rust
// Metric badge
if let Some(metric) = &node.metric {
    if !metric.is_empty() {
        let badge_pos = pos2(rect.max.x - 4.0, rect.max.y - 4.0); // bottom-right
        let badge_text = metric.as_str();
        let font = egui::FontId::proportional(10.0);
        let galley = painter.layout_no_wrap(badge_text.to_string(), font.clone(), Color32::WHITE);
        let bg_rect = Rect::from_min_size(
            pos2(badge_pos.x - galley.size().x - 6.0, badge_pos.y - galley.size().y - 4.0),
            galley.size() + vec2(6.0, 4.0),
        );
        painter.rect_filled(bg_rect, 4.0, Color32::from_rgba_unmultiplied(30, 30, 46, 200));
        painter.galley(bg_rect.min + vec2(3.0, 2.0), galley, Color32::WHITE);
    }
}

// Owner badge
if let Some(owner) = &node.owner {
    if !owner.is_empty() {
        let initials = owner.trim_start_matches('@')
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_string());
        let badge_center = pos2(rect.max.x - 10.0, rect.min.y + 10.0); // top-right
        painter.circle_filled(badge_center, 9.0, Color32::from_rgb(137, 180, 250));
        let font = egui::FontId::proportional(9.0);
        let galley = painter.layout_no_wrap(initials, font, Color32::from_rgb(30, 30, 46));
        painter.galley(
            pos2(badge_center.x - galley.size().x / 2.0, badge_center.y - galley.size().y / 2.0),
            galley,
            Color32::WHITE,
        );
    }
}
```

- [ ] **Step 2: Build**

```bash
cargo build 2>&1 | head -20
```

- [ ] **Step 3: Commit**

```bash
git add src/app/render.rs
git commit -m "feat: metric badge (bottom-right) and owner avatar (top-right) overlays on nodes"
```

---

### Task 3.3: Phase band backgrounds and lane dividers in canvas

**Files:**
- Modify: `src/app/canvas.rs`

Background: Phase bands are drawn behind nodes. Find the canvas draw function where the background grid/dots are drawn. Phase bands go after background, before nodes.

- [ ] **Step 1: Find where to insert band drawing in canvas.rs**

```bash
grep -n "draw_background\|BgPattern\|fn draw\|painter.rect\|// draw nodes\|for node in" src/app/canvas.rs | head -20
```

- [ ] **Step 2: Add phase band background drawing**

After background pattern drawing and before nodes, add:
```rust
// Phase band backgrounds (for timeline diagrams)
if self.document.timeline_mode && !self.document.timeline_periods.is_empty() {
    // Find x ranges occupied by each period from node positions
    let mut period_x_ranges: std::collections::HashMap<String, (f32, f32)> = Default::default();
    for node in &self.document.nodes {
        if let Some(period) = &node.timeline_period {
            let x = node.position[0];
            let w = node.size[0];
            let entry = period_x_ranges.entry(period.clone()).or_insert((f32::MAX, f32::MIN));
            entry.0 = entry.0.min(x - 20.0);
            entry.1 = entry.1.max(x + w + 20.0);
        }
    }
    let band_colors = [
        Color32::from_rgba_unmultiplied(137, 180, 250, 18),
        Color32::from_rgba_unmultiplied(166, 227, 161, 18),
        Color32::from_rgba_unmultiplied(249, 226, 175, 18),
        Color32::from_rgba_unmultiplied(203, 166, 247, 18),
    ];
    for (idx, period) in self.document.timeline_periods.iter().enumerate() {
        if let Some(&(x_min, x_max)) = period_x_ranges.get(period) {
            let sx_min = x_min * vp.zoom + vp.offset[0];
            let sx_max = x_max * vp.zoom + vp.offset[0];
            let band_rect = Rect::from_min_max(
                pos2(sx_min, painter.clip_rect().min.y),
                pos2(sx_max, painter.clip_rect().max.y),
            );
            let color = band_colors[idx % band_colors.len()];
            painter.rect_filled(band_rect, 0.0, color);
            // Period label at top
            let label_pos = pos2(sx_min + 6.0, painter.clip_rect().min.y + 6.0);
            painter.text(label_pos, egui::Align2::LEFT_TOP, period, egui::FontId::proportional(11.0), Color32::from_rgba_unmultiplied(205, 214, 244, 120));
        }
    }
}
```

- [ ] **Step 3: Add lane dividers for swimlane diagrams**

```rust
// Lane dividers (for swimlane diagrams)
if !self.document.timeline_lanes.is_empty() && self.document.timeline_mode {
    let lane_colors = [
        Color32::from_rgba_unmultiplied(137, 180, 250, 12),
        Color32::from_rgba_unmultiplied(166, 227, 161, 12),
        Color32::from_rgba_unmultiplied(249, 226, 175, 12),
        Color32::from_rgba_unmultiplied(203, 166, 247, 12),
    ];
    let mut lane_y_ranges: std::collections::HashMap<String, (f32, f32)> = Default::default();
    for node in &self.document.nodes {
        if let Some(lane) = &node.timeline_lane {
            let y = node.position[1];
            let h = node.size[1];
            let entry = lane_y_ranges.entry(lane.clone()).or_insert((f32::MAX, f32::MIN));
            entry.0 = entry.0.min(y - 20.0);
            entry.1 = entry.1.max(y + h + 20.0);
        }
    }
    for (idx, lane) in self.document.timeline_lanes.iter().enumerate() {
        if let Some(&(y_min, y_max)) = lane_y_ranges.get(lane) {
            let sy_min = y_min * vp.zoom + vp.offset[1];
            let sy_max = y_max * vp.zoom + vp.offset[1];
            let lane_rect = Rect::from_min_max(
                pos2(painter.clip_rect().min.x, sy_min),
                pos2(painter.clip_rect().max.x, sy_max),
            );
            painter.rect_filled(lane_rect, 0.0, lane_colors[idx % lane_colors.len()]);
            // Lane label on left
            let label_pos = pos2(painter.clip_rect().min.x + 6.0, (sy_min + sy_max) / 2.0);
            painter.text(label_pos, egui::Align2::LEFT_CENTER, lane, egui::FontId::proportional(11.0).clone(), Color32::from_rgba_unmultiplied(205, 214, 244, 150));
        }
    }
}
```

- [ ] **Step 4: Build and run**

```bash
cargo build 2>&1 | head -20
```

- [ ] **Step 5: Commit**

```bash
git add src/app/canvas.rs
git commit -m "feat: phase band backgrounds and lane dividers drawn on canvas"
```

---

### Task 3.4: New shapes in SVG export

**Files:**
- Modify: `src/export.rs`

- [ ] **Step 1: Find where shapes are rendered in SVG export**

```bash
grep -n "NodeShape::\|match shape\|svg_path\|<path\|<rect\|<circle\|<ellipse" src/export.rs | head -30
```

- [ ] **Step 2: Add SVG rendering for each new shape**

For each new `NodeShape` variant, add SVG output. Example for Person:
```rust
NodeShape::Person => {
    let head_r = w * 0.22;
    let head_cx = cx;
    let head_cy = y + h * 0.28;
    let body_x = cx - w * 0.28;
    let body_y = y + h * 0.48;
    let body_w = w * 0.55;
    let body_h = h * 0.45;
    format!(
        r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}" stroke="{}" stroke-width="1.5"/>
           <rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="10" fill="{}" stroke="{}" stroke-width="1.5"/>"#,
        head_cx, head_cy, head_r, fill_hex, stroke_hex,
        body_x, body_y, body_w, body_h, fill_hex, stroke_hex,
    )
},
NodeShape::Cylinder => {
    let cap_h = h * 0.18;
    format!(
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" stroke="{}" stroke-width="1.5"/>
           <ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}" fill="{}" stroke="{}" stroke-width="1.5"/>
           <ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}" fill="none" stroke="{}" stroke-width="1.5"/>"#,
        x, y + cap_h * 0.5, w, h - cap_h, fill_hex, stroke_hex,
        cx, y + cap_h * 0.5, w * 0.5, cap_h * 0.5, fill_hex, stroke_hex,
        cx, y + h - cap_h * 0.5, w * 0.5, cap_h * 0.5, stroke_hex,
    )
},
// Add Screen, Cloud, Document, Channel, Segment similarly
// Minimum viable: fall back to RoundedRect SVG for complex shapes in first pass
NodeShape::Screen | NodeShape::Cloud | NodeShape::Document |
NodeShape::Channel | NodeShape::Segment => {
    // Fallback to rounded rect for SVG (visual parity in v2)
    format!(
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="8" fill="{}" stroke="{}" stroke-width="1.5"/>"#,
        x, y, w, h, fill_hex, stroke_hex
    )
},
```

- [ ] **Step 3: Build**

```bash
cargo build 2>&1 | head -20
```

- [ ] **Step 4: Run all tests**

```bash
cargo test 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add src/export.rs
git commit -m "feat: SVG export for new shapes (Person, Cylinder; RR fallback for complex shapes)"
```

---

## Phase 4: Template Gallery

### Task 4.1: Create bundled template .spec files

**Files:**
- Create: `src/templates/engineering/architecture.spec`
- Create: `src/templates/strategy/roadmap.spec`
- Create: `src/templates/strategy/gtm-strategy.spec`
- Create: `src/templates/org/org-chart.spec`
- Create: (plus others as listed in the spec — minimum set listed here)

- [ ] **Step 1: Create directory structure**

```bash
mkdir -p src/templates/engineering src/templates/strategy src/templates/org src/templates/ops
```

- [ ] **Step 2: Create architecture template**

`src/templates/engineering/architecture.spec`:
```
## Config
auto-tier-color = true
spacing = 130

## Layer 0: Database
- [db] PostgreSQL {cylinder} {done}
  Replace with your database

## Layer 1: API
- [api] REST API {connector} {wip}
  Replace with your backend service

## Layer 2: Frontend
- [web] Web App {screen} {todo}
  Replace with your frontend

## Flow
db --> api: queries
api --> web: JSON
```

- [ ] **Step 3: Create roadmap template**

`src/templates/strategy/roadmap.spec`:
```
## Config
template: roadmap
flow = LR

## Timeline
- Q1 2026 {phase:Q1}
- Q2 2026 {phase:Q2}
- Q3 2026 {phase:Q3}

## Nodes
- [m1] First Milestone {milestone} {phase:Q1} {wip} {owner:@you}
  Describe what you're shipping
- [m2] Second Milestone {milestone} {phase:Q2} {todo}
- [m3] Launch {milestone} {phase:Q3} {todo} {glow}

## Flow
m1 --> m2
m2 --> m3: depends
```

- [ ] **Step 4: Create GTM strategy template**

`src/templates/strategy/gtm-strategy.spec`:
```
## Config
flow = TB

## Swimlane: Awareness
- [top1] Channel 1 {star} {todo} {icon:📢}
- [top2] Channel 2 {star} {todo}

## Swimlane: Acquisition
- [acq1] Free Tier {hexagon} {metric:0 users} {todo}
- [acq2] Growth Loop {hexagon} {todo}

## Swimlane: Revenue
- [rev1] Paid Plan {diamond} {metric:$0/mo} {todo} {glow}

## Flow
top1 --> acq1: drives
acq1 --> rev1: converts
```

- [ ] **Step 5: Create org chart template**

`src/templates/org/org-chart.spec`:
```
## OrgTree

- [ceo] CEO {shape:person} {owner:@name}
  Executive Leader
  - [cto] CTO {shape:person}
    Technology
    - [eng] Engineering Lead {shape:person}
    - [data] Data Lead {shape:person}
  - [coo] COO {shape:person}
    Operations
    - [ops] Ops Lead {shape:person}
```

- [ ] **Step 6: Create remaining templates** (incident-map, user-journey, team-topology, etc.)

Follow the same pattern — use existing HRF features with placeholder content.

- [ ] **Step 7: Commit**

```bash
git add src/templates/
git commit -m "feat: add bundled HRF template files for template gallery"
```

---

### Task 4.2: Create `template_gallery.rs`

**Files:**
- Create: `src/app/template_gallery.rs`
- Modify: `src/app/mod.rs`
- Modify: `src/app/toolbar.rs`

- [ ] **Step 1: Create `template_gallery.rs` with gallery state**

`src/app/template_gallery.rs`:
```rust
use egui::{Context, Ui};
use crate::specgraph::hrf::parse_hrf;
use crate::model::FlowchartDocument;

pub struct TemplateGallery {
    pub open: bool,
    pub search: String,
}

impl Default for TemplateGallery {
    fn default() -> Self {
        Self { open: false, search: String::new() }
    }
}

#[derive(Clone)]
pub struct TemplateEntry {
    pub category: &'static str,
    pub name: &'static str,
    pub spec: &'static str,
}

pub fn all_templates() -> Vec<TemplateEntry> {
    vec![
        TemplateEntry {
            category: "Engineering",
            name: "Architecture",
            spec: include_str!("../templates/engineering/architecture.spec"),
        },
        TemplateEntry {
            category: "Product & Strategy",
            name: "Roadmap",
            spec: include_str!("../templates/strategy/roadmap.spec"),
        },
        TemplateEntry {
            category: "Product & Strategy",
            name: "GTM Strategy",
            spec: include_str!("../templates/strategy/gtm-strategy.spec"),
        },
        TemplateEntry {
            category: "People & Org",
            name: "Org Chart",
            spec: include_str!("../templates/org/org-chart.spec"),
        },
        // Add remaining templates
    ]
}

impl TemplateGallery {
    pub fn show(&mut self, ctx: &Context) -> Option<FlowchartDocument> {
        if !self.open {
            return None;
        }
        let mut chosen_doc: Option<FlowchartDocument> = None;
        egui::Window::new("New Diagram")
            .resizable(true)
            .default_size([700.0, 500.0])
            .collapsible(false)
            .show(ctx, |ui| {
                if ui.button("✕ Close").clicked() {
                    self.open = false;
                }
                ui.add(egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search templates..."));
                ui.separator();

                let templates = all_templates();
                let filtered: Vec<_> = templates.iter()
                    .filter(|t| self.search.is_empty()
                        || t.name.to_lowercase().contains(&self.search.to_lowercase())
                        || t.category.to_lowercase().contains(&self.search.to_lowercase()))
                    .collect();

                // Group by category
                let mut categories: Vec<&str> = Vec::new();
                for t in &filtered {
                    if !categories.contains(&t.category) {
                        categories.push(t.category);
                    }
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for cat in &categories {
                        ui.label(egui::RichText::new(*cat).strong());
                        ui.horizontal_wrapped(|ui| {
                            for t in filtered.iter().filter(|t| t.category == *cat) {
                                if ui.button(t.name).clicked() {
                                    if let Ok(doc) = parse_hrf(t.spec) {
                                        chosen_doc = Some(doc);
                                        self.open = false;
                                    }
                                }
                            }
                        });
                        ui.add_space(8.0);
                    }
                    // Blank options
                    ui.label(egui::RichText::new("Blank").strong());
                    if ui.button("Empty Canvas").clicked() {
                        chosen_doc = Some(FlowchartDocument::default());
                        self.open = false;
                    }
                });
            });
        chosen_doc
    }
}
```

- [ ] **Step 2: Add `template_gallery` module to `src/app/mod.rs`**

In `src/app/mod.rs`, add:
```rust
pub mod template_gallery;
```

And add a `TemplateGallery` field to `FlowchartApp`:
```rust
pub(crate) template_gallery: template_gallery::TemplateGallery,
```

Initialize in `FlowchartApp::new()`:
```rust
template_gallery: template_gallery::TemplateGallery::default(),
```

In the `update()` function, add before the canvas draw:
```rust
if let Some(new_doc) = self.template_gallery.show(ctx) {
    self.document = new_doc;
    self.history.push(&self.document);
    // Run layout on new document
    crate::specgraph::layout::auto_layout(&mut self.document);
}
```

- [ ] **Step 3: Add `Cmd+N` keyboard shortcut**

In `src/app/shortcuts.rs` (or wherever keyboard events are handled), add:
```rust
if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::N)) {
    self.template_gallery.open = true;
}
```

- [ ] **Step 4: Add "New Diagram" button to toolbar**

In `src/app/toolbar.rs`, near the top of `draw_toolbar()`, add:
```rust
if ui.button("📋 New Diagram").clicked() {
    self.template_gallery.open = true;
}
```

- [ ] **Step 5: Build**

```bash
cargo build 2>&1 | head -30
```

- [ ] **Step 6: Run all tests**

```bash
cargo test 2>&1 | tail -10
```

- [ ] **Step 7: Commit**

```bash
git add src/app/template_gallery.rs src/app/mod.rs src/app/toolbar.rs src/app/shortcuts.rs
git commit -m "feat: template gallery panel with bundled templates, Cmd+N shortcut"
```

---

## Phase 5: CLI / Headless

### Task 5.1: Restructure `main.rs` for CLI subcommands

**Files:**
- Modify: `Cargo.toml` (add `clap`)
- Modify: `src/main.rs`

- [ ] **Step 1: Add clap to Cargo.toml**

```toml
clap = { version = "4", features = ["derive"] }
tiny_http = "0.12"
notify = "6"
```

- [ ] **Step 2: Restructure main.rs**

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "light-figma", about = "Lightweight diagramming tool")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a .spec file to SVG without opening the GUI
    Render {
        input: PathBuf,
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Validate HRF syntax and report errors
    Validate {
        input: PathBuf,
    },
    /// Export HRF grammar for a template (for LLM context injection)
    Schema {
        #[arg(long, default_value = "")]
        template: String,
    },
    /// Diff two .spec files and report changed nodes/edges
    Diff {
        before: PathBuf,
        after: PathBuf,
    },
    /// Generate HRF from prose via LLM (requires ANTHROPIC_API_KEY)
    Generate {
        #[arg(long, default_value = "")]
        template: String,
    },
    /// Watch a directory and regenerate SVG on file changes
    Watch {
        directory: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "")]
        template: String,
    },
    /// Start local HTTP render server (POST /render → SVG)
    Serve {
        #[arg(long, default_value = "8080")]
        port: u16,
    },
}

fn main() -> eframe::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Render { input, out }) => {
            cli_render(input, out);
            return Ok(());
        },
        Some(Commands::Validate { input }) => {
            cli_validate(input);
            return Ok(());
        },
        Some(Commands::Schema { template }) => {
            cli_schema(&template);
            return Ok(());
        },
        Some(Commands::Diff { before, after }) => {
            cli_diff(before, after);
            return Ok(());
        },
        Some(Commands::Generate { template }) => {
            cli_generate(&template);
            return Ok(());
        },
        Some(Commands::Watch { directory, out, template }) => {
            cli_watch(directory, out, &template);
            return Ok(());
        },
        Some(Commands::Serve { port }) => {
            cli_serve(port);
            return Ok(());
        },
        None => {} // Fall through to GUI
    }

    // GUI mode (no subcommand)
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 860.0])
            .with_title("Light Figma"),
        ..Default::default()
    };
    eframe::run_native(
        "Light Figma",
        options,
        Box::new(|cc| Ok(Box::new(app::FlowchartApp::new(cc)))),
    )
}
```

- [ ] **Step 3: Build (will fail until cli_* functions are implemented)**

```bash
cargo build 2>&1 | head -20
```

- [ ] **Step 4: Commit the stub**

```bash
git add Cargo.toml src/main.rs
git commit -m "feat: restructure main.rs with clap CLI subcommands (stub implementations)"
```

---

### Task 5.2: Implement `render` and `validate` subcommands

**Files:**
- Modify: `src/main.rs`

The `render` subcommand is trivially simple because `export_svg()` already takes `&FlowchartDocument`:

- [ ] **Step 1: Write test for CLI render**

```rust
#[test]
fn test_cli_render_produces_svg() {
    let spec = "## Nodes\n- Alpha {done}\n- Beta {todo}\n## Flow\nAlpha --> Beta\n";
    let mut doc = crate::specgraph::hrf::parse_hrf(spec).unwrap();
    crate::specgraph::layout::auto_layout(&mut doc);
    let tmp = std::env::temp_dir().join("test_render.svg");
    crate::export::export_svg(&doc, &tmp).unwrap();
    let content = std::fs::read_to_string(&tmp).unwrap();
    assert!(content.contains("<svg"));
    assert!(content.contains("Alpha"));
}
```

- [ ] **Step 2: Run to confirm test passes** (export_svg already works headless)

```bash
cargo test test_cli_render_produces_svg 2>&1 | tail -10
```

- [ ] **Step 3: Implement `cli_render()` and `cli_validate()`**

In `src/main.rs`:
```rust
fn cli_render(input: std::path::PathBuf, out: std::path::PathBuf) {
    let spec = std::fs::read_to_string(&input)
        .unwrap_or_else(|e| { eprintln!("Error reading {:?}: {}", input, e); std::process::exit(1); });
    let mut doc = crate::specgraph::hrf::parse_hrf(&spec)
        .unwrap_or_else(|e| { eprintln!("Parse error: {}", e); std::process::exit(1); });
    crate::specgraph::layout::auto_layout(&mut doc);
    crate::export::export_svg(&doc, &out)
        .unwrap_or_else(|e| { eprintln!("Export error: {}", e); std::process::exit(1); });
    println!("Rendered {:?} → {:?}", input, out);
}

fn cli_validate(input: std::path::PathBuf) {
    let spec = std::fs::read_to_string(&input)
        .unwrap_or_else(|e| { eprintln!("Error reading {:?}: {}", input, e); std::process::exit(1); });
    match crate::specgraph::hrf::parse_hrf(&spec) {
        Ok(doc) => {
            println!("✓ Valid — {} nodes, {} edges", doc.nodes.len(), doc.edges.len());
        },
        Err(e) => {
            eprintln!("✗ Invalid: {}", e);
            std::process::exit(1);
        }
    }
}
```

Note: `src/lib.rs` does NOT exist — this is a binary-only crate. All CLI functions live in `main.rs`. Replace the `crate::specgraph::hrf::parse_hrf` prefix with `crate::specgraph::hrf::parse_hrf` everywhere in `main.rs`. The `mod` declarations (`mod specgraph; mod export;`) at the top of `main.rs` make these paths available. Add `mod specgraph;` and `mod export;` to `main.rs` if not already present — check what's currently declared.

- [ ] **Step 4: Implement `cli_schema()` and `cli_diff()`**

```rust
fn cli_schema(template: &str) {
    // Output a concise summary of HRF syntax for LLM context injection
    let schema = format!(
        "HRF (Human-Readable Format) for light-figma diagrams.\n\
         Template: {}\n\
         \n\
         SYNTAX:\n\
         ## Config\n  flow = TB|LR|RL|BT\n  template: roadmap|gtm|orgchart|pipeline|journey\n\
         \n\
         ## Nodes\n  - [id] Label {{tag}} {{tag}}\n    Description text\n\
         \n\
         ## Flow\n  id --> id: edge label\n\
         \n\
         COMMON TAGS:\n\
         {{done}} {{wip}} {{todo}} {{blocked}} — status\n\
         {{milestone}} {{phase:Q1}} {{date:2026-Q1}} — timeline\n\
         {{lane:Name}} — swimlane\n\
         {{metric:$2M}} {{owner:@alice}} — badges\n\
         {{dep:id}} — dependency edge\n\
         {{shape:person|screen|cylinder|cloud|document}} — shapes\n\
         {{revenue}} {{cost}} {{growth}} {{risk}} — business\n\
         {{icon:🚀}} {{glow}} {{dim}} {{bold}} — style\n\
         ",
        if template.is_empty() { "none" } else { template }
    );
    println!("{}", schema);
}

fn cli_diff(before: std::path::PathBuf, after: std::path::PathBuf) {
    let spec_a = std::fs::read_to_string(&before).unwrap_or_default();
    let spec_b = std::fs::read_to_string(&after).unwrap_or_default();
    let doc_a = crate::specgraph::hrf::parse_hrf(&spec_a).unwrap_or_default();
    let doc_b = crate::specgraph::hrf::parse_hrf(&spec_b).unwrap_or_default();

    let ids_a: std::collections::HashSet<String> = doc_a.nodes.iter().map(|n| n.hrf_id.clone()).collect();
    let ids_b: std::collections::HashSet<String> = doc_b.nodes.iter().map(|n| n.hrf_id.clone()).collect();

    for id in ids_b.difference(&ids_a) { println!("+ node: {}", id); }
    for id in ids_a.difference(&ids_b) { println!("- node: {}", id); }

    let edges_a: std::collections::HashSet<String> = doc_a.edges.iter()
        .map(|e| format!("{} → {}", e.source, e.target)).collect();
    let edges_b: std::collections::HashSet<String> = doc_b.edges.iter()
        .map(|e| format!("{} → {}", e.source, e.target)).collect();

    for e in edges_b.difference(&edges_a) { println!("+ edge: {}", e); }
    for e in edges_a.difference(&edges_b) { println!("- edge: {}", e); }
}
```

- [ ] **Step 5: Build**

```bash
cargo build 2>&1 | head -20
```

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: cli render, validate, schema, diff subcommands"
```

---

### Task 5.3: Implement `serve` subcommand

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement `cli_serve()`**

```rust
fn cli_serve(port: u16) {
    use tiny_http::{Server, Response, Header};
    let addr = format!("0.0.0.0:{}", port);
    let server = Server::http(&addr)
        .unwrap_or_else(|e| { eprintln!("Failed to start server on {}: {}", addr, e); std::process::exit(1); });
    println!("light-figma render server listening on http://localhost:{}", port);
    println!("POST /render with HRF spec body → returns SVG");

    for request in server.incoming_requests() {
        if request.url() != "/render" || request.method().as_str() != "POST" {
            let _ = request.respond(Response::from_string("POST /render only").with_status_code(404));
            continue;
        }
        let mut body = String::new();
        let mut r = request;
        let _ = std::io::Read::read_to_string(r.as_reader(), &mut body);

        match crate::specgraph::hrf::parse_hrf(&body) {
            Ok(mut doc) => {
                crate::specgraph::layout::auto_layout(&mut doc);
                let tmp = std::env::temp_dir().join("lf_serve_tmp.svg");
                match crate::export::export_svg(&doc, &tmp) {
                    Ok(()) => {
                        let svg = std::fs::read_to_string(&tmp).unwrap_or_default();
                        let response = Response::from_string(svg)
                            .with_header(Header::from_bytes("Content-Type", "image/svg+xml").unwrap());
                        let _ = r.respond(response);
                    },
                    Err(e) => {
                        let _ = r.respond(Response::from_string(format!("Export error: {}", e)).with_status_code(500));
                    }
                }
            },
            Err(e) => {
                let _ = r.respond(Response::from_string(format!("Parse error: {}", e)).with_status_code(400));
            }
        }
    }
}
```

- [ ] **Step 2: Add `prose_to_hrf()` to `src/specgraph/llm.rs`**

The existing `prose_to_yaml()` uses an OpenAI-compatible API with SpecGraph YAML as output — not HRF. Add a new `prose_to_hrf()` function that uses Anthropic's API and outputs HRF directly. Add after the existing `prose_to_yaml()`:

```rust
const HRF_SYSTEM_PROMPT: &str = r#"You are a diagram generator. Convert the user's description into HRF (Human-Readable Format) for the light-figma diagramming tool.

Output ONLY valid HRF text. No markdown fences. No explanation.

HRF format example:
## Config
flow = LR

## Nodes
- [api] REST API {shape:cylinder}
- [db] Database {shape:cylinder} {done}
- [ui] Frontend {wip}

## Flow
ui --> api: requests
api --> db: queries

For roadmaps use ## Timeline sections with {phase:Q1} {milestone} tags.
For GTM use ## Swimlane: Name sections with {metric:N} tags.
Use {done} {wip} {todo} for status. Use {owner:@name} for ownership."#;

/// Convert prose to HRF using Anthropic API (blocking via curl).
pub fn prose_to_hrf(prose: &str, template: &str, api_key: &str) -> Result<String, String> {
    let system = format!("{}\n\nTemplate hint: {}", HRF_SYSTEM_PROMPT, template);
    let body = serde_json::json!({
        "model": "claude-opus-4-5",
        "max_tokens": 2048,
        "system": system,
        "messages": [{"role": "user", "content": prose}]
    });
    let body_str = serde_json::to_string(&body)
        .map_err(|e| format!("JSON serialize error: {}", e))?;
    let output = std::process::Command::new("curl")
        .args(["-s", "-X", "POST",
            "https://api.anthropic.com/v1/messages",
            "-H", "Content-Type: application/json",
            "-H", "anthropic-version: 2023-06-01",
            "-H", &format!("x-api-key: {}", api_key),
            "-d", &body_str])
        .output()
        .map_err(|e| format!("Failed to call Anthropic API: {}", e))?;
    if !output.status.success() {
        return Err(format!("API request failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    let response: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| format!("Parse error: {}", e))?;
    if let Some(err) = response["error"]["message"].as_str() {
        return Err(format!("API error: {}", err));
    }
    response["content"][0]["text"].as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| format!("Unexpected API response: {}", &String::from_utf8_lossy(&output.stdout)[..200.min(output.stdout.len())]))
}
```

- [ ] **Step 2b: Implement `cli_generate()` in `main.rs` using `prose_to_hrf()`**

```rust
fn cli_generate(template: &str) {
    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        eprintln!("Error: ANTHROPIC_API_KEY environment variable not set.\nSet it with: export ANTHROPIC_API_KEY=your-key");
        std::process::exit(1);
    });
    let mut prose = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut prose).unwrap();
    match crate::specgraph::llm::prose_to_hrf(&prose, template, &api_key) {
        Ok(hrf) => print!("{}", hrf),
        Err(e) => { eprintln!("LLM error: {}", e); std::process::exit(1); }
    }
}
```

- [ ] **Step 3: Implement `cli_watch()` using notify crate**

```rust
fn cli_watch(directory: std::path::PathBuf, out: std::path::PathBuf, template: &str) {
    use notify::{Watcher, RecursiveMode, recommended_watcher, Event};
    use std::sync::mpsc::channel;

    println!("Watching {:?} → {:?}", directory, out);
    let (tx, rx) = channel::<notify::Result<Event>>();
    let mut watcher = recommended_watcher(tx).unwrap();
    watcher.watch(&directory, RecursiveMode::Recursive).unwrap();

    // Initial render
    regenerate_watch(&directory, &out, template);

    for res in rx {
        if let Ok(event) = res {
            if event.paths.iter().any(|p| p.extension().map_or(false, |e| e == "spec" || e == "rs")) {
                println!("Change detected — regenerating...");
                regenerate_watch(&directory, &out, template);
            }
        }
    }
}

fn regenerate_watch(dir: &std::path::Path, out: &std::path::Path, _template: &str) {
    // Find first .spec file in directory
    if let Some(spec_path) = std::fs::read_dir(dir).ok()
        .and_then(|entries| entries.filter_map(|e| e.ok())
            .find(|e| e.path().extension().map_or(false, |x| x == "spec")))
        .map(|e| e.path())
    {
        cli_render(spec_path, out.to_path_buf());
    }
}
```

- [ ] **Step 4: Build**

```bash
cargo build 2>&1 | head -20
```

- [ ] **Step 5: Run all tests**

```bash
cargo test 2>&1 | tail -10
```

- [ ] **Step 6: Commit**

```bash
git add src/main.rs Cargo.toml
git commit -m "feat: cli serve (tiny_http), generate (LLM via llm.rs), watch (notify crate)"
```

---

## Final Integration Check

- [ ] **Verify the full GTM example spec parses and renders correctly**

Create `/tmp/test_gtm.spec` with the GTM example from the spec doc and run:
```bash
./target/debug/light-figma render /tmp/test_gtm.spec --out /tmp/test_gtm.svg
cat /tmp/test_gtm.svg | grep -c "rect\|circle\|path"  # should be > 5
```

- [ ] **Verify the roadmap example**

```bash
./target/debug/light-figma render /tmp/test_roadmap.spec --out /tmp/test_roadmap.svg
```

- [ ] **Verify validate and schema work**

```bash
./target/debug/light-figma validate /tmp/test_gtm.spec
./target/debug/light-figma schema --template roadmap
```

- [ ] **Run full test suite one final time**

```bash
cargo test 2>&1 | tail -20
```
Expected: all tests pass (including the ~64 existing HRF tests + new ones added in this plan).

- [ ] **Final commit**

```bash
git add -u
git commit -m "feat: complete general diagramming language — all 5 phases implemented"
```

---

## Backwards Compatibility Checklist

- [ ] Load `biz-roadmap.md` in the GUI and verify it renders correctly
- [ ] Load any existing `.flow` file and verify it opens without errors
- [ ] Verify `{done}` `{wip}` `{todo}` `{glow}` `{dim}` still work on existing specs
- [ ] Verify 3D view still works after changes to `FlowchartDocument`
- [ ] Verify SVG export works for existing node types (Rectangle, Hexagon, Diamond)
