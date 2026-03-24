## Config
title: Team Topology
flow = TB

## Nodes
- [stream1] Stream-Aligned Team A {rounded} {fill:#4a90d9} {bold}
  Owns the customer-facing product flow.
- [stream2] Stream-Aligned Team B {rounded} {fill:#4a90d9} {bold}
  Owns the data ingestion flow.
- [platform] Platform Team {server} {fill:#7b5ea7} {bold}
  Provides self-service infrastructure.
- [enabling] Enabling Team {rounded} {fill:#e8a838}
  Spreads best practices across streams.
- [subsystem] Complicated-Subsystem Team {rounded} {fill:#cc5a4a}
  Owns the payment processing engine.

## Flow
platform --> stream1: X-as-a-Service
platform --> stream2: X-as-a-Service
enabling --> stream1: facilitates
enabling --> stream2: facilitates
subsystem --> stream1: collaboration
