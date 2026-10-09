---
id: economy/investment-capital
title: Investment & Capital
status: draft
owner: horia
depends_on: [economy/production, economy/money-banking, economy/industries, adr/0016-firm-representation]
research: []
updated: 2026-10-09
---

# Investment & Capital

## Purpose
Firms decide how much new capacity to build; investment is both demand (for machinery, construction, electronics) and future supply. This is where interest rates, profits, confidence and subsidies turn into growth.

## Real-world basis
- Investment responds to expected demand (accelerator), profitability vs cost of capital (Tobin's q), uncertainty and credit availability. It is the most volatile GDP component.
- Capital goods are produced goods; investment has a gestation lag.
- FDI responds to profitability, institutions, stability and expropriation risk.

## State variables (per firm unit $f$; depreciation per industry $j$)
Investment is decided per **firm unit** ([ADR-0016](../../03-architecture/decisions/0016-firm-representation.md)): each named firm and each cohort firm decides for itself, per real firm, so a cohort's investment is $n_f$ (`firm_count[f]`) times the representative firm's. Industry investment is the count-weighted sum over its units. The rules below are written with $j$ and apply per unit.

| Symbol | code_name | Meaning |
|---|---|---|
| $K_f$ | `capital_stock[f]` | Capital in place |
| $K^{wip}_f$ | `capital_in_progress[f]` | Projects under construction |
| $\delta_j$ | `depreciation_rate[j]` | Per tick (industry parameter) |
| $q_f$ | `investment_attractiveness[f]` | Expected return / cost of capital |

## Update rule
**Desired investment:**
```math
I^*_j = \delta_j K_j + \beta_{cu}(cu_j - cu^*) K_j + \beta_q (q_j - 1) K_j
```
```math
q_j = \frac{\text{expected profit rate}_j \cdot (1 - \tau^{corp}) + \text{invest subsidy}_j}{r^L_j + \text{risk premium}_j - \pi^e}
```
**Financing:** retained earnings first, then bank loans (subject to credit supply, which is tighter for micro and small cohorts), then equity (FDI if foreign-owned). Credit rationing caps each unit's $I_f$.

**Entry.** In cohorts, new firms entering (a rise in `firm_count`) bring their start-up capital as investment.

**Composition:** $I_j$ buys a fixed capital-goods mix per industry (e.g. `construction_buildings` 45%, `industrial_machinery` 30%, `computers_comms` 10%, `electrical_equipment` 10%, `trucks_buses` 5%), sourced domestically or imported.

**Gestation:** new capital enters $K_j$ after $\tau^{build}_j$ ticks. $K_{j,t+1} = (1-\delta_j)K_{j,t} + \text{completed}_{j,t}$.

**FDI:** foreign-owned firm units (named foreign firms and foreign cohorts) invest by their own rule; see [foreign-ownership](foreign-ownership.md). The risk premium in $q$ includes the institutions index from [state capacity](state-capacity.md).

## Player levers
Corporate tax and investment allowances, investment subsidies, policy rate, credit rules, nationalisation (SOE strategic investment), infrastructure (raises $q$ via capacity), capital controls.

## Acceptance tests
- [ ] A 2-point rise in the policy rate lowers private investment within 2–6 ticks, more in capital-intensive industries.
- [ ] An investment subsidy for `data_centres` raises their capital stock after the gestation lag.
- [ ] Expropriation in any industry lowers FDI across all industries.

## Open questions
- [ ] Housing investment: by households (via `construction_buildings`) or a real-estate developer sector?
- [ ] Do SME cohorts face a credit-rationing rule different from large and named firms, or only a higher spread?
