---
id: game/calibration
title: Calibration Targets and the Baseline Report
status: draft
owner: horia
depends_on: [adr/0009-calibration-and-stability, adr/0006-determinism-contract, adr/0010-verification-and-testing, game/statistics-registry, game/world-loop, game/policy-scenarios]
research: []
updated: 2026-11-01
---

# Calibration Targets and the Baseline Report

## Purpose
Roadmap M7, first step (owner, 2026-11-01: "measure first"). [ADR-0009](../../03-architecture/decisions/0009-calibration-and-stability.md) calls for **targets as code**: a file of `{id, metric, band, source}` with checker functions, and quiet runs whose key ratios must stay in their bands. This spec defines that file, the quiet baseline runs, and a report that says, for each target, where the simulation is and whether it is in its band. Every later M7 change (household income, investment, the fiscal and external paths, the M5 features on) is judged by the same report, so a fix in one place that breaks another shows up at once.

The M7 exit, "a calibration report the owner accepts", is this report with every target in its band.

## The quiet baseline
The Romania scenario under the law of its folder (`law.json`, `world.json`: the 2021 law), seed 42, no command, from 1 December 2021. Two configurations:
- **core**: econ-core's default features (the population does not change);
- **people**: with `home_quality`, `education` and `approval` (every M5 mechanic: ageing, births, deaths, the life cycle, migration, education, approval, poverty, wellbeing).

A configuration is a build (the M5 mechanics are compile-time features): econ-cli with the feature `calibration` is the core configuration, with `calibration-people` the people one; `just calibrate` runs both.

Two lengths: **10 years** (the levels and the first decade's paths) and **50 years** (stability). The calibration scale is 1:1000; the report also runs 1:100 when asked, to compare (ADR-0010's three-scale check comes with the goldens at M7's end).

**The routine government** (owner, 2026-11-01). The law's money amounts stay in lei: when the player does nothing, inflation erodes them (the 2021 law alone shrinks the state by about 2% a year in real terms, and with it the economy). A real government re-votes them each year, so the quiet baseline does what a routine budget does and nothing more. On the first day of each November after the first year, the report gives ordinary orders, effective 1 January, raising every amount in force of the ministries' budgets (`spend.ministry_budget`), public pay (`spend.public_pay`), the pension point and minimum pension, the unemployment benefit's ISR, the child allowances, the minimum income and the minimum wage by the latest close's twelve-month inflation (never lowering them). Tax thresholds and brackets are not indexed (Romania does not), excise follows its own law, and the budget lines that are not levers yet (the treasury's fixed transfers) stay in lei. In the game, the Ministry of Finance's adviser will propose this budget each year, and automation may apply it for a player who chooses so ([roadmap](../../roadmap.md), after v1); nothing applies it by default.

A run that stops (a failed day check or a panic) is a result, not an error of the tool: the report records the month it stopped and fails every target whose window it did not reach.

### Where the baseline stands (2026-11-01, core, 1:1000)
Measured before this spec, to ground the bands. The run panics in month 241 (an overflow in the fiscal mechanic) after a debt spiral.

| Year | Unemployment | 12-month inflation | Lei per euro | Debt / GDP | Current account, a month |
|---|---|---|---|---|---|
| 0 | 8.5% | — | 4.94 | 39% | +2.1 bn lei |
| 2 | 12.0% | 0.6% | 4.30 | 50% | −6.9 bn |
| 5 | 13.1% | 1.0% | 4.46 | 69% | −3.4 bn |
| 10 | 15.5% | 0.8% | 4.26 | 100% | −3.5 bn |
| 15 | 17.4% | 1.0% | 4.11 | 147% | −7.2 bn |
| 19.5 | 17.8% | 2.1% | 4.38 | 482% | −223 bn |

Real GDP falls by about a fifth in the first six months (a start transient) and drifts down about 0.5% a year after. Inflation and the exchange rate stay calm; unemployment rises, and the deficit and the current-account deficit grow with it.

## The targets file
`calibration/targets.toml`, one entry per target:

```toml
[[target]]
id = "L-01"                       # stable; L = level, S = stability, R = lever response
indicator = "labour.unemployment_rate"   # a registry name (game/statistics-registry)
key = "national"                  # or "region:3", …
metric = "mean"                   # see Metrics
window = [12, 35]                 # months from 0 (December 2021), both included
band = [45_000, 75_000]           # in the indicator's unit (here ppm)
config = "core"                   # core, people or both
source = "Eurostat une_rt_a, Romania 2021: 5.6%"
note = "the 2021 law's equilibrium; 2021 itself was 5.6%"
```

### Metrics
Each metric reads the registry's monthly points of the indicator at the key over the window, and gives one number, compared with the band (both ends included):
- `mean`: the mean of the points;
- `at`: the point of the window's last month;
- `min`, `max`: the smallest and largest point;
- `drift`: the largest absolute difference between a point and the window's first point, in the indicator's unit (ADR-0009's "within ±10% of its base value" is a `drift` band of ±10% of the base);
- `trend`: the least-squares slope of the points, per year;
- `ratio(other)`: the mean of the indicator over the mean of `other` over the window, in millionths (for shares of GDP from two registry series);
- `reached`: 1 if the run has a point at the window's last month, else 0 (band `[1, 1]`: the run must not stop);
- `explained(a, b, …)`: for S-05a, the change of the indicator over the window (its last point less its first) over the sum of the named flow series' points over the window, in millionths (band `[750_000, 1_250_000]`); a zero sum has no value and the target is out.

A target the tables state for several windows ("each decade") or metrics (L-03's min and max) is one target per part, its id the table's with a suffix (`S-03-d1` … `S-03-d5`, `L-03-min`). Bands are absolute: a band "within x% of the start" is written from the scenario's starting value (S-04), and "within 10% of year 2's value" is read as within 10% of the level target's value (S-07).

Units and the evaluation use integers where the registry does (`i64` points); `mean`, `trend` and `ratio` are computed in `f64` and rounded half away from zero to the indicator's unit.

## The proposed targets
The bands are the owner's to set (roadmap M7). Proposed, with their sources; "years" count from 1 December 2021, so year 1 is months 0 to 11.

**Levels (core, years 2 to 3, months 12 to 35, unless stated):**

| Id | Indicator | Metric | Band | Source and reasoning |
|---|---|---|---|---|
| L-01 | `labour.unemployment_rate` | mean | 4.5% to 7.5% | Eurostat `une_rt_a` 2021: 5.6% |
| L-02 | `prices.inflation_yoy` | mean | 1.5% to 5.0% | BNR target 2.5% ± 1 pt; a quiet run has no 2022 energy shock, so 2022's 13.8% is not the target |
| L-03 | `fx.eur_lei` | min, max | 4.5 to 5.5 | BNR 2021 average 4.92, end of year 4.95 |
| L-04 | `money.policy_rate` | mean | 1.0% to 6.0% | 1.75% at the start; the rule's neutral rate with 2.5% inflation |
| L-05 | `gdp.growth_yoy` | mean (years 2 to 10) | 1.0% to 4.5% | potential growth estimates for Romania 2022–2030 (European Commission, about 3 to 4%); below while productivity growth is 0 |
| L-06 | `budget.deficit` over `gdp.nominal` | ratio | 3% to 8% | 2021: 7.1% (ESA), the 2021 law's deficit before consolidation |
| L-07 | `trade.current_account` over `gdp.nominal` | ratio | −8% to −2% | BNR 2021: −7.2%; 2015–2019: −0.8% to −4.9% |
| L-08 | `debt.gdp_ratio` | at month 0 | 45% to 53% | Eurostat 2021: 48.9% (the opening book against month 0's annualised GDP; 39% today: a check of the start) |
| L-09 | household consumption over `gdp.nominal` | ratio | 58% to 66% | INS 2021: about 62% (final consumption of households) |
| L-10 | wage bill over `gdp.nominal` | ratio | 32% to 42% | INS 2021: compensation of employees about 37% of GDP |

L-09 and L-10 need the registry to carry household consumption and the wage bill as series (`gdp.consumption`, `wages.bill`); they are added as registry entries with this spec (both are already in `MonthStats`; glossary names `consumption`, and a new `wage_bill`: wages paid by all employers over the month).

**Stability (core and people):**

| Id | Indicator | Metric | Window | Band |
|---|---|---|---|---|
| S-01 | any | reached | 50 years | the run completes (no failed check, no panic) |
| S-02 | `labour.unemployment_rate` | drift | years 2 to 50 | within ±3 points of year 2's first month |
| S-03 | `prices.inflation_yoy` | mean of each decade | years 2 to 50 | 0% to 6% |
| S-04 | `fx.eur_lei` | drift | years 2 to 50 | within ±25% of the start's 4.94 (1.235 lei) |
| S-05a | `debt.stock` | `explained(budget.deficit)`, each decade | years 1 to 50 | the decade's change in the debt within ±25% of the decade's deficits: debt may rise under the 2021 law, but only as the budget says (revaluations of the foreign-currency debt fall inside the ±25%) |
| S-05b | `debt.gdp_ratio` | reached | years 1 to 30 | no debt crisis before year 30 unless the bond risk premium ([fiscal policy](../economy/fiscal-policy.md)) set it off |
| S-06 | `trade.current_account` over `gdp.nominal` | ratio, each decade | years 1 to 50 | −8% to +2% |
| S-07 | `gdp.consumption` and `wages.bill` over `gdp.nominal` | ratio, each decade | years 2 to 50 | within ±10% of L-09's 62% and L-10's 37% (ADR-0009) |
| S-08 | `people.population` (people) | at year 50 | — | 14.5 to 17.5 million: the direction of INS's and Eurostat's projections (EUROPOP2023 baseline for 2070 about 15.6 million); M5's exit |

The 50-year runs keep the 2021 law and **no fiscal rule**: the player is the government, so nothing in the game corrects the budget for them (owner, 2026-11-01). The 2021 law is not sustainable, and that is a correct result; the stability targets ask that the economy's own dynamics stay sound while the debt climbs as the deficits say. A scripted consolidation run (ordinary lever orders trimming the deficit, to check stability under a governed budget) is not built now; revisited later if the targets need it (owner, 2026-11-01).

**Lever responses** (R) come after the levels: ADR-0009's R-1 (after a 1 point policy-rate rise the CPI's trough lies 8 to 24 months later and between −0.2% and −1.5%) and one per M4 scenario, each anchored in evidence, reusing the [policy-scenario](policy-scenarios.md) harness. Their bands are proposed when the baseline's levels are in band.

## The report
`econ-cli calibrate --targets calibration/targets.toml --sample-scale 1000 --years 10|50 --config core|people --out DIR [--fixtures DIR] [--io-table FILE]` (feature `calibration` of econ-cli; `calibration-people` for people):
- builds the Romania scenario from the fixtures at 1:N, seed 42, under the law of its folder;
- runs the configuration's quiet baseline, recording the statistics registry, and stops at the first failed check or panic;
- evaluates every target of the configuration whose window ends within the length;
- writes `DIR/report.json` (every target: id, value, band, in band, how far out in band widths) and `DIR/report.md` (a table of the targets, failing ones first; then one line a year of the headline indicators; then where the run stopped, if it did);
- writes the registry's history of each run as history files ([history files](history-files.md)) under `DIR/<config>/history/`, for plots;
- exits 0 if every target is in its band, 1 if any is out, 2 for bad arguments or a targets file it cannot read.

The evaluation is a pure function of the history and the targets (`econ_core::calibration`, feature `calibration`), so tests check it on built histories without running a world. Same seed, same report: no wall-clock time in the report except a separate `timing` field of `report.json`.

CI runs the 10-year core report at 1:1000 on every PR that touches code and writes `report.md` to the job's summary; it does not fail the PR until M7 closes, then every target is required.

## Acceptance tests
IDs are stable. Tests in `crates/econ-core/tests/acceptance/calibration.rs` and `crates/econ-cli/tests/acceptance/calibrate.rs` (protected; the test-writer's).

- [ ] **AC-CAL-01** `[unit]` The committed `calibration/targets.toml` parses; every target names a registry indicator that is active in its configuration's features, a key the indicator has (or national), a known metric and a band with low ≤ high; ids are unique.
- [ ] **AC-CAL-02** `[unit]` Each metric on hand-built histories: `mean`, `at`, `min`, `max`, `drift`, `trend`, `ratio` and `explained` give the stated values (including negative values, a one-point window and rounding half away from zero); a window past the history's end makes the target fail with "not reached", never panics.
- [ ] **AC-CAL-03** `[unit]` A target exactly on either end of its band is in band; one ban (or ppm) outside is out; "how far out" is the distance to the nearer end over the band's width.
- [ ] **AC-CAL-04** `[sim]` `econ-cli calibrate --years 3 --config core` at 1:1000 writes `report.json` and `report.md`; their values equal the evaluation of the registry history of a direct run of the same baseline; two runs give byte-identical `report.md` and equal `report.json` apart from `timing`.
- [ ] **AC-CAL-05** `[sim]` Exit codes: a targets file whose bands all contain the 3-year values exits 0; moving one band off exits 1 and lists that target first in `report.md`; an unreadable targets file exits 2.
- [ ] **AC-CAL-06** `[sim]` A run that stops is reported: with a check made to fail on a chosen day (a test edit of the start), the report names the month it stopped, marks S-01 out and every target whose window it did not reach "not reached", and still exits 1 with both files written.
- [ ] **AC-CAL-07** `[sim]` `gdp.consumption` and `wages.bill` are in the registry and equal `months()[t].consumption` and `months()[t].wage_bill` at every close.
- [ ] **AC-CAL-08** `[bench]` The 10-year core report at 1:1000 takes at most 60 seconds in a release build (today's run: about 7 seconds a decade).

The M7 exit criterion is not an acceptance test of this spec: it is the report with every target in band, which the owner accepts.

## Implementation status
- **2026-11-01:** built. `econ_core::calibration` (feature `calibration`), `calibration/targets.toml` (41 targets: the tables' with their parts), the registry's `gdp.consumption` and `wages.bill`, the start edit `StartEdit::FailChecksOn` (AC-CAL-06), `econ-cli calibrate` (features `calibration`, `calibration-people`), and `just calibrate`; the CI step is a patch the owner applies (the agents' token cannot change workflows). Readings: `trend` is the least-squares slope against the month times 12, computed exactly in integers; `mean`, `ratio` and `explained` too (no floats in a value); a run that stops fails `all_in_band` even when every target evaluated is in band. AC-CAL-01 to 08 pass; the 10-year core report takes about 7 s in release.
- **First report (core, 1:1000):** 10 years: 11 of 16 targets in band; out: unemployment (10.9%, L-01), inflation (0.8%, L-02), the leu's low (4.13, L-03-min), growth (−0.6% a year, L-05), the opening debt (38.5%, L-08). 50 years: the run panics in month 246 (an amount past `i64` bani); in the first two decades the debt rises as the deficits say (S-05a 97% and 100%), and the current account falls to −46% of GDP in the second (S-06-d2). People, 10 years: the five stability targets of the first decade in band; unemployment 18% and population 17.9 million in year 9.

- **With the government's fixed purchases from industries with no public unit (2026-11-01, [lever system](lever-system.md), "What it replaces"):** core, 10 years: 14 of 16 in band; unemployment 7.4% (L-01 in band), the deficit 6.7%; out: growth −0.1% a year (L-05) and the opening debt (L-08). Unemployment still rises from 6% in year 1 to about 10% in year 9.
- **With the routine government (2026-11-01):** core, 10 years: 13 of 16 in band; unemployment 9.6% (L-01), growth −0.2% a year (L-05, was −0.7%), year 9's unemployment 12.9% (was 14.5%).
- **After M7 step 2 (2026-11-01; the policy rule's monthly smoothing and the leu's γ₁, [roadmap](../../roadmap.md)):** core, 10 years: 13 of 16 in band; out: unemployment 9.7% (L-01), growth −0.7% a year (L-05), the opening debt (L-08). Inflation 2.7%, the leu between 4.92 and 5.07, the policy rate 3.2%, the deficit 3.4%, the current account −5.3%. Unemployment rises from about 8% in year 1 to 14% in year 9.

## Open questions
- [x] **Debt in a quiet baseline (owner).** *Decided 2026-11-01 (owner):* no automatic fiscal rule in the game, now or as a default, since it would play the Ministry of Finance for the player. The quiet baselines keep the 2021 law with S-05a and S-05b; a scripted consolidation run waits until needed. What answers a rising debt in the game: the bond risk premium and the debt crisis ([fiscal policy](../economy/fiscal-policy.md)), the automatic stabilisers, the EU's deficit rules (M18). Later, and never by default: a Ministry of Finance **adviser** that proposes a budget (as the central bank's rule is shown once the player overrides it), **politics** that resists or forces budget changes, and **automation** the player opts into per area, as in Europa Universalis ([roadmap](../../roadmap.md), after v1).
- [x] **The bands (owner).** *Decided 2026-11-01 (owner): accepted as proposed*, as a starting point to tighten once the baseline is in band.
- [ ] **The start transient.** Real GDP falls by about a fifth in the first six months; the levels skip year 1 for that reason. Whether the transient itself becomes a target (month 0 to 6 drift of GDP within ±5%) is proposed for after the household income fix, which is its likely cause.
