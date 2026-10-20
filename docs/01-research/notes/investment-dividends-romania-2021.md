---
id: research/investment-dividends-romania-2021
title: Investment, depreciation and distributed profits in Romania, 2021
status: draft
owner: horia
depends_on: [research/overview, research/opening-balance-sheet-romania]
updated: 2026-10-20
---

# Investment, depreciation and distributed profits in Romania, 2021

## Question
What did Romania's firms invest, write off as depreciation, pay out to their owners and keep in 2021? Who received the dividends: households, the state, the rest of the world? These are the calibration targets of [investment and capital](../../02-design/economy/investment-capital.md).

## Short answer
- **Investment:** gross fixed capital formation was 294.7 billion lei, 25.0% of GDP (1,179.8 billion), plus 22.2 billion of inventory change. Non-financial corporations made 143.5 billion of it and households (dwellings and unincorporated firms) 95.8 billion. The rest, about 51 billion, was government and financial firms.
- **Depreciation:** consumption of fixed capital was 196.5 billion lei (16.7% of GDP). Corporations account for 144.0 billion and households for 19.4 billion. Against the firm units' net fixed assets (2,491.6 billion), that is **6.6% a year**.
- **Dividends are small and go abroad.** Non-financial corporations paid 34.1 billion lei of distributed income (D42) and 16.7 billion of reinvested earnings on foreign direct investment (D43). The rest of the world received 18.6 billion of dividends and 22.7 billion of reinvested earnings. Households received **0.1 billion** of dividends in the national accounts: owners' income in Romania is recorded as mixed income (160.0 billion), not as dividends.
- **Firms kept most of their profits.** Corporations' gross saving was 248.7 billion lei against 143.5 billion of fixed investment. Their net lending was large (about 7% of GDP), the counterpart of a government deficit and a current-account deficit.
- **Payout ratios:** foreign owners took about 45% of their earnings as dividends (18.6 of 41.3 billion). For all non-financial corporations, dividends were about a fifth of profit after interest, depreciation and tax. State companies must pay at least half of their profit (OG 64/2001).

## Findings

### 1. National accounts, 2021 (confidence: high)
Eurostat `nama_10_gdp` and `nasa_10_nf_tr` (non-financial transactions by sector), Romania, current prices, read through the dissemination API on 2026-10-20. In billion lei:

| Item | Total economy (S1) | Non-financial corporations (S11) | Households (S14) | Rest of world (S2) |
|---|---|---|---|---|
| GDP | 1,179.8 | | | |
| Gross fixed capital formation (P51G) | 294.7 (sector table: 290.6) | 143.5 | 95.8 | |
| Changes in inventories (P52) | 22.2 | | | |
| Consumption of fixed capital (P51C) | 196.5 | 144.0 | 19.4 | |
| Gross operating surplus and mixed income (B2A3G) | 627.2 | 349.1 | 231.8 (of which mixed income 160.0) | |
| Interest paid / received (D41) | | 36.4 / 8.2 | 8.9 / 14.1 | |
| Distributed income paid / received (D42) | 36.5 / 18.4 | 34.1 / 13.2 | — / 0.1 | — / 18.6 |
| Reinvested earnings on FDI paid / received (D43) | 22.7 / 0.3 | 16.7 / 0 | | — / 22.7 |
| Current taxes on income (D5) paid | | 29.0 | 29.8 | |
| Gross saving (B8G) | | 248.7 | −12.0 | |

The government received 2.9 billion lei of dividends (D42, S13), mostly from state companies, and financial corporations received 2.2 billion. The two GFCF totals differ by vintage: `nama_10_gdp` is the newer release.

The near-zero dividends to households look odd for a country where owners of small limited companies pay themselves dividends taxed at 5%. One reading is that their income sits in mixed income and in the households' other entries. The figure is kept as published and flagged as such (confidence in what it measures: low).

### 2. The capital stock (confidence: high)
Fixture `firm_balance_sheets2021_ro.json`, from `nama_10_nfa_bs`, 2021, net at replacement cost: fixed assets of corporations 2,278.5 billion lei and business fixed assets of households 213.0 billion, together 2,491.6 billion, spread over the firm units (the spread is a guess, listed in the fixture). Corporations' and households' consumption of fixed capital, 163.4 billion, is 6.6% of that stock.

Typical depreciation rates are 2% to 3% a year for structures and 10% to 20% for machinery, vehicles and computers (Hulten and Wykoff; the US Bureau of Economic Analysis). A mixed stock at 6.6% is plausible (confidence: medium).

### 3. The input-output table's investment column (confidence: high, for the fixture)
Fixture `io_table2021_ro.json` (FIGARO 2021): investment bought at home 200.8 billion lei, imported 73.4 billion (EU 54.4, outside the EU 19.0), taxes on products 16.5 billion; inventories 22.8 billion. That is 290.7 billion of fixed investment, matching the sector table. The budget of 2021 ([budget-2021](budget-romania-2021.md)) spent 48.0 billion lei on investment (P5, with −0.14 billion of land sold).

### 4. Foreign direct investment income (confidence: medium)
In the national accounts, the rest of the world received 18.6 billion lei of dividends and 22.7 billion of reinvested earnings in 2021, so its payout was 45%. The National Bank's annual report on foreign direct investment gives the same split from the enterprise survey (from memory, not fetched: direct investment earnings of about 9 to 10 billion euro, a little over half reinvested). This income is the main part of the current account's primary-income deficit.

### 5. Rules on profit distribution (confidence: high for the law, medium for practice)
- **Dividend tax:** 5% in 2021, withheld at source (raised in later years). Dividends to EU parent companies holding at least 10% are exempt (parent-subsidiary directive).
- **State companies:** OG 64/2001 requires companies majority-owned by the state to pay at least 50% of profit as dividends. Government memoranda often asked for up to 90% in the late 2010s (medium confidence).

### 6. How investment responds (confidence: medium)
- **User cost:** estimates of how investment responds to its user cost range from about −0.25 (Chirinko, Fazzari and Meyer 1999) to −0.5 to −1.0 (Gilchrist and Zakrajšek 2007, with bond spreads). A 2-point rise of the real rate raises the user cost (real rate plus depreciation, about 9% in Romania) by about a fifth. Investment should then fall by 5% to 20%.
- **Time to build:** 2 to 4 quarters for typical projects (Kydland and Prescott 1982); longer for structures (Koeva 2000: about two years).
- **Accelerator:** investment follows the growth of demand and capacity utilisation. It is the most volatile part of GDP.

## Numbers worth keeping

| Quantity | Value | Source | Confidence |
|---|---|---|---|
| GDP 2021 | 1,179.8 bn lei | `nama_10_gdp` | high |
| Gross fixed capital formation 2021 | 294.7 bn lei (25.0% of GDP) | `nama_10_gdp` | high |
| … of which non-financial corporations / households | 143.5 / 95.8 bn lei | `nasa_10_nf_tr` | high |
| Government investment 2021 (budget) | 48.0 bn lei | budget fixture | high |
| Change in inventories 2021 | 22.2 bn lei (fixture: 22.8) | `nama_10_gdp` | high |
| Consumption of fixed capital, all / corporations / households | 196.5 / 144.0 / 19.4 bn lei | `nasa_10_nf_tr` | high |
| Firm units' net fixed assets, end 2021 | 2,491.6 bn lei | `nama_10_nfa_bs` (fixture) | high |
| Depreciation rate on the firm units' stock | 6.6% a year | derived | medium |
| Dividends paid by non-financial corporations (D42) | 34.1 bn lei | `nasa_10_nf_tr` | high |
| Dividends received by the rest of the world / reinvested earnings | 18.6 / 22.7 bn lei | `nasa_10_nf_tr` | high |
| Dividends received by households | 0.1 bn lei | `nasa_10_nf_tr` | high (as published), low (as a fact about owners) |
| Dividends received by the government | 2.9 bn lei | `nasa_10_nf_tr` | high |
| Mixed income of households | 160.0 bn lei | `nasa_10_nf_tr` | high |
| Corporations' gross saving | 248.7 bn lei | `nasa_10_nf_tr` | high |
| Corporate income tax (D51B, from the taxation spec) | 26.3 bn lei | `gov_10a_taxag` | high |
| Payout of foreign owners | ~45% | derived | medium |
| Payout of non-financial corporations | ~20% of profit after interest, depreciation and tax | derived | medium-low |
| Minimum payout of state companies | 50% | OG 64/2001 | high |
| Dividend tax | 5% (2021) | Fiscal Code | high |
| Elasticity of investment to its user cost | −0.25 to −1.0 | literature | medium |
| Time to build | 2–4 quarters | literature | medium |

## Implications for the game
For [investment and capital](../../02-design/economy/investment-capital.md):
1. **Depreciation 6.5% a year** on the firm units' fixed assets gives about 163 billion lei a year, the corporations' and households' consumption of fixed capital.
2. **Starting investment** is the table's column less the government's 48 billion. The gap between it and depreciation (about 3.5% of the stock a year) is the growth that 2021's investment was building for.
3. **Payouts from data** (about 20% domestic, 45% foreign, 50% state) do not by themselves stop firm deposits from piling up in the `World`. Romania's corporations did accumulate financial assets in 2021, but through trade credit, loans and assets abroad, which the ledger does not model. The spec therefore adds a payout of excess money, a deliberate departure, marked as a simplification.
4. **Households' dividends** cannot be calibrated from the national accounts (0.1 billion). The `World`'s owners' income is mixed income (the M2 placeholder) plus the dividends of the domestic private units; the check is households' total property and mixed income, not D42 alone.
5. **Dividends abroad (18.6 billion)** are the calibration target of the primary-income outflow in the current account ([trade and exchange rate](../../02-design/economy/trade-fx.md)).

## Sources
- [Eurostat `nama_10_gdp`](https://ec.europa.eu/eurostat/databrowser/view/nama_10_gdp/default/table): GDP and its expenditure parts; read through the API on 2026-10-20.
- [Eurostat `nasa_10_nf_tr`](https://ec.europa.eu/eurostat/databrowser/view/nasa_10_nf_tr/default/table): non-financial transactions by sector (P51G, P51C, B2A3G, D41, D42, D43, D5, B8G); read through the API on 2026-10-20.
- [Eurostat `nama_10_nfa_bs`](https://ec.europa.eu/eurostat/databrowser/view/nama_10_nfa_bs/default/table): fixed assets by sector (the fixture's capital stock).
- [Banca Națională a României, foreign direct investment in Romania](https://www.bnr.ro): annual FDI survey and balance of payments; not fetched for this note.
- OG 64/2001 on profit distribution at state companies; the Fiscal Code (Law 227/2015) on the dividend tax.
- Chirinko, Fazzari and Meyer (1999), "How responsive is business capital formation to its user cost?", *Journal of Public Economics* 74(1).
- Gilchrist and Zakrajšek (2007), "Investment and the cost of capital: new evidence from the corporate bond market", NBER WP 13174.
- Kydland and Prescott (1982), "Time to build and aggregate fluctuations", *Econometrica* 50(6). Koeva (2000), "The facts about time-to-build", IMF WP 00/138.
- Hulten and Wykoff (1981), "The measurement of economic depreciation"; the US Bureau of Economic Analysis's depreciation rates.
