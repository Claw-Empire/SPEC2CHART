## Config
title: System Architecture
flow = LR

## Nodes
- [client] Web Client {rounded} {fill:#89b4fa}
  Browser SPA entry point.
- [cdn] CDN Edge {rounded} {fill:#a6e3a1} {ok}
  Static asset cache.
- [api] API Gateway {hexagon} {fill:#f9e2af} {bold}
  Request auth + routing.
- [svc1] Auth Service {rounded} {fill:#cba6f7} {ok}
  Session + JWT issuance.
- [svc2] Core Service {rounded} {fill:#cba6f7} {wip}
  Business logic layer.
- [queue] Event Queue {rounded} {fill:#f2cdcd}
  Async job stream.
- [db] Database {cylinder} {fill:#74c7ec}
  Primary PostgreSQL.
- [cache] Cache {cylinder} {fill:#f38ba8} {critical}
  Redis hot set.

## Flow
client --> cdn: static
client --> api: HTTPS
api --> svc1: authenticate
api --> svc2: process
svc2 --> db: read/write
svc2 --> cache: cache
svc2 --> queue: emit events
queue --> svc1: audit log
