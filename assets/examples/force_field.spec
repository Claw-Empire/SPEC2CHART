# Force Field Analysis

Map the forces driving change for and against a decision.
Weigh driving forces vs. restraining forces to evaluate feasibility.

## Config
flow = LR
spacing = 80

## Decision
- [decision] Launch Mobile App {diamond} {fill:#6c91c2} {glow}
  The central decision being evaluated.

## Driving Forces
- [d1] Growing mobile user base {icon:📈} {done}
  65% of sessions are already on mobile devices.
- [d2] Competitor parity required {icon:⚔️} {wip}
  Top 3 competitors all have native apps.
- [d3] Higher retention on mobile {icon:🔒} {done}
- [d4] Investor commitment expected {icon:💰}
- [d5] Team has iOS/Android expertise {icon:🧑‍💻} {done}

## Restraining Forces
- [r1] Engineering bandwidth limited {icon:⏳} {blocked}
  Q2 roadmap already 80% committed.
- [r2] High development cost {icon:💸} {blocked}
  Estimated $280k for MVP.
- [r3] App store review delays {icon:⚠️}
- [r4] Web experience still incomplete {icon:🌐}

## Flow
d1 --> decision: +strong
d2 --> decision: +medium
d3 --> decision: +medium
d4 --> decision: +weak
d5 --> decision: +medium
r1 --> decision: −strong
r2 --> decision: −strong
r3 --> decision: −weak
r4 --> decision: −medium
