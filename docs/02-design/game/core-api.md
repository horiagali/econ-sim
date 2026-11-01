---
id: game/core-api
title: Core API — One Typed Query and Command Surface for the UI
status: draft
owner: horia
depends_on: [game/statistics-registry, game/group-queries, game/explanations, game/history-files, game/lever-system, adr/0013-ui-stack, adr/0018-scenario-and-commands]
research: []
updated: 2026-11-01
---

# Core API — One Typed Query and Command Surface for the UI

## Purpose
Roadmap M6, fifth slice (owner, 2026-11-01). Everything the UI (M8) asks of the simulation goes through one surface: typed **queries** that read and **commands** that act, each serialisable, so the Tauri host passes them through without knowing the core's internals ([ADR-0013](../../03-architecture/decisions/0013-ui-stack.md): every host call goes through `api.ts`). The thin UI of M2 moves onto it.

## Queries and commands

| Query | Answer |
|---|---|
| `Status` | the date, day and month index, `sample_scale`, the law's name, the features compiled |
| `Indicators` | the registry's metadata ([statistics registry](statistics-registry.md)) |
| `Series { id, key, from, to, daily }` | the points of a series in a month range (daily points for a day indicator when asked) |
| `Keys { id }` | the keys an indicator has |
| `Group { filter }`, `GroupBreakdown { filter, by }`, `Watched { name }` | [group queries](group-queries.md) |
| `Explain { id, key, from, to }`, `Explained` | [explanations](explanations.md) |
| `Levers` | the lever registry, the law in force and the pending orders ([lever system](lever-system.md)) |

| Command | Effect |
|---|---|
| `Step { days }`, `StepMonths { months }` | step the `World` (to the next close for a month) |
| `Order { order }` | a lever order (set, cancel), validated as the lever system does; the answer is its acceptance or rejection |
| `Watch { name, filter }` | watch a group from the next close |
| `WriteHistory { dir }` | write the closed years' history files ([history files](history-files.md)): the host's command (the writer is `econ_io::history`, and Arrow stays out of econ-core, ADR-0011); `World::command` answers it with an error naming the host |

A query never changes the `World`; a command answers with what it did (the days stepped and the closes made, the order's acceptance, the files written). Errors are values (`ApiError`: unknown indicator, bad key, rejected order, I/O), never panics.

**Encoding.** Queries, commands and answers are `serde` types. In-process (the Tauri host) they pass as Rust values; across IPC as JSON, with series as binary `i64` arrays in a separate buffer (`tauri::ipc::Response`, ADR-0013). `ui/src/generated/api.ts` holds the TypeScript types, generated from the Rust types (`just codegen`), so the host and the UI cannot drift.

## API sketch
Names may change before lock; shapes should not. Tests come first (ADR-0014).

```rust
// crate econ-core, module `api` (feature `api`, needs `stats`, `groups`, `explain`)
#[derive(Serialize, Deserialize)] pub enum Query { Status, Indicators, Series { id: String, key: KeyDto, from: u32, to: u32, daily: bool }, Keys { id: String },
    Group { filter: Filter }, GroupBreakdown { filter: Filter, by: Attribute }, Watched { name: String },
    Explain { id: String, key: KeyDto, from: u32, to: u32 }, Explained, Levers }
#[derive(Serialize, Deserialize)] pub enum Command { Step { days: u32 }, StepMonths { months: u32 }, Order { order: Order },
    Watch { name: String, filter: Filter }, WriteHistory { dir: String } }
#[derive(Serialize, Deserialize)] pub enum Answer { /* one variant per query and command */ }
pub enum ApiError { UnknownIndicator(String), BadKey, Rejected(Rejection), Io(String) }
impl World {
    pub fn query(&self, q: &Query) -> Result<Answer, ApiError>;
    pub fn command(&mut self, c: &Command) -> Result<Answer, ApiError>;
}
```

Indicators are named by their dotted registry names across the API (stable across engine versions; the generated `IndicatorId` is not).

## Acceptance tests
IDs are stable. Tests in `crates/econ-core/tests/acceptance/api.rs` (protected; the test-writer's), feature `api`.

- [ ] **AC-API-01** `[sim]` Every query's answer equals the direct method's (`Series` = `stats().series`, `Group` = `group`, `Explain` = `explain`, `Levers` = the registry and the law in force) at 1:1000 after 3 closes, and no query changes the state hash.
- [ ] **AC-API-02** `[unit]` Every query, command and answer survives a JSON round trip unchanged; an unknown indicator name, a key it lacks and a malformed query are errors, never panics.
- [ ] **AC-API-03** `[sim]` Commands: `StepMonths { 2 }` from day 0 makes two closes and answers the days stepped; an `Order` the lever system accepts is pending after it, one it rejects answers `Rejected` with the lever system's code; `Watch` then `StepMonths { 1 }` gives one watched summary.
- [ ] **AC-API-04** `[unit]` `ui/src/generated/api.ts` is in step with the Rust types (`just codegen-check`), and the UI's `api.ts` adapter compiles against it (the CI's `ui` job).
- [ ] **AC-API-05** `[sim]` A session through the API alone reproduces a direct run: the same orders and steps through `command` give the same state hash after 12 closes as calling the `World`'s methods.

## Implementation status
- **2026-11-01:** built (feature `api`, off by default; `api.rs`). Readings where the spec leaves room:
  - `Filter`, `GroupSummary` and `Node` pass through private mirror types with the same fields: persons' attributes by their variant names (`"F"`, `"Isced3"`, `"Retired"`), money as bani, an indicator by its dotted name. So econ-popgen and econ-types need no `serde`.
  - A key is national, or a member the indicator has points for (else `BadKey`). A national series without a point yet answers an empty series.
  - `Status`'s law is "the law of" the start year: the `World` does not name its law until M8's campaigns.
  - `WriteHistory` is answered by the Tauri host (`api_command` writes with `econ_io::history::write_all`). `World::command` answers it with `Io`, naming the host.
  - An order naming a lever or key the registry lacks answers the lever system's `unknown_lever` or `bad_key`.
  - `scripts/codegen_api.py` writes `ui/src/generated/api.ts` from the serde types of `api.rs` (and `Attribute`). It runs in `just codegen` and `codegen-check`, and writes serde_json's external tagging.
  - The UI's `api.ts` adds `query` and `command` over the Tauri commands `api_query` and `api_command`. The browser mock answers `Status` and `Step` only.
  - The M2 screens keep their own commands until M8 draws the API's answers.
  - AC-API-01 to 05 pass.

## Open questions
- [x] **Generating the TypeScript types.** *Proposed (2026-11-01):* a small generator over the serde types (`ts-rs` or a script over a JSON schema from `schemars`), run by `just codegen`; the choice is the implementation's.
