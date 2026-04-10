# Humanization Features Design Spec

**Date:** 2026-03-26
**Scope:** Add features and optimize functions for a more humanized experience
**Target:** Complete by 10 PM UTC+8

## Overview

A comprehensive humanization sprint across 5 categories: smarter feedback, visual affordances, discoverability, interaction polish, and delight. Each feature is designed to make openDraftly feel more intuitive, forgiving, and alive.

---

## 1. Color-Coded Status Toasts

**Current:** Status messages are plain white text with auto-detected emoji.
**Improved:** Messages carry a severity level that drives color, icon, and duration.

### Implementation

Add a `StatusLevel` enum:
```
enum StatusLevel { Success, Info, Warning, Error }
```

Update `status_message` from `Option<(String, Instant)>` to `Option<(String, Instant, StatusLevel)>`.

Color mapping:
- **Success** (green accent): "Saved", "Pasted 3 nodes", "Style applied"
- **Info** (blue accent): "Zoom 150%", "Grid: 20px", "Focus mode on"
- **Warning** (amber): "No nodes selected", "Node is locked"
- **Error** (red): "Save failed", "Invalid connection"

Display: Rounded pill background with left color stripe, auto-fade after 3s (errors persist 5s).

### Files
- `src/app/mod.rs` — Add `StatusLevel` enum, update `status_message` type
- `src/app/canvas.rs` — Update `show_status_message()` rendering
- `src/app/shortcuts.rs` — Tag each `set_status()` call with appropriate level

---

## 2. Invalid Action Feedback

**Current:** Invalid actions (connecting node to self, dragging locked node) silently fail.
**Improved:** Show warning toast explaining why and suggesting the fix.

### Messages
- "Can't connect a node to itself" (when edge source == target)
- "This node is locked — press Cmd+Shift+L to unlock" (when dragging locked node)
- "This node is pinned — unpin to move freely" (when dragging pinned node)
- "Canvas is locked — press Cmd+Shift+K to unlock" (when trying to edit on locked canvas)
- "No nodes selected — select nodes first" (when using alignment/distribute with nothing selected)
- "Need 2+ nodes to align" (when alignment invoked on single node)

### Files
- `src/app/canvas.rs` — Add checks at edge creation, node drag start, alignment commands
- `src/app/shortcuts.rs` — Add feedback for shortcut-triggered invalid actions

---

## 3. Visual Lock & Pin Indicators

**Current:** Locked/pinned nodes look identical to normal nodes.
**Improved:** Overlay a small lock (🔒) or pin (📌) icon on locked/pinned nodes.

### Implementation
- Draw a small 14x14 icon in the top-right corner of locked nodes
- Draw a small pin icon for pinned nodes
- Use egui text rendering with the unicode character, subtle semi-transparent background
- Show tooltip on hover: "Locked — Cmd+Shift+L to toggle"

### Files
- `src/app/render.rs` — Add icon overlay in node rendering pass

---

## 4. Port Hover Glow

**Current:** Ports show no visual feedback when hovered during edge creation.
**Improved:** Ports glow with a colored ring when the cursor is near them during `CreatingEdge` drag.

### Implementation
- During `DragState::CreatingEdge`, check proximity to all visible ports
- If cursor is within 20px of a port, draw a pulsing circle_stroke (2px, accent color, alpha oscillating)
- Additionally, snap the edge preview endpoint to the port center

### Files
- `src/app/render.rs` — Add port glow rendering
- `src/app/canvas.rs` — Track nearest port during edge creation

---

## 5. Live Edge Preview While Creating

**Current:** During `CreatingEdge`, no visual feedback of the connection path.
**Improved:** Draw a dashed bezier curve from source port to cursor, snapping to target port when close.

### Implementation
- During `DragState::CreatingEdge`, compute a bezier from source port pos to cursor pos
- Render as dashed line with 50% opacity in the theme accent color
- When near a valid target port, snap to it and show the curve as solid

### Files
- `src/app/render.rs` — Add edge preview rendering
- `src/app/canvas.rs` — Compute preview path data

---

## 6. Unsaved Changes Indicator

**Current:** No visual indication of unsaved changes.
**Improved:** Show a dot (●) next to the project title when the document has been modified since last save.

### Implementation
- Add `has_unsaved_changes: bool` field to `FlowchartApp`
- Set to `true` when `history.push()` is called
- Set to `false` after successful save
- Render a small colored dot next to the project title watermark
- Update window title: "openDraftly — myfile.spec ●" when dirty

### Files
- `src/app/mod.rs` — Add field
- `src/app/canvas.rs` — Render indicator, update window title
- `src/app/toolbar.rs` — Reset on save

---

## 7. Toolbar Shortcut Labels

**Current:** Toolbar tooltips say "Add Rectangle" without mentioning the shortcut key.
**Improved:** Tooltips include the shortcut: "Add Rectangle (R)".

### Implementation
- Update all toolbar button tooltips to include their keyboard shortcut
- Format: "Action Name (Key)" or "Action Name (Cmd+Key)"

### Files
- `src/app/toolbar.rs` — Update all `.on_hover_text()` calls

---

## 8. Enhanced Shortcuts Panel

**Current:** Shortcuts panel exists (`show_shortcuts_panel`) but may be sparse.
**Improved:** Comprehensive categorized panel with all shortcuts.

### Categories
- **Canvas:** Pan (Space+drag), Zoom (Cmd+/-, scroll), Fit (Cmd+0)
- **Nodes:** Rectangle (R), Circle (C), Diamond (D), Connector (O), Text (T)
- **Editing:** Delete (Del/Backspace), Duplicate (Cmd+D), Copy/Paste, Undo/Redo
- **Selection:** Select All (Cmd+A), Box Select (drag), Select Similar (Cmd+Shift+S)
- **View:** Grid (G), Minimap (M), 3D (3), Presentation (F), Focus (Cmd+Shift+F)
- **Workflow:** Status Cycle (S), Priority (P1-P4), Assign (A), Comment (C)

### Files
- `src/app/overlays.rs` — Enhance the shortcuts panel rendering

---

## 9. Multi-Selection Count Badge

**Current:** No visible count indicator while dragging multiple nodes.
**Improved:** Show a small badge "3 nodes" near the cursor while dragging a multi-selection.

### Implementation
- During `DragState::DraggingNode` with >1 selected nodes, render a floating badge
- Position: offset from cursor, pill-shaped background with count text

### Files
- `src/app/canvas.rs` — Add badge rendering during multi-drag

---

## 10. Connection Count Badges

**Current:** Edge counts shown only in statusbar on hover.
**Improved:** Small subtle badge showing in/out edge count on each node.

### Implementation
- For each node with edges, render a small "↑2 ↓3" label below the node
- Only show at zoom levels > 0.5 (hide at very zoomed out)
- Semi-transparent, smaller font size
- Toggle with a setting (default: off, activate via View menu or shortcut)

### Files
- `src/app/render.rs` — Add connection count label rendering
- `src/app/mod.rs` — Add `show_connection_counts: bool` field

---

## 11. Presentation Mode Enhancements

**Current:** Presentation mode exists but lacks slide counter and exit hint.
**Improved:** Add slide counter "3/5" and "Press F to exit" hint.

### Implementation
- Bottom-right corner: slide counter in pill badge "Slide 3 of 5"
- Bottom-center: fade-in hint "Press F to exit • ← → to navigate" (fades after 3s)
- Left/right arrow keys already work, just need the visual hints

### Files
- `src/app/canvas.rs` — Add presentation mode overlays

---

## 12. Property Field Improvements

**Current:** Date fields have no format hint. Color pickers don't show hex codes.
**Improved:** Date fields show "YYYY-MM-DD" placeholder. Color swatches show hex on hover.

### Implementation
- Date inputs: Use `hint_text("YYYY-MM-DD")` on TextEdit
- Color swatches: Add tooltip with hex code `#RRGGBB`
- URL fields: Add placeholder "https://..."
- Progress fields: Show "0-100" hint

### Files
- `src/app/properties.rs` — Add hint_text and tooltips

---

## 13. Friendly Error Messages

**Current:** Error messages are technical.
**Improved:** Use friendly language with actionable suggestions.

### Message Style Guide
- **Before:** "Parse error at line 3"
- **After:** "Hmm, something's off on line 3 — check for missing arrows (→) or brackets"
- **Before:** "File not found"
- **After:** "Can't find that file — it may have been moved or renamed"
- **Before:** "Invalid format"
- **After:** "This doesn't look like a .spec or .yaml file — try Cmd+E to write one"

### Files
- `src/app/overlays.rs` — Update error dialog text
- `src/specgraph/hrf.rs` — Improve parse error messages
- `src/app/toolbar.rs` — File operation error messages

---

## 14. Completion Celebration

**Current:** No visual celebration when a node reaches 100% progress.
**Improved:** Subtle sparkle animation when progress hits 100%.

### Implementation
- When a node's `progress` changes to `Some(100)`, add a sparkle effect
- Reuse the `creation_ripples` system but with a green color and slower decay
- Small "✓ Done!" toast

### Files
- `src/app/canvas.rs` — Detect progress change, trigger celebration
- `src/app/render.rs` — Render celebration sparkle

---

## 15. Smarter Context Menu Headers

**Current:** Context menu shows node label as header.
**Improved:** Show node type + label + status for better context.

### Example
- **Before:** "REST API"
- **After:** "⬡ REST API · WIP · P2"

### Files
- `src/app/context_menu.rs` — Enhance header display

---

## Architecture

All changes are additive — no breaking changes to existing data structures. The `StatusLevel` enum is the only new type. All other changes are rendering enhancements and field additions to `FlowchartApp`.

## Testing Strategy

- Manual testing of each feature via the GUI
- Verify existing HRF parser tests still pass (`cargo test`)
- Check that save/load roundtrip preserves all data
- Verify no performance regression at 100+ node diagrams

## Implementation Order

Priority order (highest impact first):
1. Color-coded status toasts + invalid action feedback (foundation for all other messaging)
2. Visual lock/pin indicators + unsaved changes indicator
3. Port hover glow + live edge preview
4. Toolbar shortcut labels + enhanced shortcuts panel
5. Multi-selection count badge + connection count badges
6. Presentation mode enhancements
7. Property field improvements + friendly errors
8. Completion celebration + smarter context menu headers
