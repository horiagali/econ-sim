---
id: game/world-loop
title: World and Daily Loop (first version)
status: locked
owner: horia
depends_on: [game/overview, adr/0005-simulation-core-architecture, adr/0017-time-base, adr/0018-scenario-and-commands, adr/0007-money-and-ledger, economy/accounting, game/scenario-builder, economy/opening-balance-sheet, economy/households, economy/labor-market, economy/production, economy/prices-inflation, economy/vat]
research: []
updated: 2026-11-01
---

# World and Daily Loop (first version)

## Purpose
The real `World` of [ADR-0005](../../03-architecture/decisions/0005-simulation-core-architecture.md): it loads a scenario, advances one day at a time, runs the processes that are due that day in a fixed order, moves every leu through the ledger and checks the invariants every day. This spec is roadmap M2's "World and tick loop" and "Clock and scheduler". It wires the four M2 mechanics, whose rules are in their own specs ([households](../economy/households.md), [labour market](../economy/labor-market.md), [production](../economy/production.md), [prices](../economy/prices-inflation.md), each "First version"), and it is the vehicle for their `[sim]` criteria, which are owed until a `World` exists. It replaces the scale world of the spikes for everything after M2.

## Real-world basis
None of its own: the order of a day follows the causal order of the [game overview](README.md) ("Tick order"), and periods follow [ADR-0017](../../03-architecture/decisions/0017-time-base.md).

## Scope
**This increment (M2):** the four mechanics on the Romania scenario, with the placeholders their specs name: government buys and pays at fixed real levels and taxes with VAT only; no banks (deposits only, no interest, no credit); imports at fixed prices; exports and investment at fixed real levels. No player commands yet: the lever system is M4, so `step` takes an empty command list and the law in force is the placeholder below.

**Not in this increment:** taxes other than VAT, benefits other than pensions, banks, interest, credit, the exchange rate and trade (M3); levers and commands (M4); people over time (M5); the "why" panel (M6).

## Inputs
- A built scenario ([scenario builder](scenario-builder.md)): the population tables (stages A to J), the firm-unit table, the opening balance sheet.
- The input-output table (`io_table2021_ro.json`): input coefficients of every catalogue industry (home and imported), the household basket (`basket_share[g]`), and the placeholder final demand of each industry (government purchases, investment, exports, a year at starting prices).
- The placeholder law (below), the four mechanics' parameters at their spec defaults, `rng_seed`.

**The placeholder law** until `law.json` exists (M4): the VAT schedule of 1 December 2021 (19%, 9%, 5%) with each good's VAT category from the [law in force](../../01-research/notes/law-in-force-romania-2021.md); the minimum wage of 2,300 lei. It is a Rust constant the M2 tests pass in, not a file.

## State
`World` holds column tables ([ADR-0005](../../03-architecture/decisions/0005-simulation-core-architecture.md)); sizes come from the scenario, never from a constant ([ADR-0003](../../03-architecture/decisions/0003-people-representation.md)).

| Table | Columns this increment adds or moves |
|---|---|
| households | `hh_cash`, `hh_deposits` (moving), `hh_income_received`, `hh_stagger_day`, `hh_hardship`, the fifth by last month's income |
| persons | `employer`, `occupation`, `wage`, `wage_declared` (moving), `months_unemployed` |
| firm units | `inventory`, `expected_demand`, `capacity`, `unit_stagger_day`, `sales_since_plan`, the month's plan and input orders, the hiring carry by occupation, `currency` and `deposits` (moving) |
| industries | `price`, `markup0`, wage cost per unit of last month, the day's demand and shortage |
| macro | the calendar and today's `Day`, CPI and its history, `inflation_expectation`, the income fifths' bounds, tightness by industry group (twelve monthly closes) |

The ledger ([accounting](../economy/accounting.md)) holds the sector balances; the household and firm-unit columns are their sub-ledgers, and check I-4 ties them: every day the weighted sums of `hh_cash` and `hh_deposits` equal the ledger's household `cash` and `deposits`, and the firm units' `currency` and `deposits` equal the firms'.

## Update rule
One call of `step` is one day. On a day, the phases below run in this order, each only when its period is due ([ADR-0017](../../03-architecture/decisions/0017-time-base.md); `econ_core::schedule`). The list is one `const` list in one file.

| # | Phase | Period | What it does (rules in the mechanic's spec) |
|---|---|---|---|
| 1 | Commands | daily | Apply the commands due today. None in M2. |
| 2 | Plan | staggered (firm units) | [Production](../economy/production.md) step 1: expected demand, planned output, desired employment, the month's input orders. |
| 3 | Staffing | staggered (firm units, same day, after 2) | [Labour market](../economy/labor-market.md) steps 1 and 2: layoffs, vacancies, hires. |
| 4 | Prices | daily | [Prices](../economy/prices-inflation.md) steps 1 to 4: unit cost, markup, target, the day's price of each industry. |
| 5 | Output and inputs | daily | Production step 2: the day's output into inventory; the day's input orders become demand on the supplying industries (home) and a payment to the rest of the world (imported, at the starting price). |
| 6 | Fixed final demand | daily | The placeholders: each industry's government purchases, investment and exports of a year at starting prices, ÷ 365, as demand. |
| 7 | Households | staggered (households) | [Households](../economy/households.md) steps 2 to 4: the decision, the basket, and the day's purchases as demand on each good's industry. |
| 8 | Goods market | daily | Production step 2: per industry, ration the day's demand against inventories, share sales over units, book the money (below), record shortages. VAT is carved out of household purchases ([VAT](../economy/vat.md)). |
| 9 | Quits | month end | Labour market step 3. |
| 10 | Pay | month end | Labour market step 4 (wages for the days worked); pensions at their starting amounts; self-employment and informal income by the households' M2 placeholder. Each household's `hh_income_received` grows by what it got. |
| 11 | Wage round | yearly, 1 January | Labour market step 5. |
| 12 | Monthly close | month end, after 10 | CPI, inflation and expectations (prices steps 5 and 6); the income fifths; tightness; unemployment; GDP three ways; the month's statistics. |
| 13 | Checks | daily, last | Ledger invariants I-1 to I-3 and I-8, check I-4 above, I-7 at the monthly close, no negative stock; the state hash. |

A household's or unit's stagger day is drawn once at load with `econ_core::schedule::stagger_day` (keyed draw, [ADR-0006](../../03-architecture/decisions/0006-determinism-contract.md)).

### How money moves without banks
Every payment is a ledger transaction; per-record amounts are multiplied by the record's weight (`hh_weight`, or 1 for a firm unit, whose amounts are already totals) at posting time ([ADR-0007](../../03-architecture/decisions/0007-money-and-ledger.md)).

| Payment | Legs |
|---|---|
| Household buys from a firm unit | `cash` and `deposits` from households to firms, split in the household's current proportion of the two (largest remainder); VAT part from households to the government through the [VAT](../economy/vat.md) route below |
| Firm unit buys inputs from another (home) | `deposits` firms → firms (inside the sector; the units' columns move, the sector balance does not) |
| Firm unit pays imported inputs; the rest of the world pays exports | `deposits` firms → rest of world, and back |
| Wages | `deposits` firms → households, through the industry's wage clearing account |
| Government pays (pensions, its purchases, its share of investment) | `gov_deposits_at_cb` government → central bank; `reserves` central bank → banks; `deposits` banks → the receiving households or firms |
| Households and firms pay the government (VAT) | the same three legs in reverse |

> **Simplification (M2):** banks hold their opening balances and only pass payments through: no interest, no lending, no reserve requirement, no fees. Firms' profits stay in their deposits ([production](../economy/production.md), M2 simplification).

### Starting the clock
Day 0 is the scenario's start date. At load: every price is 1; every unit's expected demand is its monthly output and its inventory and target ratio are those of [production](../economy/production.md) ("Starting state"); every household's `hh_income_received` is the month's income of its members (wages, pensions, the other incomes of stage I), so that the first decision of each household is the same whatever its stagger day; the CPI history is flat at 1; expected inflation is the target.

## Player levers
None in M2. `step` takes a command list for the shape of ADR-0018; it must be empty until the lever registry exists (M4).

## Tuning parameters
The four mechanics' parameters, at their specs' defaults. This spec adds none.

## Outputs
A `DayReport` per day (date, phases run, checks passed, state hash) and `MonthStats` per closed month: CPI and inflation, expected inflation, employment and unemployment, the wage bill, household consumption, GDP by production, expenditure and income, shortages by industry, households in hardship.

## Interactions
- Reads the scenario; runs the four mechanics' pure functions; posts through `econ-ledger` and the [VAT](../economy/vat.md) mechanic.
- Needs, from the ledger, what its "Not built yet" table lists for M2: the instruments and sectors of the [opening balance sheet](../economy/opening-balance-sheet.md), and the wage and sales clearing accounts with check I-8.

## Edge cases & failure modes
- A check fails → `step` returns a report that says which and stops the run in debug builds ([accounting](../economy/accounting.md), section 4).
- A household with no money and no income, a unit with no staff, an industry with no stock: as in the mechanics' specs.
- February: staggered days run 1 to 28, so every record decides in every month.
- The rounding of weighted postings: every per-record amount is whole bani before it is multiplied by the weight, so sector flows equal the weighted sums exactly.

## Acceptance tests
IDs are stable. Tests in `crates/econ-core/tests/acceptance/` (protected), written in a test-authoring session against the API sketch. The same session writes the owed `[sim]` criteria of the four mechanics on this `World` (AC-HH-03 to 05, 07, 08; AC-LM-03 to 07; AC-PROD-03 to 09; AC-PRICE-04 to 08).

- [ ] **AC-WLD-01** `[sim]` `step` advances one day. Over any 400 days, each phase runs exactly on the days its period is due, in the order of the list; each household and each firm unit runs its staggered phases exactly once in every calendar month, on its own day.
- [ ] **AC-WLD-02** `[sim]` On the Romania scenario at 1:1000, 600 days pass with every check of phase 13 holding on every day: ledger rows sum to zero, clearing accounts net to zero, households' and firm units' columns equal the ledger's sector balances (I-4), no negative cash, deposit or inventory.
- [ ] **AC-WLD-03** `[sim]` The same at 1:100; at 1:10 likewise, marked `#[ignore]` for CI time.
- [ ] **AC-WLD-04** `[sim]` Determinism: two worlds from the same scenario and seed give the same state hash on every day; a world cloned on day 200 and stepped to day 400 gives the same hashes as the original.
- [ ] **AC-WLD-05** `[golden]` The state hash on day 31 on the Romania scenario at 1:1000, seed 42, equals the committed golden value.
- [ ] **AC-WLD-06** `[sim]` At every monthly close GDP by production, by expenditure and by income agree to the ban (I-7).
- [ ] **AC-WLD-07** `[sim]` Quiet baseline (roadmap M2 exit): over 600 days on the Romania scenario at 1:100 with no command, the unemployment rate stays within 3 points of its start, twelve-month inflation (from day 365) within 3 points of the target, and the wage share and the household consumption share of GDP within 5 points of their first month.
- [ ] **AC-WLD-08** `[unit]` A world refuses a scenario whose tables disagree with its opening ledger (I-4 at day 0) with a named error, and builds nothing.
- [ ] **AC-WLD-09** `[unit]` `step` with a non-empty command list is refused in M2 with a named error and changes nothing.
- [ ] **AC-WLD-10** `[bench]` At 1:100 a simulated year takes at most 3 seconds on CI hardware in a release build (Spike 4 and spike 10 measured about 0.2 s for the scale world). *(Restated 2026-11-01, owner's delegation: 2 seconds before; the World with every M3 and M4 mechanic takes about 1.5 s on a developer machine and 2.15 to 2.3 s on slow CI runners, which failed the budget on PRs that did not touch the run. Performance with every alpha mechanic on is M7's.)*

The tests were written on 2026-10-15 against the API skeleton (`econ_core::world`, `econ_io::read_start_state`), compiled only with the crate feature `world` until the implementation turns it on. They build the Romania scenario with `econ_scenario::build`, write it and read it back with `econ_io::read_start_state`. Additions to the sketch, for the tests: read-only column tables (`households()`, `persons()`, `firm_units()`, `industries()`, with the per-record columns the criteria observe), `WorldParams::spec_defaults`, `World::inflation_target_ppm`, and `StartEdit`: edits of a starting state, applied at load, by which the mechanics' criteria that need a change M2 has no lever for compare two worlds (a one-off transfer, an industry with nothing to sell, scaled inventories, labour requirement or wages, other basket shares). AC-WLD-10 runs in release builds only (`cargo test --release -p econ-core --features world --test acceptance ac_wld_10`); debug builds skip it.

## API sketch
In `econ-core` (no I/O); loading the scenario's files is `econ-io`'s.

```rust
pub struct StartState { /* population and stage columns, firm-unit table, opening balances,
                           io coefficients, basket shares, placeholder final demand */ }
pub struct Law { pub vat: VatSchedule, pub vat_category: Vec<VatCategory>, pub minimum_wage: Bani }
impl Law { pub fn placeholder_2021() -> Law; }
pub struct WorldParams { pub households: HouseholdParams, pub labour: LabourParams,
                         pub production: ProductionParams, pub prices: PriceParams, pub rng_seed: u64 }

pub struct World { /* calendar, today, column tables, ledger, macro state */ }
impl World {
    pub fn new(start: StartState, law: Law, params: WorldParams) -> Result<World, WorldError>;   // AC-WLD-08
    pub fn step(&mut self, commands: &[Command]) -> Result<DayReport, WorldError>;            // AC-WLD-09
    pub fn date(&self) -> Date;
    pub fn day(&self) -> Day;
    pub fn state_hash(&self) -> u64;
    pub fn ledger(&self) -> &Ledger;
    pub fn months(&self) -> &[MonthStats];
    // read-only columns for tests and the UI: households(), persons(), firm_units(), industries()
}
impl Clone for World {}                                                                      // snapshot, AC-WLD-04

pub enum PhaseId { Commands, Plan, Staffing, Prices, OutputAndInputs, FixedDemand, Households,
                   GoodsMarket, Quits, Pay, WageRound, MonthlyClose, Checks }
pub struct DayReport { pub day: Day, pub date: Date, pub phases: Vec<PhaseId>, pub checks_ok: bool,
                       pub failed: Vec<InvariantError>, pub state_hash: u64 }
pub struct MonthStats { pub month: Month, pub cpi_ppm: u32, pub inflation_ppm: i32, pub expected_inflation_ppm: i32,
                        pub employed: u64, pub unemployed: u64, pub wage_bill: Bani, pub consumption: Bani,
                        pub gdp_production: Bani, pub gdp_expenditure: Bani, pub gdp_income: Bani,
                        pub shortages: Vec<i64>, pub hardship_households: u64 }
pub enum WorldError { TablesDisagreeWithLedger { what: &'static str }, CommandsNotYet, Shape(&'static str) }

// econ-io
pub fn read_start_state(scenario_dir: &Path, io_table: &Path) -> Result<StartState, SaveError>;
```

## Open questions
- [x] **Choices of the first implementation** (2026-10-17), where this spec and the mechanics' left room or the first runs broke without them; listed in the module documentation of `econ_core::world`. **Accepted (owner, 2026-10-20):**
  - `employer` is employment as an employee; employers, own-account and family workers keep their status and earn self-employment income.
  - Normal tightness θ* of the wage round is the tightness of normal turnover (a month of quits over qualified seekers) at load.
  - Households buy each good's imported part from the rest of the world at the starting price, through the seller industry's clearing account.
  - Households pay the legal VAT rates less Romania's VAT gap of 2021 (36.7%) until the informal economy (M15).
  - Fixed investment includes the input-output table's change in inventories.
  - A unit holds 60 days of its essential inputs (at least 1% of its output) and is short of one only when that stock runs out.
  - Staff meets its requirement within half a record; matching reaches the seekers of the regions where a unit has staff (the builder writes `hh_region`).
  - Cohort units pay investment goods, and the domestic ones self-employment and informal income, by their starting surplus, out of what they can spare beyond a month of wages.
  - A public producer's shortfall is paid by the government as non-market output at cost; a cohort unit short of money borrows from the others' spare money (credit inside the firm sector) until banks lend (M3).
  - The government's deposits at the central bank may go below zero until bonds exist (M3).
  - The clock starts just after November's pay day: the starting month's income is paid on day 0.
- [x] **Who pays self-employment and informal income in M2?** The households spec keeps them at their starting amounts (scaled with consumption) but names no payer. Proposed: the domestic units that [firm units' undeclared sales](../economy/firm-units-undeclared-inventories.md) gives the households' income to, each its share, as a `deposits` payment firms → households. Alternative: the firms sector in proportion to value added. **Decided (owner, 2026-10-14):** the domestic units that the firm-unit data gives the households' income to, each its share, as a deposits payment firms → households.
- [x] **The placeholder law in code until M4.** The [scenario builder](scenario-builder.md) (locked) says a loader refuses a scenario with `law: null` "for a run". Proposed reading: the refusal is the game's; the M2 tests construct the `World` directly with `Law::placeholder_2021()`. Confirm, or the builder spec gets a sentence. **Decided (owner, 2026-10-14):** the M2 tests build the `World` with `Law::placeholder_2021()`; the builder's refusal of `law: null` is the game's.
- [x] **Imports and exports in deposits.** In M2 the rest of the world pays and is paid in deposits at Romanian banks (its `deposits` balance moves), not through foreign assets and the exchange rate (M3). Accept as the placeholder? **Decided (owner, 2026-10-14):** accepted as the M2 placeholder.
- [x] **The quiet-baseline bands** of AC-WLD-07 (3 points for unemployment and inflation, 5 for the shares) and the time budget of AC-WLD-10. **Decided (owner, 2026-10-14):** accepted as written.
