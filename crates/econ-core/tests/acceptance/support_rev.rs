//! Shared by the acceptance tests of M3's investment and revenue (specs
//! economy/investment-capital, excise, eu-funds, government-revenue-2021,
//! imputed-rent): the input-output table by catalogue industry, units and
//! households by key, the ledger's receipts and the checks of a day. Builds
//! on `super::support` and `super::support_m3`.

// Each test file uses a part of this one.
#![allow(dead_code)]

use econ_core::world::{DayReport, StartState, World};
use econ_ledger::{FlowCode, Instrument, Sector};
use econ_types::Bani;
use serde_json::Value;

use super::support::io_table_json;

/// The catalogue index of an industry id.
pub fn industry(start: &StartState, id: &str) -> usize {
    start
        .industry_id
        .iter()
        .position(|x| x == id)
        .unwrap_or_else(|| panic!("no catalogue industry {id}"))
}

/// The firm units of a catalogue industry.
pub fn units_of(start: &StartState, j: usize) -> Vec<usize> {
    (0..start.unit_industry.len())
        .filter(|&u| usize::from(start.unit_industry[u]) == j)
        .collect()
}

/// Whether a unit is a public (non-market) producer: size class public.
pub fn is_public(start: &StartState, u: usize) -> bool {
    start.unit_size[u] == 3
}

/// What a sector received under a code on the latest day.
pub fn received(w: &World, c: FlowCode, s: Sector) -> Bani {
    w.ledger().flows().received(c, s)
}

/// A balance in the ledger.
pub fn balance(w: &World, s: Sector, i: Instrument) -> Bani {
    w.ledger().balance(s, i)
}

/// Every check of the day held.
pub fn assert_checks(r: &DayReport) {
    assert!(
        r.checks_ok && r.failed.is_empty(),
        "{:?}: the day's checks failed: {:?}",
        r.date,
        r.failed
    );
}

/// Sum of the weights.
pub fn weight_total(weights: &[u32]) -> i64 {
    weights.iter().map(|&w| i64::from(w)).sum()
}

/// Billions of lei, for reports.
pub fn bn(b: Bani) -> f64 {
    b.get() as f64 / 1e11
}

/// `|a / b − 1|` in millionths.
pub fn gap_ppm(a: Bani, b: Bani) -> i64 {
    let (a, b) = (i128::from(a.get()), i128::from(b.get()));
    assert!(b != 0);
    i64::try_from((a - b).abs() * 1_000_000 / b.abs()).expect("fits")
}

/// Lei as bani from the spec's millions.
pub fn million_lei(m: f64) -> Bani {
    Bani(i64::try_from((m * 1e8).round() as i128).expect("fits"))
}

// --- the input-output table, by catalogue industry ---------------------------

/// A number of the table, lei.
pub fn lei(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

/// The NACE rows of the table.
pub fn nace_count() -> usize {
    io_table_json()["industries"]
        .as_array()
        .expect("industries")
        .len()
}

/// The NACE row of a code.
pub fn nace(code: &str) -> usize {
    io_table_json()["industries"]
        .as_array()
        .expect("industries")
        .iter()
        .position(|i| i["code"] == code)
        .unwrap_or_else(|| panic!("no NACE industry {code}"))
}

/// A catalogue industry's NACE rows and its share of each: some catalogue
/// industries take parts of several NACE industries (`admin_support` is all
/// of N77 to N82).
pub fn parts(id: &str) -> Vec<(usize, f64)> {
    let t = io_table_json();
    let mut v = Vec::new();
    for c in t["catalogue"].as_array().expect("catalogue") {
        for p in c["parts"].as_array().expect("parts") {
            if p["id"] == id {
                v.push((
                    nace(c["industry"].as_str().expect("code")),
                    lei(&p["share"]) / 1e6,
                ));
            }
        }
    }
    assert!(!v.is_empty(), "no catalogue industry {id}");
    v
}

/// A catalogue industry's amount: the NACE amounts of its parts times its
/// shares.
pub fn by_parts(id: &str, f: impl Fn(usize) -> f64) -> f64 {
    parts(id).into_iter().map(|(j, share)| f(j) * share).sum()
}

/// What NACE industry `j` buys of product `i`, home and imported, lei.
pub fn bought(i: usize, j: usize) -> f64 {
    let t = io_table_json();
    lei(&t["domestic"][i][j]) + lei(&t["imports_eu"][i][j]) + lei(&t["imports_non_eu"][i][j])
}

/// NACE industry `j`'s input purchases, home and imported, lei.
pub fn inputs(j: usize) -> f64 {
    (0..nace_count()).map(|i| bought(i, j)).sum()
}

/// A final-demand column's purchases of product `i`, home and imported, lei.
pub fn final_demand(column: &str, i: usize) -> f64 {
    let c = &io_table_json()["final"][column];
    lei(&c["domestic"][i]) + lei(&c["imports_eu"][i]) + lei(&c["imports_non_eu"][i])
}

/// A row of the table by NACE industry, lei.
pub fn row(name: &str, j: usize) -> f64 {
    lei(&io_table_json()[name][j])
}

/// The fuel duty of the excise spec's table, millionths of a ban per real
/// unit; and its exempt buyers.
pub const FUEL_DUTY_PPM: u32 = 512_500;
pub const TOBACCO_DUTY_PPM: u32 = 2_030_400;
pub const BEVERAGES_DUTY_PPM: u32 = 71_200;
pub const EXEMPT_BUYERS: [&str; 4] = [
    "refined_petroleum",
    "chemicals",
    "fertilisers",
    "air_transport",
];

/// The farm units' industries (eu-funds, "Farm units").
pub const FARM_INDUSTRIES: [&str; 6] = [
    "agri_cereals",
    "agri_oilseeds",
    "agri_fruit_veg",
    "agri_vineyards",
    "livestock_meat",
    "livestock_dairy",
];

/// A catalogue industry's starting fuel duty a year, lei: the duty on its
/// share of its NACE industry's purchases of C19; none for an exempt buyer.
pub fn starting_fuel_duty(id: &str) -> f64 {
    if EXEMPT_BUYERS.contains(&id) {
        return 0.0;
    }
    let fuel = parts("refined_petroleum");
    assert_eq!(fuel.len(), 1, "refined petroleum is part of C19 only");
    let (c19, fuel_share) = fuel[0];
    by_parts(id, |j| bought(c19, j)) * fuel_share * f64::from(FUEL_DUTY_PPM) / 1e6
}
