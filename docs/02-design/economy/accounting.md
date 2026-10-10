---
id: economy/accounting
title: Accounting & Stock-Flow Consistency
status: locked
owner: horia
depends_on: [adr/0007-money-and-ledger, adr/0016-firm-representation, adr/0017-time-base]
research: []
updated: 2026-10-10
---

# Accounting & Stock-Flow Consistency

## Purpose
Guarantee that money never appears or disappears, that every financial asset has a matching liability, and that every number can be traced to transactions. This is pillars 1 and 2 made concrete, and the main defence against "buggy economy" outcomes.

## Real-world basis
National accounts (SNA 2008), flow of funds, and Godley & Lavoie *Monetary Economics* (balance-sheet matrix + transaction-flow matrix; rows and columns sum to zero).

## Design
### 1. Balance-sheet matrix (stocks, end of day)
Columns: households (sum of synthetic households × weights; also kept per household), firms (sum of firm units × firm counts, per industry; also kept per firm unit, [ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)), banks, Pillar II pension funds (PF), government, central bank, rest of world. Rows: instruments.

| Instrument | Households | Firms | Banks | PF | Gov | CB | RoW | Σ |
|---|---|---|---|---|---|---|---|---|
| Cash | +Hc | | +Hb | | | −H | | 0 |
| Deposits | +D_h | +D_f | −D | +D_pf | | | | 0 |
| Gov deposits at CB (the government holds no deposits at commercial banks) | | | | | +D_g | −D_g | | 0 |
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

### 2. Transaction-flow matrix (flows per day, summed per month, quarter and year)
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)). The matrix below holds for the flows of any day and so for any sum of days; the ledger keeps running totals for the open month, quarter and year.

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

### 4. Invariant checks (every day)
The list and its numbering are those of [ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md), which the code and the other specs cite.
1. **I-1:** every posting is balanced.
2. **I-2:** account-type constraints hold: no negative cash or deposit balance except where an overdraft instrument exists.
3. **I-3:** each financial instrument row of the balance sheet sums to exactly 0 (integer money, no tolerance); transaction-flow rows and columns sum to zero; for each sector Δstock = Σ flows + revaluations, so Δnet worth = saving + capital transfers + revaluations and the sum of sector net lending, including the rest of the world, is 0.
4. **I-4:** independent balance assertions match, computed by a different code path (for example, bank deposit liabilities = sum of weighted household and firm deposits).
5. **I-5:** postings are immutable and replay byte for byte.
6. **I-6:** explanation sums equal the observed change ([ADR-0008](../../03-architecture/decisions/0008-explainability-architecture.md)).
7. **I-7:** GDP by production, by expenditure and by income agree exactly in bani.
8. **I-8:** every clearing account nets to exactly zero at the end of the day.

A failed check halts the simulation in debug builds and logs the offending reason codes.

## Outputs
National accounts (GDP three ways, sector balances, current account), per-sector balance sheets, per-household balance sheets (aggregated for any group), flow-of-funds history.

## Edge cases & failure modes
- Floating-point drift over decades → avoided: money is integer bani (`Bani(i64)`), [ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md).
- Defaults (firm, household, bank, government): write-downs are booked as losses to creditors with a reason code. Never silently zero a balance.

## Acceptance tests
IDs are stable (added 2026-10-10 so the tests-first flow can cite them). Tests go in `crates/econ-ledger/tests/acceptance/` (protected; to be written in a test-authoring session). None exists there yet; the unit tests inside the ledger crate cover parts of AC-ACC-03 today.

- [ ] **AC-ACC-01** `[sim]` All invariant checks pass for 100 simulated years under random lever changes (fuzz test).
- [ ] **AC-ACC-02** `[sim]` GDP measured three ways agrees every tick.
- [ ] **AC-ACC-03** `[ledger]` Sum of sector financial balances is 0 every tick.
- [ ] **AC-ACC-04** `[unit]` A transfer with an unknown reason code fails at test time.
- [ ] **AC-ACC-05** `[sim]` Wage, sales and dividend clearing accounts net to exactly zero every tick at 1:1000, 1:100 and 1:10.

### Tests owed
The spec was locked before these tests could exist (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-ACC-01: the lever system and every sector on the real `World` (roadmap M4)
- AC-ACC-02: production, incomes and spending on the real `World` (roadmap M3)
- AC-ACC-03: testable on the ledger as it is; next test-authoring session
- AC-ACC-04: testable on the ledger as it is; next test-authoring session
- AC-ACC-05: clearing accounts, with the firm-unit goods market (roadmap M2)

## API sketch
For the test writer and the implementer. The ledger exists as crate `econ-ledger` (Spike 2); this is its API today, then what the Design section still asks for.

```rust
// crate econ-ledger, as implemented
pub enum Sector { Households, Firms, Banks, Government, CentralBank, RestOfWorld }
pub enum Instrument { Cash, Deposits, Loans, Bonds }
pub struct FlowCode(pub u16);                 // the "reason"; append-only numbering, e.g. TAX_VAT = 3002
impl FlowCode { pub fn name(self) -> &'static str; pub fn is_flow(self) -> bool; }

pub struct Leg { pub payer: Sector, pub payee: Sector, pub instrument: Instrument, pub amount: Bani, pub code: FlowCode }
pub struct Txn { /* legs */ }                 // linked legs: applied completely or not at all
impl Txn {
    pub fn new() -> Self;
    pub fn leg(self, payer: Sector, payee: Sector, instrument: Instrument, amount: Bani, code: FlowCode) -> Self;
    pub fn weighted_leg(self, payer: Sector, payee: Sector, instrument: Instrument,
                        per_unit: Bani, weight: u32, code: FlowCode) -> Self;   // per-unit amount x hh_weight or firm_count
}
pub enum LedgerError { NonPositiveAmount(Leg), SelfTransfer(Leg), WouldGoNegative { sector, instrument, balance } }
pub enum InvariantError { RowNotZero { .. }, NegativeBalance { .. }, FlowRowNotZero { .. }, ColumnMismatch { .. } }

pub struct Ledger { /* balances, this tick's flows */ }
impl Ledger {
    pub fn new() -> Self;
    pub fn allow_negative(&mut self, sector: Sector, instrument: Instrument);   // declare the issuer or debtor
    pub fn begin_tick(&mut self, tick: u32);
    pub fn commit(&mut self, txn: Txn) -> Result<(), LedgerError>;              // the only way a balance changes
    pub fn balance(&self, sector: Sector, instrument: Instrument) -> Bani;
    pub fn net_financial_assets(&self, sector: Sector) -> Bani;
    pub fn flows(&self) -> &TickFlows;                                          // by flow code and sector
    pub fn check_invariants(&self) -> Result<(), Vec<InvariantError>>;          // I-1, I-2, I-3 of ADR-0007
}
```

`transfer(from, to, instrument, amount, reason)` in the Design section is one `Txn` leg; the reason is a `FlowCode`.

**Not built yet** (each is in the Design section above; the API grows when the mechanic that needs it arrives):

| Missing | Design section | Needed by |
|---|---|---|
| Pension funds as a sector | 1 | social transfers |
| Instruments: government deposits at the central bank, reserves, central bank advances, equity, pension fund claims, foreign assets | 1 | banks, central bank, firms, trade |
| Real assets and net worth (the last two rows of the balance sheet) | 1 | investment, housing |
| Sub-ledgers per household and per firm unit | 3 | households, firms (the scale world keeps household deposits in its own columns today) |
| Clearing accounts and their check (I-8) | 3, 4 | the firm-unit goods market |
| Revaluation flows | 3 | housing, trade |
| Checks I-4 to I-7 of [ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md) (independent balances, replay, explanation sums, GDP three ways) | 4 | the first full tick |
| Flow codes beyond consumption, government purchases, wages, income tax, VAT and opening | 3 | every mechanic, as it arrives |

## Open questions
Answers accepted by the owner on 2026-10-10, when the spec was locked.
- [x] **The invariant lists differed.** Section 4 is now the list of [ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md), I-1 to I-8.
- [x] **Lock now or after the first full tick?** Now. The spec is the target; the table "Not built yet" says what the code still owes, and each criterion is tested when the pieces it needs exist. The API sketch may grow as that table is worked off; the Design section may not change without the owner.
- [x] Government deposits sit at the central bank only.
- [x] Integer money vs float → integer bani ([ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md), accepted).
