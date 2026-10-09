---
id: society/social-outcomes
title: Social Outcomes — Crime, Health, Wellbeing, Poverty
status: draft
owner: horia
depends_on: [society/population-groups, society/education, economy/fiscal-policy]
research: []
updated: 2026-10-08
---

# Social Outcomes — Crime, Health, Wellbeing, Poverty

## Purpose
Translate economic and policy conditions into outcomes people care about and that feed back into the economy: crime (cost, investment climate), health (labour supply, mortality, productivity), poverty and wellbeing.

## Real-world basis
- Crime rises with unemployment (esp. youth), inequality and low education; falls with policing and education (Becker economics of crime; education-crime literature).
- Health improves with income, education and health spending, with diminishing returns; life expectancy correlates with GDP per capita (Preston curve).
- Poverty measured relative (below 60% of median equivalised income) and absolute.

## Crime
**State:** `crime_rate` per region (offences per 1,000 / year), `policing_capacity` per region.
```math
\text{crime}_{r} = c_0 \cdot f(u^{youth}_r,\ \text{Gini}_r,\ \text{edu}_r,\ \text{poverty}_r) \cdot g(\text{policing}_r)
```
with $g$ decreasing and diminishing. Effects: property-crime losses (wealth transfer + destruction recorded in accounts), lower investment confidence in the region, health penalties, emigration push.

## Health
**State:** `health` per person; `health_system_capacity` per region (hospitals, staff).
- Health target per person = f(age, sex, income class, education, health spending per capita, capacity, air quality from [environment](../economy/environment.md), heatwaves, unemployment).
- Effects: mortality (demographics), labour-force participation (sick → inactive), productivity, healthcare consumption demand.

## Wellbeing and poverty
- `poverty_rate` (relative and absolute), `gini`, `real_income_per_capita` for any group.
- A composite wellbeing index (income, health, safety, employment, education) as an output for the player and politics later.

## Player levers
Police and justice budget, health budget, hospitals (infrastructure), education (see education), transfers (poverty), power plant choice (pollution).

## Acceptance tests
- [ ] A recession raising youth unemployment by 10 points raises crime in the affected regions within 12 ticks.
- [ ] Doubling police spending lowers crime with diminishing returns.
- [ ] Higher health spending raises life expectancy slowly (years), more where the baseline is low.
- [ ] Raising minimum income lowers the absolute poverty rate within 3 ticks.

## Open questions
- [x] Pollution and environment → [environment](../economy/environment.md).
- [ ] Should crime have a black-market economy component (untaxed income)?
