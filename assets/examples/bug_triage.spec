# Bug Triage Process

Map the journey of a bug report from initial submission through triage, fix, verification, and release.
Use this to standardize how your team handles incoming defects.

## Config
flow = LR
spacing = 80

## Intake
- [report] Bug reported {icon:🐛} {wip}
  Source: user report / QA / monitoring / Sentry alert
- [dup_check] Duplicate? {diamond} {icon:🔍}
- [merged] Merged into existing issue {icon:🔗} {ok}
  Link the duplicate and close

## Triage
- [triage] Triage meeting or async review {icon:👥} {wip}
  Happens daily — on-call + QA + PM
- [severity] Assess severity & priority {diamond} {icon:⚖️}
- [sev_critical] SEV-1 — Production broken {critical} {icon:🔴} {glow}
  Hotfix required — skip sprint queue
- [sev_high] SEV-2 — Major feature broken {warning} {icon:🟠}
  Fix in current sprint
- [sev_medium] SEV-3 — Minor impact {info} {icon:🟡}
  Add to backlog, next sprint candidate
- [sev_low] SEV-4 — Cosmetic / edge case {ok} {icon:🟢}
  Label: low-priority, pick up when bandwidth allows

## Investigation
- [assign] Assign to engineer {icon:👤} {wip}
- [reproduce] Reproduce the bug {icon:🔬} {wip}
  Required before any fix — add repro steps to issue
- [root_cause] Root cause identified {hypothesis} {icon:💡}
  Document in issue: what changed, what broke

## Fix
- [fix_branch] Create fix branch {icon:🌿} {wip}
  Naming: fix/<issue-id>-short-description
- [code_fix] Implement fix {icon:🔧} {wip}
- [unit_test] Add regression test {icon:🧪} {todo}
  Prevent reoccurrence — required for SEV-1/2
- [pr] Open pull request {icon:📬} {wip}
- [code_review] Code review {icon:👁} {wip}
  At least 1 approval required

## Verification
- [qa_verify] QA verifies fix in staging {icon:✅} {wip}
- [regression] Run regression suite {icon:🔄} {wip}
- [deploy] Deploy to production {icon:🚀} {todo}
- [monitor] Monitor for 30 min post-deploy {icon:📊} {todo}
  Watch error rate, latency, Sentry

## Closure
- [close] Close issue {icon:🎯} {done} {glow}
  Add: fix version, root cause summary, prevention note
- [postmortem] Postmortem (SEV-1/2 only) {icon:📝} {todo}
  Due within 5 business days

## Flow
report --> dup_check
dup_check --> merged: yes
dup_check --> triage: no
triage --> severity
severity --> sev_critical: P0
severity --> sev_high: P1
severity --> sev_medium: P2
severity --> sev_low: P3
sev_critical --> assign: hotfix
sev_high --> assign
sev_medium --> assign
sev_low --> assign
assign --> reproduce
reproduce --> root_cause
root_cause --> fix_branch
fix_branch --> code_fix
code_fix --> unit_test
unit_test --> pr
pr --> code_review
code_review --> qa_verify
qa_verify --> regression
regression --> deploy
deploy --> monitor
monitor --> close
close --> postmortem: SEV-1/2
