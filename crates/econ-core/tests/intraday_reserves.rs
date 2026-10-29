//! Regression: a bad economy must not crash the run (spec
//! economy/money-banking, step 5, the intraday advance of 2026-10-29).
//!
//! Under the Romania scenario's own law (`law.json` and `world.json`) at
//! 1:1000, the economy turns unstable after a few years (a calibration
//! matter, not this test's) and on 2029-01-31 a day's VAT came to more than
//! the banks' whole stock of reserves: the goods market's commit panicked
//! with `WouldGoNegative { Banks, Reserves }`. The central bank now advances
//! the shortfall inside the batch. This run goes past that day and checks
//! the ledger's invariants every day and that advances were drawn.
//!
//! The helpers replicate those of the acceptance suite's `support` and
//! `support_lev` (not imported: they are protected test files).
#![cfg(feature = "levers")]

use std::path::PathBuf;

use econ_core::levers::Registry;
use econ_core::world::{World, WorldParams};
use econ_io::commands::read_scenario_law;
use econ_io::read_start_state;
use econ_ledger::{Instrument, Sector};
use econ_scenario::{BuildParams, build, read_inputs, write};
use econ_types::{Bani, Date};

const SEED: u64 = 42;
const SCALE: u32 = 1000;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The Romania scenario at 1:1000 under the law of its folder, spec
/// defaults, seed 42 (as the policy harness builds it).
fn scenario_world() -> World {
    let fixtures = repo_root().join("python/pipeline/fixtures");
    let io_table = fixtures.join("io_table2021_ro.json");
    let inputs = read_inputs(&fixtures).expect("the fixtures read");
    let scenario = build(
        &inputs,
        &BuildParams {
            sample_scale: SCALE,
            rng_seed: SEED,
        },
    )
    .expect("the fixtures build");
    let parent = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("econ-core-intraday-reserves");
    if parent.exists() {
        std::fs::remove_dir_all(&parent).expect("clean the test folder");
    }
    std::fs::create_dir_all(&parent).expect("create the test folder");
    let dir = parent.join("scenario");
    write(&scenario, &inputs, &dir).expect("the scenario writes");
    let start = read_start_state(&dir, &io_table).expect("the scenario reads as a starting state");
    let registry = Registry::load(&repo_root().join("crates/econ-core/levers.toml"))
        .expect("the committed registry loads");
    let law = match read_scenario_law(&dir, &registry) {
        Ok(law) => law,
        Err((line, r)) => panic!(
            "the scenario's law.json and world.json read: line {line}: {:?} {}",
            r.code, r.message
        ),
    };
    World::with_law(
        start,
        law.registry.clone(),
        law.law.clone(),
        WorldParams::spec_defaults(SEED),
    )
    .expect("the world builds under the scenario's law")
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a seven-year run at 1:1000: release builds only"
)]
fn a_day_of_taxes_above_the_banks_reserves_is_advanced_not_a_crash() {
    let mut w = scenario_world();
    // Past the day that panicked (2029-01-31), into the next month.
    let last = Date::new(2029, 3, 31).expect("a date");
    let mut most_advanced = Bani::ZERO;
    while w.date() <= last {
        let r = w.step(&[]).expect("a day with no command runs");
        assert!(
            r.checks_ok && r.failed.is_empty(),
            "{:?}: checks failed: {:?}",
            r.date,
            r.failed
        );
        if let Err(e) = w.ledger().check_invariants() {
            panic!("{:?}: ledger invariants: {e:?}", r.date);
        }
        let l = w.ledger();
        assert!(
            l.balance(Sector::Banks, Instrument::Reserves) >= Bani::ZERO,
            "{:?}: the banks' reserves below zero",
            r.date
        );
        let advances = Bani::ZERO - l.balance(Sector::Banks, Instrument::CbAdvances);
        most_advanced = most_advanced.max(advances);
    }
    assert!(
        most_advanced > Bani::ZERO,
        "the run never drew a central bank advance"
    );
}
