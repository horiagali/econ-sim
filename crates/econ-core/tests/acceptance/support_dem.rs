//! Shared by the `[sim]` acceptance tests of demographics and the household
//! life cycle (specs economy/demographics, society/households-lifecycle;
//! features `demography`, `lifecycle`): worlds on the Romania scenario under
//! the law of its folder (the 2021 law) with the People phase's parameters,
//! stepping to month ends with every check of the day, the persons and
//! households that count (live records of live households), the life table,
//! and the exit run of AC-DEM-12, 13, 05 and AC-HHL-09, run once per scale.
//! Builds on `super::support` and `super::support_lev`.

// Each test file uses a part of this one.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::OnceLock;

use econ_core::world::{DayReport, PeopleReport, StartEdit, World, WorldParams};
use econ_mech_demography::MortalityTable;
use econ_popgen::{Activity, Role, Sex};
use econ_types::{Bani, Date, Month};

use super::support::{assert_day_checks, money, params, start_state, step};
use super::support_lev::{date, scenario_law};

/// Months of age.
pub const YEAR: u16 = 12;
/// 18 years, in months.
pub const ADULT: u16 = 18 * 12;

/// The spec defaults, seed 42, with the People phase's defaults checked
/// against the specs' tables.
pub fn dem_params() -> WorldParams {
    let p = params();
    let d = p.demography;
    assert_eq!(d.mortality_improvement_ppm, 15_000);
    assert_eq!(d.partner_fertility_ratio_ppm, 3_000_000);
    assert_eq!(d.sex_ratio_at_birth_ppm, 514_000);
    assert_eq!(d.fertility_scale_ppm, 1_000_000);
    assert_eq!(d.mortality_scale_ppm, 1_000_000);
    assert!(!d.fixed_fertility_multipliers && !d.compact_monthly && d.people_phase);
    let f = d.fertility;
    assert_eq!(
        (f.benefit_ppm, f.unemployed_ratio_ppm, f.cycle_ppm),
        (100_000, 850_000, 200_000)
    );
    assert_eq!(
        (f.normal_growth_ppm, f.lag_months, f.income_floor_ppm),
        (20_000, 9, 250_000)
    );
    let l = d.lifecycle;
    assert_eq!(
        l.leave_home_rate_ppm,
        [
            [30_000, 120_000, 150_000, 120_000],
            [20_000, 50_000, 100_000, 100_000]
        ]
    );
    assert_eq!(l.leave_home_job_ratio_ppm, 2_000_000);
    assert_eq!(l.leave_home_share_ppm, 250_000);
    assert_eq!(
        l.union_rate_ppm,
        [150_000, 200_000, 150_000, 100_000, 50_000]
    );
    assert_eq!(l.union_age_gap_months, 24);
    assert_eq!(l.separation_rate_ppm, 9_000);
    assert_eq!(l.separation_share_ppm, 500_000);
    p
}

/// The economic terms of fertility switched off (`unemployed_fertility_ratio`
/// 1, `fertility_cycle_response` 0): the benefit term stays.
pub fn without_economic_terms(mut p: WorldParams) -> WorldParams {
    p.demography.fertility.unemployed_ratio_ppm = 1_000_000;
    p.demography.fertility.cycle_ppm = 0;
    p
}

/// A world on the Romania scenario at 1:1000 or 1:100 under the law of its
/// folder (`law.json`, `world.json`: the 2021 law), with these parameters and
/// start edits.
pub fn world_dem(scale: u32, p: WorldParams, edits: Vec<StartEdit>) -> World {
    let law = scenario_law(scale);
    let mut start = start_state(scale);
    for e in edits {
        start.apply(e);
    }
    World::with_law(start, law.registry.clone(), law.law.clone(), p)
        .expect("the world builds under the scenario's law")
}

/// The same at the defaults, with no edit.
pub fn world_dem_default(scale: u32) -> World {
    world_dem(scale, dem_params(), Vec::new())
}

/// One day, every check of the day holding.
pub fn step_checked(w: &mut World) -> DayReport {
    let r = step(w);
    assert_day_checks(w, &r);
    r
}

/// Step, checking every day, through the next month end; the report of its
/// day.
pub fn to_close(w: &mut World) -> DayReport {
    loop {
        let r = step_checked(w);
        if r.date.is_month_end() {
            assert_eq!(
                w.months().last().map(|m| m.month),
                Some(Month(u32::try_from(w.months().len() - 1).expect("fits"))),
                "{:?}: the month closed",
                r.date
            );
            return r;
        }
    }
}

/// The index of the month that closed last.
pub fn last_month(w: &World) -> Month {
    Month(u32::try_from(w.months().len()).expect("fits") - 1)
}

/// The People phase's report, which must be of the month that closed last.
pub fn report(w: &World) -> &PeopleReport {
    let r = w.people_report();
    assert_eq!(
        r.month,
        Some(last_month(w)),
        "the People phase ran this month"
    );
    r
}

/// Step, checking every day, until the next day to run is `day`.
pub fn run_until(w: &mut World, day: Date) {
    assert!(w.date() <= day, "the world is past {day:?}");
    while w.date() < day {
        step_checked(w);
    }
}

// --- who counts --------------------------------------------------------------

/// Each person's weight if the person and the household are live, else 0.
pub fn live_weights(w: &World) -> Vec<u32> {
    let h = w.households();
    let life = w.household_life();
    let people = w.people();
    w.persons()
        .household_id
        .iter()
        .enumerate()
        .map(|(p, &hh)| {
            let hh = hh as usize;
            if people.alive[p] && life.hh_alive[hh] {
                h.hh_weight[hh]
            } else {
                0
            }
        })
        .collect()
}

/// The weighted live population.
pub fn population(w: &World) -> u64 {
    live_weights(w).iter().map(|&x| u64::from(x)).sum()
}

/// The weighted live population by sex (F 0, M 1) and age in whole years
/// (0 to 110, the last including older).
pub fn by_sex_age(w: &World) -> [Vec<u64>; 2] {
    let mut out = [vec![0u64; 111], vec![0u64; 111]];
    let people = w.people();
    for (p, &wt) in live_weights(w).iter().enumerate() {
        if wt > 0 {
            let a = usize::from(people.age[p] / YEAR).min(110);
            out[people.sex[p] as usize][a] += u64::from(wt);
        }
    }
    out
}

/// Weighted live households, and private ones.
pub fn real_households(w: &World) -> (u64, u64) {
    let h = w.households();
    let life = w.household_life();
    let mut all = 0;
    let mut private = 0;
    for i in 0..h.hh_weight.len() {
        if life.hh_alive[i] {
            all += u64::from(h.hh_weight[i]);
            if !life.hh_collective[i] {
                private += u64::from(h.hh_weight[i]);
            }
        }
    }
    (all, private)
}

/// Person key → index.
pub fn person_index(w: &World) -> BTreeMap<u64, usize> {
    w.people()
        .person_key
        .iter()
        .enumerate()
        .map(|(i, &k)| (k, i))
        .collect()
}

/// Household key → index.
pub fn household_index(w: &World) -> BTreeMap<u64, usize> {
    w.household_life()
        .hh_key
        .iter()
        .enumerate()
        .map(|(i, &k)| (k, i))
        .collect()
}

/// Each household's live members.
pub fn members(w: &World) -> Vec<Vec<usize>> {
    let mut m = vec![Vec::new(); w.households().hh_weight.len()];
    let people = w.people();
    for (p, &hh) in w.persons().household_id.iter().enumerate() {
        if people.alive[p] {
            m[hh as usize].push(p);
        }
    }
    m
}

/// Whether each live person lives in a couple: head or partner of a live
/// private household whose live head and live partner are of the two sexes.
pub fn in_couple(w: &World) -> Vec<bool> {
    let people = w.people();
    let life = w.household_life();
    let mut out = vec![false; people.alive.len()];
    for (hh, list) in members(w).iter().enumerate() {
        if !life.hh_alive[hh] || life.hh_collective[hh] {
            continue;
        }
        let head = list.iter().find(|&&p| people.role[p] == Role::Head);
        let partner = list.iter().find(|&&p| people.role[p] == Role::Partner);
        if let (Some(&h), Some(&q)) = (head, partner)
            && people.sex[h] != people.sex[q]
        {
            out[h] = true;
            out[q] = true;
        }
    }
    out
}

/// Whether an activity is in the labour force.
pub fn in_labour_force(a: Activity) -> bool {
    matches!(a, Activity::Employed | Activity::Unemployed)
}

/// `a / b` as a float.
pub fn ratio(a: u64, b: u64) -> f64 {
    assert!(b > 0, "a ratio over zero");
    #[allow(clippy::cast_precision_loss)]
    let r = a as f64 / b as f64;
    r
}

// --- AC-HH-03's identity ------------------------------------------------------

/// Per household key: money, income to date, spending to date, moved to date.
pub fn budgets(w: &World) -> BTreeMap<u64, (Bani, Bani, Bani, Bani)> {
    let h = w.households();
    let life = w.household_life();
    let m = money(w);
    (0..h.hh_weight.len())
        .filter(|&i| life.hh_alive[i])
        .map(|i| {
            (
                life.hh_key[i],
                (
                    m[i],
                    h.income_to_date[i],
                    h.spending_to_date[i],
                    life.hh_moved_to_date[i],
                ),
            )
        })
        .collect()
}

/// AC-HH-03's identity with `hh_moved_to_date`, between two snapshots: for
/// every household live in the second, the change in cash and deposits is
/// income − spending + moved (a new household starting from zero).
pub fn assert_moved_identity(
    before: &BTreeMap<u64, (Bani, Bani, Bani, Bani)>,
    after: &BTreeMap<u64, (Bani, Bani, Bani, Bani)>,
    when: &str,
) {
    let zero = (Bani::ZERO, Bani::ZERO, Bani::ZERO, Bani::ZERO);
    for (k, a) in after {
        let b = before.get(k).unwrap_or(&zero);
        assert_eq!(
            a.0 - b.0,
            (a.1 - b.1) - (a.2 - b.2) + (a.3 - b.3),
            "{when}: household {k}: change in money = income − spending + moved"
        );
    }
}

// --- the life table ----------------------------------------------------------

/// Life expectancy at birth of yearly death probabilities by single year of
/// age (the last one closing the table): deaths in mid-year.
pub fn e0_of_q(q: &[f64]) -> f64 {
    let mut alive = 1.0;
    let mut years = 0.0;
    for (a, &qa) in q.iter().enumerate() {
        let qa = if a + 1 == q.len() {
            1.0
        } else {
            qa.clamp(0.0, 1.0)
        };
        years += alive * (1.0 - qa / 2.0);
        alive *= 1.0 - qa;
    }
    years
}

/// Life expectancy at birth of a mortality table (monthly probabilities in
/// ppb), for one sex.
pub fn e0_of_table(t: &MortalityTable, sex: Sex) -> f64 {
    let q: Vec<f64> = t.monthly[sex as usize]
        .iter()
        .map(|&m| {
            let m = f64::from(m) / 1e9;
            1.0 - econ_num::math::powi(1.0 - m, 12)
        })
        .collect();
    e0_of_q(&q)
}

/// A year of a run: deaths and mean population by sex and single year of
/// age, weighted.
#[derive(Debug, Clone, Default)]
pub struct YearRates {
    /// `[sex][age]`, the year's deaths.
    pub deaths: [Vec<u64>; 2],
    /// `[sex][age]`, the sum over the year's month ends of the population.
    pub exposure_months: [Vec<u64>; 2],
    /// Month ends counted.
    pub months: u32,
}

impl YearRates {
    /// An empty year.
    pub fn new() -> YearRates {
        YearRates {
            deaths: [vec![0; 111], vec![0; 111]],
            exposure_months: [vec![0; 111], vec![0; 111]],
            months: 0,
        }
    }

    /// Add a closed month: its deaths and its population at the close.
    pub fn add_month(&mut self, w: &World) {
        for d in &report(w).deaths {
            let a = usize::from(d.age / YEAR).min(110);
            self.deaths[d.sex as usize][a] += u64::from(d.weight);
        }
        let pop = by_sex_age(w);
        for (row, add) in self.exposure_months.iter_mut().zip(&pop) {
            for (x, &y) in row.iter_mut().zip(add) {
                *x += y;
            }
        }
        self.months += 1;
    }

    /// Life expectancy at birth from the year's death rates: central rates
    /// over the mean population, turned into probabilities; an age nobody
    /// lived closes the table.
    pub fn e0(&self, sex: Sex) -> f64 {
        let s = sex as usize;
        let mut q = Vec::new();
        for a in 0..111 {
            let e = self.exposure_months[s][a];
            if e == 0 {
                q.push(1.0);
                break;
            }
            #[allow(clippy::cast_precision_loss)]
            let m = self.deaths[s][a] as f64 / (e as f64 / f64::from(self.months));
            q.push(m / (1.0 + m / 2.0));
        }
        e0_of_q(&q)
    }
}

// --- the exit run ------------------------------------------------------------

/// The population's shape on a day.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// The next day to run.
    pub date: Date,
    /// Weighted live persons.
    pub population: u64,
    /// Under 15.
    pub under15: u64,
    /// 15 to 64.
    pub aged15_64: u64,
    /// 65 or more.
    pub over65: u64,
    /// The weighted median age, months.
    pub median_age: u16,
    /// Person records in the tables (tombstones included).
    pub person_records: usize,
    /// Household records in the tables (tombstones included).
    pub household_records: usize,
    /// Weighted live households.
    pub real_households: u64,
    /// Weighted live persons with pension points.
    pub pensioners: u64,
    /// Σ weight × pension points of the live persons.
    pub pension_points: u128,
}

/// The shape of the population now.
pub fn snapshot(w: &World) -> Snapshot {
    let weights = live_weights(w);
    let people = w.people();
    let (mut under15, mut aged15_64, mut over65) = (0u64, 0u64, 0u64);
    let mut ages: Vec<(u16, u64)> = Vec::new();
    let (mut pensioners, mut points) = (0u64, 0u128);
    for (p, &wt) in weights.iter().enumerate() {
        if wt == 0 {
            continue;
        }
        let wt = u64::from(wt);
        let a = people.age[p];
        match a / YEAR {
            0..15 => under15 += wt,
            15..65 => aged15_64 += wt,
            _ => over65 += wt,
        }
        ages.push((a, wt));
        if people.pension_points[p] > 0 {
            pensioners += wt;
            points += u128::from(wt) * u128::from(people.pension_points[p]);
        }
    }
    ages.sort_unstable();
    let total: u64 = ages.iter().map(|&(_, x)| x).sum();
    let mut cum = 0;
    let mut median_age = 0;
    for &(a, x) in &ages {
        cum += x;
        if 2 * cum >= total {
            median_age = a;
            break;
        }
    }
    Snapshot {
        date: w.date(),
        population: total,
        under15,
        aged15_64,
        over65,
        median_age,
        person_records: w.persons().household_id.len(),
        household_records: w.households().hh_weight.len(),
        real_households: real_households(w).0,
        pensioners,
        pension_points: points,
    }
}

/// The exit run of economy/demographics: 50 years (600 month ends) of the
/// Romania scenario under the 2021 law, every M3 and M4 feature on,
/// migration off, every check of the day holding on every day.
#[derive(Debug, Clone)]
pub struct ExitRun {
    /// The sample scale.
    pub scale: u32,
    /// Day 0 (1 December 2021).
    pub start: Snapshot,
    /// After each December close, 2021 to 2070 (the next day is 1 January).
    pub year_end: Vec<Snapshot>,
    /// Each run year (12 month ends from December 2021), 50 of them.
    pub years: Vec<YearRates>,
    /// The mortality table in force in 2022.
    pub table_2022: MortalityTable,
}

impl ExitRun {
    /// The snapshot after the December close of `year`.
    pub fn end_of(&self, year: i32) -> &Snapshot {
        self.year_end
            .iter()
            .find(|s| s.date == date(year + 1, 1, 1))
            .unwrap_or_else(|| panic!("no snapshot at the end of {year}"))
    }
}

fn run_exit(scale: u32) -> ExitRun {
    let mut w = world_dem_default(scale);
    let start = snapshot(&w);
    let mut year_end = Vec::new();
    let mut years = Vec::new();
    let mut year = YearRates::new();
    let mut table_2022 = None;
    for m in 0..600u32 {
        let r = to_close(&mut w);
        year.add_month(&w);
        if m % 12 == 11 {
            years.push(std::mem::replace(&mut year, YearRates::new()));
        }
        if r.date.year == 2022 && r.date.month == 6 {
            table_2022 = Some(w.mortality_table().clone());
        }
        if r.date.month == 12 {
            year_end.push(snapshot(&w));
        }
    }
    ExitRun {
        scale,
        start,
        year_end,
        years,
        table_2022: table_2022.expect("June 2022 closed"),
    }
}

/// The exit run at 1:1000 or 1:100, run once.
pub fn exit_run(scale: u32) -> &'static ExitRun {
    static R: [OnceLock<ExitRun>; 2] = [OnceLock::new(), OnceLock::new()];
    let slot = match scale {
        1000 => 0,
        100 => 1,
        _ => panic!("no exit run at 1:{scale}"),
    };
    R[slot].get_or_init(|| run_exit(scale))
}
