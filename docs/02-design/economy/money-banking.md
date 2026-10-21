---
id: economy/money-banking
title: Money & Banking
status: locked
owner: horia
depends_on: [economy/accounting, economy/monetary-policy, economy/opening-balance-sheet, game/world-loop, adr/0016-firm-representation, adr/0017-time-base]
research: []
updated: 2026-10-21
---

# Money & Banking

## Purpose
Banks create money by lending, set the interest rates households and firms actually pay, and can fail. This is the transmission channel from the central bank to the real economy, and a source of emergent financial crises.

## Real-world basis
- **Endogenous money:** loans create deposits (Bank of England, 2014). Lending is limited by profitable demand, capital requirements and risk appetite, not by reserves.
- Loan and deposit rates follow the policy rate plus spreads that widen with risk and bank weakness.
- Bank runs happen when depositors doubt solvency; deposit insurance reduces them.
- Credit booms raise asset prices and default risk later (Minsky, Schularick & Taylor).
- **Romania, end of 2021** (from memory, medium confidence; to be checked against the BNR's statistics when the scenario's rates are collected): policy rate 1.75% (raised to 2.00% in January 2022), lending facility 2.75%, deposit facility 0.75%; minimum reserves 8% of lei deposits and 5% of foreign-currency deposits; banks' capital adequacy about 22%, non-performing loans about 3.4%; new loans to firms at about 5%, mortgages about 4.5%, consumer loans about 9%; term deposits about 1.5%. In the opening balance sheet ([opening-balance-sheet](opening-balance-sheet.md)) banks hold 316 billion lei of loans, 136 billion of government bonds and 46.7 billion of reserves, and owe 543 billion of deposits.

## Representation
v1: one aggregate commercial **banking sector** (possibly 2–3 banks later for contagion). Balance sheet: loans (households by type, firms per **firm unit**), government bonds, reserves (A); deposits, central bank advances, equity (L).

**Firm loans per firm unit** ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)). Each named firm and each cohort firm holds its own loans; a cohort's loans are per real firm, and postings with the bank are multiplied by `firm_count` ([ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md)). Spreads and credit limits can differ by size class (micro, small, medium/large, named).

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| A firm unit short of money draws on its credit line | daily, at the end of the day's payments (replaces the `World`'s credit inside the firm sector) |
| Interest on loans and deposits, repayments | monthly, on the last day, after pay |
| Rates reset (new lending and the stock's floating part) | monthly close |
| Reserve requirement checked; central bank advances | daily, at the end of the day |
| Defaults of firm units' excess debt | monthly close |
| Bank capital and the capital adequacy ratio | monthly close |

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $L_f$ | `unit_loans[f]` | a firm unit's bank loans (all its firms) | LCU |
| $L_h$ | `hh_mortgage`, `hh_consumer_debt` | a household's loans (stage H) | LCU |
| $D$ | `deposits` | the households' and firm units' columns, summed in the ledger | LCU |
| $R$ | `reserves` | banks' reserves at the central bank | LCU |
| $A$ | `cb_advances` | central bank advances to banks | LCU |
| $E^b$ | `bank_equity` | the banks' net worth in the ledger | LCU |
| $CAR$ | `capital_adequacy_ratio` | $E^b / \text{RWA}$ | ratio |
| $NPL$ | `non_performing_loans` | share of loans written off in the last twelve months | ratio |
| $r^L, r^D, r^M$ | `loan_rate`, `deposit_rate`, `mortgage_rate` | rates | %/yr |

## Update rule
**Rates:**
```math
r^L_{type} = i + s^0_{type} + s_{risk}\cdot \text{default prob}_{type} + s_{cap}\cdot \max(0, CAR^{min} + \text{buffer} - CAR)
```
$r^D = i - s^D$ (floored at ~0). Pass-through takes a few ticks for fixed-rate stock; new loans reprice immediately.

**Credit supply:** banks approve loan demand up to limits: (a) capital: $E^b / RWA \ge CAR^{min}$; (b) borrower affordability (debt-service-to-income, loan-to-value for mortgages); (c) risk appetite (falls after losses). Approved loans create deposits (accounting).

**Defaults:** a named firm defaults as a whole when its cash flow can't cover interest for N ticks. In a cohort, a number of firms exit each tick (exit rate rising with financial stress, aligned with carried rounding): `firm_count` falls by that number, and their share of the cohort's loans defaults. Households default when debt service > threshold. Losses × (1 − recovery) reduce bank equity.

**Liquidity:** if reserves < requirement, banks borrow from the central bank at the policy rate (+ penalty).

**Bank run (crisis mechanic):** if $CAR$ falls below a confidence threshold and deposit insurance is below deposits, depositors withdraw a fraction per tick (to cash or abroad). The bank must use reserves, then CB advances (lender of last resort if allowed). The player can bail out (capital injection = government buys bank equity) or let it fail (depositors above insurance lose; credit crunch).

## First version (roadmap M3)
What is built first, on the `World` of [game/world-loop](../game/world-loop.md). It replaces two M2 placeholders: credit inside the firm sector, and money that earns no interest.

**Starting state.** Each firm unit's bank loans are its `loans` in the firm-unit table, scaled so that their sum is the banks' claim on firms in the opening balance sheet (the rest of the table's loans are non-bank and cross-border, already in other rows). Households' loans are stage H's `hh_mortgage` and `hh_consumer_debt`. Rates start at the policy rate plus the spreads below; the policy rate is a constant of the scenario until [monetary policy](monetary-policy.md) sets it.

**In:**
1. **Credit lines, daily.** A cohort unit whose deposits are below zero at the end of the day's payments borrows the shortfall from the banks: its `unit_loans` and its deposits grow by the same amount (a loan creates a deposit; flow code `finance.loan`). The line is open while the unit's loans stay under `credit_limit_months` of its revenue and the banks' capital adequacy stays above `car_min`. A public producer's shortfall stays with the government as in M2.
2. **Interest, at month end.** Each borrower pays a month of interest on its loans at its rate, from its deposits; each depositor receives a month of interest on its deposits at the deposit rate (flow codes `interest.loan`, `interest.deposit`). Households and firm units are paid and charged per record; the ledger posts the weighted sums. Reserves above the requirement earn the deposit facility rate from the central bank; advances cost the lending facility rate.
3. **Repayment, at month end.** Firm units repay `firm_repayment_share` of their loans a month when their deposits allow it beyond a month of wages (a repayment destroys a deposit). Households pay interest only in M3: their loans are amortised and renewed with the housing market (M13).
4. **Default, at the monthly close.** A unit whose loans exceed `credit_limit_months` of its revenue has the excess written off: the banks lose `1 − recovery_rate` of it (their equity falls), the rest the unit keeps owing. Firm exit (falling `firm_count`) comes with firm dynamics (M9).
5. **Reserves, daily.** If banks' reserves are below `reserve_requirement` of deposits, they borrow the gap from the central bank (advances, flow code `finance.cb_advance`); advances are repaid as soon as reserves exceed the requirement.
6. **Bank capital, at the monthly close.** Banks' equity moves with interest margins, write-offs and the payments of the banking industry's units (already in production); capital adequacy is equity over risk-weighted loans (firms and consumer loans 100%, mortgages 35%, government bonds and reserves 0%).

> **Simplification (M3):** one bank; no bank runs, deposit insurance or bailouts (crisis dynamics are not modelled in v1, roadmap D10); no household borrowing or repayment; no rate differences by size class; foreign-currency loans and deposits are in lei.

**Not in M3:** the bank run, macroprudential limits (LTV, DSTI) as levers (M4), several banks, mortgages tied to house prices (M13), firm exit (M9).

## Player levers
Reserve and capital requirements, LTV and DSTI limits (macroprudential), deposit insurance, bailout or nationalisation, CB lender-of-last-resort policy. In M3 none: they come with the lever system (M4).

## Outputs
Interest rates, credit growth, money supply (M1, M2), bank capital and NPLs, credit crunch indicator.

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `policy_rate` | 1.75% a year (until monetary policy sets it) | the base of every rate |
| `spread_firms` | 3.0 points | firm loans over the policy rate |
| `spread_mortgage` | 2.75 points | mortgages |
| `spread_consumer` | 7.25 points | consumer loans |
| `deposit_spread` | 1.0 point, the rate not below 0 | deposits under the policy rate |
| `facility_corridor` | ±1 point | the central bank's lending and deposit facilities around the policy rate |
| `reserve_requirement` | 8% of deposits | reserves banks must hold |
| `car_min` | 10% | capital adequacy below which new credit stops |
| `credit_limit_months` | 12 months of revenue | how far a unit can borrow |
| `firm_repayment_share` | 1/60 a month | five-year loans |
| `recovery_rate` | 40% | what the banks recover of a written-off loan |

All are guesses until calibration (M7), marked medium or low confidence above.

## Interactions
- **From [monetary policy](monetary-policy.md):** the policy rate and the corridor. **From [production](production.md) and the [labour market](labor-market.md):** units' revenue and wage bills.
- **To [households](households.md):** interest received and paid is income and spending of the month (`hh_income_received`). **To [investment](investment-capital.md):** the loan rate and the credit line (M3).
- **To [fiscal policy](fiscal-policy.md):** banks hold government bonds and receive their coupons.

## Edge cases & failure modes
- **A unit at its credit limit and still short:** it pays what it can; its wages are paid first, then inputs; the rest becomes arrears written off at the close (step 4).
- **Banks below `car_min`:** no new credit lines open; existing ones are not called.
- **Negative policy rates:** deposits pay at least 0%; loans keep their spreads.

## Acceptance tests
IDs are stable (added 2026-10-20). Tests go in `crates/econ-core/tests/acceptance/` (protected).

- [ ] **AC-BANK-01** `[sim]` A credit draw raises the unit's loans and deposits by the same amount, and the banks' loans and deposit liabilities by the same amount (money creation); a repayment does the reverse.
- [ ] **AC-BANK-02** `[sim]` Every month end, interest paid by borrowers less interest paid to depositors equals the banks' interest income in the ledger, under its flow codes, to the ban.
- [ ] **AC-BANK-03** `[sim]` For every firm unit and day, its loans in its column equal its share of the ledger's firm loans: the units' sum equals the banks' claim on firms (I-4).
- [ ] **AC-BANK-04** `[sim]` No firm unit's deposits are negative at the end of any day, with no credit inside the firm sector.
- [ ] **AC-BANK-05** `[sim]` A one-point rise of the policy rate raises new and floating loan rates by one point at the next monthly close.
- [ ] **AC-BANK-06** `[sim]` A write-off lowers bank equity by (1 − recovery) of it; banks below the minimum capital ratio open no new credit line.
- [ ] **AC-BANK-07** `[sim]` Banks' reserves are never below the requirement at the end of a day; when they would be, central bank advances fill the gap and are repaid when reserves allow.
- [ ] **AC-BANK-08** `[sim]` Quiet baseline: on the Romania scenario with the 2021 law, over 120 months total bank loans to firms stay within 30% of their start and capital adequacy above `car_min`.
- [ ] **AC-BANK-09** `[sim]` *(M9)* A wave of firm defaults cuts bank capital, then credit supply, then investment (credit crunch).
- [ ] **AC-BANK-10** `[sim]` *(M9)* Cohort defaults lower `firm_count` and bank loans by the defaulting firms' share; the loan book still equals the count-weighted sum of firm-unit loans.
- [ ] **AC-BANK-11** `[sim]` *(later; crisis dynamics, D10)* A bank run can occur with low capital and low deposit insurance, and not with high capital.

AC-BANK-01 to 08 were written on 2026-10-20 in `crates/econ-core/tests/acceptance/banking.rs`, behind the crate feature `banking`, against the test writer's additions to the `World`: the banks' read-only state (`World::bank`), their parameters (`WorldParams::banking`, `WorldParams::policy_rate_ppm`) and two start edits (`StartEdit::PolicyRate`, a change of the policy rate from a given day, for AC-BANK-05; `StartEdit::ExtraLoan`, a loan drawn on day 0, for AC-BANK-06). The crate `econ-mech-banking` holds the API sketch as a skeleton. Flow codes are found by the names above.

### Tests owed
The spec was locked before these tests could exist (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-BANK-04: the M1 item "refine the 64 into the 90" (the data-centre unit's wage bill), by the owner's decision of 2026-10-21; its test is written and ignored
- AC-BANK-09: roadmap M9
- AC-BANK-10: roadmap M9
- AC-BANK-11: after v1

## API sketch
For the test writer and the implementer. Rates and shares in millionths; money in bani.

```rust
// crate econ-mech-banking (no I/O)
pub struct BankParams { pub spread_firms_ppm: u32, pub spread_mortgage_ppm: u32, pub spread_consumer_ppm: u32,
                        pub deposit_spread_ppm: u32, pub reserve_requirement_ppm: u32, pub car_min_ppm: u32,
                        pub credit_limit_months: u32, pub firm_repayment_ppm: u32, pub recovery_ppm: u32 }
pub struct Rates { pub firms_ppm: u32, pub mortgage_ppm: u32, pub consumer_ppm: u32, pub deposit_ppm: u32 }
pub fn rates(policy_ppm: i32, car_ppm: u32, p: &BankParams) -> Rates;                             // AC-BANK-05
pub fn month_interest(balance: Bani, rate_ppm: u32) -> Bani;                                       // a twelfth of a year, rounded once
pub fn credit_room(loans: Bani, revenue_month: Bani, car_ppm: u32, p: &BankParams) -> Bani;        // what a unit may still draw
pub fn write_off(loans: Bani, revenue_month: Bani, p: &BankParams) -> (Bani /* written off */, Bani /* bank loss */);
pub fn capital_ratio_ppm(equity: Bani, firm_loans: Bani, consumer: Bani, mortgages: Bani) -> u32;
```

The daily credit draws, the month-end interest and repayments and the reserve check live with the `World`. New flow codes, append-only: `interest.loan`, `interest.deposit`, `interest.reserves`, `finance.loan` (draws and repayments), `finance.cb_advance`, `finance.write_off`.

## Open questions
- [x] **One bank or several in v1?** Proposed: one until the bank run is modelled (after v1, D10). **Decided (owner, 2026-10-20): as proposed.**
- [x] **Shadow banking or bond markets for corporate debt?** Proposed: no; other financial firms keep their opening portfolio (owner, 2026-10-13). **Decided (owner, 2026-10-20): as proposed.**
- [x] **Households' new borrowing in M3:** proposed none (interest only, the stock constant) until the housing market (M13). The alternative, lending each month what households repay to households of the same fifths, keeps credit moving but has no model of who borrows. **Decided (owner, 2026-10-20): as proposed.**
- [x] **The credit limit and the write-off rule** replace the M2 credit inside the firm sector; a unit that is structurally loss-making (the scale artefact of small units at 1:1000) will default every year. Accept, or keep a sector-internal fallback for those. **Decided (owner, 2026-10-20): accepted; no sector-internal fallback.**
- [ ] Rates and requirements of the start date to be checked against the BNR when the scenario's rates are collected.
