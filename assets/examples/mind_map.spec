# Mind Map: Product Strategy

A radial map of interconnected ideas. The central concept sits in the middle;
branches fan out to related themes, sub-themes, and supporting details.
Use H/Y/W keys to add hypothesis, assumption, or evidence nodes quickly.

## Config
flow = LR
spacing = 60

## Central
- [core] Product Strategy 2026 {icon:🎯} {glow}
  The central organizing idea for the team.

## Themes
- [t1] User Growth {hypothesis} {icon:📈}
- [t2] Revenue {hypothesis} {icon:💰}
- [t3] Platform {hypothesis} {icon:🏗}
- [t4] Team {hypothesis} {icon:👥}

## Growth
- [g1] Viral referral program {assumption} {icon:🔗}
- [g2] Enterprise sales motion {assumption} {wip} {icon:🤝}
- [g3] SEO content strategy {evidence} {done} {icon:📝}
- [g4] Product-led onboarding {assumption} {icon:⚡}

## Revenue
- [r1] Seat-based pricing tier {assumption} {icon:💳}
- [r2] API usage billing {hypothesis} {icon:⚙️}
- [r3] Enterprise annual contracts {evidence} {wip} {icon:📋}

## Platform
- [p1] Mobile app (iOS + Android) {hypothesis} {todo} {icon:📱}
- [p2] Slack integration {evidence} {done} {icon:🔌}
- [p3] Zapier connector {assumption} {wip} {icon:🔧}
- [p4] Public API v2 {hypothesis} {icon:🌐}

## Team
- [tm1] Grow eng team 40% {assumption} {icon:🧑‍💻}
- [tm2] Hire 2 PMs {todo} {icon:👔}
- [tm3] Design systems team {evidence} {done} {icon:🎨}

## Flow
core --> t1
core --> t2
core --> t3
core --> t4
t1 --> g1
t1 --> g2
t1 --> g3
t1 --> g4
t2 --> r1
t2 --> r2
t2 --> r3
t3 --> p1
t3 --> p2
t3 --> p3
t3 --> p4
t4 --> tm1
t4 --> tm2
t4 --> tm3
g3 --> t2: enables
p2 --> g4: supports
