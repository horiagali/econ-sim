---
id: adr/0011-storage-saves-history
title: "ADR-0011: Storage, saves and history"
status: accepted
owner: horia
depends_on: [adr/0005-simulation-core-architecture, adr/0006-determinism-contract]
updated: 2026-10-10
---

# ADR-0011: Storage, saves and history

## Context
The game needs live queries (the group explorer filters ~190k persons by any attributes), saves that load fast and replay exactly, and decades of history for charts. Saves must also survive engine upgrades and stay small. Steam Cloud's limits are used below as a size yardstick, although distribution is deferred ([ADR-0015](0015-distribution-and-licensing.md)).

The [tech-stack research](../../01-research/notes/tech-stack-deep-research.md) (section "No database ships") found:
- A four-condition filter over 190k persons × 30 columns took 0.43 ms in Polars, 3.36 ms in DuckDB and 0.17 ms as a plain NumPy mask. A hand-written Rust bitmask filter will be at least as fast.
- Embedding a database is expensive to build: a trivial program took 12 minutes and 64.6 MB with Polars, and 19.5 minutes and 38.7 MB with DuckDB, against 118 s and 5.5 MB with arrow-rs (local benchmark on a sandbox VM). The Polars Rust crate is still pre-1.0.
- Arrow IPC with zstd wrote the person snapshot (11.0 MB) in 30 ms and read it in 16 ms; Parquet with zstd took 271 ms to write.
- bincode 3.0.0 is a tombstone release and development has stopped. rkyv has no schema evolution.
- Steam Cloud limits a single write to 100 MB and re-uploads only changed files.
- Factorio replays break across versions; Factorio handles this with named, ordered migrations recorded in the save.

## Options considered
**Runtime state**
1. **No database: column vectors in Rust, bitmask filters for the group explorer.** Fast, no build cost.
2. **Embedded Polars or DuckDB.** SQL and group-by for free; 12–20 minute clean builds and 39–65 MB binaries.
3. **SQLite.** Mature; a row store, slow for column scans.

**Save format**
1. **ZIP of manifest + Arrow IPC tables + command log.** Fast, random access, schema metadata, external tools can read it.
2. **Monolithic save including all history.** Simple; breaks Steam Cloud's 100 MB limit in long campaigns.
3. **bincode or rkyv blobs.** Fast; bincode is dead; rkyv has no schema evolution.

## Decision
Accepted by the owner on 2026-10-09.

**Chosen: option 1 in both groups.**

**Runtime.** No embedded database ships. Live state is the column vectors of [ADR-0005](0005-simulation-core-architecture.md). The group explorer uses bitmask filters over columns. arrow-rs is used only at the I/O boundary (`econ-io`). Polars and DuckDB stay in Python tooling.

**Save file** = a ZIP with uncompressed entries containing:
- a JSON manifest: format and engine versions, git hash, content hash (data and mods), seed, RNG algorithm ID, `sample_scale`, tick, state hash, applied migrations, per-part checksums;
- one Arrow IPC file per table, zstd-compressed, with units and stock/flow kinds in schema `custom_metadata`;
- the player command log, encoded with postcard.

**Counterfactual runs** fork in memory with a plain `Clone` of `World`.

**Replay.** A command log replays only when both the engine hash and the content hash match the manifest. After an upgrade, the migrated snapshot becomes the new replay root.

**Migrations** are named, ordered Rust functions over Arrow tables. The save records which ones have run. CI loads a golden save corpus (one save per released version) and steps each one 12 ticks.

**History lives outside each save.** Closed years are immutable `campaign_<id>/history/<year>.arrow` blocks that the save references by hash, so unchanged years are not re-uploaded to Steam Cloud.
- Macro series: monthly forever.
- Aggregation cube (about 34k cells, the same at any scale): monthly for ten years, annual after that.
- Downsampling follows meaning: stocks take the end-of-period value, flows are summed, ratios are recomputed from numerator and denominator.
- Person-level history is not stored; it is rebuilt by replay.
- At 1:100 the whole 50-year history budget is about 130 MB compressed, split across yearly files.

**Scale is recorded everywhere.** `sample_scale` is written into every save manifest, history block and export header, so history recorded at one scale is never silently mixed with another.

## Consequences
- Easier: fast saves and loads; external tools (Python, DuckDB) can open saves and history directly; exact bug reproduction from a save.
- Easier: Steam Cloud uploads stay small because closed years never change.
- Harder: hand-written filters and group-bys in Rust; migrations must be written for every schema change.
- Harder: arrow-rs releases monthly with breaking majors at most quarterly; it must stay pinned to the version pyo3-arrow supports ([ADR-0009](0009-calibration-and-stability.md)).
- Revisit if a player-facing SQL console becomes a feature, or if autosave of long campaigns gets slow (then consider an SQLite container for history).

## Open questions / to verify
- [ ] Save size and load time at 1:100 and 1:10 (Spike 6).
- [ ] Autosave frequency and how many autosaves to keep within Steam Cloud quotas.
- [ ] Whether the aggregation cube dimensions (county × status × age band × sex × education) are the right ones for the UI.

## Amendment 1 (2026-10-10): daily tick
[ADR-0017](0017-time-base.md) makes one tick one day. A save records the date and the day number. Fast macro series (exchange rate, interest rates and yields, goods price indices, electricity price, reserves) also keep daily points for the current and the two previous years; "monthly forever" and the cube are unchanged. The save-corpus test steps each golden save 31 days instead of 12 ticks.
