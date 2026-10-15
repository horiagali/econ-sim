//! Shared by the acceptance tests of the firm units' undeclared sales and
//! opening inventories (AC-FUI-*): the two fixtures, changed margin files, the
//! Python reference, and stages A to J on the fixture population.

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use econ_io::popgen::{
    read_attribute_margins, read_firm_units, read_housing_margins, read_income_margins,
    read_job_margins, read_margins, read_wage_margins,
};
use econ_popgen::{
    AttrParams, EmployerParams, FirmUnits, GenParams, HousingParams, IncomeParams, JobParams,
    UnitSize, WageParams, assign_attributes, assign_employers, assign_housing, assign_jobs,
    assign_other_incomes, assign_wages, generate,
};
use econ_scenario::{
    FirmOpening, FirmOpeningError, FirmOpeningMargins, FirmUnitAmounts, firm_opening,
    read_firm_opening_margins, read_firm_unit_amounts,
};
use serde_json::Value;

/// One million: shares and tilts are millionths.
pub const U: u64 = 1_000_000;
pub const FIRMS_FIXTURE: &str = "firm_units2021_ro.json";
pub const MARGINS_FIXTURE: &str = "firm_undeclared_inventories2021_ro.json";
/// `name HASH` lines: the fixtures, then the three changed margin files of AC-FUI-09.
pub const GOLDEN: &str = "firm_undeclared_inventories.hashes";
/// `employer` of a person without one.
pub const NO_EMPLOYER: u16 = u16::MAX;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fixture_path(name: &str) -> PathBuf {
    repo_root().join("python/pipeline/fixtures").join(name)
}

pub fn fixture_text(name: &str) -> String {
    let path = fixture_path(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture {}: {e}", path.display()))
}

pub fn fixture_json(name: &str) -> Value {
    serde_json::from_str(&fixture_text(name)).expect("valid JSON")
}

/// The firm file as JSON.
pub fn firms_json() -> &'static Value {
    static V: OnceLock<Value> = OnceLock::new();
    V.get_or_init(|| fixture_json(FIRMS_FIXTURE))
}

/// The margin file as JSON.
pub fn margins_json() -> &'static Value {
    static V: OnceLock<Value> = OnceLock::new();
    V.get_or_init(|| fixture_json(MARGINS_FIXTURE))
}

/// The firm units as stage J reads them.
pub fn firms() -> &'static FirmUnits {
    static F: OnceLock<FirmUnits> = OnceLock::new();
    F.get_or_init(|| {
        read_firm_units(&fixture_text(FIRMS_FIXTURE))
            .expect("the firm file reads")
            .margins
    })
}

/// Each unit's output, value added and wages.
pub fn amounts() -> &'static Vec<FirmUnitAmounts> {
    static A: OnceLock<Vec<FirmUnitAmounts>> = OnceLock::new();
    A.get_or_init(|| read_firm_unit_amounts(&fixture_text(FIRMS_FIXTURE)).expect("amounts read"))
}

/// The firm file's industry ids, in its order.
pub fn industry_ids() -> Vec<&'static str> {
    firms_json()["industries"]
        .as_array()
        .expect("industries")
        .iter()
        .map(|i| i["id"].as_str().expect("an id"))
        .collect()
}

/// Parse a margin file (changed or not) against the firm file's industries.
pub fn parse_margins(v: &Value) -> Result<FirmOpeningMargins, FirmOpeningError> {
    read_firm_opening_margins(&v.to_string(), &industry_ids())
}

/// The committed margin file.
pub fn margins() -> &'static FirmOpeningMargins {
    static M: OnceLock<FirmOpeningMargins> = OnceLock::new();
    M.get_or_init(|| {
        read_firm_opening_margins(&fixture_text(MARGINS_FIXTURE), &industry_ids())
            .expect("the margin file reads")
    })
}

/// The columns on the fixtures.
pub fn columns() -> &'static FirmOpening {
    static C: OnceLock<FirmOpening> = OnceLock::new();
    C.get_or_init(|| firm_opening(firms(), amounts(), margins()).expect("the fixtures place"))
}

/// The margin file with one change, as JSON.
pub fn changed(change: impl FnOnce(&mut Value)) -> Value {
    let mut v = margins_json().clone();
    change(&mut v);
    v
}

/// The three changed margin files of AC-FUI-09, by their golden names.
pub fn variants() -> Vec<(&'static str, Value)> {
    vec![
        (
            "anchor_20",
            changed(|v| v["undeclared_value_added_share"] = Value::from(200_000)),
        ),
        (
            "every_tilt_1",
            changed(|v| {
                for k in ["wage_tilt", "informal_tilt", "profit_tilt"] {
                    v[k] = Value::from(vec![1_000_000; 4]);
                }
            }),
        ),
        (
            "max_undeclared_share_75",
            changed(|v| v["max_undeclared_share"] = Value::from(750_000)),
        ),
    ]
}

/// The golden hash of a name.
pub fn golden(name: &str) -> String {
    let path = repo_root().join("tests/golden").join(GOLDEN);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("golden file {}: {e}", path.display()));
    text.lines()
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            Some((parts.next()?, parts.next()?))
        })
        .find(|&(k, _)| k == name)
        .unwrap_or_else(|| panic!("{GOLDEN} has no line {name}"))
        .1
        .to_string()
}

pub fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

// --- classes -----------------------------------------------------------------

/// A unit's class: 0 micro, 1 small, 2 large, 3 foreign-controlled; `None`
/// for a public unit.
pub fn class(f: usize) -> Option<usize> {
    let u = &firms().units[f];
    match u.size {
        UnitSize::Public => None,
        _ if !u.domestic => Some(3),
        UnitSize::Micro => Some(0),
        UnitSize::Small => Some(1),
        UnitSize::Large => Some(2),
    }
}

/// The unit's industry group.
pub fn group(f: usize) -> u8 {
    firms().industry_group[usize::from(firms().units[f].industry)]
}

// --- the Python reference ----------------------------------------------------

/// `python/reference/firm_reference.py FIRMS MARGINS --units` on a margin
/// file (written next to the test binary's temporary files).
pub fn reference(name: &str, margins: &Value) -> Value {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("econ-scenario-fui");
    std::fs::create_dir_all(&dir).expect("a temporary folder");
    let file = dir.join(format!("{name}.json"));
    std::fs::write(&file, margins.to_string()).expect("write the margin file");
    let script = repo_root().join("python/reference/firm_reference.py");
    let mut last = String::from("no Python found");
    for python in ["python3", "python"] {
        let run = Command::new(python)
            .arg(&script)
            .arg(fixture_path(FIRMS_FIXTURE))
            .arg(&file)
            .arg("--units")
            .output();
        match run {
            Ok(out) if out.status.success() => {
                return serde_json::from_slice(&out.stdout).expect("the reference prints JSON");
            }
            Ok(out) => last = String::from_utf8_lossy(&out.stderr).into_owned(),
            Err(e) => last = e.to_string(),
        }
    }
    panic!(
        "the differential test needs Python 3 on PATH to run {}: {last}",
        script.display()
    );
}

// --- stages I and J on the fixture population -----------------------------------

/// What AC-FUI-04 reads of a population: per person the weight, `wage`,
/// `wage_declared` and `employer`.
#[derive(Debug)]
pub struct Workers {
    pub weight: Vec<u64>,
    pub wage: Vec<u64>,
    pub wage_declared: Vec<u64>,
    pub employer: Vec<u16>,
}

/// Stages A to J (those that I and J need) at a scale, seed 42.
pub fn workers(scale: u32) -> Workers {
    let seed = 42;
    let file = read_margins(&fixture_text("census2021_margins_ro.json")).expect("margins");
    let pop = generate(&file.margins, &GenParams::new(scale, seed)).expect("population");
    let ea = read_attribute_margins(
        &fixture_text("census2021_edu_activity_ro.json"),
        &file.margins.counties,
    )
    .expect("attribute margins");
    let attrs = assign_attributes(&pop, &ea.margins, &AttrParams::new(seed)).expect("stage C");
    let jm = read_job_margins(&fixture_text("census2021_jobs_ro.json"), &ea.margins)
        .expect("job margins");
    let jobs = assign_jobs(&pop, &attrs, &jm.margins, &JobParams::new(seed)).expect("stage D");
    let hm = read_housing_margins(&fixture_text("census2021_housing_ro.json")).expect("housing");
    let housing = assign_housing(&pop, &hm.margins, &HousingParams::new(seed)).expect("stage E");
    let wm = read_wage_margins(&fixture_text("earnings2021_ro.json")).expect("earnings");
    let wages =
        assign_wages(&pop, &attrs, &jobs, &wm.margins, &WageParams::new(seed)).expect("stage F");
    let im = read_income_margins(&fixture_text("other_incomes2021_ro.json")).expect("incomes");
    let incomes = assign_other_incomes(
        &pop,
        &attrs,
        &jobs,
        &housing,
        &wages,
        &im.margins,
        &IncomeParams::new(seed),
    )
    .expect("stage I");
    let employers = assign_employers(
        &pop,
        &jobs,
        Some(&wages),
        firms(),
        &EmployerParams::new(seed),
    )
    .expect("stage J");
    let weight = pop
        .persons
        .household_id
        .iter()
        .map(|&h| u64::from(pop.households.hh_weight[h as usize]))
        .collect();
    Workers {
        weight,
        wage: wages.wage,
        wage_declared: incomes.wage_declared,
        employer: employers.employer,
    }
}
