# Cloud Architecture

Three-tier cloud application with load balancing, microservices, and data layer.

## Config
bg = dots
flow = TB
layer0 = Data Tier
layer1 = API Tier
layer2 = Frontend

## Layer 0: Data Tier

- [pg] PostgreSQL {database} {sublabel:v15 · us-east-1} {highlight}
  Primary relational store for users and orders.
- [redis] Redis Cache {cache} {sublabel:TTL 5min}
  Session data and hot-path query cache.
- [s3] Object Store {storage} {sublabel:S3 · us-east-1}
  User uploads, assets, and backups.

## Layer 1: API Tier

- [lb] Load Balancer {load-balancer} {sublabel:ALB · round-robin} {highlight}
  Routes traffic across API instances.
- [auth] Auth Service {service} {sublabel:OAuth2 / JWT}
  Token validation and issuance.
- [api] REST API {service} {sublabel:v2 · /api/v2} {note:Entry point for all external traffic}
  Core business logic and routing.
- [worker] Background Worker {service} {sublabel:Celery + Beat}
  Async tasks and scheduled jobs.
- [mq] Message Queue {queue} {sublabel:RabbitMQ}
  Decouples API from workers.

## Layer 2: Frontend

- [cdn] CDN {cloud} {sublabel:CloudFront} {note:Edge caching — 50+ PoPs globally}
  Static asset delivery at the edge.
- [web] Web App {service} {sublabel:React SPA}
  Single-page application served from CDN.
- [mobile] Mobile App {user} {sublabel:iOS / Android}
  Native mobile clients.

## Flow

internet -> lb {thick} {note:All external traffic enters here}
lb -> auth
lb -> api
api -> pg
api -> redis {note:session lookup}
api -> mq {dashed} {note:async publish}
mq -> worker
worker -> pg
worker -> s3 {dashed}
api -> s3 {dashed}
cdn -> web
mobile -> lb
web -> lb

## Notes

- Deployed on AWS (us-east-1 + eu-west-1) {blue}
- All services containerized with Docker/K8s {ok}
- Redis TTL = 5min for hot queries {info}
- Zero-downtime deploys via rolling updates {warning}
