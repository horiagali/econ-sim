---
id: research/imputed-rent-romania-2021
title: Imputed rent of owner-occupiers in Romania, 2021
status: draft
owner: horia
depends_on: [research/overview, research/household-wealth-romania]
updated: 2026-10-21
---

# Imputed rent of owner-occupiers in Romania, 2021

## Question
How large is the imputed rent of owner-occupied dwellings in Romania's 2021 accounts? What share of households' consumption and of the real-estate industry is it? How should the `World` treat it, given that the household basket buys `real_estate` at the table's share?

## Short answer
- **Imputed rentals (COICOP 04.2) were 80.7 billion lei** in 2021, 11.2% of household consumption (723.5 billion). Actual rentals (04.1) were only 2.4 billion. Confidence high.
- **The table's real-estate industry (L)** has output 117.8 billion and an operating surplus of 89.0 billion. Owner-occupied dwellings (L68A) are 88.8 billion of output, with value added of 84.7 billion (95.4% of output). Confidence high.
- **Most households own their home.** 7,007,622 of 7,709,139 private households in the 2021 census (90.9%); 3.3% rent and 5.8% live under other arrangements. Confidence high.
- **The `World` today charges households for imputed rent** through the basket (`real_estate` is 13.8% of it) and gives the money to the real-estate firm units as profit. In the national accounts it is both spending and income of the owner households. It does not move as cash.
- **What the `World` loses.** Households' disposable income is understated by about 77 billion lei a year (about 6.4 billion a month). About 38% of the units' surplus is foreign-controlled, so it would leave the country as dividends.

## Findings

### 1. Household consumption, 2021 (confidence: high)
Eurostat `nama_10_co3_p3`, Romania, current prices, million lei, read through the API on 2026-10-21:

| COICOP | Item | Million lei |
|---|---|---|
| TOTAL | Household final consumption | 723,479.4 |
| CP04 | Housing, water, electricity, gas, other fuels | 129,924.7 |
| CP041 | Actual rentals for housing | 2,432.6 |
| CP042 | Imputed rentals for housing | 80,744.4 |
| CP022 | Tobacco | 27,760.2 |

CP022 checks [excise](../../02-design/economy/excise.md)'s tobacco calibration. The base is 7.79, plus the duty of 15.81, times 1.19 for VAT: 28.1 billion, within 1.3%.

### 2. The real-estate industry, 2021 (confidence: high)
Eurostat `nama_10_a64`, million lei:

| NACE | Output (P1) | Intermediate consumption (P2) | Value added (B1G) | Compensation (D1) |
|---|---|---|---|---|
| L, real estate | 123,080.8 | 26,556.7 | 96,524.1 | 2,268.6 |
| L68A, imputed rents of owner-occupied dwellings | 88,808.0 | 4,097.9 | 84,710.1 | 0 |

In the input-output table (`io_table2021_ro.json`, FIGARO), L has:
- output 117.8 billion;
- households' purchases of 92.1 billion (13.8% of the basket);
- inputs 25.0 billion and compensation 2.3 billion;
- an operating surplus of 89.0 billion.

The four `real_estate` firm units have 38% of the output under foreign control (`firm_units2021_ro.json`).

### 3. Tenure and dwelling values (confidence: high)
- **Tenure.** From `census2021_housing_ro.json`, private households: owner 7,007,622, tenant 256,636, other 444,881.
- **Dwelling values.** From `household_wealth2021_ro.json`, households' dwellings total 1,157.4 billion lei.
- **What the population carries.** Stage E gives every private household `hh_tenure` (0 owner, 1 tenant, 2 other). Stage H gives `hh_dwelling_value`, 0 for non-owners.
- **The `World` does not load them yet.** They need to be carried in its start state.
- **The yield.** Imputed rent over dwelling value is 7.0% gross a year.

## Numbers worth keeping

| Quantity | Value | Source | Confidence |
|---|---|---|---|
| Imputed rentals CP042 | 80,744.4 million lei | Eurostat `nama_10_co3_p3` | high |
| Actual rentals CP041 | 2,432.6 million lei | same | high |
| L68A value added over output | 95.4% | Eurostat `nama_10_a64` | high |
| Owner-occupier households | 90.9% | census 2021 | high |
| Imputed rent paid back (80.7 × 0.954) | ~77.0 billion lei a year | derived | medium |

## Implications for the game
- **The basket keeps its spending** on `real_estate`.
- **The real-estate units pay back the imputed part** of households' real-estate spending, net of its inputs. That is 87.7% (imputed in households' real-estate purchases, 80.7 of 92.1) times 95.4% (value added over output) = 83.65% of households' monthly real-estate spending.
- **Who receives it.** It goes to owner households by dwelling value, as income.
- **The design** is drafted in [imputed rent](../../02-design/economy/imputed-rent.md). It is meant as a dated M3 addition to [households](../../02-design/economy/households.md) once the owner confirms it.

## Sources
- [Eurostat `nama_10_co3_p3`, Romania 2021](https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/nama_10_co3_p3?geo=RO&time=2021&unit=CP_MNAC&coicop=CP041&coicop=CP042&coicop=CP04&coicop=TOTAL&coicop=CP022): household consumption by purpose.
- [Eurostat `nama_10_a64`, Romania 2021](https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/nama_10_a64?geo=RO&time=2021&unit=CP_MNAC&nace_r2=L&nace_r2=L68A&na_item=P1&na_item=B1G&na_item=D1&na_item=P2): real estate and owner-occupied dwellings.
- `python/pipeline/fixtures/census2021_housing_ro.json`, `household_wealth2021_ro.json`, `io_table2021_ro.json`, `firm_units2021_ro.json`.
