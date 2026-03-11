# light-figma Skill Design

**Date:** 2026-03-12
**Status:** approved

## Overview

A single Claude skill (`~/.claude/skills/light-figma.md`) that serves two purposes:

1. **Background knowledge** — compact architecture reference loaded with any command (module map, rendering pipeline, known pitfalls).
2. **Task checklists** — structured, step-by-step workflows for the three most common development tasks.

## Commands

| Command | Purpose |
|---|---|
| `/light-figma add-node` | Add a new `NodeKind` / `NodeShape` end-to-end |
| `/light-figma add-spec` | Add a new field/feature to the SpecGraph format |
| `/light-figma read-repo <path>` | Analyze any repo and emit a `.spec` diagram |
| `/light-figma` (no args) | Print usage |

## Architecture Reference (always loaded)

~60 lines covering:
- Module ownership map (model / app / specgraph / export)
- Rendering pipeline order (2D: render.rs, 3D: render3d.rs)
- 3 known pitfalls: `pending_fit`, resize sign convention, `unproject_drag_delta`

## add-node Checklist

Files touched in order:
1. `src/model.rs` — add variant to `NodeShape` or `NodeKind`
2. `src/app/render.rs` — 2D drawing
3. `src/app/render3d.rs` — 3D extruded drawing
4. `src/app/properties.rs` — label in properties panel
5. `src/app/toolbar.rs` — shape picker entry
6. `src/export.rs` — SVG/PDF arm
7. `src/specgraph/convert.rs` — YAML ↔ model mapping
8. `src/specgraph/hrf.rs` — HRF tag alias
9. Build + verify no new warnings

## add-spec Checklist

Files touched in order:
1. `src/specgraph/schema.rs` — add serde field
2. `src/specgraph/convert.rs` — document_to_specgraph + specgraph_to_document
3. `src/specgraph/hrf.rs` — parse + export
4. `docs/spec-format-guide.md` — document the new field
5. `examples/` — add or update an example file
6. Build + run `cargo test`

## read-repo Command

1. Explore repo: files, modules, dependency edges
2. Infer node kinds: packages/modules → shape nodes; APIs/interfaces → connector nodes
3. Generate HRF `.spec` and YAML side-by-side
4. Write `<repo-name>-structure.spec` to the repo root (or cwd)
5. Print the spec text so user can paste into light-figma's Import dialog
