# AGENTS.md — Python

- `reference/` holds **independent reference models** used for differential
  testing of the Rust core (ADR-0010). Write them from the textbook/spec, not
  by reading the Rust code, so the two don't share mistakes. Standard library
  only; no numpy (exact float semantics must match IEEE double operations).
- Later: `pipeline/` (data pipeline, ADR-0012) and `calib/` (calibration,
  ADR-0009) managed with `uv`.
