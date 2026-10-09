---
id: adr/0010-verification-and-testing
title: "ADR-0010: Verification and testing strategy"
status: accepted
owner: horia
depends_on: [adr/0007-money-and-ledger, adr/0006-determinism-contract, adr/0014-agent-workflow-guardrails]
updated: 2026-10-09
---

# ADR-0010: Verification and testing strategy

## Context
Keeping a stock-flow-consistent codebase correct when AI agents write most of it is the third-biggest technical risk in the [population research](../../01-research/notes/population-modelling-deep-research.md). Ledger invariants ([ADR-0007](0007-money-and-ledger.md)) catch leaks, but not *wrong-but-balanced* economics: a tax booked to the wrong sector, or a sign error that still balances. Agents also write hollow tests and can be talked into weakening tests.

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) (sections "Verifying stock-flow consistency in AI-written code" and "Testing and CI") found:
- BeforeIT's deterministic Julia version "exactly matches" the original Matlab and checks the national income identity after every step: a differential test against an independent implementation.
- `proptest-state-machine` applies random action sequences, checks invariants after every transition and shrinks failures to a minimal case.
- cargo-mutants (with `--in-diff` for PRs) exposes tests that pass whatever the code does.
- GitHub Free private repos get 2,000 Actions minutes a month. Linux costs $0.006/min, Windows $0.010/min, macOS $0.062/min. GitHub no longer documents whether Windows minutes count at a multiplier.

## Options considered
1. **Layered verification: invariants + differential reference model + property, metamorphic, golden and mutation tests.**
   Pros: each layer catches a different class of bug; wrong-but-balanced bugs are caught by the reference model.
   Cons: more CI time; a reference model to maintain.
2. **Invariants alone.**
   Pros: cheap.
   Cons: miss wrong-but-balanced bugs.
3. **Full three-OS matrix on every PR.**
   Pros: maximum coverage.
   Cons: macOS minutes cost 10× Linux; exhausts the free allowance.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1**, with a CI layout that fits the free allowance.

**Test layers**
| Layer | What it catches | Tooling |
|---|---|---|
| Unit tests | Local logic errors | cargo-nextest, plus `cargo test --doc` (nextest skips doctests) |
| Ledger invariants I-1 to I-7 | Leaks, unbalanced postings, GDP mismatch | `econ-ledger`, every tick in test builds |
| Differential SFC reference model | Wrong-but-balanced economics | Core "SIM mode" (one household sector, one firm sector, government, fixed propensities) vs an independent ~50-line Python Godley-Lavoie SIM, then PC. Compared on every PR. sfcr's `sfcr_validate` as a third check of flow matrices |
| State-machine property tests | Invariant breaks under random lever sequences | proptest + `proptest-state-machine` |
| Metamorphic tests (nightly) | Hidden assumptions | Scale all nominal values by 10 (real outputs unchanged); permute agent IDs (aggregates statistically unchanged); set a lever to its current value (bit-identical); run at several `sample_scale` values (per-capita aggregates agree within sampling error) |
| Golden runs | Unintended behaviour changes | Seed + command log → state hash per tick, committed |
| Snapshot tests | Reviewed changes to reports and explanations | insta on `TickReport` and `ContributionTree` |
| Mutation tests | Hollow tests | cargo-mutants `--in-diff` on PR diffs |
| Benchmarks | Performance regressions | divan |
| Nightly validation | Drift from calibration targets; instability | Targets file, Jacobian check ([ADR-0009](0009-calibration-and-stability.md)) |

The Python reference model is written from the spec in a separate agent session from the Rust code, so the two do not share mistakes.

**Population scale in CI.** The tiny scenario is built and run at three scales (for example 1:1000, 1:200 and 1:100). CI asserts that every invariant passes at each scale and that per-capita aggregates agree within sampling error. This proves nothing hardcodes the scale ([ADR-0003](0003-people-representation.md)).

**Golden changes need a reason.** Any change to a golden hash or snapshot needs a `CHANGELOG-sim.md` entry that gives the economic reason.

**CI layout (GitHub Actions)**
- Every PR, on Linux: fmt, clippy `-D warnings`, nextest, doctests, cargo-deny, pytest, ruff and pyright (strict), the codegen diff, spec traceability ([ADR-0014](0014-agent-workflow-guardrails.md)), the differential test and the 3-scale test.
- PRs that touch `crates/`: Windows core tests and golden hashes, plus one job that compares Linux and Windows state hashes byte for byte ([ADR-0006](0006-determinism-contract.md)).
- macOS: manual or monthly.
- Nightly: metamorphic tests, Jacobian check, long quiet-baseline runs.
- `Swatinem/rust-cache@v2` saving only on `main`; `concurrency: cancel-in-progress`; pinned `rust-toolchain.toml`.

**Budget.** About 6 Linux minutes plus 10 Windows minutes per PR; 60 agent PRs a month use about 960 minutes, which fits 2,000 minutes if Windows counts at 1×.

## Consequences
- Easier: bookkeeping bugs, wrong-but-balanced bugs, scale leaks, platform differences and hollow tests each have a dedicated check.
- Harder: two implementations of the SIM/PC models to keep in step; more CI minutes; nightly jobs to watch.
- If the minutes run out, options are: upgrade to GitHub Pro (+1,000 minutes), make the repo public (standard runners free), or trim Windows runs. See [ADR-0014](0014-agent-workflow-guardrails.md).

## Open questions / to verify
- [ ] Whether Windows minutes count at 1× or 2× on GitHub Free (check the usage report in the first month).
- [ ] cargo-mutants run time per PR once the codebase grows.
- [ ] Spike 2 exit criterion: a mutated sign in a posting is caught by the differential test, not only by invariants.
- [ ] Exact scales for the CI scale test (1:1000, 1:200, 1:100 proposed) and the statistical tolerance per aggregate.
