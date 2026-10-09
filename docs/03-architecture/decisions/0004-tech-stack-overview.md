---
id: adr/0004-tech-stack-overview
title: "ADR-0004: Tech stack overview — Rust core, Tauri 2 + React UI, Python tooling"
status: accepted
owner: horia
depends_on: [research/tech-stack-deep-research, adr/0002-modelling-approach, adr/0003-people-representation]
updated: 2026-10-09
---

# ADR-0004: Tech stack overview — Rust core, Tauri 2 + React UI, Python tooling

This is the umbrella ADR. It picks the three main technologies and lists the focused ADRs that hold the detailed decisions.

## Context
The game needs a simulation that is deterministic, stock-flow consistent, explainable and fast enough to run thousands of calibration runs. It also needs a UI that is mostly business-app work: dozens of charts, filterable tables, about 150 lever forms and nested "why did this change" breakdowns. Most code will be written by AI agents, so the stack should turn spec violations into compile errors or failing checks that an agent can see and fix.

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) and the earlier [population research](../../01-research/notes/population-modelling-deep-research.md) found:
- Population size is not the main technical risk. Calibration and stability, explainability and verifying stock-flow consistency rank higher. Data licensing and determinism are close behind.
- Compiled languages with column storage run a person-month in about 9 ns. Pure Python is about 100× slower.
- On SWE-bench Multilingual, Rust had the highest resolve rate of nine languages for the one model tested (thin evidence, but it points the same way).
- .NET documents that `Math.Sin` depends on the C runtime and can differ between operating systems. C++ allows undefined behaviour and implicit fused multiply-add.
- Web stacks (JavaScript, TypeScript, React) have far more ready-made chart, table and form components, and more AI training data, than game engines.

The stack assumes the hybrid model that [ADR-0002](0002-modelling-approach.md) chose (SFC backbone, synthetic population, input-output industries) and the weighted synthetic population of [ADR-0003](0003-people-representation.md).

## Options considered
1. **Rust core + Tauri 2/React UI + Python tooling.**
   Pros: IEEE-exact basic floats and a pure-Rust `libm` make cross-platform determinism achievable. Newtypes, exhaustive `match` and clippy deny-lists catch agent mistakes at compile time. Tauri's host process is Rust, so the core links in-process with no FFI. Python keeps the best data and calibration libraries.
   Cons: long compile times. Two languages on the UI side (Rust host, TypeScript front end). Tauri has no proven commercial Steam game yet.
2. **C# core + Unity or Godot UI.**
   Pros: fast compiles, one language, game-engine tooling.
   Cons: maths functions depend on the C runtime (determinism risk). Every chart, table and form is custom work. Weaker Python bindings for calibration.
3. **C++ core.**
   Pros: fastest, mature.
   Cons: undefined behaviour and implicit FMA hurt determinism. Agents make more memory errors. Slower to build bindings.
4. **Python (+ NumPy/Numba) for everything, rewrite later.**
   Pros: fastest start; same language as the data pipeline.
   Cons: 10–100× slower per tick, which makes calibration sweeps too slow. Weak typing lets agent errors through at runtime. A rewrite later doubles the work.
5. **Existing ABM framework or ECS engine (krABMaga, bevy_ecs, hecs).**
   Pros: scheduling and parallelism for free.
   Cons: built around per-agent `step` calls or changing component sets. This model has fixed attributes in column tables and a fixed phase order, so plain vectors fit better.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1.**

| Layer | Technology | Detailed in |
|---|---|---|
| Simulation core | Bespoke Rust workspace, column tables, fixed phase order, single-threaded by default | [ADR-0005](0005-simulation-core-architecture.md) |
| Determinism | Written contract enforced by clippy and cross-OS hash checks | [ADR-0006](0006-determinism-contract.md) |
| Money and accounting | `Bani(i64)`, double-entry ledger with flow codes, per-tick invariants | [ADR-0007](0007-money-and-ledger.md) |
| Explainability | Five layers; `behaviour_rule!` macro as the only rule form | [ADR-0008](0008-explainability-architecture.md) |
| Calibration and stability | Parameters from data, staged workflow, history matching, stability tests | [ADR-0009](0009-calibration-and-stability.md) |
| Verification | Invariants, differential SFC reference model, property, golden and mutation tests | [ADR-0010](0010-verification-and-testing.md) |
| Storage | No runtime database; Arrow IPC saves, command log, yearly history blocks | [ADR-0011](0011-storage-saves-history.md) |
| Data pipeline and data licences | Python (uv, Polars, DuckDB, just, DVC); provenance tags for reference; IPUMS seed kept private; glossary as master schema | [ADR-0012](0012-data-pipeline-and-licensing.md) |
| Player UI | Tauri 2 + React/TypeScript, uPlot + ECharts; egui developer inspector | [ADR-0013](0013-ui-stack.md) |
| Agent workflow | `@AGENTS.md` import, hooks, tests-first two-PR flow, CI checks | [ADR-0014](0014-agent-workflow-guardrails.md) |
| Shipping and code licences | Distribution deferred (personal, non-commercial); cargo-deny allow-list; check BeforeIT licence before copying code | [ADR-0015](0015-distribution-and-licensing.md) |

Population scale (1:100 default, configurable, never hardcoded) and the population data sources are recorded in the amended [ADR-0003](0003-people-representation.md).

Python reaches the core through PyO3 bindings (`econ-py`, abi3 wheels built with maturin, tables passed zero-copy with pyo3-arrow). PyO3, arrow-rs and pyo3-arrow are bumped together. See [ADR-0009](0009-calibration-and-stability.md).

## Consequences
- Easier: determinism, accounting and explainability rules can be enforced by the compiler, clippy and CI rather than by prose. The core runs headless for tests, calibration and batch runs. The UI can use standard web chart and table libraries.
- Harder: Rust compile times in the agent loop. Mitigations: small crates, `cargo check` in the loop, Polars, DuckDB and Arrow kept out of `econ-core`, and an AGENTS.md rule that a borrow error surviving two attempts means "restructure around indices", not "add `Rc<RefCell<>>`".
- Three toolchains to maintain (Rust, Node, Python). The glossary-driven codegen in [ADR-0012](0012-data-pipeline-and-licensing.md) keeps their types in sync.
- The order of work follows the spike sequence in the [roadmap](../../roadmap.md). The hardest decisions to reverse (flow codes, rule format, RNG scheme, money type, scale as a parameter) are settled by Spikes 1–4.
- Revisit if compile times stall the agent loop despite the crate split, or if the Tauri/WebView2 path fails (fallback: Electron behind the same `api.ts` adapter).

## Open questions / to verify
- [ ] Rust compile times in the agent loop on the owner's Windows laptop (measure in Spike 0 and Spike 1).
- [ ] 50-year run at 1:100 within about 30 s, including market clearing (Spike 4).
- [x] ADR-0002 (modelling approach) accepted on 2026-10-09 (hybrid), as this stack assumed.
