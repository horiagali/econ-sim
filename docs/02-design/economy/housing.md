---
id: economy/housing
title: Housing Market
status: draft
owner: horia
depends_on: [economy/households, economy/money-banking, economy/demographics, economy/production]
research: []
updated: 2026-10-08
---

# Housing Market

## Purpose
House prices and rents by county, home ownership, new construction, vacant homes from emigration, and mortgages. Housing is most households' main wealth, a big share of spending for renters, a driver of internal migration (expensive Bucharest and Cluj vs cheap rural areas), and a channel for monetary policy.

## Real-world basis
*(Verify Romanian figures.)*
- Romania has one of the highest home-ownership rates in Europe (much of it from post-1990 privatisation of flats), a relatively small rental market, high overcrowding, and many vacant dwellings in rural and depopulating areas.
- House prices respond to incomes, interest rates, credit availability, population changes and supply; supply responds slowly to prices (permits, construction capacity).
- Rents follow local demand (jobs, students, migrants) and supply.

## State (per county, urban and rural)
| code_name | Meaning |
|---|---|
| `dwellings` | Dwelling stock (units, average size and quality) |
| `dwellings_vacant` | Vacant units |
| `house_price` | Average price per m² |
| `rent` | Average rent per m² |
| `permits`, `under_construction` | Supply pipeline |

Each household has a dwelling with tenure, size and value ([population-groups](../society/population-groups.md)).

## Update rule (monthly)
**Demand for housing services** in county $c$: households, their size and income, students, migrants in and out.

**Rent:**
```math
\Delta \ln \text{rent}_c = \lambda_r \Big(\frac{\text{demand}_c}{\text{occupied supply}_c} - 1\Big) + \pi^e
```
**House price** (user-cost / asset pricing with sticky expectations):
```math
\text{price}^*_c = \frac{\text{rent}_c \cdot 12}{r^M + \tau^{prop} + \delta - g^e_c}, \quad \text{price}_{c,t+1} = \text{price}_{c,t} + \lambda_p (\text{price}^*_c - \text{price}_{c,t}) + \text{credit effect}
```
with $g^e_c$ expected price growth (adaptive, can create booms) and a credit term from mortgage availability (LTV/DSTI limits, state guarantee schemes).

**New construction** by developers (part of `construction_buildings`): starts ∝ (price / construction cost − 1) × permits availability; completion after 18–36 months; public housing via the project system ([infrastructure](infrastructure.md)).

**Vacancies and depopulation:** emigration and internal migration leave dwellings vacant; vacant stock depreciates faster; in depopulating counties prices fall.

**Transactions:** households buy or sell when they move, form or upgrade; first-time buyers need deposits and mortgages ([money-banking](money-banking.md)).

## Effects
- Housing wealth → consumption (wealth effect) and collateral.
- Rent burden → disposable income of renters, student costs, internal migration, fertility.
- Construction activity → jobs and materials demand.
- Property tax base.

## Player levers
Property tax, reduced VAT on new homes, state-guaranteed mortgage schemes (first-home programmes), LTV/DSTI limits (central bank), public and social housing projects, rent control (cap on rent growth: lower rents for sitting tenants, less supply long-run), building-permit and zoning liberalisation (faster supply response), housing benefit.

## Outputs
Prices and rents by county, price-to-income, rent burden, ownership rate, vacancy rate, construction starts, mortgage stock.

## Acceptance tests
- [ ] Lower mortgage rates raise house prices, more where supply is tight.
- [ ] A county losing population sees rising vacancies and falling prices.
- [ ] A rent cap lowers rents for renters in the short run and reduces new rental supply in the long run.
- [ ] Building public housing lowers rents in that county relative to a counterfactual.

## Open questions
- [ ] County-level only, or separate city markets for Bucharest, Cluj-Napoca, Timișoara, Iași, Constanța, Brașov (proposed: county × urban/rural is enough)?
