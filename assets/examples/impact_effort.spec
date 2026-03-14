# Impact / Effort Matrix

Prioritize initiatives by plotting high-vs-low impact against high-vs-low effort.
Focus on Quick Wins — high impact, low effort — first.

## Config
flow = LR
spacing = 70

## Quick Wins — High Impact, Low Effort
- [qw1] Add search autocomplete {icon:🔍} {done}
  Estimated 2 days. Reduces support queries by 20%.
- [qw2] Fix mobile nav menu overflow bug {icon:📱} {done}
- [qw3] Enable email digest feature flag {icon:📧} {wip}
  Already built — just needs QA sign-off.
- [qw4] Improve error messages with actionable text {icon:💬} {todo}

## Strategic Bets — High Impact, High Effort
- [sb1] Rebuild onboarding with video walkthroughs {icon:🎬} {wip}
  Q2 initiative — 3 weeks engineering, 1 week design.
- [sb2] Launch mobile app {icon:📱} {todo}
  6-month project. Major revenue opportunity.
- [sb3] Enterprise SSO & permissions {icon:🔒} {todo}

## Fill-ins — Low Impact, Low Effort
- [fi1] Update homepage hero copy {icon:✏️} {done}
- [fi2] Fix typos in help docs {icon:📝} {done}
- [fi3] Add dark mode to settings page {icon:🌙} {todo}

## Time Sinks — Low Impact, High Effort
- [ts1] Full rebrand visual refresh {icon:🎨} {blocked}
  Deprioritized — no clear ROI evidence.
- [ts2] Rebuild data pipeline from scratch {icon:🔧} {todo}
  Existing pipeline works — incremental improvements preferred.

## Flow
qw1 --> sb1: enables
qw3 --> sb1: prerequisite
sb1 --> sb2: builds toward
qw4 --> sb3: supports

## Notes
- Focus this sprint on Quick Wins {ok}
- Review Strategic Bets monthly {info}
- Time Sinks need ROI evidence before scheduling {warning}
