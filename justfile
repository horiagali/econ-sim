# econ-sim task runner. Install `just`: `winget install Casey.Just` (Windows)
# or `cargo install just`. Run `just` to list recipes.

set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

python := if os_family() == "windows" { "python" } else { "python3" }

# List recipes
default:
    @just --list

# Everything CI runs on every PR. Must be green before a PR is opened.
check: fmt-check lint test lint-canary docs codegen-check trace pipeline-test hook-test test-guard diff-sim

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

# Differential test: Rust SIM and PC vs the independent Python references, exact in bani
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

# Assemble the starting state from the fixtures with the Python references and check that its parts agree (interim, ADR-0018)
scenario-report sample_scale="1000":
    {{python}} python/pipeline/build_scenario.py --sample-scale {{sample_scale}}

# Daily-clock spike (ADR-0017): a simulated year with daily ticks against monthly ticks
bench-days:
    cargo run -q --release -p econ-cli -- bench-days

# Starting population from census margins (the pipeline's synth_population stage), all stages; writes Arrow tables to `out`
synth-population sample_scale out="python/pipeline/data/population" fixtures="python/pipeline/fixtures" seed="42":
    cargo run -q --release -p econ-cli -- synth-population --margins {{fixtures}}/census2021_margins_ro.json --attributes {{fixtures}}/census2021_edu_activity_ro.json --jobs {{fixtures}}/census2021_jobs_ro.json --housing {{fixtures}}/census2021_housing_ro.json --wages {{fixtures}}/earnings2021_ro.json --pensions {{fixtures}}/pensions2021_ro.json --wealth {{fixtures}}/household_wealth2021_ro.json --incomes {{fixtures}}/other_incomes2021_ro.json --employers {{fixtures}}/firm_units2021_ro.json --sample-scale {{sample_scale}} --seed {{seed}} --out {{out}}

# Build the Romania scenario at one sample scale (spec game/scenario-builder): tables, opening balances and report.json in `out`
build-scenario sample_scale out="python/pipeline/data/scenario":
    cargo run -q --release -p econ-cli -- build-scenario --sample-scale {{sample_scale}} --out {{out}}

# Headless play (spec game/lever-system): a commands file on the 1:1000 Romania scenario, seed 42, five years; outputs in `out`
play file out="target/play/out":
    cargo run -q --release -p econ-cli -- build-scenario --sample-scale 1000 --out target/play/scenario_1000
    cargo run -q --release -p econ-cli --features levers -- play --scenario target/play/scenario_1000 --commands {{file}} --until 2026-11-30 --seed 42 --out {{out}}

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

# Regenerate code from the glossary (ADR-0012) and the lever registry (ADR-0018)
codegen:
    {{python}} scripts/codegen_glossary.py
    {{python}} scripts/codegen_levers.py

# Fail if generated glossary or lever code is stale
codegen-check:
    {{python}} scripts/codegen_glossary.py --check
    {{python}} scripts/codegen_levers.py --check

# Every acceptance criterion in a spec has a live test, and no test cites an unknown ID (ADR-0014)
trace:
    {{python}} scripts/traceability.py

# Generate ignored acceptance-test stubs for a spec's criteria (never overwrites existing tests)
ac-stubs spec prefix out:
    {{python}} scripts/ac_stubs.py {{spec}} {{prefix}} {{out}}

# Mutation testing for one crate (needs `cargo install cargo-mutants`)
mutants crate:
    cargo mutants -p {{crate}}

# The policy suite's mutation check (AC-POL-11): each sign mutant MUT-1..3 must fail its scenario at 1:1000 (needs cargo-mutants)
policy-mutants *which:
    {{python}} scripts/policy_mutants.py {{which}}

# Data pipeline unit tests (offline)
pipeline-test:
    {{python}} -m unittest discover -s python/pipeline/tests

# Unit tests for the protect-paths hook and the test guard (ADR-0014)
hook-test:
    {{python}} -m unittest discover -s scripts/tests

# Tests and assertions may not fall, nor tests be newly ignored, against the merge base with origin/main (ADR-0014 layer 3)
test-guard:
    {{python}} scripts/test_guard.py
