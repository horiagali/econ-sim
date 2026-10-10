---
id: architecture/spikes-5-6-results
title: "Spike results: 5–9 (calibration, saves, data pipeline, UI slice, agent workflow)"
status: draft
owner: horia
depends_on: [architecture/spikes-0-4-results, adr/0009-calibration-and-stability, adr/0011-storage-saves-history, economy/taxation]
updated: 2026-10-10
---

# Spike results: 5–9

Built 2026-10-09 (autonomous session), Linux, 2-vCPU Xeon 2.1 GHz, Rust 1.97.0, Python 3.13.

## Spike 6 — saves and replay ✅
Crate `econ-io` (the only crate that depends on Arrow, per ADR-0005).

- **Format:** ZIP (entries stored; tables already compressed) with `manifest.json`, Arrow IPC + zstd tables (`persons`, `households`, `io`, `shares`) and `commands.jsonl` (player command log).
- **Manifest:** format version, engine version, scenario, `sample_scale`, seed, tick, behaviour parameters, state hash, applied migrations, FNV-1a checksum per entry.
- **Load:** verify checksums → run named migrations in order → rebuild the world → verify the state hash.
- **Migration demo:** `v1_to_v2_add_separation_rate` upgrades a v1 save and is recorded in `migrations_applied`.
- **Tests:** save at tick 5 → load → continue to tick 10 gives the same state hash as an uninterrupted run; a **full replay** from the scenario start with the command log gives the same hash; the commands demonstrably change the outcome; saves are **byte-reproducible**; a flipped byte is detected.
- **Size and speed** (`just bench-save`, spike world with ~10 columns):

| Scale | Persons | Save size | Save | Load + verify |
|---|---|---|---|---|
| 1:100 | 190,000 | 1.7 MB | 46 ms | 24 ms |
| 1:10 | 1,900,000 | 16.3 MB | 508 ms | 287 ms |

Real saves will have ~10× more columns (≈ 15–30 MB at 1:100), still small.

Deviations from ADR-0011, for the owner to confirm: the command log is JSON lines (readable, tiny) instead of postcard; history blocks are not in this spike.

**Bug caught during the spike:** the separation-rate command was silently ignored (a refactor left a hardcoded constant). The round-trip test still passed, because a no-op command replays identically. Fixed, and the test now also asserts that commands change the outcome — a good example of why tests need a "does this test have teeth?" check (ADR-0010 mutation testing).

## Spike 5 — calibration loop ✅
- `crates/econ-py`: PyO3 0.29 bindings (abi3, Python ≥ 3.10), **not** a workspace member so plain `cargo` builds never need Python. Simulations run without the GIL and return whole series.
- `python/calib`: uv project (numpy, SALib, econ-py built by maturin). `calibration_spike.py` (`just calib-spike`).
- The spike world's behaviour constants became explicit, saved parameters (`ScaleParams`).

**Runs per hour** (120-tick = 10-year runs, single thread):

| Scale | Seconds per run | Runs per hour |
|---|---|---|
| 1:1000 | 0.10 | ~36,000 |
| 1:100 | 0.88 | ~4,100 |

**Morris screening** (60 runs, 5 parameters, μ*): unemployment is driven by matching efficiency (0.107) and the separation rate (0.099), then the vacancy ratio (0.037); the consumption/wage ratio by the propensities to consume out of wealth (0.104) and income (0.064). Consumption parameters have zero effect on unemployment — correct for this spike world, where labour demand doesn't yet respond to sales.

**History matching, wave 1** (200 runs, two illustrative targets, implausibility < 3): 53% of the space is not ruled out; one wave doesn't narrow single-parameter ranges yet, as expected with 5 parameters and 2 targets.

**100-year quiet baseline** (1,200 ticks at 1:1000): unemployment stays between 5.0% and 7.3% (5.56% in the first decade, 5.51% in the last), all values finite, no explosion.

**Calibration budget.** Calibration should run at 1:1000 (~36k runs/hour per core), checking the final candidates at 1:100. A typical workflow of Morris (~600 runs for 50 parameters) plus 3–5 history-matching waves of ~1,000 runs takes ~2–3 hours on one core at 1:1000, before the full model adds cost. Calibration time is not the constraint; building good targets is.

## Spike 7 — data pipeline slice ✅ (live fetch verified on Windows 2026-10-10)
- **Glossary codegen:** `scripts/codegen_glossary.py` turns the glossary's variable table into `crates/econ-types/src/glossary.rs` (95 variables, uniqueness tested). `just check` and CI fail if it is stale, so code and glossary can't drift.
- **Fetch with provenance:** `python/pipeline/fetch_eurostat.py` downloads the datasets in `sources.toml` (Eurostat JSON-stat API) and writes a provenance sidecar (URL, retrieval time, SHA-256, licence tag). **Not run here:** the sandbox's network policy blocks ec.europa.eu, so the first live fetch happens on the owner's machine. A JSON-stat reader is tested on a fixture.
- **IPF synthesis:** `ipf.py` (raking with convergence checked on all margins after each sweep, plus largest-remainder integer weights) and `synth_counties.py` fit a seed sample to county × age × sex margins for three counties (Bihor, Cluj, Bucharest) — converges in 4 sweeps. **Margins are illustrative placeholders**, not official figures. The scale is a command-line parameter, and a test checks the total weight is the same at 1:1000 and 1:200.
- **DVC:** `python/pipeline/dvc.yaml` declares the fetch and synth stages (not executed here).
- **Bug caught:** the first IPF version reported convergence after one sweep because it only checked the margin it had just adjusted. Fixed.

Still to do for the full pipeline: real Census 2021 marginals (INS), the IPUMS 2011 seed sample (private), Polars/DuckDB once tables grow, TypeScript codegen for the UI.

## Spike 8 — UI slice ◐ (built; Windows run pending)
- `ui/`: Vite + React 18 + TypeScript, uPlot charts, ECharts county map, nested "why" tree. `src/api.ts` is the only file that knows the transport: Tauri commands in the desktop app, a deterministic mock with the same shapes in a plain browser.
- `ui/src-tauri/`: Tauri 2 host linking `econ-core` and `econ-rules` in-process. Commands: `run_series` (returns raw little-endian f64 bytes → `ArrayBuffer` in JS, no JSON for big arrays), `county_values`, and `explain` (a real LMDI contribution tree from `behaviour_rule!`).
- `npm run fetch-geo` downloads county boundaries from geoBoundaries (CC BY 4.0); network policy blocked it in the sandbox.
- **Measured** in headless Chromium (mock data): 40 uPlot charts × 600 months ready ~330 ms after navigation, ~10 MB JS heap. TypeScript strict build passes; bundle 1.2 MB (415 KB gzipped, mostly ECharts — can be trimmed with per-chart imports).
- **Not measured:** the Tauri host could not be compiled in the sandbox (no WebView libraries on Linux). The non-Tauri parts of the host were compile-checked from an external crate. The exit criterion (1–5 MB IPC round trip on WebView2, cold start, memory) needs `npm run tauri dev` on Windows.
- Fix made on the way: `behaviour_rule!` now works from crates that don't depend on `econ-num` (it re-exports it).

## Spike 9 — agent workflow dry run (VAT) ✅ (single-session; two-PR flow untested)
The tests-first flow from ADR-0014, run end to end on one real mechanic:
1. **Spec:** a VAT section with stable IDs `AC-VAT-01..05` added to `economy/taxation` (rounding, categories incl. zero vs exempt, ledger posting under new flow code `tax.vat` = 3002, EU 15% floor as a warning, a 19→21% rate change).
2. **Stubs:** `scripts/ac_stubs.py` (`just ac-stubs …`) generated five `#[ignore]`d tests in `crates/econ-mech-tax/tests/acceptance/vat.rs`; never overwrites existing tests.
3. **Tests:** written from the spec text only, against an API the test writer chose (`VatRate` in basis points, `VatSchedule`, `collect_vat`).
4. **Implementation:** `crates/econ-mech-tax` (VAT as exact `mul_ratio` on basis points — no float multiply). 5/5 pass first run.
5. **Traceability:** `scripts/traceability.py` (`just trace`, in `just check` and CI) — every `**AC-…**` in a spec needs a live (non-ignored) test mentioning it; fails for `locked` specs and for tests citing unknown IDs.
6. **Mutation testing:** `cargo mutants -p econ-mech-tax` (`just mutants econ-mech-tax`): 25 mutants, **19 caught, 6 unviable, 0 missed**, 51 s.

**Friction found (and fixed):**
- The traceability checker first counted `#[ignore]`d stubs as covered (it split the file between the doc comment and the attribute). Fixed to group each `#[test]` with its doc comments/attributes; the dry run caught it because stubs showed "5/5 covered".
- **The protect-paths hook only watched Edit/Write.** A shell command (`sed -i …/tests/acceptance/…`, `> tests/golden/…`) bypassed it. The hook now also inspects Bash/PowerShell commands (heuristic: protected path + a write operator). Still bypassable via `just golden-print` or a script — layer 2 (CODEOWNERS / PR review) remains the real guard.
- Tests-first means the **test writer picks the API**. Fine for a leaf mechanic; for mechanics touching shared types the spec should carry an "API sketch" section, or the implementer will fight the tests.
- `clippy::double_must_use` fires on any `#[must_use]` fn returning `Bani` (already `must_use`) — note added to crates/AGENTS.md.

**Not tested:** the two-PR split (tests PR merged before implementation PR) and `cargo mutants --in-diff` in CI — both need GitHub. Suggested CI job: `cargo mutants --in-diff <(git diff origin/main)` on PRs touching `crates/econ-mech-*`.

## Windows verification ◐ (2026-10-10; Rust builds blocked on the laptop)

Run on the owner's Windows 11 Pro laptop (10.0.26200), no admin rights. Everything was installed per-user: rustup with Rust 1.97.0 on the **`x86_64-pc-windows-gnu`** host (the MSVC Build Tools need admin), WinLibs MinGW-w64 GCC 16.2.0 (msvcrt), Python 3.12.14 via `uv`, Node 24.21.0, `gh` 2.102.0, `just` 1.58.0 (prebuilt release; `cargo install just` could not build).

**Blocked: every Rust build.** Smart App Control is in enforcement mode on this machine and refuses to run some of the unsigned build scripts cargo compiles (`os error 4551`, "An Application Control policy has blocked this file"). It hit `num-traits` and `flatbuffers` in the workspace, `pyo3` in `econ-py`, `erased-serde` in `ui/src-tauri`, and `camino` / `pulldown-cmark` when building `just`. A trivial crate with a build script built and ran, so the verdict is per file, not per toolchain. Turning Smart App Control off needs an administrator and cannot be undone without resetting Windows, so it was left alone. Not run as a result:

| Check | Status |
|---|---|
| `just check` | stops at `lint` (clippy cannot build); `fmt-check`, `docs`, `codegen-check`, `trace` and `pipeline-test` pass |
| `just golden-check`, Windows state hashes | not run — **no Windows-vs-Linux hash comparison exists yet** |
| `lint-canary`, `diff-sim`, `cargo test --workspace` | not run |
| `just calib-spike` (calibration timing on Windows) | not run: `uv sync` fails building `econ-py` |
| `npm run tauri dev` (Spike 8 exit criterion) | not run: the host fails on `erased-serde`'s build script before any Tauri code compiles, so whether Tauri builds on the GNU target is still unknown |
| PractRand on `fast_u64` | skipped |

**Verified on Windows:**
- **Live Eurostat fetch.** `fetch_eurostat.py` downloaded both datasets in `sources.toml` (`demo_r_pjangrp3`: 198 rows, `nama_10r_3gdp`: 3 rows; both parse with the JSON-stat reader) and wrote provenance sidecars whose SHA-256 matches the raw file.
- **UI without the host.** `npm install`, `npm run fetch-geo` (`ui/public/geo/romania-adm1.geojson`, 1,249,797 bytes, geoBoundaries ADM1, CC BY 4.0) and `npm run build` (TypeScript strict + Vite; bundle 1.24 MB, 416 KB gzipped) all pass.
- **Protect-paths hook.** Blocks Edit/Write to `tests/golden/`, `crates/*/tests/acceptance/` and `schema/` given Windows paths (backslashes, lower-case drive letter) and lets other paths through.

**Fixed on the way:**
- **The hook never blocked.** `.claude/settings.json` ran `python3 hook || python hook`. When `python3` blocked (exit 2), the `||` fallback ran the script again with stdin already consumed; it read no payload and exited 0, so the edit was allowed. The command now picks one interpreter (`command -v python3`) and runs the script once.
- **First fetch on a fresh Windows machine failed** with `CERTIFICATE_VERIFY_FAILED`. Windows installs root certificates on first use and Python's `ssl` only sees those already installed (35 roots before, 36 after another program contacted the host). `fetch_eurostat.py` now makes one request through the system TLS stack (`curl.exe`) on that error and retries with verification still on. Two unit tests cover the retry; the real failure could not be reproduced again once the root was cached.
- **`just calib-spike` used `cd … && …`,** which is a parse error in Windows PowerShell 5.1 (the justfile's Windows shell). It now uses `uv --directory python/calib run …`.
- **The parked justfile and CI were older than the docs:** `trace`, `ac-stubs` and `mutants` recipes were missing although `crates/AGENTS.md` and Spike 9 refer to them, and traceability was in neither `just check` nor CI. Added.

**Still open:**
- `.claude/settings.json` matches only `Edit|Write|MultiEdit|NotebookEdit`, so the shell-command check described under Spike 9 never runs. Adding `Bash|PowerShell` to the matcher would enable it, at the cost of blocking any command that names a protected path together with `>` (for example `… --check-golden tests/golden/sim_200.hashes 2>&1`).
- CI's Windows job uses the MSVC toolchain; the laptop can only use GNU. If the laptop is ever unblocked, a hash difference between the two Windows toolchains would be a determinism finding in its own right.

## Not done yet (next steps)
- Windows: `just check`, the golden run, calibration timing and `npm run tauri dev` (Spike 8 exit criterion) — all waiting on Smart App Control being turned off on the laptop, or on another Windows machine. The CI `windows` and `determinism` jobs can supply the hash comparison without the laptop.
- Follow-ups: model PC in the differential test; history blocks in saves; real firm-unit goods market; real data marginals; wire `tax.vat` into the scale world's consumption.
