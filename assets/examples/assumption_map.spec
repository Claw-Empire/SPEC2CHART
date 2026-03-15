# Assumption Map

Map assumptions by importance and certainty to prioritize what to test first.

## Config
flow = TB
spacing = 80

## Test First
- [t1] Core Value Assumption {icon:⚠️} {blocked}
  Users will pay for this. HIGH importance, LOW certainty.
- [t2] Problem Existence {icon:⚠️} {wip}
  This problem is painful enough to solve. HIGH importance, LOW certainty.
- [t3] Frequency Assumption {icon:⚠️}
  Users encounter this weekly. HIGH importance, LOW certainty.

## Validate
- [v1] Market Size {icon:📊} {done}
  TAM is large enough to build a business. HIGH importance, HIGH certainty.
- [v2] Competitive Gap {icon:🔍} {done}
  No good existing solution. HIGH importance, HIGH certainty.
- [v3] Regulatory Fit {icon:📋} {wip}
  No blocking regulatory issues. HIGH importance, HIGH certainty.

## Monitor
- [m1] Platform Risk {icon:👁}
  App store / platform dependency. LOW importance, LOW certainty.
- [m2] Team Capability {icon:👥}
  Team can build this. LOW importance, LOW certainty.

## Safe to Assume
- [s1] Internet Access {icon:✅} {done}
  Target users have reliable internet. LOW importance, HIGH certainty.
- [s2] Smartphone Ownership {icon:✅} {done}
  Users own a smartphone. LOW importance, HIGH certainty.

## Flow
t1 --> v1: if validated
t2 --> v2: if validated
t3 --> m1: if low risk

## Summary
Assumption Map: quadrant tool for prioritizing which beliefs to test before building.
