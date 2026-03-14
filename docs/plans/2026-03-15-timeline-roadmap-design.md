# Timeline / Roadmap Layout — Design

**Date:** 2026-03-15
**Status:** Approved

---

## Problem

The current HRF spec format produces architecture-style diagrams when used for roadmaps. There is no concept of time progression — nodes are arranged by hierarchy, not by when they happen. The result looks like a structure diagram, not a roadmap.

---

## Goal

Add a first-class timeline mode to light-figma that lets users write roadmap specs with ordered time periods (Q1/Q2/Q3, Phase 1/2/3, Now/Next/Later, etc.) and optional swim-lanes (rows by team or category), rendered as a proper grid diagram.

---

## HRF Syntax

```
# Product Roadmap

## Config
timeline = true          # activates timeline layout mode
timeline-dir = LR        # LR = time left→right (default); TB = time top→bottom

## Period 1: Q1 — Foundation
- [mvp] MVP Launch {done} {lane:Product}
- [auth] Auth System {wip} {lane:Backend}

## Period 2: Q2 — Growth
- [api] Public API {lane:Backend}
- [onboard] Onboarding Flow {lane:Product}

## Period 3: Q3 — Scale
- [perf] Performance Hardening {lane:Backend}

## Lane 1: Product
## Lane 2: Backend
## Lane 3: Design

## Flow
auth --> api: builds on
```

Visual result (LR):

```
             | Q1 — Foundation | Q2 — Growth  | Q3 — Scale |
 Product     |  MVP Launch     |  Onboarding  |            |
 Backend     |  Auth System    |  Public API  |  Perf      |
 Design      |                 |              |            |
```

### Syntax rules

- `timeline = true` in `## Config` activates timeline mode; stored in `ImportHints.timeline_mode`
- `timeline-dir = LR` (or `TB`) in `## Config` controls axis direction; default `LR`; distinct from `flow` which controls hierarchical layout
- `## Period N: Label` — N sets the ordering (1-based integer); nodes parsed below inherit that period label
- `{lane:Name}` tag on a node assigns it to a swim-lane row
- `## Lane N: Name` — optional declarations that set lane order and labels; if absent, lanes are auto-discovered from `{lane:}` tags in encounter order
- `## Flow` edges work normally across periods
- Time unit is user-defined: Quarters, Phases, Months, or any custom label
- `## Timeline` as a bare section header is treated as a parse error / ignored with a warning (avoiding collision with existing `Section::Steps` which the keyword `timeline` maps to internally)

---

## Model Changes (`src/model.rs`)

All new fields use `#[serde(default)]` for backwards compatibility.

### `Node` — two new optional fields

```rust
pub timeline_period: Option<String>,   // e.g. "Q1 — Foundation"
pub timeline_lane:   Option<String>,   // None = unassigned ("(unlaned)")
```

**All five `Node::new_*` constructors** must be updated to initialise these fields to `None`.

### `FlowchartDocument` — all timeline fields (persisted)

`timeline_mode` and `timeline_dir` belong on `FlowchartDocument` (not `ImportHints`) because they must survive save/load cycles and be accessible at all call sites — including re-layout shortcuts in `shortcuts.rs` and `command_palette.rs` (which have no access to `ImportHints`), the render gate in `canvas.rs` (`self.document.timeline_mode`), and `export_hrf` (which takes only `&FlowchartDocument`).

```rust
// Added to FlowchartDocument (all serde persisted with #[serde(default)])
pub timeline_mode:    bool,          // true when `timeline = true` in Config
pub timeline_dir:     String,        // "LR" or "TB", default ""  (treated as "LR")
pub timeline_periods: Vec<String>,   // canonical ordered list of period labels
pub timeline_lanes:   Vec<String>,   // canonical ordered list of lane labels
```

**Source of truth:** `timeline_periods` and `timeline_lanes` are the canonical ordered lists. They are built during HRF parse from `## Period N:` and `## Lane N:` declarations (in ascending N order). Node fields `timeline_period` / `timeline_lane` are assigned by matching against these lists. At layout time, the lists drive grid construction; node fields are used to place each node into the correct cell. If a node's `timeline_lane` is not in the list (auto-discovered), it is appended.

No new node kinds or shapes. Timeline is purely a layout + overlay concern.

---

## HRF Parser (`src/specgraph/hrf.rs`)

### New `Section` variants

```rust
Section::Period {
    label: String,   // display label, e.g. "Q1 — Foundation"
    index: usize,    // 1-based from the header "## Period N:"
},
Section::Lane {
    label: String,   // display label, e.g. "Backend"
    index: usize,    // 1-based from the header "## Lane N:"
},
```

No accumulated node lists needed — nodes inherit the current period context via a `current_period: Option<String>` local variable in the parse loop. This is a new standalone local, not reusing any existing mechanism (the existing layer context is embedded in `Section::Nodes { default_z }` — a different pattern).

### Parse changes

- `timeline = true` Config key → sets `doc.timeline_mode = true`
- `timeline-dir = LR/TB` Config key → sets `doc.timeline_dir`
- `## Period N: Label` header → new `Section::Period { label, index }`; sets `current_period = Some(label)`; appends to `doc.timeline_periods` in index order
- `## Lane N: Name` header → new `Section::Lane { label, index }`; appends to `doc.timeline_lanes` in index order
- `{lane:Name}` tag in node tag loop → sets `node.timeline_lane = Some(name)`; if name not in `doc.timeline_lanes`, append it
- Nodes parsed under a `Section::Period` context get `node.timeline_period = current_period.clone()`
- `## Timeline` as a bare header → **breaking change:** `"timeline"` currently aliases to `Section::Steps` in hrf.rs alongside `"roadmap"`, `"milestones"`, and `"milestone"`. Only `"timeline"` must be removed (it now conflicts with the new section type). `"roadmap"`, `"milestones"`, and `"milestone"` remain as `Section::Steps` aliases — they do not conflict. Existing HRF files using `## Timeline` as a Steps header must migrate to `## Steps`. Emit a parse warning ("'## Timeline' is now a reserved section type; use '## Steps' for step lists") and treat as `Section::None`.

### Export

`export_hrf` takes only `&FlowchartDocument` — no signature change needed since `timeline_mode` and `timeline_dir` are now on the doc.

- Emit `timeline = true` and `timeline-dir` in `## Config` block when `doc.timeline_mode`
- Emit `## Period N: Label` sections in `doc.timeline_periods` order, grouping nodes by `timeline_period`
- Emit `## Lane N: Name` declarations from `doc.timeline_lanes` in order
- Nodes with `timeline_lane = None` → emit `{lane:(unlaned)}` or omit tag (omit is cleaner)
- Nodes with `timeline_period = None` in a timeline doc → emit in a trailing `## Nodes` section

---

## Layout (`src/specgraph/layout.rs`)

New `timeline_layout()` function:

- Builds a Period × Lane grid using `doc.timeline_periods` × `doc.timeline_lanes`
- Adds an implicit `"(unlaned)"` row at the bottom if any nodes have `timeline_lane = None`
- Each cell is sized to fit its node set; cells in the same column share the same width, cells in the same row share the same height
- Nodes within a cell are stacked using a simple top-to-bottom arrangement with padding
- Mutates node positions in place via `&mut FlowchartDocument` — same in-place mutation contract as `hierarchical_layout`

### Layout dispatch

`hierarchical_layout()` is called in multiple places. Each call site must be updated to dispatch based on `doc.timeline_mode` (on `FlowchartDocument`, so always available):

```rust
if doc.timeline_mode {
    timeline_layout(&mut doc);
} else {
    hierarchical_layout(&mut doc);
}
```

`timeline_layout` reads `doc.timeline_dir`, `doc.timeline_periods`, and `doc.timeline_lanes` directly from the document — no separate `ImportHints` parameter needed.

**Call sites to update:**
1. `src/specgraph/hrf.rs` — after parse (main HRF import pipeline)
2. `src/specgraph/convert.rs` — after YAML/SpecGraph import (`super::layout::hierarchical_layout(&mut doc)`)
3. `src/app/shortcuts.rs` — manual re-layout shortcut
4. `src/app/command_palette.rs` — re-layout palette action

---

## Rendering (`src/app/canvas.rs` + `render.rs`)

New `draw_timeline_grid()` — a method in an `impl FlowchartApp` block in `render.rs` (consistent with `draw_node`, `draw_edge`, etc.), called from `canvas.rs` before the node draw loop:

```rust
// in canvas.rs draw pipeline:
if self.document.timeline_mode {
    self.draw_timeline_grid(&painter, canvas_rect);
}
// ... then draw_nodes() as normal
```

Signature: `fn draw_timeline_grid(&self, painter: &egui::Painter, canvas_rect: egui::Rect)`

The `painter` is passed in (same convention as all other draw helpers in render.rs). The method reads `self.document.timeline_periods`, `self.document.timeline_lanes`, `self.document.timeline_dir`, and `self.viewport` to compute screen-space positions for period columns, lane rows, labels, and connectors.

Drawing order within `draw_timeline_grid()`:
1. Lane row backgrounds — alternating subtle tint
2. Period column backgrounds — slightly lighter than canvas bg
3. Period header labels — bold, top of column (LR) or left of row (TB)
4. Lane labels — muted, left edge (LR) or top edge (TB)
5. Period connectors — thin arrow between period headers: `[ Q1 ] ──→ [ Q2 ] ──→ [ Q3 ]`; drawn automatically, not stored as edges
6. Grid lines — 1px, `surface1` color

**3D view** — `draw_timeline_grid()` is 2D only; in 3D mode the overlay is skipped. Nodes render at their laid-out positions with normal 3D projection.

**Properties panel** — when a node is selected and `timeline_mode` is active, show `Period` and `Lane` dropdowns populated from `doc.timeline_periods` / `doc.timeline_lanes`. Unlaned nodes show `"(unlaned)"` in the dropdown.

---

## Edge Cases

| Case | Behaviour |
|---|---|
| Node has `{lane:X}` but X not in `## Lane` declarations | Lane auto-discovered, appended to `doc.timeline_lanes` |
| Node in period, no `{lane:}` tag (lanes exist) | `timeline_lane = None`; placed in implicit `"(unlaned)"` row at bottom of period column |
| Node not in any `## Period` section | `timeline_period = None`; excluded from timeline grid; falls back to hierarchical layout position |
| Empty cell | Renders as blank grid cell — no placeholder node |
| `timeline-dir = TB` | Periods become horizontal rows, lanes become vertical columns; connectors drawn on left margin |
| 3D view active | Timeline grid overlay skipped; nodes at computed positions |
| `## Timeline` bare header | Parse warning emitted; treated as `Section::None` |
| `flow` key present alongside `timeline = true` | `flow` controls hierarchical layout only; `timeline-dir` controls timeline axis; both can coexist |

---

## Files Changed

| File | Change |
|---|---|
| `src/model.rs` | Add `timeline_period`, `timeline_lane` to `Node` (+ all 5 constructors); add `timeline_mode`, `timeline_dir`, `timeline_periods`, `timeline_lanes` to `FlowchartDocument` (all `#[serde(default)]`) |
| `src/specgraph/hrf.rs` | Add `Section::Period` / `Section::Lane` variants; parse `{lane:}` tag, `timeline`/`timeline-dir` Config keys, `## Period`/`## Lane` headers; remove `"timeline"` from Steps aliases; update export; warn on bare `## Timeline` header |
| `src/specgraph/convert.rs` | Update `hierarchical_layout` call to dispatch on `doc.timeline_mode` |
| `src/specgraph/layout.rs` | Add `timeline_layout(&mut FlowchartDocument)`; update all 4 call sites to dispatch on `doc.timeline_mode` |
| `src/app/canvas.rs` | Call `draw_timeline_grid()` before node draw when `timeline_mode` |
| `src/app/render.rs` | Add `draw_timeline_grid()` implementation |
| `src/app/properties.rs` | Add Period / Lane dropdowns to node properties panel when `timeline_mode` |
