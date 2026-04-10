## Config
title: Threat Model
flow = TB

## Nodes
- [user] User {person} {fill:#89b4fa} {ok}
  Legitimate end-user.
- [attacker] Attacker {person} {fill:#f38ba8} {critical}
  External threat actor.
- [browser] Browser {screen} {fill:#cba6f7}
  Client-side runtime.
- [waf] WAF {diamond} {fill:#f9e2af} {wip}
  Rate limit + filter rules.
- [drop] Dropped {rounded} {fill:#f38ba8} {critical}
  Blocked by policy.
- [app] Application {rounded} {fill:#a6e3a1}
  Business logic API.
- [secrets] Secrets Vault {cylinder} {fill:#b4befe} {ok}
  KMS-backed storage.
- [db] Database {cylinder} {fill:#74c7ec}
  Encrypted at rest.
- [audit] Audit Log {document} {fill:#cba6f7} {info}
  Tamper-evident trail.

## Flow
user --> browser: interacts
attacker --> browser: XSS attempt
attacker --> waf: scan probes
browser --> waf: HTTPS
waf --> app: allowed
waf --> drop: blocked
drop --> audit: flag alert
app --> secrets: fetch creds
app --> db: queries
app --> audit: logs events
