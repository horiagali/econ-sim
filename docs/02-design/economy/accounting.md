---
id: economy/accounting
title: Accounting & Stock-Flow Consistency
status: draft
owner: horia
depends_on: []
research: []
updated: 2026-10-09
---

# Accounting & Stock-Flow Consistency

## Purpose
Guarantee that money never appears or disappears, that every financial asset has a matching liability, and that every number can be traced to transactions. This is pillars 1 and 2 made concrete, and the main defence against "buggy economy" outcomes.

## Real-world basis
National accounts (SNA 2008), flow of funds, and Godley & Lavoie *Monetary Economics* (balance-sheet matrix + transaction-flow matrix; rows and columns sum to zero).

## Design
### 1. Balance-sheet matrix (stocks, end of tick)
Columns: households (sum of synthetic households × weights; also kept per household), firms (sum of firm units × firm counts, per industry; also kept per firm unit, [ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)), banks, Pillar II pension funds (PF), government, central bank, rest of world. Rows: instruments.

| Instrument | Households | Firms | Banks | PF | Gov | CB | RoW | Σ |
|---|---|---|---|---|---|---|---|---|
| Cash | +Hc | | +Hb | | | −H | | 0 |
| Deposits | +D_h | +D_f | −D | +D_pf | | | | 0 |
| Gov deposits at CB | | | | | +D_g | −D_g | | 0 |
| Reserves | | | +R | | | −R | | 0 |
| Gov bonds | +B_h | | +B_b | +B_pf | −B | +B_cb | +B_row | 0 |
| Loans | −L_h | −L_f | +L | | | | | 0 |
| CB advances | | | −A | | | +A | | 0 |
| Equity | +E_h | −E_f | −E_b | +E_pf | +E_g (SOE) | | +E_row | 0 |
| Pension fund claims | +P_h | | | −P | | | | 0 |
| FX reserves / foreign assets | | | | | | +F_cb | −F | 0 |
| Real assets (capital, housing, inventories, public capital) | +H | +K +Inv | | | +K_g | | | ≠0 |
| **Net worth** | NW_h | NW_f | NW_b | NW_pf (≈0) | NW_g | NW_cb | NW_row | Σ = real assets |

Firm equity is split across holders by each unit's ownership vector `ownership[f]` (state → E_g, foreign → E_row, domestic private → E_h and E_pf); see the Ownership section of [ADR-0016](../../03-architecture/decisions/0016-firm-representation.md). Pillar II pension funds are a financial sub-sector holding assets on behalf of member households, whose claims are `hh_pension_assets`.

Financial rows sum to zero exactly. Real assets are the only source of aggregate net worth.

### 2. Transaction-flow matrix (flows per tick)
Rows: consumption, government purchases, investment, exports, imports, intermediate purchases (by industry pair), wages, profits and dividends, interest (by instrument), taxes (by type), transfers (by programme), subsidies, depreciation, central bank profits to government, plus "changes in" each financial stock. Each **column** (sector budget constraint) sums to zero; each **row** sums to zero.

### 3. Implementation rules
- **Double entry everywhere.** Every transaction is a function `transfer(from, to, instrument, amount, reason)` that debits one account and credits another in the same call. No mechanic may write a balance directly.
- **Reason codes.** Every transfer carries a reason (e.g. `tax.income`, `transfer.student_grant`, `purchase.good[car_parts]`). Summing transfers by reason produces the national accounts and powers the "why" panel.
- **Settlement medium.** Non-bank payments settle in bank deposits; bank–government–CB payments settle in reserves.
- **Revaluations** (asset price and FX changes) are booked as a separate "revaluation" flow, not as a transaction, so the identity "Δstock = transactions + revaluations" holds.
- **Per-household sub-ledgers.** Household entries are booked per synthetic household; the household column is their weighted sum.
- **Per-firm-unit sub-ledgers.** Firm entries are booked per firm unit (per real firm for cohorts, plus a carried remainder `posting_remainder[f]`); the firm column is their sum weighted by `firm_count`.
- **Clearing accounts.** Flows between two weighted records (households ↔ cohort firms, cohort ↔ cohort) go through per-industry clearing accounts: `wage_clearing[j]` for wages, `sales_clearing[j]` for sales, and a dividend clearing account. Payers post totals (per-unit amount × weight), receivers are paid per unit × their own weight, and each clearing account nets to zero every tick ([ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md), [ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)).
- **Dividends** are paid pro rata to each firm unit's ownership vector: state share → government revenue, foreign share → RoW (primary-income outflow), domestic private share → households and pension funds.

### 4. Invariant checks (every tick)
1. Each financial instrument row sums to exactly 0 (integer money, no tolerance; see [ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md)).
2. For each sector: Δnet worth = saving + capital transfers + revaluations.
3. Sum of sector net lending = 0 (incl. RoW).
4. GDP by production = GDP by expenditure = GDP by income (up to statistical rounding of zero).
5. No negative cash or deposit balance except where an overdraft instrument exists.
6. Every clearing account nets to exactly zero (I-8 in [ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md)).

A failed check halts the simulation in debug builds and logs the offending reason codes.

## Outputs
National accounts (GDP three ways, sector balances, current account), per-sector balance sheets, per-household balance sheets (aggregated for any group), flow-of-funds history.

## Edge cases & failure modes
- Floating-point drift over decades → avoided: money is integer bani (`Bani(i64)`), [ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md).
- Defaults (firm, household, bank, government): write-downs are booked as losses to creditors with a reason code. Never silently zero a balance.

## Acceptance tests
- [ ] All invariant checks pass for 100 simulated years under random lever changes (fuzz test).
- [ ] GDP measured three ways agrees every tick.
- [ ] Sum of sector financial balances is 0 every tick.
- [ ] A transfer with an unknown reason code fails at test time.
- [ ] Wage, sales and dividend clearing accounts net to exactly zero every tick at 1:1000, 1:100 and 1:10.

## Open questions
- [ ] Do government deposits sit at the central bank only (proposed) or also at commercial banks?
- [x] Integer money vs float → integer bani ([ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md), accepted).
