---
id: game/scenario-builder
title: Scenario Builder
status: draft
owner: horia
depends_on: [adr/0018-scenario-and-commands, economy/opening-balance-sheet, economy/accounting, society/population-generator, society/population-employers, adr/0016-firm-representation, adr/0012-data-pipeline-and-licensing, adr/0006-determinism-contract, adr/0003-people-representation]
research: [research/opening-balance-sheet-romania]
updated: 2026-10-13
---

# Scenario Builder

## Purpose
One command turns the pipeline's outputs into a scenario: the world on the start date (1 December 2021) at one `sample_scale`, as a folder of Arrow tables and small JSON files ([ADR-0018](../../03-architecture/decisions/0018-scenario-and-commands.md)). It also writes a consistency report. The report checks that every stock has a counterpart and that the totals match the published aggregates within agreed tolerances. The builder refuses to write a scenario whose opening balance sheet fails the ledger's invariants. This is roadmap M1's exit: `just build-scenario` at 1:1000, 1:100 and 1:10.

It replaces the interim `python/pipeline/build_scenario.py`, which runs the Python references. Its checks become the Rust builder's report, and the Python script stays as the builder's reference.

## Real-world basis
None of its own. Every number comes from a stage or file with its own spec: the population generator ([stages A and B](../society/population-generator.md) to [stage J](../society/population-employers.md)), the [firm units](../economy/industries.md) and their balance sheets, and the [opening balance sheet](../economy/opening-balance-sheet.md).

## Scope
**This increment:**
- the population tables;
- the firm-unit table;
- the opening balance sheet, posted through the ledger;
- `scenario.json`;
- the consistency report;
- `econ-cli build-scenario` and `just build-scenario`.

**Not in this increment:**
- `law.json` and `world.json`: they need the lever registry, roadmap M4. Until then `scenario.json` says `"law": null`, and a loader must refuse such a scenario for a run.
- `params.json`: calibration, roadmap M6.
- The input-output table as a scenario table: the production milestone, M2.
- Banks and public assets as tables: M9 and later.
- The ZIP form.

## Inputs
All from one fixtures directory (default `python/pipeline/fixtures`), plus `--sample-scale` (no default, [ADR-0003](../../03-architecture/decisions/0003-people-representation.md)) and `--seed` (default 42).

| File | Used for |
|---|---|
| `census2021_margins_ro.json`, `census2021_edu_activity_ro.json`, `census2021_jobs_ro.json`, `census2021_housing_ro.json`, `earnings2021_ro.json`, `pensions2021_ro.json`, `household_wealth2021_ro.json`, `other_incomes2021_ro.json`, `firm_units2021_ro.json` | population stages A to J |
| `firm_units2021_ro.json`, `firm_balance_sheets2021_ro.json` | the firm-unit table |
| `balance_sheets2021q4_ro.json`, `sector_details2021_ro.json` | the opening balance sheet |
| `io_table2021_ro.json` | checks only |

## Outputs
A folder `DIR`:

| File | Content |
|---|---|
| `scenario.json` | `format` (1), `name`, `start_date` "2021-12-01", `sample_scale`, `rng_seed`, `engine_version`, the hash of every stage (population, attributes, jobs, housing, wages, pensions, wealth, incomes, employers), the opening hash, the SHA-256 of every input file and of every other file of the folder, `law: null`, the provenance blocks of the inputs, `consistent` (bool) |
| `tables/households.arrow`, `tables/persons.arrow` | the columns of stages A to J, as `econ-cli synth-population` writes them with every stage |
| `tables/firm_units.arrow` | one row per firm unit (below) |
| `balances.arrow` | the opening balance sheet: one row per (instrument, sector) with `instrument` (u8), `sector` (u8), `balance` (i64, bani), in matrix order |
| `report.json` | the consistency report (below) |

**`firm_units.arrow`**, in the order of `firm_units2021_ro.json`:
- Identity and size: `unit_id` (string), `industry` (u8, catalogue index), `industry_group` (u8), `size` (u8: micro 0, small 1, large 2, public 3), `kind` (u8: cohort 0, public 1), `controller` (u8: domestic 0, foreign 1, state 2).
- `firm_count` (u32).
- Flows of 2021, in bani: `persons_employed` (u64), `output`, `value_added`, `wages` (u64).
- Ownership, in millionths: `own_state`, `own_domestic_private`, `own_foreign` (u32).
- Stocks of the unit's firms together, in bani: `fixed_assets`, `currency`, `deposits`, `debt_securities_held`, `loans_granted`, `equity_held`, `insurance`, `other_receivable`, `debt_securities_issued`, `loans`, `equity_issued`, `other_payable` (u64). These are zero for the 17 units without a balance sheet: banks, insurers and public units, whose stocks are in their sectors.
- `employees` (u64): weighted persons of the population whose `employer` is the unit.

Schema metadata gives each money column's unit (bani) and kind (stock or flow).

**The consistency report** has a list of named checks. Each check has `ok`, its figures and its tolerance, plus `known_gap` with a reason for a check that is expected to fail.

| Check | Passes when |
|---|---|
| `persons` | the weighted persons are the census's 19,053,815 |
| `wage_bill`, `pension_bill`, `mixed_income` | each is within 0.01% of its margin (earnings, pensions, mixed income) |
| `wages_against_input_output_table` | each industry group's year of wages over the table's compensation is in [0.85, 1.25] |
| `household_balance_sheet` | weighted household cash, deposits and loans are within 0.01% of the financial accounts |
| `deposits` | the firm units' deposits sum exactly to the firms' deposits, and households and firms hold no more deposits than banks owe |
| `employers` | every employed person has a unit of their industry group, and nobody else has one |
| `firm_units_against_input_output_table` | the units' output is within 0.01% of the table's |
| `income_against_consumption` | a year of household gross income over household consumption is in [0.8, 1.4] |
| `income_distribution` | each fifth's income share is within 0.08 of the survey's. Known gap until taxes and transfers apply (roadmap M3) |
| `declared_wages` | the declared share of wages is in [0.80, 0.97] |
| `ledger_invariants` | the opening ledger passes `check_invariants` (I-1 to I-3) |
| `households_against_ledger` | weighted household cash and deposits equal the ledger's household `cash` and `deposits` exactly (I-4) |
| `firms_against_ledger` | the firm units' currency and deposits sum to the ledger's firm `cash` and `deposits` exactly (I-4) |
| `left_out` | reports derivatives and other accounts by sector (always ok) |

These are the checks of the interim builder, with the same tolerances, plus the last four.

## Update rule
There is no per-day rule. `build-scenario` is a pure function of the input files, `sample_scale` and `rng_seed`:
1. Read and check every input; any rejection is a named error and nothing is written.
2. Run stages A to J ([population generator](../society/population-generator.md) and its stage specs) with `rng_seed`.
3. Build the firm-unit table: join `firm_units2021_ro.json` and `firm_balance_sheets2021_ro.json` by unit id. A balance sheet whose id is not a unit is an error. Count `employees` from stage J.
4. Compute the opening balances ([opening balance sheet](../economy/opening-balance-sheet.md)), reconcile the household column with the weighted household totals (its rule 8), and post them into a new `Ledger` with flow code `opening`. If `check_invariants` fails, stop with an error and write nothing.
5. Run the checks and write `report.json`. The other files are written to a temporary folder next to `DIR`, then renamed, so a failed build leaves no partial scenario.
6. Exit status: 0 if every check passes or is a known gap; 1 if a check fails (the scenario is still written, with `consistent: false`); 2 for bad arguments; 3 if the opening ledger fails its invariants (nothing written).

> **Simplification:** the opening balance sheet is at sector level. Households and firm units carry their own stocks in their tables, and I-4 checks the two agree. Sub-ledgers per household and per firm unit come with the mechanics that move those stocks.

## Player levers
None.

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `--seed` | 42 | the scenario's `rng_seed`; the player may choose another at "new game" (ADR-0018) |
| `--fixtures` | `python/pipeline/fixtures` | where the inputs are read (the DVC pipeline passes `data/derived`) |

Tolerances are those of the table above; changing one is an owner decision.

## Interactions
- Reads every stage of the population generator, the firm units and the opening balance sheet; writes what the real `World` (roadmap M2) loads.
- `just build-scenario SCALE [OUT]` runs it. CI builds the 1:1000 scenario from the committed fixtures (ADR-0018).

## Edge cases & failure modes
- Missing or malformed input → error naming the file; nothing written.
- `sample_scale` 0 or missing → usage error.
- `DIR` exists → it is replaced only after a successful build (step 5).
- 1:10 takes minutes (stage J and the stages before it), so it is not part of `just check`.

## Acceptance tests
IDs are stable. Tests in `crates/econ-scenario/tests/acceptance/` (protected), written in a test-authoring session. `[cli]` runs the binary on the committed fixtures.

- [ ] **AC-SCB-01** `[cli]` `econ-cli build-scenario --sample-scale 1000 --out DIR` exits 0 and writes `scenario.json`, `tables/households.arrow`, `tables/persons.arrow`, `tables/firm_units.arrow`, `balances.arrow` and `report.json`. `report.json` has every check of the table, and all pass except the known gap `income_distribution`.
- [ ] **AC-SCB-02** `[cli]` The same at 1:100. At 1:10 likewise, marked `#[ignore]` for CI time and run by `just build-scenario 10`.
- [ ] **AC-SCB-03** `[unit]` The stage hashes in `scenario.json` equal the golden files of the stages (`tests/golden/popgen_ro_census2021*.hashes`) at 1:1000 and 1:100, and the opening hash (after rule 8) equals the reference's at each scale.
- [ ] **AC-SCB-04** `[unit]` `balances.arrow` read back gives a ledger whose every instrument row sums to zero and whose balances equal the opening balance sheet's. Every balance was posted with flow code `opening`.
- [ ] **AC-SCB-05** `[unit]` `tables/firm_units.arrow` has one row per unit of `firm_units2021_ro.json`, in its order. Its stock columns sum to the `totals` of `firm_balance_sheets2021_ro.json`, and its `employees` sum to the weighted employed persons of the population.
- [ ] **AC-SCB-06** `[unit]` Weighted household cash and deposits of the tables equal the ledger's household `cash` and `deposits` exactly. The firm units' currency and deposits equal the ledger's firm `cash` and `deposits` exactly.
- [ ] **AC-SCB-07** `[unit]` Two builds with the same inputs, scale and seed produce byte-identical folders. A different seed changes the stage hashes and, through rule 8, only the households', the central bank's and the banks' balances of `cash` and `deposits`.
- [ ] **AC-SCB-08** `[unit]` If the opening balances are corrupted so that a row does not sum to zero (a test hook), the build exits 3 and writes nothing. A malformed input file gives a named error and writes nothing.
- [ ] **AC-SCB-09** `[diff]` The report's figures equal those of the reference `python/pipeline/build_scenario.py` (extended with the four new checks) at 1:1000: the same pass or fail for every check, and amounts equal to the leu.
- [ ] **AC-SCB-10** `[unit]` `scenario.json` lists the SHA-256 of every input file and of every file it wrote, and the hashes match the files.

## API sketch
A new crate `econ-scenario` (library; `econ-cli` calls it). It depends on `econ-popgen`, `econ-ledger` and `econ-io`.

```rust
pub struct BuildParams { pub sample_scale: u32, pub rng_seed: u64 }

/// Everything read from the fixtures directory.
pub struct Inputs { /* margin files of stages A..J, firm units, firm sheets, sector accounts, details, io totals */ }
pub fn read_inputs(dir: &Path) -> Result<Inputs, BuildError>;

pub struct Scenario {
    pub population: Population,
    pub stages: StageColumns,              // attributes, jobs, housing, wages, pensions, wealth, incomes, employers
    pub firm_units: FirmUnitTable,
    pub opening: OpeningBalances,          // economy/opening-balance-sheet
    pub ledger: Ledger,                    // after posting `opening`
    pub report: Report,
}
pub fn build(inputs: &Inputs, params: &BuildParams) -> Result<Scenario, BuildError>;
pub fn write(scenario: &Scenario, inputs: &Inputs, dir: &Path) -> Result<(), BuildError>;  // temp folder, then rename

pub struct Check { pub name: &'static str, pub ok: bool, pub known_gap: Option<&'static str>, pub facts: Value }
pub struct Report { pub checks: Vec<Check> }
impl Report { pub fn consistent(&self) -> bool; }   // every check ok or a known gap

pub enum BuildError {
    Input { file: String, why: String },
    Stage(String),                                  // a stage rejected its margins
    Opening(OpeningError),
    Invariants(Vec<InvariantError>),                // exit 3, nothing written
    Io(String),
}
```

`econ-cli build-scenario --sample-scale N [--seed S] [--fixtures DIR] --out DIR`. The justfile recipe is `build-scenario sample_scale out="python/pipeline/data/scenario"`.

## Open questions
- [ ] **A scenario without `law.json`** until M4: written with `law: null` and refused for a run, as proposed? Or wait for the registry before building scenarios at all?
- [ ] **Where the report's tolerances live:** in the code as constants (proposed), or in a small `checks.toml` that the owner edits.
- [ ] **CI:** build 1:1000 on every push (about a minute in release), or only in the nightly job?
- [ ] **Firm-unit stocks are cohort totals** (all firms of the unit together). Accounting's sub-ledgers per real firm divide them by `firm_count` with a carried remainder. Store totals now (proposed) and per-firm values when the firm mechanics need them?
