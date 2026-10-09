---
id: society/opinion-approval
title: Opinion, Ideology & Approval
status: draft
owner: horia
depends_on: [society/population-groups, society/interest-groups, economy/households]
research: []
updated: 2026-10-08
---

# Opinion, Ideology & Approval

## Purpose
Each person has political leanings, interests and a view of the head of state. **In v1 approval is just a number:** it is computed, shown and explained, but it does not feed back into behaviour or the economy. It's the politics-ready output for later.

## Real-world basis
- Economic voting: approval tracks real income growth, unemployment and inflation (inflation is very unpopular, even with rising wages).
- Pocketbook (own finances) and sociotropic (national economy) evaluation both matter.
- Ideology and group identity shape which policies people like regardless of personal gain.
- Loss aversion: losses hurt approval more than equal gains help.

## Per-person opinion state
| code_name | Meaning |
|---|---|
| `ideo_econ` | Economic axis: −1 (state-led, redistribution) … +1 (market, low taxes) |
| `ideo_social` | Social axis: −1 (progressive) … +1 (traditional) |
| `ideo_national` | National axis: −1 (pro-EU, open) … +1 (sovereigntist, closed) |
| `salience[issue]` | Weight on issues: inflation, jobs, own income, taxes, pensions, health, education, crime, environment, corruption (later), immigration, national independence |
| `interest[g]` | Interest-group intensities ([interest-groups](interest-groups.md)) |
| `approval` | 0–100 approval of the head of state |

Ideology starts from attributes (age, education, religiosity, urban/rural, ethnicity, income, region) per research, and drifts slowly with experience (long unemployment, rising wealth, emigration of family).

## Approval update (per person, per tick)
Target approval is a sum of separately stored components:
```math
A^*_i = 50 + \underbrace{\alpha_m M_i}_{\text{pocketbook}} + \underbrace{\alpha_s S}_{\text{national economy}} + \underbrace{\alpha_g \sum_g m_{i,g}\, \text{stance}_g(\text{policy})}_{\text{interest groups}} + \underbrace{\alpha_p \text{Align}_i(\text{policy}, \text{ideology})}_{\text{ideology}}
```
- $M_i$: change in the person's real disposable income (household, equivalised) and personal unemployment risk vs a slowly moving reference point; losses weighted ×2.
- $S$: national inflation, unemployment and growth, weighted by the person's salience.
- Stances and alignment are evaluated only on levers that changed recently, with decaying memory (people forget).

Smoothing: $A_{i,t+1} = A_{i,t} + \lambda_A (A^*_i - A_{i,t})$, clipped to 0–100.

Every component is stored, so the explorer can say: *"Students in Cluj: approval 58 (+8 this year): grant increase +6, lower youth unemployment +3, inflation −1."*

## Outputs
Approval per person → any group's average (weight-averaged), national approval, interest-group approval, components and their trends.

> **Simplification (v1):** approval has **no** behavioural effects (no strikes, protests, extra emigration, or tax evasion from low approval). Those come with the politics layer.

## Tuning parameters
`alpha_*`, `lambda_A`, loss-aversion factor, memory decay, ideology drift rates, stance tables (data).

## Acceptance tests
- [ ] A sharp rise in inflation lowers approval in almost all groups, most among pensioners and low-income households.
- [ ] Raising the top income tax rate lowers approval among high earners and business owners and raises it among people with left economic ideology, even those who don't benefit materially.
- [ ] A pension increase raises pensioners' approval within 1–3 ticks.
- [ ] Approval reacts more to a 5% real income loss than to a 5% gain.
- [ ] Changing approval parameters never changes any economic variable (v1 isolation test).

## Open questions
- [ ] Are three ideology axes enough, or do we want a fourth (e.g. authoritarian vs liberal-democratic)?
