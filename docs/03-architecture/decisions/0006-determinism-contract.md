---
id: adr/0006-determinism-contract
title: "ADR-0006: Determinism contract"
status: accepted
owner: horia
depends_on: [adr/0005-simulation-core-architecture]
updated: 2026-10-09
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

## Consequences
- Easier: golden-run tests, exact replay, reproducing any player's bug from a save, common random numbers across calibration points ([ADR-0009](0009-calibration-and-stability.md)).
- Harder: agents cannot use standard maths functions, `HashMap` or the default RNGs in state code; clippy tells them so.
- Harder: dependency bumps of `libm`, `rand_*` or the toolchain may change hashes and need a reviewed re-golden.
- This contract is never relaxed. It is extended when parallelism is added.
- Revisit the RNG only if `rand_chacha` is deprecated (fallback: a self-written Philox generator checked against Random123 vectors).

## Open questions / to verify
- [ ] clippy's disallowed lists fire only for paths that resolve; verify every path when first configured (Spike 1).
- [ ] Same hashes on the owner's Windows laptop and on Linux CI (Spike 1 exit criterion).
- [ ] Whether the hand LU stays accurate enough if the IO table grows well past 80 sectors.
