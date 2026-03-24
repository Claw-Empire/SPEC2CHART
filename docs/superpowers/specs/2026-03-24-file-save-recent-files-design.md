# File Save + Recent Files Design Spec

**Date:** 2026-03-24
**Project:** light-figma (openAtlas)
**Scope:** Native file save/open (Cmd+S / Cmd+Shift+S) and recent files in template gallery + command palette.

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

Both fields initialize to `None` / empty `Vec` in `new()`. `new_with_file()` sets `current_file_path` when a file is opened via CLI positional arg.

---

## File Save

### Keyboard shortcuts

| Shortcut | Behaviour |
|----------|-----------|
| Cmd+S | Save to `current_file_path`; if None, open Save As dialog first |
| Cmd+Shift+S | Always open Save As dialog |

Wired in `src/app/shortcuts.rs` alongside existing Cmd+Z / Cmd+Y bindings.

### Save dialog

Uses `rfd::FileDialog` (already in `Cargo.toml`). Blocking call on the main thread — acceptable on macOS/Windows.

```rust
rfd::FileDialog::new()
    .add_filter("Spec / YAML", &["spec", "yaml"])
    .set_file_name("diagram.spec")
    .save_file()
```

If the user cancels, save is a no-op with no toast.

### Save implementation (`save_to_path`)

Free function or method inside `impl FlowchartApp`:

1. Call `crate::specgraph::hrf::export_hrf(&self.document)` to get HRF text.
2. Write to path with `std::fs::write`.
3. On success: set `self.current_file_path = Some(path.clone())`, push to `recent_files`, persist recents, show toast `"Saved to <filename>"`.
4. On failure: show toast `"Save failed: <os error>"`, leave `current_file_path` unchanged.

### Window title

Currently `"Light Figma — 5N 3E"`. When `current_file_path` is `Some`:

```
diagram.spec — 5N 3E
```

Set via `ctx.send_viewport_cmd(egui::ViewportCommand::Title(...))` in the existing title update block in `update()`.

### Dirty indicator

When `autosave_dirty` is true and `current_file_path` is `Some`, append `•` to the title:

```
diagram.spec• — 5N 3E
```

No separate "unsaved changes" dialog on close — autosave already handles recovery. Cmd+S clears the dirty indicator.

---

## Recent Files

### Persistence

Stored as a JSON array of path strings at:

```
~/Library/Application Support/light-figma/recent-files.json   (macOS)
%APPDATA%\light-figma\recent-files.json                        (Windows)
$XDG_DATA_HOME/light-figma/recent-files.json                   (Linux)
```

Same base directory as `autosave.json`. Max 10 entries, newest first. Entries are deduplicated on insert (re-opened file moves to top).

Two free functions alongside `autosave_path()`:

```rust
fn recent_files_path() -> PathBuf { ... }          // returns storage path
fn load_recent_files() -> Vec<PathBuf> { ... }     // reads JSON, filters to existing files... wait, no — we keep non-existing files but show them dimmed
fn save_recent_files(files: &[PathBuf]) { ... }    // writes JSON
```

`recent_files` is loaded in `new()` via `load_recent_files()`.

### Recent files in template gallery (`src/app/template_gallery.rs`)

A **"Recent"** section renders above all template categories when `self.recent_files` is non-empty.

Each entry shows:
- Filename (bold) e.g. `arch.spec`
- Full path (dimmed, truncated to 40 chars from right) e.g. `…/project/arch.spec`
- If the file no longer exists on disk: show `(not found)` in warning color; clicking it shows a toast and removes the entry.

Clicking a valid entry:
1. Reads the file.
2. Calls the existing `parse_hrf` → `auto_layout` pipeline.
3. Sets `self.current_file_path = Some(path)`.
4. Pushes to `recent_files` (moves to top), persists.
5. Shows toast `"Opened <filename>"`.

### Recent files in command palette (`src/app/command_palette.rs`)

When `self.recent_files` is non-empty, each path is presented as a command entry:

- **Icon prefix:** file symbol (plain text `[file]` prefix or a unicode char)
- **Title:** filename stem
- **Subtitle:** full path
- **Ranking:** entries whose filename contains the current query string rank at the top of results, above built-in commands

Selecting an entry uses the same open pipeline as the template gallery.

---

## Data Flow

```
Cmd+S
  └─ current_file_path?
       ├─ Some(p) → save_to_path(p)
       └─ None    → rfd::FileDialog → user picks path → save_to_path(path)

Cmd+Shift+S
  └─ rfd::FileDialog → user picks path → save_to_path(path)

save_to_path(path)
  └─ export_hrf(&doc) → fs::write(path) → update current_file_path
                                         → push_recent(path)
                                         → save_recent_files()
                                         → toast

push_recent(path)
  └─ deduplicate → prepend → truncate to 10 → save_recent_files()
```

---

## Files Changed

| File | Change |
|------|--------|
| `src/app/mod.rs` | Add 2 struct fields; initialize in `new()` + `new_with_file()`; add `save_to_path()`, `push_recent()` methods; add `recent_files_path()`, `load_recent_files()`, `save_recent_files()` free functions; wire title update |
| `src/app/shortcuts.rs` | Add Cmd+S and Cmd+Shift+S handlers |
| `src/app/template_gallery.rs` | Add "Recent" section above template categories |
| `src/app/command_palette.rs` | Add recent file entries to palette results |

No new crates. No changes to model, parser, or export code.

---

## Error Handling

| Scenario | Behaviour |
|----------|-----------|
| Save dialog cancelled | Silent no-op |
| `fs::write` fails | Toast `"Save failed: <os error>"` |
| `export_hrf` fails | Toast `"Export failed: <error>"` (should not happen) |
| Recent file missing on open | Toast `"File not found: <path>"`, entry removed from recents |
| `recent-files.json` corrupt | `load_recent_files()` returns empty vec, no crash |

---

## Testing

- Unit test: `push_recent` deduplicates and caps at 10 entries.
- Unit test: `recent_files_path()` returns a path under the correct platform directory.
- Integration: `save_to_path` round-trip — save a document, re-parse the written file, assert node/edge counts match.
- All existing 102 tests continue to pass.
