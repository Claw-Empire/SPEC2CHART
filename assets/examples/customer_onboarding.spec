# Customer Onboarding Journey

Map the structured onboarding process for new customers — from sign-up to first value.
Use this to identify handoff points, time-to-value gaps, and drop-off risks.

## Config
flow = LR
spacing = 80

## Sign-Up
- [signup] Customer signs up {icon:✍️} {done}
  Web / SSO / invite link
- [welcome_email] Welcome email sent {icon:📧} {done}
  Subject: "You're in — here's how to get started"
  CTA: Open app + link to quickstart guide
- [verify] Email verified {icon:✅} {done}

## Setup
- [profile] Complete profile {icon:👤} {wip}
  Name, role, company size — used for personalization
- [workspace] Create first workspace {icon:🏗️} {wip}
  Workspace name + optional team invite
- [import] Import existing data? {diamond} {icon:📂}
- [import_yes] Import from CSV / Notion {icon:📥} {wip}
- [template] Choose a starter template {icon:🎨} {wip}
  Suggested: Customer Journey · 5 Whys · OKR Tree

## Activation
- [first_diagram] Create first diagram {icon:✏️} {wip}
  Milestone: user creates ≥1 node + ≥1 edge
- [share] Share with teammate {icon:👥} {todo}
  Milestone: first share or export
- [aha] Aha! moment reached {ok} {icon:🎉} {glow}
  Defined as: diagram with ≥5 nodes + shared with ≥1 teammate

## Nurture
- [day3] Day 3 email: "Try 3D view" {icon:📬} {todo}
- [day7] Day 7 email: "Templates for your use case" {icon:📬} {todo}
- [day14] Day 14 check-in: CSM outreach for paid plans {icon:📞} {todo}

## Health Check
- [health_green] Healthy — active weekly {ok} {icon:🟢} {done}
  ≥3 diagrams created, ≥1 share/export
- [health_yellow] At risk — low activity {warning} {icon:🟡}
  Logged in but no diagram created after day 7
- [health_red] Churning {critical} {icon:🔴}
  No login after day 14 — trigger win-back sequence

## Flow
signup --> welcome_email
welcome_email --> verify
verify --> profile
profile --> workspace
workspace --> import
import --> import_yes: yes
import --> template: no
import_yes --> first_diagram
template --> first_diagram
first_diagram --> share
share --> aha
aha --> day3
day3 --> day7
day7 --> day14
first_diagram --> health_green: active
first_diagram --> health_yellow: stalled
health_yellow --> day7: re-engage
verify --> health_red: no login 14d
health_red --> day14: win-back
