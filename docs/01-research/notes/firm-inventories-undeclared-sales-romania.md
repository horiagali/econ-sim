---
id: research/firm-inventories-undeclared-sales-romania
title: Inventories and undeclared sales of Romanian firms in 2021
status: draft
owner: horia
depends_on: [research/informal-economy-romania]
updated: 2026-10-13
---

# Inventories and undeclared sales of Romanian firms in 2021

## Question
Two things the firm units of the starting economy still lack ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)): how much each unit holds in inventories at the end of 2021, and how much of its sales it does not declare. What is measured for Romania, and what has to be guessed?

## Short answer
- **Inventories are measured by sector**, not by industry. At the end of 2021 non-financial corporations held **231.9 billion lei** of inventories and households (with their unincorporated businesses and farms) **141.8 billion lei**; government 33.1 and financial corporations 0.5; the economy 407.2. Confidence high (Eurostat capital stock accounts).
- Inventories **grew by 22.2 billion lei** during 2021 (changes in inventories, about 1.9% of GDP). Confidence high.
- **Nothing found gives inventories by industry or size class for Romania.** How many days of output each industry holds is borrowed from a neighbour pattern: inventory-to-sales ratios of manufacturing, wholesale and retail trade in the United States (about 1.2 to 1.5 months of sales), and reasoning for the rest. Confidence low.
- **Undeclared sales are not measured by firm.** Two anchors exist at the level of the economy: undeclared work was **27.1% of private value added** (European Labour Authority, 2019) and the **VAT gap 36.7%** in 2021 (European Commission). Both are in the [informal economy note](informal-economy-romania.md). The European Commission's later reports revise the gap down for later years (30.0% in 2023), and part of any VAT gap is VAT declared but not paid, fraud schemes and insolvencies, not hidden sales. Confidence medium.

## Findings

### 1. Inventories by sector (confidence: high)
Eurostat `nama_10_nfa_bs`, Romania, end of 2021, current prices, net, asset `N12N` (inventories), read through the dissemination API on 2026-10-13:

| Sector | Billion lei |
|---|---|
| Non-financial corporations (S11) | 231.9 |
| Financial corporations (S12) | 0.5 |
| General government (S13) | 33.1 |
| Households and non-profit institutions (S14_S15) | 141.8 |
| Total economy (S1) | 407.2 |

For scale: fixed assets of non-financial corporations were 2,279 billion lei, so their inventories are a tenth of their fixed capital; output of the economy was 2,237 billion lei, so the total is about 66 days of output. Eurostat `nama_10_gdp`: changes in inventories (P52) 9.0 billion lei in 2020, 22.2 in 2021, 26.2 in 2022.

The households' figure is large next to their businesses' output. The statistics office does not say what is in it; likely the stocks of farm produce (the reference date is after the harvest), work in progress of households' own construction, and the goods of small unincorporated traders. Confidence that this is its content: low.

### 2. Days of output by industry (confidence: low)
No Romanian or European table was found that gives inventories by industry with output. The US Census Bureau's monthly survey of manufacturing, trade and inventories gives the ratio of inventories to sales by kind of business ([Census MTIS](https://www.census.gov/mtis/index.html)): around 2021, about 1.4 months of sales in manufacturing, 1.2 to 1.3 in wholesale trade and 1.1 to 1.3 in retail trade. In manufacturing, materials and supplies are about a third of inventories, work in progress about a third and finished goods the rest.

Reasoning for the rest:
- **Trade:** its output in the national accounts is the trade margin, about a fifth to a quarter of sales. 1.3 months of sales is therefore about 150 days of output, and nearly all of it is goods bought for resale.
- **Agriculture:** at the end of the year the harvest is in store; a third of a year's output or more.
- **Construction and real estate:** buildings under construction (work in progress) and dwellings built for sale.
- **Services:** almost nothing, a few days of supplies.
- **Energy:** fuel stocks, counted as materials.

### 3. Undeclared sales (confidence: medium for the anchors, low for any split)
- **Value added:** undeclared work was 27.1% of private-sector value added in 2019 ([ELA, 2023](https://www.ela.europa.eu/sites/default/files/2023-03/Study-on-the-extent-of-undeclared-work-in-the-EU.pdf)). Applied to 2021 this is about 250 billion lei of the firm units' private value added.
- **VAT:** the gap was 36.7% of the VAT total tax liability in 2021, about 9 billion euro ([Romania Insider, on PwC's reading of the Commission's report](https://www.romania-insider.com/pwc-romania-highest-vat-gap-eu-2021)). For 2023 the Commission gives 30.0%, more than 45 billion lei ([Council of the EU country document, 2025](https://data.consilium.europa.eu/doc/document/ST-16801-2025-ADD-23/en/pdf); [profit.ro](https://www.profit.ro/taxe-si-consultanta/neincasarea-la-tva-a-crescut-puternic-in-romania-la-45-5-miliarde-lei-pentru-2023-iar-pentru-anul-2024-este-estimata-la-49-8-miliarde-lei-potrivit-datelor-comisiei-europene-22270001)). A commentary on the latest report says the methodology changed and earlier years were revised down ([Economedia](https://economedia.ro/?p=309236)); not confirmed from the report itself.
- **What the gap is made of:** the Commission's gap includes hidden sales, fraud with missing traders, insolvencies and VAT declared but not paid. No source found splits Romania's gap between them. A model in which the gap comes only from hidden sales should therefore produce less than the measured gap.
- **Where:** small firms, cash trade, construction, agriculture, accommodation and food, personal services (as in the informal economy note). Foreign-controlled groups are audited and their tax risk is transfer pricing more than hidden sales.

### 4. What was not found
- Inventories by industry or by size class of firm (the Ministry of Finance's aggregated financial statements of companies would give them by activity; not fetched).
- The split of the households' inventories by activity.
- Any measure of undeclared sales by industry or firm size for Romania.

## Numbers worth keeping

| Quantity | Value | Source | Confidence |
|---|---|---|---|
| Inventories of non-financial corporations, end of 2021 | 231.9 billion lei | Eurostat `nama_10_nfa_bs` | high |
| Inventories of households and NPISH, end of 2021 | 141.8 billion lei | Eurostat `nama_10_nfa_bs` | high |
| Inventories of government, end of 2021 | 33.1 billion lei | Eurostat `nama_10_nfa_bs` | high |
| Inventories of the economy, end of 2021 | 407.2 billion lei | Eurostat `nama_10_nfa_bs` | high |
| Changes in inventories, 2021 | 22.2 billion lei | Eurostat `nama_10_gdp` | high |
| Inventories over sales: manufacturing, wholesale, retail (US, around 2021) | about 1.4, 1.25, 1.2 months | US Census Bureau MTIS | medium (for the US); low as a pattern for Romania |
| Undeclared work, share of private value added, 2019 | 27.1% | ELA 2023 | medium |
| VAT gap, 2021 | 36.7% | European Commission, by way of PwC | medium |
| VAT gap, 2023 | 30.0% | European Commission, by way of the Council of the EU | medium |

## Implications for the game
For [firm units: undeclared sales and inventories](../../02-design/economy/firm-units-undeclared-inventories.md):
1. **Inventory totals are measured; the spread over units is a guess.** Corporations' inventories go over all cohort units, households' over the domestic micro units, both by output times a guessed number of days for the industry. Government inventories stay with the government (not with its public units).
2. **Trade's inventories are goods bought for resale,** not its own output: they are materials of the trade units, not stock the trade units sell from in the goods market.
3. **Undeclared sales follow from undeclared value added,** whose total is the 27.1% anchor; the undeclared wages and the undeclared household income of the [incomes stage](../../02-design/society/population-incomes.md) must fit inside it.
4. **The VAT gap is a check with a band below the measured gap,** because hidden sales are only part of it.

## Sources
- [Eurostat, capital stocks by sector and asset type (`nama_10_nfa_bs`)](https://ec.europa.eu/eurostat/databrowser/view/nama_10_nfa_bs/default/table) — inventories by sector, the measured totals.
- [Eurostat, GDP and main components (`nama_10_gdp`)](https://ec.europa.eu/eurostat/databrowser/view/nama_10_gdp/default/table) — changes in inventories.
- [US Census Bureau, Manufacturing and Trade Inventories and Sales](https://www.census.gov/mtis/index.html) — inventory-to-sales ratios by kind of business, the neighbour pattern for days of output.
- [European Labour Authority (2023), Extent of undeclared work in the EU](https://www.ela.europa.eu/sites/default/files/2023-03/Study-on-the-extent-of-undeclared-work-in-the-EU.pdf) — undeclared share of private value added.
- [Romania Insider (2023), PwC: Romania has the highest VAT gap in the EU](https://www.romania-insider.com/pwc-romania-highest-vat-gap-eu-2021) — the 2021 VAT gap.
- [Council of the EU (2025), country document on Romania](https://data.consilium.europa.eu/doc/document/ST-16801-2025-ADD-23/en/pdf) — the 2023 VAT gap.
- [profit.ro, VAT not collected in 2023 and 2024](https://www.profit.ro/taxe-si-consultanta/neincasarea-la-tva-a-crescut-puternic-in-romania-la-45-5-miliarde-lei-pentru-2023-iar-pentru-anul-2024-este-estimata-la-49-8-miliarde-lei-potrivit-datelor-comisiei-europene-22270001) — the same, in lei.
- [Economedia, on the revised VAT gap figures](https://economedia.ro/?p=309236) — the revision of earlier years.
