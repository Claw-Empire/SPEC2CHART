## Config
title: Team Topology
flow = TB

## Nodes
- [org] Engineering Org {hexagon} {fill:#b4befe} {bold}
  Tech organization root.

- [streams] Stream-Aligned {rounded} {fill:#89b4fa} {bold}
  Customer-facing delivery.
- [support] Support Teams {rounded} {fill:#cba6f7} {bold}
  Shared capabilities.

- [stream1] Team A {rounded} {fill:#4a90d9}
  Customer portal flow.
- [stream2] Team B {rounded} {fill:#4a90d9}
  Data ingestion flow.

- [platform] Platform {rounded} {fill:#7b5ea7}
  Self-service infra.
- [enabling] Enabling {rounded} {fill:#e8a838}
  Best practices coach.
- [subsystem] Subsystem {rounded} {fill:#cc5a4a}
  Payment engine.

## Flow
org --> streams: delivers
org --> support: sustains
streams --> stream1: delivery
streams --> stream2: delivery
support --> platform: hosts
support --> enabling: coaches
support --> subsystem: owns
platform --> stream1: X-as-a-Service
platform --> stream2: X-as-a-Service
enabling --> stream1: facilitates
subsystem --> stream2: collaboration
