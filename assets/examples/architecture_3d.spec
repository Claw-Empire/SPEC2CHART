# Cloud Architecture

A 3-layer cloud architecture diagram with database, application, and presentation tiers.
Use the 3D view (toggle with Tab) to see the layers separated in space.

## Config
bg = dots
view = 3d
camera_yaw = -0.4
camera_pitch = 0.6
layer0 = Data Tier
layer1 = App Tier
layer2 = Frontend

// Layer 0: Data tier (ground level)
## Layer 0: Data Tier
- [pg] PostgreSQL {circle} {fill:blue} {highlight} {sublabel:v15 · us-east-1}
  Primary relational database for user and order data.
- [redis] Redis Cache {circle} {fill:red} {sublabel:6.2 · cluster}
  In-memory cache for sessions and hot data.
- [s3] Object Store {parallelogram} {fill:yellow}
  File uploads and static assets.

// Layer 1: Application tier
## Layer 1: App Tier
- [api] REST API {connector} {highlight} {note:Entry point for all external traffic}
  Main backend service, handles all client requests.
- [worker] Background Worker {sublabel:3 replicas}
  Async job processing (email, reports, cleanup).
- [auth] Auth Service {connector} {fill:purple}
  JWT token issuance and validation.

// Layer 2: Presentation tier
## Layer 2: Frontend
- [web] Web App {parallelogram} {fill:teal} {sublabel:React 18}
  React single-page application.
- [mobile] Mobile App {parallelogram} {fill:teal} {sublabel:iOS + Android}
  iOS and Android clients.
- [cdn] CDN {hexagon} {note:Edge caching — 50+ PoPs globally}
  Edge caching for static assets.

## Flow
// Frontend clients
web -> api
mobile -> api

// REST API — sync calls
api -> [auth, pg]

// REST API — async + cached
api -> redis {note:session lookup}
api -> worker {dashed} {note:async jobs}

// Background worker
worker -> [pg, s3]

// CDN
cdn -> s3
web -> cdn
