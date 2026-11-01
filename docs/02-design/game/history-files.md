---
id: game/history-files
title: History Files — the Statistics History on Disk
status: draft
owner: horia
depends_on: [game/statistics-registry, game/group-queries, adr/0011-storage-saves-history, adr/0006-determinism-contract]
research: []
updated: 2026-11-01
---

# History Files — the Statistics History on Disk

## Purpose
Roadmap M6, third slice ("History stored in saves", the open follow-up of Spike 6; owner, 2026-11-01). [ADR-0011](../../03-architecture/decisions/0011-storage-saves-history.md) keeps the history outside each save, as one immutable file per closed year that the save references by hash. This spec writes and reads those files for the [statistics registry](statistics-registry.md)'s history and the watched groups ([group queries](group-queries.md)). Saving and loading the `World` itself comes with M8's saves; they will reference these files.

## The files
A campaign's history is a folder of **yearly blocks** `history/<year>.arrow` (Arrow IPC, zstd-compressed), one per closed calendar year, plus the current year in memory (written when the year closes, and on demand for a save).

- **Rows:** one per (indicator, key, period point). Columns: `indicator` (the registry's dotted name), `breakdown` (the breakdown's name, or empty for national), `index` (the member's index), `month` (the month index from the start), `day` (the day index, for daily points; null for monthly points), `value` (`i64` in the indicator's unit).
- **Schema metadata:** the format version, `sample_scale`, the scenario's id, the engine's git hash, the year, and for each indicator its unit and period (so a block is readable without the registry of the engine that wrote it).
- **Watched groups** are a second table in the same block: `group` (the name), `month`, then one column per summary field.
- **Hash:** each block's SHA-256 over its bytes, recorded in a `history/index.json` (year → file name, hash, rows); a save will record the index's hash.
- **Downsampling** ([ADR-0011](../../03-architecture/decisions/0011-storage-saves-history.md)): none in M6. Monthly points are kept forever; daily points too (the registry's day series are few). The aggregation cube and its annual downsampling after ten years come with the cube.

Determinism: the same run writes byte-identical blocks (rows sorted by indicator in the registry's order, then breakdown, index, month, day; no wall-clock time in the metadata other than none).

## API sketch
Names may change before lock; shapes should not. Tests come first (ADR-0014).

```rust
// crate econ-io, module `history` (feature `history`)
pub fn write_year(dir: &Path, world: &World, year: i32) -> Result<BlockInfo, SaveError>; // a closed year's block
pub fn write_all(dir: &Path, world: &World) -> Result<Vec<BlockInfo>, SaveError>;       // every closed year + index.json
pub fn read_history(dir: &Path) -> Result<History, SaveError>;                          // all blocks, checked against index.json
pub struct BlockInfo { pub year: i32, pub file: String, pub sha256: String, pub rows: u64 }
pub struct History { pub sample_scale: u32, pub points: Vec<Point>, pub watched: Vec<(String, u32, GroupSummary)> }
pub struct Point { pub indicator: String, pub breakdown: Option<String>, pub index: u32, pub month: u32, pub day: Option<u32>, pub value: i64 }
```

## Acceptance tests
IDs are stable. Tests in `crates/econ-io/tests/acceptance/history.rs` (protected; the test-writer's), feature `history`.

- [ ] **AC-HIST-01** `[sim]` Round trip: at 1:1000, after 25 closes (two closed calendar years, 2022 and 2023, and the December 2021 close), the blocks written and read back give every point of the registry's history exactly (every indicator, key, month and day point), and the watched groups' summaries.
- [ ] **AC-HIST-02** `[sim]` Determinism: two runs with the same seed write byte-identical blocks and the same `index.json`; a different seed changes at least one block's hash.
- [ ] **AC-HIST-03** `[unit]` Integrity: a block whose bytes do not match its hash in `index.json`, a block of another `sample_scale` than the others, or a missing block is rejected with an error naming the file; a block written by an older format version is read if the version is listed as readable, else rejected.
- [ ] **AC-HIST-04** `[unit]` The schema metadata carries the format version, `sample_scale`, the scenario id and each indicator's unit and period; reading a block needs no registry (a reader given only the files gets the units).
- [ ] **AC-HIST-05** `[bench]` A 50-year history at 1:1000 (600 monthly frames of every registry indicator with their breakdowns, and the day series; a synthetic history of that shape, as long runs wait for M7's stability) is at most 20 MB on disk and reads back in at most 2 seconds in a release build.

## Implementation status
- **2026-11-01:** built (econ-io feature `history`, off by default). Readings where the spec leaves room: an Arrow IPC file holds one table, so the watched groups' summaries are rows of the same table, named `group.<name>.<field>` (a mean that is `None` has no row); the metadata is one schema key, `econ_sim_history`, a JSON object (format version, `sample_scale`, scenario id, engine, year, start date, each indicator's unit and period), so the bytes do not depend on a map's order; SHA-256 is computed by econ-io itself (no new dependency). A history written from a `World` has an empty scenario id (the `World` does not know its scenario's name until M8's campaigns); the engine is the build's `ECON_GIT_HASH`, else econ-io's version. AC-HIST-01 to 05 pass.

## Open questions
- [x] **Where the files live before M8's campaigns.** *Proposed (2026-11-01):* a folder the caller names (the CLI's `--out`, the tests' temp folder); M8 puts it in the campaign folder.
