---
id: society/identity
title: Identity — Ethnicity, Language, Religion
status: draft
owner: horia
depends_on: [society/population-groups]
research: []
updated: 2026-10-08
---

# Identity — Ethnicity, Language, Religion

## Purpose
Give persons the cultural attributes that shape opinions, interests and some economic outcomes in Romania: ethnicity, mother tongue, religion and how religious they are. Identity mostly drives **opinion and interest groups**; where real data shows economic gaps (e.g. employment and education of Roma), those gaps come from modelled mechanisms (education, location, discrimination parameter), not from hard-coded penalties.

## Categories (v1, Romania)
Shares are calibration targets to verify against Census 2021 (INS).

| Attribute | Values |
|---|---|
| `ethnicity` | Romanian, Hungarian, Roma, Ukrainian, German, Turkish/Tatar, Lipovan Russian, other |
| `mother_tongue` | Romanian, Hungarian, Romani, Ukrainian, other |
| `religion` | Orthodox, Roman Catholic, Reformed (Calvinist), Greek-Catholic, Pentecostal, Baptist & other Protestant, Muslim, other, none / not declared |
| `religiosity` | 0–1: practice intensity (attendance, importance of religion) |

Strong regional patterns (e.g. Hungarian majorities in Harghita and Covasna; Catholic and Reformed concentration in Transylvania; Muslim communities in Constanța) come from the census-based initial population.

## Dynamics
- Ethnicity, mother tongue and religion are inherited at birth from parents (mixed households: probabilistic) and rarely change.
- `religiosity` drifts slowly: lower with urbanisation, education and younger cohorts; higher with age. Shocks (crises) can raise it slightly.
- Language and ethnicity affect internal migration (people move more to regions where their language is spoken) and emigration destinations.

## Economic links (mechanisms, all tunable)
| Link | Mechanism |
|---|---|
| Labour market | Optional discrimination parameter per group in hiring (default from research, can be reduced by anti-discrimination policy later) |
| Education | Enrolment and completion respond to the same drivers for everyone (income, location, support); gaps emerge from differences in those drivers |
| Consumption | Small basket differences (e.g. religious fasting periods lower meat consumption seasonally) — optional flavour |
| Fertility | Religiosity raises fertility slightly ([demographics](../economy/demographics.md)) |

## Opinion links
Identity sets starting ideology and interest-group memberships (e.g. high religiosity → *religious traditionalists*; Hungarian → *minority rights*) and the weight people put on cultural issues. See [opinion-approval](opinion-approval.md) and [interest-groups](interest-groups.md).

## Player levers (v1)
None that target identity directly. *(Later, with politics: minority-language education, church funding, anti-discrimination law.)*

## Acceptance tests
- [ ] Initial identity shares by county match census within 1 point.
- [ ] Over 30 years with no policy change, average religiosity declines slowly, faster in urban and educated groups.

## Open questions
- [ ] Is this category list right for the game, or should some be merged?
- [ ] Should minority-language schools, church funding and similar levers be in v1? They are cultural rather than economic, so they would normally come with politics.
