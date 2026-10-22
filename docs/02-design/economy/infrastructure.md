---
id: economy/infrastructure
title: Infrastructure & Public Projects
status: draft
owner: horia
depends_on: [economy/fiscal-policy, economy/production, economy/transport-network, economy/eu-funds, economy/labor-market, economy/state-enterprises, society/education, society/social-outcomes]
research: []
updated: 2026-10-22
---

# Infrastructure & Public Projects

## Purpose
Let the head of state build things in a place: roads, motorways, rail, high-speed rail, airports, ports, broadband, water networks, schools, universities and hospitals. Power plants and public housing use the same **project system** ([energy](energy.md), [housing](housing.md)). A project is demand while it is built: a construction company wins the contract, hires workers in the county and buys materials. The finished asset is public capital that has to be run and maintained. It changes something real in its place: travel times on the [transport network](transport-network.md), hospital beds and school places in its county.

**Owner decisions (2026-10-22):**
- **Position matters through a county graph**, in M11. Transport assets upgrade the links of a graph of the 41 counties and Bucharest, and their effect comes from how much they improve each county's market access. A Bucharest–Constanța high-speed line matters far more than a link to a small place ([transport network](transport-network.md)).
- **The player picks the contractor from bids.** Every construction company with spare capacity bids a price and a delivery time. By default the cheapest bid wins, and the player may choose another.
- **A finished hospital or school adds capacity and has running costs.** It needs staff, hired from its county's labour market, and inputs such as medicines, paid from the budget. Capacity that is not staffed or funded gives less.

## Real-world basis
- Public capital raises output, with an elasticity of about 0.08 to 0.12 in meta-analyses (Aschauer; Bom & Ligthart 2014). Returns diminish, and they vary widely with project quality.
- Where a transport link goes matters through market access: Donaldson & Hornbeck (2016) for US railroads, and Faber (2014) for Chinese motorways, where peripheral counties lost industry to the centre.
- Construction lags and cost overruns are common, worst in large rail and road projects (Flyvbjerg).
- Romanian motorways are often built by foreign contractors (Turkish, Italian, Spanish, Austrian) or by joint ventures with domestic firms. *Research note owed:* contractor shares, unit costs per km, build times.
- Under-maintenance destroys value faster than depreciation does.
- Romania has hospital beds per head above the EU average but too few staff per bed. Many schools run in shifts, and about 380,000 pupils are above the school places in the starting population ([stage K](../society/population-inactivity-enrolment.md)).

## The project system

### 1. A project
| Field | Meaning |
|---|---|
| `type` | motorway, expressway, national road upgrade, rail modernisation, high-speed rail, airport, port, broadband, water network, school, university, hospital (energy and housing types are in their specs) |
| `place` | a county for buildings and area networks; a route of graph links between county seats for transport ([transport network](transport-network.md)) |
| `size` | km, beds, school places, passengers a year, … by type |
| `estimate` | cost and duration from per-type parameters × size × the county's terrain factor |
| `funding` | the budget, EU funds (programme and co-financing rate, [eu-funds](eu-funds.md)) or a mix |
| `contract` | the contractor, the price, the planned months, the progress paid so far |
| `state` | planned, tendered, building, paused, cancelled, operating |

### 2. Tender (monthly, for a project the player commissions)
Each construction unit (`construction_civil` for networks, `construction_buildings` for buildings) with spare capacity bids. Until M9 these are cohort units, one per county and size class. From M9 the named companies bid, among them a state-owned one. A foreign contractor (rest of the world, EU or non-EU) bids too.
- **Capacity.** A unit can take work up to its capacity (staff, capital) minus its order book.
- **Price** = the estimate × the unit's cost level × (1 + markup) × (1 + a load premium that rises as the order book fills) × (1 + distance premium when the unit is far from the site on the graph).
- **Delivery time** = the work spread over the capacity the unit has free.
- **The state-owned company** bids with the cost level and efficiency of its directives. It may be more expensive than private bidders, or have no spare capacity. It can be ordered to take a contract at cost ([state enterprises](state-enterprises.md)).
- **A company with no spare capacity can still get the contract** (owner, 2026-10-22). The project waits in its queue and starts when work ends elsewhere. Its bid shows the expected start and delivery date.
- **A foreign contractor** brings its own capacity, so it is never full. Its margin and the staff it brings are imports; it still buys local materials and hires local workers.

The player sees each bid with its price, time, the share that stays in Romania and the overrun risk, and picks one. The default is the cheapest bid that can start within a set number of months. If no company has spare capacity, the tender is re-run the next month and raises a notification, unless the player queues it with one company.

**A long order book makes a company grow** (owner, 2026-10-22):
- **Price:** its bids rise with its load (the load premium above).
- **Capacity:** its order book counts as expected demand in the [investment rule](investment-capital.md), so a company whose queue is long for months hires and invests, and its capacity grows with the rule's lag.
- The same rule applies to every firm: one that cannot meet demand raises its markup with its utilisation ([prices](prices-inflation.md)) and expands through investment. *The investment rule is built and switched off until M7 calibration.*

> **Simplification:** one bid per unit; no bid rigging or corruption in v1 (that is [state capacity](state-capacity.md), M16, which then raises prices and overruns).

### 3. Building (each day the contractor works on it)
- The contractor does a day's share of the work, limited by its capacity across all its contracts.
- **Materials:** it buys by its industry's input column (steel, building materials, machinery, fuel), so supply chains feel the project.
- **Workers:** it hires through the [labour market](labor-market.md) in the project's county, so local employment and wages rise. A foreign contractor brings part of its own staff.
- **Payment:** the government pays the contract price by progress, monthly, under `gov.investment`, through the ledger. An EU-funded share is claimed from the EU by the rules of [eu-funds](eu-funds.md), with their lag and the risk that ineligible costs fall back on the budget.
- **The contractor's result:** price paid minus its real cost. A loss-making contract can slow work or end in a claim for more money (the overrun).
- **Overruns and delays** are drawn from the project type's risk, raised by the contractor's load and lowered by state capacity. Drawing uses the keyed RNG, so a run is deterministic.
- **Player actions while building:** pause it, which keeps the site but makes the contractor lay off workers; cancel it, which loses the money spent and pays a penalty; or accept an overrun claim or refuse it, which risks a stop.

When many projects run at once, construction capacity fills up. Bids then rise, delivery times lengthen and construction wages rise. That crowding is visible and has a cause the player can see.

### 4. Completion and the asset register
A finished project becomes an asset with fields `type`, `place`, `capacity`, `value` (cost × an efficiency set by state capacity), `condition` (0 to 1), `age` and `operator`. Existing assets are loaded from data at the start (see Data).

### 5. Operating
Each asset has an operator, and the operator pays its running costs:

| Asset | Operator | Running cost |
|---|---|---|
| Roads and motorways | the roads agency (a government unit) | maintenance; tolls or a vignette as revenue (lever) |
| Rail | the rail infrastructure company (state-owned) | maintenance; track access fees from rail operators |
| Airports, ports | state-owned or county-owned units | staff, maintenance; fees from airlines and shippers |
| Hospitals | the county's public health producer (`health_services`, public part) | staff by a beds-to-staff norm; inputs by the health column (pharmaceuticals, medical devices, energy, food) |
| Schools, universities | the county's public education producer (`education_services`, public part) | teachers by a pupils-per-class norm; inputs by the education column |
| Water networks | county water companies | staff, energy, chemicals; tariffs |

**Schools and hospitals are run from their ministry's budget** (owner, 2026-10-22). The education and health budgets are levers ([fiscal policy](fiscal-policy.md), M4). Each has a **running part**, which pays staff and inputs for all the ministry's schools or hospitals, and an **investment part**, which pays for new projects and maintenance. The split between them is a lever.
- **Running part below the cost of the capacity:** the money is spread over all of it. Staff are thinner, classes larger and medicines shorter, so **effective capacity** = physical capacity × the staffed share, and quality falls. The "why" panel shows "unstaffed beds" or "classrooms without teachers", by county.
- **Running part above the cost:** the extra raises quality: more spent per pupil or patient, better teacher and doctor pay (easier hiring), smaller classes ([education](../society/education.md), [social outcomes](../society/social-outcomes.md)). Returns diminish, and the game shows the quality each extra leu buys, so the player can see when moving money to the investment part, to build where places are short, does more.
- **Unspent money** of the running part at year end returns to the treasury and lowers the deficit; nothing disappears.

At commissioning, the game shows the new asset's running cost and the ministry's running part after it, so the player can raise the budget or accept thinner staffing. Hiring comes from the county's labour market, and doctors and teachers are scarce skills, so a hospital in a county short of doctors fills slowly, or draws doctors from neighbouring counties.

### 6. Maintenance and decay
Condition falls at $\delta_{type}$ a year and is restored by maintenance spending (a budget line by type, lever). Below a threshold, an asset's effect falls: a road's speed on its graph link drops, a hospital loses beds. Repairing it later costs more than maintaining it would have.

## Effects of operating assets
| Type | Effect | Through |
|---|---|---|
| Motorways, roads, rail, high-speed rail | shorter travel times on graph links → each county's market access and access to ports and borders → productivity of firms in the county, transport costs of exporters and importers; commuting and migration | [transport network](transport-network.md) |
| Airports | shorter passenger travel to EU nodes; tourism in the county; business services' productivity | transport network |
| Ports | cheaper non-EU sea trade through the port node | transport network, [trade-fx](trade-fx.md) |
| Broadband | productivity of IT, professional services and data centres in the county; remote work | production |
| Water networks | health; a condition for some industries (food, chemicals) to expand in the county | production, social outcomes |
| Hospitals | effective beds per head in the county and its catchment by travel time → access to care → health, sick days, life expectancy (M14); demand for pharmaceuticals and medical devices; jobs | [social outcomes](../society/social-outcomes.md) |
| Schools | school places in the county → enrolment of pupils who had no place; fewer shifts and smaller classes → education quality → graduates' skills and wages, years later (M14) | [education](../society/education.md) |
| Universities | tertiary places → enrolment, skills | education |

A school or hospital matters most where there was a shortage. Effects use places per pupil and staffed beds per head against need, with diminishing returns, so the tenth hospital in Bucharest adds less than the first one in an under-served county.

## Player levers
- Commission a project: type, place (a county, or a route drawn on the map between county seats), size, funding.
- Pick the contractor from the bids.
- Pause, cancel, accept or refuse an overrun.
- Education and health budgets, and each one's split between running and investment (M4 levers; new capacity is funded from them).
- Maintenance budget by type.
- Tolls or a vignette on motorways; track access fees.
- From M9: directives to the state-owned construction company (take contracts at cost, hire locally).

## Data (owed, research note)
- **Existing stocks by county:** km of motorway, expressway and national road by class; km of rail by speed; airports with passenger counts; ports; hospital beds and staff (INS); classrooms and school places by level (INS), and schools running in shifts.
- **Unit costs and build times per type in Romania:** motorway per km by terrain, rail modernisation, high-speed rail (European comparators), a regional hospital per bed, a school per place.
- **Construction companies:** capacity and order books (largest companies, M9), the state-owned one, the foreign share of large contracts.

## Tuning parameters
Per type: unit cost, duration, terrain factors, overrun and delay risk, lifetime, $\delta_{type}$, staffing norms, running cost per unit of capacity. Contractor markup and the load premium's slope. Effect elasticities: in the transport network spec for transport, and in education and social outcomes for schools and hospitals.

## Edge cases
- **A project in a county with no free construction workers:** the contractor hires from neighbouring counties (graph commuting), or wages rise.
- **Cancelling** after most of the money is spent: the spent value is lost, and the asset is not created.
- **EU funding withdrawn** (missed deadline, ineligible costs): the budget pays the rest, or the project stops.
- **A new hospital and no doctors anywhere:** its effective capacity stays near zero, and the game says so.
- **Projects past the end of the construction industry's capacity** queue; none breaks the simulation.

## Acceptance tests (draft)
- [ ] **AC-INF-01** `[sim]` A large motorway contract raises construction employment and output in its county, and purchases of building materials and steel, while it is built.
- [ ] **AC-INF-02** `[sim]` Every payment to a contractor goes through the ledger under `gov.investment`. EU-funded parts are claimed and received by the rules of eu-funds.
- [ ] **AC-INF-03** `[sim]` With several large projects at once, construction bids rise and delivery times lengthen against a world with one project.
- [ ] **AC-INF-04** `[sim]` When the cheapest bidder is full, the next bid wins, and a more expensive state-owned bid is never the default. A contract given to a full company waits in its queue and starts when it has room.
- [ ] **AC-INF-10** `[sim]` With the investment rule on, a construction company whose order book stays above its capacity for months raises its bids and its capacity, and its queue shortens.
- [ ] **AC-INF-05** `[sim]` A foreign contractor's margin shows as imports. A domestic contractor's margin shows as domestic profit.
- [ ] **AC-INF-06** `[sim]` A finished hospital with its running budget funded hires staff in its county and buys pharmaceuticals. Without the budget, its effective beds are lower, and the gap is reported.
- [ ] **AC-INF-07** `[sim]` A school built in a county with unplaced pupils raises enrolment there. One built where places already exceed pupils raises it by much less.
- [ ] **AC-INF-08** `[sim]` Cutting maintenance to zero lowers condition gradually; road speeds on the graph fall, and repair costs more later.
- [ ] **AC-INF-09** `[sim]` Cancelling a project loses what was spent and creates no asset; money is conserved.

## Open questions
- [ ] Private buildings (commercial, housing) through the same system, or through investment-capital and housing (M13)? Proposed: through those specs; the project system is for public assets and power plants.
- [ ] Public–private partnerships and concessions (a private builder paid by availability fees over 25 years): v1 or later? Proposed: later.
- [ ] Should the player see the overrun risk as a number or as a rating? Proposed: a rating, with the drawn result explained when it happens.
- [ ] Who bids before M9 has named companies: the construction cohort units by county and size class, as above?
