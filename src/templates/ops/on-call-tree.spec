## Config
title: On-Call Tree
flow = TB

## Nodes
- [primary] Primary On-Call {person} {critical}
- [secondary] Secondary On-Call {person} {warning}
- [manager] Engineering Manager {person}
- [vp] VP Engineering {person}
- [sre] SRE Team Channel {rounded}
- [runbook2] Runbook / Playbook {document}

## Flow
primary --> secondary: escalate if unresponsive
secondary --> manager: escalate if unresponsive
manager --> vp: escalate if SEV1
primary --> sre: page channel
primary --> runbook2: follow steps
