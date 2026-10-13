//! Acceptance tests for AC-PRICE-01 to AC-PRICE-03 (spec:
//! economy/prices-inflation, "Update rule", "Expectations" and "First
//! version"). Written in a test-authoring session from the spec and its API
//! sketch, before the code existed (ADR-0014). The other AC-PRICE criteria
//! need the `World`.

use econ_mech_prices::{
    PriceParams, daily_share_ppm, expectations_ppm, markup_ppm, step_price_ppm, target_price_ppm,
    unit_cost_ppm,
};

/// The starting price of every good.
const ONE: u32 = 1_000_000;

/// The spec's defaults: chi 0.5, eta 1 0.2, eta 2 0.1, cu* 0.8, at most 2% a day.
fn params() -> PriceParams {
    PriceParams {
        indexation_ppm: 500_000,
        eta_utilisation_ppm: 200_000,
        eta_inventory_ppm: 100_000,
        normal_utilisation_ppm: 800_000,
        max_daily_move_ppm: 20_000,
    }
}

/// AC-PRICE-01 [unit] Unit cost, markup and target price are the formulas above; at the starting state every target price is 1 and every price stays 1 while nothing else changes and expected inflation is zero.
#[test]
fn ac_price_01() {
    let p = params();

    // uc = sum a_home * p + a_imp * 1 + wage cost + other costs:
    // 0.30 + 0.15 + 0.10 + 0.20 + 0.05 = 0.80 at starting prices.
    let inputs = [(300_000, ONE), (150_000, ONE)];
    let uc = unit_cost_ppm(&inputs, 100_000, 200_000, 50_000);
    assert_eq!(uc, 800_000);
    // The first input 10% dearer: 0.30 * 1.1 = 0.33.
    assert_eq!(
        unit_cost_ppm(
            &[(300_000, 1_100_000), (150_000, ONE)],
            100_000,
            200_000,
            50_000
        ),
        830_000
    );

    // Markup: mu0 at normal utilisation and target stock; up 0.2 per point of
    // utilisation above normal, down 0.1 per unit of excess stock.
    assert_eq!(markup_ppm(250_000, 800_000, ONE, &p), 250_000);
    assert_eq!(markup_ppm(250_000, 900_000, 1_500_000, &p), 220_000);
    assert_eq!(markup_ppm(250_000, 800_000, 500_000, &p), 300_000);
    assert_eq!(markup_ppm(250_000, 700_000, 2_000_000, &p), 130_000);

    // Target = (1 + mu) * uc. The starting markup makes it 1.
    assert_eq!(target_price_ppm(uc, 250_000), ONE);
    assert_eq!(target_price_ppm(830_000, 250_000), 1_037_500);
    assert_eq!(target_price_ppm(ONE, -100_000), 900_000);

    // At the start, with nothing changing and no expected inflation, every
    // price stays 1 for a year, whatever its flexibility.
    for monthly in [500_000, 250_000, 100_000] {
        let daily = daily_share_ppm(monthly);
        let mut price = ONE;
        for day in 0..365 {
            price = step_price_ppm(price, ONE, daily, 0, &p);
            assert_eq!(price, ONE, "flexibility {monthly}, day {day}");
        }
    }
}

/// AC-PRICE-02 [unit] The daily step: a price moves toward its target by the daily equivalent of its monthly flexibility, so that thirty daily steps close the same share of a constant gap as one monthly step, within rounding; a day's move never exceeds the cap.
#[test]
fn ac_price_02() {
    let p = params();

    // (1 - daily)^30 = 1 - monthly.
    assert_eq!(daily_share_ppm(0), 0);
    assert_eq!(daily_share_ppm(ONE), ONE);
    for monthly in [500_000_u32, 250_000, 100_000, 10_000] {
        let daily = daily_share_ppm(monthly);
        assert!(daily > 0 && daily < monthly);
        let mut left = 1.0_f64;
        for _ in 0..30 {
            left *= 1.0 - f64::from(daily) / 1e6;
        }
        let want = 1.0 - f64::from(monthly) / 1e6;
        assert!(
            ((left - want) * 1e6).abs() <= 20.0,
            "monthly {monthly}: daily {daily} leaves {left} of the gap after 30 days, not {want}"
        );
    }

    // One monthly step is p * (p* / p)^rho. Gaps chosen so that it is exact:
    // rho 0.5 from 1 to 1.21 or 0.81 gives 1.1 or 0.9; rho 0.25 to 1.1^4 and
    // rho 0.1 to 1.1^10 give 1.1.
    for (monthly, target, after_month) in [
        (500_000, 1_210_000, 1_100_000),
        (500_000, 810_000, 900_000),
        (250_000, 1_464_100, 1_100_000),
        (100_000, 2_593_742, 1_100_000),
    ] {
        let daily = daily_share_ppm(monthly);
        let mut price = ONE;
        for _ in 0..30 {
            price = step_price_ppm(price, target, daily, 0, &p);
        }
        assert!(
            price.abs_diff(after_month) <= 40,
            "flexibility {monthly}, target {target}: {price} after 30 days, one month gives {after_month}"
        );
    }

    // Expected inflation is followed with indexation chi: at its target, a
    // price moves by (1 + 0.1%)^0.5 = 1.000499875 a day.
    let daily = daily_share_ppm(250_000);
    assert!(step_price_ppm(ONE, ONE, daily, 1_000, &p).abs_diff(1_000_500) <= 1);

    // The cap: a day's move is at most 2%.
    let fast = 900_000;
    assert_eq!(step_price_ppm(ONE, 3 * ONE, fast, 0, &p), 1_020_000);
    assert_eq!(step_price_ppm(ONE, ONE / 10, fast, 0, &p), 980_000);
    for price in [500_000_u32, ONE, 2_345_678] {
        for target in [1_u32, 400_000, ONE, 1_500_000, 50 * ONE] {
            for flex in [daily_share_ppm(500_000), fast, ONE] {
                for expected in [-5_000, 0, 5_000, 100_000] {
                    let next = step_price_ppm(price, target, flex, expected, &p);
                    let cap = u64::from(price) * 20_000 / 1_000_000 + 1;
                    assert!(
                        u64::from(next.abs_diff(price)) <= cap,
                        "price {price}, target {target}, flexibility {flex}, expected {expected}: {next}"
                    );
                }
            }
        }
    }
}

/// AC-PRICE-03 [unit] Expectations are the anchored-adaptive formula; with credibility 1 they equal the target, with credibility 0 they are adaptive.
#[test]
fn ac_price_03() {
    let target = 25_000; // 2.5%
    // Credibility 1: the target, whatever happened.
    for (previous, actual) in [(30_000, 80_000), (-10_000, 0), (25_000, 25_000)] {
        assert_eq!(
            expectations_ppm(previous, actual, target, ONE, 100_000),
            target
        );
    }
    // Credibility 0: adaptive, 3% + 0.1 * (8% - 3%) = 3.5%.
    assert_eq!(expectations_ppm(30_000, 80_000, target, 0, 100_000), 35_000);
    assert_eq!(
        expectations_ppm(-10_000, -50_000, target, 0, 100_000),
        -14_000
    );
    // With full learning, last period's inflation.
    assert_eq!(expectations_ppm(30_000, 80_000, target, 0, ONE), 80_000);
    // In between: 0.7 * 2.5% + 0.3 * 3.5% = 2.8%.
    assert_eq!(
        expectations_ppm(30_000, 80_000, target, 700_000, 100_000),
        28_000
    );
    // At the target and with inflation on target, nothing moves.
    assert_eq!(
        expectations_ppm(target, target, target, 700_000, 100_000),
        target
    );
}
