---
name: light-figma
description: Use when working on the light-figma Rust/egui canvas diagramming app — adding nodes, spec features, HRF syntax, 3D support, or generating diagrams from code repos
---

# light-figma

A skill for working on the light-figma Rust/egui canvas application, and for
using it as a general-purpose code-to-diagram tool.

**Project root:** `/Users/joe888777/Desktop/project/experiment/light-figma`

**HRF syntax reference:** See `hrf-spec-reference.md` in this skill directory for the complete spec tag/section reference.

---

## Architecture Reference

### Module ownership

| File | Owns |
|---|---|
| `src/model.rs` | All data types: `Node`, `Edge`, `NodeKind`, `NodeShape`, `Port`, `FlowchartDocument` |
| `src/app/render.rs` | 2D canvas drawing for each `NodeKind` |
| `src/app/render3d.rs` | 3D extruded drawing + camera interaction |
| `src/app/canvas.rs` | 2D drag / click / resize input handling |
| `src/app/interaction.rs` | Shared hit-test, `compute_resize`, `fit_to_content` |
| `src/app/toolbar.rs` | Top toolbar: SPEC import/export, shape picker, cheatsheet |
| `src/app/overlays.rs` | Spec import paste area, syntax overlay, preview parsing |
| `src/app/properties.rs` | Right-side properties panel |
| `src/app/statusbar.rs` | Bottom status bar: coordinates, selection info |
| `src/app/shortcuts.rs` | Keyboard shortcut handling |
| `src/app/command_palette.rs` | Cmd+K command palette |
| `src/app/context_menu.rs` | Right-click context menu |
| `src/app/export_mermaid.rs` | Mermaid syntax export |
| `src/app/camera.rs` | `Camera3D` projection, `compute_z_layers` |
| `src/app/theme.rs` | Colors, fonts, `NodeStyle` defaults |
| `src/export.rs` | SVG + PDF export |
| `src/specgraph/schema.rs` | Serde structs for SpecGraph YAML |
| `src/specgraph/convert.rs` | `FlowchartDocument` ↔ `SpecGraph` |
| `src/specgraph/hrf.rs` | Human-Readable Format parse + export |
| `src/specgraph/layout.rs` | Hierarchical auto-layout |
| `src/specgraph/llm.rs` | Prose → YAML via curl subprocess |

### Rendering pipeline order (2D)

`update()` → `draw_toolbar()` → `draw_canvas()` → `draw_edges()` → `draw_nodes()` → `draw_selection()`

### Rendering pipeline order (3D)

`draw_3d_view()` → `build_projections()` → `draw_ground_grid()` → `draw_edges_3d()` → `draw_nodes_3d()` (back-to-front sorted) → `draw_resize_handles()`

### 3D coordinate system

- World X/Y matches the 2D canvas (Y increases downward).
- `z_offset: f32` on `Node` — positive = closer to camera.
- `Z_SPACING = 120.0` world units per graph layer.
- Semantic tier names: z=0 → "db", z=120 → "api", z=240 → "frontend", z=360 → "edge", z=480 → "infra".
- Camera: orbit with yaw/pitch; `world_up = [0, -1, 0]`.
- **Pitfall — resize sign:** `unproject_drag_delta` maps screen→world with an inverted sign (screen-right = world-left for the default camera). For resize, use `screen_delta / depth_scale` to preserve the correct grow/shrink direction.
- **Pitfall — pending_fit:** `fit_to_content()` must run AFTER `draw_canvas()` has set `canvas_rect`. Use the `pending_fit: bool` flag — set it in toolbar, consume it after canvas draw.
- **Pitfall — `compute_resize` convention:** delta.x > 0 = right handle grows (add to width). delta.y > 0 = bottom handle grows (add to height).

### Key `FlowchartDocument` fields

| Field | Purpose |
|---|---|
| `layout_dir: String` | "TB" / "LR" / "RL" / "BT" |
| `layout_gap_main: f32` | Gap between layers (0 = use default 80) |
| `layout_gap_cross: f32` | Gap within a layer (0 = use default 60) |
| `layer_names: HashMap<i32, String>` | Semantic names for 3D z-tiers |
| `import_hints: ImportHints` | Config flags (view_3d, auto_z, auto_tier_color, canvas_bg, …) |

### HRF parse pipeline (hrf.rs)

Pre-pass: `expand_layers` → `expand_styles` → `expand_palette` → main loop per section.

**`id_map`** (`HashMap<String, NodeId>`): explicit `[id]` keys → NodeId.
**`label_map`** (`HashMap<String, NodeId>`): slugified display label → NodeId (fallback for Flow refs).
Both are built during Nodes/Steps/Grid section parsing and passed to `parse_flow_line_chain`.

---

### Key model types

| Type | Notable fields |
|---|---|
| `NodeStyle` | fill_color, border_color, border_width, text_color, font_size, corner_radius, border_dashed, gradient, gradient_angle, opacity (f32, default 1.0), shadow, bold, italic, text_align, text_valign, **glow (bool)** |
| `Node` | id, kind, position, size, z_offset, style, pinned, tag (Option\<NodeTag\>), icon, sublabel, depth_3d, highlight, progress, comment, url, is_frame, frame_color, collapsed, locked |
| `NodeTag` | Critical / Warning / Ok / Info — colored pill in top-left |
| `EdgeStyle` | color, width, dashed, orthogonal, arrow_head (ArrowHead), curve_bend (f32), animated, glow |
| `ArrowHead` | Filled / Open / Circle / None |
| `Edge` | id, source, target, label, source_label, target_label, source/target_cardinality, style, comment |
| `FlowchartDocument` | nodes, edges, layer_names (HashMap\<i32,String\>), import_hints, layout_dir, layout_gap_main, layout_gap_cross |
| `ImportHints` | view_3d, auto_z, auto_tier_color, canvas_bg, camera_yaw, camera_pitch, project_title |
| `DragState` | None / Panning / DraggingNode / BoxSelect / CreatingEdge / DraggingNewNode / ResizingNode / DraggingEdgeBend |
| `BgPattern` | Dots / Lines / Crosshatch / None |
| `Viewport` | offset \[f32;2\], zoom f32 |

---

### Common coding patterns

```rust
// After any mutation:
self.history.push(&self.document);

// Show a toast:
self.status_message = Some(("Message text".to_string(), std::time::Instant::now()));

// New NodeStyle must always set glow explicitly:
NodeStyle { glow: false, ..Default::default() }

// All new serde fields must have a default for backwards compat:
#[serde(default)]
pub new_field: SomeType,
```

- Export HRF: use friendly shorthand names (e.g. `{dim}` not `{opacity:35}`, `{done}` not `{ok}{progress:100}`).
- Cheatsheet: add entry to `entries: &[(&str, &str)]` in `draw_spec_panel()` in `toolbar.rs`.

---

## Commands

Parse the argument after `/light-figma` to determine the command.

---

## `/light-figma add-node`

Walk through adding a new node shape or kind end-to-end.

### Step 1 — Clarify

Ask the user:
- Is this a new **shape** for the existing `Shape` kind (add `NodeShape` variant), or a new **kind** (new `NodeKind` variant with its own fields)?
- What is its name and default size?
- Does it need special 3D rendering (extruded differently from standard cubes)?

### Step 2 — model.rs

- Add the variant to `NodeShape` (or `NodeKind`).
- Add a `Node::new_<name>()` constructor with a sensible default size.
- If it has unique fields, add them to the kind struct.

### Step 3 — render.rs (2D)

- Add a match arm in `draw_node()`.
- Use `painter.rect_filled` / `painter.circle_filled` / `convex_polygon` / custom path.
- Follow the existing style pattern: fill from `node.style.fill_color`, border from `node.style.border_color`.

### Step 4 — render3d.rs (3D)

- Add a match arm in `draw_node_3d()`.
- Use `draw_depth_faces()` for the extrusion (back rect + top/side/bottom strips + front face).
- Use `shade_color(fill, factor)` for depth shading.
- Compute `extrude` with `compute_extrude()`.

### Step 5 — properties.rs

- Add the shape name string in the `NodeShape` display match.

### Step 6 — toolbar.rs

- Add the shape button/entry in the shape picker section.
- Add a cheatsheet entry in `entries: &[(&str, &str)]` if it has a notable tag alias.

### Step 7 — export.rs

- Add SVG path or shape in the SVG export match.
- Add the same in the PDF export match.

### Step 8 — specgraph

- `convert.rs`: add to `shape_to_str()` and `str_to_shape()`.
- `hrf.rs`: add tag alias(es) in `tag_to_shape()` and `export_hrf()`.

### Step 9 — Verify

```bash
cargo build
# Check: no new warnings, no missing match arms (exhaustiveness)
cargo test
```

---

## `/light-figma add-spec`

Walk through adding a new field or feature to the SpecGraph format.

### Step 1 — Clarify

Ask the user:
- Is this a new field on nodes, edges, or the top-level graph?
- Does it need to appear in both YAML and HRF, or YAML only?
- What is the default value when the field is absent?

### Step 2 — schema.rs

- Add the field to the relevant struct (`SpecNode`, `SpecEdge`, or `SpecGraph`).
- Use `#[serde(default)]` for optional fields.
- Use `Option<T>` for nullable fields.

### Step 3 — convert.rs

- `node_to_spec()` / `edge_to_spec()`: serialize from model → spec.
- `spec_to_node()` / `spec_to_edge()`: deserialize from spec → model.
- Add a helper function if the field needs string↔enum mapping.

### Step 4 — hrf.rs (if applicable)

- `parse_node_line()`: parse the new tag in the `for tag in &tags` loop.
- `parse_flow_line_chain()`: for edge tags.
- `export_hrf()`: emit the tag when the field is non-default.
- Tag format convention: `{key:value}` (e.g. `{z:50}`, `{diamond}`).
- **Friendly shorthand tags** (boolean flags): check `tag == "name"` branch.
- **Export:** use friendly shorthand names for well-known values (e.g. `{dim}` instead of `{opacity:35}`).
- **Flow fallback pattern:** if adding node lookup, both `id_map` (exact) and `label_map` (slug) are available in `parse_flow_line_chain`.

### Step 5 — Toolbar cheatsheet

- Add entry to `entries: &[(&str, &str)]` in `draw_spec_panel()` in `toolbar.rs`.

### Step 6 — Tests

- Add a `#[test]` in the `mod tests` block at bottom of `hrf.rs`.
- Test parse + export roundtrip.
- Run: `cargo test` — current count is ~64 tests.

### Step 7 — Verify

```bash
cargo build
cargo test
```

---

## `/light-figma write-spec`

Generate a well-structured HRF spec for a diagram the user describes.

### Template

```
# Diagram Title

One-sentence description of what this diagram shows.

## Config
flow = LR          # or TB/RL/BT
auto-tier-color = true  # if using 3D layers
spacing = 100      # optional: node gap in px

## Nodes
- [id] Display Label {shape} {fill:color}
  One-line description shown as tooltip.

## Layer 0: Database
- [db] PostgreSQL {database}

## Layer 1: Backend
- [api] REST API {server}

## Layer 2: Frontend
- [web] Web App {rounded_rect}

## Flow
id1 --> id2: edge label
"Display Label" --> "Other Label"  # label-based references also work

## Notes
- Key constraint or insight {color}

## Summary
Brief description stored in doc.description.
```

### Rules for good specs

- Use `{layer:db/api/frontend/edge/infra}` instead of raw `{z:N}` — more readable.
- Use semantic shape presets: `{server}`, `{database}`, `{cloud}`, `{queue}`, `{cache}`, `{user}`.
- Use `{done}`, `{wip}`, `{blocked}` status tags instead of manual `{ok}` + `{progress:N}`.
- Add `{icon:emoji}` to make nodes scannable at a glance.
- Use `{dim}` / `{ghost}` for out-of-scope / deprecated nodes.
- Use `{glow}` on critical path nodes.
- Flow can reference nodes by display label: `"REST API" --> "Database"` (no explicit IDs needed).
- Use `## Layer N: Name` sections to auto-assign z + semantic naming instead of `{z:N}` tags.

---

## `/light-figma read-repo <path>`

Analyze any code repository and produce a light-figma diagram of its structure.

### Step 1 — Explore the repo

Read:
- Top-level directory structure (src layout, packages, modules)
- Key dependency relationships (imports, uses, calls)
- Entry points and public APIs
- Any README or architecture docs

Focus on logical structure, not file-by-file line counts.

### Step 2 — Map to node kinds

| Repo concept | Node kind |
|---|---|
| Top-level package / app | `{rounded_rect}` — main component |
| Module / subsystem | `{rectangle}` — internal block |
| External dependency | `{parallelogram}` — I/O / external |
| API / interface boundary | `{connector}` — connection node |
| Database / store | `{database}` — data store |
| Decision / branch point | `{diamond}` — logic |
| Queue / async | `{queue}` |
| Cache | `{cache}` |

### Step 3 — Build the HRF spec

Write a `.spec` file using layered `## Layer N: Name` sections:
```
# <Repo Name> Architecture

One paragraph describing what this repo does and its overall structure.

## Config
auto-tier-color = true
view = 3d

## Layer 0: Database
- [db] PostgreSQL {database} {done}
  Primary data store.

## Layer 1: Backend
- [api] REST API {server}
  HTTP API layer.

## Layer 2: Frontend
- [web] Web App {rounded_rect}
  React SPA.

## Flow
db --> api --> web

## Summary
Brief architecture overview stored on the document.
```

Rules:
- Keep node count between 5–15 (collapse trivial modules)
- Use semantic layer names (db, api, frontend, edge, infra) for the 3D view
- Write one description line per node — shown as tooltip on hover
- Add `{icon:emoji}` to important nodes for instant recognition

### Step 4 — Write the file

Save as `<repo-name>-structure.spec` in the repo root (or current directory).

### Step 5 — Print the spec

Print the entire spec content so the user can paste directly into light-figma's **SPEC → Import** dialog.

### Step 6 — Import instructions

> Open light-figma → SPEC → Import → paste or open the `.spec` file → press Import. Switch to 3D view to see the layered architecture.

---

## `/light-figma read-spec <path>`

Open light-figma and load a `.spec` file into it.

### Step 1 — Read the file

Read the spec at `<path>` and print its contents so the user can see it.

### Step 2 — Validate (quick check)

Run `cargo validate` to confirm the spec parses cleanly:

```bash
cargo run --quiet -- validate <path>
```

If there are errors, report them and stop. Do not open the GUI with a broken spec.

### Step 3 — Launch the app

```bash
cargo run &
```

Run in background so the shell returns immediately.

### Step 4 — Tell the user how to import

> The app is opening. To load the spec:
> 1. Click **SPEC** in the top toolbar → **Import**
> 2. Click **Open File** and select `<path>`
>    — OR — click **Paste** and paste the spec text printed above.

---

## `/light-figma` (no args)

Print usage:

```
light-figma skill — commands:

  /light-figma add-node            Add a new node shape or kind
  /light-figma add-spec            Add a field to the SpecGraph format
  /light-figma write-spec          Generate an HRF spec for a described diagram
  /light-figma read-repo <path>    Diagram a repository's structure
  /light-figma read-spec <path>    Open light-figma and load a spec file

HRF syntax reference: ~/.claude/skills/light-figma/hrf-spec-reference.md
Project: /Users/joe888777/Desktop/project/experiment/light-figma
```
