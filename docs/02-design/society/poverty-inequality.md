---
id: society/poverty-inequality
title: Poverty and Inequality Indicators from Incomes
status: draft
owner: horia
depends_on: [society/social-outcomes, society/population-groups, economy/households, game/world-loop, economy/demographics, adr/0006-determinism-contract, adr/0008-explainability-architecture, adr/0017-time-base]
research: [research/poverty-inequality-romania-2021]
updated: 2026-10-26
---

# Poverty and Inequality Indicators from Incomes

## Purpose
Measure, each monthly close, how incomes are shared out among the weighted persons, the way Eurostat's income survey (EU-SILC) does: the at-risk-of-poverty rate, the Gini coefficient, the S80/S20 ratio, a hardship rate (the game's proxy for material deprivation), and the **standard of living by class** decided by the owner on 2026-10-26 (roadmap M6). These are outputs for the player and for [approval](opinion-approval.md); **they change no behaviour**: households keep deciding by their income fifths ([households](../economy/households.md)). This spec is roadmap M5's "poverty and inequality indicators from incomes"; the panel that shows them is M8's, the group queries and "why" trees M6's. It replaces the "Wellbeing and poverty" bullet of [social outcomes](social-outcomes.md) for the income measures.

## Real-world basis
Research note [`poverty-inequality-romania-2021`](../../01-research/notes/poverty-inequality-romania-2021.md).
- EU-SILC's definitions: household disposable income ÷ the modified OECD equivalence scale, given to every member; indicators count persons in private households; the poverty line is 60% of the national median.
- Romania, survey 2022 (incomes of 2021): at-risk-of-poverty rate 21.2%, Gini 32.0, S80/S20 6.00, median equivalised income 27,129 lei a year. Survey 2021 (incomes of 2020): 22.5%, 34.3, 7.10, 23,367 lei. Severe material and social deprivation 23.1% (2021). Confidence high.
- The OECD's middle-income band is 75% to 200% of the median; Eurostat publishes a poverty rate anchored at a fixed year and moved with prices.

## Periods
A tick is a day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)). Everything here runs at the **monthly close**, after the month's statistics and the income fifths, on the households' income of the closed calendar month. Day 0 is 1 December 2021, so month 0 is December 2021: the **anchor month**.

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $Y_{h,t}$ | `hh_disposable_income` | Money the household received in the closed month (the `World`'s income of the month: net wages, pensions, benefits, self-employment and informal income, interest, dividends, remittances) | LCU / month |
| $e_h$ | `eq_adults` | Equivalised size, modified OECD (1, 0.5 per further member of 14 or more, 0.3 per child under 14); thousandths in code | adults |
| $\tilde y_{h,t}$ | `hh_real_income_recent` | Recent real income: a moving average of $Y_{h,t}$ at December 2021 prices | LCU of 2021 / month |
| — | `hh_income_year` | $Y_{h,t}$ summed over the calendar year so far | LCU |
| $m^{2021}$ | `poverty_anchor` | Median equivalised recent income of month 0, real; frozen | LCU of 2021 / month |
| — | `poverty_stats` | The month's indicators (below), kept in history | record |

`hh_disposable_income` and `eq_adults` already exist in the `World` (`income_this_month`, `eq_adults_milli`). [Demographics](../economy/demographics.md) recomputes `eq_adults` when a household's members change.

## Inputs
- From the households table: `hh_weight`, `hh_collective`, the month's income, `eq_adults`, `hh_hardship`, county.
- From the persons table: `household_id`, age (for the age groups of the breakdown).
- The CPI at the close ([prices](../economy/prices-inflation.md)), 1 at day 0.

## Outputs
Per month, nationally, by county, by development region and by age group (0–17, 18–64, 65 or more):

| code_name | Meaning | Unit |
|---|---|---|
| `median_eq_income` | Weighted median of equivalised recent income, nominal and real | LCU / month |
| `poverty_rate` | At-risk-of-poverty rate: persons below `poverty_line_share` × the current national median | ratio |
| `poverty_rate_anchored` | Persons below `poverty_line_share` × `poverty_anchor`, in real terms (the class "at risk") | ratio |
| `gini` | Gini coefficient of equivalised recent income over persons | 0–1 |
| `s80_s20` | Income of the richest fifth of persons over that of the poorest fifth | ratio |
| `hardship_rate` | Persons living in a household in hardship | ratio |
| `class_share[k]` | Persons in each standard-of-living class *k* (at risk, lower-middle, middle, affluent) | ratio |
| `class_flows[k]` | Persons who entered and left class *k* since the last close | persons |

The breakdowns by county and region use the **national** median and lines (one poverty line for the country, as Eurostat). M6's group queries give the same measures for any filter over person attributes.

## Update rule
At each monthly close $t$, in this order.

**1. The population.** Every person of a private household (`hh_collective` false) with at least one member counts, with the household's weight $w_h$; a household counts $n_h w_h$ persons ($n_h$ its members). Collective households (institutions) are left out, as in the survey.

**2. Recent real income.** With $P_t$ the CPI at the close:
```math
\tilde y_{h,t} = \tilde y_{h,t-1} + \lambda_y \Big(\frac{Y_{h,t}}{P_t} - \tilde y_{h,t-1}\Big), \qquad \tilde y_{h,0} = \frac{Y_{h,0}}{P_0}
```
with $P$ the CPI relative to day 0 (so real amounts are in prices of day 0, "December 2021 lei") and $\lambda_y$ = `income_recent_speed` (1/3: about a quarter's income). Whole bani, rounded half away from zero. A household created during the month (demographics) starts at its first month's real income.

**3. Equivalised income.** $x_h = \tilde y_{h,t} / e_h$ (bani × 1,000, integer). Sorted ascending by $(x_h, h)$: the order is total and independent of how the households are stored.

**4. The measures,** with $v_h = n_h w_h$, $V = \sum v_h$, $X = \sum v_h x_h$, and $V_h$ the cumulative weight up to and including $h$ in the sorted order:
- **Median** $m_t$: the smallest $x_h$ with $V_h \ge V/2$.
- **Poverty rate:** $\sum_{x_h < \ell m_t} v_h / V$, with $\ell$ = `poverty_line_share` (0.6).
- **Gini:**
```math
G = \frac{\sum_h v_h x_h (2 V_h - v_h)}{V X} - 1
```
  (0 when every income is equal; incomes below zero are taken as zero; 0 when $X$ is zero).
- **S80/S20:** the income of the first $V/5$ persons over that of the last $V/5$, splitting the household on each boundary by weight. Undefined (none) if the poorest fifth has no income.
- **Hardship rate:** $\sum_{h \in \text{hardship}} v_h / V$, with the household's `hh_hardship` of its last decision.

All in integers (i128), ratios in millionths, sums in the sorted order.

**5. The anchor** (month 0 only): `poverty_anchor` $= m_0$ (prices of day 0). It is frozen and stored in saves; it cannot be changed in v1 (owner, 2026-10-26).

**6. Standard of living by class** (owner, 2026-10-26). Lines fixed in real terms: at risk below $0.60\,m^{2021}$, lower-middle from $0.60$ to below $0.75\,m^{2021}$, middle from $0.75$ to $2.00\,m^{2021}$, affluent above $2.00\,m^{2021}$ (`class_lines`). Each household's real equivalised recent income $x_h$ places all its members in one class. In nominal lei the lines move with the CPI only. `poverty_rate_anchored` is the share of the class at risk. Unlike the fifths, a class can empty as people get richer or swell in a crisis.

**7. Flows.** For each class, the persons whose household was in another class at the last close (entered) and those who moved out (left); a household that appeared or disappeared (demographics) counts as entering or leaving. For each class, share now − share before = (entered − left) ÷ $V$ when the population is unchanged; with births, deaths and migration the report also gives the persons added and removed per class, so the parts always sum to the change.

**8. The calendar year.** `hh_income_year` += $Y_{h,t}$. At the December close the same measures are also computed on the calendar year's income (nominal; for the classes, deflated by the year's average CPI) as the **annual** indicators, the ones comparable with the survey, and `hh_income_year` is set to zero. The first December (month 0) has only one month: its annual indicators are month 0's × 12.

> **Simplification:** income is money received. Food grown at home (`hh_subsistence_income`) and owner-occupiers' imputed rent are left out, as in the survey's headline income; Romania's rural households are poorer here than in a consumption measure. Informal income is in, while survey respondents under-report it.

> **Simplification:** one equivalence scale and one national line. No regional price levels: a leu in Bucharest buys what it buys in Vaslui.

> **Simplification:** the hardship rate stands for material deprivation but is not the survey's measure (lacking 7 of 13 items, 23.1% in 2021): it is the game's own, households whose money could not cover the subsistence basket at their last decision. It is shown, not compared with the survey.

> **Simplification:** the monthly indicators use a three-month moving average of income, which is more unequal than a year's income; the annual ones are the survey's comparison.

## Player levers
None directly. Every lever that moves incomes or prices moves them: transfers (`transfer.min_income.floor`, `transfer.pension.minimum`, `transfer.child.to18`, …), taxes, `labour.minimum_wage`, public pay. The anchor year cannot be changed in v1.

## Tuning parameters
| Parameter | Default | Range | Effect |
|---|---|---|---|
| `poverty_line_share` | 0.60 | fixed (Eurostat) | the relative poverty line |
| `class_lines` | 0.60, 0.75, 2.00 × `poverty_anchor` | fixed (owner, 2026-10-26) | the class bounds |
| `income_recent_speed` | 1/3 a month | 1/12–1 | how fast the monthly indicators follow income; 1 is last month's income only |

## Interactions
- **Reads:** [households](../economy/households.md) (income received, equivalised size, hardship), [prices](../economy/prices-inflation.md) (CPI), [demographics](../economy/demographics.md) (members and ages), [migration and remittances](../economy/migration-remittances.md) (remittances are income received).
- **Read by:** [approval](opinion-approval.md) (`hh_real_income_recent`), [social outcomes](social-outcomes.md) (poverty and Gini for crime and health, later), M6's statistics registry and group queries, M8's panel.
- **Not read by** any behaviour rule: households' spending uses the income fifths of [households](../economy/households.md).

## Edge cases & failure modes
- **No private household, or all weights zero:** every measure is none; nothing fails.
- **Everyone with zero income:** median 0; poverty rate 0 (nobody is strictly below 0); Gini 0; S80/S20 none; classes: everyone at risk (the anchor is positive).
- **Anchor zero** (cannot happen with the fixture): the classes are none for the whole run.
- **A household's equivalised size changes** (birth, death, a member leaves): $x_h$ uses the new size; the recent income is not reset.
- **A household with members but no income for months:** it stays in the population at zero; it is at risk and in the poorest fifth.
- **Deflation:** real incomes rise; the lines in nominal lei fall with the CPI.
- **Very large incomes:** i128 sums cannot overflow at 1:1 (persons × weight × income < 10³⁰).

## Acceptance tests
IDs are stable. Tests go in `crates/econ-core/tests/acceptance/poverty.rs` (protected; written by the test-writer subagent before the code), with a crate feature `poverty`. "1:100" and "1:1000" are the Romania fixture scenario at that scale, seed 42, under `Law::law_2021()`.

- [ ] **AC-POV-01** `[unit]` The population and the scale: a household's equivalised size is 1 + 0.5 per further member of 14 or more + 0.3 per member under 14; collective households are left out; each member counts with the household's weight.
- [ ] **AC-POV-02** `[unit]` The measures on given distributions: equal incomes give poverty rate 0, Gini 0 and S80/S20 1; two hand-worked weighted examples give the stated median, poverty rate, Gini and S80/S20 exactly (in millionths); a poorest fifth with no income gives S80/S20 none; permuting the households changes nothing.
- [ ] **AC-POV-03** `[unit]` Scaling every real income by the same factor (1.2, 0.8) leaves the poverty rate, Gini and S80/S20 unchanged and moves `poverty_rate_anchored` the right way (down for 1.2, up for 0.8), unless the class is already empty or full.
- [ ] **AC-POV-04** `[unit]` Classes: the four class shares sum to exactly the population's weight; at month 0 `poverty_rate_anchored` equals `poverty_rate`; the class flows satisfy share now − share before = entered − left (+ added − removed) exactly at every close of a 24-month run at 1:1000.
- [ ] **AC-POV-05** `[sim]` Starting values, at 1:100, the annual indicators of month 0: poverty rate 18–26%, Gini 0.29–0.38, S80/S20 5.0–8.5, median equivalised income within 30% of 27,129 lei a year (Romania, incomes of 2021: 21.2%, 0.320, 6.00; incomes of 2020: 22.5%, 0.343, 7.10).
- [ ] **AC-POV-06** `[sim]` At 1:1000, raising `transfer.min_income.floor` by 50% from month 3 lowers the poverty rate, the class at risk and the hardship rate within 3 months against the same run without the change; cutting it to zero raises them.
- [ ] **AC-POV-07** `[sim]` Isolation: at 1:1000, the `World`'s state hash after 24 months is the same with the indicators computed and not computed.
- [ ] **AC-POV-08** `[sim]` The annual indicators of each December close are the measures of AC-POV-02 applied to the calendar year's income of every household; `hh_income_year` is zero after the close.
- [ ] **AC-POV-09** `[unit]` Recent income: the update of step 2 in whole bani for given income, CPI and previous value; equal to month 0's income at month 0.

## API sketch
Names may change before lock; shapes should not. There is no code yet (tests come first, ADR-0014).

```rust
// crate econ-core, module `poverty` (pure functions, no World), feature `poverty`
pub struct Distribution<'a> { pub income_milli: &'a [i128], pub weight: &'a [u64] } // x_h and v_h
pub struct IncomeMeasures {
    pub median_milli: i128, pub poverty_rate_ppm: u32, pub gini_ppm: u32,
    pub s80_s20_ppm: Option<u32>, pub population: u64,
}
pub fn measures(d: &Distribution, poverty_line_ppm: u32) -> IncomeMeasures;          // AC-POV-02, 03
pub fn class_of(x_milli: i128, anchor_milli: i128, lines_ppm: [u32; 3]) -> u8;      // 0 at risk … 3 affluent
pub fn eq_adults_milli(member_ages_months: &[u16]) -> u32;                          // AC-POV-01
pub fn recent_income(prev: Bani, month_income: Bani, cpi_ppm: u32, speed_ppm: u32) -> Bani; // AC-POV-09

// in the World
pub struct PovertyStats {
    pub month: Month, pub national: IncomeMeasures, pub hardship_rate_ppm: u32,
    pub class_share_ppm: [u32; 4], pub class_entered: [u64; 4], pub class_left: [u64; 4],
    pub class_added: [u64; 4], pub class_removed: [u64; 4],
    pub by_county: Vec<IncomeMeasures>, pub by_region: Vec<IncomeMeasures>, pub by_age: [IncomeMeasures; 3],
    pub annual: Option<IncomeMeasures>,       // December closes only
}
impl World {
    pub fn poverty_stats(&self) -> &[PovertyStats];   // history, one per closed month
    pub fn poverty_anchor(&self) -> Bani;              // real, December 2021 lei a month
}
```

## Open questions
- [x] **Decided as proposed (owner's delegation, 2026-10-26).** **The anchor's month.** The owner's 2021 anchor is computed from the simulation's own incomes of December 2021 (month 0), so the class at risk starts equal to the relative rate. The alternative is Eurostat's value (60% of 27,129 lei a year), which would put the simulation's starting classes off by however far its incomes are from the survey's. *Proposed:* the simulation's own month 0.
- [x] **Decided as proposed (owner's delegation, 2026-10-26).** **AC-POV-05's bands.** They are the survey's values (both years) widened by about 3 points for a model population; if the starting population misses them, is it a test owed to M7's calibration or a failure? *Proposed:* checked, and owed to M7 with the owner's yes if it fails (the households' income gap of M7 may push the poverty rate up). The bands are not widened: if the starting population misses them, the test is recorded as owed to M7 only with the owner's explicit yes (owner rule, 2026-10-26: weakening a test needs the owner's yes).
- [x] **Decided as proposed (owner's delegation, 2026-10-26).** **Food grown at home and imputed rent.** Leave them out as the survey's headline does, or show a second rate with them in? *Proposed:* out now; a "with income in kind" variant in M8's panel if the owner wants it.
- [x] **Decided as proposed (owner's delegation, 2026-10-26).** **Breakdown in the stats record.** By county, region and three age groups now, and everything else through M6's group queries? *Proposed:* yes.
- [x] **Decided as proposed (owner's delegation, 2026-10-26).** **The "why" of a change in the poverty rate** (M6, ADR-0008): decompose the class flows by the income source whose change moved the household across a line (wages, pensions, benefits, prices). *Proposed:* M6, with the explanation trees; M5 stores the flows only.
