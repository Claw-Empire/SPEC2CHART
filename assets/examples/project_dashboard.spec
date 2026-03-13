# Product Launch Dashboard

Q2 feature delivery — track progress across frontend, backend, and infra.
Use Tab to toggle 3D view. Keys 1-4 switch camera. Cmd+L to auto-layout.

## Style
// Reusable style templates
done      = {fill:green} {ok}
active    = {fill:blue} {highlight}
blocked   = {fill:red} {warning}
pending   = {fill:gray}
infra_svc = {server} {fill:teal}
data_svc  = {database} {fill:purple}

## Config
camera   = iso
view     = 3d
bg-color = #1e1e2e
bg       = dots
title    = Q2 Dashboard
layer0   = Infrastructure
layer1   = Backend
layer2   = Frontend

## Layer 0: Infrastructure

- [db_main]  Primary DB {data_svc} {sublabel:PostgreSQL 15} {progress:100}
  Production database with read replicas.
- [db_cache] Redis Cache {cache} {fill:red} {sublabel:Redis 7 · TTL 5m} {progress:80}
  Hot-path cache layer with TTL expiry.
- [queue]    Message Queue {queue} {fill:yellow} {sublabel:RabbitMQ} {progress:100}
  Async task dispatch and event streaming.
- [cdn]      CDN {cloud} {fill:sky} {sublabel:Cloudflare} {progress:100}
  Global content delivery and DDoS protection.

## Layer 1: Backend

- [api_gw]   API Gateway {connector} {fill:teal} {sublabel:Kong · v2} {progress:90} {highlight}
  Rate-limiting, auth, and request routing.
- [auth_svc] Auth Service {service} {sublabel:OAuth2 / JWT} {progress:100} {done}
  Token issuance, scoped access, session management.
- [user_svc] User Service {service} {sublabel:v3.1} {progress:85} {active}
  Profile CRUD, preferences, and notification settings.
- [billing]  Billing Service {service} {sublabel:Stripe · v4} {progress:60} {active}
  Subscription billing, invoice generation.
- [search]   Search Service {service} {sublabel:Elastic 8} {progress:40} {active}
  Full-text search with facets and scoring.
- [notif]    Notifications {service} {sublabel:v1.5} {progress:75} {active}
  Email, SMS, and push notification dispatch.

## Layer 2: Frontend

- [web]      Web App {user} {sublabel:React 18 · Vite} {progress:70} {active}
  Main SPA — dashboard, settings, analytics.
- [mobile]   Mobile App {user} {sublabel:React Native} {progress:45} {active}
  iOS + Android client with offline sync.
- [admin]    Admin Panel {user} {sublabel:Next.js} {progress:90} {active}
  Internal tooling for ops and CS teams.
- [docs]     Docs Site {text} {fill:lavender} {sublabel:Docusaurus} {progress:55} {pending}
  Developer documentation and API reference.

## Flow
// External clients hit the CDN, then gateway
[web, mobile] -> cdn  // traffic ingress
cdn -> api_gw

// Gateway validates every request
api_gw -> auth_svc  // token validation

// Gateway routes to services
api_gw -> [user_svc, billing, search, notif]  // service dispatch

// Service dependencies
user_svc -> [db_main, db_cache]  // user reads/writes
billing  -> [db_main, queue]     // billing events
search   -> db_cache             // search cache
notif    -> queue {dashed}       // async dispatch

// Auth service shared across backend
[user_svc, billing] -> auth_svc {dashed}  // token validation

// Admin panel goes direct
admin -> api_gw

## Notes
- Frontend sprint velocity: +15% this week {ok}
- Billing service gated on compliance review — unblocked Thu {warning}
- Search re-index job scheduled this weekend {info}
- Mobile offline sync de-scoped to Q3 {warning}
