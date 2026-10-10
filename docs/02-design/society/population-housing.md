---
id: society/population-housing
title: Population Generator, Stage E — Locality Size and Tenure
status: draft
owner: horia
depends_on: [society/population-generator, society/population-groups, adr/0006-determinism-contract, adr/0012-data-pipeline-and-licensing, adr/0014-agent-workflow-guardrails]
research: [research/population-modelling-deep-research]
updated: 2026-10-10
---

# Population Generator, Stage E — Locality Size and Tenure

## Purpose
Give every household of the starting population the size class of the locality it lives in (`hh_locality_size`, from which `hh_urban` follows) and, for private households, a tenure (`hh_tenure`: owner, tenant, other), so that the weighted population reproduces the Census 2021 tables county by county. It runs once, when a scenario is built, and needs only Stages A and B of the [generator](population-generator.md). [Housing](../economy/housing.md) starts from it.

## Real-world basis
- **Data (verified 2026-10-10, Eurostat Census 2021 round, free reuse with attribution),** both by county (NUTS 3):

| Table | Content |
|---|---|
| `cens_21l_r3` | persons by size of the locality they live in (13 classes), broad age group and sex |
| `cens_21hhct_r3` | private households by composition and tenure status (owner, tenant, other, unknown) |

- Romania, 1 December 2021. Persons by locality size: 1.25 million in localities under 2,000 inhabitants, 5.07 million in 2,000–4,999, 3.09 million in 5,000–9,999, 3.17 million in 10,000–49,999, 3.00 million in 50,000–199,999, 3.48 million in 200,000 or more. Households: 90.2% owners, 3.3% tenants, 5.7% other (mostly living rent-free in a relative's dwelling), 0.7% unknown. One-person households rent a little more often (4.4%) than others (2.8%).
- Both tables hold exactly the persons and the households of the county tables used by Stages A and B; the one-person households are those of the size table.
- The smallest localities are the oldest: 24.5% of their inhabitants are 65 or more, against 17.6% in localities of 5,000–9,999 and 20.0% in the largest cities.
- 287 cells of the locality table are suppressed (by age and sex in small classes). The tenure table has none.

> **Simplification:** `hh_urban` is "the locality has 10,000 inhabitants or more". The census as published by Eurostat has locality size, not the administrative status (municipality or town against commune) that Romanian statistics call urban. The threshold gives 50.6% urban; the official share is about 52%. INS publishes the administrative split per county; it can replace this rule.

## Scope
**This increment:** `hh_locality_size` (six classes), `hh_urban`, `hh_tenure` (three values).

**Not in this increment:** owner with or without a mortgage, private against social renting (the five tenure values of [population-groups](population-groups.md); they need loan and housing-stock data), `hh_dwelling_value`, the dwelling itself.

| `hh_tenure` here | population-groups |
|---|---|
| `owner` | owner outright, owner with mortgage |
| `tenant` | private renter, social renter |
| `other` | rent-free (family) |

> **Simplification:** tenure depends on the county and on whether the household is one person or larger, not on age or income. Young households rent more than the rule gives them.

> **Simplification:** a household is placed by the age of its head. Children and other members follow, so persons by their own age and locality size match the census only roughly (see the tolerances).

## Inputs
- The population of Stages A and B (households with `hh_weight`, `hh_county`, `hh_collective`; persons with `household_id`, `age`, `role`). **It is not changed.**
- Margin tables of integer counts, written by the pipeline's normalise stage with provenance:

| Table | Cells | Meaning |
|---|---|---|
| `persons_by_locality[county, age_group, locality_class]` | 42 × 6 × 6 | all residents; age groups 0–14, 15–29, 30–49, 50–64, 65–84, 85+ |
| `households_by_tenure[county, size_group, tenure]` | 42 × 2 × 3 | private households; size groups are one person, and larger |

- `rng_seed`.

**Locality classes:** 0 under 2,000 inhabitants, 1 2,000–4,999, 2 5,000–9,999, 3 10,000–49,999, 4 50,000–199,999, 5 200,000 or more. Urban from class 3.

**Suppressed cells and unknowns** are handled by the normalise stage. A locality class that a county has nobody in stays zero; inside each county, sex and age group, what the known cells leave over of the published total is spread evenly over the suppressed cells. Households of unknown tenure are spread over the three tenures in proportion. The stage checks both tables against the county margins and stops if they differ.

## Outputs
Three columns aligned with the households table, and their hash.

| Column | Values |
|---|---|
| `hh_locality_size` | 0–5 as above |
| `hh_urban` | 1 if `hh_locality_size` ≥ 3, else 0 |
| `hh_tenure` | 0 owner, 1 tenant, 2 other; 255 for a record that is not a private household |

## Update rule
There is no per-tick rule. The stage is a pure function of (population, tables, `rng_seed`), in integer arithmetic. "Deal" and "balance" are those of [Stage D](population-jobs.md), with the same pattern floor (1), at most 200 passes and the same stopping rule.

**Priority.** `draw(PopulationGen, tick 4, entity = index in the households table)`; its first `u64`. Households are always taken in ascending (priority, index). (Tick 3 is used by the scale world.)

For each county, with the county's households in that order:

### Step 1 — locality size
A household counts as its weight times its members (the persons it stands for). Its row is the age group of its head (for a record that is not a private household: of its one member).
1. **Balance** the county's census table (rows: age groups; columns: locality classes) against the persons that the households of each row stand for and the county's census persons by class.
2. Row by row, from the youngest age group, **deal** the households of the row over the classes, with their row as targets. The carry starts at zero in each county and passes from row to row.

The county then has the census number of persons in every locality class (up to rounding by whole households), and households with old heads are where the old live.

### Step 2 — tenure
For private households only. The carry starts at zero in each county. First the one-person households, then the larger ones: **deal** them (a household counts as its weight) over the three tenures with the census counts of that county and size group as targets. If the census has no household of that size group in the county, the targets are the county's counts over both groups.

**Housing hash** (for golden and differential tests): FNV-1a 64 over the household count (`u32`, little-endian), then each household's `hh_locality_size`, `hh_urban` and `hh_tenure` (one byte each).

## Player levers
None.

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| `urban_from_class` | 3 (10,000 inhabitants) | 1–5 | Where urban begins. Read from the margin file |
| `pattern_floor`, balancing passes, stop | as in Stage D | | |

## Interactions
- **Before:** Stages A and B; the pipeline's fetch and normalise stages. Independent of Stages C and D.
- **After:** [housing](../economy/housing.md) (prices and rents by county and locality size), [interest groups](interest-groups.md) (farmers, motorists), [infrastructure](../economy/infrastructure.md), wealth imputation (an owner's dwelling is most of a household's wealth).
- A county's employment and education do not yet depend on locality size: Stages C and D work by region. A later correction can use this column.

## Edge cases & failure modes
- **A county with one locality class** (Bucharest): every household gets it.
- **A class the county does not have:** its target is zero, so no household receives it.
- **A county whose census table is empty while it has households, or no tenure counts at all:** the margins do not belong to this population; rejected with a named error.
- **Records that are not private households** get a locality class and no tenure.
- **Counties of the margin file differ from those of the population, or a table's length does not match its dimensions:** rejected with a named error; nothing is produced.

## Acceptance tests
IDs are stable. Tests go in `crates/econ-popgen/tests/acceptance/housing.rs` (protected; to be written in a test-authoring session, before the Rust code), compiled only with the crate feature `housing`. "The fixtures" are the county margin file and the housing margin file committed with the pipeline. "Three scales" means 1:1000, 1:100 and 1:10, seed 42.

- [ ] **AC-POPH-01** `[unit]` Structure, at each of the three scales: one value of each column per household; `hh_locality_size` is 0 to 5; `hh_urban` is 1 exactly when `hh_locality_size` ≥ 3; a private household has `hh_tenure` 0 to 2 and any other record 255; no household is in a class its county has nobody in.
- [ ] **AC-POPH-02** `[unit]` The stage does not change the population: the state hash is that of AC-POP-06.
- [ ] **AC-POPH-03** `[unit]` At each of the three scales the weighted margins are within the tolerances of the table below.
- [ ] **AC-POPH-04** `[unit]` At each of the three scales, nationally, the share of persons aged 65 or more is highest in the smallest locality class, and in every class it is within 4 percentage points of the census share.
- [ ] **AC-POPH-05** `[unit]` Same population, margins and seed give identical columns (equal housing hash). A different seed changes which households are where, and the national totals by locality class and by tenure stay within the tolerance of AC-POPH-03.
- [ ] **AC-POPH-06** `[golden]` The housing hash of the fixture population at each of the three scales equals the committed golden value.
- [ ] **AC-POPH-07** `[diff]` On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns, household for household.
- [ ] **AC-POPH-08** `[unit]` Margins for other counties than the population's, a table whose length does not match its dimensions, no locality classes, and a county whose census table is empty are each rejected with a named error and produce no output.

**Tolerances for AC-POPH-03.** Relative error of the weighted total against the census, by expected synthetic records behind the cell (census persons, or households for tenure, ÷ `sample_scale`). Margins checked, for persons by locality class and by urban: national; county. For households by tenure: national; national × size group; county; county × size group.

| Expected synthetic records behind the cell | Tolerance | Worst measured (reference, three scales and 1:200) |
|---|---|---|
| 1,000 or more | 1% | 0.6% |
| 100 to 999 | 5% | 2.6% |
| 30 to 99 | 15% | 9.0% |
| fewer than 30 | not checked | — |

Locality cells are counted in persons but filled with whole households, and one synthetic household of six stands for six synthetic persons. That is why small cells move more here than in Stage A.

Measured on 2026-10-10 with the Python reference on the fixtures (seed 42):

| Largest error in cells of 1,000 records or more (100 or more where none is larger) | 1:1000 | 1:100 | 1:10 |
|---|---|---|---|
| National, persons by locality class | 0.6% | 0.0% | 0.0% |
| National, persons urban and rural | 0.2% | 0.0% | 0.0% |
| County, persons by locality class | 0.0% | 0.2% | 0.2% |
| National, households by tenure | 0.0% | 0.1% | 0.0% |
| County, households by tenure | 0.6% | 0.0% | 0.1% |
| Share of persons of 65 or more in a locality class: largest difference from the census (AC-POPH-04) | 1.2 points | 2.1 points | 2.0 points |

Hashes of the reference, for the test writer to record as golden values (the state hash is unchanged):

| Scale | Housing hash |
|---|---|
| 1:1000 | `a4b8ac6c2c2bb24e` |
| 1:200 | `67d899c333d94500` |
| 1:100 | `82d0d447a02d4b5e` |
| 1:10 | `3770cee8c4e06057` |

## API sketch
For the test writer and the implementer. Names may change before lock; shapes should not. The skeleton in `crates/econ-popgen/src/housing.rs` is the definition.

```rust
// crate econ-popgen, module `housing` (no I/O), re-exported at the crate root
pub const TENURES: usize = 3;                // owner, tenant, other
pub const SIZE_GROUPS: usize = 2;            // one person, larger
pub const NO_TENURE: u8 = 255;               // a record that is not a private household
pub struct HousingMargins {
    pub counties: Vec<String>,               // must equal `Margins::counties`
    pub locality_classes: u8,                // number of classes
    pub urban_from_class: u8,
    pub age_group_from: Vec<u16>,            // first age (years) of each age group, ascending from 0
    pub persons_by_locality: Vec<u64>,       // [county][age_group][locality_class], row-major
    pub households_by_tenure: Vec<u64>,      // [county][size_group][tenure], row-major
}
pub struct HousingParams { pub rng_seed: u64, pub pattern_floor: u64, pub balancing_passes: u32, pub balance_stop_ppm: u32 }
impl HousingParams { pub fn new(rng_seed: u64) -> Self; }                   // defaults 1, 200, 100
pub struct HouseholdHousing { pub hh_locality_size: Vec<u8>, pub hh_urban: Vec<bool>, pub hh_tenure: Vec<u8> }
pub enum HousingError { EmptyTable, ShapeMismatch, NoMargin { county: String } }

pub fn assign_housing(pop: &Population, margins: &HousingMargins, params: &HousingParams)
    -> Result<HouseholdHousing, HousingError>;
impl HouseholdHousing { pub fn state_hash(&self) -> u64; }
```

- Command line: `econ-cli synth-population … --housing FILE` adds the three columns to `households.arrow` and the housing hash to `fit_report.json`.
- Margin file: JSON with `counties`, `locality_classes`, `locality_min`, `urban_from_class`, `age_group_from`, `size_groups`, `tenures`, the two tables, `totals` and `provenance`.
- Python reference: `python/reference/popgen_reference.py`, functions `assign_housing` and `housing_hash` (exists; `--housing FILE` on its command line adds the three columns and `summary.housing_hash`). Where this text and the reference disagree on a detail of ordering or tie-breaking, the reference is the definition until the spec is locked.
- Margin fixture: `python/pipeline/fixtures/census2021_housing_ro.json`, built by `python/pipeline/normalise_census_housing.py`.

## Open questions
- [ ] **Tolerances** (1%, 5%, 15%): to be accepted by the owner.
- [ ] **Urban = 10,000 inhabitants or more.** Keep the rule, or get the administrative urban/rural split per county from INS (the request that is already planned for ethnicity and religion could include it)?
- [ ] **Tenure independent of age.** The census table used here has tenure by household composition; composition in the seed is rule-made (generator spec), so it is not used beyond "one person or larger". Revisit with the IPUMS seed.
- [ ] **Six locality classes.** Enough for housing and services, or keep all thirteen census classes?
- [ ] Should Stages C and D (education, activity, jobs) later be corrected by locality size? Villages have more farmers and fewer graduates than their region's average.
