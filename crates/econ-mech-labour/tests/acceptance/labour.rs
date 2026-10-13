//! Acceptance tests for AC-LM-01 and AC-LM-02 (spec: economy/labor-market,
//! "First version", steps 1 and 2). Written in a test-authoring session from
//! the spec and its API sketch, before the code existed (ADR-0014). The other
//! AC-LM criteria need the `World`.

use econ_mech_labour::{LabourParams, fill_rate_ppm, hires, layoffs};

/// The spec's defaults: mu 0.5, alpha 0.5, layoffs at most 10% of staff a
/// month, quits 1% a month, beta 1 0.1.
fn params() -> LabourParams {
    LabourParams {
        matching_efficiency_ppm: 500_000,
        matching_elasticity_ppm: 500_000,
        max_layoff_share_ppm: 100_000,
        quit_rate_ppm: 10_000,
        wage_tightness_response_ppm: 100_000,
    }
}

/// The power goes through a floating-point `libm` call; allow one millionth.
fn assert_ppm(got: u32, want: u32, what: &str) {
    assert!(got.abs_diff(want) <= 1, "{what}: got {got}, want {want}");
}

/// AC-LM-01 [unit] The fill rate and the hires are the formulas of step 2; hires never exceed the unit's vacancies or the seekers it can reach; the carried fraction makes a unit with a fractional rate hire the right number over many months; with no vacancies in the tier the fill rate is 1.
#[test]
fn ac_lm_01() {
    let p = params();

    // q = min(1, mu * (U / V)^alpha).
    assert_ppm(fill_rate_ppm(100, 100, &p), 500_000, "U = V");
    assert_ppm(fill_rate_ppm(25, 100, &p), 250_000, "U = V / 4");
    assert_ppm(fill_rate_ppm(400, 100, &p), 1_000_000, "U = 4 V");
    assert_eq!(fill_rate_ppm(1_600, 100, &p), 1_000_000, "capped at 1");
    assert_eq!(fill_rate_ppm(0, 100, &p), 0, "no seekers");
    let steeper = LabourParams {
        matching_elasticity_ppm: 1_000_000,
        ..p
    };
    assert_ppm(fill_rate_ppm(30, 100, &steeper), 150_000, "alpha 1");
    // No vacancies in the tier: 1.
    assert_eq!(fill_rate_ppm(0, 0, &p), 1_000_000);
    assert_eq!(fill_rate_ppm(500, 0, &p), 1_000_000);
    // The rate falls as the market tightens.
    let mut last = 0;
    for seekers in [1, 10, 50, 100, 200, 399] {
        let q = fill_rate_ppm(seekers, 100, &p);
        assert!(q > last, "seekers {seekers}: {q} after {last}");
        last = q;
    }

    // H = min(V_f, U_f, floor(q * V_f + carry)).
    let mut carry = 0;
    assert_eq!(hires(10, 100, 500_000, &mut carry), 5);
    assert_eq!(carry, 0);
    let mut carry = 0;
    assert_eq!(hires(10, 3, 500_000, &mut carry), 3, "few seekers");
    let mut carry = 0;
    assert_eq!(hires(10, 100, 1_000_000, &mut carry), 10, "every vacancy");
    let mut carry = 0;
    assert_eq!(hires(0, 100, 1_000_000, &mut carry), 0, "no vacancy");
    // The fraction is carried: 1.5 hires, then 1.5 + 0.5.
    let mut carry = 0;
    assert_eq!(hires(3, 100, 500_000, &mut carry), 1);
    assert_eq!(carry, 500_000);
    assert_eq!(hires(3, 100, 500_000, &mut carry), 2);
    assert_eq!(carry, 0);

    // Never more than the vacancies or the reachable seekers.
    for vacancies in [0, 1, 2, 7, 100] {
        for seekers in [0, 1, 3, 50, 1_000] {
            for q in [0, 1, 250_000, 999_999, 1_000_000] {
                for start in [0, 999_999] {
                    let mut carry = start;
                    let h = hires(vacancies, seekers, q, &mut carry);
                    assert!(
                        h <= vacancies && h <= seekers,
                        "V {vacancies}, U {seekers}, q {q}, carry {start}: {h} hires"
                    );
                    assert!(carry < 1_000_000);
                }
            }
        }
    }

    // A small unit with a fractional rate is not starved by rounding: over n
    // months it hires floor(n * q * V) when nothing else binds.
    for (vacancies, q) in [(1_u64, 300_000_u32), (2, 123_456), (1, 1)] {
        let mut carry = 0;
        let mut total = 0;
        for month in 1..=1_000_u64 {
            total += hires(vacancies, 1_000_000, q, &mut carry);
            assert_eq!(
                total,
                month * vacancies * u64::from(q) / 1_000_000,
                "V {vacancies}, q {q}, month {month}"
            );
        }
    }
}

/// AC-LM-02 [unit] Layoffs: the excess over desired employment, at most `max_layoff_share` of staff in a month.
#[test]
fn ac_lm_02() {
    let p = params();
    assert_eq!(layoffs(1_000, 990, &p), 10, "the excess");
    assert_eq!(layoffs(1_000, 500, &p), 100, "at most 10% of staff");
    assert_eq!(layoffs(1_000, 0, &p), 100);
    assert_eq!(layoffs(1_000, 1_000, &p), 0, "no excess");
    assert_eq!(layoffs(1_000, 1_200, &p), 0, "staff below desired");
    assert_eq!(layoffs(0, 0, &p), 0);
    // 10% of 1,005 is 100.5: at most that, in whole persons.
    assert_eq!(layoffs(1_005, 0, &p), 100);
    let free = LabourParams {
        max_layoff_share_ppm: 1_000_000,
        ..p
    };
    assert_eq!(layoffs(1_000, 0, &free), 1_000, "no firing friction");

    for staff in [0_u64, 1, 9, 10, 11, 333, 10_000] {
        for desired in [0_u64, 1, 5, 300, 9_999, 20_000] {
            let l = layoffs(staff, desired, &p);
            assert!(
                l <= staff.saturating_sub(desired),
                "staff {staff}, desired {desired}: {l}"
            );
            assert!(
                l * 10 <= staff,
                "staff {staff}, desired {desired}: {l} is over 10%"
            );
        }
    }
}
