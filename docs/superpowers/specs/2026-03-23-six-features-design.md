# Six-Feature Implementation Design

**Date:** 2026-03-23
**Status:** Approved

## Overview

Six features implemented in priority order:

1. **Drag-drop import** — existing approved spec/plan (`2026-03-20-drag-drop-spec-import`)
2. **4 missing templates** — write 4 HRF template files + register in `templates/mod.rs`
3. **General language extensions** — existing approved spec/plan (`2026-03-19-general-diagramming-language`)
4. **CLI `open` subcommand** — positional file argument opens GUI with spec pre-loaded
5. **Autosave** — 30-second recovery writes, restore-on-launch prompt
6. **Presentation mode** — frame nodes as slides, F5 toggle, fullscreen chromeless view

Features 1 and 3 have complete implementation plans in `docs/superpowers/plans/`. This document covers features 2, 4, 5, and 6 in detail.

---

## Feature 2 — 4 Missing Templates

### Files to Create

```
src/templates/org/team-topology.spec
src/templates/org/raci-matrix.spec
src/templates/ops/runbook.spec
src/templates/ops/on-call-tree.spec
```

### Register in `src/templates/mod.rs`

Add four `Template` entries after the existing ones.

### Template Content

**team-topology.spec** — Team Topology diagram (stream-aligned, platform, enabling, complicated-subsystem teams)

```
## Config
title: Team Topology
flow = TB

## Nodes
- [stream1] Stream-Aligned Team A {rounded} {fill:#4a90d9} {bold}
  Owns the customer-facing product flow.
- [stream2] Stream-Aligned Team B {rounded} {fill:#4a90d9} {bold}
  Owns the data ingestion flow.
- [platform] Platform Team {server} {fill:#7b5ea7} {bold}
  Provides self-service infrastructure.
- [enabling] Enabling Team {rounded} {fill:#e8a838}
  Spreads best practices across streams.
- [subsystem] Complicated-Subsystem Team {rounded} {fill:#cc5a4a}
  Owns the payment processing engine.

## Flow
platform --> stream1: X-as-a-Service
platform --> stream2: X-as-a-Service
enabling --> stream1: facilitates
enabling --> stream2: facilitates
subsystem --> stream1: collaboration
```

**raci-matrix.spec** — RACI responsibility matrix

```
## Config
title: RACI Matrix
flow = TB

## Nodes
- [feature] Feature Launch {bold}
- [pm] Product Manager {person}
- [eng] Engineering Lead {person}
- [design] Designer {person}
- [legal] Legal {person}
- [r_pm] Responsible {fill:#4caf50}
- [a_pm] Accountable {fill:#2196f3}
- [c_eng] Consulted {fill:#ff9800}
- [i_legal] Informed {fill:#9e9e9e}

## Flow
feature --> r_pm: PM Responsible
feature --> a_pm: PM Accountable
feature --> c_eng: Eng Consulted
feature --> i_legal: Legal Informed
```

**runbook.spec** — Operational runbook template

```
## Config
title: Runbook
flow = TB

## Nodes
- [trigger] Trigger: Alert / On-Call Page {critical}
- [assess] Assess Severity {diamond}
- [p1] P1 Critical {fill:#cc3333}
- [p2] P2 High {fill:#e8a838}
- [p3] P3 Low {fill:#4caf50}
- [mitigate] Apply Mitigation {rounded}
- [escalate] Escalate to Lead {person}
- [verify] Verify Service Restored {rounded}
- [postmortem] File Post-Mortem {document}
- [close] Close Incident {ok}

## Flow
trigger --> assess
assess --> p1: SEV1
assess --> p2: SEV2
assess --> p3: SEV3
p1 --> escalate
p2 --> mitigate
p3 --> mitigate
escalate --> mitigate: after brief
mitigate --> verify
verify --> postmortem: if P1/P2
verify --> close
postmortem --> close
```

**on-call-tree.spec** — On-call escalation tree

```
## Config
title: On-Call Tree
flow = TB

## OrgTree
- [primary] Primary On-Call {person} {critical}
  - [secondary] Secondary On-Call {person} {warning}
    - [manager] Engineering Manager {person}
      - [vp] VP Engineering {person}
  - [sre] SRE Team Channel {rounded}
  - [runbook2] Runbook / Playbook {document}
```

---

## Feature 4 — CLI `open` Subcommand

### Design

The simplest path: accept an optional positional `file` argument in the GUI branch of `main.rs`. When provided, the file path is passed to `FlowchartApp::new()` as a startup load hint.

**Change to `Cli` struct** (`src/main.rs`):

```rust
#[derive(Parser)]
#[command(name = "light-figma", about = "Lightweight diagramming tool")]
struct Cli {
    /// Optional .spec or .yaml file to open on launch
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}
```

**Pass to app** in the `None` arm:

```rust
None => {
    // GUI mode
    let startup_file = cli.file.clone();
    eframe::run_native(
        "Light Figma",
        options,
        Box::new(move |cc| Ok(Box::new(app::FlowchartApp::new_with_file(cc, startup_file)))),
    )
}
```

**`FlowchartApp::new_with_file`** (`src/app/mod.rs`):

```rust
pub fn new_with_file(cc: &eframe::CreationContext<'_>, file: Option<std::path::PathBuf>) -> Self {
    let mut app = Self::new(cc);
    if let Some(path) = file {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(mut doc) = crate::specgraph::hrf::parse_hrf(&content) {
                crate::specgraph::layout::auto_layout(&mut doc);
                app.document = doc;
                app.pending_fit = true;
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "file".to_string());
                app.status_message = Some((format!("Opened {name}"), std::time::Instant::now()));
            }
        }
    }
    app
}
```

**Files changed:** `src/main.rs`, `src/app/mod.rs`

---

## Feature 5 — Autosave

### Design

**Trigger:** Every 30 seconds, if the document differs from the last autosaved state (tracked via node+edge count as a quick dirty check), write a recovery file.

**Recovery file path:** `{data_dir}/light-figma/autosave.json` where `data_dir` is:
- macOS: `~/Library/Application Support`
- Linux: `~/.local/share`
- Windows: `%APPDATA%`

Computed via a simple platform-specific path (no new crate needed):

```rust
fn autosave_path() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    let base = dirs_base();  // ~/Library/Application Support
    #[cfg(not(target_os = "macos"))]
    let base = std::env::var("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let mut h = dirs_home();
            h.push(".local/share");
            h
        });
    base.join("light-figma").join("autosave.json")
}
```

To avoid adding a new crate dependency, use `std::env::var("HOME")` (Unix) and `APPDATA` (Windows) directly.

**Recovery file format:** JSON of `FlowchartDocument` (already derives `serde::Serialize`).

**Dirty tracking:** `FlowchartApp` gains:

```rust
autosave_last_hash: u64,   // hash of node_count + edge_count
autosave_last_time: std::time::Instant,
autosave_path: std::path::PathBuf,
autosave_dirty: bool,  // set to true after any mutation
```

**Restore prompt on launch:** `FlowchartApp::new()` checks for `autosave.json`. If it exists and is newer than 10 minutes ago, sets `self.show_restore_prompt = true`. The restore prompt is a simple egui Window with "Restore" and "Discard" buttons.

**Status bar indicator:** After each autosave, update `self.autosave_status: Option<String>` to `"Autosaved HH:MM"`. `statusbar.rs` displays it on the right.

**Files changed:** `src/app/mod.rs`, `src/app/statusbar.rs`

### Autosave loop in `update()`

```rust
// Autosave: check every frame if 30s have elapsed and document is dirty
if self.autosave_dirty
    && self.autosave_last_time.elapsed().as_secs() >= 30
{
    self.do_autosave();
}
```

`do_autosave()` serializes the document to JSON and writes to the autosave path.

---

## Feature 6 — Presentation Mode

### Design

**Overview:** Presentation mode hides all UI chrome (toolbar, properties panel, statusbar, command palette) and steps through "slides" in sequence. A slide is any node with `is_frame = true`. Slide order is determined by `{slide:N}` tag (if present) or by top-left position sorting.

### ViewMode Extension

Add `Presentation` variant to the existing `ViewMode` enum in `src/model.rs`:

```rust
pub enum ViewMode {
    TwoD,
    ThreeD,
    Presentation,
}
```

### New Fields on `FlowchartApp`

```rust
presentation_slide_index: usize,   // current slide (0-based)
presentation_slides: Vec<usize>,   // indices into document.nodes for frame nodes, sorted
```

### HRF Support (minimal)

The existing `is_frame` field on `Node` already marks frames. No new HRF tag needed for basic presentation mode. The `{slide:N}` ordering tag is a stretch goal for a later iteration.

### Entering Presentation Mode

- **Keyboard:** F5
- **Toolbar:** "Present" button (added to toolbar.rs)
- Computes `presentation_slides` (sorted frame nodes by position)
- If no frames exist: creates a single virtual "slide" covering all nodes (fit-to-content)
- Sets `view_mode = ViewMode::Presentation`
- Sets `pending_fit = true` to fit the first slide

### Navigation

In `update()`, when `view_mode == Presentation`:

```rust
if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::Space)) {
    self.presentation_next_slide();
}
if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
    self.presentation_prev_slide();
}
if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
    self.view_mode = ViewMode::TwoD;
}
```

`presentation_next_slide()` / `presentation_prev_slide()` advance `presentation_slide_index` and fit the viewport to the target frame's bounds.

### Fitting to a Slide

```rust
fn fit_to_frame(&mut self, node_idx: usize) {
    let node = &self.document.nodes[node_idx];
    // compute viewport offset + zoom to fit node rect with 40px padding
    let padding = 40.0;
    let w = node.size[0] + padding * 2.0;
    let h = node.size[1] + padding * 2.0;
    // zoom = min(canvas_width / w, canvas_height / h)
    // offset = center the node
    self.pending_fit_to_node = Some(node_idx);
}
```

### Overlay Rendering

In presentation mode, a minimal HUD is drawn:
- Bottom-center: `Slide N / M` pill in semi-transparent gray
- Bottom-right: `ESC to exit` hint

Drawn via `egui::Area::new("presentation_hud").order(Order::Foreground)`.

### Files Changed

| File | Change |
|---|---|
| `src/model.rs` | Add `Presentation` to `ViewMode` enum |
| `src/app/mod.rs` | `presentation_slide_index`, `presentation_slides` fields; F5 handler; slide navigation methods |
| `src/app/toolbar.rs` | "Present" button; hide toolbar in Presentation mode |
| `src/app/canvas.rs` | Arrow key slide navigation; ESC exit; HUD overlay |
| `src/app/properties.rs` | Hide panel in Presentation mode |
| `src/app/statusbar.rs` | Hide in Presentation mode; add autosave timestamp display |

---

## Implementation Checklist Summary

### Feature 2 — Templates
- [ ] Write `src/templates/org/team-topology.spec`
- [ ] Write `src/templates/org/raci-matrix.spec`
- [ ] Write `src/templates/ops/runbook.spec`
- [ ] Write `src/templates/ops/on-call-tree.spec`
- [ ] Register all 4 in `src/templates/mod.rs`
- [ ] `cargo test` — `test_all_templates_parse` must pass for all new templates

### Feature 4 — CLI Open
- [ ] Add `file: Option<PathBuf>` to `Cli` struct in `main.rs`
- [ ] Add `FlowchartApp::new_with_file()` in `mod.rs`
- [ ] Wire `cli.file` into `new_with_file` in the `None` arm

### Feature 5 — Autosave
- [ ] Add `autosave_*` fields to `FlowchartApp`
- [ ] Implement `autosave_path()` free function
- [ ] Implement `do_autosave()` method
- [ ] Add restore prompt check in `new()` and `show_restore_prompt` field
- [ ] Draw restore prompt `egui::Window` in `update()`
- [ ] Add autosave timer check at end of `update()`
- [ ] Add autosave status to statusbar.rs

### Feature 6 — Presentation Mode
- [ ] Add `Presentation` to `ViewMode` in `model.rs`
- [ ] Add `presentation_slide_index`, `presentation_slides` to `FlowchartApp`
- [ ] Add `enter_presentation_mode()`, `exit_presentation_mode()`, `presentation_next_slide()`, `presentation_prev_slide()` methods
- [ ] F5 handler in `shortcuts.rs` (or `update()`)
- [ ] "Present" button in toolbar
- [ ] Hide toolbar, properties, statusbar in Presentation mode
- [ ] Arrow key navigation in `update()` when in Presentation mode
- [ ] ESC exit
- [ ] HUD overlay (slide counter + ESC hint)
- [ ] `fit_to_frame()` method
