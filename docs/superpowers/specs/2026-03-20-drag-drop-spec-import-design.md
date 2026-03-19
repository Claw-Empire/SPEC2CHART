# Drag-and-Drop Spec Import

**Date:** 2026-03-20
**Status:** Approved

## Summary

Allow users to drag a `.spec` or `.yaml` file from the OS file manager and drop it anywhere on the light-figma window to import it. While a file is being dragged over the window and at least one hovered file has a supported extension, display a full-screen overlay prompting the drop. On release, import using the existing parse/import pipeline. The current document is replaced without a save prompt.

## Goals

- Zero-friction spec loading: no toolbar clicks or file dialogs required
- Consistent with standard desktop drop-to-open behavior
- Reuse existing import logic exactly — no new parsing code

## Non-Goals

- Web/WASM support (primary path uses `DroppedFile.path`; bytes fallback handles sandboxed macOS)
- Drag-and-drop of image or SVG files
- Drag-and-drop between two open app windows
- Save-before-replace prompt (out of scope for this iteration)

---

## Design

### Data Flow

```
frame tick
  └─ hovered: ctx.input(|i| i.raw.hovered_files.clone())
       └─ filter hovered by is_supported_ext(path)
            ├─ any match → draw_drop_overlay(ctx, &filtered_hovered)
            └─ none      → nothing

frame tick
  └─ dropped: ctx.input_mut(|i| i.raw.dropped_files.drain(..).collect::<Vec<_>>())
       └─ find first DroppedFile where is_supported_ext(path)
            ├─ found → read_content(&file)   // path first, bytes fallback
            │    ├─ Ok(content) → specgraph::import_auto(&content, llm_cfg)
            │    │    ├─ Ok(doc) → apply_import_hints(&doc)
            │    │              + self.document = doc
            │    │              + self.selection.clear()
            │    │              + self.history.push(&self.document)
            │    │              + toast "Imported <filename>"
            │    │    └─ Err(e) → toast "Drop failed: <parse error>"
            │    └─ Err(e) → toast "Drop failed: <io error>"
            └─ none matched → silently ignore (unsupported extension or no files)
```

**Consuming dropped_files:** Drain via `ctx.input_mut` each frame to prevent the same drop from re-triggering import on subsequent frames.

### Extension Matching (`is_supported_ext`)

Case-insensitive; applied identically for `HoveredFile` and `DroppedFile`:

```rust
fn is_supported_ext(path: Option<&std::path::Path>) -> bool {
    path.and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "spec" | "yaml"))
        .unwrap_or(false)
}
```

For `HoveredFile`: pass `file.path.as_deref()`.
For `DroppedFile`: pass `file.path.as_deref()`.

### Content Reading (`read_content`)

```rust
fn read_content(file: &egui::DroppedFile) -> Result<String, String> {
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

`file.bytes` is `Option<Arc<[u8]>>`; deref via `&bytes[..]` to get `&[u8]`.

### Files Changed

| File | Change |
|---|---|
| `src/app/mod.rs` | Drop detection + import logic at end of `update()`, after all panels draw |
| `src/app/overlays.rs` | `draw_drop_overlay(ctx: &egui::Context, hovered_files: &[egui::HoveredFile])` |

### Overlay (`draw_drop_overlay`)

**Signature:** `pub fn draw_drop_overlay(ctx: &egui::Context, hovered_files: &[egui::HoveredFile])`

The caller extracts and filters `hovered_files` by extension, then passes the filtered slice. The function does not call `ctx.input` internally.

- Rendered via `egui::Area::new("drop_overlay").order(egui::Order::Foreground)`
- Full-screen `painter.rect_filled(ctx.screen_rect(), 0.0, Color32::from_black_alpha(160))`
- Primary text (font size 28, centered): `"Drop .spec or .yaml to import"`
- Secondary text (font size 16, centered): filename derived as:
  ```rust
  let name = hovered_files[0].path
      .as_ref()
      .and_then(|p| p.file_name())
      .map(|n| n.to_string_lossy().into_owned())
      .unwrap_or_else(|| "unknown file".to_owned());
  ```

### Import Hint Application (`apply_import_hints`)

Follows the same logic as the toolbar paste-import path (`toolbar.rs`):

| Hint field | Effect |
|---|---|
| `canvas_bg` | `self.canvas_bg = bg` |
| `bg_pattern` | parse string → `self.bg_pattern` |
| `view_3d: Some(true)` | `self.view_mode = ViewMode::ThreeD` |
| `camera_yaw` | `self.camera.yaw = yaw` |
| `camera_pitch` | `self.camera.pitch = pitch` |
| `project_title` | `self.project_title = title` |
| `snap` | `self.snap_to_grid = snap` |
| `grid_size` | `self.grid_size = gs` |
| `zoom` | `self.viewport.zoom = z` |

**Viewport fit logic** (mirrors toolbar.rs):
```rust
let do_fit = doc.import_hints.auto_fit || doc.import_hints.zoom.is_none();
self.pending_fit = do_fit;
```
If `zoom` is set in the spec's `## Config`, `pending_fit` remains false and the zoom value is applied directly. If no zoom is set, `pending_fit = true` so the viewport fits to content.

### Error Handling

| Scenario | Behaviour |
|---|---|
| Multiple files dropped | Import first `.spec`/`.yaml` in slice order; ignore rest |
| Unsupported extension only | Silent ignore — no overlay shown, no toast |
| Path and bytes both absent | Error toast: `"Drop failed: file path unavailable"` |
| File unreadable | Error toast: `"Drop failed: <io error>"` |
| Parse error | Error toast: `"Drop failed: <parse error>"` |
| Empty file | Parse fails → error toast |
| Drop on existing diagram | Replaces document without save prompt (by design) |

---

## Implementation Checklist

- [ ] `overlays.rs`: implement `draw_drop_overlay(ctx: &egui::Context, hovered_files: &[egui::HoveredFile])`
  - [ ] Full-screen `egui::Order::Foreground` Area
  - [ ] `Color32::from_black_alpha(160)` background rect
  - [ ] Primary + secondary text with filename fallback `"unknown file"`
- [ ] `mod.rs`: add `is_supported_ext(path: Option<&Path>) -> bool` helper (case-insensitive)
- [ ] `mod.rs`: add `read_content(file: &egui::DroppedFile) -> Result<String, String>` helper
- [ ] `mod.rs` `update()`: read `hovered_files`, filter by extension → call `draw_drop_overlay` if any match
- [ ] `mod.rs` `update()`: drain `dropped_files` via `ctx.input_mut` → find first supported → `read_content` → `import_auto` → `apply_import_hints` → `selection.clear()` → `history.push` → toast `"Imported <filename>"`
- [ ] Manual test: drag `.spec` file → overlay appears with filename → drop → diagram loads, viewport fits
- [ ] Manual test: drag `.SPEC` (uppercase extension) → overlay and import both work
- [ ] Manual test: drag unsupported file type → no overlay, no toast
- [ ] Manual test: drag multiple files including one `.spec` → first `.spec` imported
- [ ] Manual test: drag file with explicit `zoom` in Config → zoom applied, no auto-fit
- [ ] Manual test: drag corrupt/invalid spec → error toast appears
- [ ] Manual test: drop onto existing diagram → diagram replaced, no save prompt, selection cleared
