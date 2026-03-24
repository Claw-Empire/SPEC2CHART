## Config
title: Runbook
flow = TB

## Nodes
- [trigger] Trigger: Alert / On-Call Page {critical}
- [assess] Assess Severity {diamond}
- [p1] P1 Critical {fill:#cc3333}
- [p2] P2 High {fill:#e8a838}
- [p3] P3 Low {fill:#4caf50}
- [mitigate] Apply Mitigation {rounded}
- [escalate] Escalate to Lead {person}
- [verify] Verify Service Restored {rounded}
- [postmortem] File Post-Mortem {document}
- [close] Close Incident {ok}

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
verify --> close
postmortem --> close
