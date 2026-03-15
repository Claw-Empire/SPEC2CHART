# 2×2 Prioritization Matrix

Plot items on two axes (e.g. Impact vs Effort, Value vs Risk) to make prioritization decisions visible.

## Config
flow = TB
spacing = 80

## High Impact / Low Effort  ← Do First
- [q1] Quick Win Zone {icon:🏆} {fill:#a6e3a1} {done}
  Experiments here: high upside, fast to run. Do these first.
- [i1] Item: Onboarding email tweak {icon:✅} {fill:#a6e3a1} {done}
  2h to implement, +8% activation expected.
- [i2] Item: Social proof counter {icon:✅} {fill:#a6e3a1}
  1 day, significant trust signal.

## High Impact / High Effort  ← Plan
- [q2] Strategic Bets {icon:🎯} {fill:#89b4fa} {wip}
  Worth doing — but plan and resource properly.
- [i3] Item: Mobile app {icon:📱} {fill:#89b4fa} {wip}
  6 weeks. High impact if we can execute.
- [i4] Item: Enterprise auth {icon:🔑} {fill:#cba6f7}
  Unlocks a new segment. Complex.

## Low Impact / Low Effort  ← Fill-in
- [q3] Fill-in Work {icon:📋} {fill:#f9e2af}
  Nice-to-have. Do when team has spare capacity.
- [i5] Item: Dark mode {icon:🌙} {fill:#f9e2af}
  Polishing. Low activation impact.

## Low Impact / High Effort  ← Avoid
- [q4] Avoid / Rethink {icon:🗑} {fill:#f38ba8} {blocked}
  High cost, low return. Kill or redesign.
- [i6] Item: Custom branding per user {icon:🎨} {fill:#f38ba8} {blocked}
  Complex. Niche use. Deferring.

## Flow
q1 --> i1
q1 --> i2
q2 --> i3
q2 --> i4
q3 --> i5
q4 --> i6

## Notes
- X-axis: Effort (Low → High left to right) {color}
- Y-axis: Impact (High → Low top to bottom) {color}

## Summary
2×2 Prioritization Matrix: plot experiments by Impact vs Effort to make priorities explicit and visible.
