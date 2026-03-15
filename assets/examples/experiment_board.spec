# Experiment Board

Track live hypotheses as they move from backlog through running to validated or invalidated.

## Config
flow = LR
spacing = 90

## Backlog
- [b1] Hypothesis A {icon:💡} {fill:#89b4fa} {todo}
  State the assumption you want to test.
- [b2] Hypothesis B {icon:💡} {fill:#89b4fa} {todo}
  What do you believe will happen if X?
- [b3] Hypothesis C {icon:💡} {fill:#89b4fa}
  If we do Y, users will Z.

## Running
- [r1] A/B Test: CTA colour {icon:🧪} {fill:#cba6f7} {wip}
  Split 50/50. Measuring click-through rate.
- [r2] Onboarding checklist {icon:🧪} {fill:#cba6f7} {wip}
  50 users in cohort. Measuring activation rate.

## Validated
- [v1] Social proof banner {icon:✅} {fill:#a6e3a1} {done}
  +12% landing page conversions. Ship it.
- [v2] Email drip: day-3 nudge {icon:✅} {fill:#a6e3a1} {done}
  +8% week-1 retention. Rolled out.

## Invalidated
- [x1] Exit-intent discount {icon:❌} {fill:#f38ba8} {blocked}
  No significant lift. Users annoyed.
- [x2] Gamification badges {icon:❌} {fill:#f38ba8} {blocked}
  Engaged users ignored. Disengaged users churned faster.

## Learnings
- [l1] Copy matters more than design {icon:📖} {fill:#f9e2af}
  Every validated experiment had clear, specific copy.
- [l2] Timing beats incentives {icon:📖} {fill:#f9e2af}
  Right message at right moment > discount.

## Flow
b1 --> r1: test design
b2 --> r2: test design
r1 --> v1: validated
r2 --> x1: invalidated
v1 --> l1: learned
x1 --> l1: learned
x2 --> l2: learned

## Summary
Experiment Board: track hypothesis experiments from backlog through running to validated or invalidated.
