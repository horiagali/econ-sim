//! Acceptance tests for AC-WLD-01 to AC-WLD-04 and AC-WLD-06 to AC-WLD-10
//! (spec: game/world-loop). Written in a test-authoring session from the spec
//! and its API sketch, before the code existed (ADR-0014). The golden state
//! hash of day 31 is owed until the first implementation produces it and the
//! owner approves it (ADR-0010).

use econ_core::Command;
use econ_core::schedule::{Staggered, stagger_day};
use econ_core::world::{Law, PhaseId, World, WorldError};
use econ_rng::KeyedRng;
use econ_types::{Bani, Day};

use super::support::{
    SEED, assert_day_checks, assert_everyone_ran_this_month, assert_staggered_today,
    build_scenario, calendar, expected_phases, params, ppm, run_checked, scenario, start_state,
    start_state_of, step, world,
};

/// Every household's and unit's stagger day is the keyed draw of the
/// scheduler.
fn assert_stagger_days_are_drawn(w: &World) {
    let seed = KeyedRng::new(SEED);
    for (i, &d) in w.households().hh_stagger_day.iter().enumerate() {
        assert_eq!(
            d,
            stagger_day(seed, Staggered::Household, i as u64),
            "household {i}"
        );
    }
    for (u, &d) in w.firm_units().unit_stagger_day.iter().enumerate() {
        assert_eq!(
            d,
            stagger_day(seed, Staggered::FirmUnit, u as u64),
            "unit {u}"
        );
    }
}

/// A world built from a scenario changed after the build: it must be refused.
fn assert_refused(start: econ_core::world::StartState, what: &str) {
    match World::new(start, Law::placeholder_2021(), params()) {
        Err(WorldError::TablesDisagreeWithLedger { what: w }) => {
            assert!(!w.is_empty(), "{what}: the error names the balance");
        }
        Err(e) => panic!("{what}: refused with {e:?}, not TablesDisagreeWithLedger"),
        Ok(_) => panic!("{what}: a world was built from tables that disagree with the ledger"),
    }
}

/// **AC-WLD-01** `[sim]` `step` advances one day. Over any 400 days, each
/// phase runs exactly on the days its period is due, in the order of the
/// list; each household and each firm unit runs its staggered phases exactly
/// once in every calendar month, on its own day.
#[test]
fn ac_wld_01() {
    let mut w = world(1000);
    let cal = calendar(&w);
    assert_stagger_days_are_drawn(&w);
    let days_before = w.households().hh_stagger_day.clone();
    for n in 0..400u32 {
        assert_eq!(w.day(), Day(n));
        let date = w.date();
        assert_eq!(date, cal.date(Day(n)));
        let r = step(&mut w);
        assert_eq!(r.date, date);
        assert_eq!(w.day(), Day(n + 1), "step advances one day");
        assert_eq!(
            r.phases,
            expected_phases(date),
            "phases of {date:?}, in the order of the list"
        );
        assert_staggered_today(&w, &r);
        if date.is_month_end() {
            assert_everyone_ran_this_month(&w, cal, &r);
        }
    }
    assert_eq!(
        w.households().hh_stagger_day,
        days_before,
        "a household keeps its day"
    );
}

/// **AC-WLD-02** `[sim]` On the Romania scenario at 1:1000, 600 days pass
/// with every check of phase 13 holding on every day: ledger rows sum to
/// zero, clearing accounts net to zero, households' and firm units' columns
/// equal the ledger's sector balances (I-4), no negative cash, deposit or
/// inventory.
#[test]
fn ac_wld_02() {
    let mut w = world(1000);
    run_checked(&mut w, 600);
    assert_eq!(w.day(), Day(600));
}

/// **AC-WLD-03** `[sim]` The same as AC-WLD-02 at 1:100: 600 days with every
/// check of phase 13 holding on every day. (The 1:10 run is the ignored test
/// below, for CI time.)
#[test]
fn ac_wld_03() {
    let mut w = world(100);
    run_checked(&mut w, 600);
    assert_eq!(w.day(), Day(600));
}

/// **AC-WLD-03** `[sim]` at 1:10: 600 days with every check of phase 13
/// holding on every day. Ignored for CI time.
#[test]
#[ignore = "1:10 takes too long for CI; run with --ignored"]
fn ac_wld_03_one_in_ten() {
    let start = start_state_of(&build_scenario(10), "romania_10");
    let mut w = World::new(start, Law::placeholder_2021(), params()).expect("the world builds");
    run_checked(&mut w, 600);
}

/// **AC-WLD-04** `[sim]` Determinism: two worlds from the same scenario and
/// seed give the same state hash on every day; a world cloned on day 200 and
/// stepped to day 400 gives the same hashes as the original.
#[test]
fn ac_wld_04() {
    let mut a = world(1000);
    let mut b = world(1000);
    assert_eq!(a.state_hash(), b.state_hash(), "day 0");
    let mut hashes = Vec::new();
    for n in 0..200 {
        let (ra, rb) = (step(&mut a), step(&mut b));
        assert_eq!(ra.state_hash, rb.state_hash, "day {n}");
        assert_eq!(ra.state_hash, a.state_hash(), "day {n}: the report's hash");
        hashes.push(ra.state_hash);
    }
    let mut c = a.clone();
    assert_eq!(c.state_hash(), a.state_hash(), "the clone on day 200");
    assert_eq!(c.day(), Day(200));
    for n in 200..400 {
        let (ra, rb, rc) = (step(&mut a), step(&mut b), step(&mut c));
        assert_eq!(ra.state_hash, rb.state_hash, "day {n}");
        assert_eq!(rc.state_hash, ra.state_hash, "day {n}: the clone");
        hashes.push(ra.state_hash);
    }
    // The hash follows the state: it does not stay the same for a year.
    assert!(
        hashes.windows(2).any(|p| p[0] != p[1]),
        "the state hash never changes"
    );
}

/// **AC-WLD-06** `[sim]` At every monthly close GDP by production, by
/// expenditure and by income agree to the ban (I-7).
#[test]
fn ac_wld_06() {
    let mut w = world(1000);
    let mut closes = 0;
    for _ in 0..400 {
        let r = step(&mut w);
        if r.phases.contains(&PhaseId::MonthlyClose) {
            closes += 1;
            let m = w.months().last().expect("a closed month");
            assert_eq!(m.gdp_production, m.gdp_expenditure, "{:?}", m.month);
            assert_eq!(m.gdp_production, m.gdp_income, "{:?}", m.month);
            assert!(m.gdp_production > Bani::ZERO, "{:?}: GDP", m.month);
        }
    }
    assert_eq!(
        closes, 13,
        "thirteen month ends in 400 days from 1 December"
    );
    assert_eq!(w.months().len(), 13);
}

/// **AC-WLD-07** `[sim]` Quiet baseline (roadmap M2 exit): over 600 days on
/// the Romania scenario at 1:100 with no command, the unemployment rate stays
/// within 3 points of its start, twelve-month inflation (from day 365) within
/// 3 points of the target, and the wage share and the household consumption
/// share of GDP within 5 points of their first month.
#[test]
fn ac_wld_07() {
    let mut w = world(100);
    let target = i64::from(w.inflation_target_ppm());
    for _ in 0..600 {
        step(&mut w);
    }
    let months = w.months();
    assert_eq!(months.len(), 19, "nineteen closed months in 600 days");
    let rate = |m: &econ_core::world::MonthStats| {
        ppm(
            i128::from(m.unemployed),
            i128::from(m.employed + m.unemployed),
        )
    };
    let wage_share = |m: &econ_core::world::MonthStats| {
        ppm(
            i128::from(m.wage_bill.get()),
            i128::from(m.gdp_production.get()),
        )
    };
    let consumption_share = |m: &econ_core::world::MonthStats| {
        ppm(
            i128::from(m.consumption.get()),
            i128::from(m.gdp_production.get()),
        )
    };
    let first = &months[0];
    for m in months {
        assert!(
            (rate(m) - rate(first)).abs() <= 30_000,
            "{:?}: unemployment rate {} ppm against {} at the start",
            m.month,
            rate(m),
            rate(first)
        );
        assert!(
            (wage_share(m) - wage_share(first)).abs() <= 50_000,
            "{:?}: wage share {} ppm against {}",
            m.month,
            wage_share(m),
            wage_share(first)
        );
        assert!(
            (consumption_share(m) - consumption_share(first)).abs() <= 50_000,
            "{:?}: consumption share {} ppm against {}",
            m.month,
            consumption_share(m),
            consumption_share(first)
        );
    }
    // Month 12 (December 2022) is the first to close after day 365.
    for m in &months[12..] {
        assert!(
            (i64::from(m.inflation_ppm) - target).abs() <= 30_000,
            "{:?}: twelve-month inflation {} ppm, target {target}",
            m.month,
            m.inflation_ppm
        );
    }
}

/// **AC-WLD-08** `[unit]` A world refuses a scenario whose tables disagree
/// with its opening ledger (I-4 at day 0) with a named error, and builds
/// nothing.
#[test]
fn ac_wld_08() {
    // One ban more in a household's deposits than the ledger holds.
    let mut s = scenario(1000).clone();
    let h = s
        .population
        .households
        .hh_weight
        .iter()
        .position(|&w| w > 0)
        .expect("a household");
    s.stages.wealth.hh_deposits[h] += 1;
    assert_refused(start_state_of(&s, "disagree_households"), "households");
    // One ban more in a firm unit's deposits.
    let mut s = scenario(1000).clone();
    s.firm_units.deposits[0] += 1;
    assert_refused(start_state_of(&s, "disagree_firms"), "firm units");
    // The unchanged scenario builds.
    assert!(World::new(start_state(1000), Law::placeholder_2021(), params()).is_ok());
}

/// **AC-WLD-09** `[unit]` `step` with a non-empty command list is refused in
/// M2 with a named error and changes nothing.
#[test]
fn ac_wld_09() {
    let mut w = world(1000);
    for n in 0..3u32 {
        let (hash, day) = (w.state_hash(), w.day());
        let refused = w.step(&[Command::SetGovSpending(Bani::from_lei(1))]);
        assert_eq!(refused, Err(WorldError::CommandsNotYet), "day {n}");
        assert_eq!(w.state_hash(), hash, "day {n}: nothing changed");
        assert_eq!(w.day(), day, "day {n}: the clock did not move");
        assert!(w.months().is_empty());
        step(&mut w);
        assert_eq!(w.day(), Day(n + 1));
    }
}

/// **AC-WLD-10** `[bench]` At 1:100 a simulated year takes at most 3 seconds
/// on CI hardware in a release build (Spike 4 and spike 10 measured about
/// 0.2 s for the scale world).
///
/// Restated on 2026-11-01 with the spec (owner's delegation): the budget was
/// 2 seconds; the World with every M3 and M4 mechanic takes about 1.5 s on a
/// developer machine and 2.15 to 2.3 s on slow CI runners.
///
/// A time budget means nothing in a debug build:
/// the test runs in release builds only (`cargo test --release -p econ-core
/// --features world --test acceptance ac_wld_10`).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build time budget: run with cargo test --release"
)]
#[allow(clippy::disallowed_types)]
fn ac_wld_10() {
    let mut w = world(100);
    let started = std::time::Instant::now();
    for _ in 0..365 {
        step(&mut w);
    }
    let took = started.elapsed();
    assert!(
        took.as_millis() <= 3_000,
        "a simulated year at 1:100 took {took:?}"
    );
    // Every phase ran (a fast year that skips work is no year).
    assert_eq!(w.months().len(), 12);
    let r = step(&mut w);
    assert_day_checks(&w, &r);
}
