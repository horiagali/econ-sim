---
id: economy/taxation
title: Taxation
status: draft
owner: horia
depends_on: [society/population-groups, economy/households, economy/production, economy/accounting, economy/vat, economy/labor-market, society/population-incomes, economy/firm-units-undeclared-inventories, economy/budget-2021, adr/0017-time-base]
research: [research/law-in-force-romania-2021]
updated: 2026-10-14
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

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| Income tax and contributions on wages and pensions | withheld on pay day (month end), by the [labour market](labor-market.md)'s pay phase and the pension payment |
| Corporate profit tax, micro-enterprise turnover tax | quarterly, on the quarter's last day, on the quarter's profit or turnover |
| Tax on dividends and interest | withheld when they are paid (from M3, with [money and banking](money-banking.md) and firms' payouts) |
| A change of a rate | from the first day of the following month ([ADR-0018](../../03-architecture/decisions/0018-scenario-and-commands.md), kind `law`) |

## First version (roadmap M3)
The taxes Romania levied on 1 December 2021 ([law in force](../../01-research/notes/law-in-force-romania-2021.md)), with a schedule general enough for the player's brackets. Integer bani; each amount rounded once, half away from zero.

**In:**
1. **Contributions on a declared wage** (`wage_declared`, gross, a month): the employee's pension contribution `cas_rate` (25%), of which `pillar2_share` (3.75 points) goes to the person's Pillar II pension fund and the rest to the government; the employee's health contribution `cass_rate` (10%); the employer's labour insurance contribution `cam_rate` (2.25%), paid on top of the gross wage. Contributions have no ceiling in 2021.
2. **Personal deduction** a month, from the gross declared wage and the number of dependants (children under 18 in the household, at most four counted): `deduction_base[d]` (510, 670, 830, 990, 1,310 lei for 0 to 4 dependants) up to a gross of `deduction_full_until` (1,950 lei), falling linearly to zero at `deduction_zero_at` (3,600 lei), rounded to whole lei.

> **Simplification:** the Fiscal Code lowers the deduction in steps of 50 lei of gross wage; v1 uses a straight line between the same end points (low confidence on the in-between values, high on the ends).

3. **Income tax on a wage** = schedule(gross − CAS − CASS − deduction, not below 0). The **schedule** is the player's brackets: thresholds $0 = b_0 < b_1 < \dots$ and marginal rates $t_m$, a month, as in "Progressive income tax" above; on the start date one bracket at 10%.
4. **Industry overrides**, data of the law: an industry can have its own income tax rate, CAS and CASS rates and a ceiling up to which they apply. On the start date: construction (no income tax, no CASS, CAS 21.25%, on gross wages up to 30,000 lei a month; medium confidence) and software (`it_software`: no income tax; medium).
5. **Only the declared part is taxed.** The undeclared part of a wage (stage I's `wage` − `wage_declared`) pays nothing; a job with no contract pays nothing. Net wage received = `wage` − contributions − income tax on the declared part.
6. **Pensions:** income tax at the schedule's rate on the part of the pension above `pension_tax_free` (2,000 lei a month); no contributions (medium confidence).
7. **Self-employment income** (declared part, [incomes](../society/population-incomes.md)): income tax at the schedule's rate on the month's declared amount; contributions on self-employment come with the [informal economy](informal-economy.md) (M15).
8. **Corporate profit tax** per firm unit, quarterly: `profit_tax_rate` (16%) of the quarter's taxable profit, which is value added less wages, employer contributions, interest and depreciation, less losses carried forward from earlier quarters; a loss is carried forward. Only the declared part counts: the unit's undeclared sales ([firm units' undeclared sales](firm-units-undeclared-inventories.md)) are left out of value added.
9. **Micro-enterprise regime:** a cohort unit of size micro whose declared turnover per firm in the last four quarters is under `micro_threshold` (1 million euro at the scenario's exchange rate) pays `micro_rate_with_staff` (1%) of declared turnover, or `micro_rate_no_staff` (3%) if it has no employees, instead of profit tax.
10. **Dividends and interest:** `dividend_tax_rate` (5%) and `interest_tax_rate` (10%), withheld at the source when those flows exist (M3). The flat 10% health contribution on non-wage income above twelve minimum wages a year comes with them (medium confidence).
11. **Ledger:** the employer pays the gross wage plus CAM. Net wages go to households; employee contributions and income tax go to the government; the Pillar II share goes to the pension funds sector. Each has its own flow code (below). Government receipts settle as in the [World](../game/world-loop.md) (deposits → reserves → government deposits at the central bank).

**Not in M3:** excise (with energy and fuel, M12), property tax (with housing, M13), wealth tax (a lever with no tax on the start date, M4), tariffs ([trade](trade-fx.md); fixed by the EU), every behavioural response (taxable-income elasticity, evasion, migration: M5 and M15), credits other than the personal deduction.

**Worked example** (the law note's): a gross declared wage of 5,000 lei, no dependants. CAS 1,250 (of which 187.50 to Pillar II), CASS 500, deduction 0 (gross above 3,600), taxable 3,250, income tax 325, net 2,925 (58.5%). The employer also pays CAM 112.50, so the labour cost is 5,112.50 lei.

**Flow codes** (proposed; added to `econ-ledger` with the code): 3003 `tax.employer_contribution` (CAM, as proposed by [budget-2021](budget-2021.md)), 3004 `contrib.pension` (CAS to the government), 3005 `contrib.pension_pillar2`, 3006 `contrib.health` (CASS), 3007 `tax.profit`, 3008 `tax.micro`, 3009 `tax.dividend`, 3010 `tax.interest`. Income tax keeps 3001 `tax.income`.

## Outputs
Revenue by tax and by group (who pays what); effective and marginal tax rates for any group; tax wedge; distributional incidence charts; revenue vs rate history (Laffer exploration).

## Acceptance tests
IDs are stable (added 2026-10-14). Tests go in `crates/econ-mech-tax/tests/acceptance/` (protected), in a new file; written in a test-authoring session.

- [ ] **AC-TAX-01** `[unit]` The worked example: a declared gross wage of 5,000 lei with no dependants gives CAS 1,250.00 (187.50 to Pillar II), CASS 500.00, income tax 325.00, net 2,925.00 and CAM 112.50, to the ban.
- [ ] **AC-TAX-02** `[unit]` The personal deduction is `deduction_base[d]` up to 1,950 lei gross, zero from 3,600 lei, falls linearly between them in whole lei, never rises with the wage, and is higher with more dependants (at most four counted).
- [ ] **AC-TAX-03** `[unit]` The schedule: a one-bracket schedule is a flat tax; with several brackets the tax is the sum of each bracket's rate on the part of taxable income inside it, so the marginal rate at any income is that bracket's rate and the tax never falls as income rises.
- [ ] **AC-TAX-04** `[unit]` An industry override replaces the rates for that industry's employees up to its ceiling and the general rates above it: a construction worker with a gross of 4,000 lei pays no income tax and no CASS and CAS of 850.00 lei.
- [ ] **AC-TAX-05** `[unit]` Only the declared part is taxed: an envelope wage pays contributions and tax on its declared part only, a job with no contract pays none, and in every case net received = wage − contributions − tax.
- [ ] **AC-TAX-06** `[unit]` A pension pays income tax only on its part above 2,000 lei; a pension of 2,500 lei pays 50.00 lei.
- [ ] **AC-TAX-07** `[unit]` Corporate tax: 16% of a quarter's positive taxable profit after earlier losses; a loss is carried forward and used up first. A micro unit under the threshold pays 1% of declared turnover with staff, 3% without, and no profit tax.
- [ ] **AC-TAX-08** `[sim]` On every pay day, for every person and summed with weights, the employer's payment = net wage + employee contributions + income tax + CAM, and the government and the pension funds receive exactly their parts in the ledger under their flow codes.
- [ ] **AC-TAX-09** `[sim]` On the Romania scenario with the 2021 law, the first twelve months' income tax, contributions and corporate tax are each within 20% of 2021's revenue in [budget-2021](budget-2021.md) (the gap is what the undeclared economy and the simplifications explain; reported, with the tolerance the owner sets).
- [ ] **AC-TAX-10** `[sim]` *(M4)* Adding a 40% bracket above five times the median wage raises revenue mostly from the top tenth.
- [ ] **AC-TAX-11** `[sim]` *(M4)* Raising VAT from 19% to 23% raises CPI once and revenue by less than in proportion (the demand response).
- [ ] **AC-TAX-12** `[sim]` *(M15)* At extreme rates (a 90% flat income tax) revenue is lower than at moderate rates, through informality and reported income (an emergent Laffer curve).
- [ ] **AC-TAX-13** `[sim]` *(M15)* Raising social contributions raises the informal share of employment.

### VAT
The tax itself (rates, categories, rounding, the ledger posting) has its own spec, [vat](vat.md), with criteria AC-VAT-01 to 06. The VAT expectations in the list above (revenue and CPI after a rate change) stay here, because they depend on prices and demand.

## API sketch
In `crates/econ-mech-tax`, next to VAT. Amounts are bani; rates are basis points (as VAT's) or millionths where a share needs more precision.

```rust
pub struct Schedule { pub thresholds: Vec<Bani> /* ascending, first is 0 */, pub rates_bp: Vec<u32> }
impl Schedule {
    pub fn flat(rate_bp: u32) -> Schedule;
    pub fn tax(&self, taxable: Bani) -> Bani;                                   // AC-TAX-03
}
pub struct DeductionRule { pub base: [Bani; 5], pub full_until: Bani, pub zero_at: Bani }
pub fn personal_deduction(gross: Bani, dependants: u8, r: &DeductionRule) -> Bani;   // AC-TAX-02
pub struct LabourTaxes { pub cas_bp: u32, pub pillar2_bp: u32, pub cass_bp: u32, pub cam_bp: u32,
                         pub income: Schedule, pub deduction: DeductionRule }
pub struct IndustryOverride { pub income_rate_bp: Option<u32>, pub cas_bp: u32, pub cass_bp: u32, pub ceiling: Bani }
pub struct WageTax { pub cas_public: Bani, pub cas_pillar2: Bani, pub cass: Bani, pub income_tax: Bani,
                     pub cam: Bani, pub net: Bani }
pub fn tax_wage(wage: Bani, declared: Bani, dependants: u8, t: &LabourTaxes,
                over: Option<&IndustryOverride>) -> WageTax;                    // AC-TAX-01, 04, 05
pub fn tax_pension(pension: Bani, tax_free: Bani, s: &Schedule) -> Bani;        // AC-TAX-06
pub struct ProfitTaxes { pub profit_bp: u32, pub micro_threshold: Bani, pub micro_with_staff_bp: u32,
                         pub micro_no_staff_bp: u32 }
pub fn tax_quarter(profit: Bani, losses_carried: &mut Bani, micro: Option<(Bani /* declared turnover */, bool /* has staff */)>,
                   t: &ProfitTaxes) -> Bani;                                     // AC-TAX-07
```

The pay-day and quarter-end phases that call these and post through the ledger live with the [World](../game/world-loop.md).

## Open questions
- [ ] Separate capital income tax vs taxed with labour income: offer both as a lever?
- [ ] Inheritance tax in v1 (bequests already exist)?
- [ ] Negative income tax option as an alternative to minimum income?
- [ ] **First version as above** (2026-10-14 draft): the 2021 rates, the linear deduction, the two industry overrides, pensions above 2,000 lei, corporate and micro taxes quarterly, the flow codes 3003 to 3010. Accept, or change?
- [ ] **Pension tax and the health contribution on non-wage income** are from memory (medium confidence); check against the Fiscal Code when the lever registry is filled (M4).
- [ ] **The revenue tolerance of AC-TAX-09** (20%): the undeclared economy and the simplifications make 2021's revenue a check, not a target.
