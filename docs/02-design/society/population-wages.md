---
id: society/population-wages
title: Population Generator, Stage F — Wages
status: review
owner: horia
depends_on: [society/population-jobs, society/population-attributes, society/population-generator, society/population-groups, adr/0006-determinism-contract, adr/0012-data-pipeline-and-licensing, adr/0014-agent-workflow-guardrails]
research: [research/population-modelling-deep-research]
updated: 2026-10-10
---

# Population Generator, Stage F — Wages

## Purpose
Give every employee of the starting population a gross monthly wage (`wage`), so that each industry group pays the wage bill of the national accounts and, inside a group, wages differ by sex, occupation, age and education as the earnings survey says. It runs after [Stage D](population-jobs.md), once, when a scenario is built. It is the first income of the starting population; [households](../economy/households.md), the [labour market](../economy/labor-market.md) and [taxation](../economy/taxation.md) start from it.

## Real-world basis
- **Levels (verified 2026-10-10, Eurostat, free reuse with attribution).** National accounts, `namq_10_a10`: wages and salaries (D11) by the same ten industry groups as Stage D, fourth quarter of 2021, unadjusted. Romania: 111.7 billion lei in the quarter, so 37.2 billion lei a month. Per census employee that is 5,684 lei gross a month; by group from about 2,200 lei (agriculture) to 13,000 lei (information and communication).
- **Proportions.** Structure of Earnings Survey 2022 (`earn_ses22_48`, `_20`, `_23`: mean monthly earnings by sex and economic activity, crossed with occupation, with age and with education; `earn_ses22_54`, `_02`, `_04`: the employees behind them; `earn_ses22_mdeci`: first decile, median and ninth decile). In the survey a manager earns about 1.8 times the average of all employees and a person in an elementary occupation 0.6 times; tertiary education 1.5 times and at most lower secondary 0.6 times; the median is 75% of the mean, the first decile 58% of the median and the ninth decile 2.4 times the median.
- **Minimum wage** (`earn_mw_cur`): 2,300 lei gross a month in the second half of 2021.

> **Simplification:** the survey is from October 2022, ten months after the start date, and covers only enterprises with 10 employees or more in NACE sections B to S. Only its proportions are used; every level comes from the national accounts of the last quarter of 2021. Small firms, where the minimum wage is most common, are therefore given the pay structure of larger ones. Confidence in the levels: high. In the proportions: medium.

> **Simplification:** agriculture is not in the survey. Its employees get the proportions of the whole survey.

> **Simplification:** everyone works full time. Hours, part-time work and undeclared pay are not in this increment ([informal economy](../economy/informal-economy.md), roadmap M15).

## Scope
**This increment:** `wage`, the gross monthly wage of every person whose `employment_status` is employee.

**Not in this increment** (each is a later stage with its own data): the income of employers, own-account and family workers; pensions and benefits; property income; taxes and contributions on the wage (they are computed by the tax mechanics from the law in force); deposits, debts and dwelling values.

The wage bill is the average month of the fourth quarter of 2021, which includes year-end bonuses. The scenario starts on 1 December 2021.

## Inputs
- The population of Stages A and B, the attributes of Stage C and the jobs of Stage D. **None is changed.**
- A margin file written by the pipeline's normalise stage with provenance:

| Table | Cells | Meaning |
|---|---|---|
| `wage_bill[industry_group]` | 10 | lei per month |
| `minimum_wage` | 1 | lei per month, gross |
| `rel_sex[industry_group, sex]` | 10 × 2 | mean earnings of the sex over the mean of the group, in millionths |
| `rel_occupation[industry_group, sex, occupation]` | 10 × 2 × 10 | the same for each occupation of each sex |
| `rel_age[industry_group, sex, age_group]` | 10 × 2 × 3 | age groups: under 30, 30 to 49, 50 or more |
| `rel_education[industry_group, sex, edu_group]` | 10 × 2 × 3 | education groups as in Stage D (ISCED 0–2, 3–4, 5–8) |
| `quantile_curve` | 8 knots | earnings at a rank over mean earnings, in millionths: a line through the knots |

- `rng_seed`.

**How the normalise stage builds the proportions.** An industry group is the employee-weighted average of its NACE sections that the survey has. A cell the survey does not publish takes the proportion of the whole survey for that sex and category; failing that, the one for both sexes times the one of the sex; failing that, 1. All proportions are above zero.

**The quantile curve** runs from the minimum wage at the time of the survey (rank 0) through the first decile, the median and the ninth decile. Above the ninth decile the survey publishes nothing; the curve follows a Pareto tail chosen so that the whole curve averages the survey's mean, drawn with straight pieces to ranks 0.95, 0.99, 0.999 and 1. Only its shape is used.

## Outputs
One column aligned with the persons table, and its hash.

| Column | Values |
|---|---|
| `wage` | gross monthly wage in bani; 0 for everyone who is not an employee |

## Update rule
There is no per-tick rule. The stage is a pure function of (population, Stage C attributes, Stage D jobs, margin file, `rng_seed`), in integer arithmetic. $\lfloor\cdot\rceil$ and millionths are those of the [generator spec](population-generator.md); $U$ = 1,000,000.

**Priority.** `draw(PopulationGen, tick 5, entity = index in the persons table)`; its first `u64`.

Each industry group is handled alone. Its employees are taken in ascending (priority, index); $w_i$ is a person's weight, $W$ the group's.

### Step 1 — a place in the distribution
A person's rank is the midpoint of their own weight in that order, in millionths: $r_i = \lfloor (2 \cdot \text{weight before } i + w_i) \cdot U / (2W) \rfloor$. Their starting relative wage is the quantile curve at that rank, pulled towards 1 by `wage_dispersion`:

```math
x_i = \left\lfloor \frac{U^2 + d \cdot (q(r_i) - U)}{U} \right\rfloor
```

with $d$ = `wage_dispersion` in millionths and $q$ the line through the knots, rounded down. Ranks are spread evenly whatever the scale, so the group has the shape of the curve and not a noisy sample of it.

`wage_dispersion` is less than 1 because the curve is the spread of all employees, and part of that spread is between the cells of step 2.

### Step 2 — the survey's proportions
The mean each sex is to have, with the group's mean at $U$: $m_s = \lfloor \text{rel\_sex}_s \cdot W \cdot U / \sum_{s'} W_{s'} \text{rel\_sex}_{s'} \rceil$.

Then `scaling_passes` passes, each over three tables in this order: occupation, age group, education group. For a table with proportions $t_c$, the cells $c$ = (sex, category), cell weights $W_c$ and cell sums $S_c = \sum_{i \in c} w_i x_i$, both taken before the table is applied:

```math
x_i \leftarrow \left\lfloor \frac{x_i \cdot t_c \cdot W_c \cdot W_s \cdot m_s}{S_c \cdot N_s} \right\rceil , \qquad N_s = \sum_{c \in s} W_c \, t_c
```

for every person of a cell with $S_c > 0$. After a table is applied, the mean of each of its cells stands to the mean of its sex as the survey's proportions do (renormalised to the persons the group really has), and the sexes stand to each other as `rel_sex` says. The three tables ask for the same sex means, so the passes settle.

### Step 3 — the level and the floor
The group's floor, in bani: the minimum wage, or `wage_floor_share` of the group's mean wage if that is lower.

```math
\text{floor} = \min\!\left(100 \cdot \text{minimum\_wage},\ \left\lfloor \frac{f \cdot B}{W \cdot U} \right\rceil\right)
```

with $B$ the group's wage bill in bani and $f$ = `wage_floor_share` in millionths. The level $\lambda$ is the smallest whole number for which the group pays at least its wage bill:

```math
\sum_i w_i \cdot \max\!\left(\left\lfloor \frac{\lambda \, x_i}{U} \right\rceil, \text{floor}\right) \ \ge\ B , \qquad \text{wage}_i = \max\!\left(\left\lfloor \frac{\lambda \, x_i}{U} \right\rceil, \text{floor}\right)
```

found by bisection between 0 and $\lfloor 2B / W \rceil + 1$ (the upper end doubled until it pays the bill).

> **Simplification:** where a group's mean wage is below the minimum wage divided by `wage_floor_share`, its floor is below the legal minimum wage. In the data this is agriculture only: its wage bill divided by the census's employees is 2,200 lei, less than the minimum wage of 2,300, because much of that work is seasonal, part-time or undeclared. Over half of its employees are then at the floor (about 1,760 lei) and the survey's proportions do not hold there.

**Wage hash** (for golden and differential tests): FNV-1a 64 over the person count (`u32`, little-endian), then each person's `wage` (`u64`, little-endian).

## Player levers
None. The minimum wage in the margin file is the law at the start date; the lever is in the [labour market](../economy/labor-market.md).

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| `wage_dispersion` | 0.5 | 0–1 | How much of the curve's spread is left inside a cell. Higher: more employees at the minimum wage and a longer top |
| `wage_floor_share` | 0.8 | 0.5–0.95 | How close to its mean the floor of a low-pay group may be |
| Scaling passes (step 2) | 20 | 5–200 | How closely the three tables are met together |

## Interactions
- **Before:** Stages A to D; the pipeline's fetch and normalise stages.
- **After:** the other incomes and wealth (later generator stages); [households](../economy/households.md) (income); [labour market](../economy/labor-market.md) (the starting wage of every job, and `wage[j,o]`); [taxation](../economy/taxation.md) and [social transfers](../economy/social-transfers.md) (income tax, contributions, earnings-related benefits); [accounting](../economy/accounting.md) (the wage flow of the first month equals the national accounts).
- The census counts 6.55 million employees, the national accounts 6.65 million. The wage bill is spread over the census's employees, so the mean wage here is 1.6% above the national accounts' own.
- Wages do not differ by county or region beyond what jobs do: Stage D places occupations and industry groups by region, and Bucharest has more of the well-paid ones. Regional pay differences inside a job come later, from regional earnings data.

## Edge cases & failure modes
- **An industry group with employees and no wage bill:** the margins do not belong to this population; rejected with a named error.
- **A cell of the survey with no synthetic person:** it plays no part; the renormalisation of step 2 uses only the persons the group has.
- **A sex absent from a group:** its mean is not used.
- **A group with one employee:** rank 0.5, the proportions cancel, the wage is the group's wage bill divided by the person's weight (or the floor).
- **Rounding:** the group pays its wage bill or slightly more: at most the group's weight times one ban of level, a relative excess below 0.001%.
- **Tables whose length does not match their dimensions, a proportion of zero, a quantile curve that does not ascend from rank 0 to 1, no age groups, attributes or jobs of another population (different length):** rejected with a named error; nothing is produced.

## Acceptance tests
IDs are stable. Tests go in `crates/econ-popgen/tests/acceptance/wages.rs` (protected; to be written in a test-authoring session, before the Rust code), compiled only with the crate feature `wages`. "The fixtures" are the four normalised margin files committed with the pipeline. "Three scales" means 1:1000, 1:100 and 1:10, seed 42.

- [ ] **AC-POPW-01** `[unit]` Structure, at each of the three scales: one wage per person; it is above zero exactly when `employment_status` is employee; no wage is below the floor of its industry group, and outside the groups with a lowered floor none is below the minimum wage.
- [ ] **AC-POPW-02** `[unit]` The stage changes nothing before it: the state hash, the attribute hash and the jobs hash are those of AC-POP-06, AC-POPA-06 and AC-POPJ-06.
- [ ] **AC-POPW-03** `[unit]` At each of the three scales every industry group pays its wage bill: Σ weight × wage is at least the bill and exceeds it by less than 0.001%. The national total follows.
- [ ] **AC-POPW-04** `[unit]` At each of the three scales, in every industry group whose floor is the minimum wage, the mean wage of each (sex, occupation), (sex, age group) and (sex, education group) cell over the mean of the group is within the tolerances of the table below of the survey's proportion, renormalised as in step 2.
- [ ] **AC-POPW-05** `[unit]` The national distribution at each of the three scales: the first decile over the median is within 0.06 of the survey's (0.578), the ninth decile over the median within 0.25 of the survey's (2.362), the median over the mean within 0.05 of the survey's (0.753); between 3% and 15% of employees are at or below the minimum wage.
- [ ] **AC-POPW-06** `[unit]` Same inputs and seed give identical wages (equal wage hash). A different seed changes who earns what; AC-POPW-03 still holds.
- [ ] **AC-POPW-07** `[golden]` The wage hash of the fixture population at each of the three scales equals the committed golden value.
- [ ] **AC-POPW-08** `[diff]` On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical wages, person for person.
- [ ] **AC-POPW-09** `[unit]` Margins with no age groups, a table whose length does not match its dimensions, a proportion of zero, a quantile curve that does not end at rank 1, jobs of a different length, and a wage bill of zero for a group that has employees are each rejected with a named error and produce no output.

**Tolerances for AC-POPW-04.** Relative error of the cell's proportion, by synthetic records in the cell (the cell's weight ÷ `sample_scale`).

| Synthetic records in the cell | Tolerance | Worst measured (reference, 1:1000, 1:200, 1:100, 1:10) |
|---|---|---|
| 1,000 or more | 5% | 4.0% |
| 100 to 999 | 6% | 4.5% |
| 30 to 99 | 8% | 5.8% |
| fewer than 30 | not checked | — |

The proportions are met almost exactly by step 2; what is left comes from the floor of step 3, which lifts the low cells of a group. That is why large cells are not much closer than small ones: the error is a shift, not noise.

Measured on 2026-10-10 with the Python reference on the fixtures (seed 42):

| | Survey | 1:1000 | 1:100 | 1:10 |
|---|---|---|---|---|
| Largest excess over a group's wage bill | | 0.0003% | 0.0002% | 0.0001% |
| Mean wage, lei | | 5,684 | 5,686 | 5,688 |
| Median, lei | | 4,355 | 4,349 | 4,356 |
| First decile ÷ median | 0.578 | 0.560 | 0.562 | 0.563 |
| Ninth decile ÷ median | 2.362 | 2.463 | 2.479 | 2.474 |
| Median ÷ mean | 0.753 | 0.766 | 0.765 | 0.766 |
| Share at or below the minimum wage | | 7.3% | 7.2% | 7.3% |
| Highest wage, lei | | 34,900 | 48,000 | 69,300 |

Hashes of the reference, for the test writer to record as golden values (the earlier hashes are unchanged):

| Scale | Wage hash |
|---|---|
| 1:1000 | `e5c524e4e785f24f` |
| 1:200 | `ed70890e72a8dba7` |
| 1:100 | `5686b27267ee3696` |
| 1:10 | `78d9a718768b61cd` |

## API sketch
For the test writer and the implementer. Names may change before lock; shapes should not. The skeleton in `crates/econ-popgen/src/wages.rs` is the definition.

```rust
// crate econ-popgen, module `wages` (no I/O), re-exported at the crate root
pub struct WageMargins {
    pub wage_bill: Vec<u64>,                 // [industry_group], lei per month
    pub minimum_wage: u64,                   // lei per month
    pub age_group_from: Vec<u16>,            // first age (years) of each age group, ascending from 0
    pub edu_group_of_level: [u8; EDU_LEVELS],
    pub rel_sex: Vec<u32>,                   // [industry_group][sex], millionths
    pub rel_occupation: Vec<u32>,            // [industry_group][sex][occupation], row-major
    pub rel_age: Vec<u32>,                   // [industry_group][sex][age_group]
    pub rel_education: Vec<u32>,             // [industry_group][sex][edu_group]
    pub curve_rank: Vec<u32>,                // millionths, ascending from 0 to 1,000,000
    pub curve_value: Vec<u32>,               // millionths of the mean, not decreasing
}
pub struct WageParams { pub rng_seed: u64, pub dispersion_ppm: u32, pub floor_share_ppm: u32, pub scaling_passes: u32 }
impl WageParams { pub fn new(rng_seed: u64) -> Self; }                      // defaults 500_000, 800_000, 20
pub struct PersonWages { pub wage: Vec<u64> }                               // bani
pub enum WageError { EmptyTable, ShapeMismatch, NoMargin { industry_group: u8 } }

pub fn assign_wages(pop: &Population, attrs: &PersonAttributes, jobs: &PersonJobs, margins: &WageMargins, params: &WageParams)
    -> Result<PersonWages, WageError>;
impl PersonWages { pub fn state_hash(&self) -> u64; }
```

- The numbers of industry groups, occupations and education groups are the constants of Stage D (`INDUSTRY_GROUPS`, `OCCUPATIONS`, `EDU_GROUPS`).
- Command line: `econ-cli synth-population … --attributes FILE --jobs FILE --wages FILE` adds the column `wage` (u64, bani) to `persons.arrow` and the wage hash to `fit_report.json`.
- Margin file: JSON with `industry_groups`, `sexes`, `occupations`, `age_group_from`, `edu_groups`, `edu_group_of_level`, `wage_bill`, `minimum_wage`, the four `rel_` tables, `quantile_curve` (`rank`, `value`), `totals` and `provenance`.
- Python reference: `python/reference/popgen_reference.py`, functions `assign_wages`, `curve_at`, `wages_hash` and `wage_fit` (exists; `--wages FILE` on its command line, together with `--attributes FILE --jobs FILE`, adds the column and `summary.wages_hash`). Where this text and the reference disagree on a detail of ordering or rounding, the reference is the definition until the spec is locked.
- Margin fixture: `python/pipeline/fixtures/earnings2021_ro.json`, built by `python/pipeline/normalise_earnings.py`.

## Open questions
Accepted by the owner as proposed on 2026-10-10 ("everything's fine"); the spec is at `review` and is locked on the owner's word, with its nine criteria listed as tests owed until the test-authoring session.
- [x] **Tolerances** of AC-POPW-04 (5%, 6%, 8%) and the bands of AC-POPW-05.
- [x] **`wage_dispersion` = 0.5,** which puts about 7% of employees at the minimum wage, as the survey of larger firms implies. Counts of contracts at the minimum wage that are often quoted for Romania are higher (not verified here); the parameter is the place to change it if a source is found.
- [x] **Agriculture below the minimum wage** until the informal economy and part-time work exist (roadmap M15).
- [x] **Proportions from 2022 and from firms of 10 employees or more.**
- [ ] **The top of the distribution** is a guess (a Pareto tail fitted to the mean). The highest synthetic wage is about 35,000 lei a month at 1:1000 and 69,000 at 1:10. Tax records would give the real top; needed only if a wealth or top-income tax is to be calibrated closely.
- [ ] Public sector pay is set by law in grids and is not distinguished here inside the group "public administration, education and health".
