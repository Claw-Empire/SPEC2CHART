# Support SLA Matrix

Response and resolution time targets by priority tier.
Reference for on-call engineers, support managers, and escalation coordinators.

## Config
flow = LR
spacing = 65

## Priority Tiers
- [p1] P1 — Critical {p1} {glow}
  Complete outage · Data loss · Revenue impact
  {sublabel:SEV-1 · All customers}
- [p2] P2 — High {p2}
  Major feature unavailable · Some customers affected
  {sublabel:SEV-2 · Partial impact}
- [p3] P3 — Medium {p3}
  Degraded performance · Workaround available
  {sublabel:SEV-3 · Minor impact}
- [p4] P4 — Low {p4}
  Cosmetic issue or enhancement request
  {sublabel:SEV-4 · No impact}

## Response Targets
- [resp1] First Response: 15 min {p1}
  {assigned:L1 On-call}
- [resp2] First Response: 1 hour {p2}
  {assigned:L1 Support}
- [resp3] First Response: 4 hours {p3}
  {assigned:L1 Support}
- [resp4] First Response: 1 business day {p4}
  {assigned:L1 Support}

## Resolution Targets
- [res1] Resolve: 4 hours {p1} {glow}
  Update customer every 30 min
  Escalate L2 at 1 hr · L3 at 2 hr
- [res2] Resolve: 24 hours {p2}
  Update customer every 2 hr
  Escalate L2 at 8 hr
- [res3] Resolve: 72 hours {p3}
  Update customer every 24 hr
- [res4] Resolve: 10 business days {p4}
  Bundle with sprint planning

## Escalation Path
- [esc_l2] Escalate → L2 {warning} {icon:⬆️}
  P1: at 1 hr  ·  P2: at 8 hr
  {assigned:L2 On-call}
- [esc_l3] Escalate → L3 {warning} {icon:🚨} {glow}
  P1 only · at 2 hr
  {assigned:L3 On-call}
- [exec_bridge] Executive Bridge {p1} {glow} {icon:👔}
  P1 only · open at 1 hr mark
  {assigned:VP Engineering}
- [comms] Customer Comms {ok} {icon:📧}
  P1/P2: proactive updates  ·  P3: on request

## Breach Response
- [breach_p1] P1 SLA Breach {p1} {glow} {icon:💥}
  Auto-page VP Eng + VP Support · postmortem required
  {assigned:VP Engineering}
- [breach_p2] P2 SLA Breach {p2} {icon:⚠️}
  Notify Support Manager · root cause analysis
  {assigned:Support Manager}
- [postmortem] Post-Breach Review {todo} {icon:📋}
  Root cause within 24 hr · action items in backlog
  {due:+1 business day}

## Flow
p1 --> resp1
p2 --> resp2
p3 --> resp3
p4 --> resp4
resp1 --> res1
resp2 --> res2
resp3 --> res3
resp4 --> res4
res1 --> esc_l2: at 1hr
esc_l2 --> esc_l3: P1 at 2hr
esc_l3 --> exec_bridge: P1 only
res1 --> comms: every 30min
res2 --> comms: every 2hr
res1 --> breach_p1: SLA miss
res2 --> breach_p2: SLA miss
breach_p1 --> postmortem
breach_p2 --> postmortem
