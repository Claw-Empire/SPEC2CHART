## Config
title: Data Pipeline
flow = TB

## Nodes
- [src] Data Source {cylinder} {fill:#74c7ec}
  Upstream events + CDC.
- [ingest] Ingestion {rounded} {fill:#89b4fa}
  Kafka consumer group.
- [transform] Transform {rounded} {fill:#cba6f7}
  Dedupe + enrich.
- [validate] Validate {diamond} {fill:#f9e2af}
  Schema + quality gate.
- [store] Data Warehouse {cylinder} {fill:#a6e3a1}
  Partitioned columnar.
- [serve] Serving Layer {rounded} {fill:#a6e3a1}
  BI + feature API.
- [dlq] Dead Letter Queue {cylinder} {fill:#cc3333}
  Quarantined records.
- [alert] Alert & Retry {rounded} {fill:#e8a838}
  Ops notification.

## Flow
src --> ingest: raw data
ingest --> transform: stream
transform --> validate: records
validate --> store: valid batch
validate --> dlq: invalid records
store --> serve: queries
dlq --> alert: notify ops
