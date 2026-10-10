# AGENTS.md — Rust crates

Rules for any agent editing code under `crates/`. The root AGENTS.md also applies.

1. **Read [DETERMINISM.md](../DETERMINISM.md) first.** Its rules are not optional.
2. **Money moves only through `econ-ledger`.** Never add/subtract balances
   directly; create a `Transfer` with a `FlowCode`. Money is `Bani`, never `f64`.
3. **Never hardcode the population or firm scale.** Sizes come from the
   scenario; record weights (`hh_weight`, `firm_count`) multiply postings.
4. **Behaviour rules use `econ_rules::behaviour_rule!`** so every rule can
   explain itself (ADR-0008). Opaque rules need an explicit waiver.
5. **Tests are protected.** Do not edit `tests/golden/**` or
   `crates/*/tests/acceptance/**` (a hook blocks it). If a golden hash must
   change, stop and tell the owner why; they update `CHANGELOG-sim.md`.
6. Before saying you are done: `just check` must pass.

Acceptance tests: each spec criterion has an ID (`**AC-VAT-01**`); `just ac-stubs` generates ignored stubs, a test-authoring session fills them, `just trace` checks coverage. Don't put `#[must_use]` on fns returning `Bani` (already `must_use`; clippy `double_must_use`).

Crate map (ADR-0005):
| Crate | Holds |
|---|---|
| `econ-types` | `Bani`, IDs, `Month`; no dependencies |
| `econ-num` | `math` (libm wrappers), `det_sum`, LU / Leontief solver |
| `econ-rng` | `KeyedRng`, `Stream`, distributions |
| `econ-ledger` | accounts, flow codes, transfers, invariant checks |
| `econ-rules` | `behaviour_rule!` macro, contribution trees |
| `econ-mech-tax` | taxation mechanics (VAT so far); acceptance tests in `tests/acceptance/` (protected) |
| `econ-core` | `World`, phases, `step`, `TickReport`, SIM mode |
| `econ-cli` | headless runner: sim, golden checks, benchmarks |
| `lint-canary` | NOT a member; must fail clippy (`just lint-canary`) |
