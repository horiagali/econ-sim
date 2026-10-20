//! Acceptance tests for AC-TFX-01 and AC-TFX-02 (spec: economy/trade-fx), the
//! `[unit]` criteria, against the API sketch. Written in a test-authoring
//! session before the code existed (ADR-0014). The spec places its tests in
//! `econ-core`; these need no `World`, so they sit with the functions they
//! test, as the `[unit]` criteria of the M2 mechanics do.
//!
//! Prices, shares and rates in millionths. The exchange rate's parameters are
//! the spec's: γ1 0.02 a month per point (0.01) of real-rate gap, γ2 0.05 a
//! month per unit of current account over GDP, rp 3 points, intervention 0.8.
//! Expected values of the Armington share were computed in double precision
//! (Python); the tests allow one millionth for rounding.

use econ_mech_trade::{FxParams, armington_share_ppm, fx_step};
use econ_types::Bani;

const ONE: u32 = 1_000_000;
/// σ = 1.5.
const SIGMA: u32 = 1_500_000;

fn near(a: u32, b: u32, what: &str) {
    assert!(a.abs_diff(b) <= 1, "{what}: {a} against {b}");
}

/// **AC-TFX-01** `[unit]` The Armington share is the formula of step 2: at
/// equal prices it is the starting share, and it rises when the domestic
/// good becomes dearer.
#[test]
fn ac_tfx_01() {
    // At equal prices, whatever their level: the starting share.
    for s0 in [0, 1, 50_000, 300_000, 999_999, ONE] {
        for price in [500_000, ONE, 1_200_000, 3_000_000] {
            assert_eq!(
                armington_share_ppm(s0, price, price, SIGMA),
                s0,
                "s0 {s0} at equal prices {price}"
            );
        }
    }
    // The formula: s0 · pM^(1−σ) / (s0 · pM^(1−σ) + (1 − s0) · p^(1−σ)).
    near(
        armington_share_ppm(300_000, ONE, 1_100_000, SIGMA),
        310_102,
        "domestic +10%",
    );
    near(
        armington_share_ppm(300_000, 1_100_000, ONE, SIGMA),
        290_089,
        "imports +10%",
    );
    near(
        armington_share_ppm(50_000, ONE, 1_100_000, SIGMA),
        52_313,
        "a small share",
    );
    near(
        armington_share_ppm(300_000, ONE, 1_100_000, 3_000_000),
        341_486,
        "σ = 3",
    );
    // With σ = 1 relative prices do not matter.
    assert_eq!(armington_share_ppm(300_000, ONE, 1_500_000, ONE), 300_000);
    // It rises as the domestic good becomes dearer, and falls as imports do.
    let mut last = 0;
    for p in (800_000..=1_500_000u32).step_by(10_000) {
        let s = armington_share_ppm(300_000, ONE, p, SIGMA);
        assert!(s > last, "domestic price {p}: {s} after {last}");
        assert!(s < ONE);
        last = s;
    }
    let mut last = ONE;
    for p in (800_000..=1_500_000u32).step_by(10_000) {
        let s = armington_share_ppm(300_000, p, ONE, SIGMA);
        assert!(s < last, "import price {p}: {s} after {last}");
        last = s;
    }
}

/// **AC-TFX-02** `[unit]` The exchange-rate step is the formula of step 4,
/// with the intervention offsetting its share of the move while reserves are
/// above the floor and none below it.
#[test]
fn ac_tfx_02() {
    let p = FxParams {
        gamma1_ppm: 20_000,
        gamma2_ppm: 50_000,
        risk_premium_ppm: 30_000,
        intervention_ppm: 800_000,
    };
    let e = 4_921_500; // 4.9215 lei per euro.
    let gdp_month = Bani::from_lei(100_000_000_000);
    // Δe/e = −γ1 [gap − rp] − γ2 · CA/GDP. A real-rate gap of 1 point, no
    // current account: −0.02 · (1 − 3) = +4% (the leu depreciates).
    let free = fx_step(e, 10_000, 0, false, gdp_month, &p);
    near(free.rate_ppm, 5_118_360, "the whole move below the floor");
    assert_eq!(
        free.intervention,
        Bani::ZERO,
        "no intervention below the floor"
    );
    // Above the floor the central bank offsets 80% of it: +0.8%, selling
    // foreign assets.
    let managed = fx_step(e, 10_000, 0, true, gdp_month, &p);
    near(
        managed.rate_ppm,
        4_960_872,
        "a fifth of the move above the floor",
    );
    assert!(
        managed.intervention > Bani::ZERO,
        "it sells foreign assets: {managed:?}"
    );
    // A gap equal to the risk premium and a surplus of 2% of GDP:
    // −0.05 · 0.02 = −0.1% (the leu appreciates).
    let free = fx_step(e, 30_000, 20_000, false, gdp_month, &p);
    near(free.rate_ppm, 4_916_578, "appreciation below the floor");
    assert_eq!(free.intervention, Bani::ZERO);
    let managed = fx_step(e, 30_000, 20_000, true, gdp_month, &p);
    near(managed.rate_ppm, 4_920_516, "a fifth of it above the floor");
    assert!(
        managed.intervention < Bani::ZERO,
        "it buys foreign assets: {managed:?}"
    );
    // No pressure, no move, no intervention.
    for floor in [false, true] {
        let still = fx_step(e, 30_000, 0, floor, gdp_month, &p);
        assert_eq!(still.rate_ppm, e);
        assert_eq!(still.intervention, Bani::ZERO);
    }
    // A higher real rate at home appreciates; a larger deficit depreciates.
    let mut last = u32::MAX;
    for gap in (-20_000..=60_000).step_by(5_000) {
        let r = fx_step(e, gap, 0, false, gdp_month, &p).rate_ppm;
        assert!(r < last, "gap {gap}: {r} after {last}");
        last = r;
    }
    let mut last = 0;
    for ca in (-60_000..=60_000).rev().step_by(10_000) {
        let r = fx_step(e, 30_000, ca, false, gdp_month, &p).rate_ppm;
        assert!(r > last, "current account {ca}: {r} after {last}");
        last = r;
    }
    // Above the floor every move is the free move times (1 − 0.8).
    for (gap, ca) in [
        (0, 0),
        (10_000, -30_000),
        (50_000, 10_000),
        (-10_000, 40_000),
    ] {
        let free = i64::from(fx_step(e, gap, ca, false, gdp_month, &p).rate_ppm) - i64::from(e);
        let managed = i64::from(fx_step(e, gap, ca, true, gdp_month, &p).rate_ppm) - i64::from(e);
        assert!(
            (managed * 5 - free).abs() <= 10,
            "gap {gap}, CA {ca}: managed move {managed} against free {free}"
        );
    }
}
