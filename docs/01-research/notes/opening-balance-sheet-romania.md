---
id: research/opening-balance-sheet-romania
title: Romania's sector balance sheets at the end of 2021, for the opening ledger
status: draft
owner: horia
depends_on: [economy/accounting]
updated: 2026-10-13
---

# Romania's sector balance sheets at the end of 2021, for the opening ledger

## Question
The financial accounts give, for each sector and each of nine statistical instruments, what it holds and what it owes on 31 December 2021 (fixture `balance_sheets2021q4_ro.json`). The ledger needs more than that. It needs to know who holds whose claim, because a deposit at a bank, a deposit at the central bank and a deposit abroad are three different ledger instruments. It also needs government debt by maturity and the central bank's reserves by kind. What do the published figures say, and where do we have to guess?

## Short answer
- **Deposits close by bookkeeping alone, with one assumption.** If government deposits issued by the government (47.3 bn lei) are held inside government, the rest of the government's deposits (41.7 bn) fit at the central bank. The central bank's remaining deposit liabilities (46.7 bn) are then the banks' reserves. The non-banks' deposits leave 10.1 bn of bank deposits held by banks, and the banks' remaining deposits (49.8 bn), added to the central bank's 88.7 bn, equal what the rest of the world owes in deposits (138.5 bn) to the leu. Confidence: medium. The arithmetic is exact; the split of government deposits is the assumption.
- **The central bank's reserves:** foreign currency EUR 40,475 million and gold EUR 5,356 million (103.6 tonnes) on 31 December 2021 (high). At 4.9481 lei per euro, gold is 26.5 bn lei. The statistics' 43.2 bn of gold and SDR then leaves 16.7 bn of SDR holdings, close to the SDR allocation of August 2021 (16.6 bn, the central bank's SDR liability; high).
- **Government debt:** 576.6 bn lei (48.9% of GDP) under EU methodology, of which 547.2 bn medium and long term. Short-term debt is therefore 29.4 bn (medium-high). By currency (end of May 2021): 47% lei, 45% euro, 8% dollar (medium). The split above one year is not in the sources read; a guess with a mean residual maturity of about seven years (low).
- **The rest of the world** holds 241.9 bn of Romanian debt securities. Almost all are government bonds: Eurobonds and lei bonds held by non-residents. Firms and banks issued only 11.7 bn in all (medium).
- **Households do not borrow abroad.** Their 187.3 bn of loans are from resident banks and non-bank lenders (high: the statistics of the central bank on household credit).

## Findings
**Deposits** (fixture, bn lei; holders' assets / issuers' liabilities):

| | Holds | Owes |
|---|---|---|
| Households | 294.2 | 0 |
| Firms | 188.9 | 0 |
| Banks | 106.5 | 552.9 |
| Other financial | 20.7 | 0 |
| Pension funds | 2.1 | 0 |
| Government | 88.9 | 47.3 |
| Central bank | 88.7 | 88.3 |
| Rest of world | 37.0 | 138.5 |

- The government "owes" deposits because the State Treasury keeps accounts for public institutions. In unconsolidated accounts these appear on both sides of the government sector (medium). Netting them leaves 41.7 bn, which matches the order of magnitude of the Treasury's buffer at the central bank (several billion euro, kept for about four months of financing; medium).
- The central bank's deposit liabilities are the Treasury's account and the banks' current accounts (minimum reserves). 46.7 bn of reserves is above the minimum reserves alone (8% on lei liabilities and 5% on foreign currency liabilities in 2021, about 30 to 35 bn; low), so some of it would be excess reserves. Plausible in 2021 (low).
- The central bank's 88.7 bn of deposits are deposits abroad (part of the foreign reserves; high).

**Debt securities:** the central bank holds 113.2 bn. With its 88.7 bn of deposits abroad that makes 201.9 bn, against official foreign currency reserves of 200.3 bn (EUR 40,475 million). The 1.6 bn difference is valuation and other reserve assets (medium). The central bank's holdings are therefore foreign securities. The rest of the world issued 134.1 bn, so 20.9 bn of foreign securities are held by other residents: banks, pension funds and insurers (medium).

**Statistical gaps:** each instrument's assets and liabilities differ by at most 0.2 million lei (rounding of the published millions). They are closed on the rest of the world.

**What the ledger leaves out:** derivatives (1.8 bn on each side) and other accounts receivable and payable (trade credit, advances, taxes due; 1,146 bn on each side, 871 bn of it owed by firms). The accounting spec has no row for them.

## Numbers worth keeping
| Quantity | Value | Source | Confidence |
|---|---|---|---|
| BNR foreign currency reserves, 31 Dec 2021 | EUR 40,475 m | BNR press release | high |
| BNR gold, 31 Dec 2021 | 103.6 t, EUR 5,356 m | BNR press release | high |
| BNR reference rate, 31 Dec 2021 | 4.9481 lei per euro | BNR | high |
| Government debt (EU methodology), end 2021 | 576.6 bn lei, 48.9% of GDP | Ministry of Finance | medium-high |
| of which medium and long term | 547.22 bn lei | Ministry of Finance | medium-high |
| Debt by currency, end of May 2021 | lei 249.2, EUR 235.9, USD 40.8 bn lei | Ministry of Finance | medium |
| Mean residual maturity of government debt | about 7 years | guess | low |
| Minimum reserve ratios, 2021 | 8% lei, 5% foreign currency | BNR (from memory) | low |

## Implications for the game
- [Opening balance sheet](../../02-design/economy/opening-balance-sheet.md) states the rules that turn these figures into the ledger's opening balances.
- Bank reserves above the minimum mean the banks of the starting state are liquid; the money-banking mechanic should not start with a reserve shortage.
- Most foreign holdings of Romanian debt are government bonds, so foreign interest rates (a world setting, roadmap D8) reach the budget mainly through the cost of new government borrowing.

## Sources
- [BNR: International reserves, December 2021 (ACTMedia)](https://actmedia.eu/financial-and-banking/bnr-international-reserves-december-2021/95714): foreign currency reserves and gold at the end of 2021.
- [Romania's public debt-to-GDP ratio rises to 48.9% at end-2021 (Romania Insider)](https://www.romania-insider.com/romania-public-debt-gdp-end-2021): government debt at the end of 2021.
- [Government debt amounts to 576.5 billion lei at end-December 2021 (ACTMedia)](https://actmedia.eu/daily/government-debt-amounts-to-576.5-billion-lei-at-end-december-2021-represents-48.9-of-gdp/96761): medium and long-term debt.
- [Romania's public debt reached 49.7% of GDP (Economedia)](https://economedia.ro/romanias-public-debt-reached-49-7-of-gdp-the-ministry-of-finance-announces.html): debt by currency, May 2021.
- [Eurostat nasq_10_f_bs](https://ec.europa.eu/eurostat/databrowser/view/nasq_10_f_bs/default/table): financial balance sheets by sector, the source of `balance_sheets2021q4_ro.json`.
