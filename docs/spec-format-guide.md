# SpecGraph Format Guide

Light Figma supports three import formats, all accessible via **SPEC → Import** in the toolbar.
The app auto-detects which format you're using.

---

## 1. Human-Readable Format (`.spec`)

The recommended format for non-technical users. Plain text with minimal syntax.

### Structure

```
# Diagram Title

Overall description of the diagram. Explain what this diagram represents,
its scope, and any important context. Can span multiple paragraphs.

## Nodes

- [id] Node Label
  Optional description for this node.
  Can span multiple indented lines.

- [id] Node Label {shape}
  Description here.

## Flow

id "connection label" --> id
id --> id

## Notes

- Sticky note text {color}
```

### Node Shapes

Add a `{shape}` tag at the end of the node line:

| Tag | Shape | Use for |
|-----|-------|---------|
| *(none)* | Rounded rectangle (default) | General steps, components |
| `{diamond}` | Diamond | Decisions, branch points |
| `{rectangle}` | Rectangle | Processes, tasks |
| `{circle}` | Circle | Start / end points |
| `{parallelogram}` | Parallelogram | I/O, data |
| `{connector}` | Pill / capsule | APIs, protocols, interfaces |

### Connector Nodes

Connector nodes (`{connector}`) model the **connection itself** — the API, protocol, or interface between two components. They render as small pill shapes and sit along the edge path.

Use them when the connection has important properties worth documenting:

```
## Nodes
- [rest_api] REST API {connector}
  HTTP/JSON over HTTPS. Auth: Bearer JWT.
  Rate limited to 1000 req/min.

- [grpc] gRPC Channel {connector}
  Protocol Buffers, HTTP/2, mTLS.

## Flow
user_service --> rest_api --> order_service
order_service --> grpc --> payment_service
```

Aliases also work: `{api}`, `{interface}`, `{protocol}`, `{gateway}` all produce connector nodes.

### Sticky Note Colors

| Tag | Color |
|-----|-------|
| `{yellow}` | Yellow (default) |
| `{pink}` | Pink |
| `{green}` | Green |
| `{blue}` | Blue |
| `{purple}` | Purple |

### Flow Syntax

```
# Simple connection
source --> target

# Labeled connection
source "label text" --> target
```

### Full Example

```
# User Login Flow

Describes the authentication flow for the web application.
Users enter credentials which are validated against the database.

## Nodes

- [start] User opens app
  Entry point. The user arrives at the login screen.

- [form] Show login form
  Displays email and password fields with a submit button.

- [check] Valid credentials? {diamond}
  Validates credentials against the user database.
  Returns true/false.

- [dashboard] Show dashboard
  Main application screen shown on successful login.

- [error] Show error message
  Displays "Invalid email or password" and lets the user retry.

## Flow

start --> form
form "submit" --> check
check "yes" --> dashboard
check "no" --> error
error --> form

## Notes

- Rate limit to 5 attempts before lockout {pink}
- Remember to hash passwords with bcrypt {yellow}
```

---

## 2. SpecGraph YAML (`.yaml`)

The structured format for technical users. Gives full control over
positions, sizes, styles, and cardinality.

```yaml
specgraph: "1.0"
title: Diagram Title
mode: flowchart  # flowchart | er | figjam

nodes:
  - id: n1
    kind: shape
    shape: rounded_rect  # rectangle | rounded_rect | diamond | circle | parallelogram
    label: "Node label"
    description: "Optional description"
    position: [100, 200]   # optional — auto-layout if omitted
    size: [160, 80]        # optional
    style:                 # optional — theme defaults apply
      fill: [49, 50, 68, 255]
      border: [137, 180, 250, 255]
      border_width: 2.0
      text_color: [205, 214, 244, 255]
      font_size: 14.0

  - id: n2
    kind: entity
    name: "TableName"
    attributes:
      - { name: "id", pk: true, fk: false, type: "uuid" }
      - { name: "email", pk: false, fk: false, type: "text" }

  - id: n3
    kind: sticky
    text: "Note text"
    color: yellow  # yellow | pink | green | blue | purple

  - id: n4
    kind: text
    content: "Plain text annotation"

edges:
  - from: { node: n1, side: bottom }  # top | bottom | left | right
    to: { node: n2, side: top }
    label: "connection label"
    source_cardinality: none         # none | exactly_one | zero_or_one | one_or_many | zero_or_many
    target_cardinality: zero_or_many
    style:
      color: [205, 214, 244, 255]
      width: 2.0

metadata:
  created: "2026-03-10"
  llm_source: "claude-opus-4-6"
  ab_variant: "A"
```

---

## 3. Prose / Free Text (`.txt`)

Write in plain English — the LLM converts it to a diagram automatically.
Requires an API key configured in **SPEC → LLM Settings**.

```
A venture capital fund receives capital from Limited Partners such as
pension funds and endowments. The General Partner manages the fund and
charges a 2% annual management fee. Capital is deployed into portfolio
companies after due diligence. When companies exit via IPO or acquisition,
proceeds are distributed: 80% to LPs and 20% carried interest to the GP.
```

### LLM Settings

Configure in the toolbar under **SPEC → LLM Settings**:

| Field | Description | Example |
|-------|-------------|---------|
| Endpoint | OpenAI-compatible API URL | `https://api.openai.com/v1/chat/completions` |
| API Key | Your secret key | `sk-...` |
| Model | Model name | `gpt-4o`, `claude-opus-4-6` |

Works with any OpenAI-compatible provider: OpenAI, Anthropic, Mistral, local Ollama, etc.

---

## Export

**SPEC → Export** saves the current diagram:
- Save as `.yaml` → SpecGraph YAML format
- Save as `.spec` → Human-Readable format (round-trips cleanly)

---

## Format Detection

The app auto-detects the format on import:

| Condition | Format |
|-----------|--------|
| Starts with `specgraph:` | YAML |
| Contains `## Nodes` or `## Flow` | Human-Readable |
| Everything else | Prose (requires LLM) |
