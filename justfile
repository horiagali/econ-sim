# econ-sim task runner. Install `just`: `winget install Casey.Just` (Windows)
# or `cargo install just`. Run `just` to list recipes.

set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

python := if os_family() == "windows" { "python" } else { "python3" }

# List recipes
default:
    @just --list

# Everything CI runs on every PR. Must be green before a PR is opened.
check: fmt-check lint test lint-canary docs codegen-check trace pipeline-test hook-test diff-sim

# Format all Rust code
fmt:
    cargo fmt --all

# Fail if Rust code is not formatted
fmt-check:
    cargo fmt --all -- --check

# Clippy with warnings as errors (includes the determinism deny-lists)
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Unit, property and invariant tests
test:
    cargo test --workspace

# Prove the determinism deny-lists are active: clippy must REJECT the canary
lint-canary:
    {{python}} scripts/lint_canary.py

# Validate doc frontmatter and the docs index
docs:
    {{python}} scripts/docs_index.py --check

# Regenerate docs/INDEX.md after changing docs
docs-index:
    {{python}} scripts/docs_index.py

# Run the SIM model (Spike 2) and print a CSV
sim ticks="200":
    cargo run -q -p econ-cli -- sim --ticks {{ticks}}

# Differential test: Rust SIM vs the independent Python reference, exact in bani
diff-sim:
    cargo build -q -p econ-cli
    {{python}} python/reference/diff_sim.py

# Print the per-tick state hashes of the SIM golden run
golden-print:
    cargo run -q -p econ-cli -- sim --ticks 200 --hashes

# Check the SIM golden run against the committed hash file
golden-check:
    cargo run -q -p econ-cli -- sim --ticks 200 --check-golden tests/golden/sim_200.hashes

# Performance spike (Spike 4): tick time at several population scales
bench-scale:
    cargo run -q --release -p econ-cli -- bench-scale

# Starting population from census margins (the pipeline's synth_population stage), all stages; writes Arrow tables to `out`
synth-population sample_scale out="python/pipeline/data/population" fixtures="python/pipeline/fixtures" seed="42":
    cargo run -q --release -p econ-cli -- synth-population --margins {{fixtures}}/census2021_margins_ro.json --attributes {{fixtures}}/census2021_edu_activity_ro.json --jobs {{fixtures}}/census2021_jobs_ro.json --housing {{fixtures}}/census2021_housing_ro.json --sample-scale {{sample_scale}} --seed {{seed}} --out {{out}}

# Run the scale world on the Romanian starting population (census households and ages; jobs and wages still invented)
sim-romania sample_scale="100" ticks="12" margins="python/pipeline/fixtures/census2021_margins_ro.json":
    cargo run -q --release -p econ-cli -- sim-population --margins {{margins}} --sample-scale {{sample_scale}} --ticks {{ticks}}

# Licence and dependency checks (needs `cargo install cargo-deny`)
deny:
    cargo deny check

# Save size and save/load/verify time at several scales (Spike 6)
bench-save:
    cargo run -q --release -p econ-cli -- bench-save

# Calibration loop from Python (Spike 5). Needs `uv`; builds the econ_py bindings.
calib-spike:
    uv --directory python/calib run python calibration_spike.py

# Regenerate code from the glossary (ADR-0012)
codegen:
    {{python}} scripts/codegen_glossary.py

# Fail if generated glossary code is stale
codegen-check:
    {{python}} scripts/codegen_glossary.py --check

# Every acceptance criterion in a spec has a live test, and no test cites an unknown ID (ADR-0014)
trace:
    {{python}} scripts/traceability.py

# Generate ignored acceptance-test stubs for a spec's criteria (never overwrites existing tests)
ac-stubs spec prefix out:
    {{python}} scripts/ac_stubs.py {{spec}} {{prefix}} {{out}}

# Mutation testing for one crate (needs `cargo install cargo-mutants`)
mutants crate:
    cargo mutants -p {{crate}}

# Data pipeline unit tests (offline)
pipeline-test:
    {{python}} -m unittest discover -s python/pipeline/tests

# Unit tests for the protect-paths hook (ADR-0014)
hook-test:
    {{python}} -m unittest discover -s scripts/tests
