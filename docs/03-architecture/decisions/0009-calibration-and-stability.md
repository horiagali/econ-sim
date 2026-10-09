---
id: adr/0009-calibration-and-stability
title: "ADR-0009: Calibration and stability strategy"
status: accepted
owner: horia
depends_on: [adr/0008-explainability-architecture, adr/0006-determinism-contract, adr/0012-data-pipeline-and-licensing]
updated: 2026-10-09
---

# ADR-0009: Calibration and stability strategy

## Context
The [population research](../../01-research/notes/population-modelling-deep-research.md) ranks calibrating the coupled economy and keeping it stable as the biggest technical risk. Calibration methods "still need development before they can reliably calibrate large-scale models" (Platt), and the target moves every time a subsystem or lever is added. Victoria 3 clamps prices to 25–175% of base and still retuned wages, investment and trade for years after launch.

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) (section "Calibration and stability") found:
- The Poledna/BeforeIT approach sets almost every parameter and the initial state directly from national accounts, IO tables, sector accounts and business demography; the result reproduces the base quarter exactly. Its own authors call their calibration scripts "an unpolished research prototype".
- The Godley-Lavoie SFC workflow checks a no-shock baseline for drift before applying any shock (`sfcr_baseline` then `sfcr_scenario`).
- History matching suits range targets ("investment 2–4× as volatile as GDP") because it removes implausible parameter regions instead of claiming one true parameter vector.
- Lever-response evidence: prices fall about 0.9% after a 1 pp rate hike, with the trough after 10–20 months in post-transition economies (Havranek & Rusnak meta-analysis).
- A partial-adjustment rule x′ = x + λ(f(x) − x) has local multiplier μ = 1 + λ(f′ − 1). It is stable when 0 < λ(1 − f′) < 2 and free of sawtooth oscillation when that product is at most 1.
- If a 50-year run takes about 20 s, 16 cores give about 2,900 runs an hour: Morris screening of 40 parameters (820 runs) takes under 20 minutes.

## Options considered
1. **Direct parameterisation from data, then history matching of a small set of free parameters, with stability measured automatically.**
   Pros: most parameters are not tuned at all; range targets fit game goals; stability problems are caught by tests, not players.
   Cons: needs a balanced base-year dataset first; tooling to build.
2. **Single-point optimisation of all parameters against targets.**
   Pros: one number to minimise.
   Cons: many parameters, many local optima, overfits; hides that several parameter sets are equally plausible.
3. **Full Bayesian posterior as the default.**
   Pros: principled uncertainty.
   Cons: too expensive per iteration; overkill for "plausible over precise".
4. **Hand tuning with clamps.**
   Pros: fast at first.
   Cons: the Victoria 3 path: years of retuning, clamps that hide broken dynamics.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1.**

**Staged workflow** (every new subsystem reruns steps 4–8 in CI):
1. Build a balanced base-year SFC dataset ([ADR-0012](0012-data-pipeline-and-licensing.md)).
2. Compute every ratio parameter from it mechanically and reproducibly.
3. Fit AR(1) or small VAR processes for exogenous series.
4. On tick 0, assert every identity; then run one "frozen behaviour" tick that must reproduce base-year flows to the bani.
5. Tune only the roughly 20–60 free behavioural parameters.
6. Run 50–100 quiet years; every key ratio must stay within ±10% of its base value.
7. Match targets.
8. Freeze the result as a versioned calibration release.

**Tools** (Python, calling the core through `econ-py`):
- Parameter generation is its own tested Python package from day one.
- Morris screening with SALib first; history matching with scikit-learn Gaussian-process emulators and a short implausibility test (hmer's method, rewritten in Python); at least 10 training runs per input.
- Optuna multi-objective samplers only to pick a pleasant point inside the surviving region. sbi only to check whether key parameters can be identified. black-it is an alternative.
- Common random numbers across parameter points (trivial with keyed RNG, [ADR-0006](0006-determinism-contract.md)).
- Bindings: PyO3 + maturin abi3 wheels, tables via pyo3-arrow; PyO3, arrow-rs and pyo3-arrow are bumped together.

**Targets as code.** A targets file lists `{id, metric, band, source}` with checker functions. Lever-response bands are anchored in evidence. Example R-1: after a 1 pp policy-rate rise, the CPI trough falls between 8 and 24 months and the peak effect lies between −0.2% and −1.5%.

**Stability as a measured property**
- Every `behaviour_rule!` records λ and an estimate of f′ in its `META` ([ADR-0008](0008-explainability-architecture.md)). A unit test fails if μ ≤ 0 or |μ| ≥ 0.95, unless the rule has a waiver.
- Every clamp goes through `clamp_logged!`, which counts how often it binds. A baseline run with clamps binding on more than about 0.1% of evaluations fails CI: the model has left its tuned region.
- Cheap per-tick monitors in the core: NaN and sign checks, growth-rate guards, a sawtooth detector.
- Nightly: estimate the one-tick Jacobian of about 30–100 aggregates by finite differences and flag any eigenvalue above 1.

**Market design for stability.** Real IO flows clear within the tick; consumer goods and labour use rationed search-and-matching (as BeforeIT does); prices, wages and expectations adjust only between ticks. No Walrasian iteration over agents within a tick.

## Consequences
- Easier: most parameters trace to a data source; every rule's provenance (`from_data`, `fitted`, `tuned`) is visible in docs and in the UI.
- Easier: a new subsystem that destabilises the economy fails CI before it reaches a player.
- Harder: the base-year dataset and reconciliation are a large work package ([ADR-0012](0012-data-pipeline-and-licensing.md)).
- Harder: every rule needs an f′ estimate and a stability waiver if it fails the μ test.
- If targets cannot be met, revisit the model structure, not the tuning.

## Open questions / to verify
- [ ] 50-year run time at 1:100 (assumed ~20 s; Spike 4) and runs per hour on the owner's machine (Spike 5).
- [ ] Romania-specific SVAR studies to tighten lever-response bands were not found; CEE-wide evidence is used meanwhile.
- [ ] The final count of free behavioural parameters (estimated 20–60).
- [ ] Whether calibration runs at 1:100 or a coarser scale (1:200) for speed, checked against the 3-scale test in [ADR-0010](0010-verification-and-testing.md).
