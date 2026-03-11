# Three-Tier Web Architecture (3D)

A classic three-tier web application shown across depth layers in 3D view.
Frontend sits closest to the viewer, the database furthest back,
with the API layer in between.

## Nodes

- [browser] Browser / Client {z:200}
  End user's browser or mobile app.

- [cdn] CDN {connector} {z:175}
  Content delivery network for static assets.

- [lb] Load Balancer {connector} {z:150}
  Distributes HTTP traffic across app servers.

- [app1] App Server 1 {z:100}
  Node.js REST API instance.

- [app2] App Server 2 {z:100}
  Node.js REST API instance.

- [cache] Redis Cache {connector} {z:50}
  In-memory cache. 24-hour TTL for session data.

- [db] PostgreSQL {z:0}
  Primary relational database. Read replicas in standby.

- [queue] Message Queue {connector} {z:50}
  Async job queue for background processing.

- [worker] Background Worker {z:0}
  Processes async jobs (emails, reports, webhooks).

## Flow

browser --> cdn --> lb
lb --> app1
lb --> app2
app1 --> cache --> db
app2 --> cache --> db
app1 --> queue --> worker
app2 --> queue --> worker

## Notes

- All inter-service traffic uses mTLS {blue}
- Database credentials injected via Vault at runtime {pink}
