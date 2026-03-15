# Causal Loop Diagram

Map reinforcing (R) and balancing (B) feedback loops between variables to reason about system behaviour.

## Config
flow = LR
spacing = 100

## Variables
- [v1] User Growth {icon:📈} {fill:#a6e3a1} {done}
  Total active users in the system.
- [v2] Word of Mouth {icon:📣} {fill:#89b4fa}
  Organic referrals from existing users.
- [v3] Product Quality {icon:⭐} {fill:#89b4fa} {wip}
  Perceived quality: reliability, UX, features.
- [v4] Dev Velocity {icon:⚡} {fill:#89b4fa}
  Speed of feature delivery and bug fixes.
- [v5] Team Size {icon:👥} {fill:#cba6f7}
  Number of engineers / designers working.
- [v6] Revenue {icon:💰} {fill:#a6e3a1}
  Monthly recurring revenue.
- [v7] Churn Rate {icon:🔻} {fill:#f38ba8} {blocked}
  % of users leaving per month.
- [v8] Support Load {icon:🎧} {fill:#f9e2af}
  Volume of support tickets per user.

## Flow
v1 --> v2: + more users
v2 --> v1: + referrals (R1)
v6 --> v5: + hire more
v5 --> v4: + more output
v4 --> v3: + better product
v3 --> v7: - less churn
v7 --> v1: - lose users
v3 --> v2: + NPS rises
v8 --> v4: - slows dev (B1)
v1 --> v8: + more tickets

## Notes
- R1 = Growth Loop: users → word of mouth → more users {fill:#a6e3a1}
- B1 = Quality Drag: scale → support load → slower dev {fill:#f9e2af}
- Arrow label convention: + reinforces, - opposes {color}

## Summary
Causal Loop Diagram: map reinforcing (R) and balancing (B) feedback loops to understand system dynamics.
