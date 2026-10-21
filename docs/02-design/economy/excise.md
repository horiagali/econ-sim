---
id: economy/excise
title: Excise Duties
status: locked
owner: horia
depends_on: [economy/taxation, economy/vat, economy/prices-inflation, economy/production, economy/households, economy/fiscal-policy, economy/budget-2021, economy/industries, game/world-loop, adr/0007-money-and-ledger, adr/0017-time-base]
research: [research/government-revenue-romania-2021]
updated: 2026-10-21
---

# Excise Duties

## Purpose
Charge the duties per unit that Romania levies on motor fuels, tobacco and alcohol, and move them to the government through the ledger. In 2021 they raised 34.7 billion lei, 8.9% of general government revenue. [Taxation](taxation.md) lists excise under "Not in M3" (planned with energy, M12). The owner pulled it into M3 on 2026-10-21, because without it the `World` has no quiet baseline under the 2021 law (deficit about 18% of GDP against 7%). This spec is that first version; energy (M12) refines the energy part.

## Real-world basis
From the [research note](../../01-research/notes/government-revenue-romania-2021.md):
- **Excise is a specific duty per physical unit:** lei per 1,000 litres of fuel, per 1,000 cigarettes, per hectolitre and degree of alcohol. Cigarettes also carry an ad valorem part.
- **When it is due.** The duty falls due when the goods leave a tax warehouse or are imported, and the seller passes it on in the price.
- **VAT is charged on the price including the duty.** Firms that use fuel pay the duty like households (diesel for transport, farms, construction), but they deduct the VAT. Exports and some uses are exempt: jet fuel, chemical feedstock, refining's own use.
- **2021 revenue** (ESA, D214A + D2122C): **34,663.5 million lei**; cash receipts were 34,484.8 million. By group (Fiscal Council's shares, confidence medium):

| Group | Share | Million lei |
|---|---|---|
| Energy products (motor fuels, heating fuels, electricity, natural gas) | 50.1% | 17,366 |
| Tobacco products | 45.6% | 15,807 |
| Alcohol, beer, wine | ~4% | 1,387 |
| Other | ~0.3% | 104 |

- **Indexation.** The Fiscal Code indexes the specific amounts every year by the CPI. Indexation was suspended for 2020 and 2021 and resumed in January 2022 (medium confidence).

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $d_g$ | `excise_duty[g]` | duty on one real unit of good $g$, millionths of a ban (one real unit is what one ban bought at the starting price); 0 for a good with none | bani / real unit, ppm |
| — | `excise_firms_pay[g]` | whether firm units' purchases of $g$ as inputs pay the duty (fuel only in M3) | bool |
| — | `excise_exempt_buyer[j]` | industries whose input purchases are exempt (feedstock, jet fuel, the refinery itself) | bool |
| — | `excise_indexation` | whether duties follow the CPI each 1 January | bool |

Revenue has no state of its own: it is the government's receipts under `tax.excise`, read from the ledger.

## Inputs
- **Households' purchases of each good, each day.** Their gross budget, VAT and duty included ([households](households.md), the [World](../game/world-loop.md)'s goods market).
- **Firm units' input purchases of each good, each day** (home and imported) ([production](production.md)).
- **Prices:** the day's price $p_g$ and the VAT rate the `World` charges on the good.
- **The law:** duties, payers, exempt buyers, indexation. In M3 this is `Law::law_2021()`; from M4 it is `law.json` with its levers.

## Outputs
- One ledger posting a day: firms → government under `tax.excise`, by the three legs of the `World` (deposits → reserves → the government's deposits at the central bank).
- The consumer price of each excised good, $(p_g + d_g)(1 + v_g)$, which the CPI uses.
- The input price of each excised good to the industries that pay it, $p_g + d_g$, which unit cost uses.
- Revenue by group in the monthly fiscal report (AC-FIS-05).

## Update rule
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)). Money is `Bani`; duties are integer millionths ([ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md)).

**Duty on a purchase** of $q$ real units (bani at the starting price):

```math
e = \left\lfloor \frac{q \cdot d_g}{10^6} \right\rceil
```

$\lfloor\cdot\rceil$ rounds half away from zero to whole bani.

**Consumer price.** VAT is charged on the net amount plus the duty:

```math
\text{gross} = (n + e)\,(1 + v_g), \qquad n = \frac{q\,p_g}{10^6}
```

Here $v_g$ is the VAT the `World` charges: the legal rate less the 2021 VAT gap, as now.

**From a gross budget** $b$ (what households give to good $g$ on a day):
- **The quantity.** $q$ is the largest whole number of units with $(n + e)(1 + v_g) \le b$.
- **The rest.** The remainder stays with the households (less than one unit's gross price per good and day, as VAT's rule leaves at most one ban).

## Periods
| What | When |
|---|---|
| Duty on households' purchases and on firms' fuel inputs | daily, in the goods market (phase 8), with VAT |
| Indexation of duties | yearly, 1 January, in the wage-round phase (11), with the twelve-month CPI of the last monthly close |
| A change of a duty or of the indexation (M4) | first day of the following month (ADR-0018, kind `law`) |

## First version (roadmap M3)
What is built first, on the `World` of [game/world-loop](../game/world-loop.md), with the taxes of [taxation](taxation.md) and the treasury of [fiscal policy](fiscal-policy.md). It acts under `Law::law_2021()`. M2's placeholder law has no duties, so M2's results and hash do not change.

**Starting state.**

*The duties of the start.* An ad-valorem equivalent at the starting price, calibrated so that the table's quantities raise 2021's revenue by group:

  $$d_g = \frac{\text{2021 revenue of the group}}{\text{value at basic prices of the purchases that pay it}}$$

| Good (catalogue industry) | Who pays | Base, billion lei a year | Target, million lei | $d_g$, ppm | Confidence |
|---|---|---|---|---|---|
| `refined_petroleum` | households and firm units, except exempt buyers | 11.84 + 22.24 = 34.09 | 17,470 (energy products and other) | 512,500 | revenue high; base medium |
| `tobacco` | households | 7.79 | 15,807 | 2,030,400 | revenue high; base low (catalogue share of C10-12) |
| `beverages` | households | 19.46 | 1,387 | 71,200 | revenue medium; base low |

- **Bases.** They come from `io_table2021_ro.json`:
  - households' column, home and imported, by the catalogue's shares;
  - industries' purchases of C19, home and imported, less those of the exempt buyers: `refined_petroleum`, `chemicals`, `fertilisers` (feedstock) and `air_transport` (jet fuel).
- **Plausibility.** The tobacco duty (about twice the net price) is that of a pack in 2021: about 20 lei retail, of which about 11 lei excise and 3.2 lei VAT. The fuel duty is lower than at the pump (about 80% of the net price of petrol), because C19 also holds products with no duty (bitumen, lubricants, petroleum coke).

> **Simplification (M3):** electricity and natural gas carry no duty of their own. Their duties (about 0.5 billion lei, confidence low) and the "other" 0.1 billion are folded into the fuel duty's target, so the total is right. Energy (M12) gives `power_supply` and `gas_heat_supply` their duties per MWh.

> **Simplification (M3):** one duty per catalogue good. Petrol, diesel, LPG and heating oil are all `refined_petroleum`; beer, wine and spirits, and soft drinks with them, are all `beverages`. Collected revenue is the target, so the duty is net of evasion: there is no separate excise gap. Cigarette smuggling comes with the informal economy (M15).

*Households' basket.* At load, the basket share of each excised good is multiplied by $(1 + d_g)$ and the shares renormalised (largest remainder, millionths). This keeps the table's real demand for the excised goods relative to the others. The total gross budget is unchanged, so households buy about 3% less of the other goods: the duty is spent on the excised goods.

*Firms' unit cost.*
- **What changes.** Each industry's "other costs" ([prices](prices-inflation.md), step 1) lose the starting duty on its fuel inputs ($d \times$ its starting C19 purchases), which is now paid through the ledger. The fuel input enters unit cost at $p + d$ instead.
- **No starting jump.** Starting unit costs and prices do not change: every industry's product taxes on inputs in the table exceed its fuel duty (checked on the fixture, 2026-10-21).

**In:**
1. **Households' purchases, daily, in the goods market.**
   - **The split.** For each good, the day's gross household money is split as now into a home part and an imported part (`basket_import_ppm`). Each part buys the quantity of the "From a gross budget" rule.
   - **The money.** Households pay the gross amount to firms (`consumption`), as now. Of it:
     - the net value goes to the selling units (home) or to the rest of the world (`imports`);
     - the VAT on net plus duty goes to the government under `tax.vat`;
     - the duty goes to the government under `tax.excise`. The firms sector pays it, through the three legs, as VAT is paid.
2. **Firm units' fuel inputs, daily, in the goods market.** A unit whose industry is not exempt pays $e$ on each purchase of `refined_petroleum`, home or imported, from its deposits, to the government under `tax.excise` (three legs).
   - **The unit's columns.** Its `deposits` and the day's input cost both carry the duty.
   - **Profit.** The duty is a cost in [investment and capital](investment-capital.md)'s profit (step 4, $X_f$) and in taxable profit.
   - **Public producers** pay it too. The government covers it in their shortfall, at cost.
3. **Exports, government purchases and investment** pay no duty. The table has almost no fuel, tobacco or beverages in the last two (under 0.02 billion).
4. **Indexation, 1 January** (if `excise_indexation`):
   $$d_g \leftarrow \left\lfloor d_g\,(1 + \pi_{12}) \right\rceil$$
   - $\pi_{12}$ is the twelve-month CPI inflation at the last monthly close.
   - Deflation leaves the duty unchanged.
   - Without indexation the duty is fixed in lei, so its weight in the price falls as prices rise.
5. **Report.** The month's excise by good goes to the fiscal report (revenue by tax, AC-FIS-05) and to the month's statistics. GDP by expenditure counts households' gross spending, and GDP by income counts the duty with net taxes on products, so I-7 holds.

**Not in M3:**
- duties on electricity and gas (M12);
- several fuels and several kinds of alcohol;
- the ad valorem part of the cigarette duty;
- the legislated calendar of tobacco increases;
- excise evasion and smuggling (M15);
- payment a month after release (the duty is collected on the day, like VAT);
- a carbon tax ([environment](environment.md)).

**Worked example.**
- **Tobacco.** A household buys tobacco worth 100.00 lei net at the starting price (10,000 units).
  - Duty: 203.04 lei.
  - VAT at the legal 19%: 57.58 lei, on 303.04.
  - Gross: 360.62 lei. The duty is 56% of the retail price, as in 2021.
- **Fuel.** A road-freight unit buys 1,000.00 lei of fuel. It pays 512.50 lei of duty to the government. Its input cost is 1,512.50 lei, with no VAT.

**Flow code** (proposed, append-only): 3011 `tax.excise`. Taxation proposes 3009 and 3010 (`tax.dividend` is in use).

## Player levers
From M4 (none in M3):

| Lever | Range | Lag |
|---|---|---|
| `excise_duty[g]` for fuel, tobacco and alcohol, in lei per unit of the start | 0 to 5 times the starting duty | first day of the following month |
| `excise_indexation` | on, off | next 1 January |

The EU minimum duties (Directives 2003/96/EC, 2011/64/EU, 92/84/EEC) are a warning, like VAT's floor ([eu-membership](eu-membership.md), M18).

## Tuning parameters
| Parameter | Default | Confidence | Effect |
|---|---|---|---|
| `excise_duty[refined_petroleum]` | 512,500 ppm | medium | fuel price and transport costs |
| `excise_duty[tobacco]` | 2,030,400 ppm | medium (revenue high, base low) | tobacco price |
| `excise_duty[beverages]` | 71,200 ppm | low | beverage price |
| `excise_indexation` | on | medium (the law of 2022; 2021 suspended it) | whether the real duty holds |
| exempt buyers | `refined_petroleum`, `chemicals`, `fertilisers`, `air_transport` | medium | the fuel base |

The duties are data of the law (`Law::law_2021()`, then `law.json`). They are derived by the scenario's reference with `calibrate_duty_ppm`, and AC-EXC-05 checks them.

## Interactions
- **[VAT](vat.md):** the VAT base includes the duty. The VAT mechanic is unchanged: it is given $n + e$ as the net amount.
- **[Prices](prices-inflation.md):**
  - **Consumer price.** It is $(p_g + d_g)(1 + v_g)$; locked step 5 reads "consumer prices include VAT."
  - **Unit cost.** Fuel enters at $p + d$, and other costs lose the starting duty.
  - **Formula.** Prices' "Starting point" writes the consumer price as $p(1 + \tau^{VAT} + \tau^{excise})$, additive. This spec uses the multiplicative form, as the law does (open questions).
- **[Households](households.md):** the basket reweighting at load; the subsistence floor follows the CPI with the duty in it.
- **[Fiscal policy](fiscal-policy.md):** revenue in the monthly report.
- **[Energy](energy.md):** M12 replaces the fuel line with duties per fuel and adds electricity and gas.
- **[Informal economy](informal-economy.md):** smuggling (M15).

## Edge cases & failure modes
- **A duty of 0, a weight of 0, a purchase of 0:** nothing is posted.
- **Tiny purchases.** The duty rounds per purchase record. Totals are sums of rounded amounts.
- **The gross price of one unit exceeds the day's budget for the good:** nothing is bought; the money stays with the household.
- **Very high duties (lever at five times):** demand falls through the households' gross budget. With an `i128` intermediate, $q \cdot d$ cannot overflow: $q < 2^{43}$ and $d < 2^{32}$.
- **A unit at its credit limit** stops investment first ([investment and capital](investment-capital.md)). The duty is part of the input payment and is never skipped while the input is bought.
- **Deflation at the indexation:** the duty is unchanged.

## Acceptance tests
IDs are stable (added 2026-10-21).
- `[unit]` tests go in `crates/econ-mech-tax/tests/acceptance/excise.rs`.
- `[sim]` tests go in `crates/econ-core/tests/acceptance/excise.rs`.
Both directories are protected. The tests are written in a test-authoring session.

- [ ] **AC-EXC-01** `[unit]` The duty on $q$ units at duty $d$ is $q \cdot d / 10^6$ rounded half away from zero to whole bani. It is zero for a zero duty or quantity, and never negative.
- [ ] **AC-EXC-02** `[unit]` The worked example: 10,000 units of tobacco at price 1 and duty 2,030,400 ppm, with VAT at 19%, give net 100.00, duty 203.04, VAT 57.58 and gross 360.62 lei, to the ban. VAT is charged on the duty.
- [ ] **AC-EXC-03** `[unit]` From a gross budget, net + duty + VAT never exceeds the budget, and the rest is less than one unit's gross price. A higher duty never buys more units from the same budget.
- [ ] **AC-EXC-04** `[unit]` Indexation multiplies every duty by one plus the twelve-month inflation, rounded. With inflation of zero or below, or with indexation off, duties are unchanged.
- [ ] **AC-EXC-05** `[unit]` Calibration: on the committed `io_table2021_ro.json`, the starting duties times the starting bases give 2021's revenue by group within 1%: fuel 17.47, tobacco 15.81 and beverages 1.39 billion lei. `calibrate_duty_ppm` reproduces the defaults.
- [ ] **AC-EXC-06** `[sim]` On every day of a year on the Romania scenario with the 2021 law, the government's receipts under `tax.excise` equal the sum, with weights, of the duties on households' purchases and firm units' fuel purchases. No exempt industry and no export pays any, and every ledger check holds.
- [ ] **AC-EXC-07** `[sim]` At load, the starting price of every good is 1 and the starting unit cost of every industry is what it is without excise (the duty moved out of other costs). Households' first-day real demand for each excised good is within 1% of the table's daily quantity times the households' spending ratio of that day.
- [ ] **AC-EXC-08** `[sim]` The first twelve months' excise on the Romania scenario with the 2021 law is within 20% of 2021's 34.66 billion lei, reported by good. The tolerance is AC-TAX-09's: households spend about 20% less than in 2021.
- [ ] **AC-EXC-09** `[sim]` *(M4)* Doubling the fuel duty raises the consumer price of fuel at once by the duty's rise times one plus VAT. It raises the price of road freight within three months and lowers households' fuel quantity.
- [ ] **AC-EXC-10** `[sim]` *(M12)* Duties on electricity and gas raise the household price of electricity by the duty per MWh, and the fuel duty's target is restated without them.

### Tests owed
Ready for the test session of M3's revenue once the owner locks the spec (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-EXC-01: the test session of M3's revenue
- AC-EXC-02: the test session of M3's revenue
- AC-EXC-03: the test session of M3's revenue
- AC-EXC-04: the test session of M3's revenue
- AC-EXC-05: the test session of M3's revenue
- AC-EXC-06: the test session of M3's revenue
- AC-EXC-07: the test session of M3's revenue
- AC-EXC-08: the test session of M3's revenue
- AC-EXC-09: roadmap M4
- AC-EXC-10: roadmap M12

## API sketch
In `crates/econ-mech-tax`, module `excise`, next to VAT. Amounts in bani, duties in millionths of a ban per real unit, VAT in basis points.

```rust
pub struct ExciseDuty(pub u32);                                   // ppm of a ban per real unit
pub fn excise_on(quantity: i64, duty: ExciseDuty) -> Bani;        // AC-EXC-01
pub struct ExciseSplit { pub quantity: i64, pub net: Bani, pub excise: Bani, pub vat: Bani }
pub fn gross_of(quantity: i64, price_ppm: u32, duty: ExciseDuty, vat_bp: u32) -> ExciseSplit;   // AC-EXC-02
pub fn split_budget(budget: Bani, price_ppm: u32, duty: ExciseDuty, vat_bp: u32) -> ExciseSplit; // AC-EXC-03
pub fn index_duty(duty: ExciseDuty, inflation_ppm: i32) -> ExciseDuty;                           // AC-EXC-04
pub fn calibrate_duty_ppm(target: Bani, base_value: Bani) -> ExciseDuty;                         // AC-EXC-05
pub struct ExciseSchedule {
    pub duty: Vec<ExciseDuty>,          // per catalogue good; 0 = none
    pub firms_pay: Vec<bool>,           // per good
    pub exempt_buyer: Vec<bool>,        // per buying industry
    pub indexation: bool,
}
pub fn basket_with_excise(basket_ppm: &[u32], duty: &[ExciseDuty]) -> Vec<u32>;                  // largest remainder
```
The goods-market postings and the reweighting at load live with the `World`.

## New glossary entries
- `excise_duty[g]`
- `excise_firms_pay[g]`
- `excise_exempt_buyer[j]`
- `excise_indexation`

## Open questions
- [x] **Firms pay the fuel duty (step 2), or households only?** Proposed: firms too. About two thirds of the fuel base is firms' (22.2 of 34.1 billion). Charging it all to households would need a duty of about 150% of their net price, a fuel price far above 2021's. The duty is already in firms' unit costs (the table's product taxes), so posting it changes no starting price. **Decided (owner, 2026-10-21):** as proposed.
- [x] **Consumer price multiplicative, VAT on the duty.** Proposed: yes, as the law does. Without it VAT loses about 2.8 billion lei a year. Prices (locked) writes the additive form in its "Starting point"; restating that line is the owner's call. **Decided (owner, 2026-10-21):** as proposed.
- [x] **Indexation on by default?** Proposed: on, the law from 2022. The scenario starts in December 2021, and the first indexation (1 January 2022) applies. Alternative: off, as in 2020–2021, so the real duty falls with inflation. **Decided (owner, 2026-10-21):** as proposed.
- [x] **The basket reweighting** keeps real demand for the excised goods and shifts about 3% of the household budget away from the rest. VAT is still applied to net shares (its gross budget buys less of every good than the table). Proposed: excise only in M3; whether the whole basket should be at purchasers' prices goes to the calibration under the 2021 law. **Decided (owner, 2026-10-21):** as proposed.
- [x] **The rest of the revenue gap** ([research note](../../01-research/notes/government-revenue-romania-2021.md), "Implications"). After excise and EU funds the `World` collects about 254 of 2021's 389 billion lei. Of the rest, about 36 billion are classification: public units' sales are netted and imputed contributions are not cash. Proposed minimal M3 treatment, in order: **Decided (owner, 2026-10-21):** as proposed.
  1. Post the product taxes already in firms' unit costs and on investment (non-deductible VAT and other taxes on products, about 48.5 billion) under a new code `tax.products` (3012): VAT's "Later" item, simplest form.
  2. Post other taxes on production (D29, 8.1 billion at the table's level) by industry from the table's row, under a new code `tax.production` (3013).
  3. Hold the small lines at their 2021 level, a twelfth a month: households' property and vehicle taxes (equal per owner household), gambling, royalties, interest, fines and other transfers; about 21 billion. Drafted, with the budget's unpaid cash benefits, in [government revenue at 2021 levels](government-revenue-2021.md).
  4. Leave the rest to the calibration under the 2021 law.

  Each item is the owner's decision; items 1 and 2 touch locked specs (VAT, prices).
