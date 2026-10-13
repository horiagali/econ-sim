---
id: economy/opening-balance-sheet
title: Opening Balance Sheet
status: draft
owner: horia
depends_on: [economy/accounting, adr/0007-money-and-ledger, adr/0018-scenario-and-commands, society/population-wealth, adr/0016-firm-representation]
research: [research/opening-balance-sheet-romania]
updated: 2026-10-13
---

# Opening Balance Sheet

## Purpose
Turn the financial accounts of 31 December 2021 into the ledger's balances on the start date. The result is one signed balance in bani for every sector and ledger instrument, and every instrument row sums to exactly zero. The [scenario builder](../game/scenario-builder.md) posts these balances through the ledger with flow code `opening` and refuses to write a scenario whose opening balance sheet fails the invariants. The spec also carries two details the ledger's later users need: government debt by maturity and currency, and the central bank's reserves by kind.

## Real-world basis
- The financial accounts (Eurostat `nasq_10_f_bs`, fixture `balance_sheets2021q4_ro.json`): what each sector holds and owes in nine instruments. They do not say who holds whose claim.
- The research note [`opening-balance-sheet-romania`](../../01-research/notes/opening-balance-sheet-romania.md): central bank reserves, government debt, and why the deposit rules below close exactly.
- The balance-sheet matrix of [accounting](accounting.md), section 1.

> **Simplification:** where the statistics do not say who holds whose claim, a claim is split in proportion to the counterparts' positions, except where a rule below says otherwise.

## Owner decision this spec builds on
- **Other financial firms are a sector of their own** (owner, 2026-10-13). This means insurers, investment funds, leasing companies and other non-bank lenders: 186 bn lei of assets and 181 bn of liabilities. In v1 the sector holds its opening portfolio and has no behaviour of its own; interest and dividends pass through it.

## Changes this asks of the accounting spec
[Accounting](accounting.md) is locked. Its balance-sheet matrix needs these changes, which take effect when the owner locks this spec:
1. **A column "other financial" (OF)** between pension funds and government.
2. **The pension fund claims row becomes "pension and insurance claims".** Insurers (OF) issue claims to households, as pension funds do.
3. **The foreign assets row is every cross-border position except government bonds and equity held by the rest of the world.** That means foreign notes, deposits abroad, foreign securities, loans to and from abroad, shares in foreign firms, and gold and SDR. A resident's balance in this row is its net claim on the rest of the world, so it can be negative (a firm that owes a foreign parent).
4. **Loans may be held by any resident sector,** not only banks. That covers non-bank lenders, government loans, firms' loans to each other and to households, and the debt securities of firms and banks (rule 5d).
5. **Derivatives and "other accounts receivable and payable" are not in the ledger in v1.** These are trade credit, advances and taxes due: 1,146 bn lei on each side, 871 bn of it owed by firms. Each sector's net position in them goes into its net worth, and the consistency report lists it.

ADR-0007's sector list gains the same column.

## Inputs
- `balance_sheets2021q4_ro.json`: `assets[instrument][sector]` and `liabilities[instrument][sector]` in lei, for 8 sectors and 9 instruments.
- `sector_details2021_ro.json` (new, written by `python/pipeline/normalise_sector_details.py`): `government_debt` (total, short-term, by residual maturity, by currency) and `central_bank_reserves` (gold, SDR holdings, foreign currency deposits and securities, the official foreign currency reserves).
- The household columns of [stage H](../society/population-wealth.md) and the firm-unit balance sheets (`firm_balance_sheets2021_ro.json`), for the checks only.

## Outputs
- `balances[instrument][sector]` in bani: 10 ledger instruments × 8 sectors.

| Ledger instrument | Rows of the accounting matrix |
|---|---|
| `cash`, `deposits`, `gov_deposits_at_cb`, `reserves`, `bonds`, `loans`, `cb_advances`, `equity`, `pension_claims`, `foreign_assets` | in that order |

  Sectors, in column order: households, firms, banks, pension funds, other financial, government, central bank, rest of world.
- `left_out[instrument][sector]`: net derivatives and other accounts, in bani.
- `gaps[instrument]`: the statistical gap closed by rule 1, in bani.
- With rule 8 applied by the scenario builder, the reconciled balances. They depend on the scale and seed through the household totals.
- The opening hash: FNV-1a 64 over the number of instruments and of sectors (u32 each, little-endian), then every balance (i64, little-endian), instrument by instrument in the order above, sector by sector.

## Update rule
There is no per-day rule. The opening balance sheet is a pure function of its two input files, in integer bani. "Apportion" is largest remainder with ties to the lower index, as in the population generator. Amounts are lei × 100.

**Rule 1 — gaps.** For each statistical instrument, a surplus of assets over liabilities is added to the rest of the world's liabilities, and a deficit to its assets. Gaps are at most 0.2 million lei.

**Rule 2 — currency.** Notes issued by the central bank are `cash`: each holder +, the central bank −. Notes issued by the rest of the world (foreign notes, 8.9 bn) are `foreign_assets` of their holders. Those holders are the central bank's own holding first, then the banks' vaults (low confidence).

**Rule 3 — cross-border, by default.** For an instrument without its own rule, the rest of the world's holdings are claims on residents, apportioned to what each owes. What the rest of the world owes is held by residents, apportioned to what each holds. Both parts are `foreign_assets`. For loans, households take no part on either side.

**Rule 4 — deposits.**
- (a) Deposits the government issues are held inside the government and net out.
- (b) The government's other deposits are at the central bank: `gov_deposits_at_cb`.
- (c) The central bank's remaining deposit liabilities are the banks' `reserves`.
- (d) Bank deposits not held by households, firms, pension funds, other financial firms or the rest of the world are held by banks themselves and net out.
- (e) The banks' remaining deposits are deposits abroad, as are the central bank's: `foreign_assets`.

  The build fails if (e) does not equal what the rest of the world owes in deposits; on the 2021 data it does, to the leu.

**Rule 5 — debt securities.**
- (a) Government holdings of government securities net out.
- (b) The central bank's holdings are foreign securities (reserves).
- (c) The rest of the world's holdings are government bonds: `bonds`.
- (d) The other residents hold the rest. Foreign securities not held by the central bank are apportioned among banks, pension funds and other financial firms. Securities of firms, banks and other financial firms are apportioned among the five holding sectors and become `loans` (issuer −, holder +). What each holder has left is government `bonds`.

**Rule 6 — equity.** Shares in residents are `equity`: holder +, issuer −, the rest of the world included as a holder. Shares in foreign firms (issued by the rest of the world) are apportioned to the residents' equity holdings and become `foreign_assets`.

**Rule 7 — the rest.** Loans: `loans`. Insurance and pensions: `pension_claims`. Gold and SDR: `foreign_assets`. In each, rule 3 first takes out the cross-border part.

`cb_advances` start at zero.

**Rule 8 — households' own totals.** The household column of the ledger is the weighted sum of the households of the population ([accounting](accounting.md), section 3). Stage H hits the statistics' cash and deposits within 0.001%, not to the ban. So the scenario builder replaces the household `cash` and `deposits` with the weighted sums of `hh_cash` and `hh_deposits`, and the issuer absorbs the difference: the central bank for cash, the banks for deposits. A difference above 10 millionths of the figure is an error. At 1:1000, seed 42, the differences are −27,268.37 lei of cash and +95,755.11 lei of deposits.

> **Simplification:** households hold 9.6 bn of loan claims, so their `loans` balance is their net position (−176.9 bn), not the 187.3 bn they owe. The per-household debts of stage H add up to the 187.3 bn.

### The result on the 2021 data
In bn lei, rounded. The exact matrix is the reference's output.

| | HH | Firms | Banks | PF | OF | Gov | CB | RoW |
|---|---|---|---|---|---|---|---|---|
| cash | 57.3 | 37.6 | 12.7 | 0.0 | 1.4 | 0.3 | −109.2 | 0 |
| deposits | 294.2 | 188.9 | −542.8 | 2.1 | 20.7 | 0 | 0 | 37.0 |
| gov deposits at CB | | | | | | 41.7 | −41.7 | |
| reserves | | | 46.7 | | | | −46.7 | |
| bonds | 13.0 | 5.0 | 136.1 | 54.8 | 27.7 | −478.6 | 0 | 241.9 |
| loans | −176.9 | −160.2 | 316.0 | 3.1 | 24.6 | −6.6 | 0 | 0 |
| equity | 263.8 | −629.1 | −68.6 | 25.9 | −49.4 | 123.7 | −32.8 | 366.4 |
| pension and insurance claims | 104.7 | 3.8 | 0.7 | −91.8 | −16.9 | −0.5 | 0 | 0 |
| foreign assets | 22.6 | −191.5 | 108.4 | 6.4 | −7.6 | −38.3 | 228.5 | −128.4 |
| **net financial assets** | 578.7 | −745.5 | 9.1 | 0.5 | 0.5 | −358.4 | −1.8 | 516.9 |

With the left-out other accounts and derivatives added back, each column equals the financial net worth of the statistics. The opening hash, before rule 8, is `3e6e8302f2eded33`.

### Government debt and central bank reserves
Data for later mechanics (fiscal policy: refinancing by maturity; monetary policy and trade: reserves). The ledger holds one `bonds` balance. The maturity and currency profile is a table of the scenario that the debt mechanic will read.
- `government_debt`: total 576.6 bn lei; up to 1 year 29.38 bn (measured); 1–3, 3–5, 5–10 and over 10 years 120.0, 110.0, 177.0 and 140.22 bn (guessed, low); lei, euro and dollar 273.2, 258.6 and 44.7 bn (shares of May 2021, medium).
- `central_bank_reserves`: gold 26.50 bn, SDR holdings 16.73 bn, foreign currency deposits 88.69 bn, foreign currency securities 113.24 bn; official foreign currency reserves 200.27 bn (EUR 40,475 million).

> **Simplification:** the debt total (576.6 bn, nominal, EU methodology) and the ledger's government bonds plus loans (market value, unconsolidated) differ. The profile is shares applied to whatever the ledger holds.

## Player levers
None. The opening balance sheet is data.

## Tuning parameters
None. The rules are fixed; the guesses are in the fixture.

## Interactions
- [Scenario builder](../game/scenario-builder.md): posts the balances and checks them.
- [Money and banking](money-banking.md) starts from these reserves, deposits and loans. [Monetary policy](monetary-policy.md) starts from `reserves` and `foreign_assets` of the central bank. [Fiscal policy](fiscal-policy.md) starts from `bonds`, `gov_deposits_at_cb` and the debt profile.
- [Population wealth](../society/population-wealth.md): household cash and deposits are the same totals.

## Edge cases & failure modes
- A statistical instrument or sector is missing → named error, nothing produced.
- A rule does not close (for example, foreign notes exceed the banks' vault cash, or deposits abroad do not equal what the rest of the world owes) → named error, nothing produced. Another year's data may need the rules revisited, not forced.
- A row does not sum to zero after the rules → error (it cannot happen if the rules close; checked anyway).

## Acceptance tests
IDs are stable. The tests will live in `crates/econ-scenario/tests/acceptance/` (or the crate the builder ends up in), written in a test-authoring session.

- [ ] **AC-OBS-01** `[unit]` On the 2021 fixtures, every ledger instrument row of the opening balances sums to exactly 0 bani, and `cb_advances` is 0 for every sector.
- [ ] **AC-OBS-02** `[unit]` Before rule 8, household `cash` and `deposits` equal the currency and deposits households hold in the financial accounts, to the ban. After rule 8, with any household totals within 10 millionths of those, they equal the given totals, every row still sums to zero, and only the central bank's `cash` and the banks' `deposits` change. Totals further off are rejected with a named error.
- [ ] **AC-OBS-03** `[unit]` Each sector's net financial assets in the ledger, plus its left-out net derivatives and other accounts, equals its financial net worth in the statistics, within the closed gaps (at most 1 million lei in all).
- [ ] **AC-OBS-04** `[unit]` Deposits close by rule 4: the central bank's deposits abroad plus the banks' remainder equal the rest of the world's deposit liabilities exactly. Data where they do not are rejected with a named error and no output.
- [ ] **AC-OBS-05** `[unit]` The rest of the world's `bonds` balance equals its holdings of debt securities in the statistics, and the central bank's `foreign_assets` include all its debt securities, deposits, gold and SDR (net of the SDR allocation).
- [ ] **AC-OBS-06** `[unit]` Every balance of the opening ledger is posted with flow code `opening`, and the ledger's invariant check passes after posting.
- [ ] **AC-OBS-07** `[golden]` The opening hash on the 2021 fixtures, before rule 8, equals the committed golden value (`3e6e8302f2eded33` from the reference).
- [ ] **AC-OBS-08** `[diff]` The Rust opening balances equal the Python reference's (`python/reference/opening_balance_reference.py`) for every instrument and sector.
- [ ] **AC-OBS-09** `[unit]` The sector details: the maturity buckets and the currencies each sum to the government debt total, the first bucket equals the measured short-term debt, and gold plus SDR holdings equal the central bank's gold and SDR in the financial accounts.
- [ ] **AC-OBS-10** `[unit]` Removing a sector or an instrument from the statistics, or making foreign notes exceed the banks' vault cash, gives a named error and no output.

## API sketch
In the crate of the scenario builder (see [scenario builder](../game/scenario-builder.md)). It needs the ledger to grow the sector `OtherFinancial` and the instruments of the accounting matrix (its "Not built yet" table); `Sector::ALL` and `Instrument::ALL` keep matrix order.

```rust
pub enum StatInstrument { GoldAndSdr, Currency, Deposits, DebtSecurities, Loans, Equity,
                          InsuranceAndPensions, Derivatives, OtherAccounts }

/// The financial accounts, in lei: [instrument][sector], sectors in `Sector::ALL` order.
pub struct SectorAccounts {
    pub assets: Vec<[i64; 8]>,       // indexed by StatInstrument
    pub liabilities: Vec<[i64; 8]>,
}

pub struct OpeningBalances {
    pub balances: Vec<[Bani; 8]>,    // indexed by Instrument (ledger), sectors in Sector::ALL order
    pub left_out: Vec<[Bani; 8]>,    // derivatives, other accounts
    pub gaps: Vec<Bani>,             // per StatInstrument
}

pub enum OpeningError {
    MissingInstrument(StatInstrument),
    MissingSector(Sector),
    RuleDoesNotClose { rule: u8, instrument: StatInstrument },
    HouseholdsTooFar { instrument: Instrument },        // rule 8
}

pub fn opening_balances(accounts: &SectorAccounts) -> Result<OpeningBalances, OpeningError>;
/// Rule 8: the household column from the population's weighted totals.
pub fn reconcile(opening: &OpeningBalances, household_cash: Bani, household_deposits: Bani)
    -> Result<OpeningBalances, OpeningError>;
impl OpeningBalances {
    pub fn state_hash(&self) -> u64;
    /// One linked transaction, flow code `opening`, that sets every balance (AC-OBS-06).
    pub fn post(&self, ledger: &mut Ledger) -> Result<(), LedgerError>;
}

pub struct SectorDetails { pub government_debt: GovernmentDebt, pub central_bank_reserves: CentralBankReserves }
pub fn read_sector_details(json: &str) -> Result<SectorDetails, SaveError>;
```

`post` sets balances from zero, so each balance is a leg between the sector and a counterpart in the same row. One way: walk the row in sector order, carrying the running sum. The legs of a row cancel, and the ledger's row invariant holds after every leg.

## Open questions
- [ ] **The accounting changes above** (OF column, insurance in the claims row, the foreign assets row, loans held by any resident, other accounts left out). Accept as written, or change?
- [ ] **Other accounts left out (1,146 bn each side).** They are mostly trade credit between firms and taxes due. Leaving them out puts firms' net −179 bn into net worth. Alternative: a passive `other_accounts` row, kept but never moved in v1. My default: leave out, as written.
- [ ] **Government deposits issued by the government (rule 4a)** are assumed intra-government. If they are in fact the Treasury's deposits from households or firms, the government's deposits at the central bank grow and bank reserves shrink by the same amount.
- [ ] **Bank reserves of 46.7 bn** come out as a residual. They are above the minimum reserves (about 30 to 35 bn, low confidence). Check against the BNR's monetary balance sheet when the money-banking milestone starts.
- [ ] **The maturity buckets above one year** are a guess. Replace them with the Ministry of Finance's debt bulletin of December 2021 when someone reads it.
