# Business Model Canvas

Map the 9 building blocks of a business model on one page.

## Config
flow = LR
spacing = 70

## Key Partners
- [kp1] Strategic Allies {icon:🤝}
  Who are our key partners and suppliers?
- [kp2] Resource Providers {icon:🔗}
  Who provides key resources we can't own?

## Key Activities
- [ka1] Core Operations {icon:⚙️} {wip}
  What do we do every day to deliver value?
- [ka2] Platform Maintenance {icon:🔧}
  What keeps the product running?

## Value Propositions
- [vp1] Main Value {icon:💎} {done}
  What value do we deliver to the customer?
- [vp2] Pain Reliever {icon:💊} {done}
  Which customer problems are we solving?
- [vp3] Gain Creator {icon:🌟}
  What customer desires do we satisfy?

## Customer Relationships
- [cr1] Acquisition {icon:📣}
  How do we get customers?
- [cr2] Retention {icon:🔄} {wip}
  How do we keep them?

## Customer Segments
- [cs1] Primary Segment {icon:👥} {done}
  For whom are we creating value?
- [cs2] Secondary Segment {icon:👤}
  Who are our most important customers?

## Key Resources
- [kr1] Technology {icon:💻}
  What key assets does our value prop require?
- [kr2] Human Capital {icon:🧠}
  What expertise is critical?

## Channels
- [ch1] Sales Channel {icon:🛒} {wip}
  How do we reach our customer segments?
- [ch2] Support Channel {icon:📞}
  How do customers get help?

## Cost Structure
- [c1] Fixed Costs {icon:🏢}
  What are our most important costs?
- [c2] Variable Costs {icon:📈}
  Which costs scale with volume?

## Revenue Streams
- [r1] Subscription {icon:💰} {wip}
  For what value are customers willing to pay?
- [r2] Transaction Fees {icon:💳}
  How are they paying now?

## Flow
kp1 --> ka1: enables
kp2 --> kr1: supplies
ka1 --> vp1: produces
ka2 --> vp1: maintains
kr1 --> vp2: supports
vp1 --> cr1
vp2 --> cr2
vp3 --> cs1
cr1 --> cs1: attracts
cr2 --> cs2: retains
ch1 --> cs1: reaches
ch2 --> cs2: supports
c1 --> ka1: funds
r1 --> c1: offsets
r2 --> c2: offsets

## Summary
Business Model Canvas: 9-block visual framework for mapping how a business creates, delivers, and captures value.
