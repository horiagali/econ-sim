//! Shared by the acceptance tests of the starting budget (AC-BUD-*): the
//! fixtures, the loaded budget, malformed variants of the file, the golden
//! file, the Python reference and the starting population.

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use econ_io::budget::{Budget, BudgetError, EconType, N_TYPES};
use econ_scenario::{BuildParams, Scenario, build, read_inputs};
use econ_types::Bani;
use serde_json::Value;

/// The budget fixture.
pub const BUDGET_FIXTURE: &str = "budget2021_ro.json";
/// The golden file of the budget hash: `fixture HASH`.
pub const BUDGET_GOLDEN: &str = "budget_ro_2021.hashes";
/// The Python reference.
pub const REFERENCE: &str = "python/reference/budget_reference.py";
/// The seed of the starting population (AC-BUD-09).
pub const SEED: u64 = 42;

/// The twelve economic types in file order, with their ids in the file and
/// the published total each must add up to.
pub const TYPES: [(EconType, &str, &str); N_TYPES] = [
    (EconType::Wages, "wages", "D11"),
    (
        EconType::EmployerContributions,
        "employer_contributions",
        "D12",
    ),
    (EconType::GoodsServices, "goods_services", "P2"),
    (EconType::SocialBenefitsCash, "social_benefits_cash", "D62"),
    (
        EconType::SocialBenefitsInKind,
        "social_benefits_in_kind",
        "D632",
    ),
    (EconType::Subsidies, "subsidies", "D3"),
    (EconType::Interest, "interest", "D4"),
    (EconType::Investment, "investment", "P5"),
    (EconType::NonProducedAssets, "non_produced_assets", "NP"),
    (EconType::CurrentTransfers, "current_transfers", "D7"),
    (EconType::CapitalTransfers, "capital_transfers", "D9"),
    (EconType::OtherTaxes, "other_taxes", "D29_D5_D8"),
];

/// The five public firm units (spec, "Public employment").
pub const PUBLIC_UNITS: [&str; 5] = [
    "defence_services.public",
    "education_services.public",
    "health_services.public",
    "public_administration.public",
    "social_care.public",
];

/// The ten COFOG divisions.
pub const DIVISIONS: [&str; 10] = [
    "GF01", "GF02", "GF03", "GF04", "GF05", "GF06", "GF07", "GF08", "GF09", "GF10",
];

/// Total occupied public posts, December 2021 (spec, "Public employment").
pub const POSTS: u64 = 1_262_318;
/// Total expenditure (TE) 2021, whole lei (spec, "Economic types").
pub const TE_LEI: i64 = 474_031_500_000;

/// `employment_status` of an employee (stage D).
pub const KIND_EMPLOYEE: u8 = 1;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fixtures_dir() -> PathBuf {
    repo_root().join("python/pipeline/fixtures")
}

pub fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = fixtures_dir().join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {}: {e}", path.display()))
}

pub fn fixture_json(name: &str) -> Value {
    serde_json::from_slice(&fixture_bytes(name)).expect("valid JSON")
}

/// The budget fixture as JSON, read once.
pub fn budget_json() -> &'static Value {
    static V: OnceLock<Value> = OnceLock::new();
    V.get_or_init(|| fixture_json(BUDGET_FIXTURE))
}

/// The budget fixture, loaded once.
pub fn budget() -> &'static Budget {
    static B: OnceLock<Budget> = OnceLock::new();
    B.get_or_init(|| {
        Budget::from_json(&fixture_bytes(BUDGET_FIXTURE)).expect("the budget fixture loads")
    })
}

/// Load a copy of the budget fixture changed by `edit`.
pub fn load_variant(edit: impl FnOnce(&mut Value)) -> Result<Budget, BudgetError> {
    let mut v = budget_json().clone();
    edit(&mut v);
    let bytes = serde_json::to_vec(&v).expect("JSON serialises");
    Budget::from_json(&bytes)
}

/// The index of the line `id` in the fixture.
pub fn line_index(v: &Value, id: &str) -> usize {
    v["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .position(|l| l["id"] == id)
        .unwrap_or_else(|| panic!("no line {id}"))
}

/// The index of the sector `id` in the fixture.
pub fn sector_index(v: &Value, id: &str) -> usize {
    v["employment"]["sectors"]
        .as_array()
        .expect("sectors")
        .iter()
        .position(|s| s["id"] == id)
        .unwrap_or_else(|| panic!("no sector {id}"))
}

/// The index of the public unit `unit` in the fixture.
pub fn unit_index(v: &Value, unit: &str) -> usize {
    v["public_units"]
        .as_array()
        .expect("public units")
        .iter()
        .position(|u| u["unit"] == unit)
        .unwrap_or_else(|| panic!("no public unit {unit}"))
}

/// Add `delta` to the whole number at `v`.
pub fn bump(v: &mut Value, delta: i64) {
    let n = v.as_i64().expect("a whole number");
    *v = Value::from(n + delta);
}

/// Whole lei of a fixture as bani.
pub fn lei(v: &Value) -> Bani {
    Bani(v.as_i64().expect("an amount in whole lei") * 100)
}

/// A published total of the loaded budget, by ESA code.
pub fn published(b: &Budget, code: &str) -> Bani {
    b.published
        .0
        .iter()
        .find(|(c, _)| c == code)
        .unwrap_or_else(|| panic!("no published total {code}"))
        .1
}

/// The sum of type `t` over the lines.
pub fn lines_sum(b: &Budget, t: EconType) -> Bani {
    Bani(b.lines.iter().map(|l| l.per_year[t as usize].0).sum())
}

/// The relative difference `got / want - 1`.
#[allow(clippy::cast_precision_loss)]
pub fn rel(got: i128, want: i128) -> f64 {
    got as f64 / want as f64 - 1.0
}

/// Assert `got` is within `tol` (relative) of `want`.
pub fn assert_within(what: &str, got: i128, want: i128, tol: f64) {
    let r = rel(got, want);
    assert!(
        r.abs() <= tol,
        "{what}: {got} against {want} ({:+.2}%), tolerance {:.0}%",
        r * 100.0,
        tol * 100.0
    );
}

/// The lines `KEY HASH` of the budget golden file.
pub fn golden(key: &str) -> String {
    let path = repo_root().join("tests/golden").join(BUDGET_GOLDEN);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("golden file {}: {e}", path.display()));
    text.lines()
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            Some((parts.next()?, parts.next()?))
        })
        .find(|&(k, _)| k == key)
        .unwrap_or_else(|| panic!("{BUDGET_GOLDEN} has no line {key}"))
        .1
        .to_string()
}

pub fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// Run a Python script of the repository and parse what it prints as JSON.
/// A non-zero exit is accepted when it still prints JSON (the reference
/// reports failed checks that way).
pub fn python_json(script: &str, args: &[&str]) -> Value {
    let script = repo_root().join(script);
    let mut last = String::from("no Python found");
    for python in ["python3", "python"] {
        match Command::new(python).arg(&script).args(args).output() {
            Ok(out) => match serde_json::from_slice(&out.stdout) {
                Ok(v) => return v,
                Err(e) => {
                    last = format!("{e}: {}", String::from_utf8_lossy(&out.stderr));
                }
            },
            Err(e) => last = e.to_string(),
        }
    }
    panic!(
        "the differential test needs Python 3 on PATH to run {}: {last}",
        script.display()
    );
}

/// The Python reference on the fixtures, with the table (`--tables`), once.
pub fn reference() -> &'static Value {
    static V: OnceLock<Value> = OnceLock::new();
    V.get_or_init(|| python_json(REFERENCE, &["--tables"]))
}

/// The starting population at 1:1000 or 1:100, seed 42, built once.
pub fn built(scale: u32) -> &'static Scenario {
    static B: [OnceLock<Scenario>; 2] = [OnceLock::new(), OnceLock::new()];
    let slot = match scale {
        1000 => 0,
        100 => 1,
        _ => panic!("no cached build at 1:{scale}"),
    };
    B[slot].get_or_init(|| {
        let inputs = read_inputs(&fixtures_dir()).expect("the fixtures read");
        let params = BuildParams {
            sample_scale: scale,
            rng_seed: SEED,
        };
        build(&inputs, &params).expect("the fixtures build")
    })
}

/// Weighted employees and their wages a year, in bani, of each of the five
/// public units (in [`PUBLIC_UNITS`] order) in a built scenario: persons
/// whose `employer` is the unit and whose `employment_status` is employee.
pub fn public_employees(sc: &Scenario) -> [(i128, i128); 5] {
    let hh_w = &sc.population.households.hh_weight;
    let status = &sc.stages.jobs.employment_status;
    let wage = &sc.stages.wages.wage;
    let mut out = [(0i128, 0i128); 5];
    for (i, &e) in sc.stages.employers.employer.iter().enumerate() {
        if e == econ_popgen::NO_EMPLOYER || status[i] != KIND_EMPLOYEE {
            continue;
        }
        let id = &sc.firm_units.unit_id[usize::from(e)];
        if let Some(u) = PUBLIC_UNITS.iter().position(|p| p == id) {
            let h = sc.population.persons.household_id[i] as usize;
            let w = i128::from(hh_w[h]);
            out[u].0 += w;
            out[u].1 += w * i128::from(wage[i]) * 12;
        }
    }
    out
}
