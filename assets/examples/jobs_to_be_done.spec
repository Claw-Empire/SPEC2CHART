# Jobs To Be Done Canvas

Map customer jobs across functional, social, and emotional dimensions to find unmet needs.

## Config
flow = TB
spacing = 80

## The Job
- [j1] Job Statement {icon:💼} {fill:#89b4fa} {wip}
  "When I ___,  I want to ___, so I can ___."
- [j2] Situation / Trigger {icon:⚡} {fill:#89b4fa}
  What moment or context triggers this job?

## Functional Dimension
- [f1] What task must get done? {icon:⚙️} {fill:#a6e3a1}
  The practical outcome the customer needs.
- [f2] Current workarounds {icon:🔧} {fill:#a6e3a1}
  How do they do it today (and why it's painful)?
- [f3] Success criteria {icon:✅} {fill:#a6e3a1} {done}
  How do they know the job is done well?

## Social Dimension
- [s1] How do others see them? {icon:👥} {fill:#cba6f7}
  What status or identity is at stake?
- [s2] What do they want to signal? {icon:📣} {fill:#cba6f7}
  Belonging, expertise, care, success?

## Emotional Dimension
- [em1] How do they want to feel? {icon:😊} {fill:#f9e2af}
  Confident, safe, in-control, delighted?
- [em2] What do they fear or dread? {icon:😟} {fill:#f38ba8}
  Failure, embarrassment, wasted time?

## Opportunity
- [o1] Under-served outcome {icon:💡} {fill:#f9e2af} {todo}
  Where does the current solution fall short?
- [o2] Our unique angle {icon:🎯} {fill:#a6e3a1} {wip}
  What can only we solve, and how?

## Flow
j1 --> f1
j1 --> s1
j1 --> em1
j2 --> j1: triggers
f1 --> o1
f2 --> o1: gap
em2 --> o1: pain
o1 --> o2: leads to

## Summary
Jobs To Be Done Canvas: understand functional, social, and emotional dimensions of the customer job.
