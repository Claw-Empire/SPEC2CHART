# Value Proposition Canvas

Map your product to your customer's world. Left side = your product.
Right side = the customer. Fit is achieved when gains/pain-relievers
match the customer's jobs, gains, and pains.

## Config
flow = LR
spacing = 90

## Product
- [prod] Product / Service {server} {icon:📦} {glow}
  What you are building.

## Features
- [f1] Real-time sync across devices {assumption} {wip} {icon:⚡}
- [f2] One-click reporting {assumption} {done} {icon:📊}
- [f3] AI-powered suggestions {hypothesis} {todo} {icon:🤖}

## Pain Relievers
- [pr1] Eliminates manual copy-paste {evidence} {done} {icon:💊}
  Reduces average task time by 40%.
- [pr2] Single dashboard replaces 3 tools {evidence} {wip} {icon:💊}
- [pr3] Auto-reminders prevent missed deadlines {assumption} {icon:💊}

## Gain Creators
- [gc1] Saves 2 hours/week per user {hypothesis} {icon:🌟}
  Projected from pilot data.
- [gc2] Improves team visibility {evidence} {done} {icon:🌟}
- [gc3] Enables async collaboration {assumption} {icon:🌟}

## Customer
- [cust] Customer Segment {user} {icon:👤} {glow}
  Primary persona being served.

## Jobs to Be Done
- [j1] Coordinate team tasks without meetings {evidence} {done} {icon:🔧}
- [j2] Track progress at a glance {evidence} {done} {icon:🔧}
- [j3] Report status to stakeholders quickly {evidence} {wip} {icon:🔧}

## Pains
- [p1] Too many disconnected tools {evidence} {critical} {icon:😖}
  7/10 users report tool-sprawl as top pain.
- [p2] Status updates take too long {evidence} {icon:😖}
- [p3] Hard to onboard new team members {evidence} {icon:😖}

## Gains
- [g1] Team stays in sync without daily standups {hypothesis} {icon:✨}
- [g2] Work is recognized by leadership {hypothesis} {icon:✨}
- [g3] Less stress at end of sprint {assumption} {icon:✨}

## Flow
prod --> f1
prod --> f2
prod --> f3
f1 --> pr1: enables
f2 --> gc2: creates
f3 --> gc1: drives
pr1 --> p1: relieves
pr2 --> p1: relieves
pr3 --> p2: relieves
gc1 --> g1: creates
gc2 --> g2: creates
gc3 --> g1: supports
cust --> j1
cust --> j2
cust --> j3
cust --> p1
cust --> p2
cust --> p3
cust --> g1
cust --> g2
cust --> g3
pr1 --> j1: fits
pr2 --> j2: fits
gc1 --> g1: fits
pr1 --> p1: relieves
