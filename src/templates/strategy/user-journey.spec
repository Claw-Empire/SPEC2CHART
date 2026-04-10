## Config
title: User Journey
flow = LR

## Nodes
- [discover] Discover {rounded} {done}
- [research] Research {document} {done}
- [trial] Free Trial {rounded} {wip}
- [compare] Compare Plans {diamond}
- [signup] Sign Up {hexagon} {todo}
- [churn] Exit Funnel {rounded} {critical}
- [onboard] Onboarding {rounded} {fill:#4a90d9} {todo}
- [retained] Active User {rounded} {fill:#a6e3a1} {todo}

## Flow
discover --> research: explores options
research --> trial: starts trial
trial --> compare: evaluates fit
compare --> signup: decides to buy
compare --> churn: no fit
signup --> onboard: gets started
onboard --> retained: succeeds
