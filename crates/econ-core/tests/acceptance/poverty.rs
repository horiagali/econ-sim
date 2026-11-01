//! Acceptance tests for AC-POV-01 to AC-POV-12 (spec:
//! society/poverty-inequality), written by the test writer from the spec and
//! its API sketch before the code (ADR-0014); behind the feature `poverty`.
//! AC-POV-10 to 12 (the standard of living, step 9, 2026-10-31) read the
//! test writer's additions `sol_milli`, `share_by_weight`,
//! `PovertyStats::sol` and `World::hh_sol_milli`; AC-POV-08 and 09 (restated
//! 2026-10-31) read `World::hh_income_month`.
//!
//! "1:100" and "1:1000" are the Romania scenario at that scale, seed 42,
//! under the law of its folder (`law.json`: the 2021 law). Month 0 is
//! December 2021. Incomes `x_h` are thousandths of a ban, ratios millionths.
//!
//! Readings shared by the tests:
//! - The population of the indicators is the resident members of private
//!   households: a household's members are the persons whose
//!   `household_id` it is (with migration, not `abroad`; with the People
//!   phase, alive: restated 2026-11-01, a dead person's row stays until the
//!   compaction and is no member); collective households are the
//!   scenario's, followed by `hh_key`; a record added during the run is
//!   private unless the life cycle made it collective (orphans,
//!   `society/households-lifecycle`; restated 2026-11-01, see
//!   `support_m5::collective`).
//! - A household's income of a month (`Y_h`, the `World`'s income of the
//!   month, `World::hh_income_month`) is what its `income_to_date` grew by
//!   over that month less the imputed rent it received in it (the
//!   simplification of step 1: imputed rent is left out, though the `World`
//!   pays it as money). Month 0's growth starts after November's pay on day
//!   0 (restated by the owner, 2026-10-31): the test cannot see that point
//!   inside day 0, so it bounds month 0's income between the growth from the
//!   end of day 0 and the growth from the load. Households are followed by
//!   `hh_key`, so the yearly compaction (with the People phase compiled in)
//!   does not mix them up. The calendar year's income is the sum of its
//!   months' incomes.
//! - `x_h = Y / e_h` in thousandths of a ban is `Y · 1,000,000 / eq_milli`,
//!   rounded half away from zero (the spec says "bani × 1,000, integer" and
//!   does not fix the rounding).

use std::collections::BTreeMap;

use econ_core::levers::{LeverId, LeverValue};
use econ_core::poverty::{
    Distribution, IncomeMeasures, class_of, eq_adults_milli, measures, recent_income,
    share_by_weight, sol_milli,
};
use econ_core::world::World;
use econ_types::Bani;

use super::support::params;
use super::support_lev::{date, give, key_of, set};
use super::support_m5::{
    collective, feb_15_2022, members as resident_members, money_in_force, next_close, round_div,
    run_closes, set_money_lever, step_checked, step_until, world_2021, world_2021_default,
};

/// Each household's members: its resident persons (`support_m5::members`)
/// that are alive. With the People phase compiled in (`demography`), a
/// dead person's row stays until the compaction; it is no member.
fn members(w: &World) -> Vec<Vec<usize>> {
    #[cfg(feature = "demography")]
    let m = {
        let alive = &w.people_life().alive;
        resident_members(w)
            .into_iter()
            .map(|m| {
                m.into_iter()
                    .filter(|&p| alive.get(p).copied().unwrap_or(true))
                    .collect()
            })
            .collect()
    };
    #[cfg(not(feature = "demography"))]
    let m = resident_members(w);
    m
}

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

/// The minimum-income floor of a run, ordered on 15 February 2022.
#[derive(Clone, Copy)]
enum Floor {
    /// The law in force: no order.
    Baseline,
    /// Every household size's floor set to the at-risk line of one adult in
    /// lei of the day.
    AtRiskLine,
    /// Every household size's floor set to zero.
    Zero,
}

/// The at-risk line of one adult in lei of the day: 60% of
/// `poverty_anchor` (December 2021 lei, the prices of day 0) times the CPI
/// of the latest close (millionths of day 0's prices), rounded to the ban
/// half away from zero.
fn at_risk_line_of_the_day(w: &World) -> Bani {
    let anchor = i128::from(w.poverty_anchor().get());
    assert!(anchor > 0, "the anchor is positive: {anchor}");
    let cpi = i128::from(w.months().last().expect("a close before the order").cpi_ppm);
    let line = round_div(anchor * 600_000 * cpi, 1_000_000_000_000);
    Bani(i64::try_from(line).expect("the line fits a ban"))
}

/// The indicators at the close of month 5 of a run with the floor `floor`
/// ordered on 15 February 2022.
fn min_income_run(floor: Floor) -> (u32, u32, u32) {
    let mut w = world_2021_default(1000);
    step_until(&mut w, feb_15_2022());
    let lever = LeverId::TransferMinIncomeFloor;
    let value = match floor {
        Floor::Baseline => None,
        Floor::AtRiskLine => Some(at_risk_line_of_the_day(&w)),
        Floor::Zero => Some(Bani::ZERO),
    };
    if let Some(value) = value {
        set_money_lever(&mut w, lever, |k, now| {
            if matches!(floor, Floor::AtRiskLine) {
                assert!(
                    value > now,
                    "size {k}: the at-risk line {value:?} raises the floor in force {now:?}"
                );
            }
            value
        });
    }
    run_closes(&mut w, 6);
    let s = &w.poverty_stats()[5];
    (
        s.national.poverty_rate_ppm,
        s.class_share_ppm[0],
        s.hardship_rate_ppm,
    )
}

/// **AC-POV-06** `[sim]` At 1:1000, raising `transfer.min_income.floor`
/// from month 3 to the at-risk line of one adult (60% of `poverty_anchor`,
/// in lei of the day) lowers the poverty rate, the class at risk and the
/// hardship rate within 3 months against the same run without the change;
/// cutting it to zero does not lower them.
///
/// Restated on 2026-11-01 with the spec (owner's delegation): the floor was
/// raised by 50% before, which moved no household across a line (2021's
/// floor is about 142 lei for one person against a line of about 2,000 lei
/// per adult), and cutting it to zero had to raise the indicators strictly;
/// it now need only not lower them (at or above the baseline).
///
/// Readings: the order is given on 15 February 2022 and takes effect on 1
/// March 2022 (month 3). "The at-risk line of one adult in lei of the day"
/// is `World::poverty_anchor()` (December 2021 lei) × 60% × the CPI of the
/// latest close before the order (January 2022's, `months().last().cpi_ppm`,
/// millionths of day 0's prices), rounded to the ban half away from zero.
/// Every key of the lever (household sizes 1 to 5) is set to that one value,
/// which the test checks is above each size's floor in force (so every floor
/// is raised); "cutting it to zero" sets every key to 0. "Within 3 months":
/// at the close of month 5 (May 2022), the third month of the change, each
/// of the three indicators is strictly below the baseline's with the raised
/// floor, and at or above it with the zero floor.
#[test]
fn ac_pov_06() {
    let base = min_income_run(Floor::Baseline);
    let up = min_income_run(Floor::AtRiskLine);
    let down = min_income_run(Floor::Zero);
    let names = ["poverty rate", "class at risk", "hardship rate"];
    let b = [base.0, base.1, base.2];
    let u = [up.0, up.1, up.2];
    let d = [down.0, down.1, down.2];
    for k in 0..3 {
        assert!(
            u[k] < b[k],
            "at-risk line: {} {} against {}",
            names[k],
            u[k],
            b[k]
        );
        assert!(
            d[k] >= b[k],
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
/// months. With the feature `home_quality` compiled in (migration reads the
/// wellbeing index, which reads SoL), step 5b's three elasticities are 0 in
/// both runs, so that its multipliers are 1 whatever the indicators.
#[test]
fn ac_pov_07() {
    let on = params();
    #[cfg(feature = "approval")]
    let on = {
        let mut on = on;
        on.approval = None;
        on
    };
    #[cfg(feature = "home_quality")]
    let on = {
        let mut on = on;
        on.migration.quality_return_elasticity_ppm = 0;
        on.migration.quality_emigration_elasticity_ppm = 0;
        on.migration.quality_immigration_elasticity_ppm = 0;
        on
    };
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

/// `income_to_date` of each household by `hh_key`.
fn to_date_by_key(w: &World) -> BTreeMap<u64, Bani> {
    let keys = &w.people().hh_key;
    w.households()
        .income_to_date
        .iter()
        .enumerate()
        .map(|(h, &y)| (keys[h], y))
        .collect()
}

/// Follows each household's income of the months from its `income_to_date`
/// and the imputed rent it is paid (the module's readings), by `hh_key`, and
/// checks `World::hh_income_month` against them at each close.
struct MonthIncome {
    /// `income_to_date` at the load.
    load: BTreeMap<u64, Bani>,
    /// At the end of day 0 (after November's pay).
    end_day0: Option<BTreeMap<u64, Bani>>,
    /// At the last close.
    last_close: Option<BTreeMap<u64, Bani>>,
    /// Imputed rent received since the last close, per record.
    imputed: BTreeMap<u64, Bani>,
}

impl MonthIncome {
    fn new(w: &World) -> MonthIncome {
        assert_eq!(w.date(), date(2021, 12, 1), "from the load");
        MonthIncome {
            load: to_date_by_key(w),
            end_day0: None,
            last_close: None,
            imputed: BTreeMap::new(),
        }
    }

    /// Step, checking every day, to the next close; check the month's
    /// `hh_income_month` and return it by `hh_key`.
    fn next_close(&mut self, w: &mut World) -> BTreeMap<u64, Bani> {
        let t = w.months().len();
        while w.months().len() == t {
            let r = step_checked(w);
            if self.end_day0.is_none() {
                self.end_day0 = Some(to_date_by_key(w));
            }
            let p = w.imputed_rent();
            if p.day == Some(r.day) {
                let keys = &w.people().hh_key;
                for (h, &x) in p.received.iter().enumerate() {
                    match keys.get(h) {
                        Some(&k) => *self.imputed.entry(k).or_insert(Bani::ZERO) += x,
                        None => assert_eq!(x, Bani::ZERO, "imputed rent to no household"),
                    }
                }
            }
        }
        let now = to_date_by_key(w);
        let keys = &w.people().hh_key;
        let weights = &w.households().hh_weight;
        let month = w.hh_income_month();
        assert_eq!(
            month.len(),
            keys.len(),
            "month {t}: one income per household"
        );
        let mut got = BTreeMap::new();
        let mut left_out = 0i128;
        for (h, &y) in month.iter().enumerate() {
            let k = keys[h];
            got.insert(k, y);
            let imputed = self.imputed.get(&k).copied().unwrap_or(Bani::ZERO);
            let to_date = now[&k];
            if let Some(last) = &self.last_close {
                // A household formed during the month is not checked.
                let Some(&before) = last.get(&k) else {
                    continue;
                };
                assert_eq!(
                    y,
                    to_date - before - imputed,
                    "month {t}: household {h}'s income is its income_to_date's growth less imputed rent"
                );
            } else {
                let from_load =
                    to_date - self.load.get(&k).copied().unwrap_or(Bani::ZERO) - imputed;
                let day0 = self.end_day0.as_ref().expect("day 0 stepped");
                let from_day0 = to_date - day0.get(&k).copied().unwrap_or(Bani::ZERO) - imputed;
                assert!(
                    from_day0 <= y && y <= from_load,
                    "month 0: household {h}'s income {y:?} is not between its growth from the end of day 0 ({from_day0:?}) and from the load ({from_load:?})"
                );
                left_out += i128::from(weights[h]) * i128::from((from_load - y).get());
            }
        }
        if self.last_close.is_none() {
            assert!(
                left_out > 0,
                "month 0's income leaves out November's pay on day 0"
            );
        }
        self.last_close = Some(now);
        self.imputed.clear();
        got
    }
}

/// **AC-POV-08** `[sim]` The annual indicators of each December close are
/// the measures of AC-POV-02 applied to the calendar year's income of every
/// household; `hh_income_year` is zero after the close.
///
/// Restated 2026-10-31 (owner's yes, the spec's "Implementation status"):
/// the month's income starts after November's pay on day 0 and leaves out
/// imputed rent (the module's readings), where the test of 2026-10-26 took
/// all of `income_to_date`'s growth.
///
/// Readings: checked at the December closes of 2021 (month 0), 2022 (month
/// 12) and 2023 (month 24) at 1:1000. Each month's income
/// (`World::hh_income_month`) is checked against `income_to_date` as the
/// module's readings say, and the calendar year's income of a household is
/// the sum of its months' incomes since the last December close; for month
/// 0, which has one month, the annual indicators are month 0's × 12 (the
/// spec's words): the measures on 12 times each household's income of month
/// 0, or those on month 0's income with the median times 12 (the two differ
/// only by rounding, and the spec does not choose). The distribution is the
/// private households with members, `x_h` the year's income over the
/// equivalised size at the close (see the module's readings), `v_h` members
/// times weight, the line 0.6. Other closes have no annual indicators.
/// Between Decembers, `hh_income_year` is the sum of the months' incomes
/// since the last December close.
#[test]
fn ac_pov_08() {
    let mut w = world_2021_default(1000);
    let mut income = MonthIncome::new(&w);
    let mut year: BTreeMap<u64, Bani> = BTreeMap::new();
    while w.months().len() < 25 {
        let month = income.next_close(&mut w);
        let t = w.months().len() - 1;
        for (&k, &y) in &month {
            *year.entry(k).or_insert(Bani::ZERO) += y;
        }
        let keys = w.people().hh_key.clone();
        let s = &w.poverty_stats()[t];
        if !t.is_multiple_of(12) {
            assert!(
                s.annual.is_none(),
                "month {t}: annual indicators outside December"
            );
            for (h, &y) in w.hh_income_year().iter().enumerate() {
                assert_eq!(
                    y,
                    year.get(&keys[h]).copied().unwrap_or(Bani::ZERO),
                    "month {t}: household {h}'s income of the year so far"
                );
            }
            continue;
        }
        let of_year: Vec<Bani> = keys
            .iter()
            .map(|k| {
                let y = year.get(k).copied().unwrap_or(Bani::ZERO);
                if t == 0 { y.mul_ratio(12, 1) } else { y }
            })
            .collect();
        let (x, v) = distribution_of(&w, 1000, &of_year);
        let want = measures(&dist(&x, &v), LINE);
        let got = s
            .annual
            .unwrap_or_else(|| panic!("month {t}: December has annual indicators"));
        if t == 0 {
            // "Month 0's × 12": the measures on 12 times the month's income, or
            // the month's measures with the median times 12 (the spec does not
            // say which; they differ only by the rounding of `x_h`).
            let of_month: Vec<Bani> = keys
                .iter()
                .map(|k| month.get(k).copied().unwrap_or(Bani::ZERO))
                .collect();
            let (x1, v1) = distribution_of(&w, 1000, &of_month);
            let m = measures(&dist(&x1, &v1), LINE);
            let scaled = IncomeMeasures {
                median_milli: m.median_milli * 12,
                ..m
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
        year.clear();
    }
}

/// **AC-POV-09** `[unit]` Recent income: the update of step 2 in whole bani
/// for given income, CPI and previous value; equal to month 0's income at
/// month 0.
///
/// Restated 2026-10-31 (owner's yes, recorded in the spec): month 0's income
/// starts after November's pay on day 0 and leaves out imputed rent, as
/// step 1's income does; the test of 2026-10-26 took all of
/// `income_to_date`'s growth from the load.
///
/// Readings: λ = 1/3 is 333,333 millionths. "Month 0's income at month 0":
/// on the `World` at 1:1000, after the close of month 0, each household's
/// recent real income is its income of month 0 (`World::hh_income_month`,
/// itself checked against `income_to_date` as the module's readings say)
/// over the CPI of the close, in whole bani rounded half away from zero
/// (`ỹ_0 = Y_0 / P_0`).
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
    let month = MonthIncome::new(&w).next_close(&mut w);
    let cpi = i128::from(w.months()[0].cpi_ppm);
    let keys = &w.people().hh_key;
    for (h, &r) in w.hh_real_income_recent().iter().enumerate() {
        let y = month[&keys[h]];
        let want = round_div(i128::from(y.get()) * 1_000_000, cpi);
        assert_eq!(i128::from(r.get()), want, "household {h}: ỹ_0 = Y_0 / P_0");
    }
}

/// `share_by_weight` with its checks: one share per record, summing to the
/// total exactly, each within one ban of its exact part (`total · r_i p_i /
/// Σ r p`), none to a record of zero weight.
fn checked_shares(total: Bani, weights: &[(u32, u32)]) -> Vec<Bani> {
    let shares = share_by_weight(total, weights);
    assert_eq!(shares.len(), weights.len(), "one share per record");
    assert_eq!(
        shares.iter().map(|b| i128::from(b.get())).sum::<i128>(),
        i128::from(total.get()),
        "the shares add up to the month's spending exactly: {shares:?}"
    );
    let k: Vec<i128> = weights
        .iter()
        .map(|&(r, p)| i128::from(r) * i128::from(p))
        .collect();
    let all: i128 = k.iter().sum();
    for (i, (&s, &ki)) in shares.iter().zip(&k).enumerate() {
        let exact_floor = i128::from(total.get()) * ki / all;
        let s = i128::from(s.get());
        assert!(
            s == exact_floor || s == exact_floor + 1,
            "record {i}: share {s} against {exact_floor} and a fraction"
        );
        if ki == 0 {
            assert_eq!(s, 0, "record {i} of zero weight");
        }
    }
    shares
}

/// **AC-POV-10** `[unit]` *(SoL, step 9, 2026-10-31)* SoL is 10 when real
/// equivalised consumption equals `sol_subsistence`, 20 at twice it, 0 at
/// half (clamped to 0 and 100 beyond), in thousandths of a point rounded to
/// the nearest; the health and education shares of a month add up to the
/// month's public health and education spending exactly (weighted, to the
/// ban).
///
/// Readings: `sol_milli(c, s)` takes the household's real equivalised
/// consumption and `sol_subsistence` (700 lei at the spec's default), so
/// 10 + 10 log2(3) = 25.8496 is 25,850 and 10 + 10 log2(10 / 7) = 15.1457
/// is 15,146; nothing (or less) is 0. "Weighted, to the ban": each element
/// of `share_by_weight` is a record's whole amount, its record weight
/// included, in whole bani, so the elements sum to the month's spending
/// exactly; each record's part is proportional to its record weight times
/// its person weight (the sum of its members' age weights for health, of
/// its pupils' level weights for education), within one ban (a largest
/// remainder split; the spec does not fix the rule).
#[test]
fn ac_pov_10() {
    let lei = Bani::from_lei;
    let s = lei(700);
    assert_eq!(sol_milli(s, s), 10_000, "at subsistence");
    assert_eq!(sol_milli(lei(1_400), s), 20_000, "twice subsistence");
    assert_eq!(sol_milli(lei(2_800), s), 30_000, "four times");
    assert_eq!(sol_milli(lei(350), s), 0, "half");
    assert_eq!(sol_milli(lei(175), s), 0, "a quarter: clamped to 0");
    assert_eq!(sol_milli(Bani::ZERO, s), 0, "nothing");
    assert_eq!(sol_milli(Bani(-10_000), s), 0, "less than nothing");
    assert_eq!(sol_milli(lei(700 * 512), s), 100_000, "2^9 times: 100");
    assert_eq!(
        sol_milli(lei(700 * 1_024), s),
        100_000,
        "2^10 times: clamped to 100"
    );
    assert_eq!(sol_milli(lei(2_100), s), 25_850, "three times");
    assert_eq!(sol_milli(lei(1_050), s), 15_850, "one and a half");
    assert_eq!(sol_milli(lei(1_000), s), 15_146, "10 / 7");
    assert_eq!(
        sol_milli(lei(2_000), lei(1_000)),
        20_000,
        "another subsistence: the ratio counts"
    );
    // Increasing in consumption.
    let mut last = 0;
    for c in (100..=20_000).step_by(100) {
        let v = sol_milli(lei(c), s);
        assert!(v >= last, "{c} lei: {v} below {last}");
        last = v;
    }

    // Health: 10,000,003 bani over records of weight 1,000 (998 for one)
    // with members' age weights (millionths: under 5 1.5, 5–64 1.0, 65 and
    // over 3.0): a couple over 65, two parents and a child under 5, one
    // adult, one adult in a record of weight 998, a freed record (weight 0).
    let health = checked_shares(
        Bani(10_000_003),
        &[
            (1_000, 6_000_000),
            (1_000, 3_500_000),
            (1_000, 1_000_000),
            (998, 1_000_000),
            (0, 2_000_000),
        ],
    );
    assert!(
        health[0] > health[1] && health[1] > health[2] && health[2] > health[3],
        "health shares follow the weights: {health:?}"
    );
    // Education: level weights of the pupils (lower secondary 1.2, tertiary
    // 2.0, primary 1.0 twice); a household with no pupil gets nothing.
    let education = checked_shares(
        Bani(7_777_777),
        &[
            (1_000, 1_200_000),
            (1_000, 2_000_000),
            (1_000, 2_000_000),
            (1_000, 0),
        ],
    );
    assert!(
        education[1].get().abs_diff(education[2].get()) <= 1,
        "equal weights, equal shares: {education:?}"
    );
    // Seven equal records and a total that does not divide.
    checked_shares(Bani(1_000_003), &[(1, 1); 7]);
    // No spending: no shares.
    assert!(
        checked_shares(Bani::ZERO, &[(1_000, 1_000_000), (500, 3_000_000)])
            .iter()
            .all(|&b| b == Bani::ZERO),
        "no spending, no shares"
    );
}

/// Each household's persons in the indicators' population (`n_h w_h` for a
/// private household with members, else 0).
fn persons_by_household(w: &World, scale: u32) -> Vec<u64> {
    let coll = collective(w, scale);
    members(w)
        .iter()
        .enumerate()
        .map(|(h, m)| {
            if coll[h] {
                0
            } else {
                m.len() as u64 * u64::from(w.households().hh_weight[h])
            }
        })
        .collect()
}

/// The weighted mean of households' SoL over the persons of the households
/// chosen, thousandths of a point; `None` with no person.
fn mean_sol(w: &World, v: &[u64], chosen: impl Fn(usize) -> bool) -> Option<i128> {
    let sol = w.hh_sol_milli();
    let (mut num, mut den) = (0i128, 0i128);
    for (h, &vh) in v.iter().enumerate() {
        if vh > 0 && chosen(h) {
            num += i128::from(vh) * i128::from(sol[h]);
            den += i128::from(vh);
        }
    }
    (den > 0).then(|| round_div(num, den))
}

/// **AC-POV-11** `[sim]` *(SoL)* At 1:1000, at every close of the first
/// year: the classes' mean SoL rises from the class at risk to the affluent;
/// households in hardship have a mean SoL below the national mean; and the
/// national mean's change is the sum of its five parts exactly.
///
/// Readings: the first year is the closes of months 0 to 11. "Rises" is
/// strict, class by class (`SolStats::by_class`), and every class has
/// persons. The mean SoL of households in hardship (`hh_hardship`) is over
/// their persons (members times weight, the indicators' population), from
/// `World::hh_sol_milli`; the same mean over every household of the
/// population is the national mean (`SolStats::national_milli`) to within
/// one thousandth (rounding), which ties the column to the stats. "The
/// change" is from one close to the next (months 1 to 11):
/// `national_milli` now less at the close before equals the sum of
/// `parts_change_milli`, exactly.
///
/// Restated 2026-11-01 (owner-approved batch closing roadmap M5; not
/// weaker): a household's collective flag follows the record by its
/// `hh_key` (`support_m5::collective`), not by its row, since the
/// compaction removes freed records; a record the life cycle adds is
/// collective when it was made collective (an orphan's record,
/// `society/households-lifecycle`), not always private as the helper read
/// before (measured 2026-11-01 with `home_quality`: one such record of
/// weight 919 at month 1 put the test's national mean 2 thousandths off
/// the stats); and a household's persons are its live
/// members (a dead person's row, with the People phase, is no member: the
/// spec's step 1 counts members).
#[test]
fn ac_pov_11() {
    let mut w = world_2021_default(1000);
    for t in 0..12 {
        next_close(&mut w);
        let s = &w.poverty_stats()[t];
        let sol = &s.sol;
        for k in 0..4 {
            assert!(s.class_persons[k] > 0, "month {t}: class {k} has persons");
        }
        for k in 1..4 {
            assert!(
                sol.by_class[k - 1] < sol.by_class[k],
                "month {t}: class {}'s mean SoL {} not below class {k}'s {}",
                k - 1,
                sol.by_class[k - 1],
                sol.by_class[k]
            );
        }
        let v = persons_by_household(&w, 1000);
        let hardship = w.households().hh_hardship.clone();
        let all = mean_sol(&w, &v, |_| true).expect("a population");
        assert!(
            (all - i128::from(sol.national_milli)).abs() <= 1,
            "month {t}: households' mean SoL {all} against the national {}",
            sol.national_milli
        );
        let poor = mean_sol(&w, &v, |h| hardship[h])
            .unwrap_or_else(|| panic!("month {t}: households in hardship"));
        assert!(
            poor < i128::from(sol.national_milli),
            "month {t}: households in hardship {poor} against the national mean {}",
            sol.national_milli
        );
        if t > 0 {
            let before = &w.poverty_stats()[t - 1].sol;
            let change = i64::from(sol.national_milli) - i64::from(before.national_milli);
            assert_eq!(
                sol.parts_change_milli.iter().sum::<i64>(),
                change,
                "month {t}: the five parts {:?} against the change",
                sol.parts_change_milli
            );
        }
    }
}

/// A run at 1:1000 to the close of month 5, with the health ministry's
/// budget cut by 30% on 15 February 2022 when `cut`.
fn health_budget_run(cut: bool) -> World {
    let mut w = world_2021_default(1000);
    step_until(&mut w, feb_15_2022());
    if cut {
        let lever = LeverId::SpendMinistryBudget;
        let line = key_of(&w, lever, "health");
        let now = money_in_force(&w, lever, Some(line));
        let a = give(
            &mut w,
            set(lever, Some(line), LeverValue::Money(now.mul_ratio(7, 10))),
        );
        assert_eq!(a.effective, date(2022, 3, 1), "the cut is from month 3");
    }
    run_closes(&mut w, 6);
    w
}

/// **AC-POV-12** `[sim]` *(SoL)* At 1:1000, cutting the health ministry's
/// budget by 30% from month 3 lowers the national mean SoL within 3 months
/// against the same run without the change, and the health part of the
/// decomposition is the largest fall; the mean SoL of persons 65 and over
/// falls more than that of persons 18 to 64.
///
/// Readings: `spend.ministry_budget[health]` set to 70% of its value in
/// force on 15 February 2022, which takes effect on 1 March 2022 (month 3),
/// as AC-POV-06. "Within 3 months": at the close of month 5, the national
/// mean SoL is strictly below the baseline's. "The largest fall": over the
/// closes of months 3 to 5, each part's change summed, less the same in the
/// baseline; health's (the fourth part) is negative and below every other
/// part's. "Falls more": at the close of month 5, the baseline's mean SoL of
/// the age group 65 and over less the cut run's is larger than the same for
/// 18 to 64 (in points, `SolStats::by_age`).
#[test]
fn ac_pov_12() {
    let base = health_budget_run(false);
    let cut = health_budget_run(true);
    let (b, c) = (&base.poverty_stats()[5].sol, &cut.poverty_stats()[5].sol);
    assert!(
        c.national_milli < b.national_milli,
        "the national mean SoL {} with the cut against {}",
        c.national_milli,
        b.national_milli
    );
    let parts = |w: &World| -> [i64; 5] {
        let mut p = [0i64; 5];
        for s in &w.poverty_stats()[3..6] {
            for (k, x) in p.iter_mut().enumerate() {
                *x += s.sol.parts_change_milli[k];
            }
        }
        p
    };
    let (pb, pc) = (parts(&base), parts(&cut));
    let diff: Vec<i64> = (0..5).map(|k| pc[k] - pb[k]).collect();
    assert!(diff[3] < 0, "the health part does not fall: {diff:?}");
    for k in [0, 1, 2, 4] {
        assert!(
            diff[3] < diff[k],
            "the health part's fall {} is not the largest: {diff:?}",
            diff[3]
        );
    }
    let fall = |k: usize| i64::from(b.by_age[k]) - i64::from(c.by_age[k]);
    assert!(
        fall(2) > fall(1),
        "65 and over fell {} thousandths, 18 to 64 {}",
        fall(2),
        fall(1)
    );
}
