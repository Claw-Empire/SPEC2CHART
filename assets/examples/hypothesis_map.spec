# Hypothesis Map

A structured canvas for testing ideas: map assumptions, gather evidence,
and track experiments toward validated conclusions.

## Config
flow = TB

## Hypotheses
- [h1] Users churn because onboarding is confusing {hypothesis} {icon:💡}
  Our primary growth hypothesis to validate this quarter.
- [h2] Power users want batch export {hypothesis} {icon:⚡}

## Assumptions
- [a1] Users read the tutorial {assumption}
- [a2] Users have 5+ items before they churn {assumption}
- [a3] Batch export saves >30 min/week {assumption}

## Evidence
- [e1] 68% drop-off at step 3 of onboarding {evidence} {done}
  Analytics data from Mixpanel — confirmed.
- [e2] Support tickets mention "export" 40% of the time {evidence} {wip}
- [e3] NPS score for power users is 72 {evidence}

## Questions
- [q1] What is the exact step where users get confused? {question}
- [q2] Who are our power users — segment definition? {question}

## Experiments
- [x1] A/B test simplified onboarding with video {experiment} {wip}
- [x2] Prototype batch export and user-test it {experiment}
- [x3] Exit survey on churned users {experiment} {done}

## Conclusions
- [c1] Onboarding video reduces drop-off by 40% {conclusion} {glow}
  Validated — ready to ship.

## Flow
a1 --> h1
a2 --> h1
e1 --> h1: supports
e3 --> h2: supports
q1 --> x1: drives
q2 --> x2: drives
x1 --> c1: result
x3 --> e1: source
h1 --> c1
