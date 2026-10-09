---
id: economy/fiscal-policy
title: Fiscal Policy — Budget, Spending, Subsidies & Public Debt
status: draft
owner: horia
depends_on: [economy/accounting, economy/taxation, economy/social-transfers, economy/money-banking]
research: []
updated: 2026-10-08
---

# Fiscal Policy — Budget, Spending, Subsidies & Public Debt

## Purpose
The government budget: what the state buys, whom it employs, what it subsidises, how it covers the deficit, and how debt and bond yields evolve. Deficit spending is demand now and debt later.

## Real-world basis
- Fiscal multipliers vary by instrument (investment and transfers to the poor > tax cuts for the rich), by slack in the economy (higher in recessions), by monetary response (lower if the CB offsets) and by openness (leakage via imports).
- Debt sustainability: $\Delta(b) \approx (r - g) \cdot b - \text{primary balance}$.
- Sovereign risk premia rise non-linearly with debt, deficits, inflation and weak institutions; foreign-currency debt is riskier.

## Budget lines (per tick)
**Revenue:** all taxes ([taxation](taxation.md)), SOE dividends ([state-enterprises](state-enterprises.md)), central bank profit, fees, privatisation receipts, ETS auction revenue ([environment](environment.md)), EU funds reimbursements ([eu-funds](eu-funds.md)).

**Spending:**
| Line | Goes to |
|---|---|
| Ministry budgets (education, health, police and justice, administration, research, culture, environment) | public wages (public employees are persons with `employer_type = public sector`) + purchases of goods by fixed mix per ministry |
| Infrastructure projects and maintenance | `construction_civil`, `construction_buildings` and inputs ([infrastructure](infrastructure.md)) |
| Social transfers | households ([social-transfers](social-transfers.md)) |
| Subsidies | firms or consumers (below) |
| Interest on debt | bond holders (households, banks, pension funds, CB, RoW) |
| SOE loss coverage | SOEs |
| EU budget contribution | EU ([eu-membership](eu-membership.md)) |
| EU-funds co-financing and pre-financing | projects ([eu-funds](eu-funds.md)) |

Ministry spending affects quality indices: education quality, health capacity, policing, admin efficiency (tax enforcement), research (TFP). How much quality each leu buys depends on [state capacity](state-capacity.md); part of procurement can leak to corruption.

**EU fiscal rules:** deficit and debt are tracked against the EU reference values and the required adjustment path ([eu-membership](eu-membership.md)).

## Subsidies
| Type | Mechanic |
|---|---|
| Production subsidy | % of output value or per unit to industry *j* → unit cost ↓ → price ↓ and/or margin ↑ |
| Investment subsidy | % of capex → $q$ ↑ ([investment-capital](investment-capital.md)) |
| Wage subsidy | % of wages for hires (e.g. youth) → labour cost ↓ |
| Consumer subsidy | % off the price of good *g* for households (e.g. energy, public transport, bread) → demand ↑ |
| Export subsidy | see [trade-fx](trade-fx.md) |

## Financing and debt
- Deficit = spending − revenue. Financed by issuing bonds (by maturity: bills, 2y, 10y), drawing down deposits, or monetary financing (CB, see [monetary-policy](monetary-policy.md)).
- Buyers: banks, households (by portfolio rules), RoW (depends on yield vs world rate and risk), CB (QE).
- **Bond yield:**
```math
r^B = i + \text{term premium} + \text{risk premium}(\tfrac{B}{Y}, \text{deficit}, \pi, \text{cred}, \text{share foreign held})
```
Risk premium is convex: small below ~60–90% debt/GDP, steep above a soft threshold that depends on credibility.
- **Rollover:** maturing debt must be refinanced at current yields.
- **Debt crisis:** if auctions fail (demand < issuance at a yield cap), the player must cut spending, raise taxes, monetise or **default** (haircut on bond holders → bank losses, household wealth loss, RoW cuts lending, yields spike for years).

## Player levers
Ministry budgets, public wages and headcount, subsidies, borrowing strategy (maturity mix), fiscal rules (optional self-imposed debt brake), default decision.

## Outputs
Budget balance, primary balance, debt/GDP, interest burden, yields, holders of debt, multiplier estimates, distribution of spending and taxes by group ("fiscal incidence").

## Acceptance tests
- [ ] Debt-financed spending raises GDP in the short run, more when unemployment is high and the CB doesn't hike.
- [ ] Debt identity holds: $B_{t+1} = B_t + \text{deficit}_t - \text{monetary financing}_t$ (+ revaluations).
- [ ] Persistent primary deficits with $r > g$ produce rising debt/GDP, rising yields and eventually a debt crisis.
- [ ] A consumer energy subsidy lowers measured CPI and raises energy demand and the fiscal cost.

## Open questions
- [ ] Should ministries have a minimum viable budget (e.g. zero police → chaos) or scale smoothly?
- [ ] Foreign-currency (EUR) borrowing: proposed yes, since Romania issues a large share of debt in EUR *(verify)*; adds FX risk to debt. Debt under foreign law matters for euro exit ([euro](euro.md)).
