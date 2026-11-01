//! Shared by the M6 acceptance tests of group queries, explanations and the
//! core API (specs game/group-queries, game/explanations, game/core-api):
//! worlds on the Romania scenario under the law of its folder (the 2021
//! law), stepping to closes with the day's checks, the registry looked up by
//! dotted name, the features compiled, and the resident persons. Builds on
//! `super::support` and `super::support_lev`.

// Each test file uses a part of this one.
#![allow(dead_code)]

use econ_core::stats::{IndicatorId, IndicatorMeta, Key, Status, indicators};
use econ_core::world::{DayReport, World, WorldParams};

use super::support::{assert_day_checks, params, start_state, step};
use super::support_lev::scenario_law;

/// The national value's key.
pub const NATIONAL: Key = Key {
    breakdown: None,
    index: 0,
};

/// The registry's entry named `name`; panics when there is none.
pub fn meta(name: &str) -> &'static IndicatorMeta {
    indicators()
        .iter()
        .find(|m| m.name == name)
        .unwrap_or_else(|| panic!("the registry has no indicator {name}"))
}

/// The id of the indicator named `name`.
pub fn id(name: &str) -> IndicatorId {
    meta(name).id
}

/// The registry's dotted name of an id.
pub fn name_of(id: IndicatorId) -> &'static str {
    indicators()
        .iter()
        .find(|m| m.id == id)
        .map(|m| m.name)
        .unwrap_or_else(|| panic!("{id:?}: not in the registry"))
}

/// Whether econ-core's feature `name` is compiled in this build. Panics on a
/// name this list does not know (add it with its `cfg!`).
pub fn feature_on(name: &str) -> bool {
    match name {
        "default" => true,
        "world" => cfg!(feature = "world"),
        "pay_day" => cfg!(feature = "pay_day"),
        "banking" => cfg!(feature = "banking"),
        "fiscal" => cfg!(feature = "fiscal"),
        "monetary" => cfg!(feature = "monetary"),
        "trade" => cfg!(feature = "trade"),
        "investment" => cfg!(feature = "investment"),
        "revenue" => cfg!(feature = "revenue"),
        "demography" => cfg!(feature = "demography"),
        "lifecycle" => cfg!(feature = "lifecycle"),
        "levers" => cfg!(feature = "levers"),
        "policy" => cfg!(feature = "policy"),
        "lever_policy_rate" => cfg!(feature = "lever_policy_rate"),
        "lever_monetary_financing" => cfg!(feature = "lever_monetary_financing"),
        "lever_fx_devaluation" => cfg!(feature = "lever_fx_devaluation"),
        "lever_fuel_price" => cfg!(feature = "lever_fuel_price"),
        "migration" => cfg!(feature = "migration"),
        "education" => cfg!(feature = "education"),
        "approval" => cfg!(feature = "approval"),
        "poverty" => cfg!(feature = "poverty"),
        "wellbeing" => cfg!(feature = "wellbeing"),
        "home_quality" => cfg!(feature = "home_quality"),
        "stats" => cfg!(feature = "stats"),
        "groups" => cfg!(feature = "groups"),
        "explain" => cfg!(feature = "explain"),
        "api" => cfg!(feature = "api"),
        other => panic!("feature {other}: not in the test's list of econ-core's features"),
    }
}

/// Whether an indicator is active and its feature compiled.
pub fn computed(m: &IndicatorMeta) -> bool {
    m.status == Status::Active && m.feature.is_none_or(feature_on)
}

/// A world on the Romania scenario at 1:1000 or 1:100 under the law of its
/// folder (`law.json`, `world.json`: the 2021 law).
pub fn world_2021(scale: u32, p: WorldParams) -> World {
    let law = scenario_law(scale);
    World::with_law(start_state(scale), law.registry.clone(), law.law.clone(), p)
        .expect("the world builds under the scenario's law")
}

/// The same at the spec defaults, seed 42.
pub fn world_2021_default(scale: u32) -> World {
    world_2021(scale, params())
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

/// The weight of each person: its household's (0 for a freed record).
pub fn person_weights(w: &World) -> Vec<u32> {
    let hh = &w.households().hh_weight;
    w.persons()
        .household_id
        .iter()
        .map(|&h| hh[h as usize])
        .collect()
}

/// Whether each person is a resident: alive, in a record of positive weight
/// and not abroad (spec game/group-queries, "The filter").
pub fn residents_mask(w: &World) -> Vec<bool> {
    let weights = person_weights(w);
    #[cfg(feature = "migration")]
    let abroad = w.abroad().to_vec();
    #[cfg(not(feature = "migration"))]
    let abroad = vec![false; weights.len()];
    #[cfg(feature = "demography")]
    let alive = w.people_life().alive.clone();
    #[cfg(not(feature = "demography"))]
    let alive: Vec<bool> = Vec::new();
    weights
        .iter()
        .enumerate()
        .map(|(p, &wt)| {
            wt > 0
                && !abroad.get(p).copied().unwrap_or(false)
                && alive.get(p).copied().unwrap_or(true)
        })
        .collect()
}
