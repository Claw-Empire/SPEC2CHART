# Cloud Architecture

Three-tier cloud application with load balancing, microservices, and data layer.

## Layer 0: Data Tier

- [pg] PostgreSQL {database} {tooltip:Primary relational store for users and orders}
- [redis] Redis Cache {cache} {tooltip:Session data and hot-path query cache}
- [s3] Object Store {storage} {tooltip:User uploads, assets, and backups}

## Layer 1: API Tier

- [lb] Load Balancer {load-balancer} {tooltip:Routes traffic across API instances}
- [auth] Auth Service {service} {tooltip:OAuth2 / JWT token validation}
- [api] REST API {service} {tooltip:Core business logic}
- [worker] Background Worker {service} {tooltip:Async tasks and scheduled jobs}
- [mq] Message Queue {queue} {tooltip:Decouples API from workers (RabbitMQ)}

## Layer 2: Frontend

- [cdn] CDN {cloud} {tooltip:Static asset delivery — CloudFront}
- [web] Web App {service} {tooltip:React SPA served from CDN}
- [mobile] Mobile App {user} {tooltip:iOS / Android clients}

## Flow

internet -> lb {thick}
lb -> auth
lb -> api
api -> pg
api -> redis
api -> mq {dashed}
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
