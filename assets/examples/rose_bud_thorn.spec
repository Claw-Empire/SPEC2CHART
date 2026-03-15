# Rose, Bud, Thorn Retrospective

A lightweight, visual retrospective format. Roses = what went well.
Buds = opportunities and ideas. Thorns = problems and frustrations.
Use after a sprint, launch, or project phase.

## Config
flow = TB
spacing = 80

## Roses
- [r1] Launched mobile MVP on schedule 🌹 {evidence} {done} {icon:✅}
  First time in 3 sprints we hit our date.
- [r2] Customer satisfaction up to 4.2/5 🌹 {evidence} {done} {icon:📈}
  Up from 3.8 last quarter — driven by UX polish.
- [r3] Eng team morale high — 2 key hires joined 🌹 {evidence} {done} {icon:👥}
- [r4] Zero Sev-1 incidents during launch week 🌹 {evidence} {done} {icon:🛡️}

## Buds
- [b1] AI auto-tagging could save 30 min/user/week 🌱 {hypothesis} {icon:💡}
  Spike showed it's feasible — not yet scheduled.
- [b2] Partner integration with Notion — 40% of users interested 🌱 {hypothesis} {icon:🔗}
- [b3] Design system v2 — reduce design/eng rework 🌱 {assumption} {wip} {icon:🎨}
- [b4] Self-serve onboarding could unlock SMB segment 🌱 {hypothesis} {icon:🚀}

## Thorns
- [t1] Staging deploy broke prod on Day 2 🌵 {evidence} {critical} {icon:🔴}
  CI/CD pipeline needs isolation improvement.
- [t2] Documentation 3 sprints behind code changes 🌵 {evidence} {warning} {icon:📝}
- [t3] Eng-PM misalignment on "done" definition caused 3 re-dos 🌵 {evidence} {icon:⚡}
- [t4] No design review for 4 features — QA found regressions 🌵 {evidence} {warning} {icon:🔎}

## Actions
- [a1] Add staging → prod gate to CI — owner: DevOps {evidence} {wip} {icon:🔧}
- [a2] Document as you go sprint policy {assumption} {icon:📋}
- [a3] Definition of Done checklist (eng + PM sign-off) {assumption} {icon:✍️}
- [a4] Mandatory design review for all customer-facing features {assumption} {icon:🎨}

## Flow
r1 --> b4: unlocks
r3 --> b3: enables
t1 --> a1: drives
t2 --> a2: drives
t3 --> a3: drives
t4 --> a4: drives
b1 --> r2: potential to improve
