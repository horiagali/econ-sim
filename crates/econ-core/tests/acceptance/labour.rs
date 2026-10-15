//! Acceptance tests for the `[sim]` criteria of the labour market on the
//! `World`: AC-LM-03 to AC-LM-07 (spec: economy/labor-market, "First
//! version"). Written in a test-authoring session from the specs and their
//! API sketches, before the code existed (ADR-0014).

use econ_core::world::{Law, StartEdit, World};
use econ_ledger::{FlowCode, Sector};
use econ_mech_labour::part_month_pay;
use econ_popgen::{EduLevel, NO_EMPLOYER};
use econ_types::{Bani, Calendar, Day};

use super::support::{
    assert_day_checks, calendar, month_stats, person_weights, ppm, staff_from_persons, step,
    weighted, world, world_under, world_with,
};

/// The minimum wage of the placeholder law.
const MINIMUM_WAGE_LEI: i64 = 2_300;

/// Whether a person's education qualifies for an occupation's skill tier
/// (high: ISCO 1 to 3, ISCED 5 to 8; mid: ISCO 4 to 8, ISCED 3 and above;
/// low: ISCO 9, any).
fn qualifies(occupation: u8, edu: EduLevel) -> bool {
    match occupation {
        1..=3 => edu >= EduLevel::Isced5To6,
        4..=8 => edu >= EduLevel::Isced3,
        9 => true,
        other => panic!("a hire into occupation {other}"),
    }
}

/// Whether a day falls in the calendar month of another.
fn same_month(cal: Calendar, a: Day, b: Day) -> bool {
    cal.month(a) == cal.month(b)
}

/// Mean wage of persons employed by the same unit in both snapshots, over
/// the second, in millionths.
fn wage_growth_ppm(w: &World, employer_before: &[u16], wage_before: &[Bani], round: Day) -> i64 {
    let weights = person_weights(w);
    let p = w.persons();
    let (mut before, mut after) = (0i128, 0i128);
    for i in 0..weights.len() {
        if p.employer[i] != NO_EMPLOYER
            && p.employer[i] == employer_before[i]
            && p.hired_on[i] != Some(round)
        {
            before += i128::from(weights[i]) * i128::from(wage_before[i].get());
            after += i128::from(weights[i]) * i128::from(p.wage[i].get());
        }
    }
    ppm(after, before) - 1_000_000
}

/// **AC-LM-03** `[sim]` Employment counted from persons equals the sum of
/// units' staff on every day, and each unit's staffing runs exactly once a
/// month.
#[test]
fn ac_lm_03() {
    let mut w = world(1000);
    let cal = calendar(&w);
    let units = w.firm_units().industry.len();
    let mut staffed = vec![0u32; units];
    for _ in 0..400 {
        let r = step(&mut w);
        let from_persons = staff_from_persons(&w);
        assert_eq!(
            from_persons,
            w.firm_units().staff,
            "day {}: each unit's staff",
            r.day.0
        );
        let f = w.firm_units();
        for (u, n) in staffed.iter_mut().enumerate() {
            let due = r.date.day <= 28 && f.unit_stagger_day[u] == r.date.day;
            assert_eq!(f.last_staffing[u] == Some(r.day), due, "unit {u}");
            if due {
                *n += 1;
            }
        }
        if r.date.is_month_end() {
            let m = cal.month(r.day);
            for (u, n) in staffed.iter_mut().enumerate() {
                assert_eq!(*n, 1, "unit {u} staffed {n} times in month {}", m.0);
                *n = 0;
            }
            let employed: u64 = from_persons.iter().sum();
            assert!(employed > 0);
        }
    }
}

/// **AC-LM-04** `[sim]` Wages paid by units in a month equal wages received
/// by households, through clearing accounts that net to zero; a person hired
/// on the 16th of a 30-day month is paid half a month.
#[test]
#[allow(clippy::needless_range_loop)]
fn ac_lm_04() {
    let mut w = world(1000);
    let cal = calendar(&w);
    let mut employer_at_start = w.persons().employer.clone();
    let mut part_months = 0;
    for _ in 0..400 {
        let r = step(&mut w);
        assert_day_checks(&w, &r);
        if !r.date.is_month_end() {
            continue;
        }
        // The month's wages: what firms paid is what households received, and
        // what the persons got.
        let flows = w.ledger().flows();
        let paid = -flows.get(FlowCode::WAGES, Sector::Firms);
        let received = flows.get(FlowCode::WAGES, Sector::Households);
        let weights = person_weights(&w);
        let p = w.persons();
        assert!(received > Bani::ZERO, "{:?}: wages paid", r.date);
        assert_eq!(paid, received, "{:?}: paid and received", r.date);
        assert_eq!(
            weighted(&weights, &p.last_pay),
            received,
            "{:?}: the persons' pay",
            r.date
        );
        assert_eq!(
            w.months().last().expect("the month closed").wage_bill,
            received
        );
        // Part months: hired this month from unemployment, still there.
        let dim = r.date.days_in_month();
        for i in 0..weights.len() {
            if p.employer[i] == NO_EMPLOYER {
                continue;
            }
            match p.hired_on[i] {
                Some(d) if same_month(cal, d, r.day) => {
                    if employer_at_start[i] != NO_EMPLOYER {
                        continue;
                    }
                    let hired = cal.date(d).day;
                    let worked = dim - hired + 1;
                    assert_eq!(
                        p.last_pay[i],
                        part_month_pay(p.wage[i], worked, dim),
                        "person {i} hired on {:?}",
                        cal.date(d)
                    );
                    part_months += 1;
                }
                _ if employer_at_start[i] == p.employer[i] => {
                    assert_eq!(p.last_pay[i], p.wage[i], "person {i}: a full month");
                }
                _ => {}
            }
        }
        employer_at_start = p.employer.clone();
    }
    assert!(part_months > 0, "someone was hired during a month");
}

/// **AC-LM-05** `[sim]` Nobody is paid below the minimum wage; raising it
/// lifts exactly the wages below the new level, from the day it takes
/// effect.
///
/// Reading: the law a world is built under is in effect from day 0, so a
/// world under a higher minimum wage starts with exactly the wages below it
/// lifted (no lever moves the law mid-run in M2).
#[test]
fn ac_lm_05() {
    let minimum = Bani::from_lei(MINIMUM_WAGE_LEI);
    let mut w = world(1000);
    assert_eq!(Law::placeholder_2021().minimum_wage, minimum);
    // Raising it: the same scenario under 2,550 lei.
    let higher = Bani::from_lei(2_550);
    let mut law = Law::placeholder_2021();
    law.minimum_wage = higher;
    let mut raised = world_under(1000, law);
    let (p, q) = (w.persons(), raised.persons());
    let mut lifted = 0;
    for i in 0..p.wage.len() {
        assert_eq!(p.employer[i], q.employer[i], "person {i}");
        if p.employer[i] == NO_EMPLOYER {
            assert_eq!(q.wage[i], p.wage[i], "person {i} has no job");
        } else {
            assert!(p.wage[i] >= minimum, "person {i}: {:?} at load", p.wage[i]);
            assert_eq!(q.wage[i], p.wage[i].max(higher), "person {i}");
            if p.wage[i] < higher {
                lifted += 1;
            }
        }
    }
    assert!(lifted > 0, "some wages were below 2,550 lei");
    // Paid: nobody below the minimum, from the first pay day on, a year long.
    for _ in 0..400 {
        let r = step(&mut w);
        assert_eq!(step(&mut raised).date, r.date);
        if !r.date.is_month_end() {
            continue;
        }
        for (world, floor) in [(&w, minimum), (&raised, higher)] {
            let p = world.persons();
            for i in 0..p.wage.len() {
                if p.employer[i] != NO_EMPLOYER {
                    assert!(p.wage[i] >= floor, "{:?}: person {i}", r.date);
                    if p.hired_on[i].is_none() {
                        assert!(p.last_pay[i] >= floor, "{:?}: person {i} paid", r.date);
                    }
                }
            }
        }
    }
}

/// **AC-LM-06** `[sim]` A permanent 10% rise in every unit's desired
/// employment lowers unemployment over the following months and raises wage
/// growth at the next January round.
#[test]
fn ac_lm_06() {
    let mut base = world(1000);
    let mut more = world_with(
        1000,
        vec![StartEdit::ScaleLabourRequirement {
            factor_ppm: 1_100_000,
        }],
    );
    let cal = calendar(&base);
    let mut growth = [0i64; 2];
    for (k, w) in [&mut base, &mut more].into_iter().enumerate() {
        // To 31 December, then the round of 1 January 2022.
        while cal.date(w.day()).month != 1 {
            step(w);
        }
        let employer = w.persons().employer.clone();
        let wage = w.persons().wage.clone();
        let round = w.day();
        let r = step(w);
        assert_eq!((r.date.month, r.date.day), (1, 1));
        assert!(r.phases.contains(&econ_core::world::PhaseId::WageRound));
        growth[k] = wage_growth_ppm(w, &employer, &wage, round);
        while w.months().len() < 6 {
            step(w);
        }
    }
    assert!(
        growth[1] > growth[0],
        "wage growth at the January round: {} ppm with more demand for labour, {} without",
        growth[1],
        growth[0]
    );
    for m in 1..6 {
        let (b, x) = (month_stats(&base, m), month_stats(&more, m));
        let rate = |s: &econ_core::world::MonthStats| {
            ppm(
                i128::from(s.unemployed),
                i128::from(s.employed + s.unemployed),
            )
        };
        assert!(
            rate(x) < rate(b),
            "month {m}: unemployment {} ppm with more demand for labour, {} without",
            rate(x),
            rate(b)
        );
    }
}

/// **AC-LM-07** `[sim]` Only persons whose education qualifies are hired
/// into a tier.
#[test]
fn ac_lm_07() {
    let mut w = world(1000);
    let mut hires = 0;
    for _ in 0..400 {
        let r = step(&mut w);
        let p = w.persons();
        for i in 0..p.hired_on.len() {
            if p.hired_on[i] == Some(r.day) {
                assert_ne!(p.employer[i], NO_EMPLOYER, "person {i}");
                assert!(
                    qualifies(p.occupation[i], p.edu_level[i]),
                    "day {}: person {i} with {:?} hired into occupation {}",
                    r.day.0,
                    p.edu_level[i],
                    p.occupation[i]
                );
                hires += 1;
            }
        }
    }
    assert!(hires > 0, "someone was hired in 400 days");
}
