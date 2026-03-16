# Knowledge Base Structure

Map your support knowledge base: categories, article types, and content lifecycle.
Use this to plan, audit, and grow a self-service support library.

## Config
flow = TB
spacing = 80

## KB Root
- [kb_root] Knowledge Base {icon:📚} {done}
  Primary self-service destination — reduces ticket volume

## Categories
- [cat_getting_started] Getting Started {icon:🚀} {done}
  Onboarding guides, first-time setup, quickstart tutorials
- [cat_features] Features & How-To {icon:📖} {wip}
  Walkthroughs for each product feature
- [cat_troubleshoot] Troubleshooting {icon:🔧} {wip}
  Common error messages, workarounds, known issues
- [cat_billing] Billing & Account {icon:💳} {done}
  Plans, invoices, upgrades, cancellations
- [cat_api] API & Integrations {icon:🔌} {info}
  API reference, webhooks, third-party integrations
- [cat_security] Security & Compliance {icon:🔐} {info}
  Privacy, data handling, certifications

## Article Types
- [type_how_to] How-To Guide {icon:📝}
  Step-by-step instructions for a specific task
- [type_concept] Concept Explanation {icon:💡}
  Background knowledge — the "why" behind a feature
- [type_faq] FAQ {icon:❓}
  Short answers to common questions
- [type_ref] Reference {icon:📋}
  Tables, parameters, field definitions
- [type_trouble] Troubleshooting Guide {icon:⚙️}
  Symptom → cause → fix format

## Content Lifecycle
- [draft] Draft {todo} {icon:✏️}
  Author writes initial content
- [review] In Review {wip} {icon:👁}
  SME reviews for accuracy; editor reviews for clarity
- [published] Published {done} {icon:✅} {glow}
  Live and indexed — update monthly
- [needs_update] Needs Update {warning} {icon:⚠️}
  Triggered by: product change / support ticket spike
- [archived] Archived {dim} {icon:🗃️}
  Outdated — not deleted, kept for reference

## Metrics
- [metric_deflection] Ticket deflection rate {hypothesis} {icon:📉}
  Goal: 40% of inbound tickets resolved via KB
- [metric_rating] Article rating {icon:⭐}
  Track thumbs-up/down per article; flag < 60% positive
- [metric_search] Search no-results rate {hypothesis} {icon:🔍}
  No-results queries → content gap candidates

## Flow
kb_root --> cat_getting_started
kb_root --> cat_features
kb_root --> cat_troubleshoot
kb_root --> cat_billing
kb_root --> cat_api
kb_root --> cat_security
cat_getting_started --> type_how_to: uses
cat_features --> type_how_to: uses
cat_features --> type_concept: uses
cat_troubleshoot --> type_trouble: uses
cat_troubleshoot --> type_faq: uses
cat_billing --> type_faq: uses
cat_api --> type_ref: uses
draft --> review
review --> published: approved
review --> draft: changes needed
published --> needs_update: product change
needs_update --> draft: revise
published --> archived: superseded
published --> metric_deflection: drives
published --> metric_rating: tracked by
metric_search --> draft: gap found
