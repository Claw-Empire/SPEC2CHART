# Cloud Architecture

Three-tier cloud application with load balancing, microservices, and data layer.

## Layer 0: Data Tier

- [pg] PostgreSQL {database} {sublabel:v15 · us-east-1} {tooltip:Primary relational store for users and orders}
- [redis] Redis Cache {cache} {sublabel:TTL 5min} {tooltip:Session data and hot-path query cache}
- [s3] Object Store {storage} {sublabel:S3 · us-east-1} {tooltip:User uploads, assets, and backups}

## Layer 1: API Tier

- [lb] Load Balancer {load-balancer} {sublabel:ALB · round-robin} {tooltip:Routes traffic across API instances}
- [auth] Auth Service {service} {sublabel:OAuth2 / JWT} {tooltip:Token validation and issuance}
- [api] REST API {service} {sublabel:v2 · /api/v2} {tooltip:Core business logic}
- [worker] Background Worker {service} {sublabel:Celery + Beat} {tooltip:Async tasks and scheduled jobs}
- [mq] Message Queue {queue} {sublabel:RabbitMQ} {tooltip:Decouples API from workers}

## Layer 2: Frontend

- [cdn] CDN {cloud} {sublabel:CloudFront} {tooltip:Static asset delivery}
- [web] Web App {service} {sublabel:React SPA} {tooltip:Served from CDN}
- [mobile] Mobile App {user} {sublabel:iOS / Android} {tooltip:Native mobile clients}

## Flow

internet -> lb {thick} {note:All external traffic enters here}
lb -> auth
lb -> api
api -> pg
api -> redis
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
