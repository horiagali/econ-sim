---
id: adr/0018-scenario-and-commands
title: "ADR-0018: Scenario files and player commands"
status: accepted
owner: horia
depends_on: [adr/0005-simulation-core-architecture, adr/0006-determinism-contract, adr/0011-storage-saves-history, adr/0012-data-pipeline-and-licensing, adr/0017-time-base, game/levers]
updated: 2026-10-10
---

# ADR-0018: Scenario files and player commands

## Context
Two things are needed before the real simulation loop is written (roadmap, D7):
- **A scenario:** everything the world is at day 0. Today the pieces exist apart: the population tables that `econ-cli synth-population` writes, parameters hardcoded in the spikes, no law in force, no balance sheets.
- **A command:** how a change of a lever reaches the core. Today each spike has its own small enum (`Command::SetTaxRate`, `ScaleCommand::SetSeparationRate`) and the save keeps a log of them ([ADR-0011](0011-storage-saves-history.md)). The lever catalogue has about 150 levers; v1 also has the world settings that stand in for the outside world (roadmap, D8).

Whatever is chosen has to hold for the determinism contract (same scenario, seed and command log give the same state, [ADR-0006](0006-determinism-contract.md)), for replay after an engine upgrade, and for a UI and tests that must list every lever without a hand-kept copy.

## Options considered
**Scenario**
1. **A folder (or ZIP) of tables and small JSON files, built once per `sample_scale`.** Fast to load, inspectable with any Arrow tool, same shape as a save.
2. **Only the margins and parameters; the population is generated on load.** Smaller files, one scenario for every scale; but loading 1:10 takes minutes and a change in the generator silently changes every scenario.
3. **One binary blob.** Fastest; not inspectable, hard to diff.

**Commands**
1. **One Rust enum variant per lever, written by hand.** Type-safe; 150 variants to keep in step with the specs, the UI and the tests by hand.
2. **A lever registry (one data file) from which the Rust types, the UI's lever list and the test list are generated; a command is (lever, value, date).** One source of truth; needs a generator and a small set of value shapes.
3. **Free-form (string key, JSON value) checked at run time.** No generator; errors surface late and replay breaks silently when a name changes.

## Decision
Accepted by the owner on 2026-10-10.

**Chosen: scenario option 1, command option 2.**

### 1. Scenario
A scenario is a folder, or the same folder as a ZIP with uncompressed entries, for **one** `sample_scale`:

| Part | Content |
|---|---|
| `scenario.json` | format version, name, start date, `sample_scale`, default `rng_seed`, the engine and generator versions that built it, a hash of every other part, the provenance list ([ADR-0012](0012-data-pipeline-and-licensing.md)) |
| `tables/*.arrow` | the column tables of `World` at day 0: households, persons, firm units, industries and the input-output table, banks, public assets; with units and stock or flow kinds in the schema metadata, as in saves |
| `balances.arrow` | the opening balance sheet: every account of the ledger, summing to zero by instrument |
| `law.json` | the law in force at the start date: the starting value of **every** lever, written as commands with the start date (see 2). A lever missing here is an error, not a default |
| `world.json` | the starting value of every world setting (roadmap D8), in the same form |
| `params.json` | behavioural parameters from a calibration release ([ADR-0009](0009-calibration-and-stability.md)), with the release's id |

- A scenario is input and is never written by the simulation. A save is the scenario's hash, the state and the command log ([ADR-0011](0011-storage-saves-history.md)); it embeds a copy of `law.json`, `world.json` and `params.json` so that it stays loadable if the scenario file is lost.
- `econ-cli build-scenario --sample-scale N --seed S --out DIR` builds it from the pipeline's outputs: it runs the population generator and its stages, the later builders of firms and balance sheets, and writes a consistency report (every stock has a counterpart; totals against the published aggregates). It refuses to write a scenario whose opening balance sheet fails the ledger invariants.
- The same data at 1:1000, 1:100 and 1:10 are three scenarios. CI builds the 1:1000 one from the committed fixtures.
- Loading checks the hashes and the format version; a scenario of an older format is migrated by named functions, as saves are.

### 2. Levers and commands
**The lever registry** is one file, `crates/econ-core/levers.toml`. Each entry:

| Field | Meaning |
|---|---|
| `id` | stable dotted name, e.g. `tax.vat.rate_standard`, `money.policy_rate`, `world.oil_price`. Never reused or renamed; a lever that goes away is marked retired |
| `kind` | `law` (takes effect on the first day of a month), `market` (takes effect the next day), `project` (starts something with a duration), `world` (a world setting) |
| `shape` | the form of the value (below) |
| `range`, `unit` | for validation and display |
| `key` | for a lever that exists once per good, industry, county, technology or programme: the table of its keys |
| `spec`, `glossary` | the spec that owns the lever and the glossary name it sets |
| `min_lag` | shortest time between decision and effect, if longer than the kind's |
| `milestone` | when the lever arrives (the tags of the [lever catalogue](../../02-design/game/levers.md)) |

A generator (as for the glossary) writes from it: the Rust lever ids and value types, the list the UI builds its panels from, and a test that every lever of the registry is set in `law.json` or `world.json`, wired to a mechanic and covered by a policy test. The lever catalogue's tables become generated from the registry once it holds every lever.

**Value shapes** (few on purpose):

| Shape | For |
|---|---|
| `rate` | integer basis points: tax rates, the policy rate, shares |
| `money` | integer bani: amounts, thresholds, budgets |
| `number` | integer count or index: retirement age in months, quotas |
| `choice` | one of the options listed in the registry: exchange-rate regime, VAT category |
| `flag` | on or off |
| `schedule` | an ordered list of (threshold in bani, rate in basis points): tax brackets, benefit formulas |

Values are integers, never floats, so a command means the same on every machine.

**A command** is:

```rust
pub struct Command {
    pub lever: LeverId,          // generated from the registry
    pub key: Option<u32>,        // which good, industry, county, ... for a keyed lever
    pub value: LeverValue,       // one of the shapes
    pub effective: Option<Date>, // None: as soon as the lever's kind and lag allow
}
```

- The player gives a command on a day. The core validates it (shape, range, key), computes the date it takes effect (not before what `kind` and `min_lag` allow) and puts it in the pending list. Pending commands are part of the state and are shown to the player.
- At the start of each day the commands due that day are applied, in the order they were given. Applying a command changes the law in force and nothing else; mechanics read the law in force.
- A command that breaks an EU rule is applied and reported as a warning, as the VAT floor already is; it is refused only if it is malformed or out of range.
- Projects (build a plant, nationalise a firm) are commands of kind `project` whose value names what to start; their progress is state of the mechanic that owns them.

**The command log** records, per command: the day it was given, the command, and the date it took effect. It is JSON lines with lever ids as text, so a log stays readable, diffable and valid when levers are added. (ADR-0011 named a binary encoding; the spike already writes JSON lines and the log is small.)

**Headless play** (roadmap M4) is a scenario plus a file of dated commands in the same form as the log.

### 3. What this replaces
`Command` in `econ-core` and `ScaleCommand` in the scale world stay as they are for the spikes and their goldens. The real `World` (roadmap M2) uses the command above from its first version, with the registry holding only the levers that exist.

## Consequences
- Easier: one list of levers for the core, the UI, the tests and the docs; adding a lever is a registry entry, a mechanic reading it and a policy test.
- Easier: the law in force is data, so a scenario for another start date or country changes files, not code.
- Easier: saves and replays survive new levers, because ids are stable text.
- Harder: a generator to write and keep; six value shapes must cover every lever (a lever that fits none needs a new shape and a reason).
- Harder: one scenario per scale takes disk space (1:10 is the large one) and the three must be rebuilt together.
- Revisit if a lever's value cannot be expressed in the shapes without contortion, or if scenario build time at 1:10 becomes a nuisance (then option 2 for the population only).

## Open questions
Settled as proposed when the ADR was accepted (2026-10-10):
- [x] The registry is `crates/econ-core/levers.toml`.
- [x] A pending command can be cancelled before it takes effect, by a second command; both stay in the log.
- [x] The default seed is part of the scenario; the player may choose another at "new game".
