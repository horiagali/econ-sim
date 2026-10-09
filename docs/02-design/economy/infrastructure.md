---
id: economy/infrastructure
title: Infrastructure & Public Projects
status: draft
owner: horia
depends_on: [economy/fiscal-policy, economy/production]
research: []
updated: 2026-10-08
---

# Infrastructure & Public Projects

## Purpose
Let the head of state build things (roads, rail, ports, broadband, schools, hospitals, public housing, power plants) through one shared **project system**. Construction is demand now (jobs, materials); the finished asset is public capital that raises productivity, capacity or service quality for decades, and needs maintenance.

## Real-world basis
- Public capital has positive output elasticity (Aschauer; IMF meta-analyses ≈ 0.08–0.12), with diminishing returns and large variation by project quality.
- Construction lags and cost overruns are common.
- Under-maintenance destroys value faster than depreciation.

## Project system (shared with energy, education, health)
A project has: `type`, `region`, `size`, `cost` (total, by input good), `duration`, `progress`, `funding source`, `owner` (state / PPP / subsidised private).

Each tick while under construction:
1. Spend `cost / duration` (adjusted by cost-overrun risk, which depends on [state capacity](state-capacity.md); EU-funded projects follow [eu-funds](eu-funds.md)) as purchases from `construction_civil` / `construction_buildings` and their inputs (government expenditure, investment).
2. Progress = spend / cost, limited by construction capacity. If the construction industry is at capacity, projects slow down and construction prices rise (crowding).
3. On completion: add the asset (effective value = spending × efficiency from state capacity) to the public capital stock $K^g_{type,r}$ (or plant capacity, school places, hospital beds).

## Effects of finished assets
| Type | Effect |
|---|---|
| Roads, rail | Transport cost ↓ for industries in region; infrastructure multiplier $\Psi_r$ ↑; regional migration attractiveness ↑ |
| Ports, airports | Trade cost ↓ (exports and imports), tourism ↑ |
| Broadband | TFP ↑ in IT, professional, data centres; remote work |
| Water | Required for agri, chemicals; health ↑ |
| Power plants, grid | See [energy](energy.md) |
| Schools, universities | Education capacity ↑ ([education](../society/education.md)) |
| Hospitals | Health capacity ↑ ([social outcomes](../society/social-outcomes.md)) |
| Public housing | Housing supply ↑, rents ↓ for low-income households |
| Police stations | Policing capacity ↑ |

$\Psi_r = 1 + \sum_{type} \epsilon_{type} \ln(K^g_{type,r} / K^{g,ref}_{type,r})$, with diminishing returns via log.

## Maintenance and depreciation
Assets depreciate at $\delta_{type}$; funded maintenance offsets it. Under-funding raises effective depreciation (potholes, failures). Maintenance cost is a budget line (fiscal-policy).

## Player levers
Commission a project (type, region, size); cancel or pause (sunk costs lost); maintenance funding %; financing choice (budget, bonds, PPP).

## Tuning parameters
Per type: cost per unit, duration, lifetime, $\delta$, output elasticity $\epsilon$, overrun risk.

## Acceptance tests
- [ ] A large project raises construction employment and output during the build phase.
- [ ] After completion, regional output capacity is higher than in a counterfactual without the project.
- [ ] Cutting maintenance to 0 lowers infrastructure quality gradually and raises later repair cost.
- [ ] Many simultaneous projects push up construction prices and delay projects.

## Open questions
- [ ] Should private firms build commercial buildings and housing via the same system (investment-capital), or only the state in v1?
- [ ] Corruption or efficiency of public procurement as a parameter (later politics)?
