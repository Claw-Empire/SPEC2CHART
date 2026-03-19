## Config
flow = LR

## Timeline

## Nodes
- [q1] Q1 {phase:Q1}
- [q2] Q2 {phase:Q2}
- [q3] Q3 {phase:Q3}
- [feat1] Feature Alpha {diamond} {phase:Q1} {wip} {owner:@team}
- [feat2] Feature Beta {diamond} {phase:Q2} {todo}
- [feat3] Feature Gamma {diamond} {phase:Q3} {todo}

## Flow
feat1 --> feat2: unlocks
feat2 --> feat3: enables
