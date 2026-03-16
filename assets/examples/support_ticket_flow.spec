# Support Ticket Flow

Track a customer support ticket from intake through resolution and follow-up.
Identify SLA risks and escalation triggers at each stage.

## Config
flow = LR
spacing = 80

## Intake
- [t1] Customer submits ticket {icon:📩} {wip}
  Channel: email / live chat / support portal
- [t2] Auto-acknowledge sent {icon:🤖} {done}
  SLA clock starts — P2 target: 4h first response

## Triage
- [triage] Triage agent reviews {icon:👀} {wip}
  Classify: Bug / Feature Request / How-To / Billing
- [pri_p1] P1 — Critical {icon:🔴} {p1}
  System down or data loss — escalate immediately
- [pri_p2] P2 — High {icon:🟠} {p2}
  Key feature broken — 4h first response
- [pri_p3] P3 — Medium {icon:🟡} {p3}
  Degraded experience — 24h first response
- [pri_p4] P4 — Low {icon:🟢} {p4}
  Question or cosmetic — 72h response

## First Response
- [resp] Agent responds {icon:💬} {wip}
  Acknowledge, gather info, set expectations
- [info_needed] More info needed? {diamond} {icon:❓}
- [waiting] Waiting on customer {icon:⏳}
  Auto-follow-up after 3 days of silence

## Resolution
- [diagnose] Diagnose root cause {icon:🔍} {wip}
- [escalate] Escalate to Tier 2 {icon:🚨} {escalated}
  Trigger: unresolved after 2h (P1) or 8h (P2)
- [resolve] Issue resolved {icon:✅} {done}
- [workaround] Workaround provided {icon:🛠️} {ok}
  Document in KB if no permanent fix yet

## Closure
- [csat] CSAT survey sent {icon:⭐} {todo}
  One-click rating — sent 30 min after close
- [kb] Update knowledge base {icon:📚} {todo}
  Write or update FAQ if a pattern is detected
- [closed] Ticket closed {icon:🎯} {done} {glow}

## Flow
t1 --> t2
t2 --> triage
triage --> pri_p1: critical
triage --> pri_p2: high
triage --> pri_p3: medium
triage --> pri_p4: low
pri_p1 --> escalate: immediately
pri_p2 --> resp
pri_p3 --> resp
pri_p4 --> resp
resp --> info_needed
info_needed --> waiting: yes
info_needed --> diagnose: no
waiting --> diagnose: info received
diagnose --> escalate: too complex
diagnose --> resolve
diagnose --> workaround: no fix yet
escalate --> resolve
workaround --> resolve
resolve --> csat
resolve --> kb: new pattern
csat --> closed
kb --> closed
