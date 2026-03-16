# Incident Postmortem

Structured blameless retrospective for a production incident.
Captures timeline, root cause, contributing factors, and action items.
Due within 5 business days of a SEV-1 or SEV-2 incident.

## Config
flow = TB
spacing = 80

## Incident Summary
- [incident] Incident: [Incident title — replace me] {sev1} {glow}
  Duration: X hours Y minutes  ·  Users affected: ~N
  {sublabel:SEV-1 · [date]}
- [impact] Business Impact {warning} {icon:💥}
  Revenue: $X  ·  Uptime SLA breached: Y%  ·  Complaints: Z tickets

## Timeline
- [t_detect] Detected {icon:🚨} {wip}
  How was the incident detected? (alert / customer report / engineer noticed)
  {assigned:On-call engineer}
- [t_ack] Acknowledged {icon:📟} {wip}
  Time to acknowledge after first alert
- [t_investigate] Investigation started {icon:🔍} {wip}
  War room opened · First hypothesis formed
- [t_mitigate] Mitigation applied {icon:🛡️} {wip}
  First action taken to reduce blast radius
- [t_resolve] Incident resolved {done} {icon:✅}
  Service fully restored · All metrics nominal

## Root Cause
- [root] Root Cause {hypothesis} {icon:💡} {glow}
  The single technical cause of the incident
- [contributing] Contributing Factors {icon:🔗} {warning}
  Other conditions that made the incident possible or worse
- [trigger] Trigger Event {cause} {icon:⚡}
  The specific change or event that initiated the failure

## What Went Well
- [well_1] Response was fast {ok} {icon:✅}
  On-call acknowledged and formed war room quickly
- [well_2] Rollback procedure worked {ok} {icon:✅}
  Playbook was up to date and effective

## What Could Be Improved
- [improve_1] Alert was too noisy {warning} {icon:⚠️}
  Too many false-positive pages before the real incident
- [improve_2] Runbook was outdated {warning} {icon:⚠️}
  Documentation for this service hadn't been updated in 6 months

## Action Items
- [a1] Add synthetic monitoring for [affected endpoint] {todo} {p1}
  {assigned:Alice} {due:2026-03-30}
- [a2] Update runbook for [service name] {todo} {p2}
  {assigned:Bob} {due:2026-04-05}
- [a3] Tune alert thresholds to reduce noise {todo} {p2}
  {assigned:Charlie} {due:2026-04-12}
- [a4] Schedule chaos engineering exercise {todo} {p3}
  Validate that rollback works before next deploy
  {assigned:Team} {due:2026-04-30}

## Flow
incident --> impact
incident --> t_detect: timeline
t_detect --> t_ack
t_ack --> t_investigate
t_investigate --> t_mitigate
t_mitigate --> t_resolve
t_investigate --> root: found
root --> contributing: context
trigger --> root: initiated
root --> a1: fix
contributing --> a2: fix
improve_1 --> a3: action
improve_2 --> a2: fix
well_1 --> t_ack: evidence
well_2 --> t_mitigate: evidence
