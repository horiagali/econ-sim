---
id: society/population-jobs
title: Population Generator, Stage D — Jobs
status: draft
owner: horia
depends_on: [society/population-attributes, society/population-generator, society/population-groups, adr/0006-determinism-contract, adr/0012-data-pipeline-and-licensing, adr/0014-agent-workflow-guardrails]
research: [research/population-modelling-deep-research]
updated: 2026-10-10
---

# Population Generator, Stage D — Jobs

## Purpose
Give every employed person of the starting population a status in employment (`employment_status`: employee, employer, own-account worker, family worker), an occupation (`occupation`) and an industry group (`industry_group`: ten groups of NACE sections), so that the weighted population reproduces the Census 2021 tables by development region, sex and age. It runs after [Stage C](population-attributes.md), once, when a scenario is built. The [labour market](../economy/labor-market.md) starts from these jobs.

## Real-world basis
- **Margins (verified 2026-10-10, Eurostat Census 2021 round, free reuse with attribution),** both by development region (NUTS 2), sex and five-year age band:

| Table | Content |
|---|---|
| `cens_21empn_r2` | employed persons by status in employment and economic activity (ten groups of NACE Rev. 2 sections) |
| `cens_21empo_r2` | employed persons by status in employment and occupation (ISCO-08 major groups) |

- Romania, 1 December 2021, of 7,689,171 employed: 6.55 million employees, 0.10 million employers, 0.81 million own-account workers, 0.23 million family workers. Half of the own-account workers and most family workers are in agriculture. By industry group: trade, transport and hospitality 2.10 million; industry 1.73 million; public administration, education and health 1.17 million; agriculture 0.89 million; construction 0.76 million.
- Both tables hold the employed of the Stage C activity table in every region, sex and age band, up to a few persons moved by suppression. Nobody has an unknown status, industry group or occupation. The census reports nobody in the armed-forces occupation group.
- The census does not say which occupations go with which education, or with which industry group. **Patterns** for that come from the EU Labour Force Survey 2021 (`lfsa_egised`: occupation by education; `lfsa_eisn2`: occupation by NACE section), by sex.

> **Simplification:** the patterns are **EU-27 totals**, not Romanian. Most cells of the Romanian tables are not published (too few respondents). Only the proportions of the patterns are used (how much more often a graduate is a professional than a clerk); every level comes from the Romanian census. Confidence: medium. A Romanian pattern from LFS microdata or the IPUMS sample replaces them later.

## Scope
**This increment:** `employment_status`, `occupation` (ten ISCO-08 major groups) and `industry_group` (ten NACE groups) for every person whose `activity` is `employed`.

**Not in this increment:** the industry of the [goods catalogue](../economy/industries.md) (about 90; assigned inside the industry group when the firm table exists), the employer (firm unit), `employer_type`, `formal`, `wage`, `hours`.

`employment_status` maps to four of the twelve `activity` values of [population-groups](population-groups.md): employee, employer, self-employed (own-account) and unpaid family worker. It is a separate column here so that Stage C's output is not changed.

> **Simplification:** as in Stage C, jobs are given to persons one by one. Members of a household do not share a farm or a family business, and partners' occupations are unrelated.

> **Simplification:** status in employment does not depend on education inside a region, sex and age band. Education acts through the occupation.

## Inputs
- The population of Stages A and B and the attributes of Stage C. **Neither is changed.**
- Margin and pattern tables, written by the pipeline's normalise stage with provenance:

| Table | Cells | Meaning |
|---|---|---|
| `by_industry_group[region, sex, age_band, kind, industry_group]` | 8 × 2 × 21 × 4 × 10 | census, real persons |
| `by_occupation[region, sex, age_band, kind, occupation]` | 8 × 2 × 21 × 4 × 10 | census, real persons |
| `pattern_edu_occupation[sex, edu_group, occupation]` | 2 × 3 × 10 | LFS, EU-27, persons; proportions only |
| `pattern_occupation_industry_group[sex, occupation, industry_group]` | 2 × 10 × 10 | LFS, EU-27, persons; proportions only |
| `edu_group_of_level[edu_level]` | 5 | the LFS education group (ISCED 0–2, 3–4, 5–8) of each `edu_level` |

- `rng_seed`.

**Occupations** are numbered by their ISCO-08 major group: 0 armed forces, 1 managers, 2 professionals, 3 technicians, 4 clerks, 5 service and sales, 6 skilled agricultural, 7 craft, 8 plant and machine operators, 9 elementary. **Industry groups:** 0 agriculture (A), 1 industry (B–E), 2 construction (F), 3 trade, transport, hospitality (G–I), 4 information and communication (J), 5 finance (K), 6 real estate (L), 7 professional and administrative services (M–N), 8 public administration, education, health (O–Q), 9 other services (R–U).

**Suppressed cells** are filled by the normalise stage: inside each region, sex and age band, what the known cells leave over of the employed total is spread evenly over the suppressed cells. About 1,400 persons are moved in each table. LFS cells that are not published count as zero. The normalise stage checks both census tables against the activity table of Stage C and stops if a cell differs by more than 25 persons.

## Outputs
Three columns aligned with the persons table, and their hash.

| Column | Values |
|---|---|
| `employment_status` | 0 not employed, 1 employee, 2 employer, 3 own-account worker, 4 family worker |
| `occupation` | 0–9 as above; 255 not employed |
| `industry_group` | 0–9 as above; 255 not employed |

## Update rule
There is no per-tick rule. The stage is a pure function of (population, Stage C attributes, tables, `rng_seed`), in integer arithmetic. Notation, apportioning and the carry are those of [Stage C](population-attributes.md).

**Priority.** `draw(PopulationGen, tick 2, entity = index in the persons table)`; its first `u64`. Persons are always taken in ascending (priority, index).

**Deal** (used three times below; replaces Stage C's "split" for groups that may hold one person). Given the members of a group in order, a target per category and a carry per category:
1. Apportion the group's weight over the targets and add the carry. This is what each category is owed.
2. Each member in turn goes to the category that is still owed the most (the lowest index on a tie), among the categories whose target is above zero; what that category is owed falls by the member's weight.
3. What is still owed, positive or negative, is the new carry.

No category is ever owed more than about one person, however small the groups. (Stage C's rule deals whole blocks, which it needs to give persons in education the highest levels; with single-person groups it would starve small categories.)

**Balance** (used twice; the "two-way balancing" of Stage C step 1, stated generally). Given a pattern table, a weight per row and a target per column: scale the pattern in millionths in alternating passes, columns to the targets (apportioned to the total weight) and rows to their weights; then apportion each row's weight over its row. Stage C always makes 20 passes. This stage makes at most 200 and stops after a pass (columns, then rows) once every column is within 100 millionths of the group's weight of its target: Romania's agriculture is far from the EU pattern, and 20 passes do not get there. Rows are exact; columns are as close as the rows allow; the proportions inside the table stay those of the pattern as far as both allow. Every cell of a pattern counts one person more than published (`pattern_floor`), so no combination is impossible.

**Census counts of a cell.** Where the census has nobody in a region, sex, age band and status, but a synthetic person is there, the counts of the same region, sex and status over all age bands are used; failing that, over all statuses. If the census has nobody employed in the region and sex at all, the margins do not belong to this population: rejected.

### Step 1 — status in employment
For each region and sex, carry starting at zero, through the age bands: **deal** the band's employed persons over the four statuses, with the census counts by status as targets.

### Step 2 — occupation, from education
For each region, sex and status, carry starting at zero, through the age bands. In one band: **balance** the pattern (rows: the five education levels, each with the pattern row of its LFS group; columns: occupations) against the weight of the band's persons at each level and the census counts by occupation. Then, level by level, **deal** the persons of that level over the occupations with their row as targets. The carry passes from level to level and from band to band.

### Step 3 — industry group, from occupation
The same, with the occupation just assigned as the rows: **balance** `pattern_occupation_industry_group` against the weight of the band's persons in each occupation and the census counts by industry group, then **deal** occupation by occupation.

The result has the census number of persons in every status, occupation and industry group of a region, sex and age band (up to rounding), and inside those totals graduates are professionals and farmers are in agriculture as often as the patterns say.

**Jobs hash** (for golden and differential tests): FNV-1a 64 over the person count (`u32`, little-endian), then each person's `employment_status`, `occupation` and `industry_group` (one byte each).

## Player levers
None.

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| `pattern_floor` | 1 person | 0–1000 | Added to every pattern cell. Larger values weaken the link between education, occupation and industry group |
| Balancing passes (upper bound) | 200 | 20–2000 | More passes fit the census columns more closely where the pattern is far from Romania |
| `balance_stop_ppm` | 100 | 0–10000 | A column fits when it is within this many millionths of the group's weight; passes stop when all fit |

## Interactions
- **Before:** Stages A to C; the pipeline's fetch and normalise stages.
- **After:** income and wealth; the [labour market](../economy/labor-market.md) (which also gives each person an employer and a catalogue industry inside the industry group); [informal economy](../economy/informal-economy.md) (own-account and family workers in agriculture are its starting point); [taxation](../economy/taxation.md) and [social transfers](../economy/social-transfers.md).

## Edge cases & failure modes
- **A census cell with nobody, where a synthetic person is:** see "Census counts of a cell".
- **An occupation the census has nobody in** (armed forces): its target is zero everywhere, so nobody receives it.
- **A pattern row of zeros:** cannot happen with `pattern_floor` ≥ 1. With 0, a level whose pattern row is empty has no row to scale and the stage would deal by the census columns alone.
- **Tables whose length does not match their dimensions, no regions, a county without a region, Stage C attributes of another population (different length):** rejected with a named error; nothing is produced.

## Acceptance tests
IDs are stable. Tests go in `crates/econ-popgen/tests/acceptance/jobs.rs` (protected; to be written in a test-authoring session, before the Rust code), compiled only with the crate feature `jobs`. "The fixtures" are the three normalised margin files committed with the pipeline. "Three scales" means 1:1000, 1:100 and 1:10, seed 42.

- [ ] **AC-POPJ-01** `[unit]` Structure, at each of the three scales: one value of each column per person; a person is employed in Stage C exactly when `employment_status` is 1 to 4, and then `occupation` and `industry_group` are 0 to 9; otherwise the three values are 0, 255, 255.
- [ ] **AC-POPJ-02** `[unit]` The stage changes neither the population nor the Stage C attributes: the state hash and the attribute hash are those of AC-POP-06 and AC-POPA-06.
- [ ] **AC-POPJ-03** `[unit]` At each of the three scales the weighted margins are within the tolerances of the table below.
- [ ] **AC-POPJ-04** `[unit]` The links follow the patterns, at each of the three scales, nationally: more than 70% of professionals have `edu_level` ≥ 3; fewer than 20% of persons in elementary occupations do; more than 60% of skilled agricultural workers are in agriculture; the largest industry group of professionals is public administration, education and health.
- [ ] **AC-POPJ-05** `[unit]` Same inputs and seed give identical columns (equal jobs hash). A different seed changes who has which job, and the national totals by status, occupation and industry group stay within the tolerance of AC-POPJ-03.
- [ ] **AC-POPJ-06** `[golden]` The jobs hash of the fixture population at each of the three scales equals the committed golden value.
- [ ] **AC-POPJ-07** `[diff]` On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns, person for person.
- [ ] **AC-POPJ-08** `[unit]` Margins with no regions, a table whose length does not match its dimensions, a county index without a region, attributes of a different length, and margins with nobody employed in a region and sex that has employed synthetic persons are each rejected with a named error and produce no output.

**Tolerances for AC-POPJ-03.** Relative error of the weighted total against the census, by expected synthetic records behind the cell, as in Stage C. Margins checked, for status: national; national × sex; region; region × sex × age band. For industry group and for occupation: national; national × sex; national × status; national × age band; region; region × sex; region × sex × age band × status.

| Expected synthetic records behind the cell | Tolerance | Worst measured (reference, three scales and 1:200) |
|---|---|---|
| 1,000 or more | 1% | 0.6% |
| 100 to 999 | 10% | 8.1% |
| 30 to 99 | 25% | 16.6% |
| fewer than 30 | not checked | — |

The small-cell tolerances are wider again than in Stage C. Two rounding errors add up here: Stage C's count of employed persons in the cell, and this stage's dealing. The worst cells are small industry groups in the oldest age bands, which inherit the carry of much larger bands before them: a cell of 40 synthetic persons that is 6 or 7 off.

Measured on 2026-10-10 with the Python reference on the fixtures (seed 42):

| Largest error in cells of 1,000 records or more (100 or more at 1:1000 where none is larger) | 1:1000 | 1:100 | 1:10 |
|---|---|---|---|
| National, by status in employment | 0.1% | 0.1% | 0.0% |
| National, by occupation | 0.4% | 0.1% | 0.1% |
| National, by industry group | 0.3% | 0.2% | 0.4% |
| Region, by occupation | 1.5% | 0.3% | 0.1% |
| Region, by industry group | 1.4% | 0.2% | 0.1% |
| All checked margins, 1,000 records or more | 0.5% | 0.6% | 0.4% |

Hashes of the reference, for the test writer to record as golden values (state and attribute hashes are unchanged):

| Scale | Jobs hash |
|---|---|
| 1:1000 | `7e3499aa1c44e7f1` |
| 1:200 | `d691cafeb19d9537` |
| 1:100 | `958d54dd9c467f42` |
| 1:10 | `6044b75c24d89f60` |

## API sketch
For the test writer and the implementer. Names may change before lock; shapes should not. The skeleton in `crates/econ-popgen/src/jobs.rs` is the definition.

```rust
// crate econ-popgen, module `jobs` (no I/O), re-exported at the crate root
pub const KINDS: usize = 4;                  // employee, employer, own-account, family worker
pub const OCCUPATIONS: usize = 10;           // ISCO-08 major groups, by digit
pub const INDUSTRY_GROUPS: usize = 10;               // NACE groups A, B-E, F, G-I, J, K, L, M-N, O-Q, R-U
pub const EDU_GROUPS: usize = 3;             // ISCED 0-2, 3-4, 5-8
pub const NOT_EMPLOYED: u8 = 255;            // occupation and industry group of a person who is not employed
pub struct JobMargins {
    pub regions: Vec<String>,
    pub region_of_county: Vec<u8>,           // as in AttributeMargins
    pub age_bands: Vec<(u16, u16)>,
    pub by_industry_group: Vec<u64>,                 // [region][sex][age_band][kind][industry_group], row-major
    pub by_occupation: Vec<u64>,             // [region][sex][age_band][kind][occupation]
    pub pattern_edu_occupation: Vec<u64>,    // [sex][edu_group][occupation]
    pub pattern_occupation_industry_group: Vec<u64>, // [sex][occupation][industry_group]
    pub edu_group_of_level: [u8; EDU_LEVELS],
}
pub struct JobParams { pub rng_seed: u64, pub pattern_floor: u64, pub balancing_passes: u32, pub balance_stop_ppm: u32 }
impl JobParams { pub fn new(rng_seed: u64) -> Self; }                       // defaults 1, 200, 100
pub struct PersonJobs { pub employment_status: Vec<u8>, pub occupation: Vec<u8>, pub industry_group: Vec<u8> }
pub enum JobError { EmptyTable, ShapeMismatch, UnknownRegion { county: u8 }, NoMargin { region: String } }

pub fn assign_jobs(pop: &Population, attrs: &PersonAttributes, margins: &JobMargins, params: &JobParams)
    -> Result<PersonJobs, JobError>;
impl PersonJobs { pub fn state_hash(&self) -> u64; }
```

- Command line: `econ-cli synth-population … --attributes FILE --jobs FILE` adds the three columns to `persons.arrow` and the jobs hash to `fit_report.json`.
- Margin file: JSON with `regions`, `sexes`, `age_bands`, `kinds`, `occupations`, `industry_groups`, `edu_levels`, `edu_groups`, `edu_group_of_level`, the four tables, `totals` and `provenance`. The county-to-region map is the one of the education-and-activity file.
- Python reference: `python/reference/popgen_reference.py`, functions `assign_jobs`, `deal`, `balance` and `jobs_hash` (exists; `--jobs FILE` on its command line, together with `--attributes FILE`, adds the three columns and `summary.jobs_hash`). Where this text and the reference disagree on a detail of ordering or tie-breaking, the reference is the definition until the spec is locked.
- Margin fixture: `python/pipeline/fixtures/census2021_jobs_ro.json`, built by `python/pipeline/normalise_census_jobs.py`.

## Open questions
- [ ] **Tolerances** (1%, 10%, 25%): to be accepted by the owner.
- [ ] **EU-27 patterns for Romania.** Acceptable until Romanian microdata is available? Romania differs most in agriculture, where the census margins already do the work.
- [ ] **Status in employment independent of education** (second simplification). Employers are in reality better educated than own-account farmers; the occupation step recovers part of this. Add an education × status pattern (LFS `lfsa_egaed` has status by education)?
- [ ] **Armed forces:** the census reports nobody in ISCO group 0 (the military are counted elsewhere). Leave empty, or move a number from public administration by rule?
- [ ] `employment_status` as its own column, or folded into `activity` as [population-groups](population-groups.md) lists it? Separate keeps Stage C's output and hash fixed.
- [x] **Name of the ten NACE groups: `industry_group`**, not "sector", which in this project means an institutional sector (households, firms, government; see [accounting](../economy/accounting.md)). The catalogue industry is `industry`.
