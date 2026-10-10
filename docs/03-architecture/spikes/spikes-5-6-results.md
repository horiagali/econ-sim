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

## Spike 8 — UI slice ✅ (Tauri host measured on Windows 2026-10-10, see "Windows verification")
- `ui/`: Vite + React 18 + TypeScript, uPlot charts, ECharts county map, nested "why" tree. `src/api.ts` is the only file that knows the transport: Tauri commands in the desktop app, a deterministic mock with the same shapes in a plain browser.
- `ui/src-tauri/`: Tauri 2 host linking `econ-core` and `econ-rules` in-process. Commands: `run_series` (returns raw little-endian f64 bytes → `ArrayBuffer` in JS, no JSON for big arrays), `county_values`, and `explain` (a real LMDI contribution tree from `behaviour_rule!`).
- `npm run fetch-geo` downloads county boundaries from geoBoundaries (CC BY 4.0); network policy blocked it in the sandbox.
- **Measured** in headless Chromium (mock data): 40 uPlot charts × 600 months ready ~330 ms after navigation, ~10 MB JS heap. TypeScript strict build passes; bundle 1.2 MB (415 KB gzipped, mostly ECharts — can be trimmed with per-chart imports).
- **Not measured:** the Tauri host could not be compiled in the sandbox (no WebView libraries on Linux). The non-Tauri parts of the host were compile-checked from an external crate. The exit criterion (1–5 MB IPC round trip on WebView2, cold start, memory) was measured on Windows on 2026-10-10: see "Tauri host" under "Windows verification" below.
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

## Windows verification ✅ (2026-10-10, second laptop)

Run on a Windows 11 Pro laptop (10.0.26200) with admin rights: Intel Core i9-13900H, 32 GB RAM. **Smart App Control is off** on this machine, so cargo build scripts run. Toolchain: rustup 1.29.1 with Rust 1.97.0 on the **`x86_64-pc-windows-msvc`** host (the same as CI), linked with the MSVC 14.51 toolset from Visual Studio Community 2026 (18.10), which was already installed. `just` 1.58.0, `uv` 0.12.16, Node 24.18.0, `gh` 2.96.0. The repo scripts ran on Python 3.14.7 (the `python` on PATH); the calibration environment on Python 3.12.14 (via `uv`).

> The Visual Studio 2022 Build Tools were **not** installed: the `winget` install exited twice with code 1602 (elevation prompt dismissed or timed out), and the existing Visual Studio 2026 C++ toolset made them unnecessary.

| Check | Result on Windows |
|---|---|
| `just check` (fmt, clippy, 49 Rust tests, lint canary, docs, codegen, traceability, pipeline tests, hook tests, differential SIM test incl. both mutations) | ✅ green on the first run, no changes needed |
| `just golden-check` | ✅ `golden OK (200 ticks)` |
| State hashes, laptop vs the committed golden file (generated on Linux) | ✅ identical for all 200 ticks |
| `just bench-scale` | 0.6 / **4.6** / 47.2 ms per tick at 1:1000 / 1:100 / 1:10 (Linux VM: 6.4 ms at 1:100); 600 ticks at 1:100 in 2.8 s |
| `just bench-save` | 1.7 MB at 1:100 (save 29 ms, load + verify 16 ms); 16.3 MB at 1:10 (264 ms, 145 ms) — same sizes as Linux |
| `uv sync` in `python/calib`, `just calib-spike` | ✅ builds `econ-py` (PyO3) and runs; see below |
| `npm install`, `npm run fetch-geo`, `npm run tauri dev` | ✅ after one fix (missing icon); see below |
| Protect-paths hook | ✅ blocks Edit/Write and shell writes to protected paths in a live session; reads pass |
| PractRand on `fast_u64` | ◐ passes to 64 GB in simulation order; fails at 16 GB when only the entity varies; see below |

**No Windows-vs-Linux difference was found in any simulation result.** The employment and wage-bill columns of `bench-scale`, and every number in the calibration report (Morris μ*, the 106/200 history-matching survivors, the 100-year baseline), equal the Linux values.

### Calibration timing (Spike 5 on Windows)
120-tick (10-year) runs, single thread, Python 3.12:

| Scale | Seconds per run | Runs per hour | Linux VM |
|---|---|---|---|
| 1:1000 | 0.068 | ~53,300 | ~36,000 |
| 1:100 | 0.674 | ~5,340 | ~4,100 |

The whole spike script takes 26 s. The calibration budget above holds with room to spare.

### Tauri host (Spike 8 exit criterion) ✅
`ui/src-tauri` compiled for the first time (Tauri 2.12, WebView2). Measured through WebView2's DevTools port on a release build (`npm run tauri build -- --no-bundle`); the app shows 40 uPlot charts × 600 months plus the county map:

| Measure | Result |
|---|---|
| Cold start: process launch → all charts rendered | 1.4 s first launch, 0.8 s second |
| Header timing (release) | `backend=tauri data=392ms render=24ms` |
| Header timing (`tauri dev`, unoptimised) | `backend=tauri data=7400ms render=40ms` |
| IPC round trip, payload only (`ipc_probe`, median of 15) | 0.19 MB: 2.5 ms · 1 MB: 8.7 ms · 2 MB: 14.7 ms · **5 MB: 33.4 ms** (≈ 6.6 ms per MB) |
| `run_series` 40 × 600 (0.18 MB, includes 600 ticks of the 1:1000 world) | 352 ms, almost all simulation time |
| Memory, app + its WebView2 processes (7 processes) | ~310 MB private (~460 MB summed working sets); the Rust host itself 6 MB private; JS heap 16 MB |
| Release executable | 9.6 MB |

Reading: binary IPC is cheap enough to send whole chart sets every tick (a 1:100 tick is 4.6 ms; 1 MB of series costs 9 ms). `data=` is dominated by running the simulation inside the command, so real commands should return already-computed history. The debug build is ~20× slower in `data=`; judge UI performance on release builds only. Memory is the WebView2 baseline and should be re-measured when the real UI exists.

### PractRand on the fast RNG ◐ (passes in simulation order; one finding for the owner)
`econ-cli rng-raw` (new) writes raw `fast_u64` output; PractRand 0.95 (built from source with MSVC) read it with `RNG_test stdin64 -multithreaded`. Stream `Labour`, `k = 0`, seed 42 unless noted:

| Pattern | Varies | Result |
|---|---|---|
| `grid --entities 190000` | the simulation's order: entities 0..190,000, tick after tick | **no failure to 64 GB** (2^33 draws, ≈ 45,000 ticks' worth) |
| `tick` | tick = 0, 1, 2, … for one entity | no failure to 8 GB (not run further) |
| `entity` | entity = 0, 1, 2, … at one tick | clean to 8 GB, then **FAIL at 16 GB**: `BRank(12):8K(1)`, p ≈ 6e-235 |

Mild "unusual" flags (the lowest PractRand grade) appeared once each in single runs and were gone at the next length, as expected by chance.

**Finding.** With only the entity varying, the output is `mix64(const ^ entity)`: one SplitMix64 mix of a counter that steps by 1. The binary-rank test finds linear structure in it after 2^31 consecutive entities (16 GB). It reproduces with seed 7 (FAIL at 16 GB, p ≈ 1e-189). Two alternatives from `python/reference/rng_quality.py`, fed to PractRand on the same pattern from NumPy (the NumPy version of the current mixer matches `rng-raw` byte for byte):

| Mixer | Change | `entity` pattern |
|---|---|---|
| current | `mix64(st ^ entity)` | FAIL at 16 GB |
| "gamma" | `mix64(st ^ entity·γ)`, one extra multiply | no failure to 64 GB |
| "double" | `mix64(mix64(st ^ entity))`, one extra mix | no failure to 64 GB |

**What it means.** No simulation run is near the failing regime: it needs ~2 billion consecutive entity IDs inside one (stream, tick, k), and a 1:1 Romania has 19 million persons. In the order the simulation actually draws, the generator passes 64 GB. So this is a margin question, not a known error in results. The owner decides between (a) keeping the mixer and recording the limit in ADR-0006 Amendment 1, or (b) switching to the "gamma" variant, which is a re-golden event (`fast_u64`, `FAST_KAT`) that changes scale-world results such as the benchmark and calibration numbers; the SIM golden run does not use the fast hash (only `scale_spike.rs` does). Nothing was changed.

### Fixed on the way
- **The Tauri host could not build on Windows:** `tauri-build` needs `icons/icon.ico` to generate the Windows resource file. Added a placeholder icon (`ui/src-tauri/icons/`). Nothing else in the host needed changing.
- **`ipc_probe` command** added to the host: returns N zero bytes with no computation, so the IPC cost can be measured apart from simulation time.
- **Protect-paths hook, shell commands.** The old check blocked any command that named a protected path together with `>` (so `… --check-golden tests/golden/sim_200.hashes 2>&1` would have been blocked) and was never enabled. `scripts/hooks/protect_paths.py` now tokenises Bash and PowerShell commands and blocks only when a write targets a protected path: a redirect into it, or `sed -i`, `perl -i`, `mv`, `cp`, `rm`, `tee`, `touch`, `dd of=`, `curl -o`, `find -delete`, `git checkout/restore/rm/mv`, `Set-Content`, `Add-Content`, `Out-File`, `Remove-Item`, `Move-Item`, `Copy-Item`, `New-Item`, … with it as the target, including after `cd` into it, through `bash -c` / `powershell -Command`, and when a parent directory is removed. Reads, copies *out of* a protected path, `2>&1` and quoted text (commit messages, here-documents) pass. `.claude/settings.json` now matches `Bash|PowerShell` too. 12 unit tests covering 128 commands (`just hook-test`, in `just check` and CI). Still a heuristic: a script, a variable holding the path, or `just golden-print`-style tools that write on their own are not seen, so layer 2 (PR review) remains the real guard.

### History: the first laptop (2026-10-10)
The first Windows laptop had no admin rights and Smart App Control in enforcement mode, which blocked some cargo build scripts (`os error 4551`) on the GNU toolchain, so no Rust build ran there. That session verified the live Eurostat fetch, the UI in browser mode and the hook's Edit/Write path, and fixed: the hook's `python3 || python` fallback (which silently allowed edits), a certificate fallback through `curl.exe` in `fetch_eurostat.py`, `just calib-spike` under PowerShell 5.1, and the missing `trace` / `ac-stubs` / `mutants` recipes. The GNU-vs-MSVC hash comparison it raised was never run and is no longer needed: laptop and CI both use MSVC.

**CI:** the first run (commit `b1869c0`, [run 38039618551](https://github.com/horiagali/econ-sim/actions/runs/38039618551)) passed all four jobs, including `determinism` (Linux and Windows hashes identical). The run for this verification branch is recorded in its pull request.

## Not done yet (next steps)
- Follow-ups: model PC in the differential test; history blocks in saves; real firm-unit goods market; real data marginals; wire `tax.vat` into the scale world's consumption.
- Two-PR tests-first flow and `cargo mutants --in-diff` in CI (Spike 9 leftovers).
- `ui/src-tauri` is outside the root workspace, so `just check` and CI never compile it; a Windows CI step (`cargo check` in `ui/src-tauri`) would keep it building.
