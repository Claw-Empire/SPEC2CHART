## Config
title: On-Call Tree
flow = TB

## Nodes
- [alert] Page Received {rounded} {fill:#f38ba8} {critical}
  PagerDuty high-sev.
- [primary] Primary On-Call {person} {fill:#89b4fa} {wip}
  First responder, 5min SLA.
- [secondary] Secondary On-Call {person} {fill:#cba6f7} {todo}
  Backup, 15min SLA.
- [manager] Engineering Manager {person} {fill:#f9e2af}
  Escalation point.
- [vp] VP Engineering {person} {fill:#f2cdcd} {critical}
  SEV1 only.
- [sre] SRE Channel {rounded} {fill:#a6e3a1} {info}
  #incident broadcast.
- [runbook2] Runbook {document} {fill:#b4befe}
  Playbook steps.

## Flow
alert --> primary: pages
primary --> secondary: unresponsive 5m
secondary --> manager: unresponsive 15m
manager --> vp: SEV1 escalate
primary --> sre: broadcast
primary --> runbook2: follow
secondary --> sre: broadcast
