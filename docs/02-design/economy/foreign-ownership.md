---
id: economy/foreign-ownership
title: Foreign-Owned Firms & FDI
status: draft
owner: horia
depends_on: [economy/production, economy/investment-capital, economy/trade-fx, economy/state-capacity, adr/0016-firm-representation]
research: []
updated: 2026-10-09
---

# Foreign-Owned Firms & FDI

## Purpose
Foreign-owned firms (multinationals) own a large part of Romania's manufacturing, banking, retail, energy and IT *(verify shares)*. They bring capital, technology, exports and higher wages, but send profits abroad, can relocate when conditions change, and may shift profits to lower-tax countries. Their location and expansion decisions are among the biggest economic events a player will see (a carmaker opening or closing a plant).

## Real-world basis
- FDI responds to labour costs and skills, market access (EU single market), infrastructure, energy prices, taxes and incentives, and institutional quality (rule of law, corruption, policy stability).
- Foreign affiliates are more productive, more export-oriented and more import-intensive than domestic firms; productivity spillovers to local suppliers exist but vary.
- Profits are partly repatriated as dividends (primary-income outflow in the current account) and partly reinvested.

## Representation
Ownership, control, dividends and equity valuation are defined once in the **Ownership** section of [ADR-0016](../../03-architecture/decisions/0016-firm-representation.md) (accepted); this spec only adds the foreign-specific rules. Each firm unit has an ownership vector `ownership[f]` (state, domestic private, foreign) and its own capital, TFP, employment, financing and investment behaviour. A cohort is split into a domestic and a foreign-owned cohort where the foreign share of that size class is material (≥ 10%); within each split control is 100%. All units share the industry's market and price (v1). The industry's foreign share (`foreign_share[j]`) is derived from its units.

A unit whose `controller[f]` is foreign (foreign holders ≥ 50%, or the largest bloc when no one has 50%) follows the rules below on repatriation, transfer pricing and relocation. Minority foreign holders only receive their pro-rata dividends (a primary-income outflow).

Large foreign investments can also be **lumpy projects**: a named investment (e.g. "new car plant in county X, 3,000 jobs") enters as a project with a build time, so it is visible and explainable.

### Named companies (real names)
The largest employers and investors are modelled as **named firms** with their **real names**, e.g. Dacia (Renault group) in Mioveni, Ford Otosan in Craiova, OMV Petrom, the big foreign-owned banks, retailers and IT companies. *(Exact list, ownership, locations and employment to be compiled from public sources and verified; see research backlog.)*
- A named firm is a firm unit with a firm count of 1: it owns a share of the industry's capacity in specific counties (a capacity vector over regions), with its own balance sheet, employment, exports and investment decisions. Industries have 0–5 named firms, chosen by the ADR-0016 threshold, so fragmented industries have none.
- Its decisions (expand, cut, close, new plant) follow the same rules as the rest of the industry, but are shown as events with the company name.
- Named firms live in a data file (`firms`), so the list can be corrected or extended without code changes.
- State-owned companies (e.g. Hidroelectrica, Nuclearelectrica, Romgaz, CFR) are named in the same way under [state-enterprises](state-enterprises.md).
- **Disclaimer in-game:** company behaviour is simulated, not a prediction of real corporate decisions.

## Update rule
**Foreign investment attractiveness** per industry and region:
```math
\text{FDI}^*_{j,r} \propto q_j \cdot \Big(\frac{\text{productivity-adjusted labour cost}_r}{\text{abroad}}\Big)^{-\epsilon_L} \cdot \Psi_r \cdot (1-\tau^{corp}_{eff}) \cdot \text{energy cost}^{-\epsilon_E} \cdot \text{institutions}^{\epsilon_I} \cdot \text{skills}_r \cdot \text{incentives}_{j,r}
```
- `institutions` comes from [state capacity](state-capacity.md) (corruption, rule of law, policy stability: frequent tax changes hurt).
- Expropriation or capital controls (when allowed) cut it sharply for years.

**Profit allocation** of foreign-controlled units each tick: share shifted abroad via transfer pricing (rises with the corporate tax gap vs low-tax countries; reduces domestic taxable profit); of the declared profit after tax, a payout share (higher than for domestic firms) is paid as dividends **pro rata to the ownership vector** (the foreign part is a current-account outflow; any state or domestic part goes to government or domestic holders), and the rest is reinvested (the foreign part of reinvested earnings counts as FDI inflow).

**Relocation** (foreign-controlled units only): when profitability stays below the alternative location for long, capacity is reduced or closed (lumpy closure events), with layoffs and supplier effects.

**Ownership changes** (ADR-0016):
- **Greenfield:** new foreign capacity, either as investment by an existing foreign-controlled unit or as a new foreign unit (a new named firm when it passes the threshold, otherwise added to the foreign cohort).
- **Brownfield acquisition:** a foreign buyer acquires a stake in an existing unit at the transaction value (book equity × `equity_multiple[f]`); if the stake moves `controller[f]` to foreign, the unit switches to these rules (and a domestic cohort's acquired firms move to the foreign cohort). Includes buying a named firm under administration.
- **Exit by sale:** a foreign owner sells its stake to domestic buyers, the state or on the stock exchange.

**Spillovers:** domestic suppliers to foreign firms gain TFP (∝ local sourcing share and absorptive capacity, i.e. education).

## Player levers
Corporate tax, investment incentives and state aid (EU limits), special economic zones/industrial parks (project system), infrastructure, skills (education), energy prices, policy stability, negotiating a named investment (offer incentives for a pending FDI project).

## Outputs
FDI inflows by industry and region, foreign share of output and employment, repatriated profits, profit-shifting estimate, announced and closed projects.

## Acceptance tests
- [ ] Improving infrastructure and skills in a region raises foreign investment there over several years.
- [ ] Higher corporate tax raises estimated profit shifting and lowers FDI.
- [ ] Repatriated dividends appear as a primary-income outflow in the current account.
- [ ] A long period of high energy prices triggers capacity reductions in energy-intensive foreign plants.
- [ ] A unit with a minority foreign stake pays the foreign share of its dividends to RoW but never relocates or shifts profits; acquiring a majority switches it to foreign rules from the next tick.

## Open questions
- [x] Real company names (decided 2026-10-08).
- [ ] How many named firms in v1: set by the ADR-0016 threshold (≥ 5% of output or employment, top ~20 exporters, or SOE with ≥ 1% of output; max 5 per industry), expected to land near the ~50–100 largest employers and exporters.
