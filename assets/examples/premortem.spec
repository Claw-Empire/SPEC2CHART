# Project Premortem

Imagine the project has already failed — one year from now. Work backward to
identify the most likely causes. Far more effective than risk-listing because
it removes optimism bias and surfaces real concerns.

## Config
flow = TB
spacing = 80

## Scenario
- [scenario] Project Launched — and FAILED {hypothesis} {critical} {icon:💀} {glow}
  Fictional future: the product launched but did not meet goals. WHY?

## Product Failures
- [pf1] Core feature shipped too late — competitors won the window {cause} {critical} {icon:⚠️}
- [pf2] UX too complex — users churned in week 1 {cause} {critical} {icon:😤}
- [pf3] Performance issues — mobile app too slow {cause} {icon:🐢}
- [pf4] Wrong persona — built for ops, bought by finance {cause} {icon:🎯}

## Team Failures
- [tf1] Key engineer left 3 months in — velocity collapsed {cause} {critical} {icon:👤}
- [tf2] Scope kept expanding — never shipped v1 {cause} {icon:↗️}
- [tf3] No clear decision-maker — design-by-committee {cause} {icon:🗳️}
- [tf4] PM and eng misaligned on priorities for 4 sprints {cause} {icon:⚡}

## Market Failures
- [mf1] Competitor shipped equivalent feature free {cause} {critical} {icon:🏁}
- [mf2] Economic downturn — enterprise froze all new SaaS {cause} {icon:📉}
- [mf3] Regulatory change blocked our data model {cause} {icon:⚖️}

## Customer Failures
- [cf1] Beta users did not convert — value unclear {cause} {icon:👥}
- [cf2] Wrong champions — sponsors moved teams {cause} {icon:🚶}
- [cf3] NPS 2/10 — product felt unfinished at launch {cause} {icon:📊}

## Prevention
- [prev1] Monthly milestones with demo-or-die gates {evidence} {done} {icon:🛡️}
  Mitigation for tf2 (scope creep).
- [prev2] Competitor monitoring weekly {evidence} {wip} {icon:🔍}
  Mitigation for mf1.
- [prev3] Full-time PM from kickoff {assumption} {icon:👔}
  Mitigation for tf3/tf4.
- [prev4] User retention metric as launch criterion {assumption} {icon:📈}
  Mitigation for pf2/cf3.

## Flow
scenario --> pf1
scenario --> pf2
scenario --> pf3
scenario --> pf4
scenario --> tf1
scenario --> tf2
scenario --> tf3
scenario --> tf4
scenario --> mf1
scenario --> mf2
scenario --> mf3
scenario --> cf1
scenario --> cf2
scenario --> cf3
pf2 --> prev4: mitigated by
tf2 --> prev1: mitigated by
mf1 --> prev2: mitigated by
tf3 --> prev3: mitigated by
