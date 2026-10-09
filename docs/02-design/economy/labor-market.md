---
id: economy/labor-market
title: Labour Market
status: draft
owner: horia
depends_on: [economy/production, society/population-groups, society/education, economy/demographics]
research: []
updated: 2026-10-08
---

# Labour Market

## Purpose
Match persons to jobs (industry × occupation × region), set wages, and produce employment, unemployment and wage income per person. This is where "more education → better jobs" lives.

## Real-world basis
- Search-and-matching (Diamond–Mortensen–Pissarides): vacancies and job seekers meet via a matching function; flows of hires and separations.
- Wage Phillips curve: wage growth rises with labour-market tightness, expected inflation and productivity growth.
- Minimum wages raise low-skill pay with modest employment effects up to a point (bite measured by min/median ratio), large effects well above ~60% of median.
- Unemployment benefits raise reservation wages and search duration.
- Romania-specific (verify): a large share of employment in agriculture, much of it unpaid family work and subsistence farming; significant informal work; regional gaps between Bucharest-Ilfov and the rest; heavy emigration of workers.

## Jobs: industry × occupation × region
- **Occupation** uses ISCO-08 major groups: managers; professionals; technicians and associate professionals; clerical support; service and sales; skilled agricultural; craft and trades; plant and machine operators; elementary occupations; armed forces (inactive until military).
- Each industry needs a mix of occupations per unit of output (data, from Labour Force Survey and I-O). E.g. `it_software` is mostly professionals; `agri_crops` mostly skilled agricultural and elementary.
- **Skill tier** (low, mid, high) is derived from occupation and used for education matching:

| Education | Qualifies for |
|---|---|
| ISCED ≤ 2 | low-skill occupations |
| ISCED 3–4 | low, mid |
| ISCED 5–8 | low, mid, high (field of study improves match for matching occupations) |

Workers in jobs below their qualification are **underemployed**.

- **Region:** labour markets clear at the level of the 8 development regions, with commuting between neighbouring counties. *(Simplification; county-level markets would be heavier.)*

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $V_{j,o,r}$ | `vacancies` | Open positions by industry, occupation, region | jobs |
| $W_{j,o}$ | `wage[j,o]` | Wage by industry and occupation (persons also have individual wages around it) | LCU / month |
| $u$ | `unemployment_rate` | National; any group by filtering persons | ratio |
| $W^{min}$ | `minimum_wage` | Gross minimum wage | LCU / month |
| — | `informal_share` | Share of employment that is informal, by industry | ratio |

## Update rule (per tick)
1. **Labour demand:** industries' required employment by occupation and region (from production). Vacancies where demand > staff; layoffs where staff > demand, at most $x\%$ per tick (firing friction).
2. **Separations:** layoffs + quits (person-level probability by tenure, age, contract) → unemployed or inactive.
3. **Matching** per (occupation tier, region):
```math
H = \mu \, U^{\alpha} V^{1-\alpha}
```
Hires are allocated among job seekers by alignment, weighted by education fit, field of study, experience, past occupation and (optionally) a discrimination parameter ([identity](../society/identity.md)). Overqualified seekers may take lower-tier jobs if their tier is slack.
4. **Individual wages:** a person's wage = $W_{j,o}$ × personal factor (experience, education quality, tenure), so wage dispersion within jobs is realistic and income taxes bite correctly.
5. **Wage setting** per industry × occupation (gradual, annual-contract-like with monthly smoothing):
```math
\frac{\Delta W_{j,o}}{W_{j,o}} = \pi^e + g^{A}_j + \beta_1 (\text{tightness}_{o} - \text{tightness}^*) + \beta_2 \cdot \text{profitability}_j
```
with $W \ge W^{min}$ for formal jobs.
6. **Informal work** (full spec: [informal-economy](informal-economy.md)): a share of low-wage jobs (agriculture, construction, hospitality, personal services) is informal; the share rises with labour tax wedge and minimum-wage bite and falls with enforcement. Informal workers pay no taxes or contributions and don't qualify for unemployment benefits or contributory pensions.
7. **Self-employment and farming:** persons with farm land can work their own plot (unpaid family workers / subsistence farmers); this is the fallback when no job is found in rural areas.
8. **Public sector** hiring follows ministry headcount targets and public wage policy.

> **Simplification:** no explicit unions or strikes in v1; bargaining strength is a parameter.

## Player levers
Minimum wage; unemployment benefit (via reservation wage and search intensity); public wages and headcount; education (skill supply); immigration quotas by skill; wage subsidies; social contributions; labour inspection funding (informality).

## Outputs
Employment and unemployment for any group; wages by industry, occupation and region; labour share; underemployment; informality; vacancy rate; wage inflation.

## Edge cases
- Minimum wage above the productivity of low-skill jobs in some industries → those jobs disappear, become informal, or get automated over time.
- Very tight markets → wage-price spiral risk (with prices-inflation).
- Zero seekers or zero vacancies → matching function handles 0.

## Acceptance tests
- [ ] A demand boom lowers unemployment and raises wage growth with a lag of a few ticks.
- [ ] Raising the minimum wage from 45% to 55% of the median raises low-skill wages with a small employment effect; to 80% with a large one and more informality.
- [ ] More tertiary graduates raise high-skill employment if demand exists, otherwise underemployment rises.
- [ ] Higher unemployment benefits raise average unemployment duration.
- [ ] Employment counted from persons equals employment demanded by industries (after matching) every tick.

## Open questions
- [ ] ISCO major groups (10) enough, or sub-major groups (~40) for finer jobs?
- [ ] Hours (part-time) as a margin, or headcount only (proposed)?
