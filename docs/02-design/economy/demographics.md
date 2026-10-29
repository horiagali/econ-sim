---
id: economy/demographics
title: Demographics — Ageing, Births, Deaths, Retirement
status: draft
owner: horia
depends_on: [society/population-groups, society/households-lifecycle, society/population-generator, society/population-pensions, economy/social-transfers, economy/labor-market, economy/households, economy/money-banking, game/world-loop, game/lever-system, adr/0003-people-representation, adr/0006-determinism-contract, adr/0007-money-and-ledger, adr/0017-time-base]
research: [research/demographics-romania-2021, research/law-in-force-romania-2021]
updated: 2026-10-26
---

# Demographics — Ageing, Births, Deaths, Retirement

> Lives under `economy/` for historical reasons; conceptually part of [society](../society/README.md). Move it when files can be renamed.

## Purpose
People age, are born, die and retire. These transitions change the synthetic population ([population-groups](../society/population-groups.md)) month by month, so that a long run is not a frozen 2021: the population shrinks and ages as Romania's does, the labour force loses its oldest workers, and the pension bill follows the number of pensioners. Households forming and dissolving (young people leaving home, couples, separation, orphans) are in [household life cycle](../society/households-lifecycle.md); **migration, the diaspora and remittances** are in their own spec, `economy/migration-remittances` (written separately; see [Migration](#migration)).

## Real-world basis
Data and sources are in the [research note](../../01-research/notes/demographics-romania-2021.md). In short:
- Romania had 19.05 million residents at the census of 1 December 2021, 16.2% under 15 and 19.5% aged 65 or more.
- Before the pandemic about 200,000 children were born and 262,000 people died a year (2017–2019); the total fertility rate was 1.77 (2019) and life expectancy at birth 71.9 for men and 79.4 for women (2019), rising about 0.2 years a year.
- The official projection (Eurostat EUROPOP2023) has the population falling to 15.0 million by 2070 (15.75 million with no migration) and the share aged 65+ rising to 29%.
- Fertility falls with education and income across countries, rises modestly with child benefits (Quebec, Poland's 500+, Austria), and dips in recessions, most among the young. **The first version has the short-run responses to child benefits and to the parents' unemployment** (owner, 2026-10-26), with small elasticities from that literature. Mortality falls with income, health spending and education; that response is not in the first version.
- The 2021 law sets the standard retirement age at 65 for men and about 61 years 9 months for women, rising to 63 by January 2030; pensions are points × the point value, a survivor receives half of the deceased spouse's pension, and employment contracts end at the standard age.

**Owner decision, 2026-10-26: the simulation runs as if COVID never happened.** 2021 had 335,527 deaths, about 70,000 more than the pre-pandemic level.

> **Simplification:** mortality is the pre-pandemic table (the average of 2017–2019 by sex and single year of age), improved each year by the pre-COVID trend from 2018 onwards, so the first year has about 255,000–265,000 deaths, not 2021's 335,527. Fertility uses the 2019 rates (2021's were not depressed). Checks against the official projection use EUROPOP2023's path from 2025, whose first years still carry some of the pandemic, with EUROPOP2019 as a pre-COVID cross-check. Pandemics as events are an after-v1 item of the [roadmap](../../roadmap.md).

## Scope
**In the first version (roadmap M5):** ageing; deaths by age and sex; births by the mother's age; estates of the dead (money, debts, dwelling) through the ledger; survivor pensions for spouses; retirement at the standard age, early retirement, and the pensions of new retirees; the retirement-age lever; the statistics (population, births, deaths, life expectancy, fertility, dependency ratios, pensioners).

**In [household life cycle](../society/households-lifecycle.md):** what happens to a household when a member is born, dies or leaves; young adults leaving home; couples forming and separating; children left without an adult; the operations on weighted records that every transition uses.

**Not in the first version:** fertility responding to housing cost (M13) and to education; mortality responding to income, health spending or education (Open question 4); mortality by county or health; survivor pensions for children; disability as its own pension; working pensioners; old people moving in with their children or into care homes; migration (its own spec).

## State variables
New per person (the World's persons table gains them; [population-groups](../society/population-groups.md) already lists `age`, `sex` and `role`, which the World has not loaded so far):

| Name | Meaning | Unit | Notes |
|---|---|---|---|
| `age` | age | months | exists in the scenario; advanced monthly |
| `sex` | F or M | category | from the scenario |
| `role` | head, partner, child, other | category | from the scenario; changed by [household life cycle](../society/households-lifecycle.md) |
| `person_key` | stable identity of the person, never reused | id (u64) | the start's index, then a counter; the entity of every keyed draw |
| `contribution_months` | months of pension contributions | months | proxy at load, then accrued |
| `career_points` | sum of yearly pension points earned (wage over `wage_nominal`) | points, millionths | proxy at load, then accrued |
| `pension_points` | exists ([social transfers](social-transfers.md)) | points, millionths | set at retirement for new pensioners |
| `survivor_points` | pension points the person would draw as a surviving spouse | points, millionths | 0 unless widowed |
| `months_since_birth` | months since the woman last gave birth | months | none if never in the run |
| `alive` | the record counts | bool | a dead person's record is a tombstone until the yearly compaction |

Economy-wide: `carry[transition][stratum]` (alignment remainders, millionths of a person), the mortality table of the current year, the retirement age in force by sex.

## Inputs
- From the scenario: persons (`age`, `sex`, `role`, `activity`, `edu_level`, `wage`, `wage_declared`, `pension`), households (`hh_weight`, `hh_county`, `hh_collective`), `sample_scale` (never a constant: [ADR-0003](../../03-architecture/decisions/0003-people-representation.md)).
- A demographic data file from the pipeline, with provenance ([research note](../../01-research/notes/demographics-romania-2021.md)): `mortality_rate[sex, age]` (probability of dying within a year, single years 0 to 109, 2017–2019 average), `fertility_rate[age]` (births per woman a year, single years 15 to 49, 2019), `sex_ratio_at_birth`, the census share retired by sex and single year of age (already read by Stage C).
- From the law in force (`law.json`, [lever system](../game/lever-system.md)): `retirement_age_schedule[sex]`, `retirement_age[sex]` commands, `full_contribution_years`, `min_contribution_years`, `early_retirement_reduction`, `survivor_share`, `pension_point_value` and `minimum_pension` ([social transfers](social-transfers.md)).
- From the [labour market](labor-market.md): `employer`, `wage_declared`, the unemployment benefit; from the monthly close: `wage_nominal` (the mean gross wage of employees over the last calendar year).

## Outputs
`population` (weighted persons, by any filter), `births`, `deaths` (weighted, a month and a year), `life_expectancy[sex]` and `total_fertility_rate` (period measures from the year's rates), `old_age_dependency_ratio`, `pensioners`, the new retirees and their pensions. Persons appear, disappear and change activity; [social transfers](social-transfers.md) pays their pensions and child allowances, the [labour market](labor-market.md) loses its retirees and the dead.

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)). Everything here runs in a **new phase "People"**, at month end, after Pay and before the wage round and the monthly close (phases 10, 11, 12 of the [World](../game/world-loop.md)), so that the month's pensions and wages are paid to the people who lived the month, and the month's statistics count the new population. Inside the phase, in this order:

1. **Ageing** — every live person's `age` + 1 month.
2. **Contributions** — the month's `contribution_months` and `career_points`.
3. **Deaths** — then, for each household that lost a member, the [household life cycle](../society/households-lifecycle.md)'s rules (new head, orphans) and, for a household left empty, the estate.
4. **Births.**
5. **Retirement** — standard, then early.
6. **Household life cycle** — leaving home, unions, separations ([household life cycle](../society/households-lifecycle.md)).
7. **Migration** — its own spec's slot.

**Yearly, on 1 January** (the first People phase of the year runs at the end of January; these steps run at the close of December, before it): the mortality table of the new year; the yearly compaction of tombstones (below). **Monthly, on the first:** the retirement age in force moves by the law's schedule or a command's phase-in ([lever system](../game/lever-system.md)).

## First version (roadmap M5)

### Alignment
Every count below is aligned as in [population-groups](../society/population-groups.md): the rate model gives the expected number, the alignment picks exactly that many persons, by weight.

For a transition and a stratum $s$ with eligible persons $i$ of weight $w_i$ (their household's `hh_weight`) and monthly probability $p_i$ (parts per billion):

```math
T_s = \sum_{i \in s} w_i \, p_i \;+\; \text{carry}_s
```

in millionths of a person, exact integer arithmetic. Persons are ranked by $r_i = \lfloor u_i / p_i \rfloor$ ascending (ties by `person_key`), where $u_i$ is the first `u64` of `draw(Demography, day, person_key, purpose)`: the smaller the key, the earlier the person is taken, and a person twice as likely is taken about twice as often. Walking the ranking, person $i$ is selected while it brings the selected weight closer to the target ($S + w_i/2 \le T_s$, with $S$ the weight selected so far); persons that would overshoot are skipped and the walk goes on. Then $\text{carry}_s = T_s - S$, kept for the next month (it stays within half the largest weight of the stratum). With weights near `sample_scale`, a stratum expecting 0.3 events a month gets one every three months or so, and nothing is rounded away ([ADR-0003](../../03-architecture/decisions/0003-people-representation.md)).

### 1. Ageing
`age` + 1 for every live person. No other attribute changes with age here: entering school, the working age and the labour market are the [education flows](../society/education.md)' and the [labour market](labor-market.md)'s.

### 2. Contributions
Every live person who is not retired, in this month:
- an employee with `wage_declared` > 0, or a self-employed person with declared income: `contribution_months` + 1 and `career_points` + declared monthly income ÷ (12 × `wage_nominal`);
- a person drawing the unemployment benefit: `contribution_months` + 1 and `career_points` + benefit ÷ (12 × `wage_nominal`) (the law counts benefit months as contributed);
- anyone else: nothing.

**At load**, a proxy sets both (no contribution history exists in the population):

```math
\text{years}_i = \text{clamp}\big(\text{age}_i - 18 - \text{extra\_school}(\text{edu}_i),\; 0,\; \text{age}_i - 15\big) \cdot \text{career\_density}[\text{activity}_i]
```

with `extra_school` the years of schooling above upper secondary (as in [social transfers](social-transfers.md)'s proxy) and `career_density` the share of those years with contributions (employee 0.9, employer and own-account 0.6, family worker 0.2, unemployed 0.7, other inactive 0.3, pupil and student 1; low confidence). `contribution_months` = 12 × years; `career_points` = years × (the person's `wage` if an employee, else the mean wage of the person's sex and education among employees × `nonemployed_wage_share` 0.8) ÷ `wage_nominal`.

> **Simplification (low confidence):** a career is guessed from age, education and today's activity. Calibrated in M7 so that the pensions of the first years' new retirees average close to the 2021 average pension and the share at the minimum pension stays near 21%.

### 3. Deaths
**Probability.** `mortality_rate[sex, age]` is turned into a monthly probability by the pipeline, $m = 1 - (1 - q)^{1/12}$, stored in parts per billion. The table of calendar year $y$ is

```math
m_{y}[x, a] = \text{round}\big(m_{base}[x, a] \cdot (1 - \texttt{mortality\_improvement})^{\,y - 2018}\big)
```

computed once a year by repeated integer multiplication, half away from zero (2018 is the centre of the 2017–2019 base). At 110 years or more the monthly probability is 1. Strata: sex × five-year age band (0–4, …, 100 and over); all live persons, private and collective, are eligible.

**For each person selected:**
1. The person's job ends (the employer's `staff` falls by the weight; the labour market's separation `death`, which pays no benefit); the unemployment benefit and the pension stop. The pay for the month was paid on pay day, before this phase.
2. **Survivor right.** If a partner remains in the household (the other of a head-and-partner pair), the partner's `survivor_points` = max(own `survivor_points`, `survivor_share` × the deceased's pension points). The deceased's pension points are `pension_points` if retired, else `career_points` ÷ `full_contribution_years` (what the deceased would have had). A survivor who is already retired draws max(`pension_points`, `survivor_points`) from the next pay day; one who is not draws it when retiring (step 5).
3. The record becomes a tombstone (`alive` false); its weight leaves the population. [Household life cycle](../society/households-lifecycle.md) then names a new head if needed and takes care of children left without an adult.
4. **Estate.** If no live member remains, the household's estate is settled and its record ends.

> **Simplification:** a survivor pension needs no length of marriage and no age condition beyond the survivor's own retirement; children's survivor pensions are not modelled. Survivors' pensions at the start are already inside the starting pensions (Stage G).

**Estate settlement.** Per real household of the deceased record (amounts are per household; the record has weight $w_d$):
1. **Debts.** The money $M$ = `hh_cash` + `hh_deposits` repays the debts $D$ = `hh_mortgage` + `hh_consumer_debt` up to $\min(M, D)$, deposits first: a repayment to the banks, flow code `loan` (7003), as any repayment in [money and banking](money-banking.md). What is left of $D$ is written off by the banks (`write_off`, 7004).
2. **The heir** is one household: the live household of the first (ascending `person_key`) live person whose `origin_household` is this one (a child who left home; [household life cycle](../society/households-lifecycle.md)); else a keyed draw (`Demography`, purpose `heir`) among private households of the same county with a member 18 to 45 years younger than the deceased; else any private household of the county; else none.
3. **Money to the heir.** The remaining cash and deposits pass separately, each as a total $w_d \cdot a$ spread over the heir's weight $w_h$: the heir's per-household balance rises by $\lfloor w_d a / w_h \rfloor$, and the remainder ($< w_h$ bani) goes to the government. With no heir, everything goes to the government (in Romania a vacant estate passes to the state). Payments to the government use a new flow code **4109 `transfer.estate`** (households → government; a capital transfer).
4. **Dwelling.** `hh_dwelling_value` passes the same way (it is not money: the remainder is dropped and counted in the month's report). The heir's tenure does not change until housing (M13).
5. Money moved between two households is not a ledger posting (the ledger holds the household sector's totals, [world loop](../game/world-loop.md)); it is recorded in both households' `hh_moved_to_date` so the household identity of AC-HH-03 still holds. Only the legs to banks and government are posted.

> **Simplification:** one heir per estate; heirs inherit the dwelling but never the debt (debts are paid from the money or written off); no inheritance tax (Romania has none; notary fees are ignored). An heir with a much smaller weight than the deceased receives proportionally more per household.

### 4. Births
**Eligible:** live women aged 15 to 49 in private households, except those who gave birth in the last 12 months (`months_since_birth` < 12). **Strata:** five-year bands of the mother's age (15–19, …, 45–49). **Target:** $T_s$ with

```math
p_i = \frac{\texttt{fertility\_rate}[a_i]}{12} \cdot k_s \cdot M^{pol}_h \cdot M^{job}_i \cdot M^{cyc}
```

(parts per billion; the three multipliers are below, and $k_s$ normalises $M^{job}$). **Ranking:** $p_i$ × `partner_fertility_ratio` (3) for a woman who is head or partner of a household with a partner of the other sex; the partner ratio moves who gives birth, not how many. The multipliers move how many.

**Fertility responds to policy and the economy** (owner, 2026-10-26). Evidence and confidence are in the [research note](../../01-research/notes/demographics-romania-2021.md#8-fertility-responses-to-policy-and-the-economy). Each multiplier is exactly 1 under the 2021 law and the reference state, and bounded:

1. **Child benefits, relative to income** ($M^{pol}_h$, per household). The extra support a newborn would bring, a month averaged over its childhood, compared with the 2021 law:

   ```math
   \Delta B = \frac{2\,\Delta\texttt{child\_allowance\_under2} + 16\,\Delta\texttt{child\_allowance}}{18} + \Delta\texttt{child\_tax\_credit}
   ```

   where each Δ is the law in force `fertility_lag` (9) months earlier minus the 2021 law's amount (levers `transfer.child.under2`, `transfer.child.to18`, `tax.income.credit_per_child`; the credit is 0 in 2021). Then

   ```math
   M^{pol}_h = \text{clamp}\Big(1 + \beta_b \,\frac{\Delta B}{Y_h},\; 0.8,\; 1.3\Big)
   ```

   with $Y_h$ the household's money income per equivalised adult over the last 12 months, not below `fertility_income_floor` (25%) of the national median, and $\beta_b$ = `fertility_benefit_response` (0.1). A benefit worth 10% of a household's income per adult raises its birth probability by 1%; poorer households respond more for the same lei, as in Poland's 500+. Doubling both 2021 allowances (ΔB ≈ 238 lei a month) raises births by about 1%; adding a 500-lei child tax credit on top, by about 3–4%.

2. **The potential parents' unemployment** ($M^{job}_i$, per woman): `unemployed_fertility_ratio` (0.85) if she or her partner is unemployed, else 1. At load, $k_s$ is set per stratum so that the starting population gives exactly the base-rate births: $k_s = \sum w f / \sum w f M^{job}$. A recession that adds 5 points of unemployment among people of 20 to 44 lowers births by about 0.5–1.5%, the size of the European estimates (Goldstein et al. 2013: 0.1–0.2% per point, more among the young).

3. **The cycle of real income** ($M^{cyc}$, national): $\text{clamp}(1 + \beta_y (g - g^{ref}), 0.97, 1.03)$, with $g$ the growth over 12 months of real disposable income per equivalised adult (deflated by the CPI), measured `fertility_lag` months earlier, $g^{ref}$ = `normal_income_growth` (2% a year) and $\beta_y$ = `fertility_cycle_response` (0.2). A year with real income 5% below normal growth lowers births by 1%.

> **Simplification:** the responses are short-run and proportional: they raise or lower the rate at every age alike and do not distinguish a birth brought forward (tempo) from an extra child (quantum), so a lasting benefit rise keeps births higher for as long as it lasts, while the literature finds part of the effect is timing (Lalive & Zweimüller 2009). The benefit term compares nominal amounts with the 2021 law's, so inflation does not erode a frozen benefit's effect. Individual unemployment is read without a lag (spells are long). Housing cost joins with the housing market (M13).

All three can be switched off for calibration (`fertility_benefit_response` = 0, `unemployed_fertility_ratio` = 1, `fertility_cycle_response` = 0); then births are exactly the base-rate births (AC-DEM-20).

**The newborn** is a new person record appended to the persons table, in the mother's household (so with its weight: each selected record stands for $w$ real births): `age` 0, `sex` M with probability `sex_ratio_at_birth` (0.514; `draw(Demography, day, mother's person_key, birth_sex)`), `role` child, `edu_level` lowest, `activity` child, no employer, no pension, contributions 0, a new `person_key`. The mother's `months_since_birth` = 0. The child allowance at the under-2 rate starts at the next pay day ([social transfers](social-transfers.md)); the household's equivalised size is recomputed ([households](households.md)).

> **Simplification:** fertility depends on the mother's age and partnership only; no twins; no births in collective records.

### 5. Retirement
**The retirement age in force**, `retirement_age[sex]` in months of age, follows the 2021 law's schedule `retirement_age_schedule[sex]` (dated values: men 780 months; women 741 months in December 2021, rising one month of age about every 6.5 months to 756 on 1 January 2030; medium confidence, to check against Annex 5 of Law 263/2010 when `law.json` is filled). A command on the lever `transfer.pension.retirement_age` replaces the schedule for that sex from its effective date; the value in force then moves toward the commanded one by one month of age per month (the lever's `phase_in`).

**Standard retirement.** Every live person who is not retired, whose `age` ≥ `retirement_age[sex]` and whose `contribution_months` ≥ 12 × `min_contribution_years` (15) retires this month (no alignment: the law decides). A person short of the minimum contribution stays as they are (keeps working, if working, and keeps accruing).

**Early retirement.** Persons up to 120 months below the standard age, not retired, with at least the minimum contribution: strata sex × whole years before the standard age (1 to 10); target with the hazard `early_retirement_rate[sex, years_before]` ÷ 12. The hazard is derived at load from the starting population's share retired $R(a)$ at each single year of age in the ten years before the standard age of 2021: $h(a) = (R(a+1) - R(a)) / (1 - R(a))$, floored at 0; it moves with the standard age (a later standard age moves early retirement later by the same amount).

> **Simplification:** early retirement stands for every pension that starts before the standard age (early and partial early pensions, disability, special schemes such as the military's), with one rule. Retirement ends all work: there are no working pensioners in the first version.

**What retiring does:** `activity` retired; the job ends (separation `retirement`, no benefit; the labour market may open a vacancy) or self-employment ends; the unemployment benefit stops; and

```math
\text{pension\_points} = \max\!\Big(\frac{\text{career\_points}}{\texttt{full\_contribution\_years}} \cdot (1 - \rho),\; \text{survivor\_points}\Big)
```

with $\rho$ = `early_retirement_reduction` (0.75%) × months before the standard age, up to 60 months and only for persons with fewer than `full_contribution_years` + 8 years of contribution; 0 otherwise. The pension (points × `pension_point_value`, not below `minimum_pension`) is paid from the next pay day by [social transfers](social-transfers.md), which pays new and starting pensioners alike.

> **Simplification:** `full_contribution_years` is 35 for both sexes (women's was about 32 in 2021, rising to 35 by 2030); the reduction is not lifted at the standard age.

### 6. Records
A dead person's record stays as a tombstone (weight excluded from every sum, never selected, skipped by every phase) until the **yearly compaction** at the close of December: tombstones and ended households are removed, the tables are rewritten in their order, and every stored index (households' members, firm units' `members`, `origin_household`) is remapped. Keyed draws use `person_key`, never the index, so compaction changes no result. New persons and households are appended at the end of their tables.

### Migration
Emigration, return, immigration, moves between counties, the diaspora and remittances are specified in `economy/migration-remittances` (a separate spec, M5), which uses the record operations of [household life cycle](../society/households-lifecycle.md) and the alignment above. Its step runs seventh in the People phase. The earlier draft of this spec sketched them (Harris–Todaro expected-income gaps, networks, a diaspora stock with remittances, return migration, immigration capped by a quota); that text now belongs to that spec.

### Alignment to the official projection
**Behaviour, not forcing** (owner, 2026-10-26): births, deaths and migration come from rates and decisions (the rates above, with fertility's responses); the official projection is a **check**, not a target, so that policy and the economy can move the result. A 50-year run with migration off is compared with EUROPOP2023's no-migration variant (AC-DEM-12). Calibration (M7) only may scale fertility and mortality by `fertility_scale` and `mortality_scale` (default 1) if the run drifts; they are tuning parameters, not levers, and nothing sets them during a game.

## Player levers
| Lever | Effect | Lag |
|---|---|---|
| `transfer.pension.retirement_age` (`retirement_age[sex]`, months of age, 600 to 840; registered, wired here) | the standard age by sex; early retirement moves with it | phased in one month of age per month from the effective date; first effect the next month |
| `transfer.pension.point_value`, `minimum_pension`, `pension_indexation` | amounts of new and old pensions ([social transfers](social-transfers.md)) | next pay day |
| `transfer.child.under2`, `transfer.child.to18`, `tax.income.credit_per_child` (`child_allowance_under2`, `child_allowance`, `child_tax_credit`) | income of families with children at the next pay day; **births**, through $M^{pol}$ | births respond after `fertility_lag` (9 months) |
| any lever that moves unemployment or real income (minimum wage, taxes, spending, the policy rate…) | **births**, through $M^{job}$ and $M^{cyc}$ | 9 months and more |

The lever's registry entry names `MonthlyClose` as its reader; it becomes `People` when wired.

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| `mortality_improvement` | 1.5% a year | 0–3% | how fast death rates fall; 1.5% gives about +0.2 years of life expectancy a year (pre-COVID trend; medium confidence) |
| `partner_fertility_ratio` | 3 | 1–10 | how much more likely a woman in a couple is to give birth than one who is not (low confidence) |
| `sex_ratio_at_birth` | 0.514 | 0.50–0.53 | share of boys (2021) |
| `career_density[activity]` | 0.9, 0.6, 0.2, 0.7, 0.3, 1 | 0–1 | contribution years at load (low confidence) |
| `nonemployed_wage_share` | 0.8 | 0.5–1 | career wage of persons not employed at load (low confidence) |
| `fertility_benefit_response` $\beta_b$ | 0.1 | 0–0.4 | births per unit of extra child support relative to income per adult (Milligan 2005, Poland's 500+; low confidence) |
| `unemployed_fertility_ratio` | 0.85 | 0.6–1 | birth probability of a woman who or whose partner is unemployed, relative to others (Goldstein et al. 2013; low confidence) |
| `fertility_cycle_response` $\beta_y$ | 0.2 | 0–0.5 | births per point of real income growth above or below normal (low confidence) |
| `normal_income_growth` | 2% a year | 0–4% | the real income growth at which the cycle term is 1 |
| `fertility_lag` | 9 months | 6–12 | from a change in policy or income to births |
| `fertility_income_floor` | 25% of the median | 10–50% | the lowest income per adult the benefit term divides by |
| `fertility_scale`, `mortality_scale` | 1 | 0.8–1.2 | calibration (M7) only |
| `full_contribution_years`, `min_contribution_years`, `early_retirement_reduction`, `survivor_share` | 35, 15, 0.75% a month, 50% | law | the 2021 law (medium confidence) |

## Interactions
- **[Social transfers](social-transfers.md):** pays the pensions of new retirees and survivors and the child allowance of newborns; nothing changes in its rules.
- **Births respond** to the child allowance and the child tax credit ([taxation](taxation.md)) relative to household income, to unemployment ([labour market](labor-market.md)) and to real income growth (the monthly close), after `fertility_lag`.
- **[Labour market](labor-market.md):** loses retirees and the dead (separations `retirement` and `death`); its tightness and wage round see the smaller labour force. Entry of school leavers is the [education flows](../society/education.md)'.
- **[Households](households.md):** new, merged and ended households; equivalised sizes change; estates and moves keep money exact.
- **[Money and banking](money-banking.md):** estates repay or write off debts.
- **Fiscal:** the pension bill grows with the number of pensioners; estates without heirs are a small revenue (4109).
- **Statistics (M6):** population, births, deaths, life expectancy, fertility, dependency, by group.

## Edge cases & failure modes
- **A stratum with no eligible person:** its target goes into its carry; nothing is selected.
- **A woman selected for a birth who is also selected to die the same month:** deaths run first; she is no longer eligible.
- **A newborn in a household whose weight changes later** (a split): the child is copied with the household ([household life cycle](../society/households-lifecycle.md)).
- **Estate with debts larger than its money:** the shortfall is written off; the heir gets the dwelling and no money.
- **No household in the county can be heir:** the estate goes to the government.
- **The deceased was the only employee of a firm unit:** the unit's staff falls to 0; the labour market's rules for empty units apply.
- **A retirement-age command below the current age of many people:** they all retire the month the value in force reaches their age; with the phase-in that is one cohort-month at a time.
- **A person older than 110:** dies with probability 1.
- **Fertility's history not yet there:** $M^{cyc}$ is 1 until 12 + `fertility_lag` months of income exist; $M^{pol}$ reads the 2021 law for months before the start. A household with no income has $Y_h$ at the floor, so its multiplier stays within the bounds.
- **A benefit cut** makes ΔB negative: births fall, down to the floor of 0.8.
- **Collective records:** members die and age; nobody is born there; an ended collective record is removed like a household.

## Acceptance tests
IDs are stable. Tests are written by the test-writer ([ADR-0014](../../03-architecture/decisions/0014-agent-workflow-guardrails.md)): `[unit]` in a new crate `crates/econ-mech-demography`, `[sim]` on the `World` in `crates/econ-core/tests/acceptance/`. "Three scales" means 1:1000, 1:100 and 1:10, seed 42; "the exit run" is a 50-year run of the Romania scenario under the 2021 law with every M3 and M4 feature on and migration off.

- [ ] **AC-DEM-01** `[unit]` The mortality table: a yearly probability turns into the monthly one of the pipeline's rule; the table of year $y$ is the base × (1 − improvement)^(y − 2018), rounded half away from zero, the same on every platform; at 110 the monthly probability is one.
- [ ] **AC-DEM-02** `[unit]` Alignment: for a stratum of records with weights between 1 and 5,000, the weight selected each month is within half the largest weight of target + carry; over 120 months the cumulative selected weight is within the largest weight of the cumulative expected; the result does not depend on the order of the input; a person with twice the probability is selected 1.8 to 2.2 times as often over 10,000 trials.
- [ ] **AC-DEM-03** `[sim]` Person accounting, every month of 20 years at 1:1000: the weighted population changes by exactly births − deaths (+ immigrants − emigrants when migration is on); every live person is in exactly one live household; no live household is empty; every live person's age rose by one month; tombstones count nowhere.
- [ ] **AC-DEM-04** `[sim]` The first year at 1:100 (pre-COVID): deaths between 240,000 and 280,000 (not 2021's 335,527), with men 50% to 55% of them; births between 175,000 and 205,000; boys 50.5% to 52.3% of births over the first 10 years.
- [ ] **AC-DEM-05** `[sim]` Life expectancy at birth from the run's first-year death rates is within 0.6 years of the one computed directly from the year's mortality table (the 2017–2019 average, 71.7 for men and 79.1 for women, improved to 2022: about 72.5 and 79.9) and rises between 0.1 and 0.3 years a year over 50 years.
- [ ] **AC-DEM-06** `[sim]` Fertility in the first year: the total fertility rate from the run's births is within 0.05 of the input's (1.77), the mothers' mean age within 0.5 years of 28.1, and births to women in a couple are a larger share than their share of women aged 15–49.
- [ ] **AC-DEM-07** `[sim]` Estates conserve money: on every month of 20 years at 1:1000 the ledger's invariants and I-4 hold; at each estate, the households' money falls by exactly the debts repaid plus the amount paid to the government under 4109, and the banks' loans fall by the debts repaid plus written off; AC-HH-03's identity holds for every household with `hh_moved_to_date`.
- [ ] **AC-DEM-08** `[sim]` When a retired person dies leaving a retired partner whose pension is below `survivor_share` of the deceased's, the partner's pension at the next pay day is half the deceased's points × the point value (to the ban); a partner not yet retired gets at least that at retirement.
- [ ] **AC-DEM-09** `[sim]` Standard retirement: at every monthly close no live non-retired person has both an age at or above the age in force and the minimum contribution; women's age in force is 741 months in December 2021 and 756 in January 2030, men's 780 throughout; an employee who retires is off the employer's staff from that month and paid no wage after it.
- [ ] **AC-DEM-10** `[unit]` A new pension: points = max(career points ÷ full contribution years × (1 − 0.75% × months early, up to 60, only below full + 8 years), survivor points); no pension below 15 years of contribution; the pension paid is points × point value, not below the minimum pension.
- [ ] **AC-DEM-11** `[sim]` A newborn is in the mother's household with age 0, role child and the household's weight; the household's child allowance at the next pay day rises by the under-2 amount; a woman gives birth at most once in 12 months.
- [ ] **AC-DEM-12** `[sim]` **The exit run**, at 1:1000 and 1:100: the population falls in every decade; on 1 January 2070 it is within 7% of EUROPOP2023's no-migration figure (15.75 million); the share aged 65+ rises from 19.5% to between 25% and 31%, the share under 15 falls, the median age rises by at least three years, and the old-age dependency ratio rises.
- [ ] **AC-DEM-13** `[sim]` Pension spending responds, in the exit run: the weighted number of pensioners and the sum of their pension points in 2045 both exceed their December 2021 values by at least 5%.
- [ ] **AC-DEM-14** `[sim]` The retirement-age lever: of two worlds, one with both ages raised by 24 months from January 2023, after five years the raised world has fewer pensioners (by at least 3% of the pensioners), a lower pension bill, and more persons aged 60 to 67 employed or unemployed; the value in force rose one month of age a month.
- [ ] **AC-DEM-15** `[sim]` Determinism and scale: two 10-year runs with the same seed have equal state hashes every month; a run compacted every month and one compacted yearly have equal statistics; yearly births and deaths at 1:1000 and 1:100 differ by less than 5% in each of the first 10 years.
- [ ] **AC-DEM-16** `[sim]` Performance: at 1:100 the People phase adds at most 15% to the time of a simulated year (AC-WLD-10's budget), and the exit run at 1:100 stays within the 50-year time budget of Spike 4.
- [ ] **AC-DEM-17** `[unit]` Fertility multipliers: each is exactly 1 under the 2021 law at the reference state (no benefit change, nobody unemployed in the stratum's normalisation, income growth at normal); $M^{pol}$ rises with ΔB and falls with income, and every multiplier stays within its bounds for any input (including zero income and negative ΔB); the normalisation $k_s$ gives the base-rate target on the starting population.
- [ ] **AC-DEM-18** `[sim]` A large child-benefit rise: of two worlds at 1:100, one with both child allowances doubled and a 500-lei child tax credit from 1 January 2023, and the economic terms switched off in both (so only the benefit term differs), births before October 2023 are equal person for person (the lag), and births in 2024–2025 are 1% to 6% higher in the raised world; with every term on, they are still higher, by 0.5% to 7%.
- [ ] **AC-DEM-19** `[sim]` A recession lowers births slightly: of two worlds at 1:100, one with a lasting layoff that raises the unemployment of persons aged 20–44 by at least 4 points from January 2023, births in 2024 are 0.2% to 3% lower in the recession world.
- [ ] **AC-DEM-20** `[sim]` With the law at its 2021 values and the economic terms switched off (`unemployed_fertility_ratio` 1, `fertility_cycle_response` 0), the births of every month are those of the base rates exactly: the same targets and the same mothers as a run whose three multipliers are fixed at 1.

AC-PG-02 (person conservation every day of 100 years) is owed to this spec and is covered by AC-DEM-03 at 20 years; its own test runs 100 years.

## API sketch
**A proposal.** A new crate `crates/econ-mech-demography` (pure functions, no I/O); the People phase walks the World's tables and posts the ledger legs.

```rust
pub type Ppb = u32;                       // parts per billion
pub struct MortalityTable { pub base_year: u16, pub monthly: [[Ppb; 111]; 2] }   // [sex][age in years]
pub fn monthly_from_yearly(q_ppb: Ppb) -> Ppb;                                  // pipeline rule, AC-DEM-01
pub fn table_for_year(t: &MortalityTable, year: u16, improvement_ppm: u32) -> MortalityTable;

pub struct Candidate { pub person_key: u64, pub weight: u32, pub p: Ppb, pub rank_p: Ppb }
pub struct Selection { pub chosen: Vec<u64>, pub carry_micro: i64 }               // carry in millionths of a person
pub fn align(candidates: &[Candidate], carry_micro: i64, rng: &KeyedRng, day: Day, purpose: u32) -> Selection;   // AC-DEM-02

pub struct PensionLaw { pub full_years: u8, pub min_years: u8, pub early_reduction_ppm_per_month: u32,
                        pub survivor_share_ppm: u32, pub point_value: Bani, pub minimum: Bani }
pub fn new_pension_points(career_points: u64, survivor_points: u64, months_early: u16,
                          contribution_months: u16, law: &PensionLaw) -> Option<u64>;            // AC-DEM-10
pub fn early_hazards(share_retired_by_age: &[u32], standard_age_years: u8) -> Vec<u32>;          // ppm a year

pub struct FertilityResponse { pub benefit_ppm: u32, pub unemployed_ratio_ppm: u32, pub cycle_ppm: u32,
                               pub normal_growth_ppm: i32, pub lag_months: u8, pub income_floor_ppm: u32 }
pub fn benefit_gap(law_then: &ChildSupport, law_2021: &ChildSupport) -> Bani;                    // ΔB a month
pub fn m_pol(delta_b: Bani, income_per_adult: Bani, median: Bani, r: &FertilityResponse) -> u32; // ppm, 800_000..=1_300_000
pub fn m_job(woman_or_partner_unemployed: bool, r: &FertilityResponse) -> u32;                   // ppm
pub fn m_cyc(real_growth_ppm: Option<i32>, r: &FertilityResponse) -> u32;                        // ppm, 970_000..=1_030_000; AC-DEM-17

pub struct Estate { pub cash: Bani, pub deposits: Bani, pub debts: Bani, pub dwelling: Bani, pub weight: u32 }
pub struct EstateSplit { pub repaid: Bani, pub written_off: Bani,                                // totals over the weight
                         pub heir_cash: Bani, pub heir_deposits: Bani, pub heir_dwelling: Bani,   // per heir household
                         pub to_government: Bani }
pub fn settle(estate: &Estate, heir_weight: Option<u32>) -> EstateSplit;                         // AC-DEM-07
```

```rust
// econ-core: the World's side.
pub enum PhaseId { /* … */ Pay, People, WageRound, MonthlyClose, /* … */ }
pub struct PeopleReport { pub births: u64, pub deaths: u64, pub retired: u64, pub early_retired: u64,
                          pub estates: u32, pub to_government: Bani, pub dwelling_rounding: Bani }   // weighted
impl World {
    pub fn people_report(&self) -> &PeopleReport;                  // the latest People phase
    pub fn retirement_age_in_force(&self, sex: Sex) -> u16;        // months
    pub fn population_by_age(&self, sex: Sex) -> Vec<u64>;         // weighted, single years
    pub fn life_expectancy(&self, sex: Sex) -> u32;                // hundredths of a year, from the last 12 months
}
pub enum StartEdit { /* … */ RetirementAge { sex: Sex, months: u16, from: Date } }   // for AC-DEM-14
```

New flow code: 4109 `transfer.estate`. New random purposes on `Stream::Demography`: `death`, `birth`, `birth_sex`, `early_retirement`, `heir` (and the life cycle's).

## Open questions
- [x] **COVID.** Recorded as the owner's decision of 2026-10-26: pre-pandemic mortality (2017–2019 table improved from 2018), 2019 fertility, checks against EUROPOP2023 from 2025 and EUROPOP2019. *Proposed: as written.* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **Rates or hard alignment to the projection?** Aligning yearly births and deaths to EUROPOP2023's totals would reproduce it exactly but leave no room for policy to move fertility or mortality. *Proposed: rates as targets, the projection as a check (AC-DEM-12), scale factors for calibration in M7.* **Decided by the owner (2026-10-26):** behaviour, not forcing: births, deaths and migration come from rates and decisions; the projection is a check; the scale factors are for M7 only.
- [x] **Behavioural fertility.** Should child benefits, income, unemployment and housing cost move fertility in M5? *Proposed: not before M14.* **Decided by the owner (2026-10-26): fertility responds now, in M5**, to child benefits relative to income, to the parents' unemployment and to the cycle of real income, with small, bounded, lagged elasticities from the literature, tunable for M7 (step 4, AC-DEM-17 to 20). Housing cost joins in M13.
- [x] **Mortality by health, income, education or county.** *Proposed: not in v1's first version; the health index of [social outcomes](../society/social-outcomes.md) can multiply the probability when it exists (M14).* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **The law's schedule against the lever.** *Proposed: the 2021 law's dated schedule for women is data in `law.json`; a command replaces it for that sex and is phased in at one month of age per month. Check the schedule against Annex 5 of Law 263/2010 then.* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **Early retirement as one rule** for early, disability and special pensions, derived from the census profile. *Proposed: accept for M5; disability pensions get their own programme with M14's benefits.* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **Working pensioners.** *Proposed: none in the first version (retiring ends all work); revisit with M14 if the employment of people over 65 matters.* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **Survivor pensions for children** (under 16, or 26 in education). *Proposed: not in the first version (a small part of the 6.3 billion lei of survivors' pensions).* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **Estates: one heir, debts not inherited, dwelling to the heir with no change of tenure.** *Proposed: accept; housing (M13) decides what heirs do with a second dwelling.* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **The career proxy at load** (`career_density`, `nonemployed_wage_share`) is a guess. *Proposed: accept, calibrate in M7 against the average new pension and the share at the minimum.* Decided as proposed (owner's delegation, 2026-10-26).
- [x] **Where the phase sits.** *Proposed: a new phase "People" at month end between Pay and the wage round; the registry's reader of `transfer.pension.retirement_age` changes from `MonthlyClose` to `People`.* Decided as proposed (owner's delegation, 2026-10-26).
