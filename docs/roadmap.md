# light-figma Roadmap

> Companion document to `roadmap.spec` (product) and `biz-roadmap.spec` (business).
> Import either into light-figma → SPEC → Import → 3D view.

---

## Where We Are

light-figma started on Feb 28, 2026. In 13 days it grew to ~19,200 lines of Rust across 24 source files, with 149 commits and over 220 iteration features. The development velocity is extreme — roughly 15+ features shipped per day by a single developer using AI-assisted iteration loops.

The app is already a capable diagramming tool: a full canvas engine with zoom/pan/undo, seven node shapes with tags and badges, bezier and orthogonal edges with animated styles, a unique 3D architecture view that no competitor offers, and export to PNG, SVG, PDF, and Mermaid. The SpecGraph system lets users write diagrams as plain text (HRF format) or generate them from prose via LLM — this is the product's most distinctive capability.

What's missing is distribution. Nobody can use light-figma today without cloning the repo and running `cargo build`. The macOS .app bundle is scaffolded but not complete. There's no CI, no installer, no website. The test suite covers the HRF parser (~64 tests) but nothing else. The product is rich in features but fragile — one bad save could corrupt a user's work with no recovery path.

---

## Phase 1 — Stability & Distribution

The first priority is making light-figma installable and trustworthy. None of the later phases matter if users can't get the app or lose their work.

**Ship a macOS binary.** The .app bundle scaffolding exists. Finish it, code-sign it, and put a download link on a landing page. This is the single highest-leverage thing to do next — every other growth strategy depends on people being able to run the app.

**Add crash hardening.** Implement autosave (write a recovery file every 30 seconds), file versioning (keep the last 5 saves), and corruption detection on load. Users will forgive missing features but not lost work.

**Set up cross-platform CI.** GitHub Actions to build Linux AppImage and Windows .exe on every push. The egui ecosystem supports all three platforms — this is mechanical work, not risky engineering.

**Expand test coverage.** The HRF parser has 64 tests, but the model layer, export pipeline, and spec conversion have zero. Add roundtrip tests (parse → export → parse should be identity) and model unit tests. This is the safety net that makes rapid iteration sustainable.

---

## Phase 2 — Collaboration & Ecosystem

Once people can install light-figma and trust it with their work, the next step is making it social and extensible.

**Real-time collaboration** is the table-stakes feature that separates a tool from a product. CRDT-based sync (something like Automerge or Yrs) would let multiple users edit the same diagram simultaneously. This is hard engineering — probably 2-3 months of focused work — but it's what unlocks the Teams pricing tier.

**Cloud sync** is the simpler prerequisite: save/load from a remote backend (S3, Supabase, or a custom service). This also enables the freemium-to-paid conversion — local files are free, cloud files require a subscription.

**A plugin system** would let users add custom node types, renderers, and export formats without forking the codebase. This creates an ecosystem moat: the more plugins exist, the harder it is to switch away.

**A WASM web build** is strategically important because it removes the install barrier entirely. egui already supports WebAssembly — the work is in handling file I/O, clipboard, and fonts in the browser sandbox. A web version dramatically expands the addressable market.

**A template library** (pre-built architecture, ER, flowchart, and org chart diagrams) lowers the blank-canvas problem and gives new users immediate value.

---

## Phase 3 — Differentiators

This is where light-figma stops competing with Figma/Excalidraw on their terms and builds capabilities that are genuinely new.

**LLM-native workflows** are the biggest bet. The foundation already exists in `llm.rs` — users can describe a system in prose and get a diagram. The vision is to make this the primary way people create diagrams: "describe your microservice architecture" → instant visual. No dragging boxes. No learning a tool. Just describe and refine.

**Code-to-diagram** generalizes the `/light-figma read-repo` skill into a product feature. Point light-figma at a Git repo, and it generates a live architecture diagram that updates as the code changes. This is the "figma-for-code" positioning — diagrams that never go stale because they're derived from source.

**Presentation mode** would let users step through diagram states like slides — zoom into a subsystem, highlight a path, animate a flow. This turns light-figma from a design tool into a communication tool, which is where the real value is for engineering teams.

**An embeddable widget** (for docs, READMEs, Notion, Confluence) turns every diagram into a distribution channel. The embed links back to light-figma, creating organic growth.

**A marketplace** for community-shared shapes, themes, and spec templates creates a content flywheel and a revenue-share opportunity.

---

## Phase 4 — Business Model

The monetization strategy is open-core: the desktop app is free forever, and revenue comes from cloud services and AI features.

**Freemium desktop** — all shapes, all export formats, unlimited local files. This maximizes adoption. The free tier should feel generous, not crippled.

**Pro plan ($12/mo)** — cloud sync, sharing links, real-time collaboration, priority support. This is the individual-to-paid conversion path. The trigger is when someone wants to share a diagram with a teammate or access it from a second machine.

**Teams plan ($8/seat/mo)** — SSO, shared template library, admin controls, audit log. This is the enterprise wedge. The per-seat price is deliberately low to encourage broad adoption within organizations.

**API / Embed ($0.01/render)** — usage-based pricing for the embeddable diagram widget. This scales with the customer's traffic, not their team size. Documentation-heavy companies (Stripe, Vercel, etc.) would be ideal customers.

**AI diagram generation** — pay-per-diagram pricing for the LLM-powered "describe → diagram" feature. The cost structure is token pass-through plus margin. This is the highest-margin revenue stream because the value-to-user far exceeds the LLM inference cost.

---

## Go-to-Market

The launch sequence matters. Each step builds the audience for the next.

1. **Open-source on GitHub** with a downloadable binary, a compelling README, and a 30-second demo GIF showing the text-to-diagram workflow. Post to Hacker News and r/rust. The HRF format and 3D view are the hooks — they're visually distinctive and technically interesting enough to generate discussion.

2. **Developer content** — write 2-3 blog posts: "Diagram your codebase in 30 seconds with AI", "Why I built a Figma alternative in Rust", "Architecture diagrams that live in your Git repo". These establish the narrative and drive organic search traffic.

3. **Tool integrations** — a VS Code extension that renders `.spec` files inline, a CLI that pipes specs into the app (`cat arch.spec | light-figma`), and a CI action that generates architecture diagram badges. Each integration creates a new surface where developers encounter light-figma.

4. **Community** — Discord server and GitHub Discussions for template sharing, feature requests, and spec gallery. The community becomes both a support channel and a growth engine.

---

## Growth Flywheel

The long-term growth model has five reinforcing loops:

**Spec file virality.** Every `.spec` file someone shares requires light-figma to view and edit. The file format itself is a distribution mechanism — like how `.sketch` files drove Sketch adoption.

**Embed backlinks.** Embedded diagrams in documentation include a "Made with light-figma" watermark (removable on paid plans). Every embed is a backlink and a brand impression.

**Template marketplace.** Community-contributed templates attract new users looking for starting points. Revenue share (70/30 to creators) incentivizes supply. Popular templates become free marketing.

**CI/CD integration.** When light-figma generates architecture docs on every push, it becomes part of the daily development workflow. This creates deep stickiness — removing it means losing automated documentation.

**Education pipeline.** Free for students and coding bootcamps. New developers learn to diagram with light-figma before they've formed habits around other tools. This is a long-term play that pays off as students enter the workforce.

---

## Competitive Landscape

The diagramming market is crowded but undifferentiated. Figma, Miro, Lucidchart, Excalidraw, draw.io, and Mermaid all compete on roughly the same feature set: drag boxes, draw arrows, export PNG.

light-figma's strategic advantages are structural, not incremental:

**Native Rust performance.** The app loads instantly and maintains 60fps with 1000+ nodes. Browser-based tools (Figma, Excalidraw, Miro) hit performance walls on large diagrams. This matters most for the power users who generate the most word-of-mouth.

**Code-first workflow.** HRF specs are plain text that lives in Git alongside source code. Diagrams can be diffed, reviewed in PRs, and generated in CI. No other tool treats diagrams as code this natively.

**AI-native diagramming.** The LLM integration isn't a bolt-on feature — it's woven into the spec format from day one. Competitors will add "AI features" eventually, but light-figma's architecture was designed around it.

**3D architecture maps.** No competitor offers layered 3D visualization of system architecture. This is an instant brand differentiator — people remember the first time they see their microservices stacked in 3D.

**Iteration velocity.** 220+ features in 13 days. Even if a competitor starts copying, light-figma can out-iterate them with AI-assisted development loops.

---

## Key Risks

**Single developer.** The entire codebase lives in one person's head. If development stops, the project dies. Mitigation: open-source the code, write architectural docs, and cultivate contributors.

**No distribution.** Users currently need to build from source. Every day without a downloadable binary is a day of lost adoption. This is the most urgent risk.

**Limited test coverage.** Rapid iteration without tests accumulates hidden bugs. One broken save/load cycle could destroy user trust. The 64 HRF tests are a start, but the model and export layers need coverage.

**Figma ships AI diagramming.** Figma has 4M+ users and unlimited engineering resources. If they add AI-generated diagrams, light-figma loses its novelty advantage. Mitigation: lean into code-first workflow and native performance — things Figma can't easily replicate in a browser.

**Feature breadth vs. depth.** 220 features in 13 days means many of them are shallow. Users forgive missing features but not broken ones. At some point, the priority needs to shift from "ship new things" to "make existing things bulletproof."

---

## TAM

There are roughly 10M+ developers worldwide who regularly draw architecture diagrams, using tools like Miro, Lucidchart, Excalidraw, draw.io, and Mermaid. The wedge is the subset of engineers who find drag-and-drop tedious and would prefer to write a text spec — probably 1-2M developers today, growing as AI-generated content becomes normal.

At $12/mo Pro pricing with a 5% conversion rate on 100K free users, that's $720K ARR. At $8/seat Teams pricing with 1,000 teams of 10 seats, that's $960K ARR. The API/embed and AI revenue streams are additive. A realistic 3-year target is $2-5M ARR.
