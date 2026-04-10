## Config
title: Product Roadmap
flow = LR

## Nodes
- [q1] Q1 {hexagon} {fill:#4a90d9}
- [q2] Q2 {hexagon} {fill:#7b5ea7}
- [q3] Q3 {hexagon} {fill:#cc5a4a}
- [feat1] Alpha {diamond} {fill:#4a90d9} {done}
- [feat2] Beta {diamond} {fill:#7b5ea7} {wip}
- [feat3] Gamma {diamond} {fill:#cc5a4a} {todo}

## Flow
q1 --> q2: next
q2 --> q3: next
q1 --> feat1: delivers
q2 --> feat2: delivers
q3 --> feat3: delivers
feat1 --> feat2: unlocks
feat2 --> feat3: enables
