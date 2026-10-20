//! Acceptance test for AC-FIS-04 (spec: economy/fiscal-policy), the `[unit]`
//! criterion, against the API sketch. Written in a test-authoring session
//! before the code existed (ADR-0014). The spec places its tests in
//! `econ-core`; this one needs no `World`, so it sits with the function it
//! tests, as the `[unit]` criteria of the M2 mechanics do.
//!
//! Rates in millionths a year; the parameters are the spec's (term premium
//! 1.5 points, risk_0 20 points per unit of debt-to-GDP squared, threshold
//! 60% of GDP), written out here.

use econ_mech_fiscal::{DebtParams, bond_yield_ppm};
use econ_types::Bani;

fn params() -> DebtParams {
    DebtParams {
        new_issue_years: 7,
        term_premium_ppm: 15_000,
        risk_ppm: 200_000,
        threshold_ppm: 600_000,
    }
}

fn bn(lei: i64) -> Bani {
    Bani::from_lei(lei * 1_000_000_000)
}

/// **AC-FIS-04** `[unit]` The yield is the formula of step 4; it never falls
/// as debt rises.
#[test]
fn ac_fis_04() {
    let p = params();
    let gdp = bn(1_000);
    let policy = 17_500;
    // r = i + term + risk_0 · max(0, B/Y − 0.6)², on values it gives exactly.
    assert_eq!(
        bond_yield_ppm(policy, Bani::ZERO, gdp, &p),
        32_500,
        "no debt"
    );
    assert_eq!(
        bond_yield_ppm(policy, bn(500), gdp, &p),
        32_500,
        "50% of GDP"
    );
    assert_eq!(
        bond_yield_ppm(policy, bn(600), gdp, &p),
        32_500,
        "at the threshold"
    );
    assert_eq!(
        bond_yield_ppm(policy, bn(900), gdp, &p),
        50_500,
        "90%: + 0.2 · 0.09"
    );
    assert_eq!(
        bond_yield_ppm(policy, bn(1_200), gdp, &p),
        104_500,
        "120%: + 0.2 · 0.36"
    );
    assert_eq!(
        bond_yield_ppm(policy, bn(1_100), gdp, &p),
        82_500,
        "110%: + 0.2 · 0.25"
    );
    // The policy rate passes through one for one.
    assert_eq!(bond_yield_ppm(27_500, bn(900), gdp, &p), 60_500);
    assert_eq!(bond_yield_ppm(-5_000, bn(500), gdp, &p), 10_000);
    // The ratio, not the scale: the same debt over GDP gives the same yield.
    assert_eq!(bond_yield_ppm(policy, bn(450), bn(500), &p), 50_500);
    // It never falls as debt rises, below and above the threshold.
    let mut last = i32::MIN;
    for billions in 0..=3_000i64 {
        let debt = bn(billions);
        let y = bond_yield_ppm(policy, debt, gdp, &p);
        assert!(y >= last, "debt {debt:?}: {y} after {last}");
        if billions <= 600 {
            assert_eq!(y, 32_500, "debt {debt:?} below the threshold");
        }
        last = y;
    }
    assert!(last > 32_500, "the risk premium rises above the threshold");
}
