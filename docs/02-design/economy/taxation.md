---
id: economy/taxation
title: Taxation
status: draft
owner: horia
depends_on: [society/population-groups, economy/households, economy/production, economy/accounting, economy/vat]
research: []
updated: 2026-10-10
---

# Taxation

## Purpose
Every tax the head of state can set, its base, how it is computed per person, household or industry, and how people respond (evasion, avoidance, labour supply, migration). Detailed enough to design custom progressive brackets.

## Real-world basis
- Progressive income tax with marginal-rate brackets.
- Taxable income elasticity ~0.2–0.5 overall, higher for top earners (avoidance, timing, migration). The Laffer curve emerges from this, not as a formula.
- VAT is roughly proportional to consumption, so regressive relative to income; reduced rates on necessities partly offset this.
- Corporate tax incidence falls partly on workers (via lower investment and wages) in the long run.
- Tax evasion and informal work rise with rates and fall with enforcement.
- **Romania starting point** (verify current rules in research): flat personal income tax, high social contributions (pension CAS, health CASS) paid mostly by the employee, standard VAT with reduced rates, a micro-enterprise turnover regime for small firms, local property taxes.

## Taxes in v1
| Tax | Base | Computed |
|---|---|---|
| Personal income tax (PIT) | each person's wages, self-employment, pensions (+ capital income if not taxed separately) − allowances | **per person, exactly**, with the player's bracket schedule |
| Capital income tax | interest, dividends | flat rate, per household |
| Social contributions | gross wages up to a ceiling | employee and employer rates (pension, health, unemployment); employer part is a labour cost |
| Corporate profit tax | industry profits (after depreciation and interest) | rate per industry (default uniform); losses carried forward |
| Micro-enterprise turnover tax | small firms' turnover (optional regime) | rate on turnover; eligibility threshold |
| VAT | household consumption by good | rate per VAT category (standard, reduced, zero, exempt) assigned per good; own spec: [vat](vat.md) |
| Excise | quantities of fuel, tobacco, alcohol, carbon content | per unit |
| Property tax | dwellings and commercial property value | annual rate / 12, per household and industry |
| Wealth tax | household net worth above a threshold | annual rate / 12 |
| Tariffs | import value per good | see [trade-fx](trade-fx.md) |

## Progressive income tax
Brackets: thresholds $0 = b_0 < b_1 < \dots < b_m$ and marginal rates $t_0 \dots t_m$ on annual taxable income after allowance $a$:
```math
T_i = \sum_{m} t_m \cdot \min\big(\max(y_i - a - b_m, 0),\ b_{m+1} - b_m\big) - \text{credits}_i
```
Applied per synthetic person each tick (monthly withholding with annual reconciliation). Because every person has their own income, there is no approximation. A flat tax is a one-bracket schedule. Credits: per child, per student, low-income credit.

> **Simplification:** individual taxation (no joint filing) in v1.

## Behavioural responses
- **Reported income and sales** are reduced by informality and evasion; full mechanism in [informal-economy](informal-economy.md). Evasion rises with marginal rates and falls with enforcement ([state capacity](state-capacity.md)).
- **EU constraints:** standard VAT ≥ 15% and reduced rates only on allowed categories ([eu-membership](eu-membership.md)).
- **Informal work:** high labour taxes push some low-wage jobs to informal status (no tax, no contributions, no benefits). See [labor-market](labor-market.md).
- **Top-earner avoidance:** elasticity of taxable income rises with income.
- **Labour supply:** participation effects for second earners and near-retirees.
- **Migration:** emigration of high earners and skilled workers responds to after-tax income vs abroad ([demographics](demographics.md)).
- **Corporate:** profit shifting abroad (especially foreign-owned firms) at high rates; investment response via $q$ ([investment-capital](investment-capital.md)).

## Timing
Monthly withholding for PIT and contributions, monthly VAT, quarterly corporate tax with annual true-up. Rate changes apply from the next tick.

## Outputs
Revenue by tax and by group (who pays what); effective and marginal tax rates for any group; tax wedge; distributional incidence charts; revenue vs rate history (Laffer exploration).

## Acceptance tests
- [ ] A one-bracket schedule reproduces a flat tax exactly.
- [ ] Adding a 40% bracket above 5× median income raises revenue mostly from the top decile, and over a few years their reported income falls (elasticity).
- [ ] Raising VAT from 21% to 25% raises CPI once and revenue by less than proportionally (demand response).
- [ ] At extreme rates (e.g. 90% flat PIT) revenue is lower than at moderate rates (emergent Laffer).
- [ ] Raising social contributions increases the informal employment share.

### VAT
The tax itself (rates, categories, rounding, the ledger posting) has its own spec, [vat](vat.md), with criteria AC-VAT-01 to 05. The VAT expectations in the list above (revenue and CPI after a rate change) stay here, because they depend on prices and demand.

## Open questions
- [ ] Separate capital income tax vs taxed with labour income: offer both as a lever?
- [ ] Inheritance tax in v1 (bequests already exist)?
- [ ] Negative income tax option as an alternative to minimum income?
