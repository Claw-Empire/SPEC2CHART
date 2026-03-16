# Voice of Customer (VoC)

Collect, cluster, and prioritize customer feedback to drive product and support decisions.
Map raw signals to themes, then to actions.

## Config
flow = TB
spacing = 80

## Raw Signals
- [s1] "Setup was confusing on day 1" {icon:😤} {critical}
  Source: onboarding survey, Mar 2026. Recurrence: 23 mentions
- [s2] "Can't find the export button" {icon:🔍} {warning}
  Source: support tickets. Recurrence: 15 mentions
- [s3] "The 3D view is amazing for architecture reviews" {icon:🤩} {ok}
  Source: NPS comment. Recurrence: 8 mentions
- [s4] "I wish I could share a read-only link" {icon:🔗} {info}
  Source: feature request. Recurrence: 31 mentions
- [s5] "Keyboard shortcuts take too long to learn" {icon:⌨️} {warning}
  Source: churn exit survey. Recurrence: 12 mentions
- [s6] "Importing our Notion pages would save hours" {icon:📋} {info}
  Source: community forum. Recurrence: 19 mentions

## Themes
- [t_onboard] Onboarding friction {critical} {icon:🚪}
  Users can't get started fast enough
- [t_discoverability] Feature discoverability {warning} {icon:🔍}
  Core features are hard to find
- [t_delight] 3D visualization delight {ok} {icon:✨}
  Users love spatial architecture reviews
- [t_collab] Collaboration & sharing {info} {icon:👥}
  Teams want lightweight sharing workflows
- [t_learning_curve] Learning curve {warning} {icon:📈}
  Shortcuts and power features feel hidden
- [t_import] Import / integrations {info} {icon:🔌}
  Users want to bring in data from existing tools

## Actions
- [a1] Redesign day-1 onboarding flow {critical} {wip} {icon:🎯}
  Owner: Product. Target: Q2 2026
- [a2] Add feature discoverability overlay {warning} {todo} {icon:🗺️}
  Contextual hints for hidden features
- [a3] Read-only share link (viewer mode) {info} {todo} {icon:🔒}
  Owner: Engineering. Target: Q3 2026
- [a4] Interactive keyboard shortcut trainer {info} {todo} {icon:⌨️}
  Guided tour on first use
- [a5] Notion / markdown import {info} {todo} {icon:📥}
  Owner: Integrations team. Target: Q4 2026

## Flow
s1 --> t_onboard: theme
s2 --> t_discoverability: theme
s3 --> t_delight: theme
s4 --> t_collab: theme
s5 --> t_learning_curve: theme
s6 --> t_import: theme
t_onboard --> a1: drives
t_discoverability --> a2: drives
t_collab --> a3: drives
t_learning_curve --> a4: drives
t_import --> a5: drives
