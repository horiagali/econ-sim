---
id: economy/demographics
title: Demographics — Births, Deaths, Migration
status: draft
owner: horia
depends_on: [society/population-groups, society/social-outcomes]
research: []
updated: 2026-10-08
---

# Demographics — Births, Deaths, Migration

> Lives under `economy/` for historical reasons; conceptually part of [society](../society/README.md). Move it when files can be renamed.

## Purpose
Population grows and shrinks through births, deaths and migration. Rates respond to economic conditions and policy and drive transitions in the synthetic population ([population-groups](../society/population-groups.md)). Ageing drives pension costs and labour supply. For Romania, **emigration and the diaspora** are a first-order force: a large share of working-age people live abroad and send remittances *(verify magnitudes)*.

## Real-world basis
- Fertility falls with education and income, rises modestly with child benefits and childcare; dips in recessions and with high housing costs.
- Mortality falls with income, health spending and education.
- Migration follows expected-income and employment gaps (Harris–Todaro), network effects (people go where family already is) and free movement within the EU ([eu-membership](eu-membership.md)).

## Update rule (monthly probabilities per person, aligned to totals)
**Births:** for women aged 15–49:
```math
p^{birth}_i = f_{age} \cdot m_{edu} \cdot m_{relig} \cdot m_{econ}(\text{income}, u, \text{housing cost}) \cdot m_{pol}(\text{child benefit}, \text{childcare}) \cdot m_{partner}
```
A newborn person is added to the mother's household, inheriting county and identity attributes ([identity](../society/identity.md)).

**Deaths:** $p^{death}_i = \text{mort}(age, sex) \cdot m(\text{health}_i)$. Assets stay in the household, or go to heirs if the person lived alone.

**Internal migration** (household moves county): probability ∝ expected income and job gap, housing cost gap, distance, language, age (young move more), education.

**Emigration** (person or household leaves):
```math
p^{emig}_i \propto \Big(\frac{\text{expected after-tax income abroad}_{skill}}{\text{expected income at home}_i}\Big)^{\epsilon} \cdot \text{network}_i \cdot \text{age factor} \cdot (1 + \beta_u u_{r})
```
`network` is higher when the household already has family abroad. Emigrants take liquid assets; they join the **diaspora stock**.

**Diaspora and remittances:**
- The diaspora is tracked as a stock of emigrated persons (by skill, age, origin county), not simulated in detail.
- They send **remittances** to linked households: amount ∝ their foreign income, falls with time abroad (ties weaken).
- **Return migration:** probability ∝ improvement of home wages vs abroad, age, and policy (return incentives later). Returnees re-enter as persons with savings and skills gained abroad.

**Immigration:** desired inflow by skill depends on domestic wages and jobs vs origin countries; **capped by the player's quota** for non-EU immigrants; EU citizens can come freely. New persons are cloned from a donor pool with appropriate attributes and low initial wealth.

## Player levers
Immigration policy (quotas by skill), child benefit and childcare (fertility), health spending (mortality), regional development (internal migration). *(Later: diaspora return incentives.)*

## Outputs
Population, growth, fertility rate, life expectancy, dependency ratio, net migration by skill, diaspora size, remittance inflows, population by county.

## Acceptance tests
- [ ] With fertility below replacement and zero migration, the population ages and shrinks.
- [ ] A regional boom draws young workers from other regions within a few years.
- [ ] Skilled emigration rises when after-tax skilled wages fall relative to abroad, and remittances rise with the diaspora.
- [ ] Faster domestic wage growth raises return migration.

## Open questions
- [ ] Do we name destination countries for the diaspora (Italy, Spain, Germany, UK…) or keep "abroad" as one place?
