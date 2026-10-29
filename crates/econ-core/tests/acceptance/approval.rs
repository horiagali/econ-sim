//! Acceptance tests for AC-APP-01 to AC-APP-08 (spec:
//! society/opinion-approval, "First version (roadmap M5)"), written by the
//! test writer from the spec and its API sketch before the code (ADR-0014);
//! behind the feature `approval`.
//!
//! "1:1000" is the Romania scenario at that scale, seed 42, under the law of
//! its folder (`law.json`: the 2021 law); "the baseline" is the same run with
//! no lever changed. Month 0 is December 2021. Points are thousandths
//! (millipoints); the parameters are the spec's defaults unless a test says
//! otherwise.
//!
//! Readings shared by the tests:
//! - A person "looking for work" is one whose activity is `Unemployed`.
//! - Activity groups are indexed by `econ_popgen::Activity` as its code
//!   (employed 3, retired 5).
//! - Orders are given on 15 February 2022, as in the M4 suite: a law lever
//!   then takes effect on 1 March 2022 (month 3), and a market lever is given
//!   that day as its effective day.

use econ_core::approval::{
    ApprovalParams, ApprovalStats, GroupApproval, PersonInputs, next_reference, step, targets,
};
use econ_core::levers::{LeverId, LeverValue};
use econ_core::world::World;
use econ_popgen::{Activity, NO_EMPLOYER};
use econ_types::Bani;

use super::support::params;
use super::support_lev::{date, give, set_on};
use super::support_m5::{
    adults_mask, feb_15_2022, members, next_close, run_closes, step_checked, step_until,
    weights_of_persons, world_2021, world_2021_default,
};

const INCOME: usize = 0;
const UNEMPLOYMENT: usize = 1;
const INFLATION: usize = 2;
const HARDSHIP: usize = 3;
const BOUNDS: usize = 4;

fn defaults() -> ApprovalParams {
    let p = ApprovalParams::spec_defaults();
    assert_eq!(p.income_weight_milli, 60_000);
    assert_eq!(p.loss_aversion_ppm, 2_000_000);
    assert_eq!(p.income_cap_milli, 25_000);
    assert_eq!(p.unemployed_self_milli, 10_000);
    assert_eq!(p.unemployed_household_milli, 4_000);
    assert_eq!(p.inflation_weight_milli, 1_000);
    assert_eq!(p.inflation_neutral_ppm, 30_000);
    assert_eq!(p.hardship_milli, 8_000);
    assert_eq!(p.smoothing_ppm, 500_000);
    p
}

/// Inputs with nothing changed: income at its reference, nobody looking for
/// work, inflation 2%, no hardship.
fn calm() -> PersonInputs {
    PersonInputs {
        recent: Bani(100_000),
        reference: Bani(100_000),
        seeking: false,
        others_seeking: 0,
        inflation_ppm: 20_000,
        hardship: false,
    }
}

/// **AC-APP-01** `[unit]` The parts for given inputs are the formulas of
/// steps 1 to 4 in millipoints: a 5% real loss against the reference gives
/// exactly κ times the magnitude of a 5% gain (unless capped); the cap holds;
/// zero incomes give 0; the reference moves by step 1's rule.
///
/// Readings: with the defaults a 5% gain is 60 · 0.05 = 3 points (3,000) and
/// a 5% loss −6,000; inflation of 5% a year is 2 points above π0 (−2,000);
/// a reference of 0 with a positive income is the cap; the reference speed
/// 1/24 is 41,667 millionths, so 24,000 bani of gap move it by 1,000.
#[test]
fn ac_app_01() {
    let p = defaults();
    assert_eq!(targets(&calm(), &p), [0, 0, 0, 0], "nothing changed");

    let gain = PersonInputs {
        recent: Bani(105_000),
        ..calm()
    };
    let loss = PersonInputs {
        recent: Bani(95_000),
        ..calm()
    };
    let g = targets(&gain, &p)[INCOME];
    let l = targets(&loss, &p)[INCOME];
    assert_eq!(g, 3_000, "a 5% gain");
    assert_eq!(l, -6_000, "a 5% loss");
    assert_eq!(
        i64::from(l),
        -2 * i64::from(g),
        "κ times the gain's magnitude"
    );
    // Another κ.
    let p3 = ApprovalParams {
        loss_aversion_ppm: 3_000_000,
        ..p
    };
    assert_eq!(targets(&loss, &p3)[INCOME], -9_000);
    assert_eq!(targets(&gain, &p3)[INCOME], 3_000);

    // The cap: ±25 points.
    let big_gain = PersonInputs {
        recent: Bani(150_000),
        ..calm()
    };
    let big_loss = PersonInputs {
        recent: Bani(50_000),
        ..calm()
    };
    assert_eq!(targets(&big_gain, &p)[INCOME], 25_000);
    assert_eq!(targets(&big_loss, &p)[INCOME], -25_000);
    // A loss capped even though κ · gain would not be: 30% loss → −36 → −25.
    assert_eq!(
        targets(
            &PersonInputs {
                recent: Bani(70_000),
                ..calm()
            },
            &p
        )[INCOME],
        -25_000
    );

    // Zero incomes.
    let zero = PersonInputs {
        recent: Bani::ZERO,
        reference: Bani::ZERO,
        ..calm()
    };
    assert_eq!(targets(&zero, &p)[INCOME], 0, "both incomes zero");
    let from_zero = PersonInputs {
        recent: Bani(1),
        reference: Bani::ZERO,
        ..calm()
    };
    assert_eq!(
        targets(&from_zero, &p)[INCOME],
        25_000,
        "only the reference zero"
    );
    let to_zero = PersonInputs {
        recent: Bani::ZERO,
        ..calm()
    };
    assert_eq!(
        targets(&to_zero, &p)[INCOME],
        -25_000,
        "income lost entirely"
    );

    // Unemployment: −u_self, −u_hh per other member, at most two counted.
    let u = |seeking, others| {
        targets(
            &PersonInputs {
                seeking,
                others_seeking: others,
                ..calm()
            },
            &p,
        )[UNEMPLOYMENT]
    };
    assert_eq!(u(true, 0), -10_000);
    assert_eq!(u(true, 1), -14_000);
    assert_eq!(u(true, 2), -18_000);
    assert_eq!(u(true, 5), -18_000, "at most two others counted");
    assert_eq!(u(false, 1), -4_000);
    assert_eq!(u(false, 3), -8_000);

    // Inflation above π0 = 3%, one point a point; deflation gives nothing.
    let i = |ppm| {
        targets(
            &PersonInputs {
                inflation_ppm: ppm,
                ..calm()
            },
            &p,
        )[INFLATION]
    };
    assert_eq!(i(50_000), -2_000, "5% a year");
    assert_eq!(i(30_000), 0, "at π0");
    assert_eq!(i(0), 0);
    assert_eq!(i(-30_000), 0, "deflation");
    assert_eq!(i(1_030_000), -100_000, "hyperinflation is not capped");

    // Hardship.
    let h = targets(
        &PersonInputs {
            hardship: true,
            ..calm()
        },
        &p,
    );
    assert_eq!(h, [0, 0, 0, -8_000]);
    assert_eq!(h[HARDSHIP], -8_000);

    // The reference: r + λ_r (ỹ − r).
    assert_eq!(
        next_reference(Bani(100_000), Bani(124_000), &p),
        Bani(101_000)
    );
    assert_eq!(
        next_reference(Bani(100_000), Bani(76_000), &p),
        Bani(99_000)
    );
    assert_eq!(
        next_reference(Bani(100_000), Bani(100_000), &p),
        Bani(100_000)
    );
    assert_eq!(next_reference(Bani::ZERO, Bani(240_000), &p), Bani(10_000));
}

/// **AC-APP-02** `[unit]` Smoothing and bounds: each part follows step 5;
/// `A_i` stays within 0 and 100; `A_i` = 50 + the five parts exactly, for
/// any inputs including extreme inflation and a capped income.
///
/// Readings: a person new to approval (`prev` `None`) starts at the targets;
/// then each part moves by λ_A = 0.5 of its gap. The test walks a grid of
/// previous parts and targets, including targets far beyond the bounds.
#[test]
fn ac_app_02() {
    let p = defaults();
    // New: the parts are the targets.
    let (parts, a) = step(None, [3_000, -10_000, -2_000, -8_000], &p);
    assert_eq!(parts, [3_000, -10_000, -2_000, -8_000, 0]);
    assert_eq!(a, 33_000);
    // Smoothing by half the gap.
    let (parts, a) = step(Some([0, 0, 0, 0]), [3_000, -10_000, -2_000, -8_000], &p);
    assert_eq!(parts, [1_500, -5_000, -1_000, -4_000, 0]);
    assert_eq!(a, 41_500);
    let (parts, _) = step(
        Some([1_500, -5_000, -1_000, -4_000]),
        [3_000, -10_000, -2_000, -8_000],
        &p,
    );
    assert_eq!(&parts[..4], &[2_250, -7_500, -1_500, -6_000]);
    // Smoothing 1: the targets at once.
    let p1 = ApprovalParams {
        smoothing_ppm: 1_000_000,
        ..p
    };
    let (parts, _) = step(
        Some([7_000, -3_000, 0, 0]),
        [1_000, -1_000, -2_000, -8_000],
        &p1,
    );
    assert_eq!(&parts[..4], &[1_000, -1_000, -2_000, -8_000]);

    // Extreme inflation: held at 0 by the bounds part.
    let (parts, a) = step(None, [0, 0, -1_000_000, 0], &p);
    assert_eq!(a, 0);
    assert_eq!(parts[BOUNDS], 950_000);
    // Above 100: held at 100.
    let (parts, a) = step(None, [60_000, 0, 0, 0], &p);
    assert_eq!(a, 100_000);
    assert_eq!(parts[BOUNDS], -10_000);

    let values = [
        -1_000_000, -60_000, -25_000, -8_000, -1, 0, 1, 3_000, 25_000, 60_000,
    ];
    for &prev in &[
        None,
        Some([0, 0, 0, 0]),
        Some([25_000, -18_000, -50_000, -8_000]),
        Some([-25_000, 0, 0, 0]),
    ] {
        for &t0 in &values {
            for &t2 in &values {
                for t13 in [(0, 0), (-18_000, -8_000), (-10_000, 0)] {
                    let t = [t0, t13.0, t2, t13.1];
                    let (parts, a) = step(prev, t, &p);
                    assert!((0..=100_000).contains(&a), "A {a} for {prev:?} {t:?}");
                    let sum: i64 = parts.iter().map(|&x| i64::from(x)).sum();
                    assert_eq!(
                        i64::from(a),
                        50_000 + sum,
                        "A = 50 + the parts for {prev:?} {t:?}"
                    );
                    if let Some(s) = prev {
                        for k in 0..4 {
                            let gap = i64::from(t[k]) - i64::from(s[k]);
                            // Half the gap, rounded half away from zero.
                            let half = gap / 2 + if gap % 2 != 0 { gap.signum() } else { 0 };
                            let want = i64::from(s[k]) + half;
                            assert_eq!(
                                i64::from(parts[k]),
                                want,
                                "part {k}: {} + 0.5 · ({} − {}), half away from zero",
                                s[k],
                                t[k],
                                s[k]
                            );
                        }
                    } else {
                        assert_eq!(&parts[..4], &t, "a new person starts at the targets");
                    }
                }
            }
        }
    }
}

/// The parameters with every approval parameter at the low or high end of
/// its range.
fn ends(high: bool) -> ApprovalParams {
    if high {
        ApprovalParams {
            income_weight_milli: 200_000,
            loss_aversion_ppm: 4_000_000,
            income_cap_milli: 50_000,
            reference_speed_ppm: 333_333,
            unemployed_self_milli: 30_000,
            unemployed_household_milli: 15_000,
            inflation_weight_milli: 5_000,
            inflation_neutral_ppm: 100_000,
            hardship_milli: 30_000,
            smoothing_ppm: 1_000_000,
        }
    } else {
        ApprovalParams {
            income_weight_milli: 0,
            loss_aversion_ppm: 1_000_000,
            income_cap_milli: 5_000,
            reference_speed_ppm: 16_667,
            unemployed_self_milli: 0,
            unemployed_household_milli: 0,
            inflation_weight_milli: 0,
            inflation_neutral_ppm: 0,
            hardship_milli: 0,
            smoothing_ppm: 100_000,
        }
    }
}

/// The daily state hashes of 24 months at 1:1000 with these approval
/// parameters.
fn hashes_with(approval: Option<ApprovalParams>) -> Vec<u64> {
    let mut p = params();
    p.approval = approval;
    let computed = approval.is_some();
    let mut w = world_2021(1000, p, Vec::new());
    let mut h = Vec::new();
    while w.months().len() < 24 {
        h.push(step_checked(&mut w).state_hash);
    }
    if computed {
        let stats = w.approval_stats();
        assert_eq!(stats.len(), 24, "approval was computed at every close");
        assert!(
            stats[23].national.is_some(),
            "the nation has an approval after 24 months"
        );
    }
    h
}

/// **AC-APP-03** `[sim]` Isolation: at 1:1000, the `World`'s state hash
/// after 24 months is the same with approval computed and not computed, and
/// with every approval parameter at its lowest and highest value.
///
/// Readings: "not computed" is `WorldParams::approval` `None`; the lowest
/// and highest values are the ends of the ranges of the spec's table (α_y
/// 0–200, κ 1–4, c^max 5–50, λ_r 1/60–1/3, u_self 0–30, u_hh 0–15, α_π 0–5,
/// π0 0–10%, h 0–30, λ_A 0.1–1), all low in one run and all high in
/// another. The test compares every day's hash, which includes the one after
/// 24 months, and checks that the runs with parameters did compute approval
/// at every close. Four runs of two years: release builds only.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "four two-year runs: cargo test --release -p econ-core --features approval --test acceptance approval::"
)]
fn ac_app_03() {
    let on = hashes_with(Some(defaults()));
    for (what, a) in [
        ("not computed", None),
        ("lowest", Some(ends(false))),
        ("highest", Some(ends(true))),
    ] {
        let other = hashes_with(a);
        assert_eq!(other.len(), on.len());
        if let Some(d) = on.iter().zip(&other).position(|(x, y)| x != y) {
            panic!("approval {what}: the state hash differs on day {d}");
        }
    }
}

/// The approval statistics of a run with an optional order given on 15
/// February 2022, after `months` closes; and the run's world.
fn run_with<F: FnOnce(&mut World)>(months: usize, order: Option<F>) -> World {
    let mut w = world_2021_default(1000);
    if let Some(give_order) = order {
        step_until(&mut w, feb_15_2022());
        give_order(&mut w);
    }
    run_closes(&mut w, months);
    w
}

fn group(s: &ApprovalStats, a: Activity) -> GroupApproval {
    s.by_activity[a as usize].unwrap_or_else(|| panic!("no approval for activity {a:?}"))
}

/// **AC-APP-04** `[sim]` At 1:1000, raising `transfer.pension.point_value`
/// by 10% from month 3 raises the mean approval of the retired within 3
/// months against the baseline, by more than that of the employed.
///
/// Readings: the point value in force is raised by 10% on 15 February 2022,
/// effective 1 March (month 3). "Within 3 months": at the close of month 5,
/// the retired's mean approval is above the baseline's, and its rise over
/// the baseline is larger than the employed's (which may be any sign).
#[test]
fn ac_app_04() {
    let base = run_with::<fn(&mut World)>(6, None);
    let rise = run_with(
        6,
        Some(|w: &mut World| {
            let now =
                super::support_m5::money_in_force(w, LeverId::TransferPensionPointValue, None);
            give(
                w,
                super::support_lev::set(
                    LeverId::TransferPensionPointValue,
                    None,
                    LeverValue::Money(now.mul_ratio(11, 10)),
                ),
            );
        }),
    );
    let b = &base.approval_stats()[5];
    let r = &rise.approval_stats()[5];
    let d_ret = i64::from(group(r, Activity::Retired).mean_milli)
        - i64::from(group(b, Activity::Retired).mean_milli);
    let d_emp = i64::from(group(r, Activity::Employed).mean_milli)
        - i64::from(group(b, Activity::Employed).mean_milli);
    assert!(
        d_ret > 0,
        "the retired's approval rises: {d_ret} millipoints"
    );
    assert!(
        d_ret > d_emp,
        "more than the employed's: {d_ret} against {d_emp}"
    );
}

/// **AC-APP-05** `[sim]` At 1:1000, a 30% `fx.devaluation` on month 3 lowers
/// the mean approval of every activity group of at least 50 synthetic adults
/// within 6 months against the baseline, and lowers it more among the
/// retired than among the employed.
///
/// Readings: the devaluation (3,000 basis points) is given on 15 February
/// 2022 with effective day 1 March 2022 (month 3). "Within 6 months": at the
/// close of month 8 (August 2022). A group's synthetic adults are the
/// baseline's resident person records of 18 or more with that activity at
/// that close (unweighted).
#[test]
fn ac_app_05() {
    let base = run_with::<fn(&mut World)>(9, None);
    let deval = run_with(
        9,
        Some(|w: &mut World| {
            give(
                w,
                set_on(
                    LeverId::FxDevaluation,
                    None,
                    LeverValue::Rate(3_000),
                    date(2022, 3, 1),
                ),
            );
        }),
    );
    let b = &base.approval_stats()[8];
    let d = &deval.approval_stats()[8];
    let adults = adults_mask(&base);
    let activity = base.activity();
    let mut checked = 0;
    for code in 0..b.by_activity.len() {
        let records = (0..adults.len())
            .filter(|&p| adults[p] && activity[p] as usize == code)
            .count();
        if records < 50 {
            continue;
        }
        let gb = b.by_activity[code].unwrap_or_else(|| panic!("activity {code}: no approval"));
        let gd = d.by_activity[code].unwrap_or_else(|| panic!("activity {code}: no approval"));
        assert!(
            gd.mean_milli < gb.mean_milli,
            "activity {code} ({records} adults): {} against {}",
            gd.mean_milli,
            gb.mean_milli
        );
        checked += 1;
    }
    assert!(
        checked >= 3,
        "at least the employed, the unemployed and the retired"
    );
    let fall = |a: Activity| i64::from(group(b, a).mean_milli) - i64::from(group(d, a).mean_milli);
    assert!(
        fall(Activity::Retired) > fall(Activity::Employed),
        "the retired lose more: {} against {}",
        fall(Activity::Retired),
        fall(Activity::Employed)
    );
}

/// Each person's employer and approval at a close.
type Snapshot = (Vec<u16>, Vec<Option<(i32, [i32; 5])>>);

/// **AC-APP-06** `[sim]` At 1:1000, every person who loses a job in a month
/// and is still looking for work at the close has approval at least λ_A
/// u_self lower than at the close before, other things (income, hardship,
/// inflation parts) aside: the unemployment part alone falls by that much.
///
/// Readings: a person "loses a job in a month" when they had an employer at
/// the close before and have none at this close, and their activity at this
/// close is `Unemployed`. The check is on the unemployment part, which falls
/// by λ_A u_self = 5 points only if it was 0 before, so it covers the persons
/// whose unemployment part could not be below 0 at the close before: the
/// person and every other member of their household were never looking for
/// work at any close before (the test follows 24 months). The persons must
/// have had approval at the close before (adults).
#[test]
fn ac_app_06() {
    let p = defaults();
    let fall = i64::from(p.unemployed_self_milli) * i64::from(p.smoothing_ppm) / 1_000_000;
    let mut w = world_2021_default(1000);
    run_closes(&mut w, 1);
    let n_hh = w.households().hh_weight.len();
    let mut ever_seeking_person = vec![false; w.persons().household_id.len()];
    let mut ever_seeking_household = vec![false; n_hh];
    let mut seen = 0;
    let snapshot = |w: &World| -> Snapshot {
        let n = w.persons().household_id.len();
        (
            w.persons().employer.clone(),
            (0..n)
                .map(|i| w.approval(u32::try_from(i).expect("fits")))
                .collect(),
        )
    };
    let mark = |w: &World, ep: &mut Vec<bool>, eh: &mut Vec<bool>| {
        ep.resize(w.persons().household_id.len(), false);
        eh.resize(w.households().hh_weight.len(), false);
        for (i, &a) in w.activity().iter().enumerate() {
            if a == Activity::Unemployed {
                ep[i] = true;
                eh[w.persons().household_id[i] as usize] = true;
            }
        }
    };
    mark(&w, &mut ever_seeking_person, &mut ever_seeking_household);
    let mut before = snapshot(&w);
    while w.months().len() < 24 {
        next_close(&mut w);
        let now = snapshot(&w);
        let activity = w.activity();
        for i in 0..before.0.len() {
            let lost = before.0[i] != NO_EMPLOYER
                && now.0[i] == NO_EMPLOYER
                && activity[i] == Activity::Unemployed;
            let h = w.persons().household_id[i] as usize;
            if !lost || ever_seeking_person[i] || ever_seeking_household[h] {
                continue;
            }
            let (Some((_, prev)), Some((_, parts))) = (before.1[i], now.1[i]) else {
                continue;
            };
            seen += 1;
            let d = i64::from(parts[UNEMPLOYMENT]) - i64::from(prev[UNEMPLOYMENT]);
            assert!(
                d <= -fall,
                "month {}: person {i} lost a job; unemployment part {} → {}",
                w.months().len() - 1,
                prev[UNEMPLOYMENT],
                parts[UNEMPLOYMENT]
            );
        }
        mark(&w, &mut ever_seeking_person, &mut ever_seeking_household);
        before = now;
    }
    assert!(seen > 0, "some adults lost a job in two years");
}

/// The groups of a close, by name and index, `None` included.
fn groups(s: &ApprovalStats) -> Vec<(String, Option<GroupApproval>)> {
    let mut g = vec![("national".to_string(), s.national)];
    let mut add = |name: &str, v: &[Option<GroupApproval>]| {
        for (i, x) in v.iter().enumerate() {
            g.push((format!("{name} {i}"), *x));
        }
    };
    add("activity", &s.by_activity);
    add("fifth", &s.by_fifth);
    add("region", &s.by_region);
    add("county", &s.by_county);
    add("age", &s.by_age);
    add("edu", &s.by_edu);
    add("sex", &s.by_sex);
    g
}

/// The weighted adults of a world, as (person, weight).
fn adult_set(w: &World) -> Vec<(usize, u32)> {
    let adults = adults_mask(w);
    let weights = weights_of_persons(w);
    (0..adults.len())
        .filter(|&p| adults[p])
        .map(|p| (p, weights[p]))
        .collect()
}

/// **AC-APP-07** `[sim]` Explanation: at every close of a 24-month run at
/// 1:1000, for the nation and for each built-in group, the change of mean
/// approval equals the sum of the changes of its parts' means plus the
/// composition part, exactly in millipoints.
///
/// Readings: checked for every group that has approval at both closes. The
/// composition part must also be what its name says: when the set of adults
/// (persons and weights) is the same at both closes, the nation's
/// composition part is 0.
#[test]
fn ac_app_07() {
    let mut w = world_2021_default(1000);
    run_closes(&mut w, 1);
    let mut adults_before = adult_set(&w);
    while w.months().len() < 24 {
        next_close(&mut w);
        let stats = w.approval_stats();
        let t = stats.len() - 1;
        let (now, before) = (&stats[t], &stats[t - 1]);
        let a = groups(now);
        let b = groups(before);
        assert_eq!(a.len(), b.len(), "month {t}: the same groups");
        for ((name, x), (_, y)) in a.iter().zip(&b) {
            let (Some(x), Some(y)) = (x, y) else { continue };
            let change = i64::from(x.mean_milli) - i64::from(y.mean_milli);
            let parts: i64 = (0..5)
                .map(|k| i64::from(x.parts_milli[k]) - i64::from(y.parts_milli[k]))
                .sum();
            assert_eq!(
                change,
                parts + i64::from(x.composition_milli),
                "month {t}, {name}: change {change} against parts {parts} + composition {}",
                x.composition_milli
            );
        }
        let adults_now = adult_set(&w);
        if adults_now == adults_before {
            let n = now.national.expect("adults have approval");
            assert_eq!(
                n.composition_milli, 0,
                "month {t}: no adult entered or left the nation"
            );
        }
        adults_before = adults_now;
    }
}

/// **AC-APP-08** `[unit]` Who counts: persons under 18 have no approval and
/// are not in any mean; a group with no adult has none.
///
/// Readings: checked on the `World` at 1:1000 at the closes of months 0 and
/// 1: a person has approval exactly when they are a resident of 18 or more,
/// with `A_i` = 50 + its parts; the nation's adults are the weighted
/// residents of 18 or more, and so are the sums of the activity, sex and
/// fifth groups' adults; the activity group "child" (below school age) has
/// no adult and so no approval.
#[test]
fn ac_app_08() {
    let mut w = world_2021_default(1000);
    for t in 0..2 {
        next_close(&mut w);
        let adults = adults_mask(&w);
        let weights = weights_of_persons(&w);
        let mut weighted = 0u64;
        for (i, &adult) in adults.iter().enumerate() {
            let a = w.approval(u32::try_from(i).expect("fits"));
            assert_eq!(a.is_some(), adult, "month {t}: person {i} (adult {adult})");
            if let Some((ai, parts)) = a {
                weighted += u64::from(weights[i]);
                let sum: i64 = parts.iter().map(|&x| i64::from(x)).sum();
                assert_eq!(i64::from(ai), 50_000 + sum, "month {t}: person {i}");
                assert!((0..=100_000).contains(&ai));
            }
        }
        let s = &w.approval_stats()[t];
        let n = s.national.expect("the nation has adults");
        assert_eq!(n.adults, weighted, "month {t}: the nation's adults");
        let total =
            |v: &[Option<GroupApproval>]| -> u64 { v.iter().flatten().map(|g| g.adults).sum() };
        assert_eq!(
            total(&s.by_activity),
            weighted,
            "month {t}: activity groups"
        );
        assert_eq!(total(&s.by_sex), weighted, "month {t}: sex groups");
        assert_eq!(total(&s.by_fifth), weighted, "month {t}: fifths");
        assert!(
            s.by_activity[Activity::Child as usize].is_none(),
            "month {t}: children below school age have no approval"
        );
    }
    // Households with only children: none of their members has approval.
    let ages = w.person_age_months();
    for m in members(&w) {
        if !m.is_empty() && m.iter().all(|&p| ages[p] < super::support_m5::ADULT_MONTHS) {
            for &p in &m {
                assert!(w.approval(u32::try_from(p).expect("fits")).is_none());
            }
        }
    }
}
