//! AC-POL-11: the mutation check (spec game/policy-scenarios, "The mutation
//! check").

use super::harness::{prepare, scenario_failures};
use super::wired;

/// **AC-POL-11** `[sim]` Mutation: each mutant of the mutation table (MUT-1
/// to 3) makes its named scenario fail at 1:1000. The unmutated build passes
/// all nine.
///
/// Readings: the mechanism is a `just policy-mutants` recipe running
/// cargo-mutants on the three named functions (owner's delegation,
/// 2026-10-25). This test is its unmutated half and its test command: on
/// the unmutated build every wired scenario passes at 1:1000 (all nine once
/// the four lever features are on). The recipe runs each mutant against
/// its named scenario's test (MUT-1 `fx_step`'s interest-parity sign
/// against `ac_pol_01`, MUT-2 the purchaser price's VAT sign against
/// `ac_pol_03`, MUT-3 the monetary-financing term's sign against
/// `ac_pol_05`) and fails unless each of the three is caught; cargo-mutants'
/// own baseline run is the unmutated build passing.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the policy suite runs in release builds only: cargo test --release -p econ-core --test acceptance -- --test-threads=1 policy::"
)]
fn ac_pol_11() {
    let scenarios = wired();
    prepare(&[1000], &scenarios);
    let mut failed = Vec::new();
    for s in &scenarios {
        failed.extend(scenario_failures(s, 1000));
    }
    assert!(
        failed.is_empty(),
        "AC-POL-11: the unmutated build fails {} checked expectation(s) at 1:1000:\n{}",
        failed.len(),
        failed.join("\n")
    );
}
