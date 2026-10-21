---
id: economy/imputed-rent
title: Imputed Rent of Owner-Occupiers (M3 addition to households)
status: locked
owner: horia
depends_on: [economy/households, economy/production, economy/investment-capital, economy/taxation, game/world-loop, society/population-housing, society/population-wealth, adr/0007-money-and-ledger, adr/0016-firm-representation, adr/0017-time-base]
research: [research/imputed-rent-romania-2021]
updated: 2026-10-21
---

# Imputed Rent of Owner-Occupiers (M3 addition to households)

## Purpose
Make imputed rent both spending and income of the owner households, as in the national accounts. Today the `World`:
- charges households for it through the basket (`real_estate` is 13.8% of the basket);
- keeps it as real-estate units' profit;
- pays 38% of it abroad as foreign-controlled units' dividends.

This draft is written to be moved, as a dated M3 addition, into [households](households.md), with one sentence in [production](production.md). Both specs are locked, so that needs the owner's confirmation. The criteria then take households' IDs (AC-HH-11 to 14).

## Real-world basis
From the [research note](../../01-research/notes/imputed-rent-romania-2021.md):
- **Size.** Imputed rentals were 80.7 billion lei in 2021 (11.2% of household consumption); actual rentals were 2.4 billion.
- **Output.** Owner-occupied dwellings (L68A): value added is 95.4% of output.
- **Tenure.** 90.9% of private households own their home.

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $\theta$ | `imputed_rent_share` | part of households' spending on `real_estate` paid back to owner-occupiers | ratio, ppm |
| — | `hh_imputed_rent_key[h]` | the household's `hh_dwelling_value` if `hh_tenure` is owner, else 0; fixed at load | LCU |

## First version (roadmap M3)
**Starting state.**
- **The share.** $\theta$ = (imputed rentals ÷ households' real-estate purchases in the table) × (L68A value added ÷ output) = (80.7 ÷ 92.1) × 0.954 = **836,500 ppm** (medium).
- **The key.** It comes from stage E's `hh_tenure` and stage H's `hh_dwelling_value`, which the start state must now carry. Owner households with no dwelling value get the owners' median, so that no owner is left out.

**In:**
1. **Spending, daily:** unchanged. Households buy `real_estate` by the basket; it is VAT-exempt.
2. **The payback, on the 15th, for the month before** (restated 2026-10-21 by the owner from pay day: paid on pay day it would change a pay day's income, which AC-ST-05 holds to the pay slips; it is still in the next decision's income):
   - **The amount.** $R = \theta \times$ households' spending on `real_estate` in the month (net, with weights).
   - **Who pays.** Each real-estate unit pays its share of $R$, in proportion to its sales in the month.
   - **How it reaches households.** Each owner household receives $R \times$ its key ÷ the weighted sum of keys, in whole bani per record, through a clearing account (as dividends, ADR-0016). The rounding remainder stays with the units.
   - **The money.** It moves as `deposits`, firms → households, under a new flow code **2013 `income.imputed_rent`** (proposed, append-only; 2010 is in use, and 2011 and 2012 are proposed by [government revenue at 2021 levels](government-revenue-2021.md)).
   - **Income.** It adds to `hh_income_received`, so it is in the next decision's income.
3. **The units' accounts.** The payback is deducted from the unit's profit before tax ([investment and capital](investment-capital.md), step 4) and from taxable profit ([taxation](taxation.md), step 8). Imputed rent is not taxed. It is not a cost in unit cost, so prices do not change.
4. **GDP:** the payback is a distribution of operating surplus. It changes no GDP measure, and I-7 holds.

**Expected amounts (a year, at the table's levels).**
- **Payback.** About 77.0 billion lei (6.4 billion a month), about 11,000 lei a year per owner household on average (7.0% of dwelling value).
- **The real-estate units' margins** (output 117.8 billion): surplus before the payback 89.0 billion (76% of output), after it about 13.1 billion (11%). That is close to L less L68A in Eurostat: 96.5 − 84.7 = 11.8 billion of value added.
- **Dividends abroad** of foreign-controlled real-estate units fall by about 29 billion a year.
- **Households' disposable income** rises by about 77 billion. This moves consumption toward the table's (households spend about 20% less than in 2021, roadmap 2026-10-21).

> **Simplification (M3):** the share $\theta$ is fixed. Tenants' actual rent stays with the units, and households under "other" tenure (5.8%, living rent-free) get nothing. The key is fixed at load until housing (M13) moves dwelling values.

**Not in M3:** rents set by a housing market, tenants and landlords as households, buying and selling dwellings (M13).

## Edge cases & failure modes
- **A month with no real-estate spending:** nothing is paid.
- **A unit whose deposits fall short of its share:** it borrows on its credit line, as for other payments ([money and banking](money-banking.md)).
- **No owner households** (a tiny test scenario): nothing is paid, and the amount stays with the units.

## Acceptance tests
IDs are draft IDs. If the text moves into households, they become AC-HH-11 to 14. Tests go in `crates/econ-core/tests/acceptance/` (protected).

- [ ] **AC-IMP-01** `[sim]` At every month end on the Romania scenario with the 2021 law:
  - the real-estate units' payments under `income.imputed_rent` equal θ times households' month's spending on `real_estate`, less the rounding remainder (under one ban per owner record);
  - every leu reaches an owner household and none reaches a tenant;
  - the clearing account nets to zero and every ledger check holds.
- [ ] **AC-IMP-02** `[sim]` Each owner household's receipt is within one ban per record of its key's share. Each household's `hh_income_received` grows by it.
- [ ] **AC-IMP-03** `[sim]` The real-estate units' profit and taxable profit exclude the payback. Their first-year surplus is within 30% of 13.1 billion lei, and prices of `real_estate` are those of a world without the payback on day 1.
- [ ] **AC-IMP-04** `[sim]` Compare two worlds on the Romania scenario with the 2021 law, with and without the payback (a start edit). Over the first twelve months, households' consumption over GDP is higher with the payback, and closer to the table's share.

### Tests owed
- AC-IMP-01: the test session of M3's revenue, once the owner places the text in households
- AC-IMP-02: the test session of M3's revenue
- AC-IMP-03: the test session of M3's revenue
- AC-IMP-04: the test session of M3's revenue

## API sketch
In the `World` (`econ_core::world`), with the pay phase:

```rust
pub fn imputed_rent_month(hh_real_estate_spending: Bani, share_ppm: u32) -> Bani;
pub fn imputed_rent_key(tenure: &[u8], dwelling_value: &[Bani], owner: u8) -> Vec<Bani>;   // median for owners at 0
```

## New glossary entries
- `imputed_rent_share`
- `hh_imputed_rent_key[h]`

## Open questions
- [x] **Key by dwelling value or equal per owner household?** Proposed: by dwelling value; imputed rent is proportional to the dwelling's rent. **Decided (owner, 2026-10-21):** as proposed.
- [x] **Households under "other" tenure** (5.8%, rent-free): include them in the key? Proposed: no in M3. ESA imputes their rent only for some countries. **Decided (owner, 2026-10-21):** as proposed.
- [x] **Paid on pay day** (month end, before households decide on the month's income), or on the 15th with dividends? Proposed: pay day, so it is in the same month's income as wages. **Decided (owner, 2026-10-21):** as proposed.
