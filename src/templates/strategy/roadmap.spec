## Config
title: Product Roadmap
flow = LR

## Nodes
- [q1] Q1 2025 {hexagon} {fill:#4a90d9}
  Jan–Mar: Foundation.
- [q2] Q2 2025 {hexagon} {fill:#7b5ea7}
  Apr–Jun: Growth.
- [q3] Q3 2025 {hexagon} {fill:#cc5a4a}
  Jul–Sep: Scale.
- [feat1] Core Auth {rounded} {fill:#4a90d9} {done}
  Login, SSO, MFA.
- [feat2] Dashboard {rounded} {fill:#7b5ea7} {wip}
  Analytics + reports.
- [feat3] API v2 {rounded} {fill:#cc5a4a} {todo}
  Public REST surface.

## Flow
q1 --> q2: next
q2 --> q3: next
q1 --> feat1: delivers
q2 --> feat2: delivers
q3 --> feat3: delivers
feat1 --> feat2: unlocks
feat2 --> feat3: enables
