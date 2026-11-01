---
id: game/explanations
title: Explanations — "Why Did This Change?" for the Headline Indicators
status: draft
owner: horia
depends_on: [game/information, game/statistics-registry, adr/0008-explainability-architecture, economy/prices-inflation, economy/accounting, economy/fiscal-policy, society/poverty-inequality, society/opinion-approval, society/social-outcomes]
research: []
updated: 2026-11-01
---

# Explanations — "Why Did This Change?" for the Headline Indicators

## Purpose
Roadmap M6, fourth slice; its exit is this slice's (owner, 2026-11-01): **for each headline indicator, any month's change is decomposed into named causes that sum exactly.** The [information](information.md) spec's causal panel reads it; M8 draws it as nested tooltips ([ADR-0013](../../03-architecture/decisions/0013-ui-stack.md)).

[ADR-0008](../../03-architecture/decisions/0008-explainability-architecture.md) layers the explanations: the ledger's flow codes (L1), rules' driver terms (L2), distribution shift-share (L3), counterfactuals (L4, after v1) and events (L5). Few of the `World`'s rules are written with `behaviour_rule!` yet, so the M6 trees use exact **accounting decompositions** (L1 and L3): the indicator's own definition splits its change into parts that add up by construction. L2 trees of individual rules join them as rules move to `behaviour_rule!` (each milestone).

## The tree
`explain(indicator, key, from, to)` returns a `ContributionTree` ([econ-rules](../../../crates/econ-rules/src/tree.rs)): the root's value is the indicator's value at month `to` less at month `from` (in the indicator's unit), and its children's values **sum to it exactly** (integers; a decomposition computed in floats is rounded to integers by largest remainders, so the sum holds). A child that is itself a registry indicator carries a reference to it, so the panel can drill down. A window of several months adds the monthly trees child by child (the same children each month).

## The headline decompositions

| Indicator | Children of a month's change | Method |
|---|---|---|
| `gdp.nominal` | consumption, government purchases, investment, exports, inventories, less imports | the expenditure identity of [national accounts](../economy/accounting.md) (exact) |
| `gdp.real`, `wages.mean_real`, `gdp.per_capita` | the nominal value, and the CPI (or population) | LMDI on value = nominal ÷ deflator (exact in logs, ADR-0008) |
| `gdp.growth_yoy` | each expenditure component's contribution to the year's real growth | contributions to growth (the components' changes over GDP a year before) |
| `prices.cpi` | each good: its basket share × its consumer price's change | the CPI is a fixed-basket sum (exact); this is `prices.cpi_contribution` (planned until this slice) |
| `prices.inflation_yoy` | each good's contribution to the twelve-month change of the CPI | as `prices.cpi` over twelve months, over the CPI a year before |
| `labour.unemployment_rate` | each region's rate change (within) and the regions' labour-force shares' change (composition) | shift-share with midpoint weights (exact) |
| `labour.employed` | each region's change | the sum by region (exact) |
| `wages.mean_nominal` | each industry's wage change (within) and its employment share's change (composition) | shift-share, midpoint weights (exact) |
| `budget.balance` | each revenue flow code (+) and each spending flow code and interest (−) | the ledger's flow codes, L1 (exact) |
| `debt.gdp_ratio` | the debt's change and the GDP's change | LMDI on debt ÷ GDP |
| `money.policy_rate` | the player's orders, and the advisory rule's change (with its terms when the rule exposes them) | who set the rate (exact) |
| `fx.eur_lei` | the exchange-rate step's terms ([trade and FX](../economy/trade-fx.md), step 4) where it exposes them, else one child "the exchange-rate step" | the step's own terms |
| `approval.national` | the five parts of approval (income, unemployment, inflation, hardship, bounds) | the parts' change (exact: the mean is 50 plus the parts) |
| `people.population` | births, resident deaths, emigrants, returns, immigrants (with migration), else births and deaths | the balance of persons (exact) |
| `people.sol` (people view) | purchases, home-grown food, imputed rent, health, education | the five parts of step 9 (exact) |
| `people.poverty_rate_anchored` | households entering and leaving the class at risk, and persons added and removed (births, deaths, migration) | the class flows of [poverty](../society/poverty-inequality.md), step 7 (exact) |
| `people.wellbeing` | health, education and living standard | LMDI on the geometric mean (each dimension's Δln ÷ 3) |

`people.gini` and `trade.balance` have no tree in M6: the Gini has no exact decomposition of its change that the player can read (its income-source decomposition comes with M14's distribution depth), and `trade.balance` is `planned` in the registry.

## API sketch
Names may change before lock; shapes should not. Tests come first (ADR-0014).

```rust
// crate econ-core, module `explain` (feature `explain`, needs `stats`)
pub struct Node { pub label: String, pub value: i64, pub indicator: Option<(IndicatorId, Key)>, pub children: Vec<Node> }
impl World {
    pub fn explain(&self, id: IndicatorId, key: Key, from: Month, to: Month) -> Option<Node>; // None: no tree for it
    pub fn explained(&self) -> &'static [IndicatorId];                                        // the indicators with a tree
}
pub fn lmdi(y1: f64, y0: f64, parts: &[(f64, f64)]) -> Vec<f64>;   // ADR-0008's helper, exact sum
pub fn round_to_sum(values: &[f64], total: i64) -> Vec<i64>;        // largest remainders
```

## Acceptance tests
IDs are stable. Tests in `crates/econ-core/tests/acceptance/explain.rs` (protected; the test-writer's), feature `explain`.

- [ ] **AC-WHY-01** `[unit]` `round_to_sum` returns integers summing to the total, each within one of its float; `lmdi` contributions sum to `y1 − y0` within 1e-9 relative; a node's children sum to its value.
- [ ] **AC-WHY-02** `[sim]` At 1:1000, for every indicator of `explained()` and every month `t` of 1 to 12: the tree of (t − 1, t) has the root value `indicator(t) − indicator(t − 1)` exactly and its children sum to it exactly; the same for the window (0, 12) (for `gdp.growth_yoy` and `prices.inflation_yoy`, the window of the year to month 12). With the M5 features compiled, the people trees too.
- [ ] **AC-WHY-03** `[sim]` Causes: raising `world.fuel_price[oil]` by 50% from month 2 makes the oil goods' children the largest positive contribution to the CPI's change over months 2 to 4; raising the standard VAT rate by 2 points from month 2 makes VAT the largest positive child of the budget balance's change over months 2 to 4.
- [ ] **AC-WHY-04** `[sim]` The trees agree with their records: the SoL tree's children are the stats' `parts_change_milli`; the approval tree's children are the change of the national group's `parts_milli`; the population tree's children are the migration month's flows.
- [ ] **AC-WHY-05** `[sim]` A policy rate set by an order: the month's tree of `money.policy_rate` has the child "player's orders" equal to the change.
- [ ] **AC-WHY-06** `[unit]` Drill-down: every child that names a registry indicator names an active one with that key, and its value equals that indicator's change over the window when the child is the indicator itself (for example the VAT child of `budget.balance` equals `budget.revenue`'s VAT member's change).
- [ ] **AC-WHY-07** `[sim]` Isolation: computing trees changes no state (the state hash after 6 closes is the same whether `explain` is called at every close or never).

## Implementation status
- **2026-11-01:** built (feature `explain`, off by default; `explain.rs`, `world/explain_state.rs`). The trees read the registry's history, the mechanics' month records, and a few details the `World` keeps at each close: GDP's six expenditure parts, each good's CPI term, employed and unemployed persons by region, each industry's wages and staff, and whether the policy rate at the close is the player's. Readings where the spec leaves room:
  - Each month's parts are computed in floats and rounded by largest remainders to the month's change (records that are integers, such as the flows, the SoL parts and the approval parts, come out unchanged); a window adds the monthly trees child by child. The year-on-year rates' window of twelve months explains their value, another window their change (from month 12 on).
  - `prices.cpi`'s children are `prices.cpi_contribution`'s members. `prices.inflation_yoy`'s first year (annualised from day 0) shares the rate among the goods by their terms' change since day 0.
  - `labour.unemployment_rate` and `wages.mean_nominal` have two children (within, composition), each with one part per region or industry. The mean wage's shares are of all employed persons, so that persons outside the firm units' staff (the self-employed) count as a group without wages of the bill.
  - `labour.employed` by region: the indicator has no regional breakdown, so the regions' children name no indicator.
  - `budget.balance`: one child per revenue and spending flow code with a point other than zero in the history, and interest.
  - `money.policy_rate`, "who set the rate": once an order has taken effect the rule no longer sets the rate, so a month whose closing rate is the player's gives the player's orders the whole change; before that, the advisory rule.
  - `people.population` without migration: each region's change (its registry member), as births and deaths have no month record without the migration feature.
  - `people.poverty_rate_anchored`: five children from the class flows of step 7 (entering, leaving, persons added, persons removed, and the change of the persons counted).
  - AC-WHY-01 to 07 pass, with and without the M5 features.

## Open questions
- [x] **The Gini's tree.** *Proposed (2026-11-01):* with M14's distribution depth (the income-source decomposition of the level, then of the change); none in M6.
- [x] **Trees of rules (L2).** *Proposed:* as each rule moves to `behaviour_rule!` with its milestone; the M6 trees are accounting decompositions.
