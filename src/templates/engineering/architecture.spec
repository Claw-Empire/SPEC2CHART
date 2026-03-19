## Config
title: System Architecture
flow = LR

## Nodes
- [client] Web Client {rounded}
- [api] API Gateway {rounded}
- [svc1] Auth Service {rounded}
- [svc2] Core Service {rounded}
- [db] Database {cylinder}
- [cache] Cache {cylinder}

## Flow
client --> api: HTTPS
api --> svc1: authenticate
api --> svc2: process
svc2 --> db: read/write
svc2 --> cache: cache
