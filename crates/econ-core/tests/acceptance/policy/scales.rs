//! AC-POL-10: the suite at three scales (spec game/policy-scenarios, "Three
//! scales").

use super::harness::{placebo_failures, placebo_split_failure, prepare, scenario_failures};
use super::{all, wired};

/// Every wired scenario's checked expectations, and the placebo against
/// every scenario's checked bands, at one scale. Returns the failures.
fn failures_at(scale: u32) -> Vec<String> {
    let scenarios = wired();
    prepare(&[scale], &scenarios);
    let mut failed = Vec::new();
    for s in &scenarios {
        failed.extend(scenario_failures(s, scale));
    }
    failed.extend(placebo_split_failure(scale));
    failed.extend(placebo_failures(&all(), scale));
    failed
}

fn assert_scale(scale: u32) {
    let failed = failures_at(scale);
    assert!(
        failed.is_empty(),
        "AC-POL-10 at 1:{scale}: {} failure(s):\n{}",
        failed.len(),
        failed.join("\n")
    );
}

/// **AC-POL-10** `[sim]` Three scales: AC-POL-01 to 09 pass at 1:1000, 1:100
/// and 1:10 (1:10 in the nightly run). Every run (baseline, placebo,
/// scenarios) keeps invariants I-1 to I-8 and person conservation every
/// day. At each scale, the placebo's measured differences are below every
/// checked band's lower bound in absolute value.
///
/// Readings: this test is 1:1000 and 1:100 (every PR); the 1:10 run is the
/// ignored test below, for the nightly run (owner, 2026-10-25). "Pass" is
/// as in `policy/mod.rs`; the scenarios run are those wired in this build
/// (`policy::wired`: all nine once the four lever features are on, which
/// the PRs that wire the levers add to `default`). The invariants and person
/// conservation are checked on every day of every run by the harness. "The
/// band's lower bound in absolute value" is the band's edge nearest zero
/// (`min(|lo|, |hi|)`): the placebo's measured difference, by the check's
/// kind and window, is strictly below it in absolute value, for every
/// checked expectation of all nine scenarios (the placebo needs no lever
/// but VAT's). The noise floor is "measured, not assumed" (spec, "Three
/// scales") from a placebo that "splits the paths" (spec, "Placebo and
/// noise floor"), so the test also requires the placebo's months after T0
/// to differ from the baseline's somewhere (statistics, trade, budget,
/// exchange rate, credibility or prices): a placebo identical to the
/// baseline would make every floor zero and the 2ν rule empty.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the policy suite runs in release builds only: cargo test --release -p econ-core --test acceptance -- --test-threads=1 policy::"
)]
fn ac_pol_10() {
    let mut failed = failures_at(1000);
    failed.extend(failures_at(100));
    assert!(
        failed.is_empty(),
        "AC-POL-10: {} failure(s):\n{}",
        failed.len(),
        failed.join("\n")
    );
}

/// **AC-POL-10** `[sim]` at 1:10: every wired scenario passes, every run
/// keeps the checks of every day, and the placebo stays below every checked
/// band. Ignored for CI time; the nightly run includes it.
#[test]
#[ignore = "1:10 runs nightly (spec game/policy-scenarios, Three scales); run with --ignored"]
fn ac_pol_10_one_in_ten() {
    assert_scale(10);
}
