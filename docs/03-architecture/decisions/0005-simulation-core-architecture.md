---
id: adr/0005-simulation-core-architecture
title: "ADR-0005: Simulation core architecture"
status: accepted
owner: horia
depends_on: [adr/0004-tech-stack-overview, adr/0003-people-representation]
updated: 2026-10-09
---

# ADR-0005: Simulation core architecture

## Context
The core holds the whole economy: about 190k persons and 75k households at the default 1:100 scale, about 90 industries, banks, government, central bank and the rest of the world. Each tick touches a few of about 100 attributes per person. The core must be headless (tests, calibration and batch runs need no UI), deterministic and easy for agents to change in small, reviewable pieces.

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) (section "The core is a bespoke Rust workspace") found:
- Column storage ("struct of arrays") was 10–15× faster than one object per person when a tick touches 4 of 100 attributes.
- krABMaga follows a per-agent `step` model and marks its parallel mode "experimental". ECS libraries (bevy_ecs, hecs) are built for entities whose set of components changes. Persons here change attribute *values*, not components.
- A monthly tick at 1:100 costs milliseconds on one thread (about 4–12 ms estimated), so parallelism is not needed to start.

## Options considered
1. **Bespoke workspace with plain column tables and an explicit phase list.**
   Pros: fastest layout for this access pattern. Columns map one-to-one onto Arrow arrays for saves and export. Phase order is visible in one file.
   Cons: scheduling, queries and parallelism are hand-written.
2. **ECS (bevy_ecs, hecs).**
   Pros: ready-made storage, queries and parallel scheduling.
   Cons: built for changing component sets, which this model does not have. Implicit system ordering makes phase order harder to review. Bevy's frequent breaking releases mean agents write stale APIs.
3. **ABM framework (krABMaga).**
   Pros: agent scheduling built in.
   Cons: per-agent `step` objects, experimental parallelism, no ledger or column export.
4. **Async or actor-based core.**
   Pros: natural for UI concurrency.
   Cons: non-deterministic ordering; adds complexity with no benefit for a batch tick.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1.**

**Workspace crates** (small, so agents get fast compiles and narrow edits):

| Crate | Holds |
|---|---|
| `econ-types` | ID newtypes (`PersonId(u32)`, `FirmId(u32)`, `IndustryId(u8)`), `Bani(i64)`, `Month`, flow-code enums. No dependencies. Generated from the glossary |
| `econ-rng` | Keyed RNG wrapper, distributions written on `libm`, known-answer vectors |
| `econ-num` | `libm` wrappers, fixed-chunk sums, largest-remainder splitting, the input-output LU solver |
| `econ-ledger` | Accounts, typed transfers, flow codes, tag accumulators, per-tick invariant checks |
| `econ-core` | `World` (the column tables), the `Phase` trait, the fixed phase order, `Command`, `TickReport`, explanation records. No I/O |
| `econ-mechanics-*` | One module per spec doc (labour, consumption, production/IO, pricing, credit, fiscal, monetary, external, …) |
| `econ-io` | Saves, Arrow IPC/Parquet export, scenario loading |
| `econ-py` | PyO3 bindings |
| `econ-cli` | Headless runner for golden tests, benchmarks and calibration batches |

**State.** `World` holds column tables: one `Vec` per attribute per table (persons, households, firms, industries, banks, …), linked by integer IDs. No table or array is sized by a population constant. Capacities come from the loaded scenario, whose size follows from the data and `sample_scale` ([ADR-0003](0003-people-representation.md)).

**Phase order.** Mechanics implement a `Phase` trait. The order is one `const` list in one file, not dynamic registration, so any change to it shows up in review. Within a tick, real input-output flows clear first; prices, wages and expectations adjust only between ticks ([ADR-0009](0009-calibration-and-stability.md)).

**API.**
- `step(&mut self, cmds: &[Command]) -> TickReport`: applies player commands, runs all phases, returns aggregates, invariant results, the state hash and explanation records.
- `snapshot()` / `restore()`: full state for saves and counterfactual forks (a plain `Clone` of `World` in memory).
- `query(...)`: read-only access that returns columns; used by the UI, the group explorer and Python.
- `explain(indicator, region, window) -> ContributionTree` ([ADR-0008](0008-explainability-architecture.md)).

**Concurrency.** No async in the core. Single-threaded by default. Parallelism (rayon) may be added later behind a feature flag, under the rules in [ADR-0006](0006-determinism-contract.md); keyed RNG and disjoint writes already allow it.

**Dependencies.** Polars, DuckDB and Arrow stay out of `econ-core`. arrow-rs is used only in `econ-io` at the I/O boundary ([ADR-0011](0011-storage-saves-history.md)).

## Consequences
- Easier: profiling (one loop per phase over a few columns), exporting state to Arrow, reviewing changes in phase order, running thousands of headless runs from Python.
- Easier: mechanics map one-to-one onto spec docs, so an agent edits one crate module against one spec.
- Harder: hand-written queries and joins in Rust (a bitmask filter is enough for the group explorer; see [ADR-0011](0011-storage-saves-history.md)).
- Harder: adding parallelism later needs care, which the determinism contract governs.
- Revisit if compile times stall the agent loop despite the crate split, or if Spike 4 shows a tick at 1:100 far above the estimate.

## Open questions / to verify
- [ ] Market clearing (labour matching, goods market, 80×80 IO solve) was not benchmarked; Spike 4 measures tick time at 1:1000, 1:100 and 1:10.
- [ ] Exact split of `econ-mechanics-*` crates once the specs settle (one crate per spec, or grouped).
- [ ] The firm-unit table (named firms plus size-class cohort firms with `firm_count`) is a new table in `World`; see [ADR-0016](0016-firm-representation.md) (accepted 2026-10-09).
