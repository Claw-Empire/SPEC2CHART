# General Diagramming Language Design
**Date:** 2026-03-19
**Status:** Approved

## Goal

Make light-figma a general-purpose diagramming language and tool that serves all user roles — engineers, product managers, businessmen, AI agents, and more — through a universal HRF vocabulary backed by role-specific templates and an LLM fast-path entry point.

---

## User Roles

| Role | Entry Point | Primary Diagram Types |
|---|---|---|
| Platform / Backend Engineer | Write HRF directly | Architecture, dependency maps |
| Tech Lead / Architect | HRF in git | Architecture, ADR, RFC diagrams |
| DevOps / SRE | HRF or template | Incident maps, runbooks, infra topology |
| Data Engineer | HRF or LLM | Data pipelines, ETL flows, DAGs |
| Security Engineer | HRF template | Threat models, network topology |
| Engineering Manager | Template + edit | Org charts, team topology |
| Product Manager | LLM fast-path | Roadmaps, user journeys, swimlanes |
| Businessman / Founder | LLM fast-path | GTM strategy, roadmaps, investor decks |
| AI Agent | CLI / stdin pipe | Any diagram type, programmatic |
| UX Designer | Template + GUI | User journeys, screen flows |

---

## Architecture

```
Plain English
      ↓  [LLM fast-path]
   HRF Spec  ←──────── AI Agent (writes directly)
      ↓  [parser]      Human (writes or edits)
  FlowchartDocument    Template (scaffolds HRF)
      ↓  [layout engine]
  Timeline / Swimlane / OrgTree / Kanban / Existing
      ↓  [renderer]
  Canvas (interactive) → SVG / PDF / Mermaid export
```

One language. Every role. Every diagram type. All existing `.spec` files remain valid.

---

## Section 1: HRF Language Extensions

### New Layout Sections

```
## Timeline              → horizontal time axis, nodes placed by {date:} or {phase:}
## Swimlane: <Name>      → horizontal lane, nodes flow left-to-right inside it
## OrgTree               → forced top-down hierarchy layout
## Kanban: <Name>        → vertical column, nodes stack inside it
```

Multiple layout sections can coexist in one spec. A GTM diagram can combine `## Swimlane: Channel` with `## Timeline`.

### New Data Decorators

```
{date:2026-Q2}           → places node on timeline axis (layout only, not visible)
{phase:Discovery}        → groups node into a named phase band
{lane:Sales}             → assigns node to a swimlane
{metric:$2.4M ARR}       → badge overlay pinned to bottom-right of node
{owner:@alice}           → avatar circle + name pinned to top-right
{milestone}              → diamond shape shorthand on timeline axis
{dep:nodeId}             → auto-generates dashed dependency edge, no Flow entry needed
```

### New Shape Tags

```
{shape:person}           → circle head + body silhouette
{shape:milestone}        → diamond (alias for {milestone})
{shape:screen}           → rounded rect + top chrome bar
{shape:cylinder}         → database drum with top ellipse
{shape:cloud}            → cloud blob outline
{shape:document}         → rectangle with folded corner
{shape:callout}          → speech bubble annotation
{shape:channel}          → funnel shape
{shape:segment}          → person-group shape
```

### Business-Specific Tags

```
{revenue}                → green fill preset
{cost}                   → red fill preset
{growth}                 → upward arrow badge overlay
{risk}                   → warning triangle badge
{opportunity}            → star badge overlay
```

### Template Bundles (Config key)

```
template: roadmap        → activates {milestone} {phase:} {date:} {dep:} defaults
template: gtm            → activates {channel} {segment} {revenue} funnel shapes
template: orgchart       → activates {shape:person} {owner:} {capacity:} {team:}
template: pipeline       → activates {shape:cylinder} {throughput:} {transform}
template: journey        → activates {shape:screen} {lane:} {step:} {emotion:}
template: threat-model   → activates {trust-zone} {threat:} {asset:} {attack-path}
```

Bundles activate layout defaults and autocomplete hints only — any tag works in any template.

### HRF Examples

**Roadmap:**
```
## Config
template: roadmap
flow = LR

## Timeline
- [q1] Q1 2026 {phase:Q1}
- [q2] Q2 2026 {phase:Q2}

## Nodes
- [oss] OSS Launch {milestone} {phase:Q1} {done} {owner:@joe}
- [pro] Pro Plan {milestone} {phase:Q2} {wip}
- [api] Embed API {milestone} {phase:Q2} {todo}

## Flow
oss --> pro: unlocks
oss --> api: drives demand
```

**GTM Strategy:**
```
## Config
flow = TB

## Swimlane: Awareness
- [hn] HN Launch {star} {done}
- [blog] Dev Blog {star} {wip}

## Swimlane: Acquisition
- [dl] Free Download {hexagon} {metric:800 users}
- [gh] GitHub Stars {hexagon} {metric:2.4k}

## Swimlane: Revenue
- [pro] Pro Plan {diamond} {metric:$12/mo} {todo}

## Flow
hn --> dl: traffic
blog --> gh: stars
dl --> pro: upgrade
```

---

## Section 2: Layout Engine

New layout functions added to `layout.rs`, all assign `Node.position` before render. No new render model required.

### `timeline_layout()`
- Draws horizontal time axis across canvas
- Phase bands rendered as soft background rectangles with label at top
- Nodes with `{phase:}` or `{date:}` snap into correct band column
- Milestone nodes render as diamonds on the axis line
- Nodes without date float in an unscheduled zone below axis

### `swimlane_layout()`
- Each `## Swimlane: <Name>` creates a horizontal row
- Lane label rendered as bold pill on left margin
- Nodes flow left-to-right within their lane (existing hierarchical spacing)
- Cross-lane edges render as curved arcs

### `orgtree_layout()`
- Forces Reingold-Tilford top-down tree layout
- Ignores `flow =` config, always TB
- Nodes with `{shape:person}` get avatar styling
- `{metric:}` badge shows headcount / capacity
- Respects existing `collapsed` field for subtree folding

### `kanban_layout()`
- Each `## Kanban: <Name>` creates a vertical column
- Columns render side by side
- Nodes stack vertically inside columns
- `auto-kanban = true` in Config: `{wip}` `{todo}` `{done}` nodes auto-assign to matching column

### Renderer Additions (render.rs)
- **Phase band backgrounds** — colored `Rect` behind node groups, semi-transparent fill
- **Lane dividers** — horizontal lines with left-margin bold labels
- All other rendering unchanged

---

## Section 3: Template Gallery

### UI

New "New Diagram" button in toolbar opens a full-canvas overlay panel (`template_gallery.rs`).

```
┌─────────────────────────────────────────────────────┐
│  New Diagram                              [× close]  │
│  [Search templates...]                               │
│                                                      │
│  ── Engineering ──────────────────────────────────   │
│  [Architecture]  [Data Pipeline]  [Threat Model]     │
│                                                      │
│  ── Product & Strategy ───────────────────────────   │
│  [Roadmap]  [GTM Strategy]  [User Journey]           │
│                                                      │
│  ── People & Org ─────────────────────────────────   │
│  [Org Chart]  [Team Topology]  [RACI Matrix]         │
│                                                      │
│  ── Operations ────────────────────────────────────  │
│  [Incident Map]  [Runbook]  [On-Call Tree]           │
│                                                      │
│  ── Blank ─────────────────────────────────────────  │
│  [Empty Canvas]  [Empty HRF]                         │
└─────────────────────────────────────────────────────┘
```

`Cmd+N` opens the gallery.

### Template Files

```
templates/
  engineering/
    architecture.spec
    data-pipeline.spec
    threat-model.spec
  strategy/
    roadmap.spec
    gtm-strategy.spec
    user-journey.spec
  org/
    org-chart.spec
    team-topology.spec
  ops/
    incident-map.spec
    runbook.spec
```

Templates bundled via `include_str!()` at compile time — zero file I/O at runtime. Each template is valid HRF with placeholder content, fully editable after loading.

### LLM Fast-Path

Each template card includes a plain-English input field:

```
┌────────────────────────────────────────────────┐
│ Roadmap                                        │
│ ┌──────────────────────────────────────────┐  │
│ │ Describe your roadmap in plain English…  │  │
│ └──────────────────────────────────────────┘  │
│                          [Generate with AI →]  │
└────────────────────────────────────────────────┘
```

User types plain English → LLM generates valid HRF using template vocabulary → canvas loads. Reuses existing `llm.rs` pipeline with template-specific system prompt injection. Zero-friction entry for PM, Businessman, AI Agent.

---

## Section 4: AI Agent Support

The AI Agent role requires headless operation — no GUI.

### CLI Extensions

```bash
# Render spec to SVG without opening GUI
light-figma render arch.spec --out arch.svg

# Pipe prose to diagram
cat repo-summary.txt | light-figma generate --template architecture > arch.spec

# Diff two specs, output changed nodes/edges
light-figma diff old.spec new.spec

# Watch directory, auto-regenerate on change
light-figma watch src/ --template pipeline --out docs/pipeline.svg

# Validate HRF syntax
light-figma validate arch.spec

# Export HRF grammar for LLM context injection
light-figma schema --template roadmap
```

### Embed API

REST endpoint for embedding use case:
```
POST /render
Content-Type: text/plain
Body: <HRF spec text>

Response: image/svg+xml
```

Enables Notion/Confluence embeds, CI/CD PR comment rendering, documentation pipelines.

### CI/CD Use Case

On every PR:
1. AI agent reads changed source files
2. Calls `light-figma generate` to update architecture spec
3. Calls `light-figma render` to produce SVG
4. Posts SVG as PR comment
5. Diagrams never go stale

---

## Implementation Plan (high level)

### Phase 1 — Language (hrf.rs + model.rs)
- Add new shape tags to `NodeKind` / shape parsing
- Add data decorator parsing (`{metric:}` `{owner:}` `{phase:}` `{date:}` `{dep:}` `{lane:}`)
- Add business tags (`{revenue}` `{cost}` `{growth}` `{risk}` `{opportunity}`)
- Add `template:` config key (activates layout hints)
- Add `## Timeline`, `## Swimlane`, `## OrgTree`, `## Kanban` section parsing

### Phase 2 — Layout Engine (layout.rs)
- `timeline_layout()`
- `swimlane_layout()`
- `orgtree_layout()`
- `kanban_layout()`

### Phase 3 — Renderer (render.rs)
- Phase band backgrounds
- Lane dividers with labels
- New shape renderers: person, screen, cylinder, cloud, document, callout
- Metric badge overlay
- Owner avatar overlay

### Phase 4 — Template Gallery (template_gallery.rs)
- Gallery UI panel
- Bundled template files
- LLM fast-path integration
- `Cmd+N` shortcut

### Phase 5 — CLI / Headless (main.rs)
- `render` subcommand
- `generate` subcommand
- `diff` subcommand
- `validate` subcommand
- `schema` subcommand

---

## Backwards Compatibility

- All existing `.spec` files load without modification
- All existing tags, shapes, edge styles preserved
- `hierarchical_layout()` unchanged, still default
- 3D view works with all new layout types
- New sections/tags are additive only
