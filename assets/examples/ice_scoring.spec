# ICE Scoring Matrix

Prioritize experiments by Impact × Confidence × Ease. Score each 1-10; multiply for rank.

## Config
flow = LR
spacing = 90

## Experiments
- [e1] Onboarding Checklist {icon:🧪} {fill:#89b4fa}
  Show a progress checklist during first-run.
- [e2] Social Proof Banner {icon:🧪} {fill:#89b4fa}
  Display live user count on the landing page.
- [e3] Pricing Page Redesign {icon:🧪} {fill:#89b4fa}
  Simplify pricing to 2 tiers instead of 3.
- [e4] Exit-Intent Popup {icon:🧪} {fill:#cba6f7}
  Offer a discount when user moves to leave.

## Scores
- [s1] I:9 · C:7 · E:8 = 504 {icon:🏆} {fill:#a6e3a1} {done}
  High confidence, easy to ship. Run first.
- [s2] I:8 · C:6 · E:9 = 432 {icon:📊} {fill:#a6e3a1} {done}
  Quick to implement, good uplift expected.
- [s3] I:7 · C:5 · E:5 = 175 {icon:📊} {fill:#f9e2af} {wip}
  Medium confidence. Validate copy first.
- [s4] I:6 · C:4 · E:7 = 168 {icon:📊} {fill:#f9e2af}
  Low confidence. Run after higher scores.

## Priority Tiers
- [run] Run Now (≥ 400) {icon:🚀} {fill:#a6e3a1} {ok}
- [later] Run Later (150–399) {icon:⏳} {fill:#f9e2af} {warning}
- [kill] Kill / Rethink (< 150) {icon:🗑} {fill:#f38ba8} {critical}

## Flow
e1 --> s1
e2 --> s2
e3 --> s3
e4 --> s4
s1 --> run
s2 --> run
s3 --> later
s4 --> later

## Summary
ICE Scoring Matrix: rank experiments by Impact × Confidence × Ease to decide what to run first.
