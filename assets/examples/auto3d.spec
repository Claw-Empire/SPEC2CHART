# API Request Lifecycle

How a single HTTP request flows from browser to database and back.
Tab toggles 3D. Keys 1-4 switch camera angles.

## Style
frontend = {user} {fill:sky}
server   = {service} {fill:blue}
storage  = {database} {fill:purple}
cache    = {cache} {fill:red}
infra    = {server} {fill:teal}

## Config
view    = 3d
camera  = iso
auto-z  = true  // auto-assign z from flow topology
bg      = dots
title   = Request Lifecycle

## Nodes

- [browser]  Browser {frontend} {sublabel:React SPA}
  User's web browser. Sends fetch() requests.
- [cdn]      CDN {infra} {sublabel:Cloudflare}
  Serves static assets. Forwards API calls.
- [lb]       Load Balancer {infra} {sublabel:nginx}
  Round-robin to API servers. TLS termination.
- [api]      API Server {server} {sublabel:Express · 3 replicas} {highlight}
  Business logic: auth, validation, dispatch.
- [redis]    Session Cache {cache} {sublabel:Redis · TTL 30m}
  Validates session tokens in <1ms.
- [pg]       Postgres {storage} {sublabel:Primary DB}
  Authoritative data store. ACID transactions.
- [elastic]  Search Index {storage} {fill:orange} {sublabel:Elasticsearch}
  Full-text search over product catalogue.

## Flow
browser -> cdn  // static assets served from edge
cdn     -> lb   // API requests forwarded

lb -> api  // load balanced

api -> redis  {dashed}  // session lookup (hot path)
api -> pg               // data reads/writes
api -> elastic          // search queries

// Search re-indexer: async write-through
pg -> elastic {dashed} {note:async re-index on mutation}

## Notes
- P99 latency target: < 200ms end-to-end {ok}
- Postgres primary-replica lag: < 10ms {info}
- CDN cache hit rate: 94% {ok}
- Redis eviction policy: allkeys-lru {info}
