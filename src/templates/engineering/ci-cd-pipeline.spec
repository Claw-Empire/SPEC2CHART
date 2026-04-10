## Config
title: CI/CD Pipeline
flow = LR

## Nodes
- [commit] Commit Push {rounded} {fill:#74c7ec}
  Developer pushes to feature branch.
- [build] Build {rounded} {fill:#89b4fa}
  Compile + cache dependencies.
- [lint] Lint & Format {rounded} {fill:#89b4fa}
  Static analysis, style checks.
- [unit] Unit Tests {rounded} {fill:#89b4fa}
  Fast in-memory test suite.
- [security] Security Scan {rounded} {fill:#cba6f7}
  SAST + dependency audit.
- [quality_gate] Quality Gate {diamond} {fill:#f9e2af}
  All checks green?
- [artifact] Publish Artifact {rounded} {fill:#a6e3a1}
  Tag container, push registry.
- [deploy_stg] Deploy Staging {rounded} {fill:#a6e3a1}
  Rolling update to staging env.
- [smoke] Smoke Tests {rounded} {fill:#a6e3a1}
  Critical path + health checks.
- [approve] Manual Approval {diamond} {fill:#f9e2af}
  Release engineer sign-off.
- [deploy_prod] Deploy Production {rounded} {fill:#a6e3a1}
  Blue/green cutover.
- [monitor] Monitor & Alert {rounded} {fill:#a6e3a1}
  Watch SLOs, auto-rollback.
- [fail] Fail & Notify {rounded} {fill:#cc3333}
  Slack + revert branch.

## Flow
commit --> build: trigger
build --> lint: artifact
build --> unit: artifact
build --> security: artifact
lint --> quality_gate
unit --> quality_gate
security --> quality_gate
quality_gate --> artifact: pass
quality_gate --> fail: fail
artifact --> deploy_stg: promote
deploy_stg --> smoke
smoke --> approve: green
smoke --> fail: red
approve --> deploy_prod: approved
approve --> fail: rejected
deploy_prod --> monitor
monitor --> fail: SLO breach
