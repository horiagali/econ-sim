---
id: society/education
title: Education
status: draft
owner: horia
depends_on: [society/population-groups, economy/fiscal-policy]
research: []
updated: 2026-10-08
---

# Education

## Purpose
Turn spending, student finance and family background into enrolment, graduation and skills. Skills feed labour productivity, wages, crime and health. Education is the main long-run growth lever.

## Real-world basis
- Mincer returns to schooling (~8–10% wage per extra year, more for tertiary in many countries).
- Enrolment responds to cost (tuition, foregone earnings), student support and parental income; dropout is higher for low-income students.
- Education quality (spending per pupil, teacher pay, class size) has a measurable but modest effect on outcomes.

## State variables
| Symbol | code_name | Meaning | Unit |
|---|---|---|---|
| $Q^{edu}_{r,\ell}$ | `edu_quality` | Quality per region and level (primary, secondary, vocational, tertiary) | index 0–1 |
| $\text{Cap}_{r,\ell}$ | `edu_capacity` | Places available (schools, universities built) | students |
| $\text{Teach}_{r,\ell}$ | `edu_teachers` | Teachers employed (public sector employment) | persons |

Enrolment and attainment are person attributes (`enrolled`, `edu_level`, `edu_field`).

## Inputs
Education ministry budget and its split by level (fiscal-policy); school and university infrastructure (infrastructure); student finance and tuition (social-transfers); household income class and parents' education; graduate wage premium (labor-market); youth unemployment (low job prospects raise enrolment).

## Outputs
Transitions between education levels; skill composition of the labour force; `edu_quality` (read by labor-market productivity).

## Update rule (per tick; school-year events yearly)
**Enrolment probability** for a person eligible for level $\ell$ (aligned to capacity):
```math
p^{enrol}_{k,\ell} = \text{logistic}\big(\beta_0 + \beta_w \cdot \text{premium}_\ell + \beta_f \cdot \text{support}_k - \beta_c \cdot \text{cost}_{k,\ell} + \beta_q \cdot q_k + \beta_u \cdot u^{youth}_r\big)
```
capped by `edu_capacity`. If capacity binds, quality falls (crowding).

**Completion** (per year of study): $p^{complete} = f(Q^{edu}, \text{support}_k, q_k)$, else dropout back to the labour market with the previous education level.

**Quality:**
```math
Q^{edu}_{t+1} = Q^{edu}_t + \lambda \big(Q^*(\text{spend per student}, \text{teacher pay ratio}, \text{class size}) - Q^{edu}_t\big)
```
Slow adjustment (λ small: years).

**Skill effect:** worker effective skill $=$ base by education level × $(1 + \gamma \cdot Q^{edu}$ when educated$)$. Feeds the labour productivity index in [production](../economy/production.md).

> **Simplification:** primary and lower-secondary schooling is compulsory and near-universal; quality matters, enrolment doesn't.

## Player levers
Education budget and split by level; teacher pay (public wages); building schools and universities; tuition fee (tertiary); student grants and loans; vocational training subsidies.

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `beta_*` | to calibrate | Enrolment sensitivities |
| `edu_quality_lambda` | 0.02 / month | Speed of quality change |
| `skill_gamma` | 0.2 | Quality impact on skill |

## Interactions
→ labour market (supply by skill), production (productivity), social outcomes (crime ↓, health ↑), opinion (educated people have different ideology and interests), demographics (fertility ↓ with education).

## Edge cases
- Capacity shortage → waiting lists, quality drops, not negative enrolment.
- Over-education: graduates exceeding high-skill jobs → underemployment (employed in mid-skill jobs) → lower measured return and lower approval.

## Acceptance tests
- [ ] Raising tertiary student grants by 50% raises tertiary enrolment within 1–2 school years, more for low-income households.
- [ ] Cutting education spending by 30% lowers education quality gradually over 5–10 years, not instantly.
- [ ] A cohort with higher tertiary attainment has higher average wages 5–10 years later, holding demand constant.
- [ ] Graduates beyond high-skill demand show up as underemployment, not as unemployment only.

## Open questions
- [ ] Private schools and universities as an industry (education industry) in addition to public?
- [ ] Brain drain: emigration of graduates when domestic premium is low?
