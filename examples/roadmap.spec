# light-figma Roadmap

Strategic roadmap from current state through monetization.

## Config
flow = TB
auto-tier-color = true
spacing = 120

## Layer 0: Foundation
- [engine] Core Engine {rounded_rect} {done} {icon:⚙️}
  Canvas, zoom/pan, undo/redo, save/load — 19k lines Rust/egui.
- [nodes] Node System {rounded_rect} {done} {icon:🔷}
  7+ shapes, tags, badges, icons, grouping, 3D extrusion.
- [edges] Edge System {rounded_rect} {done} {icon:🔗}
  Bezier, orthogonal, arrow heads, labels, animated/glow styles.
- [specgraph] SpecGraph / HRF {rounded_rect} {done} {icon:📝}
  YAML + HRF import, LLM prose-to-diagram, hierarchical layout.
- [exports] Export Pipeline {rounded_rect} {done} {icon:📤}
  PNG, SVG, PDF, Mermaid export.
- [ux] UX Polish {rounded_rect} {done} {icon:✨}
  Command palette, search, minimap, bookmarks, animations, rulers.

## Layer 1: Stability
- [pkg] macOS .app Bundle {hexagon} {wip} {icon:📦}
  Ship downloadable binary — scaffolded but not complete.
- [autosave] Crash Hardening {hexagon} {todo} {icon:🛡️}
  Autosave, file versioning, corruption recovery.
- [ci] Cross-Platform CI {hexagon} {todo} {icon:🏗️}
  Linux AppImage, Windows .exe via GitHub Actions.
- [tests] Test Coverage {hexagon} {todo} {icon:🧪}
  Unit tests for model, roundtrip tests for spec import/export.

## Layer 2: Ecosystem
- [collab] Real-Time Collab {diamond} {todo} {icon:👥}
  CRDT or OT-based multi-user editing.
- [cloud] Cloud Sync {diamond} {todo} {icon:☁️}
  Save/load from S3, Supabase, or similar.
- [plugins] Plugin System {diamond} {todo} {icon:🔌}
  User-defined node types, renderers, extensions.
- [templates] Template Library {diamond} {todo} {icon:📋}
  Pre-built diagrams: architecture, ER, flowchart, org chart.
- [wasm] WASM Web Build {diamond} {todo} {icon:🌐}
  Browser version via egui WebAssembly support.

## Layer 3: Differentiators
- [llm] LLM-Native Workflows {star} {todo} {glow} {icon:🤖}
  "Describe → Diagram" as first-class UX. Already seeded in llm.rs.
- [codediag] Code-to-Diagram {star} {todo} {glow} {icon:💻}
  Auto-generate live architecture diagrams from repos.
- [present] Presentation Mode {star} {todo} {icon:🎬}
  Step through diagram states like slides.
- [embed] Embed / API Widget {star} {todo} {icon:🧩}
  Embeddable diagram for docs, READMEs, Notion.
- [market] Marketplace {star} {todo} {icon:🏪}
  Community-shared shapes, themes, spec templates.

## Layer 4: Business
- [opencore] Open-Core Model {parallelogram} {todo} {icon:💎}
  Free desktop app, paid cloud sync and collaboration.
- [clitool] CLI Dev Tool {parallelogram} {todo} {icon:⌨️}
  "figma-for-code" — diagram-as-code in CI pipelines.
- [apirev] API Revenue {parallelogram} {todo} {icon:💰}
  Charge for embeddable diagram widget.
- [llmrev] LLM Integration Rev {parallelogram} {todo} {icon:🧠}
  Charge for AI-generated diagrams — infra already exists.

## Flow
engine --> pkg: ship it
engine --> autosave
engine --> tests
nodes --> plugins: extensibility
edges --> plugins
specgraph --> llm: expand
specgraph --> codediag: generalize
exports --> embed: widgetize
ux --> present: reuse animations

pkg --> wasm: next platform
autosave --> cloud: remote storage
ci --> wasm
tests --> collab: safety net

collab --> opencore: monetize
cloud --> opencore
plugins --> market: ecosystem
templates --> market
wasm --> embed: browser-native

llm --> llmrev: revenue
codediag --> clitool: dev workflow
embed --> apirev: charge
market --> opencore: flywheel

## Notes
- Strengths: native Rust perf, LLM-native, code-aware, unique 3D view {fill:#2d5a3d}
- Risk: single developer, no tests yet, no distribution {fill:#5a2d2d}
- 220+ iterations shipped in 13 days — velocity is high {fill:#2d3a5a}

## Summary
light-figma roadmap: from a 19k-line Rust/egui canvas app through stability, ecosystem, differentiation, and monetization. Key bets are LLM-native diagramming and code-to-diagram workflows.
