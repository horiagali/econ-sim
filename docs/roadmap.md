---
id: roadmap
title: Roadmap
status: draft
owner: horia
depends_on: [vision/scope]
updated: 2026-10-26
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
| 2 ✅ (PC model added 2026-10-26) | Ledger + differential SIM: typed transfers, invariants I-1 to I-7, core "SIM mode", independent Python SIM then PC | Exact match in bani for 200 ticks; a mutated posting sign is caught by the differential test | ADR-0007, 0010 |
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
- [x] CI runs the tiny scenario at three population scales (2026-10-20: CI job `scales`, see M2).
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

**First real mechanic: the population generator from census marginals**, built through the tests-first two-PR flow. Its acceptance criteria:
- margins within a set tolerance at 1:1000, 1:100 and 1:10;
- total weight independent of the scale;
- golden hashes;
- a Python reference with a differential test.

**Order of work.**
1. The follow-ups PR above (RNG, VAT in the scale world, CI).
2. The population generator. **Done 2026-10-10** (version 1: county, household size, sex and age): spec [`society/population-generator`](02-design/society/population-generator.md), crate `econ-popgen`, pipeline stage `econ-cli synth-population`. Second increment (education and activity, spec [`society/population-attributes`](02-design/society/population-attributes.md)): **done 2026-10-10** (spec, data, Python reference, acceptance tests, Rust code). Third increment (jobs: status in employment, occupation, industry group; spec [`society/population-jobs`](02-design/society/population-jobs.md)): **done 2026-10-10**. Fourth increment (locality size, urban, tenure; spec [`society/population-housing`](02-design/society/population-housing.md)): **done 2026-10-10**. `econ-cli synth-population` runs the three increments when given their margin files and adds their columns to the tables. **Wired into the simulation 2026-10-10:** `ScaleWorld::from_population` builds the scale world on the generated households, weights, counties and ages (`just sim-romania 100`); education, jobs, wages and deposits are still invented there. The invented population of `ScaleWorld::generate` stays for the benchmarks and its golden.
3. Other follow-ups as they are needed: the firm-unit goods market (before the firms spec locks), the PC model in the differential test (done 2026-10-26, M4), history blocks in saves.

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

### M0 — Decisions ✅ (closed 2026-10-20)
Mostly owner time. Nothing here is code; all of it unblocks code.
- [x] **D1 Order:** the three parts and the order of M9–M20 as written (owner, 2026-10-13).
- [x] **D2 Lever list.** Tagged by milestone in [levers](02-design/game/levers.md) ("When each lever arrives"), 2026-10-10. The owner changes it there.
- [x] **D3 Indicator list.** Tagged by milestone in [information](02-design/game/information.md) ("When each view arrives"), 2026-10-10.
- [x] **D4 Start date: 1 December 2021,** the census reference date (owner, 2026-10-10). No roll-forward of the population. Law, budget and balance sheets are collected as of that date. The years 2022 to 2025 are then real history to compare a run against (M21).
- [x] **D5 Tick length: one tick = one day** (owner, 2026-10-10; replaces the month every spec assumed). An hour was considered and dropped: only electricity needs hours, and nobody sets policy by the hour. Still to do:
  - [x] [ADR-0017](03-architecture/decisions/0017-time-base.md), accepted 2026-10-10: a daily clock on the real calendar; each process at its own period (daily, staggered over the month per household and firm, monthly on fixed dates, yearly); electricity with 24 hourly slots inside its daily step; weekends ignored; daily history for three years, monthly forever.
  - [ ] Reword specs from "per tick" and "next tick" to calendar time as each is locked; the spike numbers (Spike 4: per monthly tick) are re-measured in M2.
- [x] **D6 Market clearing and expectations:** decided 2026-10-11 under the owner's delegation. Markets clear by inventories ([production](02-design/economy/production.md), "First version"); expectations are anchored-adaptive ([prices](02-design/economy/prices-inflation.md), "Expectations").
- [x] **D7 Scenario and command format:** [ADR-0018](03-architecture/decisions/0018-scenario-and-commands.md), accepted 2026-10-10. A scenario is a folder of tables and small files for one scale, with the law in force as data; a lever registry (`crates/econ-core/levers.toml`) generates the command types, the UI's lever list and the test list.
- [x] **D8 The outside world: not simulated in v1** (owner, 2026-10-10). Foreign prices, fuel prices, foreign demand, foreign interest rates, wages abroad, the EU carbon price and weather are constants that the player can move with sliders ("World settings" in [levers](02-design/game/levers.md)). A world that moves by itself comes after v1. Step 3 of the calibration workflow of ADR-0009 (fitting processes for exogenous series) is postponed with it.
- [x] **D9 Cost of changing a lever: none in v1** (owner, 2026-10-10).
- [x] **D10 Extreme states: not modelled in v1** (owner, 2026-10-10). No dynamics of sovereign default, bank runs or hyperinflation. What stays: the simulation must not crash or break an invariant under any lever setting (tested in M7 and M21), which needs simple backstops (M10).
- [x] **D11 Language of the app: English only** (owner, 2026-10-13). Proper names stay in Romanian: companies, institutions, counties, cities and other places (for example *Banca Națională a României*, *Cluj-Napoca*). Text still goes through one string table (M8).
- [x] **Locked 2026-10-10:** [population-groups](02-design/society/population-groups.md) and [accounting](02-design/economy/accounting.md).
- [x] **D12 Lock rule relaxed** (owner, 2026-10-10; [ADR-0014](03-architecture/decisions/0014-agent-workflow-guardrails.md) Amendment 1): a spec can be locked before its tests; each criterion without a live test is listed in the spec under "Tests owed", and `just trace` checks that list. Ten criteria of the two specs are owed.
- [x] **[vat](02-design/economy/vat.md):** two reduced rates (19%, 9% and 5% on the start date). Test for AC-VAT-06 and code written 2026-10-13; locked by the owner 2026-10-14.
- [x] Owner review pass over the open questions of the alpha specs (households, labour market, taxation, production, prices, money-banking, monetary policy, fiscal policy, trade-fx, social transfers, demographics). The other specs are reviewed when their milestone starts. **Closed by the owner 2026-10-20:** the questions were decided as proposed when the specs were locked; the twelve still open each wait for a later step and are not alpha decisions: starting values to collect (household spending propensities, utilisation, inflation, BNR rates), legal amounts to check when the lever registry is filled in M4 (pension tax, health contribution, benefit floors), regional tightness if the runs call for it, and later scope (central-bank independence, the euro, more partner blocs, diaspora destinations, ministries' minimum budgets).

**Exit:** VAT tests and code done, then locked. **Reached 2026-10-14** (D1 to D6 and D8 to D12 are recorded; ADR-0017 accepted; seven specs locked). The rewording of older specs and the review of open questions continue milestone by milestone.

### M1 — Complete starting state
Everything the simulation needs at tick 0, from data files, consistent with the accounting matrix. Continues the population generator. Data for the systems of Part 2 is collected in their own milestones; here the tables only get the columns listed under "Built in from M1".
- [x] **Income and wealth of persons and households**, as further generator stages (all in Rust 2026-10-13, PRs #21 to #25):
  - [x] **Wages** (stage F, spec [`society/population-wages`](02-design/society/population-wages.md)): spec, data, Python reference and Rust skeleton done 2026-10-10. Levels are the national accounts' wage bill of 2021-Q4 by industry group; proportions by sex, occupation, age and education come from the Structure of Earnings Survey 2022. Open questions accepted by the owner 2026-10-10. Locked by the owner 2026-10-13, tests written (PR #14). Rust stage done 2026-10-13, matching the reference person for person, and run by `econ-cli synth-population --wages`.
  - [x] **Pensions** (stage G, spec [`society/population-pensions`](02-design/society/population-pensions.md)): spec, data, Python reference and Rust skeleton done 2026-10-10. The level is the pension expenditure of 2021; the gap between women and men and the share at the minimum pension are measured; the shape above the minimum and the link to education are assumptions marked low confidence. Locked by the owner 2026-10-13, tests written (PR #14). Rust stage done 2026-10-13 (`econ-cli synth-population --pensions`). Benefits other than pensions need no stage: the transfer mechanics compute them from the law in force (M3).
  - [x] **Declared and undeclared income** (stage I, spec [`society/population-incomes`](02-design/society/population-incomes.md)): spec, data, Python reference and Rust skeleton done 2026-10-11. Every wage has a declared part; the self-employed, informal earners and rural households that grow their own food share the mixed income of the national accounts (15.4 billion lei a month). The total is measured; every split is an educated guess held as data. Undeclared sales of firms wait for the firm table. Locked by the owner 2026-10-13, tests written (PR #14). Rust stage done 2026-10-13 (`econ-cli synth-population --incomes`).
  - [x] **Wealth** (stage H, spec [`society/population-wealth`](02-design/society/population-wealth.md)): spec, data, Python reference and Rust skeleton done 2026-10-11. Cash, deposits and loans are the totals of the financial accounts; who holds them is an educated guess with Hungary's wealth shares as a check (owner's decision of 2026-10-11: plausible interaction over matching 2021; research note [`household-wealth-romania`](01-research/notes/household-wealth-romania.md)). Stakes in firms, shares, bonds and pension fund claims wait for the firm table. Locked by the owner 2026-10-13, tests written (PR #14). Rust stage done 2026-10-13 (`econ-cli synth-population --wealth`).
- [x] **Remaining person attributes needed by the core:** split of the employed's industry group into the catalogue industry (done: stage J); split of "other inactive"; enrolment. **2026-10-13:** stage K drafted (spec [`society/population-inactivity-enrolment`](02-design/society/population-inactivity-enrolment.md), research note, fixture, Python reference: `inactive_kind` and `enrolled`, hash `d23518ac1ea5168c` at 1:1000, PR #29). Open questions decided by the owner 2026-10-14 (keep the 200,000; leave the surplus pupils unenrolled; accept age 19 and the bands). **2026-10-14:** reference reworked: pupils above the school places are left not enrolled (about 380,000; primary and lower secondary now on the school counts), hash `6bbd2499e40e5b15` at 1:1000. Locked by the owner 2026-10-14. Tests written 2026-10-15 (PR #46). **Rust stage done 2026-10-17** (feature `inactivity_enrolment`, on by default), equal to the reference person for person.
- [x] **Industry catalogue and input-output table** (2026-10-11): Eurostat's inter-country table for 2021 as `fixtures/io_table2021_ro.json`: 64 industries, purchases by origin (home, EU, rest of the world), final uses, exports, value added; balanced, and its compensation of employees equals the national accounts'. The 90 industries of [industries](02-design/economy/industries.md) are mapped into the 64 with guessed output shares.
- [ ] **Refine the 64 into the 90:** own cost structures for the catalogue industries that share a table industry (agriculture, mining, electricity and gas above all), from business statistics, agricultural accounts and energy balances. With the firm table and with energy (M12).
- [x] **Firm table** ([ADR-0016](03-architecture/decisions/0016-firm-representation.md)). Done 2026-10-11: `fixtures/firm_units2021_ro.json`, 349 cohort and public units over the 90 industries (size classes, foreign control, public producers), adding up to the input-output table. The stage that gives each employed person a catalogue industry and an employer unit is specified with its reference (stage J, [`society/population-employers`](02-design/society/population-employers.md), 2026-10-11; locked 2026-10-13 with AC-POPE-04 reworded, tests written; Rust stage done 2026-10-13, `econ-cli synth-population --employers`). Each non-financial cohort unit has an opening balance sheet (`fixtures/firm_balance_sheets2021_ro.json`, 2026-10-11: sector totals measured, spread over units guessed). Undeclared sales per unit and inventories drafted 2026-10-13 (spec [`economy/firm-units-undeclared-inventories`](02-design/economy/firm-units-undeclared-inventories.md), fixture `firm_undeclared_inventories2021_ro.json`, reference `python/reference/firm_reference.py`, PR #28): 27.1% of private value added undeclared, inventories 373.6 bn lei. Decided by the owner 2026-10-14: part of the informal income moves onto large domestic units; households' inventories kept on all domestic micro units, as written; the VAT-gap band accepted; production starts from these inventories (written into production). Small units take half the informal income (owner 2026-10-14: micro 49.9%, small 38.6%, large 34.7% of sales undeclared). Locked by the owner 2026-10-14. **2026-10-15:** tests written (AC-FUI-01 to 10, module `firm_opening` of `econ-scenario` with the API skeleton, feature `firm-opening`; golden `tests/golden/firm_undeclared_inventories.hashes`). **Rust done 2026-10-17:** `econ_scenario::firm_opening` (feature `firm-opening`, on by default), equal to the reference unit for unit. Named firms and state-owned market firms come in M9.
- [x] **Sector balance sheets.** Done 2026-10-11: the financial accounts at the end of 2021 as `fixtures/balance_sheets2021q4_ro.json`: assets and liabilities by instrument for households, firms, banks, other financial firms, pension funds, government, central bank and the rest of the world. Every instrument balances across sectors to within 0.1%, and the household column equals the totals of the wealth stage. Still to do: government debt by maturity; the central bank's foreign reserves by kind; mapping the instruments of the statistics onto the ledger's (reserves of banks at the central bank are inside "deposits" here); what to do with the sector the accounting spec does not have (insurers, funds and other lenders, 186 billion lei of assets). **2026-10-13:** the owner decided that other financial firms are a passive sector of their own. Spec [`economy/opening-balance-sheet`](02-design/economy/opening-balance-sheet.md) drafted with its research note, fixture (`sector_details2021_ro.json`: government debt by maturity and currency, central bank reserves by kind) and reference (`python/reference/opening_balance_reference.py`); every ledger row closes to the ban. It asks five changes of the locked accounting spec. Locked by the owner 2026-10-14 with the five changes to accounting (other financial sector, insurance claims, the foreign assets row, loans by any resident, other accounts left out). Tests written 2026-10-15 (PR #47). **Rust done 2026-10-17:** the ledger has the sectors and instruments of the accounting matrix (PR #52), and `econ-scenario` (feature `opening`, on by default) maps the financial accounts onto it to the ban of the reference, reconciles the household column (rule 8) and posts the balances with flow code `opening`.
- [ ] **Law in force as data:** tax rates and brackets, contribution rates, benefit rules, pension formula, minimum wage, VAT categories per good, on 1 December 2021 (D4). Collected on 2026-10-11 in the research note [`law-in-force-romania-2021`](01-research/notes/law-in-force-romania-2021.md), mostly from memory of the legislation with confidence marks; it becomes `law.json` when the lever registry exists (M4), with each value checked against the legal text then.
- [ ] **Budget as data:** spending by ministry line, public employment and wages. **2026-10-13:** drafted (spec [`economy/budget-2021`](02-design/economy/budget-2021.md), research note, fixture `budget2021_ro.json`, reference `python/reference/budget_reference.py`, PR #27): 474.0 bn lei of spending in 2021, 1,262,318 public posts, public wages 120.0 bn. Decided by the owner 2026-10-14: `budget.json` part of the scenario (ADR-0018 Amendment 1); investment inside Budget per ministry; defence held; the 16% gap as vacant posts; the public firm units rebuilt from the budget later, together with a re-golden of stage J. **2026-10-15:** done (the five public units' persons and wages are the budget's posts and D11; stage J re-golden). Owed: lock, then tests. **2026-10-20:** the loader done (`econ_io::budget`, feature `budget` on by default): AC-BUD-01 to 13 pass, the golden hash included. Owed: AC-BUD-14's test; the `World` to pay the lines it does not pay yet (fiscal policy).
- [x] **Scenario builder:** one command turns pipeline outputs into a scenario file; a fit and consistency report (every stock has a counterpart; flows of the first month reproduce the national accounts within a tolerance). **Interim, 2026-10-11:** `python/pipeline/build_scenario.py` (`just scenario-report`) runs every stage with the Python references and checks twelve things across the parts; eleven pass. The twelfth is a finding: household income by fifth is too unequal against the survey (the poorest fifth has 2.4% of income against 5.4%, and 6% of households have none), because benefits other than pensions and taxes are not applied before the simulation runs. Recorded as a known gap to re-check after M3. The Rust `econ-cli build-scenario` of ADR-0018, with Arrow tables, the opening ledger and `law.json`, follows when stages F to J exist in Rust. **2026-10-13:** stages F to J are in Rust; spec [`game/scenario-builder`](02-design/game/scenario-builder.md) drafted (tables, opening ledger, report with four new checks: ledger invariants, households and firms against the ledger, what is left out); the interim script has the new checks and passes them at 1:1000. `law.json` waits for the lever registry (M4). Locked by the owner 2026-10-14. **2026-10-15:** tests written (AC-SCB-01 to 10 and the opening balance sheet's AC-OBS-01 to 10, crate `econ-scenario` with the API skeleton, features `builder` and `opening`; golden `tests/golden/opening_ro_2021.hashes`); the interim script now reports amounts in whole lei for the differential test. **Rust builder done 2026-10-17:** `econ-cli build-scenario` (`just build-scenario SCALE`) writes the folder of the spec and its report; at 1:1000, 1:100 and 1:10 (seed 42) every check passes except the known gap `income_distribution`, and the opening ledger passes its invariants. 1:10 takes 7 seconds in release.

**Owner:** accept tolerances per stage as before; test-authoring sessions per stage.
**Exit:** `just build-scenario` produces a Romania scenario at 1:1000, 1:100 and 1:10 whose opening balance sheet passes the invariants and whose totals match the published aggregates within the agreed tolerances. **Met 2026-10-17.** Still open in M1 and not needed by M2: the law as data (M4), the budget's Rust reader, refining the 64 industries into the 90.

### M2 — Real economy loop (headless)
The first version of each mechanic is the simplest rule that is in its spec, not the whole spec. Order follows the owner's lock order.
- [x] **World and tick loop:** the real `World` (replacing the scale-world spike) loads a scenario, runs the phases of the tick order in the [game overview](02-design/game/README.md), checks invariants I-1 to I-7 and person conservation every tick, writes Arrow and CSV. The scale world stays as a benchmark. **2026-10-14:** spec [`game/world-loop`](02-design/game/world-loop.md) drafted: the M2 phase list and periods, how money moves without banks, AC-WLD-01 to 10, the API sketch. Locked by the owner 2026-10-14 (open questions decided as proposed). Owed: the test-authoring session (which also writes the four mechanics' owed `[sim]` criteria on the `World`), then the code. **2026-10-15:** acceptance tests written (test-authoring session) against the API skeleton `econ_core::world` and `econ_io::read_start_state`, behind the feature `world`: AC-WLD-01 to 04 and 06 to 10 (10 in release builds only), AC-HH-03 to 05, 07, 08, AC-LM-03 to 07, AC-PROD-03 to 07, 09, AC-PRICE-04, 05, 07, 08. Still owed: AC-WLD-05 (the golden hash comes from the first implementation), AC-PROD-08 (a staff cut that lasts) and AC-PRICE-06 (the VAT lever, M4). Next: the code.
- [x] **Clock and scheduler** as decided in D5 (the calendar types `Day`, `Date` and `Calendar` exist in `econ-types` since 2026-10-10; the cost was measured on the scale world the same day, see [spike 10](03-architecture/spikes/spike-10-daily-clock.md): a year of daily ticks at 1:100 takes 0.21 s, 1.6 times a year of monthly ticks). Still to build for the real `World`: a daily clock, processes run at their declared period, a spike that measures the cost of a simulated year (365 daily ticks, 12 of them with the monthly processes) before the mechanics are written against it. Specs that say "per tick" or "next tick" are reworded to calendar time as each is locked. **Built 2026-10-11** in `econ-core::schedule`: the periods, what is due on a date, the stagger day of a household and of a firm unit by keyed draw, cohorts by day, and the ordered list of phases. Left: the real `World` that runs them.
- [x] **Households:** income, consumption by good from baskets, saving. Pure functions done 2026-10-14 (`econ-mech-households`: the monthly decision with floor and hardship, the basket split; AC-HH-01, 02 and 06 pass); the staggered phase, buying through VAT and the ledger come with the `World`.
- [x] **Labour market:** vacancies, matching by region and skill, wages, job loss. Pure functions done 2026-10-14 (`econ-mech-labour`: fill rate, hires with the carried fraction, layoffs, part-month pay, wage growth; AC-LM-01 and 02 pass); staffing, matching and pay over units and persons come with the `World`.
- [x] **Firm-unit goods market,** then **production:** output from demand, inputs through the input-output table, inventories, hiring plans. Pure functions done 2026-10-14 (`econ-mech-production`: the monthly plan, rationing, sales shared over units; AC-PROD-01 and 02 pass); the daily output, input purchases and the goods market come with the `World`.
- [x] **Prices:** markup on unit cost, CPI. Pure functions done 2026-10-14 (`econ-mech-prices`: unit cost, markup, target, the daily step, expectations; AC-PRICE-01 to 03 pass); the daily phase and the monthly CPI come with the `World`.
- [x] Placeholders so the loop closes: government buys and pays wages at fixed real levels and taxes with VAT only; no banks (deposits only); imports at fixed prices.
- **2026-10-17, the `World` built** (`econ_core::world`, feature `world`; spec [`game/world-loop`](02-design/game/world-loop.md)): the four mechanics on the Romania scenario, every leu through the ledger, the checks of phase 13 every day, GDP equal three ways at every close. 27 of the 29 `[sim]` and `World` criteria pass on a local copy of the tests with the new spending defaults; AC-HH-05 waits for an owner decision (spec [households](02-design/economy/households.md), open questions), and AC-WLD-10 (a year at 1:100 within 2 seconds) passes alone (1.4 s) but not with the other tests running in parallel. **2026-10-20, owner:** the choices of the implementation accepted (spec [`game/world-loop`](02-design/game/world-loop.md), open questions); AC-HH-05 waits for M3's benefits; the day-31 golden `407f56fb8ccd24ea` approved. Owed by the owner's next test session: the defaults asserted in `support.rs`, AC-HH-05 marked owed, the golden committed, two SHA-256 constants of `support_builder.rs`, the SIM re-golden of PR #52. Then the feature `world` goes on by default. **2026-10-21:** done in the test session of 2026-10-20 (PR #58); the feature `world` is on by default and 28 of the 29 World and `[sim]` criteria pass, AC-HH-05 owed until M3's benefits and AC-WLD-10 passing in release when run alone.
- [x] CI runs a tiny scenario at three scales. **2026-10-20:** CI job `scales` runs the World's acceptance tests in a release build at 1:1000, 1:100 and 1:10 (AC-WLD-02, 03 and the ignored 1:10 run), with AC-WLD-10's time budget. A year at 1:100 was 2.3 s on the Windows laptop, over the 2 s budget; result-preserving speed-ups brought it to 1.3 s (the goldens are unchanged): `split` and `apportion` divide once and select instead of sorting, the payment of households splits in two without allocating, essential inputs' coefficients are looked up once, the state hash keeps its lanes in registers. 1:1000 takes about 0.6 s a year, 1:10 about 20 s. `econ-cli run-world --sample-scale N --years N` runs the World from the command line.
- [ ] A 50-year run at 1:100 within the time budget of Spike 4 (about 70 s at 1.3 s a year). **2026-10-20:** the World runs about 30 years, then stops: the rest of the world's lei deposits go negative (a steady trade surplus with no exchange rate to correct it). Over those years unemployment falls slowly from 6.4% to 1.5% and the wage share rises from 38% to 44%, with no investment or growth. Waits for M3's trade and exchange rate, and investment.
- [x] Thin UI window on this core (time controls, a dozen charts). **2026-10-20:** the Tauri app opens the World on the Romania scenario at 1:1000 or 1:100 and steps it by a day, a month, a year or continuously; twelve monthly charts (unemployment, inflation, expected inflation, CPI, employment, GDP, consumption, wage bill, wage and consumption shares, shortages, hardship). The host compiles and the frontend builds in CI (job `ui`); the first launch of the window on the laptop (`cd ui; npm run tauri dev`) is still to be done.

**Owner:** households, labour market, production and prices were reworded for the daily tick and locked on 2026-10-11 (delegated: "accept the drafts"), each with its criteria under "Tests owed". Owed: the test-authoring session for them.
**Exit:** 600 ticks at three scales with no invariant failure; unemployment, wage share, consumption share and inflation stay in a plausible band with no intervention.

### M3 — Money, state and the rest of the world
Closes the loop: after this, every sector of the [economy overview](02-design/economy/README.md) is live.
- [x] **Taxation, the rest:** personal income tax with brackets and allowance, social contributions, corporate profit tax, capital income tax, excise, property tax, wealth tax. **2026-10-14:** first version drafted in [`economy/taxation`](02-design/economy/taxation.md) (the 2021 rates, deduction, industry overrides, pensions, corporate and micro taxes; AC-TAX-01 to 13; API sketch). Locked by the owner 2026-10-14. Owed: tests, then the code. **2026-10-15:** tests of the unit criteria AC-TAX-01 to 07 written (module `econ_mech_tax::income`, feature `taxation`); AC-TAX-08 and 09 wait for the World's pay-day phase. **2026-10-17:** the pure functions done (AC-TAX-01 to 07 pass; feature `taxation` on by default). **2026-10-20:** the pay-day and quarter-end phases built in the `World`, with flow codes 3003 to 3008 (see the spec for the first year's figures and what they show for calibration). Owed: the tests of AC-TAX-08 and 09. **2026-10-20:** tests written by the test session; the pay-day record (`World::pay_slips`) built behind feature `pay_day`: AC-TAX-08 passes; AC-TAX-09 (corporate tax) waits for depreciation and interest (investment, banking).
- [x] **Social transfers, core set:** state pension, unemployment benefit, child benefit, minimum income. **2026-10-14:** first version drafted in [`economy/social-transfers`](02-design/economy/social-transfers.md) (pension points, unemployment benefit by contribution years, child allowance, guaranteed minimum income, all at their 2021 values; AC-ST-01 to 12; API sketch for a new crate `econ-mech-transfers`). Locked by the owner 2026-10-14. Owed: tests, then the code. **2026-10-15:** tests of the unit criteria AC-ST-01 to 04 written (new crate `econ-mech-transfers`, feature `mechanic`); AC-ST-05 to 08 wait for the World's pay-day phase. **2026-10-17:** the pure functions done (AC-ST-01 to 04 pass; feature `mechanic` on by default). **2026-10-20:** the pay-day phase built in the `World`, with flow codes 4003, 4004 and 4006. Owed: the tests of AC-ST-05 to 08. **2026-10-20:** tests written; the pay-day record and the layoff edit built behind feature `pay_day`: AC-ST-05 to 07 pass. AC-ST-08 fails because the world under `Law::law_2021()` is not a quiet baseline. The starting state was calibrated with wages as take-home pay, so the real withholdings cut household consumption by about 21% (47 bn lei a month against about 60 bn) and employment falls about 12% in six months. The base world sheds retail staff by itself, which hides the edit's layoffs. Owed: calibrating the start under the 2021 law (with fiscal policy's spending), then `pay_day` on by default.
- [x] **Fiscal policy:** budget lines, public wages and headcount, deficit, bond issuance and the bond market, debt service. **2026-10-20:** the pure functions done (`econ-mech-fiscal`: the debt book by year of maturity, coupons, issues and redemptions, the bond yield; AC-FIS-04 passes). **2026-10-20:** the `World`'s treasury (feature `fiscal`, on by default; acts under the 2021 law): the budget lines the `World` did not pay (the EU budget to the rest of the world, other current transfers to households, capital transfers and subsidies to firm units, a twelfth a month), coupons to every holder, redemptions, and issues back to a month of spending; the scenario now carries `budget.json` and the debt profile (over ten years taken as ten to twenty). AC-FIS-02, 03 and 05 pass. Choices made under the owner's delegation, to review: the central bank buys no new issue (no monetary financing unless chosen); the rest of the world pays for its half of each issue from its lei deposits, which may go negative until trade's intervention; the treasury's payments to households go to their deposits, not to the month's income. **What the first runs show:** under the 2021 law the first year's deficit is 15% of GDP (2021: 7%), unemployment rises to 14–16%, and firm units' deposits grow by about 7 billion lei a month: firms keep their profits, because dividends (households' property income) do not exist yet. After about two and a half years other financial firms run out of deposits to buy their share of the issues and the run stops. Owed: dividends and retained earnings ([investment and capital](02-design/economy/investment-capital.md), below), then calibration under the 2021 law; AC-FIS-01's test.
- [x] **The rest of 2021's revenue, benefits and imputed rent** (owner, 2026-10-21: pulled forward from M12, M13 and M18 so the `World` under the 2021 law can hold a quiet baseline): [excise](02-design/economy/excise.md) (fuel, tobacco, alcohol; 34.7 billion lei), [EU funds](02-design/economy/eu-funds.md) M3 (grants and farm payments; 32.4 billion), [government revenue 2021](02-design/economy/government-revenue-2021.md) (the product taxes inside firms' prices, about 48.5 billion; taxes on production; the small revenue lines and the budget's other cash benefits at 2021 levels) and [imputed rent](02-design/economy/imputed-rent.md) (about 77 billion paid back to owner households). Locked by the owner 2026-10-21 with every criterion owed; dated additions in prices-inflation, vat, trade-fx, households and production. Expected: revenue about 331 billion against a comparable 352.8 in 2021 ([research note](01-research/notes/government-revenue-romania-2021.md)). **2026-10-21:** built (feature `revenue`, off by default, before its tests by the owner's decision): imputed rent paid back on the 15th (restated from pay day by the owner, for AC-ST-05), the product taxes inside firms' prices (`tax.products`), other taxes on production, excise (households with VAT on the duty, firms on their fuel; the duty stays in firms' other costs, so prices do not move), the small revenue lines, the budget's other cash benefits (on the 15th as well) and EU funds (grants to the government, farm payments to the crop and livestock units); flow codes 2011 to 2013, 3011 to 3015, 4002, 4005, 4007, 4105 to 4108, 5002. **With dividends on and the investment rule off**, the `World` under the 2021 law over three years: deficit 6-8 billion lei a month (2021: about 7), unemployment 8-11% (2021: 5.6%), household consumption 53-58 billion a month (2021: about 60). Owed: calibration (unemployment, the slow fall of consumption), then the test session of M3's revenue.
- [x] **Money and banking:** aggregate bank; loans to households and firms, deposit and loan rates, capital and reserve constraints, defaults. **2026-10-20:** the pure functions done (`econ-mech-banking`: rates, a month's interest, credit room, write-offs, the capital ratio). **2026-10-20:** the `World`'s bank (feature `banking`, off by default; acts under the 2021 law): credit lines in place of credit inside the firm sector, interest per record at month end, repayments, write-offs, rates and capital at the close, reserves topped up with central bank advances; flow codes 2004, 2005, 7003 to 7005. Households keep the month's interest aside when they decide (disposable income net of interest, as spec `economy/households` defines it). AC-BANK-01, 02, 03, 05, 06 and 07 pass. AC-BANK-04 fails on one unit: the data-centre cohort pays about 19 million lei a month in wages on 2.6 million of revenue (its 1,045 employees are paid the information-and-communication wages of stage F, its output is the catalogue's small share of the table row), so its credit line closes and it falls back on credit inside the firm sector: the M1 item "refine the 64 into the 90" above. AC-BANK-08 (ten years of quiet baseline) waits for dividends and the calibration under the 2021 law. Owed: those, then `banking` on by default.
- [x] **Monetary policy:** policy rate and its pass-through, reserves, QE, monetary financing, inflation target and credibility. **2026-10-20:** the pure functions done (`econ-mech-monetary`: credibility, the advisory rule; AC-MP-01 and 02 pass). **2026-10-20:** the `World`'s central bank (feature `monetary`, off by default; acts with the banks): interest at the deposit facility on excess reserves and at the lending facility on advances, the foreign assets' yield (paid by the rest of the world in foreign assets), the profit remitted to the government before the treasury's financing, monetary financing, credibility (now what expectations use) and the advisory rule at the close; the deficit-financing edit; flow codes 2006, 2007, 4104. AC-MP-03 and 04 pass. AC-MP-05 shows the financing and the fall of credibility, but expected inflation falls: credibility only weights the target against past inflation (the spec's update rule), and inflation runs below target in the World under the 2021 law, so less credibility pulls expectations down; nothing in M3 makes monetary financing raise demand or prices. **To decide (spec):** a direct term for monetary financing in expectations, or AC-MP-05 restated for inflation above target. With the yield in foreign assets, AC-TFX-03's identity is: current account + change of the rest of the world's deposits + its net bond purchases − change of the central bank's foreign assets = 0.
- [x] **Trade and exchange rate:** exports and imports per good with two foreign blocs, floating rate, managed float and peg, FX intervention, reserves. EU rules reduced to constants until M18 (tariffs fixed, no capital controls). **2026-10-20:** the pure functions done (`econ-mech-trade`: the exchange-rate step with intervention, the Armington share, export demand; AC-TFX-01 and 02 pass; the risk premium's default is the starting real-rate gap, 1.75 points, as restated). **2026-10-20:** trade on the `World` (feature `trade`, off by default; acts under the 2021 law): import prices at e / e0 on every import, export demand by price, households' Armington shares, the exchange rate and the central bank's intervention at the close (flow code 7006), the month's current account. AC-TFX-04 passes. AC-TFX-03's identity holds to the ban once the rest of the world's bond purchases are in it (the test predates them; the spec's restatement had its signs wrong, corrected): owed to the test session. AC-TFX-05 (ten years) waits for the calibration under the 2021 law. In the first month the current account is a deficit of about 4 billion lei and the rate barely moves.
- [x] **Investment and capital:** firm investment, depreciation, credit demand, productivity growth. Needed for the policy rate to matter and for long runs to grow. **2026-10-20:** [`economy/investment-capital`](02-design/economy/investment-capital.md) rewritten for M3 and locked under the owner's delegation (open questions as proposed): dividends (the missing property income that leaves firm deposits growing under the 2021 law), depreciation and capital, an investment rule, government investment moved to the government; calibration targets from Eurostat's 2021 accounts in [a research note](01-research/notes/investment-dividends-romania-2021.md). **2026-10-21:** by the owner's decision, dividends and what they need built before the test session (one big session after): crate `econ-mech-investment` (the capital book, the investment budget, dividends and their split by owner) and the `World`'s feature `investment` (off by default): each unit's capital from its fixed assets, depreciated monthly, with M2's investment purchases completing into it after three months; each market unit's monthly profit after an accrued tax; dividends on the 15th by ownership (the government, the rest of the world, pension funds, households by their money at load, less the 5% dividend tax); the quarterly profit-tax base now deducts depreciation and the mixed income paid; flow codes 2010 and 3009. The investment rule (steps 1, 2, 7, 8) and government investment come later. **What it shows:** with dividends, firms' deposits grow far more slowly (216 billion lei after two years against 307), households spend about 3 billion a month more and unemployment is about 2 points lower, but the deficit stays about 18 billion a month. The cause is revenue: the `World` collects about 199 billion lei a year against 389 in 2021 (TR). Excises (about 33 billion), EU grants, non-tax revenue and property taxes are not modelled, VAT is 40 billion against about 72 (households still spend about 20% less than in 2021), and the profit tax shows only at quarter ends. **2026-10-21, the investment rule** (owner: build it too): each unit's budget on its plan day by `desired_investment`, financed by spare money and credit room; daily purchases by the investment column's shares; the government's own investment (48 billion lei a year) paid by the government under `gov.investment` (1003); capacity from capital, expected growth from realised sales, productivity (`TFP_GROWTH_PPM`). **What it shows:** in the depressed World under the 2021 law the rule deepens the slump: utilisation is low, so firms cut investment, which cuts demand (unemployment 16-22% over three years at 1% productivity growth, about 13% at 0%; firms' deposits grow again). Productivity growth is 0 until calibration (open question 7). AC-TAX-09 now passes (the profit tax after depreciation and mixed income). Owed: the households' income against the 2021 accounts (they spend about 49 billion lei a month against about 60), excise and EU funds, then the test session (AC-INV-01 to 10).

**Owner:** lock the specs above, one or two at a time. **2026-10-20:** money and banking, monetary policy, fiscal policy and trade reworked for M3 and locked by the owner's delegation (the open questions decided as proposed), with the budget of 2021; every criterion listed as owed until the test session. **2026-10-22, closing attempt:** the test writer restated AC-GRV-03, AC-GRV-06, AC-EUF-02, AC-EUF-05 and AC-IMP-03 and owed AC-INV-06 and 10 to the investment rule (M7) (PR #82). The implementation then fixed the EU farm key (it included `agri_construction_machinery`), paid the held benefits from the first 15th, rounded the exchange-rate index of EU funds to the nearest ppm and filled `StartEdit::Depreciation`. With every M3 feature on (`revenue investment banking trade monetary pay_day`), 68 acceptance tests pass, 8 are ignored and 5 fail. **M3 stays open and its features stay off by default (owner)** until these are decided:
  - AC-GRV-03: the fall in firm deposits plus the fall in dividends (47.2 billion lei) is 17% short of the taxes paid (57.1). Counting the lower profit tax brings it to about 8%, still outside 5%.
  - AC-GRV-06: the spec's key "employees" is read by the test as the employees at load, while the `World` pays the employees of the day.
  - AC-IMP-03: real estate makes −30.4 billion lei net of depreciation against the restated −15.1. The net target is a loss because the fixture gives real estate 433 billion of capital (open question in [imputed rent](02-design/economy/imputed-rent.md)).
  - AC-BANK-08: firm loans reach 16.9 trillion bani by April 2023, against 12.9 at the start.
  - AC-WLD-10: passes alone (a year at 1:100 in about 2.3 s), but not alongside the other tests.

**2026-10-22, M3 closed (owner).** The test writer restated AC-GRV-03 (taxes against the fall in deposits, dividends and profit tax, within 10%) and AC-GRV-06 (keys read on the day of payment) in PR #87. It also listed three tests as owed to M7: AC-BANK-08 (firm loans grow about 31% by April 2023), AC-TFX-05 (the leu drifts about 10.3% over ten years) and AC-IMP-03. For AC-IMP-03, the real estate fix of M7's item below was pulled forward: the surplus is now -19.5 billion lei, inside its band, but profit falls by less than the payback. The implementation now pays the held benefits from the first 15th without paying month 0's twelfth twice, and the M3 features (`pay_day banking monetary trade investment revenue`) are on by default. 69 acceptance tests pass, 11 are ignored, and AC-WLD-10 passes alone. Two items moved to M4: the outside world's world-settings command (it needs the lever system's commands) and the PC model in the differential test. **The exit criterion's 50-year quiet baseline is not met yet:** AC-BANK-08, AC-TFX-05 and AC-INV-09 are owed to M7's calibration. **Owner, 2026-10-22:** closing with the baseline pushed to M7 is accepted because it is a calibration result; M7's exit criterion now names it. The two moves to M4 are accepted. Every key of a payment (employees, the poorest fifth, owners) describes households as they are when the money is paid, as AC-GRV-06 now says. Owners' tenure does not change before housing (M13), so the owner keys of imputed rent and property tax are unaffected until then.
**Exit:** the full transaction matrix balances every tick; a 50-year quiet baseline is stable; government debt, the current account and bank balance sheets move for reasons that can be read off the ledger.

### M4 — Lever system and alpha levers
Spec: [lever system](02-design/game/lever-system.md).
- [x] **The outside world** as decided in D8: a table of constants read by the mechanics, each changeable by a world-settings command (moved from M3 at its close, 2026-10-22).
- [x] PC model added to the differential test (open follow-up of Spike 2; moved from M3 at its close, 2026-10-22). **Done 2026-10-26:** Godley and Lavoie's model PC (bills, a central bank that buys what households do not hold and remits its interest, the rate raised from 2.5% to 3.5% at tick 100) runs on the real ledger (`econ_core::pc`, `econ-cli pc`) and matches an independent Python reference (`python/reference/pc_reference.py`) exactly in bani for 200 ticks; two mutants (interest left out of income, the portfolio rule's sign) are caught. `just diff-sim` runs SIM and PC.
- [x] **Lever system:** every lever and world setting is a typed command with validation, range, effective date and lag; commands are logged and replayable (ADR-0011); a lever registry generated from the specs so the UI and the tests share one list.
- [x] **Pay and job choice** (owner, 2026-10-22; [labour market](02-design/economy/labor-market.md)): hiring and quits respond to a unit's pay against the market, with job-to-job moves, so the public pay lever bites. Ministry budgets with a running and an investment part for every line; a wage bill above the running part freezes hiring.
- [x] Wire every lever tagged `alpha` in D2 to its mechanic. Since 2026-10-10 that includes three tools against the informal economy (labour inspection budget, tax audit budget, penalties), with a first version of detection: taxes are computed on declared amounts from the start (M3), because the starting state carries the split. **Owner, 2026-10-24:** the three informal levers move to M15 with detection (its criteria AC-INF-01 to 09 have no tests yet); with them, `transfer.pension.retirement_age` and `world.wages_abroad` wait for M5 and `world.fuel_price` for the energy mechanic. **Owner, 2026-10-25:** `world.fuel_price[oil]` is wired in M4 (the euro import price of crude and refined petroleum, for the oil-shock scenario S9); its gas and coal keys wait for M12 and reject commands with `not_yet`. A new `market` lever `fx.devaluation` (a one-off step of the exchange rate, S6) is added and wired the same day. AC-LEV-14 covers the other 53 alpha levers (`world.fuel_price` stays outside it until gas and coal are wired, since it moves every key together).
- [x] **Policy-scenario test suite** (qualitative: direction, rough size, timing): rate hike, income-tax cut, VAT rise, deficit spending, monetary financing, devaluation, minimum-wage rise, pension rise, oil-price shock. One test file per scenario, written tests-first. Every later milestone adds its own scenarios to this suite. Spec: [policy scenarios](02-design/game/policy-scenarios.md).
- [x] Headless "play" from a command file: scenario plus dated lever changes in, time series out.

**Exit:** each scenario test passes at three scales; a mutated rule (wrong sign) makes at least one of them fail. **Met 2026-10-26.**

**2026-10-26, M4 closed (pending the owner's review of PR "close M4").** What M4 built: the **lever registry** (`levers.toml`, generated code, one list for the UI and the tests), **validation** (range, units, effective date and lag, the scenario's law as data), the **command log with replay** and saves, **headless play** (`econ-cli play`, `just play`), and **11 lever groups** wired: (1) the registry, its checks and the generator; (2) validation, effective dates and the scenario's law; (3) logging, replay and saves; (4) headless play; (5) ministry budgets, public pay and headcount; (6) pay and job choice; (7) the tax levers; (8) the money levers; (9) the FX levers, with the one-off `fx.devaluation`; (10) the world settings and the group counterfactual; (11) bond maturity and pension indexation. The **policy-scenario suite** (AC-POL-01 to 11): all nine scenarios pass at 1:1000 and 1:100 in every PR's CI (the four lever features `lever_policy_rate`, `lever_monetary_financing`, `lever_fx_devaluation` and `lever_fuel_price` are on by default), at 1:10 in the new nightly workflow (about 6 minutes on the Windows laptop), and the three sign mutants MUT-1 to 3 are each caught by their scenario (`just policy-mutants`, also nightly). Pulled into M4: the **insolvency backstop** from M10 (a firm that cannot pay its wage bill pays what it can and cuts staff, so no lever setting stops the run), and the **price "why"** (AC-LEV-15, `World::price_why`). Also done: the PC model in the differential test, and a faster daily step (a 64-bit path for the 128-bit divisions in the hot loops, bit-identical; a simulated year at 1:100 went from about 1.6 s to 1.4 s on the Windows laptop, AC-WLD-10's budget being 2.0 s). Moved out: the three informal-economy levers to M15 (with detection), `transfer.pension.retirement_age` and `world.wages_abroad` to M5, the gas and coal keys of `world.fuel_price` to M12 (commands rejected with `not_yet`). With the policy suite on by default, `cargo test --workspace` (a debug build) runs it too: econ-core's acceptance tests take about 7 minutes in debug on the laptop.

### M5 — People over time
Without this a long run is a frozen population.
- [ ] **Demographics:** ageing, deaths, births, retirement, household formation and dissolution, moves between counties, emigration and return, the diaspora and remittances, with alignment to official projections.
- [ ] **Education flows:** enrolment, graduation, entry to the labour market (simple version; depth in M14).
- [ ] **Approval, first version:** the per-person number of [opinion-approval](02-design/society/opinion-approval.md), driven by own finances. Display only. Depth in M14.
- [ ] Poverty and inequality indicators from incomes: [poverty and inequality](02-design/society/poverty-inequality.md) (with the standard-of-living classes of M6).

**Exit:** a 50-year run reproduces the direction of the official population projection (shrinking, ageing); pension spending responds to it.

### M6 — Indicators and explanations
- [ ] **Statistics registry:** every indicator tagged `alpha` in D3 computed each tick from state and ledger (GDP three ways, CPI and components, employment and unemployment, wages, budget and debt, trade and current account, money and credit, distribution, by county and by group).
- [ ] **Group queries:** any filter over person attributes returns counts, incomes, taxes paid, transfers received, approval.
- [ ] **Standard of living by class** (owner, 2026-10-26; display only, behaviour stays on income fifths). Classes by fixed real lines anchored at 2021 and moved only with prices: at risk of poverty below 60% of 2021's median equivalised income, lower-middle 60–75%, middle 75–200%, affluent above 200% (OECD's middle-class band, Eurostat's anchored poverty rate). Unlike fifths, a class can shrink to nothing as people get richer, or grow in a crisis. The anchor is 2021 and cannot be changed in v1 (owner, 2026-10-26); the relative rate (below 60% of today's median) and hardship stay meaningful in any year. Shown next to the relative fifths (how equal) and the hardship count (spending below the subsistence basket): shares over time, by county and by group, with the "why"; the panel itself in M8.
- [ ] **History** stored in saves (open follow-up of Spike 6).
- [ ] **"Why did this change":** explanation trees (ADR-0008) for the headline indicators; every rule added in M2–M5 already uses `behaviour_rule!`, here they are joined up and tested (parts sum to the change).
- [ ] Core API for the UI: one typed query and command surface.

**Exit:** for each headline indicator, any month's change is decomposed into named causes that sum exactly.

### M7 — Alpha calibration and stability
- [ ] Calibration targets from data: national accounts shares, labour market rates, budget lines, CPI weights, trade, credit.
- [ ] **Households' income under the 2021 law** (2026-10-21): with every M3 revenue and benefit in, households receive about 650 billion lei a year and spend about 645, against the input-output table's 720 that production is calibrated to; the gap (about 10% of household demand) keeps unemployment at 8-11% against 2021's 5.6%. Candidates: remittances (M5, about 17 billion a year), households' new borrowing (none in M3), what the table counts as household consumption (non-residents' spending in Romania), the propensity to spend out of wealth.
- [ ] **Switch the investment rule on and tune it** (owner, 2026-10-21). It is built (spec [investment and capital](02-design/economy/investment-capital.md), steps 1 to 3 and 6 to 8, the government paying its own investment) but off: `INVESTMENT_RULE` in `crates/econ-core/src/world/investment.rs`. In the `World` under the 2021 law it deepened the slump (units below the utilisation it takes as normal cut investment, which cut demand; unemployment 14-16% against 7-10% without it). Tune `beta_cu`, the neutral utilisation, expected growth's adjustment and its financing, then AC-INV-08 and 09 with it on; productivity growth (`TFP_GROWTH_PPM`, 0 until then) with it.
  - With it off, units pay out what they would have reinvested: dividends paid abroad are 38.6 billion lei in the first year against 18.6 (AC-INV-10; Romania's 2021 FDI income was about twice its dividends, the rest reinvested earnings). AC-INV-06 also waits for the rule.
- [ ] **Real estate's mixed income and surplus** (2026-10-21). Mixed income is paid by the domestic cohort units by value added less wages, which for `real_estate` includes owner-occupiers' imputed rent; the payback already gives that surplus to owner households, so real estate pays mixed income out of the same surplus twice and its profit before tax is negative (AC-IMP-03 reads -30.6 billion against the spec's 13.1, which is gross of depreciation, about 28 billion a year). Take the imputed part out of real estate's mixed-income weight, and restate the spec's target net of depreciation.
- [ ] **If needed: randomness in firms' decisions** (owner, 2026-10-22). Pull the after-v1 item "Randomness in firms' decisions" forward only if the runs move in lockstep (whole industries reacting alike and swinging together) or are too smooth against Romania's volatility.
- [ ] **Minimum pension in the starting data** (owner, 2026-10-25): pensioners at the 800-lei minimum get their own (lower) entitlement from their pension points, so the minimum pension tops them up and cutting it lowers their income; today stage G puts 21% of pensioners at exactly 800 on their points (AC-POPP-01), so the lever's bottom cannot bind. Also check `economy/social-transfers.md`'s 2026-10-20 note that "the 800-lei minimum lifts many of stage G's pensions".
- [ ] **Calibration note: the baseline's inflation and the leu** (owner, 2026-10-25): with no lever changes, the 2021-law World runs 8–9% inflation and the leu drifts from 4.95 to 6.2 lei per euro over three years (found by the policy suite's baseline, 2026-10-25).
- [ ] **Firms respond gradually to financial strain** (owner, 2026-10-26). Today production, hiring and prices follow demand only; a firm's money matters only at the hard limits (input caps of PR #107 and the insolvency backstop), so a firm heading for its credit limit carries on and then falls off a cliff. Add a soft response: as a unit uses more of its credit line (or its cash falls below its buffer), it lowers its hiring target, its desired investment and its dividends, raises its margin and buys inputs more cautiously, well before any limit (the financial accelerator). Tune it with the investment rule, which already finances only from spare money and credit room. The hard backstop stays as the last line.
- [ ] **A starting state without COVID** (owner, 2026-10-26; research first, then decide whether it is doable). The simulation runs as if COVID never happened (demographics use pre-pandemic rates, M5). The 2021 starting data still carries COVID: in the budget (technical unemployment and short-time work support, vaccines, tests and hospital bonuses, grants and support schemes for firms), and possibly in the input-output table, employment, consumption and savings. Research how much of 2021's spending is identifiable as COVID measures (Ministry of Finance budget execution, Eurostat's and the European Commission's tables of COVID-19 fiscal measures, the Fiscal Council's reports), whether removing it keeps the opening balance sheet consistent, and whether to use normalised values instead; then propose a change to the locked budget-2021 spec.
- [ ] Sensitivity screening and history matching with the loop of Spike 5 ([ADR-0009](03-architecture/decisions/0009-calibration-and-stability.md)).
- [ ] **Government bond yields from investor confidence** (owner, 2026-10-22; research when implementing). Today a new bond's yield is the policy rate + a term premium + a risk premium that only starts above 60% debt-to-GDP ([fiscal policy](02-design/economy/fiscal-policy.md), step 4), so at Romania's 49% small deficits and growth lower nothing. Make the risk premium an investor-confidence term built from, for example: expected inflation, expected growth, the deficit, debt-to-GDP (below 60% too), the past record (deficits and inflation of the last years, missed targets), the central bank's credibility, and the share of debt held abroad. A credible path of small deficits in a growing economy should lower yields; Romania's 2021 spread came mostly from its deficit (about 7% of GDP), not its debt. **Keep the lag:** the yield applies to new issues only, so the average coupon of the stock moves as debt is refinanced (7-year bonds: over years). Research the weights against published estimates of sovereign spreads (EU countries, rating agencies' factors). The fiscal-policy spec (locked) is amended when this is built.
- [ ] Response checks against published estimates: fiscal multipliers, pass-through of the policy rate and of the exchange rate to inflation.
- [ ] 100-year quiet baseline; extreme-lever runs (tax at 90%, rate at 50%, heavy money printing) end in a bad economy, not in a crash or a NaN.
- [ ] Performance at 1:100 and 1:10 with every alpha mechanic on. The budget must leave room: Part 2 roughly doubles the work per tick.
- [ ] The calibration is scripted end to end, because it is re-run after every milestone of Part 2 and in full in M21.

**Exit:** a calibration report the owner accepts; goldens recorded; **M3's 50-year quiet baseline holds** (owed here when M3 closed, owner 2026-10-22: AC-BANK-08, AC-TFX-05 and AC-INV-09 pass, with AC-IMP-03, AC-INV-06, AC-INV-10 and AC-ST-08).

### M8 — Alpha app
On the stack of [ADR-0013](03-architecture/decisions/0013-ui-stack.md), growing the thin UI kept since M2. Built so that Part 2 adds screens without changing the frame.
- [ ] New game from a scenario; save, load and autosave; time controls and speeds.
- [ ] Macro dashboard with pinned indicators; notifications for large moves.
- [ ] Lever panels generated from the lever registry: range, current law, lag, pending changes; tax-bracket editor; budget screen (revenue, spending, deficit, debt).
- [ ] Central bank screen (rate, reserves, QE, monetary financing, FX).
- [ ] Graph browser for every indicator in the registry; group explorer; county map.
- [ ] "Why" panel on every headline number.
- [ ] Text through one string table (D11: English only; proper names in Romanian).

**Exit:** the owner plays 20 game-years of the core in one sitting without touching a terminal. **This is the alpha.** Review the order of Part 2 here with what playing has taught.

---

## Part 2 — Systems: the rest of scope
The order follows what depends on what: firm units before anything owned by a firm (plants, foreign owners, state companies); projects before energy, housing and EU funds; energy and EU before environment; EU, money and trade before the euro.

### M9 — Firms in depth and state ownership
Specs: [production](02-design/economy/production.md), [state-enterprises](02-design/economy/state-enterprises.md), [investment-capital](02-design/economy/investment-capital.md), [fiscal-policy](02-design/economy/fiscal-policy.md) (subsidies), [prices-inflation](02-design/economy/prices-inflation.md) (caps).
- [ ] **Named firms:** the largest real companies per industry as their own units (foreign-owned, domestic, state-owned), from public company data. Legal check on using real names is not needed for a personal project; revisit before any public release (M23).
- [ ] Firm entry, exit and bankruptcy in the cohorts; administration for named firms.
- [ ] Equity, dividends and a simple stock exchange, enough for share sales.
- [ ] **What firms do with a surplus** (owner, 2026-10-21): a choice by expected return between investing in their own activity, buying government bonds, buying shares of other firms, lending, holding assets abroad and paying their owners more. M3 has the first and the last (the investment rule and dividends with a cash sweep) and a fixed share of each bond issue.
- [ ] State-owned enterprises: objectives (profit, employment, price), directives.
- [ ] **Innovation** (owner, 2026-10-22): a firm can spend on research and its own efficiency, among its surplus choices; productivity rises with a lag and uncertainty. Replaces the calibrated productivity growth for firms that do it.
- [ ] Levers: nationalise, privatise, bail out a named firm, directives; all subsidy kinds (production, investment, wage, consumer); price caps with shortages; research grants and an R&D tax credit.
- [ ] Industry and supply-chain views; company pages.
- [ ] **A stock index like the BET** (owner, 2026-10-25): an index of the largest named companies, weighted like the Bucharest Stock Exchange's BET, shown as a chart of how the top companies are doing. First version: a visual only, from their profits and value (no trading); with the stock exchange above, it follows share prices.

**Exit:** nationalising a firm moves equity through the ledger at the right value; a price cap below cost produces a shortage that the "why" panel names.

### M10 — Bank regulation and backstops
Specs: [money-banking](02-design/economy/money-banking.md), [fiscal-policy](02-design/economy/fiscal-policy.md). Small in v1, because crises are not modelled (D10).
- [ ] Levers: bank capital requirement, deposit insurance coverage, bank bailout (a capital injection for equity).
- [ ] Backstops, so that no lever setting stops the simulation: a bank below its minimum capital stops new lending until profits or a bailout restore it; government bonds that find no buyer at the current yield are placed at a higher yield, up to a cap, then with the central bank (booked as monetary financing and shown as such); a household or firm that cannot pay defaults by the rule of [accounting](02-design/economy/accounting.md), with the loss booked to the creditor. **Owner, 2026-10-26:** the firm part's wage and tax backstop was built in M4 so that S5 runs to its end ([labour market](02-design/economy/labor-market.md#backstop-firms-that-cannot-pay-owner-2026-10-26)): a firm unit that cannot pay pays wages pro rata and lays off, taxes and interest it cannot pay are reported unpaid, dividends are skipped; no debt stock, default rule or loss to creditors yet, which stay here.
- [ ] Each backstop raises a notification, so the player sees that a limit was hit.
- [ ] After v1: failure of a bank, runs, sovereign default and restructuring, high-inflation dynamics; and whether there are several banks.

**Exit:** every extreme-lever run of M7 ends with invariants intact and with the backstops that fired listed in the event log.

### M11 — Infrastructure and the project system
Specs: [infrastructure](02-design/economy/infrastructure.md), [transport network](02-design/economy/transport-network.md). **Owner decisions 2026-10-22:** position matters through a county graph; the player picks the contractor from bids; a finished hospital or school adds capacity and has running costs (staff, inputs).
- [ ] Projects: type, place (county, or a route of graph links), size, funding from the budget or EU funds; cost, build time, progress, delays, overruns, completion; a queue the player manages; pause and cancel.
- [ ] Tenders: construction units with spare capacity bid a price and a delivery time; the state-owned company can be more expensive or have no spare capacity, and can still be given the contract (it queues it); a long order book raises a company's prices and, through the investment rule, its capacity; a foreign contractor's margin is imports; the cheapest wins by default and the player may pick another. Contractors hire in the project's county and buy materials by their input column; many projects at once raise bids and delays.
- [ ] Assets by type and place (motorways, roads, rail, high-speed rail, airports, ports, broadband, water, schools, universities, hospitals); operators and running costs (hospital and school staff hired from the county's labour market, inputs such as medicines); unfunded or unstaffed capacity counts for less; maintenance and decay.
- [ ] Transport network: a graph of the 41 counties, Bucharest and gateway nodes, links by mode; shortest paths on each change; each county's market access sets its firms' productivity, its access to ports and borders its trade costs; travel times set commuting, migration and hospital catchments.
- [ ] Effects for M14: school places enrol pupils who had none (about 380,000 above the places at the start) and raise quality where there was a shortage; staffed beds per head set access to care.
- [ ] Data: existing stocks by county, travel times between county seats, unit costs and build times, construction companies (research note).
- [ ] Levers: commission, pick the contractor, pause, cancel, overruns, running budgets, maintenance budget, tolls. Map: draw a route between county seats and see each county's market access change before building; layers for assets and projects.

**Exit:** a Bucharest–Constanța line raises market access and, years later, output more than a line of the same length between two small counties, and the path from spending to output is visible in the "why" panel; a hospital without its running budget shows unstaffed beds; unfunded maintenance lowers speeds and output.

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
- [ ] Generator stages: ethnicity, language, religion (census tables), ideology, interest-group memberships, field of study.
- [ ] Education in depth: levels and fields, capacity from M11, quality from budget, returns in wages; student finance (grants, loans, repayment).
- [ ] Health and life expectancy, crime, wellbeing: driven by budgets, capacity, income, unemployment; feeding back into mortality, labour supply and approval.
- [ ] Approval in full: by issue and interest group, explained per group. Still no behavioural effect (that is the politics layer, after v1).
- [ ] Remaining transfers and levers: charity drives, non-EU immigration quotas.
- [ ] Group explorer in full; society screens.

**Exit:** for any filter of people the game shows how they live, what they pay and get, how much they approve and why; a rise in the health budget shows up in life expectancy over a decade.

### M15 — Informal economy
Spec: [informal-economy](02-design/economy/informal-economy.md). The split into declared and undeclared is in the starting state (M1), taxes use it from M3 and three enforcement levers are in the alpha (M4); this milestone is the behaviour and the rest of the tools.
- [ ] Undeclared work and envelope wages by industry and firm size; unreported sales and the VAT gap; the choice to evade as a rule of taxes, enforcement and penalties.
- [ ] Data: published estimates of undeclared work and the VAT gap.
- [ ] Detection and the alpha levers moved from M4 (owner, 2026-10-24): `informal.inspection_budget`, `informal.audit_budget`, `informal.penalty_multiple`, registered and planned; tests of AC-INF-01 to 09 first.
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
- [ ] Full trade between counties: goods moving along the transport network with transport costs, rather than one national market per good (owner, 2026-10-22).
- [ ] **Automation of parts of the game** (owner, 2026-10-22): let the player hand areas (for example the budget's details, infrastructure, energy) to an automatic minister and play only the parts they choose, as in Europa Universalis. To research: how other games do it, and what an automatic policy for each area should aim at.
- [ ] **Projections and policy previews** (owner, 2026-10-22): to think about and discuss. Projected GDP growth, inflation, unemployment and the deficit, and a preview when a lever moves ("VAT to 21%: the forecast is …"). Building one is easy: fork the `World` and run it ahead. The question is honesty. With the outside world constant (D8), a forked run is a perfect crystal ball that removes uncertainty and lets the player test every policy for free. Options: an imperfect forecaster (a simpler model, as a finance ministry forecasts, with a range, sometimes wrong); noise on a forked run (cheap, feels fake); or forked runs only once the outside world moves by itself, when they show a scenario rather than the future.
- [ ] **Re-anchoring the living-standard classes** (owner, 2026-10-26; to consider): over decades real growth makes nearly everyone "affluent" by 2021's lines, so consider letting the player re-anchor them to a later year, as Eurostat's anchored poverty rate is re-based.
- [ ] **Arrears as debt** (owner, 2026-10-26; to consider): the insolvency backstop (M4) records wages a firm cannot pay as lost to workers. Consider making them a debt the firm owes and repays first when cash returns (wage arrears, as in Romania and Russia in the 1990s), and unpaid suppliers as debts between firms that spread the crisis along supply chains (Romania's 1990s "blocaj financiar").
- [ ] **Anticipation of a falling currency** (owner, 2026-10-26): with the crisis mechanics (D10). Today firms and workers anticipate inflation (expected inflation in prices and the January wage round) and shift slowly to domestic goods as imports get dearer, but nobody expects the leu itself to keep falling. To add: importers pricing at the expected exchange rate; buying imported inputs ahead; households and firms moving savings into foreign currency (dollarisation, as in Romania's 1990s); investment freezing under uncertainty; banks cutting lending or lending only in euros. Several of these are self-fulfilling (selling lei pushes the leu down further), which is why they wait for the crisis mechanics and for calibration rather than going in before it.
- [ ] **Randomness in firms' decisions** (owner, 2026-10-22): to research and discuss; earlier, in M7, only if calibration needs it. Today only people's events are random (quits, hiring, layoffs; keyed RNG, so runs replay exactly); firms' prices, production and investment follow fixed rules, and every unit of an industry reacts alike. Two layers: **traits** drawn once per firm unit (speed of price changes, markup aggressiveness, investment appetite), which spread responses over time and smooth the aggregate; and a small **luck term** in each decision, mostly realism, shown as "other factors" in the "why" panel. Noise must not hide a bad rule: runs must make sense with it off, and with it on match Romania's volatility. Shocks to the whole economy stay with the moving outside world (D8).
- [ ] **Events** (owner, 2026-10-22): to think about and research. Events such as a bank run, either scripted or generated by an LLM, and whether they fit the "systems over scripts" pillar.

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
