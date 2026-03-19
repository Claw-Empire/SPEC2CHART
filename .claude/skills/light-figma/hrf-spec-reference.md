# HRF Spec Syntax Reference

Human-Readable Format for light-figma `.spec` files.
Parse/export lives in `src/specgraph/hrf.rs`. Tests: ~64 in `mod tests` block.

---

## Sections

| Header | Alias(es) | Purpose |
|---|---|---|
| `## Nodes` | `## Node` | Node definitions |
| `## Flow` | `## Connections`, `## Edges` | Edge definitions |
| `## Notes` | `## Note`, `## Stickies` | Sticky note nodes |
| `## Groups` | `## Group`, `## Clusters` | Named bounding frames |
| `## Steps` | `## Step` | Sequential flowchart steps |
| `## Grid cols=N` | — | Auto-grid layout, N columns |
| `## Config` | `## Settings`, `## Meta`, `## Options`, `## Diagram` | Import-time settings |
| `## Palette` | — | Named color variables |
| `## Style` | — | Named tag template expansions |
| `## Summary` | `## About`, `## Overview`, `## Readme`, `## Description` | Document description |
| `## Layer N` | `## Layer N: Name` | Nodes at z=N×120; optional semantic name |

---

## Node Syntax

```
- [id] Display Label {tags...}
  Optional tooltip / description line (indented).
```

- `[id]` is optional — auto-generated from slugified label if omitted.
- Multiple description lines can be indented under the node line.
- Inline edges: `- [api] Service → db, cache {dashed}`

---

## Node Tags — Shapes

| Tag | Shape |
|---|---|
| `{rectangle}` / `{rect}` | Rectangle (default) |
| `{rounded_rect}` / `{rounded}` | Rounded rectangle |
| `{circle}` / `{ellipse}` | Circle |
| `{diamond}` / `{decision}` | Diamond |
| `{parallelogram}` / `{para}` | Parallelogram |
| `{hexagon}` / `{hex}` | Hexagon |
| `{connector}` / `{dot}` | Small connector dot |
| `{frame}` | Bounding frame container |
| `{entity}` | ER entity table |
| `{text}` | Plain text node |

**Semantic presets (set shape + fill + icon):**
`{server}` `{database}` `{cloud}` `{user}` `{service}` `{queue}` `{cache}` `{internet}` `{decision}` `{start}` `{end}`

**Property-style syntax also accepted:**
`{shape:circle}` `{type:diamond}` `{kind:hexagon}`

---

## Node Tags — Color & Style

| Tag | Effect |
|---|---|
| `{fill:blue}` | Named color fill |
| `{fill:#rrggbb}` | Hex fill |
| `{fill:none}` | Transparent fill |
| `{border-color:red}` / `{bc:red}` | Border color |
| `{text-color:white}` / `{tc:white}` | Text color |
| `{bold}` `{italic}` | Text style |
| `{shadow}` | Drop shadow |
| `{gradient}` | Top→bottom gradient |
| `{gradient-angle:45}` / `{grad-angle:45}` | Gradient direction (degrees) |
| `{dashed-border}` / `{dashed-outline}` | Dashed border |
| `{highlight}` | Highlight tint |
| `{glow}` / `{neon}` | Neon border halo |
| `{frame-color:#rrggbb}` | Frame background fill |
| `{r:N}` / `{corner-radius:N}` | Corner radius override |
| `{border:N}` | Border width in px |
| `{font-size:N}` / `{fs:N}` | Font size override |
| `{align:left/right/center}` | Text alignment |
| `{valign:top/bottom/middle}` | Vertical text alignment |

**Named colors:** `blue` `green` `red` `yellow` `purple` `teal` `orange` `sky` `lavender` `gray` `white` `black` `pink` `none`

---

## Node Tags — Opacity

| Tag | Opacity |
|---|---|
| `{dim}` / `{dimmed}` | 35% — de-emphasise secondary |
| `{ghost}` / `{faded}` | 18% — barely visible (out-of-scope) |
| `{muted}` | 60% — subtle de-emphasis |
| `{hidden}` / `{invisible}` | 0% — invisible |
| `{opacity:50}` | N% (0–100) or 0.0–1.0 float |

Export uses friendly names for the canonical values.

---

## Node Tags — Size & Position

| Tag | Effect |
|---|---|
| `{w:200}` `{h:80}` | Explicit width / height |
| `{size:200x80}` | Shorthand for w+h |
| `{pos:100,200}` | Position (pinned) |
| `{x:100}` `{y:200}` | Explicit coordinate |
| `{pinned}` | Pin node (prevent auto-layout) |
| `{locked}` | Lock against user editing |
| `{collapsed}` | Render as compact pill |

---

## Node Tags — Status & Progress

| Tag | Badge | Progress |
|---|---|---|
| `{done}` / `{complete}` / `{finished}` | Ok (green) | 100% |
| `{wip}` / `{in-progress}` / `{doing}` | Info (blue) | 50% |
| `{review}` / `{reviewing}` / `{pending}` | Warning (yellow) | 75% |
| `{blocked}` / `{issue}` / `{bug}` | Critical (red) | — |
| `{todo}` / `{planned}` / `{not-started}` | Warning | — |
| `{critical}` | Critical | — |
| `{warning}` / `{warn}` | Warning | — |
| `{ok}` / `{success}` | Ok | — |
| `{info}` | Info | — |
| `{progress:75}` | — | 75% |
| `{status:done}` | (property-style alias) | |

---

## Node Tags — 3D

| Tag | Effect |
|---|---|
| `{z:120}` | Z-offset in world units |
| `{layer:db}` | z=0 |
| `{layer:api}` | z=120 |
| `{layer:frontend}` | z=240 |
| `{layer:edge}` | z=360 |
| `{layer:infra}` | z=480 |
| `{layer:N}` | z=N×120 |
| `{3d-depth:80}` | Per-node extrusion depth |
| `{tier-color}` | Apply tier-based fill tint |

`## Layer N: Name` section header assigns z=N×120 and sets a semantic layer name.
Config: `auto-tier-color = true` tints all non-z0 nodes with their tier color.

---

## Node Tags — Metadata

| Tag | Effect |
|---|---|
| `{icon:🔒}` | Emoji icon rendered in top-left (large watermark behind label) |
| `{url:https://...}` / `{link:...}` | Clickable URL |
| `{note:text}` / `{comment:text}` / `{annotation:text}` | Tooltip annotation |
| `{tooltip:text}` | Tooltip text (alias for note) |
| `{sublabel:v2}` | Small secondary text below label |
| `{group:name}` / `{cluster:name}` / `{in:name}` | Assign to named group frame |

---

## Node Labels

- `\n` in label = actual newline on canvas: `- [a] Line 1\nLine 2`
- Export escapes `\n` back to `\\n` for round-trip safety.

---

## Flow Syntax

```
source --> target
source --> middle --> target    # chain
source -> target                # -> is alias for -->
source <-- target               # reverse arrow
source <-> target               # bidirectional
```

**Edge label variants:**
```
a "calls" --> b             # quoted label on source side
a --> b: performs auth      # colon suffix
a --> |performs auth| b     # pipe label (Mermaid-style)
```

**Style shorthand arrows:**
```
a -.-> b      # dashed edge
a ==> b       # thick edge
a ~~> b       # animated edge
```

**Unicode arrows:** `→` `⇒` `⟶` `←` `⟵` `↔` `⇔` `⟷` all accepted.

**Multi-source/target:**
```
[a, b, c] --> target {tags}
source --> [a, b, c] {tags}
```

**Natural language references (label-based lookup):**
```
REST API --> Database           # slugified label match
"REST API" --> "Database"      # quoted label reference (explicit)
```
Both are fallbacks when explicit `[id]` lookup fails.

**Inline comments:** `a --> b  // ignored`

---

## Edge Tags

| Tag | Effect |
|---|---|
| `{dashed}` | Dashed line |
| `{thick}` | Thick line |
| `{animated}` | Animated flow |
| `{ortho}` / `{orthogonal}` | Right-angle routing |
| `{glow}` | Glowing edge |
| `{arrow:open/circle/none/filled}` | Arrowhead style |
| `{bend:0.3}` | Curve bend (-1 to 1) |
| `{weight:2}` | Edge weight (maps to width) |
| `{color:blue/#rrggbb}` | Edge color |
| `{from:label}` | Source endpoint label |
| `{to:label}` | Target endpoint label |
| `{c-src:1}` | Source cardinality |
| `{c-tgt:0..N}` | Target cardinality |
| `{src-port:top/l/r/bottom}` / `{sport:...}` | Source port side |
| `{tgt-port:...}` / `{tport:...}` | Target port side |
| `{note:text}` / `{comment:text}` | Edge annotation tooltip |

---

## Config Keys (`## Config`)

| Key | Values | Effect |
|---|---|---|
| `flow` / `direction` | `TB` `LR` `RL` `BT` | Layout direction |
| `spacing` / `gap` | number | Both gap-main and gap-cross (cross = 75% of value) |
| `gap-main` / `layer-spacing` | number | Between-layer gap (default 80) |
| `gap-cross` / `node-gap` | number | Within-layer gap (default 60) |
| `bg` / `background` | `dots` `lines` `crosshatch` `none` | Canvas background pattern |
| `bg-color` / `background-color` | `#rrggbb` or named | Canvas background color |
| `grid-size` | number | Snap grid size |
| `snap` | `true/false` | Snap to grid |
| `zoom` | float | Initial zoom level |
| `view` / `mode` | `2d` `3d` | Initial view mode |
| `camera` | `iso` `top` `front` `side` | 3D camera preset |
| `auto-z` / `auto_z` | `true/false` | Auto-assign z from topology |
| `auto-tier-color` / `tier-color` | `true/false` | Apply tier-based fill tints |
| `layer0` / `layer 0` | string | Semantic name for z=0 tier |
| `layer1` / `layer 1` | string | Semantic name for z=120 tier |
| `project-title` / `watermark` | string | Canvas watermark text |

---

## Palette (`## Palette`)

```
brand    = #1e3a5f
accent   = orange
danger   = #e63946
```

Use anywhere as `{fill:brand}` `{border-color:accent}`.

---

## Style Templates (`## Style`)

```
primary = {fill:blue} {bold} {highlight}
warning = {fill:yellow} {warning}
```

Use as `{primary}` `{warning}` on any node or edge.

---

## Layer Sections

```
## Layer 0: Database
- [db] PostgreSQL {database}

## Layer 1: Backend
- [api] REST API
```

- Sets `z_offset = N × 120.0` on all nodes in section.
- Sets `doc.layer_names[N] = "Database"` for 3D legend.
- `auto-tier-color = true` tints nodes by tier (z≠0 nodes only).

---

## Steps Section

```
## Steps
1. Receive Request {diamond}
2. Validate Input
3. Process {server}: handles auth
4. Return Response
```

- Auto-connects nodes left→right.
- Number prefix increments `current_step`.
- Colon after label becomes sublabel (step description).

---

## Groups Section

```
## Groups
- [backend] Backend Services {fill:teal}
  api, db, cache
```

Or inline on node: `{group:backend}` `{cluster:backend}` `{in:backend}`.

---

## Summary Section

```
## Summary
This diagram shows the microservice architecture for the payments platform.
All services communicate via REST over mTLS.
```

Text is stored in `doc.description`. Shown in properties panel.

---

## Export Roundtrip Notes

- Nodes at default position `[0,0]` are re-laid-out on import; others keep position.
- `#[serde(default)]` on all new fields ensures backwards compat.
- HRF export uses `## Layer N: Name` headers when layer names are set.
- Friendly opacity names (`{dim}`, `{ghost}`, `{muted}`, `{hidden}`) used on export for canonical values.
- Style arrows (`-.->`, `==>`, `~~>`) parsed before Unicode normalization.
- `\n` in labels is escaped to `\\n` on export, unescaped on parse.
