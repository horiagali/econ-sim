---
id: vision/scope
title: Scope
status: draft
owner: horia
depends_on: [vision/vision]
updated: 2026-10-09
---

# Scope

## v1: economy + people, no politics, no military

**World**
- One player-controlled country: **Romania**, with its 42 counties (41 + Bucharest) and 8 development regions. See [game overview](../02-design/game/README.md).
- **Rest of the world** as an aggregate trading partner with exogenous prices, demand and interest rates (EU / non-EU split proposed).

**People** ([society](../02-design/society/README.md))
- **The most important system.** Population modelled as a **weighted synthetic population** (~75k households, ~190k persons at 1 : 100; see [ADR-0003](../03-architecture/decisions/0003-people-representation.md)). Every person has county, urban/rural, age, sex, education, job (occupation, industry, employer), income, wealth, family, ethnicity, language, religion, ideology, interest-group memberships and approval.
- People are born, age, study, work, lose jobs, retire, move, emigrate, return and die. The diaspora sends remittances.
- **Groups** are filters over people (any combination of attributes), including overlapping **interest groups** (environmentalists, motorists, farmers, pensioners…).
- Social outcomes: education attainment, crime, health and life expectancy, wellbeing.
- **Approval** per person, so for any group and nationally. In v1 it is just a number: no behavioural effects, no elections.

**Economy** ([economy](../02-design/economy/README.md))
- About 90 industries linked by an input-output supply chain; each produces one good or service ([industries](../02-design/economy/industries.md)).
- Sectors: households (the synthetic population), firms (by industry; domestic, foreign-owned and state-owned), banks, government, central bank, rest of world.
- Prices per good, inflation, wages, employment, interest rates, credit, exchange rate, trade, public debt, all stock-flow consistent.
- Energy system with power plants of different types.
- Infrastructure and public investment projects.
- **EU membership:** single market and common external tariff, free movement, **Schengen (leave / rejoin)**, state-aid and VAT rules, EU fiscal rules, EU budget contribution, EU funds with absorption.
- **Euro:** join via ERM II and the convergence criteria, or leave after joining.
- **Real company names** for the largest firms (foreign-owned, domestic and state-owned).
- **Informal economy:** undeclared work, envelope wages, VAT gap.
- **Foreign-owned firms:** FDI decisions, profit repatriation, relocation.
- **Housing market** by county: prices, rents, construction, vacancies.
- **Environment:** emissions and EU carbon price, air pollution, droughts, floods, heatwaves.
- **State capacity and corruption:** how efficiently public money becomes results.

**Player levers** (full catalogue: [levers](../02-design/game/levers.md))
- Detailed taxation (custom progressive brackets, VAT rates, corporate, payroll, property, wealth, excise). Tariffs are set by the EU while Romania is a member.
- Spending by ministry, public wages, pensions, unemployment and child benefits, student finance, minimum income, charity drives.
- Subsidies (per industry, production or consumption), price caps.
- Nationalisation and privatisation.
- Build infrastructure and power plants.
- Central bank policy rate and tools, controlled directly by the player.
- Minimum wage, immigration policy, trade policy.

**Outputs** ([information](../02-design/game/information.md))
- Many graphs: macro dashboard, group explorer, map of Romania, industry and supply-chain views, budget, prices, trade, finance.
- A causal "why did this change?" panel for any indicator.

## Later
- Imperfect information: statistics with lag and noise, paid polls, statistics-office funding.
- Scenarios with goals.
- Politics: parliament, parties, elections, legislation, stability; approval with behavioural effects (strikes, protests). Ideology and interest groups already exist in v1 so this can plug in.
- Central bank independence option.
- Military: budget, readiness, arms industry, conflicts.
- Multiple simulated countries trading with each other; other country presets.

## Explicitly out
- Multiplayer.
- Real-time tactical combat.
