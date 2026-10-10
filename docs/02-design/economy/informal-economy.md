---
id: economy/informal-economy
title: Informal Economy & Tax Compliance
status: draft
owner: horia
depends_on: [economy/labor-market, economy/taxation, economy/state-capacity, economy/vat, society/population-wages]
research: [research/informal-economy-romania]
updated: 2026-10-10
---

# Informal Economy & Tax Compliance

## Purpose
A large part of Romania's economy is not declared: undeclared work, "envelope wages" (a contract at the minimum wage plus cash), unreported sales, and farming for one's own table. It lowers tax revenue, leaves workers without social protection, and responds to taxes, enforcement and trust in the state. This spec says how the model holds that part, how the player sees it (**the map**), and what the player can do about it (**the tools**).

## Real-world basis
From the [research note](../../01-research/notes/informal-economy-romania.md), which has the sources and confidence of each figure:
- **Undeclared work** was 21.7% of private-sector labour input and 27.1% of private-sector value added in 2019, the highest in the EU (average 9.7% and 14.6%).
- **VAT gap:** 36.7% of the VAT due was not collected in 2021, about 9 billion euro, the highest in the EU.
- **Mixed income of households** was 160 billion lei in 2021: about 45 billion from farming, the rest declared self-employment and, mostly, informal production that the national accounts attribute to households.
- About **one million people farm for their own use** and are not employed in the census.
- National accounts already include estimates of all of this. GDP, the wage bill and mixed income in the model are totals; tax revenue comes from their declared part only.
- Informality rises with the tax and contribution wedge, with a minimum wage that is high against productivity, with weak enforcement, low trust and cash use; it falls with digital reporting and with better public services. It clusters in agriculture, construction, trade, accommodation and food, and personal services, and in small firms.

## Representation
Everything is a split of something the model already has. Nothing informal is added on top.

### Persons
| Variable | Meaning |
|---|---|
| `formality` | of an employee's job: **formal**, **partly declared** (a declared wage, the rest in cash) or **informal** (nothing declared) |
| `wage_declared` | the part of `wage` on the contract: all of it, a part, or zero by `formality`. Income tax and contributions are due on this part |
| `informal_income` | monthly income from undeclared activity outside a job: casual and seasonal work, undeclared self-employment, a second activity. Any person can have it, also the unemployed, the inactive and the retired |
| `self_employment_income`, `self_employment_declared` | income of employers and own-account workers, and its declared part |

### Households
| Variable | Meaning |
|---|---|
| `hh_farm_land` | land the household farms (already in [population-groups](../society/population-groups.md)) |
| `hh_subsistence_income` | value per month of what the household grows and eats itself. In kind: it lowers what the household must buy, it is in GDP, it is not taxed and it is not illegal |

### Firm units
| Variable | Meaning |
|---|---|
| `undeclared_sales_share` | share of a unit's sales not reported for VAT and profit tax |
| `undeclared_wage_share` | share of a unit's wage bill paid outside contracts; equals what its employees' `wage` − `wage_declared` add up to |

### What follows from the split
| On | Effect |
|---|---|
| Tax revenue | income tax and contributions on `wage_declared` and `self_employment_declared` only; VAT and profit tax on declared sales only |
| The worker | pension rights and unemployment insurance build on the declared wage only; an informal worker who loses the job gets no unemployment benefit |
| The buyer | pays the same price; the seller keeps the VAT that was not declared (part of it shows as a lower price in informal sales) |
| Statistics | every indicator of output, income and employment exists as *declared* and as *total* |
| Productivity | informal units stay small and invest less |

## Starting state
A stage of the population generator, after wages and pensions, and the firm table give every person, household and firm unit the variables above (roadmap M1). It is not specified here yet; its outline:
1. **Split mixed income** of the national accounts in three: farm income (to the census's agricultural self-employed and family workers in cash, and to rural households who farm for themselves in kind), declared self-employment income (to non-farm employers and own-account workers, tied to the wages of their occupation), and informal income (the rest).
2. **Give informal income and formality to persons** by a propensity that depends on industry group, occupation, education, activity, age and locality size, with the same splitting device as the other stages, so that national and industry totals are met.
3. **Anchors:** at the start the model's own undeclared share of private labour input and its own VAT gap are within a band of the measured 21.7% and 36.7%.

Data still to collect for it: the share of employees paid partly in cash by industry (Eurobarometer), the output of household farms for own use (economic accounts for agriculture, farm structure survey), declared self-employment income.

## Update rule
Periods are those of the [time base](../../03-architecture/decisions/0017-time-base.md): decisions about a job or a firm's reporting are taken once a month on the agent's own day; detection runs monthly.

**Propensity to be informal** for a job (person × firm unit), recomputed when the job starts and when a lever or the wage changes:
```math
p^{inf} = \text{logistic}\Big(\beta_0 + \beta_w \cdot \text{tax wedge} + \beta_m \cdot \frac{W^{min}}{\text{productivity}} - \beta_e \cdot \text{enforcement} - \beta_d \cdot \text{digitalisation} - \beta_t \cdot \text{trust} + \beta_c \cdot \text{cash share} + \beta_u \cdot \text{unemployment} + \text{industry effect} + \text{size effect}\Big)
```
- `tax wedge` = (income tax + employee and employer contributions) ÷ labour cost, at the job's wage.
- `enforcement` = probability that an undeclared job is found in a year × the penalty if found, in months of the evaded tax. The probability comes from the inspection and audit budgets through [state capacity](state-capacity.md).
- `digitalisation` = the share of transactions that leave a trace the tax office sees (e-invoicing, connected cash registers, card payments, the cash limit).
- `trust` = perceived quality of public services and of the use of public money (people evade more when they think the state wastes it).
- Workers accept informality more when unemployment is high and benefits are low.

**Undeclared sales** of a firm unit follow the same form, with digitalisation as the strongest term and the VAT and profit tax rates in place of the wedge.

**Detection.** Each month a share of undeclared jobs and sales is found, in proportion to enforcement. Found: back taxes and a fine are booked as transfers to the government ([accounting](accounting.md)), and the job becomes formal or ends. It ends when its declared cost is above what the work brings in; that is the cost of enforcement.

**Farming for own use** is not a choice against the tax office. It shrinks as households age out of it, move, or get paid work; it is a buffer in bad times.

> **Simplification:** bribes paid to avoid detection belong to [state capacity](state-capacity.md), not here.

## The map (outputs)
The player sees the informal economy as the difference between *total* and *declared*:
- **By county and locality size:** informal share of employment and of income; subsistence farming; map layers.
- **By industry group:** undeclared share of wages and of sales; the VAT gap by good category.
- **By group of people** (any filter of the group explorer): share with informal income, share of their income that is informal, share without pension rights building.
- **For the budget:** revenue lost by tax (income tax, contributions, VAT, profit tax), and revenue recovered by detection.
- **Over time:** all of the above as series, with the player's enforcement decisions marked.

Headline indicators: `informal_employment_share`, `undeclared_labour_share`, `vat_gap`, `revenue_lost`.

In v1 the player sees the true figures (perfect information). Later, with imperfect information, the informal economy is exactly what the statistics office can only estimate.

## Player levers (the tools)
| Lever | Form | Acts through | Lag | Side effects |
|---|---|---|---|---|
| Labour inspection budget | amount per year | probability that an undeclared job is found | months | some found jobs end |
| Tax audit budget | amount per year | probability that undeclared sales are found | months | compliance costs for honest firms |
| Penalties | fine as a multiple of the evaded tax | enforcement | first day of the following month | little effect when detection is low |
| E-invoicing | off / firms above a size / all firms | digitalisation of sales between firms | project, 1–3 years | set-up cost for firms |
| Connected cash registers | off / on | digitalisation of retail sales | project, 1–2 years | |
| Cash payment limit | amount per transaction | cash share | first day of the following month | approval falls among cash users |
| Lower wedge on low wages | reduced contributions up to a wage level | tax wedge where informality is highest | first day of the following month | revenue lost on wages that were declared anyway |
| Day-labourer scheme | flat tax per day of casual work, no contract | a legal form for seasonal and casual work | first day of the following month | some regular jobs move into it |
| Simplified regime for small firms | turnover tax in place of profit tax, up to a ceiling | undeclared sales of small units | first day of the following month | firms split to stay under the ceiling |
| Tax amnesty | one-off: interest and fines waived for back taxes paid | one-off revenue | first day of the following month | lowers compliance afterwards |
| Information campaign | budget | trust, for a while | months | fades |

Levers of other specs act here too: the minimum wage, every tax rate, the quality of public services, the unemployment benefit, anti-corruption funding. The first three levers are in the alpha (roadmap M4); the rest come with roadmap M15.

## Tuning parameters
| Parameter | Effect |
|---|---|
| β coefficients of the propensity | how strongly each cause moves informality; set in calibration so that the starting state reproduces the anchors and the responses below are in the range of the evidence |
| Detection efficiency | found cases per unit of budget, before state capacity |
| Formalisation share | share of found jobs that continue as formal jobs |
| Amnesty memory | how much and how long an amnesty lowers compliance |

## Interactions
- [taxation](taxation.md), [vat](vat.md): all bases are the declared parts.
- [labour market](labor-market.md): formality is a property of a job; informal jobs can pay below the minimum wage.
- [social transfers](social-transfers.md): pension rights and unemployment insurance from the declared wage.
- [households](households.md): subsistence income lowers food purchases.
- [state capacity](state-capacity.md): budgets become detection through capacity; corruption lowers it.
- [production](production.md), [firm representation](../../03-architecture/decisions/0016-firm-representation.md): undeclared shares are per firm unit, higher in small cohorts.
- [accounting](accounting.md): undeclared flows are booked like declared ones, with their own flow codes, so the ledger always shows the total economy.

## Edge cases & failure modes
- **Enforcement at zero:** informality drifts up to the level the other causes give; no revenue from detection.
- **Enforcement very high:** the undeclared shares fall, employment in low-productivity industries falls with them, revenue rises less than the evaded tax.
- **A wedge of zero on low wages:** envelope wages lose their reason; fully undeclared work (to keep a benefit, or to hide from other obligations) remains.
- **Inside the euro, or with any lever of another spec:** nothing special.

## Acceptance tests
IDs are stable (added 2026-10-10). Tests go in the crate of the mechanic, in `tests/acceptance/` (protected).

- [ ] **AC-INF-01** `[unit]` Income tax, contributions, VAT and profit tax are computed on declared amounts only: with every undeclared share set to zero, revenue equals the rates applied to the totals.
- [ ] **AC-INF-02** `[sim]` GDP and household income including informal activity equal the ledger's totals; the declared figures plus the undeclared figures equal the totals exactly, every month.
- [ ] **AC-INF-03** `[sim]` At the start the model's undeclared share of private labour input and its VAT gap are within the bands accepted with the starting-state stage.
- [ ] **AC-INF-04** `[sim]` Raising social contributions on low wages raises informal employment within two years.
- [ ] **AC-INF-05** `[sim]` Doubling the tax audit budget lowers the VAT gap over one to three years, by less when state capacity is low.
- [ ] **AC-INF-06** `[sim]` Raising penalties while detection is near zero changes the undeclared shares by less than the same rise does at the starting level of detection.
- [ ] **AC-INF-07** `[sim]` An informal worker who loses the job receives no unemployment benefit; years worked informally add nothing to pension rights.
- [ ] **AC-INF-08** `[sim]` A tax amnesty raises revenue in its month and lowers compliance afterwards.
- [ ] **AC-INF-09** `[sim]` Detection books back taxes and fines as transfers; a found job either becomes formal or ends, and the ledger invariants hold.
- [ ] **AC-INF-10** `[sim]` The map is consistent: informal shares by county, by industry group and by any partition of persons sum to the national figures.

## Open questions
- [ ] **The starting split** (roadmap M1) needs three more data sets, listed above. Until then the model has wages and pensions but no income for the self-employed.
- [ ] How much of the informal income goes to people who are not employed in the census? No source found gives it; it decides how poor the "inactive" really are.
- [ ] Should subsistence farming be a lever target (land consolidation, payments to small farms), or only a slowly shrinking state? Proposed: state only in v1; the EU farm payments of [eu-funds](eu-funds.md) reach it.
- [ ] Should the player see the true informal economy in v1 (proposed: yes), given that perfect information is the rule for v1?
