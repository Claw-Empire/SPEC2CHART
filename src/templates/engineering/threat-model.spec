## Config
title: Threat Model
flow = TB

## Nodes
- [user] User {person}
- [browser] Browser {screen}
- [waf] WAF {diamond}
- [app] Application {rounded}
- [db] Database {cylinder}
- [attacker] Attacker {person} {fill:#cc3333}

## Flow
user --> browser: interacts
browser --> waf: HTTPS
waf --> app: filtered
app --> db: queries
attacker --> browser: XSS attempt
attacker --> app: injection attempt
