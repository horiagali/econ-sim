---
id: research/household-wealth-romania
title: Household wealth in Romania, end of 2021 — what data exists
status: draft
owner: horia
depends_on: [research/overview]
updated: 2026-10-10
---

# Household wealth in Romania, end of 2021 — what data exists

## Question
What can the wealth stage of the population generator (deposits, debts, dwelling values for every household) be built from?

## Short answer
- **Totals exist and are good.** Eurostat's financial accounts give the households' balance sheet at the end of 2021 instrument by instrument, and the capital stock accounts give the value of their dwellings.
- **No distribution exists for Romania.** Romania does not take part in the euro-area household finance survey, and Eurostat's experimental statistics on income, consumption and wealth have Romania for income and consumption only. Who holds the wealth has to be assumed.
- **Neighbours have it.** The same experimental statistics give the shares of net wealth by wealth quintile and by income quintile for Hungary, Croatia, Poland, Slovakia, Latvia and Lithuania. Borrowing the proportions of a similar country, as the jobs stage borrows EU patterns, is the best available option. Hungary 2020 is proposed.
- **Useful by-product:** the same statistics give Romania's household income and consumption by income quintile for 2020, which the model's households should reproduce once all incomes exist.

## Findings

### 1. The households' balance sheet (confidence: high)
Eurostat `nasq_10_f_bs`, Romania, households and non-profit institutions serving them, end of the fourth quarter of 2021:

| | Billion lei |
|---|---|
| Currency | 57.3 |
| Deposits (transferable 183.0, other 111.2) | 294.2 |
| Debt securities (mostly government bonds) | 13.8 |
| Listed shares | 28.8 |
| Unlisted shares and other equity (stakes in own and family firms) | 227.5 |
| Investment fund units | 26.1 |
| Life insurance | 11.2 |
| Pension entitlements (the mandatory private pension funds) | 92.1 |
| Other accounts receivable | 201.8 |
| **Financial assets** | **967.8** |
| Loans (short-term 8.0, long-term 179.3) | 187.3 |
| Other accounts payable | 68.2 |
| **Liabilities** | **255.5** |
| **Financial net worth** | **712.3** |

Eurostat `nama_10_nfa_bs`, 2021: dwellings owned by households, net of depreciation, at replacement cost: **428.6 billion lei**. This is the buildings without the land under them; the market value of homes is higher. No Eurostat table has land by sector for Romania.

The split of loans into housing loans and consumer loans, and the number of borrowers, are published by the central bank, not through Eurostat; not fetched.

### 2. The distribution (confidence: none for Romania; medium for the neighbours)
Eurostat experimental statistics `icw_res_01` (joint distribution of income, consumption and wealth). Share of net wealth, in percent:

| Country, year | By wealth quintile, poorest to richest | Top tenth | By income quintile, poorest to richest | Top income tenth |
|---|---|---|---|---|
| Hungary 2020 | 0.8, 4.3, 8.8, 15.7, 70.6 | 55.6 | 10.4, 11.9, 15.2, 20.5, 42.0 | 29.7 |
| Croatia 2020 | 0.2, 4.7, 9.6, 14.7, 70.8 | 60.5 | 12.2, 17.9, 17.1, 23.0, 29.9 | 15.7 |
| Latvia 2020 | 0.2, 4.9, 8.4, 16.8, 69.7 | 52.9 | 7.5, 13.3, 16.0, 21.7, 41.5 | 28.7 |
| Lithuania 2020 | 1.4, 7.6, 13.3, 19.6, 58.1 | 42.4 | 11.2, 14.8, 18.2, 24.0, 31.8 | 20.0 |
| Slovakia 2020 | 2.6, 9.0, 13.5, 20.1, 54.7 | 41.0 | 13.1, 16.8, 18.5, 20.3, 31.3 | 17.8 |
| Poland 2015 | 0.3, 5.8, 11.5, 21.8, 60.6 | 43.0 | 12.6, 15.1, 18.8, 20.9, 32.6 | 19.4 |

Two things hold in every one of them: the richest fifth by wealth owns 55% to 70% of it, and wealth is much less tied to income than one would guess (the poorest fifth by income still owns about a tenth), because most of it is the family home, owned outright by old and poor households too. Romania has the highest home-ownership rate in the EU (about 95%; the census gives 90% owners among households), so the second point should hold at least as strongly.

Net wealth here includes homes. No table separates deposits from homes in this source.

### 3. Romania's income and consumption by income quintile (confidence: medium; experimental)
`icw_res_02`, 2020, euro per household per year: mean disposable income 9,173 (median 7,324); by income quintile 2,483, 5,438, 7,764, 11,156, 18,998; top tenth 22,256. Mean consumption 8,072 (median 7,164).

## Numbers worth keeping

| Quantity | Value | Source | Confidence |
|---|---|---|---|
| Household currency and deposits, end 2021 | 351.5 billion lei | `nasq_10_f_bs` | high |
| Household loans, end 2021 | 187.3 billion lei | `nasq_10_f_bs` | high |
| Household pension fund claims, end 2021 | 92.1 billion lei | `nasq_10_f_bs` | high |
| Household equity and fund units, end 2021 | 282.4 billion lei | `nasq_10_f_bs` | high |
| Dwellings of households, net replacement value, 2021 | 428.6 billion lei | `nama_10_nfa_bs` | high (for what it measures) |
| Share of net wealth held by the richest fifth, Hungary 2020 | 70.6% | `icw_res_01` | medium |
| Share of net wealth held by the top income fifth, Hungary 2020 | 42.0% | `icw_res_01` | medium |
| Mean household disposable income, Romania 2020 | 9,173 euro a year | `icw_res_02` | medium |

## Implications for the game
For the wealth stage of the population generator (roadmap M1) and for [accounting](../../02-design/economy/accounting.md):
1. **Totals per instrument come from the table in section 1.** They are also the household column of the opening balance sheet, so the stage and the balance sheets of the other sectors must use the same figures.
2. **The distribution is borrowed.** Proposed: Hungary 2020. Households are ranked by a mix of their income (wages and pensions exist; the other incomes come with the next stage) and a keyed draw, with the mix set so that the shares by income quintile come out as the pattern's; each household's place on a wealth curve drawn through the pattern's shares by wealth quintile gives its net wealth. This is the device of the wage stage with one more step.
3. **Homes first, money second.** An owner household's wealth is mostly its dwelling, which the tenure column of Stage E already fixes. Deposits and securities are far more concentrated than homes. Without a source that separates them, the stage needs one more assumption: how much more concentrated. Deposit-guarantee statistics (depositors by size of deposit) would replace it; not fetched.
4. **Debt needs the central bank's figures:** housing against consumer loans, number of borrowers, typical loan sizes. With them, mortgages go to owner households of working age in cities, consumer loans widely.
5. **Dwelling values need prices:** the replacement value above is not what a home sells for. Price per square metre by county and locality size, with dwelling sizes, gives market values; the capital stock figure is a floor for the total.
6. **A check for later:** once every income exists, households' disposable income by quintile should be near section 3.

Still to collect before the stage can be specified with tolerances: loans by type and borrower counts (central bank), depositors by size (deposit guarantee fund), house prices by county.

## Sources
- Eurostat tables `nasq_10_f_bs`, `nama_10_nfa_bs`, `icw_res_01`, `icw_res_02`, read through the dissemination API on 2026-10-10.
