//! Shared by the acceptance tests of the lever system (spec
//! game/lever-system) and of the labour market's pay and job choice
//! (AC-LM-11, AC-LM-12): the committed registry, the Romania scenario's law
//! (`law.json` and `world.json`), worlds under it, orders given on their
//! days, replay from a log, and the public firm units. Builds on
//! `super::support`.

// Each test file uses a part of this one.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command as Process;
use std::sync::OnceLock;

use econ_core::levers::{
    Accepted, BudgetLineMonth, Command, LeverId, LeverMeta, LeverValue, LogLine, Order, Registry,
    Status,
};
use econ_core::world::{DayReport, World};
use econ_io::commands::{ScenarioLaw, read_scenario_law};
use econ_types::{Date, Month};

use super::support::{assert_day_checks, params, repo_root, scenario_dir, start_state, step};

/// The registry file.
pub fn registry_path() -> PathBuf {
    repo_root().join("crates/econ-core/levers.toml")
}

/// The text of the committed registry.
pub fn registry_text() -> String {
    let path = registry_path();
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The committed registry, parsed once.
pub fn registry() -> &'static Registry {
    static R: OnceLock<Registry> = OnceLock::new();
    R.get_or_init(|| Registry::load(&registry_path()).expect("the committed registry loads"))
}

/// A date that exists.
pub fn date(year: i32, month: u8, day: u8) -> Date {
    Date::new(year, month, day).unwrap_or_else(|| panic!("no date {year}-{month}-{day}"))
}

/// The start date of the Romania scenario.
pub fn start_date() -> Date {
    date(2021, 12, 1)
}

/// The law of the Romania scenario's folder at 1:1000 or 1:100, read once.
pub fn scenario_law(scale: u32) -> &'static ScenarioLaw {
    static S: [OnceLock<ScenarioLaw>; 2] = [OnceLock::new(), OnceLock::new()];
    let slot = match scale {
        1000 => 0,
        100 => 1,
        _ => panic!("no cached scenario at 1:{scale}"),
    };
    S[slot].get_or_init(
        || match read_scenario_law(scenario_dir(scale), registry()) {
            Ok(law) => law,
            Err((line, r)) => panic!(
                "the scenario's law.json and world.json read: line {line}: {:?} {}",
                r.code, r.message
            ),
        },
    )
}

/// A world on the Romania scenario under the law of its folder, spec
/// defaults, seed 42.
pub fn lever_world(scale: u32) -> World {
    let law = scenario_law(scale);
    World::with_law(
        start_state(scale),
        law.registry.clone(),
        law.law.clone(),
        params(),
    )
    .expect("the world builds under the scenario's law")
}

/// The index of a key of a keyed lever, in the world's registry.
pub fn key_of(w: &World, lever: LeverId, key: &str) -> u32 {
    let meta = w.registry().get(lever);
    let i = meta
        .keys
        .iter()
        .position(|k| k == key)
        .unwrap_or_else(|| panic!("{} has no key {key}: {:?}", lever.as_str(), meta.keys));
    u32::try_from(i).expect("a key index fits u32")
}

/// The index of an option of a choice lever.
pub fn option_of(meta: &LeverMeta, option: &str) -> u16 {
    let i = meta
        .options
        .iter()
        .position(|o| o == option)
        .unwrap_or_else(|| panic!("{} has no option {option}", meta.id.as_str()));
    u16::try_from(i).expect("an option index fits u16")
}

/// A change as soon as the lever allows.
pub fn set(lever: LeverId, key: Option<u32>, value: LeverValue) -> Order {
    Order::Set(Command {
        lever,
        key,
        value,
        effective: None,
    })
}

/// A change on a requested day.
pub fn set_on(lever: LeverId, key: Option<u32>, value: LeverValue, effective: Date) -> Order {
    Order::Set(Command {
        lever,
        key,
        value,
        effective: Some(effective),
    })
}

/// Give an order today; it must be accepted.
pub fn give(w: &mut World, order: Order) -> Accepted {
    let shown = format!("{order:?}");
    w.give(order).unwrap_or_else(|r| {
        panic!(
            "{shown} on {:?}: rejected {:?}: {}",
            w.date(),
            r.code,
            r.message
        )
    })
}

/// Step until the next day to run is `day` (no order given).
pub fn step_to(w: &mut World, day: Date) {
    assert!(w.date() <= day, "the world is past {day:?}");
    while w.date() < day {
        step(w);
    }
}

/// Step through `last` (included), checking every day; the daily state hashes.
pub fn run_checked_through(w: &mut World, last: Date) -> Vec<u64> {
    let mut hashes = Vec::new();
    while w.date() <= last {
        let r = step(w);
        assert_day_checks(w, &r);
        hashes.push(r.state_hash);
    }
    hashes
}

/// What a test plans to give: a change, or the cancel of the n-th order it
/// gave (counted from 0), whose sequence number it learns when giving it.
#[derive(Debug, Clone)]
pub enum Plan {
    /// Give this order.
    Give(Order),
    /// Cancel the n-th order given in this run.
    CancelNth(usize),
}

/// Run a world through `last`, giving each planned order before the day it
/// is dated runs, checking every day. Returns the daily state hashes and
/// what each order's acceptance said.
pub fn run_plan(w: &mut World, plan: &[(Date, Plan)], last: Date) -> (Vec<u64>, Vec<Accepted>) {
    let mut accepted: Vec<Accepted> = Vec::new();
    let mut hashes = Vec::new();
    let mut i = 0;
    while w.date() <= last {
        let today = w.date();
        while i < plan.len() && plan[i].0 == today {
            let order = match &plan[i].1 {
                Plan::Give(o) => o.clone(),
                Plan::CancelNth(n) => Order::Cancel {
                    seq: accepted[*n].seq,
                },
            };
            accepted.push(give(w, order));
            i += 1;
        }
        assert!(
            i == plan.len() || plan[i].0 > today,
            "the plan is in date order and starts on or after {today:?}"
        );
        let r = step(w);
        assert_day_checks(w, &r);
        hashes.push(r.state_hash);
    }
    assert_eq!(i, plan.len(), "every planned order was given");
    (hashes, accepted)
}

/// Replay a command log on a fresh world of the scenario: each line given on
/// its `given` day with its logged effective day. Returns the world and the
/// daily state hashes through `last`.
pub fn replay(scale: u32, log: &[LogLine], last: Date) -> (World, Vec<u64>) {
    let mut w = lever_world(scale);
    let mut hashes = Vec::new();
    let mut i = 0;
    while w.date() <= last {
        let today = w.date();
        while i < log.len() && log[i].given == today {
            let order = match &log[i].order {
                Order::Set(c) => Order::Set(Command {
                    effective: log[i].effective,
                    ..c.clone()
                }),
                cancel @ Order::Cancel { .. } => cancel.clone(),
            };
            let a = give(&mut w, order);
            assert_eq!(Some(a.seq), log[i].seq, "replay gives the logged seq");
            i += 1;
        }
        assert!(
            i == log.len() || log[i].given > today,
            "the log is in the order given"
        );
        let r = step(&mut w);
        assert_day_checks(&w, &r);
        hashes.push(r.state_hash);
    }
    assert_eq!(i, log.len(), "every logged order was replayed");
    (w, hashes)
}

/// The first day whose hashes differ, if any.
pub fn first_difference(a: &[u64], b: &[u64]) -> Option<usize> {
    assert_eq!(a.len(), b.len(), "runs of the same length");
    a.iter().zip(b).position(|(x, y)| x != y)
}

/// The public firm unit of a catalogue industry (size class public, 3).
pub fn public_unit(scale: u32, industry_id: &str) -> usize {
    let s = start_state(scale);
    let j = s
        .industry_id
        .iter()
        .position(|x| x == industry_id)
        .unwrap_or_else(|| panic!("no industry {industry_id}"));
    let units: Vec<usize> = (0..s.unit_industry.len())
        .filter(|&u| usize::from(s.unit_industry[u]) == j && s.unit_size[u] == 3)
        .collect();
    assert_eq!(units.len(), 1, "one public unit of {industry_id}");
    units[0]
}

/// A budget line in a closed month.
pub fn budget_month<'a>(w: &'a World, line: &str, month: u32) -> &'a BudgetLineMonth {
    w.budget_months()
        .iter()
        .find(|b| b.line == line && b.month == Month(month))
        .unwrap_or_else(|| panic!("no budget month {month} of line {line}"))
}

/// Whether a lever is active in a registry.
pub fn is_active(meta: &LeverMeta) -> bool {
    meta.status == Status::Active
}

/// The keys of a lever's commands: every key of a keyed lever, or none.
pub fn keys_of(meta: &LeverMeta) -> Vec<Option<u32>> {
    if meta.key.is_some() {
        assert!(
            !meta.keys.is_empty(),
            "{} is keyed by {:?} and has no keys",
            meta.id.as_str(),
            meta.key
        );
        (0..meta.keys.len())
            .map(|k| Some(u32::try_from(k).expect("fits")))
            .collect()
    } else {
        vec![None]
    }
}

/// The working Python interpreter (`python3`, else `python`).
pub fn python() -> &'static str {
    static P: OnceLock<&'static str> = OnceLock::new();
    P.get_or_init(|| {
        for p in ["python3", "python"] {
            if Process::new(p)
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success())
            {
                return p;
            }
        }
        panic!("no Python found")
    })
}

/// A report's day, for messages.
pub fn day_of(r: &DayReport) -> String {
    format!("{:?}", r.date)
}
