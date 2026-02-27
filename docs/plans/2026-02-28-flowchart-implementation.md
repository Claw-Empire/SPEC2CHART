# Light Figma — Flowchart App Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Figma-inspired drag-and-drop flowchart application as a native macOS app in pure Rust.

**Architecture:** egui + eframe immediate-mode GUI with a central `FlowchartApp` state struct. Canvas renders nodes/edges with zoom/pan. Sidebar toolbar for node creation, properties panel for editing. JSON serialization for save/load, image export via dedicated crates.

**Tech Stack:** Rust, egui, eframe, serde, serde_json, uuid, image, printpdf, cargo-bundle

---

## Batch 1: Project Scaffold + Data Model

### Task 1: Initialize Cargo project with dependencies

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "light-figma"
version = "0.1.0"
edition = "2021"
description = "A Figma-inspired flowchart application"

[dependencies]
eframe = "0.31"
egui = "0.31"
egui_extras = { version = "0.31", features = ["image"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
image = "0.25"
printpdf = "0.7"
rfd = "0.15"

[package.metadata.bundle]
name = "Light Figma"
identifier = "com.lightfigma.app"
icon = ["icons/icon.icns"]
category = "public.app-category.graphics-design"
```

**Step 2: Create minimal main.rs that launches an empty eframe window**

```rust
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Light Figma"),
        ..Default::default()
    };
    eframe::run_native(
        "Light Figma",
        options,
        Box::new(|cc| Ok(Box::new(LightFigmaApp::new(cc)))),
    )
}

struct LightFigmaApp;

impl LightFigmaApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self
    }
}

impl eframe::App for LightFigmaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Light Figma");
        });
    }
}
```

**Step 3: Build and run**

Run: `cargo run`
Expected: A window opens with "Light Figma" heading.

**Step 4: Commit**

```bash
git init
git add Cargo.toml src/main.rs
git commit -m "feat: initialize project with eframe scaffold"
```

---

### Task 2: Define core data model

**Files:**
- Create: `src/model.rs`
- Modify: `src/main.rs` (add `mod model;`)

**Step 1: Create src/model.rs with all core types**

```rust
use egui::{Color32, Pos2, Vec2};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub Uuid);

impl EdgeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeShape {
    Rectangle,
    RoundedRect,
    Diamond,
    Circle,
    Parallelogram,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortSide {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStyle {
    pub fill_color: [u8; 4],
    pub border_color: [u8; 4],
    pub border_width: f32,
    pub text_color: [u8; 4],
    pub font_size: f32,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            fill_color: [255, 255, 255, 255],
            border_color: [60, 60, 60, 255],
            border_width: 2.0,
            text_color: [30, 30, 30, 255],
            font_size: 14.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub shape: NodeShape,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub label: String,
    pub description: String,
    pub style: NodeStyle,
}

impl Node {
    pub fn new(shape: NodeShape, position: Pos2) -> Self {
        let size = match shape {
            NodeShape::Circle => [80.0, 80.0],
            NodeShape::Diamond => [120.0, 100.0],
            _ => [140.0, 60.0],
        };
        Self {
            id: NodeId::new(),
            shape,
            position: [position.x, position.y],
            size,
            label: String::from("New Node"),
            description: String::new(),
            style: NodeStyle::default(),
        }
    }

    pub fn pos(&self) -> Pos2 {
        Pos2::new(self.position[0], self.position[1])
    }

    pub fn set_pos(&mut self, pos: Pos2) {
        self.position = [pos.x, pos.y];
    }

    pub fn size_vec(&self) -> Vec2 {
        Vec2::new(self.size[0], self.size[1])
    }

    pub fn rect(&self) -> egui::Rect {
        egui::Rect::from_min_size(self.pos(), self.size_vec())
    }

    pub fn port_position(&self, side: PortSide) -> Pos2 {
        let rect = self.rect();
        match side {
            PortSide::Top => rect.center_top(),
            PortSide::Bottom => rect.center_bottom(),
            PortSide::Left => rect.left_center(),
            PortSide::Right => rect.right_center(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub node_id: NodeId,
    pub side: PortSide,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeStyle {
    pub color: [u8; 4],
    pub width: f32,
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self {
            color: [100, 100, 100, 255],
            width: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub source: Port,
    pub target: Port,
    pub label: String,
    pub style: EdgeStyle,
}

impl Edge {
    pub fn new(source: Port, target: Port) -> Self {
        Self {
            id: EdgeId::new(),
            source,
            target,
            label: String::new(),
            style: EdgeStyle::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub offset: [f32; 2],
    pub zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            zoom: 1.0,
        }
    }
}

impl Viewport {
    pub fn screen_to_canvas(&self, screen_pos: Pos2) -> Pos2 {
        Pos2::new(
            (screen_pos.x - self.offset[0]) / self.zoom,
            (screen_pos.y - self.offset[1]) / self.zoom,
        )
    }

    pub fn canvas_to_screen(&self, canvas_pos: Pos2) -> Pos2 {
        Pos2::new(
            canvas_pos.x * self.zoom + self.offset[0],
            canvas_pos.y * self.zoom + self.offset[1],
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Selection {
    pub node_ids: Vec<NodeId>,
    pub edge_ids: Vec<EdgeId>,
}

impl Selection {
    pub fn clear(&mut self) {
        self.node_ids.clear();
        self.edge_ids.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.node_ids.is_empty() && self.edge_ids.is_empty()
    }

    pub fn contains_node(&self, id: &NodeId) -> bool {
        self.node_ids.contains(id)
    }

    pub fn contains_edge(&self, id: &EdgeId) -> bool {
        self.edge_ids.contains(id)
    }

    pub fn toggle_node(&mut self, id: NodeId) {
        if let Some(pos) = self.node_ids.iter().position(|n| *n == id) {
            self.node_ids.remove(pos);
        } else {
            self.node_ids.push(id);
        }
    }

    pub fn select_node(&mut self, id: NodeId) {
        self.clear();
        self.node_ids.push(id);
    }

    pub fn select_edge(&mut self, id: EdgeId) {
        self.clear();
        self.edge_ids.push(id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowchartDocument {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl Default for FlowchartDocument {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

impl FlowchartDocument {
    pub fn find_node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == *id)
    }

    pub fn find_node_mut(&mut self, id: &NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == *id)
    }

    pub fn find_edge(&self, id: &EdgeId) -> Option<&Edge> {
        self.edges.iter().find(|e| e.id == *id)
    }

    pub fn find_edge_mut(&mut self, id: &EdgeId) -> Option<&mut Edge> {
        self.edges.iter_mut().find(|e| e.id == *id)
    }

    pub fn node_at_pos(&self, pos: Pos2) -> Option<NodeId> {
        // Iterate in reverse so topmost (last drawn) nodes are hit first
        for node in self.nodes.iter().rev() {
            if node.rect().contains(pos) {
                return Some(node.id);
            }
        }
        None
    }

    pub fn remove_node(&mut self, id: &NodeId) {
        self.edges.retain(|e| e.source.node_id != *id && e.target.node_id != *id);
        self.nodes.retain(|n| n.id != *id);
    }

    pub fn remove_edge(&mut self, id: &EdgeId) {
        self.edges.retain(|e| e.id != *id);
    }
}
```

**Step 2: Add `mod model;` to main.rs**

Add `mod model;` at top of `src/main.rs`.

**Step 3: Build to verify**

Run: `cargo build`
Expected: Compiles without errors.

**Step 4: Commit**

```bash
git add src/model.rs src/main.rs
git commit -m "feat: add core data model (Node, Edge, Viewport, Selection)"
```

---

### Task 3: Add undo/redo history system

**Files:**
- Create: `src/history.rs`
- Modify: `src/main.rs` (add `mod history;`)

**Step 1: Create src/history.rs**

```rust
use crate::model::FlowchartDocument;

#[derive(Debug, Clone)]
pub struct UndoStack {
    states: Vec<FlowchartDocument>,
    current: usize,
    max_size: usize,
}

impl UndoStack {
    pub fn new(max_size: usize) -> Self {
        Self {
            states: Vec::new(),
            current: 0,
            max_size,
        }
    }

    pub fn push(&mut self, doc: &FlowchartDocument) {
        // Remove any future states (if we undid and are now making a new change)
        self.states.truncate(self.current);
        self.states.push(doc.clone());
        if self.states.len() > self.max_size {
            self.states.remove(0);
        }
        self.current = self.states.len();
    }

    pub fn undo(&mut self) -> Option<&FlowchartDocument> {
        if self.current > 1 {
            self.current -= 1;
            Some(&self.states[self.current - 1])
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<&FlowchartDocument> {
        if self.current < self.states.len() {
            self.current += 1;
            Some(&self.states[self.current - 1])
        } else {
            None
        }
    }

    pub fn can_undo(&self) -> bool {
        self.current > 1
    }

    pub fn can_redo(&self) -> bool {
        self.current < self.states.len()
    }
}
```

**Step 2: Add `mod history;` to main.rs**

**Step 3: Build to verify**

Run: `cargo build`
Expected: Compiles.

**Step 4: Commit**

```bash
git add src/history.rs src/main.rs
git commit -m "feat: add undo/redo history stack"
```

---

## Batch 2: Canvas + Node Rendering

### Task 4: Build the app state struct and canvas with zoom/pan

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs` (use app module, delegate to it)

**Step 1: Create src/app.rs with FlowchartApp and canvas rendering**

```rust
use eframe::egui;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::history::UndoStack;
use crate::model::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Connect,
}

#[derive(Debug, Clone, Copy)]
pub enum DragState {
    None,
    Panning { start_offset: [f32; 2], start_pos: Pos2 },
    DraggingNode { node_id: NodeId, offset: Vec2 },
    BoxSelect { start: Pos2 },
    CreatingEdge { source: Port, current_pos: Pos2 },
    DraggingNewNode { shape: NodeShape },
}

pub struct FlowchartApp {
    pub document: FlowchartDocument,
    pub viewport: Viewport,
    pub selection: Selection,
    pub history: UndoStack,
    pub clipboard: Vec<Node>,
    pub tool: Tool,
    pub drag: DragState,
    pub show_grid: bool,
    pub snap_to_grid: bool,
    pub grid_size: f32,
}

impl FlowchartApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            document: FlowchartDocument::default(),
            viewport: Viewport::default(),
            selection: Selection::default(),
            history: UndoStack::new(100),
            clipboard: Vec::new(),
            tool: Tool::Select,
            drag: DragState::None,
            show_grid: true,
            snap_to_grid: true,
            grid_size: 20.0,
        };
        app.history.push(&app.document);
        app
    }

    pub fn save_state(&mut self) {
        self.history.push(&self.document);
    }

    pub fn snap_pos(&self, pos: Pos2) -> Pos2 {
        if self.snap_to_grid {
            Pos2::new(
                (pos.x / self.grid_size).round() * self.grid_size,
                (pos.y / self.grid_size).round() * self.grid_size,
            )
        } else {
            pos
        }
    }
}

impl eframe::App for FlowchartApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts
        self.handle_shortcuts(ctx);

        // Left toolbar
        self.draw_toolbar(ctx);

        // Right properties panel
        self.draw_properties_panel(ctx);

        // Central canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::from_rgb(30, 30, 30)))
            .show(ctx, |ui| {
                self.draw_canvas(ui);
            });
    }
}

impl FlowchartApp {
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let modifiers = ctx.input(|i| i.modifiers);

        ctx.input(|i| {
            // Cmd+Z = undo
            if modifiers.command && !modifiers.shift && i.key_pressed(egui::Key::Z) {
                if let Some(doc) = self.history.undo() {
                    self.document = doc.clone();
                    self.selection.clear();
                }
            }
            // Cmd+Shift+Z = redo
            if modifiers.command && modifiers.shift && i.key_pressed(egui::Key::Z) {
                if let Some(doc) = self.history.redo() {
                    self.document = doc.clone();
                    self.selection.clear();
                }
            }
            // Delete/Backspace = remove selected
            if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                let node_ids = self.selection.node_ids.clone();
                let edge_ids = self.selection.edge_ids.clone();
                for id in &node_ids {
                    self.document.remove_node(id);
                }
                for id in &edge_ids {
                    self.document.remove_edge(id);
                }
                if !node_ids.is_empty() || !edge_ids.is_empty() {
                    self.selection.clear();
                    self.save_state();
                }
            }
            // Cmd+C = copy
            if modifiers.command && i.key_pressed(egui::Key::C) {
                self.clipboard.clear();
                for id in &self.selection.node_ids {
                    if let Some(node) = self.document.find_node(id) {
                        self.clipboard.push(node.clone());
                    }
                }
            }
            // Cmd+V = paste
            if modifiers.command && i.key_pressed(egui::Key::V) {
                if !self.clipboard.is_empty() {
                    self.selection.clear();
                    for node in &self.clipboard {
                        let mut new_node = node.clone();
                        new_node.id = NodeId::new();
                        new_node.position[0] += 20.0;
                        new_node.position[1] += 20.0;
                        self.selection.node_ids.push(new_node.id);
                        self.document.nodes.push(new_node);
                    }
                    self.save_state();
                }
            }
            // Cmd+A = select all
            if modifiers.command && i.key_pressed(egui::Key::A) {
                self.selection.clear();
                for node in &self.document.nodes {
                    self.selection.node_ids.push(node.id);
                }
                for edge in &self.document.edges {
                    self.selection.edge_ids.push(edge.id);
                }
            }
        });
    }

    fn draw_canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let canvas_rect = response.rect;

        // Handle zoom
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 1.0 / 1.1 };
            let mouse_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(canvas_rect.center());
            let old_zoom = self.viewport.zoom;
            self.viewport.zoom = (self.viewport.zoom * zoom_factor).clamp(0.1, 5.0);
            // Zoom towards mouse position
            let zoom_ratio = self.viewport.zoom / old_zoom;
            self.viewport.offset[0] = mouse_pos.x - (mouse_pos.x - self.viewport.offset[0]) * zoom_ratio;
            self.viewport.offset[1] = mouse_pos.y - (mouse_pos.y - self.viewport.offset[1]) * zoom_ratio;
        }

        // Handle space+drag for panning
        let space_held = ui.input(|i| i.key_down(egui::Key::Space));
        let pointer_pos = ui.input(|i| i.pointer.hover_pos());

        // Draw grid
        if self.show_grid {
            self.draw_grid(&painter, canvas_rect);
        }

        // Handle interactions
        if response.drag_started() {
            if let Some(pos) = pointer_pos {
                if space_held || ui.input(|i| i.pointer.middle_down()) {
                    self.drag = DragState::Panning {
                        start_offset: self.viewport.offset,
                        start_pos: pos,
                    };
                } else if matches!(self.drag, DragState::DraggingNewNode { .. }) {
                    // Already dragging from toolbar, don't override
                } else {
                    let canvas_pos = self.viewport.screen_to_canvas(pos);

                    // Check if clicking on a port (for edge creation)
                    if self.tool == Tool::Connect {
                        if let Some((node_id, side)) = self.hit_test_port(canvas_pos) {
                            self.drag = DragState::CreatingEdge {
                                source: Port { node_id, side },
                                current_pos: canvas_pos,
                            };
                        }
                    }

                    // Check if clicking on a node
                    if matches!(self.drag, DragState::None) {
                        if let Some(node_id) = self.document.node_at_pos(canvas_pos) {
                            let node_pos = self.document.find_node(&node_id).unwrap().pos();
                            let offset = node_pos - canvas_pos;
                            let modifiers = ui.input(|i| i.modifiers);
                            if modifiers.command {
                                self.selection.toggle_node(node_id);
                            } else if !self.selection.contains_node(&node_id) {
                                self.selection.select_node(node_id);
                            }
                            self.drag = DragState::DraggingNode { node_id, offset };
                        } else {
                            // Start box select
                            if !ui.input(|i| i.modifiers.command) {
                                self.selection.clear();
                            }
                            self.drag = DragState::BoxSelect { start: canvas_pos };
                        }
                    }
                }
            }
        }

        if response.dragged() {
            if let Some(pos) = pointer_pos {
                match self.drag {
                    DragState::Panning { start_offset, start_pos } => {
                        self.viewport.offset[0] = start_offset[0] + (pos.x - start_pos.x);
                        self.viewport.offset[1] = start_offset[1] + (pos.y - start_pos.y);
                    }
                    DragState::DraggingNode { node_id, offset } => {
                        let canvas_pos = self.viewport.screen_to_canvas(pos);
                        let new_pos = self.snap_pos(Pos2::new(canvas_pos.x + offset.x, canvas_pos.y + offset.y));
                        if let Some(node) = self.document.find_node_mut(&node_id) {
                            let delta = new_pos - node.pos();
                            node.set_pos(new_pos);
                            // Move other selected nodes too
                            let other_ids: Vec<NodeId> = self.selection.node_ids.iter()
                                .filter(|id| **id != node_id)
                                .cloned()
                                .collect();
                            for id in other_ids {
                                if let Some(n) = self.document.find_node_mut(&id) {
                                    let p = n.pos() + delta;
                                    n.set_pos(self.snap_pos(p));
                                }
                            }
                        }
                    }
                    DragState::CreatingEdge { ref source, ref mut current_pos } => {
                        *current_pos = self.viewport.screen_to_canvas(pos);
                    }
                    DragState::DraggingNewNode { shape } => {
                        // Preview handled in draw
                    }
                    DragState::BoxSelect { .. } => {}
                    DragState::None => {}
                }
            }
        }

        if response.drag_stopped() {
            match self.drag {
                DragState::DraggingNode { .. } => {
                    self.save_state();
                }
                DragState::CreatingEdge { ref source, current_pos } => {
                    // Check if dropped on a port
                    if let Some((target_node, target_side)) = self.hit_test_port(current_pos) {
                        if target_node != source.node_id {
                            let edge = Edge::new(
                                source.clone(),
                                Port { node_id: target_node, side: target_side },
                            );
                            self.document.edges.push(edge);
                            self.save_state();
                        }
                    }
                }
                DragState::DraggingNewNode { shape } => {
                    if let Some(pos) = pointer_pos {
                        let canvas_pos = self.viewport.screen_to_canvas(pos);
                        let snapped = self.snap_pos(canvas_pos);
                        let node = Node::new(shape, snapped);
                        self.selection.select_node(node.id);
                        self.document.nodes.push(node);
                        self.save_state();
                    }
                }
                DragState::BoxSelect { start } => {
                    if let Some(pos) = pointer_pos {
                        let end = self.viewport.screen_to_canvas(pos);
                        let select_rect = Rect::from_two_pos(start, end);
                        for node in &self.document.nodes {
                            if select_rect.intersects(node.rect()) {
                                if !self.selection.contains_node(&node.id) {
                                    self.selection.node_ids.push(node.id);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            self.drag = DragState::None;
        }

        // Click without drag = select/deselect
        if response.clicked() {
            if let Some(pos) = pointer_pos {
                let canvas_pos = self.viewport.screen_to_canvas(pos);
                if let Some(node_id) = self.document.node_at_pos(canvas_pos) {
                    if ui.input(|i| i.modifiers.command) {
                        self.selection.toggle_node(node_id);
                    } else {
                        self.selection.select_node(node_id);
                    }
                } else {
                    self.selection.clear();
                }
            }
        }

        // Draw edges
        for edge in &self.document.edges {
            self.draw_edge(&painter, edge);
        }

        // Draw in-progress edge
        if let DragState::CreatingEdge { ref source, current_pos } = self.drag {
            if let Some(node) = self.document.find_node(&source.node_id) {
                let start = self.viewport.canvas_to_screen(node.port_position(source.side));
                let end = self.viewport.canvas_to_screen(current_pos);
                let mid_x = (start.x + end.x) / 2.0;
                painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
                    points: [start, Pos2::new(mid_x, start.y), Pos2::new(mid_x, end.y), end],
                    closed: false,
                    fill: Color32::TRANSPARENT,
                    stroke: Stroke::new(2.0, Color32::from_rgb(100, 150, 255)).into(),
                }));
            }
        }

        // Draw nodes
        for node in &self.document.nodes {
            self.draw_node(&painter, node);
        }

        // Draw box selection
        if let DragState::BoxSelect { start } = self.drag {
            if let Some(pos) = pointer_pos {
                let screen_start = self.viewport.canvas_to_screen(start);
                let rect = Rect::from_two_pos(screen_start, pos);
                painter.rect(
                    rect,
                    0.0,
                    Color32::from_rgba_unmultiplied(100, 150, 255, 30),
                    Stroke::new(1.0, Color32::from_rgb(100, 150, 255)),
                );
            }
        }

        // Draw new node preview
        if let DragState::DraggingNewNode { shape } = self.drag {
            if let Some(pos) = pointer_pos {
                let canvas_pos = self.viewport.screen_to_canvas(pos);
                let preview = Node::new(shape, canvas_pos);
                // Draw semi-transparent
                let screen_rect = Rect::from_min_size(
                    self.viewport.canvas_to_screen(preview.pos()),
                    preview.size_vec() * self.viewport.zoom,
                );
                painter.rect(
                    screen_rect,
                    4.0,
                    Color32::from_rgba_unmultiplied(100, 150, 255, 60),
                    Stroke::new(2.0, Color32::from_rgba_unmultiplied(100, 150, 255, 150)),
                );
            }
        }
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        let grid_screen = self.grid_size * self.viewport.zoom;
        if grid_screen < 5.0 {
            return; // Too small to draw
        }

        let top_left = self.viewport.screen_to_canvas(rect.min);
        let bottom_right = self.viewport.screen_to_canvas(rect.max);

        let start_x = (top_left.x / self.grid_size).floor() * self.grid_size;
        let start_y = (top_left.y / self.grid_size).floor() * self.grid_size;

        let grid_color = Color32::from_rgba_unmultiplied(255, 255, 255, 15);

        let mut x = start_x;
        while x < bottom_right.x {
            let screen_x = self.viewport.canvas_to_screen(Pos2::new(x, 0.0)).x;
            painter.line_segment(
                [Pos2::new(screen_x, rect.min.y), Pos2::new(screen_x, rect.max.y)],
                Stroke::new(0.5, grid_color),
            );
            x += self.grid_size;
        }

        let mut y = start_y;
        while y < bottom_right.y {
            let screen_y = self.viewport.canvas_to_screen(Pos2::new(0.0, y)).y;
            painter.line_segment(
                [Pos2::new(rect.min.x, screen_y), Pos2::new(rect.max.x, screen_y)],
                Stroke::new(0.5, grid_color),
            );
            y += self.grid_size;
        }
    }

    fn draw_node(&self, painter: &egui::Painter, node: &Node) {
        let zoom = self.viewport.zoom;
        let screen_pos = self.viewport.canvas_to_screen(node.pos());
        let screen_size = node.size_vec() * zoom;
        let screen_rect = Rect::from_min_size(screen_pos, screen_size);

        let fill = Color32::from_rgba_unmultiplied(
            node.style.fill_color[0], node.style.fill_color[1],
            node.style.fill_color[2], node.style.fill_color[3],
        );
        let border_color = if self.selection.contains_node(&node.id) {
            Color32::from_rgb(80, 160, 255)
        } else {
            Color32::from_rgba_unmultiplied(
                node.style.border_color[0], node.style.border_color[1],
                node.style.border_color[2], node.style.border_color[3],
            )
        };
        let border_width = if self.selection.contains_node(&node.id) {
            3.0
        } else {
            node.style.border_width
        };

        match node.shape {
            NodeShape::Rectangle => {
                painter.rect(screen_rect, 0.0, fill, Stroke::new(border_width, border_color));
            }
            NodeShape::RoundedRect => {
                painter.rect(screen_rect, 8.0 * zoom, fill, Stroke::new(border_width, border_color));
            }
            NodeShape::Circle => {
                let center = screen_rect.center();
                let radius = screen_size.x.min(screen_size.y) / 2.0;
                painter.circle(center, radius, fill, Stroke::new(border_width, border_color));
            }
            NodeShape::Diamond => {
                let c = screen_rect.center();
                let hw = screen_size.x / 2.0;
                let hh = screen_size.y / 2.0;
                let points = vec![
                    Pos2::new(c.x, c.y - hh),
                    Pos2::new(c.x + hw, c.y),
                    Pos2::new(c.x, c.y + hh),
                    Pos2::new(c.x - hw, c.y),
                ];
                painter.add(egui::Shape::convex_polygon(points, fill, Stroke::new(border_width, border_color)));
            }
            NodeShape::Parallelogram => {
                let skew = screen_size.x * 0.15;
                let points = vec![
                    Pos2::new(screen_rect.min.x + skew, screen_rect.min.y),
                    Pos2::new(screen_rect.max.x, screen_rect.min.y),
                    Pos2::new(screen_rect.max.x - skew, screen_rect.max.y),
                    Pos2::new(screen_rect.min.x, screen_rect.max.y),
                ];
                painter.add(egui::Shape::convex_polygon(points, fill, Stroke::new(border_width, border_color)));
            }
        }

        // Draw label
        let text_color = Color32::from_rgba_unmultiplied(
            node.style.text_color[0], node.style.text_color[1],
            node.style.text_color[2], node.style.text_color[3],
        );
        painter.text(
            screen_rect.center(),
            egui::Align2::CENTER_CENTER,
            &node.label,
            egui::FontId::proportional(node.style.font_size * zoom),
            text_color,
        );

        // Draw ports (small circles on each side)
        let port_radius = 4.0 * zoom;
        let port_color = Color32::from_rgb(80, 160, 255);
        for side in [PortSide::Top, PortSide::Bottom, PortSide::Left, PortSide::Right] {
            let port_pos = self.viewport.canvas_to_screen(node.port_position(side));
            painter.circle(port_pos, port_radius, Color32::WHITE, Stroke::new(1.5, port_color));
        }
    }

    fn draw_edge(&self, painter: &egui::Painter, edge: &Edge) {
        let source_node = self.document.find_node(&edge.source.node_id);
        let target_node = self.document.find_node(&edge.target.node_id);

        if let (Some(src), Some(tgt)) = (source_node, target_node) {
            let start = self.viewport.canvas_to_screen(src.port_position(edge.source.side));
            let end = self.viewport.canvas_to_screen(tgt.port_position(edge.target.side));

            let edge_color = if self.selection.contains_edge(&edge.id) {
                Color32::from_rgb(80, 160, 255)
            } else {
                Color32::from_rgba_unmultiplied(
                    edge.style.color[0], edge.style.color[1],
                    edge.style.color[2], edge.style.color[3],
                )
            };
            let width = if self.selection.contains_edge(&edge.id) {
                edge.style.width + 1.0
            } else {
                edge.style.width
            };

            // Bezier curve with control points based on port direction
            let offset = 50.0 * self.viewport.zoom;
            let cp1 = match edge.source.side {
                PortSide::Top => Pos2::new(start.x, start.y - offset),
                PortSide::Bottom => Pos2::new(start.x, start.y + offset),
                PortSide::Left => Pos2::new(start.x - offset, start.y),
                PortSide::Right => Pos2::new(start.x + offset, start.y),
            };
            let cp2 = match edge.target.side {
                PortSide::Top => Pos2::new(end.x, end.y - offset),
                PortSide::Bottom => Pos2::new(end.x, end.y + offset),
                PortSide::Left => Pos2::new(end.x - offset, end.y),
                PortSide::Right => Pos2::new(end.x + offset, end.y),
            };

            painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
                points: [start, cp1, cp2, end],
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: Stroke::new(width, edge_color).into(),
            }));

            // Arrow head at end
            let arrow_size = 8.0 * self.viewport.zoom;
            let dir = (end - cp2).normalized();
            let perp = Vec2::new(-dir.y, dir.x);
            let arrow_points = vec![
                end,
                end - dir * arrow_size + perp * arrow_size * 0.4,
                end - dir * arrow_size - perp * arrow_size * 0.4,
            ];
            painter.add(egui::Shape::convex_polygon(arrow_points, edge_color, Stroke::NONE));

            // Draw label if present
            if !edge.label.is_empty() {
                let mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
                painter.text(
                    mid,
                    egui::Align2::CENTER_CENTER,
                    &edge.label,
                    egui::FontId::proportional(12.0 * self.viewport.zoom),
                    Color32::from_rgb(200, 200, 200),
                );
            }
        }
    }

    fn hit_test_port(&self, canvas_pos: Pos2) -> Option<(NodeId, PortSide)> {
        let threshold = 12.0;
        for node in self.document.nodes.iter().rev() {
            for side in [PortSide::Top, PortSide::Bottom, PortSide::Left, PortSide::Right] {
                let port_pos = node.port_position(side);
                if (port_pos - canvas_pos).length() < threshold {
                    return Some((node.id, side));
                }
            }
        }
        None
    }

    fn draw_toolbar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("toolbar")
            .default_width(180.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Tools");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tool, Tool::Select, "Select");
                    ui.selectable_value(&mut self.tool, Tool::Connect, "Connect");
                });
                ui.separator();

                ui.heading("Shapes");
                ui.separator();

                let shapes = [
                    (NodeShape::Rectangle, "Rectangle"),
                    (NodeShape::RoundedRect, "Rounded Rect"),
                    (NodeShape::Diamond, "Diamond"),
                    (NodeShape::Circle, "Circle"),
                    (NodeShape::Parallelogram, "Parallelogram"),
                ];

                for (shape, name) in shapes {
                    let btn = ui.button(name);
                    if btn.drag_started() {
                        self.drag = DragState::DraggingNewNode { shape };
                    }
                    if btn.clicked() {
                        // Add at center of viewport
                        let center = self.viewport.screen_to_canvas(Pos2::new(640.0, 400.0));
                        let node = Node::new(shape, self.snap_pos(center));
                        self.selection.select_node(node.id);
                        self.document.nodes.push(node);
                        self.save_state();
                    }
                }

                ui.separator();
                ui.heading("View");
                ui.separator();
                ui.checkbox(&mut self.show_grid, "Show Grid");
                ui.checkbox(&mut self.snap_to_grid, "Snap to Grid");

                ui.separator();
                ui.label(format!("Zoom: {:.0}%", self.viewport.zoom * 100.0));
                if ui.button("Reset Zoom").clicked() {
                    self.viewport.zoom = 1.0;
                    self.viewport.offset = [0.0, 0.0];
                }
            });
    }

    fn draw_properties_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("properties")
            .default_width(200.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Properties");
                ui.separator();

                if self.selection.node_ids.len() == 1 {
                    let node_id = self.selection.node_ids[0];
                    if let Some(node) = self.document.find_node_mut(&node_id) {
                        ui.label("Label:");
                        ui.text_edit_singleline(&mut node.label);
                        ui.label("Description:");
                        ui.text_edit_multiline(&mut node.description);

                        ui.separator();
                        ui.label("Style:");

                        let mut fill = Color32::from_rgba_unmultiplied(
                            node.style.fill_color[0], node.style.fill_color[1],
                            node.style.fill_color[2], node.style.fill_color[3],
                        );
                        ui.horizontal(|ui| {
                            ui.label("Fill:");
                            if ui.color_edit_button_srgba(&mut fill).changed() {
                                node.style.fill_color = [fill.r(), fill.g(), fill.b(), fill.a()];
                            }
                        });

                        let mut border = Color32::from_rgba_unmultiplied(
                            node.style.border_color[0], node.style.border_color[1],
                            node.style.border_color[2], node.style.border_color[3],
                        );
                        ui.horizontal(|ui| {
                            ui.label("Border:");
                            if ui.color_edit_button_srgba(&mut border).changed() {
                                node.style.border_color = [border.r(), border.g(), border.b(), border.a()];
                            }
                        });

                        ui.add(egui::Slider::new(&mut node.style.border_width, 0.5..=5.0).text("Border Width"));
                        ui.add(egui::Slider::new(&mut node.style.font_size, 8.0..=32.0).text("Font Size"));

                        ui.separator();
                        ui.label("Size:");
                        ui.add(egui::Slider::new(&mut node.size[0], 40.0..=400.0).text("Width"));
                        ui.add(egui::Slider::new(&mut node.size[1], 30.0..=300.0).text("Height"));
                    }
                } else if self.selection.edge_ids.len() == 1 {
                    let edge_id = self.selection.edge_ids[0];
                    if let Some(edge) = self.document.find_edge_mut(&edge_id) {
                        ui.label("Label:");
                        ui.text_edit_singleline(&mut edge.label);

                        let mut color = Color32::from_rgba_unmultiplied(
                            edge.style.color[0], edge.style.color[1],
                            edge.style.color[2], edge.style.color[3],
                        );
                        ui.horizontal(|ui| {
                            ui.label("Color:");
                            if ui.color_edit_button_srgba(&mut color).changed() {
                                edge.style.color = [color.r(), color.g(), color.b(), color.a()];
                            }
                        });
                        ui.add(egui::Slider::new(&mut edge.style.width, 0.5..=5.0).text("Width"));
                    }
                } else if !self.selection.is_empty() {
                    ui.label(format!("{} items selected", self.selection.node_ids.len() + self.selection.edge_ids.len()));
                } else {
                    ui.label("Nothing selected");
                    ui.separator();
                    ui.label("Click a node or edge to edit its properties.");
                }
            });
    }
}
```

**Step 2: Update main.rs to use app module**

```rust
mod app;
mod history;
mod model;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Light Figma"),
        ..Default::default()
    };
    eframe::run_native(
        "Light Figma",
        options,
        Box::new(|cc| Ok(Box::new(app::FlowchartApp::new(cc)))),
    )
}
```

**Step 3: Build and run**

Run: `cargo run`
Expected: Window with dark canvas, left toolbar with shape buttons, right properties panel. Can click shapes to add nodes, drag nodes, zoom/pan.

**Step 4: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: add canvas with zoom/pan, node rendering, toolbar, properties panel"
```

---

## Batch 3: Save/Load + Export

### Task 5: Add save/load (JSON .flow files)

**Files:**
- Create: `src/io.rs`
- Modify: `src/app.rs` (add file menu / save-load buttons)
- Modify: `src/main.rs` (add `mod io;`)

**Step 1: Create src/io.rs**

```rust
use crate::model::FlowchartDocument;
use std::path::Path;

pub fn save_document(doc: &FlowchartDocument, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(doc).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn load_document(path: &Path) -> Result<FlowchartDocument, String> {
    let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
```

**Step 2: Add save/load buttons to toolbar in app.rs**

Add to the bottom of `draw_toolbar` method, before the closing `});`:

```rust
ui.separator();
ui.heading("File");
ui.separator();
if ui.button("Save (.flow)").clicked() {
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Save Flowchart")
        .add_filter("Flowchart", &["flow"])
        .save_file()
    {
        if let Err(e) = crate::io::save_document(&self.document, &path) {
            eprintln!("Save error: {}", e);
        }
    }
}
if ui.button("Load (.flow)").clicked() {
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Open Flowchart")
        .add_filter("Flowchart", &["flow"])
        .pick_file()
    {
        match crate::io::load_document(&path) {
            Ok(doc) => {
                self.document = doc;
                self.selection.clear();
                self.save_state();
            }
            Err(e) => eprintln!("Load error: {}", e),
        }
    }
}
```

**Step 3: Add `mod io;` to main.rs**

**Step 4: Build and test save/load**

Run: `cargo run`
Expected: Save/Load buttons work, files saved as JSON with `.flow` extension.

**Step 5: Commit**

```bash
git add src/io.rs src/app.rs src/main.rs
git commit -m "feat: add save/load flowchart files (.flow JSON)"
```

---

### Task 6: Add PNG export

**Files:**
- Create: `src/export.rs`
- Modify: `src/app.rs` (add export button)
- Modify: `src/main.rs` (add `mod export;`)

**Step 1: Create src/export.rs with PNG export**

```rust
use crate::model::*;
use egui::{Color32, Pos2, Rect, Vec2};

pub fn export_png(doc: &FlowchartDocument, path: &std::path::Path, scale: f32) -> Result<(), String> {
    // Calculate bounding box of all nodes
    if doc.nodes.is_empty() {
        return Err("No nodes to export".into());
    }

    let padding = 50.0;
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for node in &doc.nodes {
        let rect = node.rect();
        min_x = min_x.min(rect.min.x);
        min_y = min_y.min(rect.min.y);
        max_x = max_x.max(rect.max.x);
        max_y = max_y.max(rect.max.y);
    }

    let width = ((max_x - min_x + padding * 2.0) * scale) as u32;
    let height = ((max_y - min_y + padding * 2.0) * scale) as u32;

    let mut imgbuf = image::RgbaImage::new(width, height);

    // Fill with white background
    for pixel in imgbuf.pixels_mut() {
        *pixel = image::Rgba([255, 255, 255, 255]);
    }

    let offset_x = -min_x + padding;
    let offset_y = -min_y + padding;

    // Draw nodes as filled rectangles (simplified rasterization)
    for node in &doc.nodes {
        let x1 = ((node.position[0] + offset_x) * scale) as i32;
        let y1 = ((node.position[1] + offset_y) * scale) as i32;
        let x2 = x1 + (node.size[0] * scale) as i32;
        let y2 = y1 + (node.size[1] * scale) as i32;

        let fill = node.style.fill_color;
        let border = node.style.border_color;

        // Fill
        for y in y1.max(0)..y2.min(height as i32) {
            for x in x1.max(0)..x2.min(width as i32) {
                imgbuf.put_pixel(x as u32, y as u32, image::Rgba(fill));
            }
        }

        // Border (top, bottom, left, right lines)
        let bw = (node.style.border_width * scale) as i32;
        for y in y1.max(0)..(y1 + bw).min(height as i32) {
            for x in x1.max(0)..x2.min(width as i32) {
                imgbuf.put_pixel(x as u32, y as u32, image::Rgba(border));
            }
        }
        for y in (y2 - bw).max(0)..y2.min(height as i32) {
            for x in x1.max(0)..x2.min(width as i32) {
                imgbuf.put_pixel(x as u32, y as u32, image::Rgba(border));
            }
        }
        for y in y1.max(0)..y2.min(height as i32) {
            for x in x1.max(0)..(x1 + bw).min(width as i32) {
                imgbuf.put_pixel(x as u32, y as u32, image::Rgba(border));
            }
        }
        for y in y1.max(0)..y2.min(height as i32) {
            for x in (x2 - bw).max(0)..x2.min(width as i32) {
                imgbuf.put_pixel(x as u32, y as u32, image::Rgba(border));
            }
        }
    }

    imgbuf.save(path).map_err(|e| e.to_string())
}
```

**Step 2: Add export button to toolbar in app.rs**

After the Load button in `draw_toolbar`:

```rust
ui.separator();
ui.heading("Export");
ui.separator();
if ui.button("Export PNG").clicked() {
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export as PNG")
        .add_filter("PNG Image", &["png"])
        .save_file()
    {
        if let Err(e) = crate::export::export_png(&self.document, &path, 2.0) {
            eprintln!("Export error: {}", e);
        }
    }
}
```

**Step 3: Add `mod export;` to main.rs**

**Step 4: Build and test**

Run: `cargo run`
Expected: Export PNG button creates a PNG image of the flowchart.

**Step 5: Commit**

```bash
git add src/export.rs src/app.rs src/main.rs
git commit -m "feat: add PNG export"
```

---

### Task 7: Add SVG export

**Files:**
- Modify: `src/export.rs` (add SVG export function)
- Modify: `src/app.rs` (add SVG export button)

**Step 1: Add SVG export function to src/export.rs**

```rust
pub fn export_svg(doc: &FlowchartDocument, path: &std::path::Path) -> Result<(), String> {
    if doc.nodes.is_empty() {
        return Err("No nodes to export".into());
    }

    let padding = 50.0;
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for node in &doc.nodes {
        let rect = node.rect();
        min_x = min_x.min(rect.min.x);
        min_y = min_y.min(rect.min.y);
        max_x = max_x.max(rect.max.x);
        max_y = max_y.max(rect.max.y);
    }

    let width = max_x - min_x + padding * 2.0;
    let height = max_y - min_y + padding * 2.0;
    let ox = -min_x + padding;
    let oy = -min_y + padding;

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
        width, height, width, height
    );
    svg.push_str(r#"<rect width="100%" height="100%" fill="white"/>"#);

    // Draw edges
    for edge in &doc.edges {
        if let (Some(src), Some(tgt)) = (doc.find_node(&edge.source.node_id), doc.find_node(&edge.target.node_id)) {
            let start = src.port_position(edge.source.side);
            let end = tgt.port_position(edge.target.side);
            let sx = start.x + ox;
            let sy = start.y + oy;
            let ex = end.x + ox;
            let ey = end.y + oy;
            let c = format!("rgb({},{},{})", edge.style.color[0], edge.style.color[1], edge.style.color[2]);
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}" stroke-width="{}"/>"#,
                sx, sy, ex, ey, c, edge.style.width
            ));
        }
    }

    // Draw nodes
    for node in &doc.nodes {
        let x = node.position[0] + ox;
        let y = node.position[1] + oy;
        let w = node.size[0];
        let h = node.size[1];
        let fill = format!("rgb({},{},{})", node.style.fill_color[0], node.style.fill_color[1], node.style.fill_color[2]);
        let stroke = format!("rgb({},{},{})", node.style.border_color[0], node.style.border_color[1], node.style.border_color[2]);
        let text_color = format!("rgb({},{},{})", node.style.text_color[0], node.style.text_color[1], node.style.text_color[2]);

        match node.shape {
            NodeShape::Rectangle => {
                svg.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                    x, y, w, h, fill, stroke, node.style.border_width
                ));
            }
            NodeShape::RoundedRect => {
                svg.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" rx="8" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                    x, y, w, h, fill, stroke, node.style.border_width
                ));
            }
            NodeShape::Circle => {
                let cx = x + w / 2.0;
                let cy = y + h / 2.0;
                let r = w.min(h) / 2.0;
                svg.push_str(&format!(
                    r#"<circle cx="{}" cy="{}" r="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                    cx, cy, r, fill, stroke, node.style.border_width
                ));
            }
            NodeShape::Diamond => {
                let cx = x + w / 2.0;
                let cy = y + h / 2.0;
                let points = format!("{},{} {},{} {},{} {},{}",
                    cx, y, x + w, cy, cx, y + h, x, cy);
                svg.push_str(&format!(
                    r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                    points, fill, stroke, node.style.border_width
                ));
            }
            NodeShape::Parallelogram => {
                let skew = w * 0.15;
                let points = format!("{},{} {},{} {},{} {},{}",
                    x + skew, y, x + w, y, x + w - skew, y + h, x, y + h);
                svg.push_str(&format!(
                    r#"<polygon points="{}" fill="{}" stroke="{}" stroke-width="{}"/>"#,
                    points, fill, stroke, node.style.border_width
                ));
            }
        }

        // Label
        let label_escaped = node.label.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" text-anchor="middle" dominant-baseline="central" fill="{}" font-size="{}">{}</text>"#,
            x + w / 2.0, y + h / 2.0, text_color, node.style.font_size, label_escaped
        ));
    }

    svg.push_str("</svg>");
    std::fs::write(path, svg).map_err(|e| e.to_string())
}
```

**Step 2: Add SVG export button after PNG button in app.rs toolbar**

```rust
if ui.button("Export SVG").clicked() {
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export as SVG")
        .add_filter("SVG Image", &["svg"])
        .save_file()
    {
        if let Err(e) = crate::export::export_svg(&self.document, &path) {
            eprintln!("SVG export error: {}", e);
        }
    }
}
```

**Step 3: Build and test**

Run: `cargo run`
Expected: SVG export creates valid SVG with all shapes.

**Step 4: Commit**

```bash
git add src/export.rs src/app.rs
git commit -m "feat: add SVG export"
```

---

### Task 8: Add PDF export

**Files:**
- Modify: `src/export.rs` (add PDF export function)
- Modify: `src/app.rs` (add PDF export button)

**Step 1: Add PDF export to src/export.rs**

```rust
use printpdf::*;

pub fn export_pdf(doc: &FlowchartDocument, path: &std::path::Path) -> Result<(), String> {
    if doc.nodes.is_empty() {
        return Err("No nodes to export".into());
    }

    let padding = 50.0;
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for node in &doc.nodes {
        let rect = node.rect();
        min_x = min_x.min(rect.min.x);
        min_y = min_y.min(rect.min.y);
        max_x = max_x.max(rect.max.x);
        max_y = max_y.max(rect.max.y);
    }

    let width_mm = Mm((max_x - min_x + padding * 2.0) as f64 * 0.264583);
    let height_mm = Mm((max_y - min_y + padding * 2.0) as f64 * 0.264583);
    let ox = -min_x + padding;
    let oy = -min_y + padding;
    let total_h = max_y - min_y + padding * 2.0;

    let (pdfdoc, page1, layer1) = PdfDocument::new("Flowchart", width_mm, height_mm, "Layer 1");
    let layer = pdfdoc.get_page(page1).get_layer(layer1);

    let to_mm = |v: f32| Mm(v as f64 * 0.264583);

    // Draw nodes
    for node in &doc.nodes {
        let x = node.position[0] + ox;
        let y = total_h - (node.position[1] + oy) - node.size[1]; // flip Y
        let w = node.size[0];
        let h = node.size[1];

        let fill = node.style.fill_color;
        layer.set_fill_color(Color::Rgb(Rgb::new(
            fill[0] as f64 / 255.0, fill[1] as f64 / 255.0, fill[2] as f64 / 255.0, None,
        )));
        let border = node.style.border_color;
        layer.set_outline_color(Color::Rgb(Rgb::new(
            border[0] as f64 / 255.0, border[1] as f64 / 255.0, border[2] as f64 / 255.0, None,
        )));
        layer.set_outline_thickness(node.style.border_width as f64);

        let rect = printpdf::Rect::new(to_mm(x), to_mm(y), to_mm(x + w), to_mm(y + h));
        layer.add_rect(rect);
    }

    pdfdoc.save(&mut std::io::BufWriter::new(
        std::fs::File::create(path).map_err(|e| e.to_string())?
    )).map_err(|e| e.to_string())
}
```

**Step 2: Add PDF export button in app.rs after SVG button**

```rust
if ui.button("Export PDF").clicked() {
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Export as PDF")
        .add_filter("PDF Document", &["pdf"])
        .save_file()
    {
        if let Err(e) = crate::export::export_pdf(&self.document, &path) {
            eprintln!("PDF export error: {}", e);
        }
    }
}
```

**Step 3: Build and test**

Run: `cargo run`
Expected: PDF export creates a valid PDF with node rectangles.

**Step 4: Commit**

```bash
git add src/export.rs src/app.rs
git commit -m "feat: add PDF export"
```

---

## Batch 4: Mini-map + Polish

### Task 9: Add mini-map

**Files:**
- Modify: `src/app.rs` (add `draw_minimap` method, call from `update`)

**Step 1: Add draw_minimap method to FlowchartApp in app.rs**

```rust
fn draw_minimap(&self, ui: &mut egui::Ui, canvas_rect: Rect) {
    if self.document.nodes.is_empty() {
        return;
    }

    let minimap_size = Vec2::new(180.0, 120.0);
    let minimap_pos = Pos2::new(
        canvas_rect.max.x - minimap_size.x - 10.0,
        canvas_rect.max.y - minimap_size.y - 10.0,
    );
    let minimap_rect = Rect::from_min_size(minimap_pos, minimap_size);

    let painter = ui.painter();

    // Background
    painter.rect(
        minimap_rect,
        4.0,
        Color32::from_rgba_unmultiplied(20, 20, 20, 200),
        Stroke::new(1.0, Color32::from_rgb(60, 60, 60)),
    );

    // Calculate bounds of all nodes
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for node in &self.document.nodes {
        let r = node.rect();
        min_x = min_x.min(r.min.x);
        min_y = min_y.min(r.min.y);
        max_x = max_x.max(r.max.x);
        max_y = max_y.max(r.max.y);
    }

    let padding = 50.0;
    min_x -= padding;
    min_y -= padding;
    max_x += padding;
    max_y += padding;

    let content_w = max_x - min_x;
    let content_h = max_y - min_y;
    let scale_x = (minimap_size.x - 8.0) / content_w;
    let scale_y = (minimap_size.y - 8.0) / content_h;
    let scale = scale_x.min(scale_y);

    let to_minimap = |pos: Pos2| -> Pos2 {
        Pos2::new(
            minimap_pos.x + 4.0 + (pos.x - min_x) * scale,
            minimap_pos.y + 4.0 + (pos.y - min_y) * scale,
        )
    };

    // Draw nodes as small dots
    for node in &self.document.nodes {
        let p = to_minimap(node.rect().center());
        painter.circle_filled(p, 2.0, Color32::from_rgb(100, 150, 255));
    }

    // Draw viewport rectangle
    let vp_tl = self.viewport.screen_to_canvas(canvas_rect.min);
    let vp_br = self.viewport.screen_to_canvas(canvas_rect.max);
    let vp_min = to_minimap(vp_tl);
    let vp_max = to_minimap(vp_br);
    let vp_rect = Rect::from_min_max(vp_min, vp_max).intersect(minimap_rect);
    painter.rect(
        vp_rect,
        0.0,
        Color32::from_rgba_unmultiplied(100, 150, 255, 30),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 150, 255, 150)),
    );
}
```

**Step 2: Call draw_minimap from draw_canvas, after drawing everything else**

Add at end of `draw_canvas`:
```rust
self.draw_minimap(ui, canvas_rect);
```

**Step 3: Build and test**

Run: `cargo run`
Expected: Mini-map appears in bottom-right showing node positions and current viewport.

**Step 4: Commit**

```bash
git add src/app.rs
git commit -m "feat: add mini-map overlay"
```

---

### Task 10: macOS .app bundle configuration

**Files:**
- Create: `icons/` directory with placeholder
- Verify: `Cargo.toml` has bundle metadata

**Step 1: Install cargo-bundle if not already installed**

Run: `cargo install cargo-bundle`

**Step 2: Create a placeholder icon**

Create `icons/` directory. For now the app will use a default icon.

**Step 3: Build the .app bundle**

Run: `cargo bundle --release`
Expected: Creates `target/release/bundle/osx/Light Figma.app`

**Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "feat: configure macOS .app bundle packaging"
```

---

## Summary

| Batch | Tasks | What it delivers |
|-------|-------|-----------------|
| 1 | Tasks 1-3 | Project scaffold, data model, undo/redo |
| 2 | Task 4 | Full canvas with zoom/pan, node rendering, drag-and-drop, toolbar, properties panel, edge creation |
| 3 | Tasks 5-8 | Save/load (.flow), PNG/SVG/PDF export |
| 4 | Tasks 9-10 | Mini-map, macOS .app bundle |
