# Decision Record: Architecture Decision Log

A structured record of architectural and product decisions — context, options considered,
outcome, and consequences. Based on the ADR (Architectural Decision Record) pattern.

## Config
flow = TB
spacing = 80

## Decisions
- [d1] Adopt event-sourcing for order system {hypothesis} {done} {icon:✅}
  Decided 2026-02-15. Owner: Backend Guild.
- [d2] Use PostgreSQL as primary database {hypothesis} {done} {icon:🗄️}
  Decided 2026-01-10. Overturned a 2024 MongoDB decision.
- [d3] Ship mobile via React Native (not native) {hypothesis} {wip} {icon:📱}
  In review — evaluating Flutter vs RN. Decision expected 2026-04-01.
- [d4] Monorepo vs multi-repo {hypothesis} {todo} {icon:📦}
  Blocked on tooling evaluation.

## Context
- [c1] Order volume now 10k/day — audit trail required by compliance {evidence} {icon:📋}
- [c2] Mobile sessions at 65% — native performance table stakes {evidence} {icon:📊}
- [c3] Four legacy databases — inconsistent query patterns {evidence} {icon:⚠️} {critical}
- [c4] Monorepo candidate: Nx workspace already in use for frontend {evidence} {icon:🔧}

## Options Considered
- [o1] Event-sourcing (CQRS) {assumption} {icon:💡}
  + Full audit trail. + Replay capability. − Higher complexity.
- [o2] Append-only log with triggers {assumption} {icon:💡}
  + Simpler. − Limited replay. − Still complex triggers.
- [o3] Stick with mutable DB + audit table {assumption} {icon:💡} {dim}
  Rejected — no replay support.
- [o4] React Native {assumption} {icon:📱}
  + 80% code share. + Team already knows React.
- [o5] Flutter {assumption} {icon:🎯}
  + True native feel. − New stack for team.

## Consequences
- [con1] Need CQRS infrastructure — 2 weeks setup {risk} {warning} {icon:⏳}
- [con2] Event schema migrations need careful planning {risk} {info} {icon:🗺}
- [con3] Team needs event-sourcing training {risk} {info} {icon:📚}
- [con4] React Native bundle size ~12MB concern {risk} {info} {icon:📦}

## Flow
c1 --> d1: drove
c3 --> d2: drove
c2 --> d3: drove
c4 --> d4: considering
d1 --> o1: chose
d1 --> o3: rejected
o1 --> con1: creates
o1 --> con2: creates
o1 --> con3: creates
d3 --> o4: leading
d3 --> o5: also considering
o4 --> con4: risk
