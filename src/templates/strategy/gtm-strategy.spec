## Config
title: GTM Strategy
flow = TB

## Nodes
- [hn] Launch Post {rounded} {fill:#f9e2af} {wip}
  HN / ProductHunt day-one.
- [blog] Dev Blog {rounded} {fill:#89b4fa}
  Technical deep-dive series.
- [twitter] Twitter/X {rounded} {fill:#b4befe} {todo}
  Build-in-public thread.
- [dl] Free Download {hexagon} {fill:#a6e3a1} {ok}
  Self-serve binary.
- [gh] GitHub Stars {hexagon} {fill:#cba6f7}
  Community signal.
- [trial] Trial Signup {rounded} {fill:#f9e2af} {wip}
  14-day full access.
- [pro] Pro Plan {rounded} {fill:#a6e3a1} {done}
  Paid conversion.

## Flow
hn --> dl: traffic
hn --> gh: stars
blog --> gh: stars
blog --> trial: qualified lead
twitter --> dl: referral
twitter --> gh: viral loop
dl --> trial: upgrade prompt
gh --> trial: repo cta
trial --> pro: converts
