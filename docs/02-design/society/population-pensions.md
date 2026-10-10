---
id: society/population-pensions
title: Population Generator, Stage G — Pensions
status: draft
owner: horia
depends_on: [society/population-wages, society/population-attributes, society/population-generator, society/population-groups, adr/0006-determinism-contract, adr/0012-data-pipeline-and-licensing, adr/0014-agent-workflow-guardrails]
research: [research/population-modelling-deep-research]
updated: 2026-10-10
---

# Population Generator, Stage G — Pensions

## Purpose
Give every retired person of the starting population a gross monthly pension (`pension`), so that the year's pension bill is paid, women's and men's pensions differ as measured, and about as many pensioners sit at the minimum pension as in reality. It runs after [Stage C](population-attributes.md), once, when a scenario is built. [Social transfers](../economy/social-transfers.md) start from these pensions: a pension already in payment is a fact about a person, not something a formula can recompute without their work history.

## Real-world basis
- **Level (verified 2026-10-10, Eurostat, free reuse with attribution).** Social protection statistics, `spr_exp_pens`: expenditure on all pensions in 2021 was 104.7 billion lei (old age 91.6, survivors 6.3, disability 2.5, others 4.3), so 8.72 billion lei a month. Per retired person of the census (4.41 million) that is 1,978 lei a month.
- **Women and men.** EU-SILC, `ilc_pnp13`: the average pension of women of 65 or more was 23.1% below that of men in 2021.
- **The minimum pension** (the social indemnity for pensioners) was 800 lei a month from September 2020 to the end of 2021; about 926,000 pensioners received it in June 2021, which is 21% of the census's retired. Not in Eurostat: entered by hand from press reports of the pension house's figures ([level](https://www.gandul.ro/social/unul-din-cinci-pensionari-traieste-la-limita-extrema-a-saraciei-ce-pensii-primesc-lunar-19663243), [count](https://economedia.ro/?p=2129)). Confidence: level high, count medium.
- **Replacement ratio.** EU-SILC, `ilc_pnp3`: the median pension of people aged 65 to 74 was 43% of the median earnings of people aged 50 to 59 (survey year 2021). Used only as a check.

Nothing published through Eurostat gives the distribution of Romanian pensions or pensions by education or former job. Two parts of this stage are therefore built from assumptions:

> **Simplification (low confidence):** above the minimum pension, pensions are spread like earnings: the earnings quantile curve of [Stage F](population-wages.md), stretched so that the pension curve averages the mean pension. The national pension house publishes pensioners by pension bracket; that table would replace this.

> **Simplification (low confidence):** pensions follow career earnings, and the only trace of a career in the starting population is education. Education groups differ in pension by half of what they differ in earnings (`pension_earnings_link`). No data sets this number.

> **Simplification:** only persons whose activity is `retired` have a pension, and they share the whole pension bill. In reality some pensioners work (they are `employed` in the census) and some disability and survivor pensions go to people counted as other inactive; the census's "retired" also includes a few people living on capital income.

## Scope
**This increment:** `pension`, the gross monthly pension of every person whose `activity` is retired.

**Not in this increment:** the kind of pension (old age, disability, survivor, service pension), contribution years and pension points, Pillar II savings, benefits other than pensions (they are computed from the law in force by the transfer mechanics), and the income of the self-employed (see Open questions).

The pension bill is the average month of 2021. The scenario starts on 1 December 2021; the pension point rose in January 2022.

## Inputs
- The population of Stages A and B and the attributes of Stage C. **Neither is changed.**
- A margin file written by the pipeline's normalise stage with provenance:

| Table | Cells | Meaning |
|---|---|---|
| `pension_bill` | 1 | lei per month |
| `minimum_pension` | 1 | lei per month |
| `rel_sex[sex]` | 2 | mean pension of each sex, as proportions, in millionths (769,000 and 1,000,000) |
| `rel_education[sex, edu_group]` | 2 × 3 | mean earnings of the education group over the mean of its sex, whole earnings survey, in millionths |
| `quantile_curve` | 9 knots | pension at a rank over the mean pension, in millionths: a line through the knots |
| `replacement_ratio` | 1 | for checking only |

- `rng_seed`.

**The quantile curve** is flat at the minimum pension from rank 0 to the share of pensioners who receive it (0.21). Above that it has the knots of the earnings curve, moved to the remaining ranks, and the earnings curve's rises, stretched by one factor so that the whole curve averages 1.

## Outputs
One column aligned with the persons table, and its hash.

| Column | Values |
|---|---|
| `pension` | gross monthly pension in bani; 0 for everyone who is not retired |

## Update rule
There is no per-tick rule. The stage is a pure function of (population, Stage C attributes, margin file, `rng_seed`), in integer arithmetic.

**Priority.** `draw(PopulationGen, tick 6, entity = index in the persons table)`; its first `u64`.

The retired are one group, taken in ascending (priority, index). The three steps are those of [Stage F](population-wages.md) (place in the distribution, proportions, level and floor), with:
- the pension quantile curve, used in full (`pension_dispersion` = 1);
- `rel_sex` for the means of the two sexes;
- one table of proportions, by (sex, education group): the earnings proportions pulled towards 1,

```math
t_{s,e} = \left\lfloor \frac{U^2 + k \cdot (\text{rel\_education}_{s,e} - U)}{U} \right\rfloor
```

  with $k$ = `pension_earnings_link` in millionths and $U$ = 1,000,000;
- the pension bill in bani as the amount to pay;
- the floor: the minimum pension, or the mean pension (bill ÷ weight, rounded down) if that is lower.

The floor lifts the lowest pensions after the proportions were set, so women's mean ends about one point closer to men's than `rel_sex` says, and the share at the minimum ends a few points below the curve's flat part: people the curve put at the minimum are scaled up or down with their sex and education group before the floor applies.

**Pension hash** (for golden and differential tests): FNV-1a 64 over the person count (`u32`, little-endian), then each person's `pension` (`u64`, little-endian).

## Player levers
None. The minimum pension in the margin file is the law at the start date; pension levers are in [social transfers](../economy/social-transfers.md).

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| `pension_earnings_link` | 0.5 | 0–1 | 0: education does not matter for a pension. 1: education groups differ as much in pension as in earnings |
| `pension_dispersion` | 1 | 0–1 | Below 1 the curve is pulled towards the mean: fewer at the minimum, a shorter top |
| Scaling passes | 20 | 5–200 | As in Stage F; with one table the proportions settle in the first pass |

## Interactions
- **Before:** Stages A to C; the pipeline's fetch and normalise stages (which also read the earnings margin file for the curve's shape).
- **After:** [social transfers](../economy/social-transfers.md) (pensions in payment, indexation, the minimum pension as a lever); [households](../economy/households.md) (income); [fiscal policy](../economy/fiscal-policy.md) (the pension bill of the first month equals the published one); wealth imputation.
- New pensions, granted after the start date, come from the pension formula and the retiree's wage history in the simulation. This stage only covers the stock at the start.

## Edge cases & failure modes
- **Nobody retired:** every pension is 0; nothing else happens.
- **Retired persons and no pension bill:** rejected with a named error.
- **A bill too small for the minimum pension:** the floor is the mean pension and everyone gets it.
- **A sex or an education group with no retired person:** it plays no part.
- **Tables whose length does not match their dimensions, a proportion of zero, a quantile curve that does not ascend from rank 0 to 1, no education groups, attributes of another population:** rejected with a named error; nothing is produced.

## Acceptance tests
IDs are stable. Tests go in `crates/econ-popgen/tests/acceptance/pensions.rs` (protected; to be written in a test-authoring session, before the Rust code), compiled only with the crate feature `pensions`. "The fixtures" are the normalised margin files committed with the pipeline. "Three scales" means 1:1000, 1:100 and 1:10, seed 42.

- [ ] **AC-POPP-01** `[unit]` Structure, at each of the three scales: one pension per person; it is above zero exactly when `activity` is retired; none is below the minimum pension.
- [ ] **AC-POPP-02** `[unit]` The stage changes nothing before it: the state hash and the attribute hash are those of AC-POP-06 and AC-POPA-06.
- [ ] **AC-POPP-03** `[unit]` At each of the three scales Σ weight × pension is at least the pension bill and exceeds it by less than 0.001%.
- [ ] **AC-POPP-04** `[unit]` At each of the three scales: the mean pension of women over that of men is within 0.03 of `rel_sex` (0.769); inside each sex, the mean pension of each education group with at least 100 synthetic records, over the mean of the sex, is within 4% of the proportion of the update rule, renormalised to the persons present as in Stage F.
- [ ] **AC-POPP-05** `[unit]` At each of the three scales: between 15% and 24% of the retired are at the minimum pension; the median pension is between 60% and 80% of the mean; retired persons with tertiary education have a mean pension at least 1.3 times that of those with at most lower secondary education.
- [ ] **AC-POPP-06** `[unit]` Same inputs and seed give identical pensions (equal pension hash). A different seed changes who gets what; AC-POPP-03 still holds. With `pension_earnings_link` = 0 the mean pensions of the education groups of a sex are within 8% of each other.
- [ ] **AC-POPP-07** `[golden]` The pension hash of the fixture population at each of the three scales equals the committed golden value.
- [ ] **AC-POPP-08** `[diff]` On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical pensions, person for person.
- [ ] **AC-POPP-09** `[unit]` Margins with no education groups, a table whose length does not match its dimensions, a proportion of zero, a quantile curve that does not end at rank 1, attributes of a different length, and a pension bill of zero while someone is retired are each rejected with a named error and produce no output.

Measured on 2026-10-10 with the Python reference on the fixtures (seed 42):

| | Target | 1:1000 | 1:100 | 1:10 |
|---|---|---|---|---|
| Excess over the pension bill | | 0.0002% | 0.0004% | 0.0001% |
| Mean pension, lei | 1,978 | 1,986 | 1,979 | 1,978 |
| Median pension, lei | | 1,377 | 1,371 | 1,364 |
| Ninth decile, lei | | 4,082 | 3,999 | 4,002 |
| Highest pension, lei | | 12,000 | 16,000 | 22,600 |
| Women's mean ÷ men's | 0.769 | 0.780 | 0.781 | 0.781 |
| Largest error of an education group's proportion | | 1.4% | 1.5% | 1.5% |
| Tertiary ÷ at most lower secondary | | 1.67 | 1.69 | 1.70 |
| Share at the minimum pension | 0.21 | 0.178 | 0.181 | 0.181 |
| Median pension of 65–74 ÷ median wage of 50–59 (with Stage F) | 0.43 (survey) | 0.34 | 0.34 | — |

The mean differs from the target because the synthetic population has slightly fewer or more retired persons than the census (Stage C's tolerance); the bill is exact. The replacement ratio is lower than the survey's: the survey's earnings are a year's earnings of everyone who worked in the year, part-year workers included, while Stage F's wages are a full month at the national accounts' level.

Hashes of the reference, for the test writer to record as golden values (the earlier hashes are unchanged):

| Scale | Pension hash |
|---|---|
| 1:1000 | `963e671ef7035b92` |
| 1:200 | `07432cd231e4aa00` |
| 1:100 | `2969848640c3f676` |
| 1:10 | `f26404ef4190809f` |

## API sketch
For the test writer and the implementer. Names may change before lock; shapes should not. The skeleton in `crates/econ-popgen/src/pensions.rs` is the definition.

```rust
// crate econ-popgen, module `pensions` (no I/O), re-exported at the crate root
pub struct PensionMargins {
    pub pension_bill: u64,                   // lei per month
    pub minimum_pension: u64,                // lei per month
    pub rel_sex: [u32; 2],                   // millionths
    pub edu_group_of_level: [u8; EDU_LEVELS],
    pub rel_education: Vec<u32>,             // [sex][edu_group], row-major, millionths
    pub curve_rank: Vec<u32>,                // millionths, ascending from 0 to 1,000,000
    pub curve_value: Vec<u32>,               // millionths of the mean, not decreasing
}
pub struct PensionParams { pub rng_seed: u64, pub dispersion_ppm: u32, pub earnings_link_ppm: u32, pub scaling_passes: u32 }
impl PensionParams { pub fn new(rng_seed: u64) -> Self; }                   // defaults 1_000_000, 500_000, 20
pub struct PersonPensions { pub pension: Vec<u64> }                         // bani
pub enum PensionError { EmptyTable, ShapeMismatch, NoMargin }

pub fn assign_pensions(pop: &Population, attrs: &PersonAttributes, margins: &PensionMargins, params: &PensionParams)
    -> Result<PersonPensions, PensionError>;
impl PersonPensions { pub fn state_hash(&self) -> u64; }
```

- The three steps are shared with Stage F: one function in the Rust crate, as `spread` in the reference.
- Command line: `econ-cli synth-population … --attributes FILE --pensions FILE` adds the column `pension` (u64, bani) to `persons.arrow` and the pension hash to `fit_report.json`.
- Margin file: JSON with `sexes`, `edu_groups`, `edu_group_of_level`, `pension_bill`, `minimum_pension`, `rel_sex`, `rel_education`, `quantile_curve` (`rank`, `value`), `replacement_ratio`, `totals` and `provenance` (with a `manual` list for the two figures entered by hand).
- Python reference: `python/reference/popgen_reference.py`, functions `assign_pensions`, `spread`, `pensions_hash` and `pension_fit` (exists; `--pensions FILE` on its command line, together with `--attributes FILE`, adds the column and `summary.pensions_hash`). Where this text and the reference disagree on a detail of ordering or rounding, the reference is the definition until the spec is locked.
- Margin fixture: `python/pipeline/fixtures/pensions2021_ro.json`, built by `python/pipeline/normalise_pensions.py`.

## Open questions
- [ ] **The two low-confidence simplifications** (the shape above the minimum; the link to education at 0.5). Accept until the pension house's table of pensioners by pension bracket is brought in by hand, or get that table first?
- [ ] **Only the census's retired have a pension.** Working pensioners and inactive disability pensioners are left out; their money goes to the retired. Acceptable for the start?
- [ ] **Bands of AC-POPP-04 and AC-POPP-05:** to be accepted by the owner.
- [ ] **Income of the self-employed is not here, and cannot be done the same way.** The national accounts give households 46 billion lei of mixed income in the fourth quarter of 2021, 15 billion a month. Shared among the census's 0.9 million employers and own-account workers that would be 17,000 lei a month each, three times the mean wage. The figure includes what households grow for themselves, building of own dwellings and the estimate of undeclared activity, and the national accounts count 1.9 million self-employed, most of them in agriculture, where the census counts 0.5 million. Splitting it needs a decision on subsistence farming and the informal economy (`hh_farm_land`, [informal economy](../economy/informal-economy.md)) and data on farm households. Proposed: a research note first.
