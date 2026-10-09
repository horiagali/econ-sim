---
id: architecture/overview
title: Architecture — Overview
status: draft
owner: horia
depends_on: [economy/overview, research/tech-stack-deep-research, adr/0004-tech-stack-overview]
updated: 2026-10-09
---

# Architecture

This page is the map of the technical architecture. The decisions themselves live in [`decisions/`](decisions/). ADR-0001 to ADR-0016 are accepted (2026-10-09). This is a personal, non-commercial project. The evidence behind them is in the [tech-stack research](../01-research/notes/tech-stack-deep-research.md) and the [population research](../01-research/notes/population-modelling-deep-research.md).

## Layers

```
 ┌──────────────────────────────────────────────────────────────────┐
 │  Player UI: Tauri 2 shell + React/TypeScript                     │
 │  uPlot charts, ECharts map and "why" views, api.ts adapter       │
 └───────────────▲──────────────────────────────────┬───────────────┘
                 │ binary IPC (Response, Channel)   │ Commands
 ┌───────────────┴──────────────────────────────────▼───────────────┐
 │  Simulation core: Rust workspace, linked in-process              │
 │  step / snapshot / restore / query / explain                     │
 │  column tables · fixed phase order · keyed RNG · Bani ledger     │
 │  invariants every tick · behaviour_rule! explanations            │
 └──────▲───────────────────────▲─────────────────────────▲─────────┘
        │ scenario (Arrow,      │ PyO3 bindings           │ links directly
        │ CSV, Parquet, TOML)   │ (econ-py)               │
 ┌──────┴──────────────┐ ┌──────┴──────────────────┐ ┌────┴────────────────┐
 │ Offline data        │ │ Calibration tooling     │ │ Developer inspector │
 │ pipeline (Python)   │ │ (Python)                │ │ (egui, not shipped) │
 │ uv · Polars · DuckDB│ │ SALib · GP emulators    │ └─────────────────────┘
 │ just · DVC          │ │ history matching ·      │
 │ provenance tags,    ├─▶ targets as code         │
 │ glossary codegen    │ │ (base-year dataset in)  │
 └─────────────────────┘ └─────────────────────────┘

 Saves: ZIP = manifest + Arrow IPC tables + command log; yearly history blocks.
 CI: Linux on every PR, Windows on core changes, cross-OS state-hash check.
```

## Decisions

| ADR | Status | One-line summary |
|---|---|---|
| [0001](decisions/0001-docs-first-ai-driven.md) | accepted | Docs-first specs with frontmatter; code only against `locked` specs. |
| [0002](decisions/0002-modelling-approach.md) | accepted | Hybrid model: SFC backbone, synthetic population, input-output industries. |
| [0003](decisions/0003-people-representation.md) | accepted | Weighted synthetic population; 1:100 default scale, configurable and never hardcoded; built from the IPUMS 2011 sample reweighted to 2021 census totals (raw microdata kept private). |
| [0004](decisions/0004-tech-stack-overview.md) | accepted | Umbrella: Rust core, Tauri 2 + React/TS UI, Python tooling. |
| [0005](decisions/0005-simulation-core-architecture.md) | accepted | Rust workspace crates, column tables, fixed phase order, step/snapshot/query API, no ECS or async, single-threaded. |
| [0006](decisions/0006-determinism-contract.md) | accepted | Keyed ChaCha8 RNG, `libm` only, fixed-chunk sums, ordered maps, clippy deny-lists, Linux-vs-Windows hash check. |
| [0007](decisions/0007-money-and-ledger.md) | accepted | `Bani(i64)` checked money; double-entry typed transfers with append-only flow codes; aggregated storage; invariants I-1 to I-7; weighted postings. |
| [0008](decisions/0008-explainability-architecture.md) | accepted | Five explanation layers; `behaviour_rule!` is the only way to write a rule; explanations sum exactly. |
| [0009](decisions/0009-calibration-and-stability.md) | accepted | Parameters from data; 8-stage workflow; Morris + history matching; lever-response bands; μ stability test; logged clamps; nightly Jacobian. |
| [0010](decisions/0010-verification-and-testing.md) | accepted | Invariants + differential SFC reference model + property, metamorphic, golden, snapshot and mutation tests; CI layout within the free minutes. |
| [0011](decisions/0011-storage-saves-history.md) | accepted | No runtime database; Arrow IPC + zstd saves with manifest and command log; migrations; immutable yearly history blocks. |
| [0012](decisions/0012-data-pipeline-and-licensing.md) | accepted | Python pipeline (uv, Polars, DuckDB, just, DVC); provenance-tagged raw archive (reference only); restricted microdata kept private; glossary as master schema. |
| [0013](decisions/0013-ui-stack.md) | accepted | Tauri 2 + React/TS, uPlot + ECharts, binary IPC, egui developer inspector, geoBoundaries map (GADM allowed if more convenient). |
| [0014](decisions/0014-agent-workflow-guardrails.md) | accepted | CLAUDE.md imports `@AGENTS.md`; nested AGENTS.md; hooks protect tests; tests-first two-PR flow; spec-ID traceability. |
| [0015](decisions/0015-distribution-and-licensing.md) | accepted | Distribution deferred; cargo-deny allow-list and notices file; check BeforeIT licence before copying code; real firm names in a swappable data file. |
| [0016](decisions/0016-firm-representation.md) | accepted | Firm units: 0–5 named firms (by threshold) plus size-class cohort firms per industry, with integer firm counts as weights (~550–700 units); ownership vector, control, dividends and equity valuation. |

The research report drafted 24 decisions (its numbers 0004–0027). They map onto the ADRs above as follows: core language and architecture → 0004/0005; determinism, randomness → 0006; money and numerics, ledger → 0007; explainability, log-linear aggregation → 0008; calibration, stability, Python bindings → 0009; verification, CI → 0010; runtime storage, saves → 0011; single schema, offline pipeline, config and modding → 0012; UI, map data → 0013; agent workflow, CI hosting limits → 0014; distribution, licensing → 0015; population scale and data sources → amended 0003.

## Top technical risks

Ranked by the [population research](../01-research/notes/population-modelling-deep-research.md) and extended by the [tech-stack research](../01-research/notes/tech-stack-deep-research.md).

| # | Risk | Why it is hard | Addressed by |
|---|---|---|---|
| 1 | Calibration and stability | Unsolved for large models; every new subsystem or lever can destabilise tuned dynamics | [ADR-0009](decisions/0009-calibration-and-stability.md) |
| 2 | Explainability ("why did this change") | Very costly to retrofit; agents drift to opaque logic | [ADR-0008](decisions/0008-explainability-architecture.md), [ADR-0007](decisions/0007-money-and-ledger.md) |
| 3 | Stock-flow consistency in AI-written code | Bookkeeping bugs ship even at major studios; invariants miss wrong-but-balanced bugs | [ADR-0007](decisions/0007-money-and-ledger.md), [ADR-0010](decisions/0010-verification-and-testing.md), [ADR-0014](decisions/0014-agent-workflow-guardrails.md) |
| 4 | Data integration | Reconciling sources is a large work package; restricted microdata (IPUMS) must stay out of any public repo | [ADR-0012](decisions/0012-data-pipeline-and-licensing.md), [ADR-0003](decisions/0003-people-representation.md), [ADR-0015](decisions/0015-distribution-and-licensing.md) |
| 5 | Determinism across machines and versions | Platform maths, parallel float sums and RNG library changes all break bit-identity | [ADR-0006](decisions/0006-determinism-contract.md), [ADR-0010](decisions/0010-verification-and-testing.md) |
| 6 | Population scale and noise | Hardcoded scale would lock the game; small groups are noisy | [ADR-0003](decisions/0003-people-representation.md), [ADR-0007](decisions/0007-money-and-ledger.md), [ADR-0010](decisions/0010-verification-and-testing.md) |
| 7 | Agents weakening tests on GitHub Free | CODEOWNERS and rulesets are not enforced on private Free repos | [ADR-0014](decisions/0014-agent-workflow-guardrails.md) |
| 8 | Tauri on Steam | No commercial Tauri game found; Steam overlay over WebView2 unverified. Dormant while distribution is deferred | [ADR-0013](decisions/0013-ui-stack.md) (Electron fallback), [ADR-0015](decisions/0015-distribution-and-licensing.md) |

The order of work that retires these risks is the spike sequence in the [roadmap](../roadmap.md).

## Open questions
- [x] Owner review and acceptance of ADR-0002 and ADR-0004 to ADR-0015 (2026-10-09).
- [ ] Owner confirmation of [ADR-0016](decisions/0016-firm-representation.md) (firm representation: named firms plus size-class cohort firms).
