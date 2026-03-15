# Support Escalation Matrix

Define who handles what at each tier and the conditions that trigger escalation.
Use this to set expectations, reduce response time, and prevent unnecessary escalations.

## Config
flow = LR
spacing = 100

## Tier 1: Self-Service
- [tier1_faq] FAQ / Knowledge Base {icon:📚} {done}
  Password reset, billing FAQ, setup guides
- [tier1_bot] Chatbot {icon:🤖} {done}
  Handles ~40% of inbound tickets automatically
- [tier1_l1] L1 Agent {icon:👤} {wip}
  How-to questions, account settings, billing
  SLA: 1h response · 4h resolution

## Tier 2: Specialist
- [tier2] L2 — Technical Specialist {icon:🛠️} {info}
  Bugs, configuration, integrations, API issues
  SLA: 4h response · 24h resolution

## Tier 3: Engineering
- [tier3] L3 — Engineering {icon:💻} {warning}
  Code-level bugs, data issues, security incidents
  SLA: 8h response · 72h resolution

## Tier 4: Leadership
- [tier4] L4 — Leadership / Exec {icon:🎯} {critical}
  Enterprise SLA breach, legal, critical outage

## Escalation Triggers
- [e_sla] SLA breach approaching {warning} {icon:⏰}
  Unresolved within 80% of SLA window
- [e_customer] Customer requests escalation {info} {icon:📢}
- [e_revenue] Revenue impact detected {critical} {icon:💰}
- [e_security] Security or data concern {critical} {icon:🔐}

## Flow
tier1_faq --> tier1_l1: needs agent
tier1_bot --> tier1_l1: bot fails
tier1_l1 --> tier2: technical issue
tier2 --> tier3: code-level bug
tier3 --> tier4: SLA breach or exec escalation
e_sla --> tier2: from L1
e_sla --> tier3: from L2
e_customer --> tier2: from L1
e_customer --> tier3: from L2
e_revenue --> tier4: any tier
e_security --> tier3: investigate
e_security --> tier4: confirmed breach
