---
id: research/poverty-inequality-romania-2021
title: Poverty and inequality in Romania around 2021 — EU-SILC definitions and values
status: draft
owner: horia
depends_on: [research/overview]
updated: 2026-10-26
---

# Poverty and inequality in Romania around 2021 — EU-SILC definitions and values

## Question
How does Eurostat measure the at-risk-of-poverty rate, the Gini coefficient, the income quintile share ratio (S80/S20) and material deprivation, and what were Romania's values around the start date (1 December 2021), so that [poverty and inequality](../../02-design/society/poverty-inequality.md) can compute them the same way and check the starting population against them?

## Short answer
- **Definitions (EU-SILC):** income is the household's total disposable income (after taxes and social contributions, with all transfers received), divided by the household's equivalised size on the **modified OECD scale** (1 for the first adult, 0.5 for each other person aged 14 or more, 0.3 for each child under 14) and given to every member. Indicators count **persons**, in **private households** only. The income reference period is the **previous calendar year**: the survey of 2022 measures the incomes of 2021. Confidence high.
- **Romania, survey 2022 (incomes of 2021):** at-risk-of-poverty rate 21.2%, Gini 32.0, S80/S20 6.00, median equivalised income 27,129 lei a year (poverty threshold for one adult about 16,280 lei a year, 1,357 lei a month). Confidence high.
- **Romania, survey 2021 (incomes of 2020):** at-risk-of-poverty rate 22.5%, Gini 34.3, S80/S20 7.10, median 23,367 lei. Confidence high.
- **Severe material and social deprivation** (lacking 7 of 13 items): 23.1% in 2021, 24.3% in 2022 (EU 6.3%, 6.7%). It is not an income measure; the simulation's hardship (spending below the subsistence basket) is only a proxy. Confidence high for the values, low for the proxy.
- **Imputed rent and own production are left out** of the headline income. Romania has many owner-occupiers and much home-grown food, so the money-income measures overstate its poverty against consumption-based measures. Confidence medium.

## Findings

### 1. Definitions (confidence: high)
From Eurostat's EU-SILC methodology ([Statistics Explained, "At-risk-of-poverty rate"](https://ec.europa.eu/eurostat/statistics-explained/index.php?title=Glossary:At-risk-of-poverty_rate), ["Equivalised disposable income"](https://ec.europa.eu/eurostat/statistics-explained/index.php?title=Glossary:Equivalised_disposable_income), ["Income quintile share ratio"](https://ec.europa.eu/eurostat/statistics-explained/index.php?title=Glossary:Income_quintile_share_ratio), ["Gini coefficient"](https://ec.europa.eu/eurostat/statistics-explained/index.php?title=Glossary:Gini_coefficient)):
- **At-risk-of-poverty rate:** share of persons with equivalised disposable income below 60% of the national median (after social transfers).
- **Equivalised disposable income:** the household's total disposable income divided by its equivalised size (modified OECD scale), attributed equally to each member.
- **S80/S20:** total equivalised income of the 20% of persons with the highest income over that of the 20% with the lowest.
- **Gini:** the relationship of cumulative shares of the population ranked by equivalised income to the cumulative share of income they receive, 0 to 100.
- **Anchored at-risk-of-poverty rate** (`ilc_li22b`): the share below a threshold fixed in a base year and moved only with prices (HICP). Eurostat publishes it anchored at 2008; the owner's decision of 2026-10-26 anchors the game's classes at 2021 the same way.
- **Population:** private households; people in institutions are not surveyed.
- **Income:** cash and near-cash; imputed rent is published separately and is not in the headline income; the value of goods produced for own consumption is not in total disposable income (confidence medium: it was optional before 2010 and Eurostat's headline indicators leave it out).

### 2. Romania's values (confidence: high)
Eurostat, retrieved 2026-10-26 through the dissemination API. The year is the survey's; incomes are those of the year before.

| Indicator | 2020 | 2021 | 2022 | 2023 | EU 2021 | Dataset |
|---|---|---|---|---|---|---|
| At-risk-of-poverty rate, % | 23.4 | 22.5 | 21.2 | — | — | `ilc_li02` |
| Gini, 0–100 | 33.8 | 34.3 | 32.0 | 31.0 | 30.2 | `ilc_di12` |
| S80/S20 | 6.62 | 7.10 | 6.00 | 5.83 | 4.98 | `ilc_di11` |
| Median equivalised income, lei a year | 20,250 | 23,367 | 27,129 | — | — | `ilc_di03` |
| Mean equivalised income, lei a year | 22,995 | 26,363 | 30,168 | — | — | `ilc_di03` |
| Severe material and social deprivation, % | 25.3 | 23.1 | 24.3 | 19.8 | 6.3 | `ilc_mdsd11` |
| Below 50% of the median, % | 16.4 | 17.7 | 14.8 | — | — | `ilc_li02` |
| Below 70% of the median, % | 30.2 | 31.5 | 28.0 | — | — | `ilc_li02` |

- Children under 18: Gini 37.1 (2021); S80/S20 of persons under 65 was 7.80, of persons of 65 or more 4.62.
- The large drop from survey 2023 onwards (Gini 28.0 in 2024) follows minimum wage and pension rises; it is not a calibration target.

### 3. Which year to compare with (confidence: medium)
The game's day 0 is 1 December 2021 and its incomes are those of December 2021. The survey of 2022 measures the incomes of 2021, so it is the closer comparison; the survey of 2021 (incomes of 2020, a COVID year) is the value usually quoted as "2021". A starting check should accept both: AROP 21–23%, Gini 32–34.5, S80/S20 6.0–7.1.

### 4. Monthly against annual income (confidence: medium)
The survey measures a year's income. A single month's income is more unequal: a person who lost a job in the month has little income in it but not over the year. Measures computed on one month should therefore come out above the survey; measures on a twelve-month total are the comparable ones.

## Numbers worth keeping

| Quantity | Value | Source | Confidence |
|---|---|---|---|
| AROP, survey 2022 (incomes of 2021) | 21.2% | Eurostat `ilc_li02` | high |
| AROP, survey 2021 (incomes of 2020) | 22.5% | same | high |
| Gini, survey 2022 / 2021 | 32.0 / 34.3 | Eurostat `ilc_di12` | high |
| S80/S20, survey 2022 / 2021 | 6.00 / 7.10 | Eurostat `ilc_di11` | high |
| Median equivalised income, incomes of 2021 | 27,129 lei a year | Eurostat `ilc_di03` | high |
| Poverty threshold, one adult, incomes of 2021 | about 16,280 lei a year (60% of the median) | derived | high |
| Severe material and social deprivation, 2021 | 23.1% | Eurostat `ilc_mdsd11` | high |
| OECD middle-income band | 75% to 200% of the median | [OECD, "Under Pressure: The Squeezed Middle Class" (2019)](https://www.oecd.org/en/publications/under-pressure-the-squeezed-middle-class_689afed1-en.html) | high |

## Implications for the game
- [Poverty and inequality](../../02-design/society/poverty-inequality.md) computes the four measures on persons in private households, by the modified OECD scale (which the `World` already uses for its income fifths), on money income received (not in kind, not imputed rent).
- Its starting checks (AC-POV-05) use the bands of finding 3, on twelve months of income.
- Hardship is shown next to them as the game's own measure, not compared with the survey's deprivation rate.

## Sources
- [Eurostat `ilc_li02`](https://ec.europa.eu/eurostat/databrowser/view/ilc_li02/default/table) — at-risk-of-poverty rate by poverty threshold.
- [Eurostat `ilc_di12`](https://ec.europa.eu/eurostat/databrowser/view/ilc_di12/default/table) — Gini coefficient of equivalised disposable income.
- [Eurostat `ilc_di11`](https://ec.europa.eu/eurostat/databrowser/view/ilc_di11/default/table) — S80/S20 income quintile share ratio.
- [Eurostat `ilc_di03`](https://ec.europa.eu/eurostat/databrowser/view/ilc_di03/default/table) — mean and median income.
- [Eurostat `ilc_mdsd11`](https://ec.europa.eu/eurostat/databrowser/view/ilc_mdsd11/default/table) — severe material and social deprivation rate.
- [Eurostat `ilc_li22b`](https://ec.europa.eu/eurostat/databrowser/view/ilc_li22b/default/table) — at-risk-of-poverty rate anchored at a fixed moment in time.
- Eurostat Statistics Explained glossary pages linked in finding 1 — definitions.
