# Light Figma — Flowchart App Design

## Summary
A Figma-inspired drag-and-drop flowchart application built in pure Rust using egui + eframe. Native macOS app with smooth interactions, smart layout, and export capabilities.

## Architecture
- **egui + eframe** for native macOS app (single binary, `.app` bundle)
- Immediate-mode rendering on a zoomable/pannable infinite canvas
- All state in a central `FlowchartApp` struct (nodes, edges, selection, viewport)

## Core Components

| Component | Description |
|-----------|-------------|
| Canvas | Infinite 2D surface with zoom (scroll), pan (middle-click/space+drag) |
| Node System | Draggable shapes: Rectangle, Diamond, Circle, Parallelogram, Rounded Rect. Each has title + description text |
| Connector System | Bezier-curve edges between node ports (top/bottom/left/right). Auto-routing to avoid overlap |
| Toolbar | Left sidebar with node palette (drag to canvas) + tools (select, connect, text) |
| Properties Panel | Right sidebar showing selected node/edge properties (color, label, style) |
| Snap & Align | Grid snapping, smart guides when dragging near other nodes |
| Mini-map | Bottom-right corner overview of the full flowchart |

## Data Model
```
FlowchartApp
├── nodes: Vec<Node>        // id, type, position, size, label, style
├── edges: Vec<Edge>        // id, source_port, target_port, label, curve_points
├── selection: Selection    // selected node/edge ids
├── viewport: Viewport      // offset, zoom level
├── history: UndoStack      // undo/redo support
└── clipboard: Clipboard    // copy/paste
```

## Key Interactions
- Drag from toolbar → create node at drop position
- Click node port + drag to another port → create edge
- Multi-select (Cmd+click or drag-select box)
- Cmd+Z / Cmd+Shift+Z → undo/redo
- Cmd+C / Cmd+V → copy/paste nodes
- Delete/Backspace → remove selected
- Scroll → zoom, Space+drag → pan

## Export
- PNG/SVG/PDF via rendering the canvas to image buffer
- JSON project files for save/load (.flow extension)
- macOS .app bundle via cargo-bundle

## Tech Stack
| Crate | Purpose |
|-------|---------|
| egui + eframe | GUI framework + native wrapper |
| serde + serde_json | Save/load project files |
| image | PNG export |
| svg or resvg | SVG export |
| printpdf | PDF export |
| uuid | Node/edge IDs |
| cargo-bundle | macOS .app packaging |
