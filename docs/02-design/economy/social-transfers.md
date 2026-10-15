---
id: economy/social-transfers
title: Social Transfers — Pensions, Benefits, Student Finance, Charity
status: locked
owner: horia
depends_on: [society/population-groups, economy/fiscal-policy, economy/households, economy/taxation, economy/labor-market, society/population-pensions, economy/budget-2021, game/world-loop, adr/0017-time-base]
research: [research/law-in-force-romania-2021]
updated: 2026-10-15
---

# Social Transfers — Pensions, Benefits, Student Finance, Charity

## Purpose
Money from the state (or charities) to specific people. Transfers change their households' disposable income and so their spending, saving, education choices, health and approval. Each programme is defined by eligibility, amount, take-up and behavioural side effects.

## Real-world basis
- Pensions: PAYG systems; replacement rates of 40–80%; retirement age responds to rules and health.
- Unemployment insurance raises consumption smoothing and search duration.
- Child benefits modestly raise fertility and reduce child poverty.
- Student grants raise enrolment and completion, especially for low-income students.
- Take-up of means-tested benefits is incomplete (often 60–80%).

## Programmes (v1)
| Programme | Eligible persons | Amount per eligible person | Side effects |
|---|---|---|---|
| State pension | activity = retired | flat + replacement rate × past earnings proxy (by education and class); indexation (prices, wages, or mix) | retirement-age rule drives transitions to `retired` |
| Unemployment benefit | unemployed, months unemployed < duration | replacement rate × previous wage (capped) | reservation wage ↑ ([labor-market](labor-market.md)) |
| Child benefit | adults with dependants | per child, optionally income-tested | fertility ↑ slightly ([demographics](demographics.md)) |
| Student grant | students (tertiary, vocational) | flat, optionally means-tested by class | enrolment and completion ↑ ([education](../society/education.md)) |
| Student loan | students | loan amount; repaid as % of income above threshold after graduation | student debt stock per household; write-offs to government |
| Minimum income | adults below floor | top-up to floor × (1 − withdrawal rate × earnings) | participation ↓ if withdrawal rate high (poverty trap) |
| Housing benefit | renters in poor and lower-middle classes | share of rent | rents ↑ partly (incidence) |
| Disability and sickness | inactive due to health | flat | |

Means testing is done per person or household with their actual income and assets.

## Charity drive ("organise charity for the poor")
A temporary campaign:
- Player sets the target group (e.g. poor households, all regions), duration and government match rate (e.g. 1:1).
- Donations come from households with high income and wealth: donation rate $= d_0 \cdot \text{generosity}(\text{religiosity}, \text{ideology}) \cdot \text{income}$ (voluntary; higher with religiosity and left economic ideology, per data file).
- Funds are transferred (SFC) from donor households (+ government match) to recipients; small admin cost.
- Effects: recipients' consumption ↑ (high MPC), poverty ↓, approval ↑ (recipients, and donors who like the cause; approval has no feedback in v1); campaign fatigue lowers donations if repeated too often.

## Update rule
Per tick: for each programme and eligible person, take-up × amount → transfer from government to their household (reason code `transfer.<programme>`). Amounts indexed per rule each January.

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)). Every programme of the first version pays on the last day of the month, with wages ([World](../game/world-loop.md), phase "Pay"), for the month that ends. A change of an amount or rule applies from the first day of the following month ([ADR-0018](../../03-architecture/decisions/0018-scenario-and-commands.md), kind `law`).

> **Simplification:** Romania pays pensions and allowances on several days of the month; one pay day changes when money arrives within the month, not how much.

## First version (roadmap M3)
The four core programmes of 1 December 2021 ([law in force](../../01-research/notes/law-in-force-romania-2021.md)). Integer bani; each amount rounded once, half away from zero. The government pays; each programme posts under its own flow code; payments settle as in the [World](../game/world-loop.md) (government deposits at the central bank → reserves → household deposits).

**In:**
1. **State pension.** Each retired person holds `pension_points`, set once at load from the starting pension ([stage G](../society/population-pensions.md)): points = `pension` ÷ `pension_point_value` (1,442 lei), in millionths of a point. The month's pension = points × `pension_point_value`, not below `minimum_pension` (800 lei, the social indemnity for pensioners). At load this gives back each starting pension to the ban, except below the minimum, where it gives the minimum. Income tax on pensions is [taxation](taxation.md)'s.
2. **Unemployment benefit.** A person who loses a job (layoff, not a quit) gets the benefit for `benefit_months_left` months, set when the job is lost from the person's contribution years: 6 months for 1 to 5 years, 9 for 5 to 10, 12 for more; nothing under one year. Amount a month = `ui_isr_share` (75%) × `reference_social_indicator` (500 lei) + a share of `last_wage` (3% for 3 to 5 contribution years, 5% for 5 to 10, 7% for 10 to 20, 10% for more). It stops when the person finds a job or the months run out.

> **Simplification:** there is no contribution history in the population; contribution years are the years since the person turned 18 less the years of schooling above upper secondary, capped by age (low confidence). Persons already unemployed on the start date have no benefit: their job loss and record are unknown (low confidence; the registered beneficiaries in 2021 were far fewer than the census's unemployed).

3. **Child allowance**, to the child's household, every child under 18: `child_allowance_under2` (427 lei) under 2, `child_allowance` (214 lei) from 2 to 18. No means test; full take-up.
4. **Guaranteed minimum income**, to the household: the floor for its size (142, 255, 357, 442, 527 lei for 1 to 5 members, plus 36 lei for each further member; medium confidence) less the household's money income of the month after taxes and the other programmes, if positive. Take-up: a household that qualifies claims with probability `gmi_take_up` (60%, low confidence), decided once per household by a keyed draw and kept while it qualifies. The withdrawal rate is 100% (each leu earned lowers the benefit by a leu).

> **Simplification:** the means test looks at the month's income only, not at assets; collective households receive nothing from programmes 3 and 4.

**Not in M3:** retirement and new pensioners (people over time, M5); pension indexation rules (the point's value is a lever, M4); child-raising leave, disability and sickness benefits, housing benefit, student grants and loans (M4 and M14); the charity drive (M14); behavioural side effects (reservation wages, participation, fertility: M5 and later).

**Flow codes** (proposed; consistent with [budget-2021](budget-2021.md)): 4001 `benefit.pension`, 4003 `benefit.family` (child allowance), 4004 `benefit.unemployment`, 4006 `benefit.social_assistance` (minimum income).

## Player levers
All parameters in the table, plus programme on/off. See [levers](../game/levers.md).

## Acceptance tests
IDs are stable (added 2026-10-14). Tests go in a new crate `crates/econ-mech-transfers`, in `tests/acceptance/` (protected), written in a test-authoring session.

- [ ] **AC-ST-01** `[unit]` A pension is points × point value, not below the minimum pension. At load, points from a starting pension give that pension back to the ban (the minimum where it was below); raising the point's value by 10% raises every pension above the minimum by 10%, rounded to the ban.
- [ ] **AC-ST-02** `[unit]` The unemployment benefit: 375 lei plus the share of the last wage for the contribution years (3%, 5%, 7%, 10%), for 6, 9 or 12 months; nothing under one year of contributions or after a quit; it stops the month the person is hired or the months run out.
- [ ] **AC-ST-03** `[unit]` The child allowance is 427 lei under 2 and 214 lei from 2 to 18, nothing from 18, paid to the child's household.
- [ ] **AC-ST-04** `[unit]` The minimum income is the floor for the household's size less its month's income after taxes and the other programmes, never below zero; with the 100% withdrawal rate, a leu more of earnings lowers it by a leu until it reaches zero.
- [ ] **AC-ST-05** `[sim]` On every pay day, for each programme, the government's payment equals the weighted sum of the persons' or households' amounts, and the ledger records it under the programme's flow code; each household's `hh_income_received` grows by its share.
- [ ] **AC-ST-06** `[sim]` On the Romania scenario, the first month's pensions equal stage G's pension column person by person, except the persons raised to the minimum; the year's pension bill is within 2% of 2021's ([budget-2021](budget-2021.md), old-age and survivors).
- [ ] **AC-ST-07** `[sim]` Doubling the child allowance lowers the number of households with children in hardship ([households](households.md)) within two months, and raises their consumption.
- [ ] **AC-ST-08** `[sim]` Laying off 10% of one industry's staff raises the unemployment benefit bill in the following month by about the laid-off persons' benefit, and the bill falls as they are hired back.
- [ ] **AC-ST-09** `[sim]` *(M6)* Raising pensions by 10% raises pensioner households' consumption of health, food and energy within three months.
- [ ] **AC-ST-10** `[sim]` *(M14)* A student grant increase transfers exactly grant × students × take-up from government to student households every month.
- [ ] **AC-ST-11** `[sim]` *(M14)* A charity drive lowers the wealth of donor households and raises the consumption of recipient households by the same total, less its administrative cost.
- [ ] **AC-ST-12** `[sim]` *(M5)* Minimum income with a 100% withdrawal rate produces lower participation among eligible persons than with 50%.

### Tests owed
The spec was locked before these tests could exist (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-ST-05: the World's pay-day phase calling `econ-mech-transfers` and posting through the ledger (roadmap M3)
- AC-ST-06: the World's pay-day phase calling `econ-mech-transfers` and posting through the ledger (roadmap M3)
- AC-ST-07: the World's pay-day phase calling `econ-mech-transfers` and posting through the ledger (roadmap M3)
- AC-ST-08: the World's pay-day phase calling `econ-mech-transfers` and posting through the ledger (roadmap M3)
- AC-ST-09: roadmap M6
- AC-ST-10: roadmap M14
- AC-ST-11: roadmap M14
- AC-ST-12: roadmap M5

## API sketch
A new crate `crates/econ-mech-transfers` (pure functions, no I/O). Amounts are bani; shares millionths; points millionths of a point.

```rust
pub struct PensionRule { pub point_value: Bani, pub minimum: Bani }
pub fn pension_points(starting_pension: Bani, point_value: Bani) -> u64;            // millionths of a point
pub fn pension(points: u64, r: &PensionRule) -> Bani;                               // AC-ST-01
pub struct UnemploymentRule { pub isr: Bani, pub isr_share_ppm: u32,
                              pub wage_share_by_years: Vec<(u8 /* from years */, u32 /* ppm */)>,
                              pub months_by_years: Vec<(u8, u8)> }
pub fn benefit_months(contribution_years: u8, r: &UnemploymentRule) -> u8;
pub fn unemployment_benefit(last_wage: Bani, contribution_years: u8, r: &UnemploymentRule) -> Bani;   // AC-ST-02
pub struct ChildRule { pub under2: Bani, pub to18: Bani }
pub fn child_allowance(age_months: u16, r: &ChildRule) -> Bani;                     // AC-ST-03
pub struct MinimumIncomeRule { pub floor_by_size: Vec<Bani> /* 1..=5 members */, pub per_extra: Bani,
                               pub take_up_ppm: u32 }
pub fn minimum_income(size: u16, month_income: Bani, r: &MinimumIncomeRule) -> Bani;   // AC-ST-04
```

The pay-day phase that walks persons and households, applies take-up and posts through the ledger lives with the [World](../game/world-loop.md).

## Open questions
- [x] Private pension funds: Pillar II is in v1 as a financial sub-sector holding equity and bonds for member households ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md), [accounting](accounting.md)); contribution rules and Pillar III still open. **Decided (owner, 2026-10-14):** Pillar II as written (its share of CAS is routed by [taxation](taxation.md)); Pillar III not in v1.
- [x] Universal basic income as a separate programme? **Decided (owner, 2026-10-14):** revisited with the M4 levers, not in the first version.
- [x] **First version as above** (2026-10-14 draft): one pay day, pension points, the unemployment benefit's contribution-years proxy and no benefit for the starting unemployed, the minimum income's floors and 60% take-up. Accept, or change? **Decided (owner, 2026-10-14):** accepted.
- [ ] **The minimum-income floors and the benefit shares by contribution years** are from memory (medium confidence); check against the law when the lever registry is filled (M4).
