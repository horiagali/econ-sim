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
| `econ-popgen` | population generator from census margins (spec `society/population-generator`) and the stages after it: `attributes` (C, education and activity), `jobs` (D), `housing` (E, locality size and tenure), which share `assign` (split, deal, balance); `wages` (F), `pensions` (G), `wealth` (H), `incomes` (I, declared and undeclared income), `employers` (J, industry and firm unit) `inactivity` (K, kinds of inactivity and enrolment; feature `inactivity_enrolment`) and `emigrants` (L, linked emigrants and the diaspora stock; feature `linked_emigrants`, no Python twin yet), each behind a feature of its name, all on by default; acceptance tests in `tests/acceptance/` (protected). Integer arithmetic only; must stay identical, record for record, to `python/reference/popgen_reference.py` (AC-POP-07, AC-POPA-07, AC-POPJ-07, AC-POPH-07), so change both together |
| `econ-scenario` | scenario builder (spec `game/scenario-builder`): `opening` (the opening balance sheet), `firm_opening` (undeclared sales and inventories of the firm units), the build, the consistency report and the folder writer; `econ-cli build-scenario` calls it. Acceptance tests in `tests/acceptance/` (protected) |
| `econ-mech-demography` | demographics (spec `economy/demographics`): so far the starting tables (`Demography`: mortality, fertility, sex ratio at birth, retirement schedule) and the career proxy at load; pure functions |
| `econ-mech-transfers` | social transfers (pension points, unemployment claims, child allowance, minimum income); pure functions |
| `econ-core` | `World`, phases, `step`, `TickReport`, SIM and PC modes (`sim`, `pc`: Godley and Lavoie's models, compared exactly with `python/reference/` by `just diff-sim`); `schedule` (ADR-0017: `Period`, `Due`, `Schedule`, stagger days and `Cohorts`); `levers` (spec `game/lever-system`; feature `levers`, on by default since M4 with the policy suite's four lever features) with the registry `levers.toml` |
| `econ-io` | saves, migrations, replay; margin files in and population tables out (`popgen`, all stages; `people`, stages K and L and `demography.json`); `table` (plain columns as an Arrow IPC file, for the scenario builder). Arrow lives here only |
| `econ-cli` | headless runner: sim, golden checks, benchmarks, `synth-population` (`--attributes`, `--jobs`, `--housing` run stages C, D and E), `sim-population` (scale world on a generated population), `bench-days` (daily against monthly stepping) |
| `lint-canary` | NOT a member; must fail clippy (`just lint-canary`) |
