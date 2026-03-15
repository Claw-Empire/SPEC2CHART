# What? So What? Now What?

A structured debrief framework for extracting learning from experiments, retros, and incidents.

## Config
flow = TB
spacing = 80

## What? (Observe)
- [w1] What happened? {icon:👁} {fill:#89b4fa} {done}
  Describe the raw facts without interpretation.
- [w2] What data did we collect? {icon:📊} {fill:#89b4fa} {done}
  Metrics, logs, user quotes — just the evidence.
- [w3] What surprised us? {icon:😲} {fill:#89b4fa}
  Events that differed from our expectations.

## So What? (Interpret)
- [s1] What does this mean? {icon:💡} {fill:#cba6f7} {wip}
  What patterns or insights do we see?
- [s2] Why did this happen? {icon:🔍} {fill:#cba6f7} {wip}
  Root causes, contributing factors, context.
- [s3] What assumptions were wrong? {icon:❓} {fill:#f38ba8}
  What did we believe that turned out to be false?

## Now What? (Act)
- [n1] What will we change? {icon:🔄} {fill:#a6e3a1} {todo}
  Specific actions to take next.
- [n2] What will we keep doing? {icon:✅} {fill:#a6e3a1}
  Things that worked and should continue.
- [n3] What will we stop doing? {icon:🛑} {fill:#f38ba8}
  Things that didn't work or wasted time.
- [n4] What will we test next? {icon:🧪} {fill:#f9e2af}
  The next hypothesis this learning generates.

## Flow
w1 --> s1: interpret
w2 --> s1: evidence
w3 --> s2: investigate
s1 --> n1: leads to
s2 --> n2: informs
s3 --> n4: generates
n1 --> n4: iterate

## Summary
What? So What? Now What? — a structured debrief for extracting learning and deciding next actions.
