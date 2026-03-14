# OKR Tree

Objectives and Key Results — connect company goals to measurable outcomes.
Cascade from company OKRs to team OKRs to key initiatives.

## Config
flow = TB
spacing = 90

## Company Objective
- [co] Become the #1 developer tool for SaaS teams {icon:🎯} {glow}
  Company-level objective for H1 2026.

## Product Key Results
- [kr1] 10,000 active installs by end of Q2 {icon:📈} {wip}
- [kr2] NPS score ≥ 50 by end of Q2 {icon:⭐} {todo}
- [kr3] Time-to-first-diagram < 90 seconds {icon:⚡} {wip}

## Engineering Key Results
- [kr4] P95 render latency < 16ms {icon:🔧} {wip}
- [kr5] Zero critical bugs for 4 consecutive weeks {icon:✅} {done}
- [kr6] 80% test coverage on core modules {icon:🧪} {todo}

## Growth Key Results
- [kr7] 500 organic signups/week from SEO {icon:🔍} {todo}
- [kr8] 3 high-profile partnerships signed {icon:🤝} {todo}

## Initiatives
- [i1] Redesign onboarding flow {wip} {icon:✨}
  Linked to kr3 — reduce time-to-first-diagram.
- [i2] Launch public changelog {todo} {icon:📣}
  Linked to kr1 — drive awareness.
- [i3] Performance profiling sprint {wip} {icon:⚙️}
  Linked to kr4 — fix top 5 render bottlenecks.
- [i4] SEO content blitz (20 articles) {todo} {icon:📝}
  Linked to kr7.

## Flow
co --> kr1
co --> kr2
co --> kr3
co --> kr4
co --> kr5
co --> kr6
co --> kr7
co --> kr8
kr3 --> i1: drives
kr1 --> i2: supported by
kr4 --> i3: drives
kr7 --> i4: drives
