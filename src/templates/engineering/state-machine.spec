## Config
title: Payment State Machine
flow = LR

## Nodes
- [new] New {circle} {fill:#89b4fa}
  Payment intent just created.
- [pending] Pending {rounded} {fill:#f9e2af}
  Awaiting gateway response.
- [authorize] Authorize? {diamond} {fill:#f9e2af}
  Gateway decision.
- [authorized] Authorized {rounded} {fill:#a6e3a1}
  Funds held, not yet captured.
- [capture] Capture? {diamond} {fill:#f9e2af}
  Merchant capture decision.
- [captured] Captured {rounded} {fill:#a6e3a1}
  Funds transferred to merchant.
- [settled] Settled {rounded} {fill:#a6e3a1} {ok}
  End-of-day batch settled.
- [refund_req] Refund Requested {rounded} {fill:#f9e2af}
  Customer or merchant initiated.
- [refunded] Refunded {rounded} {fill:#cba6f7}
  Funds returned to customer.
- [declined] Declined {rounded} {fill:#cc3333}
  Gateway rejected authorization.
- [voided] Voided {rounded} {fill:#cba6f7}
  Authorization released before capture.
- [failed] Failed {circle} {fill:#cc3333}
  Terminal failure state.

## Flow
new --> pending: submit
pending --> authorize: gateway responds
authorize --> authorized: approved
authorize --> declined: rejected
authorized --> capture: merchant action
authorized --> voided: timeout or cancel
capture --> captured: capture API
capture --> voided: abandon
captured --> settled: batch close
captured --> refund_req: refund requested
settled --> refund_req: refund requested
refund_req --> refunded: gateway refund ok
refund_req --> failed: gateway error
declined --> failed: terminal
voided --> failed: terminal
