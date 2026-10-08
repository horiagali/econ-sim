---
id: roadmap
title: Roadmap
status: draft
owner: horia
depends_on: [vision/scope]
updated: 2026-10-08
---

# Roadmap

## Phase 0 — Design & research *(now)*
- [ ] Finish P0 research (modelling approach, competitor teardown, tick length).
- [ ] ADR: modelling approach.
- [ ] ADR: tech stack.
- [ ] Draft all MVP economy specs to `draft`.
- [ ] Define accounting matrices (`economy/accounting`) to `review`.

## Phase 1 — Headless simulation prototype
- [ ] Simulation core runs from config, no UI; outputs CSV/plots.
- [ ] Automated acceptance tests from each spec.
- [ ] SFC consistency check every tick.
- [ ] Policy-scenario test suite (rate hike, tax cut, deficit spending, devaluation, oil shock).

## Phase 2 — Playable vertical slice
- [ ] Minimal UI: dashboard, levers, time controls.
- [ ] "Why did this change" causal breakdown.
- [ ] Save/load.

## Phase 3 — Depth
- [ ] More industries, income cohorts, multiple countries.
- [ ] Politics layer.
- [ ] Military layer.
