---
id: research/tech-stack-deep-research
title: "Tech stack — deep research: Anchor econ-sim on a deterministic Rust ledger"
status: draft
owner: horia
depends_on: [research/population-modelling-deep-research]
updated: 2026-10-09
---

> Deep-research report (2026-10-09) on the tech stack for econ-sim and on the engineering decisions behind its hardest technical problems: calibration and stability, explainability, verifying stock-flow consistency in AI-written code, data integration and licensing, determinism and population scale. It builds on the [population modelling research](population-modelling-deep-research.md). Sources were fetched directly; benchmarks and experiments ran on a small sandbox VM and are order-of-magnitude. Items the report marks as unverified are carried into the "Open questions / to verify" lists of ADR-0004 to ADR-0015, which turn this note into proposed decisions.

# Anchor econ-sim on a deterministic Rust ledger

Build econ-sim as a **headless Rust workspace** that holds the economy as plain column tables. Money is whole bani in `i64`, random numbers are keyed on (seed, stream, tick, entity), maths that changes state goes only through `libm`, and every money movement posts to a tagged, double-entry ledger that is checked every tick. Put a **Tauri 2 desktop shell with a React/TypeScript front end** on top (uPlot for time series, ECharts for the county map and breakdown charts), linking the core in-process. Ship **no database**: the live state is Rust vectors, saves are Arrow IPC tables plus a command log in a small ZIP, and long history lives in immutable yearly blocks. Around the game, run a **Python factory**: uv, Polars and DuckDB for the offline Romania data pipeline, just and DVC to run it, PyO3 bindings for calibration, and GitHub Actions that run everything on Linux and check that Windows and Linux produce bit-identical runs. Four findings change earlier plans. **IPUMS-International bans commercial use and redistribution**, so the shipped population must be built from public aggregate tables unless INS Romania gives written approval. **BeforeIT.jl's licence is ambiguous** (AGPL-3.0 on GitHub's sidebar, Apache-2.0 in the LICENSE file), so treat it as AGPL and work only from the papers. **Claude Code ignores AGENTS.md whenever a CLAUDE.md exists** unless CLAUDE.md imports it with `@AGENTS.md`. **GitHub Free does not enforce CODEOWNERS or rulesets on private repos**, so test protection has to come from agent hooks and CI checks, or from upgrading to Pro. The hardest decisions to reverse are the ledger codes, the decomposable rule format, the RNG scheme, the money type and population scale as a parameter. All five belong in the first two weeks of spikes, before any mechanic is written.

## The game itself: a plain Rust core, a web UI and files instead of a database

### The core is a bespoke Rust workspace, not an engine or framework

No existing framework fits a columnar, phase-ordered, ledger-checked macro model. krABMaga, the main Rust ABM engine, follows MASON's per-agent `step` model and marks its parallel mode "experimental" ([krABMaga](https://github.com/krABMaga/krABMaga)). ECS libraries such as bevy_ecs and hecs are built for many entity types whose sets of components change over time. Your persons change attribute *values*, not their set of components, so plain `Vec` columns per table give the same cache benefit and map one-to-one onto Arrow arrays for export. The earlier benchmark found column storage **10–15× faster than one object per person** when a tick touches 4 of 100 attributes, and Rust ran at about **9 ns per person per tick** (local benchmark on a 2-vCPU sandbox VM).

Split the workspace into small crates so that agents get fast compiles and narrow edits (working notes from a sandbox VM, not published):

| Crate | Holds |
|---|---|
| `econ-types` | ID newtypes (`PersonId(u32)`, `FirmId(u32)`, `IndustryId(u8)`), `Bani(i64)`, `Month`, flow-code enums. No dependencies. Generated from the glossary |
| `econ-rng` | The keyed RNG wrapper, distributions written on `libm`, pinned known-answer vectors |
| `econ-num` | `libm` wrappers, fixed-chunk sums, largest-remainder splitting, the 80×80 LU solver |
| `econ-ledger` | Accounts, typed transfers, flow codes, tag accumulators, per-tick invariant checks |
| `econ-core` | `World` (the column tables), the `Phase` trait, a fixed `const` phase order, `Command`, `TickReport`, explanation records. No I/O |
| `econ-mechanics-*` | One module per spec doc (labour, consumption, production/IO, pricing, credit, fiscal, monetary, external) |
| `econ-io` | Saves, Arrow IPC/Parquet export, scenario loading |
| `econ-py` | PyO3 bindings |
| `econ-cli` | Headless runner for golden tests, benchmarks and calibration batches |

The core API is `step(&mut self, cmds: &[Command]) -> TickReport`, plus `snapshot`/`restore` and a read-only `query` that returns columns. The scheduler is an explicit ordered list of phases rather than dynamic registration, so phase order is visible in one file and reviewable when an agent changes it.

Rust is a sound choice for agent-written code. On SWE-bench Multilingual, **Rust had the highest resolve rate of nine languages (58.14%)** for the one model tested, against 28.57% for C/C++ ([SWE-bench Multilingual](https://www.swebench.com/multilingual.html)). Microsoft's RustAssistant fixed about **74% of real Rust compile errors** by looping between an LLM and the compiler ([arXiv 2308.05177](https://arxiv.org/abs/2308.05177)). The evidence is thin (one model, 43 Rust tasks), but it points the same way as the main argument: newtypes, exhaustive `match`, `#[must_use]` on postings and clippy deny-lists turn spec violations into compile errors that an agent can see and fix. C# would compile faster, but .NET documents that `Math.Sin` "calls into the underlying C runtime" and can differ between operating systems ([Microsoft Learn](https://learn.microsoft.com/en-us/dotnet/api/system.math.sin)). Rust's main cost is compile time. Keep Polars, DuckDB and Arrow out of `econ-core`, use `cargo check` in the agent loop, and add an AGENTS.md rule that a borrow error that survives two attempts means "restructure around indices", not "add `Rc<RefCell<>>`".

### The UI is Tauri 2 with React, uPlot and ECharts, with the core linked in-process

The hard UI work in this game is business-app work: dozens of charts, a group explorer with filters and cross-tabs, about 150 lever forms, and nested "why did this change" breakdowns. Web stacks have the most ready-made components and the most AI training data for this. In Stack Overflow's 2025 survey, **JavaScript (66%), TypeScript (43.6%) and React (44.7%)** dwarf GDScript (3.3%) ([Stack Overflow 2025](https://survey.stackoverflow.co/2025/technology)). Tauri 2 has been stable since October 2024 ([Tauri 2.0](https://v2.tauri.app/blog/tauri-20/)) and uses the system WebView2 on Windows, so a minimal app "can be less than 600KB" ([Tauri](https://v2.tauri.app/start/)). Its host process *is* Rust, so the sim crate becomes an ordinary dependency, with no FFI layer and no separate server. Return chart data as binary arrays through `tauri::ipc::Response`, since default command returns are JSON and "can slow down your application" for large data, and push per-tick updates over a `Channel`, which Tauri recommends for "ordered, high-throughput data delivery" ([Tauri: Calling Rust](https://v2.tauri.app/develop/calling-rust/)).

The data volume is small: 50 charts × 5 series × 600 months is 150k points. uPlot draws **166,650 points in 25 ms from a cold start** and streams at about 10% CPU where ECharts needs about 70% ([uPlot](https://raw.githubusercontent.com/leeoniya/uPlot/master/README.md)), so use it for every time series. Use ECharts, which handles maps, heatmaps and sankey charts and takes typed arrays ([ECharts](https://echarts.apache.org/en/feature.html)), for the 42-county choropleth and the waterfall and treemap views of the "why" panel. Avoid Plotly, which took 310 ms on the same benchmark. Hide the host behind a thin `api.ts` adapter so you can fall back to Electron cheaply. Electron is the proven Steam path: steamworks.js ships an Electron overlay helper ([steamworks.js](https://raw.githubusercontent.com/ceifa/steamworks.js/main/README.md)), while no commercial Tauri game appears in the curated awesome-tauri list ([awesome-tauri](https://raw.githubusercontent.com/tauri-apps/awesome-tauri/dev/README.md)). Whether the Steam overlay works over WebView2 is the one unverified risk, and it is optional for a Steam release.

Build a second, developer-only UI in **egui/eframe**. It links the core directly and is quick for an agent to build into a ledger, invariant and calibration inspector. egui's own README warns that customisation is "not yet as powerful as say CSS" and that "new releases will have breaking changes", which rules it out for the player UI ([egui](https://raw.githubusercontent.com/emilk/egui/main/README.md)). Godot, Unity and Bevy would make every chart, table and form custom work. Bevy has also shipped four breaking releases between April 2025 and June 2026 ([Bevy news](https://bevy.org/news/)), so agents would keep writing stale APIs.

For the county map, use **geoBoundaries (CC BY 4.0, commercial use allowed** with "geoBoundaries" named in the credits) ([geoBoundaries](https://www.geoboundaries.org/)). **Do not use GADM**: "Redistribution or commercial use is not allowed without prior permission" ([GADM](https://www.gadm.org/license.html)). Treat the Eurostat GISCO NUTS boundaries as unusable until you have read their dataset-specific terms ([GISCO](https://gisco-services.ec.europa.eu/distribution/v2/nuts/)), which the research recalls as non-commercial but could not fetch. Before relying on geoBoundaries, also confirm that its Romania ADM1 layer is the CC BY "gbOpen" release.

### No database ships: column vectors at runtime, Arrow files on disk

The game does not need an embedded database. A four-condition filter over 190k persons × 30 columns took **0.43 ms in Polars, 3.36 ms in DuckDB and 0.17 ms as a plain NumPy mask**. A hand-written Rust bitmask filter will be at or below the NumPy figure, so the group explorer needs no SQL engine. The build costs of embedding one are large. On the 2-vCPU research sandbox, a trivial program took **12 minutes and 64.6 MB with Polars, and 19.5 minutes and 38.7 MB with DuckDB**, against 118 s and 5.5 MB with arrow-rs (local benchmark on a 2-vCPU sandbox VM). Those clean builds recur on every CI cache miss and every dependency bump. The Polars Rust crate is also still pre-1.0 (0.55.2) and asks you to opt into features because they "put strain on compile times" ([docs.rs polars](https://docs.rs/polars/latest/polars/)). Use **arrow-rs only at the I/O boundary**. It releases monthly, with breaking majors at most quarterly ([arrow-rs](https://raw.githubusercontent.com/apache/arrow-rs/main/README.md)). Keep Polars and DuckDB in Python, where they are free to use.

A save is a **ZIP with uncompressed entries** containing:

- a JSON manifest: format and engine versions, git hash, content hash, seed, RNG algorithm ID, `scale_factor`, tick, state hash, applied migrations, per-part checksums;
- one **Arrow IPC file per table, zstd-compressed**;
- the player command log.

Arrow IPC with zstd beat Parquet on the person snapshot: 11.0 MB, written in 30 ms and read in 16 ms, against 271 ms to write Parquet with zstd (local benchmark on a 2-vCPU sandbox VM). Its footer gives random access to record batches, and schema-level `custom_metadata` can carry units and stock/flow kinds ([Arrow format](https://arrow.apache.org/docs/format/Columnar.html)). Do not use bincode: version 3.0.0 is a tombstone whose whole source is a `compile_error!`, and development has stopped (working notes from a sandbox VM, not published). Use postcard for the command log, and take in-memory forks for counterfactual runs with a plain `Clone` of `World`.

Keep long history *outside* each save. Steam Cloud limits a single write to 100 MB and advises splitting saved state by how often it changes, because unchanged files are not uploaded again ([Steam Cloud](https://partner.steamgames.com/doc/features/cloud)). Store closed years as immutable `campaign_<id>/history/<year>.arrow` blocks that the save references by hash. At 1:100 the whole 50-year history budget is about 130 MB compressed. Keep macro series monthly forever, keep the aggregation cube monthly for ten years and annually after that, and downsample by meaning: stocks take the end-of-period value, flows are summed, and ratios are recomputed from their numerator and denominator.

Replay has a known limit. Factorio's replays break across versions and when mods change ([Factorio replay](https://wiki.factorio.com/Replay_system)). The manifest therefore stores the engine and content hashes. A command log can be replayed only when both match. After an upgrade, the migrated snapshot becomes the new replay root. Migrations are named, ordered Rust functions over Arrow tables, and the save records which ones have run, as Factorio does ([Factorio migrations](https://lua-api.factorio.com/latest/auxiliary/migrations.html)). CI loads a **golden save corpus**, one save per released version, and steps each 12 ticks.

## The factory around the game: one schema, a reproducible pipeline and CI agents cannot dodge

### The Python side uses uv, Polars, DuckDB, just and DVC, all fed from one glossary

The offline pipeline is Python managed by **uv**. It uses **Polars** for transforms and **DuckDB** for SQL-heavy joins and for querying raw files in place. Use **just** as the single command surface for you and the agents. It is "a command runner, not a build system" and runs on Windows without extra dependencies ([just](https://just.systems/man/en/)). Add **DVC** pipelines once the graph passes about five stages. DVC hashes file contents and "only invalidates stages when the corresponding part of the params file has changed" ([DVC](https://doc.dvc.org/user-guide/pipelines/defining-pipelines)), so changing `scale` in `params.yaml` rebuilds only the population stages. Archiving matters more than usual because **Eurostat keeps only the latest version of each dataset** ([Eurostat API](https://ec.europa.eu/eurostat/web/user-guides/data-browser/api-data-access/api-introduction)). If raw downloads are not stored immutably, with URL, timestamp, sha256 and a licence tag, a past starting state cannot be rebuilt.

The pipeline runs these stages:

1. `fetch_*` per source
2. `normalise_*` to tidy Parquet with glossary codes
3. `reconcile` national, sector, financial and IO accounts, with explicit discrepancy accounts
4. `synth_population`, taking `scale` as a parameter
5. `firms`: named firms plus a sampled tail
6. `assemble_scenario`
7. `validate`
8. `report`: a diff of key aggregates against the previous build

Use the MIT-licensed `eurostat` client for Eurostat ([PyPI eurostat](https://pypi.org/project/eurostat/)). Validate tables with pandera, and note that on lazy frames it checks only schema-level properties unless you collect them or raise the validation depth ([pandera](https://pandera.readthedocs.io/en/stable/polars.html)). Validate config with pydantic. Ship the derived scenario as a GitHub Release asset, which allows files up to 2 GiB with no total size or bandwidth cap ([GitHub releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)). Keep raw data in a DVC remote and small fixtures in Git LFS.

**One glossary file is the single source of truth for the schema.** Each entry records name, table, dtype, unit, kind (stock, flow or ratio), enum, range, source and `since_version`. A short Jinja generator emits the Rust enums and column structs, Arrow schemas, Polars and pandera models, TypeScript types for the UI, and the docs page. CI reruns `just gen` and fails on any diff. Enum and ledger codes are append-only: never renumber or reuse one, which keeps old saves readable.

Hand-edited parameters live in **TOML**, which is comment-friendly and "designed to map unambiguously to a hash table" ([TOML 1.0](https://toml.io/en/v1.0.0)). Python 3.11+ reads TOML in the standard library ([tomllib](https://docs.python.org/3/library/tomllib.html)). Per-industry tables are CSV and matrices are Parquet. Avoid YAML for anything players edit, because older resolvers turn a county or country code such as `NO` into a boolean. Rust deserialises with `deny_unknown_fields`. Mods are data-only ordered overrides, hashed into the save's `content_hash`. Do not add scripting mods: they break determinism and replay.

For Python bindings, use **PyO3 0.29 and maturin 1.15 with an `abi3` wheel**, which works across Python versions ([PyO3](https://pyo3.rs/main/building-and-distribution.html)). Build it natively on `windows-latest` with maturin-action ([maturin](https://www.maturin.rs/distribution.html)). Return tables through **pyo3-arrow**, which does zero-copy transfer over the Arrow PyCapsule interface and needs no pyarrow at runtime ([pyo3-arrow](https://docs.rs/pyo3-arrow/latest/pyo3_arrow/)). One version trap: pyo3-arrow 0.19 targets arrow-rs 59 while crates.io already has arrow 60. Pin the workspace to the version pyo3-arrow supports, and bump pyo3, arrow and pyo3-arrow together. On the Python side, enforce ruff and pyright in strict mode, because Python's weak typing is where agent errors slip through at runtime.

### Testing and CI run on Linux, with a Windows build to prove determinism

The test stack is cargo-nextest, which does not run doctests, so add `cargo test --doc` ([nextest](https://nexte.st/)). Add proptest and `proptest-state-machine` for invariants, insta for reviewed snapshot diffs of `TickReport`s and explanations ([insta](https://insta.rs/docs/)), and divan for benchmarks ([divan](https://github.com/nvzqz/divan)). Run `cargo-mutants --in-diff` on PR diffs ([cargo-mutants](https://mutants.rs/in-diff.html)). Add golden-run hash tests, where a seed plus a command log produces a hash per tick.

A private repo on GitHub Free gets **2,000 Actions minutes a month**. Linux costs $0.006 a minute, Windows $0.010 and macOS $0.062 ([GitHub billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions)). GitHub's pages no longer say whether Windows uses up included minutes at a multiplier ([rates](https://docs.github.com/en/billing/reference/actions-minute-multipliers)). So run the full suite on Linux for every PR: fmt, clippy `-D warnings`, nextest, cargo-deny, pytest, ruff/pyright, the codegen diff and spec traceability. Run Windows core tests and golden hashes only on PRs that touch `crates/`. Add one job that compares the Linux and Windows hashes byte for byte. That job matters most here because **you develop on Windows and CI runs on Linux**. Leave macOS to a manual or monthly job.

Use `Swatinem/rust-cache@v2` with saving only on `main` ([rust-cache](https://raw.githubusercontent.com/Swatinem/rust-cache/master/README.md)), set `concurrency: cancel-in-progress`, and pin `rust-toolchain.toml`. At about 6 Linux minutes plus 10 Windows minutes per PR, 60 agent PRs a month use about 960 minutes, which fits the free allowance if Windows counts at 1×; check the usage report in your first month, because a 2× Windows multiplier would eat it quickly. Making the repo public would make standard runners free. Whether that suits a commercial game is your call.

### The agent workflow puts every hard rule in a hook or a CI check, never only in prose

Your repo currently has AGENTS.md plus a CLAUDE.md "pointer". That is not enough. Claude Code reads AGENTS.md only when no CLAUDE.md exists, and the documented way to share one file is a **CLAUDE.md that begins with the literal line `@AGENTS.md`**, followed by Claude-only notes ([Claude Code memory](https://code.claude.com/docs/en/memory)). Keep each file under about 200 lines, because "longer files consume more context and reduce adherence", and imported files load at launch too. Codex stops reading instruction files once their combined size reaches **32 KiB** ([Codex AGENTS.md](https://learn.chatgpt.com/docs/agent-configuration/agents-md)). Make the root AGENTS.md a router: where things live, the five or six `just` commands, the hard invariants, and "read `docs/02-design/...` before implementing". Put area rules in nested AGENTS.md files under `crates/`, `python/` and `ui/`, since "the closest AGENTS.md to the edited file wins" ([agents.md](https://agents.md/)). Use `.claude/rules/*.md` files with path globs for rules that apply only to some files, and skills for occasional workflows.

Anthropic's own guidance is that CLAUDE.md is advisory while "hooks are deterministic and guarantee the action happens" ([best practices](https://code.claude.com/docs/en/best-practices)). The same guidance gives the core loop: "give Claude a check it can run". Use separate sessions to write tests and code, and a fresh-context reviewer "so the agent doing the work isn't the one grading it" ([best practices](https://code.claude.com/docs/en/best-practices)).

Protecting tests needs special handling on your plan. **CODEOWNERS on private repos requires GitHub Pro, Team or Enterprise** ([GitHub CODEOWNERS](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners)), and so do rulesets ([GitHub rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets)). As the only owner, a code-owner review rule would also block your own PRs. Use four layers instead:

1. A **PreToolUse hook** that denies agent edits to `tests/golden/**`, `crates/*/tests/acceptance/**` and `schema/**` unless the session is in test-authoring mode.
2. A CI job that fails any PR touching both rule code and its acceptance tests, unless it carries a label you add by hand.
3. A CI check that test and assertion counts never fall and no `#[ignore]` or skip markers appear.
4. Agent permissions that deny `gh pr merge`.

On Free, a red check is only a signal, and you are the one who refuses to merge it. Upgrading to Pro turns these checks into required status checks and adds 1,000 minutes.

## Shipping and licensing: what may go into the box

### Release on itch.io first and Steam later, and keep Steam builds free of self-updaters

Steam Direct costs **$100 per product**, recoupable once the product passes $1,000 in adjusted gross revenue. It requires a coming-soon page for at least two weeks and imposes a waiting period after you pay. The Steam Direct page says 30 days ([Steam Direct](https://partner.steamgames.com/steamdirect)) and the onboarding doc says 21 ([onboarding](https://partner.steamgames.com/doc/gettingstarted/onboarding)), so plan for 30. Start with restricted itch.io pages pushed from CI with **butler**, whose diff uploads save "80% to 95%" of data per push and whose app auto-updates players ([butler](https://itch.io/docs/butler/pushing.html)). Open the Steam page at the vertical slice to collect wishlists.

Tauri builds NSIS or MSI installers. Code signing is not needed for Steam but avoids SmartScreen warnings on direct downloads. Since 2024, OV and EV certificates build SmartScreen reputation the same way, and Azure Artifact Signing is the cheapest route ([Tauri signing](https://v2.tauri.app/distribute/sign/windows/)), although its availability to an individual in the Netherlands is unverified. Leave the Tauri updater out of Steam builds, because Steam's depot updates conflict with self-updating executables. If you use the updater for direct downloads, guard its key: losing it means "you will NOT be able to publish new updates" ([Tauri updater](https://v2.tauri.app/plugin/updater/)).

### IPUMS cannot feed the shipped population, Eurostat can, and BeforeIT stays read-only

The earlier population plan drew households from the IPUMS-International 2011 Romania sample. Its terms say **"Commercial use is strictly prohibited"**, ban redistribution to third parties, and limit use to "teaching and scholarly research" unless the "relevant official statistical authority" gives "explicit written approval" ([IPUMS terms](https://www.ipums.org/about/terms)). A synthetic population derived from those records and shipped in a game for sale is therefore off the table unless **INS Romania approves it in writing**. Write to INS now, since the reply could take months. Meanwhile, build the default pipeline from public aggregates, which Eurostat allows "both for non-commercial and commercial purposes" if you cite Eurostat, state your modifications and add a disclaimer ([Eurostat copyright](https://ec.europa.eu/eurostat/about-us/policies/copyright)). Do not use IPUMS even as a private validation set for the game: game development is not "teaching and scholarly research". It would be legitimate only inside actual academic work, such as a TU Delft project.

The same Eurostat policy has a second trap. Data on countries outside the EU, EFTA and candidate countries must be removed "before reusing them commercially". FIGARO's inter-country tables therefore need non-EU partners collapsed into a rest-of-world block, or you need legal advice. Romania's own input-output tables avoid the issue. INS, BNR, data.gov.ro and Ministry of Finance terms could not be fetched and need checking in a browser.

Make the licence rule mechanical. Each raw dataset's provenance record carries a tag (`eurostat-reuse`, `eurostat-nonEU`, `ins`, `ogl-ro`, `bnr`, `ipums-restricted`), and `assemble_scenario` **fails if any shipped table's lineage includes a non-shippable tag**.

On code, enforce an SPDX allow-list with **cargo-deny**, which denies everything not listed ([cargo-deny](https://embarkstudios.github.io/cargo-deny/checks/licenses/cfg.html)). Allow MIT, Apache-2.0 (also with the LLVM exception), BSD-2/3, ISC, Zlib and Unicode-3.0. Review MPL-2.0 case by case, and deny GPL, AGPL and LGPL. Generate a third-party notices file for the credits.

**BeforeIT.jl is the licence hazard.** GitHub's sidebar says AGPL-3.0 ([GitHub](https://github.com/bancaditalia/BeforeIT.jl)), but the LICENSE file on `main` is Apache-2.0 ([LICENSE](https://raw.githubusercontent.com/bancaditalia/BeforeIT.jl/main/LICENSE)). AGPL is the "strongest copyleft" licence ([choosealicense](https://choosealicense.com/licenses/agpl-3.0/)), and an agent's line-by-line port from Julia to Rust risks becoming a derivative work. Until the maintainers or the file's commit history settle the question, specify mechanics from the Poledna and Glielmo papers, never paste BeforeIT source into an agent prompt, and record provenance in each ADR, for example "Taylor rule per Poledna et al. 2023, eq. X". If it proves to be Apache-2.0, porting with attribution becomes legal.

Real company names carry trademark and defamation risk when firms are shown failing. Keep names in a data file so you can swap to fictional names, and get a lawyer's review before the Steam launch. If you ever add telemetry, GDPR applies.

## Six hard problems and the engineering decisions that tame them

### Calibration and stability: derive most parameters from data, history-match the rest, and monitor stability automatically

Chain two proven workflows. The **Poledna/BeforeIT** approach sets almost every parameter and the whole initial state from national accounts, IO tables, sector accounts and business demography. The result "reproduces exactly the state of the economy in that quarter" ([BeforeIT.jl paper](https://arxiv.org/html/2502.13267)). The **Godley-Lavoie SFC** approach checks a no-shock baseline for drift before applying any shock, which the R package sfcr encodes as `sfcr_baseline` followed by `sfcr_scenario` ([sfcr](https://github.com/joaomacalos/sfcr)).

The Bank of Italy's own team calls its calibration scripts "an unpolished research prototype" that is "not modularly written" ([BeforeIT.jl paper](https://arxiv.org/html/2502.13267)). So make parameter generation its own tested Python package from day one.

The concrete staging runs as follows:

1. Build a balanced base-year SFC dataset.
2. Compute every ratio parameter from it mechanically and reproducibly.
3. Fit AR(1) or small VAR processes for exogenous series.
4. On tick 0, assert every identity, then run one "frozen behaviour" tick that must reproduce base-year flows to the bani.
5. Tune only the roughly 20–60 free behavioural parameters.
6. Run 50–100 quiet years and require every key ratio to stay within ±10% of its base value.
7. Match targets.
8. Freeze the result as a versioned calibration release.

Every new subsystem reruns steps 4 to 8 in CI.

For the tuning itself, use **history matching rather than optimisation**. The game's targets are ranges, such as "investment 2–4× as volatile as GDP", and history matching "iteratively removes regions that cannot satisfy the conditions" without pretending there is one true parameter vector ([hmer](https://cran.r-project.org/web/packages/hmer/vignettes/low-dimensional-examples.html)). Its rule of thumb is training points of at least ten times the number of inputs. Screen the parameters first with Morris in SALib ([SALib](https://salib.readthedocs.io/en/latest/)). Use Optuna's multi-objective samplers only to pick a pleasant point inside the surviving region ([Optuna](https://optuna.readthedocs.io/en/stable/reference/samplers/index.html)). Keep sbi for checking whether key parameters can be identified at all ([sbi](https://sbi-dev.github.io/sbi/latest/)). black-it, the Bank of Italy's toolkit, is an alternative with ready-made method-of-moments losses ([black-it](https://bancaditalia.github.io/black-it/)). hmer is R, so write the implausibility test as about 20 lines on scikit-learn Gaussian-process emulators.

The budget works if a 50-year run takes about 20 s, an assumption the performance spike must confirm. On 16 cores that is about 2,900 runs an hour. Morris on 40 parameters (820 runs) then takes under 20 minutes, and four to six history-matching waves fit in about an hour. Use common random numbers across parameter points. Keyed RNG makes that trivial.

Write the targets as code: a `targets.yaml` with `{id, metric, band, source}` and checker functions. Anchor the lever-response bands in evidence. A meta-analysis of 67 studies finds that prices fall about **0.9% after a 1pp rate hike**, with the trough after **10–20 months in post-transition economies**, against 25–50 months in developed ones ([Havranek & Rusnak](https://www.ijcb.org/journal/v9n4/transmission-lags-monetary-policy-meta-analysis)). CEE impulse responses are "broadly similar" to the euro area's ([Jarociński](https://ideas.repec.org/a/jae/japmet/v25y2010i5p833-868.html)). A test such as R-1 then requires a CPI trough between 8 and 24 months and a peak effect between −0.2% and −1.5%. Romania-specific SVAR studies would tighten these bands and were not found.

Make stability a measured property. A partial-adjustment rule x′ = x + λ(f(x) − x) has the local multiplier **μ = 1 + λ(f′ − 1)**. It is stable when 0 < λ(1 − f′) < 2 and free of sawtooth oscillation when that product is at most 1. Every rule therefore records λ and an estimate of f′, and a unit test fails if μ ≤ 0 or |μ| ≥ 0.95 unless the rule has a waiver.

Every clamp goes through a `clamp_logged!` macro that counts how often it binds. In CI, a baseline run with clamps binding on more than about 0.1% of evaluations is evidence that the model has left its tuned region. Victoria 3 shows why this matters: it ships a clamp of ±75% on prices ([Vic3 Market](https://vic3.paradoxwikis.com/Market)), and its dev-diary index shows investment and trade reworked for years after launch ([Vic3 dev diaries](https://vic3.paradoxwikis.com/Developer_diaries)).

Add cheap monitors to the core that run every tick: NaN and sign checks, growth-rate guards, and a sawtooth detector. A nightly job estimates the one-tick Jacobian of about 30–100 aggregates by finite differences and flags any eigenvalue above 1.

Clear real IO flows within the tick, use rationed search-and-matching for consumer goods and labour as BeforeIT does, and let prices, wages and expectations adjust only between ticks.

### The "why did this change" ledger: five layers, with the rule format doing the heavy lifting

Explanations need different machinery at different depths, so use five layers:

| Layer | Answers | Mechanism | Cost and storage |
|---|---|---|---|
| L1 Ledger | "Why did household deposits fall?" | Every posting carries a flow code. Integer tag × sector accumulators | O(1) per posting. About 230 KB/month with county detail, 5 KB nationally |
| L2 Driver terms | "Why did sector S raise prices?" | Rules are additive (Δ per term) or log-linear (wᵢ·Δln xᵢ, exact) | 10–100 KB/tick |
| L3 Distribution | "Why did poverty rise?" | Shift-share of within-group change vs composition, plus L1 tags of households crossing the line | On demand |
| L4 Counterfactual | "How much is my rate hike vs the energy shock?" | Forked runs with Shapley/Owen attribution over at most 6 lever groups | Offline |
| L5 Events | "What happened?" | Append-only typed events: defaults, clamp hits, policy changes | Small |

Storing raw per-household postings would cost about 22.5 MB a month at 1:100, or 13.5 GB over 50 years. Keep them only in a debug buffer of 1–3 ticks and rebuild them by replay (working notes from a sandbox VM, not published). A sandbox check confirmed that the per-driver log contributions of a multiplicative rule sum to the change in the log of the output to machine precision (working notes from a sandbox VM, not published).

The decisive engineering choice is to make the **decomposable form the only way to write a behaviour rule**. A `behaviour_rule!` macro takes a declaration: named terms, a shape (additive, log-linear or partial adjustment), λ, clamp IDs, and parameter provenance (`from_data`, `fitted` or `tuned`). From that one declaration it generates three things:

- a fast `eval` with no allocation;
- an `eval_explained` that returns the contribution per term;
- a `META` record that feeds the docs, the μ stability test, the causal-graph view and the SALib parameter list.

An agent cannot slip in opaque threshold logic without a `shape: opaque` waiver, and CI counts the waivers. One semantic question is open and needs its own ADR: how to aggregate log-linear contributions across agents of different sizes (LMDI weights in levels, or weighted mean log contributions).

The UI then copies Victoria 3, whose UX team wrote "There is one piece of technology we can not see this game without: Nested Tooltips" ([Vic3 Dev Diary #29](https://forum.paradoxplaza.com/forum/threads/victoria-3-dev-diary-29-user-experience.1506484/)). The core exposes `explain(indicator, region, window) → ContributionTree`. The UI shows the 3–7 largest children plus "other", every child can be hovered for its own breakdown, and hovering any point on a chart shows that period's tree. The invariant **Σ contributions = Δ** holds exactly for L1 and within 1e-9 relative for L2, and is a test.

### Verifying stock-flow consistency in AI-written code: make unbalanced postings unrepresentable, then test against an independent model

Borrow the hard guarantees of production ledgers. TigerBeetle keeps "one invariant (every debit has an equal and opposite credit)" ([TigerBeetle](https://docs.tigerbeetle.com/concepts/debit-credit/)). It also uses integer amounts at a fixed asset scale, a `code` field for the "why", per-currency ledgers, atomic linked transfers, non-negative balance flags, and immutable records corrected only by new entries ([TigerBeetle data modelling](https://docs.tigerbeetle.com/coding/data-modeling/)). Beancount adds periodic balance assertions as "checkpoints" ([beancount](https://beancount.github.io/docs/balance_assertions_in_beancount.html)).

Applied to econ-sim, behaviour code can post only through `econ-ledger`'s typed `Transfer {debit, credit, code, amount > 0}`. A wage is one linked transaction (gross wage, contributions, income-tax withholding, net pay) that applies completely or not at all. RON and EUR are separate ledgers joined through an FX account. Any residual left after reconciling the opening balance sheet goes to a named `9xxx` discrepancy account, never into a silent plug.

Seven invariants run every tick in debug and test builds, and on save and every N ticks in release builds:

1. Every posting is balanced.
2. Account-type constraints hold.
3. The transaction-flow matrix rows and columns sum to zero, and Δstock = Σ flows plus revaluations.
4. Independent balance assertions match: bank deposit liabilities equal the sum of agent deposits, computed by a different code path.
5. Postings are immutable and replay byte-for-byte.
6. Explanation sums equal the observed change.
7. GDP by production, expenditure and income agree exactly in bani.

Invariants catch leaks. They miss *wrong but consistent* economics, such as a tax booked to the wrong sector or a sign error that still balances. The most valuable single test is therefore a **differential reference model**. Run the core in a "SIM mode" (one household sector, one firm sector, government, fixed propensities) and compare it every PR against a roughly 50-line Python implementation of Godley-Lavoie SIM, then PC. Write the reference from the spec, in a separate session from the Rust code. sfcr's `sfcr_validate` offers a third, independent check of transaction-flow matrices ([sfcr_validate](https://joaomacalos.github.io/sfcr/reference/sfcr_validate.html)). This mirrors BeforeIT, whose deterministic Julia version "exactly matches" the original Matlab and checks "that the national income identity holds after every step" ([BeforeIT.jl paper](https://arxiv.org/html/2502.13267)).

Around that reference model sit these layers:

- **State-machine property tests.** `proptest-state-machine` applies random lever sequences, runs `check_invariants` "after every transition" and shrinks failures to a minimal sequence ([proptest](https://proptest-rs.github.io/proptest/proptest/state-machine.html)).
- **Nightly metamorphic tests:**
  - scaling all nominal values by 10 leaves real outputs unchanged;
  - permuting agent IDs leaves aggregates statistically unchanged;
  - a lever set to its current value is bit-identical to the baseline;
  - 1:100 and 1:200 give the same per-capita aggregates within sampling error.
- **Mutation testing.** cargo-mutants swaps operators and replaces function bodies ([mutants](https://mutants.rs/mutants.html)), which exposes hollow tests that agents write.

Run all of it through the **tests-first two-PR flow**:

1. Each spec acceptance criterion gets an ID such as `AC-VAT-03`, with a type tag.
2. A script generates ignored test stubs from the IDs.
3. A test-writer session fills the stubs and you merge that PR.
4. The implementation PR may not touch those tests.
5. An extension of `scripts/docs_index.py` fails CI when a locked spec's criterion has no live test.
6. Any golden snapshot change needs a `CHANGELOG-sim.md` entry giving the economic reason.

### Data integration and licensing: build the population from public tables and make lineage fail the build

Without IPUMS records as seeds, the population has to be synthesised from published cross-tabs. Use the 2021 census tables by county, age, sex, education and activity from INS and the Eurostat Census Hub, household-structure tables, and EU-SILC aggregate tables for income and its distribution. Fit them with multi-way iterative proportional fitting, or with combinatorial optimisation that picks integer records per county. Then assemble households from household-composition tables and impute income and wealth from the published distributions. The research found these methods in the microsimulation literature but did not verify which Romanian 2021 tables exist at county level.

The honest cost is **thinner joint distributions**. Combinations like income × wealth × debt × tenure will rest on assumptions and published marginals rather than observed households. That matters because the earlier report found these joint distributions are what make national ABMs credible.

Mitigate in three ways. Design the pipeline so that the seed source is a swappable stage, so INS approval would simply switch it to IPUMS. Check whether Eurostat's public LFS microdata or EU-SILC public-use files allow commercial reuse; both exist, but their terms were not read ([Eurostat EU-SILC microdata](https://ec.europa.eu/eurostat/web/microdata/european-union-statistics-on-income-and-living-conditions)). Validate the synthetic population against published aggregates such as SILC deciles, poverty rates by household type and county employment.

Reconciliation is the other big work item. Expect GRAS balancing of supply-use tables, sector accounts and financial accounts. Every residual should land in a visible discrepancy account. INS's TEMPO-Online appears to be an undocumented single-page app (unverified), so keep its fetcher isolated, throttled and cached, and prefer the same series via Eurostat or the ECB wherever possible.

### Determinism: a written contract, enforced by clippy, proven by cross-OS hashes

Rust is unusually good for this, if the team follows rules. RFC 3514 guarantees that basic float operations "exactly match IEEE 754-2008", with no flush-to-zero and no implicit fused multiply-add ([RFC 3514](https://github.com/rust-lang/rfcs/blob/master/text/3514-float-semantics.md)). The standard library's `sin`, `exp`, `ln` and `powf` are explicitly "non-deterministic" in precision across "platform, Rust version" ([std f64](https://doc.rust-lang.org/std/primitive.f64.html)). On 200,000 inputs, std and the pure-Rust `libm` crate gave **bit-different results in 6,045 to 20,269 cases** per function. A rayon `par_iter().sum()` over a million floats gave **2–6 different bit patterns per thread count, even between runs**, while fixed 4,096-element chunks combined in order gave one result at every thread count (local experiment on a sandbox VM).

The RNG library is also not a fixed point. rand 0.9 changed the output of `shuffle`, `choose`, `Uniform` and `SmallRng` ([rand changelog](https://github.com/rust-random/rand/blob/master/CHANGELOG.md)). rand's policy says `StdRng` and `SmallRng` "may change output in any release" and that you should "never sample a `usize`" if you need portability ([rand book](https://rust-random.github.io/book/crate-reprod.html)).

The contract, written as `DETERMINISM.md` and enforced where possible, has eight rules:

1. All transcendental maths goes through `econ_num` wrappers around a pinned `libm`.
2. No NaN may reach state; assert `is_finite()` at phase boundaries.
3. Float sums over agents use either integer bani or a fixed-chunk ordered `det_sum`, never a reduction whose order depends on thread count.
4. Parallel code may only write to disjoint indices or return per-chunk partials collected in index order.
5. Iterate only over `Vec`, `BTreeMap` or `IndexMap`, never `std::HashMap`.
6. Sort with `total_cmp` plus an ID tiebreak.
7. Target 64-bit platforms only.
8. Build with default flags: no `target-cpu=native` and no fast-math.

Back the contract with clippy's `disallowed_types` and `disallowed_methods`, set to deny. These ban `HashMap`, `HashSet`, `StdRng`, `SmallRng`, `Instant`, `SystemTime`, `f64::sin`, `exp`, `ln`, `powf`, `powi`, `rand::rng`, `thread_rng`, rayon float `sum` and `reduce`, and `std::env::var` in state code. The lint fires "only if types are defined in the clippy.toml file" ([clippy](https://raw.githubusercontent.com/rust-lang/rust-clippy/master/clippy_lints/src/disallowed_types.rs)), so verify that the paths resolve when you first configure it.

For randomness, use **ChaCha8 from `rand_chacha`**. It is maintained by the rust-random organisation and covered by its portability policy. It also offers `set_stream` for "2^64 unique streams … per seed" and `set_word_pos` for random access ([rand_chacha](https://docs.rs/rand_chacha/latest/rand_chacha/struct.ChaCha8Rng.html)), and seeking matched sequential generation exactly in the sandbox. Derive the key from (master seed, stream ID), the stream from the entity ID, and the position from the tick. Write your own distributions on `libm`. Pin known-answer vectors in a test so that any dependency bump that changes values fails CI, and treat every `rand_*` upgrade as a re-golden event.

Money is `Bani(i64)` with checked arithmetic that panics on overflow in every build, using `i128` for products of an amount and a rate. Rates stay `f64` and convert to bani through one named rounding function, and splits use largest remainder with an index tiebreak so that parts always sum exactly. Enable clippy's `arithmetic_side_effects` and `cast_possible_truncation` lints in the money crates ([clippy lints](https://rust-lang.github.io/rust-clippy/master/index.html#arithmetic_side_effects)).

For the 80×80 Leontief solve, write a scalar partial-pivot LU of about 60 lines. faer is fast, but it dispatches SIMD kernels by CPU at runtime, so operation order could differ between your laptop and CI ([faer](https://docs.rs/faer/latest/faer/)). Keep faer as a test oracle.

Start single-threaded. A monthly tick at 1:100 costs milliseconds, and rayon can come later behind a feature flag, because keyed RNG and disjoint writes already allow it. The per-tick state hash goes into every `TickReport`, and CI compares it across Linux and Windows and across thread counts.

### Population scale: one number in the scenario manifest, and nothing else may know it

You want the scale to be a parameter that nothing hardcodes. The architecture supports this, provided a few rules are written down.

**Scale lives in one place.** It is `scale` in the pipeline's `params.yaml` and `scale_factor` in the scenario manifest. It is copied into every save, history block and export header, so history recorded at 1:100 is never silently mixed with 1:20. No array is sized by population: every table is a `Vec` whose capacity comes from the scenario.

**Every record carries an integer weight**, the number of persons or households it represents, rather than the code multiplying by a global constant. This is my own synthesis, and it has two benefits. Integer weights keep scaled-up money exact in `i128` sums. Per-record weights also allow oversampling small counties later, to cut noise, without any code change.

The subtle accounting point needs its own ADR. Weighted households transact with banks, government and named firms that are not weighted. Record-level balances should therefore mean "per represented unit", and every posting between a weighted record and a sector-level counterparty is multiplied by the integer weight at posting time. Then the sum of weighted household deposits equals bank deposit liabilities exactly, and the ledger invariants hold at every scale.

**Scale-independent stores stay that way.** The aggregation cube has the same 34k cells at any scale, and the aggregated ledger has no per-household dimension. Saves grow linearly with N, but even at 1:10 they stay far below Steam Cloud's 100 MB per write. The earlier estimates still hold: about 4–12 ms per tick at 1:100 and 40–120 ms at 1:10. 1:1 needs 10–20 GB of RAM (earlier benchmark on a sandbox VM; see the [population research](population-modelling-deep-research.md)).

**CI proves that nothing is hardcoded** by building and running the tiny scenario at three scales, such as 1:1000, 1:200 and 1:100. It asserts that per-capita aggregates agree within sampling error and that every invariant passes. The group explorer always shows how many records sit behind a view.

At small counts, demographic and labour transitions use **logit-scaling alignment**, which JAS-mine calls "the clear choice" for multi-outcome events ([JAS-mine](https://www.microsimulation.ac.uk/jas-mine/resources/cookbook/alignment/)). Rounding error carries into the next period, as with LIAM2's `errors="carry"` ([LIAM2](https://liam2.readthedocs.io/en/stable/processes.html)). Without that carry, a county's 0.3 expected births a month would be rounded away every month.

## Decision table for ADR-0004 onward

> **Repo note (2026-10-09):** the ADR numbers in this table and in the spike table below are the report's draft numbering. The repo consolidates them into twelve ADRs, 0004–0015; see the [architecture overview](../../03-architecture/README.md) for the mapping. The population-scale and data-source rows (0011, 0012 here) were folded into an amended [ADR-0003](../../03-architecture/decisions/0003-people-representation.md).

ADR-0001 to 0003 already exist. ADR-0003's "Generation" row must be amended to remove IPUMS as the shipped seed source.

| ADR | Decision | Choice | Rejected, and why | Revisit when |
|---|---|---|---|---|
| 0004 | Core language and architecture | Rust workspace (`econ-types`, `-rng`, `-num`, `-ledger`, `-core`, `-mechanics-*`, `-io`, `-py`, `-cli`). Column tables, `Phase` trait, `const` phase order, no async, single-threaded by default | C# (CRT-dependent maths, weaker Python bindings); C++ (undefined behaviour, implicit FMA); ECS or krABMaga (fit heterogeneous agents, not columns) | Compile times stall the agent loop despite crate splits |
| 0005 | Determinism contract | `DETERMINISM.md` with 8 rules; clippy deny-lists; `libm` only; fixed-chunk sums; ordered collections; 64-bit targets; pinned toolchain; Linux↔Windows hash CI | Tolerance-based "close enough" replay (breaks golden tests and bug repro) | Never relaxed; extended when adding parallelism |
| 0006 | Randomness | ChaCha8 (`rand_chacha`) keyed by (seed, stream, tick, entity); own `libm` distributions; known-answer test vectors; rand bumps are re-golden events | `StdRng`/`SmallRng`, `rand_distr`, shared sequential generator | `rand_chacha` deprecated, then switch to self-written Philox with Random123 vectors |
| 0007 | Money and numerics | `Bani(i64)` checked, `i128` intermediates, one rounding function, largest-remainder split; `f64` rates; hand LU for IO | `rust_decimal`/`fixed` (overhead, pre-2.0); faer/LAPACK in the core (CPU-dependent kernels) | IO grows past ~500 sectors |
| 0008 | Ledger and accounting | Typed transfers (amount > 0, debit/credit), append-only `u16` codes in ranges per module, linked legs, RON/EUR ledgers with FX account, discrepancy accounts, invariants I-1 to I-7, aggregated postings only | Per-household posting log (13.5 GB per 50 years); silent plugs | Code count or sector sparsity differs a lot from the ~150-code estimate |
| 0009 | Explainability | Layers L1–L5; `behaviour_rule!` macro as the only rule form; opaque-rule waivers counted in CI; `explain() → ContributionTree`; Σ = Δ test | Per-agent provenance; runtime Shapley; interpreted rule trees (10–100× slower) | — |
| 0010 | Aggregating log-linear contributions | LMDI weights in levels vs weighted mean log contributions, decided per indicator | — (open) | Settled by Spike 3 |
| 0011 | Population scale as a parameter | `scale` in params and manifest; integer per-record weights; weighted↔sector postings multiplied at posting time; no population-sized arrays; scale in every header; 3-scale CI test | Global constant multiplier; fixed 1:100 | Never; scales above 1:10 need a memory review |
| 0012 | Population data sources (amends 0003) | Synthesise from public 2021 census cross-tabs and SILC aggregates via IPF or combinatorial optimisation; seed stage swappable; IPUMS excluded unless INS approves in writing | IPUMS 2011 seed (licence forbids commercial use) | INS approval arrives, or public microdata terms permit reuse |
| 0013 | Runtime storage | No embedded DB; column vectors; bitmask group explorer; arrow-rs at the I/O boundary only | Polars or DuckDB in the game (12–20 min clean builds, 39–65 MB); SQLite (row store) | A player-facing SQL console becomes a feature |
| 0014 | Saves and history | ZIP of manifest + Arrow IPC zstd per table + postcard command log; per-campaign immutable yearly history blocks; named migrations; replay only on matching engine and content hash; golden save corpus | Monolithic save with history (Steam Cloud 100 MB/write); bincode (dead); rkyv (no schema evolution) | Autosave of long campaigns gets slow; then consider SQLite container |
| 0015 | Single schema | Glossary file → codegen for Rust, Python, TypeScript and docs; CI diff check; append-only enums | JSON Schema as master (no column dtypes); hand-kept parallel types | — |
| 0016 | Offline pipeline | uv, Polars, DuckDB, just (front door) + DVC (DAG, cache, remote), pandera + pydantic, raw archive with provenance and licence tags, lineage guard | Make (timestamps, Windows); Snakemake (second DSL) | — |
| 0017 | Config and modding | TOML knobs, CSV tables, Parquet matrices, `deny_unknown_fields`, data-only mods hashed into `content_hash` | YAML for player-edited files; Lua/WASM mods | Modders demand logic, then a sandboxed, versioned design |
| 0018 | Python bindings | PyO3 0.29 + maturin abi3 wheels, pyo3-arrow, lockstep version bumps | pyo3-polars (huge compile, version coupling) | — |
| 0019 | Calibration workflow | 8 stages from base-year SFC dataset to frozen calibration release; Morris → history matching → optional Optuna; parameter provenance in rule `META`; targets as code | Single-point optimisation; full Bayesian posterior as the default | Targets cannot be met; then revisit model structure, not tuning |
| 0020 | Stability engineering | μ test per rule, `clamp_logged!` with CI threshold, per-tick monitors, nightly Jacobian eigenvalues; markets: IO within tick, rationed matching, lagged prices | Walrasian iteration over agents per tick | — |
| 0021 | Verification strategy | Unit + property + state machine + metamorphic + differential (Python SIM/PC) + golden + mutation (`--in-diff`) + nightly validation | Invariants alone (miss wrong-but-balanced bugs) | — |
| 0022 | UI stack | Tauri 2 + React/TS + uPlot + ECharts; core in-process; binary `ipc::Response` and `Channel`; `api.ts` adapter; egui dev inspector | Godot, Unity, Bevy, ImGui (charts, tables and forms all custom); Plotly (slow) | WebView2 or Steam overlay fails; then move to Electron |
| 0023 | Map data | geoBoundaries ADM1 (CC BY 4.0), pre-projected in the pipeline, credit line | GADM (no commercial use); GISCO (terms unverified) | GISCO terms verified as commercial-friendly |
| 0024 | Agent workflow | Root `AGENTS.md` as router (under 200 lines); `CLAUDE.md` = `@AGENTS.md` + extras; nested AGENTS.md; path-scoped rules; hooks for protected paths; tests-first two-PR flow; AC-ID traceability; fresh-context reviewer | Rules in prose only; one big instruction file | Agents repeatedly bypass a rule, then turn it into a hook or CI check |
| 0025 | CI and hosting | GitHub Actions: full Linux per PR; Windows on `crates/` changes; cross-OS hash job; macOS manual; rust-cache; cancel-in-progress. Decide Free vs Pro (enforcement) vs public repo | Full three-OS matrix on every PR (minutes budget) | Minutes run out, or enforcement is needed |
| 0026 | Distribution | itch.io via butler first; Steam page at vertical slice; Tauri NSIS; signing only for direct downloads; no self-updater in Steam builds | Steam-first; custom updater everywhere | — |
| 0027 | Licensing policy | cargo-deny allow-list; notices file; BeforeIT treated as AGPL (papers only, provenance in ADRs); data attribution screen; firm names in a swappable data file; legal review before Steam | Porting BeforeIT code; hardcoded real names | BeforeIT licence confirmed Apache-2.0 |

## First build sequence: nine spikes, ordered by what is hardest to retrofit

Run a legal and admin track in parallel from day one. Email INS about IPUMS-derived commercial use. Ask the BeforeIT maintainers which licence applies, or read the LICENSE file's commit history. Read the INS, data.gov.ro, BNR and GISCO terms in a browser. Then run the spikes below. Spikes 1–3 come first because they lock the decisions that are most expensive to change later. Spikes 4–6 can overlap. Spike 7 is the first time real Romanian data enters, and the UI comes late because the core's API defines what it shows.

| # | Spike | What gets built | Exit criterion | Settles |
|---|---|---|---|---|
| 0 | Repo hygiene (1–2 days) | `CLAUDE.md` with `@AGENTS.md`; root AGENTS.md router; justfile; `rust-toolchain.toml`; empty workspace; cargo-deny; CI skeleton with path filters; PreToolUse hook for protected paths | An agent session on Windows runs `just check` green and is blocked from editing `tests/golden/` | 0024, 0025 |
| 1 | Numeric foundation | `econ-types`, `econ-rng`, `econ-num`: `Bani`, split, rounding, ChaCha8 keying plus known-answer vectors, `det_sum`, `libm` wrappers, clippy deny config with paths verified | Same hashes on your Windows laptop and Linux CI; clippy rejects `f64::exp` and `HashMap` in core | 0005–0007 |
| 2 | Ledger + differential SIM | `econ-ledger` with typed transfers and I-1 to I-7; core "SIM mode"; independent Python Godley-Lavoie SIM, then PC | Trajectories match exactly in bani for 200 ticks; a mutated sign in a posting is caught by the differential test, not only by invariants | 0008, 0021 |
| 3 | Explainability | `behaviour_rule!` macro on 3 rules (price, wage, consumption); `explain()` tree; Σ = Δ test; μ metadata test | Nested tree for one indicator sums exactly; settle the log-linear aggregation question | 0009, 0010, 0020 (part) |
| 4 | Performance and market clearing at variable scale | Fake (not real-data) population generator with integer weights; labour matching, goods market, 80×80 hand LU, cube fill; `econ-cli` | Tick time measured at 1:1000, 1:100 and 1:10 on your laptop; 50-year run ≤ ~30 s at 1:100; 3-scale metamorphic test passes | 0011, 0004, 0013 |
| 5 | Calibration loop | `econ-py` bindings; 600-tick runs from Python; Morris on ~10 parameters; one history-matching wave with GP emulators; 100-year quiet-baseline check | Runs per hour measured on your machine; budget for full calibration known | 0018, 0019 |
| 6 | Saves and replay | ZIP + manifest + Arrow IPC tables + command log; one example migration; replay-equals-snapshot test; history year block | Load, replay and state-hash check pass; save size at 1:100 and 1:10 recorded | 0014, 0015 (part) |
| 7 | Data pipeline slice | Glossary codegen; Eurostat fetch with provenance and licence tags; DVC; population synthesised from public 2021 tables for 2–3 counties via IPF; lineage guard | A scenario builds end to end, and the build fails when an `ipums-restricted` file is put into its lineage | 0012, 0015, 0016 |
| 8 | UI slice | Tauri 2 + React; 40 uPlot charts × 600 months; ECharts 42-county choropleth from geoBoundaries; one nested "why" tooltip from `explain()` | IPC round trip for 1–5 MB measured on WebView2; cold start and memory acceptable on your laptop | 0022, 0023 |
| 9 | Agent workflow dry run | One real mechanic spec (e.g. VAT) through the tests-first two-PR flow with traceability check and `cargo-mutants --in-diff` | Spec → tests → implementation merged with no hand edits to protected paths; friction points logged | 0021, 0024 |

## Conclusion

Most of this stack follows from one principle: every rule that matters should fail a build, not just sit in a document. Determinism is a clippy deny-list plus a cross-OS hash. Explainability is a macro that cannot express an opaque rule. Accounting is a transfer type that cannot be unbalanced. Licensing is a lineage tag that fails `assemble_scenario`. Scale-independence is a CI run at three scales. This matters more than usual because the main author is a fleet of agents that read AGENTS.md selectively and can be talked into weakening tests. Your own GitHub plan does not enforce protections on a private repo, so the guarantees have to live in hooks, types and CI.

The IPUMS finding also changes what "realistic people" can mean in a game sold commercially. The population's attributes will be as rich as before, but their *joint* distribution will rest on public cross-tabs and assumptions rather than observed households, unless INS says yes. That makes the INS email and the swappable seed stage two of the cheapest high-value actions available. It also means the validation suite, not the generator, is what will show players and reviewers that the people of Romania behave believably.
