---
id: society/overview
title: Society — People Model Overview
status: draft
owner: horia
depends_on: [vision/pillars, game/overview]
research: [research/people-model-approaches]
updated: 2026-10-08
---

# Society — People Model Overview

## Why this domain exists
Modelling people is the core of the game. The population of Romania is simulated as a **weighted synthetic population** of households and persons ([ADR-0003](../../03-architecture/decisions/0003-people-representation.md)). Each person has a region and county, age, sex, education, job, income, family, wealth, ethnicity, language, religion, ideology, interest-group memberships and approval. Each person earns, pays tax, receives transfers, spends, saves, borrows, learns, ages, moves, gets sick and forms opinions. Economic aggregates are sums over people.

Any slice of the population (*"well-educated students in Cluj: 5,000 people"*) is a **group**: a filter the player can view, save and track.

## Specs in this domain
| Spec | Owns |
|---|---|
| [population-groups](population-groups.md) | Synthetic population: households, persons, attributes, transitions, groups as views |
| [identity](identity.md) | Ethnicity, mother tongue, religion, religiosity |
| [interest-groups](interest-groups.md) | Overlapping interest groups, memberships and stances |
| [opinion-approval](opinion-approval.md) | Ideology axes, issue salience, approval (a number in v1) |
| [education](education.md) | Schooling, enrolment, graduation, skills, returns to education |
| [social-outcomes](social-outcomes.md) | Crime, health and life expectancy, wellbeing, poverty |
| [demographics](../economy/demographics.md) | Births, deaths, migration rates |

The economic behaviour of households (budget, consumption, saving, borrowing) is specified in [households](../economy/households.md).

## A policy's journey (worked example: student finance)
1. Player raises the student grant from 300 to 500 per month. [levers](../game/levers.md) → [social transfers](../economy/social-transfers.md).
2. From the next tick, every person with activity = `student` (and eligible under the means test) gets +200 per month, paid into their household. Government spending rises by 200 × eligible students. [fiscal policy](../economy/fiscal-policy.md)
3. Those households' disposable income rises. Students' high propensity to consume means most of it is spent on their basket: rent, food, transport, electronics, hospitality. [households](../economy/households.md)
4. Demand for those goods rises → those industries raise output and hire, maybe raise prices slightly. [production](../economy/production.md), [labor market](../economy/labor-market.md), [prices](../economy/prices-inflation.md)
5. Higher grants raise enrolment and lower dropout, especially for low-income students. [education](education.md)
6. Years later more graduates enter the labour force → more high-skill workers → productivity and wages up, unemployment and crime down. [social outcomes](social-outcomes.md)
7. Students' approval rises; the *students & youth* interest group likes it. Approval of people who pay for it via taxes or a bigger deficit may dip, depending on their ideology. [opinion-approval](opinion-approval.md)
8. The deficit widens unless financed by taxes; debt and bond yields respond. [fiscal policy](../economy/fiscal-policy.md)

Every arrow above must be a documented mechanism with a causal trace.
