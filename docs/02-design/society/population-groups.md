---
id: society/population-groups
title: Population Model — Synthetic Population & Groups
status: draft
owner: horia
depends_on: [society/overview, economy/accounting, adr/0003-people-representation]
research: [research/people-model-approaches]
updated: 2026-10-09
---

# Population Model — Synthetic Population & Groups

> This is the most important and most expensive part of the game. The representation is chosen in [ADR-0003](../../03-architecture/decisions/0003-people-representation.md) after the [research note](../../01-research/notes/people-model-approaches.md).

## Purpose
Represent Romania's population (~19 million people) in fine detail, so that every policy lands on concrete people with their own job, income, wealth, family, culture, religion, ideology and interests, and so that aggregates (GDP, unemployment, approval) are sums over people.

## Representation in one paragraph
The country is simulated as a **weighted synthetic population**: about 75,000 **households** containing about 190,000 **persons** at the default scale of 1 : 100. Each synthetic household has a **weight** (how many real households it represents, ~100). Every person carries every attribute as a column. **Groups** ("well-educated students in Cluj", "Hungarian-speaking farmers", "car lovers") are not stored; they are **filters** over persons, and their size is the sum of weights. This replaces the earlier "pop cell" design, which could not scale to this many attributes.

## Household attributes
| code_name | Meaning | Type |
|---|---|---|
| `hh_weight` | Real households represented (integer; see [ADR-0003](../../03-architecture/decisions/0003-people-representation.md)) | int |
| `hh_county` | One of Romania's 41 counties + Bucharest | category (42) |
| `hh_region` | Development region (derived from county) | category (8) |
| `hh_urban` | Urban / rural | bool |
| `hh_tenure` | Owner outright, owner with mortgage, private renter, social renter, rent-free (family) | category |
| `hh_dwelling_value` | Market value of dwelling | LCU |
| `hh_farm_land` | Agricultural land owned or used (ha); subsistence production | float |
| `hh_vehicles` | Number of cars | int |
| Balance sheet | `hh_cash`, `hh_deposits`, `hh_bonds`, `hh_equity`, `hh_pension_fund`, `hh_mortgage`, `hh_consumer_debt`, `hh_student_debt` | LCU |
| `hh_remittances_in` | Money received from family abroad | LCU / tick |
| `hh_income_class` | Derived from equivalised disposable income vs national median | category (5) |

## Person attributes
| Group | Attributes |
|---|---|
| Demography | `age` (months), `sex`, `household_id`, `role` (head, partner, child, other relative), `born_abroad`, `returned_migrant`, `has_family_abroad` |
| Education | `edu_level` (ISCED groups: ≤2, 3, 4, 5–6, 7–8), `edu_field` (STEM, health, business & law, social & humanities, education, agriculture, technical-vocational, none), `enrolled` (level being studied), `years_in_level` |
| Work | `activity` (child, pupil, student, employee, self-employed, employer, unpaid family worker, unemployed, inactive-homemaker, inactive-disabled, inactive-other, retired), `occupation` (ISCO-08 major group, sub-major later), `industry` (catalogue id), `employer_type` (private domestic, private foreign-owned, SOE, public sector), `formal` (formal / informal), `wage`, `hours`, `tenure_months`, `months_unemployed` |
| Income (per tick) | wage, self-employment, pension, benefits, other; taxes and contributions paid |
| Health | `health` (0–1), `disability` |
| Identity ([identity](identity.md)) | `ethnicity`, `mother_tongue`, `religion`, `religiosity` (0–1) |
| Opinion ([opinion-approval](opinion-approval.md)) | ideology axes, issue salience, `approval` (0–100) |
| Interests ([interest-groups](interest-groups.md)) | membership intensity 0–1 for each interest group |

Children are persons too: they consume through their household, attend school, and age into adults.

## Initial population (data pipeline)
1. Start from microdata or cross-tables: Romanian Census 2021 (INS), the household budget survey (ABF/HBS), EU-SILC (income and living conditions), the Labour Force Survey, and religion and ethnicity tables from the census.
2. Build synthetic households by sampling and **raking** (iterative proportional fitting) so that county × age × sex × education × activity × ethnicity × religion marginals and key joint tables match official totals.
3. Impute wealth, debt and interest-group memberships from survey relationships (e.g. car ownership → motorist interest).
4. Validate against published aggregates: employment, income distribution (Gini), poverty rate, regional GDP shares.

See the [research backlog](../../01-research/README.md). The pipeline lives in `tools/` or `data/` later (needs an ADR, since it's a new top-level folder).

## Dynamics (each tick, with alignment)
| Transition | Driven by | Owner spec |
|---|---|---|
| Ageing | time | this spec |
| Births (new child person in the mother's household) | fertility model | [demographics](../economy/demographics.md) |
| Deaths | mortality × health | [demographics](../economy/demographics.md) |
| Household formation and dissolution (young adults leave home, couples form, widowhood) | age, income, housing cost | this spec (simple rates in v1) |
| Enrolment, graduation, dropout | [education](education.md) | |
| Hires, separations, job and industry changes, self-employment | [labor-market](../economy/labor-market.md) | |
| Retirement | pension rules, health | [social-transfers](../economy/social-transfers.md) |
| Internal migration (household moves county) | income and job gaps, housing cost | [demographics](../economy/demographics.md) |
| Emigration, return migration, immigration | wage gap abroad, unemployment, immigration policy | [demographics](../economy/demographics.md) |
| Ideology drift, interest-group membership changes | life events, experience, information | [opinion-approval](opinion-approval.md), [interest-groups](interest-groups.md) |

**Alignment rule:** for each transition and stratum (e.g. county × age band × sex), compute the expected number of transitions from the rate model; then select exactly that many persons, ranked by their individual probability plus seeded noise. Aggregates are then exact and deterministic, while who transitions stays realistic.

**Money moves with people.** A person leaving a household to form a new one takes a share of household liquid assets per rule; a household that moves keeps its balance sheet; emigrants take liquid assets abroad (capital outflow); the dead leave assets to their household, or to heirs in other households if alone. Every move is booked as a transfer ([accounting](../economy/accounting.md)).

## Groups (views)
- A **group** is a saved filter over person and household attributes, e.g. `edu_level ≥ 5–6 AND activity = student AND county = Cluj`.
- Built-in groups: by region and county, age band, sex, education, activity, occupation, industry, income class, ethnicity, religion, urban or rural, and every interest group.
- Player-defined groups can be saved and pinned; any indicator can be shown for any group (size, income, taxes, transfers, spending, inflation, wealth, unemployment, health, approval).
- For interest groups, a person counts with their membership intensity (weight × intensity).
- The group explorer shows the **sample size** (synthetic persons) and flags estimates from fewer than ~200 synthetic persons as uncertain.

## Invariants (every tick)
1. **Person conservation:** Σ weights of persons changes only by births − deaths + immigrants − emigrants.
2. **Household wealth conservation** under moves, splits and merges.
3. No negative ages, weights or sizes; every person belongs to exactly one household.

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `sample_scale` | 1 : 100 (development default) | Real people per synthetic person. A parameter, never hardcoded; CI runs at several scales ([ADR-0003](../../03-architecture/decisions/0003-people-representation.md)) |
| `oversample[ethnicity]` | Hungarian ×2, Roma ×3 | Lower noise for minorities (weights adjusted) |
| `min_group_sample` | 200 | Below this a group statistic is flagged uncertain |
| `rng_seed` | per save | Determinism |

## Acceptance tests
- [ ] Starting population matches census totals by county, age, sex, ethnicity and religion within 1%.
- [ ] Person conservation holds exactly every tick for 100 years.
- [ ] Two runs with the same seed and inputs produce identical results.
- [ ] National unemployment computed from persons equals the labour-market aggregate exactly.
- [ ] Changing `sample_scale` from 1 : 100 to 1 : 200 changes national indicators by less than sampling error.

## Open questions
- [ ] Default scale 1 : 100 or finer (1 : 50) if performance allows?
- [ ] Household formation in v1: simple rates (proposed), or partner matching?
- [ ] Do we let the player inspect individual synthetic persons ("meet a citizen")? Fun and explainable, but they are statistical stand-ins.
