# Customer Journey Map

Map the end-to-end experience across touchpoints and emotions.
Identify moments of delight and points of friction in the user journey.

## Config
flow = LR
spacing = 70

## Stage 1: Awareness
- [s1a] Sees ad on LinkedIn {icon:📣} {done}
  Paid social — targeted to "founder, 25-45" segment.
- [s1b] Reads blog post via Google {icon:🔍} {done}
  Organic traffic — high-intent keyword: "SaaS boilerplate 2026".
- [s1c] Emotion: Curious, cautious {icon:😐} {ok}
  Neutral — not yet sold.

## Stage 2: Consideration
- [s2a] Views product demo video {icon:▶️} {done}
- [s2b] Reads feature comparison table {icon:📋} {wip}
  Friction: comparison is hard to find (buried in docs).
- [s2c] Signs up for waitlist {icon:✉️} {done}
- [s2d] Emotion: Hopeful, but skeptical {icon:🤔} {info}
  Still comparing with 2 other tools.

## Stage 3: Trial
- [s3a] Receives onboarding email {icon:📧} {done}
- [s3b] Clones repo and reads README {icon:💻} {done}
- [s3c] Runs setup script — error on step 3 {icon:❌} {blocked}
  Friction: env variable docs are incomplete.
- [s3d] Emotion: Frustrated {icon:😤} {critical}

## Stage 4: Purchase
- [s4a] Reads refund policy {icon:📄} {done}
- [s4b] Buys $249 license {icon:💳} {done}
  Converted after second attempt (fixed setup issue via Discord).
- [s4c] Emotion: Relieved, committed {icon:😌} {ok}

## Stage 5: Advocacy
- [s5a] Ships MVP in 4 days {icon:🚀} {done}
- [s5b] Posts "Day 1 review" on Twitter {icon:🐦} {done}
  Organic virality — 1.2k likes, 80 link clicks.
- [s5c] Emotion: Delighted, proud {icon:😍} {ok} {glow}

## Pain Points
- [p1] Setup docs incomplete {hypothesis} {critical} {icon:⚠️}
  Hypothesis: better docs reduce churn-before-purchase by 30%.
- [p2] Demo video not discoverable on homepage {hypothesis} {warning} {icon:⚠️}
- [p3] Comparison table buried {hypothesis} {warning} {icon:⚠️}

## Flow
s1a --> s2a
s1b --> s2b
s2a --> s2b
s2b --> s2c
s2c --> s3a
s3a --> s3b
s3b --> s3c
s3c --> s4a: despite friction
s4a --> s4b
s4b --> s5a
s5a --> s5b
s3c --> p1: caused by
s2b --> p3: caused by
p1 --> s2d: amplifies
p2 --> s2a
