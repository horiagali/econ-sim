---
id: economy/social-transfers
title: Social Transfers — Pensions, Benefits, Student Finance, Charity
status: draft
owner: horia
depends_on: [society/population-groups, economy/fiscal-policy, economy/households]
research: []
updated: 2026-10-09
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

## Player levers
All parameters in the table, plus programme on/off. See [levers](../game/levers.md).

## Acceptance tests
- [ ] Raising the pension by 10% raises pensioner households' consumption of health, food and energy and their approval within 1–3 ticks.
- [ ] A student grant increase transfers exactly grant × students × take-up from government to student households every tick.
- [ ] A charity drive reduces the wealth of donor households and raises the consumption of recipient households by the same total (minus admin).
- [ ] Minimum income with 100% withdrawal produces lower participation among eligible persons than with 50%.

## Open questions
- [ ] Private pension funds: Pillar II is in v1 as a financial sub-sector holding equity and bonds for member households ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md), [accounting](accounting.md)); contribution rules and Pillar III still open.
- [ ] Universal basic income as a separate programme?
