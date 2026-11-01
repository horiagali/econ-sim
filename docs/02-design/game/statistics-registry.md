---
id: game/statistics-registry
title: Statistics Registry — One Catalogue and History of Every Indicator
status: draft
owner: horia
depends_on: [game/information, game/world-loop, game/lever-system, adr/0008-explainability-architecture, adr/0011-storage-saves-history, adr/0017-time-base, glossary]
research: []
updated: 2026-11-01
---

# Statistics Registry — One Catalogue and History of Every Indicator

## Purpose
Roadmap M6, first slice (owner, 2026-11-01: close M5, then M6, then M7). Every number the player can look at in the alpha ([information](information.md), "D3": views 1 to 9 and 15) comes from one place: a **registry** of indicators, each with an id, a unit, a period, its breakdowns and the spec that defines it, and one **history** the `World` fills at every close. The UI (M8), the group queries, the saves' history and the "why did this change" trees of the later M6 slices all read it.

Today each mechanic keeps its own month record (`months`, `trade_months`, `fiscal_months`, `budget_months`, `investment_months`, `revenue_months`, `poverty_stats`, `approval_stats`, `wellbeing_stats`, `migration_months`, `remittance_months`, and the state views of the banks, the central bank and the treasury). The registry does not replace them in M6: it reads them, so that each indicator has exactly one definition, and the tests compare the two.

## The registry
A file `crates/econ-core/indicators.toml`, one entry per indicator, with a generator (as `levers.toml`, [lever system](lever-system.md)): `scripts/codegen_indicators.py` writes `IndicatorId` and its metadata to `crates/econ-core/src/indicators_gen.rs` and `ui/src/generated/indicators.json`; `just codegen-check` keeps them in step.

| Field | Meaning |
|---|---|
| `id` | dotted, stable: `gdp.nominal`, `prices.cpi`, `labour.unemployment_rate`, `budget.deficit`, `people.poverty_rate`, … |
| `glossary` | the glossary `code_name` it shows (each indicator is a glossary term, or derived from one by the `derive` field) |
| `unit` | `bani` (money, national, current prices), `real_bani` (day 0's prices), `ppm` (rates and shares, millionths), `persons`, `index_ppm` (an index, 1,000,000 at the start), `milli` (thousandths: years, points), `count` |
| `period` | `month` (a point at each monthly close) or `day` (also a point each day: the fast series of [ADR-0017](../../03-architecture/decisions/0017-time-base.md)) |
| `breakdowns` | any of `region`, `county`, `industry`, `class`, `fifth`, `age_band`, `sex`, `edu`, `activity`, `level`, `flow`, `line` (a budget line), `good`; national is always there |
| `aggregate` | how breakdowns relate to the national value: `sum` (counts and money: the parts add up exactly), `mean` (weighted by the breakdown's population, to within rounding), `none` (rates over different bases, ratios) |
| `source` | the spec that defines it, and the month record or state it is read from |
| `derive` | for derived indicators: `ratio(a, b)`, `real(a)` (deflated by the CPI), `growth_yoy(a)`, `growth_mom(a)`, `per_capita(a)` |
| `feature` | the cargo feature that computes its source (an indicator whose feature is not compiled has no points, not zeros) |
| `views` | the views of [information](information.md) it appears in (`overview`, `budget`, `prices`, `finance`, `trade`, `people`, `informal`, `map`) |
| `headline` | true for the overview's indicators: they get a "why did this change" tree in M6's fourth slice |
| `milestone`, `status` | as levers: `alpha` and `active`, or `planned` for an indicator whose mechanic comes later |

## The history
At the end of every monthly close, after every mechanic's own close, the `World` appends one **frame**: for each active indicator whose feature is compiled, its national value and its breakdowns' values (`i64` in the unit's scale). `period = day` indicators also get a point at the end of every day. The history is outputs only: no rule reads it, and it is outside the state hash.

- **Storage in memory:** per indicator and breakdown key, a column of monthly points from month 0, and for day indicators a column of daily points (all of them in M6; [ADR-0011](../../03-architecture/decisions/0011-storage-saves-history.md) bounds what saves keep, M6's third slice).
- **Derived indicators** are computed from the frame's own values with the rounding of [ADR-0006](../../03-architecture/decisions/0006-determinism-contract.md) (integers, half away from zero): `ratio` in millionths, `real` = value × 1,000,000 / CPI, `growth_yoy` = value / value 12 months before − 1 (none before month 12), `per_capita` over resident persons.
- **Values equal their source exactly:** an indicator read from a month record has that record's value for that month (the tests compare them); the registry computes nothing a mechanic already computes.

## The alpha indicators
Grouped by the [information](information.md) views. **(new)** marks a value no month record holds today: it is computed by the registry from the state at the close, as its spec defines it.

| View | Indicators (ids) | Breakdowns |
|---|---|---|
| 1 Overview (headline) | `gdp.nominal`, `gdp.real`, `gdp.per_capita`, `gdp.growth_yoy`, `prices.cpi`, `prices.inflation_yoy`, `labour.unemployment_rate`, `labour.employed`, `wages.mean_nominal`, `wages.mean_real`, `budget.balance`, `debt.gdp_ratio`, `money.policy_rate` (day), `fx.eur_lei` (day), `trade.balance`, `approval.national`, `people.gini`, `people.population` | region, county where the source has them |
| 3 People (M5) | `people.poverty_rate`, `people.poverty_rate_anchored`, `people.class_share` (class), `people.hardship_rate`, `people.s80_s20`, `people.sol` (region, county, class, fifth, age_band), `people.wellbeing` (region), `people.life_expectancy`, `people.births`, `people.deaths`, `people.emigrants`, `people.returns`, `people.immigrants`, `people.diaspora`, `people.remittances`, `education.enrolled` (level), `education.early_leavers_rate`, `education.tertiary_attainment`, `approval.by_group` (activity, fifth, region, county, age_band, edu, sex) | as listed (each with its feature) |
| 4 Industry | `industry.output_real`, `industry.price`, `industry.employment`, `industry.wage_cost_per_unit` **(new)**, `industry.investment` | industry |
| 5 Budget | `budget.revenue` (flow), `budget.spending` (flow), `budget.interest`, `budget.deficit`, `debt.stock`, `budget.line_paid` (line), `budget.eu_net_balance` | as listed |
| 6 Prices | `prices.cpi`, `prices.inflation_mom`, `prices.good` (good, day), `prices.cpi_contribution` **(new)** (good: basket weight × the good's price change), `prices.expected_inflation` | as listed |
| 7 Finance | `money.policy_rate` (day), `money.deposits_households`, `money.credit_households`, `money.credit_firms`, `money.cash` (ledger), `money.cb_foreign_assets` | — |
| 8 Trade | `fx.eur_lei` (day), `trade.exports_volume`, `trade.current_account`, `trade.capital_account`, `trade.current_account_proper` (current less capital), `fx.intervention` | — |
| 9 Map | every indicator above with a `county` or `region` breakdown | county, region |
| 15 Informal | `informal.wages_declared_share` **(new)**, `informal.mixed_income_declared_share` **(new)**, `budget.revenue_lost` **(new, from the held payments and the tax gap of [informal economy](../economy/informal-economy.md))** | county, industry |

The list is the M6 minimum; each later milestone adds its indicators to the registry with its mechanic.

## API sketch
Names may change before lock; shapes should not. There is no code yet (tests come first, ADR-0014).

```rust
// generated from indicators.toml
pub enum IndicatorId { GdpNominal, GdpReal, /* … */ }
pub struct IndicatorMeta { pub id: IndicatorId, pub name: &'static str, pub glossary: &'static str,
    pub unit: Unit, pub period: Period, pub breakdowns: &'static [Breakdown], pub aggregate: Aggregate,
    pub derive: Option<Derive>, pub feature: Option<&'static str>, pub views: &'static [View],
    pub headline: bool, pub status: Status }
pub fn indicators() -> &'static [IndicatorMeta];

// crate econ-core, module `stats` (feature `stats`)
pub struct Key { pub breakdown: Option<Breakdown>, pub index: u32 }      // None: national
pub struct Series<'a> { pub months: &'a [i64], pub days: Option<&'a [i64]>, pub first_month: Month }
impl World {
    pub fn stats(&self) -> &StatsHistory;
}
impl StatsHistory {
    pub fn series(&self, id: IndicatorId, key: Key) -> Option<Series<'_>>;  // None: not computed
    pub fn latest(&self, id: IndicatorId, key: Key) -> Option<i64>;
    pub fn keys(&self, id: IndicatorId) -> &[Key];
}
```

## Acceptance tests
IDs are stable. Tests go in `crates/econ-core/tests/acceptance/stats.rs` (protected; written by the test-writer before the code), behind a feature `stats`.

- [ ] **AC-STAT-01** `[unit]` The registry parses; every entry has a known unit, period, breakdown, aggregate and view; ids are unique; every `glossary` names a glossary `code_name` (or the entry derives from one); `just codegen-check` passes (the generated files are in step).
- [ ] **AC-STAT-02** `[sim]` At 1:1000 under the 2021 law with the default features, after 12 closes, every active indicator whose feature is compiled has exactly 12 monthly national points, and each indicator read from a month record equals that record's value for every month (e.g. `gdp.nominal` = `months()[t].gdp_production`, `budget.deficit` = `fiscal_months()[t].deficit`, `trade.current_account` = `trade_months()[t].current_account`); an indicator whose feature is not compiled has no series.
- [ ] **AC-STAT-03** `[sim]` Breakdowns: at every close of AC-STAT-02's run, for indicators with `aggregate = sum` the breakdown's values add up to the national value exactly; with `aggregate = mean`, the national value is the population-weighted mean of the breakdown's values to within one unit of the last place per breakdown member.
- [ ] **AC-STAT-04** `[unit]` Derived indicators follow their formulas on given inputs: `ratio`, `real`, `growth_yoy` (absent before month 12), `growth_mom`, `per_capita`, in integers rounded half away from zero.
- [ ] **AC-STAT-05** `[sim]` Day indicators (`fx.eur_lei`, `money.policy_rate`, `prices.good`) have one point per day stepped, the last day's point of each month equal to the month's point; month indicators have none.
- [ ] **AC-STAT-06** `[sim]` Isolation: at 1:1000, the state hash after 24 months is the same with `stats` compiled and not.
- [ ] **AC-STAT-07** `[sim]` With the M5 features compiled (`home_quality`, `education`, `approval`), the people indicators exist and equal their records (`people.poverty_rate` = `poverty_stats()[t].national.poverty_rate_ppm`, `people.sol` = `poverty_stats()[t].sol.national_milli`, `people.wellbeing` = `wellbeing_stats()[t].national.index_ppm`, `people.emigrants` = `migration_months()[t].emigrants`, `approval.national` = the approval stats' national mean).
- [ ] **AC-STAT-08** `[bench]` The registry adds at most 10% to a simulated year at 1:100 in a release build.

## Open questions
- [x] **Replace the month records?** *Proposed (2026-11-01):* not in M6; the registry reads them and the tests tie each indicator to its record. M8's UI reads only the registry; the records may then become private.
- [x] **On by default?** *Proposed:* the feature `stats` joins the defaults once AC-STAT-01 to 08 pass (it changes no state, so no golden value).
- [x] **Daily points' retention.** *Proposed:* all of them in memory in M6 (a 50-year run at one point a day for a few dozen series is small); what saves keep is ADR-0011's and M6's third slice.
- [ ] **The new indicators** (wage cost per unit by industry, CPI contributions, the informal shares and revenue lost): computed by the registry from the state as their specs define them. Confirm the list, or move some to their mechanics' milestones.
