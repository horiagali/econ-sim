---
id: economy/government-revenue-2021
title: Government Revenue and Other Benefits at 2021 Levels
status: locked
owner: horia
depends_on: [economy/vat, economy/taxation, economy/excise, economy/eu-funds, economy/fiscal-policy, economy/budget-2021, economy/social-transfers, economy/prices-inflation, economy/investment-capital, game/world-loop, adr/0007-money-and-ledger, adr/0017-time-base]
research: [research/government-revenue-romania-2021]
updated: 2026-10-22
---

# Government Revenue and Other Benefits at 2021 Levels

## Purpose
Close what is left of the gap between the `World`'s budget and 2021's, after [excise](excise.md) and [EU funds](eu-funds.md):
1. **Product taxes already in firms' prices,** posted to the government when firms buy inputs and investment goods. Proposed as a dated M3 addition to [VAT](vat.md), which is locked; the text is here until the owner places it.
2. **Other taxes on production** (D29) by industry.
3. **Small revenue lines** held at their 2021 level.
4. **The budget's cash social benefits** that no mechanism pays yet, held at their 2021 level.

It adds no behaviour: each part either moves money that prices already assume, or holds a 2021 line, as [fiscal policy](fiscal-policy.md) holds the budget lines the `World` does not pay. Every amount comes from the [research note](../../01-research/notes/government-revenue-romania-2021.md) and the fixtures.

## Data
- **Where the data lives.** Three new sections of the budget fixture `python/pipeline/fixtures/budget2021_ro.json`, copied by the scenario builder into the scenario's `budget.json`, and read by `econ_io::budget` as optional fields of `Budget`:
  - `eu_receipts` ([EU funds](eu-funds.md));
  - `revenue_held` (part 3), each line with its ESA code, lei a year, payer and flow code;
  - `benefits_held` (part 4), each programme's held cash amount and key.
- **What does not change.** The budget's hash (AC-BUD-11) covers spending only.
- **Rates from the table.** The per-industry rates of parts 1 and 2 are computed at load from `io_table2021_ro.json`, as the input coefficients are.

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| Product taxes on input purchases | daily, with the purchase (goods market, phase 8) |
| Product taxes on investment purchases | daily, with the purchase |
| Other taxes on production | month end, on the month's output |
| Held revenue lines | month end, a twelfth, before the treasury's financing |
| Held benefits | pay day (month end), with the other benefits |

## First version (roadmap M3)
Under `Law::law_2021()` only; M2's placeholder law has none of it.

### 1. Product taxes on firms' purchases (proposed M3 addition to VAT)
- **What it is.** The table's product taxes on industries' inputs are 46.1 billion lei. They are mostly the VAT that exempt industries cannot deduct, and the other taxes on products (the clawback on medicines, stamp duties). They are in each industry's "other costs" ([prices](prices-inflation.md), step 1), so prices carry them. Today no one pays them, and firms keep them as profit.
- **The rate.** Each buying industry $j$ gets $t_j$ = (its product taxes on inputs − the starting fuel duty it now pays as [excise](excise.md)) ÷ (its input purchases, home and imported), in ppm, at load.
- **On inputs, daily.** A firm unit pays $t_j \times$ the value of each day's input purchases, home and imported, from its deposits, to the government (three legs).
- **On investment, daily.** A firm unit pays $t_I$ = 16.45 ÷ (274.2 + 22.8) = **55,400 ppm** on its investment purchases (the table's product taxes on investment, over investment and inventories).
  - The government's own investment pays none: it would pay itself.
  - Government purchases carry none in the table.
- **Code.** New flow code **3012 `tax.products`**, recommended over `tax.vat`. It keeps `tax.vat` what households pay (comparable with VAT's own criteria), while the report compares `tax.vat` + `tax.products` + `tax.excise` with D21.
- **Unit cost.** It does not change: its other-cost term keeps its starting value. Investment budgets are gross, so the tax is inside the unit's budget; while investment is at fixed real levels, firms pay it on top.

**Expected amounts (a year, at the table's levels).**
- **Inputs: 34.7 billion lei** (mean rate 3.1%). Highest: hotels and restaurants (I) 9.4%, membership organisations 6.5%, public administration 5.8%, health 5.3%. Lowest: basic metals 1.0%, insurance 1.6%, energy 1.6%. Public units' share comes back to them in the government's payment at cost.
- **Investment: about 13.8 billion lei** (5.54% of firms' 249 billion).
- **Total about 48.5 billion lei.** With households' VAT (about 40 billion in the `World`, VAT on excise about 2.8) and excise, D21 would be within about 10% of 2021's 117.1 billion.

### 2. Other taxes on production (D29)
- **The rate.** Each industry with a positive "other taxes less subsidies" row in the table gets $o_j$ = that row ÷ its output (ppm).
- **The payment.** At month end each of its units pays $o_j \times$ the month's output value to the government under **3013 `tax.production`**.
- **Prices.** The amount is already in unit cost, so prices do not change.
- **Expected.** **8.10 billion lei a year**, against ESA's 9.68; the difference stays out. Largest: telecom (J61) 1.15, energy (D35) 0.90, mining (B) 0.86, retail (G47) 0.83.
- **Negative rows** (agriculture −12.2, public administration −0.7, warehousing −0.2) are subsidies, paid by [EU funds](eu-funds.md) and the treasury's `subsidy.production`, not here.

### 3. Small revenue lines held at 2021 levels
At month end, a twelfth of each line by [budget-2021](budget-2021.md)'s monthly rule. Payments into the government go by the three legs (`pay_gov_any`).

| Line (ESA) | Million lei a year | Payer and key | Flow code |
|---|---|---|---|
| Households' property and vehicle taxes (D59) | 3,864.1 | owner households (`hh_tenure` = owner), equal per household: 45.95 lei a month | 3014 `tax.property` |
| Tax on gambling winnings (D51D) | 2,376.3 | households, by last month's consumption | 3015 `tax.gambling` |
| Rent and royalties (D45) | 2,619.6 | units of `crude_oil`, `natural_gas`, `coal_mining`, `metal_ores`, `quarrying`, by starting output | 2011 `rent.natural_resources` |
| Interest received (D41) | 1,287.1 | banks, from reserves | 2012 `interest.gov_received` |
| Other current transfers received: fines, penalties, other (D75, D72) | 6,418.7 | half households (equal per household), half firm units (by starting value added) | 4107 `transfer.other_received` |
| Other capital transfers received (D9 not from the EU) | 4,857.2 | firm units, by starting value added | 4108 `transfer.capital_received` |
| **Total** | **21,423.0** | | |

- **Payer short of money.** A household that cannot pay its share pays what it has; the rest is forgiven and reported. A unit borrows, as for other payments.
- **Not held separately.** The other taxes on products (6.1 billion) are inside part 1.
- **Classification only.** Sales of public units (26.5 billion) and imputed contributions (9.6) are reported, not posted (research note).

> **Simplification (M3):** fixed nominal amounts, as the budget's held spending lines. Property tax is equal per owner household until housing (M13) gives it a base and a rate; the other lines get mechanisms when their topic comes (gambling never, royalties with energy M12).

### 4. Other cash social benefits held at 2021 levels
- **What the `World` already pays** (first year, 2026-10-20):
  - pensions, 113.4 billion, which cover old-age and survivors (112.3);
  - the child allowance, 11.2;
  - the unemployment benefit, 4.2 (2021: 0.56; it follows the `World`'s unemployment);
  - the minimum income, 0.5.

  In all 129.3 billion of the budget's 148.2 billion of cash benefits (D62).
- **The rest.** On each 15th (restated by the owner from pay day, 2026-10-22, as for AC-ST-05: paid on pay day it would change a pay day's income, which AC-ST-05 holds to the pay slips), a twelfth of each programme below is paid from the government to households (three legs). It goes into `hh_income_received`, under the programme's code from [budget-2021](budget-2021.md), split equally over the key's persons or households (with weights, whole bani per record).
- **Residuals.** "Residual" lines are the programme's cash less 12 times the `World`'s own payment in the starting month, fixed at load.

| Programme | Million lei a year | Key | Flow code | Confidence of the key |
|---|---|---|---|---|
| `sickness_disability`, disability part (70%) | 7,138.8 | persons with `inactive_kind` = disabled | 4002 `benefit.disability` | low (split guessed) |
| `sickness_disability`, sickness part (30%) | 3,059.5 | employees on the day of payment (persons with a declared wage on the 15th, not at load) | 4002 `benefit.disability` | low |
| `family_children`, residual (child-raising leave and other) | ≈7,736 (18,936.2 − ≈11,200) | households with a child under 2 | 4003 `benefit.family` | medium |
| `social_exclusion`, residual | ≈1,107 (1,607.2 − ≈500) | households in the lowest income fifth | 4006 `benefit.social_assistance` | low |
| `housing` | 68.7 | households in the lowest income fifth | 4005 `benefit.housing` | low |
| `social_protection_other` | 1,520.6 | persons aged 65 or over | 4008 `benefit.other` | low |
| `education_grants` | 2,926.3 | persons `enrolled` (primary to doctorate) | 4007 `benefit.student` | medium |
| **Total** | **≈23,557** | | | |

With them the `World` pays about 152.8 billion of cash benefits: 2021's 148.2, plus the unemployment benefit's excess.

> **Simplification (M3):** flat amounts per member of the key group, fixed in lei. The real rules come with the levers (disability benefit, student finance: M4, M14). The start state must carry `inactive_kind` and `enrolled` (stage of [inactivity and enrolment](../society/population-inactivity-enrolment.md)) and persons' ages.

**Not in M3:** property tax by dwelling value (M13), royalties by price and volume (M12), benefits by their rules (M4, M14).

## Acceptance tests
Draft IDs (2026-10-21). Tests go in `crates/econ-core/tests/acceptance/revenue.rs` (protected). If part 1 moves into VAT, AC-GRV-01 to 03 become AC-VAT-07 to 09.

- [ ] **AC-GRV-01** `[unit]` The rates at load: for every industry, $t_j$ times its starting input purchases plus its starting fuel duty equals its product taxes on inputs in the table, within one ban per million. Over all industries this is 34.67 billion lei, and $t_I$ times the table's investment and inventories is 16.45 billion.
- [ ] **AC-GRV-02** `[sim]` Every day on the Romania scenario with the 2021 law:
  - the receipts under `tax.products` equal the sum of $t_j$ times each unit's day's input purchases plus $t_I$ times its investment purchases;
  - the government pays none on its own purchases or investment;
  - every ledger check holds.
- [ ] **AC-GRV-03** `[sim]` With parts 1 and 2 on, every industry's starting unit cost and price are those of the `World` without them. After twelve months, the fall in firm units' deposits plus the fall in the dividends they paid plus the fall in the profit tax they paid (`tax.profit` and `tax.micro`) equals the taxes paid (`tax.products` and `tax.production`), all summed over each day's flows, within 10%. *(Restated by the owner, 2026-10-22: the taxes lower profit, so firms also pay less in dividends and in profit tax; the deposits alone fall by the taxes less those changes. The tolerance is 10% because the two worlds' paths differ in other ways too, about 8% in the first run.)*
- [ ] **AC-GRV-04** `[sim]` At every month end, `tax.production` equals the sum of $o_j$ times each unit's month's output value. The first year is within 20% of 8.10 billion lei.
- [ ] **AC-GRV-05** `[sim]` Each held revenue line is paid every month end by its payers only, under its code. Twelve months add to its 2021 amount exactly, less any household shortfall, which is reported. Every owner household pays the same property tax and no tenant pays any.
- [ ] **AC-GRV-06** `[sim]` On every 15th (the held benefits' payment day) each held benefit pays exactly its monthly amount, less rounding under one ban per record, to its key group only. Recipients' `hh_income_received` grows by it. Twelve months add to the programme's held amount. The keys are read on the day of payment: the sickness part's employees are those employed on the 15th, not those at load. *(Restated by the owner, 2026-10-22: the held benefits are paid on the 15th, not on pay day, see part 4; and the sickness key is the day's employees.)*
- [ ] **AC-GRV-07** `[sim]` The first year's fiscal report (AC-FIS-05) lists revenue by ESA line next to the research note's 2021 figures, and total revenue against TR less public units' sales and imputed contributions (352.8 billion lei). With excise and EU funds on, the total is within 15% (a check, not a target, as AC-TAX-09).

AC-GRV-01 to 07 were written on 2026-10-21, after the code (owner's decision of 2026-10-21), from this spec only:
- **Where.** They are in `crates/econ-core/tests/acceptance/revenue.rs`, behind the feature `revenue`; AC-GRV-06 also needs `pay_day`.
- **What they read:** the starting state's rates, plus four test writer's additions, which the implementation fills: the day's purchases and taxes (`World::revenue_today`), the closed months of revenue (`World::revenue_months`, with the held lines and each household's property tax), the held benefits of the latest 15th (`World::held_benefits`) and each industry's unit cost (`World::unit_costs_ppm`). They also use a world without parts 1 and 2 (`StartEdit::WithoutRevenue(RevenuePart::ProductTaxes)`).
- **Amounts.** The held amounts are this spec's tables, because the budget fixture has no `revenue_held` or `benefits_held` section yet.

### Tests owed
Ready for the test session of M3's revenue once the owner locks the spec (ADR-0014, Amendment 1). A line is removed when its test is written.

## API sketch
In the `World` (`econ_core::world::treasury` and the goods market). The pure parts:

```rust
pub fn product_tax_rate_ppm(product_taxes: Bani, fuel_duty: Bani, purchases: Bani) -> u32;   // AC-GRV-01
pub fn production_tax_rate_ppm(other_taxes: Bani, output: Bani) -> u32;                        // 0 when negative
pub struct HeldLine { pub esa: String, pub per_year: Bani, pub code: FlowCode, pub key: HeldKey }
pub enum HeldKey { OwnerHouseholds, HouseholdsByConsumption, Units(Vec<u16> /* industries */), Banks,
                   HouseholdsAndUnits, UnitsByValueAdded, Persons(PersonFilter), LowestFifth, ChildUnder2 }
```

## New flow codes
Proposed, append-only; none used on main as of 2026-10-21:

| Code | Name | For |
|---|---|---|
| 2011 | `rent.natural_resources` | royalties and rents received by the government (D45) |
| 2012 | `interest.gov_received` | interest received by the government (D41) |
| 3012 | `tax.products` | taxes on products paid by firms on inputs and investment (non-deductible VAT and other D21) |
| 3013 | `tax.production` | other taxes on production (D29) |
| 3014 | `tax.property` | households' property and vehicle taxes (D59) |
| 3015 | `tax.gambling` | tax on gambling winnings (D51D) |
| 4107 | `transfer.other_received` | other current transfers received by the government (D75) |
| 4108 | `transfer.capital_received` | capital transfers received from residents (D9) |

The held benefits use 4002, 4005 and 4007, proposed by [budget-2021](budget-2021.md) and not yet in `codes.rs`, and 4003, 4006, 4008, which exist.

## Open questions
- [x] **One code `tax.products` or `tax.vat`** for part 1? Proposed: `tax.products`, so households' VAT stays comparable and D21's other taxes are not called VAT. **Decided (owner, 2026-10-21):** as proposed.
- [x] **Part 1 in VAT's spec** (locked) as a dated M3 addition, with AC-GRV-01 to 03 becoming AC-VAT-07 to 09? Proposed: yes, when the owner confirms. **Decided (owner, 2026-10-21):** as proposed.
- [x] **The 70/30 split** of sickness and disability benefits. Proposed: hold it until the Ministry of Labour's 2021 figures (disability pensions, handicap allowances, sick leave) are collected. **Decided (owner, 2026-10-21):** as proposed.
- [x] **D29 at the table's 8.10 or ESA's 9.68 billion?** Proposed: the table's, so that prices and payments agree. **Decided (owner, 2026-10-21):** as proposed.
