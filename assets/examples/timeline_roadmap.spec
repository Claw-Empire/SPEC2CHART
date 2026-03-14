# Product Roadmap 2026

A quarterly roadmap with swim-lanes by team.

## Config
timeline = true
timeline-dir = LR

## Lane 1: Product
## Lane 2: Engineering
## Lane 3: Design
## Lane 4: Growth

## Period 1: Q1 — Foundation
- [mvp] MVP Launch {done} {lane:Product} {icon:🚀}
  Core feature set shipped to first 100 users.
- [auth] Auth & Permissions {done} {lane:Engineering} {icon:🔒}
- [onb1] Onboarding v1 {done} {lane:Design} {icon:✨}
- [waitlist] Waitlist Campaign {done} {lane:Growth} {icon:📣}

## Period 2: Q2 — Growth
- [api] Public API {wip} {lane:Engineering} {icon:⚡}
  REST API for third-party integrations.
- [onb2] Onboarding v2 {wip} {lane:Design} {icon:🎨}
  Simplified 3-step flow with video walkthroughs.
- [dash] Analytics Dashboard {lane:Product} {icon:📊}
- [seo] SEO Content Push {lane:Growth} {icon:🔍}

## Period 3: Q3 — Scale
- [perf] Performance Hardening {lane:Engineering} {icon:⚙️}
- [integ] 3rd-party Integrations {lane:Product} {icon:🔗}
- [ref] Design System Refresh {lane:Design} {icon:🎯}
- [paid] Paid Ads Launch {lane:Growth} {icon:💰}

## Period 4: Q4 — Expansion
- [mobile] Mobile App Beta {lane:Engineering} {icon:📱}
- [enterprise] Enterprise Tier {lane:Product} {icon:🏢}
- [i18n] Internationalisation {lane:Design} {icon:🌍}
- [partner] Partner Program {lane:Growth} {icon:🤝}

## Flow
auth --> api: enables
onb1 --> onb2: v2 builds on
mvp --> dash: data source
api --> integ: prerequisite
