---
id: economy/environment
title: Environment & Climate — Emissions, Pollution, Weather
status: draft
owner: horia
depends_on: [economy/energy, economy/production, economy/eu-membership, society/social-outcomes]
research: []
updated: 2026-10-09
---

# Environment & Climate — Emissions, Pollution, Weather

## Purpose
Track greenhouse-gas emissions and local pollution from energy, industry, transport, heating and agriculture; price carbon through the EU emissions trading system and national taxes; link pollution to health; and model weather shocks (droughts, floods, heatwaves) that hit agriculture, infrastructure and energy, getting more frequent with climate change.

## Real-world basis
*(Verify Romanian data and commitments.)*
- **EU ETS:** power plants and heavy industry need emission allowances; the price is set by the EU market (exogenous to Romania). Free allocation for some industries; auction revenue goes to the state.
- **Non-ETS sectors** (transport, buildings, agriculture, small industry) have national targets under EU law; a second EU ETS for buildings and road fuels is planned.
- Romania has committed to phasing out coal power by a set date as part of its recovery plan *(verify date)*.
- Air pollution (PM2.5, NO₂) comes from traffic, wood and coal heating, industry and power plants; it raises mortality and respiratory illness.
- Droughts sharply cut cereal and oilseed yields in some years; floods damage infrastructure and housing.

## State
| code_name | Meaning | Level |
|---|---|---|
| `emissions_co2e` | GHG emissions by source (plant, industry, transport, buildings, agriculture) | national, county |
| `ets_price` | EU carbon allowance price (exogenous scenario) | EU |
| `air_quality` | PM2.5 index | county |
| `forest_cover` | Forest area (carbon sink, tourism, logging) | county |
| `climate_trend` | Slowly rising probability and severity of extreme weather | national |

## Update rule
**Emissions:** emission factor × activity (fuel burned by each plant, output of each industry, fuel use of households' cars and heating, livestock and fertiliser use).

**Power generation by technology.** Each generation industry ([energy](energy.md)) has its own emission and pollution factors, applied to each plant's generation in its county *(typical values, verify against Romanian plant data)*:

| Industry | CO₂ (t/MWh) | ETS | Local air pollution |
|---|---|---|---|
| `power_coal` | ~1.0–1.2 (lignite highest) | yes | high (PM, SO₂, NOₓ) |
| `power_gas` | ~0.35–0.5 | yes | low–medium (NOₓ) |
| `power_biomass` | biogenic, counted as 0 | no | medium (PM) |
| `power_nuclear`, `power_hydro`, `power_wind`, `power_solar` | ~0 in operation | no | none |
| `power_storage` | none directly (emissions of the charging mix) | no | none |

**Carbon cost:** ETS sectors pay `ets_price` × (emissions − free allocation) → enters unit cost ([production](production.md), [energy](energy.md)); auction revenue → government. Non-ETS: national carbon tax and excise if the player sets them.

**Air quality** per county = background + Σ sources × dispersion factor; → health target in [social outcomes](../society/social-outcomes.md).

**Weather shocks (seeded, deterministic per save):** each month, probability of drought, flood, heatwave per region, rising with `climate_trend`.
| Shock | Effect |
|---|---|
| Drought | Yield loss for `agri_cereals`, `agri_oilseeds`, `agri_fruit_veg` (reduced by irrigation infrastructure); `power_hydro` capacity factor ↓ (and river cooling limits for `power_nuclear`, `power_coal`); food prices ↑ |
| Flood | Damage to infrastructure and dwellings (capital destroyed, booked as loss); reduced by flood defences |
| Heatwave | Electricity demand ↑, health ↓ (elderly), labour productivity ↓ outdoors |

## Player levers
Coal phase-out schedule (close `power_coal` plants), per-technology renewable subsidies and CfDs ([levers](../game/levers.md)), rooftop solar subsidies for households and firms (prosumers, [energy](energy.md)), national carbon tax on non-ETS fuels, vehicle emissions tax and fuel excise, subsidies for heat pumps and insulation, irrigation and flood-defence projects, afforestation and logging limits, emission standards for industry, public transport investment.

## Outputs
Emissions by sector and county, ETS costs paid, auction revenue, air quality map, extreme-weather events and their damage, progress vs EU targets.

## Acceptance tests
- [ ] A rise in the ETS price raises the wholesale electricity price when `power_coal` or `power_gas` plants are marginal, and raises costs in steel, cement and fertilisers.
- [ ] Shifting generation from `power_coal` to `power_solar` or `power_wind` lowers emissions by the difference in their emission factors.
- [ ] Closing coal plants lowers emissions and improves air quality in their counties.
- [ ] A drought lowers cereal output and raises food prices; irrigation reduces the loss.
- [ ] With the same seed, weather events are identical across runs.

## Open questions
- [ ] Should missing EU climate targets have consequences (fines, fund conditions) in v1?
- [ ] Do we model forest fires and illegal logging (a known Romanian issue) or keep forestry simple?
