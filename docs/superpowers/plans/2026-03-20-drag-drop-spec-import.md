# Drag-and-Drop Spec Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to drag a `.spec` or `.yaml` file onto the app window to import it, showing a full-screen overlay while hovering and importing via the existing pipeline on drop.

**Architecture:** Three free functions added to `src/app/mod.rs` (`is_supported_ext`, `read_content`, `draw_drop_overlay`) plus ~30 lines wired into the end of `update()`. No new modules, no new traits, no changes to parsing or model code.

**Tech Stack:** Rust, egui (`HoveredFile`, `DroppedFile`, `Area`, `Order::Foreground`), existing `specgraph::import_auto`.

---

## File Map

| File | Change |
|---|---|
| `src/app/mod.rs` | Add 3 free functions + ~30 lines in `update()` at line ~633 |

That's it. `overlays.rs` is not touched — `draw_drop_overlay` is a free function in `mod.rs` (does not need `self` state, so it does not belong in an impl block).

---

## Task 1: Helper functions + unit tests

**Files:**
- Modify: `src/app/mod.rs` (add free functions after the closing `}` of the `impl` block, around line 636)

### Step 1 — Add `is_supported_ext` and `read_content` after the impl block

At the very bottom of `src/app/mod.rs` (after the closing `}` of `impl eframe::App for FlowchartApp`), add:

```rust
/// Returns true if the path has a .spec or .yaml extension (case-insensitive).
fn is_supported_ext(path: Option<&std::path::Path>) -> bool {
    path.and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "spec" | "yaml"))
        .unwrap_or(false)
}

/// Read a dropped file's content: try path first, then bytes, then error.
fn read_dropped_content(file: &egui::DroppedFile) -> Result<String, String> {
    if let Some(path) = &file.path {
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    } else if let Some(bytes) = &file.bytes {
        std::str::from_utf8(&bytes[..])
            .map(|s| s.to_owned())
            .map_err(|e| e.to_string())
    } else {
        Err("file path unavailable".to_string())
    }
}
```

- [ ] **Step 2: Add unit tests immediately below those functions**

```rust
#[cfg(test)]
mod drop_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_supported_ext_spec() {
        assert!(is_supported_ext(Some(Path::new("diagram.spec"))));
    }

    #[test]
    fn test_is_supported_ext_yaml() {
        assert!(is_supported_ext(Some(Path::new("diagram.yaml"))));
    }

    #[test]
    fn test_is_supported_ext_uppercase() {
        assert!(is_supported_ext(Some(Path::new("diagram.SPEC"))));
        assert!(is_supported_ext(Some(Path::new("diagram.YAML"))));
    }

    #[test]
    fn test_is_supported_ext_mixed_case() {
        assert!(is_supported_ext(Some(Path::new("diagram.Spec"))));
    }

    #[test]
    fn test_is_supported_ext_unsupported() {
        assert!(!is_supported_ext(Some(Path::new("image.png"))));
        assert!(!is_supported_ext(Some(Path::new("doc.txt"))));
        assert!(!is_supported_ext(Some(Path::new("file.svg"))));
    }

    #[test]
    fn test_is_supported_ext_no_ext() {
        assert!(!is_supported_ext(Some(Path::new("README"))));
    }

    #[test]
    fn test_is_supported_ext_none_path() {
        assert!(!is_supported_ext(None));
    }

    #[test]
    fn test_read_dropped_content_bytes_fallback() {
        let content = "## Nodes\n- [a] Alpha\n";
        let file = egui::DroppedFile {
            path: None,
            name: "test.spec".to_string(),
            last_modified: None,
            bytes: Some(std::sync::Arc::from(content.as_bytes())),
            mime: String::new(),
        };
        let result = read_dropped_content(&file);
        assert_eq!(result.unwrap(), content);
    }

    #[test]
    fn test_read_dropped_content_no_path_no_bytes() {
        let file = egui::DroppedFile {
            path: None,
            name: "test.spec".to_string(),
            last_modified: None,
            bytes: None,
            mime: String::new(),
        };
        let result = read_dropped_content(&file);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("file path unavailable"));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail (functions don't exist yet — confirm compile error)**

```bash
cd /Users/joe888777/Desktop/project/experiment/light-figma
cargo test drop_tests 2>&1 | head -20
```

Expected: compile error or test failures — functions referenced in tests don't exist yet if tests were written before functions. Since we're adding both at once, expected: PASS (all 9 tests green).

- [ ] **Step 4: Run full test suite to confirm no regressions**

```bash
cargo test 2>&1 | tail -10
```

Expected: all existing tests pass + 9 new drop_tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/app/mod.rs
git commit -m "feat: add is_supported_ext and read_dropped_content helpers with tests"
```

---

## Task 2: Drop overlay function

**Files:**
- Modify: `src/app/mod.rs` (add `draw_drop_overlay` free function between `read_dropped_content` and the test block)

- [ ] **Step 1: Add `draw_drop_overlay` function**

Insert between `read_dropped_content` and `#[cfg(test)]`:

```rust
/// Renders a full-screen dim overlay with drop instructions.
/// `hovered_files` must already be filtered to supported extensions by the caller.
fn draw_drop_overlay(ctx: &egui::Context, hovered_files: &[egui::HoveredFile]) {
    use egui::{Align2, Area, Color32, FontId, Order, RichText};

    let name = hovered_files
        .first()
        .and_then(|f| f.path.as_ref())
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown file".to_owned());

    Area::new(egui::Id::new("drop_overlay"))
        .order(Order::Foreground)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(ctx, |ui| {
            let screen = ctx.screen_rect();
            ui.painter()
                .rect_filled(screen, 0.0, Color32::from_black_alpha(160));

            let center = screen.center();

            ui.painter().text(
                center - egui::vec2(0.0, 20.0),
                Align2::CENTER_CENTER,
                "Drop .spec or .yaml to import",
                FontId::proportional(28.0),
                Color32::WHITE,
            );

            ui.painter().text(
                center + egui::vec2(0.0, 20.0),
                Align2::CENTER_CENTER,
                &name,
                FontId::proportional(16.0),
                Color32::from_gray(180),
            );
        });
}
```

- [ ] **Step 2: Build to confirm it compiles**

```bash
cargo build 2>&1 | grep -E "error|warning" | head -20
```

Expected: clean build, no errors.

- [ ] **Step 3: Commit**

```bash
git add src/app/mod.rs
git commit -m "feat: add draw_drop_overlay full-screen hint function"
```

---

## Task 3: Wire into update()

**Files:**
- Modify: `src/app/mod.rs` — `update()` method, after line 633 (end of spec editor debounce block, before closing `}` of `update()`)

The insertion point is immediately before the final `}` that closes `update()` (currently line 634). Insert after the spec-editor debounce block.

- [ ] **Step 1: Add hover detection and overlay call**

Find this exact closing sequence at the end of `update()`:

```rust
        // Spec editor debounce: re-parse 400ms after last keystroke
        if self.show_spec_editor {
            if let Some(t) = self.spec_editor_last_edit {
                let now = ctx.input(|i| i.time);
                if now - t > 0.4 {
                    self.spec_editor_last_edit = None;
                    self.apply_spec_editor_text();
                }
            }
        }
    }
```

Replace with:

```rust
        // Spec editor debounce: re-parse 400ms after last keystroke
        if self.show_spec_editor {
            if let Some(t) = self.spec_editor_last_edit {
                let now = ctx.input(|i| i.time);
                if now - t > 0.4 {
                    self.spec_editor_last_edit = None;
                    self.apply_spec_editor_text();
                }
            }
        }

        // Drag-and-drop spec import — hover overlay
        let hovered: Vec<egui::HoveredFile> = ctx.input(|i| {
            i.raw
                .hovered_files
                .iter()
                .filter(|f| is_supported_ext(f.path.as_deref()))
                .cloned()
                .collect()
        });
        if !hovered.is_empty() {
            draw_drop_overlay(ctx, &hovered);
        }

        // Drag-and-drop spec import — drop handling
        let dropped: Vec<egui::DroppedFile> =
            ctx.input_mut(|i| i.raw.dropped_files.drain(..).collect());
        if let Some(file) = dropped
            .into_iter()
            .find(|f| is_supported_ext(f.path.as_deref()))
        {
            let filename = file
                .path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "file".to_owned());

            match read_dropped_content(&file) {
                Ok(content) => {
                    let llm_cfg = Some(&self.llm_config);
                    match crate::specgraph::import_auto(&content, llm_cfg) {
                        Ok(doc) => {
                            // Apply ## Config import hints (mirrors toolbar.rs:503-544)
                            if let Some(bg) = doc.import_hints.bg_pattern.as_deref() {
                                self.bg_pattern = match bg {
                                    "dots" | "dot" => BgPattern::Dots,
                                    "lines" | "line" | "grid" => BgPattern::Lines,
                                    "crosshatch" | "cross" | "hash" => BgPattern::Crosshatch,
                                    "none" | "off" | "blank" => BgPattern::None,
                                    _ => self.bg_pattern,
                                };
                            }
                            if let Some(snap) = doc.import_hints.snap {
                                self.snap_to_grid = snap;
                            }
                            if let Some(gs) = doc.import_hints.grid_size {
                                self.grid_size = gs;
                            }
                            let specific_zoom = doc.import_hints.zoom;
                            if let Some(z) = specific_zoom {
                                self.viewport.zoom = z;
                            }
                            if let Some(yaw) = doc.import_hints.camera_yaw {
                                self.camera3d.yaw = yaw;
                            }
                            if let Some(pitch) = doc.import_hints.camera_pitch {
                                self.camera3d.pitch = pitch;
                            }
                            if let Some(true) = doc.import_hints.view_3d {
                                self.view_mode = ViewMode::ThreeD;
                            }
                            if let Some(bg) = doc.import_hints.canvas_bg {
                                self.canvas_bg = bg;
                            }
                            if let Some(ref title) = doc.import_hints.project_title.clone() {
                                self.project_title = title.clone();
                            }
                            let do_fit =
                                doc.import_hints.auto_fit || specific_zoom.is_none();
                            self.document = doc;
                            self.selection.clear();
                            self.history.push(&self.document);
                            self.pending_fit = do_fit;
                            self.show_toast(&format!("Imported {filename}"));
                        }
                        Err(e) => {
                            self.show_toast(&format!("Drop failed: {e}"));
                        }
                    }
                }
                Err(e) => {
                    self.show_toast(&format!("Drop failed: {e}"));
                }
            }
        }
    }
```

> **Note on `show_toast`:** Check whether `FlowchartApp` has a `show_toast(&str)` helper method (line ~438 in mod.rs). If it does, use it. If not, use the inline pattern directly:
> ```rust
> self.status_message = Some((format!("Imported {filename}"), std::time::Instant::now()));
> ```

- [ ] **Step 2: Build**

```bash
cargo build 2>&1 | grep -E "^error" | head -20
```

Expected: clean build. If `BgPattern` or `ViewMode` are not in scope, add `use crate::app::{BgPattern, ViewMode};` at the top of the inserted block, or use the full path `crate::app::BgPattern::Dots` etc.

- [ ] **Step 3: Run full test suite**

```bash
cargo test 2>&1 | tail -15
```

Expected: all tests pass including the 9 drop_tests.

- [ ] **Step 4: Manual test — happy path**

1. `cargo run`
2. Drag any `.spec` file from Finder onto the app window
3. **Expected:** Dark overlay appears with "Drop .spec or .yaml to import" and the filename
4. Drop the file
5. **Expected:** Diagram loads, viewport fits to content, toast "Imported <filename>" appears

- [ ] **Step 5: Manual test — uppercase extension**

Rename a spec file to `diagram.SPEC`, drag it.
Expected: overlay appears, diagram imports correctly.

- [ ] **Step 6: Manual test — unsupported extension**

Drag a `.png` or `.txt` file.
Expected: no overlay, no toast, nothing happens.

- [ ] **Step 7: Manual test — multiple files**

Select 1 `.spec` + 1 `.png` in Finder, drag both at once.
Expected: overlay shows, only the `.spec` is imported.

- [ ] **Step 8: Manual test — corrupt file**

Create a file `bad.spec` with contents `this is not valid HRF !!!@#$`.
Drag it onto the app.
Expected: toast "Drop failed: <parse error message>" appears.

- [ ] **Step 9: Manual test — zoom hint**

Create a spec with `## Config\nzoom = 1.5` and drop it.
Expected: viewport zoom is 1.5 after import, viewport does NOT auto-fit.

- [ ] **Step 10: Manual test — replace existing diagram**

Load any diagram, then drop a different `.spec`.
Expected: diagram is replaced immediately, no save prompt, previous selection cleared.

- [ ] **Step 11: Commit**

```bash
git add src/app/mod.rs
git commit -m "feat: drag-and-drop .spec/.yaml import with hover overlay"
```

---

## Done

All manual tests pass → feature complete.
