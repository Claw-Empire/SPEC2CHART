## Config
title: Data Pipeline
flow = TB

## Nodes
- [src] Data Source {cylinder}
- [ingest] Ingestion {rounded}
- [transform] Transform {rounded}
- [validate] Validate {diamond}
- [store] Data Warehouse {cylinder}
- [serve] Serving Layer {rounded}
- [dlq] Dead Letter Queue {cylinder} {fill:#cc3333}
- [alert] Alert & Retry {rounded} {fill:#e8a838}

## Flow
src --> ingest: raw data
ingest --> transform: stream
transform --> validate: records
validate --> store: valid batch
validate --> dlq: invalid records
store --> serve: queries
dlq --> alert: notify ops
