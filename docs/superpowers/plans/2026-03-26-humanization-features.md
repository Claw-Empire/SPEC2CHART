# Humanization Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add humanized features and polish to openDraftly — smarter feedback, visual affordances, discoverability improvements, and delightful interactions.

**Architecture:** All changes are additive to the existing `FlowchartApp` struct and rendering pipeline. A new `StatusLevel` enum drives color-coded toasts. Visual indicators (lock/pin icons, port glow, edge preview, connection badges) are added to the rendering pass. No breaking changes to data model or serialization.

**Tech Stack:** Rust, egui 0.31, eframe

---

### Task 1: Color-Coded Status Toasts with StatusLevel

**Files:**
- Modify: `src/app/mod.rs:26-30` (add StatusLevel enum)
- Modify: `src/app/mod.rs:122` (change status_message type)
- Modify: `src/app/mod.rs:315-350` (update default initialization)
- Modify: `src/app/canvas.rs:2305-2376` (update draw_status_toast)

- [ ] **Step 1: Add StatusLevel enum and update status_message type in mod.rs**

In `src/app/mod.rs`, after the `Tool` enum (around line 30), add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Success,
    Info,
    Warning,
    Error,
}
```

Change the `status_message` field (line 122) from:
```rust
pub(crate) status_message: Option<(String, std::time::Instant)>,
```
to:
```rust
pub(crate) status_message: Option<(String, std::time::Instant, StatusLevel)>,
```

- [ ] **Step 2: Add a helper method to set status messages**

In `src/app/mod.rs`, inside the `impl FlowchartApp` block (around line 560), add:

```rust
    pub(crate) fn set_status(&mut self, msg: impl Into<String>, level: StatusLevel) {
        self.status_message = Some((msg.into(), std::time::Instant::now(), level));
    }
```

- [ ] **Step 3: Update draw_status_toast to use StatusLevel for colors**

In `src/app/canvas.rs`, replace the `draw_status_toast` method (lines 2305-2376) with:

```rust
    fn draw_status_toast(
        &self,
        painter: &egui::Painter,
        canvas_rect: Rect,
        ctx: &egui::Context,
    ) {
        if let Some((ref msg, time, level)) = self.status_message {
            let elapsed = time.elapsed().as_secs_f32();
            let fade_duration = match level {
                super::StatusLevel::Error => 5.0,
                super::StatusLevel::Warning => 3.5,
                _ => 2.0 + (msg.len() as f32 / 20.0).min(2.0) * 0.5,
            };
            if elapsed < fade_duration {
                let alpha = ((fade_duration - elapsed).min(1.0) * 255.0) as u8;
                let toast_pos = Pos2::new(canvas_rect.center().x, canvas_rect.max.y - 40.0);
                let font = FontId::proportional(12.0);

                // Level-based colors
                let (stripe_color, icon) = match level {
                    super::StatusLevel::Success => (
                        Color32::from_rgba_premultiplied(166, 227, 161, alpha),
                        "\u{2713} ", // ✓
                    ),
                    super::StatusLevel::Info => (
                        Color32::from_rgba_premultiplied(137, 180, 250, alpha),
                        "\u{2139} ", // ℹ
                    ),
                    super::StatusLevel::Warning => (
                        Color32::from_rgba_premultiplied(249, 226, 175, alpha),
                        "\u{26A0} ", // ⚠
                    ),
                    super::StatusLevel::Error => (
                        Color32::from_rgba_premultiplied(243, 139, 168, alpha),
                        "\u{2717} ", // ✗
                    ),
                };

                // Contextual icon override: use message-specific icons when they make more sense
                let msg_lower = msg.to_lowercase();
                let display_icon = if msg_lower.contains("copied") || msg_lower.contains("clipboard") {
                    "\u{2398} " // ⎘
                } else if msg_lower.contains("undo") {
                    "\u{21A9} " // ↩
                } else if msg_lower.contains("redo") {
                    "\u{21AA} " // ↪
                } else if msg_lower.contains("pasted") || msg_lower.contains("paste") {
                    "\u{2398} " // ⎘
                } else if msg_lower.contains("duplicated") {
                    "\u{2750} " // ❐
                } else if msg_lower.contains("template") || msg_lower.contains("loaded") {
                    "\u{2605} " // ★
                } else if msg_lower.contains("fresh canvas") || msg_lower.contains("new") {
                    "\u{2728} " // ✨
                } else {
                    icon
                };

                let galley = painter.layout_no_wrap(msg.clone(), font.clone(), stripe_color);
                let pill_rect = Rect::from_center_size(
                    toast_pos,
                    Vec2::new(galley.size().x + 28.0, galley.size().y + 14.0),
                );
                let bg_alpha = (alpha as f32 * 0.85) as u8;
                painter.rect_filled(
                    pill_rect,
                    CornerRadius::same(16),
                    self.theme.mantle.gamma_multiply(bg_alpha as f32 / 255.0),
                );
                // Left color stripe
                let stripe_rect = Rect::from_min_size(
                    pill_rect.left_top(),
                    Vec2::new(3.0, pill_rect.height()),
                );
                painter.rect_filled(stripe_rect, CornerRadius::same(16), stripe_color);
                painter.rect_stroke(
                    pill_rect,
                    CornerRadius::same(16),
                    Stroke::new(1.0, stripe_color.gamma_multiply(0.5)),
                    StrokeKind::Outside,
                );
                painter.text(
                    toast_pos,
                    Align2::CENTER_CENTER,
                    &format!("{}{}", display_icon, msg),
                    font,
                    stripe_color,
                );
                ctx.request_repaint();
            }
        }
    }
```

- [ ] **Step 4: Update all existing status_message assignments to use StatusLevel**

This is the largest sub-step. In `src/app/canvas.rs`, `src/app/shortcuts.rs`, `src/app/toolbar.rs`, `src/app/template_gallery.rs`, and `src/app/mod.rs`, change every instance of:
```rust
self.status_message = Some(("message".to_string(), std::time::Instant::now()));
```
to use the new `set_status` helper with an appropriate level. Use these rules:
- Messages about saving, applying, creating, connecting → `StatusLevel::Success`
- Messages about undo, redo, navigation, filter, zoom, grid → `StatusLevel::Info`
- Messages about "no selection", "can't", locked → `StatusLevel::Warning`
- Messages about errors, failures → `StatusLevel::Error`

For example:
```rust
// Before:
self.status_message = Some(("Style copied — Cmd+Shift+V to apply".to_string(), std::time::Instant::now()));
// After:
self.set_status("Style copied — Cmd+Shift+V to apply", StatusLevel::Success);
```

- [ ] **Step 5: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds with no errors.

- [ ] **Step 6: Commit**

```bash
git add src/app/mod.rs src/app/canvas.rs src/app/shortcuts.rs src/app/toolbar.rs src/app/template_gallery.rs
git commit -m "feat: color-coded status toasts with StatusLevel enum"
```

---

### Task 2: Invalid Action Feedback Messages

**Files:**
- Modify: `src/app/canvas.rs` (edge creation validation, locked node feedback)
- Modify: `src/app/shortcuts.rs` (alignment/selection validation)

- [ ] **Step 1: Add self-connection prevention feedback**

In `src/app/canvas.rs`, find where edges are created (search for `DragState::CreatingEdge` handling where source and target are compared). When `source.node_id == target_node_id`, add:

```rust
self.set_status("Can't connect a node to itself", StatusLevel::Warning);
```

- [ ] **Step 2: Add locked node drag feedback**

In `src/app/canvas.rs`, find the node drag initiation code. Before starting a `DragState::DraggingNode`, check if the primary selected node is locked:

```rust
if node.locked {
    self.set_status("Node is locked — Cmd+Shift+L to unlock", StatusLevel::Warning);
    return; // or skip drag initiation
}
```

- [ ] **Step 3: Add canvas-locked feedback**

In `src/app/canvas.rs`, where canvas_locked is checked, add feedback:

```rust
if self.canvas_locked {
    self.set_status("Canvas is locked — Cmd+Shift+K to unlock", StatusLevel::Warning);
}
```

- [ ] **Step 4: Add alignment validation feedback**

In `src/app/shortcuts.rs`, in the alignment commands section, add a check before alignment:

```rust
if selected_nodes.len() < 2 {
    self.set_status("Select 2+ nodes to align", StatusLevel::Warning);
    return;
}
```

- [ ] **Step 5: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 6: Commit**

```bash
git add src/app/canvas.rs src/app/shortcuts.rs
git commit -m "feat: invalid action feedback with friendly warning messages"
```

---

### Task 3: Visual Lock & Pin Indicators on Nodes

**Files:**
- Modify: `src/app/render.rs:95` (in draw_node, after main node rendering)

- [ ] **Step 1: Add lock/pin icon overlay in draw_node**

In `src/app/render.rs`, inside `draw_node()`, after the main node shape is rendered (after the label text is drawn, near the end of the method before the `}` closing brace), add:

```rust
        // Lock / pin indicator icons
        if node.locked || node.pinned {
            let icon_text = if node.locked { "🔒" } else { "📌" };
            let icon_size = (10.0 * self.viewport.zoom).clamp(8.0, 14.0);
            let icon_pos = Pos2::new(
                screen_rect.right() - icon_size - 2.0,
                screen_rect.top() + 2.0,
            );
            let icon_bg_rect = Rect::from_min_size(
                icon_pos - Vec2::new(2.0, 1.0),
                Vec2::new(icon_size + 4.0, icon_size + 4.0),
            );
            painter.rect_filled(
                icon_bg_rect,
                CornerRadius::same(3),
                Color32::from_rgba_unmultiplied(0, 0, 0, 120),
            );
            painter.text(
                icon_bg_rect.center(),
                Align2::CENTER_CENTER,
                icon_text,
                FontId::proportional(icon_size),
                Color32::from_rgba_unmultiplied(205, 214, 244, 200),
            );
        }
```

- [ ] **Step 2: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/app/render.rs
git commit -m "feat: visual lock and pin indicators on nodes"
```

---

### Task 4: Port Hover Glow During Edge Creation

**Files:**
- Modify: `src/app/render.rs` (add port glow rendering)
- Modify: `src/app/mod.rs` (add nearest_port field)

- [ ] **Step 1: Add nearest_port tracking field**

In `src/app/mod.rs`, after the `hover_node_id` field (line ~207), add:

```rust
    /// Nearest valid port during edge creation (for glow effect)
    pub(crate) nearest_port: Option<(Pos2, Port)>,
```

Initialize it to `None` in the `new()` constructor.

- [ ] **Step 2: Compute nearest port during edge creation in canvas.rs**

In `src/app/canvas.rs`, inside the input handling section where `DragState::CreatingEdge` is processed, compute the nearest port:

```rust
if let DragState::CreatingEdge { ref source, current_screen } = self.drag {
    let cursor_canvas = self.viewport.screen_to_canvas(current_screen);
    let mut best: Option<(f32, Pos2, Port)> = None;
    for node in &self.document.nodes {
        if node.id == source.node_id { continue; }
        for side in &[PortSide::Top, PortSide::Bottom, PortSide::Left, PortSide::Right] {
            let port = Port { node_id: node.id, side: *side };
            let port_canvas = node.port_pos(*side);
            let port_screen = self.viewport.canvas_to_screen(port_canvas);
            let dist = port_screen.distance(current_screen);
            if dist < 30.0 {
                if best.as_ref().map_or(true, |(d, _, _)| dist < *d) {
                    best = Some((dist, port_screen, port));
                }
            }
        }
    }
    self.nearest_port = best.map(|(_, pos, port)| (pos, port));
}
```

- [ ] **Step 3: Render port glow in render.rs**

In `src/app/render.rs`, add a new method:

```rust
    pub(crate) fn draw_port_hover_glow(&self, painter: &egui::Painter) {
        if let Some((screen_pos, _port)) = self.nearest_port {
            let time = painter.ctx().input(|i| i.time);
            let pulse = ((time * 3.0).sin() as f32) * 0.3 + 0.7;
            let alpha = (pulse * 180.0) as u8;
            let glow_color = Color32::from_rgba_premultiplied(
                self.theme.accent.r(), self.theme.accent.g(), self.theme.accent.b(), alpha,
            );
            painter.circle_stroke(
                screen_pos,
                PORT_RADIUS * self.viewport.zoom + 4.0,
                Stroke::new(2.0, glow_color),
            );
            painter.circle_filled(
                screen_pos,
                PORT_RADIUS * self.viewport.zoom + 2.0,
                Color32::from_rgba_premultiplied(
                    self.theme.accent.r(), self.theme.accent.g(), self.theme.accent.b(), 40,
                ),
            );
            painter.ctx().request_repaint_after(std::time::Duration::from_millis(33));
        }
    }
```

Call this from `draw_canvas()` in `canvas.rs` right after `self.draw_edge_tooltip(...)`.

- [ ] **Step 4: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/app/mod.rs src/app/canvas.rs src/app/render.rs
git commit -m "feat: port hover glow during edge creation"
```

---

### Task 5: Live Edge Preview While Creating

**Files:**
- Modify: `src/app/render.rs` (add edge preview rendering)
- Modify: `src/app/canvas.rs` (call the new method)

- [ ] **Step 1: Add edge preview rendering method**

In `src/app/render.rs`, add:

```rust
    pub(crate) fn draw_edge_creation_preview(&self, painter: &egui::Painter) {
        if let super::DragState::CreatingEdge { ref source, current_screen } = self.drag {
            // Get source port screen position
            let source_node = self.document.find_node(&source.node_id);
            let Some(source_node) = source_node else { return };
            let source_canvas = source_node.port_pos(source.side);
            let source_screen = self.viewport.canvas_to_screen(source_canvas);

            // Target: snap to nearest port if available, else use cursor
            let target_screen = if let Some((port_pos, _)) = self.nearest_port {
                port_pos
            } else {
                current_screen
            };

            // Compute bezier control points
            let dx = (target_screen.x - source_screen.x).abs() * 0.4;
            let dy = (target_screen.y - source_screen.y).abs() * 0.4;
            let offset = dx.max(dy).max(40.0);
            let cp1 = match source.side {
                PortSide::Right => source_screen + Vec2::new(offset, 0.0),
                PortSide::Left => source_screen + Vec2::new(-offset, 0.0),
                PortSide::Bottom => source_screen + Vec2::new(0.0, offset),
                PortSide::Top => source_screen + Vec2::new(0.0, -offset),
            };
            let cp2 = target_screen + Vec2::new(0.0, -offset * 0.5);

            // Draw dashed bezier
            let is_snapped = self.nearest_port.is_some();
            let alpha = if is_snapped { 180u8 } else { 100u8 };
            let color = Color32::from_rgba_premultiplied(
                self.theme.accent.r(), self.theme.accent.g(), self.theme.accent.b(), alpha,
            );
            let segments = 30;
            for i in 0..segments {
                let t0 = i as f32 / segments as f32;
                let t1 = (i + 1) as f32 / segments as f32;
                // Skip odd segments for dashed effect (unless snapped → solid)
                if !is_snapped && i % 3 == 2 { continue; }
                let p0 = cubic_bezier_point(source_screen, cp1, cp2, target_screen, t0);
                let p1 = cubic_bezier_point(source_screen, cp1, cp2, target_screen, t1);
                painter.line_segment([p0, p1], Stroke::new(1.5, color));
            }

            // Arrow head at target
            let tip = target_screen;
            let prev = cubic_bezier_point(source_screen, cp1, cp2, target_screen, 0.95);
            let dir = (tip - prev).normalized();
            let perp = Vec2::new(-dir.y, dir.x);
            let arrow_size = 8.0;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    tip,
                    tip - dir * arrow_size + perp * arrow_size * 0.4,
                    tip - dir * arrow_size - perp * arrow_size * 0.4,
                ],
                color,
                Stroke::NONE,
            ));
        }
    }
```

- [ ] **Step 2: Call from draw_canvas**

In `src/app/canvas.rs`, in `draw_canvas()`, right after `self.draw_port_hover_glow(...)`, add:

```rust
        self.draw_edge_creation_preview(&painter);
```

- [ ] **Step 3: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/app/render.rs src/app/canvas.rs
git commit -m "feat: live edge preview curve while creating connections"
```

---

### Task 6: Unsaved Changes Indicator

**Files:**
- Modify: `src/app/mod.rs` (add has_unsaved_changes field)
- Modify: `src/app/canvas.rs` (render indicator, update window title)
- Modify: `src/app/shortcuts.rs` or `src/app/toolbar.rs` (set dirty on history push, clear on save)

- [ ] **Step 1: Add has_unsaved_changes field**

In `src/app/mod.rs`, after `autosave_status` (line ~286), add:

```rust
    /// True when the document has been modified since last save
    pub(crate) has_unsaved_changes: bool,
```

Initialize to `false` in the `new()` constructor.

- [ ] **Step 2: Set dirty flag on document changes**

In every place `self.history.push(&self.document)` is called, add after it:

```rust
self.has_unsaved_changes = true;
```

A more efficient approach: create a helper method in `mod.rs`:

```rust
    pub(crate) fn push_history(&mut self) {
        self.history.push(&self.document);
        self.has_unsaved_changes = true;
        self.autosave_dirty = true;
    }
```

Then do a find-and-replace of `self.history.push(&self.document);` → `self.push_history();` across all files. But also keep the separate `self.autosave_dirty = true;` calls that may exist — deduplicate them into the helper.

- [ ] **Step 3: Clear dirty flag on save**

In `src/app/toolbar.rs`, in the save handler (search for `save` or `Cmd+S`), after a successful save add:

```rust
self.has_unsaved_changes = false;
```

- [ ] **Step 4: Render unsaved indicator dot**

In `src/app/canvas.rs`, in the watermark/title rendering section, when `has_unsaved_changes` is true, append a dot:

```rust
let title_suffix = if self.has_unsaved_changes { " ●" } else { "" };
// Use title_suffix when rendering the project_title watermark
```

Also update the window title via `ctx.send_viewport_cmd(...)`:

```rust
let window_title = if self.has_unsaved_changes {
    format!("openDraftly — {} ●", self.current_file_path.as_ref().map_or("Untitled", |p| p.file_name().unwrap_or_default().to_str().unwrap_or("Untitled")))
} else {
    format!("openDraftly — {}", self.current_file_path.as_ref().map_or("Untitled", |p| p.file_name().unwrap_or_default().to_str().unwrap_or("Untitled")))
};
ctx.send_viewport_cmd(egui::ViewportCommand::Title(window_title));
```

- [ ] **Step 5: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 6: Commit**

```bash
git add src/app/mod.rs src/app/canvas.rs src/app/toolbar.rs src/app/shortcuts.rs
git commit -m "feat: unsaved changes indicator dot in title and watermark"
```

---

### Task 7: Toolbar Shortcut Labels

**Files:**
- Modify: `src/app/toolbar.rs` (update all on_hover_text calls)

- [ ] **Step 1: Update shape button tooltips**

In `src/app/toolbar.rs`, find `draw_flowchart_shapes` and update each shape button's tooltip to include the shortcut key:

Key mappings to add:
- Rectangle → "(R)"
- Rounded Rectangle → "(Shift+R)"
- Diamond → "(D)"
- Circle → "(C)"
- Connector/Pill → "(O)"
- Text → "(T)"
- Frame → "(F key — hold to place)"

Update each `.on_hover_text("Click or drag onto canvas")` for shape buttons to include the shortcut.

- [ ] **Step 2: Update tool button tooltips**

Already mostly done — verify and enhance:
- Select tool: "Select & move nodes (V)"
- Connect tool: "Connect nodes (E)"
- Undo: "Undo (⌘Z)" ✓
- Redo: "Redo (⌘⇧Z)" ✓

- [ ] **Step 3: Update view/mode button tooltips**

Add shortcut keys to view toggles:
- Grid: "Toggle grid (G)"
- Minimap: "Toggle minimap (M)"
- 3D view: "Toggle 3D view (3)"
- Heatmap: "Toggle heatmap (H)"

- [ ] **Step 4: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/app/toolbar.rs
git commit -m "feat: keyboard shortcut labels on all toolbar tooltips"
```

---

### Task 8: Enhanced Shortcuts Cheat Sheet Panel

**Files:**
- Modify: `src/app/overlays.rs` (enhance shortcuts panel content)

- [ ] **Step 1: Enhance the shortcuts panel with categorized sections**

In `src/app/overlays.rs`, find the shortcuts panel rendering (search for `show_shortcuts_panel`). Replace or enhance with a comprehensive categorized layout:

```rust
// Categories with (key, description) pairs
let categories: &[(&str, &[(&str, &str)])] = &[
    ("Canvas", &[
        ("Space + Drag", "Pan canvas"),
        ("⌘ + / ⌘ -", "Zoom in/out"),
        ("⌘ 0", "Fit to screen"),
        ("Scroll", "Pan up/down"),
        ("⌘ Scroll", "Zoom"),
        ("G", "Toggle grid"),
        ("M", "Toggle minimap"),
        ("3", "Toggle 3D view"),
    ]),
    ("Create Nodes", &[
        ("Double-click", "New node at cursor"),
        ("R", "Rectangle"),
        ("C", "Circle"),
        ("D", "Diamond"),
        ("O", "Connector/Pill"),
        ("T", "Text node"),
        ("N", "New sticky note"),
    ]),
    ("Selection", &[
        ("Click", "Select node"),
        ("⌘ A", "Select all"),
        ("Shift + Click", "Add to selection"),
        ("Drag (empty)", "Box select"),
        ("Esc", "Deselect all"),
    ]),
    ("Editing", &[
        ("Delete / ⌫", "Delete selected"),
        ("⌘ D", "Duplicate"),
        ("⌘ C / ⌘ V", "Copy / Paste"),
        ("⌘ Z / ⌘ ⇧ Z", "Undo / Redo"),
        ("Enter", "Edit label"),
        ("S", "Cycle status"),
        ("A", "Quick assign"),
    ]),
    ("View", &[
        ("F / F5", "Presentation mode"),
        ("⌘ ⇧ F", "Focus mode"),
        ("H", "Toggle heatmap"),
        ("⇧ A", "Toggle flow animation"),
        ("⇧ R", "Toggle rulers"),
        ("⌘ K", "Command palette"),
        ("?", "This shortcuts panel"),
    ]),
];
```

Render each category as a section header with a grid of key-description pairs. Use `ui.columns(2, ...)` for compact layout.

- [ ] **Step 2: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/app/overlays.rs
git commit -m "feat: enhanced categorized shortcuts cheat sheet panel"
```

---

### Task 9: Multi-Selection Count Badge While Dragging

**Files:**
- Modify: `src/app/canvas.rs` (add badge rendering during multi-drag)

- [ ] **Step 1: Add multi-drag count badge**

In `src/app/canvas.rs`, inside `draw_canvas()`, after the main node rendering loop, find where `DragState::DraggingNode` handling occurs. Add a badge when dragging multiple nodes:

```rust
        // Multi-selection drag count badge
        if let DragState::DraggingNode { .. } = &self.drag {
            let n = self.selection.node_ids.len();
            if n > 1 {
                if let Some(mouse_pos) = pointer_pos {
                    let badge_text = format!("{} nodes", n);
                    let font = FontId::proportional(11.0);
                    let badge_pos = mouse_pos + Vec2::new(16.0, -20.0);
                    let galley = painter.layout_no_wrap(badge_text.clone(), font.clone(), self.theme.text_primary);
                    let badge_rect = Rect::from_min_size(
                        badge_pos - Vec2::new(4.0, 2.0),
                        galley.size() + Vec2::new(8.0, 4.0),
                    );
                    painter.rect_filled(badge_rect, CornerRadius::same(8), self.theme.accent);
                    painter.text(
                        badge_rect.center(),
                        Align2::CENTER_CENTER,
                        &badge_text,
                        font,
                        Color32::from_rgb(30, 30, 46),
                    );
                }
            }
        }
```

- [ ] **Step 2: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/app/canvas.rs
git commit -m "feat: multi-selection count badge while dragging nodes"
```

---

### Task 10: Connection Count Badges on Nodes

**Files:**
- Modify: `src/app/mod.rs` (add show_connection_counts field)
- Modify: `src/app/render.rs` (render badges below nodes)
- Modify: `src/app/shortcuts.rs` (toggle shortcut)

- [ ] **Step 1: Add toggle field**

In `src/app/mod.rs`, add field:

```rust
    /// Show incoming/outgoing edge count badges below each node
    pub(crate) show_connection_counts: bool,
```

Initialize to `false` in `new()`.

- [ ] **Step 2: Add toggle shortcut**

In `src/app/shortcuts.rs`, add a shortcut (e.g., Shift+C or a command palette entry):

```rust
        // Shift+C = toggle connection count badges
        if !any_text_focused && ctx.input(|i| i.key_pressed(Key::C) && i.modifiers.shift && !i.modifiers.command) {
            self.show_connection_counts = !self.show_connection_counts;
            let state = if self.show_connection_counts { "on" } else { "off" };
            self.set_status(format!("Connection counts: {}", state), StatusLevel::Info);
        }
```

- [ ] **Step 3: Render connection count badges in render.rs**

In `src/app/render.rs`, at the end of `draw_node()`, before the lock/pin indicator code, add:

```rust
        // Connection count badge
        if self.show_connection_counts && self.viewport.zoom > 0.5 {
            let in_count = self.document.edges.iter()
                .filter(|e| e.target.node_id == node.id)
                .count();
            let out_count = self.document.edges.iter()
                .filter(|e| e.source.node_id == node.id)
                .count();
            if in_count > 0 || out_count > 0 {
                let badge_text = format!("↓{} ↑{}", in_count, out_count);
                let badge_font = FontId::proportional((8.0 * self.viewport.zoom).clamp(7.0, 10.0));
                let badge_pos = Pos2::new(
                    screen_rect.center().x,
                    screen_rect.bottom() + 8.0 * self.viewport.zoom,
                );
                painter.text(
                    badge_pos,
                    Align2::CENTER_TOP,
                    &badge_text,
                    badge_font,
                    Color32::from_rgba_unmultiplied(
                        self.theme.text_dim.r(), self.theme.text_dim.g(),
                        self.theme.text_dim.b(), 160,
                    ),
                );
            }
        }
```

- [ ] **Step 4: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/app/mod.rs src/app/render.rs src/app/shortcuts.rs
git commit -m "feat: toggleable connection count badges on nodes (Shift+C)"
```

---

### Task 11: Presentation Mode Enhancements

**Files:**
- Modify: `src/app/canvas.rs` (add slide counter and exit hint overlays)

- [ ] **Step 1: Add presentation mode HUD**

In `src/app/canvas.rs`, in `draw_canvas()`, find the presentation mode section. Add a slide counter and navigation hint overlay:

```rust
        // Presentation mode HUD
        if self.presentation_mode && !self.presentation_slides.is_empty() {
            let total = self.presentation_slides.len();
            let current = self.presentation_slide_index + 1;

            // Slide counter pill — bottom right
            let counter_text = format!("{} / {}", current, total);
            let counter_font = FontId::proportional(13.0);
            let counter_pos = Pos2::new(canvas_rect.right() - 60.0, canvas_rect.bottom() - 30.0);
            let galley = painter.layout_no_wrap(counter_text.clone(), counter_font.clone(), self.theme.text_primary);
            let pill = Rect::from_center_size(counter_pos, galley.size() + Vec2::new(16.0, 8.0));
            painter.rect_filled(pill, CornerRadius::same(12), Color32::from_rgba_unmultiplied(30, 30, 46, 200));
            painter.text(counter_pos, Align2::CENTER_CENTER, &counter_text, counter_font, self.theme.text_secondary);

            // Navigation hint — bottom center, fades after 3s
            let time = ui.ctx().input(|i| i.time);
            let pres_start = self.node_birth_times.get(&NodeId(uuid::Uuid::nil())).copied().unwrap_or(time);
            let hint_age = (time - pres_start) as f32;
            if hint_age < 4.0 {
                let hint_alpha = ((4.0 - hint_age).min(1.0) * 200.0) as u8;
                let hint_text = "← → navigate  •  F to exit";
                let hint_font = FontId::proportional(11.0);
                let hint_pos = Pos2::new(canvas_rect.center().x, canvas_rect.bottom() - 30.0);
                painter.text(
                    hint_pos,
                    Align2::CENTER_CENTER,
                    hint_text,
                    hint_font,
                    Color32::from_rgba_unmultiplied(205, 214, 244, hint_alpha),
                );
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
            }
        }
```

- [ ] **Step 2: Record presentation start time**

In `src/app/mod.rs`, in `enter_presentation_mode()`, after setting `self.presentation_mode = true`, add:

```rust
// Use nil UUID as a sentinel key for presentation start time
self.node_birth_times.insert(NodeId(uuid::Uuid::nil()), painter_time);
```

Actually, since we don't have painter time there, use a simpler approach: add a `presentation_start_time: Option<f64>` field to `FlowchartApp`, set it in `enter_presentation_mode`, and use it in the HUD rendering.

- [ ] **Step 3: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/app/canvas.rs src/app/mod.rs
git commit -m "feat: presentation mode slide counter and navigation hints"
```

---

### Task 12: Property Field Improvements

**Files:**
- Modify: `src/app/properties.rs` (add hint text and tooltips)

- [ ] **Step 1: Add format hints to property fields**

In `src/app/properties.rs`, find date input fields and add `hint_text`:

```rust
// For date fields (created_date, due date, etc.):
egui::TextEdit::singleline(&mut node.created_date)
    .hint_text("YYYY-MM-DD")
    // ... existing code
```

For URL fields:
```rust
egui::TextEdit::singleline(&mut node.url)
    .hint_text("https://...")
    // ... existing code
```

- [ ] **Step 2: Add hex code tooltips on color swatches**

In `src/app/properties.rs`, find color swatch buttons and update their tooltips:

```rust
// For color buttons, instead of just the color name, include hex:
let hex = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
.on_hover_text(format!("{} ({})", name, hex))
```

- [ ] **Step 3: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/app/properties.rs
git commit -m "feat: property field hints and hex code tooltips on color swatches"
```

---

### Task 13: Smarter Context Menu Headers

**Files:**
- Modify: `src/app/context_menu.rs:155-175` (enhance node context menu header)

- [ ] **Step 1: Enhance context menu node header**

In `src/app/context_menu.rs`, in `context_menu_node()`, replace the header rendering (lines ~158-174) with:

```rust
        if let Some(node) = self.document.find_node(&node_id) {
            let label = node.display_label();
            let conn_in = self.document.edges.iter().filter(|e| e.target.node_id == node_id).count();
            let conn_out = self.document.edges.iter().filter(|e| e.source.node_id == node_id).count();

            // Shape icon
            let shape_icon = match &node.kind {
                NodeKind::Shape { shape, .. } => match shape {
                    NodeShape::Rectangle => "▭",
                    NodeShape::RoundedRect => "▢",
                    NodeShape::Diamond => "◇",
                    NodeShape::Circle => "○",
                    NodeShape::Parallelogram => "▱",
                    NodeShape::Hexagon => "⬡",
                    NodeShape::Triangle => "△",
                    NodeShape::Connector => "⬬",
                    NodeShape::Callout => "💬",
                    NodeShape::Person => "👤",
                    NodeShape::Screen => "🖥",
                    NodeShape::Cylinder => "🗄",
                    NodeShape::Cloud => "☁",
                    NodeShape::Document => "📄",
                    NodeShape::Channel => "📡",
                    NodeShape::Segment => "◔",
                },
                NodeKind::StickyNote { .. } => "📝",
                NodeKind::Entity { .. } => "📋",
                NodeKind::Text { .. } => "T",
            };

            // Status badge
            let status_str = if node.progress > 0.0 {
                if node.progress >= 100.0 { " · ✓ Done" }
                else { " · WIP" }
            } else { "" };

            // Priority badge
            let priority_str = match node.priority {
                1 => " · P1",
                2 => " · P2",
                3 => " · P3",
                4 => " · P4",
                _ => "",
            };

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{} {}", shape_icon, label))
                    .size(11.0).strong().color(self.theme.text_primary));
                let meta = format!("{}{}", status_str, priority_str);
                if !meta.is_empty() {
                    ui.label(egui::RichText::new(&meta).size(9.0).color(self.theme.text_dim));
                }
            });
            if conn_in > 0 || conn_out > 0 {
                ui.label(egui::RichText::new(format!("↓{} in  ↑{} out", conn_in, conn_out))
                    .size(9.0).color(self.theme.text_dim));
            }
            ui.separator();
        }
```

- [ ] **Step 2: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/app/context_menu.rs
git commit -m "feat: richer context menu headers with shape icon, status, and priority"
```

---

### Task 14: Completion Celebration Effect

**Files:**
- Modify: `src/app/canvas.rs` (detect progress → 100%, trigger celebration)
- Modify: `src/app/mod.rs` (add celebration ripples field)

- [ ] **Step 1: Add celebration tracking**

In `src/app/mod.rs`, add field:

```rust
    /// Celebration sparkle effects: (world_center, birth_time_secs)
    pub(crate) celebration_sparkles: Vec<([f32; 2], f64)>,
```

Initialize to `Vec::new()` in `new()`.

- [ ] **Step 2: Detect progress reaching 100% and trigger celebration**

In `src/app/properties.rs` or wherever progress is changed, after setting a node's progress to 100:

```rust
if node.progress >= 100.0 {
    let center = [node.position[0] + node.size[0] / 2.0, node.position[1] + node.size[1] / 2.0];
    self.celebration_sparkles.push((center, ui.ctx().input(|i| i.time)));
    self.set_status("✨ Task complete!", StatusLevel::Success);
}
```

- [ ] **Step 3: Render celebration sparkles**

In `src/app/canvas.rs`, near the creation_ripples rendering code, add celebration sparkle rendering:

```rust
        // Celebration sparkles (progress → 100%)
        self.celebration_sparkles.retain(|&(_, birth)| {
            let age = time - birth;
            age < 1.5 // live for 1.5 seconds
        });
        for &(center_canvas, birth) in &self.celebration_sparkles {
            let age = (time - birth) as f32;
            if age < 1.5 {
                let screen_center = self.viewport.canvas_to_screen(Pos2::new(center_canvas[0], center_canvas[1]));
                let t = age / 1.5;
                let radius = 20.0 + t * 40.0;
                let alpha = ((1.0 - t) * 200.0) as u8;
                // Green success ring expanding outward
                painter.circle_stroke(
                    screen_center,
                    radius * self.viewport.zoom,
                    Stroke::new(2.0, Color32::from_rgba_premultiplied(166, 227, 161, alpha)),
                );
                // Small star particles
                for j in 0..6 {
                    let angle = (j as f32 / 6.0) * std::f32::consts::TAU + age * 2.0;
                    let star_r = radius * 0.8 * self.viewport.zoom;
                    let star_pos = screen_center + Vec2::new(angle.cos() * star_r, angle.sin() * star_r);
                    painter.circle_filled(
                        star_pos,
                        (2.0 * (1.0 - t)).max(0.5),
                        Color32::from_rgba_premultiplied(249, 226, 175, alpha),
                    );
                }
                painter.ctx().request_repaint_after(std::time::Duration::from_millis(16));
            }
        }
```

- [ ] **Step 4: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/app/mod.rs src/app/canvas.rs src/app/properties.rs
git commit -m "feat: celebration sparkle effect when node progress reaches 100%"
```

---

### Task 15: Friendly Error Messages

**Files:**
- Modify: `src/app/overlays.rs` (update error dialog text)
- Modify: `src/app/toolbar.rs` (file operation error messages)

- [ ] **Step 1: Improve file operation error messages**

In `src/app/toolbar.rs`, find file save/load error handling. Replace technical messages:

```rust
// Before:
"File not found"
// After:
"Can't find that file — it may have been moved or renamed"

// Before:
"Invalid format"
// After:
"This doesn't look like a .spec or .yaml file — try Cmd+E to create one"

// Before:
"Parse error"
// After:
"Something's not quite right in this file — check the format and try again"
```

- [ ] **Step 2: Improve spec editor error messages**

In `src/app/overlays.rs`, find where `spec_editor_error` is displayed. Make error messages more friendly:

```rust
// Wrap parser errors with context:
let friendly = if err.contains("line") {
    format!("Hmm, something's off — {}\nTip: check for missing arrows (→) or unmatched brackets", err)
} else {
    format!("Could not parse the spec — {}", err)
};
```

- [ ] **Step 3: Build and verify**

Run: `cargo build 2>&1 | head -30`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/app/toolbar.rs src/app/overlays.rs
git commit -m "feat: friendly human-readable error messages with suggestions"
```
