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

The existing Cmd+Shift+S binding (`src/app/shortcuts.rs:934–959`) copies the HRF spec to the system clipboard. Save As takes over Cmd+Shift+S (the macOS standard). Clipboard-copy moves to **Cmd+Shift+Y** ("yank" — the Unix/vim idiom for copy-to-clipboard, and confirmed free in the current shortcut map).

Full audit of Cmd+Shift bindings confirmed taken: Z, ], [, C, V, A, K, ., ,, E, I, T, F, S, R, X, W, H. Cmd+Shift+Y is unused.

| Shortcut | Action |
|----------|--------|
| Cmd+S | Save to current path; if none, open Save As dialog |
| Cmd+Shift+S | Always open Save As dialog |
| Cmd+Shift+Y | Copy current diagram as HRF spec to clipboard *(moved from Cmd+Shift+S)* |

Both Cmd+S and Cmd+Shift+S are wired in `src/app/shortcuts.rs` alongside existing bindings. The existing Cmd+Shift+S handler at line 934 has its modifier check updated from `cmd_shift_s` to `cmd_shift_y`.

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

Save uses `export_hrf_ex` (signature: `pub fn export_hrf_ex(doc: &FlowchartDocument, title: &str, vp: Option<&ViewportExportConfig>) -> String`).

`export_hrf_ex` has an internal override at `hrf.rs:1611`: `if doc.title.is_empty() { title } else { &doc.title }`. This means the `title` arg only takes effect when `doc.title` is empty. `save_to_path` therefore passes only the fallback for the empty-title case:

```rust
let fallback_title = self.current_file_path.as_ref()
    .and_then(|p| p.file_stem())
    .map(|s| s.to_string_lossy().into_owned())
    .unwrap_or_else(|| "Untitled Diagram".to_string());
let hrf = export_hrf_ex(&self.document, &fallback_title, None);
```

`ViewportExportConfig` is `None` for a plain save (no viewport hints embedded).

### `save_to_path(&mut self, path: PathBuf)` method

```
1. Compute fallback_title as above.
2. export_hrf_ex(&self.document, &fallback_title, None) → hrf: String
3. std::fs::write(&path, &hrf)
4. On success:
   a. let is_new_path = self.current_file_path.as_ref() != Some(&path);
   b. self.push_recent(path.clone())     [in-memory, see below]
   c. self.current_file_path = Some(path) [consumes path]
   d. save_recent_files(&self.recent_files) [best-effort, see below]
   e. self.autosave_dirty = false
   f. if is_new_path { show toast "Saved to <filename>" }
      // Cmd+S to already-known path is silent to avoid toast spam
      // Cmd+Shift+S (Save As) always passes a path != old path, so always toasts
5. On failure: toast "Save failed: <os error>"; leave state unchanged.
```

### `push_recent(&mut self, path: PathBuf)` — in-memory only

Does NOT call `save_recent_files`. Callers persist after calling this.

```
1. Remove any existing entry equal to path
2. Prepend path to front
3. Truncate to 10 entries
```

### `new_with_file` update

Inside the `Ok(mut doc) => { ... }` arm of `new_with_file` (around `src/app/mod.rs:444`), after `app.pending_fit = true`, add:

```rust
app.push_recent(path.clone());    // path is still owned here (inside Ok arm)
app.current_file_path = Some(path.clone());
save_recent_files(&app.recent_files);
```

Note: `path` is the variable bound by the outer `if let Some(path) = file` at the top of `new_with_file`. It is still in scope inside the `Ok` arm. Do not place these lines after the outer `if let` block — `path` would be out of scope.

---

## Window Title

The existing title block at `src/app/mod.rs:783–791` is **replaced entirely** (delete those lines and substitute the block below — do not add alongside):

```rust
let filename = self.current_file_path.as_ref()
    .and_then(|p| p.file_name())
    .map(|n| n.to_string_lossy().into_owned());
// dirty_mark is only shown when a file path is known; for unsaved new documents
// autosave already handles recovery, so no indicator is intentional.
let dirty_mark = if self.autosave_dirty && self.current_file_path.is_some() { "•" } else { "" };

let title = match (&filename, n) {
    (Some(f), 0) => format!("{f}{dirty_mark}"),
    (Some(f), _) => format!("{f}{dirty_mark} — {n}N {e}E"),
    (None,    0) => "Light Figma".to_string(),
    (None,    _) => format!("Light Figma — {n}N {e}E"),
};
ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
```

The `•` dirty indicator is U+2022 BULLET, correct on macOS/Windows. Linux depends on window manager; acceptable.

---

## Recent Files

### Persistence

Same base directory as `autosave.json`:

```
~/Library/Application Support/light-figma/recent-files.json   (macOS)
%APPDATA%\light-figma\recent-files.json                        (Windows)
$XDG_DATA_HOME/light-figma/recent-files.json                   (Linux)
```

Three free functions alongside `autosave_path()` in `src/app/mod.rs`:

```rust
fn recent_files_path() -> PathBuf { /* same base-dir logic as autosave_path() */ }
fn load_recent_files() -> Vec<PathBuf> { /* deserialize JSON; return empty vec on any error */ }
fn save_recent_files(files: &[PathBuf]) { /* serialize to JSON; silently ignore all errors */ }
```

`recent_files` is loaded in `new()` via `load_recent_files()`. Non-existing paths are retained and shown dimmed; removed only when clicked.

---

## Recent Files in Template Gallery

### `GallerySelection` enum

Defined in `src/app/template_gallery.rs` with `pub(crate)` visibility (accessible from `mod.rs`'s `update()` loop via the `super::template_gallery::GallerySelection` path, or re-exported at the top of `mod.rs` with `use crate::app::template_gallery::GallerySelection`).

```rust
pub(crate) enum GallerySelection {
    Template(String),      // HRF content from a built-in template
    RecentFile(PathBuf),   // path chosen from recents
    EmptyCanvas,           // "New empty canvas" button
}
```

`draw_template_gallery` return type changes from `Option<String>` to `Option<GallerySelection>`. The call site in `update()` matches on the new enum. Existing template-load and empty-canvas arms are unchanged in behaviour. New `RecentFile(path)` arm: read file, parse HRF, auto-layout, set `current_file_path`, `push_recent`, `save_recent_files`, toast `"Opened <filename>"`.

### Rendering

A **"Recent"** section renders above all template categories when `self.recent_files` is non-empty. Each entry:
- Filename bold, e.g. `arch.spec`
- Full path dimmed, right-truncated to 40 chars, e.g. `…/project/arch.spec`
- Missing file: warning color, `(not found)` suffix

**Deferred removal:** collect missing paths into `to_remove: Vec<PathBuf>` during rendering; after the UI closure call `self.recent_files.retain(|p| !to_remove.contains(p))` then `save_recent_files`. This avoids mutating `self.recent_files` inside the UI closure.

---

## Recent Files in Command Palette

### `PaletteAction` change

Remove `Copy` from the derive list in `command_palette.rs:15`:

```rust
#[derive(Clone)]  // Copy removed
pub(crate) enum PaletteAction {
    // ... existing variants unchanged ...
    OpenRecentFile(usize),  // index into FlowchartApp::recent_files
}
```

All sites in `command_palette.rs` that copy a `PaletteAction` (lines ~136, ~237: `execute_action = Some(entry.action)`) become `entry.action.clone()`. Change is mechanical and confined to `command_palette.rs`.

### `OpenRecentFile` dispatch

Handled inside `run_palette_action` (`command_palette.rs:264`), which is already `&mut self` on `FlowchartApp` with direct access to `self.recent_files`:

```rust
PaletteAction::OpenRecentFile(idx) => {
    if let Some(path) = self.recent_files.get(idx).cloned() {
        // same open pipeline as GallerySelection::RecentFile
    }
}
```

### Ranking

Two filtered lists, concatenated for display:

1. **Recent file matches:** `recent_files` entries whose filename contains the query (case-insensitive); up to 5 when query is empty.
2. **Built-in command matches:** existing static entries matching the query.

When query is empty: up to 5 recents first, then all built-ins. When non-empty: recent matches precede built-ins. Ties within each group preserve insertion order.

---

## Data Flow Summary

```
Cmd+S
  └─ current_file_path?
       ├─ Some(p) → save_to_path(p) → silent if same path
       └─ None    → rfd Save dialog → save_to_path(chosen) → toast

Cmd+Shift+S
  └─ rfd Save dialog → save_to_path(chosen) → toast

Cmd+Shift+Y  (moved from Cmd+Shift+S)
  └─ export_hrf_ex → copy to clipboard → toast "Spec copied"

save_to_path(path)
  └─ export_hrf_ex(fallback_title, None)
     → fs::write
     → push_recent(path.clone())   [in-memory]
     → current_file_path = Some(path)
     → save_recent_files()         [best-effort]
     → autosave_dirty = false
     → toast if new path

Gallery / Palette open recent file
  └─ fs::read → parse_hrf → auto_layout
     → document = doc
     → push_recent(path.clone())   [in-memory]
     → current_file_path = Some(path)
     → save_recent_files()
     → toast "Opened <filename>"
```

---

## Files Changed

| File | Change |
|------|--------|
| `src/app/mod.rs` | 2 new fields; init in `new()`; explicit additions inside `Ok` arm of `new_with_file()`; `save_to_path()`, `push_recent()` methods; `recent_files_path()`, `load_recent_files()`, `save_recent_files()` free functions; replace title block at lines 783–791 |
| `src/app/shortcuts.rs` | Add Cmd+S and Cmd+Shift+S handlers; rebind existing Cmd+Shift+S clipboard-copy to Cmd+Shift+Y (change modifier at line 935 only) |
| `src/app/template_gallery.rs` | `GallerySelection` enum; `draw_template_gallery` return type; "Recent" section; deferred removal |
| `src/app/command_palette.rs` | `PaletteAction` loses `Copy`; `OpenRecentFile(usize)` variant; `.clone()` at copy sites; two-list ranking; `OpenRecentFile` arm in `run_palette_action` |

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
- Unit test: title string covers all four `(filename, node_count)` + dirty-mark combinations.
- Integration: `save_to_path` round-trip — save a document, re-parse the written file, assert node/edge counts match.
- All existing 102 tests continue to pass.
