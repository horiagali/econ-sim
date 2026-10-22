---
id: economy/transport-network
title: Transport Network & Market Access
status: draft
owner: horia
depends_on: [economy/infrastructure, economy/production, economy/trade-fx, economy/labor-market, adr/0006-determinism-contract]
research: []
updated: 2026-10-22
---

# Transport Network & Market Access

## Purpose
Make **where** a road or rail line goes matter, without modelling goods moving between counties. Romania is a graph of county seats. Transport projects ([infrastructure](infrastructure.md)) upgrade its links, and each county's **market access** follows from the shortest travel times to all the others, weighted by their economic size. Market access sets the productivity of firms in the county. Access to the ports and borders sets the transport cost of its exports and imports. Travel times also set commuting, migration and hospital catchments.

**Owner decision (2026-10-22):** a county graph in M11, rather than a per-county bonus (too blind to routes) or full trade between counties (too hard for v1).

Why Bucharest–Constanța beats a link to a small place: market access sums over every destination weighted by its GDP. A line between the largest economy and the main port cuts travel times on the routes that carry most weight, for both ends and for every county whose shortest path now uses it.

## Real-world basis
- **Market access** $MA_c = \sum_d Y_d \, \tau_{cd}^{-\theta}$ explains land values and output across US counties (Donaldson & Hornbeck 2016), with $\theta$ the trade elasticity (about 3 to 8). Its effect on productivity in Europe is an elasticity of about 0.03 to 0.08 (research note owed).
- New links redistribute activity as well as create it: peripheral places linked to a large centre can lose industry (Faber 2014).
- Romania in 2021 had about 960 km of motorway, the lowest density in the EU per head, and slow rail (average passenger speed under 50 km/h). Its main non-EU gateway is the port of Constanța; EU trade crosses mainly in the west (Nădlac, Borș) and at Giurgiu–Ruse.

## The graph
- **Nodes:** the 41 county seats and Bucharest, plus gateway nodes for the port of Constanța, the main western border crossings and the Danube bridges, and the main airports.
- **Links:** between neighbouring nodes. Each link carries layers by mode: road (its best class, from county road to motorway), rail (conventional, modernised, high-speed), air (between airports) and Danube shipping. Each layer has a length and a speed by class and condition.
- **The starting graph** comes from data: real distances and today's travel times by mode, built once in the Python tooling (for example from OpenStreetMap routing) into a scenario fixture.

## Costs
For a pair of nodes, by mode, the freight cost per unit of value is
$$\tau^{m}_{cd} = 1 + \frac{\text{distance} \times \text{cost per km}_m + \text{time} \times \text{value of time}_m}{\text{value of a typical load}}$$
The cost $\tau_{cd}$ is the cheapest route over all modes and paths. Passenger time, for commuting, catchments and migration, is the fastest route.

**Shortest paths** are computed with Dijkstra's algorithm on about 50 nodes, in integer arithmetic. Ties are broken by node order (determinism contract). They are recomputed only when a link changes: a finished project, or a change in a link's condition class, checked monthly. That costs microseconds.

## Effects
1. **Productivity.** Each county's firm units get a productivity factor
   $$A_c = \left(\frac{MA_c}{MA_c^{0}}\right)^{\varepsilon}$$
   It moves toward its new value gradually over several years, as firms reorganise. $MA^0$ is the starting access, so the starting state is unchanged.
2. **Gateways.** Each county's cost to its nearest EU border node and to the port of Constanța sets an export and import cost wedge for its units: EU trade through the border, non-EU trade by a mix of the port and the border ([trade-fx](trade-fx.md)).
3. **Commuting.** Counties within a commuting time of each other share part of their labour market ([labour market](labor-market.md): region already pools counties). A fast link widens that pool.
4. **Migration and catchments.** Travel time to services enters migration attractiveness (M5) and hospital catchments ([infrastructure](infrastructure.md)).
5. **Transport industries.** Faster, cheaper links lower the input needs of `road_freight` and `rail_transport` per unit, and so their prices.

> **Simplification:** goods are still sold in one national market per good. The network changes costs and productivity by county, not where goods are shipped. Full trade between counties stays after v1.

> **Simplification:** no congestion in v1. A link's speed depends on its class and condition, not on traffic.

> **Simplification:** a route is a sequence of links between nodes; where it runs inside a county is not modelled.

## On the map
The player draws a route by picking nodes. The game shows the links it upgrades, the cost estimate, and before building, the change in each county's market access and the largest gains. After it opens, the "why" panel traces a county's productivity to its market access, and that to the links that changed.

## State variables
Nodes, links by mode with length, class, speed and condition; the cost matrix $\tau_{cd}$; travel times; $MA_c$, $MA^0_c$ and $A_c$ by county.

## Player levers
None of its own: transport projects, maintenance and tolls are levers of [infrastructure](infrastructure.md).

## Tuning parameters
$\theta$; $\varepsilon$; the adjustment speed of $A_c$; cost per km and value of time by mode; the speed by class.

## Edge cases
- **An unconnected node** (a bridge closed by neglect): costs through it are routed around it; if no route exists, use a high cap, not infinity.
- **A link built twice:** the better class wins; no double counting.

## Acceptance tests (draft)
- [ ] **AC-NET-01** `[unit]` Shortest paths on a small graph match a hand calculation; ties are broken by node order; reruns are identical.
- [ ] **AC-NET-02** `[sim]` At the start, every county's productivity factor is exactly 1.
- [ ] **AC-NET-03** `[sim]` A high-speed line Bucharest–Constanța raises total market access more than a line of the same length and class between two small counties.
- [ ] **AC-NET-04** `[sim]` After a motorway opens, output in the counties it connects is higher years later than in a world without it, and the "why" panel names the links.
- [ ] **AC-NET-05** `[sim]` A western motorway lowers export costs for counties near the border more than for those in the east.

## Open questions
- [ ] $\theta$ and $\varepsilon$ for Romania: research note owed.
- [ ] Gateway nodes: which border crossings, and whether airports are nodes in v1 or only a passenger effect?
- [ ] Should a new link also lower productivity in places that lose activity to the centre (Faber)? Proposed: it follows from market access being relative only if wages and prices respond by county; not forced.
