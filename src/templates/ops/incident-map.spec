## Config
title: Incident Map
flow = LR

## Nodes
- [alert] Alert Triggered {critical} {fill:#cc3333}
- [oncall] On-Call Eng {person}
- [slack] Slack Notify {rounded}
- [runbook] Runbook {document}
- [mitigate] Mitigation {rounded}
- [postmortem] Post-Mortem {document}

## Flow
alert --> oncall: pages
oncall --> slack: notifies team
oncall --> runbook: follows
runbook --> mitigate: applies fix
mitigate --> postmortem: creates
