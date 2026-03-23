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
    let base = {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join("Library").join("Application Support")
    };
    #[cfg(target_os = "windows")]
    let base = std::path::PathBuf::from(
        std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string())
    );
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = std::env::var("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            std::path::PathBuf::from(home).join(".local").join("share")
        });
    base.join("light-figma").join("autosave.json")
}
```

All platform branches use only `std::env::var` — no new crate dependencies.

**Recovery file format:** JSON of `FlowchartDocument` (already derives `serde::Serialize`).

**Dirty tracking:** `FlowchartApp` gains:

```rust
autosave_last_hash: u64,        // hash of node_count + edge_count at last save
autosave_last_time: std::time::Instant,
autosave_path: std::path::PathBuf,
autosave_dirty: bool,           // set to true after any document mutation
show_restore_prompt: bool,      // true on launch if recent autosave exists
autosave_status: Option<String>,// "Autosaved HH:MM" displayed in statusbar
```

`autosave_dirty` must be set to `true` at **every call site** of `self.history.push(&self.document)` — that is the single canonical mutation point. No other explicit sites are needed.

**Restore prompt on launch:** `FlowchartApp::new()` checks for `autosave.json`. If it exists and is newer than 10 minutes ago, sets `self.show_restore_prompt = true`. The restore prompt is a simple egui Window with "Restore" and "Discard" buttons.

**Status bar indicator:** After each autosave, update `self.autosave_status` to `"Autosaved HH:MM"`. `statusbar.rs` displays it on the right.

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

**Overview:** Presentation mode steps through "slides" in sequence. A slide is any node with `is_frame = true`. Slide order is determined by top-left position sorting.

**Key constraint:** `presentation_mode: bool` already exists on `FlowchartApp` (line 165 of `src/app/mod.rs`). It is already toggled by `Key::F` in `shortcuts.rs`, already hides toolbar + properties panel (`!self.presentation_mode` guard), and already hides the statusbar (early return in `statusbar.rs`). A partial HUD (`draw_presentation_spotlight`) is also already wired in `canvas.rs`. **Do NOT add a new `ViewMode::Presentation` variant** — build slide navigation on top of the existing bool.

### New Fields on `FlowchartApp` (in `src/app/mod.rs`)

```rust
presentation_slide_index: usize,         // current slide (0-based)
presentation_slides: Vec<usize>,         // indices into document.nodes, sorted by position
pending_fit_to_node: Option<usize>,      // consumed in canvas draw to fit viewport to frame
```

### HRF Support (minimal)

The existing `is_frame` field on `Node` already marks frames. No new HRF tag needed for basic presentation mode.

### Entering Presentation Mode

- **Keyboard:** F5 (replaces the existing `Key::F` toggle in `shortcuts.rs`)
- **Toolbar:** "Present" button (added to `toolbar.rs`)

`enter_presentation_mode()` method:
1. Collect indices of all `document.nodes` where `node.is_frame == true`, sorted by `(node.position[1], node.position[0])` (top-to-bottom, left-to-right)
2. If no frames exist: set `presentation_slides = vec![usize::MAX]` as a sentinel for "all nodes" and call `pending_fit = true`
3. Set `presentation_slide_index = 0`
4. Set `presentation_mode = true`
5. Call `self.fit_to_frame(0)` (or `pending_fit = true` if no frames)

`exit_presentation_mode()` method:
1. Set `presentation_mode = false`
2. Clear `presentation_slides`
3. Set `pending_fit = true` to restore overview

### Navigation

In `update()`, **before** the existing arrow-key node-nudging code, add a guard:

```rust
// Slide navigation — consumes arrow keys in presentation mode before nudging
if self.presentation_mode {
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)
                  || i.key_pressed(egui::Key::Space))
    {
        self.presentation_next_slide();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
        self.presentation_prev_slide();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        self.exit_presentation_mode();
    }
    return; // skip rest of normal update (no node-nudge, no editing)
}
```

The early `return` ensures the existing arrow-key node-nudge block never fires while in presentation mode.

`presentation_next_slide()` / `presentation_prev_slide()` advance `presentation_slide_index` (clamped/wrapping) and call `fit_to_frame(new_index)`.

### Fitting to a Slide

```rust
fn fit_to_frame(&mut self, slide_pos: usize) {
    if slide_pos < self.presentation_slides.len() {
        let node_idx = self.presentation_slides[slide_pos];
        if node_idx == usize::MAX {
            // sentinel: no frames — fit all nodes
            self.pending_fit = true;
        } else {
            self.pending_fit_to_node = Some(node_idx);
        }
    }
}
```

`pending_fit_to_node` is consumed in `canvas.rs` after `canvas_rect` is known:

```rust
if let Some(idx) = self.pending_fit_to_node.take() {
    if let Some(node) = self.document.nodes.get(idx) {
        let padding = 40.0;
        // zoom = min(canvas_rect.width() / (node.size[0] + 2*padding),
        //            canvas_rect.height() / (node.size[1] + 2*padding))
        // offset = center node in canvas
        let zoom = (canvas_rect.width() / (node.size[0] + padding * 2.0))
            .min(canvas_rect.height() / (node.size[1] + padding * 2.0));
        self.viewport.zoom = zoom;
        self.viewport.offset = [
            canvas_rect.center().x - node.position[0] * zoom - node.size[0] * zoom / 2.0,
            canvas_rect.center().y - node.position[1] * zoom - node.size[1] * zoom / 2.0,
        ];
    }
}
```

### Overlay Rendering

In presentation mode, a slide-counter HUD replaces the existing `draw_presentation_spotlight`. Draw via `egui::Area::new("presentation_hud").order(Order::Foreground)`:
- Bottom-center: `Slide N / M` pill in semi-transparent gray (only if `presentation_slides` is non-empty and not the sentinel)
- Bottom-right: `ESC to exit` hint

### Files Changed

| File | Change |
|---|---|
| `src/app/mod.rs` | Add 3 new fields; `enter_presentation_mode()`, `exit_presentation_mode()`, `presentation_next_slide()`, `presentation_prev_slide()`, `fit_to_frame()` methods; update F5 handler to call `enter_presentation_mode()` |
| `src/app/shortcuts.rs` | Replace `Key::F` simple toggle with call to `enter_presentation_mode()` / `exit_presentation_mode()` |
| `src/app/toolbar.rs` | "Present" button calls `enter_presentation_mode()` |
| `src/app/canvas.rs` | Consume `pending_fit_to_node` after canvas rect is known; add slide-counter HUD overlay; arrow-key guard (see Navigation section) |

No changes to `src/model.rs` (no `ViewMode` change). `toolbar.rs`, `properties.rs`, and `statusbar.rs` already hide in presentation mode.

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
- [ ] Add `autosave_last_hash`, `autosave_last_time`, `autosave_path`, `autosave_dirty`, `show_restore_prompt`, `autosave_status` fields to `FlowchartApp`
- [ ] Implement `autosave_path()` free function (using `std::env::var`)
- [ ] Implement `do_autosave()` method
- [ ] Set `autosave_dirty = true` at every `self.history.push()` call site
- [ ] Add restore prompt check in `new()` (check file age)
- [ ] Draw restore prompt `egui::Window` in `update()`
- [ ] Add autosave timer check at end of `update()`
- [ ] Add autosave timestamp to `statusbar.rs`

### Feature 6 — Presentation Mode
- [ ] Add `presentation_slide_index`, `presentation_slides`, `pending_fit_to_node` fields to `FlowchartApp`
- [ ] Add `enter_presentation_mode()`, `exit_presentation_mode()`, `presentation_next_slide()`, `presentation_prev_slide()`, `fit_to_frame()` methods
- [ ] Update `Key::F` handler in `shortcuts.rs` to call `enter/exit_presentation_mode()`
- [ ] "Present" button in `toolbar.rs`
- [ ] Arrow key + ESC guard in `canvas.rs` or `update()` (before node-nudge block)
- [ ] Consume `pending_fit_to_node` in `canvas.rs` after `canvas_rect` is set
- [ ] Slide-counter HUD overlay in `canvas.rs`
