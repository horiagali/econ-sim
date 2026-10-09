---
id: economy/informal-economy
title: Informal Economy & Tax Compliance
status: draft
owner: horia
depends_on: [economy/labor-market, economy/taxation, economy/state-capacity]
research: []
updated: 2026-10-08
---

# Informal Economy & Tax Compliance

## Purpose
A significant part of economic activity happens off the books: undeclared work, "envelope wages" (declared minimum wage plus cash), unreported sales and VAT fraud, subsistence production. This lowers tax revenue, leaves workers without social protection, and responds to tax rates, enforcement and trust in the state. Realistic for Romania, which has one of the largest VAT gaps in the EU *(verify)*.

## Real-world basis
- Informality rises with the tax and contribution wedge, regulatory burden, weak enforcement, low trust and high cash use; falls with digitalisation (e-invoicing, electronic payments) and better public services.
- Undeclared work clusters in agriculture, construction, hospitality, personal and domestic services.
- National accounts include estimates of the non-observed economy, so GDP includes informal activity, but tax revenue doesn't.

## Representation
### Persons
`formality` status per employed person: **formal**, **partly declared** (declared at a lower wage, the rest in cash), **informal** (fully undeclared). Subsistence farming of own plots is separate ([households](households.md)).

### Firms / industries
Per industry: `undeclared_sales_share` (sales not reported for VAT and profit tax), `undeclared_wage_share`.

## Update rule (per tick, aligned)
**Informality propensity** for a job (person × industry):
```math
p^{inf} = \text{logistic}\Big(\beta_0 + \beta_w \cdot \text{tax wedge} + \beta_m \cdot \frac{W^{min}}{\text{productivity}} - \beta_e \cdot \text{enforcement}_r - \beta_d \cdot \text{digitalisation} - \beta_t \cdot \text{trust} + \beta_c \cdot \text{cash share} + \text{industry effect}\Big)
```
- `enforcement` = audit capacity (tax administration budget × [state capacity](state-capacity.md)) × penalty level.
- `trust` = perceived quality of public services and corruption (people evade more when they think the state wastes money).
- Workers accept informality more when unemployment is high and benefits are low.

**Undeclared sales** (VAT gap) follow a similar logic per industry, with e-invoicing and cash limits as strong levers.

**Detection:** each tick a share of informal activity is detected (∝ enforcement) → back taxes + fines (booked transfers) and formalisation of some jobs.

## Effects
| On | Effect |
|---|---|
| Tax revenue | No PIT, contributions, VAT or profit tax on informal parts |
| Workers | No unemployment benefits, sick pay or contributory pension rights for undeclared time → poverty in old age later |
| Wages | Informal wages lower on average; can undercut the minimum wage |
| Productivity | Informal firms stay small, invest less, lower TFP growth |
| Statistics | Official statistics show only the observed economy; the game shows both observed and total |

## Player levers
Tax administration budget and digitalisation (e-invoicing, cash payment limits, electronic receipts), penalties, labour inspection funding, tax amnesty (one-off revenue, lowers future compliance), lower tax wedge on low wages, simplified regimes (micro-enterprise, flat contributions).

## Outputs
Informal employment share by industry and county, VAT gap, estimated revenue loss, formalisation rate, people without pension rights.

## Acceptance tests
- [ ] Raising social contributions on low wages increases informal employment.
- [ ] Doubling tax-administration capacity reduces the VAT gap over 1–3 years.
- [ ] Informal workers who lose their jobs don't receive unemployment benefit.
- [ ] Measured GDP including informal activity matches total output; tax bases exclude the informal parts.
- [ ] A tax amnesty raises revenue once and slightly lowers compliance afterwards.

## Open questions
- [ ] Should corruption-related bribes be part of this spec or of [state capacity](state-capacity.md) (proposed: state capacity)?
