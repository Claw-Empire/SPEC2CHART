## Config
title: Data Pipeline
flow = LR

## Nodes
- [src] Data Source {cylinder}
- [ingest] Ingestion {rounded}
- [transform] Transform {rounded}
- [validate] Validate {rounded}
- [store] Data Warehouse {cylinder}
- [serve] Serving Layer {rounded}

## Flow
src --> ingest: raw data
ingest --> transform: stream
transform --> validate: records
validate --> store: batch
store --> serve: queries
