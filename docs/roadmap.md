---
id: roadmap
title: Roadmap
status: draft
owner: horia
depends_on: [vision/scope]
updated: 2026-10-10
---

# Roadmap

## Phase 0 — Design & research *(now)*
- [x] v1 game definition: vision, scope, game, society and economy specs at `draft` (2026-10-08).
- [ ] Owner review pass: answer open questions in each spec.
- [ ] Finish P0 research (modelling approach, competitor teardown, tick length). Population and tech-stack research done ([population](01-research/notes/population-modelling-deep-research.md), [tech stack](01-research/notes/tech-stack-deep-research.md)).
- [x] ADR: modelling approach (ADR-0002, hybrid SFC + synthetic population + I-O industries; accepted 2026-10-09).
- [x] ADR: tech stack drafted as ADR-0004 to ADR-0015, status `proposed` (2026-10-09). See the [architecture overview](03-architecture/README.md).
- [x] Owner accepts (or amends) ADR-0004 to ADR-0015 (accepted 2026-10-09).
- [ ] Data plan: Romania calibration pack and synthetic population pipeline from the IPUMS 2011 seed reweighted to 2021 census totals ([ADR-0012](03-architecture/decisions/0012-data-pipeline-and-licensing.md)).
- [x] v1 additions specified: EU membership and funds, informal economy, foreign ownership, housing, environment, state capacity (2026-10-08).
- [x] ADR: people representation (ADR-0003, accepted 2026-10-09; 1:100 default, scale configurable and never hardcoded).
- [x] **Firm table:** [ADR-0016](03-architecture/decisions/0016-firm-representation.md) (accepted 2026-10-09) replaces "one aggregate firm per industry" with firm units: 0–5 named firms plus size-class cohort firms per industry, with one ownership model. Specs updated (production, industries, investment-capital, money-banking, foreign-ownership, state-enterprises, accounting).
- [ ] Move accounting, population-groups, production and households to `review`.

### Legal and admin track
This is a personal, non-commercial project, so data-terms checks are not needed ([ADR-0012](03-architecture/decisions/0012-data-pipeline-and-licensing.md)).
- [ ] Optional: check BeforeIT.jl licence before copying any code (reading papers is fine).
- [x] GitHub Free with hooks + CI checks ([ADR-0014](03-architecture/decisions/0014-agent-workflow-guardrails.md)).

## Phase 1 — Technical spikes, then the headless simulation prototype
Spikes are throwaway-quality prototypes that settle the decisions hardest to change later. Spikes 1–3 come first; 4–6 can overlap; the UI comes late because the core's API defines what it shows. Starting Phase 1 needs the owner's go-ahead (AGENTS.md forbids game code in Phase 0).

| # | Spike | Exit criterion | Settles |
|---|---|---|---|
| 0 ✅ (`just check` green on the Windows laptop and hook blocks edits, 2026-10-10) | Repo hygiene: `@AGENTS.md` import, justfile, toolchain pin, empty workspace, cargo-deny, CI skeleton, PreToolUse hook | An agent on Windows runs `just check` green and is blocked from editing `tests/golden/` | ADR-0014, 0010 |
| 1 ✅ (Windows laptop, Windows CI and Linux CI hashes all match, 2026-10-10) | Numeric foundation: `Bani`, rounding, splits, keyed ChaCha8 + known-answer vectors, `det_sum`, `libm` wrappers, clippy deny config | Same hashes on Windows laptop and Linux CI; clippy rejects `f64::exp` and `HashMap` in core | ADR-0006, 0007 |
| 2 ✅ (PC model pending) | Ledger + differential SIM: typed transfers, invariants I-1 to I-7, core "SIM mode", independent Python SIM then PC | Exact match in bani for 200 ticks; a mutated posting sign is caught by the differential test | ADR-0007, 0010 |
| 3 ✅ | Explainability: `behaviour_rule!` on price, wage and consumption rules; `explain()` tree; Σ = Δ and μ tests | A nested tree sums exactly; log-linear aggregation question settled | ADR-0008, 0009 (part) |
| 4 ✅ 6.4 ms/tick at 1:100 (9.9 ms on the Windows laptop with VAT wired in, 2026-10-10) | Performance and market clearing at variable scale: fake weighted population, labour and goods matching, 80×80 LU, cube | Tick time at 1:1000, 1:100, 1:10; 50-year run ≤ ~30 s at 1:100; 3-scale test passes | ADR-0003, 0005, 0011 |
| 5 ✅ ~36k runs/h at 1:1000 (~27k on the Windows laptop with VAT wired in) | Calibration loop: `econ-py`, 600-tick runs from Python, Morris on ~10 parameters, one history-matching wave, 100-year quiet baseline | Runs per hour measured; full calibration budget known | ADR-0009 |
| 6 ✅ 1.7 MB at 1:100 | Saves and replay: ZIP + manifest + Arrow tables + command log, one migration, history block | Load, replay and state-hash check pass; save size at 1:100 and 1:10 recorded | ADR-0011 |
| 7 ✅ offline slice; live Eurostat fetch verified 2026-10-10 | Data pipeline slice: glossary codegen, Eurostat fetch with provenance tags, DVC with a private remote, population for 2–3 counties from the IPUMS seed via IPF | Scenario builds end to end; provenance listed in the report; no raw restricted microdata tracked by Git | ADR-0012, 0003 |
| 8 ✅ 5 MB IPC round trip 33 ms, cold start 1.4 s, ~310 MB (see "Windows verification" in the [spikes 5–9 results](03-architecture/spikes/spikes-5-6-results.md)) | UI slice: Tauri 2 + React, 40 uPlot charts × 600 months, ECharts county map from geoBoundaries, one nested "why" tooltip | 1–5 MB IPC round trip measured on WebView2; cold start and memory acceptable | ADR-0013 |
| 9 ✅ single-session (two-PR flow needs GitHub) | Agent workflow dry run: one real mechanic (e.g. VAT) through the tests-first two-PR flow with traceability and `cargo-mutants --in-diff` | Spec → tests → implementation merged with no hand edits to protected paths; friction logged | ADR-0010, 0014 |

Results: [spikes 0–4](03-architecture/spikes/spikes-0-4-results.md), [spikes 5–6](03-architecture/spikes/spikes-5-6-results.md).

## Phase 1b — headless prototype
The spikes are done; this phase turns them into a simulation core that runs from a scenario with no UI. The open items below are ordered, with everything after them, in "Path to v1" (M2 to M4).

- [x] Follow-ups from the Windows verification (2026-10-10): fast RNG switched to the "gamma" mixer ([ADR-0006](03-architecture/decisions/0006-determinism-contract.md) Amendment 1), `tax.vat` wired into the scale world through the ledger, Tauri host checked in Windows CI.
- [ ] Simulation core runs from a scenario, no UI; outputs Arrow/CSV and plots.
- [ ] Automated acceptance tests from each spec (tests-first two-PR flow).
- [ ] SFC invariants I-1 to I-7 every tick ([ADR-0007](03-architecture/decisions/0007-money-and-ledger.md)).
- [ ] Person-conservation check every tick.
- [ ] CI runs the tiny scenario at three population scales.
- [ ] Policy-scenario test suite (rate hike, tax cut, deficit spending, devaluation, oil shock, student-grant rise, power-plant build).

### Owner decisions for Phase 1b (2026-10-10)
Recorded here so they are not lost; none of this is implemented yet.

**Spec lock order.** Only the owner moves a spec to `locked`. Every spec gets an "API sketch" section before it locks (Spike 9 lesson: tests-first means the test writer picks the API).
1. [`society/population-groups`](02-design/society/population-groups.md) and [`economy/accounting`](02-design/economy/accounting.md).
2. VAT: split it out of [`economy/taxation`](02-design/economy/taxation.md) into its own spec file and lock that. Split done 2026-10-10: [`economy/vat`](02-design/economy/vat.md), ready for the owner's review.
3. [`economy/households`](02-design/economy/households.md) and [`economy/labor-market`](02-design/economy/labor-market.md), then the rest of taxation (income tax, social contributions).
4. Firms and [production](02-design/economy/production.md), after the firm-unit goods market exists.

**Data** (keeps [ADR-0012](03-architecture/decisions/0012-data-pipeline-and-licensing.md)).
- The IPUMS 2011 seed sample lives in the private DVC store, never in git.
- Census 2021 tables (INS) and EU-SILC aggregates are the IPF margins.
- The owner applies for IPUMS International access and writes to INS in parallel. INS microdata is optional.

**First real mechanic: the population generator from census marginals**, built through the tests-first two-PR flow. Its acceptance criteria:
- margins within a set tolerance at 1:1000, 1:100 and 1:10;
- total weight independent of the scale;
- golden hashes;
- a Python reference with a differential test.

**Order of work.**
1. The follow-ups PR above (RNG, VAT in the scale world, CI).
2. The population generator. **Done 2026-10-10** (version 1: county, household size, sex and age): spec [`society/population-generator`](02-design/society/population-generator.md), crate `econ-popgen`, pipeline stage `econ-cli synth-population`. Second increment (education and activity, spec [`society/population-attributes`](02-design/society/population-attributes.md)): **done 2026-10-10** (spec, data, Python reference, acceptance tests, Rust code). Third increment (jobs: status in employment, occupation, industry group; spec [`society/population-jobs`](02-design/society/population-jobs.md)): **done 2026-10-10**. Fourth increment (locality size, urban, tenure; spec [`society/population-housing`](02-design/society/population-housing.md)): **done 2026-10-10**. `econ-cli synth-population` runs the three increments when given their margin files and adds their columns to the tables. **Wired into the simulation 2026-10-10:** `ScaleWorld::from_population` builds the scale world on the generated households, weights, counties and ages (`just sim-romania 100`); education, jobs, wages and deposits are still invented there. The invented population of `ScaleWorld::generate` stays for the benchmarks and its golden.
3. Other follow-ups as they are needed: the firm-unit goods market (before the firms spec locks), the PC model in the differential test, history blocks in saves.

## Path to v1 (written 2026-10-10)
This section replaces the old "Phase 2" list and orders the open items of Phase 1b. It is a plan, not a commitment: the owner reorders or cuts it.

**v1 is everything in [scope](00-vision/scope.md):** the whole economy and society listed there, every lever of the [catalogue](02-design/game/levers.md), every view of [information](02-design/game/information.md), in an app someone else can install and play. Nothing of scope is deferred past v1.

**The plan has three parts.**
1. **Alpha (M0–M8): the core, playable.** A calibrated Romania with all six sectors, the main taxes, spending, transfers and central-bank tools, and a first app. It is a checkpoint, not a release: the earliest point where the game can be played and where the closed money loop is proven stable.
2. **Systems (M9–M20): the rest of scope,** one system at a time on top of a game that already runs.
3. **Finish (M21–M23):** calibration of the whole, the complete app, release.

**How every mechanic is built** (unchanged, ADR-0014): spec to `draft` with acceptance criteria and API sketch, data and Python reference, owner locks the spec, tests in a test-authoring session, then Rust. Criteria that cannot be tested yet are listed in the locked spec as owed (D12). The owner's steps (answer open questions, lock, switch on test authoring) are the pacing item, so milestones list them.

**Milestones at a glance.** Sizes are relative: S is like one population stage, L is many of them.

| # | Milestone | You can then… | Size |
|---|---|---|---|
| | **Part 1 — Alpha** | | |
| M0 | Decisions | know the order and the open design choices | S |
| M1 | Complete starting state | build a full, balanced Romania scenario from data | L |
| M2 | Real economy loop | run households, jobs, firms and prices headless | L |
| M3 | Money, state and the rest of the world | run the closed economy with all six sectors | L |
| M4 | Lever system and alpha levers | change policy from a command file and see effects | L |
| M5 | People over time | run for decades with ageing, births, migration | M |
| M6 | Indicators and explanations | query any statistic, its history and its causes | M |
| M7 | Alpha calibration and stability | trust the baseline and the policy responses | M |
| M8 | Alpha app | play the core | L |
| | **Part 2 — Systems** | | |
| M9 | Firms in depth and state ownership | see real companies; subsidise, nationalise, privatise | L |
| M10 | Bank regulation and backstops | set bank rules; push the economy hard without breaking it | S |
| M11 | Infrastructure and the project system | build roads, schools, hospitals | M |
| M12 | Energy | build and close power plants, steer the mix | L |
| M13 | Housing market | watch prices and rents by county, act on them | M |
| M14 | Society in depth | read approval of any group; fund education and health | L |
| M15 | Informal economy | fight undeclared work and the VAT gap | M |
| M16 | State capacity and corruption | see why public money does not become results | S |
| M17 | Foreign-owned firms and FDI | attract or lose investors | M |
| M18 | EU membership and EU funds | follow or break EU rules, absorb funds, leave Schengen | L |
| M19 | Environment and weather | cut emissions, face droughts and floods | M |
| M20 | Euro | join through ERM II, or leave | M |
| | **Part 3 — Finish** | | |
| M21 | Whole-game calibration and balance | trust all of it together | L |
| M22 | The complete app | play all of it without the docs | L |
| M23 | Release | hand it to someone else | S |

**What every milestone from M9 on must deliver** (so a system is finished when its milestone closes, not "except the UI"):
- its spec locked, with the first version stated;
- its starting data in the scenario, with provenance;
- the mechanic, tests-first, with every rule explainable (`behaviour_rule!`);
- its levers wired, each with a policy test (direction, rough size, timing);
- its indicators in the statistics registry and its headline numbers in the "why" panel;
- its screen or panel in the app;
- the quiet baseline re-run and still stable; goldens updated with a `CHANGELOG-sim.md` entry; a save migration if the state shape changed;
- tick time at 1:100 measured against the budget.

**Built in from M1 so later systems plug in without rework.** Cheap now, expensive later:
- firm units with an ownership vector (state, domestic, foreign) and a controller, even while every unit is a cohort firm;
- county on every household, firm unit and public asset; region on labour markets;
- two foreign blocs (EU, non-EU) in every trade and financial flow;
- a project slot in the tick order and public capital stocks, even if empty;
- goods tagged with VAT category, excise class, energy content and emissions factor;
- money always in the national currency unit with one conversion point, so a change of currency (M20) is an event, not a rewrite;
- a formal-or-informal flag on jobs and sales, all formal at first;
- an outside-world input table (foreign prices, demand, rates, EU carbon price, weather) read by the mechanics: constants with sliders in v1 (D8), a moving world later without touching the mechanics;
- every rule and parameter states its period in calendar time (per day, month, year), never "per tick", and every process is registered with the period it runs at (D5). Law, lags and build times are dates, not tick counts.

A thin UI does not wait for M8. From the end of M2 a minimal window (time controls, a dozen charts) is kept running on the real core, so each later milestone is seen, not only tested.

---

## Part 1 — Alpha: the core, playable

### M0 — Decisions
Mostly owner time. Nothing here is code; all of it unblocks code.
- [ ] **D1 Order.** Confirm the three parts and the order of M9–M20, or reorder.
- [x] **D2 Lever list.** Tagged by milestone in [levers](02-design/game/levers.md) ("When each lever arrives"), 2026-10-10. The owner changes it there.
- [x] **D3 Indicator list.** Tagged by milestone in [information](02-design/game/information.md) ("When each view arrives"), 2026-10-10.
- [x] **D4 Start date: 1 December 2021,** the census reference date (owner, 2026-10-10). No roll-forward of the population. Law, budget and balance sheets are collected as of that date. The years 2022 to 2025 are then real history to compare a run against (M21).
- [x] **D5 Tick length: one tick = one day** (owner, 2026-10-10; replaces the month every spec assumed). An hour was considered and dropped: only electricity needs hours, and nobody sets policy by the hour. Still to do:
  - [x] [ADR-0017](03-architecture/decisions/0017-time-base.md), accepted 2026-10-10: a daily clock on the real calendar; each process at its own period (daily, staggered over the month per household and firm, monthly on fixed dates, yearly); electricity with 24 hourly slots inside its daily step; weekends ignored; daily history for three years, monthly forever.
  - [ ] Reword specs from "per tick" and "next tick" to calendar time as each is locked; the spike numbers (Spike 4: per monthly tick) are re-measured in M2.
- [ ] **D6 Market clearing and expectations:** the two open core decisions in the [economy overview](02-design/economy/README.md) (inventory buffers; anchored-adaptive expectations). ADR or a line in each spec.
- [x] **D7 Scenario and command format:** [ADR-0018](03-architecture/decisions/0018-scenario-and-commands.md), accepted 2026-10-10. A scenario is a folder of tables and small files for one scale, with the law in force as data; a lever registry (`crates/econ-core/levers.toml`) generates the command types, the UI's lever list and the test list.
- [x] **D8 The outside world: not simulated in v1** (owner, 2026-10-10). Foreign prices, fuel prices, foreign demand, foreign interest rates, wages abroad, the EU carbon price and weather are constants that the player can move with sliders ("World settings" in [levers](02-design/game/levers.md)). A world that moves by itself comes after v1. Step 3 of the calibration workflow of ADR-0009 (fitting processes for exogenous series) is postponed with it.
- [x] **D9 Cost of changing a lever: none in v1** (owner, 2026-10-10).
- [x] **D10 Extreme states: not modelled in v1** (owner, 2026-10-10). No dynamics of sovereign default, bank runs or hyperinflation. What stays: the simulation must not crash or break an invariant under any lever setting (tested in M7 and M21), which needs simple backstops (M10).
- [ ] **D11 Language of the app:** English only, or English and Romanian. Cheap if decided before M8, costly after M22.
- [x] **Locked 2026-10-10:** [population-groups](02-design/society/population-groups.md) and [accounting](02-design/economy/accounting.md).
- [x] **D12 Lock rule relaxed** (owner, 2026-10-10; [ADR-0014](03-architecture/decisions/0014-agent-workflow-guardrails.md) Amendment 1): a spec can be locked before its tests; each criterion without a live test is listed in the spec under "Tests owed", and `just trace` checks that list. Ten criteria of the two specs are owed.
- [ ] **[vat](02-design/economy/vat.md):** the spec now has two reduced rates (19%, 9% and 5% on the start date) and is at `review`. Owed: a test-authoring session for AC-VAT-06, then the code change, then the lock.
- [ ] Owner review pass over the open questions of the alpha specs (households, labour market, taxation, production, prices, money-banking, monetary policy, fiscal policy, trade-fx, social transfers, demographics). The other specs are reviewed when their milestone starts.

**Exit:** D1, D6 and D11 answered; VAT tests and code done, then locked. (D2 to D5, D8 to D10 and D12 are recorded; ADR-0017 accepted; two specs locked.)

### M1 — Complete starting state
Everything the simulation needs at tick 0, from data files, consistent with the accounting matrix. Continues the population generator. Data for the systems of Part 2 is collected in their own milestones; here the tables only get the columns listed under "Built in from M1".
- [ ] **Income and wealth of persons and households**, as further generator stages:
  - [ ] **Wages** (stage F, spec [`society/population-wages`](02-design/society/population-wages.md)): spec, data, Python reference and Rust skeleton done 2026-10-10. Levels are the national accounts' wage bill of 2021-Q4 by industry group; proportions by sex, occupation, age and education come from the Structure of Earnings Survey 2022. Open questions accepted by the owner 2026-10-10. Owed: a test-authoring session (to be done together with the other owed tests), then the Rust code.
  - [ ] Other incomes: employers, own-account and family workers (mixed income of the national accounts); pensions and benefits by status.
  - [ ] Wealth: deposits, debts and dwelling value by household. Candidate sources to verify: sector financial accounts for the totals, EU-SILC aggregates for the shape.
- [ ] **Remaining person attributes needed by the core:** split of the employed's industry group into the catalogue industry; split of "other inactive"; enrolment.
- [ ] **Industry catalogue and input-output table:** reconcile the 90 industries of [industries](02-design/economy/industries.md) with the Eurostat supply-use and input-output tables; output, value added, wages, imports and exports per industry and foreign bloc.
- [ ] **Firm table** ([ADR-0016](03-architecture/decisions/0016-firm-representation.md)): cohort firm units by industry and size class from business statistics, with ownership shares per unit. Each employed person gets an employer unit. Named firms come in M9.
- [ ] **Sector balance sheets:** government (debt by instrument and maturity, deposits), banks (loans, deposits, bonds, reserves, equity), central bank (FX reserves, bonds, reserves, cash), rest of world. From the financial accounts; made to sum to zero with the household and firm sides.
- [ ] **Law in force as data:** tax rates and brackets, contribution rates, benefit rules, pension formula, minimum wage, VAT categories per good, on 1 December 2021 (D4).
- [ ] **Budget as data:** spending by ministry line, public employment and wages.
- [ ] **Scenario builder:** one command turns pipeline outputs into a scenario file; a fit and consistency report (every stock has a counterpart; flows of the first month reproduce the national accounts within a tolerance).

**Owner:** accept tolerances per stage as before; test-authoring sessions per stage.
**Exit:** `just build-scenario` produces a Romania scenario at 1:1000, 1:100 and 1:10 whose opening balance sheet passes the invariants and whose totals match the published aggregates within the agreed tolerances.

### M2 — Real economy loop (headless)
The first version of each mechanic is the simplest rule that is in its spec, not the whole spec. Order follows the owner's lock order.
- [ ] **World and tick loop:** the real `World` (replacing the scale-world spike) loads a scenario, runs the phases of the tick order in the [game overview](02-design/game/README.md), checks invariants I-1 to I-7 and person conservation every tick, writes Arrow and CSV. The scale world stays as a benchmark.
- [ ] **Clock and scheduler** as decided in D5 (the calendar types `Day`, `Date` and `Calendar` exist in `econ-types` since 2026-10-10; the cost was measured on the scale world the same day, see [spike 10](03-architecture/spikes/spike-10-daily-clock.md): a year of daily ticks at 1:100 takes 0.21 s, 1.6 times a year of monthly ticks). Still to build for the real `World`: a daily clock, processes run at their declared period, a spike that measures the cost of a simulated year (365 daily ticks, 12 of them with the monthly processes) before the mechanics are written against it. Specs that say "per tick" or "next tick" are reworded to calendar time as each is locked.
- [ ] **Households:** income, consumption by good from baskets, saving.
- [ ] **Labour market:** vacancies, matching by region and skill, wages, job loss.
- [ ] **Firm-unit goods market,** then **production:** output from demand, inputs through the input-output table, inventories, hiring plans.
- [ ] **Prices:** markup on unit cost, CPI.
- [ ] Placeholders so the loop closes: government buys and pays wages at fixed real levels and taxes with VAT only; no banks (deposits only); imports at fixed prices.
- [ ] CI runs a tiny scenario at three scales; a 50-year run at 1:100 stays within the time budget of Spike 4.
- [ ] Thin UI window on this core (time controls, a dozen charts).

**Owner:** lock households, labour market, production, prices.
**Exit:** 600 ticks at three scales with no invariant failure; unemployment, wage share, consumption share and inflation stay in a plausible band with no intervention.

### M3 — Money, state and the rest of the world
Closes the loop: after this, every sector of the [economy overview](02-design/economy/README.md) is live.
- [ ] **Taxation, the rest:** personal income tax with brackets and allowance, social contributions, corporate profit tax, capital income tax, excise, property tax, wealth tax.
- [ ] **Social transfers, core set:** state pension, unemployment benefit, child benefit, minimum income.
- [ ] **Fiscal policy:** budget lines, public wages and headcount, deficit, bond issuance and the bond market, debt service.
- [ ] **Money and banking:** aggregate bank; loans to households and firms, deposit and loan rates, capital and reserve constraints, defaults.
- [ ] **Monetary policy:** policy rate and its pass-through, reserves, QE, monetary financing, inflation target and credibility.
- [ ] **Trade and exchange rate:** exports and imports per good with two foreign blocs, floating rate, managed float and peg, FX intervention, reserves. EU rules reduced to constants until M18 (tariffs fixed, no capital controls).
- [ ] **Investment and capital:** firm investment, depreciation, credit demand, productivity growth. Needed for the policy rate to matter and for long runs to grow.
- [ ] **The outside world** as decided in D8: a table of constants read by the mechanics, each changeable by a world-settings command.
- [ ] PC model added to the differential test (open follow-up of Spike 2).

**Owner:** lock the specs above, one or two at a time.
**Exit:** the full transaction matrix balances every tick; a 50-year quiet baseline is stable; government debt, the current account and bank balance sheets move for reasons that can be read off the ledger.

### M4 — Lever system and alpha levers
- [ ] **Lever system:** every lever and world setting is a typed command with validation, range, effective date and lag; commands are logged and replayable (ADR-0011); a lever registry generated from the specs so the UI and the tests share one list.
- [ ] Wire every lever tagged `alpha` in D2 to its mechanic.
- [ ] **Policy-scenario test suite** (qualitative: direction, rough size, timing): rate hike, income-tax cut, VAT rise, deficit spending, monetary financing, devaluation, minimum-wage rise, pension rise, oil-price shock. One test file per scenario, written tests-first. Every later milestone adds its own scenarios to this suite.
- [ ] Headless "play" from a command file: scenario plus dated lever changes in, time series out.

**Exit:** each scenario test passes at three scales; a mutated rule (wrong sign) makes at least one of them fail.

### M5 — People over time
Without this a long run is a frozen population.
- [ ] **Demographics:** ageing, deaths, births, retirement, household formation and dissolution, moves between counties, emigration and return, the diaspora and remittances, with alignment to official projections.
- [ ] **Education flows:** enrolment, graduation, entry to the labour market (simple version; depth in M14).
- [ ] **Approval, first version:** the per-person number of [opinion-approval](02-design/society/opinion-approval.md), driven by own finances. Display only. Depth in M14.
- [ ] Poverty and inequality indicators from incomes.

**Exit:** a 50-year run reproduces the direction of the official population projection (shrinking, ageing); pension spending responds to it.

### M6 — Indicators and explanations
- [ ] **Statistics registry:** every indicator tagged `alpha` in D3 computed each tick from state and ledger (GDP three ways, CPI and components, employment and unemployment, wages, budget and debt, trade and current account, money and credit, distribution, by county and by group).
- [ ] **Group queries:** any filter over person attributes returns counts, incomes, taxes paid, transfers received, approval.
- [ ] **History** stored in saves (open follow-up of Spike 6).
- [ ] **"Why did this change":** explanation trees (ADR-0008) for the headline indicators; every rule added in M2–M5 already uses `behaviour_rule!`, here they are joined up and tested (parts sum to the change).
- [ ] Core API for the UI: one typed query and command surface.

**Exit:** for each headline indicator, any month's change is decomposed into named causes that sum exactly.

### M7 — Alpha calibration and stability
- [ ] Calibration targets from data: national accounts shares, labour market rates, budget lines, CPI weights, trade, credit.
- [ ] Sensitivity screening and history matching with the loop of Spike 5 ([ADR-0009](03-architecture/decisions/0009-calibration-and-stability.md)).
- [ ] Response checks against published estimates: fiscal multipliers, pass-through of the policy rate and of the exchange rate to inflation.
- [ ] 100-year quiet baseline; extreme-lever runs (tax at 90%, rate at 50%, heavy money printing) end in a bad economy, not in a crash or a NaN.
- [ ] Performance at 1:100 and 1:10 with every alpha mechanic on. The budget must leave room: Part 2 roughly doubles the work per tick.
- [ ] The calibration is scripted end to end, because it is re-run after every milestone of Part 2 and in full in M21.

**Exit:** a calibration report the owner accepts; goldens recorded.

### M8 — Alpha app
On the stack of [ADR-0013](03-architecture/decisions/0013-ui-stack.md), growing the thin UI kept since M2. Built so that Part 2 adds screens without changing the frame.
- [ ] New game from a scenario; save, load and autosave; time controls and speeds.
- [ ] Macro dashboard with pinned indicators; notifications for large moves.
- [ ] Lever panels generated from the lever registry: range, current law, lag, pending changes; tax-bracket editor; budget screen (revenue, spending, deficit, debt).
- [ ] Central bank screen (rate, reserves, QE, monetary financing, FX).
- [ ] Graph browser for every indicator in the registry; group explorer; county map.
- [ ] "Why" panel on every headline number.
- [ ] Text through one string table (D11).

**Exit:** the owner plays 20 game-years of the core in one sitting without touching a terminal. **This is the alpha.** Review the order of Part 2 here with what playing has taught.

---

## Part 2 — Systems: the rest of scope
The order follows what depends on what: firm units before anything owned by a firm (plants, foreign owners, state companies); projects before energy, housing and EU funds; energy and EU before environment; EU, money and trade before the euro.

### M9 — Firms in depth and state ownership
Specs: [production](02-design/economy/production.md), [state-enterprises](02-design/economy/state-enterprises.md), [investment-capital](02-design/economy/investment-capital.md), [fiscal-policy](02-design/economy/fiscal-policy.md) (subsidies), [prices-inflation](02-design/economy/prices-inflation.md) (caps).
- [ ] **Named firms:** the largest real companies per industry as their own units (foreign-owned, domestic, state-owned), from public company data. Legal check on using real names is not needed for a personal project; revisit before any public release (M23).
- [ ] Firm entry, exit and bankruptcy in the cohorts; administration for named firms.
- [ ] Equity, dividends and a simple stock exchange, enough for share sales.
- [ ] State-owned enterprises: objectives (profit, employment, price), directives.
- [ ] Levers: nationalise, privatise, bail out a named firm, directives; all subsidy kinds (production, investment, wage, consumer); price caps with shortages; research grants.
- [ ] Industry and supply-chain views; company pages.

**Exit:** nationalising a firm moves equity through the ledger at the right value; a price cap below cost produces a shortage that the "why" panel names.

### M10 — Bank regulation and backstops
Specs: [money-banking](02-design/economy/money-banking.md), [fiscal-policy](02-design/economy/fiscal-policy.md). Small in v1, because crises are not modelled (D10).
- [ ] Levers: bank capital requirement, deposit insurance coverage, bank bailout (a capital injection for equity).
- [ ] Backstops, so that no lever setting stops the simulation: a bank below its minimum capital stops new lending until profits or a bailout restore it; government bonds that find no buyer at the current yield are placed at a higher yield, up to a cap, then with the central bank (booked as monetary financing and shown as such); a household or firm that cannot pay defaults by the rule of [accounting](02-design/economy/accounting.md), with the loss booked to the creditor.
- [ ] Each backstop raises a notification, so the player sees that a limit was hit.
- [ ] After v1: failure of a bank, runs, sovereign default and restructuring, high-inflation dynamics; and whether there are several banks.

**Exit:** every extreme-lever run of M7 ends with invariants intact and with the backstops that fired listed in the event log.

### M11 — Infrastructure and the project system
Spec: [infrastructure](02-design/economy/infrastructure.md).
- [ ] Projects: cost, build time, county, progress, delays, completion; a queue the player manages.
- [ ] Public capital stocks by type and county (roads, rail, ports, airports, broadband, water, schools, universities, hospitals, police stations, public housing); maintenance and decay.
- [ ] Effects: productivity of firms by county, service capacity for M14.
- [ ] Data: existing stocks by county.
- [ ] Levers: build, maintenance budget. Map layers for stocks and projects.

**Exit:** a motorway raises output in its counties years later, and the path from spending to output is visible in the "why" panel; unfunded maintenance lowers it.

### M12 — Energy
Spec: [energy](02-design/economy/energy.md).
- [ ] Power plants as firm units by technology, with real plants named (Cernavodă, Porțile de Fier and so on); capacity, availability, fuel, merit-order dispatch; wholesale and retail prices; the grid and interconnectors; storage.
- [ ] Fuels in trade; energy in every industry's costs and every household's basket.
- [ ] Data: plant list, capacities, fuel use, prices.
- [ ] Levers: the whole energy block of the catalogue (build, close, phase-out, subsidies, contracts for difference, windfall tax, price caps, consumer subsidies, network tariff, interconnectors).
- [ ] Energy screen: mix, prices, plants on the map.

**Exit:** closing coal without replacement raises prices and imports; a gas price shock passes through to inflation with the published order of magnitude.

### M13 — Housing market
Spec: [housing](02-design/economy/housing.md). Uses tenure and locality size from the generator.
- [ ] Dwellings by county: stock, prices, rents, vacancies; construction responding to prices and permits; mortgages in the bank's book; owner-with-mortgage and social-renter tenures.
- [ ] Housing costs in household budgets; housing wealth in consumption and in the wealth and property taxes.
- [ ] Levers: first-home guarantee, reduced VAT on new homes, rent control, zoning, public housing (through M11), housing benefit.
- [ ] Housing screen and map layer.

**Exit:** a rate cut raises prices and construction with a lag; rent control lowers rents and, over years, supply.

### M14 — Society in depth
Specs: [identity](02-design/society/identity.md), [interest-groups](02-design/society/interest-groups.md), [opinion-approval](02-design/society/opinion-approval.md), [education](02-design/society/education.md), [social-outcomes](02-design/society/social-outcomes.md), [social-transfers](02-design/economy/social-transfers.md).
- [ ] Generator stages: ethnicity, language, religion (census tables; an INS request is planned), ideology, interest-group memberships, field of study.
- [ ] Education in depth: levels and fields, capacity from M11, quality from budget, returns in wages; student finance (grants, loans, repayment).
- [ ] Health and life expectancy, crime, wellbeing: driven by budgets, capacity, income, unemployment; feeding back into mortality, labour supply and approval.
- [ ] Approval in full: by issue and interest group, explained per group. Still no behavioural effect (that is the politics layer, after v1).
- [ ] Remaining transfers and levers: charity drives, non-EU immigration quotas.
- [ ] Group explorer in full; society screens.

**Exit:** for any filter of people the game shows how they live, what they pay and get, how much they approve and why; a rise in the health budget shows up in life expectancy over a decade.

### M15 — Informal economy
Spec: [informal-economy](02-design/economy/informal-economy.md).
- [ ] Undeclared work and envelope wages by industry and firm size; unreported sales and the VAT gap; the choice to evade as a rule of taxes, enforcement and penalties.
- [ ] Data: published estimates of undeclared work and the VAT gap.
- [ ] Levers: tax administration and inspection budget, digitalisation, penalties, amnesty.
- [ ] Official statistics against the true economy: both visible (imperfect information stays after v1).

**Exit:** raising labour taxes moves some work into the shadow and collects less than a no-evasion calculation says; e-invoicing narrows the VAT gap.

### M16 — State capacity and corruption
Spec: [state-capacity](02-design/economy/state-capacity.md).
- [ ] Capacity and corruption as states by ministry or function; the share of each budget line that becomes results; effects on projects (M11), tax collection (M15), EU funds (M18), services (M14).
- [ ] Levers: civil service pay and merit reform, anti-corruption and justice funding, procurement transparency, e-government.

**Exit:** the same hospital budget buys more health after reform; the loss is shown as a line in the "why" panel, not hidden.

### M17 — Foreign-owned firms and FDI
Spec: [foreign-ownership](02-design/economy/foreign-ownership.md).
- [ ] Location decisions of foreign investors (wages, taxes, infrastructure, stability, incentives); pending named investments; relocation; profit repatriation and shifting in the balance of payments.
- [ ] Levers: investment incentives, industrial parks (through M11), negotiating a pending investment.

**Exit:** a corporate tax rise shows a trade-off: more revenue now, fewer new plants later, with both visible.

### M18 — EU membership and EU funds
Specs: [eu-membership](02-design/economy/eu-membership.md), [eu-funds](02-design/economy/eu-funds.md).
- [ ] Rules as mechanics, replacing the constants of M3: common external tariff, VAT floors, state-aid limits on the subsidies of M9, M12 and M17, the fiscal rules and the excessive-deficit procedure, free movement in migration (M5).
- [ ] EU budget contribution; cohesion, agricultural and recovery funds with a project pipeline, co-financing, absorption limited by capacity (M16), milestones.
- [ ] Schengen: leaving and rejoining, effects on trade costs and movement; external border budget.
- [ ] The compliance flag: a lever that breaks a rule is allowed, flagged, and followed by the procedure.
- [ ] Levers: the EU block of the catalogue, and export promotion.
- [ ] EU screen: rules status, funds pipeline.

**Exit:** running a deficit above the limit triggers the procedure with stated consequences; funds absorbed depend on prepared projects and capacity, not on the allocation alone.

### M19 — Environment and weather
Spec: [environment](02-design/economy/environment.md).
- [ ] Emissions by industry and household from the factors on goods; the EU carbon price for covered sectors; air pollution by county into health (M14).
- [ ] Weather from the world settings (D8): normal by default; the player triggers a drought, flood or heatwave year. Droughts hit agriculture and hydro, floods capital, heatwaves demand and health.
- [ ] Levers: the environment block (phase-out schedule, carbon tax, vehicle tax, green subsidies, irrigation and flood defence through M11, afforestation, logging limits).
- [ ] Environment screen and map layers.

**Exit:** a drought year lowers farm output and hydro generation and raises food and power prices; a carbon tax cuts emissions at a visible cost.

### M20 — Euro
Spec: [euro](02-design/economy/euro.md). Last, because it touches money, trade, the EU rules and every price.
- [ ] ERM II entry and the band; the convergence criteria computed from the simulation; assessment; conversion of every balance and price at the fixed rate.
- [ ] Inside the euro: no policy rate, no exchange rate, the ECB rate from the outside world, no monetary financing.
- [ ] Leaving: redenomination, bank holiday and capital controls during exit, the aftermath.
- [ ] Levers: the euro block.

**Exit:** the conversion conserves every balance exactly; after adoption the monetary levers are disabled with the reason shown; an exit can be played through.

---

## Part 3 — Finish

### M21 — Whole-game calibration and balance
- [ ] Full calibration with every system on.
- [ ] History check: set the world settings to the real path of 2022 to 2025 (energy prices, foreign rates and demand) and the levers to what Romania did; the run should give the direction and rough size of what happened (inflation peak, deficit, wages).
- [ ] Interaction tests: pairs of systems that were built apart (energy shock with a bank under stress; EU procedure during a housing bust).
- [ ] Long runs: 100 years quiet; many seeds; no drift in any stock without a cause.
- [ ] Balance: no lever that is free money; no lever with no visible effect; every lever's cost and benefit show up within its stated lag.
- [ ] Performance: tick time and memory at 1:100 on the reference laptop; 1:10 usable.
- [ ] Mutation testing over the mechanics (`cargo-mutants`, from Spike 9).

**Exit:** a calibration and balance report the owner accepts; parameters frozen for v1.

### M22 — The complete app
- [ ] Every view of [information](02-design/game/information.md) and every lever of the catalogue reachable; nothing only in a debug panel.
- [ ] Navigation that scales to about 150 levers: search, favourites, a "what changed" list of the player's own decisions over time.
- [ ] Notifications and an event log for what the simulation did (a bank failed, a plant opened, a procedure started).
- [ ] Onboarding: a guided first session; glossary tooltips on every term; the "why" panel as the main teacher.
- [ ] Scenario start options: seed, `sample_scale`. A world-settings panel, kept apart from the policy levers.
- [ ] Performance of the UI with full history (decades of monthly data for every indicator).

**Exit:** someone who has not seen the project plays 20 game-years and can explain why inflation moved.

### M23 — Release
- [ ] Packaging for Windows (and one other OS if cheap); licence and attribution list ([ADR-0015](03-architecture/decisions/0015-distribution-and-licensing.md)); data provenance shown in the app.
- [ ] Real company names and data terms re-checked if the release is public (ADR-0012 covers personal use only).
- [ ] Save compatibility test; crash and invariant-failure reports that include the replay.
- [ ] Outside play sessions; fix what confuses.

---

### After v1
- [ ] An outside world that moves by itself (shocks, cycles); a cost for changing levers; crises (bank failure, sovereign default, hyperinflation). Postponed by D8 to D10.
- [ ] Imperfect information: statistics lags, noise, paid polls.
- [ ] Scenarios with goals.
- [ ] Politics layer: approval with behavioural effects, parties, elections. Central bank independence.
- [ ] More industries, multiple countries.
- [ ] Military layer.

### Risks to watch
- **Stability of the closed loop (M3)** is the hardest technical step: agent-based stock-flow models tend to oscillate or drift. Keep first rules simple, add one sector at a time, and keep the quiet-baseline test from the first day of M2.
- **Every system of Part 2 can break the baseline again.** That is why each one ends with a re-run and why calibration is scripted in M7.
- **Size.** Part 2 is larger than Part 1. The alpha exists so that there is a playable game long before v1, and so that the order of Part 2 can follow what playing shows to be missing.
- **Data for the opening balance sheets (M1) and for several systems** (household wealth, informal economy, capacity and corruption) is thin for Romania; expect modelled distributions fitted to aggregates, marked as such.
- **Owner time:** about thirty specs to lock and a test-authoring session for each. Batch them by milestone.
- **Scope creep inside specs:** each spec describes more than its first version needs. The first version is what the milestone's exit test needs; the rest is listed in the spec as later.
- **Performance:** twelve more systems per tick. Measure at every milestone, not at the end.
- **The daily tick (D5)** changes the time base under every spec, the saves (history is 30 times denser if stored daily), the determinism streams (`WORDS_PER_TICK`) and the calibration (most data is monthly or quarterly). The ADR comes before M2; changing the time base after mechanics exist means rewriting them.
- **The app at 150 levers:** generated panels (M4 registry) keep it maintainable; hand-built screens for each lever do not.
