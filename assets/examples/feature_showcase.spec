# Feature Showcase

Demonstrates modern spec syntax: Unicode arrows, pipe labels, colon labels,
named tier tags, grid layout, style templates, and inline comments.
Switch to 3D view (Tab) to see the layered architecture.

## Config
view    = 3d
camera  = iso
bg      = dots
title   = Feature Showcase
auto-z  = false  // z-offsets explicitly set via {layer:name} tags

## Style
// Reusable style templates — use {name} in any node line
frontend = {fill:sky}    {layer:frontend}
backend  = {fill:blue}   {layer:api}
storage  = {fill:purple} {layer:db}
edge_svc = {fill:teal}   {layer:edge}

## Nodes

- [browser]  Browser      {user}    {frontend}  {sublabel:React SPA}
  User-facing single-page app. Sends fetch() requests.

- [mobile]   Mobile App   {user}    {frontend}  {sublabel:iOS · Android}
  Native clients. Shares REST API with browser.

- [gateway]  API Gateway  {service} {edge_svc}  {sublabel:nginx · TLS}
  TLS termination, rate limiting, routing.  // single entry point

- [api]      API Server   {service} {backend}   {sublabel:Go · 4 replicas} {highlight}
  Business logic: auth, validation, dispatch.

- [auth]     Auth Service {service} {backend}   {sublabel:JWT · OAuth2}
  Token issuance and verification.

- [pg]       Postgres     {database} {storage}  {sublabel:Primary DB}
  Authoritative data store. ACID transactions.

- [redis]    Redis Cache  {cache}   {storage}   {sublabel:TTL 30m}
  Session tokens and hot query results.

## Flow

// Pipe-label syntax (Mermaid-style):
browser →|HTTPS requests| gateway
mobile  →|HTTPS requests| gateway

// Colon-label suffix syntax:
gateway → api: routes request
api → auth: verify token {dashed}

// Standard quoted-label prefix syntax:
api "reads / writes" → pg
api "cache lookup" → redis {dashed}

// Reverse arrow:
pg ← redis {dashed}  // write-through cache invalidation

## Grid cols=3
// Progress comparison — grid layout with 3 columns
- [f1] Auth Flow       {fill:blue}   {progress:100} {ok}
- [f2] Search API      {fill:blue}   {progress:80}  {info}
- [f3] Analytics       {fill:purple} {progress:60}
- [f4] Mobile Push     {fill:sky}    {progress:40}  {warning}
- [f5] Data Export     {fill:purple} {progress:20}
- [f6] Admin Panel     {fill:teal}   {progress:0}   {critical}

## Notes
- P99 latency: < 120ms end-to-end {ok}
- Auth cache hit rate: 98% {ok}
- Postgres replica lag: < 5ms {info}
- Mobile push queue depth: elevated {warning}
