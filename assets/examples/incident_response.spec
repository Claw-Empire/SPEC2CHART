# Incident Response Runbook

Structured 5-phase playbook for detecting, triaging, resolving, and learning from production incidents.

## Config
flow = TB
spacing = 90

## Phase 1: Detect
- [alert] Alert fires {icon:🚨} {critical}
  PagerDuty / Grafana / customer report
- [oncall] On-call engineer paged {icon:📟} {critical}
  Acknowledge within 5 min
- [severity] Assess severity {diamond} {icon:⚖️}
- [sev1] SEV-1 — Total outage {sev1} {glow} {icon:🔴}
  All users affected — war room in 10 min
- [sev2] SEV-2 — Major degradation {sev2} {icon:🟠}
  >20% users affected — response in 30 min
- [sev3] SEV-3 — Minor issue {sev3} {icon:🟡}
  <20% affected — best-effort same day

## Phase 2: Communicate
- [comms] Open incident channel {icon:💬} {wip}
  #incident-YYYY-MM-DD in Slack
- [status] Post status page update {icon:📢} {wip}
  "We are investigating…" — within 15 min of SEV-1
- [stakeholders] Notify stakeholders {icon:📧} {todo}
  CTO + Customer Success for SEV-1

## Phase 3: Investigate
- [runbook] Check runbook for known pattern {icon:📖} {wip}
- [metrics] Pull metrics / logs / traces {icon:📊} {wip}
  Datadog / Grafana / CloudWatch — look back 2h
- [hypothesis] Form hypothesis {hypothesis} {icon:💡}
- [rollback] Rollback viable? {diamond} {icon:🔄}

## Phase 4: Resolve
- [rollback_do] Execute rollback {icon:⏪} {wip}
- [hotfix] Deploy hotfix {icon:🔧} {wip}
- [mitigate] Apply mitigation {icon:🛡️} {ok}
  Feature flag / rate limit / cache bypass
- [resolved] Incident resolved {icon:✅} {done}
  Verify all metrics return to baseline

## Phase 5: Learn
- [postmortem] Write postmortem {icon:📝} {todo}
  Due within 5 business days (SEV-1/SEV-2)
- [action_items] Assign action items {icon:✅} {todo}
  Each with owner + due date
- [close] Incident closed {icon:🎯} {done} {glow}

## Flow
alert --> oncall
oncall --> severity
severity --> sev1: total outage
severity --> sev2: major degradation
severity --> sev3: minor
sev1 --> comms
sev2 --> comms
sev3 --> comms
comms --> status
comms --> stakeholders: SEV-1
status --> runbook
runbook --> metrics
metrics --> hypothesis
hypothesis --> rollback: recent deploy?
rollback --> rollback_do: yes
rollback --> hotfix: no
rollback_do --> resolved
hotfix --> mitigate: interim relief
mitigate --> resolved
resolved --> postmortem
postmortem --> action_items
action_items --> close
