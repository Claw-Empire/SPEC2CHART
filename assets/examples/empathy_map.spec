# Empathy Map

Understand users deeply — what they say, think, do, and feel. Used to build
empathy before generating hypotheses about user needs and pain points.

## Config
flow = TB
spacing = 80

## User
- [user] Target User {user} {icon:👤} {glow}
  The persona or user segment being mapped.

## Says
- [s1] "This process takes forever" {quote} {done} {icon:💬}
  Direct quote from interviews or observations.
- [s2] "I wish I could see everything in one place" {quote} {done} {icon:💬}
  Expresses desire for unified view.
- [s3] "I always forget where I left off" {quote} {wip} {icon:💬}

## Thinks
- [t1] Worried about making the wrong decision {hypothesis} {icon:🧠}
  Internal concern not expressed openly.
- [t2] Hopes their work will be recognized {hypothesis} {icon:🧠}
- [t3] Skeptical about whether this tool will stick {assumption} {icon:🤔}

## Does
- [d1] Switches between 4 different apps daily {evidence} {icon:⚡} {done}
  Observed in user sessions — tool-switching logged.
- [d2] Takes manual notes in a separate doc {evidence} {icon:📝} {done}
- [d3] Shares screen during every standup {evidence} {icon:🖥}

## Feels
- [f1] Frustrated by context-switching {evidence} {icon:😤} {critical}
  CSAT score 2.8/5 for workflow continuity.
- [f2] Proud when they complete complex tasks {evidence} {icon:😊}
- [f3] Anxious before deadlines {evidence} {icon:😰}

## Pains
- [p1] Lost context when switching tools {risk} {icon:🔴} {warning}
- [p2] Hard to communicate progress to team {risk} {icon:🔴}

## Gains
- [g1] Faster onboarding for new teammates {evidence} {icon:🟢} {done}
- [g2] Single source of truth for project status {hypothesis} {icon:🟢}

## Flow
user --> s1
user --> s2
user --> s3
user --> t1
user --> t2
user --> t3
user --> d1
user --> d2
user --> d3
user --> f1
user --> f2
user --> f3
d1 --> p1: causes
f1 --> p1: reinforces
d2 --> g1: contributes to
t3 --> p2: causes
