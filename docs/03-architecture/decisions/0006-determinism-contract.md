---
id: adr/0006-determinism-contract
title: "ADR-0006: Determinism contract"
status: accepted
owner: horia
depends_on: [adr/0005-simulation-core-architecture]
updated: 2026-10-10
---

# ADR-0006: Determinism contract

## Context
Pillar 2 requires the simulation to be deterministic given a seed. Golden-run tests, bug reproduction from a save, replay from a command log and common random numbers in calibration all depend on bit-identical results. The owner develops on Windows and CI runs on Linux, so results must also match across operating systems.

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) (section "Determinism: a written contract") found:
- Rust guarantees IEEE 754 results for basic float operations, with no flush-to-zero and no implicit fused multiply-add (RFC 3514).
- Rust's standard `sin`, `exp`, `ln` and `powf` are documented as non-deterministic across platforms and Rust versions. On 200,000 inputs, std and the pure-Rust `libm` crate differed in 6,045 to 20,269 cases per function.
- A rayon parallel float sum over a million values gave 2–6 different bit patterns per thread count, even between runs. Fixed 4,096-element chunks combined in order gave one result at every thread count.
- rand 0.9 changed the output of several functions; rand's policy says `StdRng` and `SmallRng` "may change output in any release". `rand_chacha`'s ChaCha8 is covered by the portability policy and supports 2^64 streams per seed and random access by word position.
- faer dispatches SIMD kernels by CPU at runtime, so its operation order can differ between machines.

## Options considered
1. **Strict bit-for-bit determinism, enforced by a written contract plus lints and CI.**
   Pros: golden hashes, exact replay, exact bug reproduction, common random numbers.
   Cons: rules agents must follow; some library functions are banned.
2. **Tolerance-based "close enough" replay.**
   Pros: fewer rules.
   Cons: breaks golden tests and replay; small differences grow over decades; bugs cannot be reproduced exactly.
3. **One shared sequential random generator.**
   Pros: simple.
   Cons: any change in call order changes every later number; blocks parallelism and common random numbers.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1.** The contract is written as `DETERMINISM.md` in the code repo and enforced where possible.

**Rules**
1. All transcendental maths goes through `econ_num` wrappers around a pinned `libm`.
2. No NaN may reach state; assert `is_finite()` at phase boundaries.
3. Float sums over agents use integer bani or a fixed-chunk ordered `det_sum`, never a reduction whose order depends on the thread count.
4. Parallel code may only write to disjoint indices or return per-chunk partials collected in index order.
5. Iterate only over `Vec`, `BTreeMap` or `IndexMap`, never `std::HashMap` or `HashSet`.
6. Sort with `total_cmp` plus an ID tiebreak.
7. Target 64-bit platforms only.
8. Build with default flags: no `target-cpu=native`, no fast-math. Pin the toolchain in `rust-toolchain.toml`.

**Randomness.** ChaCha8 from `rand_chacha`. Each draw is keyed: key from (master seed, stream ID), stream from the entity ID, position from the tick. Every random draw is therefore a pure function of (seed, stream, tick, entity), independent of call order. Distributions are written on `libm`, not taken from `rand_distr`. Known-answer vectors are pinned in a test, so any dependency bump that changes values fails CI. Every `rand_*` upgrade is a re-golden event.

**Linear algebra.** The 80×80 Leontief solve uses a hand-written scalar partial-pivot LU (about 60 lines) in `econ-num`. faer is kept only as a test oracle.

**Enforcement**
- clippy `disallowed_types` and `disallowed_methods`, set to deny, ban: `HashMap`, `HashSet`, `StdRng`, `SmallRng`, `Instant`, `SystemTime`, `f64::sin`, `exp`, `ln`, `powf`, `powi`, `rand::rng`, `thread_rng`, rayon float `sum` and `reduce`, and `std::env::var` in state code.
- Every `TickReport` carries a per-tick state hash.
- CI compares state hashes byte for byte between Linux and Windows, and between thread counts once parallelism exists ([ADR-0010](0010-verification-and-testing.md)).

**Concurrency.** Start single-threaded. Parallelism is added later behind a feature flag and must keep the same hashes.

## Amendment 1 (Spike 4) — accepted by owner 2026-10-10: two-tier randomness
Setting up a ChaCha8 context per person per draw made a 1:100 tick take 240 ms. Two tiers, both pure functions of `(seed, stream, tick, entity, k)`, so order- and thread-independence are unchanged:

| Tier | API | Use for |
|---|---|---|
| Fast | `KeyedRng::fast_u64` / `fast_uniform` / `fast_bernoulli` (SplitMix64 mixing, entity multiplied by the SplitMix64 gamma; `k` = draw index within the context) | per-agent, per-tick draws in hot loops: separations, matching priorities, consumption noise, demographic events |
| ChaCha8 | `KeyedRng::draw(...)` → `Draw` | everything else: population generation, shocks, weather, any context needing many or correlated draws (normals, shuffles) |

When unsure, use ChaCha8; moving a call site to the fast tier needs a benchmark showing it matters.

**Quality evidence (2026-10-10).** A NumPy re-implementation (`python/reference/rng_quality.py`, checked against `FAST_KAT`) ran a battery of 4M-draw sequences along each input axis (entity, tick, k, seed): 16-bit uniformity, per-bit balance, neighbour avalanche, 64×64 neighbour bit correlation, serial correlation at lags 1/2/1024, low-p Bernoulli co-occurrence of neighbours and a 2-D serial test. All passed; flipping any entity bit flips each output bit with probability 0.5 ± 0.004 (sampling noise). Two stronger variants were tested and gave no improvement in this battery, so the mixer was left unchanged at the time. `fast_draws_neighbours_independent` in `econ-rng` keeps a fast subset as a regression test. PractRand/TestU01 were not reachable from the build sandbox.

**Revision (owner decision, 2026-10-10): the "gamma" mixer.** PractRand 0.95, run on a desktop through `econ-cli rng-raw`, found what the battery above could not:

- The original last step was `mix64(st ^ entity)`. With only the entity varying (entity = 0, 1, 2, … at one tick) it **fails PractRand `BRank` at 16 GB** (p ≈ 1e-235 with seed 42, 1e-189 with seed 7).
- The defect is not far out: the first 2^20 consecutive entities, laid out as an 8192×8192 bit matrix, already have a rank deficiency of 32–35 (a random matrix: 0–2). PractRand reports it late only because it first tries a matrix that large at 16 GB. A 1:10 population (1.9 million persons) is inside that range.
- In the order the simulation draws (190,000 entities per tick, tick after tick) the original mixer passed 64 GB, and no effect on results was observed. The change is made for margin and because the fix is one multiply.
- **The fast tier now ends with `mix64(st ^ entity·γ)`**, γ = `0x9E3779B97F4A7C15` (the "gamma" variant of `rng_quality.py`). It passes PractRand to 64 GB on the entity-only pattern and has rank deficiency 0–1 on the matrix above.
- Regression tests in `econ-rng`: `fast_draws_binary_rank_over_consecutive_entities` (the 8192×8192 rank check; it fails on the original mixer) and a new `FAST_KAT`. `rng_quality.py` now has the gamma mixer as `cur` and the original as `old`.
- This was a re-golden event for everything that uses the fast tier (the scale world); see `CHANGELOG-sim.md`. The SIM golden run does not use it and is unchanged.

With the fast tier (plus a consumption-loop change made at the same time) a 1:100 tick took 6.4 ms (the gamma multiply does not change this measurably). See [spike results](../spikes/spikes-0-4-results.md).

## Consequences
- Easier: golden-run tests, exact replay, reproducing any player's bug from a save, common random numbers across calibration points ([ADR-0009](0009-calibration-and-stability.md)).
- Harder: agents cannot use standard maths functions, `HashMap` or the default RNGs in state code; clippy tells them so.
- Harder: dependency bumps of `libm`, `rand_*` or the toolchain may change hashes and need a reviewed re-golden.
- This contract is never relaxed. It is extended when parallelism is added.
- Changing either tier's algorithm or `FAST_KAT` is a re-golden event.
- Revisit the RNG only if `rand_chacha` is deprecated (fallback: a self-written Philox generator checked against Random123 vectors).

## Open questions / to verify
- [ ] clippy's disallowed lists fire only for paths that resolve; verify every path when first configured (Spike 1).
- [ ] Same hashes on the owner's Windows laptop and on Linux CI (Spike 1 exit criterion).
- [ ] Whether the hand LU stays accurate enough if the IO table grows well past 80 sectors.
