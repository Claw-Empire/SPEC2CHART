# light-figma Business Roadmap

Go-to-market strategy, revenue model, and growth plan.

## Config
flow = TB
auto-tier-color = true
spacing = 130

## Layer 0: Assets Today
- [product] 19k-Line Native App {rounded_rect} {done} {glow} {icon:🚀}
  Rust/egui desktop app — fast, offline-first, 220+ features shipped in 13 days.
- [hrf] HRF Spec Language {rounded_rect} {done} {icon:📄}
  Text-to-diagram format with 64 tests. Diagram-as-code primitive.
- [llmeng] LLM Integration {rounded_rect} {done} {icon:🤖}
  Prose → diagram pipeline via llm.rs. Unique differentiator.
- [threed] 3D Architecture View {rounded_rect} {done} {icon:🧊}
  No competitor has layered 3D. Visual moat.
- [velocity] Dev Velocity {rounded_rect} {done} {icon:⚡}
  Solo developer shipping 15+ features/day. Extreme iteration speed.

## Layer 1: Go-to-Market
- [oss] Open-Source Launch {hexagon} {wip} {icon:🌟}
  GitHub release with .app binary. README, demo GIF, HN/Reddit launch.
- [devrel] Developer Content {hexagon} {todo} {icon:📢}
  Blog posts: "diagram your codebase in 30s", "Figma for engineers".
- [integrations] Tool Integrations {hexagon} {todo} {icon:🔧}
  VS Code extension, CLI pipe (cat arch.spec | light-figma), CI badge.
- [community] Discord / GitHub Discussions {hexagon} {todo} {icon:💬}
  Template sharing, spec gallery, feature requests.

## Layer 2: Revenue Streams
- [freemium] Freemium Desktop {diamond} {todo} {icon:🆓}
  Free: local files, all shapes, export. Unlimited personal use.
- [pro] Pro Plan ($12/mo) {diamond} {todo} {glow} {icon:💳}
  Cloud sync, team sharing, real-time collab, priority support.
- [teams] Teams Plan ($8/seat/mo) {diamond} {todo} {icon:👥}
  SSO, shared template library, admin controls, audit log.
- [api] API / Embed ($0.01/render) {diamond} {todo} {icon:🔌}
  Embeddable widget for docs, Notion, Confluence. Usage-based pricing.
- [llmpay] AI Diagram Generation {diamond} {todo} {icon:🧠}
  Pay-per-diagram: "describe your system, get a diagram". Token pass-through + margin.

## Layer 3: Growth Flywheel
- [viral] Spec File Sharing {star} {todo} {icon:🔄}
  Every .spec file is a growth vector — recipients need light-figma to view/edit.
- [seo] Embed SEO {star} {todo} {icon:📈}
  Embedded diagrams link back. "Made with light-figma" watermark on free tier.
- [templates] Template Marketplace {star} {todo} {icon:🏪}
  Community-contributed templates. Revenue share with creators (70/30).
- [cicd] CI/CD Pipeline Tool {star} {todo} {icon:🏗️}
  Auto-generate architecture docs on every push. Stickiness through automation.
- [edu] Education / Bootcamps {star} {todo} {icon:🎓}
  Free for students. Partnerships with coding bootcamps, CS programs.

## Layer 4: Competitive Moat
- [native] Native Performance {parallelogram} {done} {icon:🏎️}
  Rust = instant load, 60fps on 1000+ nodes. Figma/Excalidraw can't match.
- [codefirst] Code-First Workflow {parallelogram} {todo} {glow} {icon:💻}
  HRF specs live in git alongside code. Diagrams that never go stale.
- [aimoat] AI-Native Diagramming {parallelogram} {todo} {glow} {icon:🤖}
  LLM reads your repo, generates diagrams. No one else does this natively.
- [threedmoat] 3D System Maps {parallelogram} {done} {icon:🗺️}
  Unique visual — layered architecture in 3D. Instant brand recognition.
- [ecosystem] Plugin Ecosystem {parallelogram} {todo} {icon:🌐}
  Third-party shapes, exporters, integrations. Lock-in through ecosystem.

## Flow
product --> oss: ship binary
hrf --> devrel: content fuel
llmeng --> devrel: demo hook
threed --> devrel: visual hook
velocity --> oss: fast iteration

oss --> freemium: user funnel
devrel --> community: audience
integrations --> cicd: automation
community --> templates: contributions

freemium --> pro: upgrade path
freemium --> viral: .spec sharing
pro --> teams: expand seat
api --> seo: backlinks
llmpay --> aimoat: deepen

viral --> edu: reach students
seo --> community: organic growth
templates --> ecosystem: flywheel
cicd --> codefirst: stickiness
edu --> community: future devs

native --> pro: perf sells
codefirst --> teams: enterprise value
aimoat --> llmpay: monetize
threedmoat --> api: embed demand
ecosystem --> templates: supply

## Notes
- TAM: 10M+ developers who draw architecture diagrams (Miro, Lucidchart, Excalidraw) {fill:#2d3a5a}
- Wedge: "diagram-as-code" for engineers who hate drag-and-drop {fill:#2d5a3d}
- Risk: Figma ships AI diagramming; mitigate with code-first + native speed {fill:#5a2d2d}
- Unit economics: $0 COGS on desktop, LLM cost only on AI features {fill:#3a2d5a}

## Summary
Business model: open-core with freemium desktop → Pro/Teams cloud upsell + API/embed usage revenue. Growth via .spec file virality, CI/CD integration stickiness, and AI-generated diagrams. Moat is native perf + code-first workflow + 3D views that no competitor offers.
