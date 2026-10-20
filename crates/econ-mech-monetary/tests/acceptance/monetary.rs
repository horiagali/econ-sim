//! Acceptance tests for AC-MP-01 and AC-MP-02 (spec: economy/monetary-policy),
//! the `[unit]` criteria, against the API sketch. Written in a test-authoring
//! session before the code existed (ADR-0014). The spec places its tests in
//! `econ-core`; these two need no `World`, so they sit with the functions
//! they test, as the `[unit]` criteria of the M2 mechanics do.
//!
//! Units: credibility, inflation, rates and shares in millionths; the
//! parameters are the spec's (λ_c 0.05; k1 to k4 = 1, 10, 20, 1; ρ 0.8, r* 1%,
//! φ_π 1.5, φ_y 0.5), written out here so the tests do not lean on the
//! crate's defaults.

use econ_mech_monetary::{CredibilityParams, RuleParams, advisory_rate_ppm, credibility_ppm};

/// The spec's credibility parameters.
fn cred_params() -> CredibilityParams {
    CredibilityParams {
        speed_ppm: 50_000,
        k1_ppm: 1_000_000,
        k2_ppm: 10_000_000,
        k3_ppm: 20_000_000,
        k4_ppm: 1_000_000,
    }
}

/// The spec's rule parameters.
fn rule_params() -> RuleParams {
    RuleParams {
        smoothing_ppm: 800_000,
        neutral_ppm: 10_000,
        phi_pi_ppm: 1_500_000,
        phi_y_ppm: 500_000,
    }
}

/// **AC-MP-01** `[unit]` Credibility follows the update rule, clipped to
/// [0, 1]; with inflation inside the band for twelve months and no monetary
/// financing it rises, and it falls with monetary financing.
#[test]
fn ac_mp_01() {
    let p = cred_params();
    let c = |prev, months, miss, monfin, changes| {
        credibility_ppm(prev, months, miss, monfin, changes, &p)
    };
    // The rule, on values it gives exactly:
    // cred + 0.05 · [1 · months/12 − 10 · miss − 20 · monfin/GDP − 1 · changes].
    assert_eq!(c(500_000, 12, 0, 0, 0), 550_000, "a full track record");
    assert_eq!(
        c(500_000, 12, 5_000, 0, 0),
        547_500,
        "a miss of half a point"
    );
    assert_eq!(c(500_000, 6, 0, 0, 0), 525_000, "half the track record");
    assert_eq!(
        c(500_000, 12, 0, 10_000, 0),
        540_000,
        "monetary financing of 1% of GDP"
    );
    assert_eq!(
        c(500_000, 12, 0, 100_000, 0),
        450_000,
        "monetary financing of 10% of GDP"
    );
    assert_eq!(c(500_000, 12, 0, 0, 1), 500_000, "a change of the target");
    assert_eq!(
        c(500_000, 0, 20_000, 0, 0),
        490_000,
        "a miss of two points, no record"
    );
    // Clipped to [0, 1].
    assert_eq!(c(980_000, 12, 0, 0, 0), 1_000_000, "clipped at 1");
    assert_eq!(c(1_000_000, 12, 0, 0, 0), 1_000_000, "stays at 1");
    assert_eq!(c(20_000, 0, 50_000, 0, 0), 0, "clipped at 0");
    assert_eq!(c(0, 0, 0, 200_000, 2), 0, "stays at 0");
    for prev in (0..=1_000_000u32).step_by(50_000) {
        for miss in [0, 5_000, 10_000] {
            // Inside the ±1-point band all year, no monetary financing: it
            // rises (until 1).
            let next = c(prev, 12, miss, 0, 0);
            if prev < 1_000_000 {
                assert!(next > prev, "from {prev} with a miss of {miss}: {next}");
            } else {
                assert_eq!(next, 1_000_000);
            }
            assert!(next <= 1_000_000);
        }
        // Monetary financing makes it lower than without, and makes it fall.
        let without = c(prev, 12, 0, 0, 0);
        let with = c(prev, 12, 0, 100_000, 0);
        assert!(
            with < without || without == 0,
            "from {prev}: {with} against {without}"
        );
        if prev > 0 {
            assert!(
                with < prev,
                "from {prev}: monetary financing of 10% of GDP gives {with}"
            );
        }
        // More monetary financing never raises it.
        let mut last = without;
        for monfin in (0..=200_000u32).step_by(10_000) {
            let v = c(prev, 12, 0, monfin, 0);
            assert!(
                v <= last,
                "from {prev}: monfin {monfin} gives {v} after {last}"
            );
            last = v;
        }
    }
}

/// **AC-MP-02** `[unit]` The advisory rule recommends a higher rate when
/// inflation is above target and output above trend than when both are at
/// their normal levels.
#[test]
fn ac_mp_02() {
    let p = rule_params();
    let (previous, expected, target) = (17_500, 25_000, 25_000);
    // Normal: 0.8 · 1.75% + 0.2 · (1% + 2.5%) = 2.1%.
    let normal = advisory_rate_ppm(previous, expected, target, target, 0, &p);
    assert_eq!(normal, 21_000, "inflation at target, output at trend");
    // Hot: inflation 4.5%, output 2% above trend:
    // 0.8 · 1.75% + 0.2 · (1% + 2.5% + 1.5 · 2% + 0.5 · 2%) = 2.9%.
    let hot = advisory_rate_ppm(previous, expected, 45_000, target, 20_000, &p);
    assert_eq!(hot, 29_000, "inflation above target and output above trend");
    assert!(hot > normal);
    // Each part alone raises it too.
    let inflation_only = advisory_rate_ppm(previous, expected, 45_000, target, 0, &p);
    let gap_only = advisory_rate_ppm(previous, expected, target, target, 20_000, &p);
    assert_eq!(inflation_only, 27_000);
    assert_eq!(gap_only, 23_000);
    assert!(inflation_only > normal && gap_only > normal);
    // And it never falls as inflation or the gap rises.
    let mut last = i32::MIN;
    for inflation in (-20_000..=100_000).step_by(5_000) {
        let r = advisory_rate_ppm(previous, expected, inflation, target, 0, &p);
        assert!(r >= last, "inflation {inflation}: {r} after {last}");
        last = r;
    }
    let mut last = i32::MIN;
    for gap in (-50_000..=50_000).step_by(5_000) {
        let r = advisory_rate_ppm(previous, expected, target, target, gap, &p);
        assert!(r >= last, "gap {gap}: {r} after {last}");
        last = r;
    }
}
