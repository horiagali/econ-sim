---
id: economy/trade-fx
title: Trade & Exchange Rate
status: locked
owner: horia
depends_on: [economy/production, economy/prices-inflation, economy/money-banking, economy/monetary-policy, economy/opening-balance-sheet, game/world-loop, adr/0017-time-base]
research: []
updated: 2026-11-01
---

# Trade & Exchange Rate

## Purpose
Connect the country to the rest of the world: per-good exports and imports, tariffs, the exchange rate, capital flows and foreign reserves. Devaluation helps exporters and hurts importers. Interest rates move the currency.

## Real-world basis
- **Armington:** domestic and foreign varieties of a good are imperfect substitutes; import share depends on relative price.
- Export demand depends on foreign income and the relative price in foreign currency; trade elasticities ~0.5–1.5; **J-curve** (trade balance worsens before improving after depreciation).
- Exchange rates respond to interest differentials, risk and the current account (uncovered interest parity with a risk premium, plus BoP flow pressure).
- Exchange-rate pass-through to import prices is high, to CPI lower.
- **Romania, 2021** (input-output table; the exchange rate from memory, high confidence): exports 425 billion lei, imports 483 billion; the BNR ran a tightly managed float, EUR/RON 4.92 to 4.95 through 2021 (the table's 4.9215 is its average). Its foreign assets in the opening balance sheet are 228.5 billion lei.

## Rest of world: two partner blocs (exogenous, data-driven)
- **EU** (single market; euro as currency) and **non-EU** (everyone else; USD as reference currency). Each has, per good: price, demand index; plus interest rate, inflation, growth.
- Two exchange rates: **EUR/RON** (the one that matters most) and **USD/RON**; the EUR/USD cross rate is exogenous.
- **Trade rules come from [EU membership](eu-membership.md):** no tariffs on EU trade; the EU common external tariff on non-EU imports; no capital controls. Schengen status adds or removes a border cost on EU road trade.
- **Currency regime** (float, ERM II, euro, post-exit currency) is set by [euro](euro.md). In the euro, EUR/RON no longer exists and only EUR/USD (exogenous) matters.
- Shock scenarios (oil price spike, EU recession, global recession) are scripted changes to these exogenous paths, which is allowed by pillar 6.

## Update rule
**Imports** of good $g$ (for any domestic use), with the import share split between EU and non-EU varieties by a second Armington nest:
```math
\frac{M_g}{D_g} = \frac{\alpha_g (p^w_g e (1+\tau_g))^{1-\sigma_g}}{\alpha_g (p^w_g e (1+\tau_g))^{1-\sigma_g} + (1-\alpha_g) p_g^{1-\sigma_g}}
```
with smoothing (contracts adjust over ticks).

**Exports** of good $g$ (to each bloc $b$, with its own demand, price and exchange rate):
```math
X_g = \bar X_g \cdot \Big(\frac{p_g / e \cdot (1 - s^{X}_g)}{p^w_g}\Big)^{-\epsilon_g} \cdot (\text{world demand}_g)
```
limited by capacity and supply.

**Exchange rate** (managed float, the BNR's actual regime), RON per EUR:
```math
\frac{\Delta e}{e} = -\gamma_1 \big[(i - \pi^e) - (i^* - \pi^{*e}) - \text{rp}\big] - \gamma_2 \frac{CA + \text{capital flows}}{GDP} + \text{FX intervention effect}
```
with the risk premium rp rising with debt, inflation, political instability (later) and capital controls. Under a **peg**, the CB must intervene to hold $e$; reserves deplete if the peg is overvalued → possible forced devaluation (emergent currency crisis).

**Balance of payments:** current account (goods, services incl. tourism, primary income incl. dividends of foreign-owned firms ([foreign-ownership](foreign-ownership.md)) and interest, secondary income incl. remittances ([demographics](demographics.md)) and EU current transfers) + capital account (EU capital transfers, [eu-funds](eu-funds.md)) + financial account = −Δ reserves. Booked SFC.

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| Import and export purchases, at the day's prices in lei | daily, as in the `World` |
| The exchange rate | monthly close |
| Import shares and export volumes adjust to relative prices | monthly close, for the next month |
| The central bank's intervention | monthly close, with the rate |

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $e$ | `exchange_rate` | lei per euro (the dollar follows at a constant cross rate) | ratio |
| — | `import_share[g]` | imported part of the uses of good $g$, by use (inputs, households, government, investment) | ratio |
| $\bar X_g$ | `export_base[g]` | exports of the table at starting relative prices | real units / day |
| — | `world.*` | the outside world's constants (decision D8): price of each good in euro, foreign policy rate and expected inflation, demand index, EUR/USD | table |

## First version (roadmap M3)
What is built first, on the `World` of [game/world-loop](../game/world-loop.md). It replaces M2's imports at the fixed starting price and exports at fixed real levels.

**The outside world** (decision D8) is a table of constants in the scenario (`world.json`), each changeable later by a world-settings command (M4): the euro price of every good (1 at the start), the foreign policy rate (ECB: −0.5%), foreign expected inflation (2%), a demand index per bloc (1), EUR/USD (1.13). Until `world.json` exists the `World` holds these constants.

**In:**
1. **Import prices** in lei are the euro price times $e$ over its starting value. Imported inputs, the imported part of household purchases and the imported parts of government purchases and investment pay that price instead of 1.
2. **Import shares, monthly** (Armington): each use's import share of good $g$ moves toward
   $$s_g = \frac{s^0_g\, (p^M_g)^{1-\sigma}}{s^0_g\, (p^M_g)^{1-\sigma} + (1-s^0_g)\, p_g^{1-\sigma}}$$
   by `share_adjustment` a month, where $s^0_g$ is the starting share, $p^M_g$ the import price and $p_g$ the domestic price (both 1 at the start).
3. **Exports, monthly:** each day's export demand on good $g$ is $\bar X_g \cdot \big(\tfrac{p_g}{e/e_0 \cdot p^*_g}\big)^{-\epsilon} \cdot \text{demand index}$; the rest of the world pays in lei as in M2.
4. **The exchange rate, monthly close:**
   $$\frac{\Delta e}{e} = -\gamma_1\big[(i - \pi^e) - (i^* - \pi^{*e}) - \text{rp}\big] - \gamma_2\,\frac{CA}{GDP} + \text{intervention}$$
   with $CA$ the month's current account (exports less imports, less primary income paid abroad, from the ledger) over the month's GDP, and $\text{rp}$ the risk premium: the real-rate gap of the start, so that with the starting rates the exchange rate does not move (with the policy rate at 1.75%, expected inflation at the 2.5% target, the euro area's −0.5% and 2%: 1.75 points). **Managed float:** the central bank offsets `intervention_share` of the move by selling or buying foreign assets against the rest of the world's lei deposits (flow code `finance.fx_intervention`), as long as its foreign assets stay above `reserve_floor` months of imports.
5. **Settlement.** The rest of the world keeps paying and being paid in its lei deposits at Romanian banks (M2's placeholder); a deficit of the current account is the growth of those deposits, and the central bank's intervention converts them against its foreign assets.

> **Simplification (M3):** one foreign bloc for prices (the euro area) with the dollar at a constant cross rate; no tariffs (EU single market; the common external tariff on non-EU goods is small and constant); no capital account beyond the rest of the world's deposits and the central bank's intervention; no J-curve beyond what the monthly adjustment gives.

**Not in M3:** two blocs with their own demand (when D8 becomes dynamic), pegs and ERM II ([euro](euro.md), M20), tariffs and capital controls (EU rules, M18), remittances (M5), dividends of foreign-owned firms (with investment and capital, M3, owner of that spec).

## Amendment (owner, 2026-10-25): one-off devaluation
Asked by the owner for the policy-scenario suite ([policy-scenarios](../game/policy-scenarios.md), S6). A `market` lever `fx.devaluation` (glossary `fx_devaluation`), a signed rate in basis points: on its effective day the exchange rate steps once,
$$e \leftarrow e \cdot (1 + d)$$
with $d$ the command's rate (1000 = +10%, the leu 10% weaker; a negative rate revalues). Import prices in lei and the day's export demand follow the same day; $e_0$ is unchanged. The managed float of step 4 then continues from the new level: the intervention offsets only later moves, never the step. Each command that takes effect is one step; the value in force records the latest command and is not applied again. Its starting value is 0.

> **Simplification (M4):** a one-off step, not a regime. A peg at a chosen rate comes with the euro and ERM II ([euro](euro.md), M18/M20).

**Oil (owner's decision of the same day, game/lever-system).** `world.fuel_price[oil]` scales the euro price of `crude_oil` and `refined_petroleum` by its ratio to the value in force before it, from its effective day. Imported inputs pay a price per using industry: its oil share of imported inputs (from the input-output table: the output shares of the two oil industries in each imported product, and refining's imported mining products counted whole as crude) at oil's import price, the rest at the index of the other goods. Gas and coal wait for [energy](energy.md) (M12).

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `armington_sigma` $\sigma$ | 1.5 | how far import shares move with relative prices |
| `share_adjustment` | 0.1 a month | how fast they move |
| `export_elasticity` $\epsilon$ | 1.0 | export volume per relative price |
| $\gamma_1$ | 0.005 a month per point of real-rate gap | interest parity pull (0.02 until 2026-11-01; see Calibration) |
| $\gamma_2$ | 0.05 a month per unit of CA/GDP | flow pressure |
| `risk_premium` rp | the starting real-rate gap (1.75 points) | Romania's premium over the euro area; a constant of 3 points made the rate drift about 1% a month at the start (restated 2026-10-21) |
| `intervention_share` | 0.8 | share of a move the BNR offsets (a tightly managed float) |
| `reserve_floor` | 3 months of imports | below it the BNR stops selling |

## Player levers
FX regime (float, managed, peg), FX intervention, export promotion. **Tariffs and capital controls are locked while Romania is in the EU** ([eu-membership](eu-membership.md)); export subsidies count as state aid.

## Outputs
Exports and imports by good, trade balance, current account, terms of trade, exchange rate (nominal and real effective), reserves, import price index.

## Worked example: devaluation
10% depreciation → export prices in foreign currency −10% → export volumes ↑ over 3–12 ticks (car makers, chemicals, tourism) → those industries hire. Import prices +10% in LCU immediately → CPI ↑ (energy, electronics, car parts) → real wages ↓ for import-heavy baskets. Short-run trade balance may worsen (J-curve), then improve.

## Acceptance tests
IDs are stable (added 2026-10-20). Tests go in `crates/econ-core/tests/acceptance/` (protected).

- [ ] **AC-TFX-01** `[unit]` The Armington share is the formula of step 2: at equal prices it is the starting share, and it rises when the domestic good becomes dearer.
- [ ] **AC-TFX-02** `[unit]` The exchange-rate step is the formula of step 4, with the intervention offsetting its share of the move while reserves are above the floor and none below it.
- [ ] **AC-TFX-03** `[sim]` The balance of payments closes every month end: current and capital account (exports less imports less primary income paid abroad, the coupons on government bonds included, plus the EU's grants of [eu-funds](eu-funds.md); the month report keeps the sum in `current_account` and the capital grants in a `capital_account` memo) + the change in the rest of the world's lei deposits + its purchases of government bonds less redemptions − the change of the central bank's foreign assets (its intervention sells them; its yield, [monetary policy](monetary-policy.md), adds to them) = 0, to the ban. Restated 2026-10-21 to include the bonds, and the same day (owner) the capital account; its signs corrected 2026-10-20 to the identity the ledger keeps (the first restatement wrote it as an equality with the wrong signs).
- [ ] **AC-TFX-04** `[sim]` A permanent 10% depreciation (a world-constants edit) raises import prices at once, export volumes over the following months and CPI by less than 10% (restated, owner 2026-10-29: both runs hold the policy rate fixed). With the central bank following the advisory rule by default ([monetary policy](monetary-policy.md), owner 2026-10-29), the rule raised the rate in the depreciated world and its month-8 export volume fell below the baseline's (3,726.4 bn against 3,734.6 bn); both runs now keep the policy rate at its starting value (the rule at pure smoothing, rho = 1), so the test isolates the exchange rate's effect. The checks are unchanged.
- [ ] **AC-TFX-05** `[sim]` Quiet baseline: on the Romania scenario with the 2021 law and constant world, the exchange rate over 120 months stays within 10% of its start and the central bank's reserves above their floor.
- [ ] **AC-TFX-06** `[sim]` *(M4)* A 2-point rise in the domestic policy rate appreciates the currency.
- [ ] **AC-TFX-07** `[sim]` *(M20)* A peg defended with insufficient reserves eventually breaks.
- [ ] **AC-TFX-08** `[sim]` *(M18)* A change in the EU common external tariff on a good raises its domestic price and lowers non-EU imports.
- [ ] **AC-TFX-09** `[sim]` *(later, two blocs)* An EU recession lowers exports more than a non-EU recession of the same size.

AC-TFX-01 and 02 (`[unit]`) were written on 2026-10-20 in the mechanic's crate, `crates/econ-mech-trade/tests/acceptance/`, behind the crate feature `mechanic`; AC-TFX-03 to 05 in `crates/econ-core/tests/acceptance/trade.rs`, behind the crate feature `trade`, against the outside world's read-only state (`World::foreign`), the monthly rest-of-world report (`World::trade_months`), trade's parameters (`WorldParams::trade`) and a world-constants edit (`StartEdit::WorldPrices`). AC-TFX-04's depreciation is every euro price times 1.1: for import prices in lei and export demand it is the same as e times 1.1, and the euro prices are the world's constants.

### Tests owed
The spec was locked before these tests could exist (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-TFX-05: waits for M7's calibration (the leu drifts about 10.3% over ten years against the 10% limit)
- AC-TFX-06: roadmap M4
- AC-TFX-07: roadmap M20
- AC-TFX-08: roadmap M18
- AC-TFX-09: after v1

## API sketch
```rust
// crate econ-mech-trade (no I/O); prices, shares and rates in millionths
pub fn armington_share_ppm(start_share_ppm: u32, import_price_ppm: u32, domestic_price_ppm: u32, sigma_ppm: u32) -> u32;   // AC-TFX-01
pub fn export_demand(base: i64, domestic_price_ppm: u32, rate_index_ppm: u32, world_price_ppm: u32,
                     demand_index_ppm: u32, elasticity_ppm: u32) -> i64;
pub struct FxParams { pub gamma1_ppm: u32, pub gamma2_ppm: u32, pub risk_premium_ppm: i32, pub intervention_ppm: u32 }
pub struct FxStep { pub rate_ppm: u32, pub intervention: Bani /* > 0: the BNR sells foreign assets */ }
pub fn fx_step(rate_ppm: u32, real_gap_ppm: i32, ca_over_gdp_ppm: i32, reserves_above_floor: bool,
               gdp_month: Bani, p: &FxParams) -> FxStep;                                                                 // AC-TFX-02
```
Powers through `econ_num::math` (ADR-0006). The monthly close and the settlement live with the `World`.

## Calibration (M7, 2026-11-01; owner's delegation for M7 changes to locked specs)
- **γ₁: 0.02 → 0.005 a month per point of real-rate gap.** The update rule moves the rate of change of the leu with the gap, so a gap held for a year moves the level by about twelve months' worth: at 0.02, a rate cycle of 0% to 8.6% moved the leu from 5.28 to 4.13 within a year (after the 80% intervention), pulled demand abroad and drove unemployment from 7% to 13%. Romania's leu moved within about 4.87 to 4.95 a euro in 2021 to 2023 under the BNR's managed float. At 0.005 the 2021-law baseline's leu stays between 4.88 and 5.07 over ten years (L-03 in band). A rule on the level (parity as a jump, not a drift) is a later option if the lever responses need it.

## Open questions
- [x] **AC-TFX-03 and the risk premium restated** (2026-10-21, under the owner's delegation, after the test session): the balance of payments includes the rest of the world's bond purchases and coupons; the risk premium is the starting real-rate gap, so the quiet baseline (AC-TFX-05) does not drift.
- [x] **Settlement in the rest of the world's lei deposits** (M2's placeholder) instead of foreign assets per sector: proposed for M3; residents' foreign assets move only by the central bank's intervention. A portfolio of foreign assets per sector comes with the capital account. **Decided (owner, 2026-10-20): as proposed.**
- [x] **One price bloc** (the euro area) in M3; the two blocs of the draft when the outside world becomes dynamic (D8). **Decided (owner, 2026-10-20): as proposed.**
- [ ] Name more partner blocs later (e.g. separate China, US, Turkey, Ukraine/Moldova)?
- [x] Tourism as a services export via `hotels_tourism` and `restaurants` (proposed)? **Decided (owner, 2026-10-20): as proposed.**
