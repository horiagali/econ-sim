---
id: society/interest-groups
title: Interest Groups
status: draft
owner: horia
depends_on: [society/population-groups, society/identity]
research: []
updated: 2026-10-08
---

# Interest Groups

## Purpose
Overlapping groups of people who share a concern, such as environmentalists, motorists, farmers or pensioners. A person can belong to many, each with an intensity between 0 and 1. Groups give policies a readable political footprint ("this fuel tax angers motorists and pleases environmentalists") and are how opinions react to policies beyond pure pocketbook effects. Model inspired by the Democracy series, but driven by the simulated economy.

## Membership
Each person $i$ has an intensity $m_{i,g} \in [0,1]$ for each group $g$.
- **Initial:** from attributes via a data-driven rule. Example: `motorists` = f(owns car, rural, commute distance, age, sex); `farmers` = f(works in agriculture, has farm land); `environmentalists` = f(education, age, urban, ideology).
- **Dynamics:** intensities drift toward the attribute-implied target (e.g. buying a car raises `motorists`; retiring raises `pensioners`) plus slow opinion drift.
- Group size (shown to player) = Σ weight × intensity.

## v1 group list (proposal)
| Group | Typical members | Cares about |
|---|---|---|
| Pensioners | retired | pension level and indexation, health, inflation |
| Public sector workers | public employees | public wages, headcount, job security |
| Private sector workers / unions | employees | wages, minimum wage, labour rights, payroll tax |
| Business owners & entrepreneurs | employers, self-employed | corporate and payroll taxes, regulation, interest rates |
| Farmers | agriculture workers, landholders | subsidies, input prices (fuel, fertiliser), food prices, tariffs |
| IT & tech workers | IT, data centres, high-skill professional | income tax on high earners, broadband, education |
| Students & youth | students, age 18–29 | student finance, tuition, youth jobs, housing cost |
| Parents & families | households with children | child benefit, schools, childcare |
| Motorists | car owners, commuters | fuel prices and excise, roads, car taxes |
| Environmentalists | educated, urban, young | emissions, coal plants, renewables, pollution |
| Religious traditionalists | high religiosity | church-related and family policy (mostly later); social conservatism |
| Nationalists / sovereigntists | various | national industry, foreign ownership, EU dependence, immigration |
| Pro-EU / pro-Western | educated, urban, young, diaspora links | EU funds, rule-following, openness |
| Anti-immigration | various | immigration quotas |
| Minority rights (Hungarian) | Hungarian ethnicity | regional development, language (later) |
| Roma communities | Roma ethnicity | social assistance, education, jobs |
| Rural communities | rural households | rural infrastructure, agriculture, local services |
| Homeowners & landlords | owners, rental income | property tax, rents, house prices |
| Tenants | renters | rents, housing benefit, public housing |
| Savers | high deposits | interest rates, inflation |
| Borrowers & mortgage holders | indebted households | interest rates |
| Patients & health advocates | poor health, elderly | health spending, hospitals |
| Diaspora families | households with family abroad | remittance costs, exchange rate, return incentives |
| Industrial workers | manufacturing, mining, energy employees | jobs in heavy industry, energy prices, plant closures |

## Stances
Each group has a **stance** on each relevant lever or indicator: direction, strength and threshold. Example: `motorists`: fuel excise ↑ → strongly negative; road investment ↑ → positive. Stances are data, not code. They feed approval ([opinion-approval](opinion-approval.md)) and are visible to the player ("who likes this policy?") before and after a decision.

## Outputs
Group size, average approval, mood trend, top grievances per group. Readable for the politics layer later (lobbying, party support).

## Acceptance tests
- [ ] Raising fuel excise lowers motorists' approval and raises environmentalists' approval within 1–2 ticks.
- [ ] Group memberships respond to life events (a person retiring joins `pensioners` with high intensity).
- [ ] Group sizes are consistent with the attributes they derive from (e.g. pensioners ≈ retired population).

## Open questions
- [ ] Add or remove groups? (E.g. LGBT rights, gun owners, hunters, the Orthodox Church as an institution, sports fans, digital nomads.)
- [ ] Should interest groups ever act (protests, lobbying) in v1, or only show opinions (current: only opinions)?
