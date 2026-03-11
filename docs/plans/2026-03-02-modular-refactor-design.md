# Modular Refactor Design

Split the monolithic `app.rs` (2888 LOC) into focused sub-modules for human readability and agent editability.

## Target Structure

```
src/app/
├── mod.rs         (~180) FlowchartApp struct, enums, DragState, new(), eframe::App impl
├── theme.rs       (~70)  Color constants, to_color32(), panel dimensions
├── shortcuts.rs   (~120) handle_shortcuts()
├── toolbar.rs     (~280) draw_toolbar(), draw_shape_button(), draw_section_header()
├── properties.rs  (~340) draw_properties_panel()
├── canvas.rs      (~490) draw_canvas(), draw_grid(), draw_minimap(), input dispatch
├── render.rs      (~530) draw_node, draw_shape/sticky/entity/text_node, draw_edge, crow_foot, arrow
└── interaction.rs (~200) Hit testing, resize handles, zoom helpers, snap, bezier math, tests
```

## Key Decisions

- Directory module pattern: `app.rs` becomes `app/mod.rs` with sub-modules
- Each sub-module adds `impl FlowchartApp` methods (idiomatic Rust inherent impl splitting)
- Struct fields become `pub(super)` for sub-module access
- Free functions (bezier math) move to `interaction.rs`
- Color constants consolidate in `theme.rs` (remove duplicates from `new()`)
- Remove unused `_is_hovered` params from draw methods
- Tests remain in `interaction.rs` (they test `compute_resize`)
- No behavior changes — pure refactor + cleanup
