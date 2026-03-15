# Double Diamond Design Process

A structured design thinking framework: discover widely, define precisely, develop ideas, deliver solutions.

## Config
flow = LR
spacing = 90

## Discover
- [d1] Research {icon:🔍} {wip}
  Ethnographic research and user interviews.
- [d2] Observations {icon:👁}
  Field notes and behavioral patterns.
- [d3] Data Collection {icon:📊}
  Surveys, analytics, and secondary research.
- [d4] Stakeholders {icon:👥}
  Identify all parties and their needs.

## Define
- [def1] Insights {icon:💡} {done}
  Synthesized patterns from research.
- [def2] Problem Statement {icon:🎯} {done}
  HMW: How might we… framing.
- [def3] Personas {icon:🧑}
  Key user archetypes and needs.

## Develop
- [dev1] Ideation {icon:🌱} {wip}
  Brainstorm broad range of solutions.
- [dev2] Prototypes {icon:🔧}
  Low-fidelity concepts to test.
- [dev3] Testing {icon:🧪}
  Validate ideas with real users.
- [dev4] Iteration {icon:🔄}
  Refine based on feedback.

## Deliver
- [del1] Solution {icon:✨}
  Validated, refined design.
- [del2] Implementation {icon:🚀}
  Build and launch plan.
- [del3] Measure {icon:📈}
  Track outcomes against success metrics.

## Flow
d1 --> def1
d2 --> def1
d3 --> def2
d4 --> def2
def1 --> dev1
def2 --> dev1
def3 --> dev1
dev1 --> dev2
dev2 --> dev3
dev3 --> dev4
dev4 --> del1
del1 --> del2
del2 --> del3

## Summary
Double Diamond: diverge in Discover, converge in Define, diverge in Develop, converge in Deliver.
