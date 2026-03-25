# File Save + Recent Files Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add native file save (Cmd+S / Cmd+Shift+S) and persistent recent-files tracking surfaced in the template gallery and command palette.

**Architecture:** Four files change — `mod.rs` gains two new fields, three free functions, and two methods; `shortcuts.rs` rebinds Cmd+Shift+S→Y and adds two save handlers; `template_gallery.rs` gains a `GallerySelection` enum and a "Recent" section; `command_palette.rs` gains `Cow`-based labels, `OpenRecentFile` action, and two-list ranking. No new crates.

**Tech Stack:** Rust, egui/eframe, `rfd::FileDialog` (already in `Cargo.toml`), `serde_json` (already in `Cargo.toml`), `std::borrow::Cow`.

**Spec:** `docs/superpowers/specs/2026-03-24-file-save-recent-files-design.md`

---

## Files Changed

| File | What changes |
|------|-------------|
| `src/app/mod.rs` | 2 new struct fields; init in `new()`; `new_with_file()` Ok arm update; `push_recent()`, `save_to_path()`, `compute_window_title()` methods/fns; `recent_files_path()`, `load_recent_files()`, `save_recent_files()` free fns; replace title block at lines 783–791; tests |
| `src/app/shortcuts.rs` | Rebind clipboard-copy from Cmd+Shift+S → Cmd+Shift+Y (change one line); add Cmd+S and Cmd+Shift+S handlers |
| `src/app/template_gallery.rs` | `GallerySelection` enum (pub crate); change return type; add "Recent" section with deferred removal |
| `src/app/command_palette.rs` | `PaletteEntry.label` → `Cow<'static, str>`; `PaletteAction` drops `Copy`, gains `OpenRecentFile(usize)`; fix `.action` copy sites; build `recent_entries` before `Window::show`; two-list ranking; `OpenRecentFile` dispatch arm |

---

## Task 1: Persistence helpers, struct fields, and `push_recent`

**Files:**
- Modify: `src/app/mod.rs`

Before writing any code: confirm `cargo test --tests` passes (102 tests expected).

### Step 1.1 — Write the failing tests

In `src/app/mod.rs`, find the `#[cfg(test)]` module (starts after line ~993). Add these tests **at the end of the existing test module**, before the closing `}`:

```rust
    #[test]
    fn test_push_recent_deduplicates() {
        let mut recent: Vec<std::path::PathBuf> = Vec::new();
        let p = std::path::PathBuf::from("/tmp/foo.spec");
        super::push_recent_list(&mut recent, p.clone());
        super::push_recent_list(&mut recent, p.clone());
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0], p);
    }

    #[test]
    fn test_push_recent_prepends() {
        let mut recent: Vec<std::path::PathBuf> = Vec::new();
        let a = std::path::PathBuf::from("/tmp/a.spec");
        let b = std::path::PathBuf::from("/tmp/b.spec");
        super::push_recent_list(&mut recent, a.clone());
        super::push_recent_list(&mut recent, b.clone());
        assert_eq!(recent[0], b); // newest first
        assert_eq!(recent[1], a);
    }

    #[test]
    fn test_push_recent_caps_at_10() {
        let mut recent: Vec<std::path::PathBuf> = Vec::new();
        for i in 0..12u32 {
            super::push_recent_list(&mut recent, std::path::PathBuf::from(format!("/tmp/{i}.spec")));
        }
        assert_eq!(recent.len(), 10);
    }

    #[test]
    fn test_recent_files_path_contains_app_dir() {
        let p = super::recent_files_path();
        let s = p.to_string_lossy();
        assert!(s.contains("light-figma"), "expected 'light-figma' in {s}");
        assert!(s.ends_with("recent-files.json"), "expected 'recent-files.json' suffix in {s}");
    }
```

### Step 1.2 — Run tests to confirm they fail

```bash
cd /Users/joe888777/Desktop/project/experiment/openAtlas
cargo test --tests push_recent recent_files_path 2>&1 | tail -15
```

Expected: `error[E0425]: cannot find function 'push_recent_list'` (or similar). Good — the functions don't exist yet.

### Step 1.3 — Add two struct fields to `FlowchartApp`

In `src/app/mod.rs`, after line 286 (the `autosave_status` field line), **before the closing `}`** of the struct, add:

```rust
    /// Path of the currently open file; None if unsaved.
    pub(crate) current_file_path: Option<std::path::PathBuf>,
    /// Recently opened/saved files, newest first. Max 10 entries. Persisted.
    pub(crate) recent_files: Vec<std::path::PathBuf>,
```

### Step 1.4 — Initialize the two fields in `new()`

In `src/app/mod.rs`, after line 424 (`autosave_status: None,`), **before the closing `}` of the `Self { ... }` literal**, add:

```rust
            current_file_path: None,
            recent_files: load_recent_files(),
```

### Step 1.5 — Add `recent_files_path`, `load_recent_files`, `save_recent_files` free functions

In `src/app/mod.rs`, after the closing `}` of `autosave_path()` (currently around line 896), add:

```rust
/// Returns the platform-specific path for the recent-files list.
fn recent_files_path() -> std::path::PathBuf {
    autosave_path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("recent-files.json")
}

/// Loads recent files from disk. Returns empty vec on any error.
fn load_recent_files() -> Vec<std::path::PathBuf> {
    let path = recent_files_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<std::path::PathBuf>>(&s).ok())
        .unwrap_or_default()
}

/// Serializes recent files to disk. Silently ignores all errors.
fn save_recent_files(files: &[std::path::PathBuf]) {
    if let Ok(json) = serde_json::to_string(files) {
        let path = recent_files_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, json);
    }
}

/// Pure push logic extracted for unit-testing without constructing FlowchartApp.
fn push_recent_list(recent: &mut Vec<std::path::PathBuf>, path: std::path::PathBuf) {
    recent.retain(|p| p != &path);
    recent.insert(0, path);
    recent.truncate(10);
}
```

### Step 1.6 — Add `push_recent` method to `impl FlowchartApp`

Inside `impl FlowchartApp`, before the closing `}` of the impl block (currently around line 875), add:

```rust
    /// Prepend path to recent_files, deduplicating and capping at 10.
    /// Does NOT call save_recent_files — callers persist after calling this.
    pub(crate) fn push_recent(&mut self, path: std::path::PathBuf) {
        push_recent_list(&mut self.recent_files, path);
    }
```

### Step 1.7 — Build to confirm it compiles

```bash
cd /Users/joe888777/Desktop/project/experiment/openAtlas
cargo build 2>&1 | grep "^error" | head -20
```

Expected: no errors (there may be `unused` warnings — ignore them for now).

### Step 1.8 — Run tests to confirm new tests pass

```bash
cargo test --tests push_recent recent_files_path 2>&1 | tail -15
```

Expected: 4 new tests pass.

### Step 1.9 — Run full test suite to confirm no regressions

```bash
cargo test --tests 2>&1 | tail -5
```

Expected: `test result: ok. 106 passed` (102 + 4 new).

### Step 1.10 — Commit

```bash
git add src/app/mod.rs
git commit -m "feat: add current_file_path, recent_files state + persistence helpers + push_recent"
```

---

## Task 2: `save_to_path`, `compute_window_title`, `new_with_file` update, title block

**Files:**
- Modify: `src/app/mod.rs`

### Step 2.1 — Write failing tests

In `src/app/mod.rs`, add these tests at the end of the `#[cfg(test)]` module (after the tests added in Task 1):

```rust
    #[test]
    fn test_compute_window_title_no_file_no_nodes() {
        let t = super::compute_window_title(None, false, 0, 0);
        assert_eq!(t, "Light Figma");
    }

    #[test]
    fn test_compute_window_title_no_file_with_nodes() {
        let t = super::compute_window_title(None, false, 3, 2);
        assert_eq!(t, "Light Figma — 3N 2E");
    }

    #[test]
    fn test_compute_window_title_with_file_no_nodes() {
        let t = super::compute_window_title(Some("arch.spec"), false, 0, 0);
        assert_eq!(t, "arch.spec");
    }

    #[test]
    fn test_compute_window_title_with_file_dirty() {
        let t = super::compute_window_title(Some("arch.spec"), true, 5, 3);
        assert_eq!(t, "arch.spec• — 5N 3E");
    }

    #[test]
    fn test_save_to_path_round_trip() {
        use crate::specgraph::hrf::{parse_hrf, export_hrf_ex};
        let hrf = "## Nodes\n- [a] Alpha\n- [b] Beta\n\n## Flow\na --> b\n";
        let mut doc = parse_hrf(hrf).expect("parse");
        crate::specgraph::layout::auto_layout(&mut doc);
        let orig_nodes = doc.nodes.len();
        let orig_edges = doc.edges.len();

        let path = std::env::temp_dir().join("lf_test_save.spec");
        let content = export_hrf_ex(&doc, "Test", None);
        std::fs::write(&path, &content).expect("write");

        let read_back = std::fs::read_to_string(&path).expect("read");
        let doc2 = parse_hrf(&read_back).expect("re-parse");
        assert_eq!(doc2.nodes.len(), orig_nodes);
        assert_eq!(doc2.edges.len(), orig_edges);

        let _ = std::fs::remove_file(&path);
    }
```

### Step 2.2 — Run tests to confirm they fail

```bash
cargo test --tests compute_window_title save_to_path_round_trip 2>&1 | tail -10
```

Expected: `error[E0425]: cannot find function 'compute_window_title'`.

### Step 2.3 — Add `compute_window_title` free function

In `src/app/mod.rs`, after the `save_recent_files` and `push_recent_list` functions added in Task 1, add:

```rust
/// Computes the window title string.
/// Extracted as a free function for unit-testability.
fn compute_window_title(filename: Option<&str>, dirty: bool, nodes: usize, edges: usize) -> String {
    let dirty_mark = if dirty { "•" } else { "" };
    match (filename, nodes) {
        (Some(f), 0) => format!("{f}{dirty_mark}"),
        (Some(f), _) => format!("{f}{dirty_mark} — {nodes}N {edges}E"),
        (None,    0) => "Light Figma".to_string(),
        (None,    _) => format!("Light Figma — {nodes}N {edges}E"),
    }
}
```

### Step 2.4 — Replace the title block in `update()`

In `src/app/mod.rs`, **replace lines 783–791** (the entire `// Dynamic window title` comment through `ctx.send_viewport_cmd(...)`) with:

```rust
        // Dynamic window title: filename + optional dirty mark + node/edge count
        let n = self.document.nodes.len();
        let e = self.document.edges.len();
        let filename = self.current_file_path.as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        let dirty = self.autosave_dirty && self.current_file_path.is_some();
        let title = compute_window_title(filename.as_deref(), dirty, n, e);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
```

### Step 2.5 — Add `save_to_path` method inside `impl FlowchartApp`

Add this method before the closing `}` of `impl FlowchartApp` (alongside `push_recent` added in Task 1):

```rust
    /// Saves the current document to `path`. Updates `current_file_path` and `recent_files`.
    /// On success, shows a toast only if the path changed (avoids spam for repeated Cmd+S).
    pub(crate) fn save_to_path(&mut self, path: std::path::PathBuf) {
        let fallback_title = self.current_file_path.as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled Diagram".to_string());
        let hrf = crate::specgraph::hrf::export_hrf_ex(&self.document, &fallback_title, None);
        match std::fs::write(&path, &hrf) {
            Ok(()) => {
                let is_new_path = self.current_file_path.as_ref() != Some(&path);
                self.push_recent(path.clone());
                self.current_file_path = Some(path);
                save_recent_files(&self.recent_files);
                self.autosave_dirty = false;
                if is_new_path {
                    let fname = self.current_file_path.as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    self.status_message = Some((
                        format!("Saved to {fname}"),
                        std::time::Instant::now(),
                    ));
                }
            }
            Err(e) => {
                self.status_message = Some((
                    format!("Save failed: {e}"),
                    std::time::Instant::now(),
                ));
            }
        }
    }
```

### Step 2.6 — Update `new_with_file` Ok arm

In `src/app/mod.rs`, inside `new_with_file`, find the `Ok(mut doc) =>` arm. After the `app.status_message = Some(...)` block (currently the last statement inside the Ok arm, around line 443), **before the closing `}` of the Ok arm**, add:

```rust
                        app.push_recent(path.clone());
                        app.current_file_path = Some(path);
                        save_recent_files(&app.recent_files);
```

The Ok arm after the edit should end:

```rust
                        app.status_message = Some((
                            format!("Opened {name}"),
                            std::time::Instant::now(),
                        ));
                        app.push_recent(path.clone());
                        app.current_file_path = Some(path);
                        save_recent_files(&app.recent_files);
                    }  // ← closes Ok arm
```

### Step 2.7 — Build to confirm it compiles

```bash
cargo build 2>&1 | grep "^error" | head -20
```

Expected: no errors.

### Step 2.8 — Run new tests

```bash
cargo test --tests compute_window_title save_to_path_round_trip 2>&1 | tail -10
```

Expected: 5 new tests pass.

### Step 2.9 — Run full test suite

```bash
cargo test --tests 2>&1 | tail -5
```

Expected: `test result: ok. 111 passed` (106 + 5 new).

### Step 2.10 — Commit

```bash
git add src/app/mod.rs
git commit -m "feat: save_to_path, compute_window_title, new_with_file path tracking"
```

---

## Task 3: Save keyboard shortcuts

**Files:**
- Modify: `src/app/shortcuts.rs`

No new unit tests for this task (keyboard shortcuts are integration-level; the build and existing tests are the gate).

### Step 3.1 — Rebind clipboard-copy from Cmd+Shift+S to Cmd+Shift+Y

In `src/app/shortcuts.rs`, find lines 934–936:

```rust
        // Cmd+Shift+S = copy current diagram as HRF spec to system clipboard
        let cmd_shift_s = Modifiers { shift: true, ..cmd };
        if ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.matches_exact(cmd_shift_s)) {
```

Replace with:

```rust
        // Cmd+Shift+Y = copy current diagram as HRF spec to system clipboard (moved from Cmd+Shift+S)
        let cmd_shift_y = Modifiers { shift: true, ..cmd };
        if ctx.input(|i| i.key_pressed(Key::Y) && i.modifiers.matches_exact(cmd_shift_y)) {
```

(Only the comment line and the two `S` → `Y` renames change; the body of the block is unchanged.)

### Step 3.2 — Add Cmd+S and Cmd+Shift+S save handlers

Immediately **before** the rebinded `Cmd+Shift+Y` block (before the comment line from Step 3.1), add:

```rust
        // Cmd+S = save to current path (or open Save As if no path yet)
        let cmd_s = Modifiers { command: true, ..Default::default() };
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.matches_exact(cmd_s)) {
            if let Some(path) = self.current_file_path.clone() {
                self.save_to_path(path);
            } else {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Spec / YAML", &["spec", "yaml"])
                    .set_file_name("diagram.spec")
                    .save_file()
                {
                    self.save_to_path(path);
                }
            }
        }

        // Cmd+Shift+S = always open Save As dialog
        let cmd_shift_s = Modifiers { shift: true, ..cmd };
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::S) && i.modifiers.matches_exact(cmd_shift_s)) {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Spec / YAML", &["spec", "yaml"])
                .set_file_name("diagram.spec")
                .save_file()
            {
                self.save_to_path(path);
            }
        }

```

Note: `rfd` is already imported — check for an existing `use rfd;` or just use the full path `rfd::FileDialog`. Look at how drag-drop uses rfd in `io.rs` to match the style. The full path `rfd::FileDialog::new()` always works.

### Step 3.3 — Build to confirm it compiles

```bash
cargo build 2>&1 | grep "^error" | head -20
```

Expected: no errors.

### Step 3.4 — Run full test suite

```bash
cargo test --tests 2>&1 | tail -5
```

Expected: `test result: ok. 111 passed` (unchanged count — no new tests in this task).

### Step 3.5 — Commit

```bash
git add src/app/shortcuts.rs
git commit -m "feat: Cmd+S save, Cmd+Shift+S save-as, rebind clipboard-copy to Cmd+Shift+Y"
```

---

## Task 4: Template gallery `GallerySelection` + Recent section

**Files:**
- Modify: `src/app/template_gallery.rs`
- Modify: `src/app/mod.rs` (call site only)

### Step 4.1 — Define `GallerySelection` and update return type

In `src/app/template_gallery.rs`, add the enum **before** the `impl super::FlowchartApp {` line (line 8):

```rust
/// Returned by `draw_template_gallery` to indicate what the user selected.
#[derive(Debug)]
pub(crate) enum GallerySelection {
    Template(String),          // HRF content from a built-in template
    RecentFile(std::path::PathBuf), // path chosen from recents
    EmptyCanvas,               // "New empty canvas" button
}
```

### Step 4.2 — Update `draw_template_gallery` signature and internals

Change the function signature at line 14 from:

```rust
    pub(crate) fn draw_template_gallery(&mut self, ctx: &egui::Context) -> Option<String> {
```

to:

```rust
    pub(crate) fn draw_template_gallery(&mut self, ctx: &egui::Context) -> Option<GallerySelection> {
```

Change the internal `selected_content` variable (line 24) from:

```rust
        let mut selected_content: Option<String> = None;
```

to:

```rust
        let mut selected_content: Option<GallerySelection> = None;
```

Change the "Empty Canvas" button click (line 74) from:

```rust
                                selected_content = Some(String::new());
```

to:

```rust
                                selected_content = Some(GallerySelection::EmptyCanvas);
```

Change each template button click (line 111) from:

```rust
                                            selected_content = Some(template.content.to_string());
```

to:

```rust
                                            selected_content = Some(GallerySelection::Template(template.content.to_string()));
```

### Step 4.3 — Add "Recent" section above the template categories

In `src/app/template_gallery.rs`, inside the `egui::ScrollArea` closure, **before** the `// Blank canvas option` comment (before line 65), add:

```rust
                        // Recent files section — shown only when there are recents
                        if !self.recent_files.is_empty() {
                            ui.heading("Recent");
                            ui.add_space(4.0);
                            let mut to_remove: Vec<std::path::PathBuf> = Vec::new();
                            for path in &self.recent_files {
                                let fname = path.file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                                let full = {
                                    let s = path.to_string_lossy();
                                    if s.len() > 40 { format!("…{}", &s[s.len()-39..]) } else { s.into_owned() }
                                };
                                let exists = path.exists();
                                let label = if exists {
                                    egui::RichText::new(format!("{fname}\n{full}")).size(12.0)
                                } else {
                                    egui::RichText::new(format!("{fname} (not found)\n{full}"))
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(200, 100, 60))
                                };
                                let btn = egui::Button::new(label).min_size([280.0, 44.0].into());
                                if ui.add(btn).clicked() {
                                    if exists {
                                        selected_content = Some(GallerySelection::RecentFile(path.clone()));
                                        keep_open = false;
                                    } else {
                                        to_remove.push(path.clone());
                                    }
                                }
                            }
                            // Deferred removal: avoid mutating recent_files inside the closure
                            if !to_remove.is_empty() {
                                self.recent_files.retain(|p| !to_remove.contains(p));
                                save_recent_files(&self.recent_files);
                            }
                            ui.add_space(16.0);
                        }

```

Note: `save_recent_files` is a free function in `mod.rs` — it's in scope inside `impl FlowchartApp` methods in other files because those files use `use super::...` patterns. If the compiler can't find it, replace `save_recent_files(&self.recent_files)` with a direct call via `crate::app::save_recent_files` — but since `template_gallery.rs` is inside `mod app`, the plain name should resolve. If it doesn't, extract the removal into a separate method call on self (see note below).

**Alternative if `save_recent_files` is not in scope:** Add a helper method to `FlowchartApp` in `mod.rs`:

```rust
    pub(crate) fn remove_recent_and_persist(&mut self, paths: Vec<std::path::PathBuf>) {
        self.recent_files.retain(|p| !paths.contains(p));
        save_recent_files(&self.recent_files);
    }
```

Then in `template_gallery.rs`, replace the last block with:

```rust
                            if !to_remove.is_empty() {
                                self.remove_recent_and_persist(to_remove);
                            }
```

### Step 4.4 — Update the call site in `mod.rs`

In `src/app/mod.rs`, at line 687, change:

```rust
        if let Some(content) = self.draw_template_gallery(ctx) {
            if !content.is_empty() {
                match crate::specgraph::hrf::parse_hrf(&content) {
                    Ok(mut doc) => {
                        crate::specgraph::layout::auto_layout(&mut doc);
                        self.history.push(&self.document);
                        self.autosave_dirty = true;
                        self.document = doc;
                        self.selection.clear();
                        self.pending_fit = true;
                        self.status_message = Some(("Template loaded".to_string(), std::time::Instant::now()));
                    }
                    Err(e) => {
                        self.status_message = Some((format!("Template error: {}", e), std::time::Instant::now()));
                    }
                }
            } else {
                // Empty canvas
                self.history.push(&self.document);
                self.autosave_dirty = true;
                self.document = crate::model::FlowchartDocument::default();
                self.selection.clear();
                self.status_message = Some(("New empty canvas".to_string(), std::time::Instant::now()));
            }
        }
```

with:

```rust
        use crate::app::template_gallery::GallerySelection;
        if let Some(selection) = self.draw_template_gallery(ctx) {
            match selection {
                GallerySelection::Template(content) => {
                    match crate::specgraph::hrf::parse_hrf(&content) {
                        Ok(mut doc) => {
                            crate::specgraph::layout::auto_layout(&mut doc);
                            self.history.push(&self.document);
                            self.autosave_dirty = true;
                            self.document = doc;
                            self.selection.clear();
                            self.pending_fit = true;
                            self.status_message = Some(("Template loaded".to_string(), std::time::Instant::now()));
                        }
                        Err(e) => {
                            self.status_message = Some((format!("Template error: {}", e), std::time::Instant::now()));
                        }
                    }
                }
                GallerySelection::EmptyCanvas => {
                    self.history.push(&self.document);
                    self.autosave_dirty = true;
                    self.document = crate::model::FlowchartDocument::default();
                    self.selection.clear();
                    self.status_message = Some(("New empty canvas".to_string(), std::time::Instant::now()));
                }
                GallerySelection::RecentFile(path) => {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => match crate::specgraph::hrf::parse_hrf(&content) {
                            Ok(mut doc) => {
                                crate::specgraph::layout::auto_layout(&mut doc);
                                self.history.push(&self.document);
                                self.autosave_dirty = false;
                                self.document = doc;
                                self.selection.clear();
                                self.pending_fit = true;
                                let fname = path.file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                self.push_recent(path.clone());
                                self.current_file_path = Some(path);
                                save_recent_files(&self.recent_files);
                                self.status_message = Some((format!("Opened {fname}"), std::time::Instant::now()));
                            }
                            Err(e) => {
                                self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                            }
                        },
                        Err(_) => {
                            let fname = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                            self.status_message = Some((format!("File not found: {fname}"), std::time::Instant::now()));
                            self.recent_files.retain(|p| p != &path);
                            save_recent_files(&self.recent_files);
                        }
                    }
                }
            }
        }
```

Note: The `use crate::app::template_gallery::GallerySelection;` can instead go at the top of `mod.rs` (with the other `use` imports) rather than inline. Either works; inline is fine since it's a one-time use.

### Step 4.5 — Build to confirm it compiles

```bash
cargo build 2>&1 | grep "^error" | head -20
```

Expected: no errors.

### Step 4.6 — Run full test suite

```bash
cargo test --tests 2>&1 | tail -5
```

Expected: `test result: ok. 111 passed` (no regression, no new unit tests in this task).

### Step 4.7 — Commit

```bash
git add src/app/template_gallery.rs src/app/mod.rs
git commit -m "feat: GallerySelection enum, recent files section in template gallery"
```

---

## Task 5: Command palette recent files

**Files:**
- Modify: `src/app/command_palette.rs`

### Step 5.1 — Add `use std::borrow::Cow` and update `PaletteEntry.label`

At the top of `src/app/command_palette.rs` (after line 3, the existing `use egui::{...}` line), add:

```rust
use std::borrow::Cow;
```

Change the `PaletteEntry` struct (lines 8–13) from:

```rust
struct PaletteEntry {
    icon:     &'static str,
    label:    &'static str,
    category: &'static str,
    action:   PaletteAction,
}
```

to:

```rust
struct PaletteEntry {
    icon:     &'static str,
    label:    Cow<'static, str>,
    category: &'static str,
    action:   PaletteAction,
}
```

### Step 5.2 — Remove `Copy` from `PaletteAction` and add `OpenRecentFile` variant

Change line 15 from:

```rust
#[derive(Clone, Copy)]
enum PaletteAction {
```

to:

```rust
#[derive(Clone)]
enum PaletteAction {
```

Add `OpenRecentFile(usize),` as the last variant in the `PaletteAction` enum, before the closing `}`.

### Step 5.3 — Fix the two `.action` copy sites

In `src/app/command_palette.rs`, find the two `execute_action = Some(entry.action)` lines:
- Line ~140 (inside the Enter-key handler)
- Line ~237 (inside the click handler)

Change both from `entry.action` to `entry.action.clone()`.

### Step 5.4 — Update all `label:` literals in `build_entries()` to `.into()`

`build_entries()` starts at line 955. Every `label: "some text"` field in that function needs to become `label: "some text".into()`.

This is a mechanical search-and-replace inside the function. Run:

```bash
grep -n 'label:.*"' src/app/command_palette.rs | grep -v '//' | head -5
```

to confirm the pattern. The change for each line is: `label: "text"` → `label: "text".into()`.

Also update the rendering at the line that calls `RichText::new(entry.label)` (around line 228):

Change:
```rust
                                        ui.label(
                                            RichText::new(entry.label)
```

to:

```rust
                                        ui.label(
                                            RichText::new(entry.label.as_ref())
```

### Step 5.5 — Build to confirm it compiles

```bash
cargo build 2>&1 | grep "^error" | head -20
```

If there are errors about `Copy` or `Clone`, track down any remaining `entry.action` copies. If there are errors about `RichText::new`, check what type is being passed and adjust (e.g., `.to_string()` instead of `.as_ref()`).

### Step 5.6 — Inject `recent_entries` and replace the match/filter block

In `src/app/command_palette.rs`, find lines 116–124 (the `let entries = build_entries();` through the closing `};` of `matches`). Replace the entire block with:

```rust
        let recent_entries: Vec<PaletteEntry> = self.recent_files
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let filename = path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                PaletteEntry {
                    icon:     "📄",
                    label:    filename.into(),
                    category: "Recent",
                    action:   PaletteAction::OpenRecentFile(i),
                }
            })
            .collect();
        let entries = build_entries();
        let q = self.command_palette_query.to_lowercase();
        let recent_limit = if q.is_empty() { 5 } else { recent_entries.len() };
        let recent_matches: Vec<&PaletteEntry> = recent_entries.iter()
            .filter(|e| q.is_empty() || e.label.to_lowercase().contains(&q))
            .take(recent_limit)
            .collect();
        let builtin_matches: Vec<&PaletteEntry> = entries.iter()
            .filter(|e| q.is_empty() || e.label.to_lowercase().contains(&q) || e.category.to_lowercase().contains(&q))
            .collect();
        let matches: Vec<&PaletteEntry> = recent_matches.into_iter()
            .chain(builtin_matches)
            .collect();
```

### Step 5.7 — Add `OpenRecentFile` arm in `run_palette_action`

In `src/app/command_palette.rs`, inside `run_palette_action` (starts at line 264), add a new arm at the end of the `match action` block, before the closing `}`:

```rust
            PaletteAction::OpenRecentFile(idx) => {
                if let Some(path) = self.recent_files.get(idx).cloned() {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => match crate::specgraph::hrf::parse_hrf(&content) {
                            Ok(mut doc) => {
                                crate::specgraph::layout::auto_layout(&mut doc);
                                self.history.push(&self.document);
                                self.autosave_dirty = false;
                                self.document = doc;
                                self.selection.clear();
                                self.pending_fit = true;
                                let fname = path.file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                self.push_recent(path.clone());
                                self.current_file_path = Some(path);
                                super::save_recent_files(&self.recent_files);
                                self.status_message = Some((format!("Opened {fname}"), std::time::Instant::now()));
                            }
                            Err(e) => {
                                self.status_message = Some((format!("Parse error: {e}"), std::time::Instant::now()));
                            }
                        },
                        Err(_) => {
                            let fname = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                            self.status_message = Some((format!("File not found: {fname}"), std::time::Instant::now()));
                            self.recent_files.retain(|p| p != &path);
                            super::save_recent_files(&self.recent_files);
                        }
                    }
                }
            }
```

Note: `save_recent_files` is a private free function in `src/app/mod.rs`. From `command_palette.rs` (which is a child module of `app`), it is accessible as `super::save_recent_files(...)` or as the bare name `save_recent_files` — both work because child modules in Rust can access private items in their parent.

### Step 5.8 — Build to confirm it compiles

```bash
cargo build 2>&1 | grep "^error" | head -20
```

Expected: no errors. Common issues to fix:
- `entry.action` not cloned → add `.clone()`
- `RichText::new(entry.label)` type mismatch → use `.as_ref()` or `.clone()`
- `save_recent_files` not in scope → use `super::save_recent_files`

### Step 5.9 — Run full test suite

```bash
cargo test --tests 2>&1 | tail -5
```

Expected: `test result: ok. 111 passed` (no regression).

### Step 5.10 — Commit

```bash
git add src/app/command_palette.rs
git commit -m "feat: command palette recent files — Cow labels, OpenRecentFile action, two-list ranking"
```

---

## Implementation Checklist

### Task 1 — Persistence helpers + push_recent
- [ ] 4 tests written and failing
- [ ] `current_file_path` and `recent_files` added to struct
- [ ] Fields initialized in `new()`
- [ ] `recent_files_path()`, `load_recent_files()`, `save_recent_files()`, `push_recent_list()` added
- [ ] `push_recent()` method added to impl
- [ ] Build clean, 4 new tests pass, 106 total

### Task 2 — save_to_path + title
- [ ] 5 tests written and failing
- [ ] `compute_window_title()` free function added
- [ ] Title block in `update()` replaced
- [ ] `save_to_path()` method added
- [ ] `new_with_file()` Ok arm updated
- [ ] Build clean, 5 new tests pass, 111 total

### Task 3 — Shortcuts
- [ ] Cmd+Shift+S rebinded to Cmd+Shift+Y (one line change)
- [ ] Cmd+S handler added
- [ ] Cmd+Shift+S save-as handler added
- [ ] Build clean, 111 tests pass

### Task 4 — Template gallery GallerySelection
- [ ] `GallerySelection` enum defined in `template_gallery.rs`
- [ ] `draw_template_gallery` return type updated
- [ ] EmptyCanvas / Template branches updated
- [ ] "Recent" section added with deferred removal
- [ ] Call site in `mod.rs` updated to match on GallerySelection
- [ ] Build clean, 111 tests pass

### Task 5 — Command palette
- [ ] `use std::borrow::Cow` added
- [ ] `PaletteEntry.label: Cow<'static, str>`
- [ ] `PaletteAction` loses `Copy`, gains `OpenRecentFile(usize)`
- [ ] Both `entry.action` copy sites → `.clone()`
- [ ] All `build_entries` labels get `.into()`
- [ ] `RichText::new(entry.label)` → `.as_ref()`
- [ ] `recent_entries` built before Window::show
- [ ] Two-list ranking replaces lines 116–124
- [ ] `OpenRecentFile` arm in `run_palette_action`
- [ ] Build clean, 111 tests pass
