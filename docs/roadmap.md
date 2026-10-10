---
id: roadmap
title: Roadmap
status: draft
owner: horia
depends_on: [vision/scope]
updated: 2026-10-10
---

# Roadmap

## Phase 0 — Design & research *(now)*
- [x] v1 game definition: vision, scope, game, society and economy specs at `draft` (2026-10-08).
- [ ] Owner review pass: answer open questions in each spec.
- [ ] Finish P0 research (modelling approach, competitor teardown, tick length). Population and tech-stack research done ([population](01-research/notes/population-modelling-deep-research.md), [tech stack](01-research/notes/tech-stack-deep-research.md)).
- [x] ADR: modelling approach (ADR-0002, hybrid SFC + synthetic population + I-O industries; accepted 2026-10-09).
- [x] ADR: tech stack drafted as ADR-0004 to ADR-0015, status `proposed` (2026-10-09). See the [architecture overview](03-architecture/README.md).
- [x] Owner accepts (or amends) ADR-0004 to ADR-0015 (accepted 2026-10-09).
- [ ] Data plan: Romania calibration pack and synthetic population pipeline from the IPUMS 2011 seed reweighted to 2021 census totals ([ADR-0012](03-architecture/decisions/0012-data-pipeline-and-licensing.md)).
- [x] v1 additions specified: EU membership and funds, informal economy, foreign ownership, housing, environment, state capacity (2026-10-08).
- [x] ADR: people representation (ADR-0003, accepted 2026-10-09; 1:100 default, scale configurable and never hardcoded).
- [x] **Firm table:** [ADR-0016](03-architecture/decisions/0016-firm-representation.md) (accepted 2026-10-09) replaces "one aggregate firm per industry" with firm units: 0–5 named firms plus size-class cohort firms per industry, with one ownership model. Specs updated (production, industries, investment-capital, money-banking, foreign-ownership, state-enterprises, accounting).
- [ ] Move accounting, population-groups, production and households to `review`.

### Legal and admin track
This is a personal, non-commercial project, so data-terms checks are not needed ([ADR-0012](03-architecture/decisions/0012-data-pipeline-and-licensing.md)).
- [ ] Optional: check BeforeIT.jl licence before copying any code (reading papers is fine).
- [x] GitHub Free with hooks + CI checks ([ADR-0014](03-architecture/decisions/0014-agent-workflow-guardrails.md)).

## Phase 1 — Technical spikes, then the headless simulation prototype
Spikes are throwaway-quality prototypes that settle the decisions hardest to change later. Spikes 1–3 come first; 4–6 can overlap; the UI comes late because the core's API defines what it shows. Starting Phase 1 needs the owner's go-ahead (AGENTS.md forbids game code in Phase 0).

| # | Spike | Exit criterion | Settles |
|---|---|---|---|
| 0 ✅ (`just check` green on the Windows laptop and hook blocks edits, 2026-10-10) | Repo hygiene: `@AGENTS.md` import, justfile, toolchain pin, empty workspace, cargo-deny, CI skeleton, PreToolUse hook | An agent on Windows runs `just check` green and is blocked from editing `tests/golden/` | ADR-0014, 0010 |
| 1 ✅ (Windows laptop, Windows CI and Linux CI hashes all match, 2026-10-10) | Numeric foundation: `Bani`, rounding, splits, keyed ChaCha8 + known-answer vectors, `det_sum`, `libm` wrappers, clippy deny config | Same hashes on Windows laptop and Linux CI; clippy rejects `f64::exp` and `HashMap` in core | ADR-0006, 0007 |
| 2 ✅ (PC model pending) | Ledger + differential SIM: typed transfers, invariants I-1 to I-7, core "SIM mode", independent Python SIM then PC | Exact match in bani for 200 ticks; a mutated posting sign is caught by the differential test | ADR-0007, 0010 |
| 3 ✅ | Explainability: `behaviour_rule!` on price, wage and consumption rules; `explain()` tree; Σ = Δ and μ tests | A nested tree sums exactly; log-linear aggregation question settled | ADR-0008, 0009 (part) |
| 4 ✅ 6.4 ms/tick at 1:100 (9.9 ms on the Windows laptop with VAT wired in, 2026-10-10) | Performance and market clearing at variable scale: fake weighted population, labour and goods matching, 80×80 LU, cube | Tick time at 1:1000, 1:100, 1:10; 50-year run ≤ ~30 s at 1:100; 3-scale test passes | ADR-0003, 0005, 0011 |
| 5 ✅ ~36k runs/h at 1:1000 (~27k on the Windows laptop with VAT wired in) | Calibration loop: `econ-py`, 600-tick runs from Python, Morris on ~10 parameters, one history-matching wave, 100-year quiet baseline | Runs per hour measured; full calibration budget known | ADR-0009 |
| 6 ✅ 1.7 MB at 1:100 | Saves and replay: ZIP + manifest + Arrow tables + command log, one migration, history block | Load, replay and state-hash check pass; save size at 1:100 and 1:10 recorded | ADR-0011 |
| 7 ✅ offline slice; live Eurostat fetch verified 2026-10-10 | Data pipeline slice: glossary codegen, Eurostat fetch with provenance tags, DVC with a private remote, population for 2–3 counties from the IPUMS seed via IPF | Scenario builds end to end; provenance listed in the report; no raw restricted microdata tracked by Git | ADR-0012, 0003 |
| 8 ✅ 5 MB IPC round trip 33 ms, cold start 1.4 s, ~310 MB (see "Windows verification" in the [spikes 5–9 results](03-architecture/spikes/spikes-5-6-results.md)) | UI slice: Tauri 2 + React, 40 uPlot charts × 600 months, ECharts county map from geoBoundaries, one nested "why" tooltip | 1–5 MB IPC round trip measured on WebView2; cold start and memory acceptable | ADR-0013 |
| 9 ✅ single-session (two-PR flow needs GitHub) | Agent workflow dry run: one real mechanic (e.g. VAT) through the tests-first two-PR flow with traceability and `cargo-mutants --in-diff` | Spec → tests → implementation merged with no hand edits to protected paths; friction logged | ADR-0010, 0014 |

Results: [spikes 0–4](03-architecture/spikes/spikes-0-4-results.md), [spikes 5–6](03-architecture/spikes/spikes-5-6-results.md).

## Phase 1b — headless prototype
The spikes are done; this phase turns them into a simulation core that runs from a scenario with no UI. (Phase 2 stays the playable vertical slice.)

- [x] Follow-ups from the Windows verification (2026-10-10): fast RNG switched to the "gamma" mixer ([ADR-0006](03-architecture/decisions/0006-determinism-contract.md) Amendment 1), `tax.vat` wired into the scale world through the ledger, Tauri host checked in Windows CI.
- [ ] Simulation core runs from a scenario, no UI; outputs Arrow/CSV and plots.
- [ ] Automated acceptance tests from each spec (tests-first two-PR flow).
- [ ] SFC invariants I-1 to I-7 every tick ([ADR-0007](03-architecture/decisions/0007-money-and-ledger.md)).
- [ ] Person-conservation check every tick.
- [ ] CI runs the tiny scenario at three population scales.
- [ ] Policy-scenario test suite (rate hike, tax cut, deficit spending, devaluation, oil shock, student-grant rise, power-plant build).

### Owner decisions for Phase 1b (2026-10-10)
Recorded here so they are not lost; none of this is implemented yet.

**Spec lock order.** Only the owner moves a spec to `locked`. Every spec gets an "API sketch" section before it locks (Spike 9 lesson: tests-first means the test writer picks the API).
1. [`society/population-groups`](02-design/society/population-groups.md) and [`economy/accounting`](02-design/economy/accounting.md).
2. VAT: split it out of [`economy/taxation`](02-design/economy/taxation.md) into its own spec file and lock that.
3. [`economy/households`](02-design/economy/households.md) and [`economy/labor-market`](02-design/economy/labor-market.md), then the rest of taxation (income tax, social contributions).
4. Firms and [production](02-design/economy/production.md), after the firm-unit goods market exists.

**Data** (keeps [ADR-0012](03-architecture/decisions/0012-data-pipeline-and-licensing.md)).
- The IPUMS 2011 seed sample lives in the private DVC store, never in git.
- Census 2021 tables (INS) and EU-SILC aggregates are the IPF margins.
- The owner applies for IPUMS International access and writes to INS in parallel. INS microdata is optional.

**First real mechanic: the population generator from census marginals**, built through the tests-first two-PR flow. Its acceptance criteria:
- margins within a set tolerance at 1:1000, 1:100 and 1:10;
- total weight independent of the scale;
- golden hashes;
- a Python reference with a differential test.

**Order of work.**
1. The follow-ups PR above (RNG, VAT in the scale world, CI).
2. The population generator. **Done 2026-10-10** (version 1: county, household size, sex and age): spec [`society/population-generator`](02-design/society/population-generator.md), crate `econ-popgen`, pipeline stage `econ-cli synth-population`. Second increment (education and activity, spec [`society/population-attributes`](02-design/society/population-attributes.md)): spec, data and Python reference done 2026-10-10; acceptance tests and Rust code to follow. Then wiring the generated population into the simulation in place of the scale world's invented one.
3. Other follow-ups as they are needed: the firm-unit goods market (before the firms spec locks), the PC model in the differential test, history blocks in saves.

## Phase 2 — Playable vertical slice
- [ ] Minimal UI: dashboard, levers, time controls, graphs, group explorer, map.
- [ ] "Why did this change" causal breakdown.
- [ ] Save/load.

## Phase 3 — Depth
- [ ] Imperfect information: statistics lags, noise, paid polls.
- [ ] Scenarios with goals.
- [ ] More industries, multiple countries.
- [ ] Politics layer.
- [ ] Military layer.
