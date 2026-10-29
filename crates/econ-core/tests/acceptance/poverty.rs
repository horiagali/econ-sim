//! Acceptance tests for AC-POV-01 to AC-POV-09 (spec:
//! society/poverty-inequality), written by the test writer from the spec and
//! its API sketch before the code (ADR-0014); behind the feature `poverty`.
//!
//! "1:100" and "1:1000" are the Romania scenario at that scale, seed 42,
//! under the law of its folder (`law.json`: the 2021 law). Month 0 is
//! December 2021. Incomes `x_h` are thousandths of a ban, ratios millionths.
//!
//! Readings shared by the tests:
//! - The population of the indicators is the resident members of private
//!   households: a household's members are the persons whose
//!   `household_id` it is (with migration, not `abroad`); collective
//!   households are the scenario's (a record added during the run is
//!   private).
//! - A household's income of a month (`Y_h`, the `World`'s income of the
//!   month) is what its `income_to_date` grew by over that month: the money
//!   it received in it. The calendar year's income is its growth over the
//!   year's months.
//! - `x_h = Y / e_h` in thousandths of a ban is `Y · 1,000,000 / eq_milli`,
//!   rounded half away from zero (the spec says "bani × 1,000, integer" and
//!   does not fix the rounding).

use econ_core::levers::LeverId;
use econ_core::poverty::{
    Distribution, IncomeMeasures, class_of, eq_adults_milli, measures, recent_income,
};
use econ_core::world::World;
use econ_types::Bani;

use super::support::params;
use super::support_m5::{
    collective, feb_15_2022, members, round_div, run_closes, scale_money_lever, step_until,
    world_2021, world_2021_default,
};

const LINE: u32 = 600_000;
const CLASS_LINES: [u32; 3] = [600_000, 750_000, 2_000_000];

/// Years, in months.
const fn years(y: u16) -> u16 {
    y * 12
}

fn dist<'a>(x: &'a [i128], v: &'a [u64]) -> Distribution<'a> {
    Distribution {
        income_milli: x,
        weight: v,
    }
}

/// `x_h` and `v_h` of the private households with members, from each
/// household's income and equivalised size.
fn distribution_of(w: &World, scale: u32, income: &[Bani]) -> (Vec<i128>, Vec<u64>) {
    let coll = collective(w, scale);
    let ages = w.person_age_months();
    let mut x = Vec::new();
    let mut v = Vec::new();
    for (h, m) in members(w).iter().enumerate() {
        let wt = w.households().hh_weight[h];
        if coll[h] || m.is_empty() || wt == 0 {
            continue;
        }
        let member_ages: Vec<u16> = m.iter().map(|&p| ages[p]).collect();
        let e = eq_adults_milli(&member_ages);
        x.push(round_div(
            i128::from(income[h].get()) * 1_000_000,
            i128::from(e),
        ));
        v.push(m.len() as u64 * u64::from(wt));
    }
    (x, v)
}

/// **AC-POV-01** `[unit]` The population and the scale: a household's
/// equivalised size is 1 + 0.5 per further member of 14 or more + 0.3 per
/// member under 14; collective households are left out; each member counts
/// with the household's weight.
///
/// Readings: the first member is the oldest (the scale's 1 goes to an
/// adult); a member of exactly 14 years counts 0.5. The population part is
/// checked on the `World` at the close of month 0 at 1:1000: the national
/// population is the members of private households times their weights, and
/// the three age groups split it by each member's age.
#[test]
fn ac_pov_01() {
    assert_eq!(eq_adults_milli(&[years(40)]), 1_000);
    assert_eq!(eq_adults_milli(&[years(40), years(38)]), 1_500);
    assert_eq!(eq_adults_milli(&[years(40), years(38), years(10)]), 1_800);
    assert_eq!(
        eq_adults_milli(&[years(40), years(38), years(15), years(13), years(2)]),
        2_600
    );
    assert_eq!(
        eq_adults_milli(&[years(40), years(14)]),
        1_500,
        "14 counts 0.5"
    );
    assert_eq!(
        eq_adults_milli(&[years(40), years(14) - 1]),
        1_300,
        "13 and 11 months"
    );
    assert_eq!(eq_adults_milli(&[years(70), years(68), years(45)]), 2_000);
    // The order of the members does not matter.
    assert_eq!(
        eq_adults_milli(&[years(10), years(40), years(38)]),
        eq_adults_milli(&[years(40), years(38), years(10)])
    );

    let mut w = world_2021_default(1000);
    run_closes(&mut w, 1);
    let coll = collective(&w, 1000);
    let ages = w.person_age_months();
    let (mut all, mut young, mut middle, mut old) = (0u64, 0u64, 0u64, 0u64);
    let mut in_collective = 0u64;
    for (h, m) in members(&w).iter().enumerate() {
        let wt = u64::from(w.households().hh_weight[h]);
        if coll[h] {
            in_collective += m.len() as u64 * wt;
            continue;
        }
        all += m.len() as u64 * wt;
        for &p in m {
            match ages[p] {
                a if a < years(18) => young += wt,
                a if a < years(65) => middle += wt,
                _ => old += wt,
            }
        }
    }
    assert!(in_collective > 0, "the fixture has collective households");
    let s = &w.poverty_stats()[0];
    assert_eq!(
        s.national.population, all,
        "members of private households × weights"
    );
    assert_eq!(
        [
            s.by_age[0].population,
            s.by_age[1].population,
            s.by_age[2].population
        ],
        [young, middle, old],
        "the age groups 0–17, 18–64, 65 or more"
    );
}

/// The measures, checked field by field.
fn assert_measures(
    m: &IncomeMeasures,
    median: i128,
    rate: u32,
    gini: u32,
    s80: Option<u32>,
    what: &str,
) {
    assert_eq!(m.median_milli, median, "{what}: median");
    assert_eq!(m.poverty_rate_ppm, rate, "{what}: poverty rate");
    assert_eq!(m.gini_ppm, gini, "{what}: Gini");
    assert_eq!(m.s80_s20_ppm, s80, "{what}: S80/S20");
}

/// **AC-POV-02** `[unit]` The measures on given distributions: equal incomes
/// give poverty rate 0, Gini 0 and S80/S20 1; two hand-worked weighted
/// examples give the stated median, poverty rate, Gini and S80/S20 exactly
/// (in millionths); a poorest fifth with no income gives S80/S20 none;
/// permuting the households changes nothing.
///
/// The examples (`x_h` with `v_h`, the line 0.6 of the median):
/// - A: 100, 200, 300, 400, one person each. `V` 4, `X` 1,000. Median 200
///   (the smallest `x` with cumulative weight ≥ 2). Below 120: 1 of 4,
///   250,000. Gini: Σ v x (2V_h − v) = 100 + 600 + 1,500 + 2,800 = 5,000;
///   5,000 / 4,000 − 1 = 0.25. S80/S20: the fifth is 0.8 persons, split on
///   the boundary: 0.8 · 400 / (0.8 · 100) = 4.
/// - C: 100, 200, 600, 900 with 2, 3, 3, 2 persons. `V` 10, `X` 4,400.
///   Median 200 (cumulative 5 ≥ 5). Below 120: 2 of 10, 200,000. Gini:
///   400 + 4,200 + 23,400 + 32,400 = 60,400; 60,400 / 44,000 − 1 =
///   0.372727 (372,727 by any rounding). S80/S20: 2 · 900 / (2 · 100) = 9.
/// - B: 0, 100, 200, 600 with 1, 2, 1, 1. `V` 5, `X` 1,000. Median 100.
///   Below 60: 1 of 5. Gini: 0 + 800 + 1,400 + 5,400 = 7,600; 7,600 / 5,000
///   − 1 = 0.52. The poorest fifth (1 person) has no income: S80/S20 none.
#[test]
fn ac_pov_02() {
    // Equal incomes, any weights.
    let m = measures(&dist(&[500, 500, 500, 500], &[3, 1, 7, 2]), LINE);
    assert_measures(&m, 500, 0, 0, Some(1_000_000), "equal incomes");
    assert_eq!(m.population, 13);

    let m = measures(&dist(&[100, 200, 300, 400], &[1, 1, 1, 1]), LINE);
    assert_measures(&m, 200, 250_000, 250_000, Some(4_000_000), "example A");
    assert_eq!(m.population, 4);

    let x = [100, 200, 600, 900];
    let v = [2, 3, 3, 2];
    let m = measures(&dist(&x, &v), LINE);
    assert_measures(&m, 200, 200_000, 372_727, Some(9_000_000), "example C");
    assert_eq!(m.population, 10);

    let m = measures(&dist(&[0, 100, 200, 600], &[1, 2, 1, 1]), LINE);
    assert_measures(&m, 100, 200_000, 520_000, None, "example B");

    // Permutations of C.
    for order in [[3, 2, 1, 0], [2, 0, 3, 1], [1, 3, 0, 2]] {
        let xp: Vec<i128> = order.iter().map(|&i| x[i]).collect();
        let vp: Vec<u64> = order.iter().map(|&i| v[i]).collect();
        assert_eq!(
            measures(&dist(&xp, &vp), LINE),
            m_c(),
            "C permuted {order:?}"
        );
    }
}

/// Example C's measures.
fn m_c() -> IncomeMeasures {
    measures(&dist(&[100, 200, 600, 900], &[2, 3, 3, 2]), LINE)
}

/// **AC-POV-03** `[unit]` Scaling every real income by the same factor
/// (1.2, 0.8) leaves the poverty rate, Gini and S80/S20 unchanged and moves
/// `poverty_rate_anchored` the right way (down for 1.2, up for 0.8), unless
/// the class is already empty or full.
///
/// Readings: `poverty_rate_anchored` is the weighted share of the class at
/// risk (`class_of` = 0) against a fixed anchor; the distribution has
/// incomes 100 to 300 by 10, one person each, the anchor 200 (line 120): 2
/// of 21 at risk; ×1.2 none, ×0.8 five. "The right way" is strict here
/// because the class is neither empty nor full.
#[test]
fn ac_pov_03() {
    let x: Vec<i128> = (0..21).map(|i| 100 + 10 * i).collect();
    let v = vec![1u64; 21];
    let anchor = 200;
    let at_risk = |x: &[i128]| -> u64 {
        x.iter()
            .zip(&v)
            .filter(|(xi, _)| class_of(**xi, anchor, CLASS_LINES) == 0)
            .map(|(_, &vi)| vi)
            .sum()
    };
    let base = measures(&dist(&x, &v), LINE);
    let base_risk = at_risk(&x);
    assert_eq!(base_risk, 2);
    for (num, den, what) in [(6, 5, "×1.2"), (4, 5, "×0.8")] {
        let xs: Vec<i128> = x.iter().map(|&xi| xi * num / den).collect();
        let m = measures(&dist(&xs, &v), LINE);
        assert_eq!(
            m.poverty_rate_ppm, base.poverty_rate_ppm,
            "{what}: poverty rate"
        );
        assert_eq!(m.gini_ppm, base.gini_ppm, "{what}: Gini");
        assert_eq!(m.s80_s20_ppm, base.s80_s20_ppm, "{what}: S80/S20");
        assert_eq!(
            m.median_milli,
            base.median_milli * num / den,
            "{what}: median"
        );
        let risk = at_risk(&xs);
        if num > den {
            assert!(
                risk < base_risk,
                "{what}: the class at risk shrinks ({risk})"
            );
        } else {
            assert!(risk > base_risk, "{what}: the class at risk grows ({risk})");
        }
    }
    // Example C scaled.
    let c = m_c();
    for (num, den) in [(6, 5), (4, 5)] {
        let xs: Vec<i128> = [100i128, 200, 600, 900]
            .iter()
            .map(|&xi| xi * num / den)
            .collect();
        let m = measures(&dist(&xs, &[2, 3, 3, 2]), LINE);
        assert_eq!(
            (m.poverty_rate_ppm, m.gini_ppm, m.s80_s20_ppm),
            (c.poverty_rate_ppm, c.gini_ppm, c.s80_s20_ppm),
            "C ×{num}/{den}"
        );
    }
    // The classes' bounds: [0, 0.6), [0.6, 0.75), [0.75, 2.0], above 2.0.
    let a = 1_000_000;
    for (x, k) in [
        (0, 0),
        (599_999, 0),
        (600_000, 1),
        (749_999, 1),
        (750_000, 2),
        (2_000_000, 2),
        (2_000_001, 3),
    ] {
        assert_eq!(class_of(x, a, CLASS_LINES), k, "x {x} of anchor {a}");
    }
}

/// **AC-POV-04** `[unit]` Classes: the four class shares sum to exactly the
/// population's weight; at month 0 `poverty_rate_anchored` equals
/// `poverty_rate`; the class flows satisfy share now − share before =
/// entered − left (+ added − removed) exactly at every close of a 24-month
/// run at 1:1000.
///
/// Readings: "exactly" is in persons: the class counts (`class_persons`)
/// sum to the national population, and each class's count changes by
/// entered − left + added − removed. The shares in millionths are each
/// class's count over the population, to within one millionth (rounding).
#[test]
fn ac_pov_04() {
    let mut w = world_2021_default(1000);
    run_closes(&mut w, 24);
    let stats = w.poverty_stats();
    assert_eq!(stats.len(), 24);
    for (t, s) in stats.iter().enumerate() {
        let v = s.national.population;
        assert!(v > 0);
        assert_eq!(
            s.class_persons.iter().sum::<u64>(),
            v,
            "month {t}: the classes sum to the population"
        );
        for k in 0..4 {
            let exact = s.class_persons[k] * 1_000_000 / v;
            assert!(
                u64::from(s.class_share_ppm[k]).abs_diff(exact) <= 1,
                "month {t}: class {k} share {} against {} of {v}",
                s.class_share_ppm[k],
                s.class_persons[k]
            );
        }
        if t == 0 {
            assert_eq!(
                s.class_share_ppm[0], s.national.poverty_rate_ppm,
                "month 0: anchored rate = poverty rate"
            );
        } else {
            let before = &stats[t - 1];
            for k in 0..4 {
                let lhs = i128::from(s.class_persons[k]) - i128::from(before.class_persons[k]);
                let rhs = i128::from(s.class_entered[k]) - i128::from(s.class_left[k])
                    + i128::from(s.class_added[k])
                    - i128::from(s.class_removed[k]);
                assert_eq!(lhs, rhs, "month {t}: class {k} flows");
            }
        }
    }
}

/// **AC-POV-05** `[sim]` Starting values, at 1:100, the annual indicators of
/// month 0: poverty rate 18–26%, Gini 0.29–0.38, S80/S20 5.0–8.5, median
/// equivalised income within 30% of 27,129 lei a year (Romania, incomes of
/// 2021: 21.2%, 0.320, 6.00; incomes of 2020: 22.5%, 0.343, 7.10).
///
/// Readings: the bands are inclusive; the median of the annual indicators
/// is in thousandths of a ban a year (`x_h` on the year's income), so 27,129
/// lei is 2,712,900,000 and the band 1,899,030,000 to 3,526,770,000.
#[test]
fn ac_pov_05() {
    let mut w = world_2021_default(100);
    run_closes(&mut w, 1);
    let a = w.poverty_stats()[0]
        .annual
        .expect("month 0 is a December close: annual indicators");
    assert!(
        (180_000..=260_000).contains(&a.poverty_rate_ppm),
        "poverty rate {}",
        a.poverty_rate_ppm
    );
    assert!(
        (290_000..=380_000).contains(&a.gini_ppm),
        "Gini {}",
        a.gini_ppm
    );
    let s80 = a.s80_s20_ppm.expect("the poorest fifth has income");
    assert!((5_000_000..=8_500_000).contains(&s80), "S80/S20 {s80}");
    let median = a.median_milli;
    let survey: i128 = 27_129 * 100 * 1_000;
    assert!(
        median * 10 >= survey * 7 && median * 10 <= survey * 13,
        "median equivalised income {median} (thousandths of a ban a year) against {survey}"
    );
}

/// The indicators at the close of month 5 of a run with an order given on
/// 15 February 2022 (none for the baseline).
fn min_income_run(change: Option<(i64, i64)>) -> (u32, u32, u32) {
    let mut w = world_2021_default(1000);
    step_until(&mut w, feb_15_2022());
    if let Some((num, den)) = change {
        scale_money_lever(&mut w, LeverId::TransferMinIncomeFloor, num, den);
    }
    run_closes(&mut w, 6);
    let s = &w.poverty_stats()[5];
    (
        s.national.poverty_rate_ppm,
        s.class_share_ppm[0],
        s.hardship_rate_ppm,
    )
}

/// **AC-POV-06** `[sim]` At 1:1000, raising `transfer.min_income.floor` by
/// 50% from month 3 lowers the poverty rate, the class at risk and the
/// hardship rate within 3 months against the same run without the change;
/// cutting it to zero raises them.
///
/// Readings: every key (household size) of the lever is set to 150% (or 0)
/// of its value in force on 15 February 2022, which takes effect on 1 March
/// 2022 (month 3). "Within 3 months": at the close of month 5 (May 2022),
/// the third month of the change, each of the three indicators is strictly
/// below (above) the baseline's.
#[test]
fn ac_pov_06() {
    let base = min_income_run(None);
    let up = min_income_run(Some((3, 2)));
    let down = min_income_run(Some((0, 1)));
    let names = ["poverty rate", "class at risk", "hardship rate"];
    let b = [base.0, base.1, base.2];
    let u = [up.0, up.1, up.2];
    let d = [down.0, down.1, down.2];
    for k in 0..3 {
        assert!(u[k] < b[k], "+50%: {} {} against {}", names[k], u[k], b[k]);
        assert!(
            d[k] > b[k],
            "to zero: {} {} against {}",
            names[k],
            d[k],
            b[k]
        );
    }
}

/// **AC-POV-07** `[sim]` Isolation: at 1:1000, the `World`'s state hash after
/// 24 months is the same with the indicators computed and not computed.
///
/// Readings: "not computed" is `WorldParams::poverty` `None`. With the
/// feature `approval` compiled in, approval (which reads the recent income)
/// is off in both runs, so that only the indicators differ. The test
/// compares the state hash of every day, which includes the one after 24
/// months.
#[test]
fn ac_pov_07() {
    let mut on = params();
    #[cfg(feature = "approval")]
    {
        on.approval = None;
    }
    let mut off = on;
    off.poverty = None;
    assert!(on.poverty.is_some());
    let mut a = world_2021(1000, on, Vec::new());
    let mut b = world_2021(1000, off, Vec::new());
    while a.months().len() < 24 {
        let ra = super::support_m5::step_checked(&mut a);
        let rb = super::support_m5::step_checked(&mut b);
        assert_eq!(
            ra.state_hash, rb.state_hash,
            "{:?}: the state hashes differ",
            ra.date
        );
    }
    assert_eq!(b.months().len(), 24);
    assert_eq!(a.state_hash(), b.state_hash(), "after 24 months");
    assert_eq!(a.poverty_stats().len(), 24, "the indicators were computed");
}

/// **AC-POV-08** `[sim]` The annual indicators of each December close are
/// the measures of AC-POV-02 applied to the calendar year's income of every
/// household; `hh_income_year` is zero after the close.
///
/// Readings: checked at the December closes of 2021 (month 0), 2022 (month
/// 12) and 2023 (month 24) at 1:1000. The calendar year's income of a
/// household is the growth of its `income_to_date` over the year's months;
/// for month 0, which has one month, the annual indicators are month 0's ×
/// 12 (the spec's words): the measures on 12 times each household's income
/// of month 0, or those on month 0's income with the median times 12 (the
/// two differ only by rounding, and the spec does not choose). The distribution is the private households
/// with members, `x_h` the year's income over the equivalised size at the
/// close (see the module's readings), `v_h` members times weight, the line
/// 0.6. Other closes have no annual indicators. Between Decembers,
/// `hh_income_year` is the income received since the last December close.
#[test]
fn ac_pov_08() {
    let mut w = world_2021_default(1000);
    let mut at_december: Vec<Bani> = Vec::new();
    while w.months().len() < 25 {
        next_close_and_check(&mut w, &mut at_december);
    }
}

/// Step one month and check the month's annual indicators and
/// `hh_income_year`.
fn next_close_and_check(w: &mut World, at_december: &mut Vec<Bani>) {
    super::support_m5::next_close(w);
    let t = w.months().len() - 1;
    let to_date = w.households().income_to_date.clone();
    let s = &w.poverty_stats()[t];
    let december = t.is_multiple_of(12);
    if !december {
        assert!(
            s.annual.is_none(),
            "month {t}: annual indicators outside December"
        );
        for (h, &y) in w.hh_income_year().iter().enumerate() {
            let since = at_december.get(h).copied().unwrap_or(Bani::ZERO);
            assert_eq!(
                y,
                to_date[h] - since,
                "month {t}: household {h}'s income of the year so far"
            );
        }
        return;
    }
    let year: Vec<Bani> = if t == 0 {
        to_date.iter().map(|&y| y.mul_ratio(12, 1)).collect()
    } else {
        to_date
            .iter()
            .enumerate()
            .map(|(h, &y)| y - at_december.get(h).copied().unwrap_or(Bani::ZERO))
            .collect()
    };
    let (x, v) = distribution_of(w, 1000, &year);
    let want = measures(&dist(&x, &v), LINE);
    let got = s
        .annual
        .unwrap_or_else(|| panic!("month {t}: December has annual indicators"));
    if t == 0 {
        // "Month 0's × 12": the measures on 12 times the month's income, or
        // the month's measures with the median times 12 (the spec does not
        // say which; they differ only by the rounding of `x_h`).
        let (x1, v1) = distribution_of(w, 1000, &to_date);
        let month = measures(&dist(&x1, &v1), LINE);
        let scaled = IncomeMeasures {
            median_milli: month.median_milli * 12,
            ..month
        };
        assert!(
            got == want || got == scaled,
            "month 0: the annual indicators {got:?}, against {want:?} or {scaled:?}"
        );
    } else {
        assert_eq!(got, want, "month {t}: the annual indicators");
    }
    assert!(
        w.hh_income_year().iter().all(|&y| y == Bani::ZERO),
        "month {t}: hh_income_year is zero after the December close"
    );
    *at_december = to_date;
}

/// **AC-POV-09** `[unit]` Recent income: the update of step 2 in whole bani
/// for given income, CPI and previous value; equal to month 0's income at
/// month 0.
///
/// Readings: λ = 1/3 is 333,333 millionths. "Month 0's income at month 0":
/// on the `World` at 1:1000, after the close of month 0, each household's
/// recent real income is its income of month 0 over the CPI of the close,
/// in whole bani rounded half away from zero (`ỹ_0 = Y_0 / P_0`).
#[test]
fn ac_pov_09() {
    let third = 333_333;
    let one = 1_000_000;
    // 100,000 + (130,000 − 100,000) / 3 = 109,999.99 → 110,000.
    assert_eq!(
        recent_income(Bani(100_000), Bani(130_000), one, third),
        Bani(110_000)
    );
    // Falling income: 100,000 − 30,000 / 3.
    assert_eq!(
        recent_income(Bani(100_000), Bani(70_000), one, third),
        Bani(90_000)
    );
    // Prices 10% up: 130,000 / 1.1 = 118,181.8; 100,000 + 18,181.8 / 3 = 106,060.6.
    assert_eq!(
        recent_income(Bani(100_000), Bani(130_000), 1_100_000, third),
        Bani(106_061)
    );
    // Speed 1: last month's real income only.
    assert_eq!(
        recent_income(Bani(55_555), Bani(110_000), 1_100_000, one),
        Bani(100_000)
    );
    // No change: no move.
    assert_eq!(
        recent_income(Bani(80_000), Bani(80_000), one, third),
        Bani(80_000)
    );
    // Zero income: falls by a third.
    assert_eq!(
        recent_income(Bani(90_000), Bani::ZERO, one, third),
        Bani(60_000)
    );

    let mut w = world_2021_default(1000);
    run_closes(&mut w, 1);
    let cpi = i128::from(w.months()[0].cpi_ppm);
    let to_date = &w.households().income_to_date;
    for (h, &r) in w.hh_real_income_recent().iter().enumerate() {
        let want = round_div(i128::from(to_date[h].get()) * 1_000_000, cpi);
        assert_eq!(i128::from(r.get()), want, "household {h}: ỹ_0 = Y_0 / P_0");
    }
}
