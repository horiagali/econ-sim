//! Shared by the acceptance tests of the M3 criteria on the `World`
//! (taxation and social transfers on pay day, money and banking, monetary
//! policy, fiscal policy, trade): worlds under the 2021 law, flow codes by
//! name, the ledger's flows of a day and over a span of days, the postings
//! of the latest day. Builds on `super::support`.

// Each test file uses a part of this one.
#![allow(dead_code)]

use std::collections::BTreeMap;

use econ_core::world::{DayReport, Law, StartEdit, World, WorldParams};
use econ_ledger::{FlowCode, Instrument, Leg, Sector};
use econ_types::Bani;
use serde_json::Value;

use super::support::{params, repo_root, start_state, step};

/// The flow code the ledger names so. The M3 codes are named by the specs
/// (`interest.loan`, `finance.loan`, `interest.gov_debt`, …) and numbered by
/// the implementation, so the tests find them by name.
pub fn code(name: &str) -> FlowCode {
    FlowCode::ALL
        .iter()
        .copied()
        .find(|c| c.name() == name)
        .unwrap_or_else(|| panic!("the ledger has no flow code named {name}"))
}

/// Whether a code is a financial transaction (the `finance.*` codes), not a
/// current or capital flow of a budget.
pub fn is_financing(c: FlowCode) -> bool {
    c.name().starts_with("finance.")
}

/// A world on the Romania scenario under the 2021 law, spec defaults.
pub fn world_2021(scale: u32) -> World {
    world_2021_with(scale, Vec::new(), params())
}

/// The same with edits of its starting state and other parameters.
pub fn world_2021_with(scale: u32, edits: Vec<StartEdit>, p: WorldParams) -> World {
    let mut start = start_state(scale);
    for e in edits {
        start.apply(e);
    }
    World::new(start, Law::law_2021(), p).expect("the world builds under the 2021 law")
}

/// The latest day's flow of a code for a sector (received positive).
pub fn flow(w: &World, c: FlowCode, s: Sector) -> Bani {
    w.ledger().flows().get(c, s)
}

/// The latest day's flows of a sector by code (flow codes only).
pub fn flows_of(w: &World, s: Sector) -> BTreeMap<FlowCode, Bani> {
    w.ledger()
        .flows()
        .by_code_sector
        .iter()
        .filter(|((c, sec), _)| *sec == s && c.is_flow())
        .map(|((c, _), &v)| (*c, v))
        .collect()
}

/// Flows added up over days, by (code, sector).
#[derive(Debug, Default, Clone)]
pub struct FlowSum(pub BTreeMap<(FlowCode, Sector), Bani>);

impl FlowSum {
    /// Add the latest day's flows.
    pub fn add_day(&mut self, w: &World) {
        for (&(c, s), &v) in &w.ledger().flows().by_code_sector {
            if c.is_flow() {
                *self.0.entry((c, s)).or_insert(Bani::ZERO) += v;
            }
        }
    }

    /// The sum for a code and sector.
    pub fn get(&self, c: FlowCode, s: Sector) -> Bani {
        self.0.get(&(c, s)).copied().unwrap_or(Bani::ZERO)
    }

    /// A sector's sums by code.
    pub fn of(&self, s: Sector) -> BTreeMap<FlowCode, Bani> {
        self.0
            .iter()
            .filter(|((_, sec), _)| *sec == s)
            .map(|((c, _), &v)| (*c, v))
            .collect()
    }
}

/// The legs posted on the day of a report.
pub fn legs_of_day(w: &World, r: &DayReport) -> Vec<Leg> {
    w.ledger()
        .debug_postings()
        .iter()
        .filter(|p| p.tick == r.day.0)
        .map(|p| p.leg)
        .collect()
}

/// Sum of the legs of a code moving an instrument from one sector to another.
pub fn legs_sum(legs: &[Leg], c: FlowCode, inst: Instrument, payer: Sector, payee: Sector) -> Bani {
    legs.iter()
        .filter(|l| l.code == c && l.instrument == inst && l.payer == payer && l.payee == payee)
        .map(|l| l.amount)
        .fold(Bani::ZERO, |a, b| a + b)
}

/// Step to the end of the next month end; returns that day's report and
/// adds every day's flows to `sum`.
pub fn run_to_month_end(w: &mut World, sum: &mut FlowSum) -> DayReport {
    loop {
        let r = step(w);
        sum.add_day(w);
        if r.date.is_month_end() {
            return r;
        }
    }
}

/// The budget fixture (`budget2021_ro.json`): whole lei.
pub fn budget_json() -> Value {
    let path = repo_root().join("python/pipeline/fixtures/budget2021_ro.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {}: {e}", path.display()));
    serde_json::from_str(&text).expect("valid JSON")
}

/// A programme's cash benefits in the budget, bani.
pub fn budget_programme_cash(id: &str) -> Bani {
    let b = budget_json();
    let p = b["programmes"]
        .as_array()
        .expect("programmes")
        .iter()
        .find(|p| p["id"] == id)
        .unwrap_or_else(|| panic!("no programme {id}"));
    Bani::from_lei(p["cash"].as_i64().expect("whole lei"))
}

/// 2021's GDP in the budget fixture, bani.
pub fn gdp_2021() -> Bani {
    Bani::from_lei(budget_json()["gdp"].as_i64().expect("whole lei"))
}

/// `|a − b| ≤ tolerance_ppm` of `b`.
pub fn within(a: Bani, b: Bani, tolerance_ppm: i64) -> bool {
    let (a, b) = (i128::from(a.get()), i128::from(b.get()));
    (a - b).abs() * 1_000_000 <= b.abs() * i128::from(tolerance_ppm)
}

/// `a / b` in millionths, signed.
pub fn ratio_ppm(a: Bani, b: Bani) -> i64 {
    assert!(b > Bani::ZERO, "a ratio over zero");
    i64::try_from(i128::from(a.get()) * 1_000_000 / i128::from(b.get())).expect("fits")
}
