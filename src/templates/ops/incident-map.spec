## Config
title: Incident Map
flow = TB

## Nodes
- [alert] Alert Triggered {rounded} {fill:#f38ba8} {critical}
  PagerDuty high-sev page.
- [oncall] On-Call Eng {person} {fill:#89b4fa}
  Primary responder.
- [slack] Slack Notify {rounded} {fill:#a6e3a1} {ok}
  #incident channel ping.
- [triage] Triage {rounded} {fill:#f9e2af} {wip}
  Root cause hunt.
- [runbook] Runbook {document} {fill:#cba6f7}
  Step-by-step playbook.
- [mitigate] Mitigation {rounded} {fill:#a6e3a1} {ok}
  Apply fix + verify.
- [comms] Status Comms {rounded} {fill:#f9e2af} {info}
  Customer updates.
- [postmortem] Post-Mortem {document} {fill:#cba6f7} {todo}
  Blameless write-up.
- [closed] Incident Closed {rounded} {fill:#a6e3a1} {done}
  Service restored, ticket resolved.

## Flow
alert --> oncall: pages
oncall --> slack: notifies team
oncall --> triage: investigates
triage --> runbook: follows
runbook --> mitigate: applies fix
mitigate --> comms: updates
mitigate --> postmortem: creates
comms --> closed: all clear
postmortem --> closed: filed
