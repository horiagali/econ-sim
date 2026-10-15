//! Acceptance tests for AC-ST-01..04 (spec: economy/social-transfers, "First
//! version"). Written in a test-authoring session from the spec and its API
//! sketch, before the code existed (ADR-0014). AC-ST-05..08 need the World's
//! pay-day phase; the rest come with later milestones.
//!
//! Amounts in comments are lei; `lei(x)` is whole lei, `Bani(n)` is bani.

use econ_mech_transfers::{
    ChildRule, MinimumIncomeRule, PensionRule, Separation, UnemploymentClaim, UnemploymentRule,
    benefit_months, child_allowance, claim_month, minimum_income, pension, pension_points,
    start_claim, unemployment_benefit,
};
use econ_types::Bani;

fn lei(x: i64) -> Bani {
    Bani::from_lei(x)
}

/// 1 December 2021: point value 1,442 lei, minimum pension 800 lei.
fn pension_2021() -> PensionRule {
    PensionRule {
        point_value: lei(1_442),
        minimum: lei(800),
    }
}

/// 1 December 2021: 75% of the ISR of 500 lei, plus 3%, 5%, 7% or 10% of the
/// last wage from 3, 5, 10 or 20 contribution years; 6, 9 or 12 months from 1,
/// 5 or 10 years.
fn unemployment_2021() -> UnemploymentRule {
    UnemploymentRule {
        isr: lei(500),
        isr_share_ppm: 750_000,
        wage_share_by_years: vec![(3, 30_000), (5, 50_000), (10, 70_000), (20, 100_000)],
        months_by_years: vec![(1, 6), (5, 9), (10, 12)],
    }
}

/// AC-ST-01 [unit] A pension is points × point value, not below the minimum pension. At load, points from a starting pension give that pension back to the ban (the minimum where it was below); raising the point's value by 10% raises every pension above the minimum by 10%, rounded to the ban.
#[test]
fn ac_st_01() {
    let r = pension_2021();

    // Points x point value: one point is 1,442 lei, two points 2,884 lei.
    assert_eq!(pension_points(lei(1_442), r.point_value), 1_000_000);
    assert_eq!(pension_points(lei(2_884), r.point_value), 2_000_000);
    assert_eq!(pension(1_000_000, &r), lei(1_442));
    assert_eq!(pension(2_500_000, &r), lei(3_605));
    // 2,000 lei is 2,000 / 1,442 = 1.386962552 points -> 1,386,963 millionths.
    assert_eq!(pension_points(lei(2_000), r.point_value), 1_386_963);
    // Not below the minimum: half a point is 721 lei -> 800 lei.
    assert_eq!(pension(500_000, &r), lei(800));
    assert_eq!(pension(0, &r), lei(800));

    // At load: every starting pension comes back to the ban, or the minimum.
    let raised = PensionRule {
        point_value: lei(1_442).mul_ratio(11, 10), // 1,586.20 lei
        ..r
    };
    assert_eq!(raised.point_value, Bani(158_620));
    let mut p = Bani(0);
    while p <= lei(20_000) {
        let pts = pension_points(p, r.point_value);
        assert_eq!(pension(pts, &r), p.max(r.minimum), "starting pension {p:?}");

        // Raising the point's value by 10% raises every pension above the
        // minimum by 10%, rounded to the ban. The points carry at most half a
        // millionth of a point of rounding (0.07 bani at 1,442 lei, 0.08 after
        // the raise), so the result is exactly 1.1 x p rounded, except where
        // 1.1 x p ends in exactly half a ban (p ending in 5 bani), where that
        // residue may decide the rounding: there it is within one ban.
        if p > r.minimum {
            let after = pension(pts, &raised);
            let expected = p.mul_ratio(11, 10);
            if p.get() % 10 == 5 {
                assert!(
                    (after - expected).abs() <= Bani(1),
                    "pension {p:?}: {after:?}"
                );
            } else {
                assert_eq!(after, expected, "pension {p:?}");
            }
        }
        p += Bani(1_237);
    }
    // A pension raised but still under the minimum stays at the minimum:
    // 700 lei of points -> 770 lei -> 800 lei.
    let pts = pension_points(lei(700), r.point_value);
    assert_eq!(pension(pts, &raised), lei(800));
}

/// AC-ST-02 [unit] The unemployment benefit: 375 lei plus the share of the last wage for the contribution years (3%, 5%, 7%, 10%), for 6, 9 or 12 months; nothing under one year of contributions or after a quit; it stops the month the person is hired or the months run out.
#[test]
fn ac_st_02() {
    let r = unemployment_2021();
    let wage = lei(4_000);

    // 375 lei + the share of 4,000 lei for the years, for 6, 9 or 12 months.
    for (years, amount, months) in [
        (0u8, 0, 0),
        (1, 375, 6),
        (2, 375, 6),
        (3, 375 + 120, 6), // 3% x 4,000
        (4, 375 + 120, 6),
        (5, 375 + 200, 9), // 5%
        (9, 375 + 200, 9),
        (10, 375 + 280, 12), // 7%
        (19, 375 + 280, 12),
        (20, 375 + 400, 12), // 10%
        (40, 375 + 400, 12),
    ] {
        assert_eq!(
            unemployment_benefit(wage, years, &r),
            lei(amount),
            "years {years}"
        );
        assert_eq!(benefit_months(years, &r), months, "years {years}");
    }
    // Rounded once, half away from zero: 375 + 5% of 3,333.33 = 541.6665 -> 541.67.
    assert_eq!(unemployment_benefit(Bani(333_333), 7, &r), Bani(54_167));

    // A layoff opens a claim; nothing under one year or after a quit.
    assert_eq!(
        start_claim(Separation::Layoff, wage, 7, &r),
        Some(UnemploymentClaim {
            amount: lei(575),
            months_left: 9
        })
    );
    assert_eq!(start_claim(Separation::Layoff, wage, 0, &r), None);
    assert_eq!(start_claim(Separation::Quit, wage, 7, &r), None);
    assert_eq!(start_claim(Separation::Quit, wage, 25, &r), None);

    // The months run out: six payments, then nothing.
    let mut c = start_claim(Separation::Layoff, wage, 2, &r).unwrap();
    for m in 0..6u8 {
        assert_eq!(c.months_left, 6 - m);
        assert_eq!(claim_month(&mut c, false), lei(375), "month {m}");
    }
    assert_eq!(c.months_left, 0);
    assert_eq!(claim_month(&mut c, false), Bani(0));

    // Hired in the third month: two payments, nothing that month or after.
    let mut c = start_claim(Separation::Layoff, wage, 12, &r).unwrap();
    assert_eq!(claim_month(&mut c, false), lei(655));
    assert_eq!(claim_month(&mut c, false), lei(655));
    assert_eq!(claim_month(&mut c, true), Bani(0));
    assert_eq!(c.months_left, 0);
    assert_eq!(claim_month(&mut c, false), Bani(0));
}

/// AC-ST-03 [unit] The child allowance is 427 lei under 2 and 214 lei from 2 to 18, nothing from 18, paid to the child's household.
#[test]
fn ac_st_03() {
    let r = ChildRule {
        under2: lei(427),
        to18: lei(214),
    };
    for (age_months, lei_amount) in [
        (0u16, 427),
        (1, 427),
        (23, 427),
        (24, 214),
        (100, 214),
        (215, 214),
        (216, 0),
        (217, 0),
        (480, 0),
        (u16::MAX, 0),
    ] {
        assert_eq!(
            child_allowance(age_months, &r),
            lei(lei_amount),
            "age {age_months} months"
        );
    }
    // The household of a baby, a five-year-old and an eighteen-year-old gets
    // 427 + 214 + 0 = 641 lei.
    let household: Bani = [6u16, 60, 216]
        .iter()
        .map(|&a| child_allowance(a, &r))
        .sum();
    assert_eq!(household, lei(641));
}

/// AC-ST-04 [unit] The minimum income is the floor for the household's size less its month's income after taxes and the other programmes, never below zero; with the 100% withdrawal rate, a leu more of earnings lowers it by a leu until it reaches zero.
#[test]
fn ac_st_04() {
    let r = MinimumIncomeRule {
        floor_by_size: vec![lei(142), lei(255), lei(357), lei(442), lei(527)],
        per_extra: lei(36),
        take_up_ppm: 600_000,
    };

    // The floor with no income: 142 ... 527 lei for 1 to 5, + 36 a member above.
    for (size, floor) in [
        (1u16, 142),
        (2, 255),
        (3, 357),
        (4, 442),
        (5, 527),
        (6, 563),
        (7, 599),
        (10, 707),
    ] {
        assert_eq!(minimum_income(size, Bani(0), &r), lei(floor), "size {size}");
    }
    // Floor less income, never below zero.
    assert_eq!(minimum_income(3, lei(100), &r), lei(257));
    assert_eq!(minimum_income(3, Bani(35_699), &r), Bani(1));
    assert_eq!(minimum_income(3, lei(357), &r), Bani(0));
    assert_eq!(minimum_income(3, lei(500), &r), Bani(0));
    assert_eq!(minimum_income(6, lei(563) + Bani(1), &r), Bani(0));

    // 100% withdrawal: each leu more lowers it by a leu until it reaches zero.
    for size in 1..=7u16 {
        let floor = minimum_income(size, Bani(0), &r);
        let mut income = Bani(0);
        while income <= floor + lei(50) {
            let now = minimum_income(size, income, &r);
            let next = minimum_income(size, income + lei(1), &r);
            assert_eq!(
                now,
                (floor - income).max(Bani(0)),
                "size {size}, income {income:?}"
            );
            assert_eq!(
                next,
                (now - lei(1)).max(Bani(0)),
                "size {size}, income {income:?}"
            );
            income += Bani(37);
        }
    }
}
