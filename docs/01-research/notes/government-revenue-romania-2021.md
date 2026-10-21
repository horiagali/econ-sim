---
id: research/government-revenue-romania-2021
title: General government revenue in Romania, 2021
status: draft
owner: horia
depends_on: [research/overview, research/budget-romania-2021, research/investment-dividends-romania-2021]
updated: 2026-10-21
---

# General government revenue in Romania, 2021

## Question
What did Romania's general government (S13) collect in 2021, line by line in the ESA 2010 accounts? Which lines does the `World` collect already, which do [excise](../../02-design/economy/excise.md) and [EU funds](../../02-design/economy/eu-funds.md) add in M3, and what is left? For what is left, what is the least M3 can do?

## Short answer
- **Total revenue (TR) was 388,854.5 million lei** (33.0% of GDP, 1,179.8 billion), against 474,031.5 million of spending: an ESA deficit of 85,177.0 million (7.2% of GDP). Confidence high (Eurostat `gov_10a_main`).
- **Taxes and contributions were 321.8 billion:** VAT 76.3, excise 34.7, other taxes on products 6.1, other taxes on production 9.7, income and other current taxes 61.0, social contributions 133.9 (of which 9.6 imputed, with no cash). The other 67.1 billion are sales of public units (26.5), property income (6.8), current transfers (13.0, of which 6.6 from the EU) and capital transfers (20.9, of which 16.0 from the EU).
- **Excise was 34.7 billion lei** (D214A 32,639.5 + D2122C 2,024.0). The Fiscal Council splits the cash receipts (34.48 billion): energy products 50.1%, tobacco 45.6%, alcohol about 4%, the rest under 1%. Confidence high for the total, medium for the split.
- **EU money through the government: 22.6 billion lei** (current 6.55, capital 16.02). Farmers' direct payments (about 9.8 billion) are not government revenue in ESA: they are subsidies paid by the EU institutions directly. Own resources paid (D76): 12.1 billion. Confidence high for the ESA lines, medium for the direct payments.
- **The `World` collects about 199 billion a year.** With excise and EU funds it would be about 254 billion. Of the 135 billion left, about 36 billion are classification: public units' sales are netted against what the government pays them, and imputed contributions are not cash. About 99 billion is a real gap: VAT (about 33: VAT on firms' inputs and on investment, priced in but never posted, and households' lower spending); labour and profit taxes (about 23) and excise (about 5), mostly because households and firms earn and spend less than in 2021; small taxes and non-tax receipts (about 38, of which about 26 M3 can hold at their 2021 level).

## Findings

### 1. Revenue by ESA line, 2021 (confidence: high)
Eurostat `gov_10a_main` and `gov_10a_taxag`, S13, million lei, read through the dissemination API on 2026-10-21. The lines add up exactly to TR.

| ESA | Line | Million lei | Detail |
|---|---|---|---|
| P11, P12, P131 | Sales: market output, own use, payments for non-market output | 26,482.3 | P11 7,647.5; P12 1,362.8; P131 17,472.0 |
| D211 | VAT | 76,336.1 | |
| D214A, D2122C | Excise duties | 34,663.5 | 32,639.5 on domestic release, 2,024.0 on non-EU imports |
| other D21 | Other taxes on products | 6,120.2 | general sales or turnover taxes D214I 3,579.9 (mostly the clawback tax on medicines, medium); n.e.c. D214L 1,718.0; financial transactions 391.9; stamp 365.9; other 84.5 |
| D29 | Other taxes on production | 9,683.5 | land and buildings 3,601.4; pollution 3,900.5; licences 953.2; use of fixed assets (firms' vehicles) 758.1; other 470.3 |
| D51A | Taxes on household income | 28,493.9 | |
| D51B | Taxes on corporate income | 26,285.3 | profit and micro taxes |
| D51D | Taxes on winnings from gambling | 2,376.3 | |
| D59 | Other current taxes | 3,864.1 | households' buildings and land 2,532.7; vehicle licences 1,228.2; other 103.2 |
| D91 | Capital taxes | 0.0 | |
| D611 + D613 | Actual social contributions | 124,334.0 | employers 11,289.1; households 113,044.9 |
| D612 | Imputed social contributions | 9,598.1 | counterpart of the special pensions; no cash moves |
| D41 | Interest | 1,287.1 | |
| D42 | Distributed income of corporations | 2,866.9 | dividends of state companies and the central bank's profit |
| D45 | Rent | 2,619.6 | royalties on oil, gas and minerals; concessions of land |
| D74 | Current international cooperation | 6,550.2 | all from the EU institutions (S212) |
| D75, D72 | Miscellaneous current transfers | 6,418.7 | fines, penalties, other |
| D92 | Investment grants | 20,101.7 | from the EU: 16,017.5 (D9 from S212); from residents: about 4,084 |
| D99 | Other capital transfers | 773.0 | |
| | **Total revenue (TR)** | **388,854.5** | 33.0% of GDP |

### 2. The Ministry of Finance's cash budget (confidence: high)
The consolidated budget execution of 31 December 2021 (cash, national methodology) gives total revenue of 379,612.9 million lei. The lines that matter here:
- excise 34,484.8 million, +12.3% on 2020;
- VAT, net of refunds, 79,350.9;
- property taxes 6,547.4;
- non-tax revenue 26,149.1 (concessions and rents, dividends of state companies);
- customs duties 1,526.8 (traditional own resources, passed to the EU);
- **EU money received against payments made, 38,412.9:** 38,125.2 for the 2014–2020 framework, 257.9 other, 22.6 pre-financing and 3.2 donations.

The cash EU figure is larger than ESA's 22.6 billion, for two reasons:
- **Direct payments to farmers** (EAGF) are pre-financed from the state budget and reimbursed by the EU, so they pass through the cash budget. In ESA they are subsidies from the EU institutions to the farmers.
- **Timing:** ESA records grants when the spending they finance happens.

### 3. Excise by group (confidence: medium)
The Fiscal Council's annual report (cited by Forbes Romania) splits 2021's excise receipts:
- energy products 50.1%: motor fuels, heating oil, LPG, electricity and natural gas;
- tobacco 45.6% (46.8% in 2020);
- alcohol and alcoholic beverages about 4%;
- every other category under 1%.

Applied to the ESA total (34,663.5 million):

| Group | Share | Million lei |
|---|---|---|
| Energy products | 50.1% | 17,366 |
| Tobacco products | 45.6% | 15,807 |
| Alcohol, beer, wine | 4.0% | 1,387 |
| Other | 0.3% | 104 |

- **Electricity and natural gas** are inside "energy products." Their duties are near the EU minimums (0.5 to 1 euro per MWh of electricity, about 0.15 to 0.3 euro per GJ of gas). They are likely about 0.5 billion lei together. Confidence low.
- **Rates in force in 2021** (medium): beer 3.77 lei per hectolitre per degree Plato; still wine 0; cigarettes a specific plus a 14% ad valorem element, raised on 1 January and 1 April 2021.
- **Indexation:** the Fiscal Code indexes the specific amounts every year by the CPI. Indexation was suspended in 2020 and 2021 and resumed in January 2022. Confidence medium.

### 4. EU flows, 2021 (confidence: high for ESA, medium for the rest)
- **To the government (ESA):**
  - current grants (D74 from S212), 6,550.2 million lei: the current parts of the operational programmes (ESF and technical assistance);
  - capital grants (D9 from S212), 16,017.5 million: ERDF, Cohesion Fund, EAFRD investment measures, and the first RRF spending.
- **To farmers, outside the government (ESA):** the EAGF direct payments. The input-output table gives agriculture (A01) net other subsidies on production of 12,185.1 million lei. Less the government's own other subsidies on production (D39, 2,379.1, nearly all to farmers), that leaves **about 9,806 million lei (about 2.0 billion euro)** for the EU's direct payments. Confidence medium.
- **From Romania to the EU:**
  - VAT- and GNI-based own resources (D76): 12,105.2 million lei;
  - customs duties (traditional own resources), about 1.5 billion collected, which ESA records as paid directly to the EU.
- **The net position** was about +4 to +5 billion euro (secondary sources: 7.1 billion received against 2.3 paid, or 6.34 against 2.76 without the recovery facility). In this note's terms: 22.6 + 9.8 − 12.1 ≈ 20.3 billion lei (4.1 billion euro).
- **RRF pre-financing,** paid in December 2021 (about 1.85 billion euro of grants and 1.94 billion of loans), is a financial advance in ESA, not revenue.

### 5. What the `World` collects (measured, low precision)
- **Total.** Under the 2021 law, with dividends (roadmap, 2026-10-21), the `World` collects about 199 billion lei a year.
- **VAT:** about 40 billion. Households spend about 20% less than in 2021, and the `World` charges VAT on household purchases only.
- **Income tax, contributions, profit and micro tax, the dividend tax, the central bank's profit and state units' dividends:** about 159 billion together. 2021's D51A + D51B + D611 + D613 + D42 is 182.0 billion.
- The first year's split by tax will be in AC-FIS-05's report.

### 6. Prices already carry taxes that nobody collects (confidence: medium)
- **What the table holds.** Product taxes in the input-output table (`io_table2021_ro.json`, FIGARO):
  - 46.1 billion lei on industries' inputs;
  - 52.0 billion on household consumption;
  - 16.5 billion on investment.

  These 114.5 billion are within 3% of D21 (117.1).
- **Where the `World` puts the input part.** Industries' product taxes on inputs, and their other taxes on production (8.1 billion gross), are in each industry's "other costs." These enter unit cost and therefore prices ([prices](../../02-design/economy/prices-inflation.md), step 1).
- **What happens to that money.** No one posts it to the government, so firms keep it as surplus.
- **The rest of VAT is there.** The VAT 2021 collected beyond households is mostly:
  - the input VAT that exempt industries cannot deduct (finance, health, education, real estate);
  - VAT on new dwellings and on exempt buyers' investment.
- **Agriculture's negative other taxes** (−12.2 billion) are subsidies built into its prices. No one pays them yet.

## Numbers worth keeping

| Quantity | Value | Source | Confidence |
|---|---|---|---|
| Total revenue TR 2021 | 388,854.5 million lei | Eurostat `gov_10a_main` | high |
| Excise (D214A + D2122C) | 34,663.5 million lei | Eurostat `gov_10a_taxag` | high |
| Excise, cash | 34,484.8 million lei | MF execution, Dec 2021 | high |
| Excise split energy / tobacco / alcohol | 50.1% / 45.6% / ~4% | Fiscal Council, via Forbes Romania | medium |
| EU current grants to S13 (D74 from S212) | 6,550.2 million lei | `gov_10a_main` | high |
| EU capital grants to S13 (D9 from S212) | 16,017.5 million lei | `gov_10a_main` | high |
| EAGF direct payments to farmers | ~9,806 million lei | io table A01 net subsidies less D39 | medium |
| EU own resources paid (D76) | 12,105.2 million lei | `gov_10a_main` | high |
| EU money received, cash | 38,412.9 million lei | MF execution | high |
| Property taxes, cash | 6,547.4 million lei | MF execution | high |
| Non-tax revenue, cash | 26,149.1 million lei | MF execution | high |

## The gap, line by line
In billion lei a year.
- **2021** is the ESA figure (high confidence).
- **World now** is the `World` under the 2021 law with dividends: a measured total, and a split that is partly estimated.
- **After** adds [excise](../../02-design/economy/excise.md) and [EU funds](../../02-design/economy/eu-funds.md) at their M3 calibration. Excise is the first-year estimate with households spending 20% below 2021; at the table's quantities it is 34.7.

| Line | 2021 | World now | After excise and EU funds | Left | What it is |
|---|---|---|---|---|---|
| VAT | 76.3 | ≈40 | ≈43 (VAT on the excise, ≈2.8) | ≈33 | households' spending gap; non-deductible input VAT and VAT on investment, priced but not posted |
| Excise | 34.7 | 0 | ≈30 | ≈5 | households' spending gap |
| Other taxes on products | 6.1 | 0 | 0 | 6.1 | clawback, stamp, financial transactions |
| Other taxes on production (D29) | 9.7 | 0 | 0 | 9.7 | in unit costs, kept by firms |
| Income and corporate taxes, contributions, dividends (D51A, D51B, D611, D613, D42) | 182.0 | ≈159 | ≈159 | ≈23 | lower wages, employment and profit than 2021 |
| Gambling winnings tax (D51D) | 2.4 | 0 | 0 | 2.4 | |
| Households' property and vehicle taxes (D59) | 3.9 | 0 | 0 | 3.9 | |
| Imputed contributions (D612) | 9.6 | 0 | 0 | 9.6 | no cash; classification |
| Sales of public units (P1) | 26.5 | netted | netted | 26.5 | classification (below) |
| Interest (D41) | 1.3 | 0 | 0 | 1.3 | |
| Rent, royalties (D45) | 2.6 | 0 | 0 | 2.6 | |
| EU current grants (D74) | 6.6 | 0 | 6.6 | 0 | |
| Other current transfers (D75) | 6.4 | 0 | 0 | 6.4 | fines, penalties |
| EU capital grants (D9 from EU) | 16.0 | 0 | 16.0 | 0 | |
| Other capital transfers | 4.9 | 0 | 0 | 4.9 | |
| **Total** | **388.9** | **≈199** | **≈254** | **≈135** | |

**Classification, not a deficit (≈36 billion).**
- **Public units' sales.** The `World` pays a public producer's shortfall at cost (the [World](../../02-design/game/world-loop.md)'s open questions). Its sales to households and firms (fees, co-payments, market output: P1, 26.5 billion) therefore already lower what the government pays, instead of showing as revenue.
- **Imputed contributions** (9.6 billion) have no cash leg on either side.
- **Both belong in the report, not the ledger.** AC-FIS-05's comparison with TR should either add them as memo items or compare with TR less both (352.8 billion).

## Implications for the game
**Minimal M3 treatment of what is left.** In order of size, with a recommendation each.

1. **Taxes already in prices: post them (recommended, M3; ≈ 35 to 45 billion).**
   - **Product taxes on inputs.** Each industry's "other costs" from the table's product taxes on its inputs (less the fuel excise, which [excise](../../02-design/economy/excise.md) now posts) become a payment to the government with each day's input purchases, under `tax.products`. This is non-deductible VAT in its simplest form, VAT's "Later" item: a fixed share of the industry's input purchases.
   - **Investment.** The table's product taxes on investment (16.5 billion; VAT on dwellings and on exempt buyers' equipment) are charged on investment purchases the same way.
   - **Why.** Prices do not change, since the cost is already in them. The money goes to the government instead of firms' surplus. It closes most of VAT's gap and makes price and revenue consistent.
   - **Owner.** [VAT](../../02-design/economy/vat.md) is locked, so this is the owner's decision. Drafted in [government revenue at 2021 levels](../../02-design/economy/government-revenue-2021.md), part 1 (code `tax.products`): about 34.7 billion on inputs and 13.8 on firms' investment. It includes the other taxes on products (6.1), which are therefore not held below.
2. **Other taxes on production, D29 (recommended, M3; 8.1 billion at the table's level, 9.7 in ESA).**
   - **What moves.** The positive part of each industry's "other taxes" row in the table is paid monthly by its units, in proportion to their output, under a new code `tax.production` (3013).
   - **Prices.** Already in unit cost, so prices do not change.
   - **Later.** Land and building tax on firms becomes the real property tax with [housing](../../02-design/economy/housing.md) (M13); pollution fees move to [environment](../../02-design/economy/environment.md).
3. **Small taxes and non-tax receipts, held at their 2021 level (recommended, M3; ≈ 21 billion; drafted in government revenue at 2021 levels, part 3).** Each is a twelfth of its 2021 amount at month end, like the budget's held spending lines, from the payer below:
   - households' property and vehicle taxes (D59, 3.9), from owner households (`hh_tenure` = owner), equal per household, about 46 lei a month;
   - gambling winnings tax (2.4), from households in proportion to consumption;
   - rent and royalties (2.6), from the extraction units (`crude_oil`, `natural_gas`, `coal_mining`, `metal_ores`, `quarrying`) by output;
   - other current transfers received (6.4), half from households and half from firms;
   - other capital transfers received (4.9), from firms.

   - interest received (D41, 1.3), from the banks.

   Proposed codes: 3014 `tax.property`, 3015 `tax.gambling`, 2011 `rent.natural_resources`, 2012 `interest.gov_received`, 4107 `transfer.other_received`, 4108 `transfer.capital_received`.

   > **Simplification (M3):** the property tax on dwellings is equal per owner household until housing (M13) gives dwellings a value; confidence low that the distribution is right, high that the total is.
4. **Interest received** (D41, 1.3 billion): held with item 3, paid by the banks. The treasury's deposits at the central bank earn nothing in the `World`.
5. **Agricultural subsidies built into prices** (−12.2 billion in A01's other taxes). Two payments cover them; together they make agriculture's surplus the table's:
   - the EU's direct payments, in [EU funds](../../02-design/economy/eu-funds.md)'s first version (9.8 billion, rest of the world → farm units);
   - the budget's other subsidies on production (D39, 2.4 billion), which the treasury already pays under `subsidy.production`. They should go to the farm units by the same key.
6. **What only calibration fixes** (≈ 23 billion of labour and profit taxes, ≈ 5 billion of excise and part of VAT): households spend about 20% less than in 2021 and employment is lower. This is the "calibration under the 2021 law" item of the roadmap, not a missing tax.

**Expected result.**
- **Revenue.** With items 1 to 3, the `World` would collect about 254 + 48 + 8 + 21 ≈ 331 billion lei, against TR less the classification items, 352.8 billion.
- **Deficit.** It would be about 2 to 4% of GDP above 2021's 7.2% before calibration, instead of about 11 points above.

## Sources
- [Eurostat `gov_10a_main`, Romania, S13, 2021, million lei](https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/gov_10a_main?geo=RO&time=2021&unit=MIO_NAC&sector=S13): every ESA revenue and spending line, by counterpart (EU institutions S212).
- [Eurostat `gov_10a_taxag`, Romania, S13, 2021, million lei](https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/gov_10a_taxag?geo=RO&time=2021&unit=MIO_NAC&sector=S13): taxes and contributions by detailed ESA code.
- [Ministry of Finance, budget execution at 31 December 2021 (via HotNews)](https://media.hotnews.ro/media_server1/document-2022-01-28-25332963-0-executia-bugetara-2021.pdf): cash revenue by line, excise, EU receipts, property taxes, non-tax revenue.
- [Forbes Romania, Fiscal Council on 2021 excise](https://www.forbes.ro/?p=279347): excise shares by group (energy 50.1%, tobacco 45.6%, alcohol about 4%).
- [Economedia, 2021 deficit](https://economedia.ro/romania-a-incheiat-2021-cu-un-deficit-de-80-miliarde-lei-respectiv-672-din-pib.html): cash deficit 80 billion lei, 6.72% of GDP.
- [Gandul, EU funds received and paid](https://www.gandul.ro/actualitate/exclusiv-avem-bani-europeni-ce-facem-cu-ei-cat-mai-are-de-cheltuit-romania-pana-in-2031-europarlamentar-media-sa-fie-de-peste-10-mld-e-an-19942271) and [Profit.ro, Romania's net position](https://www.profit.ro/taxe-si-consultanta/document-romania-la-15-ani-de-la-aderarea-la-ue-a-primit-cu-aproape-45-miliarde-euro-mai-mult-decat-a-contribuit-la-bugetul-ue-20559264): 2021 gross receipts and contribution (secondary, medium).
- `python/pipeline/fixtures/io_table2021_ro.json` (FIGARO 2021): product taxes by use, other taxes less subsidies by industry, household consumption by product.
