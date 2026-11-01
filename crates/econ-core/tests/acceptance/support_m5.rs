//! Shared by the M5 acceptance tests of migration, education, approval and
//! poverty: worlds on the Romania scenario under the law of its folder (the
//! 2021 law) with chosen parameters and start edits, stepping to month ends,
//! and the person counts the criteria read. Builds on `super::support` and
//! `super::support_lev`.

// Each test file uses a part of this one.
#![allow(dead_code)]

use econ_core::levers::{LeverId, LeverValue};
use econ_core::world::{DayReport, StartEdit, World, WorldParams};
use econ_types::{Bani, Date};

use super::support::{assert_day_checks, inputs, params, start_state, step};
use super::support_lev::{date, give, key_of, scenario_law, set};

/// A world on the Romania scenario at 1:1000 or 1:100 under the law of its
/// folder (`law.json`, `world.json`: the 2021 law), with these parameters and
/// start edits.
pub fn world_2021(scale: u32, p: WorldParams, edits: Vec<StartEdit>) -> World {
    let law = scenario_law(scale);
    let mut start = start_state(scale);
    for e in edits {
        start.apply(e);
    }
    World::with_law(start, law.registry.clone(), law.law.clone(), p)
        .expect("the world builds under the scenario's law")
}

/// The same at the spec defaults, seed 42, with no edit.
pub fn world_2021_default(scale: u32) -> World {
    world_2021(scale, params(), Vec::new())
}

/// The same with another seed of the keyed draws.
pub fn world_2021_seed(scale: u32, seed: u64) -> World {
    let mut p = params();
    p.rng_seed = seed;
    world_2021(scale, p, Vec::new())
}

/// Step one day, checking the day.
pub fn step_checked(w: &mut World) -> DayReport {
    let r = step(w);
    assert_day_checks(w, &r);
    r
}

/// Step until `months` months are closed, checking every day; the last day
/// stepped is the close of month `months − 1`.
pub fn run_closes(w: &mut World, months: usize) {
    while w.months().len() < months {
        step_checked(w);
    }
}

/// Step until one more month is closed, checking every day.
pub fn next_close(w: &mut World) {
    let n = w.months().len();
    run_closes(w, n + 1);
}

/// Step through the day before `day`, checking every day.
pub fn step_until(w: &mut World, day: Date) {
    while w.date() < day {
        step_checked(w);
    }
}

/// The weight of each person: its household's (0 for a freed record).
pub fn weights_of_persons(w: &World) -> Vec<u32> {
    let hh = &w.households().hh_weight;
    w.persons()
        .household_id
        .iter()
        .map(|&h| hh[h as usize])
        .collect()
}

/// Whether each person is a resident: in a record of positive weight and,
/// with migration, not abroad.
pub fn residents_mask(w: &World) -> Vec<bool> {
    let weights = weights_of_persons(w);
    #[cfg(feature = "migration")]
    let abroad = w.abroad().to_vec();
    #[cfg(not(feature = "migration"))]
    let abroad = vec![false; weights.len()];
    weights
        .iter()
        .zip(&abroad)
        .map(|(&wt, &a)| wt > 0 && !a)
        .collect()
}

/// Weighted resident persons.
pub fn residents(w: &World) -> u64 {
    let weights = weights_of_persons(w);
    residents_mask(w)
        .iter()
        .zip(&weights)
        .filter(|(r, _)| **r)
        .map(|(_, &wt)| u64::from(wt))
        .sum()
}

/// Age of 18, in months.
pub const ADULT_MONTHS: u16 = 18 * 12;

/// Whether each resident person is an adult (18 or more).
pub fn adults_mask(w: &World) -> Vec<bool> {
    let ages = w.person_age_months();
    residents_mask(w)
        .iter()
        .zip(ages)
        .map(|(&r, &a)| r && a >= ADULT_MONTHS)
        .collect()
}

/// Each household's resident members.
pub fn members(w: &World) -> Vec<Vec<usize>> {
    let mut m = vec![Vec::new(); w.households().hh_weight.len()];
    let mask = residents_mask(w);
    for (p, &h) in w.persons().household_id.iter().enumerate() {
        if mask[p] {
            m[h as usize].push(p);
        }
    }
    m
}

/// Whether each household record is collective (an institution): the
/// scenario's flag for the records of the start; a record added since is
/// private (household formation, arrivals) unless the life cycle made it
/// collective (an orphan's one-member record, `society/households-lifecycle`,
/// "Make collective"), which the People phase's `hh_collective` column
/// (`World::household_life`) records.
///
/// A record is followed by its `hh_key` (`World::people`), which at load is
/// the start's index; records added later get keys after the start's. The
/// compaction (yearly with the People phase, monthly with migration)
/// removes freed records, so rows no longer line up with the start's
/// (restated 2026-11-01; the helper read the start's flag by row before,
/// and took every added record as private, which missed "Make collective").
pub fn collective(w: &World, scale: u32) -> Vec<bool> {
    let start = start_state(scale).hh_collective;
    let keys = &w.people().hh_key;
    let n = w.households().hh_weight.len();
    assert_eq!(keys.len(), n, "one `hh_key` per household record");
    #[cfg(feature = "demography")]
    let made = w.household_life().hh_collective.clone();
    #[cfg(not(feature = "demography"))]
    let made = vec![false; n];
    keys.iter()
        .enumerate()
        .map(
            |(h, &k)| match usize::try_from(k).ok().and_then(|i| start.get(i)) {
                Some(&c) => c,
                None => made.get(h).copied().unwrap_or(false),
            },
        )
        .collect()
}

/// The national weighted sum of a household money column.
pub fn weighted_sum(w: &World, column: &[Bani]) -> i128 {
    w.households()
        .hh_weight
        .iter()
        .zip(column)
        .map(|(&wt, v)| i128::from(wt) * i128::from(v.get()))
        .sum()
}

/// `p · num / den`, rounded half away from zero.
pub fn round_div(p: i128, d: i128) -> i128 {
    assert!(d > 0, "a division by {d}");
    let q = p / d;
    let r = p % d;
    if r.abs() * 2 >= d { q + p.signum() } else { q }
}

/// The index of a county by its NUTS 3 code, in the scenario's order.
pub fn county(code: &str) -> usize {
    let doc = &inputs().docs["census2021_margins_ro.json"];
    doc["counties"]
        .as_array()
        .expect("the margins list their counties")
        .iter()
        .position(|c| c.as_str() == Some(code))
        .unwrap_or_else(|| panic!("no county {code}"))
}

/// The development region of a county: that of its households at the start.
pub fn region_of_county(scale: u32, c: usize) -> u8 {
    let s = start_state(scale);
    let h = s
        .hh_county
        .iter()
        .position(|&x| usize::from(x) == c)
        .unwrap_or_else(|| panic!("no household in county {c}"));
    s.hh_region[h]
}

/// The money value in force of a lever, for a key.
pub fn money_in_force(w: &World, lever: LeverId, key: Option<u32>) -> Bani {
    match w.law().get(lever, key) {
        Some(LeverValue::Money(b)) => *b,
        other => panic!("{} {key:?}: money in force, got {other:?}", lever.as_str()),
    }
}

/// The rate value in force of a lever, basis points.
pub fn rate_in_force(w: &World, lever: LeverId, key: Option<u32>) -> i32 {
    match w.law().get(lever, key) {
        Some(LeverValue::Rate(r)) => *r,
        other => panic!("{} {key:?}: a rate in force, got {other:?}", lever.as_str()),
    }
}

/// Every key of a keyed money lever set to `value(key, value in force)`,
/// given today.
pub fn set_money_lever(w: &mut World, lever: LeverId, value: impl Fn(&str, Bani) -> Bani) {
    let keys: Vec<String> = w.registry().get(lever).keys.clone();
    assert!(!keys.is_empty(), "{} is keyed", lever.as_str());
    for k in keys {
        let key = key_of(w, lever, &k);
        let now = money_in_force(w, lever, Some(key));
        give(w, set(lever, Some(key), LeverValue::Money(value(&k, now))));
    }
}

/// 15 February 2022: the day the M4 suite gives its orders (a law lever then
/// takes effect on 1 March 2022, month 3).
pub fn feb_15_2022() -> Date {
    date(2022, 2, 15)
}
