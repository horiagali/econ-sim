//! Acceptance tests of the labour market's pay and job choice (spec
//! economy/labor-market, "Pay and job choice", roadmap M4): AC-LM-11 and
//! AC-LM-12. Written from the spec before the code (ADR-0014); behind the
//! feature `levers`, since public pay is a lever (spec game/lever-system,
//! `spend.public_pay`). The formulas of the section are reached through the
//! test writer's addition `econ_mech_labour::pay_choice`.

use econ_core::levers::{LeverId, LeverValue};
use econ_core::world::World;
use econ_mech_labour::pay_choice::{PayParams, fill_rate_with_pay_ppm, quit_probability_ppm};
use econ_popgen::NO_EMPLOYER;
use econ_types::{Date, Day};

use super::support::{assert_day_checks, person_weights, staff_from_persons, step};
use super::support_lev::{date, give, key_of, lever_world, public_unit, set};

/// What one unit did over a span of days, in weighted persons.
#[derive(Debug, Default, Clone, Copy)]
struct UnitYear {
    /// Vacancies on its staffing days: desired employment above the staff it
    /// had at the end of the day before.
    vacancies: u64,
    /// Persons who joined it.
    hires: u64,
    /// Persons who left it.
    losses: u64,
    /// Its staff summed over the days.
    staff_days: u64,
}

/// Run a world through `last`, following unit `u` from `from` on, checking
/// every day.
fn follow_unit(w: &mut World, u: usize, from: Date, last: Date) -> UnitYear {
    let weights = person_weights(w);
    let mut y = UnitYear::default();
    while w.date() <= last {
        let counting = w.date() >= from;
        let before = w.persons().employer.clone();
        let staff_before = w.firm_units().staff[u];
        let r = step(w);
        assert_day_checks(w, &r);
        if !counting {
            continue;
        }
        let f = w.firm_units();
        if f.last_staffing[u] == Some(r.day) {
            y.vacancies += f.desired_employment[u].saturating_sub(staff_before);
        }
        for (p, (&was, &now)) in before.iter().zip(&w.persons().employer).enumerate() {
            let (was_here, is_here) = (usize::from(was) == u, usize::from(now) == u);
            let wt = u64::from(weights[p]);
            if is_here && !was_here {
                y.hires += wt;
            }
            if was_here && !is_here {
                y.losses += wt;
            }
        }
        y.staff_days += f.staff[u];
    }
    y
}

/// **AC-LM-11** `[sim]` *(M4)* Two worlds differing only in one public
/// unit's pay (80% against 100% of the start): over a year the lower-paid
/// unit fills fewer of its vacancies and loses more staff; with every
/// relative pay at 1 the rates are those of M2.
///
/// Readings: the unit is the education public unit; its pay is set by
/// `spend.public_pay[education_services.public]` given on 1 December 2021,
/// in effect from 1 January 2022, and the year is 2022. "Fills fewer of its
/// vacancies": its hires over its vacancies (desired employment above its
/// staff on each staffing day) are a smaller share; "loses more staff": the
/// persons who leave it over its staff summed over the days are a larger
/// share (rates, since the two units' sizes drift apart). "The rates of
/// M2": the formulas of the section at relative pay 1 (`rho` = 1,000,000)
/// give M2's fill rate and quit rate unchanged; the spec's defaults and the
/// formulas at other relative pay are checked with them.
#[test]
fn ac_lm_11() {
    // The formulas at relative pay 1, and around it.
    let p = PayParams::spec_defaults();
    assert_eq!(p.fill_wage_elasticity_ppm, 2_000_000);
    assert_eq!(p.quit_wage_elasticity_ppm, 2_000_000);
    assert_eq!(p.quit_min_ppm, 3_000);
    assert_eq!(p.quit_max_ppm, 40_000);
    assert_eq!(p.job_to_job_share_ppm, 500_000);
    for q in [0, 1, 250_000, 500_000, 999_999, 1_000_000] {
        assert_eq!(
            fill_rate_with_pay_ppm(q, 1_000_000, &p),
            q,
            "M2's fill rate {q}"
        );
    }
    assert_eq!(
        quit_probability_ppm(10_000, 1_000_000, &p),
        10_000,
        "M2's quit rate"
    );
    let close = |got: u32, want: u32, what: &str| {
        assert!(got.abs_diff(want) <= 1, "{what}: got {got}, want {want}");
    };
    close(
        fill_rate_with_pay_ppm(500_000, 800_000, &p),
        320_000,
        "q at 80% pay",
    );
    close(
        fill_rate_with_pay_ppm(500_000, 1_200_000, &p),
        720_000,
        "q at 120% pay",
    );
    assert_eq!(
        fill_rate_with_pay_ppm(500_000, 1_500_000, &p),
        1_000_000,
        "q capped at 1"
    );
    close(
        quit_probability_ppm(10_000, 800_000, &p),
        15_625,
        "quits at 80% pay",
    );
    assert_eq!(
        quit_probability_ppm(10_000, 400_000, &p),
        40_000,
        "quit_max"
    );
    assert_eq!(
        quit_probability_ppm(10_000, 2_000_000, &p),
        3_000,
        "quit_min"
    );

    // Two worlds.
    let u = public_unit(1000, "education_services");
    let run = |bp: i32| {
        let mut w = lever_world(1000);
        let key = key_of(&w, LeverId::SpendPublicPay, "education_services.public");
        let a = give(
            &mut w,
            set(LeverId::SpendPublicPay, Some(key), LeverValue::Rate(bp)),
        );
        assert_eq!(a.effective, date(2022, 1, 1));
        follow_unit(&mut w, u, date(2022, 1, 1), date(2022, 12, 31))
    };
    let full = run(10_000);
    let low = run(8_000);
    assert!(full.vacancies > 0 && low.vacancies > 0, "{full:?} {low:?}");
    assert!(full.staff_days > 0 && low.staff_days > 0);
    // hires / vacancies lower at 80%.
    assert!(
        u128::from(low.hires) * u128::from(full.vacancies)
            < u128::from(full.hires) * u128::from(low.vacancies),
        "the lower-paid unit fills fewer of its vacancies: {low:?} against {full:?}"
    );
    // losses / staff higher at 80%.
    assert!(
        u128::from(low.losses) * u128::from(full.staff_days)
            > u128::from(full.losses) * u128::from(low.staff_days),
        "the lower-paid unit loses more staff: {low:?} against {full:?}"
    );
}

/// **AC-LM-12** `[sim]` *(M4)* Job-to-job moves: underpaid quitters hired by
/// better-paying units count as hires with no unemployment spell, and
/// employment counted from persons still equals units' staff every day.
///
/// Readings: over 2022, on a world whose education public unit is paid 80%
/// of its start (so that some of its quitters are underpaid), a job-to-job
/// move is a person whose employer changes from one unit to another between
/// two consecutive days with no day in between without one. Each such move
/// is a hire of the new unit (the person's `hired_on` is that day, and the
/// unit's staff counts the person: staff equals employment counted from
/// persons every day), with no unemployment spell (`months_unemployed` stays
/// 0), into a job paying more than the one left (the spec: "only for
/// vacancies paying more than their current wage"; not checked on 1
/// January, when the wage round and the pay lever move wages on the same
/// day). At least one such move happens.
#[test]
fn ac_lm_12() {
    let mut w = lever_world(1000);
    let key = key_of(&w, LeverId::SpendPublicPay, "education_services.public");
    give(
        &mut w,
        set(LeverId::SpendPublicPay, Some(key), LeverValue::Rate(8_000)),
    );
    let last = date(2022, 12, 31);
    let mut moves = 0u32;
    while w.date() <= last {
        let today = w.date();
        let employer = w.persons().employer.clone();
        let wage = w.persons().wage.clone();
        let r = step(&mut w);
        assert_day_checks(&w, &r);
        assert_eq!(
            staff_from_persons(&w),
            w.firm_units().staff,
            "{today:?}: employment from persons equals units' staff"
        );
        let p_now = w.persons();
        for (p, (&was, &now)) in employer.iter().zip(&p_now.employer).enumerate() {
            if was == NO_EMPLOYER || now == NO_EMPLOYER || was == now {
                continue;
            }
            moves += 1;
            assert_eq!(
                p_now.hired_on[p],
                Some(r.day),
                "person {p}: hired on {today:?}"
            );
            assert_eq!(
                p_now.months_unemployed[p], 0,
                "person {p}: no unemployment spell"
            );
            if !(today.month == 1 && today.day == 1) {
                assert!(
                    p_now.wage[p] > wage[p],
                    "person {p} moved on {today:?} to a better-paid job: {:?} after {:?}",
                    p_now.wage[p],
                    wage[p]
                );
            }
        }
    }
    assert!(moves > 0, "job-to-job moves happen in a year");
    assert!(w.day() > Day(365));
}
