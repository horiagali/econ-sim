---
id: economy/labor-market
title: Labour Market
status: locked
owner: horia
depends_on: [economy/production, society/population-groups, society/education, economy/demographics, society/population-employers, society/population-wages, adr/0016-firm-representation, adr/0017-time-base]
research: []
updated: 2026-10-22
---

# Labour Market

## Purpose
Match persons to jobs (firm unit × occupation × region), set wages, and produce employment, unemployment and wage income per person. This is where "more education → better jobs" lives.

## Real-world basis
- Search-and-matching (Diamond–Mortensen–Pissarides): vacancies and job seekers meet via a matching function; flows of hires and separations.
- Wage Phillips curve: wage growth rises with labour-market tightness, expected inflation and productivity growth.
- Minimum wages raise low-skill pay with modest employment effects up to a point (bite measured by min/median ratio), large effects well above ~60% of median.
- Unemployment benefits raise reservation wages and search duration.
- Romania, December 2021 (census and national accounts, see the [generator stages](../society/population-jobs.md)): 7.69 million employed, of whom 6.55 million employees; 0.50 million unemployed; mean gross wage 5,684 lei; the minimum wage 2,300 lei, 53% of the median wage.

## Jobs: firm unit × occupation × region
- A job is a person's place in a **firm unit** ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)), with an occupation and a wage. The starting jobs are those of the generator ([employers](../society/population-employers.md), [wages](../society/population-wages.md)).
- **Occupation** uses ISCO-08 major groups (ten; armed forces unused).
- **Skill tier** (low, mid, high) is derived from occupation and used for education matching:

| Tier | Occupations | Education that qualifies |
|---|---|---|
| high | managers, professionals, technicians | ISCED 5–8 |
| mid | clerks, service and sales, skilled agricultural, craft, operators | ISCED 3–4 and above |
| low | elementary | any |

Workers in jobs below their qualification are **underemployed**.

- **Region:** labour markets clear at the level of the 8 development regions. *(Simplification; county-level markets would be heavier.)*

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| A firm unit's staffing decision: layoffs, vacancies, hires | **staggered**: once a month, on the unit's own day of the month (1 to 28) |
| Quits | monthly, on the last day |
| Wages paid | monthly, on the last day, for the days worked |
| Wage setting | yearly, in January, per industry group; the minimum wage binds from the day it changes |
| Statistics (employment, unemployment, vacancies, wages) | monthly close |

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| — | `employer`, `industry`, `occupation`, `wage` | a person's job (generator) | |
| — | `months_unemployed` | months since the person lost a job or started looking | months |
| $V_{f,o}$ | `vacancies` | open positions of a firm unit by occupation | jobs |
| $N^*_{f}$ | `desired_employment[f]` | what the unit wants to staff, from [production](production.md) | persons |
| $W_{j,o}$ | `wage[j,o]` | reference wage by industry group and occupation: the median wage of those employed there | LCU / month |
| $u$ | `unemployment_rate` | national; any group by filtering persons | ratio |
| $W^{min}$ | `minimum_wage` | gross minimum wage | LCU / month |
| — | `unit_stagger_day` | the unit's day of the month | 1–28 |

## Update rule
1. **Labour demand:** units' desired employment by occupation (from production). Vacancies where demand > staff; layoffs where staff > demand, at most a share per month (firing friction).
2. **Separations:** layoffs + quits (person-level probability by tenure, age, contract) → unemployed or inactive.
3. **Matching** per (skill tier, region):
```math
H = \mu \, U^{\alpha} V^{1-\alpha}
```
Hires are allocated among job seekers by alignment, weighted by education fit, field of study, experience, past occupation and (optionally) a discrimination parameter ([identity](../society/identity.md)). Overqualified seekers may take lower-tier jobs if their tier is slack.
4. **Individual wages:** a person's wage = $W_{j,o}$ × personal factor (experience, education quality, tenure).
5. **Wage setting** per industry group × occupation:
```math
\frac{\Delta W_{j,o}}{W_{j,o}} = \pi^e + g^{A}_j + \beta_1 (\text{tightness}_{o} - \text{tightness}^*) + \beta_2 \cdot \text{profitability}_j
```
with $W \ge W^{min}$ for formal jobs.
6. **Informal work** (full spec: [informal-economy](informal-economy.md)).
7. **Self-employment and farming:** the fallback when no job is found in rural areas.
8. **Public sector** hiring follows ministry headcount targets and public wage policy.

> **Simplification:** no explicit unions or strikes in v1; bargaining strength is a parameter.

## First version (roadmap M2)
**In:**
1. **Staffing, on a unit's day.** Staff = the weight of the persons whose `employer` is the unit.
   - If staff exceeds `desired_employment`: lay off the excess, at most `max_layoff_share` of staff this month. Who: by alignment in ascending (tenure-free in M2) keyed priority. A laid-off person becomes unemployed, with `months_unemployed` 0.
   - If staff is below: the difference is the unit's vacancies this month, spread over occupations in the proportions of its current staff.
2. **Matching, on the same day.** For each of the unit's vacancy occupations, job seekers are the unemployed of the regions where the unit has staff (all regions if it has none) whose education qualifies for the tier. A vacancy is filled this month with the tier's **fill rate**, which falls as the market tightens:

```math
q = \min\Big(1,\ \mu \cdot \Big(\frac{U}{V}\Big)^{\alpha}\Big), \qquad H = \min\big(V_f,\ U_f,\ \lfloor q \cdot V_f + \text{carry} \rfloor\big)
```

   $U$ and $V$ are the tier's job seekers and vacancies at the last monthly close, nationally (the starting values in the first month); $V_f$ the unit's vacancies in the tier and $U_f$ the seekers it can reach, in weighted persons. The fraction of a hire that is left is carried to the unit's next month (a small unit must not be starved by rounding: [spike 10](../../03-architecture/spikes/spike-10-daily-clock.md)). Hired: seekers in ascending (months unemployed descending, keyed priority, index): the longest unemployed first.
   - The hired person gets `employer`, `industry`, the occupation, `employment_status` employee, and the wage `wage[j,o]` of the unit's industry group and that occupation, not below the minimum wage. `wage_declared` is the wage (new jobs are formal until the [informal economy](informal-economy.md) decides otherwise).
3. **Quits, at month end:** each employee quits with probability `quit_rate` a month, by keyed draw with alignment per industry group; a quitter becomes unemployed.
4. **Pay, at month end:** each unit pays its employees their wages through its industry's wage clearing account ([accounting](accounting.md)). A person hired or separated during the month is paid for the days worked, rounded to whole bani.
5. **Wage setting, each January:** for each industry group, every wage and every reference wage moves by

```math
g_j = \pi^e + \beta_1 \cdot (\theta_j - \theta^*)
```

   where $\pi^e$ is expected inflation ([prices](prices-inflation.md)) and $\theta_j$ the group's tightness: vacancies over job seekers qualified for them, averaged over the last twelve monthly closes. Then every wage below the minimum wage is raised to it.
6. **Unemployed persons** look for work every month; nobody leaves the labour force or enters it in M2.

> **Simplification (M2):** employers, own-account workers and family workers keep their status; public units hire and fire like any other; productivity growth and profitability do not enter wages yet.

**Not in M2:** individual wage factors (tenure, experience); underemployment and taking lower-tier jobs; entry of school leavers, retirement, migration (M5); unemployment benefit and its effect on search (M3); informal jobs changing status (M15); public headcount targets and public pay policy (M4); hours.

## Pay and job choice (roadmap M4)
*Added at the owner's request, 2026-10-22, with the public pay lever of the [lever system](../game/lever-system.md): without it a unit paying far below the market would hire as fast as any other, and the pay lever would do nothing.*

1. **The market wage** $\bar w_{t,r}$ of a tier in a region is the weighted mean wage of the tier's employees there, at the last monthly close.
2. **Hiring responds to pay.** A unit's relative pay in a tier is $\rho = w / \bar w_{t,r}$, with $w$ its wage for the tier. Its fill rate becomes
```math
q' = \min\big(1,\ q \cdot \rho^{\eta_f}\big)
```
   so a unit paying above the market fills its vacancies faster, and one paying below fills them slower.
3. **Quits respond to pay.** An employee quits with probability
```math
\text{quit} = \text{clamp}\big(\text{quit\_rate} \cdot \rho^{-\eta_q},\ \text{quit\_min},\ \text{quit\_max}\big)
```
4. **Job-to-job moves.** Of the quitters paid below the market ($\rho < 1$), a share `job_to_job_share` look for work while employed. They go first in the next day's matching, but only for vacancies paying more than their current wage, and they are hired without a spell of unemployment. The rest become unemployed as before.
5. **Applies to every unit,** public and private. Public pay moves only by its lever, so frozen public pay against rising private wages lowers $\rho$ for public units: posts fill slowly and staff leave, first where private pay is highest.

| Parameter | Default | Meaning |
|---|---|---|
| `fill_wage_elasticity` $\eta_f$ | 2 (low confidence) | how much relative pay speeds or slows hiring |
| `quit_wage_elasticity` $\eta_q$ | 2 (low confidence) | how much relative pay raises or lowers quits |
| `quit_min`, `quit_max` | 0.3%, 4% a month | bounds of the monthly quit probability |
| `job_to_job_share` | 50% (low confidence) | share of underpaid quitters who move straight to another job |

> **Simplification:** relative pay is against the tier's regional mean, not against each person's own outside offers. Pay is the only job attribute; working conditions and job security come later.

## Player levers
Minimum wage; unemployment benefit (via reservation wage and search intensity); public wages and headcount; education (skill supply); immigration quotas by skill; wage subsidies; social contributions; labour inspection funding (informality).

## Outputs
Employment and unemployment for any group; wages by industry, occupation and region; labour share; underemployment; informality; vacancy rate; wage inflation.

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `matching_efficiency` $\mu$ | 0.5 | share of vacancies filled in a month when seekers equal vacancies |
| `matching_elasticity` $\alpha$ | 0.5 | how fast the fill rate rises with seekers per vacancy |
| `max_layoff_share` | 10% of staff a month | firing friction |
| `quit_rate` | 1% a month | voluntary separations |
| `wage_tightness_response` $\beta_1$ | 0.1 | wage growth per unit of tightness above normal |
| `normal_tightness` $\theta^*$ | the value of the first year | where wages grow with expected inflation only |

## Interactions
- **From [production](production.md):** `desired_employment` of each unit, set on the unit's day before staffing.
- **To [households](households.md):** wages at month end. **To production:** each unit's staff by occupation, which caps its output.
- **[Prices](prices-inflation.md):** wages are the largest part of unit cost; expected inflation enters wage setting.

## Edge cases
- Minimum wage above the productivity of low-skill jobs in some industries → those jobs disappear, become informal, or get automated over time.
- Very tight markets → wage-price spiral risk (with prices-inflation).
- Zero seekers the unit can reach → no hires; the carry is kept. No vacancies in the tier at the last close → the fill rate is 1.
- A unit with no staff and positive desired employment: its vacancies are spread over occupations in the proportions of its industry group.
- A unit whose `firm_count` falls to zero: its staff is laid off without the monthly limit.

## Acceptance tests
IDs are stable (added 2026-10-11). Tests go in the mechanic's crate, `crates/econ-mech-labour`, in `tests/acceptance/` (protected). AC-LM-01 and 02 were written there on 2026-10-13 against the API skeleton, compiled only with the crate feature `mechanic` until the implementation turns it on. The `[sim]` criteria AC-LM-03 to 07 were written on 2026-10-15 on the `World` ([world loop](../game/world-loop.md)), in `crates/econ-core/tests/acceptance/`, behind the feature `world`. AC-LM-05 reads the law a world is built under as in effect from day 0 (two worlds, minimum wage 2,300 and 2,550 lei); AC-LM-06 compares two worlds whose labour requirement differs by 10% from load.

- [ ] **AC-LM-01** `[unit]` The fill rate and the hires are the formulas of step 2; hires never exceed the unit's vacancies or the seekers it can reach; the carried fraction makes a unit with a fractional rate hire the right number over many months; with no vacancies in the tier the fill rate is 1.
- [ ] **AC-LM-02** `[unit]` Layoffs: the excess over desired employment, at most `max_layoff_share` of staff in a month.
- [ ] **AC-LM-03** `[sim]` Employment counted from persons equals the sum of units' staff on every day, and each unit's staffing runs exactly once a month.
- [ ] **AC-LM-04** `[sim]` Wages paid by units in a month equal wages received by households, through clearing accounts that net to zero; a person hired on the 16th of a 30-day month is paid half a month.
- [ ] **AC-LM-05** `[sim]` Nobody is paid below the minimum wage; raising it lifts exactly the wages below the new level, from the day it takes effect.
- [ ] **AC-LM-06** `[sim]` A permanent 10% rise in every unit's desired employment lowers unemployment over the following months and raises wage growth at the next January round.
- [ ] **AC-LM-07** `[sim]` Only persons whose education qualifies are hired into a tier.
- [ ] **AC-LM-08** `[sim]` *(M3)* Higher unemployment benefits raise average unemployment duration.
- [ ] **AC-LM-09** `[sim]` *(M4)* Raising the minimum wage from 45% to 55% of the median raises low-skill wages with a small employment effect; to 80% with a large one and more informality.
- [ ] **AC-LM-10** `[sim]` *(M5)* More tertiary graduates raise high-skill employment if demand exists, otherwise underemployment rises.
- [ ] **AC-LM-11** `[sim]` *(M4)* Two worlds differing only in one public unit's pay (80% against 100% of the start): over a year the lower-paid unit fills fewer of its vacancies and loses more staff; with every relative pay at 1 the rates are those of M2.
- [ ] **AC-LM-12** `[sim]` *(M4)* Job-to-job moves: underpaid quitters hired by better-paying units count as hires with no unemployment spell, and employment counted from persons still equals units' staff every day.

### Tests owed
The spec was locked before these tests could exist (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-LM-08: unemployment benefit (roadmap M3)
- AC-LM-09: the minimum-wage lever with firms' responses (roadmap M4)
- AC-LM-10: education flows (roadmap M5)

## API sketch
For the test writer and the implementer; names may change, shapes should not.

```rust
// crate econ-mech-labour (no I/O)
pub struct LabourParams { pub matching_efficiency_ppm: u32, pub matching_elasticity_ppm: u32, pub max_layoff_share_ppm: u32,
                          pub quit_rate_ppm: u32, pub wage_tightness_response_ppm: u32 }
pub fn fill_rate_ppm(tier_seekers: u64, tier_vacancies: u64, p: &LabourParams) -> u32;        // AC-LM-01
pub fn hires(unit_vacancies: u64, reachable_seekers: u64, fill_rate_ppm: u32, carry_ppm: &mut u32) -> u64;   // weighted persons
pub fn layoffs(staff: u64, desired: u64, p: &LabourParams) -> u64;                             // AC-LM-02
pub fn part_month_pay(wage: Bani, days_worked: u8, days_in_month: u8) -> Bani;                 // AC-LM-04
pub fn wage_growth_ppm(expected_inflation_ppm: i32, tightness_ppm: u32, normal_ppm: u32, p: &LabourParams) -> i32;
```

`fill_rate_ppm` uses the pinned `libm` power through `econ_num::math` (ADR-0006). The phases that walk today's units and the month-end pay run live with the `World`.

## Open questions
- [x] ISCO major groups (ten) are enough for v1. Decided 2026-10-11 under the owner's delegation.
- [x] Headcount only; hours come with part-time work, if ever. Decided 2026-10-11.
- [x] First version, periods and parameters as above: accepted by the owner's delegation, 2026-10-11 ("accept the drafts").
- [ ] The fill rate uses last month's national seekers and vacancies per tier. Regional tightness would be truer and costs little; add it if the M2 runs show regions that should diverge and do not.
