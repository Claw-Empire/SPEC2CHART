# 5 Whys — Root Cause Analysis

Drill down from a problem statement through 5 levels of "Why?"
to uncover the true root cause.

## Config
flow = TB
spacing = 80

## Problem Statement
- [prob] Website checkout conversion dropped 35% overnight {hypothesis} {icon:⚠️} {critical}
  Observed: 2026-03-10. Baseline was 4.2%, now 2.7%.

## Why 1
- [w1] Why? → Users are abandoning at the payment step {question} {icon:❓}
  Funnel data shows 78% drop-off at the Stripe checkout screen.

## Why 2
- [w2] Why? → Payment page loads in 9 seconds on mobile {question} {icon:❓}
  Lighthouse score dropped from 82 → 41 after last Thursday's deploy.

## Why 3
- [w3] Why? → A 4MB uncompressed JS bundle was shipped {cause} {icon:💡}
  PR #412 — analytics library added without tree-shaking config.

## Why 4
- [w4] Why? → No bundle-size CI gate was in place {cause} {icon:💡}
  The team added a third-party lib without checking the size delta.

## Why 5
- [w5] Why? → No process to review bundle impact in pull requests {cause} {icon:💡} {glow}
  Root cause — a culture and tooling gap, not a one-time mistake.

## Countermeasures
- [fix1] Add bundlesize check to CI pipeline {experiment} {wip} {icon:🔧}
  Target: fail build if JS > 250 KB (gzipped).
- [fix2] Add performance budget to Lighthouse CI {experiment} {todo} {icon:🚀}
- [fix3] Create PR checklist for third-party dependencies {experiment} {todo} {icon:📋}

## Flow
prob --> w1: Why?
w1 --> w2: Why?
w2 --> w3: Why?
w3 --> w4: Why?
w4 --> w5: Root cause
w5 --> fix1: fix
w5 --> fix2: fix
w5 --> fix3: fix
