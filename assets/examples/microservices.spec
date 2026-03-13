# Microservices Platform

Event-driven microservices with API gateway, domain services, and shared infra.
Use Tab to toggle 3D view. Keys 1-4 switch camera angle.

## Style
// Shared style templates
svc     = {service} {fill:purple}
db      = {database} {fill:blue}
queue   = {queue} {fill:yellow}
gateway = {connector} {fill:teal} {highlight}
client  = {user} {fill:green}

## Config
camera = iso
layer0 = Data
layer1 = Services
layer2 = Gateway
layer3 = Clients

## Layer 0: Data

- [pg_orders] Orders DB {db} {sublabel:PostgreSQL}
  Persistent order and inventory state.
- [pg_users] Users DB {db} {sublabel:PostgreSQL}
  Account, profile, and auth data.
- [redis] Cache {cache} {fill:red} {sublabel:Redis · TTL 5min}
  Hot-path query cache and session store.
- [kafka] Event Bus {queue} {sublabel:Kafka · 3 brokers} {highlight}
  Durable event stream for inter-service communication.

## Layer 1: Services

- [order_svc] Order Service {svc} {sublabel:v2.3}
  Places orders, manages inventory, publishes OrderCreated events.
- [user_svc] User Service {svc} {sublabel:v1.8}
  User CRUD, authentication, JWT issuance.
- [notify_svc] Notification Service {svc} {sublabel:v1.2}
  Consumes events and sends email/SMS/push.
- [search_svc] Search Service {svc} {sublabel:v3.0}
  Full-text product and order search via Elasticsearch.
- [auth_svc] Auth Service {connector} {fill:mauve} {sublabel:OAuth2 / JWT}
  Validates tokens, issues scoped credentials.

## Layer 2: Gateway

- [gw] API Gateway {gateway} {sublabel:Kong · /api/v2} {note:Single entry point for all external traffic}
  Rate limiting, auth, and routing to backend services.

## Layer 3: Clients

- [spa] Web App {client} {sublabel:React 18}
  Browser single-page application.
- [ios] iOS App {client} {sublabel:SwiftUI}
  Native iOS client.
- [android] Android App {client} {sublabel:Kotlin}
  Native Android client.

## Flow
// External traffic enters through the gateway
[spa, ios, android] -> gw

// Gateway validates and routes
gw -> auth_svc
gw -> [order_svc, user_svc, search_svc]

// Service → data dependencies
order_svc -> [pg_orders, redis, kafka]
user_svc -> [pg_users, redis]
search_svc -> redis

// Async event consumers
kafka -> notify_svc {dashed} {note:OrderCreated, PaymentFailed, etc.}

// Auth service integration
[order_svc, user_svc] -> auth_svc {dashed} {note:token validation}

## Notes
- All services run in Kubernetes (3 replicas) {ok}
- Kafka topics partitioned by user_id for ordering {info}
- Zero-downtime via rolling deploys + health checks {warning}
