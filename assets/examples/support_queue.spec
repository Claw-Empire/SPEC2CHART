# Customer Support Queue

Live kanban for active support tickets. 🔥 = P1 overdue · ⚠ = overdue
⇧E escalate (P1+move to Triage) · ⇧R resolve · C comment · A assign · Z snooze (+1 day)
⌃1/2/3/4 jump to section · ⌘⇧I new intake ticket · Alt+P sort by priority
Hover ticket 0.8s for SLA detail · ⌘⇧W workload panel · ] / [ advance/demote column

## Config
flow = LR
spacing = 80
sla-p1 = 1
sla-p2 = 3
sla-p3 = 7
sla-p4 = 14

## Intake
- [t1] Login page 500 errors {p1} {glow} {critical}
  {assigned:Alice} {due:2026-03-15} {created:2026-03-13}
  {comment:Spike at 14:20 UTC — auth service pod OOMKilled. Alice investigating JWT library upgrade.}
  Auth service returning 500 on POST /login. Affects 100% of new sessions.
- [t2] CSV export drops last column {p2}
  {assigned:Bob} {due:2026-03-17} {created:2026-03-15}
  {comment:Reproduced on Firefox + Chrome. Only triggers when row count exceeds 500.}
  CSV export omits last column when >500 rows. Finance team blocked on month-end reports.
- [t3] Dark mode flash on iOS navigation {p3}
  {assigned:Carol} {due:2026-03-23} {created:2026-03-16}
  Background flashes white for ~80ms on route push. Cosmetic but high visibility.

## Triage
- [t4] Payment gateway 3DS timeout {p1} {urgent} {glow} {critical}
  {assigned:Alice} {due:2026-03-14} {created:2026-03-11}
  {comment:Stripe dashboard shows 42% webhook failure rate. Escalated to Stripe support — ticket #STR-88421. Alice on call.}
  Stripe webhook not firing for 3DS payments — 40% checkout failure rate since 09:00.
- [t5] Duplicate email notifications {p2}
  {assigned:Bob} {due:2026-03-19} {created:2026-03-13}
  {comment:Seems tied to the SES retry logic. Bob found duplicate enqueue in event consumer.}
  Users receiving 2–4 duplicate alert emails per event. Unsubscribe rate up 8%.
- [t6] Monthly PDF report blank pages {p3}
  {assigned:Dana} {due:2026-03-24} {created:2026-03-14}
  {comment:Only reproducible when report spans >12 pages. Likely a page-break calculation bug in wkhtmltopdf.}
  Intermittent blank pages on reports > 12 pages. Affects enterprise tier customers.

## In Progress
- [t7] Full-text search stale results {p1}
  {assigned:Carol} {due:2026-03-17} {created:2026-03-10}
  {comment:Elasticsearch index refresh interval set to 30s — should be 1s. Carol deploying config change now.}
  Search misses content indexed in last 30 minutes. Enterprise SLA breach if not resolved today.
- [t8] Password reset email delay {p2}
  {assigned:Bob} {due:2026-03-20} {created:2026-03-12}
  Reset emails delayed 10–15 min. Root cause: SES queue depth spike during batch sends.
- [t9] API rate limit too aggressive on paid tier {p3}
  {assigned:Dana} {due:2026-03-25} {created:2026-03-14}
  /v2/items throttles at 10 req/s for paid tier users. Contracted limit is 100 req/s.

## Resolved
- [t10] Mobile push notifications silent {p2} {done}
  {assigned:Carol} {due:2026-03-15} {created:2026-03-09}
  FCM topic subscription bug — deployed fix v3.4.2 on 2026-03-14.
- [t11] Chart tooltip renders off-screen {p4} {done}
  {assigned:Dana} {due:2026-03-14} {created:2026-03-12}
  z-index + clamp() fix applied. Verified on all breakpoints.
- [t12] Timezone wrong for DST regions {p3} {done}
  {assigned:Bob} {due:2026-03-13} {created:2026-03-10}
  UTC offset calculation corrected for DST transition. Deployed v3.4.1.

## Flow
t1 --> t4 {escalate}
t2 --> t5
t4 --> t7 {escalate}
t5 --> t8
t7 --> t10 {resolves}
t8 --> t11 {resolves}
t4 --> t1 {blocks}
