---
id: economy/households
title: Households — Budgets, Consumption, Saving & Borrowing
status: locked
owner: horia
depends_on: [society/population-groups, economy/labor-market, economy/taxation, economy/social-transfers, economy/money-banking, economy/prices-inflation, economy/vat, adr/0017-time-base]
research: [research/household-wealth-romania, research/informal-economy-romania]
updated: 2026-10-11
---

# Households — Budgets, Consumption, Saving & Borrowing

## Purpose
Each synthetic household ([population-groups](../society/population-groups.md)) earns income, pays taxes, receives transfers and remittances, buys goods by its own basket, produces some food for itself (rural), saves, borrows and repays. This is where policy hits people's wallets and turns into demand.

## Real-world basis
- Marginal propensity to consume (MPC) is high for low-income and liquidity-constrained households (0.5–0.9 of a windfall) and low for the wealthy (0.05–0.2) (HANK literature; stimulus-cheque studies).
- Engel curves: food and energy shares fall with income; services, travel and durables rise.
- Precautionary saving rises with unemployment risk and uncertainty; interest rates shift saving and borrowing.
- Household inflation differs by basket (poor households face more food and energy inflation).
- Romania: about 90% of households own their home; 60% of rural households grow food for themselves; household consumption was 720 billion lei in 2021 against about 736 billion of gross money and in-kind income ([starting state](../society/population-wealth.md), [income split](../society/population-incomes.md), input-output table).

## Periods
A tick is one day ([ADR-0017](../../03-architecture/decisions/0017-time-base.md)).

| What | When |
|---|---|
| Wages, pensions and the other incomes are received | on their pay dates (month end for wages), by the mechanics that pay them |
| The household's budget decision and its purchases | **staggered**: once a month, on the household's own day of the month (1 to 28) |
| Statistics (consumption, saving rate, personal inflation) | monthly close |

"Last month's income" below means everything the household received since its previous decision day.

## State variables
Per household, all in the starting state: `hh_cash`, `hh_deposits`, `hh_mortgage`, `hh_consumer_debt`, `hh_dwelling_value`, `hh_subsistence_income`; per person: `wage`, `wage_declared`, `pension`, `self_employment_income`, `informal_income`. New here:

| code_name | Meaning | Unit |
|---|---|---|
| `hh_income_received` | money received since the last decision day | LCU |
| `hh_stagger_day` | the household's day of the month | 1–28 |
| `hh_hardship` | the household could not pay for the subsistence basket this month | bool |

## Budget (per household $h$, per month)
```math
Y^{disp}_h = \underbrace{\textstyle\sum_{i \in h} (W_i + SE_i + Pen_i + TR_i)}_{\text{members' income}} + \underbrace{r^D D_h + r^B B_h + Div_h}_{\text{capital income}} + \underbrace{Rem_h}_{\text{remittances}} - \underbrace{T_h}_{\text{taxes}} - \underbrace{r^L L_h}_{\text{interest paid}}
```
Household income is pooled. All amounts are booked per household and multiplied by its weight in the national accounts.

**Own production:** `hh_subsistence_income` is food the household grows and eats (valued at producer prices, in GDP). It is not money: it lowers what the household must buy and makes it less exposed to food inflation.

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
$\gamma_g$ = subsistence quantity (food, housing, energy, transport), eq = equivalised household size, $\beta_{h,g}$ = marginal budget shares that depend on income class, members' ages, urban or rural, and interest groups where it matters. Own-price elasticities shift shares when prices move.

**Personal price index** $P_h = \sum_g w_{h,g} p_g$ → household-specific inflation, shown for any group.

## Saving, borrowing and wealth
- Saving = $Y^{disp} - C$ → cash, deposits, bonds, equity, pension funds (portfolio shares by class and returns).
- **Borrowing:** mortgages, consumer credit, student loans ([social-transfers](social-transfers.md)). Default if debt service exceeds a share of income for long → bank losses.
- **Housing:** households own dwellings; renters pay rent ([housing](housing.md)).
- **Durables:** cars are tracked per household (`hh_vehicles`) with replacement cycles.

## First version (roadmap M2)
What is built first. Everything above that is not listed here comes with the milestone named.

**In:**
1. **Income.** Wages arrive from the [labour market](labor-market.md) at month end. Pensions arrive from the government at month end at their starting amounts. Self-employment and informal income arrive at month end too.

> **Simplification (M2 placeholder):** self-employment income, informal income and the value of food grown at home keep their starting amounts, scaled each month by the ratio of last month's nominal household consumption to that of the first month. They become firm units' own profits when production distributes them (M3) and react to enforcement with the [informal economy](informal-economy.md) (M15).

2. **The decision,** on the household's day, in integer money:

```math
C_h = \min\Big(\max\big(\lfloor c_y(q_h) \cdot Y_h + c_w \cdot M_h \rceil,\ \text{floor}_h\big),\ M_h\Big)
```

   - $Y_h$ = `hh_income_received`; $M_h$ = `hh_cash` + `hh_deposits` after income; $q_h$ = the household's fifth by last month's income per equivalised adult (the fifths' bounds are those of the previous monthly close).
   - $c_y(q)$ = `mpc[q]`, from 0.95 in the poorest fifth to 0.60 in the richest; $c_w$ = `mpc_wealth`, 0.5% a month.
   - $\text{floor}_h$ = `subsistence_basket` × equivalised adults (1 for the first adult, 0.5 per further adult, 0.3 per child under 14) × the consumer price index, less `hh_subsistence_income`, not below zero.
   - `hh_hardship` is set when $M_h$ is below the floor.
3. **The basket:** $C_h$ is split over the goods by fixed shares `basket_share[g]`, the same for every household: the household consumption column of the input-output table, mapped to the catalogue's goods. Largest-remainder split to whole bani.
4. **Buying.** For each good the household pays the purchaser price; VAT is carved out by the [VAT](vat.md) mechanic; the rest goes to the good's industry through its sales clearing account ([accounting](accounting.md)). If the industry cannot deliver (no inventory), the unspent money stays with the household and the shortfall is recorded as a shortage.
5. **Saving** is what is left: money stays in `hh_cash` and `hh_deposits` in their current proportion.
6. **Budget identity:** income received − spending = change in cash and deposits, per household, every month, exactly.

**Not in M2:** taxes other than VAT and all benefits other than pensions (M3); interest, dividends, borrowing, repayment and default (M3); remittances (M5); baskets that differ by income, age and place, and personal inflation (M6, with the group statistics); durables and housing costs (M13); the influence of the interest rate, confidence and unemployment risk on $c_y$ (M3).

## Player levers
Indirect: all taxes, transfers, minimum wage, interest rate, credit rules, price caps, VAT per category, subsidies on goods.

## Outputs
Consumption by good (demand for production), saving, credit demand, real income, personal inflation, poverty, debt burden. Available for any group. In M2: consumption by good and in total, the saving rate, households in hardship, shortages by good.

## Tuning parameters
| Parameter | Default | Effect |
|---|---|---|
| `mpc[q]` | 0.95, 0.90, 0.85, 0.75, 0.60 by income fifth | share of the month's income spent |
| `mpc_wealth` | 0.005 a month | share of cash and deposits spent each month |
| `subsistence_basket` | 700 lei a month per equivalised adult at starting prices | the floor |
| later | LES parameters by class, age and place; confidence sensitivity; default threshold; durable replacement cycle | |

The defaults make the starting population spend about what the national accounts record (720 billion lei a year) out of its starting income; calibration (M7) sets them.

## Interactions
- **Before, in a month:** incomes are paid ([labour market](labor-market.md), social transfers); prices are set ([prices](prices-inflation.md)).
- **After:** the day's purchases are the demand that [production](production.md) serves from inventories; VAT goes to the government.
- The stagger means that a pay rise at month end reaches spending over the following four weeks, not on one day.

## Edge cases & failure modes
- **No income and no money:** the household buys nothing and is in hardship; it is not removed.
- **Money below the floor:** it spends all it has.
- **A good with no supply:** see step 4. No substitution between goods in M2.
- **A household that forms or dissolves** ([demographics](demographics.md), M5): it keeps or gets a day of the month; money moves by the rules of [population-groups](../society/population-groups.md).

## Acceptance tests
IDs are stable (added 2026-10-11). Tests go in the mechanic's crate, in `tests/acceptance/` (protected).

- [ ] **AC-HH-01** `[unit]` The decision rule: for given income, money, fifth, floor and parameters, spending is the formula of step 2, in whole bani; it never exceeds the household's money.
- [ ] **AC-HH-02** `[unit]` The basket split gives every good its share and sums to the household's spending exactly.
- [ ] **AC-HH-03** `[sim]` Budget identity: for every household and every month, income received − spending = change in cash and deposits, exactly; summed with weights it equals the ledger's household flows.
- [ ] **AC-HH-04** `[sim]` Each household decides exactly once in every calendar month, on its own day; over a month every household has decided.
- [ ] **AC-HH-05** `[sim]` A transfer of 1,000 lei to every household of the poorest fifth raises aggregate consumption over the next two months by more than the same total given to the richest fifth.
- [ ] **AC-HH-06** `[sim]` A household whose money is below its floor spends all of it and is flagged in hardship; one with food grown at home has a lower floor.
- [ ] **AC-HH-07** `[sim]` When an industry has nothing to sell, the money meant for its good stays with the households and the shortage is recorded; no money leaves without a sale.
- [ ] **AC-HH-08** `[sim]` On the Romania scenario with every other mechanic at its starting values, household consumption of the first twelve months is within 10% of the input-output table's 720 billion lei.
- [ ] **AC-HH-09** `[sim]` *(M3)* Raising interest rates lowers mortgage borrowing and car purchases within a few months.
- [ ] **AC-HH-10** `[sim]` *(M6)* A food price spike raises personal inflation of poor urban households more than of rich households or rural households with own production.

### Tests owed
The spec was locked before these tests could exist (ADR-0014, Amendment 1). A line is removed when its test is written.
- AC-HH-01: the households mechanic (roadmap M2)
- AC-HH-02: the households mechanic (roadmap M2)
- AC-HH-03: the households mechanic on the real `World` (roadmap M2)
- AC-HH-04: the scheduler and the households mechanic (roadmap M2)
- AC-HH-05: the households mechanic on the real `World` (roadmap M2)
- AC-HH-06: the households mechanic (roadmap M2)
- AC-HH-07: households with the goods market (roadmap M2)
- AC-HH-08: the full M2 loop on the Romania scenario
- AC-HH-09: borrowing and interest (roadmap M3)
- AC-HH-10: baskets by group (roadmap M6)

## API sketch
For the test writer and the implementer; names may change, shapes should not.

```rust
// crate econ-mech-households (no I/O)
pub struct HouseholdParams { pub mpc: [u32; 5] /* millionths, by income fifth */, pub mpc_wealth_ppm: u32, pub subsistence_basket: Bani }
pub struct Budget { pub income_received: Bani, pub money: Bani, pub fifth: u8, pub equivalised_adults_milli: u32,
                    pub subsistence_income: Bani, pub price_index_ppm: u32 }
pub struct Decision { pub spending: Bani, pub hardship: bool }
pub fn decide(b: &Budget, p: &HouseholdParams) -> Decision;                          // AC-HH-01, 06
pub fn split_basket(spending: Bani, basket_share_ppm: &[u32]) -> Vec<Bani>;          // AC-HH-02
```

The phase that walks today's households, calls these, posts through the ledger and the VAT mechanic and records shortages lives with the `World` (roadmap M2).

## Open questions
- [x] Landlords: households own the dwellings they let; rents are part of [housing](housing.md) (M13). Decided 2026-10-11 under the owner's delegation.
- [x] First version, periods and parameters as above: accepted by the owner's delegation, 2026-10-11 ("accept the drafts").
- [ ] `mpc[q]` and the floor are guesses until calibration (M7); the survey behind the fifths is Eurostat's income-consumption statistics for 2020.
