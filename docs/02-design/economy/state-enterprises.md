---
id: economy/state-enterprises
title: State-Owned Enterprises, Nationalisation & Privatisation
status: draft
owner: horia
depends_on: [economy/production, economy/fiscal-policy, economy/accounting, adr/0016-firm-representation]
research: []
updated: 2026-10-09
---

# State-Owned Enterprises, Nationalisation & Privatisation

## Purpose
Allow the state to own all or part of an industry, run it with non-profit goals, and buy or sell stakes. Owning firms gives control (prices, jobs, investment) at the cost of efficiency, fiscal risk and investor confidence.

## Real-world basis
- SOEs tend to have lower productivity and profitability on average, with large variation; they are often used for employment, price control or strategic goals.
- Compensated nationalisation is a financial transaction (bonds for equity). Uncompensated expropriation hurts investor confidence and FDI.
- Privatisation raises one-off revenue and may raise efficiency, but loses future dividends.

## Representation
Ownership, control, dividends and equity valuation are defined once in the **Ownership** section of [ADR-0016](../../03-architecture/decisions/0016-firm-representation.md) (accepted); this spec only adds the state-specific rules. The state share $\omega_f$ (`state_share[f]`) is the `state` component of each firm unit's ownership vector `ownership[f]`. Named SOEs carry their real state share; cohorts can carry a state share too (for example municipal water, heating and transport companies). The industry's state share $\omega_j$ is derived as the capital-weighted average over its units. A unit whose `controller[f]` is the state (state holds ≥ 50%, or is the largest bloc when no one does) follows the SOE directives below; minority state stakes only receive dividends. Nationalising an industry means raising $\omega_f$ on some or all of its units; nationalising a company acts on all its units (shared `company` id).

Non-market public producers (public administration, public schools, hospitals and social care) are 100% state-owned firm units whose output is valued at cost and paid for from the budget (ADR-0016). They are not subject to the directives below; their levers are ministry budgets, headcount and public wages ([fiscal-policy](fiscal-policy.md)).

## Nationalise / privatise
- **Compensated:** government pays the transaction value of the equity acquired, $\Delta\omega_f \cdot n_f V_f$ per unit ($V_f$ = book equity × `equity_multiple[f]`, ADR-0016), financed by deposits or new bonds; private equity holders (households, pension funds, RoW) receive deposits or bonds pro rata to their shares. SFC-neutral for net worth at that value.
- **Expropriation:** no payment; private holders lose equity (wealth loss booked as a capital transfer); investor confidence and FDI fall nationally for years; RoW holders → capital-flight shock.
- **Privatise:** sell $\Delta\omega$ at the transaction value, by direct sale to a buyer or by an IPO / share sale on the stock exchange (ADR-0016 rule: buyers are wealthier households, pension funds and RoW, with a discount that grows with the stake relative to market depth).
- **Bailout of a failing named firm** under administration (ADR-0016): capital injection in exchange for equity at the transaction value, or full nationalisation.

## Named SOEs
Major state-owned companies are named with their real names (e.g. Hidroelectrica, Nuclearelectrica, Romgaz, Transelectrica, Complexul Energetic Oltenia, CFR, Poșta Română — *verify current state shares*), with their real shareholder structure at start. A state-owned company is named if it has ≥ 1% of its industry's output (ADR-0016 threshold). They are named firm units in the `firms` data file.

## SOE directives
| Directive | Behaviour |
|---|---|
| Profit (default) | Same rules as private firms |
| Employment | Hire above cost-minimising level; losses covered by government |
| Price | Sell below market (cap); losses covered by government; may cause shortages if output doesn't expand |
| Strategic investment | Invest per player target regardless of profitability |

## Efficiency
SOE TFP drift: $g^{SOE} = g^{private} - \xi$ (default ξ ≈ 0.5%/yr), smaller when [state capacity](state-capacity.md) is high and corruption low.

## Outputs
SOE dividends to government (pro rata to the state share, ADR-0016) or losses covered by subsidies, SOE employment, investor confidence effect, government equity holdings.

## Acceptance tests
- [ ] Compensated nationalisation leaves total household net worth unchanged at the moment of transfer.
- [ ] Expropriation lowers FDI and private investment for several years.
- [ ] An SOE with a price directive below cost produces fiscal losses each tick.
- [ ] A firm with a 20% state share pays exactly 20% of its dividends to government and does not follow SOE directives.

## Open questions
- [ ] Should SOEs and private firms be able to charge different prices in the same market?
- [ ] Nationalising banks (see [money-banking](money-banking.md)) via this same mechanic?
- [x] Control threshold for directives: the state is `controller[f]` (≥ 50%, or largest bloc when no holder has 50%); see ADR-0016 Ownership.
