## Config
title: User Journey
flow = LR

## Swimlane: Awareness
- [discover] Discover {rounded} {done}
- [research] Research {rounded} {wip}

## Swimlane: Consideration
- [trial] Free Trial {rounded} {todo}
- [compare] Compare Plans {rounded} {todo}

## Swimlane: Decision
- [signup] Sign Up {rounded} {todo}
- [onboard] Onboarding {rounded} {todo}

## Flow
discover --> research
research --> trial
trial --> compare
compare --> signup
signup --> onboard
