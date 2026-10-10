---
id: adr/0017-time-base
title: "ADR-0017: Time base — a daily tick with processes at their own period"
status: accepted
owner: horia
depends_on: [adr/0005-simulation-core-architecture, adr/0006-determinism-contract, adr/0007-money-and-ledger, adr/0009-calibration-and-stability, adr/0011-storage-saves-history, game/overview]
updated: 2026-10-10
---

# ADR-0017: Time base — a daily tick with processes at their own period

## Context
Every spec and spike so far assumed one tick = one month. On 2026-10-10 the owner decided that a month is too coarse to play: **one tick is one day** (roadmap, D5). An hour was considered and dropped: only electricity needs hours, and nobody sets policy by the hour.

A day cannot simply replace the month in the existing rules:
- Most economic events have no daily meaning. Wages, pensions and taxes are paid monthly; budgets and school years are yearly; almost all calibration data is monthly or quarterly.
- 50 years are 18,262 days against 600 months. Spike 4 measured about 10 ms for a full monthly tick at 1:100 on the owner's laptop; the same work every day would make a 50-year run take three minutes instead of six seconds, and a calibration campaign thirty times longer ([ADR-0009](0009-calibration-and-stability.md)).
- Adjustment speeds (the λ of ADR-0009) and rates are per period. Used unchanged on a shorter period they make every market thirty times faster.
- Random draws are keyed by tick ([ADR-0006](0006-determinism-contract.md)); history is stored monthly ([ADR-0011](0011-storage-saves-history.md)); `econ-types` has a `Month` as "one simulation tick".

The scenario starts on **1 December 2021**, the census reference date (roadmap, D4).

## Options considered
1. **Monthly tick (as before).** Cheapest; all data fits. The player jumps a month at a time and cannot watch or react to anything faster. Rejected by the owner.
2. **Hourly tick.** Finest. 438,000 ticks for 50 years; almost no rule has an hourly meaning; history and saves grow by 730. Rejected.
3. **Daily tick, every process every day.** Simple to state. Thirty times the compute; every monthly rule and parameter must be converted to a daily one, including those that are monthly in reality (pay, tax).
4. **Daily tick, each process at its own period.** The clock advances by the day; a process runs when its period is due. Costs little more than the monthly tick; keeps real monthly and yearly events on their dates. Needs a scheduler and discipline about units.
5. **Weekly tick.** A compromise with no natural data and no natural events; months are not whole weeks.

## Decision
Accepted by the owner on 2026-10-10.

**Chosen: option 4.**

### 1. Clock and calendar
- One tick is one calendar day. `Day(u32)` counts days from the scenario's start date; day 0 is 1 December 2021 in the Romania scenario. The start date is scenario data.
- The calendar is the real (Gregorian) one: months of 28 to 31 days, leap years. Law, lags and data refer to real dates. `Date { year, month, day }` is computed from `Day` by integer arithmetic in `econ-types`; no time zones, no wall clock (ADR-0006 rule 10).
- The day of the week is known.

> **Simplification:** v1 mechanics ignore weekends and public holidays: every day is a working and trading day. Monthly totals are right; the weekly rhythm is absent.

### 2. Periods
Every phase declares one period. The phase list stays one `const` list in one file ([ADR-0005](0005-simulation-core-architecture.md)); on a given day the phases that are due run in list order.

| Period | Runs | For |
|---|---|---|
| `Daily` | every day | prices and quantities of markets held in aggregate: exchange rate, bond yields and interbank rate, goods prices, firm inventories and deliveries, electricity |
| `Staggered` | once a month for each agent, on that agent's own day of the month | decisions of individual households and firm units: budgeting and purchases, job search, hiring and firing, production plans, investment |
| `Monthly(day)` | on a fixed day of each month (the last day if the month is shorter) | the same date for everyone: pay day, tax and contribution due dates, pensions and benefits, loan instalments, demographic transitions with alignment, the monthly statistics close |
| `Quarterly`, `Yearly(date)` | on their dates | national accounts quarters, the budget year, the school year, indexation |

**Staggering** is what makes daily play cheap. Each household and firm unit has a day-of-month offset, drawn once with a keyed draw. On any day about one thirtieth of them take their monthly decisions. The work per month is the same as with a monthly tick, and aggregate flows are smooth from day to day instead of arriving in one lump.

> **Simplification:** a household does its month's spending decision on one day. Its purchases are booked that day. Nationally, spending is smooth; for one household it is a monthly lump.

A spec states the period of each of its rules. First assignment, each spec may refine it:

| Mechanic | Period |
|---|---|
| Exchange rate, bond market, bank rates, policy-rate pass-through | daily |
| Goods prices, inventories, deliveries, imports and exports | daily |
| Electricity dispatch and price | daily, with 24 hourly slots inside the phase (see 5) |
| Household budgets and purchases; VAT on them | staggered |
| Job search, hiring, separations; firm output plans, investment | staggered |
| Wages, income tax, contributions, pensions, benefits, loan service | monthly, on dates from the law in force |
| Births, deaths, ageing, retirement, migration, household changes | monthly |
| Statistics: CPI, unemployment, monthly budget execution | monthly close (last day) |
| GDP and national accounts | quarterly close; also available for any closed month |
| Budget, indexation, school year, yearly taxes | yearly |
| Projects and construction progress | daily (progress is a date, not a counter) |

### 3. Units, rates and adjustment speeds
- Specs, the glossary and data state every flow, rate and speed **per calendar period** (per day, month or year), never "per tick". The glossary's "LCU / tick" entries are restated as each spec is reworded.
- A parameter is converted to the period of the process that uses it once, at scenario load, by one helper in `econ-num` (compound conversion through the pinned `libm`). A monthly adjustment speed λ used daily becomes 1 − (1 − λ)^(1/30); the stability condition of ADR-0009 is checked on the converted value.
- Money stays integer. An amount due monthly is paid on its date in full, not divided by the days.
- Durations in law and levers (lags, build times, benefit duration) are calendar durations or dates. "Next tick" in the lever catalogue becomes "the first day of the following month" for law, and "the next day" for market operations of the central bank.

### 4. Ledger, invariants and statistics
- The ledger's tick is the day ([ADR-0007](0007-money-and-ledger.md)). Postings carry the day. Invariants I-1 to I-8 are checked every day; they hold on any day because every posting is balanced when made.
- The ledger accumulates flows by flow code for the open month, quarter and year, so period statistics are sums, not re-scans.
- Person conservation is checked on the days demographic phases run.
- "Frozen behaviour" reproduction of base-period flows (ADR-0009 step 4) is checked over the first month, not the first tick.

### 5. Electricity inside the day
The electricity phase runs once a day and solves 24 hourly slots from an hourly demand profile and hourly availability (solar, wind). It posts the day's totals and records hourly prices as a 24-value series of that day. There is no hourly clock and no state that lives between hours except storage inside the phase.

### 6. The player
- Commands carry an effective date. A command given today is applied at the start of the next day, or on its effective date if later.
- Speeds: step one day, one week, one month; run continuously at a chosen number of days per second; pause. `World::step` advances one day; `run_until(date)` is the loop.
- Changing levers every day is possible. Whether that has a cost is roadmap decision D9 (none in v1).

### 7. Determinism (amends ADR-0006)
- The `tick` in every keyed draw is the day number. `WORDS_PER_TICK` and the stream layout do not change: 100 years are 36,525 ticks, far below the range of `u32` and of the ChaCha8 word position.
- Staggered and monthly phases key their draws with the day they run on.
- Existing goldens are indexed by tick number and do not change: the SIM golden (200 ticks), the scale-world goldens (12 ticks) and the population stages (which use ticks 0 to 4 of their own stream as stage numbers).
- The state hash is taken every day.

### 8. History and saves (amends ADR-0011)
- A save records the date and the day number.
- Macro series: **daily for the current and the previous two years** for the fast series (exchange rate, interest rates and yields, price indices of goods, electricity price, reserves); **monthly forever** for every series, as before. The aggregation cube stays monthly for ten years, annual after that.
- Closed history blocks stay yearly files.
- The save-corpus test steps each golden save 31 days instead of 12 ticks.

### 9. Calibration and performance (amends ADR-0009 in numbers only)
- Targets compare model output aggregated to the period of the data (month, quarter, year).
- Budget to confirm by a spike before mechanics are written (roadmap M2): at 1:100 a day with only daily phases and one thirtieth of the staggered work should cost about 1 ms, a month-end day about what a monthly tick costs today. Target: one simulated year in about half a second, 50 years in about 30 seconds, as in Spike 4. If the spike misses this by much, daily phases are moved to aggregates or the staggered share is cut before more code depends on it.
- Runs per hour for calibration are re-measured with the daily clock.
- **Measured 2026-10-10** ([spike 10](../spikes/spike-10-daily-clock.md)): on the scale world at 1:100 an ordinary day costs 0.5 ms, a month-end day 2.9 ms and a year 0.21 s, which is 1.6 times a year of monthly ticks. The budget holds for that workload. The spike also showed that a staggered rule which selects whole persons from a rate must carry its remainder from day to day.

### 10. Types and names
- `econ-types` gains `Day(u32)` and `Date`; `Month` stays, meaning a calendar month (the index of a monthly statistic), no longer "one tick".
- New glossary entries (`day`, `date`, the period of a process, the stagger offset of a household and of a firm unit) are added with the code that introduces them, through `just codegen`.

## Consequences
- Easier: the player watches markets move day by day and sees law take effect on dates; pay days, tax days and budget years are real events; fast crises (a run, a currency slide) have room to unfold when they are modelled.
- Easier: aggregate flows are smooth without any smoothing rule, because agents are staggered.
- Harder: every spec written "per tick" is reworded when it is next touched or locked; until then "tick" in a spec means that process's period ([game overview](../../02-design/game/README.md)).
- Harder: two kinds of bug become possible: a rate used at the wrong period, and a process that reads a monthly statistic before its close. Both get lints or tests: the conversion helper is the only way to change period, and monthly statistics carry the month they belong to.
- Harder: daily phases must not loop over every person. Anything per person is staggered or monthly.
- Revisit if the M2 spike shows a simulated year far above the budget, or if staggering produces artefacts (for example a wage rise reaching spending over thirty days when it should be immediate).

## Open questions
Settled as proposed when the ADR was accepted (2026-10-10):
- [x] Weekends and holidays are ignored in v1.
- [x] Daily history is kept for the current and the two previous years; monthly forever.
- [x] Tax and benefit dates come from the law in force; pay day is the last day of the month for everyone.
- [x] Household spending is staggered: one decision a month per household, on its own day.
