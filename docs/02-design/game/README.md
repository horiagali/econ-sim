---
id: game/overview
title: Game — Player Role, Core Loop & Time
status: draft
owner: horia
depends_on: [vision/vision, vision/scope]
research: []
updated: 2026-10-10
---

# Game — Player Role, Core Loop & Time

## Player role
- The player is the **head of state** with full executive power over economic policy.
- **v1 has no politics layer.** No parliament, parties or elections; policies take effect without votes. A slider change is law.
- The player runs the central bank directly in v1. See [monetary policy](../economy/monetary-policy.md).
- **Approval** is tracked for every person, so for any group and nationally. In v1 it is just a number: shown and explained, with no effect on behaviour or game-over. See [opinion & approval](../society/opinion-approval.md).

## Game type
- **Sandbox.** No win or lose condition. The game runs until the player stops.
- The player picks their own goals. The UI makes it easy to track a few chosen indicators ("pinned goals").
- *Later:* scenarios with objectives, for example "bring inflation under 3% within 4 years without unemployment above 8%".

## The country: Romania
- v1 recreates **Romania**: its population, regions, industries, institutions, tax and benefit system, central bank, currency (leu, RON), EU membership and diaspora.
- **Geography:** the 41 counties + Bucharest (42 units) are the finest level for people, housing, schools, hospitals and infrastructure. The 8 **development regions** (Nord-Vest, Centru, Nord-Est, Sud-Est, Sud-Muntenia, București-Ilfov, Sud-Vest Oltenia, Vest) are the level for labour markets and industry capacity. Urban vs rural is a household attribute.
- **Start date: 1 December 2021,** the reference date of the census the population is built from (owner decision 2026-10-10). Law, budgets and balance sheets are those of that date.
- All starting data (population, I-O tables, tax rules, budgets, balance sheets) lives in data files, not code, so another country could be added later.
- "Plausible over precise" still applies: we aim for Romania's real shape and magnitudes, not decimal-exact reproduction.
- The **rest of the world** is aggregate; split into EU and non-EU is proposed. See [trade & FX](../economy/trade-fx.md).

## Time
- **1 tick = 1 day** (owner decision 2026-10-10, see D5 in the [roadmap](../../roadmap.md); the ADR for the time base is still to be written). Each process runs at its own period: markets and payments daily, wages, taxes and statistics monthly, budgets and school years yearly. Where this or another spec still says "tick" or "month" for a step of the simulation, read it as that process's period until the spec is reworded.
- Pausable real time with speeds (e.g. 1×, 2×, 4×, max). The player can change any lever while paused.
- Some sub-systems run less often: demographic transitions monthly; education cohorts yearly (school year); statistics releases monthly or quarterly.
- **Implementation lags are explicit per lever.** Tax changes apply from the next tick, new benefits after an admin lag, infrastructure after a build period of years. Each lever's lag is listed in [levers](levers.md).

## Core loop
```
observe (dashboards, graphs, "why" panel, group explorer, map)
   → decide (adjust levers, launch projects)
   → simulate (ticks advance; mechanisms propagate)
   → feedback (indicators move, people's finances and approval move, notifications)
   → observe …
```
Two nested rhythms:
- **Short loop (months):** react to prices, unemployment, budget, approval.
- **Long loop (years to decades):** education, infrastructure, energy mix, demographics, debt, productivity growth.

## Tick order (high level)
The detailed order belongs to the architecture docs. The intended causal order within a tick:
1. Apply player lever changes due this tick; progress projects.
2. Demography and person transitions (ageing, graduation, retirement, migration, births, deaths), with alignment.
3. Rest of world: exogenous prices, demand, rates for this tick.
4. Financial conditions: policy rate → bank rates, bond yields, exchange rate.
5. Production plans: firms set output targets from expected demand; buy inputs; hire and fire (labour market matching).
6. Prices and wages set (markup on unit cost, demand pressure, expectations).
7. Incomes paid: wages, profits, interest, rents, transfers. Taxes collected.
8. Spending: households consume per their baskets; firms invest; government purchases; exports and imports.
9. Goods markets clear (inventories absorb mismatch); financial flows settle; balance sheets update.
10. Social outcomes and opinion update (education, crime, health, approval).
11. Invariant checks (SFC, person conservation, no NaN). Statistics recorded; causal trace stored.

## Open questions
- [x] Tick length: one day, with mixed rates (2026-10-10).
- [x] Start date: 1 December 2021 (2026-10-10).
- [ ] Pinned goals or advisors that comment on trends: in v1 or later?
