---
id: economy/prices-inflation
title: Prices & Inflation
status: locked
owner: horia
depends_on: [economy/production, economy/labor-market, economy/trade-fx, economy/monetary-policy, economy/vat, adr/0017-time-base]
research: []
updated: 2026-10-21
---

# Prices & Inflation

## Purpose
Set the price of every good, compute CPI and inflation for any group, and model inflation expectations. Prices carry shocks along supply chains (car parts → cars) and connect money, wages and demand.

## Real-world basis
- Firms price as a **markup over unit cost**, adjusting the markup with demand pressure; prices are sticky (Calvo-style: a fraction adjusts each period; food and energy fast, services slow).
- Cost pass-through is partial and gradual; exchange-rate pass-through to import prices is high, to CPI lower (10–30%).
- Expectations: anchored if the central bank is credible, otherwise adaptive (de-anchoring after persistent high inflation, 1970s).
- Inflation needs a monetary or demand mechanism to persist; one-off cost shocks fade if expectations stay anchored.
- Romania, December 2021: consumer prices were 8.2% above a year earlier and rising, against a target of 2.5% *(from memory; to be checked when the scenario's starting values are collected)*.

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| The price of each good moves toward its target | **daily**, by the daily equivalent of its monthly speed |
| Unit cost and the target price are recomputed | daily, from the day's wages, input prices and yesterday's inventories |
| CPI, core CPI, producer prices, inflation, expectations | monthly close |
| CPI weights | yearly, in January |

Speeds and rates are stated per month or per year and converted once ([ADR-0017](../../03-architecture/decisions/0017-time-base.md) section 3): a monthly adjustment share $\rho$ used daily becomes $1 - (1 - \rho)^{1/30}$.

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $p_g$ | `price[g]` | producer price of good $g$, without VAT; 1 at the start | index |
| $\mu_g$ | `markup[g]` | markup over unit cost | ratio |
| $P$ | `price_level` | consumer price index (CPI); 1 at the start | index |
| $\pi^e$ | `inflation_expectation` | expected inflation | % / year |

## Update rule (per good $g$)
**Target price:**
```math
p^*_g = (1 + \mu_g) \cdot uc_g
```
Taxes on the product (VAT, excise) are added on top when a household buys ([vat](vat.md)); the consumer price is $(p_g + d_g)(1 + \tau^{VAT}_g)$, with $d_g$ the excise duty per unit of [excise](excise.md): VAT is charged on the duty, as the law does. *(Restated 2026-10-21 by the owner, with excise; it read $p_g (1 + \tau^{VAT}_g + \tau^{excise}_g)$.)*

**Markup** responds to demand pressure (inventory ratio and capacity utilisation):
```math
\mu_{g,t} = \mu^0_g + \eta_1 (cu_g - cu^*) - \eta_2 \Big(\frac{V_g}{v^* D^e_g} - 1\Big)
```
**Sticky adjustment** with expectations, per month:
```math
p_{g,t} = p_{g,t-1}\big(1 + \pi^e_{monthly}\big)^{\chi} \cdot \Big(\frac{p^*_g}{p_{g,t-1}}\Big)^{\rho_g}
```
$\rho_g$ = per-good flexibility (energy and food high, services low).

**Electricity exceptions** ([energy](energy.md), from roadmap M12): `wholesale_electricity` has no markup rule; its price is set by the merit order. The network good `power_grid` has a regulated tariff. The retail good `electricity` is wholesale + network tariff + taxes + supplier margin with sticky adjustment.

**Price caps:** $p_g \le \bar p_g$; if binding, excess demand → shortages (rationing: every buyer gets the same share), black-market pressure later.

**Imported goods:** landed price $= p^{world}_g \cdot e \cdot (1+\tau^{tariff}_g)$. Domestic price of tradables is a weighted mix of domestic and imported variants.

**CPI:** $P = \sum_g w_g p_g (1 + \tau_g)$ with weights from aggregate household baskets (re-weighted yearly, like statistics offices). Core CPI excludes food and energy. Group CPI uses household baskets ([households](households.md)).

## Expectations
Anchored-adaptive (the open decision D6 of the roadmap, decided 2026-10-11 under the owner's delegation):
```math
\pi^e_{t+1} = \text{cred} \cdot \pi^{target} + (1 - \text{cred}) \big[\pi^e_t + \lambda (\pi_t - \pi^e_t)\big]
```
`cred` (central bank credibility, 0–1) rises with a track record of hitting the target; falls with monetary financing and persistent misses ([monetary-policy](monetary-policy.md)). Household expectations may differ (less educated households more backward-looking).

## First version (roadmap M2)
**In:**
1. **Unit cost of an industry, daily,** per unit of output, from its input coefficients ([production](production.md)):

```math
uc_j = \sum_i a^{home}_{ij} \, p_i + \sum_i a^{imp}_{ij} \cdot 1 + \frac{\text{wages paid last month}_j}{\text{output last month}_j} + \text{other costs}_j
```

   Imported inputs cost their starting price in M2. $\text{other costs}_j$ = the starting share of other taxes on production, constant. All units of an industry sell at one price, so unit cost is the industry's (output-weighted over its units).
2. **Starting markup** $\mu^0_j$ = what makes $p^*_j = 1$ at the starting unit cost: the industry's starting surplus over its other costs.
3. **Markup and target,** daily, with the industry's inventories and utilisation as in the update rule.
4. **Price,** daily: $p_j \leftarrow p_j \cdot (1 + \pi^e_{daily})^{\chi} \cdot (p^*_j / p_j)^{\rho^{daily}_j}$, in millionths, through the pinned `libm` power. A day's move is capped at `max_daily_price_move`; hitting the cap is recorded.
5. **CPI,** at the monthly close: weights $w_g$ = `basket_share[g]` of [households](households.md), fixed in M2; consumer prices include VAT at the rate of the good's category. Inflation = CPI over the CPI twelve months earlier (over the starting CPI, annualised, in the first year).
6. **Expectations,** at the monthly close, with `cb_credibility` constant in M2 and the starting $\pi^e$ and $\pi^{target}$ from the scenario.

> **Simplification (M2):** three flexibility classes of goods (below); no price caps, excise or tariffs; no separate price of the imported variant; one national price per good.

**Not in M2:** the exchange rate in import prices (M3); credibility that moves (M3); price caps (M9); the electricity exceptions (M12); group price indices (M6).

## Player levers
VAT, excise, tariffs, price caps (incl. retail and wholesale electricity caps), subsidies (lower unit cost), policy rate and credibility (via expectations), energy supply per technology.

## Outputs
Prices by good, CPI, core CPI, inflation by group, PPI, import price index, inflation expectations, decomposition of each price change (unit-cost terms, markup, taxes, expectations).

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `price_flexibility[g]` $\rho_g$ | a month: 0.5 for farm goods, fuels and energy; 0.25 for other goods; 0.1 for services | how fast a price closes the gap to its target |
| `expectation_indexation` $\chi$ | 0.5 | how much of expected inflation prices follow without any cost push |
| `markup_utilisation_response` $\eta_1$ | 0.2 | markup per point of utilisation above normal |
| `markup_inventory_response` $\eta_2$ | 0.1 | markup per unit of excess inventory ratio |
| `normal_utilisation` $cu^*$ | 0.8 | the `starting_utilisation` of [production](production.md) |
| `expectation_learning` $\lambda$ | 0.1 a month | how fast expectations follow actual inflation |
| `cb_credibility` | 0.7 | weight of the target in expectations (state of [monetary-policy](monetary-policy.md) from M3) |
| `max_daily_price_move` | 2% | numerical guard |

The stability rule of [ADR-0009](../../03-architecture/decisions/0009-calibration-and-stability.md) is checked on the daily speeds: each is far below 1, so no price can overshoot its target in a day.

## Interactions
- **From [production](production.md):** input coefficients, inventories, utilisation, output. **From the [labour market](labor-market.md):** wages paid.
- **To [households](households.md):** consumer prices and the CPI (the subsistence floor follows it). **To the labour market:** expected inflation in the January wage round. **To production:** sales are booked at the day's price.
- A wage rise reaches prices through unit cost over the following months; prices reach wages at the next January round. That loop is the wage-price spiral; with `cb_credibility` high and $\chi$ below 1 it dies out.

## Edge cases & failure modes
- **Hyperinflation:** monetary financing + de-anchored expectations → `cred` → 0 → spiral. Must be possible but require sustained bad policy (from M3; v1 has no special crisis dynamics, roadmap D10).
- **Deflation:** falling prices + debt burden; zero lower bound in monetary policy.
- **An industry with no output last month:** its wage cost per unit keeps the last value.
- **Numerical:** the daily cap; a price never reaches zero.

## Acceptance tests
IDs are stable (added 2026-10-11). Tests go in the mechanic's crate, `crates/econ-mech-prices`, in `tests/acceptance/` (protected). AC-PRICE-01 to 03 were written there on 2026-10-13 against the API skeleton, compiled only with the crate feature `mechanic` until the implementation turns it on. The `[sim]` criteria AC-PRICE-04, 05, 07 and 08 were written on 2026-10-15 on the `World` ([world loop](../game/world-loop.md)), in `crates/econ-core/tests/acceptance/`, behind the feature `world`; AC-PRICE-05 and 07 compare two worlds whose starting states differ by an edit applied after the starting markups are set (every wage +10%; one industry's stock halved or doubled).

- [ ] **AC-PRICE-01** `[unit]` Unit cost, markup and target price are the formulas above; at the starting state every target price is 1 and every price stays 1 while nothing else changes and expected inflation is zero.
- [ ] **AC-PRICE-02** `[unit]` The daily step: a price moves toward its target by the daily equivalent of its monthly flexibility, so that thirty daily steps close the same share of a constant gap as one monthly step, within rounding; a day's move never exceeds the cap.
- [ ] **AC-PRICE-03** `[unit]` Expectations are the anchored-adaptive formula; with credibility 1 they equal the target, with credibility 0 they are adaptive.
- [ ] **AC-PRICE-04** `[sim]` CPI at each monthly close is the weighted mean of consumer prices with the basket's weights; the weights sum to 1.
- [ ] **AC-PRICE-05** `[sim]` A 10% rise in every wage raises prices over the following months, more and sooner in industries with a higher wage share of cost, and by less than 10% in total while imports cost the same.
- [ ] **AC-PRICE-06** `[sim]` A one-off rise of the standard VAT rate by 5 points raises the CPI once (the level), and twelve-month inflation returns to its earlier path a year later, when expectations are anchored.
- [ ] **AC-PRICE-07** `[sim]` When an industry's inventories fall below target its price rises relative to the others; when they pile up it falls.
- [ ] **AC-PRICE-08** `[sim]` Quiet baseline: on the Romania scenario with no change of policy, with the target as starting expectation, twelve-month inflation over 120 months stays within 3 points of the target.
- [ ] **AC-PRICE-09** `[sim]` *(M3)* A 20% currency depreciation raises import prices quickly and CPI by a smaller amount over 6–12 months.
- [ ] **AC-PRICE-10** `[sim]` *(M3)* Persistent monetary financing at 10% of GDP per year eventually de-anchors expectations and causes accelerating inflation.
- [ ] **AC-PRICE-11** `[sim]` *(M9)* A price cap below market price produces shortages, and shortage size rises with the gap.

### Tests owed
The spec was locked before these tests could exist (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-PRICE-06: the full M2 loop with the VAT lever: a rate change after the CPI's base is set (roadmap M4); a world built under another rate starts its CPI at 1 and shows no rise
- AC-PRICE-09: trade and the exchange rate (roadmap M3)
- AC-PRICE-10: monetary policy and credibility (roadmap M3)
- AC-PRICE-11: price caps (roadmap M9)

## API sketch
For the test writer and the implementer; names may change, shapes should not. Prices, shares and rates are millionths.

```rust
// crate econ-mech-prices (no I/O)
pub struct PriceParams { pub indexation_ppm: u32, pub eta_utilisation_ppm: u32, pub eta_inventory_ppm: u32,
                         pub normal_utilisation_ppm: u32, pub max_daily_move_ppm: u32 }
pub fn unit_cost_ppm(home_inputs: &[(u32 /* coefficient */, u32 /* price */)], imported_ppm: u32, wage_cost_ppm: u32, other_ppm: u32) -> u32;
pub fn markup_ppm(base_ppm: i32, utilisation_ppm: u32, inventory_ratio_ppm: u32, p: &PriceParams) -> i32;
pub fn daily_share_ppm(monthly_share_ppm: u32) -> u32;                                         // 1 - (1 - rho)^(1/30)
pub fn step_price_ppm(price: u32, target: u32, daily_flexibility_ppm: u32, expected_daily_ppm: i32, p: &PriceParams) -> u32;   // AC-PRICE-02
pub fn expectations_ppm(previous: i32, actual: i32, target: i32, credibility_ppm: u32, learning_ppm: u32) -> i32;              // AC-PRICE-03
```

Powers go through `econ_num::math` (ADR-0006). The daily phase over industries and the monthly CPI live with the `World`.

## Open questions
- [x] Negative markups are allowed (a firm can sell below cost to clear stock); the price cannot fall below a tenth of unit cost. Decided 2026-10-11 under the owner's delegation.
- [x] Rationing under shortages: every buyer gets the same share ([production](production.md)); a player-chosen rule can come with price caps (M9). Decided 2026-10-11.
- [x] Expectations anchored-adaptive; first version, periods and parameters as above: accepted by the owner's delegation, 2026-10-11 ("accept the drafts").
- [ ] The starting inflation rate and expectation for December 2021 are to be taken from the statistics when the scenario's starting values are collected; the quiet-baseline test uses the target instead.
