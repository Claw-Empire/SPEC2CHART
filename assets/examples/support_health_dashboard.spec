# Support Health Dashboard

Visualize the current health of your support team:
volume, SLA compliance, CSAT, backlog, and escalation rates.
Update weekly to track trends.

## Config
flow = TB
spacing = 90

## Volume
- [vol_total] Total tickets this week {icon:🎫} {info}
  127 tickets opened · 118 closed · 9 open
  {sublabel:↑ 12% vs last week}
- [vol_p1] P1 tickets {p1} {icon:🔴}
  3 opened · 3 resolved · 0 open
  {sublabel:Target: <5/week}
- [vol_p2] P2 tickets {p2} {icon:🟠}
  18 opened · 16 resolved · 2 open
  {sublabel:Target: <25/week}
- [vol_p3] P3 tickets {p3} {icon:🟡}
  62 opened · 60 resolved · 2 open
- [vol_p4] P4 tickets {p4} {icon:🟢}
  44 opened · 39 resolved · 5 open

## SLA Compliance
- [sla_p1] P1 SLA: 4h response {done} {icon:✅} {glow}
  3/3 tickets within SLA  ·  100%
  {sublabel:Target: ≥99%}
- [sla_p2] P2 SLA: 8h response {done} {icon:✅}
  15/18 tickets within SLA  ·  83%
  {sublabel:Target: ≥95% ⚠️ below target}
- [sla_p3] P3 SLA: 24h response {wip} {icon:🔄}
  58/62 tickets within SLA  ·  94%
  {sublabel:Target: ≥90%}

## Customer Satisfaction
- [csat_score] CSAT Score {done} {icon:⭐} {glow}
  4.6 / 5.0  ·  89% response rate
  {sublabel:↑ 0.2 vs last week}
- [csat_positive] Positive feedback {ok} {icon:😊}
  "Fast response" · "Problem solved first contact"
- [csat_negative] Negative feedback {warning} {icon:😟}
  "Took too long on P2 ticket" · "Had to follow up twice"
  {sublabel:Action: review P2 SLA process}

## Backlog Health
- [backlog] Open backlog {wip} {icon:📋}
  9 tickets open  ·  2 over SLA
  {sublabel:Target: <15 open at end of week}
- [oldest] Oldest open ticket {p2} {icon:⏰}
  Ticket #4821 — 6 days open  ·  Needs escalation review
  {sublabel:Assign to: Senior Agent}
- [escl] Active escalations {escalated} {icon:🚨}
  2 tickets currently escalated to Tier 2
  {sublabel:Both P2 — review in daily standup}

## Team
- [team_l1] L1 Agents {ok} {icon:👥}
  5 agents  ·  avg handle time: 18 min
  {assigned:Team Lead: Sarah}
- [team_l2] L2 Specialists {info} {icon:🛠️}
  2 agents  ·  3 tickets in queue
  {assigned:On-call: Marcus}

## Flow
vol_total --> vol_p1
vol_total --> vol_p2
vol_total --> vol_p3
vol_total --> vol_p4
vol_p1 --> sla_p1
vol_p2 --> sla_p2
vol_p3 --> sla_p3
sla_p1 --> csat_score
sla_p2 --> csat_score
sla_p3 --> csat_score
csat_score --> csat_positive
csat_score --> csat_negative
csat_negative --> backlog: drives
backlog --> oldest: highest risk
backlog --> escl
escl --> team_l2: escalated to
vol_total --> team_l1: handled by
