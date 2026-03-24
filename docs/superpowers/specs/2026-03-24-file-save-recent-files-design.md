# File Save + Recent Files Design Spec

**Date:** 2026-03-24
**Project:** light-figma (openAtlas)
**Scope:** Native file save (Cmd+S / Cmd+Shift+S) and recent files in template gallery + command palette.

---

## Goal

Close the biggest desktop usability gap: users can import `.spec` files but have no way to save their work back to disk from the GUI. Add persistent recent-files tracking surfaced in two existing entry points (template gallery and command palette).

---

## Architecture

### New state fields on `FlowchartApp` (`src/app/mod.rs`)

```rust
/// Path of the currently open file; None if unsaved.
pub(crate) current_file_path: Option<std::path::PathBuf>,
/// Recently opened/saved files, newest first. Max 10 entries. Persisted.
pub(crate) recent_files: Vec<std::path::PathBuf>,
```

Both initialize to `None` / empty `Vec` in `new()`.

---

## Shortcut Rebinding

The existing Cmd+Shift+S binding (`src/app/shortcuts.rs:934–959`) copies the HRF spec to the system clipboard. Save As takes over Cmd+Shift+S (the macOS standard). Clipboard-copy moves to **Cmd+Shift+C** ("copy spec"). The handler logic is unchanged; only the modifier check changes from `cmd_shift_s` to `cmd_shift_c`.

| Shortcut | Action |
|----------|--------|
| Cmd+S | Save to current path; if none, open Save As dialog |
| Cmd+Shift+S | Always open Save As dialog |
| Cmd+Shift+C | Copy current diagram as HRF spec to clipboard *(moved from Cmd+Shift+S)* |

Both Cmd+S and Cmd+Shift+S are wired in `src/app/shortcuts.rs` alongside existing bindings.

The app has no native menu bar (egui does not render one by default on macOS). Save is keyboard-only.

---

## File Save

### Save dialog

Uses `rfd::FileDialog` (already in `Cargo.toml`). Blocking call on the main thread.

```rust
rfd::FileDialog::new()
    .add_filter("Spec / YAML", &["spec", "yaml"])
    .set_file_name("diagram.spec")
    .save_file()
```

If the user cancels, save is a silent no-op.

### `export_hrf_ex` and title resolution

Save uses `export_hrf_ex` (the richer variant already used throughout the app), which has signature:

```rust
pub fn export_hrf_ex(doc: &FlowchartDocument, title: &str, vp: Option<&ViewportExportConfig>) -> String
```

`export_hrf_ex` applies its own internal override: if `doc.title` is non-empty it uses `doc.title` regardless of the `title` argument. Therefore `save_to_path` only needs to supply a fallback title for documents that have no title set:

```rust
let fallback_title = self.current_file_path.as_ref()
    .and_then(|p| p.file_stem())
    .map(|s| s.to_string_lossy().into_owned())
    .unwrap_or_else(|| "Untitled Diagram".to_string());
// export_hrf_ex will use doc.title if non-empty, fallback_title otherwise
let hrf = export_hrf_ex(&self.document, &fallback_title, None);
```

The `ViewportExportConfig` argument is `None` for a plain save (no viewport hints embedded). If the user wants to save with viewport state, that is a future enhancement.

### `save_to_path(&mut self, path: PathBuf)` method

1. Compute `fallback_title` as above.
2. Call `export_hrf_ex(&self.document, &fallback_title, None)`.
3. Call `std::fs::write(&path, &hrf)`.
4. On success:
   - Set `self.current_file_path = Some(path.clone())`
   - Call `self.push_recent(path)` (in-memory only)
   - Call `save_recent_files(&self.recent_files)` (best-effort, failures silently ignored)
   - Set `self.autosave_dirty = false`
   - Show toast only when `path != old_current_file_path` (i.e. first save or Save As to new path): `"Saved to <filename>"`
   - Cmd+S to the already-known path is silent (no toast) to avoid spam
5. On failure: show toast `"Save failed: <os error>"`, leave state unchanged.

### `push_recent(&mut self, path: PathBuf)` method — in-memory only

Does NOT call `save_recent_files`. Callers persist after calling this.

```
1. Remove any existing entry equal to path
2. Prepend path to front
3. Truncate to 10 entries
```

### `new_with_file` update

`new_with_file(cc, Some(path))` must be updated to add these three lines after the document is loaded successfully:

```rust
app.current_file_path = Some(path.clone());
app.push_recent(path.clone());
save_recent_files(&app.recent_files);
```

This is an explicit addition to the existing `new_with_file` body in `src/app/mod.rs` (~line 428–461).

---

## Window Title

The existing title block at `src/app/mod.rs:783–791` is **replaced** (not supplemented — only one `ctx.send_viewport_cmd(Title(...))` call must exist):

```rust
let filename = self.current_file_path.as_ref()
    .and_then(|p| p.file_name())
    .map(|n| n.to_string_lossy().into_owned());
let dirty_mark = if self.autosave_dirty && self.current_file_path.is_some() { "•" } else { "" };

let title = match (filename, n) {
    (Some(f), 0) => format!("{f}{dirty_mark}"),
    (Some(f), _) => format!("{f}{dirty_mark} — {n}N {e}E"),
    (None,    0) => "Light Figma".to_string(),
    (None,    _) => format!("Light Figma — {n}N {e}E"),
};
ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
```

The `•` dirty indicator is U+2022 BULLET, which renders correctly on macOS and Windows. Linux rendering depends on the window manager; acceptable.

---

## Recent Files

### Persistence

Same base directory as `autosave.json`:

```
~/Library/Application Support/light-figma/recent-files.json   (macOS)
%APPDATA%\light-figma\recent-files.json                        (Windows)
$XDG_DATA_HOME/light-figma/recent-files.json                   (Linux)
```

Three free functions alongside `autosave_path()`:

```rust
fn recent_files_path() -> PathBuf { /* same base-dir logic as autosave_path() */ }
fn load_recent_files() -> Vec<PathBuf> { /* deserialize JSON; return empty vec on any error */ }
fn save_recent_files(files: &[PathBuf]) { /* serialize to JSON; silently ignore write errors */ }
```

`recent_files` is loaded in `new()` via `load_recent_files()`. Non-existing paths are retained and shown dimmed; they are removed only when clicked.

---

## Recent Files in Template Gallery

### Return type change

`draw_template_gallery` currently returns `Option<String>`. This changes to:

```rust
pub(crate) enum GallerySelection {
    Template(String),      // HRF content from a built-in template
    RecentFile(PathBuf),   // path chosen from recents
    EmptyCanvas,           // "New empty canvas" button
}
```

The call site in `update()` matches on the new enum. Existing template-load and empty-canvas arms are unchanged. The new `RecentFile(path)` arm reads the file, parses, auto-layouts, sets `current_file_path`, calls `push_recent`, calls `save_recent_files`, shows toast `"Opened <filename>"`.

### Rendering

A **"Recent"** section renders above all template categories when `self.recent_files` is non-empty. Each entry shows:
- Filename bold, e.g. `arch.spec`
- Full path dimmed, right-truncated to 40 chars, e.g. `…/project/arch.spec`
- Missing file: warning color with `(not found)` suffix

**Deferred removal of missing entries:** collect missing paths into a `to_remove: Vec<PathBuf>` during rendering, then after the UI closure: `self.recent_files.retain(|p| !to_remove.contains(p))` and call `save_recent_files`. This avoids mutating `self.recent_files` while iterating inside the UI closure.

---

## Recent Files in Command Palette

### `PaletteAction` change

`PaletteAction` is currently `#[derive(Clone, Copy)]`. Adding file indexing requires removing `Copy`:

```rust
#[derive(Clone)]  // Copy removed
pub(crate) enum PaletteAction {
    // ... existing variants unchanged ...
    OpenRecentFile(usize),  // index into FlowchartApp::recent_files
}
```

All existing sites in `command_palette.rs` that copy a `PaletteAction` (e.g. `execute_action = Some(entry.action)`) become `entry.action.clone()`. This is a mechanical change confined to `command_palette.rs`.

### `OpenRecentFile` dispatch

The `OpenRecentFile(idx)` variant is handled inside the existing `run_palette_action` method in `command_palette.rs`, which is already a `&mut self` method on `FlowchartApp` and has direct access to `self.recent_files`. No dispatch through `update()` is needed.

```rust
PaletteAction::OpenRecentFile(idx) => {
    if let Some(path) = self.recent_files.get(idx).cloned() {
        // same open pipeline as GallerySelection::RecentFile
    }
}
```

### Ranking

Two filtered lists, concatenated for display:

1. **Recent file matches:** `recent_files` entries whose filename contains the query (case-insensitive), up to 5 when query is empty.
2. **Built-in command matches:** existing static entries matching the query.

When query is empty: up to 5 recents first, then all built-ins. When query is non-empty: recent matches precede built-in matches. Ties within each group preserve insertion order.

---

## Data Flow Summary

```
Cmd+S
  └─ current_file_path?
       ├─ Some(p) → save_to_path(p) → silent if same path
       └─ None    → rfd Save dialog → save_to_path(chosen) → toast

Cmd+Shift+S
  └─ rfd Save dialog → save_to_path(chosen) → toast

Cmd+Shift+C  (moved from Cmd+Shift+S)
  └─ export_hrf_ex → copy to clipboard → toast "Spec copied"

save_to_path(path)
  └─ export_hrf_ex(fallback_title, None)
     → fs::write
     → current_file_path = path
     → push_recent(path)        [in-memory]
     → save_recent_files()      [best-effort]
     → autosave_dirty = false
     → toast if new path

Gallery / Palette open recent file
  └─ fs::read → parse_hrf → auto_layout
     → document = doc
     → current_file_path = path
     → push_recent(path)        [in-memory]
     → save_recent_files()
     → toast "Opened <filename>"
```

---

## Files Changed

| File | Change |
|------|--------|
| `src/app/mod.rs` | 2 new struct fields; init in `new()` + explicit additions to `new_with_file()`; `save_to_path()`, `push_recent()` methods; `recent_files_path()`, `load_recent_files()`, `save_recent_files()` free functions; replace title block at lines 783–791 |
| `src/app/shortcuts.rs` | Cmd+S and Cmd+Shift+S handlers; move clipboard-copy from Cmd+Shift+S to Cmd+Shift+C |
| `src/app/template_gallery.rs` | `GallerySelection` enum replaces `Option<String>`; "Recent" section; deferred removal |
| `src/app/command_palette.rs` | `PaletteAction` loses `Copy`; `OpenRecentFile(usize)` variant; two-list ranking; `clone()` at copy sites |

No new crates. No changes to model, parser, or export code.

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| Save dialog cancelled | Silent no-op |
| `fs::write` fails | Toast `"Save failed: <os error>"` |
| `save_recent_files` write fails | Silent ignore — recents are best-effort |
| Recent file missing on click | Toast `"File not found: <path>"`, entry removed |
| `recent-files.json` corrupt | `load_recent_files()` returns empty vec, no crash |

---

## Testing

- Unit test: `push_recent` deduplicates and caps at 10 entries.
- Unit test: `recent_files_path()` returns a path under the correct platform directory.
- Unit test: title string logic covers all four `(filename, node_count)` combinations including dirty marker.
- Integration: `save_to_path` round-trip — save a document, re-parse the written file, assert node/edge counts match.
- All existing 102 tests continue to pass.
