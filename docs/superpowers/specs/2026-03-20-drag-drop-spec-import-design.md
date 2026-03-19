# Drag-and-Drop Spec Import

**Date:** 2026-03-20
**Status:** Approved

## Summary

Allow users to drag a `.spec` or `.yaml` file from the OS file manager and drop it anywhere on the light-figma window to import it. While a file is being dragged over the window, display a full-screen overlay prompting the drop. On release, import using the existing parse/import pipeline.

## Goals

- Zero-friction spec loading: no toolbar clicks or file dialogs required
- Consistent with standard desktop drop-to-open behavior
- Reuse existing import logic exactly — no new parsing code

## Non-Goals

- Web/WASM support (file bytes fallback not required for this iteration)
- Drag-and-drop of image or SVG files
- Drag-and-drop between two open app windows

---

## Design

### Data Flow

```
frame tick
  └─ ctx.input(hovered_files non-empty?)
       ├─ yes → draw_drop_overlay()        # semi-transparent full-screen hint
       └─ no  → nothing

frame tick
  └─ ctx.input(dropped_files non-empty?)
       └─ take first file with .spec/.yaml extension
            ├─ read file.path → String
            ├─ specgraph::import_auto(&content, llm_cfg)
            │    ├─ Ok(doc) → apply import_hints + replace document + history.push
            │    │            + toast "Imported <filename>"
            │    └─ Err(e)  → toast "Drop failed: <error>"
            └─ other extension → silently ignore
```

### Files Changed

| File | Change |
|---|---|
| `src/app/mod.rs` | Add drop detection + import call at end of `update()`, after all panels draw |
| `src/app/overlays.rs` | Add `draw_drop_overlay(ctx)` — full-screen semi-transparent rect + centered label |

### Overlay Appearance

- Full-screen `painter.rect_filled` with `Color32::from_black_alpha(160)`
- Centered `RichText` at font size 28: `"Drop .spec or .yaml to import"`
- Secondary line at font size 16: filename(s) being hovered (from `hovered_files[0].name`)
- Rendered on top of all other content via `Area` with a high `order` (e.g. `Order::Foreground`)

### Import Hint Application

Identical to the toolbar paste-import path in `toolbar.rs`:
- `import_hints.canvas_bg` → `self.canvas_bg`
- `import_hints.view_3d` → `self.view_3d`
- `import_hints.zoom` → `self.viewport.zoom`
- `import_hints.camera_yaw / camera_pitch` → `self.camera`
- `import_hints.auto_fit` → `self.pending_fit = true`
- `import_hints.project_title` → `self.project_title`

### Error Handling

| Scenario | Behaviour |
|---|---|
| Multiple files dropped | Import first `.spec`/`.yaml`; ignore rest |
| Unsupported extension | Silent ignore (no toast) |
| File unreadable | Error toast: `"Drop failed: <io error>"` |
| Parse error | Error toast: `"Drop failed: <parse error>"` |
| Empty file | Parse fails → error toast |

---

## Implementation Checklist

- [ ] `overlays.rs`: add `draw_drop_overlay(ctx: &egui::Context, hovered_files: &[egui::HoveredFile])`
- [ ] `mod.rs` `update()`: check `hovered_files` → call `draw_drop_overlay`
- [ ] `mod.rs` `update()`: check `dropped_files` → filter `.spec`/`.yaml` → read → import_auto → apply hints → history → toast
- [ ] Manual test: drag `.spec` file → overlay appears → drop → diagram loads
- [ ] Manual test: drag unsupported file type → no overlay, no import
- [ ] Manual test: drag corrupt file → error toast appears
