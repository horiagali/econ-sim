//! Acceptance tests for AC-TAX-01..07 (spec: economy/taxation, "First
//! version"). Written in a test-authoring session from the spec and its API
//! sketch, before the code existed (ADR-0014). AC-TAX-08 and 09 need the
//! World's pay-day phase; the rest come with later milestones.
//!
//! Amounts in comments are lei; `lei(x)` is whole lei, `Bani(n)` is bani.

use econ_mech_tax::income::{
    DeductionRule, IndustryOverride, LabourTaxes, ProfitTaxes, Schedule, WageTax,
    personal_deduction, tax_pension, tax_quarter, tax_wage,
};
use econ_types::Bani;

fn lei(x: i64) -> Bani {
    Bani::from_lei(x)
}

/// The deduction of 1 December 2021: 510, 670, 830, 990, 1,310 lei for 0 to 4
/// dependants, full up to 1,950 lei gross, zero from 3,600 lei.
fn deduction_2021() -> DeductionRule {
    DeductionRule {
        base: [lei(510), lei(670), lei(830), lei(990), lei(1_310)],
        full_until: lei(1_950),
        zero_at: lei(3_600),
    }
}

/// The labour taxes of 1 December 2021: CAS 25% (3.75 points to Pillar II),
/// CASS 10%, CAM 2.25%, income tax one bracket at 10%.
fn labour_2021() -> LabourTaxes {
    LabourTaxes {
        cas_bp: 2_500,
        pillar2_bp: 375,
        cass_bp: 1_000,
        cam_bp: 225,
        income: Schedule::flat(1_000),
        deduction: deduction_2021(),
    }
}

/// Construction on the start date: no income tax, no CASS, CAS 21.25%, on
/// gross wages up to 30,000 lei a month.
fn construction() -> IndustryOverride {
    IndustryOverride {
        income_rate_bp: Some(0),
        cas_bp: 2_125,
        cass_bp: 0,
        ceiling: lei(30_000),
    }
}

fn cas(w: &WageTax) -> Bani {
    w.cas_public + w.cas_pillar2
}

/// AC-TAX-01 [unit] The worked example: a declared gross wage of 5,000 lei with no dependants gives CAS 1,250.00 (187.50 to Pillar II), CASS 500.00, income tax 325.00, net 2,925.00 and CAM 112.50, to the ban.
#[test]
fn ac_tax_01() {
    let w = tax_wage(lei(5_000), lei(5_000), 0, &labour_2021(), None);
    assert_eq!(
        w,
        WageTax {
            cas_public: Bani(106_250), // 1,250.00 - 187.50
            cas_pillar2: Bani(18_750),
            cass: Bani(50_000),
            income_tax: Bani(32_500),
            cam: Bani(11_250),
            net: Bani(292_500),
        }
    );
    // The labour cost is the gross wage plus CAM: 5,112.50 lei.
    assert_eq!(lei(5_000) + w.cam, Bani(511_250));
}

/// AC-TAX-02 [unit] The personal deduction is `deduction_base[d]` up to 1,950 lei gross, zero from 3,600 lei, falls linearly between them in whole lei, never rises with the wage, and is higher with more dependants (at most four counted).
#[test]
fn ac_tax_02() {
    let r = deduction_2021();

    // The full base up to 1,950 lei gross.
    for d in 0..=4u8 {
        for g in [Bani(0), lei(1_000), lei(1_949), lei(1_950)] {
            assert_eq!(
                personal_deduction(g, d, &r),
                r.base[usize::from(d)],
                "d {d}, gross {g:?}"
            );
        }
    }
    // Zero from 3,600 lei.
    for d in 0..=4u8 {
        for g in [lei(3_600), Bani(360_001), lei(5_000), lei(100_000)] {
            assert_eq!(personal_deduction(g, d, &r), Bani(0), "d {d}, gross {g:?}");
        }
    }
    // Linear in between: half way (2,775 lei) is half the base.
    for (d, half) in [(0u8, 255), (1, 335), (2, 415), (3, 495), (4, 655)] {
        assert_eq!(personal_deduction(lei(2_775), d, &r), lei(half), "d {d}");
    }
    // Off the half-way point the line is rounded to the nearest whole leu:
    // 510 x (3,600 - 2,000) / 1,650 = 494.55 -> 495;
    // 1,310 x (3,600 - 3,000) / 1,650 = 476.36 -> 476.
    assert_eq!(personal_deduction(lei(2_000), 0, &r), lei(495));
    assert_eq!(personal_deduction(lei(3_000), 4, &r), lei(476));

    // Whole lei, never rising with the wage, higher with more dependants.
    let mut prev = [Bani(i64::MAX); 5];
    let mut g = Bani(0);
    while g <= lei(4_000) {
        for d in 0..=4u8 {
            let v = personal_deduction(g, d, &r);
            assert_eq!(v.get() % 100, 0, "whole lei: d {d}, gross {g:?}");
            assert!(v >= Bani(0));
            assert!(
                v <= prev[usize::from(d)],
                "rises with the wage: d {d}, gross {g:?}"
            );
            prev[usize::from(d)] = v;
            if d > 0 {
                let fewer = personal_deduction(g, d - 1, &r);
                assert!(v >= fewer, "d {d}, gross {g:?}");
                // Between two bases at least 160 lei apart, the exact line
                // differs by at least 1 leu up to 3,600 - 1,650 / 160 = 3,589.7
                // lei of gross, so the rounded values differ too.
                if g <= lei(3_580) {
                    assert!(v > fewer, "higher with more dependants: d {d}, gross {g:?}");
                }
            }
        }
        g += Bani(731);
    }

    // At most four dependants counted.
    for g in [Bani(0), lei(2_000), lei(2_775), lei(3_000), lei(4_000)] {
        let four = personal_deduction(g, 4, &r);
        for d in [5u8, 6, 9, u8::MAX] {
            assert_eq!(personal_deduction(g, d, &r), four, "d {d}, gross {g:?}");
        }
    }
}

/// AC-TAX-03 [unit] The schedule: a one-bracket schedule is a flat tax; with several brackets the tax is the sum of each bracket's rate on the part of taxable income inside it, so the marginal rate at any income is that bracket's rate and the tax never falls as income rises.
#[test]
fn ac_tax_03() {
    // A one-bracket schedule is a flat tax.
    let flat = Schedule::flat(1_000);
    assert_eq!(
        flat,
        Schedule {
            thresholds: vec![Bani(0)],
            rates_bp: vec![1_000]
        }
    );
    assert_eq!(flat.tax(Bani(0)), Bani(0));
    assert_eq!(flat.tax(lei(3_250)), lei(325));
    let mut y = Bani(0);
    while y <= lei(20_000) {
        // 10% of y, rounded once, half away from zero.
        assert_eq!(flat.tax(y), y.mul_ratio(1_000, 10_000), "taxable {y:?}");
        y += Bani(12_347);
    }
    // Half a ban rounds away from zero: 10% of 5 bani = 0.5 -> 1.
    assert_eq!(flat.tax(Bani(5)), Bani(1));

    // Three brackets: 5% to 1,000 lei, 15% to 5,000 lei, 40% above.
    let s = Schedule {
        thresholds: vec![Bani(0), lei(1_000), lei(5_000)],
        rates_bp: vec![500, 1_500, 4_000],
    };
    // Each bracket's rate on the part inside it, summed.
    for (taxable, tax) in [
        (0, 0),
        (500, 25),                  // 5% x 500
        (1_000, 50),                // 5% x 1,000
        (3_000, 50 + 300),          // + 15% x 2,000
        (5_000, 50 + 600),          // + 15% x 4,000
        (8_000, 50 + 600 + 1_200),  // + 40% x 3,000
        (20_000, 50 + 600 + 6_000), // + 40% x 15,000
    ] {
        assert_eq!(s.tax(lei(taxable)), lei(tax), "taxable {taxable} lei");
    }
    // The marginal rate is the bracket's rate: 100 lei more inside a bracket
    // costs the rate x 100 lei.
    for (y, rate_bp) in [
        (0i64, 500i64),
        (200, 500),
        (2_000, 1_500),
        (4_800, 1_500),
        (7_000, 4_000),
    ] {
        let step = s.tax(lei(y + 100)) - s.tax(lei(y));
        // The rate in basis points of 10,000 bani is that many bani.
        assert_eq!(step, Bani(rate_bp), "marginal at {y} lei");
    }

    // The tax never falls as income rises, here and for a schedule whose
    // rates are not increasing.
    let odd = Schedule {
        thresholds: vec![Bani(0), lei(2_000), lei(6_000)],
        rates_bp: vec![2_000, 0, 500],
    };
    for sched in [&flat, &s, &odd] {
        let mut prev = Bani(0);
        let mut y = Bani(0);
        while y <= lei(25_000) {
            let t = sched.tax(y);
            assert!(t >= prev, "tax fell at {y:?}");
            prev = t;
            y += Bani(1_337);
        }
    }
}

/// AC-TAX-04 [unit] An industry override replaces the rates for that industry's employees up to its ceiling and the general rates above it: a construction worker with a gross of 4,000 lei pays no income tax and no CASS and CAS of 850.00 lei.
#[test]
fn ac_tax_04() {
    let t = labour_2021();
    let c = construction();

    // 4,000 lei in construction: CAS 21.25% = 850, no CASS, no income tax;
    // net 4,000 - 850 = 3,150. CAM is not overridden: 2.25% = 90.
    let w = tax_wage(lei(4_000), lei(4_000), 0, &t, Some(&c));
    assert_eq!(cas(&w), lei(850));
    assert_eq!(w.cass, Bani(0));
    assert_eq!(w.income_tax, Bani(0));
    assert_eq!(w.net, lei(3_150));
    assert_eq!(w.cam, lei(90));

    // The same wage elsewhere: CAS 1,000, CASS 400, deduction 0 (above 3,600),
    // income tax 10% x 2,600 = 260.
    let g = tax_wage(lei(4_000), lei(4_000), 0, &t, None);
    assert_eq!(
        (cas(&g), g.cass, g.income_tax),
        (lei(1_000), lei(400), lei(260))
    );

    // At the ceiling, the override still applies to all of it: CAS 6,375.
    let w = tax_wage(lei(30_000), lei(30_000), 0, &t, Some(&c));
    assert_eq!(
        (cas(&w), w.cass, w.income_tax),
        (lei(6_375), Bani(0), Bani(0))
    );

    // 40,000 lei: the override up to 30,000 (CAS 6,375) and the general rates
    // on the 10,000 above it (CAS 2,500, CASS 1,000, income tax 10% x (10,000
    // - 2,500 - 1,000) = 650; no deduction above 3,600 lei of gross).
    let w = tax_wage(lei(40_000), lei(40_000), 0, &t, Some(&c));
    assert_eq!(cas(&w), lei(8_875));
    assert_eq!(w.cass, lei(1_000));
    assert_eq!(w.income_tax, lei(650));
    assert_eq!(w.net, lei(40_000 - 8_875 - 1_000 - 650));
}

/// AC-TAX-05 [unit] Only the declared part is taxed: an envelope wage pays contributions and tax on its declared part only, a job with no contract pays none, and in every case net received = wage − contributions − tax.
#[test]
fn ac_tax_05() {
    let t = labour_2021();

    // An envelope wage of 2,500 lei with 1,600 declared, no dependants: on
    // 1,600 only, CAS 400 (60 to Pillar II), CASS 160, deduction 510 (gross
    // under 1,950), income tax 10% x (1,600 - 400 - 160 - 510) = 53, CAM 36;
    // net 2,500 - 400 - 160 - 53 = 1,887.
    assert_eq!(
        tax_wage(lei(2_500), lei(1_600), 0, &t, None),
        WageTax {
            cas_public: lei(340),
            cas_pillar2: lei(60),
            cass: lei(160),
            income_tax: lei(53),
            cam: lei(36),
            net: lei(1_887),
        }
    );
    // The undeclared part changes nothing but the net.
    let base = tax_wage(lei(1_600), lei(1_600), 2, &t, None);
    for wage in [lei(1_600), lei(2_500), lei(6_000)] {
        let w = tax_wage(wage, lei(1_600), 2, &t, None);
        assert_eq!(
            (w.cas_public, w.cas_pillar2, w.cass, w.income_tax, w.cam),
            (
                base.cas_public,
                base.cas_pillar2,
                base.cass,
                base.income_tax,
                base.cam
            ),
            "wage {wage:?}"
        );
        // Two dependants: deduction 830, income tax 10% x 210 = 21.
        assert_eq!(w.income_tax, lei(21));
        assert_eq!(w.net, wage - lei(400 + 160 + 21));
    }

    // A job with no contract pays nothing.
    let w = tax_wage(lei(2_500), Bani(0), 0, &t, None);
    assert_eq!(
        w,
        WageTax {
            cas_public: Bani(0),
            cas_pillar2: Bani(0),
            cass: Bani(0),
            income_tax: Bani(0),
            cam: Bani(0),
            net: lei(2_500),
        }
    );

    // Taxable income is not below 0: 1,000 lei declared, four dependants:
    // 1,000 - 250 - 100 - 1,310 < 0, so no income tax; net 1,800 - 350.
    let w = tax_wage(lei(1_800), lei(1_000), 4, &t, None);
    assert_eq!(
        (cas(&w), w.cass, w.income_tax, w.net),
        (lei(250), lei(100), Bani(0), lei(1_450))
    );

    // In every case, net = wage - contributions - tax, and nothing is taxed
    // on a wage with nothing declared.
    let c = construction();
    for over in [None, Some(&c)] {
        for wage_lei in [0i64, 900, 2_300, 3_333, 5_000, 12_000, 45_000] {
            let wage = lei(wage_lei) + Bani(wage_lei % 97);
            for declared in [Bani(0), wage.mul_ratio(1, 3), wage.mul_ratio(4, 5), wage] {
                for d in [0u8, 1, 4] {
                    let w = tax_wage(wage, declared, d, &t, over);
                    assert_eq!(
                        w.net,
                        wage - w.cas_public - w.cas_pillar2 - w.cass - w.income_tax,
                        "wage {wage:?}, declared {declared:?}, d {d}"
                    );
                    if declared == Bani(0) {
                        assert_eq!(w.net, wage);
                        assert_eq!(w.cam, Bani(0));
                    }
                }
            }
        }
    }
}

/// AC-TAX-06 [unit] A pension pays income tax only on its part above 2,000 lei; a pension of 2,500 lei pays 50.00 lei.
#[test]
fn ac_tax_06() {
    let s = Schedule::flat(1_000);
    let free = lei(2_000);
    assert_eq!(tax_pension(lei(2_500), free, &s), lei(50));
    for p in [Bani(0), lei(800), lei(1_442), lei(1_999), lei(2_000)] {
        assert_eq!(tax_pension(p, free, &s), Bani(0), "pension {p:?}");
    }
    // 10% of the part above 2,000 lei.
    assert_eq!(tax_pension(lei(3_000), free, &s), lei(100));
    assert_eq!(tax_pension(lei(6_500), free, &s), lei(450));
    assert_eq!(tax_pension(Bani(200_100), free, &s), Bani(10));
}

/// AC-TAX-07 [unit] Corporate tax: 16% of a quarter's positive taxable profit after earlier losses; a loss is carried forward and used up first. A micro unit under the threshold pays 1% of declared turnover with staff, 3% without, and no profit tax.
#[test]
fn ac_tax_07() {
    // The 2021 rates; 1 million euro at an assumed scenario rate of 4.95 lei.
    let t = ProfitTaxes {
        profit_bp: 1_600,
        micro_threshold: lei(4_950_000),
        micro_with_staff_bp: 100,
        micro_no_staff_bp: 300,
    };

    // 16% of a positive profit.
    let mut losses = Bani(0);
    assert_eq!(tax_quarter(lei(50_000), &mut losses, None, &t), lei(8_000));
    assert_eq!(losses, Bani(0));
    assert_eq!(tax_quarter(Bani(0), &mut losses, None, &t), Bani(0));

    // A loss is carried forward, losses add up, and they are used up first.
    let mut losses = Bani(0);
    assert_eq!(tax_quarter(lei(-100_000), &mut losses, None, &t), Bani(0));
    assert_eq!(losses, lei(100_000));
    assert_eq!(tax_quarter(lei(-20_000), &mut losses, None, &t), Bani(0));
    assert_eq!(losses, lei(120_000));
    // 80,000 of profit against 120,000 of losses: no tax, 40,000 left.
    assert_eq!(tax_quarter(lei(80_000), &mut losses, None, &t), Bani(0));
    assert_eq!(losses, lei(40_000));
    // 100,000 of profit: 16% x (100,000 - 40,000) = 9,600, nothing left.
    assert_eq!(tax_quarter(lei(100_000), &mut losses, None, &t), lei(9_600));
    assert_eq!(losses, Bani(0));
    // Then 16% of the whole profit again.
    assert_eq!(tax_quarter(lei(25_000), &mut losses, None, &t), lei(4_000));

    // A micro unit under the threshold: 1% of declared turnover with staff,
    // 3% without, and no profit tax whatever its profit.
    let turnover = lei(1_000_000);
    for profit in [lei(200_000), Bani(0), lei(-50_000)] {
        let mut losses = Bani(0);
        assert_eq!(
            tax_quarter(profit, &mut losses, Some((turnover, true)), &t),
            lei(10_000),
            "profit {profit:?}"
        );
        let mut losses = Bani(0);
        assert_eq!(
            tax_quarter(profit, &mut losses, Some((turnover, false)), &t),
            lei(30_000),
            "profit {profit:?}"
        );
    }
    // Rounded once, half away from zero: 1% of 1,234.50 lei = 12.345 -> 12.35.
    let mut losses = Bani(0);
    assert_eq!(
        tax_quarter(lei(500), &mut losses, Some((Bani(123_450), true)), &t),
        Bani(1_235)
    );
}
