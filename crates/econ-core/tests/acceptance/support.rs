//! Shared by the acceptance tests of the `World` (spec game/world-loop) and
//! of the four M2 mechanics' `[sim]` criteria: the Romania scenario built
//! once at 1:1000 and 1:100 (seed 42), written once and read back as a
//! `StartState`; worlds at the spec defaults under the placeholder law;
//! stepping; weighted sums; the checks of the day.

// Each test file uses a part of this one.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use econ_core::schedule::{Due, Period};
use econ_core::world::{
    DayReport, Law, MonthStats, PhaseId, StartEdit, StartState, World, WorldParams,
};
use econ_io::read_start_state;
use econ_ledger::{Instrument, Sector};
use econ_popgen::NO_EMPLOYER;
use econ_scenario::{BuildParams, Inputs, Scenario, build, read_inputs, write};
use econ_types::{Bani, Calendar, Date, Day, Month};
use serde_json::Value;

/// The seed of every test scenario and world.
pub const SEED: u64 = 42;
/// One, in millionths.
pub const ONE: i64 = 1_000_000;
/// The input-output table.
pub const IO_TABLE: &str = "io_table2021_ro.json";

/// The phases of a day and their periods: the spec's table.
#[cfg(not(feature = "demography"))]
pub const PHASES: [(PhaseId, Period); 13] = [
    (PhaseId::Commands, Period::Daily),
    (PhaseId::Plan, Period::Staggered),
    (PhaseId::Staffing, Period::Staggered),
    (PhaseId::Prices, Period::Daily),
    (PhaseId::OutputAndInputs, Period::Daily),
    (PhaseId::FixedDemand, Period::Daily),
    (PhaseId::Households, Period::Staggered),
    (PhaseId::GoodsMarket, Period::Daily),
    (PhaseId::Quits, Period::MonthEnd),
    (PhaseId::Pay, Period::MonthEnd),
    (PhaseId::WageRound, Period::Yearly { month: 1, day: 1 }),
    (PhaseId::MonthlyClose, Period::MonthEnd),
    (PhaseId::Checks, Period::Daily),
];

/// The same with demographics' People phase, at month end after Pay and
/// before the wage round and the monthly close (spec economy/demographics,
/// "Periods").
#[cfg(feature = "demography")]
pub const PHASES: [(PhaseId, Period); 14] = [
    (PhaseId::Commands, Period::Daily),
    (PhaseId::Plan, Period::Staggered),
    (PhaseId::Staffing, Period::Staggered),
    (PhaseId::Prices, Period::Daily),
    (PhaseId::OutputAndInputs, Period::Daily),
    (PhaseId::FixedDemand, Period::Daily),
    (PhaseId::Households, Period::Staggered),
    (PhaseId::GoodsMarket, Period::Daily),
    (PhaseId::Quits, Period::MonthEnd),
    (PhaseId::Pay, Period::MonthEnd),
    (PhaseId::People, Period::MonthEnd),
    (PhaseId::WageRound, Period::Yearly { month: 1, day: 1 }),
    (PhaseId::MonthlyClose, Period::MonthEnd),
    (PhaseId::Checks, Period::Daily),
];

/// The phases due on a date, in order.
pub fn expected_phases(date: Date) -> Vec<PhaseId> {
    PHASES
        .iter()
        .filter(|(_, period)| period.due(date) != Due::No)
        .map(|&(id, _)| id)
        .collect()
}

// --- the scenario ------------------------------------------------------------

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fixtures_dir() -> PathBuf {
    repo_root().join("python/pipeline/fixtures")
}

pub fn io_table_path() -> PathBuf {
    fixtures_dir().join(IO_TABLE)
}

/// The input-output table as JSON.
pub fn io_table_json() -> &'static Value {
    static V: OnceLock<Value> = OnceLock::new();
    V.get_or_init(|| {
        let path = io_table_path();
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("fixture {}: {e}", path.display()));
        serde_json::from_str(&text).expect("valid JSON")
    })
}

/// The committed fixtures, read once.
pub fn inputs() -> &'static Inputs {
    static I: OnceLock<Inputs> = OnceLock::new();
    I.get_or_init(|| read_inputs(&fixtures_dir()).expect("the fixtures read"))
}

fn slot(scale: u32) -> usize {
    match scale {
        1000 => 0,
        100 => 1,
        _ => panic!("no cached scenario at 1:{scale}"),
    }
}

/// The Romania scenario at 1:1000 or 1:100, seed 42, built once.
pub fn scenario(scale: u32) -> &'static Scenario {
    static S: [OnceLock<Scenario>; 2] = [OnceLock::new(), OnceLock::new()];
    S[slot(scale)].get_or_init(|| build_scenario(scale))
}

/// A build at any scale, not cached (1:10 for the ignored test).
pub fn build_scenario(scale: u32) -> Scenario {
    build(
        inputs(),
        &BuildParams {
            sample_scale: scale,
            rng_seed: SEED,
        },
    )
    .expect("the fixtures build")
}

/// A folder under the tests' temporary directory, emptied: its parent exists,
/// the folder itself does not.
pub fn fresh_dir(name: &str) -> PathBuf {
    let parent = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("econ-core-world")
        .join(name);
    if parent.exists() {
        std::fs::remove_dir_all(&parent).expect("clean the test folder");
    }
    std::fs::create_dir_all(&parent).expect("create the test folder");
    parent.join("scenario")
}

/// Write a scenario into a fresh folder and read it back as a starting state.
pub fn start_state_of(scenario: &Scenario, name: &str) -> StartState {
    let dir = fresh_dir(name);
    write(scenario, inputs(), &dir).expect("the scenario writes");
    read_start_state(&dir, &io_table_path()).expect("the scenario reads as a starting state")
}

/// The folder of [`scenario`], written once.
pub fn scenario_dir(scale: u32) -> &'static Path {
    static W: [OnceLock<PathBuf>; 2] = [OnceLock::new(), OnceLock::new()];
    W[slot(scale)].get_or_init(|| {
        let dir = fresh_dir(&format!("romania_{scale}"));
        write(scenario(scale), inputs(), &dir).expect("the scenario writes");
        dir
    })
}

/// The starting state of the Romania scenario, read once.
pub fn start_state(scale: u32) -> StartState {
    static S: [OnceLock<StartState>; 2] = [OnceLock::new(), OnceLock::new()];
    S[slot(scale)]
        .get_or_init(|| {
            read_start_state(scenario_dir(scale), &io_table_path())
                .expect("the scenario reads as a starting state")
        })
        .clone()
}

// --- worlds ------------------------------------------------------------------

/// The spec defaults, seed 42. Checks the defaults the specs' tables give.
pub fn params() -> WorldParams {
    let p = WorldParams::spec_defaults(SEED);
    assert_eq!(p.rng_seed, SEED);
    assert_eq!(
        p.households.mpc,
        [1_000_000, 1_000_000, 1_000_000, 980_000, 950_000]
    );
    assert_eq!(p.households.mpc_wealth_ppm, 5_000);
    assert_eq!(p.households.subsistence_basket, Bani::from_lei(700));
    assert_eq!(p.labour.matching_efficiency_ppm, 500_000);
    assert_eq!(p.labour.matching_elasticity_ppm, 500_000);
    assert_eq!(p.labour.max_layoff_share_ppm, 100_000);
    assert_eq!(p.labour.quit_rate_ppm, 10_000);
    assert_eq!(p.labour.wage_tightness_response_ppm, 100_000);
    assert_eq!(p.production.theta_ppm, 300_000);
    assert_eq!(p.production.phi_ppm, 250_000);
    assert_eq!(p.prices.indexation_ppm, 500_000);
    assert_eq!(p.prices.eta_utilisation_ppm, 200_000);
    assert_eq!(p.prices.eta_inventory_ppm, 100_000);
    assert_eq!(p.prices.normal_utilisation_ppm, 800_000);
    assert_eq!(p.prices.max_daily_move_ppm, 20_000);
    p
}

/// A world on the Romania scenario under the placeholder law.
pub fn world(scale: u32) -> World {
    World::new(start_state(scale), Law::placeholder_2021(), params()).expect("the world builds")
}

/// The same with edits of its starting state.
pub fn world_with(scale: u32, edits: Vec<StartEdit>) -> World {
    let mut start = start_state(scale);
    for e in edits {
        start.apply(e);
    }
    World::new(start, Law::placeholder_2021(), params()).expect("the edited world builds")
}

/// The same under another law.
pub fn world_under(scale: u32, law: Law) -> World {
    World::new(start_state(scale), law, params()).expect("the world builds")
}

/// One day with no command.
pub fn step(w: &mut World) -> DayReport {
    let day = w.day();
    let r = w.step(&[]).expect("a day with no command runs");
    assert_eq!(r.day, day, "the report is of the day that ran");
    r
}

/// The calendar of a world that has not stepped yet.
pub fn calendar(w: &World) -> Calendar {
    assert_eq!(w.day(), Day(0), "a calendar from day 0");
    Calendar::new(w.date())
}

/// The month a report's day falls in.
pub fn month_of(cal: Calendar, r: &DayReport) -> Month {
    cal.month(r.day)
}

/// The statistics of a month, once closed.
pub fn month_stats(w: &World, m: u32) -> &MonthStats {
    let s = w
        .months()
        .get(m as usize)
        .unwrap_or_else(|| panic!("month {m} is not closed"));
    assert_eq!(s.month, Month(m));
    s
}

/// Step until `months` months are closed.
pub fn run_months(w: &mut World, months: usize) {
    while w.months().len() < months {
        step(w);
    }
}

// --- sums --------------------------------------------------------------------

/// Sum of per-record money times weights.
pub fn weighted(weights: &[u32], column: &[Bani]) -> Bani {
    assert_eq!(weights.len(), column.len());
    let total: i128 = weights
        .iter()
        .zip(column)
        .map(|(&w, v)| i128::from(w) * i128::from(v.get()))
        .sum();
    Bani(i64::try_from(total).expect("fits i64 bani"))
}

/// Plain sum of money.
pub fn total(column: &[Bani]) -> Bani {
    column.iter().copied().fold(Bani::ZERO, |a, b| a + b)
}

/// The weight of each person: its household's.
pub fn person_weights(w: &World) -> Vec<u32> {
    let hh = &w.households().hh_weight;
    w.persons()
        .household_id
        .iter()
        .map(|&h| hh[h as usize])
        .collect()
}

/// Each household's cash plus deposits.
pub fn money(w: &World) -> Vec<Bani> {
    let h = w.households();
    h.hh_cash
        .iter()
        .zip(&h.hh_deposits)
        .map(|(&c, &d)| c + d)
        .collect()
}

/// The day's flows of the ledger for a sector, every flow code together.
pub fn sector_flows(w: &World, sector: Sector) -> Bani {
    w.ledger()
        .flows()
        .by_code_sector
        .iter()
        .filter(|((code, s), _)| *s == sector && code.is_flow())
        .map(|(_, &v)| v)
        .fold(Bani::ZERO, |a, b| a + b)
}

/// Units of an industry.
pub fn units_of(w: &World, industry: usize) -> Vec<usize> {
    w.firm_units()
        .industry
        .iter()
        .enumerate()
        .filter(|&(_, &j)| usize::from(j) == industry)
        .map(|(u, _)| u)
        .collect()
}

/// The catalogue index of an industry.
pub fn industry(w: &World, id: &str) -> usize {
    w.industries()
        .id
        .iter()
        .position(|x| x == id)
        .unwrap_or_else(|| panic!("no industry {id}"))
}

/// Output of an industry on the latest day.
pub fn industry_output_today(w: &World, industry: usize) -> i64 {
    units_of(w, industry)
        .iter()
        .map(|&u| w.firm_units().output_today[u])
        .sum()
}

/// `a / b` in millionths.
pub fn ppm(a: i128, b: i128) -> i64 {
    assert!(b > 0, "a ratio over zero");
    i64::try_from(a * i128::from(ONE) / b).expect("a ratio that fits")
}

// --- the checks of a day -----------------------------------------------------

/// Phase 13 and the tests' own reading of it: the report's checks; the
/// ledger's invariants; households' and firm units' columns equal to the
/// ledger's sector balances (I-4); no negative cash, deposit or inventory.
pub fn assert_day_checks(w: &World, r: &DayReport) {
    assert!(
        r.checks_ok && r.failed.is_empty(),
        "day {}: checks failed: {:?}",
        r.day.0,
        r.failed
    );
    if let Err(e) = w.ledger().check_invariants() {
        panic!("day {}: ledger invariants: {e:?}", r.day.0);
    }
    let h = w.households();
    let l = w.ledger();
    assert_eq!(
        weighted(&h.hh_weight, &h.hh_cash),
        l.balance(Sector::Households, Instrument::Cash),
        "day {}: households' cash (I-4)",
        r.day.0
    );
    assert_eq!(
        weighted(&h.hh_weight, &h.hh_deposits),
        l.balance(Sector::Households, Instrument::Deposits),
        "day {}: households' deposits (I-4)",
        r.day.0
    );
    let f = w.firm_units();
    assert_eq!(
        total(&f.currency),
        l.balance(Sector::Firms, Instrument::Cash),
        "day {}: firm units' currency (I-4)",
        r.day.0
    );
    assert_eq!(
        total(&f.deposits),
        l.balance(Sector::Firms, Instrument::Deposits),
        "day {}: firm units' deposits (I-4)",
        r.day.0
    );
    for (name, col) in [
        ("hh_cash", &h.hh_cash),
        ("hh_deposits", &h.hh_deposits),
        ("unit currency", &f.currency),
        ("unit deposits", &f.deposits),
    ] {
        if let Some(i) = col.iter().position(|v| v.get() < 0) {
            panic!("day {}: {name} of record {i} is {:?}", r.day.0, col[i]);
        }
    }
    if let Some(u) = f.inventory.iter().position(|&v| v < 0) {
        panic!(
            "day {}: inventory of unit {u} is {}",
            r.day.0, f.inventory[u]
        );
    }
}

/// `n` days, every check holding on every day.
pub fn run_checked(w: &mut World, days: u32) {
    for _ in 0..days {
        let r = step(w);
        assert_day_checks(w, &r);
    }
}

/// Staggered records ran today exactly when today is their day: households
/// decided, firm units planned and staffed.
pub fn assert_staggered_today(w: &World, r: &DayReport) {
    let today = Some(r.day);
    let due = |stagger: u8| r.date.day <= 28 && stagger == r.date.day;
    let h = w.households();
    for (i, (&s, &last)) in h.hh_stagger_day.iter().zip(&h.last_decision).enumerate() {
        assert!((1..=28).contains(&s), "household {i}: stagger day {s}");
        assert_eq!(
            last == today,
            due(s),
            "household {i} (day {s}) on {:?}: decided {last:?}",
            r.date
        );
    }
    let f = w.firm_units();
    for (u, &s) in f.unit_stagger_day.iter().enumerate() {
        assert!((1..=28).contains(&s), "unit {u}: stagger day {s}");
        assert_eq!(
            f.last_plan[u] == today,
            due(s),
            "unit {u} (day {s}) on {:?}: planned {:?}",
            r.date,
            f.last_plan[u]
        );
        assert_eq!(
            f.last_staffing[u] == today,
            due(s),
            "unit {u} (day {s}) on {:?}: staffed {:?}",
            r.date,
            f.last_staffing[u]
        );
    }
}

/// At a month end: every household and unit ran in this month.
pub fn assert_everyone_ran_this_month(w: &World, cal: Calendar, r: &DayReport) {
    let this = cal.month(r.day);
    let ran = |d: Option<Day>| d.is_some_and(|d| cal.month(d) == this);
    let h = w.households();
    if let Some(i) = h.last_decision.iter().position(|&d| !ran(d)) {
        panic!(
            "household {i} did not decide in month {}: {:?}",
            this.0, h.last_decision[i]
        );
    }
    let f = w.firm_units();
    if let Some(u) = (0..f.last_plan.len()).find(|&u| !ran(f.last_plan[u])) {
        panic!("unit {u} did not plan in month {}", this.0);
    }
    if let Some(u) = (0..f.last_staffing.len()).find(|&u| !ran(f.last_staffing[u])) {
        panic!("unit {u} did not staff in month {}", this.0);
    }
}

/// Persons employed by each unit, weighted.
pub fn staff_from_persons(w: &World) -> Vec<u64> {
    let weights = person_weights(w);
    let mut staff = vec![0u64; w.firm_units().industry.len()];
    for (p, &e) in w.persons().employer.iter().enumerate() {
        if e != NO_EMPLOYER {
            staff[usize::from(e)] += u64::from(weights[p]);
        }
    }
    staff
}
