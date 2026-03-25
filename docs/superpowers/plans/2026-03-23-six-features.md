# Six Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement six features for light-figma in priority order: drag-drop import, 4 missing templates, general language extensions, CLI open subcommand, autosave, and presentation mode slide navigation.

**Architecture:** Features 1 and 3 each have their own dedicated plans. Features 2, 4, 5, and 6 are implemented directly per the spec at `docs/superpowers/specs/2026-03-23-six-features-design.md`. Features are independent of each other and can be executed sequentially without conflicts.

**Tech Stack:** Rust, egui/eframe, `serde_json` (already in `Cargo.toml`), `std::env::var` (no new crates needed for autosave).

---

## Feature 1 — Drag-and-Drop Spec Import

**Existing plan:** `docs/superpowers/plans/2026-03-20-drag-drop-spec-import.md`

Execute that plan in full (Task 1 → Task 2 → Task 3). No additional work needed here.

---

## Feature 2 — 4 Missing Templates

### Task 2.1: Write the 4 template spec files

**Files:**
- Create: `src/templates/org/team-topology.spec`
- Create: `src/templates/org/raci-matrix.spec`
- Create: `src/templates/ops/runbook.spec`
- Create: `src/templates/ops/on-call-tree.spec`

- [ ] **Step 1: Write `src/templates/org/team-topology.spec`**

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

- [ ] **Step 2: Write `src/templates/org/raci-matrix.spec`**

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

- [ ] **Step 3: Write `src/templates/ops/runbook.spec`**

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

- [ ] **Step 4: Write `src/templates/ops/on-call-tree.spec`**

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

### Task 2.2: Register templates and verify

**Files:**
- Modify: `src/templates/mod.rs` (after line 57, before the `];`)

- [ ] **Step 1: Add four entries to `TEMPLATES` in `src/templates/mod.rs`**

Add after the `Incident Map` entry (currently at line 52-57), before the closing `];`:

```rust
    Template {
        name: "Team Topology",
        category: "Org",
        description: "Stream-aligned, platform, enabling, and subsystem teams",
        content: include_str!("org/team-topology.spec"),
    },
    Template {
        name: "RACI Matrix",
        category: "Org",
        description: "Responsibility assignment matrix",
        content: include_str!("org/raci-matrix.spec"),
    },
    Template {
        name: "Runbook",
        category: "Ops",
        description: "Operational incident runbook with severity triage",
        content: include_str!("ops/runbook.spec"),
    },
    Template {
        name: "On-Call Tree",
        category: "Ops",
        description: "On-call escalation tree",
        content: include_str!("ops/on-call-tree.spec"),
    },
```

- [ ] **Step 2: Run the template parse test**

```bash
cd /Users/joe888777/Desktop/project/experiment/light-figma
cargo test test_all_templates_parse 2>&1
```

Expected: all 12 templates pass (8 existing + 4 new).

- [ ] **Step 3: Run full test suite**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/templates/org/team-topology.spec src/templates/org/raci-matrix.spec src/templates/ops/runbook.spec src/templates/ops/on-call-tree.spec src/templates/mod.rs
git commit -m "feat: add team-topology, raci-matrix, runbook, on-call-tree templates"
```

---

## Feature 3 — General Language Extensions

**Existing plan:** `docs/superpowers/plans/2026-03-19-general-diagramming-language.md`

Execute that plan in full (all phases and tasks). No additional work needed here.

---

## Feature 4 — CLI `open` Subcommand

### Task 4.1: Add positional `file` arg to `Cli` and implement `new_with_file`

**Files:**
- Modify: `src/main.rs` (lines 13-18 `Cli` struct; line 107 `None` arm)
- Modify: `src/app/mod.rs` (after line 394 — add `new_with_file` method)

- [ ] **Step 1: Add `file: Option<PathBuf>` to `Cli` struct in `src/main.rs`**

Replace the `Cli` struct (lines 13-18):

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

- [ ] **Step 2: Wire `cli.file` into the GUI `None` arm in `src/main.rs`**

Replace the `None => {}` arm (line 107) and the `eframe::run_native` call (lines 117-121):

```rust
        None => {
            // GUI mode
            let startup_file = cli.file.clone();
            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([1400.0, 860.0])
                    .with_title("Light Figma"),
                ..Default::default()
            };
            return eframe::run_native(
                "Light Figma",
                options,
                Box::new(move |cc| Ok(Box::new(app::FlowchartApp::new_with_file(cc, startup_file)))),
            );
        }
```

Also remove the old `let options = ...` block and old `eframe::run_native` call that followed the `match`.

- [ ] **Step 3: Add `new_with_file` method to `src/app/mod.rs` after line 394**

Insert after the closing `}` of `new()` at line 394 (before `section_for_canvas_x`):

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

- [ ] **Step 4: Build to confirm it compiles**

```bash
cd /Users/joe888777/Desktop/project/experiment/light-figma
cargo build 2>&1 | grep "^error" | head -20
```

Expected: clean build, no errors.

- [ ] **Step 5: Run full test suite**

```bash
cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/app/mod.rs
git commit -m "feat: CLI open subcommand — positional file arg opens GUI with spec pre-loaded"
```

---

## Feature 5 — Autosave

### Task 5.1: Add autosave fields and `autosave_path()` free function

**Files:**
- Modify: `src/app/mod.rs`

- [ ] **Step 1: Add 6 autosave fields to `FlowchartApp` struct**

Add immediately before the closing `}` of the struct (currently at line 271, before `BgPattern`):

```rust
    /// Tracks whether the document has changed since the last autosave
    pub(crate) autosave_dirty: bool,
    /// Instant of the last successful autosave
    pub(crate) autosave_last_time: std::time::Instant,
    /// Path of the autosave recovery file
    pub(crate) autosave_path: std::path::PathBuf,
    /// When true, show the restore-from-autosave prompt on first frame
    pub(crate) show_restore_prompt: bool,
    /// Status string shown in statusbar: e.g. "Autosaved 14:32"
    pub(crate) autosave_status: Option<String>,
```

- [ ] **Step 2: Initialize the 5 new fields in `new()` (near line 390, just before the closing `}` of the struct literal)**

Add before `show_template_gallery: false,` (line 392):

```rust
            autosave_dirty: false,
            autosave_last_time: std::time::Instant::now(),
            autosave_path: {
                let p = autosave_path();
                // Check for a recent autosave on launch and set the prompt flag below
                p
            },
            show_restore_prompt: false,
            autosave_status: None,
```

Then, after the `Self { ... }` literal is constructed (but before the closing `}` of `new()`), add the restore-prompt check:

```rust
        // Check for recent autosave on launch
        let p = autosave_path();
        if p.exists() {
            if let Ok(meta) = std::fs::metadata(&p) {
                if let Ok(modified) = meta.modified() {
                    if let Ok(age) = modified.elapsed() {
                        if age.as_secs() < 600 {
                            // autosave is < 10 minutes old
                            // We need a mutable reference, so set the flag after construction
                        }
                    }
                }
            }
        }
```

Wait — the restore prompt check needs a mutable `self`. The cleanest approach is to do it inside `new_with_file` or set it in the `Self { ... }` literal inline. Use the inline closure approach in the struct literal already shown in step 1.

**Revised Step 2** — replace the `autosave_path` and `show_restore_prompt` init lines:

```rust
            autosave_dirty: false,
            autosave_last_time: std::time::Instant::now(),
            autosave_path: autosave_path(),
            show_restore_prompt: {
                let p = autosave_path();
                if p.exists() {
                    std::fs::metadata(&p).ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.elapsed().ok())
                        .map(|age| age.as_secs() < 600)
                        .unwrap_or(false)
                } else {
                    false
                }
            },
            autosave_status: None,
```

- [ ] **Step 3: Add the `autosave_path()` free function after the closing `}` of the impl block (after line 635)**

```rust
/// Returns the platform-specific path for the autosave recovery file.
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

- [ ] **Step 4: Build to confirm it compiles**

```bash
cd /Users/joe888777/Desktop/project/experiment/light-figma
cargo build 2>&1 | grep "^error" | head -20
```

Expected: clean build.

### Task 5.2: Implement `do_autosave()` and mark dirty at mutation sites

**Files:**
- Modify: `src/app/mod.rs`

- [ ] **Step 1: Add `do_autosave()` method inside `impl FlowchartApp`**

Add just before the closing `}` of `impl FlowchartApp` (currently at line 635):

```rust
    fn do_autosave(&mut self) {
        if let Some(dir) = self.autosave_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match serde_json::to_string(&self.document) {
            Ok(json) => {
                if std::fs::write(&self.autosave_path, &json).is_ok() {
                    // Format "HH:MM" for the statusbar
                    let now = chrono::Local::now();
                    self.autosave_status = Some(format!("Autosaved {:02}:{:02}", now.hour(), now.minute()));
                    self.autosave_dirty = false;
                    self.autosave_last_time = std::time::Instant::now();
                }
            }
            Err(_) => {}
        }
    }
```

Note: `chrono` is not in the project. Use a simpler time approach instead:

```rust
    fn do_autosave(&mut self) {
        if let Some(dir) = self.autosave_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string(&self.document) {
            if std::fs::write(&self.autosave_path, &json).is_ok() {
                // Use SystemTime for a simple timestamp string
                use std::time::{SystemTime, UNIX_EPOCH};
                let secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let hour = (secs % 86400) / 3600;
                let min = (secs % 3600) / 60;
                self.autosave_status = Some(format!("Autosaved {:02}:{:02}", hour, min));
                self.autosave_dirty = false;
                self.autosave_last_time = std::time::Instant::now();
            }
        }
    }
```

- [ ] **Step 2: Set `autosave_dirty = true` at line 549 (template load mutation)**

In `update()`, the template load `history.push` at line 549 is followed by `self.document = doc`. Add `self.autosave_dirty = true;` after line 549:

```rust
                        self.history.push(&self.document);
                        self.autosave_dirty = true;
                        self.document = doc;
```

- [ ] **Step 3: Set `autosave_dirty = true` at line 561 (empty canvas mutation)**

After line 561:

```rust
                self.history.push(&self.document);
                self.autosave_dirty = true;
                self.document = crate::model::FlowchartDocument::default();
```

- [ ] **Step 4: Build to confirm it compiles**

```bash
cd /Users/joe888777/Desktop/project/experiment/light-figma
cargo build 2>&1 | grep "^error" | head -20
```

Expected: clean build.

### Task 5.3: Restore prompt, autosave timer, and statusbar display

**Files:**
- Modify: `src/app/mod.rs` (restore prompt window + autosave timer in `update()`)
- Modify: `src/app/statusbar.rs` (show autosave_status on the right)

- [ ] **Step 1: Add restore prompt window in `update()` (add after line 566 — after the template gallery handler block)**

```rust
        // Restore-from-autosave prompt (shown once at launch if recent autosave exists)
        if self.show_restore_prompt {
            let mut keep = true;
            egui::Window::new("Restore Previous Session?")
                .id(egui::Id::new("autosave_restore"))
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("An autosave from less than 10 minutes ago was found.");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Restore").clicked() {
                            if let Ok(json) = std::fs::read_to_string(&self.autosave_path.clone()) {
                                if let Ok(doc) = serde_json::from_str::<crate::model::FlowchartDocument>(&json) {
                                    self.document = doc;
                                    self.pending_fit = true;
                                    self.status_message = Some(("Restored from autosave".to_string(), std::time::Instant::now()));
                                }
                            }
                            keep = false;
                        }
                        if ui.button("Discard").clicked() {
                            let _ = std::fs::remove_file(&self.autosave_path.clone());
                            keep = false;
                        }
                    });
                });
            self.show_restore_prompt = keep;
        }
```

- [ ] **Step 2: Add autosave timer check in `update()` before the closing `}` at line 634**

After the spec editor debounce block (after line 633), before the closing `}`:

```rust
        // Autosave: write recovery file if dirty and 30 seconds have elapsed
        if self.autosave_dirty && self.autosave_last_time.elapsed().as_secs() >= 30 {
            self.do_autosave();
        }
```

- [ ] **Step 3: Display autosave status in `src/app/statusbar.rs`**

Find the right-side area of the statusbar (grep for `right_to_left` or the last label in the statusbar). Add display of `self.autosave_status` on the right side. In `draw_statusbar()`, before the final closing `}`:

Look for a pattern like `ui.with_layout(egui::Layout::right_to_left(...), |ui| {` — add inside that block:

```rust
if let Some(ref msg) = self.autosave_status {
    ui.label(
        egui::RichText::new(msg)
            .small()
            .color(egui::Color32::from_gray(140)),
    );
}
```

- [ ] **Step 4: Build and run full test suite**

```bash
cd /Users/joe888777/Desktop/project/experiment/light-figma
cargo build 2>&1 | grep "^error" | head -20
cargo test 2>&1 | tail -10
```

Expected: clean build, all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/app/mod.rs src/app/statusbar.rs
git commit -m "feat: autosave — 30-second recovery writes and restore-on-launch prompt"
```

---

## Feature 6 — Presentation Mode Slide Navigation

### Task 6.1: Add new fields to `FlowchartApp` struct

**Files:**
- Modify: `src/app/mod.rs`

- [ ] **Step 1: Add 3 presentation fields to the struct**

Add immediately after the `presentation_mode: bool` field (after line 165):

```rust
    /// Index into presentation_slides of the currently displayed slide
    pub(crate) presentation_slide_index: usize,
    /// Indices into document.nodes for all frame nodes, sorted top-to-bottom left-to-right
    pub(crate) presentation_slides: Vec<usize>,
    /// When Some(idx), canvas.rs will fit the viewport to document.nodes[idx] after layout
    pub(crate) pending_fit_to_node: Option<usize>,
```

- [ ] **Step 2: Initialize the 3 new fields in `new()` (in the struct literal)**

Add before `autosave_dirty: false,` (or near any other simple field):

```rust
            presentation_slide_index: 0,
            presentation_slides: Vec::new(),
            pending_fit_to_node: None,
```

- [ ] **Step 3: Build to confirm it compiles**

```bash
cd /Users/joe888777/Desktop/project/experiment/light-figma
cargo build 2>&1 | grep "^error" | head -20
```

Expected: clean build.

### Task 6.2: Add presentation mode methods to `impl FlowchartApp`

**Files:**
- Modify: `src/app/mod.rs`

- [ ] **Step 1: Add `enter_presentation_mode()` method**

Add inside `impl FlowchartApp`, before the closing `}`:

```rust
    pub(crate) fn enter_presentation_mode(&mut self) {
        // Collect frame node indices sorted top-to-bottom, left-to-right
        let mut slides: Vec<usize> = self.document.nodes.iter()
            .enumerate()
            .filter(|(_, n)| n.is_frame)
            .map(|(i, _)| i)
            .collect();
        slides.sort_by(|&a, &b| {
            let na = &self.document.nodes[a];
            let nb = &self.document.nodes[b];
            na.position[1].partial_cmp(&nb.position[1])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(na.position[0].partial_cmp(&nb.position[0])
                    .unwrap_or(std::cmp::Ordering::Equal))
        });
        if slides.is_empty() {
            // No frames: use sentinel, fit all nodes
            self.presentation_slides = vec![usize::MAX];
        } else {
            self.presentation_slides = slides;
        }
        self.presentation_slide_index = 0;
        self.presentation_mode = true;
        self.fit_to_frame(0);
        self.status_message = Some(("Presentation mode".to_string(), std::time::Instant::now()));
    }
```

- [ ] **Step 2: Add `exit_presentation_mode()` method**

```rust
    pub(crate) fn exit_presentation_mode(&mut self) {
        self.presentation_mode = false;
        self.presentation_slides.clear();
        self.pending_fit = true;
        self.status_message = Some(("Presentation mode off".to_string(), std::time::Instant::now()));
    }
```

- [ ] **Step 3: Add `presentation_next_slide()` and `presentation_prev_slide()` methods**

```rust
    pub(crate) fn presentation_next_slide(&mut self) {
        if self.presentation_slides.is_empty() { return; }
        let max = self.presentation_slides.len() - 1;
        if self.presentation_slide_index < max {
            self.presentation_slide_index += 1;
            self.fit_to_frame(self.presentation_slide_index);
        }
    }

    pub(crate) fn presentation_prev_slide(&mut self) {
        if self.presentation_slides.is_empty() { return; }
        if self.presentation_slide_index > 0 {
            self.presentation_slide_index -= 1;
            self.fit_to_frame(self.presentation_slide_index);
        }
    }
```

- [ ] **Step 4: Add `fit_to_frame()` method**

```rust
    fn fit_to_frame(&mut self, slide_pos: usize) {
        if slide_pos < self.presentation_slides.len() {
            let node_idx = self.presentation_slides[slide_pos];
            if node_idx == usize::MAX {
                self.pending_fit = true;
            } else {
                self.pending_fit_to_node = Some(node_idx);
            }
        }
    }
```

- [ ] **Step 5: Build to confirm it compiles**

```bash
cd /Users/joe888777/Desktop/project/experiment/light-figma
cargo build 2>&1 | grep "^error" | head -20
```

Expected: clean build.

### Task 6.3: Update shortcuts.rs — replace Key::F toggle

**Files:**
- Modify: `src/app/shortcuts.rs`

- [ ] **Step 1: Find and replace the `Key::F` presentation_mode toggle**

Find the block at line ~1352 (toggles `presentation_mode` with a toast). Replace the simple toggle with calls to the new methods:

Find:
```rust
Key::F => {
    self.presentation_mode = !self.presentation_mode;
    // (some toast message)
}
```

Replace with:
```rust
Key::F | Key::F5 => {
    if self.presentation_mode {
        self.exit_presentation_mode();
    } else {
        self.enter_presentation_mode();
    }
}
```

Note: Check whether F5 is `Key::F5` in egui — it is available as `egui::Key::F5`. If it requires a separate arm, add it separately.

- [ ] **Step 2: Add arrow key + ESC navigation guard before the nudge block in shortcuts.rs**

Find the node-nudge block (arrow keys to move selected nodes, around line ~1833). Add the following guard BEFORE it:

```rust
        // Presentation mode: arrow keys navigate slides, ESC exits
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
            return; // skip normal shortcut processing in presentation mode
        }
```

- [ ] **Step 3: Build to confirm it compiles**

```bash
cd /Users/joe888777/Desktop/project/experiment/light-figma
cargo build 2>&1 | grep "^error" | head -20
```

Expected: clean build.

### Task 6.4: Add "Present" button to toolbar.rs

**Files:**
- Modify: `src/app/toolbar.rs`

- [ ] **Step 1: Add a "Present" button in the toolbar**

Find the toolbar's top row (near the View 2D/3D toggle buttons). Add a "Present" button:

```rust
if ui.button("Present").on_hover_text("Enter presentation mode (F / F5)").clicked() {
    self.enter_presentation_mode();
}
```

Add it in the same horizontal group as other view-control buttons.

- [ ] **Step 2: Build to confirm it compiles**

```bash
cd /Users/joe888777/Desktop/project/experiment/light-figma
cargo build 2>&1 | grep "^error" | head -20
```

### Task 6.5: Consume `pending_fit_to_node` and add slide HUD in canvas.rs

**Files:**
- Modify: `src/app/canvas.rs`

- [ ] **Step 1: Consume `pending_fit_to_node` after `canvas_rect` is known**

In `draw_canvas()`, after `canvas_rect` is established (look for where `let canvas_rect = ui.max_rect()` or similar), add:

```rust
        if let Some(idx) = self.pending_fit_to_node.take() {
            if let Some(node) = self.document.nodes.get(idx) {
                let padding = 40.0;
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

- [ ] **Step 2: Add slide-counter HUD overlay in `draw_canvas()`**

In `draw_canvas()`, after drawing nodes/edges (near where the presentation badge is currently drawn in `mod.rs` at lines 583-598 — note: canvas.rs may draw additional overlays). Add a slide counter HUD using an `egui::Area`:

```rust
        // Presentation mode slide-counter HUD
        if self.presentation_mode
            && !self.presentation_slides.is_empty()
            && self.presentation_slides[0] != usize::MAX
        {
            let total = self.presentation_slides.len();
            let current = self.presentation_slide_index + 1;
            egui::Area::new(egui::Id::new("presentation_hud"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -20.0])
                .show(ui.ctx(), |ui| {
                    let pill = format!("Slide {current} / {total}");
                    ui.label(
                        egui::RichText::new(pill)
                            .size(13.0)
                            .color(egui::Color32::from_gray(200)),
                    );
                });
            egui::Area::new(egui::Id::new("presentation_hud_hint"))
                .order(egui::Order::Foreground)
                .anchor(egui::Align2::RIGHT_BOTTOM, [-12.0, -20.0])
                .show(ui.ctx(), |ui| {
                    ui.label(
                        egui::RichText::new("ESC to exit")
                            .size(11.0)
                            .color(egui::Color32::from_gray(140)),
                    );
                });
        }
```

- [ ] **Step 3: Build and run full test suite**

```bash
cd /Users/joe888777/Desktop/project/experiment/light-figma
cargo build 2>&1 | grep "^error" | head -20
cargo test 2>&1 | tail -10
```

Expected: clean build, all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/app/mod.rs src/app/shortcuts.rs src/app/toolbar.rs src/app/canvas.rs
git commit -m "feat: presentation mode — frame-based slide navigation with F5, arrow keys, HUD overlay"
```

---

## Implementation Checklist

### Feature 1 — Drag-Drop Import
- [ ] Execute `docs/superpowers/plans/2026-03-20-drag-drop-spec-import.md` (Task 1 → Task 2 → Task 3)

### Feature 2 — Templates
- [ ] `src/templates/org/team-topology.spec`
- [ ] `src/templates/org/raci-matrix.spec`
- [ ] `src/templates/ops/runbook.spec`
- [ ] `src/templates/ops/on-call-tree.spec`
- [ ] Register all 4 in `src/templates/mod.rs`
- [ ] `test_all_templates_parse` passes (12 templates)

### Feature 3 — General Language Extensions
- [ ] Execute `docs/superpowers/plans/2026-03-19-general-diagramming-language.md` (all phases)

### Feature 4 — CLI Open
- [ ] `file: Option<PathBuf>` in `Cli` struct (`src/main.rs`)
- [ ] `new_with_file()` method in `src/app/mod.rs`
- [ ] Wire in `None` arm of `main.rs`

### Feature 5 — Autosave
- [ ] Add 5 autosave fields to struct + initialize in `new()`
- [ ] `autosave_path()` free function after impl block
- [ ] `do_autosave()` method
- [ ] `autosave_dirty = true` at lines 549 and 561
- [ ] Restore prompt window in `update()`
- [ ] Autosave timer check before end of `update()`
- [ ] Autosave status in `statusbar.rs`

### Feature 6 — Presentation Mode
- [ ] 3 new fields (`presentation_slide_index`, `presentation_slides`, `pending_fit_to_node`)
- [ ] `enter_presentation_mode()`, `exit_presentation_mode()`, `presentation_next_slide()`, `presentation_prev_slide()`, `fit_to_frame()` methods
- [ ] `Key::F`/`Key::F5` → `enter/exit_presentation_mode()` in `shortcuts.rs`
- [ ] Arrow key + ESC navigation guard (before nudge block) in `shortcuts.rs`
- [ ] "Present" button in `toolbar.rs`
- [ ] `pending_fit_to_node` consumer in `canvas.rs`
- [ ] Slide-counter HUD in `canvas.rs`
