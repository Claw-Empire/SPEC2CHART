# Microservice Architecture

A backend system showing three services communicating through
explicitly modeled connection interfaces. Each connection node
describes the protocol, authentication, and rate limits.

## Nodes

- [user_svc] User Service
  Manages authentication and user profiles.
  Owns the users and sessions tables.

- [rest] REST API {connector}
  HTTP/JSON over HTTPS (TLS 1.3).
  Auth: Bearer token (JWT, 15min expiry).
  Rate limit: 1000 req/min per client.

- [order_svc] Order Service
  Processes orders and manages inventory.
  Publishes events on order state changes.

- [grpc] gRPC Channel {connector}
  Protocol Buffers over HTTP/2.
  Bi-directional streaming enabled.
  Auth: mTLS certificates.

- [payment_svc] Payment Service
  Stripe integration for payment processing.
  PCI-DSS compliant. All card data tokenized.

- [event_bus] Event Bus {connector}
  Apache Kafka. Topic: order-events.
  Retention: 7 days. Partitions: 12.
  Consumers use consumer groups.

- [notification_svc] Notification Service
  Sends email and push notifications.
  Uses SendGrid and Firebase Cloud Messaging.

## Flow

user_svc --> rest --> order_svc
order_svc --> grpc --> payment_svc
order_svc --> event_bus --> notification_svc

## Notes

- All internal traffic stays within VPC {blue}
- Deploy each service independently via Docker {green}
- Monitor with Prometheus + Grafana {yellow}
