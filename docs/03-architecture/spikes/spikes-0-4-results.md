---
id: architecture/spikes-0-4-results
title: "Spike results: 0–4 (repo setup, numerics, ledger + SIM, explanations, scale)"
status: draft
owner: horia
depends_on: [roadmap, adr/0006-determinism-contract, adr/0007-money-and-ledger, adr/0008-explainability-architecture, adr/0003-people-representation]
updated: 2026-10-10
---

# Spike results: 0–4

Built 2026-10-09 in an autonomous session. All code is in `crates/`,
`python/` and `scripts/`; `just check` is green on Linux (Rust 1.97.0,
2-vCPU Xeon VM). **Not yet run on Windows** — that is the owner's first step.

## Spike 0 — repo setup ✅
| Item | Where |
|---|---|
| Cargo workspace (edition 2024, resolver 3), pinned toolchain 1.97.0 | `Cargo.toml`, `rust-toolchain.toml` |
| Determinism deny-lists (HashMap/HashSet, `f64::exp/ln/powf/…`, `mul_add`, `Instant`, `env::var`) | `clippy.toml` + `[workspace.lints]` |
| Proof the deny-lists work: a canary crate clippy must reject | `crates/lint-canary`, `just lint-canary` |
| Licence policy | `deny.toml` (`just deny`) |
| Task runner | `justfile` (`just check` = fmt, clippy, tests, canary, docs, differential test) |
| CI: Linux every PR; Windows when core changes; Linux-vs-Windows hash comparison; cargo-deny | `.github/workflows/ci.yml` |
| Claude Code hook blocking edits to `tests/golden/**`, `crates/*/tests/acceptance/**`, `schema/**` (unless `ECON_TEST_AUTHORING=1`); `gh pr merge` was denied until the owner removed that rule on 2026-10-10 (ADR-0014) | `.claude/settings.json`, `scripts/hooks/protect_paths.py` |
| Rules for agents | `DETERMINISM.md`, `crates/AGENTS.md`, `python/AGENTS.md`, root `AGENTS.md` (Phase 1) |

> Three files (`justfile`, `.github/workflows/ci.yml`, `.claude/settings.json`) could not be written remotely and were delivered in `setup-pending/` with move instructions.

**Exit criterion** ("an agent on Windows runs `just check` green and is blocked from editing `tests/golden/`"): hook verified locally; **passed on a Windows laptop 2026-10-10** (`just check` green, hook blocks Edit/Write and shell writes; see "Windows verification" in the [spikes 5–9 results](spikes-5-6-results.md)).

## Spike 1 — numeric foundation ✅
- `econ-types::Bani(i64)`: checked arithmetic (overflow panics in release too: `overflow-checks = true`), one rounding rule (half away from zero), exact `mul_ratio` via `i128`, `mul_rate` guarded to |x| < 2⁵³, largest-remainder `split_largest_remainder` (property-tested: parts always sum exactly).
- `econ-num`: `math` wrappers on pinned `libm` 0.2.16 with **bit-exact known-answer vectors**; `det_sum` (fixed 4096 chunks, parallel-safe); hand-written partial-pivot LU + `leontief_output` (90×90 residual < 1e-9, bit-reproducible).
- `econ-rng`: ChaCha8 keyed on (seed, stream, tick, entity) with pinned KAT vectors; order-independence tested.

**Exit criterion** (same hashes on Windows and Linux): Linux done; **Windows CI and a Windows laptop produce the same hashes (2026-10-10)**.

## Spike 2 — ledger + differential SIM ✅
- `econ-ledger`: sectors × instruments, typed linked `Txn`s applied atomically, flow codes, weighted legs (`per_unit × weight`), invariant checks I-1 (rows sum to zero), I-2 (sign constraints), I-3 (TFM rows zero; columns = Δ net financial assets) computed on a separate path. Property test: random payment sequences never break invariants; a corrupted balance is detected.
- `econ-core::sim`: Godley–Lavoie model SIM on the real ledger, integer bani, per-tick FNV-1a state hash.
- `python/reference/sim_reference.py`: independent reference; `diff_sim.py` compares **every aggregate, every tick, exactly in bani for 200 ticks — exact match**. Two deliberate mutations (tax sign flipped; consumption on gross income) are both caught, so the test has teeth.
- Golden run `tests/golden/sim_200.hashes` (`just golden-check`).

**Exit criterion met.** Model PC (with bonds and a central bank) is the follow-up.

## Spike 3 — explanations ✅
- `econ-rules::behaviour_rule!` macro: shapes `additive` and `log_linear`; generates `Inputs`, `Params`, `eval`, `eval_explained`, `explain_change → ContributionTree`, and `META` (terms, labels, provenance `FromData`/`Fitted`/`Tuned`).
- Applied to example price (log-linear), wage-growth and consumption (additive) rules. A **nested tree** (wage drivers under the unit-cost driver of a price change) sums exactly.
- `partial_adjust` + `stability_multiplier` (μ = 1 + λ(f′ − 1); stable iff |μ| < 1) with tests.
- **Open question settled (ADR-0008):** for indicators that are sums of levels, aggregate per-agent **LMDI contributions in levels** — they sum exactly to the change in the total (tested on 1,000 weighted agents). Weighted-mean log contributions are only for indices/averages (CPI, average wage growth) and must never be used for totals.

**Exit criterion met.** Not yet done: CI counter for `opaque` waivers (no opaque rules exist yet).

## Spike 4 — performance at variable scale ✅
Fake weighted population of Romania at any `sample_scale`; tick = separations, labour matching per (region × skill) with alignment-style selection, wages via per-industry clearing accounts (I-8), consumption over 83 goods, 90×90 Leontief solve, aggregation cube (county × activity × age band × education). Release build, single thread, 2-vCPU Xeon 2.1 GHz:

| Scale | Persons | Households | ms per tick | 50-year run (600 ticks) |
|---|---|---|---|---|
| 1:1000 | 19,000 | 7,584 | 0.7 | 0.4 s |
| **1:100** | **190,000** | **75,960** | **6.4** | **3.9 s** |
| 1:10 | 1,900,000 | 759,895 | 64.6 | 38.8 s |

Target was ≤ ~30 s for 50 years at 1:100: **met with ~8× headroom**. Even 1:10 is playable.

**Key finding — RNG cost.** The first version took **240 ms/tick** at 1:100. Setting up a ChaCha8 context per person per draw (key schedule + 256-byte block) dominated. Adding a counter-based fast path (`KeyedRng::fast_u64/fast_bernoulli`: SplitMix64 mixing of (seed, stream, tick, entity, k), still a pure function of its inputs) plus splitting consumption per basket class instead of per household brought it to 6.4 ms. → **ADR-0006 Amendment 1, accepted 2026-10-10**: hot per-agent draws use the counter-based fast path; ChaCha8 stays for everything else. Statistical battery passed (see the ADR).

**3-scale test** (ADR-0010) implemented: at 1:2000, 1:1000 and 1:500 the clearing invariant holds and wage bill per capita and the unemployment rate agree within sampling error. Proves nothing is hardcoded to one scale.

Not covered yet: real firm-unit goods market, save size (Spike 6), parallelism (not needed at these timings).

## What the owner needs to do
1. Move the three files out of `setup-pending/` (see its README).
2. Install `just` (`winget install Casey.Just`) and run `just check` on Windows.
3. Commit and push; check the CI run, especially the Linux-vs-Windows hash comparison.
4. ~~Accept or reject the ADR-0006 amendment~~ — accepted 2026-10-10.
