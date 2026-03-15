# Hypothesis Validation Canvas

Structure a single experiment: what we believe, how we'll test it, what we found.

## Config
flow = TB
spacing = 80

## Hypothesis
- [h1] We believe that... {icon:💡} {wip}
  State your assumption as a falsifiable hypothesis.
- [h2] For users who... {icon:👥}
  Who is the target user segment?
- [h3] Solving the problem of... {icon:🎯}
  What specific problem does this address?

## Test
- [t1] We will test by... {icon:🧪} {todo}
  Design the minimum viable experiment.
- [t2] Success metric {icon:📊}
  Define the threshold that proves/disproves the hypothesis.
- [t3] Time box {icon:⏰}
  When will we decide? Set a deadline.

## Result
- [r1] We found that... {icon:🔍}
  Describe what actually happened.
- [r2] Evidence collected {icon:📋}
  Quantitative or qualitative data supporting the finding.
- [r3] Confidence level {icon:🎯}
  How confident are we in this result?

## Learning
- [l1] Therefore we will... {icon:🔄}
  Pivot / Persevere / Kill?
- [l2] Next hypothesis {icon:💡}
  What to test next based on what we learned.

## Notes
- Frame hypotheses as "If X, then Y, because Z" {color}

## Flow
h1 --> t1: test design
h2 --> t1: scope
h3 --> t1: focus
t1 --> r1: experiment
t2 --> r2: measured by
r1 --> l1: leads to
r2 --> l1: informs
l1 --> l2: generates

## Summary
Hypothesis Validation Canvas: structure a single lean experiment from belief to learning.
