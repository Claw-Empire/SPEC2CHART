# Theory of Change

Map the causal chain from inputs through activities and outputs to the long-term impact you aim to create.

## Config
flow = LR
spacing = 100

## Inputs
- [i1] Team capacity {icon:👥} {fill:#89b4fa}
  Engineers, designers, PMs committed to this.
- [i2] Budget {icon:💰} {fill:#89b4fa}
  Funding for tooling, research, and growth.
- [i3] Domain knowledge {icon:📚} {fill:#89b4fa}
  Deep understanding of the user problem.

## Activities
- [a1] User research {icon:🔍} {fill:#cba6f7} {wip}
  Interviews, surveys, shadowing sessions.
- [a2] Rapid prototyping {icon:⚡} {fill:#cba6f7} {wip}
  Build → Test → Learn cycles (2-week sprints).
- [a3] Community building {icon:🤝} {fill:#cba6f7}
  Onboard early adopters and collect feedback.

## Outputs
- [o1] Validated product features {icon:✅} {fill:#a6e3a1} {done}
  Proven features shipped to users.
- [o2] Engaged user community {icon:👥} {fill:#a6e3a1}
  Active users who refer others.
- [o3] Knowledge base {icon:📋} {fill:#a6e3a1}
  Documented learnings and playbooks.

## Short-Term Outcomes
- [s1] Improved user activation {icon:📈} {fill:#f9e2af}
  More users reach their "aha moment."
- [s2] Reduced churn {icon:📉} {fill:#f9e2af}
  Fewer users leave in month 1.

## Long-Term Impact
- [im1] Market leadership {icon:🏆} {fill:#fab387} {glow}
  Recognised as the go-to solution in our category.
- [im2] Sustainable growth {icon:🌱} {fill:#fab387} {glow}
  Profitable, compounding user growth.

## Assumptions
- [as1] Users want this problem solved {icon:💭} {fill:#313244}
- [as2] We can reach them cost-effectively {icon:💭} {fill:#313244}

## Flow
i1 --> a1
i2 --> a2
i3 --> a1
a1 --> o1: insights
a2 --> o1: features
a3 --> o2: community
o1 --> s1: activation
o2 --> s2: retention
s1 --> im1: growth loop
s2 --> im2: compounding
as1 --> a1: hypothesis
as2 --> a3: hypothesis

## Summary
Theory of Change: trace the causal chain from inputs through activities, outputs, and outcomes to long-term impact.
