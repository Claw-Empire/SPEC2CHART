## Config
title: Q1 OKRs
flow = TB

## Nodes
- [company] Company Mission {hexagon} {fill:#cba6f7} {bold}
  Empower teams to ship 10x faster.
- [obj_product] Product Excellence {rounded} {fill:#89b4fa} {bold}
  Deliver a delightful, reliable product.
- [obj_growth] Business Growth {rounded} {fill:#a6e3a1} {bold}
  Scale revenue and reach.
- [obj_ops] Operational Health {rounded} {fill:#f9e2af} {bold}
  Keep the lights on, sustainably.
- [kr_p1] Ship v2 launch {rounded} {fill:#74c7ec} {wip}
  Target: public GA by Q1 end.
- [kr_p2] NPS ≥ 50 {rounded} {fill:#74c7ec} {todo}
  Measure via in-app survey.
- [kr_p3] p99 < 200ms {rounded} {fill:#74c7ec} {done}
  API latency SLA.
- [kr_g1] ARR +40% {rounded} {fill:#a6e3a1} {wip}
  From $5M to $7M baseline.
- [kr_g2] 200 new customers {rounded} {fill:#a6e3a1} {wip}
  Paid seats, not trials.
- [kr_o1] 99.95% uptime {rounded} {fill:#f9e2af} {done}
  Rolling 30-day SLO.
- [kr_o2] MTTR < 15min {rounded} {fill:#f9e2af} {wip}
  Incident recovery time.

## Flow
company --> obj_product
company --> obj_growth
company --> obj_ops
obj_product --> kr_p1
obj_product --> kr_p2
obj_product --> kr_p3
obj_growth --> kr_g1
obj_growth --> kr_g2
obj_ops --> kr_o1
obj_ops --> kr_o2
