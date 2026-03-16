# Customer Support Queue

Live kanban for active support tickets.
Use ⇧D/⇧W to set due dates · P key to set priority · S key to cycle status.
] / [ to advance or demote a ticket through sections · ⌘⇧R for markdown report · ⌘⇧X for CSV export.

## Config
flow = LR
spacing = 80

## Intake
- [t1] Login page 500 errors {p1} {glow}
  {assigned:Alice} {due:2026-03-16} {created:2026-03-14}
  Auth service returning 500 on POST /login
- [t2] Export feature missing data {p2}
  {assigned:Bob} {due:2026-03-18} {created:2026-03-15}
  CSV export omits last column when >500 rows
- [t3] Dark mode glitch on iOS {p3}
  {assigned:Carol} {due:2026-03-22} {created:2026-03-16}
  Background flashes white on navigation

## Triage
- [t4] Payment gateway timeout {p1} {urgent}
  {assigned:Alice} {due:2026-03-16} {created:2026-03-12}
  Stripe webhook not firing for 3DS payments — 40% checkout failure
- [t5] Notification spam {p2}
  {assigned:Bob} {due:2026-03-19} {created:2026-03-13}
  Users getting duplicate email alerts
- [t6] Report PDF blank pages {p3}
  {assigned:Dana} {due:2026-03-24} {created:2026-03-14}
  Monthly reports intermittently print blank pages

## In Progress
- [t7] Search returns wrong results {p1}
  {assigned:Carol} {due:2026-03-17}
  Full-text search misses recently indexed content
- [t8] Password reset email delay {p2}
  {assigned:Bob} {due:2026-03-20}
  Reset emails delayed 10–15 minutes — SES queue issue
- [t9] API rate limit too aggressive {p3}
  {assigned:Dana} {due:2026-03-25}
  /v2/items endpoint throttles at 10 req/s for paid tier

## Resolved
- [t10] Mobile push not firing {p2} {done}
  {assigned:Carol} {due:2026-03-15}
  FCM topic subscription bug — deployed fix v3.4.2
- [t11] Chart tooltip off-screen {p4} {done}
  {assigned:Dana} {due:2026-03-14}
  z-index fix applied
- [t12] Timezone display wrong {p3} {done}
  {assigned:Bob} {due:2026-03-13}
  UTC offset calculation corrected for DST transition

## Flow
t1 --> t4
t2 --> t5
t4 --> t7
t5 --> t8
t7 --> t10
t8 --> t11 {dashed}
