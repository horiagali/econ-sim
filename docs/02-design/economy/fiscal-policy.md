---
id: economy/fiscal-policy
title: Fiscal Policy — Budget, Spending, Subsidies & Public Debt
status: locked
owner: horia
depends_on: [economy/accounting, economy/taxation, economy/social-transfers, economy/money-banking, economy/monetary-policy, economy/budget-2021, economy/opening-balance-sheet, game/world-loop, adr/0017-time-base]
research: [research/budget-romania-2021]
updated: 2026-10-20
---

# Fiscal Policy — Budget, Spending, Subsidies & Public Debt

## Purpose
The government budget: what the state buys, whom it employs, what it subsidises, how it covers the deficit, and how debt and bond yields evolve. Deficit spending is demand now and debt later.

## Real-world basis
- Fiscal multipliers vary by instrument (investment and transfers to the poor > tax cuts for the rich), by slack in the economy (higher in recessions), by monetary response (lower if the CB offsets) and by openness (leakage via imports).
- Debt sustainability: $\Delta(b) \approx (r - g) \cdot b - \text{primary balance}$.
- Sovereign risk premia rise non-linearly with debt, deficits, inflation and weak institutions; foreign-currency debt is riskier.

## Budget lines (per tick)
**Revenue:** all taxes ([taxation](taxation.md)), SOE dividends ([state-enterprises](state-enterprises.md)), central bank profit, fees, privatisation receipts, ETS auction revenue ([environment](environment.md)), EU funds reimbursements ([eu-funds](eu-funds.md)).

**Spending:**
| Line | Goes to |
|---|---|
| Ministry budgets (education, health, police and justice, administration, research, culture, environment) | public wages (public employees are persons with `employer_type = public sector`) + purchases of goods by fixed mix per ministry |
| Infrastructure projects and maintenance | `construction_civil`, `construction_buildings` and inputs ([infrastructure](infrastructure.md)) |
| Social transfers | households ([social-transfers](social-transfers.md)) |
| Subsidies | firms or consumers (below) |
| Interest on debt | bond holders (households, banks, pension funds, CB, RoW) |
| SOE loss coverage | SOEs |
| EU budget contribution | EU ([eu-membership](eu-membership.md)) |
| EU-funds co-financing and pre-financing | projects ([eu-funds](eu-funds.md)) |

The starting values of these lines (2021: 474 billion lei, 40.2% of GDP), public posts and the public wage bill are scenario data: see [the starting budget](budget-2021.md).

Ministry spending affects quality indices: education quality, health capacity, policing, admin efficiency (tax enforcement), research (TFP). How much quality each leu buys depends on [state capacity](state-capacity.md); part of procurement can leak to corruption.

**EU fiscal rules:** deficit and debt are tracked against the EU reference values and the required adjustment path ([eu-membership](eu-membership.md)).

## Subsidies
| Type | Mechanic |
|---|---|
| Production subsidy | % of output value or per unit to industry *j* → unit cost ↓ → price ↓ and/or margin ↑ |
| Investment subsidy | % of capex → $q$ ↑ ([investment-capital](investment-capital.md)) |
| Wage subsidy | % of wages for hires (e.g. youth) → labour cost ↓ |
| Consumer subsidy | % off the price of good *g* for households (e.g. energy, public transport, bread) → demand ↑ |
| Export subsidy | see [trade-fx](trade-fx.md) |

## Financing and debt
- Deficit = spending − revenue. Financed by issuing bonds (by maturity: bills, 2y, 10y), drawing down deposits, or monetary financing (CB, see [monetary-policy](monetary-policy.md)).
- Buyers: banks, households (by portfolio rules), RoW (depends on yield vs world rate and risk), CB (QE).
- **Bond yield:**
```math
r^B = i + \text{term premium} + \text{risk premium}(\tfrac{B}{Y}, \text{deficit}, \pi, \text{cred}, \text{share foreign held})
```
Risk premium is convex: small below ~60–90% debt/GDP, steep above a soft threshold that depends on credibility.
- **Rollover:** maturing debt must be refinanced at current yields.
- **Debt crisis:** if auctions fail (demand < issuance at a yield cap), the player must cut spending, raise taxes, monetise or **default** (haircut on bond holders → bank losses, household wealth loss, RoW cuts lending, yields spike for years).

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| Purchases, public wages, transfers | as in the `World` (daily purchases, pay day) |
| The budget lines the `World` does not yet pay (subsidies, transfers to the EU and others, capital transfers) | month end, a twelfth of the year's line |
| Coupons on government debt | month end |
| Bond issues and redemptions | month end, after every other payment of the day |
| Yields | monthly close |

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $B$ | `gov_debt` | government bonds outstanding (the ledger's bonds row) | LCU |
| — | `debt_maturity[k]` | bonds outstanding by year of maturity, from `sector_details2021_ro.json` | LCU |
| $r^B$ | `bond_yield` | the yield of new issues | % / year |
| — | `debt_coupon` | the average coupon of the stock | % / year |
| — | `gov_cash_target` | the deposits at the central bank the treasury keeps | LCU |

## First version (roadmap M3)
What is built first, on the `World` of [game/world-loop](../game/world-loop.md), with the taxes and transfers of the 2021 law ([taxation](taxation.md), [social-transfers](social-transfers.md)).

**Starting state.** Debt is the ledger's government bonds (478.6 billion lei in the opening balance sheet), spread over years of maturity in the proportions of the debt profile (`sector_details2021_ro.json`: up to 1 year 29.4 billion, 1–3 years 120.0, 3–5 years 110.0, 5–10 years 177.0, over 10 years 140.2, of a 576.6 billion total; each bucket spread evenly over its years). The starting coupon is `starting_coupon`. Holders are those of the ledger: banks, pension funds, other financial firms, households, firms and the rest of the world.

**In:**
1. **Spending** stays as the `World` pays it: purchases at fixed real levels, public units at cost, pensions and the other transfers. The budget lines it does not pay yet ([budget-2021](budget-2021.md): subsidies, current transfers to the EU and others, capital transfers, other taxes) are paid at month end, a twelfth of the year's amount each, under the budget's proposed flow codes.
2. **Coupons, month end.** The government pays a twelfth of `debt_coupon` on the stock to each holder in proportion to its holding (flow code `interest.gov_debt`): banks and the central bank in reserves, the rest in deposits (the three legs of the `World`).
3. **Financing, month end, last.** Maturing bonds are redeemed at par. Then the government issues new bonds, or redeems early, so that its deposits at the central bank return to `gov_cash_target` (a month of spending): issue = target − deposits after the day's payments. New bonds mature in `new_issue_years` and pay the yield of the day; the average coupon of the stock moves accordingly. Buyers take the issue in proportion to their current holdings and pay from their deposits (banks from reserves).
4. **The yield, monthly close:**

```math
r^B = i + \text{term} + \text{risk}, \qquad \text{risk} = \text{risk}_0 \cdot \max\big(0,\ \tfrac{B}{Y} - \text{threshold}\big)^2
```

   $Y$ is the last twelve months' GDP.
5. **Monetary financing** is what the central bank buys or lends to the government ([monetary-policy](monetary-policy.md)); with step 3 the government's deposits stay at their target, so there is none unless the player chooses it (M4).

> **Simplification (M3):** bonds at book value (no revaluation, no secondary market); one maturity for new issues; holders' shares fixed (no portfolio choice); no auction failure or default (crisis dynamics, D10); debt in lei only (the euro and dollar shares of the profile are reported, not modelled).

**Not in M3:** ministry budgets as levers and their quality effects (M4, M14); subsidies by type as levers (M4); EU fiscal rules (M18); default and debt crises (D10).

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `starting_coupon` | 4.5% a year | the stock's average coupon at the start (medium confidence; to be checked against the Ministry of Finance's debt bulletin) |
| `new_issue_years` | 7 | maturity of new bonds |
| `term_premium` | 1.5 points | 7-year yield over the policy rate |
| `risk_premium` $\text{risk}_0$ | 20 points per unit of debt-to-GDP squared | how steeply yields rise with debt |
| `debt_threshold` | 60% of GDP | where the risk premium starts |
| `gov_cash_target` | a month of spending | the treasury's buffer |

## Player levers
Ministry budgets, public wages and headcount, subsidies, borrowing strategy (maturity mix), fiscal rules (optional self-imposed debt brake), default decision.

## Outputs
Budget balance, primary balance, debt/GDP, interest burden, yields, holders of debt, multiplier estimates, distribution of spending and taxes by group ("fiscal incidence").

## Acceptance tests
IDs are stable (added 2026-10-20). Tests go in `crates/econ-core/tests/acceptance/` (protected).

- [ ] **AC-FIS-01** `[sim]` Debt identity, every month end: debt = debt a month earlier + the month's deficit − the change of the government's deposits − monetary financing, to the ban.
- [ ] **AC-FIS-02** `[sim]` After every month end the government's deposits at the central bank equal `gov_cash_target`, and no maturing bond is left unpaid.
- [ ] **AC-FIS-03** `[sim]` Coupons: each holder sector receives its share of a twelfth of the coupon on the stock, the sum is the government's interest under `interest.gov_debt`.
- [ ] **AC-FIS-04** `[unit]` The yield is the formula of step 4; it never falls as debt rises.
- [ ] **AC-FIS-05** `[sim]` On the Romania scenario with the 2021 law, the first year's deficit is reported with its parts (revenue by tax, spending by line, interest), and debt over GDP moves by the deficit less nominal growth.
- [ ] **AC-FIS-06** `[sim]` *(M4)* Debt-financed spending raises GDP in the short run, more when unemployment is high and the central bank does not raise rates.
- [ ] **AC-FIS-07** `[sim]` *(M4)* Persistent primary deficits with $r > g$ produce rising debt over GDP and rising yields.
- [ ] **AC-FIS-08** `[sim]` *(M12)* A consumer energy subsidy lowers measured CPI and raises energy demand and the fiscal cost.

AC-FIS-04 (`[unit]`) was written on 2026-10-20 in the mechanic's crate, `crates/econ-mech-fiscal/tests/acceptance/`, behind the crate feature `mechanic`; AC-FIS-02, 03 and 05 in `crates/econ-core/tests/acceptance/fiscal.rs`, behind the crate feature `fiscal`, against the government's debt (`World::treasury`), its monthly budget report (`World::fiscal_months`) and parameters (`WorldParams::fiscal`). AC-FIS-05 reads "moves by the deficit less nominal growth" as Δb = deficit/Y1 − b0·g/(1+g), to within one point of GDP.

### Tests owed
The spec was locked before these tests could exist (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-FIS-01: the identity as written does not hold: with a deficit financed by bonds, a rise of the government's deposits needs more debt, not less (debt = debt a month earlier + deficit + change of deposits), and bonds the central bank buys are already in the debt; waits for the owner to restate it (test session, 2026-10-20)
- AC-FIS-06: roadmap M4
- AC-FIS-07: roadmap M4
- AC-FIS-08: roadmap M12

## API sketch
```rust
// crate econ-mech-fiscal (no I/O); money in bani, rates in millionths
pub struct DebtParams { pub new_issue_years: u8, pub term_premium_ppm: u32, pub risk_ppm: u32, pub threshold_ppm: u32 }
pub fn bond_yield_ppm(policy_ppm: i32, debt: Bani, gdp_year: Bani, p: &DebtParams) -> i32;      // AC-FIS-04
pub struct DebtBook { pub by_year: Vec<Bani> /* maturing in year k from now */, pub coupon_ppm: u32 }
impl DebtBook {
    pub fn from_profile(total: Bani, buckets: &[(u8, u8, Bani)], coupon_ppm: u32) -> DebtBook;
    pub fn month_coupon(&self) -> Bani;
    pub fn mature(&mut self, months_elapsed: u32) -> Bani;                                          // what falls due this month
    pub fn issue(&mut self, amount: Bani, yield_ppm: i32, p: &DebtParams);                         // the coupon is the weighted mean
}
pub fn split_by_holdings(amount: Bani, holdings: &[Bani]) -> Vec<Bani>;                            // largest remainder
```
The month-end postings live with the `World`.

## Open questions
- [x] **Budget lines and the `World`'s purchases:** the input-output table's government purchases (211 billion lei) already contain public wages and goods; proposed that M3 adds only the budget lines the `World` does not pay (subsidies, transfers, capital), so nothing is paid twice. The budget's lines become the levers in M4, when purchases follow the ministries' budgets instead of the table. **Decided (owner, 2026-10-20): as proposed.**
- [x] **Holders' shares fixed:** proposed for M3; a portfolio rule (banks' liquidity, the rest of the world's yield gap) comes with trade and the exchange rate. **Decided (owner, 2026-10-20): as proposed.**
- [ ] Should ministries have a minimum viable budget (e.g. zero police → chaos) or scale smoothly?
- [x] Foreign-currency (EUR) borrowing: proposed yes, since Romania issues a large share of debt in EUR *(verify)*; adds FX risk to debt. Debt under foreign law matters for euro exit ([euro](euro.md)). **Decided (owner, 2026-10-20): debt in lei in M3 (the first version's simplification); foreign-currency debt with the exchange rate's later blocs.**
