---
id: economy/households
title: Households — Budgets, Consumption, Saving & Borrowing
status: draft
owner: horia
depends_on: [society/population-groups, economy/labor-market, economy/taxation, economy/social-transfers, economy/money-banking, economy/prices-inflation]
research: []
updated: 2026-10-08
---

# Households — Budgets, Consumption, Saving & Borrowing

## Purpose
Each synthetic household ([population-groups](../society/population-groups.md)) earns income, pays taxes, receives transfers and remittances, buys goods by its own basket, produces some food for itself (rural), saves, borrows and repays. This is where policy hits people's wallets and turns into demand.

## Real-world basis
- Marginal propensity to consume (MPC) is high for low-income and liquidity-constrained households (0.5–0.9 of a windfall) and low for the wealthy (0.05–0.2) (HANK literature; stimulus-cheque studies).
- Engel curves: food and energy shares fall with income; services, travel and durables rise.
- Precautionary saving rises with unemployment risk and uncertainty; interest rates shift saving and borrowing.
- Household inflation differs by basket (poor households face more food and energy inflation).
- Romania-specific: very high home ownership, a large share of rural households with own food production, and significant remittances from family abroad *(verify magnitudes in research)*.

## Budget (per household $h$, per tick)
```math
Y^{disp}_h = \underbrace{\textstyle\sum_{i \in h} (W_i + SE_i + Pen_i + TR_i)}_{\text{members' income}} + \underbrace{r^D D_h + r^B B_h + Div_h}_{\text{capital income}} + \underbrace{Rem_h}_{\text{remittances}} - \underbrace{T_h}_{\text{taxes}} - \underbrace{r^L L_h}_{\text{interest paid}}
```
Household income is pooled. All amounts are booked per household and multiplied by its weight in the national accounts.

**Own production:** households with farm land produce food for own use (valued at producer prices, counted in GDP as in national accounts). This lowers their food purchases and makes them less exposed to food inflation.

## Consumption
```math
C_h = c_y(\text{class}_h, \kappa_h, r - \pi^e, \text{risk}_h) \cdot Y^{disp}_h + c_w \cdot NW^{liquid}_h + \text{new borrowing}_h
```
- $c_y$ falls with income class, rises with confidence, falls with the real interest rate and with members' unemployment risk.
- Floor: consumption can't drop below a subsistence level per equivalised adult (paid from deposits, borrowing, family or charity). If there's no way to pay → **hardship** flag (feeds health, crime and poverty statistics).

**Basket split** (linear expenditure system):
```math
C_{h,g} = p_g \gamma_{g} \cdot \text{eq}_h + \beta_{h,g} \Big(C_h - \sum_{g'} p_{g'} \gamma_{g'} \cdot \text{eq}_h\Big)
```
$\gamma_g$ = subsistence quantity (food, housing, energy, transport), eq = equivalised household size, $\beta_{h,g}$ = marginal budget shares that depend on income class, members' ages (students: rent, food, electronics, hospitality; retirees: health, food, energy), urban or rural, and interest groups where it matters (motorists spend more on fuel and vehicles). Own-price elasticities shift shares when prices move.

**Personal price index** $P_h = \sum_g w_{h,g} p_g$ → household-specific inflation, shown for any group.

## Saving, borrowing and wealth
- Saving = $Y^{disp} - C$ → cash, deposits, bonds, equity, pension funds (portfolio shares by class and returns).
- **Borrowing:** mortgages (house purchase; loan rate, income, house price, bank lending standards), consumer credit (durables like cars), student loans ([social-transfers](social-transfers.md)). Default if debt service exceeds a share of income for long → bank losses.
- **Housing:** households own dwellings (value = dwelling × county house price); renters pay rent. Prices, rents, construction and vacancies are specified in [housing](housing.md).
- **Durables:** cars are tracked per household (`hh_vehicles`) with replacement cycles, so a car-price rise delays purchases rather than just cutting a flow.

## Player levers
Indirect: all taxes, transfers, minimum wage, interest rate, credit rules, price caps, VAT per category, subsidies on goods.

## Outputs
Consumption by good (demand for production), saving, credit demand, real income, personal inflation, poverty, debt burden. Available for any group.

## Tuning parameters
MPC by class, wealth effect $c_w$, LES parameters by class, age and urban/rural, confidence sensitivity, default threshold, durable replacement cycle.

## Acceptance tests
- [ ] A transfer of 1,000 to poor households raises aggregate consumption more than the same transfer to rich households.
- [ ] Raising interest rates lowers mortgage borrowing and car purchases within a few ticks.
- [ ] A food price spike raises personal inflation of poor urban households more than of rich households or rural households with own production.
- [ ] Budget identity holds per household every tick: $Y^{disp} - C = \Delta$ net financial assets.

## Open questions
- [ ] Do landlord households own rental dwellings directly (proposed), or does the `real_estate` industry own all rental stock?
