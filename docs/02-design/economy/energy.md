---
id: economy/energy
title: Energy & Power Plants
status: draft
owner: horia
depends_on: [economy/production, economy/industries, economy/infrastructure, economy/trade-fx, economy/investment-capital, adr/0016-firm-representation]
research: []
updated: 2026-10-09
---

# Energy & Power Plants

## Purpose
Electricity is an input to every industry and household. Building or closing power plants changes supply and price, which ripples through costs, inflation and the profitability of energy-heavy industries. The power-plant example is a core promise of the vision. The player must be able to act on **one technology at a time** (e.g. expand solar specifically), not on "energy in general".

## Real-world basis
- **Merit-order pricing:** plants are dispatched from cheapest marginal cost upward; the price is set by the marginal plant needed to meet demand. More cheap capacity (renewables, nuclear) pushes expensive gas plants out of the merit order and lowers the price.
- **Capture prices:** solar and wind produce at the same hours as each other, so they earn less than the average wholesale price, and the gap grows as their share rises (cannibalisation).
- Typical build times: solar 1–2 yr, wind 2–4 yr, batteries 1–2 yr, gas 2–4 yr, coal 4–6 yr, hydro 5–10 yr, nuclear 8–15 yr.
- Capacity factors: solar ~10–25%, wind ~25–45%, nuclear ~90%, gas or coal dispatchable.
- Contracts for difference (CfD): the state auctions a strike price per technology; when the market price is below it the generator is topped up, when above it pays back. Romania has run CfD auctions for wind and solar *(verify dates and volumes)*.
- Electricity is weakly tradable (interconnectors); fuels are globally traded.

## Structure: industries and firm units
Electricity is split into separate industries ([industries](industries.md)), each with its own firm units, capacity, investment, subsidies, taxes and ownership ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)):

| Industry | Technology | Typical role in the merit order |
|---|---|---|
| `power_nuclear` | Nuclear | baseload, near-zero marginal cost |
| `power_hydro` | Hydro (run-of-river and reservoir) | low marginal cost; output limited by water |
| `power_coal` | Lignite and hard coal | high marginal cost with ETS |
| `power_gas` | Gas, incl. CHP | usually the marginal plant |
| `power_wind` | Onshore wind (offshore later) | zero marginal cost, variable |
| `power_solar` | Utility-scale solar | zero marginal cost, variable, low in winter |
| `power_biomass` | Biomass, biogas, waste | dispatchable, medium cost |
| `power_storage` | Batteries, pumped-hydro storage | buys when cheap, sells when expensive |
| `power_grid` | Transmission and distribution | regulated network tariff |
| `power_supply` | Retail suppliers | buys wholesale + network, sells retail `electricity` |

The eight generation industries sell the homogeneous good `wholesale_electricity` (the exception to "one good per industry"; see [industries](industries.md)). `power_supply` sells the retail good `electricity`, which is what every other industry and every household buys.

**Plants belong to firm units.** Each plant $p$ belongs to one firm unit in the generation industry of its technology (e.g. Cernavodă to Nuclearelectrica in `power_nuclear`). Named firms own real plants; a cohort unit's plants are per real firm and scale with its `firm_count`, so small solar and wind owners are aggregated. A unit's capacity is the sum of its plants' capacity, split across regions.

> **Simplification:** rooftop solar prosumers (households and firms owning panels) are not a generation industry. Their panels are an asset of the household or firm; self-consumption lowers their purchases of `electricity`, and surplus is sold to `power_supply` at a compensation price. Green subsidies for rooftop solar act on them ([environment](environment.md)).

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $\text{Cap}_{p}$ | `plant_capacity[p]` | Installed capacity per plant (technology, region, owning firm unit) | MW |
| $cf_p$ | `capacity_factor[p]` | Availability this tick | ratio |
| $mc_p$ | `marginal_cost[p]` | Fuel + variable O&M + carbon cost (EU ETS price × emissions, see [environment](environment.md)) | LCU / MWh |
| $E^D$ | `electricity_demand` | From households and industries (retail), plus storage charging and exports | MWh / tick |
| $p^{W}$ | `wholesale_electricity_price` | Wholesale price from the merit order | LCU / MWh |
| $p^E$ | `electricity_price` | Retail price of the good `electricity` | LCU / MWh |
| — | `capture_price[j]` | Average price earned per MWh by generation industry *j* | LCU / MWh |
| — | `grid_capacity[r]` | Transmission capacity per region | MW |
| — | `cfd_contract[f]` | CfD strike price and volume held by a generation unit, if any | LCU / MWh, MWh |

## Inputs
Retail electricity demand (production $a_{electricity,j}$ × output, households' baskets, net of prosumer self-consumption); fuel prices (world price × exchange rate for imported oil, gas, coal; domestic production); ETS price and carbon tax (taxation, environment); plant construction projects (infrastructure); private investment decisions per generation unit (investment-capital); subsidies, CfDs, technology taxes and price caps.

## Outputs
Wholesale price → `power_supply` costs → retail `electricity` price → unit cost of every industry and household energy bills. Fuel demand → `coal_mining`, `natural_gas`, `uranium_fuel`, imports. Emissions and pollution per technology and county → [environment](environment.md). Profits per generation unit, paid as dividends by ownership ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)). Capacity and generation mix by technology.

## Update rule
1. **Supply curve:** every plant offers $\text{Cap}_p \cdot cf_p \cdot \text{hours}$ at $mc_p$, across all generation industries. Storage offers its discharge capacity at a price above its charging cost (see 6).
2. **Clearing:** $p^W$ is the lowest price where supply ≥ demand (+ imports via interconnectors at world price × e, up to capacity; exports when $p^W$ is below neighbours').
3. **Dispatch and revenue:** each plant below the clearing price runs; its unit sells `wholesale_electricity` to `power_supply` through the sales clearing account. Solar and wind earn their capture price:
```math
\text{capture\_price}_j = p^W \cdot \big(1 - \kappa_j \cdot \text{share}_j\big)
```
where $\text{share}_j$ is technology $j$'s share of generation and $\kappa_j$ the cannibalisation parameter (0 for dispatchable technologies). CfD holders receive (or pay back) the strike price minus the capture price on contracted volume, through the government (or a levy on suppliers).
4. **Retail price:**
```math
p^{E,*} = p^W + \text{network tariff} + \text{taxes and levies} + \text{supplier margin}
```
$p^E$ moves toward $p^{E,*}$ slowly (retail contracts adjust over 1–6 ticks), unless capped.
5. If supply < demand at any price → **shortage / blackouts**: rationing to industries (output loss) and households (hardship).
6. **Storage:** in v1 (no hourly curve) storage earns a peak/off-peak spread parameter on its throughput, loses round-trip energy, and counts as firm capacity in the shortage check, which lowers blackout risk in low-renewable months.
7. **Capacity changes:** new plants enter via private investment or state projects (below); old plants retire at end of life or by decree.

> **Simplification:** one national wholesale market per tick, regional grid limits only as a capacity cap. No hourly or seasonal load curve in v1; capacity factors are averaged per month (solar lower in winter, hydro lower in drought), and the capture-price formula stands in for hourly price shapes.

## Investment per technology
- **Private investment** is decided per generation firm unit with the [investment-capital](investment-capital.md) rules, using the technology's plant-type table (capex/MW, build time, lifetime, capacity factor). Expected revenue uses the expected **capture price** for solar and wind and the expected wholesale price for dispatchable technologies, plus CfD strike prices and subsidies where awarded, minus technology taxes and carbon costs. Named firms invest in lumpy plants (projects with names and build times); cohorts add capacity smoothly.
- **State building:** the player commissions a plant of a chosen technology, size and region as a project ([infrastructure](infrastructure.md)). On completion it belongs to a state-controlled unit in that technology's industry (an existing SOE such as Nuclearelectrica, or a new state company created by the project).
- **CfD auctions:** the player sets a volume and maximum strike price per technology; units bid by their levelised cost; winners invest with lower risk premium.

## Worked example: building a nuclear plant
Year 0: player commissions a 1,500 MW plant; construction demand flows to `construction_civil`, `steel`, `building_materials`, `industrial_machinery`, `electrical_equipment` for ~10 years (a fiscal stimulus in its own right). Year 10: the plant joins Nuclearelectrica in `power_nuclear` with $mc$ far below gas plants → several `power_gas` plants drop out of the merit order → wholesale price falls (e.g. −15%) → `power_supply` costs fall → retail `electricity` price falls over a few ticks → unit costs fall in proportion to energy intensity: `data_centres`, `nonferrous_metals`, `steel`, `fertilisers`, `chemicals` margins rise most → they invest and expand; CPI falls slightly (energy share of CPI + pass-through); gas imports fall → trade balance improves. `power_gas` units' profits fall and they invest less.

## Player levers
Per technology (each generation industry separately; see [levers](../game/levers.md)):
- production subsidy (per MWh) or investment subsidy (% of capex);
- CfD auctions (volume, maximum strike price);
- technology-specific taxes (e.g. windfall or solidarity tax on generators) and carbon tax / fuel excise;
- build plants (state projects) or close plants by decree, incl. the coal phase-out schedule;
- nationalise or privatise per firm unit or company ([state-enterprises](state-enterprises.md)).

Market-wide: retail and wholesale price caps, consumer energy subsidies, the network tariff of `power_grid` (regulator), interconnector projects.

## Tuning parameters
Plant type table per technology (capex/MW, build time, lifetime, capacity factor, fuel use, emissions, O&M); cannibalisation $\kappa_j$ per variable technology; storage spread and round-trip efficiency; retail stickiness; supplier margin; interconnector capacity.

## Edge cases & failure modes
- Price cap below marginal cost → producers lose money → no private investment, eventual shortages (unless the state covers losses).
- World gas price spike → emergent energy crisis (no script needed).
- All renewables + low capacity factor month → shortage unless storage or dispatchable backup exists.
- Very high solar share → capture price collapses → solar investment stalls without CfDs or storage.
- Drought → hydro output falls → gas becomes marginal more often → price rises.

## Acceptance tests
- [ ] Adding cheap capacity lowers the wholesale price and raises margins of energy-intensive industries within 1–3 ticks of the plant going online; the retail price follows within 1–6 ticks.
- [ ] A doubling of the world gas price raises the wholesale price when gas is the marginal plant, and not when it isn't.
- [ ] A price cap below cost leads to falling private plant investment and eventually shortages.
- [ ] A production subsidy for `power_solar` raises solar investment and capacity, and not wind or gas capacity.
- [ ] As solar's share rises, its capture price falls below the average wholesale price and new solar investment slows unless supported by CfDs or storage.
- [ ] Wholesale revenue paid by `power_supply` equals the sum of generation units' revenues every tick (sales clearing account nets to zero).

## Open questions
- [x] Do private power companies propose plants on their own? Yes: private investment per technology by generation units, plus state projects (2026-10-09).
- [x] Storage in v1? Yes, as `power_storage` with a spread parameter; seasonal variation via monthly capacity factors.
- [ ] Do large industrial consumers buy on the wholesale market directly (v1: all through `power_supply`)?
- [ ] Green certificates (Romania's legacy renewable support scheme) in v1, or only CfDs and subsidies?
