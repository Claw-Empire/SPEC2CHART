# Problem/Solution Fit

Validate that your solution actually solves the right problems for the right people, before investing in product-market fit.

## Config
flow = LR
spacing = 100

## Problem Space
- [ps1] Target Customer {icon:👤} {fill:#89b4fa} {done}
  Who has this problem? Be specific.
- [ps2] Jobs To Be Done {icon:💼} {fill:#89b4fa} {done}
  What are they trying to accomplish?
- [ps3] Pains {icon:😣} {fill:#f38ba8} {wip}
  What frustrates them today? What risks / obstacles?
- [ps4] Gains {icon:😊} {fill:#a6e3a1}
  What outcomes and benefits do they desire?
- [ps5] Existing alternatives {icon:🔧} {fill:#cba6f7}
  How do they solve it today, however imperfectly?

## Solution Space
- [ss1] Product / Feature {icon:📦} {fill:#89b4fa} {wip}
  What are you building?
- [ss2] Pain Relievers {icon:💊} {fill:#a6e3a1} {wip}
  How exactly does it remove the pains?
- [ss3] Gain Creators {icon:🎁} {fill:#a6e3a1}
  How does it create the gains they desire?
- [ss4] Unique Advantage {icon:🏆} {fill:#cba6f7} {glow}
  What can only you deliver that alternatives can't?

## Fit Assessment
- [f1] Fit Score {icon:📊} {fill:#f9e2af} {todo}
  Do our solutions address the most critical pains and gains?
- [f2] Riskiest assumption {icon:⚠️} {fill:#f38ba8} {blocked}
  What must be true for this fit to hold?
- [f3] Next experiment {icon:🧪} {fill:#cba6f7} {todo}
  What's the smallest test to validate fit?

## Flow
ps1 --> ps2: has these
ps2 --> ps3: causes these
ps2 --> ps4: wants these
ps3 --> ss2: relieved by
ps4 --> ss3: created by
ps5 --> ss4: differentiated from
ss1 --> ss2
ss1 --> ss3
ss2 --> f1: contributes to
ss3 --> f1: contributes to
ss4 --> f1: differentiates
f1 --> f2: exposes
f2 --> f3: generates

## Summary
Problem/Solution Fit: validate that your solution directly addresses the right pains and gains before investing in growth.
