## Config
title: User Journey
flow = TB

## Nodes
- [discover] Discover {circle} {fill:#a6e3a1} {done}
  First touch via search or ad.
- [research] Research {document} {fill:#89b4fa} {done}
  Reads docs + reviews.
- [trial] Free Trial {rounded} {fill:#f9e2af} {wip}
  14-day sandbox access.
- [compare] Compare Plans {diamond} {fill:#f9e2af}
  Evaluates pricing tiers.
- [signup] Sign Up {hexagon} {fill:#cba6f7} {todo}
  Account creation gate.
- [churn] Exit Funnel {circle} {fill:#f38ba8} {critical}
  Drops before purchase.
- [onboard] Onboarding {rounded} {fill:#4a90d9} {todo}
  Guided product tour.
- [retained] Active User {circle} {fill:#a6e3a1} {done}
  Weekly engagement.

## Flow
discover --> research: explores options
research --> trial: starts trial
trial --> compare: evaluates fit
compare --> signup: decides to buy
compare --> churn: no fit
signup --> onboard: gets started
onboard --> retained: succeeds
