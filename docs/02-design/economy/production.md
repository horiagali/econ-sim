---
id: economy/production
title: Production & Supply Chains
status: locked
owner: horia
depends_on: [economy/accounting, economy/industries, economy/labor-market, economy/investment-capital, adr/0016-firm-representation, adr/0017-time-base]
research: []
updated: 2026-10-13
---

# Production & Supply Chains

## Purpose
Decide how much each firm unit produces, what it buys from other industries, how many workers it needs and what its unit cost is. Supply chains make cost and demand shocks propagate: car parts → cars, electricity → everything (industries buy the retail good `electricity` from `power_supply`; see [energy](energy.md)).

## Real-world basis
- Input-output (Leontief) structure of intermediate demand; short-run complementarity of inputs (you can't build a car without parts).
- Value added from labour and capital with limited substitution (CES / Cobb-Douglas).
- Firms produce to expected demand and keep inventories as a buffer (post-Keynesian / SFC firm behaviour). Prices are sticky in the short run, so quantities adjust first.
- Romania 2021 (input-output table, [industries](industries.md)): output 2,237 billion lei, of which intermediate inputs bought at home 30%, imported inputs 12%, wages and contributions 20%, surplus and other costs the rest; exports 425 and imports 483 billion lei.

## State variables (per firm unit $f$ in industry $j$; see [ADR-0016](../../03-architecture/decisions/0016-firm-representation.md))
Each industry is a small table of **firm units**: named firms (from roadmap M9) plus size-class cohort firms (micro, small, large; split domestic/foreign where the foreign share is material) and public units. For a cohort firm the variables below are **per real firm**, and $n_f$ (`firm_count[f]`) says how many real firms it stands for. The starting units are in [industries](industries.md) ("The firm units").

| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $n_f$ | `firm_count[f]` | Real firms represented by unit $f$ (integer; read from the scenario, never hardcoded) | count |
| $K_f$ | `capital_stock[f]` | Productive capital | real units |
| $\bar Y_{f,r}$ | `capacity[f,r]` | Max output given $K_f$, infrastructure, energy; a vector over regions $r$ | real units / month |
| $Y_f$ | `output[f]` | Output | real units / month |
| $V_f$ | `inventory[f]` | Finished-goods inventory | real units |
| $A_f$ | `tfp[f]` | Total factor productivity | index |
| $N_{f,o}$ | `employment[f,o]` | Workers by occupation $o$ | persons |
| $D^e_f$ | `expected_demand[f]` | Expected sales | real units / month |
| $cu_f$ | `capacity_utilisation[f]` | $Y_f / \bar Y_f$ | ratio |

**Industry = sum of its units.** Industry variables are derived, never stored separately. Industry demand is shared among units in proportion to what they have to sell (all units sell at the industry's market price). **Exception:** the generation industries jointly sell `wholesale_electricity` by merit-order dispatch ([energy](energy.md), from roadmap M12). The update rule below is written per industry for readability and is applied per firm unit, with $j$ read as $f$.

**A real unit** of a good is what one leu bought at the start date: quantities are in lei at starting prices, and every price starts at 1.

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| Sales out of inventory; output added to inventory; the goods market | **daily** |
| A unit's plan: expected demand, planned output, input orders, desired employment | **staggered**: once a month, on the unit's own day (1 to 28), before its staffing ([labour market](labor-market.md)) |
| Statistics: output, value added (GDP by production), utilisation, inventories | monthly close |

Monthly quantities are spread over the days of the plan's month in equal parts, the remainder on the last day.

## Inputs
Demand by buyer type (households, firms, investment, government, exports) from the owning specs; input prices (prices-inflation); wages (labor-market); energy price (energy); infrastructure index by region (infrastructure); credit availability (money-banking); subsidies (fiscal-policy); education-adjusted skill (education).

## Outputs
Output, intermediate demand to suppliers, labour demand, unit cost breakdown, profits, capacity utilisation, inventory changes, shortages.

## Update rule
**1. Expected demand** (adaptive):
```math
D^e_{j,t} = D^e_{j,t-1} + \theta (S_{j,t-1} - D^e_{j,t-1})
```
**2. Planned output** fills expected demand and moves inventories toward a target $v^*_j \cdot D^e_j$:
```math
Y^{plan}_j = D^e_j + \phi (v^*_j D^e_j - V_j)
```
**3. Constraints.** Actual output is the minimum of planned output, capacity, available inputs and labour:
```math
Y_j = \min\Big(Y^{plan}_j,\ \bar Y_j,\ \min_i \frac{\text{input}_i}{a_{ij}},\ \frac{N_j}{\ell_j}\Big)
```
Input shortages (e.g. car-parts import disruption) therefore cap output downstream. That is the supply-chain bottleneck mechanic.

**4. Capacity:** $\bar Y_j = A_j \cdot \Psi_r(\text{infra}) \cdot F(K_j)$, where $\Psi$ is the infrastructure multiplier for the industry's regions.

**5. Labour requirement** per unit falls with TFP and skill quality: $\ell_{j,s} = \ell^0_{j,s} / (A_j \cdot \text{skill}_{s})$.

**6. Unit cost:**
```math
uc_j = \sum_i a_{ij} p_i (1+\tau^{tariff}_i \cdot m_i) + \sum_s \ell_{j,s} W_{j,s}(1+\tau^{payroll}) + \frac{\delta_j K_j p^K + r^L_j L_j}{Y_j} + \text{carbon}_j - \text{subsidy}_j
```
Each term is stored for the cost-breakdown view and the "why" panel.

**7. TFP growth:**
```math
A_{j,t+1} = A_{j,t} \big(1 + g^0 + g_{edu}\Delta\text{skill} + g_{R\&D} \cdot \text{research}_j + g_{infra}\Delta\text{infra} + g_{FDI}\big)
```

> **Simplification:** firms are firm units ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)), not individual firms. Within a cohort all firms are identical; bankruptcy and entry change the cohort's `firm_count`, and a named firm's failure is a discrete event.

> **Simplification:** fixed input coefficients $a_{ij}$ in the short run. Energy input can be partly substituted over years via investment.

## First version (roadmap M2)
**Market clearing is by inventories** (the open decision D6 of the roadmap, decided 2026-10-11 under the owner's delegation): prices do not jump to clear a market within a day; buyers are served from stock, and stock running out is a shortage that the price rule sees.

**Starting state.** For each unit, from the firm-unit file and the input-output table: monthly output = its annual output ÷ 12; `expected_demand` = the same; `inventory` = `inventory_target_ratio` × expected demand; capacity = starting output ÷ `starting_utilisation`; labour requirement per unit of output = its starting staff ÷ its starting output; input coefficients $a_{ij}$, for inputs bought at home and imported from each bloc, = those of its industry's column of the table (a catalogue industry has the coefficients of its table industry, split over the supplying catalogue industries by their output shares).

**In:**
1. **The plan, on a unit's day** (integers; real units are lei at starting prices):
   - sales of the last month $S$ = what the unit sold since its last plan;
   - $D^e \leftarrow D^e + \lfloor \theta (S - D^e) \rceil$;
   - $Y^{plan} = \max(0,\ D^e + \lfloor \phi (v^* D^e - V) \rceil)$, capped at capacity;
   - `desired_employment` = $\lceil Y^{plan} \cdot \ell \rceil$ (persons; the [labour market](labor-market.md) then lays off or opens vacancies);
   - input orders for the month: $a_{ij} Y^{plan}$ of each input, home and imported.
2. **Each day:**
   - **Output:** the day's part of the plan, cut in proportion if the unit's staff is below the plan's requirement or an input was short yesterday. Output goes into `inventory`.
   - **Input purchases:** the day's part of the orders is demand on the supplying industries (home inputs) and a payment to the rest of the world (imported inputs, at the fixed starting price in M2).
   - **The goods market, per industry:** the day's demand is the sum of household purchases ([households](households.md)), other units' input purchases, and the placeholders below. Units serve it in proportion to their inventories. If demand exceeds the industry's inventories, every buyer gets the same share of what they asked for; the rest is a **shortage** (recorded per industry and day; the buyer keeps the money).
   - **Money:** buyers pay the industry's price ([prices](prices-inflation.md)); sales go to the units through the industry's sales clearing account ([accounting](accounting.md)).
3. **Placeholders for final demand that M2 does not model.** Government purchases, investment and exports of each industry stay at their starting real amounts per day (the table's annual figures ÷ 365), paid for by the government, by the buying firms' sector and by the rest of the world.

> **Simplification (M2):** capital, capacity and TFP are constant; no depreciation, no investment decision, no entry or exit of firms; imports are always available at the starting price; profits stay in the units as deposits. Each arrives with [investment and capital](investment-capital.md), [trade](trade-fx.md) and [money and banking](money-banking.md) in M3.

4. **Value added and GDP.** A unit's value added in a month = its output at the month's prices less its input purchases. Summed over units it equals GDP by expenditure (consumption + government + investment + inventory change + exports − imports) and by income (wages + surplus + taxes on products), exactly, at every monthly close (I-7).

**Not in M2:** steps 4 to 7 of the update rule beyond constants; tariffs, subsidies and carbon costs in unit cost; substitution between home and imported inputs; electricity by merit order (M12); regional capacity vectors (capacity is one number per unit).

## Worked example: car-parts price shock
1. Tariff on imported car parts +20% → landed price of the import share of `car_parts` +20%; domestic car-parts producers raise prices partly (prices-inflation).
2. `cars` unit cost: $a_{parts,cars} \approx 0.35$ of output value, so a 10% rise in the average parts price → ~3.5% rise in unit cost.
3. Car makers pass through most of it over a few months → car prices +3%.
4. Domestic car demand falls (price elasticity ~−1), car exports fall (vs world car price × exchange rate).
5. `cars` output falls → less hiring at car plants and less demand for steel and electronics. Domestic `car_parts` producers gain share vs imports (the protectionist effect).
6. The net effect on GDP and jobs depends on parameters. Both sides must be visible in the industry view.

## Player levers
Indirect: subsidies, tariffs, taxes, infrastructure, research grants, nationalisation. Direct for SOEs only (see [state-enterprises](state-enterprises.md)).

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `demand_expectation_theta` | 0.3 a month | Speed of demand learning |
| `inventory_target_ratio[j]` | 1 month of expected demand (0.25 for services) | Buffer size |
| `inventory_adjust_phi` | 0.25 a month | Speed toward target |
| `starting_utilisation` | 0.8 | Capacity over starting output |
| `tfp_base_growth` | 1%/yr | Exogenous technical progress (from M3) |

## Interactions
Upstream: industries (coefficients), labour, energy, infrastructure, investment, credit. Downstream: prices (unit cost), labour demand, profits (households via dividends, government via corporate tax and SOE profits), trade.

## Edge cases & failure modes
- **An essential input is short:** output is cut the next day in the proportion of the worst-supplied input.
- **Circular dependencies** in the input-output structure: none within a day, because input purchases draw on suppliers' inventories, not on today's output.
- **Inventories cannot go negative:** sales are capped at inventory; unmet demand is a shortage.
- **A unit with no staff:** it produces nothing and sells its inventory down.
- **Services** have a small buffer (they cannot be stored for long); the rule is the same.

## Acceptance tests
IDs are stable (added 2026-10-11). Tests go in the mechanic's crate, `crates/econ-mech-production`, in `tests/acceptance/` (protected). AC-PROD-01 and 02 were written there on 2026-10-13 against the API skeleton, compiled only with the crate feature `mechanic` until the implementation turns it on.

- [ ] **AC-PROD-01** `[unit]` The plan: expected demand, planned output and desired employment are the formulas of step 1, in integers; planned output is never negative and never above capacity.
- [ ] **AC-PROD-02** `[unit]` Rationing: when demand on an industry exceeds its inventories every buyer gets the same share, nothing more than the inventories is sold, and the shortage is demand less sales.
- [ ] **AC-PROD-03** `[sim]` Inventories: for every unit and day, inventory change = output − sales, exactly; no inventory is negative.
- [ ] **AC-PROD-04** `[sim]` Money: what buyers pay for an industry's goods in a day equals what its units receive, through a sales clearing account that nets to zero.
- [ ] **AC-PROD-05** `[sim]` GDP by production, by expenditure and by income agree exactly at every monthly close.
- [ ] **AC-PROD-06** `[sim]` Each unit plans exactly once in every calendar month, on its own day, before its staffing.
- [ ] **AC-PROD-07** `[sim]` A permanent rise in household demand for `food_bakery_milling` raises the output of `agri_cereals` in the following months (input-output propagation).
- [ ] **AC-PROD-08** `[sim]` Cutting the staff of `car_parts` units by half reduces the output of `cars` once car-parts inventories run down (bottleneck), and shortages of car parts are recorded.
- [ ] **AC-PROD-09** `[sim]` Quiet baseline: on the Romania scenario with no change of policy, output of every industry group over 120 months stays within 15% of its starting level, and inventories neither run out nor grow without limit.
- [ ] **AC-PROD-10** `[sim]` *(M3)* With TFP growth > 0 and stable policy, real GDP grows in the long run.
- [ ] **AC-PROD-11** `[sim]` *(M12)* A 20% rise in the retail electricity price raises unit costs most in `nonferrous_metals`, `steel`, `fertilisers`, `data_centres`, and least in `professional_services`.

### Tests owed
The spec was locked before these tests could exist (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-PROD-03: production on the real `World` (roadmap M2)
- AC-PROD-04: the goods market with the ledger's clearing accounts (roadmap M2)
- AC-PROD-05: the full M2 loop with invariant I-7
- AC-PROD-06: the scheduler and the production mechanic (roadmap M2)
- AC-PROD-07: the full M2 loop
- AC-PROD-08: the full M2 loop
- AC-PROD-09: the full M2 loop on the Romania scenario
- AC-PROD-10: investment and productivity (roadmap M3)
- AC-PROD-11: energy and its own cost structures (roadmap M12)

## API sketch
For the test writer and the implementer; names may change, shapes should not. Quantities are whole real units (`i64`); shares are millionths.

```rust
// crate econ-mech-production (no I/O)
pub struct ProductionParams { pub theta_ppm: u32, pub phi_ppm: u32 }
pub struct UnitState { pub expected_demand: i64, pub inventory: i64, pub capacity: i64, pub inventory_target_ppm: u32,
                       pub labour_per_million_output: u64 }
pub struct Plan { pub expected_demand: i64, pub planned_output: i64, pub desired_employment: u64 }
pub fn plan(u: &UnitState, sales_last_month: i64, p: &ProductionParams) -> Plan;                       // AC-PROD-01
pub struct Served { pub sold: Vec<i64> /* per buyer */, pub shortage: i64 }
pub fn ration(demands: &[i64], available: i64) -> Served;                                              // AC-PROD-02
pub fn share_sales(sold: i64, inventories: &[i64]) -> Vec<i64>;                                        // by units' inventories, exact
```

The daily phases (output, input purchases, the goods market, postings) live with the `World`.

## Open questions
- [x] Fixed proportions (Leontief) for all inputs in v1; substitution for energy over years comes with [energy](energy.md). Decided 2026-10-11 under the owner's delegation.
- [x] Per-region production units or a regional capacity split → each firm unit carries capacity as a vector over regions ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)); one number per unit in M2.
- [x] **Firm table instead of one aggregate firm per industry?** Resolved by [ADR-0016](../../03-architecture/decisions/0016-firm-representation.md).
- [x] Market clearing by inventories; first version, periods and parameters as above: accepted by the owner's delegation, 2026-10-11 ("accept the drafts").
- [ ] `starting_utilisation` and the inventory targets are guesses; business surveys publish utilisation for industry.
