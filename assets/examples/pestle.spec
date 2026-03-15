# PESTLE Analysis

Environmental scan of macro factors affecting a product, strategy, or decision.
Political, Economic, Social, Technological, Legal, Environmental.

## Config
flow = TB
spacing = 80

## Focus
- [focus] Launch in EU Market {hypothesis} {glow} {icon:🎯}
  The strategic decision or hypothesis being evaluated.

## Political
- [po1] GDPR compliance mandatory {evidence} {done} {icon:🏛️}
  Full GDPR readiness completed Q1 2026.
- [po2] Upcoming EU AI Act regulations {risk} {warning} {icon:🏛️}
  AI features may require conformity assessment.
- [po3] Trade barriers with UK post-Brexit {assumption} {icon:🏛️}

## Economic
- [ec1] EUR/USD exchange rate volatility {risk} {info} {icon:💶}
  ±15% swing affects pricing margins.
- [ec2] EU SaaS market growing 18% YoY {evidence} {done} {icon:📈}
  Source: Gartner 2025 report.
- [ec3] Recession risk in DE/FR markets {assumption} {icon:💶}

## Social
- [so1] High adoption of remote work tools {evidence} {done} {icon:👥}
  65% of EU knowledge workers hybrid/remote.
- [so2] Data sovereignty concerns in Germany {risk} {info} {icon:👥}
  Prefer local data centers.
- [so3] English acceptable as product language {assumption} {icon:👥}

## Technological
- [te1] Dominant cloud providers: AWS EU, Azure {evidence} {done} {icon:💻}
- [te2] AI assistant features expected by 2027 {hypothesis} {icon:💻}
- [te3] Legacy ERP integration required by 40% of prospects {evidence} {wip} {icon:💻}

## Legal
- [le1] GDPR data residency — must offer EU hosting {evidence} {done} {icon:⚖️}
- [le2] Digital Markets Act affects large platform integrations {risk} {warning} {icon:⚖️}
- [le3] VAT rules vary per country {evidence} {done} {icon:⚖️}

## Environmental
- [en1] Carbon reporting requirements for enterprise buyers {evidence} {wip} {icon:🌿}
- [en2] Green SaaS certification attractive to buyers {hypothesis} {icon:🌿}
- [en3] Data center energy use under scrutiny {risk} {info} {icon:🌿}

## Flow
po1 --> focus: enables
po2 --> focus: threatens
po3 --> focus: blocks
ec2 --> focus: supports
ec1 --> focus: threatens
so1 --> focus: supports
so2 --> focus: complicates
te1 --> focus: enables
te3 --> focus: complicates
le1 --> focus: requires
le2 --> focus: threatens
en1 --> focus: requires
en2 --> focus: opportunity
