# SpecGraph Format Design

**Date:** 2026-03-10
**Status:** Approved
**Approach:** Layered — YAML core + natural language skin (LLM-assisted)

## Problem

Convert bidirectionally between natural language product specifications and the visual graph in light-figma. Enable A/B testing of different LLM models/providers for conversion quality.

## Architecture

Two layers:
- **Core layer**: YAML format ("SpecGraph") that maps 1:1 to `FlowchartDocument`. Deterministic, lossless, always correct.
- **Natural language layer**: Freeform prose that the LLM converts to/from the YAML core. This is what humans read/write.

Round-trip flow:
```
Prose spec ──LLM──> YAML ──parser──> Graph
Graph ──serializer──> YAML ──LLM──> Prose spec
```

A/B testing lives at the LLM boundaries — swap prompts, models, or providers to compare conversion quality. The YAML↔Graph path is deterministic.

## SpecGraph YAML Format (v1.0)

```yaml
specgraph: "1.0"
title: User Login Flow
mode: flowchart  # flowchart | er | figjam

nodes:
  - id: n1
    kind: shape
    shape: rounded_rect  # rectangle | rounded_rect | diamond | circle | parallelogram
    label: "User opens login page"
    description: ""
    position: [100, 200]       # optional — auto-layout fills defaults
    size: [160, 80]            # optional
    style:                     # optional — theme defaults apply
      fill: [30, 30, 46, 255]
      border: [137, 180, 250, 255]
      border_width: 2.0
      text_color: [205, 214, 244, 255]
      font_size: 14.0

  - id: n2
    kind: entity
    name: "User"
    attributes:
      - { name: "id", pk: true, fk: false }
      - { name: "email", pk: false, fk: false }

  - id: n3
    kind: sticky
    text: "TODO: confirm with PM"
    color: yellow  # yellow | pink | green | blue | purple

  - id: n4
    kind: text
    content: "Annotation text here"

edges:
  - from: { node: n1, side: bottom }
    to: { node: n2, side: top }
    label: "validates"
    source_label: ""
    target_label: ""
    source_cardinality: none      # none | exactly_one | zero_or_one | one_or_many | zero_or_many
    target_cardinality: none
    style:
      color: [205, 214, 244, 255]
      width: 2.0

metadata:
  created: "2026-03-10"
  llm_source: "claude-opus-4-6"  # tracks which model generated this
  ab_variant: "A"                 # for A/B testing
```

### Field Rules

- `specgraph` (required): Format version string.
- `title` (required): Human-readable name for the diagram.
- `mode` (required): One of `flowchart`, `er`, `figjam`.
- `nodes[].id` (required): Unique string identifier, referenced by edges.
- `nodes[].kind` (required): One of `shape`, `entity`, `sticky`, `text`.
- `nodes[].position`, `nodes[].size`, `nodes[].style`: Optional. When omitted (typical for LLM output), the app applies auto-layout and theme defaults.
- `edges[].from`, `edges[].to`: Required. `node` references a node id, `side` is one of `top`, `bottom`, `left`, `right`.
- `edges[].source_cardinality`, `edges[].target_cardinality`: For ER mode. Default `none`.
- `metadata`: Optional. Tracks provenance for A/B testing.

## LLM Integration

### Configuration

Stored in app settings (persisted alongside other preferences):

```yaml
llm:
  provider: "openai"          # label only, not functional — endpoint determines behavior
  endpoint: "https://api.openai.com/v1/chat/completions"
  api_key: "sk-..."
  model: "gpt-4o"
  temperature: 0.2
```

Provider/endpoint/key/model are all user-configurable. The app sends standard OpenAI-compatible chat completion requests (most providers support this format).

### Conversion: Spec → Graph

1. User pastes or loads a prose spec.
2. App sends prose + system prompt to LLM endpoint.
3. LLM returns SpecGraph YAML.
4. App validates YAML against schema, parses with `serde_yaml`.
5. App converts SpecGraph structs → `FlowchartDocument`.
6. Auto-layout positions nodes if positions were omitted.
7. Graph appears on canvas.

### Conversion: Graph → Spec

1. User triggers "Export as Spec".
2. App converts `FlowchartDocument` → SpecGraph YAML.
3. App sends YAML + system prompt to LLM endpoint.
4. LLM returns readable natural language prose.
5. App displays prose for copy/export.

### A/B Testing

1. User configures two LLM profiles (e.g., Profile A: Claude, Profile B: GPT-4o).
2. Same prose spec is sent to both.
3. Both resulting graphs are displayed side-by-side.
4. Metadata tracks `llm_source` and `ab_variant` for comparison.

## App Integration

### New UI Elements

- **Import Spec** button (toolbar) — paste or load prose → LLM → YAML → graph
- **Export Spec** button (toolbar) — graph → YAML → LLM → readable prose
- **Edit YAML** panel — direct YAML editor for power users (no LLM needed)
- **A/B Compare** view — side-by-side graph comparison from two LLM runs
- **LLM Settings** in preferences — provider, endpoint, key, model config

### New Rust Modules

```
src/
├── specgraph/
│   ├── mod.rs          # SpecGraph format types, public API
│   ├── schema.rs       # SpecGraph YAML schema structs (serde)
│   ├── convert.rs      # Bidirectional FlowchartDocument ↔ SpecGraph
│   ├── llm.rs          # HTTP client for LLM API calls (reqwest)
│   └── prompts.rs      # System/user prompt templates for spec↔YAML
```

### Dependencies

- `serde_yaml` — YAML parsing/serialization
- `reqwest` — HTTP client for LLM API calls (async with tokio)
- Existing: `serde`, `serde_json`, `uuid`

## Design Decisions

1. **YAML over JSON for the spec format**: More human-readable, supports comments, LLMs produce it reliably. JSON remains the native `.flow` save format.
2. **OpenAI-compatible API format**: Most LLM providers (Anthropic, OpenAI, local Ollama, etc.) support this format, making the provider truly pluggable.
3. **Optional position/size/style**: LLMs shouldn't need to calculate pixel positions. Auto-layout handles this.
4. **String IDs over UUIDs in spec**: Simpler for humans and LLMs to read/write. Converted to UUIDs on import.
5. **Metadata for A/B testing**: Embedded in the format so provenance travels with the data.
