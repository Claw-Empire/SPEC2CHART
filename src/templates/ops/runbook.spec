## Config
title: Runbook
flow = TB

## Nodes
- [trigger] Alert Page {rounded} {fill:#f38ba8} {critical}
  Initial PagerDuty ping.
- [assess] Assess Severity {diamond} {fill:#f9e2af} {wip}
  SEV1 / SEV2 / SEV3?
- [p1] P1 Critical {rounded} {fill:#f38ba8} {critical}
  Customer-facing outage.
- [p2] P2 High {rounded} {fill:#f9e2af} {warning}
  Degraded service.
- [p3] P3 Low {rounded} {fill:#a6e3a1} {ok}
  Minor issue.
- [escalate] Escalate to Lead {person} {fill:#cba6f7}
  Wake the EM.
- [mitigate] Apply Mitigation {rounded} {fill:#89b4fa} {wip}
  Rollback / hotfix / failover.
- [verify] Verify Restored {diamond} {fill:#f9e2af}
  Dashboards green?
- [postmortem] File Post-Mortem {document} {fill:#cba6f7} {todo}
  Blameless write-up.
- [close] Close Incident {rounded} {fill:#a6e3a1} {done}
  Resolved + archived.

## Flow
trigger --> assess
assess --> p1: SEV1
assess --> p2: SEV2
assess --> p3: SEV3
p1 --> escalate
p2 --> mitigate
p3 --> mitigate
escalate --> mitigate: after brief
mitigate --> verify
verify --> postmortem: if P1/P2
verify --> close: if P3
postmortem --> close
