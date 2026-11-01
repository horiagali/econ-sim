---
id: game/lever-system
title: Lever System — Commands, Registry, Replay and the Alpha Levers
status: draft
owner: horia
depends_on: [game/levers, adr/0018-scenario-and-commands, adr/0011-storage-saves-history, adr/0017-time-base, adr/0008-explainability-architecture, adr/0006-determinism-contract, economy/budget-2021, economy/fiscal-policy, economy/infrastructure, game/world-loop, game/scenario-builder]
research: [research/law-in-force-romania-2021]
updated: 2026-11-01
---

# Lever System — Commands, Registry, Replay and the Alpha Levers

## Purpose
How a decision of the player (or a line of a test file) becomes a change of the law in force on a date, how it is checked, logged, replayed and explained. It is the machinery of roadmap M4; [ADR-0018](../../03-architecture/decisions/0018-scenario-and-commands.md) fixed its shape (a registry, six value shapes, the `Command` struct, a JSON-lines log) and this spec fills in the rules. It adds no economic behaviour: each lever's effect is owned by the mechanic that reads it. The [catalogue](levers.md) stays the menu.

## Scope
**In M4:** the registry and its generator, validation and rejection messages, effective dates and phase-in, the command log and replay, headless play, world settings, the "why" of a lever change, the ministry budgets (the owner's decisions of 2026-10-22), and wiring every lever tagged `alpha` ([alpha levers](#the-alpha-levers)).
**Not in M4:** the app's lever panels (M8, built from the generated list); levers of later milestones (registered as `planned`, rejected until wired); a cost for changing a lever (D9); laws that need parliament.

## What a lever is
A lever is a named, typed setting of the law in force (or of the outside world, D8) that a mechanic reads. One registry entry describes it:

| Field | Meaning | Example |
|---|---|---|
| `id` | stable dotted name, never reused or renamed (ADR-0018) | `tax.vat.rate_standard` |
| `kind` | `law`, `market`, `project` or `world`: sets the earliest effective date (below) | `law` |
| `shape` | `rate`, `money`, `number`, `choice`, `flag` or `schedule` (ADR-0018) | `rate` |
| `unit` | display unit; the stored value is the shape's integer (basis points, bani, count) | `%` |
| `range` | inclusive bounds in the shape's integer; for `schedule` the bounds of each rate and the number of rows | `[0, 4000]` |
| `key` | for a lever set once per good, industry, budget line, public unit, county or bloc: the table of keys, and optionally the subset allowed | `budget_line: [education, health]` |
| `planned_keys` | optional, for an `active` keyed lever: keys registered but not wired yet; their commands are rejected with `not_yet` (added 2026-10-25) | `[gas, coal]` |
| `options` | for `choice`: the allowed values, in order | `[float, managed_float, peg]` |
| `min_lag` | an extra calendar delay beyond the kind's, if the law has one | `1 month` |
| `phase_in` | optional: the in-force value moves toward the commanded one by at most this step per month | `{ per_month = 1 }` (months of retirement age) |
| `spec`, `glossary` | the spec that owns the effect, the glossary name it sets | `economy/vat`, `vat_rate_standard` |
| `reader` | the `World` phase that reads it (one of `PhaseId`, [world-loop](world-loop.md)) | `Prices` |
| `group` | one of six groups for counterfactual attribution (ADR-0008 L4): `taxes`, `spending`, `transfers`, `money`, `trade`, `world` | `taxes` |
| `milestone` | the catalogue tag: `alpha`, `M9`, … | `alpha` |
| `status` | `active`, `planned` (registered, not wired: commands rejected) or `retired` | `active` |
| `label` | string-table key of its name and help text (D11) | `lever.tax.vat.rate_standard` |

**Default.** A lever has no default in the registry (one exception of behaviour, not of value: `money.policy_rate`, see "The alpha levers", group 8). Its starting value is the law in force on 1 December 2021 (D4), written by the scenario builder into `law.json` (or `world.json` for `kind = world`) as commands dated the start date; a lever missing there is a load error (ADR-0018). The builder reads the values from a pipeline fixture `python/pipeline/fixtures/law2021_ro.json`, keyed by lever id, filled from the [research note](../../01-research/notes/law-in-force-romania-2021.md) with its confidence marks; each value is checked against the legal text when it is entered (the M0 carry-over: pension tax, health contribution on other income, benefit floors).

**Effective date.** The day a command is given is $d$. The earliest day it can take effect:

| `kind` | Earliest effective day | Why |
|---|---|---|
| `law` | the first day of the month after $d$, then `min_lag` | law and budgets change on the 1st (ADR-0017 §3) |
| `market` | $d + 1$ | central-bank operations: policy rate, QE, FX intervention |
| `project` | $d + 1$: the project starts; its duration belongs to its mechanic | build, close (M11, M12) |
| `world` | $d + 1$ | a sandbox slider, not a law |

A command may name a later `effective` date; an earlier one is rejected. The **effective date** is when the law in force changes; the **response lag** (prices take weeks, investment years) is the reading mechanic's behaviour and is not a registry field. The catalogue's "Lag" column mixes the two; the registry keeps only the first.

**Phase-in.** A lever with `phase_in` does not jump: from its effective date, on the first of each month, the value in force moves one step toward the commanded value until it reaches it. The steps are computed when the command is applied, so they are deterministic and visible as pending changes. Alpha use: retirement age (if wired, see the table).

## The registry
**Source.** One file, `crates/econ-core/levers.toml` (ADR-0018), one `[[lever]]` table per lever with the fields above. It is the single list: the specs own each lever's effect and link to its id in their "Player levers" section, and a check verifies the two agree (below). Keyed levers name a key table that exists in the scenario (`industries`, `budget_lines`, `public_units`, `counties`, `blocs`, `fuels`), so keys are text ids, never indices.

```toml
[[lever]]
id = "tax.vat.rate_standard"
kind = "law"
shape = "rate"
unit = "%"
range = [0, 4000]
spec = "economy/vat"
glossary = "vat_rate_standard"
reader = "Prices"
group = "taxes"
milestone = "alpha"
status = "active"
label = "lever.tax.vat.rate_standard"
```

**Generator.** `scripts/codegen_levers.py`, run by `just codegen` beside the glossary's generator and checked by `just codegen-check`, writes:
1. `crates/econ-core/src/levers_gen.rs`: `enum LeverId`, each lever's metadata as a `const` table, and `struct LawInForce` with one typed field per lever (a `Vec` indexed by key for keyed levers) and `fn set(&mut self, cmd: &Command)`;
2. `ui/src/generated/levers.json`: the list the app builds its panels from (id, label, group, shape, unit, range, keys, milestone, status);
3. `crates/econ-core/tests/generated_lever_list.rs`: the list the coverage tests iterate (AC-LEV-02, AC-LEV-14);
4. the catalogue's "When each lever arrives" table, between marker comments in [levers](levers.md), once the registry holds every lever.

**Checks** (in `just check`): every entry parses and has the fields of its shape; ids are unique and dotted; `spec` is an id in [INDEX](../../INDEX.md); `glossary` is a glossary `code_name`; `reader` is a `PhaseId`; every `active` lever is set in the scenario's `law.json` or `world.json`; a `retired` lever is never deleted.

## Commands and validation
A command is ADR-0018's `Command { lever, key, value, effective }`, given on a day. The core validates it at once, against the registry, today's date and the pending list only (never against the economy's state), so validation is the same in the app, in headless play and in replay.

| Code | Rejected when | Message (string table, English, D11) |
|---|---|---|
| `unknown_lever` | the id is not in the registry | "There is no lever `{id}`." |
| `retired` | the lever is retired | "`{label}` was removed in version {v}." |
| `not_yet` | `status = planned` | "`{label}` arrives with {milestone}." |
| `wrong_shape` | the value is not of the lever's shape | "`{label}` takes {shape}, not {given}." |
| `out_of_range` | a value, a schedule rate or the row count is outside `range` | "`{label}` must be between {min} and {max}; you asked {value}." |
| `bad_key` | key missing on a keyed lever, given on an unkeyed one, unknown, or not in the allowed subset | "`{label}` has no `{key}`; choose one of {keys}." |
| `bad_option` | a `choice` not in `options` | "`{label}` can be {options}." |
| `bad_schedule` | thresholds not strictly ascending, or the first not 0 | "Brackets of `{label}` must start at 0 and rise." |
| `too_early` | `effective` before the earliest day of the kind and `min_lag` | "`{label}` can change from {earliest} at the soonest." |
| `nothing_to_cancel` | a cancel names a command that is not pending | "That change has already taken effect or does not exist." |

Values appear in display units in messages (`21%`, `2,550 lei`), converted from the integers by one formatter. A rejected command changes nothing and is not logged; the caller gets the code and the message.

**Warnings, not rejections.** A valid command that breaks an EU rule (VAT standard rate below 15%, a reduced rate below 5%) or an internal consistency the law usually keeps (a reduced VAT rate above the standard one) is accepted and logged with a warning event (L5), as ADR-0018 says.

**Order.** Pending commands are part of the state (saved, shown to the player). In the `Commands` phase at the start of each day, the commands due that day are applied in the order they were given (a sequence number); for one lever and key, the last given wins. A cancel is a command `{ cancel: seq }` and both stay in the log (ADR-0018). Applying changes `LawInForce` and nothing else; mechanics read typed fields of `LawInForce`, never strings.

## Logging and deterministic replay
**The command log** is JSON lines (ADR-0018 supersedes ADR-0011's postcard for the log): one line per accepted command or cancel.

```json
{"seq":17,"given":"2022-02-14","lever":"tax.vat.rate_standard","value":2100,"effective":"2022-03-01"}
{"seq":18,"given":"2022-02-14","lever":"spend.ministry_budget","key":"education","value":4200000000000,"effective":"2022-03-01"}
{"seq":19,"given":"2022-02-20","cancel":17}
```

- Values are the shape's integers (basis points, bani, counts; a `choice` as its option text; a `schedule` as `[[threshold_bani, rate_bp], …]`). The largest money values (a ministry's year, ~10¹³ bani) fit exactly in a JSON number below 2⁵³.
- `law.json` and `world.json` are the same lines with `given` and `effective` equal to the start date and no `seq`.
- **Replay** (ADR-0006, ADR-0011): the scenario (by hash), the seed and the log give the same state hash on every day. A save stores `LawInForce`, the pending list and the log; replay from a save needs the engine and content hashes to match, as ADR-0011 says.
- Every applied command emits an L5 event `PolicyChanged { lever, key, old, new, given, effective, seq }` (below).

## Headless play
```
econ-cli play --scenario DIR --commands FILE.jsonl --until 2026-12-31 [--seed S] [--out DIR] [--series a,b,…] [--daily]
just play FILE.jsonl            # the 1:1000 Romania scenario, seed 42, five years
```
- **In:** a scenario folder (ADR-0018) and a commands file in the form of the log; `seq` is optional (line order), `effective` is optional (the earliest allowed). The whole file is validated before the first day runs; any rejection stops with the line number and the message.
- **Out**, in `--out`: `series_monthly.arrow` and `.csv` (one row per month; columns are the monthly series the `World` reports until the statistics registry of M6 exists, then the registry's alpha indicators; units in the header), with `--daily` also the fast daily series of ADR-0017 §8; `commands_applied.jsonl` (the log, every line with `seq` and `effective` filled); `events.jsonl` (L5, warnings included); `run.json` (scenario hash, seed, engine hash, the last day, the state hash of each month end).
- **Round trip:** feeding `commands_applied.jsonl` back as the commands file gives byte-identical outputs (AC-LEV-08). This is how a bug report or a policy test is reproduced.
- The policy-scenario suite of M4 (rate hike, VAT rise, …) is one commands file per scenario plus a Rust test that runs it at three scales and checks direction, rough size and timing, in `crates/econ-core/tests/acceptance/policy/` (protected, written in a test-authoring session).

## World settings
World settings (D8) are levers with `kind = world`, ids under `world.`, `group = world`, values in `world.json`. They are validated, logged, replayed and explained exactly like law. Two differences: the app shows them in their own "World settings" panel, marked as sandbox controls; and a value given as growth a year (foreign prices, foreign demand) is converted to the period of the process that reads it by the `econ-num` helper (ADR-0017 §3), once, when the command is applied. The constants the M3 mechanics hold today (`FOREIGN_RATE_PPM`, `DEMAND_INDEX_PPM`, the world price factor) become fields of `LawInForce` read from `world.json`.

## Ministry budgets (owner decisions, 2026-10-22)
- **The education and health budgets are levers,** each in two parts: **running** (staff and inputs: the budget cells `wages`, `employer_contributions`, `goods_services`) and **investment** (projects and maintenance: `investment`, `non_produced_assets`), handled through the project and maintenance system of [infrastructure](../economy/infrastructure.md). **The split is itself a lever.** Levers: `spend.ministry_budget[line]` (money, bani a year) and `spend.investment_share[line]` (rate), keyed by [budget line](../economy/budget-2021.md); starting values from `budget.json` (the five cells' sum less the imputed employers' contributions, which budget-2021 does not post: D12 above the CAM's 2.25% of wages, 3,966 million lei on defence and 5,966 million on police and justice; investment over that sum; owner, 2026-10-29).
- **Teacher and doctor pay are their own lever,** `spend.public_pay[unit]` (rate: pay of the unit's employees as a share of their pay on the start date; 100% at the start), keyed by public unit. A larger budget does not raise pay; a pay rise does not raise the budget.
- **Unspent running money returns to the treasury at year end.**

**Rule** (monthly, on the 1st and at pay day; amounts by budget-2021's twelfths rule). The running allowance of the year to date is $R_l = \sum$ monthly twelfths of $A_l (1 - s_l)$, with $A_l$ = `ministry_budget[l]` and $s_l$ = `ministry_investment_share[l]` in force each month (a change restarts the twelfths from its month). The public unit is paid its wage bill (employees × pay) first, then its inputs at the line's starting mix, up to $R_l$ less what it has already been paid this year. The unpaid rest is never posted; on 31 December the year's allowance closes and the next year starts from zero, so "returns to the treasury" means it was never drawn, and the deficit is smaller by it. The investment part, $A_l s_l$ in twelfths, goes to the infrastructure spec's maintenance and projects for that ministry's assets (schools, hospitals).

> **Simplification (M4):** until the project system exists (M11), the investment part is spent each month as `gov.investment` purchases from the construction and equipment industries in the line's starting mix, and any of it unspent also lapses at year end. Quality effects of either part (education quality, health capacity) come with M14.

- **Every budget line has the split** (owner, 2026-10-22): administration, police and justice, culture and the rest have the same two levers as education and health, so police stations and courts are built from their line's investment part.

**Wage bill above the running allowance** (pay raised, budget not; owner, 2026-10-22): **a hiring freeze.** Wages already owed are always paid in full. Inputs are cut first, down to zero. While the wage bill still exceeds the allowance, the unit posts no vacancies, so staff who quit or retire are not replaced, and headcount falls until the bill fits. A warning event names the line. Any overrun left in the month (the bill above the allowance even with inputs at zero) is paid by the treasury and reported. Classes grow and wards thin as staff leave, through the [labour market](../economy/labor-market.md)'s quits.

**Public pay and the labour market** (owner, 2026-10-22): pay is % of the start, and it bites because hiring and quits respond to pay. A public unit paying less than similar jobs in its region fills vacancies more slowly and loses staff to better-paid jobs ([labour market](../economy/labor-market.md), "Pay and job choice"). Frozen pay while private wages rise therefore shows up as unfilled posts, rural ones first. The ratio of public to private pay by sector is an indicator.

**How the `World` carries it** (owner's decisions, 2026-10-23; in worlds built from `law.json`):
- **Lines and units.** Each line's COFOG subgroups belong to public units (the budget's `public_units[].cofog`); eleven lines fall in one unit, `social_protection` and `research` in several. A line's 2021 wage cell (D11) is split over its units by their own D11 totals. A unit's wage bill is shared among its lines in proportion to those parts, and a line's inputs and investment go to its units in the same proportion. Eight lines share `public_administration.public`.
- **Public units are the government's agents.** At each month end, right after wages, each line takes its share of its units' labour cost (gross wages and the employer's contribution), then the rest of its running allowance of the year to date as inputs. The government pays each unit the sum under `gov.purchases` and the line's investment twelfth under `gov.investment`. A wage bill above what is left is paid in full, reported as overrun, with a `BudgetOverrun` event. The unit's output is non-market output valued at what the government paid (the at-cost rule of [budget-2021](../economy/budget-2021.md)).
- **What the units buy.** Over the next month, each unit spends its input money day by day in its own industry's input structure in the input-output table (imports included), and its investment money in the mix of the investment column. Rationing can leave money unspent; it stays with the unit for later days. *(2026-10-23, implementation: the owner's choice was the government column's mix; on the `World` it put about 120 billion lei a year into industries that sell the government 46 billion, rationed households there and sank output by half within eight months. A public producer's inputs are its industry's, so that mix is used.)*
- **Market sales.** A public unit still produces and sells to households and other industries (health and education fees, intermediate services), planning that output from its sales as any unit; the government no longer buys its output. *(2026-10-23, implementation: with no production, the industries that hold its services as essential inputs ran out within months.)*
  **Its sales are the government's** (owner, 2026-10-29): at each month end, after the government's payment, each public unit's market sales of the month (never more than its deposits) go to the government under **1006 `gov.sales`** (ESA's sales of non-market producers, P11 + P12 + P131: 26.5 billion lei in 2021), so they lower the net cost of the lines. Its non-market output is valued at what the government paid less those sales (ESA's P13), so GDP does not count the market output twice. Where public units sell beside other units of their industry, **the other units sell first**, as far as their stocks go, and the public units serve the rest (the public units' share of their industries' market, owner, 2026-10-29). In the industry's prices a public unit counts as used at the normal rate and stocked at its target, and its non-market output (what the government paid, in real units at the day's price) counts in the industry's output for the wage cost per unit: it staffs for the non-market output, so its capacity and stocks say nothing of the market. *(2026-10-29, implementation: without this the wage cost of public administration and defence rose elevenfold within a year, their market output being a tenth of their staff's, and their sales with it.)*
  Measured (1:1000, seed 42, 2022): 42.4 billion lei of sales (health 18.5, education 15.3, administration 6.4, defence 1.4, social care 1.4), against ESA's 26.5. The other units of health and education are at their capacity (10.9 and 0.9 billion lei of output a year against markets of about 32 and 17 billion), so the public units serve most of those markets; the sales come down toward ESA's only as the other units invest.
- **Staffing.** Its staff target is its starting staff times `spend.public_headcount` over the starting posts. While its monthly labour cost exceeds its lines' running allowance for the month, it posts no vacancies (the hiring freeze). A lower target lays staff off through the labour market's layoff rule.
- **What it replaces.** The government's fixed purchases of the input-output table's government column where a public unit makes the good, their imports, and its share of the investment column. Public units pay no profit tax. *(Restated 2026-11-01, M7 calibration under the owner's delegation: the column's purchases from industries with no public unit, about 45 billion lei a year (road freight 7.7, culture 6.8, water and waste 4.7, post 3.8, passenger transport 3.6, …: the table's government consumption of local public services classified there), stay at the table's real level and post under **1007 `gov.purchases_fixed`**, since no line pays them. Stopping them left those industries without a buyer: they shrank by 16% to 45%, and unemployment ran about 2 points higher. With them the 2021-law World's spending is about 45 billion above the 2021 budget, which offsets its revenue, about 43 billion above 2021's (roadmap M7), so its deficit is close to 2021's 7.1% (6.7% in the calibration report). They become a budget line with the budget's later work.)*
- **Extreme spending.** The treasury's bond issue is bought out of the banks' reserves by every buyer but the central bank; when an issue would take more than the reserves hold, it is scaled down and the government's deposits stay below the target (an extreme ministry budget or pension point ends in a bad economy, not a crash).

> **Simplification (M4):** a line's inputs are fixed at the month end for the next month (paid then, spent over the month), so in the first month the units have no input money; the input mix is the unit's industry's, not the line's. *(Until 2026-10-29: a public unit's surplus on its market sales stayed with it, about 76 billion lei a year of deposits.)*

**Benefits in kind and the pensions' residual** (owner, 2026-10-29; worlds built from `law.json`): at each month end a twelfth of the budget's benefits in kind (D632, 8,342.7 million lei a year) is paid to the units that deliver them under **4009 `benefit.in_kind`**, the programme `health_in_kind` to the other units of `health_services` and the rest to those of `social_care`, by their staff at load; and a twelfth of the old-age and survivors' cash the pension rule does not pay (the budget's 112,337 million lei less twelve times the pension bill at load: the special pensions of budget-2021's imputed contributions, about 7.7 billion) to the pensioners' households, the same per pensioner, into deposits, under 4001 `benefit.pension`.

> **Simplification:** benefits in kind are money to the delivering units, not goods to households; the residual is fixed nominal and equal per pensioner (the special pensions are not in the pension file, so who gets them is unknown).

## The tax levers on the `World` (implementation, 2026-10-25)
In worlds built from `law.json`; M3's `Law::law_2021()` keeps its own rules (no tax on deposit interest, the fixed property total), so worlds without `law.json` and their hashes are unchanged.
- **Dividends and interest.** `tax.capital.dividend_rate` is the rate withheld on households' dividends (it was a 5% constant). `tax.capital.interest_rate` is withheld by the banks on households' deposit interest at each month end (10% in 2021; firms' and other depositors' interest is not taxed); the household receives the net, and the banks pay the tax from their reserves to the government's account under 3010 `tax.interest` (the code taxation proposed).
- **Contribution ceiling.** `tax.contrib.ceiling` caps the declared wage on which CAS (Pillar II share included) and CASS are due, the industry override's part first; CAM stays on the whole declared wage; income tax is then computed on the wage less the capped contributions. 0 = none.
- **Child tax credit.** `tax.income.credit_per_child` times the household's children under 18, to the earner who claims the household's dependants, taken off that wage's income tax, never below zero (not refundable).
- **Property tax.** `tax.property.rate` a year on each owner household's dwelling value (the median owner's when it has none recorded), a twelfth a month at the month end (remainders to the first months), none paying more than its deposits; the rest falls short and is reported as before. Calibrated on the `World`: owner households' dwellings are worth 1,157.4 billion lei (weighted, the same at 1:1000 and 1:100), so 2021's 3,864.1 million is 0.334%; the lever holds whole basis points, and 0.33% gives 3,819.3 million a year (1.2% under 2021).
- **Wealth tax.** `tax.wealth.rate` a year on each household's net worth (cash, deposits and dwelling value less its mortgage and consumer loans) above `tax.wealth.threshold`, a twelfth a month at the month end, none paying more than its deposits (the rest forgiven); under 3016 `tax.wealth`. None in 2021.
- **Excise.** `tax.excise.duty[good]` in bani per 1,000 units is ten times the duty in millionths of the starting price per unit (51,250 → 512,500 for fuel). A change replaces that good's indexed duty on its effective day; the others keep theirs.
- **Extreme levers.** The central bank's sale of foreign currency at the month end is capped by the banks' reserves (an excise duty at the top of its range drained them below zero).

> **Simplification (M4):** at the top of its range, `tax.contrib.ceiling` (100,000 lei a month) is above every wage of the 1:1000 sample, and `tax.wealth.threshold` does nothing while `tax.wealth.rate` is 0, so neither end moves the state hash in AC-LEV-14's three months: both are read, but only bite with a wage above the ceiling or a wealth tax. AC-LEV-14 moves them under the conditions of [the alpha levers](#the-alpha-levers).

## The money, FX, world, bond and pension levers on the `World` (implementation, 2026-10-25)
In worlds built from `law.json`, like the tax levers; worlds without it are unchanged.
- **Money (group 8).** `money.reserve_requirement` and `money.inflation_target` set the reserve requirement and the target at load and whenever a command takes effect. `money.policy_rate` sets the policy rate on the start date (1.75%); from the first monthly close the central bank follows its advisory rule ([monetary policy](../economy/monetary-policy.md), "Control"), until a `money.policy_rate` command that changes the rate in force takes effect, whose rate holds from its effective date and for the rest of the run (owner, 2026-10-29; a command that leaves the rate as it is changes nothing, AC-LEV-09). At each month end, before the treasury's issue, the central bank buys `money.monetary_financing` of the new bonds and `money.qe` of the banks' bonds (a negative amount sells its own, never more than the banks' reserves pay for), against reserves.
- **FX (group 9).** `fx.regime` sets the share of the exchange rate's monthly move the central bank offsets: none for `float`, trade's parameter for `managed_float` (kept from load), all of it for `peg`. `fx.intervention` adds the player's purchase (> 0) or sale of foreign assets to the rule's; a sale never takes reserves below the floor, nor more than the banks' reserves pay for. `fx.devaluation` (added 2026-10-25, owner's decision) steps the exchange rate once on its effective day by its rate (1000 = the leu 10% weaker), and the managed float continues from the new level ([trade & FX](../economy/trade-fx.md), amendment of 2026-10-25); every command that takes effect is one step, and the value in force only records the latest.
- **World settings (group 10).** `world.foreign_rate` replaces trade's constant foreign rate; `world.foreign_price_growth` and `world.foreign_demand_growth` grow each good's euro price and foreign demand every month end, at the blocs' mean rate a year converted once to a month. `world.fuel_price[oil]` (wired 2026-10-25, owner's decision) scales the euro price of `crude_oil` and `refined_petroleum` by the new value over the old on its effective day, so the two goods' euro prices are their growth path times the oil price over its starting value; gas and coal are `planned_keys` until the energy mechanic (M12).
- **Bond maturity (group 11).** `spend.new_issue_years` is the maturity of the treasury's issues from its effective day; bonds already issued keep theirs.
- **Pension indexation (group 11).** At the wage round (1 January) the pension point in force grows by the twelve-month inflation of the December close (`prices`), by the round's mean wage growth across industry groups (`wages`), or by their mean (`mix`); `none` keeps it frozen, as in 2021. Neither rate lowers the point. The new point becomes the value of `transfer.pension.point_value` in force (no command is logged; a replay recomputes it), so a later command on the point replaces it.

> **Simplification (M4):** the player's FX intervention moves money and reserves; its effect on the rate itself waits for calibration (M7). One world price per good, so the two blocs' growth rates are averaged. The `wages` indexation uses the industry groups' mean growth, unweighted by employment.

## How a lever change is explained
Built on the five layers of [ADR-0008](../../03-architecture/decisions/0008-explainability-architecture.md):
- **L5 event.** `PolicyChanged` when a command takes effect, `PolicyGiven` when it is accepted (pending), `PolicyWarning` for EU and consistency warnings. Charts mark the effective date with the lever's label.
- **L2 driver terms name their lever.** A `behaviour_rule!` term whose input is a field of `LawInForce` declares the lever id as its source, so the "why" tree of an indicator shows, for example, "VAT standard rate 19% → 21% (1 March 2022): +1.4% on the price of cars". The macro's `META` gains an optional `lever` per term; the generator checks the id exists.
- **Built (2026-10-26): the price "why" terms** (`World::price_why(industry, month)`, AC-LEV-15). The world marks the parts of every good's consumer price `c = (p + d)(1 + v)·s` at load and at each monthly close (producer price `p`, excise duty `d`, the VAT schedule and the good's category; an observation, outside the state hash). A month's change of `c` (close of the month before, or load, to close of the month; millionths of the starting price) is split into five terms by moving one part at a time from its old to its new value, the others at the values already moved, so the terms telescope and add up exactly, in integers, to the change: producer price (no lever), excise duty (`tax.excise.duty` when the law's duty changed, else the January indexation, no lever), the VAT rate of the good's old category (`tax.vat.rate_standard`, `…rate_reduced` or `…rate_reduced_2`), a change of category (`tax.vat.category`), and other (the rest of the held price's change: rounding). Example, the 19% → 21% rise of 1 January 2022 on cars at 1:1000: producer price −2,150, VAT rate +10,744, the others 0; the held price went from 952,852 to 961,446 (+8,594).
  > **Simplification:** the terms are a plain ordered decomposition, not yet `behaviour_rule!` terms with a declared `lever` per term; the producer price is one term (its unit-cost and markup parts are not split, because the price only steps part of the way to its target each day); the order (producer, excise, VAT) charges the VAT change on the new base.
- **L1 flow codes** show where the money moved (VAT revenue up, household spending by good).
- **The lever's own page** in the panel shows: the value in force, the value on 1 December 2021, pending changes with their dates, the mechanic that reads it (`reader`, `spec`) and the indicators that mechanic moves.
- **L4 counterfactuals** use the registry's `group` as the lever groups (six, ADR-0008's maximum): "your tax changes vs the oil price" forks runs with and without each group's commands since a date.

## The alpha levers
Levers tagged `alpha` in the [catalogue](levers.md) (D2). Ids are proposals for the registry; "Read today" is a check of `crates/` on 2026-10-22: **yes** = a field of the law the `World` reads (`Law`, `IncomeTaxLaw`, `TransferLaw`); **param** = a parameter or constant that exists but is not law yet (fixed at load); **gap** = no mechanic reads such a value.

| Proposed id | Shape | 1 Dec 2021 | Reader (spec) | Read today |
|---|---|---|---|---|
| `tax.income.schedule` | schedule | flat 10% | pay day ([taxation](../economy/taxation.md)) | yes (`LabourTaxes.income`) |
| `tax.income.deduction_base[dependants]`, `tax.income.deduction_full_until`, `tax.income.deduction_zero_at` | money | 510–1,310; 1,950; 3,600 lei | pay day (taxation) | yes (`DeductionRule`) |
| `tax.income.credit_per_child` | money | 0 | pay day (taxation) | **gap** |
| `tax.capital.dividend_rate`, `tax.capital.interest_rate` | rate | 5%, 10% | (taxation) | **gap**: no tax on dividends or interest |
| `tax.profit.rate`, `tax.profit.micro_rate_staff`, `tax.profit.micro_rate_no_staff`, `tax.profit.micro_threshold` | rate, money | 16%, 1%, 3%, 1 m euro | quarterly tax (taxation) | yes (`ProfitTaxes`) |
| `tax.vat.rate_standard`, `…rate_reduced`, `…rate_reduced_2` | rate | 19%, 9%, 5% | prices ([vat](../economy/vat.md)) | yes (`Law.vat`) |
| `tax.vat.category[industry]` | choice | per good | prices (vat) | yes (`Law.vat_category`) |
| `tax.contrib.cas`, `…pillar2_share`, `…cass`, `…cam` | rate | 25%, 3.75 pts, 10%, 2.25% | pay day (taxation) | yes (`LabourTaxes`) |
| `tax.contrib.ceiling` | money | none (0 = none) | pay day (taxation) | **gap** (only the industry overrides have a ceiling) |
| `tax.property.rate` | rate a year | 0.33% (national stand-in, calibrated to 2021's revenue) | revenue ([taxation](../economy/taxation.md)) | **param**: a fixed yearly total (3.86 bn lei) shared by owners, no rate |
| `tax.wealth.threshold`, `tax.wealth.rate` | money, rate | none, 0% | (taxation) | **gap** |
| `tax.excise.duty[good]` (fuel, tobacco, alcohol) | money per unit | the 2021 duties | goods market ([excise](../economy/excise.md)) | **param**: constants in `world/revenue.rs`, indexed each January |
| `spend.ministry_budget[line]` | money a year | `budget.json` | public units ([fiscal policy](../economy/fiscal-policy.md)) | **gap**: purchases follow the input-output table; `budget.json` is loaded but not paid |
| `spend.investment_share[education, health]` | rate | from `budget.json` | public units, [infrastructure](../economy/infrastructure.md) | **gap** |
| `spend.public_pay[unit]` | rate | 100% | pay day ([budget-2021](../economy/budget-2021.md)) | **gap** |
| `spend.public_headcount[unit]` | number | `public_posts[f]` | staffing ([labour market](../economy/labor-market.md)) | **gap**: no public hiring rule |
| `spend.new_issue_years` | number | 7 | month end (fiscal policy) | **param** (`DebtParams`) |
| `transfer.pension.point_value`, `…minimum` | money | 1,442, 800 lei | pay day ([social transfers](../economy/social-transfers.md)) | yes (`PensionRule`) |
| `transfer.pension.retirement_age[sex]` | number (months) | 780; ~741 | demographics (M5) | **gap**: no retirement transition yet |
| `transfer.pension.indexation` | choice | none (frozen) | yearly (social transfers) | **gap** |
| `transfer.unemployment.isr`, `…isr_share`, `…wage_share_by_years`, `…months_by_years` | money, rate, schedule | 500 lei, 75%, … | pay day (social transfers) | yes (`UnemploymentRule`) |
| `transfer.child.under2`, `transfer.child.to18` | money | 427, 214 lei | pay day (social transfers) | yes (`ChildRule`) |
| `transfer.min_income.floor[size]`, `…per_extra` | money | 142 lei for one, … | pay day (social transfers) | yes (`MinimumIncomeRule`) |
| `labour.minimum_wage` | money | 2,300 lei | wage round ([labour market](../economy/labor-market.md)) | yes (`Law.minimum_wage`) |
| `money.inflation_target` | rate | 2.5% | central bank ([monetary policy](../economy/monetary-policy.md)) | **param**: constant at load |
| `money.policy_rate` | rate | 1.75% | central bank, banks | yes: the start date's rate; the advisory rule sets it from the first close until a command overrides it (owner, 2026-10-29) |
| `money.reserve_requirement` | rate | 8% | banks ([money & banking](../economy/money-banking.md)) | **param** (`BankParams`) |
| `money.qe` | money a month (signed) | 0 | central bank (monetary policy) | **gap** |
| `money.monetary_financing` | money a month | 0 | central bank, treasury | **param**: measured, not chosen |
| `fx.regime` | choice | managed float | FX close ([trade & FX](../economy/trade-fx.md)) | **param**: managed float only (`intervention_share`) |
| `fx.intervention` | money a month (signed) | 0 beyond the rule | FX close (trade & FX) | **param**: the rule's share, no player amount |
| `fx.devaluation` | rate (signed, one-off) | 0 | the exchange rate on its effective day (trade & FX) | added 2026-10-25 (owner) |
| `informal.inspection_budget`, `informal.audit_budget` | money a year | 2021 spending | detection ([informal economy](../economy/informal-economy.md)) | **gap** |
| `informal.penalty_multiple` | number (tenths) | ~1× | detection (informal economy) | **gap** |
| `world.foreign_price_growth[bloc]` | rate a year | 2% | trade (trade & FX) | **param**: a constant level, no growth |
| `world.fuel_price[fuel]` | money per unit | Dec 2021 prices | goods market, [energy](../economy/energy.md) | oil: the euro import price of crude and refined petroleum (2026-10-25); gas, coal: **gap** until M12 |
| `world.foreign_demand_growth[bloc]` | rate a year | 0 | trade | **param** (`DEMAND_INDEX_PPM`) |
| `world.foreign_rate` | rate | −0.5% | FX close | **param** (`FOREIGN_RATE_PPM`) |
| `world.wages_abroad` | rate (vs Romania) | 100% of start | emigration (M5, [demographics](../economy/demographics.md)) | **gap** |

**Deferred beyond M4** (owner, 2026-10-24, [roadmap](../../roadmap.md) M4): registered `planned`, so a command on them is rejected with `not_yet` until their mechanic wires them.

| Lever | Wired with |
|---|---|
| `transfer.pension.retirement_age[sex]` | M5 (retirement transition, [demographics](../economy/demographics.md)) |
| `world.wages_abroad` | M5 (emigration) |
| `informal.inspection_budget`, `informal.audit_budget`, `informal.penalty_multiple` | M15 (detection, [informal economy](../economy/informal-economy.md)) |

`world.fuel_price[fuel]` left this table on 2026-10-26: the lever is `active` with its oil key wired (owner, 2026-10-25); its gas and coal keys are `planned_keys`, rejected with `not_yet` until M12 (the [energy](../economy/energy.md) mechanic).

**Ends that bind only under a condition** (AC-LEV-14, 2026-10-25). Six ends of active levers cannot bind in the starting state, by the rules of the mechanics that read them; AC-LEV-14 moves those levers against a baseline that holds the condition under which they bind (orders given on the start date to both runs):

| Lever | Why an end does not bind at the start | Condition in both runs |
|---|---|---|
| `tax.contrib.ceiling` | top: above every wage of the 1:1000 sample (the start is 0, none) | a ceiling of 6,000 lei a month (about the mean wage) |
| `tax.wealth.threshold` | top: no wealth tax at the start (rate 0) | `tax.wealth.rate` 1% a year |
| `transfer.pension.minimum` | bottom: stage G puts no pension below the minimum ([population-pensions](../society/population-pensions.md), AC-POPP-01), so the points give every pension at least 800 lei and the minimum never lifts one | a minimum pension of 1,000 lei |
| `transfer.min_income.per_extra` | bottom: no household of six or more qualifies for the minimum income at the 2021 floors | the floor of five members at 10,000 lei |
| `labour.minimum_wage` | bottom: the minimum only lifts wages below it; no wage is below 2,300 lei at the start and the January round raises every group's wages | the minimum wage at 2,550 lei (the law of January 2022) |
| `money.qe` | bottom: a sale of the central bank's bonds, and it holds none at the start | QE of 1 billion lei in December, nothing from 1 January (the end's effective day) |

## Edge cases
- Two commands for one lever and key, effective the same day: the later given wins; both logged.
- A command given on the last day of a month (`law`) takes effect the next day, the 1st.
- A ministry budget cut below what the year has already paid: the allowance left is zero for the rest of the year; nothing is clawed back.
- A schedule with one row is a flat rate; a schedule may not be empty.
- A save holding pending commands of a lever later retired loads, applies them and warns.
- Moving any lever to either end of its range must not crash or break an invariant (D10); backstops are M10's.

## Acceptance tests
Written on 2026-10-22 by the test writer (ADR-0014) in `crates/econ-core/tests/acceptance/` (`levers.rs` for the `[unit]` criteria, `levers_sim.rs` for the `[sim]` ones), behind the feature `levers`, against the API skeleton and the test writer's additions (`World::with_law`, `registry`, `events`, `budget_months`, `price_why`, `attribute_groups`; `econ_io::commands::read_scenario_law`, `save_world`, `load_world`; `econ-cli play`). Until the generator writes `generated_lever_list.rs`, the coverage tests iterate the committed registry.

- [ ] **AC-LEV-01** `[unit]` The registry checks pass on the committed `levers.toml`, and each kind of fault (duplicate id, unknown shape, `spec` not in the index, `glossary` not in the glossary, unknown `reader`, missing key table) fails with the entry named; `just codegen-check` fails when the generated files are stale.
- [ ] **AC-LEV-02** `[unit]` Every `active` lever is set exactly once in the Romania scenario's `law.json` or `world.json`; a scenario missing one fails to load, naming it.
- [ ] **AC-LEV-03** `[unit]` Each rejection code of the table is returned for a command built to trigger it, with an English message that names the lever's label and, for `out_of_range`, the bounds and the value in display units; state and log are unchanged. *(2026-10-25: `not_yet` is shown on a lever the registry holds as `planned` when the test runs; the test named `money.policy_rate`, which M4 group 8 wired.)*
- [ ] **AC-LEV-04** `[unit]` Effective dates: a `law` command given on any day of a month takes effect on the 1st of the next month (plus `min_lag`); `market` and `world` the next day; a later requested date is kept; an earlier one is `too_early`.
- [ ] **AC-LEV-05** `[sim]` A command changes the law in force on its effective day and not before: until the day before, the daily state hashes equal those of a run without it.
- [ ] **AC-LEV-06** `[sim]` Replay: a year at 1:1000 with twenty commands (a cancel and two same-day changes of one lever included), replayed from the scenario, seed and log, gives the same state hash every day.
- [ ] **AC-LEV-07** `[sim]` Saving with commands pending, loading and running on gives the same daily hashes as the uninterrupted run.
- [ ] **AC-LEV-08** `[sim]` `econ-cli play` writes the series, the applied log, the events and `run.json`; feeding the applied log back gives byte-identical outputs.
- [ ] **AC-LEV-09** `[sim]` Setting every active lever to its starting value on the second day gives the same state hashes as no commands.
- [ ] **AC-LEV-10** `[unit]` A VAT standard rate of 14% is accepted with a `PolicyWarning` (EU floor), not rejected.
- [ ] **AC-LEV-11** `[sim]` Raising the education budget with pay unchanged raises education's input purchases and leaves its employees' pay unchanged; raising education's pay with the budget unchanged raises its wage bill and lowers its input purchases by the same amount, until inputs reach zero.
- [ ] **AC-LEV-12** `[sim]` With an education budget above what the unit can spend, the year's government spending on education equals what was paid; no posting moves the unspent part, and on 1 January the allowance restarts from the new year's twelfths.
- [ ] **AC-LEV-13** `[sim]` An investment share of 0 puts the whole health budget into running; a share of 100% sends all of it to investment purchases, wages are still paid and an overrun warning is emitted.
- [ ] **AC-LEV-14** `[sim]` For every alpha lever with `status = "active"`: moving it to the bottom and the top of its range for three months changes the state hash against the baseline (it is read) and breaks no invariant I-1 to I-8; a lever with an end that cannot bind in the starting state is moved against a baseline holding the condition under which it binds ([the alpha levers](#the-alpha-levers)). For a keyed lever with `planned_keys`, its active keys are moved to each end, and a command changing a planned key is rejected with `not_yet`. Every alpha lever still `planned` is one the owner deferred, and a command at either end of its range is rejected with `not_yet`. *(Restated 2026-10-25: it covered every alpha lever; the owner deferred six beyond M4 on 2026-10-24, and six ends of active levers cannot bind in the starting state. Restated 2026-10-26: `world.fuel_price` is active with only oil wired, gas and coal `planned_keys` until M12 (owner, 2026-10-25), so moving all its keys was rejected `not_yet`.)*
- [ ] **AC-LEV-15** `[sim]` After a VAT rise, `PolicyChanged` is in the events with old, new, given and effective, and the price "why" tree of a standard-rated good in the month of effect has a term tagged `tax.vat.rate_standard` with a non-zero contribution.
- [ ] **AC-LEV-16** `[sim]` A change of `world.foreign_rate` is validated, logged and replayed like a law command, and in a two-group counterfactual (taxes, world) its effect is attributed to `world`.

## API sketch
```rust
// crate econ-core; levers_gen.rs is generated from levers.toml
pub enum LeverValue { Rate(i32 /* bp */), Money(Bani), Number(i64), Choice(u16), Flag(bool), Schedule(Vec<(Bani, i32)>) }
pub struct Command { pub lever: LeverId, pub key: Option<u32>, pub value: LeverValue, pub effective: Option<Date> }
pub enum Order { Set(Command), Cancel { seq: u64 } }
pub struct Rejection { pub code: RejectCode, pub lever: Option<LeverId>, pub message: String }
pub struct Accepted { pub seq: u64, pub effective: Date, pub warnings: Vec<Warning> }
impl World {
    pub fn give(&mut self, order: Order) -> Result<Accepted, Rejection>;     // today = the current day
    pub fn law(&self) -> &LawInForce;
    pub fn pending(&self) -> &[Pending];
    pub fn command_log(&self) -> &[LogLine];
}
pub fn read_commands(path: &Path, registry: &Registry) -> Result<Vec<LogLine>, (usize, Rejection)>; // econ-io
```
The hand-written rule structs (`LabourTaxes`, `PensionRule`, `VatSchedule`, …) are filled from `LawInForce` by small adapters; `Law::law_2021()` is replaced by loading `law.json`.

## Open questions
- [x] **Registry source:** one `levers.toml` (ADR-0018) checked against the specs, or a block in each spec gathered by the generator? Proposed: the TOML, as ADR-0018 decided; a spec lists its lever ids, and the check of the registry verifies them. Locked specs are not edited to add blocks. Decided as proposed (owner's delegation, 2026-10-22).
- [x] **Values in files a person writes:** integers only, or also `"21%"` and `"2,550 lei"`? Proposed: the commands file accepts both, parsed exactly; the applied log always writes integers. Decided as proposed (owner's delegation, 2026-10-22).
- [x] **Form of public pay:** % of the start (proposed), or the catalogue's "% vs private equivalent"? Proposed: % of the start, so pay does not drift with private wages; the ratio to private pay is shown as an indicator. **Decided (owner, 2026-10-22):** % of the start, on condition that hiring and quits respond to relative pay, added to the labour market for M4, so underpaid public jobs go unfilled.
- [x] **Wage bill above the running allowance:** proposed: pay in full, inputs to zero, the treasury covers the overrun with a warning. Alternatives: a hiring freeze until it fits, or arrears. **Decided (owner, 2026-10-22):** a hiring freeze (see "Ministry budgets").
- [x] **The running/investment split for the other lines** (administration, police and justice, …): proposed yes, the same two levers for every line from M4, since the rule is the same. **Decided (owner, 2026-10-22):** yes, every line.
- [x] **Unspent investment money:** proposed: lapses at year end in M4 like running money; the infrastructure spec decides whether multi-year projects carry their money over (M11). Decided as proposed (owner's delegation, 2026-10-22).
- [x] **Budget year:** proposed: the calendar year; the first year is December 2021 alone, with a twelfth of 2021's amounts. Decided as proposed (owner's delegation, 2026-10-22).
- [x] **Property tax:** proposed: a national rate on dwelling value replacing the fixed total, calibrated so 2021's revenue is reproduced. **Decided (owner, 2026-10-22):** as proposed.
- [x] **QE and FX intervention:** proposed: signed amounts a month (buy > 0); the peg of `fx.regime` = intervention share 100% while reserves stay above the floor. Decided as proposed (owner's delegation, 2026-10-22).
- [x] **Penalty multiple:** proposed: shape `number` in tenths (10 = 1×) rather than a new shape. Decided as proposed (owner's delegation, 2026-10-22).
- [ ] **Handing the policy rate back to the rule:** once a `money.policy_rate` command has taken effect the rule no longer sets the rate for the rest of the run. Should the player be able to hand control back (a command value such as "rule", or a `money.cb_independence` choice), and does credibility reward it? Owner's decision (2026-10-29).
- [ ] **Legal amounts in `law2021_ro.json`** (pension tax threshold, health contribution on other income, benefit floors): to be checked against the legal text by the owner or a research pass when the fixture is filled (carried from M0).
- [x] **Errors in a commands file:** proposed: the whole file is validated first and the first error stops the run, with its line number. Decided as proposed (owner's delegation, 2026-10-22).
