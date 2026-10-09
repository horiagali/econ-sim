---
id: game/information
title: Information, Graphs & Causal Explanations
status: draft
owner: horia
depends_on: [game/overview, vision/pillars]
research: []
updated: 2026-10-08
---

# Information, Graphs & Causal Explanations

## Purpose
The player steers by looking at data. v1 shows **a lot of graphs** and makes every number explainable. This doc defines *what* information exists. Layout belongs to UI design later.

## v1: perfect information
All statistics are exact and current. *(Later: imperfect data, see below.)*

## Views
1. **Overview dashboard.** GDP (real, nominal, per capita, growth), inflation (CPI, core, by group), unemployment, wages (real and nominal), budget balance, public debt/GDP, policy rate, exchange rate, trade balance, national approval, inequality (Gini), population.
2. **Time-series graphs.** Any recorded variable can be plotted over selectable horizons (1y, 5y, 20y, all). Multiple series and both axes. Filters by any group (region, county, age, education, job, identity, interest group…) and industry. Events and player decisions are marked on the time axis.
3. **Group explorer.** Build any group by filtering persons on any attribute (county, urban/rural, age, sex, education, activity, occupation, industry, income class, ethnicity, religion, interest group…), or break a group down by an attribute. For each group: size, income breakdown, taxes paid, transfers received, spending basket, wealth and debt, personal inflation rate, unemployment, approval and its drivers, ideology mix, interest-group composition, crime and health indicators, and the synthetic sample size. Shows how the group changed since a chosen date.
4. **Industry and supply-chain view.** Per industry: output, price, unit-cost breakdown (labour, each input, energy, capital, taxes), margins, employment by skill, investment, capacity utilisation, exports and imports, ownership (private or state). A supply-chain graph shows upstream suppliers and downstream customers with flows.
5. **Budget view.** Revenues by tax, spending by ministry and programme, transfers by type, interest, deficit, debt, maturity profile, bond yields. A "who pays and who receives" breakdown by group.
6. **Prices view.** Price index per good; CPI contributions by good; imported vs domestic inflation.
7. **Finance view.** Interest rates (policy, loans, deposits, bonds), credit growth, bank capital ratios, non-performing loans, money supply, central bank balance sheet.
8. **Trade and currency view.** Exports and imports by good, terms of trade, exchange rate, reserves, current and capital account.
9. **Map of Romania.** Choropleth of any indicator by county or development region.
10. **Projects view.** Ongoing projects, progress, cost, expected effects, EU co-financing.
11. **EU view.** Rule compliance, fiscal-rule path, procedures, EU funds envelope vs absorbed, money at risk of loss, net EU balance.
12. **Housing view.** Prices, rents, price-to-income, vacancies and construction by county.
13. **Environment view.** Emissions by sector, carbon costs, air quality map, weather events and damage.
14. **Governance view.** Administrative capacity by ministry, corruption, money lost to inefficiency, informal economy and VAT gap.

## "Why did this change?" (causal panel)
For any indicator and time window, show a decomposition of its change into contributions, ordered by size. Example for a food price change over 6 months: input costs (+1.2%), wages (+0.4%), energy (−0.3%), demand pressure (+0.6%), VAT change (+0.8%), import prices via FX (+0.5%).
- Requires each mechanic's update rule to be **additively decomposable** (or decomposable via a documented method such as log-changes or Shapley for products). Each spec's *Interactions* section names its contributors.
- Drill-down: click a contribution to explain *that* variable in turn.
- Policy attribution: optionally show the estimated effect of a specific lever change. This needs a counterfactual run; see open questions.

## Notifications
Threshold-based alerts (inflation over target, a bank under capital minimum, debt-service spike, a group's approval collapsing, a project completed, shortages from price caps). Informational, not scripted events.

## Later: imperfect information
- Statistics published with **lags** (e.g. GDP quarterly, a month late) and **revisions**.
- **Measurement noise** that depends on statistics-office funding.
- **Polls**: approval and issue salience by group are not visible directly; the player commissions polls (cost, sample size → margin of error).
- Hidden variables (true inflation expectations, bank asset quality) only estimated.

## Open questions
- [ ] Counterfactual "what if I hadn't done X" runs: valuable for learning, but doubles compute. v1 or later?
- [ ] Forecasts or advisor projections shown to the player?
- [ ] How much history to keep (full monthly history per person for decades is large; proposal: full history for aggregates and built-in groups, snapshots for persons)?
