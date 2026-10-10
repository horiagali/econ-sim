---
id: society/population-attributes
title: Population Generator, Stage C — Education and Activity
status: draft
owner: horia
depends_on: [society/population-generator, society/population-groups, adr/0006-determinism-contract, adr/0012-data-pipeline-and-licensing, adr/0014-agent-workflow-guardrails]
research: [research/population-modelling-deep-research]
updated: 2026-10-10
---

# Population Generator, Stage C — Education and Activity

## Purpose
Give every person of the starting population an education level (`edu_level`) and an activity (`activity`), so that the weighted population reproduces the Census 2021 tables by development region, sex and age. It is the second increment of the [population generator](population-generator.md) and runs after its Stages A and B, once, when a scenario is built. The [labour market](../economy/labor-market.md) and [education](education.md) cannot start without these two attributes.

## Real-world basis
- **Data (verified 2026-10-10, Eurostat Census 2021 round, free reuse with attribution).** Both tables exist only for development regions (NUTS 2), not for counties.

| Table | Content | Used for |
|---|---|---|
| `cens_21a_r2` | persons by activity status (employed, unemployed, in education, retired or living on capital income, other inactive, below the minimum working age), sex and **single year of age** | who does what at each age |
| `cens_21ae_r2` | persons by labour status (employed, unemployed, inactive), **educational attainment** (ISCED 2011 levels 0 to 8), sex and five-year age band | education, jointly with labour status |

- Romania, 1 December 2021: 7,689,171 employed, 495,848 unemployed, 1,132,190 in education, 4,410,077 retired, 2,252,627 other inactive, 3,073,902 under 15. By education: 7.72 million at most lower secondary (including everyone under 15), 7.55 million upper secondary, 0.74 million post-secondary, 1.36 million short-cycle or bachelor, 1.69 million master or doctorate.
- Both tables hold exactly the persons of the county tables used by Stages A and B, for every region, sex and age band. No person has an unknown status or level.
- 679 cells of the education table and 350 of the activity table are suppressed for confidentiality. They hold about 700 persons in all.
- The census counts "retired persons and capital income recipients" as one category.

## Scope
**This increment:** `edu_level` (five levels) and `activity` (seven values) for every person, including people not in private households.

**Not in this increment** (each needs its own data): the split of the employed into employee, self-employed, employer and unpaid family worker, with occupation and industry (`cens_21empn_r2`, the next increment that touches jobs); the split of other inactive persons into homemaker, disabled and other; `edu_field`, `enrolled`, `years_in_level`.

How the seven values map to the twelve of [population-groups](population-groups.md):

| `activity` here | population-groups | Rule |
|---|---|---|
| `child` | child | census "below the minimum working age", younger than `school_age` |
| `pupil` | pupil | the same census category from `school_age` on; or census "in education" with at most lower secondary completed |
| `student` | student | census "in education" with upper secondary or more completed |
| `employed` | employee, self-employed, employer, unpaid family worker | split by a later increment |
| `unemployed` | unemployed | |
| `retired` | retired | includes people living on capital income |
| `inactive_other` | inactive-homemaker, inactive-disabled, inactive-other | split by a later increment |

> **Simplification:** attributes are given to persons one by one. Who lives with whom plays no part, so partners are not more alike in education than strangers, and a household's members are not more often jobless together. An observed seed (IPUMS, [ADR-0003](../../03-architecture/decisions/0003-people-representation.md)) carries these links; then this stage only corrects totals.

> **Simplification:** every child from `school_age` to 14 is a pupil. Real enrolment at those ages is high but not complete.

> **Simplification:** people in institutions get attributes by the same rule as everyone else of their region, sex and age.

## Inputs
- The population of Stages A and B: households (`hh_weight`, `hh_county`) and persons (`household_id`, `age`, `sex`). **It is not changed.** Weights, households and the state hash of the [generator spec](population-generator.md) stay exactly as they are.
- Margin tables of **integer counts of real persons**, written by the pipeline's normalise stage with provenance:

| Table | Cells | Meaning |
|---|---|---|
| `activity[region, sex, year_of_age, margin_activity]` | 8 × 2 × 101 × 6 | year of age 0 to 99, then 100 or over; margin activity is child, in education, employed, unemployed, retired, other inactive |
| `education[region, sex, age_band, status, edu_level]` | 8 × 2 × 21 × 3 × 5 | age bands as in the generator; status is employed, unemployed, inactive |
| `region_of_county[county]` | 42 | the region each county belongs to |

- `rng_seed` (the scenario's seed, the same as in Stages A and B).

**Education levels.** ISCED 0–2 (at most lower secondary; also everyone under 15, whose level the census gives as "not applicable"), 3 (upper secondary), 4 (post-secondary non-tertiary), 5–6 (short-cycle tertiary and bachelor), 7–8 (master and doctorate).

**Suppressed cells.** The normalise stage fills them, so this stage sees complete tables. Inside each published total (region × sex × age), what the known cells leave over is spread evenly over the suppressed cells by largest remainder. Cells that must be zero are set to zero first: nobody under 15 has an activity or a level, nobody of 15 or more is below the minimum age. The normalise stage also checks that both tables sum to the county margins in every region, sex and age band, and stops if they do not.

## Outputs
One column table, aligned with the persons table, and its hash.

| Column | Values |
|---|---|
| `edu_level` | 0 = ISCED 0–2, 1 = ISCED 3, 2 = ISCED 4, 3 = ISCED 5–6, 4 = ISCED 7–8 |
| `activity` | 0 child, 1 pupil, 2 student, 3 employed, 4 unemployed, 5 retired, 6 other inactive |

## Update rule
There is no per-tick rule. The stage is a pure function of (population, margin tables, `rng_seed`). All arithmetic is integer, as in Stages A and B, so the Rust code and the Python reference agree person for person.

**Notation.** A person's weight $w_i$ is the weight of their household. Their region is the region of their household's county; their year of age is age in months ÷ 12, rounded down, at most 100. "Apportion", $\lfloor\cdot\rceil$ and millionths are those of the [generator spec](population-generator.md).

**Priority.** Every person has one keyed draw context, `draw(PopulationGen, tick 1, entity = index in the persons table)`; its first `u64` is the person's priority. (Tick 0 is Stage A. The tick number is used as a stage number, since the generator runs before the first tick.)

**Split** (used twice below). Given the members of a group in a fixed order, a target for each category, and a carry for each category:
1. Apportion the group's total weight $W$ over the targets. Add the carry. This is what each category is owed.
2. Apportion $W$ again over what is owed, counting a negative amount, or any amount for a category whose target is zero, as nothing. These are the shares. (If that leaves nothing at all, the shares are the targets.)
3. Lay the shares end to end in category order, and the members end to end in their order. A member belongs to the category in whose share the midpoint of its own weight falls (a midpoint on a boundary belongs to the later category).
4. The new carry of each category is what it was owed minus the weight it received.

The carry is what makes small categories come out right. A group of twelve synthetic persons cannot hold a third of an unemployed person, but three such groups in a row hold one.

### Step 1 — activity
For each region and sex, with the carry starting at zero, go through the age bands from youngest to oldest. For one band:
1. **Targets for each year of age.** Synthetic ages are spread evenly inside a band (Stage A), and census ages are not: Romania's cohorts differ a lot in size from one year to the next. Census shares per year would therefore give the wrong totals for the band. So take the census table of the band (year of age × activity, in millionths), using the band's own totals for a year that the census has nobody in, and dropping years with no synthetic person. Balance it both ways, 20 alternating passes: columns to the band's census activity totals (apportioned to the band's synthetic weight), rows to each year's synthetic weight. Then apportion each year's synthetic weight over its row. The band then has the census number of employed, retired and so on, and each year keeps its own pattern.
2. **Split** each year's persons, in ascending (priority, index), with those targets. The carry passes from one year of age to the next, across bands.

### Step 2 — education
For each region, sex and labour status (employed; unemployed; inactive, which is everyone else), with the carry starting at zero, go through the age bands from youngest to oldest. **Split** the band's persons with the census counts by level as targets. If the census has nobody of that status in the band, the targets are the band's counts by level over all statuses.

Order of the persons: ascending (priority, index), except that among the inactive, persons in education come after all the others. They therefore take the highest levels of the group.

> **Simplification:** the census gives education by labour status, not by the kind of inactivity. Putting persons in education last means that, of the inactive aged 20 to 24, those in education have completed upper secondary and the others mostly have not. The direction is right (early school leavers are the least educated), the strength is exaggerated.

### Step 3 — the values of the population model
- Census "below the minimum working age": `child` if younger than `school_age`, else `pupil`.
- Census "in education": `student` if `edu_level` ≥ 1 (upper secondary completed), else `pupil`.
- The other four categories keep their name.

**Attribute hash** (for golden and differential tests): FNV-1a 64 over the person count (`u32`, little-endian), then each person's `edu_level` and `activity` (one byte each).

## Player levers
None.

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| `school_age` | 6 years | 5–7 | From this age a child is a pupil |
| Balancing passes (step 1) | 20 | 5–200 | How closely band totals match before rows are made exact |

## Interactions
- **Before:** Stages A and B of the generator; the pipeline's fetch and normalise stages.
- **After:** [Stage D, jobs](population-jobs.md) (status in employment, occupation, industry group), then income and wealth; [labour market](../economy/labor-market.md), [education](education.md), [social transfers](../economy/social-transfers.md) and [demographics](../economy/demographics.md) read the result.
- Because region totals are fitted, and counties are not, a county's employment rate is its region's rate at its own age and sex structure. County differences inside a region come later, from county employment data.

## Edge cases & failure modes
- **A person where the census has nobody** of that region, sex and year of age: the band's totals are used (step 1). If the census has nobody in the whole band, the margins do not belong to this population: rejected with a named error.
- **A category the census has but no synthetic person can take** (too few persons in the group): its weight waits in the carry and is given to a later age of the same region and sex. What is still owed after the last age is dropped; it is less than one synthetic person per category.
- **Carry that could cross a hard boundary.** A category with a zero target never receives anyone, so nobody under 15 is employed and nobody of 15 or more is a child, whatever the carry holds.
- **Tables whose length does not match their dimensions, no regions, a county without a region:** rejected with a named error; nothing is produced.
- **Suppressed cells:** filled before this stage (see Inputs). The fill moves about 700 persons nationally.

## Acceptance tests
IDs are stable. Tests go in `crates/econ-popgen/tests/acceptance/attributes.rs` (protected; to be written in a test-authoring session, before the Rust code). They are compiled only with the crate feature `attributes`, which the implementation change switches on by default. "The fixtures" are the two normalised margin files committed with the pipeline. "Three scales" means 1:1000, 1:100 and 1:10, seed 42.

- [ ] **AC-POPA-01** `[unit]` Structure, at each of the three scales: one `edu_level` and one `activity` per person; everyone under 15 is `child` or `pupil` with `edu_level` 0, `child` below `school_age` and `pupil` from it; nobody of 15 or more is `child`; a `student` has `edu_level` ≥ 1; a `pupil` of 15 or more has `edu_level` 0.
- [ ] **AC-POPA-02** `[unit]` The stage does not change the population: households and persons are equal before and after, and the state hash still equals the golden value of AC-POP-06.
- [ ] **AC-POPA-03** `[unit]` At each of the three scales the weighted margins are within the tolerances of the table below.
- [ ] **AC-POPA-04** `[unit]` At each of the three scales, for every year of age with at least 100 expected synthetic persons nationally, the share of each census activity among persons of that age is within 5 percentage points of the census share. In particular the share in education is higher at age 15 than at 19, and at 19 than at 24; the share retired is higher at age 69 than at 64, and at 64 than at 59.
- [ ] **AC-POPA-05** `[unit]` Same population, margins and seed give identical attributes (equal attribute hash). A different seed changes who has which attribute, and the national weighted totals by activity and by level stay within the tolerance of AC-POPA-03.
- [ ] **AC-POPA-06** `[golden]` The attribute hash of the fixture population at each of the three scales equals the committed golden value.
- [ ] **AC-POPA-07** `[diff]` On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical attributes, person for person.
- [ ] **AC-POPA-08** `[unit]` Margins with no regions, a table whose length does not match its dimensions, a county index without a region, and margins with nobody in an age band that has synthetic persons are each rejected with a named error and produce no output.

**Tolerances for AC-POPA-03.** Relative error of the weighted total against the census, by how many synthetic records stand behind the cell (census count ÷ `sample_scale`). Census activities are the six of the margin table (`pupil` under 15 counts as child; `pupil` and `student` of 15 or more count as in education). Margins checked, for activity: national; national × sex; national × age band; region; region × sex × broad age (0–14, 15–64, 65+); region × sex × age band. For education level: national; national × sex; national × age band; national × status; region; region × status; region × sex × broad age; region × sex × age band × status.

| Expected synthetic records behind the cell | Tolerance | Worst measured (reference, three scales and 1:200) |
|---|---|---|
| 1,000 or more | 1% | 0.8% |
| 100 to 999 | 6% | 4.7% |
| 30 to 99 | 15% | 13.9% |
| fewer than 30 | not checked | — |

The tolerances for small cells are wider than in the generator spec (4% and 5%). There the weights are fitted to each cell. Here whole persons are dealt into categories, so a cell with 50 synthetic persons moves by 2% for each person more or less, and the carry moves single persons between neighbouring ages.

Measured on 2026-10-10 with the Python reference on the fixtures (seed 42). Weighted persons, against the census:

| | Census | 1:1000 | 1:100 | 1:10 |
|---|---|---|---|---|
| Employed | 7,689,171 | within 0.5% | 7,692,286 | within 0.01% |
| Unemployed | 495,848 | within 0.7% | 496,282 | within 0.01% |
| Largest error, cells of 1,000 records or more | | 0.8% | 0.7% | 0.4% |
| Largest difference in an activity's share at one age (AC-POPA-04) | | 3.6 points | 1.8 points | 2.0 points |

The share at one age differs from the census by up to 2 points even at 1:10. That is the balancing of step 1 at work, not noise: where the synthetic population has more 64-year-olds than the census, some of them must work for the band's total to be right.

Attribute hashes of the reference, for the test writer to record as golden values:

| Scale | State hash (unchanged, AC-POP-06) | Attribute hash |
|---|---|---|
| 1:1000 | `247112c87ae60cf2` | `8a0509e8ebc90647` |
| 1:200 | `4f987e2a39c8cb17` | `788d8c36bfb43660` |
| 1:100 | `6e3617f3f21a0420` | `1e4b3dc3805b32a1` |
| 1:10 | `e651dbd866f38ea0` | `6276def1534b5960` |

## API sketch
For the test writer and the implementer. Names may change before lock; shapes should not. The skeleton in `crates/econ-popgen/src/attributes.rs` is the definition.

```rust
// crate econ-popgen, module `attributes` (no I/O), re-exported at the crate root
pub enum EduLevel { Isced0To2 = 0, Isced3 = 1, Isced4 = 2, Isced5To6 = 3, Isced7To8 = 4 }
pub enum Activity { Child = 0, Pupil = 1, Student = 2, Employed = 3, Unemployed = 4, Retired = 5, InactiveOther = 6 }
pub const MARGIN_ACTIVITIES: usize = 6;      // child, in education, employed, unemployed, retired, other inactive
pub const STATUSES: usize = 3;               // employed, unemployed, inactive
pub const EDU_LEVELS: usize = 5;
pub struct AttributeMargins {
    pub regions: Vec<String>,                // NUTS 2 codes, in table order
    pub region_of_county: Vec<u8>,           // for each county of `Margins::counties`, index into `regions`
    pub age_bands: Vec<(u16, u16)>,          // as in `Margins`
    pub years_of_age: u16,                   // number of single years in `activity`; the last one is open
    pub activity: Vec<u64>,                  // [region][sex][year_of_age][margin activity], row-major
    pub education: Vec<u64>,                 // [region][sex][age_band][status][edu_level], row-major
}
pub struct AttrParams { pub rng_seed: u64, pub school_age: u16, pub balancing_passes: u32 }
impl AttrParams { pub fn new(rng_seed: u64) -> Self; }                      // defaults 6, 20
pub struct PersonAttributes { pub edu_level: Vec<EduLevel>, pub activity: Vec<Activity> }
pub enum AttrError { EmptyTable, ShapeMismatch, UnknownRegion { county: u8 }, NoMargin { region: String } }

pub fn assign_attributes(pop: &Population, margins: &AttributeMargins, params: &AttrParams)
    -> Result<PersonAttributes, AttrError>;
impl PersonAttributes { pub fn state_hash(&self) -> u64; }
```

- Command line: `econ-cli synth-population … --attributes FILE` adds the columns `edu_level` and `activity` to `persons.arrow` and the attribute hash to `fit_report.json`.
- Margin file: JSON with `regions`, `region_of_county` (county code → region code), `sexes`, `age_bands`, `activities`, `statuses`, `edu_levels`, `activity`, `education`, plus `totals` and `provenance`.
- Python reference: `python/reference/popgen_reference.py`, functions `assign_attributes` and `attributes_hash` (exists; `--attributes FILE` on its command line adds the two columns and `summary.attributes_hash` to its output). Where this text and the reference disagree on a detail of ordering or tie-breaking, the reference is the definition until the spec is locked.
- Margin fixture: `python/pipeline/fixtures/census2021_edu_activity_ro.json`, built by `python/pipeline/normalise_census_attributes.py` from the two Eurostat tables.

## Open questions
- [x] **Tolerances** in the table above (1%, 6%, 15%): to be accepted by the owner. They are what the reference achieves, with a little room. Accepted by the owner, 2026-10-10.
- [x] **Persons in education take the highest levels among the inactive** (step 2). Keep this rule, or deal levels at random among the inactive? Random is simpler and gives some 22-year-old "pupils" who have only primary school. Accepted by the owner, 2026-10-10.
- [x] **Children of 6 to 14 are all pupils.** Good enough until the education mechanic has enrolment data? Accepted by the owner, 2026-10-10.
- [x] **Attributes ignore the household** (first simplification above). Acceptable until the IPUMS seed arrives, or should partners be made alike in education by rule now? Accepted by the owner, 2026-10-10.
- [ ] **County differences inside a region.** Employment by county exists in other sources (INS TEMPO, Eurostat regional labour statistics at NUTS 3). Add it as a later correction?
- [ ] "Retired" includes people living on capital income. Fine for the starting population, or separate them when income is imputed?
