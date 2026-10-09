---
id: economy/money-banking
title: Money & Banking
status: draft
owner: horia
depends_on: [economy/accounting, economy/monetary-policy, adr/0016-firm-representation]
research: []
updated: 2026-10-09
---

# Money & Banking

## Purpose
Banks create money by lending, set the interest rates households and firms actually pay, and can fail. This is the transmission channel from the central bank to the real economy, and a source of emergent financial crises.

## Real-world basis
- **Endogenous money:** loans create deposits (Bank of England, 2014). Lending is limited by profitable demand, capital requirements and risk appetite, not by reserves.
- Loan and deposit rates follow the policy rate plus spreads that widen with risk and bank weakness.
- Bank runs happen when depositors doubt solvency; deposit insurance reduces them.
- Credit booms raise asset prices and default risk later (Minsky, Schularick & Taylor).

## Representation
v1: one aggregate commercial **banking sector** (possibly 2–3 banks later for contagion). Balance sheet: loans (households by type, firms per **firm unit**), government bonds, reserves (A); deposits, central bank advances, equity (L).

**Firm loans per firm unit** ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)). Each named firm and each cohort firm holds its own loans; a cohort's loans are per real firm, and postings with the bank are multiplied by `firm_count` ([ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md)). Spreads and credit limits can differ by size class (micro, small, medium/large, named).

## State variables
| Symbol | code_name | Meaning |
|---|---|---|
| $L$ | `bank_loans[type]` | Loans by borrower type |
| $D$ | `deposits` | |
| $R$ | `reserves` | Held at central bank |
| $E^b$ | `bank_equity` | Capital |
| $CAR$ | `capital_adequacy_ratio` | $E^b / \text{RWA}$ |
| $NPL$ | `non_performing_loans` | Share of loans in default |
| $r^L, r^D, r^M$ | `loan_rate`, `deposit_rate`, `mortgage_rate` | %/yr |

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

## Player levers
Reserve and capital requirements, LTV and DSTI limits (macroprudential), deposit insurance, bailout or nationalisation, CB lender-of-last-resort policy.

## Outputs
Interest rates, credit growth, money supply (M1, M2), bank capital and NPLs, credit crunch indicator.

## Acceptance tests
- [ ] New lending increases deposits by the same amount (money creation) every tick.
- [ ] A policy-rate hike raises new loan rates within 1 tick and average loan rates gradually.
- [ ] A wave of firm defaults cuts bank capital, then credit supply, then investment (credit crunch).
- [ ] Cohort defaults lower `firm_count` and bank loans by the defaulting firms' share; the loan book still equals the count-weighted sum of firm-unit loans.
- [ ] A bank run can occur with low capital and low deposit insurance, and not with high capital.

## Open questions
- [ ] One bank or several in v1?
- [ ] Shadow banking or bond markets for corporate debt?
