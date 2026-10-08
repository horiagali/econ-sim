---
id: architecture/overview
title: Architecture — Overview
status: stub
owner: horia
depends_on: [economy/overview]
updated: 2026-10-08
---

# Architecture

To be filled once the modelling approach and tech stack ADRs are accepted. Decisions live in [`decisions/`](decisions/).

## Working assumptions (not yet decided)
- **Headless simulation core** separate from UI, so it can be tested and run in batch by agents.
- **Deterministic** given a seed — needed for reproducible tests and debugging.
- **Data-driven parameters** (YAML/JSON) so tuning doesn't require code changes.
- **Mechanics as modules** mirroring `docs/02-design/` one-to-one, each with tests derived from its spec's acceptance tests.
- **Causal trace**: each tick records contributions to key variables to power the "why" UI.

## Open questions
- [ ] Language/engine: Python prototype first, then port? TypeScript for web? Godot / Unity?
- [ ] Where does the simulation run (browser, desktop)?
- [ ] State representation: arrays/ECS vs object graph.
