---
id: economy/production
title: Production & Supply Chains
status: draft
owner: horia
depends_on: [economy/accounting, economy/industries, economy/labor-market, economy/investment-capital, adr/0016-firm-representation]
research: []
updated: 2026-10-09
---

# Production & Supply Chains

## Purpose
Decide how much each industry produces, what it buys from other industries, how many workers it needs and what its unit cost is. Supply chains make cost and demand shocks propagate: car parts → cars, electricity → everything (industries buy the retail good `electricity` from `power_supply`; see [energy](energy.md)).

## Real-world basis
- Input-output (Leontief) structure of intermediate demand; short-run complementarity of inputs (you can't build a car without parts).
- Value added from labour and capital with limited substitution (CES / Cobb-Douglas).
- Firms produce to expected demand and keep inventories as a buffer (post-Keynesian / SFC firm behaviour). Prices are sticky in the short run, so quantities adjust first.

## State variables (per firm unit $f$ in industry $j$; see [ADR-0016](../../03-architecture/decisions/0016-firm-representation.md))
Each industry is a small table of **firm units**: 0–5 named firms plus up to three size-class cohort firms (micro, small, medium/large; split domestic/foreign where the foreign share is material). For a cohort firm the variables below are **per real firm**, and $n_f$ (`firm_count[f]`) says how many real firms it stands for; a named firm has $n_f = 1$. Ownership shares (domestic private, foreign, state) are attributes of each unit ([foreign-ownership](foreign-ownership.md), [state-enterprises](state-enterprises.md)).

| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $n_f$ | `firm_count[f]` | Real firms represented by unit $f$ (integer; read from the scenario, never hardcoded) | count |
| $K_f$ | `capital_stock[f]` | Productive capital | real units |
| $\bar Y_{f,r}$ | `capacity[f,r]` | Max output given $K_f$, infrastructure, energy; a vector over regions $r$ | real units / tick |
| $Y_f$ | `output[f]` | Output this tick | real units / tick |
| $V_f$ | `inventory[f]` | Finished-goods inventory | real units |
| $A_f$ | `tfp[f]` | Total factor productivity | index |
| $N_{f,o}$ | `employment[f,o]` | Workers by occupation $o$ (skill tier $s$ derived from occupation) | persons |
| $D^e_f$ | `expected_demand[f]` | Expected sales | real units / tick |
| $cu_f$ | `capacity_utilisation[f]` | $Y_f / \bar Y_f$ | ratio |

**Industry = sum of its units.** Industry variables are derived, never stored separately: $Y_j = \sum_{f \in j} n_f Y_f$, and likewise for capacity, capital, employment and inventories. Industry demand is shared among units in proportion to their available capacity (v1; all units sell at the industry's market price). **Exception:** the generation industries (`power_*` except `power_grid` and `power_supply`) jointly sell `wholesale_electricity`; their output is set by merit-order dispatch across technologies at `wholesale_electricity_price`, not by this demand split ([energy](energy.md), [industries](industries.md)). The update rule below is written per industry for readability and is applied per firm unit, with $j$ read as $f$.

## Inputs
Demand by buyer type (households, firms, investment, government, exports) from the owning specs; input prices (prices-inflation); wages by skill (labor-market); energy price (energy); infrastructure index by region (infrastructure); credit availability (money-banking); subsidies (fiscal-policy); education-adjusted skill (education).

## Outputs
Output, intermediate demand to suppliers, labour demand by skill and region, unit cost breakdown, profits, capacity utilisation, inventory changes.

## Update rule (per tick)
**1. Expected demand** (adaptive, with trend):
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
Each term is stored for the cost-breakdown view and the "why" panel. $\text{carbon}_j$ = ETS price × uncovered emissions per unit ([environment](environment.md)). Weather shocks change capacity of agriculture and hydro ([environment](environment.md)).

**7. TFP growth:**
```math
A_{j,t+1} = A_{j,t} \big(1 + g^0 + g_{edu}\Delta\text{skill} + g_{R\&D} \cdot \text{research}_j + g_{infra}\Delta\text{infra} + g_{FDI}\big)
```
This is what lets GDP grow without limit.

> **Simplification:** firms are firm units ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)), not individual firms. Within a cohort all firms are identical; bankruptcy and entry change the cohort's `firm_count` (`firm_exit_rate[f]`, `firm_entry_rate[f]`, aligned with a carried remainder; exiting firms are treated as average firms), and a named firm's failure is a discrete event (administration, then rescue or absorption into its cohort).

> **Simplification:** fixed input coefficients $a_{ij}$ in the short run. Energy input can be partly substituted over years via investment (energy-efficiency parameter).

## Worked example: car-parts price shock
1. Tariff on imported car parts +20% → landed price of the import share of `car_parts` +20%; domestic car-parts producers raise prices partly (prices-inflation).
2. `cars` unit cost: $a_{parts,cars} \approx 0.35$ of output value, so a 10% rise in the average parts price → ~3.5% rise in unit cost.
3. Car makers pass through most of it over a few ticks → car prices +3%.
4. Domestic car demand falls (price elasticity ~−1), car exports fall (vs world car price × exchange rate).
5. `cars` output falls → less hiring at car plants and less demand for steel and electronics. Domestic `car_parts` producers gain share vs imports (the protectionist effect).
6. The net effect on GDP and jobs depends on parameters. Both sides must be visible in the industry view.

## Player levers
Indirect: subsidies, tariffs, taxes, infrastructure, research grants, nationalisation. Direct for SOEs only (see [state-enterprises](state-enterprises.md)).

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `demand_expectation_theta` | 0.3 | Speed of demand learning |
| `inventory_target_ratio[j]` | 0.5–2 months | Buffer size |
| `inventory_adjust_phi` | 0.25 | Speed toward target |
| `tfp_base_growth` | 1%/yr | Exogenous technical progress |

## Interactions
Upstream: industries (coefficients), labour, energy, infrastructure, investment, credit. Downstream: prices (unit cost), labour demand, profits (households via dividends, government via corporate tax and SOE profits), trade.

## Edge cases & failure modes
- Zero output when any essential input is zero → allow substitute imports first (trade-fx), then cap.
- Circular dependencies in the I-O matrix within one tick → resolve with last tick's prices and quantities, or solve the Leontief system $(I-A)^{-1}$ once per tick.
- Inventories going negative → sales capped at output + inventory; unmet demand is recorded as **shortage** (feeds prices and statistics).

## Acceptance tests
- [ ] A 20% rise in the retail electricity price (`electricity_price`) raises unit costs most in `nonferrous_metals`, `steel`, `fertilisers`, `data_centres`, and least in `professional_services`.
- [ ] Cutting car-parts supply by 50% reduces `cars` output within 1–2 ticks (bottleneck).
- [ ] A permanent rise in household demand for `food_bakery_milling` raises `agri_cereals` output too (I-O propagation).
- [ ] With TFP growth > 0 and stable policy, real GDP grows in the long run.
- [ ] GDP by production (value added sum) equals GDP by expenditure every tick.

## Open questions
- [ ] Leontief (fixed proportions) for all inputs, or CES for energy and labour?
- [x] Per-region production units or a regional capacity split → each firm unit carries capacity as a vector over regions, not separate units ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)).
- [x] **Firm table instead of one aggregate firm per industry?** Resolved by [ADR-0016](../../03-architecture/decisions/0016-firm-representation.md) (accepted 2026-10-09): variable named firms (0–5 per industry) plus size-class cohort firms with integer firm counts, applied in this spec, [industries](industries.md), [investment-capital](investment-capital.md), [money-banking](money-banking.md), [foreign-ownership](foreign-ownership.md) and [state-enterprises](state-enterprises.md). Background: [population research](../../01-research/notes/population-modelling-deep-research.md).
