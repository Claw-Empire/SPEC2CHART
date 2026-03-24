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

Both initialize to `None` / empty `Vec` in `new()`. See `new_with_file` changes below.

---

## File Save

### Keyboard shortcuts

| Shortcut | Behaviour |
|----------|-----------|
| Cmd+S | Save to `current_file_path`; if None, open Save As dialog first |
| Cmd+Shift+S | Always open Save As dialog |

Wired in `src/app/shortcuts.rs` alongside existing Cmd+Z / Cmd+Y bindings.

The app has no native menu bar (egui does not render one by default on macOS). Save is keyboard-only.

### Save dialog

Uses `rfd::FileDialog` (already in `Cargo.toml`). Blocking call on the main thread.

```rust
rfd::FileDialog::new()
    .add_filter("Spec / YAML", &["spec", "yaml"])
    .set_file_name("diagram.spec")
    .save_file()
```

If the user cancels, save is a silent no-op.

### `export_hrf` title resolution

`export_hrf` has signature `pub fn export_hrf(doc: &FlowchartDocument, title: &str) -> String`.

Title resolution order for the save call:
1. `self.document.title` if non-empty
2. `current_file_path.file_stem()` if available
3. `"Untitled Diagram"` as final fallback

### `save_to_path(&mut self, path: PathBuf)` method

1. Resolve title as above.
2. Call `export_hrf(&self.document, &title)` to get HRF text.
3. Call `std::fs::write(&path, &hrf)`.
4. On success:
   - Set `self.current_file_path = Some(path.clone())`
   - Call `self.push_recent(path)` (in-memory only — see below)
   - Call `save_recent_files(&self.recent_files)` (writes JSON)
   - Set `self.autosave_dirty = false`
   - Show toast `"Saved to <filename>"`
5. On failure: show toast `"Save failed: <os error>"`, leave state unchanged.
6. `save_recent_files` failure (disk full, permissions) is silently ignored — recent-files list is best-effort; a write failure must not block or crash the save operation.

Subsequent Cmd+S saves to the same path show no toast (silent save) to avoid toast spam. Only the first save to a new path shows the toast.

### `push_recent(&mut self, path: PathBuf)` method — in-memory only

Mutates `self.recent_files` only. Does NOT call `save_recent_files`. Callers are responsible for persisting.

```
1. Remove any existing entry equal to path (deduplicate)
2. Prepend path to front
3. Truncate to 10 entries
```

### `new_with_file` update

`new_with_file(cc, Some(path))` must be updated to:
- Set `app.current_file_path = Some(path.clone())`
- Call `app.push_recent(path.clone())`
- Call `save_recent_files(&app.recent_files)`

This is an explicit addition to `src/app/mod.rs` as part of this feature.

---

## Window Title

The existing title block becomes a three-state expression:

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
```

The `•` dirty indicator is a U+2022 BULLET, which renders correctly on macOS and Windows title bars. On Linux it depends on the window manager; this is acceptable.

---

## Recent Files

### Persistence

Stored as a JSON array of path strings at the same base directory as `autosave.json`:

```
~/Library/Application Support/light-figma/recent-files.json   (macOS)
%APPDATA%\light-figma\recent-files.json                        (Windows)
$XDG_DATA_HOME/light-figma/recent-files.json                   (Linux)
```

Three free functions alongside `autosave_path()`:

```rust
fn recent_files_path() -> PathBuf { /* same base dir logic as autosave_path() */ }
fn load_recent_files() -> Vec<PathBuf> { /* deserialize JSON; return empty vec on any error */ }
fn save_recent_files(files: &[PathBuf]) { /* serialize to JSON; silently ignore write errors */ }
```

`recent_files` is loaded in `new()` via `load_recent_files()`. Non-existing file paths are retained in the list; they are shown dimmed in the UI and removed only when the user clicks them.

---

## Recent Files in Template Gallery

### Return type change

`draw_template_gallery` currently returns `Option<String>` (raw HRF content). This must be extended to:

```rust
pub(crate) enum GallerySelection {
    Template(String),           // HRF content from a built-in template
    RecentFile(PathBuf),        // path chosen from recents
    EmptyCanvas,                // "New empty canvas" button
}
```

The call site in `update()` matches on the new enum. The existing template-load and empty-canvas arms remain unchanged; a new `RecentFile(path)` arm reads the file, parses, layouts, sets `current_file_path`, calls `push_recent`, persists, and shows a toast.

### Rendering

A **"Recent"** section renders above all template categories when `self.recent_files` is non-empty. Each entry shows:
- Filename bold, e.g. `arch.spec`
- Full path dimmed, right-truncated to 40 chars with `…` prefix, e.g. `…/project/arch.spec`
- If file does not exist on disk: label shown in warning color with `(not found)` suffix

**Deferred removal of missing entries:** during rendering, collect missing-file paths into a `Vec<PathBuf>`. After rendering, call `self.recent_files.retain(|p| !to_remove.contains(p))` and persist. This avoids mutating `self.recent_files` while iterating it inside the UI closure.

---

## Recent Files in Command Palette

### `PaletteAction` change

`PaletteAction` is currently `#[derive(Clone, Copy)]` with no heap data. Adding file paths requires:

```rust
#[derive(Clone)]  // remove Copy
pub(crate) enum PaletteAction {
    // ... existing variants unchanged ...
    OpenRecentFile(usize),  // index into FlowchartApp::recent_files
}
```

Removing `Copy` from `PaletteAction` requires updating all match arms to use `clone()` or references where the value was previously copied. This is a mechanical change affecting `command_palette.rs` only.

### Ranking

Recent file entries are generated dynamically each frame from `self.recent_files`. The palette builds two filtered lists and concatenates them:

1. **Recent file matches:** `recent_files` entries whose filename contains the query string (case-insensitive).
2. **Built-in command matches:** existing static entries that match the query.

When the query is empty, recent files appear first (up to 5), then all built-in commands. When the query is non-empty, recent file matches always precede built-in command matches. Ties within each group preserve insertion order.

Selecting `OpenRecentFile(idx)` looks up `self.recent_files.get(idx)` in the `update()` handler and uses the same open pipeline as the template gallery's `RecentFile(path)` arm.

---

## Data Flow Summary

```
Cmd+S
  └─ current_file_path?
       ├─ Some(p) → save_to_path(p) → silent (no toast if same path)
       └─ None    → rfd dialog → save_to_path(chosen path) → toast

Cmd+Shift+S
  └─ rfd dialog → save_to_path(chosen path) → toast

save_to_path(path)
  └─ export_hrf(title) → fs::write → current_file_path = path
                                    → push_recent(path)   [in-memory]
                                    → save_recent_files() [best-effort]
                                    → autosave_dirty = false
                                    → toast (first save only)

Gallery/Palette open recent file
  └─ fs::read → parse_hrf → auto_layout → document = doc
                                         → current_file_path = path
                                         → push_recent(path) [in-memory]
                                         → save_recent_files()
                                         → toast "Opened <filename>"
```

---

## Files Changed

| File | Change |
|------|--------|
| `src/app/mod.rs` | 2 new struct fields; init in `new()` + `new_with_file()`; `save_to_path()`, `push_recent()` methods; `recent_files_path()`, `load_recent_files()`, `save_recent_files()` free functions; title update block |
| `src/app/shortcuts.rs` | Cmd+S and Cmd+Shift+S handlers |
| `src/app/template_gallery.rs` | `GallerySelection` enum replaces `Option<String>`; "Recent" section; deferred removal of missing entries |
| `src/app/command_palette.rs` | `PaletteAction` loses `Copy`; dynamic recent-file entries; two-list ranking |

No new crates. No changes to model, parser, or export code.

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| Save dialog cancelled | Silent no-op |
| `fs::write` fails | Toast `"Save failed: <os error>"` |
| `export_hrf` fails | Toast `"Export failed: <error>"` (should not happen) |
| `save_recent_files` write fails | Silent ignore — recents are best-effort |
| Recent file missing on click | Toast `"File not found: <path>"`, entry removed |
| `recent-files.json` corrupt | `load_recent_files()` returns empty vec, no crash |

---

## Testing

- Unit test: `push_recent` deduplicates and caps at 10 entries.
- Unit test: `recent_files_path()` returns a path under the correct platform directory.
- Unit test: title string logic across all four `(filename, node_count)` combinations.
- Integration: `save_to_path` round-trip — save a document, re-parse the written file, assert node/edge counts match.
- All existing 102 tests continue to pass.
