---
id: economy/investment-capital
title: Investment & Capital
status: locked
owner: horia
depends_on: [economy/production, economy/money-banking, economy/taxation, economy/trade-fx, economy/fiscal-policy, economy/budget-2021, economy/households, economy/opening-balance-sheet, economy/industries, game/world-loop, adr/0016-firm-representation, adr/0017-time-base]
research: [research/investment-dividends-romania-2021]
updated: 2026-10-20
---

# Investment & Capital

## Purpose
This spec covers three things:
- **What firms do with their profits:** pay them to their owners or keep them.
- **How much new capacity firms build.**
- **How capital wears out.**

Investment is demand now (machinery, construction, electronics) and supply later. Here interest rates, profits and subsidies turn into growth. Dividends are how profits reach households, the state and foreign owners.

## Real-world basis
- Investment responds to:
  - expected demand (the accelerator);
  - profitability against the cost of capital (Tobin's q);
  - uncertainty;
  - access to credit.

  It is the most volatile part of GDP. It needs time to build: 2 to 4 quarters (Kydland and Prescott 1982). Its response to the user cost of capital is −0.25 to −1.0 (medium confidence).
- Firms pay part of their profit to their owners and keep the rest. Foreign owners repatriate dividends, which are a primary-income outflow, and reinvest the rest.
- **Romania 2021** ([research note](../../01-research/notes/investment-dividends-romania-2021.md), Eurostat, high confidence unless marked):

| Item | Billion lei | Share or note |
|---|---|---|
| Gross fixed capital formation | 294.7 | 25.0% of GDP |
| … by corporations | 143.5 | |
| … by households | 95.8 | |
| … by the government | 48.0 | budget |
| Consumption of fixed capital, corporations and households | 163.4 | 6.6% of the firm units' 2,491.6 billion of net fixed assets |
| Dividends paid by non-financial corporations | 34.1 | |
| … of which received by the rest of the world | 18.6 | plus 22.7 of reinvested earnings |
| … of which received by the government | 2.9 | |
| … of which received by households | 0.1 | their owners' income is mixed income, 160.0 |
| Corporations' gross saving | 248.7 | net lending about 7% of GDP |

## State variables
Per **firm unit** $f$ ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)). A unit's amounts are totals for all the firms it stands for, as everywhere in the `World`.

| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $K_f$ | `capital_stock[f]` | capital in place, at starting prices | real bani |
| $K^{wip}_{f,k}$ | `capital_in_progress[f]` | investment bought $k$ months ago, not yet in use; one slot per month of gestation | real bani |
| $\kappa_f$ | `capital_output_ratio[f]` | capital per unit of monthly capacity, fixed at load | ratio |
| $g^e_f$ | `expected_growth[f]` | the growth the unit invests for | % / year |
| $\delta$ | `depreciation_rate` | per year (one value in M3; per industry later) | % / year |
| $p^I$ | `investment_price_index` | price of the investment column's goods against the start | index |
| $I_f$ | `investment[f]` | the month's investment budget, then what it spent | bani / month |
| $\Pi_f$ | `profit_after_tax[f]` | the month's profit after depreciation, interest, mixed income and accrued tax | bani / month |
| $Div_f$ | `dividends[f]` | paid to its owners this month | bani / month |
| $m^0_f$ | `cash_target_ratio[f]` | its starting deposits over its starting monthly costs | ratio |
| — | `hh_equity_key[h]` | a household's share of the households' equity, fixed at load | weight |

## Update rule (full design)
**Desired investment:**
```math
I^*_f = \Big(\delta + g^e_f + \beta_{cu}(cu_f - cu^*) + \beta_q (q_f - 1)\Big)\, K_f, \qquad
q_f = \frac{\text{expected profit rate}_f\,(1-\tau^{corp}) + \text{invest subsidy}_f}{r^L_f + \text{risk premium}_f - \pi^e}
```
**Financing:** retained earnings first, then bank loans (subject to credit supply, tighter for micro and small cohorts), then new equity (FDI if foreign-owned). Credit rationing caps $I_f$.

**Composition:** each industry buys its own mix of capital goods (buildings, machinery, computers, vehicles), at home or imported.

**Entry:** firms entering a cohort bring their start-up capital as investment.

**FDI:** foreign-controlled units invest by the rules of [foreign-ownership](foreign-ownership.md). The risk premium in $q$ includes the institutions index of [state capacity](state-capacity.md).

**Distribution:** profit after tax is paid pro rata to `ownership[f]`: the state share to the government, the foreign share to the rest of the world, the domestic private share to households and pension funds (ADR-0016). The payout depends on the controller.

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| The month's investment budget | staggered, on the unit's plan day (phase 2) |
| Investment purchases, private and government | daily (phase 6, in place of M2's fixed investment) |
| Profit, dividends and retained earnings | month end, after pay, the banks' interest and repayments, the treasury and the quarter's taxes; before the monthly close |
| Depreciation, completed projects, capacity, expected growth, productivity | monthly close |

## First version (roadmap M3)
This is what is built first, on the `World` of [game/world-loop](../game/world-loop.md), with [money and banking](money-banking.md), [taxation](taxation.md) and [trade](trade-fx.md). It replaces four M2 placeholders:
- investment paid out of surplus at fixed real levels;
- the change in inventories counted as capital formation;
- capital and capacity held constant;
- profits staying in the units.

**Starting state.**
- **Capital.** $K_f$ is the unit's `fixed_assets` in `firm_balance_sheets2021_ro.json` (2,491.6 billion lei in all). Units with no row in that file (banks, insurers, the public producers) have none.
- **Capital per unit of capacity.** $\kappa_f = K_f / \text{capacity}_f$, with capacity from [production](production.md) (starting output ÷ `starting_utilisation`).
- **Starting investment.** The firms' starting investment $I^0$ is a year of the table's investment column plus its inventories column, less the government's investment line of [budget-2021](budget-2021.md):

  $$I^0 = 200.8 + 73.4 + 22.8 - 48.0 = 249.0 \text{ billion lei a year.}$$

  It is shared over the units with capital in proportion to $K_f$: $I^0_f = I^0 K_f / \sum K$.
- **Expected growth.** $g^e_f = I^0_f/K_f - \delta$, about 3.5% a year, so that the first plan reproduces M2's demand.
- **Projects in progress.** Each of the `gestation_months` slots of $K^{wip}_f$ holds a month of $I^0_f$.
- **Starting price and rate.** $p^I = 1$. The starting real loan rate $\rho_0$ is the firms' loan rate less the inflation target.
- **Cash target.** $m^0_f$ is the unit's starting deposits over its starting monthly costs (inputs and labour).
- **Households' equity key.** `hh_equity_key[h]` is each household's `hh_cash` + `hh_deposits` at load.
- **Pension funds' share.** The share of domestic dividends that goes to pension funds, $\phi_{PF}$, is their share of the households' and pension funds' equity in the opening balance sheet: $25.9 / (263.8 + 25.9) = 8.9\%$.

**In:**
1. **The investment budget, on the plan day.** Each unit with capital sets the next month's budget:
   $$I^*_f = \frac{p^I K_f}{12}\,\max\!\Big(0,\ \delta + g^e_f + \beta_{cu}\,(cu_f - cu^*) - \beta_r\,(\rho - \rho_0)\Big)$$
   - $cu_f$ is its mean utilisation over the last three monthly closes.
   - $cu^*$ is production's `starting_utilisation`.
   - $\rho$ is the firms' loan rate less expected inflation, at the last close.

   The budget is what it can finance: $I_f = \min\big(I^*_f,\ \text{spare}_f + \text{credit room}_f\big)$.
   - Spare money is deposits beyond a month of wages (M2's rule).
   - Credit room is `econ-mech-banking::credit_room`.
2. **Purchases, daily.**
   - **What a unit buys.** Each unit spends $I_f$ ÷ the days of the month. Its spending is split over goods by the investment column's starting shares.
   - **Domestic part.** It is demand in the goods market, rationed like other final demand. Payments between firm units move only the units' columns, as input purchases do: no ledger posting.
   - **Imported part.** It is paid to the rest of the world at the day's import price (`imports`).
   - **Demand that is not served** is lost, not carried over.
   - **Running short.** A unit that runs short at the end of the day borrows on its credit line (money-banking step 1, `finance.loan`). At its credit limit, investment is the first payment it stops, before inputs and wages.
3. **Government investment, daily.**
   - **What is paid.** The budget's `investment` and `non_produced_assets` lines (48.0 billion lei a year) are paid at fixed real levels, with the column's shares. The domestic part goes from the government to firms under `gov.investment`; the imported part is paid under `imports`. It is the part of M2's placeholder that firm units used to pay.
   - **Public capital.** No public capital stock is kept until infrastructure (M11).
4. **Profit, at month end.** For each market unit:
   $$\Pi^{pre}_f = V_f - X_f - W_f - \tfrac{\delta}{12}\,p^I K_f - (\text{interest paid} - \text{interest received})_f - MI_f$$
   - $V$ is the value of the month's output.
   - $X$ is its input purchases, home and imported.
   - $W$ is its wages and employer contributions.
   - $MI$ is the self-employment and informal income it paid this month (`income.mixed`, the households' M2 placeholder).

   $\Pi_f = \Pi^{pre}_f - \hat\tau_f$. The accrued tax $\hat\tau_f$ is:
   - `profit_tax_rate` × $\max(0, \Pi^{pre}_f)$ for a profit-tax unit;
   - the micro rate × the month's declared sales for a micro unit ([taxation](taxation.md), steps 8 and 9).

   The tax itself is still paid at the quarter's end, and any difference between accrued and paid stays in retained earnings.

   The month's depreciation $\tfrac{\delta}{12} p^I K_f$ is also the depreciation that taxation's step 8 deducts from taxable profit.
5. **Dividends, on the 15th of the next month** (the month's profit of step 4; paid mid-month so that a pay day's income stays what the pay slips show, AC-ST-05):
   $$Div_f = \min\Big(\text{spare}_f,\ \theta_{c(f)} \max(0, \Pi_f) + \sigma \max\big(0,\ M_f - m^0_f\, C_f\big)\Big)$$
   - $\theta_c$ is the payout of the unit's controller (`payout_domestic`, `payout_foreign`, `payout_state`).
   - $M_f$ is its deposits.
   - $C_f$ is the month's inputs and labour cost.
   - $\sigma$ pays out a share of the money above the unit's starting liquidity.

   What is not paid is retained. Public (non-market) producers pay none.

   **Split.** $Div_f$ is split by `ownership[f]` (largest remainder):
   - the **state** share goes to the government (deposits at the central bank, the three legs of the `World`);
   - the **foreign** share goes to the rest of the world's lei deposits; it counts as primary income paid abroad in trade's current account;
   - the **domestic private** share goes $\phi_{PF}$ to pension funds' deposits and the rest to households.

   All of it is under `income.dividend`.

   **Households.**
   - **How they are paid.** Households are paid through the dividend clearing account (ADR-0016), each by `hh_equity_key` × weight, in whole bani per record.
   - **The remainder.** The part that cannot be paid in whole bani stays with the units, split by their parts (as M2's mixed income).
   - **Dividend tax.** `dividend_tax_rate` (5%) of it goes to the government under `tax.dividend` (taxation step 10).
   - **Income.** The net amount adds to `hh_income_received`.
6. **Capital, at the monthly close.** For each unit:
   $$K_f' = K_f - \text{round}\big(K_f\,\delta/12\big) + K^{wip}_{f,\text{oldest}}$$
   The month's spending, divided by $p^I$, enters the newest slot of work in progress. Then:
   - **Capacity** becomes $A \cdot K_f / \kappa_f$.
   - **The price index.** $p^I$ is the starting-share mean of the column's domestic prices and import prices.
7. **Expected growth, at the monthly close.**
   $$g^e_f \leftarrow g^e_f + \lambda\,(\hat g_f - g^e_f)$$
   $\hat g_f$ is the growth of the unit's real sales over the last twelve months, bounded by `growth_bound`. Until twelve months exist, $\hat g_f = g^e_f$ of the start.
8. **Productivity, at the monthly close.**
   - Production's TFP index $A$ grows by `tfp_base_growth`/12.
   - Each unit's labour requirement per unit of output is divided by the same factor.

> **Simplification (M3):** one capital good mix for every industry (the table's investment column); one depreciation rate; no financial assets of firms other than deposits. Money beyond a unit's starting liquidity is paid out (the $\sigma$ term) instead of lent to other firms or invested abroad, as Romania's corporations did in 2021 (net lending about 7% of GDP). Households' dwellings (S14 investment, 95.8 billion lei) stay in the firms' starting investment until the housing market (M13). The tax accrual ignores losses carried forward.

**Not in M3:**
- Tobin's q with expected profits, and investment subsidies and allowances (M9).
- FDI decisions, relocation and transfer pricing (M17).
- Entry and exit of firms (M9).
- Capital-goods mixes by industry.
- SOE directives and share sales (M9).
- Public capital (M11).
- Capacity by region.

## Player levers
- **Through the rules above:** the policy rate (via the loan rate), the profit tax and micro regime, the dividend tax, the credit rules.
- **From M9:** investment subsidies and allowances, nationalisation and SOE directives (payout and strategic investment).
- **Other milestones:** infrastructure (M11), capital controls (outside the EU only).

In M3 there are none of its own: levers come with M4.

## Outputs
- Investment by unit and industry, private and public.
- Capital stock and capacity; consumption of fixed capital.
- Profit before and after tax, dividends by receiving sector, retained earnings.
- Primary income paid abroad; the investment share of GDP.

## Tuning parameters
| Parameter | Default | Effect | Confidence |
|---|---|---|---|
| `depreciation_rate` $\delta$ | 6.5% a year | consumption of fixed capital, 163 bn lei a year at the start | medium |
| `gestation_months` | 3 | months from purchase to capital in use | medium-low |
| $cu^*$ | production's `starting_utilisation` (0.8) | utilisation at which the accelerator is neutral | low |
| `beta_cu` $\beta_{cu}$ | 0.5 a year per unit of utilisation gap | accelerator | low |
| `beta_r` $\beta_r$ | 1.0 a year per unit of real rate | a 2-point rise cuts desired investment by 2% of capital a year, about a fifth | low-medium |
| `growth_adjustment` $\lambda$ | 1/24 a month | how fast expected growth follows realised growth | low |
| `growth_bound` | ±10% a year | bounds on realised growth | low |
| `payout_domestic` | 0.3 | share of profit after tax paid to domestic owners | low |
| `payout_foreign` | 0.45 | share paid to foreign owners | medium |
| `payout_state` | 0.5 | share paid by state-controlled units | high for the legal minimum |
| `cash_sweep` $\sigma$ | 0.1 a month | share of excess money paid out | low |
| `tfp_base_growth` | production's (1% a year) | productivity growth | low |

The households' equity key, $\phi_{PF}$ and the cash targets are derived at load. They are not tuned.

## Interactions
- **From [production](production.md):** output, utilisation, the value of output and sales. **To production:** capacity and the labour requirement.
- **From [money and banking](money-banking.md):** the firms' loan rate, credit room and interest. **To money and banking:** credit demand.
- **From [prices](prices-inflation.md):** prices of investment goods and expected inflation.
- **To [taxation](taxation.md):** depreciation in taxable profit; dividends for the dividend tax.
- **To [trade](trade-fx.md):** imported investment goods; dividends paid abroad in the current account.
- **To [households](households.md):** dividends as income.
- **To [fiscal policy](fiscal-policy.md):** government investment; dividends from state units.

## Edge cases & failure modes
- **A unit with no capital** (banks, insurers): it does not invest, keeps its capacity and pays dividends by the rule.
- **A loss-making unit** pays no profit-based dividend. It pays the excess-money part only if it has excess money.
- **At the credit limit:** investment stops first; work in progress still completes.
- **Capital falls to zero** (long disinvestment): capacity is zero and the unit cannot produce. The floor of $I^*$ at zero means this happens only through depreciation, over decades.
- **A sample scale where a household's share rounds to zero bani:** it gets nothing that month; the units keep the rounding remainder.

## Acceptance tests
IDs are stable (added 2026-10-20). Tests go in `crates/econ-core/tests/acceptance/` (protected), and in `crates/econ-mech-investment/tests/acceptance/` for `[unit]`.

- [ ] **AC-INV-01** `[unit]` **The investment budget follows step 1.**
  - With the starting inputs it equals $I^0_f/12$.
  - It is never negative.
  - It does not rise with the real rate, and does not fall with utilisation or expected growth.
- [ ] **AC-INV-02** `[unit]` **Capital follows step 6.** Each month $K' = K - \text{round}(K\delta/12) + $ the oldest slot. A purchase enters capital exactly `gestation_months` later. With no purchases capital falls by $\delta/12$ a month.
- [ ] **AC-INV-03** `[unit]` **Dividends follow step 5.** The dividend is the formula, never negative, never above spare money. The ownership split sums to the dividend to the ban, each part within one ban of its share.
- [ ] **AC-INV-04** `[sim]` **The month's dividends add up**, every month end on the Romania scenario with the 2021 law:
  - The `income.dividend` postings to households, pension funds, government and the rest of the world sum to the units' dividends, less the households' rounding remainder.
  - Each sector receives its ownership share.
  - Public producers pay none.
  - The dividend clearing account nets to zero (I-8).
  - `tax.dividend` is 5% of the households' gross dividends, to the ban.
- [ ] **AC-INV-05** `[sim]` **Foreign units' dividends leave the country.** They reach the rest of the world's deposits, appear as primary income paid abroad in the month's current account, and the balance of payments still closes (AC-TFX-03).
- [ ] **AC-INV-06** `[sim]` **Capital and capacity.** At every monthly close, each unit's capital follows AC-INV-02 with its month's real spending, and its capacity is $A K/\kappa$. The month's private and government investment are the investment of GDP by expenditure, and I-7 holds.
- [ ] **AC-INV-07** `[sim]` **Financing.** No unit's monthly budget exceeds its spare money plus credit room on its plan day. Spending beyond its deposits is a `finance.loan` draw, and AC-BANK-04 holds.
- [ ] **AC-INV-08** `[sim]` **The policy rate matters.** Compare two worlds, one with the policy rate 2 points higher from month 3 (`StartEdit::PolicyRate`). Firms' investment is lower in every month from month 6 to month 24. The fall is larger, relative to the baseline, for the third of units with the most capital per unit of output than for the third with the least.
- [ ] **AC-INV-09** `[sim]` **Quiet baseline.** Run the Romania scenario at 1:100 with the 2021 law and no command, over 120 months. At every annual close:
  - firm units' deposits over the last twelve months' nominal GDP are within 30% of the first year's ratio;
  - gross investment is between 15% and 30% of GDP;
  - total capital stays between 0.9 and 1.6 times its start.
- [ ] **AC-INV-10** `[sim]` **The first year is reported** next to the research note's targets, on the Romania scenario with the 2021 law: consumption of fixed capital (163 bn lei), private investment (249), dividends paid abroad (18.6), dividends to residents, profit tax (26.3), household consumption (720). Within: consumption of fixed capital ±15%, private investment ±25%, dividends paid abroad ±50%, profit tax ±50%, household consumption ±10% (set under the owner's delegation, 2026-10-20).
- [ ] **AC-INV-11** `[sim]` *(M9)* An investment subsidy for `data_centres` raises their capital stock after the gestation lag.
- [ ] **AC-INV-12** `[sim]` *(M9)* Firms entering a cohort bring their start-up capital as investment.
- [ ] **AC-INV-13** `[sim]` *(M17)* Expropriation in any industry lowers foreign investment in all industries.

### Tests owed
Locked 2026-10-20 under the owner's delegation, before its tests (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-INV-01: the test session of M3's investment
- AC-INV-02: the test session of M3's investment
- AC-INV-03: the test session of M3's investment
- AC-INV-04: the test session of M3's investment
- AC-INV-05: the test session of M3's investment (with trade on the `World`)
- AC-INV-06: the test session of M3's investment
- AC-INV-07: the test session of M3's investment
- AC-INV-08: the test session of M3's investment
- AC-INV-09: the test session of M3's investment
- AC-INV-10: the test session of M3's investment
- AC-INV-11: roadmap M9
- AC-INV-12: roadmap M9
- AC-INV-13: roadmap M17

With step 8 in place, production's AC-PROD-10 (TFP growth gives long-run growth) and taxation's AC-TAX-09 (corporate tax after depreciation) can be tested.

## API sketch
For the test writer and the implementer. Money is in bani and real capital in real bani; rates and shares are in millionths (a year for rates).

```rust
// crate econ-mech-investment (no I/O)
pub struct InvestParams { pub depreciation_ppm: u32, pub gestation_months: u8, pub cu_target_ppm: u32,
                          pub beta_cu_ppm: u32, pub beta_r_ppm: u32, pub growth_adjust_ppm: u32,
                          pub growth_bound_ppm: u32, pub payout_domestic_ppm: u32, pub payout_foreign_ppm: u32,
                          pub payout_state_ppm: u32, pub cash_sweep_ppm: u32 }
impl InvestParams { pub fn spec_defaults() -> InvestParams; }
pub fn starting_growth_ppm(start_investment_year: Bani, capital: i64, p: &InvestParams) -> i32;
pub fn desired_investment(capital: i64, price_index_ppm: u32, utilisation_ppm: u32, real_rate_gap_ppm: i32,
                          expected_growth_ppm: i32, p: &InvestParams) -> Bani;                          // AC-INV-01, a month
pub fn financed(desired: Bani, spare: Bani, credit_room: Bani) -> Bani;
pub fn next_expected_growth_ppm(current_ppm: i32, realised_ppm: i32, p: &InvestParams) -> i32;
pub struct CapitalBook { pub capital: i64, pub in_progress: Vec<i64> /* oldest first, gestation_months long */ }
impl CapitalBook {
    pub fn new(capital: i64, start_month_investment: i64, p: &InvestParams) -> CapitalBook;
    pub fn depreciation(&self, p: &InvestParams) -> i64;                                            // round(K δ / 12)
    pub fn close_month(&mut self, spent_real: i64, p: &InvestParams) -> (i64 /* depreciated */, i64 /* completed */); // AC-INV-02
}
pub enum Controller { Domestic, Foreign, State }
pub fn dividend(profit_after_tax: Bani, deposits: Bani, cash_target: Bani, spare: Bani,
                controller: Controller, p: &InvestParams) -> Bani;                                  // AC-INV-03
pub fn split_by_ownership(amount: Bani, state_ppm: u32, domestic_ppm: u32, foreign_ppm: u32) -> [Bani; 3]; // largest remainder
```

Profit, the tax accrual, purchases, postings and the monthly close live with the `World`. New flow codes, proposed, append-only:
- `income.dividend` (2010), whoever receives it. 2004 to 2009 are left for money and banking's interest codes.
- Already proposed elsewhere: `gov.investment` (1003, [budget-2021](budget-2021.md)) and `tax.dividend` (3009, [taxation](taxation.md)).
- Investment purchases between firm units post nothing. Their imported part is `imports`, and private investment is counted in the month's statistics.
- Depreciation is not a payment and has no code.

## New glossary entries
To add to the glossary with `just codegen` when the spec is accepted:
- `capital_in_progress[f]`
- `capital_output_ratio[f]`
- `expected_growth[f]`
- `depreciation_rate`
- `investment_price_index`
- `investment[f]`
- `profit_after_tax[f]`
- `dividends[f]`
- `cash_target_ratio[f]`
- `hh_equity_key[h]`
- `payout_domestic`, `payout_foreign`, `payout_state`
- `cash_sweep`
- `gestation_months`
- `beta_cu`, `beta_r`
- `growth_adjustment`, `growth_bound`
- `consumption_of_fixed_capital` (derived: $\delta p^I K$ summed)

## Open questions
- [x] **Monthly dividends with an accrued tax, or quarterly after the tax is paid?** Proposed: monthly. Households decide on last month's income, so quarterly payouts would make consumption jump every quarter. The accrual's error stays in retained earnings. **Decided (owner's delegation, 2026-10-20):** monthly, the month's profit paid on the 15th of the next month (step 5), so that a pay day's income is what the pay slips show (AC-ST-05).
- [x] **The excess-money payout ($\sigma$)** departs from the data: Romania's corporations kept most of their profit in 2021 (payout about 20%). They lent it on as trade credit and loans and held it abroad, which the ledger does not have. Proposed: keep $\sigma$ in M3, so that firm deposits stay bounded (AC-INV-09). Revisit when firms hold other financial assets. **Decided (owner's delegation, 2026-10-20):** as proposed.
- [x] **Which households receive dividends?** Proposed: by `hh_cash` + `hh_deposits` at load, fixed. Equity is as concentrated as deposits, and stage H has no equity column (population-wealth leaves stakes for later). Alternative: households with an employer member, by their self-employment income, since 227.5 of their 256.3 billion of shares are stakes in their own firms. The national accounts give households only 0.1 billion of dividends, so neither rule can be calibrated on D42. **Decided (owner's delegation, 2026-10-20):** as proposed.
- [x] **Government investment moves from firm units to the government** (48.0 billion lei a year under `gov.investment`). Proposed: yes; demand is unchanged. It raises the deficit and firms' money by the same amount, and changes fiscal policy's first-year figures (AC-FIS-05). **Decided (owner's delegation, 2026-10-20):** as proposed.
- [x] **The inventories column** (22.8 billion) is folded into firms' starting investment, so the start reproduces M2's demand. Inventories themselves change by production's rules. Alternative: drop it, and the start loses 2% of GDP of demand. Proposed: fold it in. **Decided (owner's delegation, 2026-10-20):** as proposed.
- [x] **Mixed income in the profit-tax base.** The `World` taxes the units' whole surplus, including what they pay as self-employment and informal income. Proposed: taxation's taxable profit also deducts `income.mixed` paid, as this spec's profit does. This needs a change to taxation, the owner's call. **Decided (owner's delegation, 2026-10-20):** as proposed; recorded in [taxation](taxation.md).
- [x] **Productivity growth (step 8)** with no source of demand growth may raise unemployment over a long quiet run. Proposed: build it at 1% a year and run AC-INV-09 at 1% and at 0%; if it fails at 1%, the default is 0 until calibration (M7). **Decided (owner's delegation, 2026-10-20):** as proposed.
- [x] **Excess money on gross deposits or net of loans?** Proposed: gross. Indebted units repay through money and banking's repayment rule. **Decided (owner's delegation, 2026-10-20):** as proposed.
- [x] **Pension funds keep their dividends** (8.9% of the domestic share) as deposits. With the treasury, these fund their share of new bonds. Proposed: yes. **Decided (owner's delegation, 2026-10-20):** as proposed.
- [x] Housing investment: by households through `construction_buildings` (M13), or a developer sector? Proposed: by households, with the housing market. **Decided (owner's delegation, 2026-10-20):** as proposed.
- [x] Do SME cohorts face their own credit-rationing rule, or only a higher spread? Proposed: only a higher spread, with bank regulation (M10). **Decided (owner's delegation, 2026-10-20):** as proposed.
