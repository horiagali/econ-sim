//! Shared by the acceptance tests of the population stages F to J
//! (AC-POPW-*, AC-POPP-*, AC-POPH2-*, AC-POPI-*, AC-POPE-*): the margin
//! fixtures of those stages, the earlier stages on the fixture population, the
//! Python reference with every stage, and the measures several criteria use.
//!
//! The fixture population, the margins of stages A to E and the golden-file
//! helpers are those of `support.rs`.

// Each stage's tests use a part of this file, and a stage can be compiled alone.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use econ_popgen::{
    BorrowerRule, Curve, EDU_GROUPS, EDU_LEVELS, FirmUnit, FirmUnits, HouseholdHousing,
    HousingParams, INDUSTRY_GROUPS, IncomeMargins, InformalPropensity, JobParams, OCCUPATIONS,
    PensionMargins, PensionParams, PersonAttributes, PersonJobs, PersonPensions, PersonWages,
    Population, UnitSize, WageMargins, WageParams, WealthMargins, assign_housing, assign_jobs,
    assign_pensions, assign_wages,
};
use serde_json::{Value, json};

use super::support::{
    DIFF_SCALES, SCALES, SEED, attributes, attributes_at, housing_margins, job_margins, population,
    population_at,
};

/// One million: proportions, shares and ranks are in millionths.
pub const U: u64 = 1_000_000;
/// Margin files are in lei, columns in bani.
pub const BANI_PER_LEU: u64 = 100;
/// `employment_status` of an employee; 0 is not employed, 2 to 4 the self-employed kinds.
pub const EMPLOYEE: u8 = 1;
/// `employment_status` of a person who is not employed.
pub const NO_JOB: u8 = 0;

pub const WAGES_GOLDEN: &str = "popgen_ro_census2021_wages.hashes";
pub const PENSIONS_GOLDEN: &str = "popgen_ro_census2021_pensions.hashes";
pub const WEALTH_GOLDEN: &str = "popgen_ro_census2021_wealth.hashes";
pub const INCOMES_GOLDEN: &str = "popgen_ro_census2021_incomes.hashes";
pub const EMPLOYERS_GOLDEN: &str = "popgen_ro_census2021_employers.hashes";

pub const EARNINGS_FIXTURE: &str = "earnings2021_ro.json";
pub const PENSIONS_FIXTURE: &str = "pensions2021_ro.json";
pub const WEALTH_FIXTURE: &str = "household_wealth2021_ro.json";
pub const INCOMES_FIXTURE: &str = "other_incomes2021_ro.json";
pub const FIRMS_FIXTURE: &str = "firm_units2021_ro.json";

/// The three scales, then the fourth scale of the golden files and of the
/// differential tests. A stage is run once at each.
const CACHED: [u32; 4] = [1000, 100, 10, 200];

/// One result per cached scale.
pub type Cache<T> = [OnceLock<T>; 4];

/// The place of a scale in a [`Cache`].
pub fn slot(scale: u32) -> usize {
    CACHED
        .iter()
        .position(|&s| s == scale)
        .expect("a scale of the acceptance tests")
}

// --- fixtures ---------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture_file(name: &str) -> PathBuf {
    repo_root().join("python/pipeline/fixtures").join(name)
}

/// A margin fixture as JSON, for the entries that are checks and not inputs.
pub fn fixture(name: &str) -> Value {
    let path = fixture_file(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("margin fixture {}: {e}", path.display()));
    serde_json::from_str(&text).expect("valid JSON")
}

/// A non-negative integer of a fixture.
pub fn num(v: &Value) -> u64 {
    v.as_u64().expect("non-negative integer")
}

fn ppm(v: &Value) -> u32 {
    u32::try_from(num(v)).expect("millionths fit 32 bits")
}

fn years(v: &Value) -> u16 {
    u16::try_from(num(v)).expect("age in years")
}

fn flatten(v: &Value, out: &mut Vec<u64>) {
    match v {
        Value::Array(items) => items.iter().for_each(|i| flatten(i, out)),
        other => out.push(num(other)),
    }
}

/// A table of a fixture, row-major.
pub fn table(v: &Value) -> Vec<u64> {
    let mut out = Vec::new();
    flatten(v, &mut out);
    out
}

/// A table of millionths, row-major.
pub fn table_ppm(v: &Value) -> Vec<u32> {
    table(v)
        .into_iter()
        .map(|x| u32::try_from(x).expect("millionths fit 32 bits"))
        .collect()
}

fn len(v: &Value) -> usize {
    v.as_array().expect("array").len()
}

fn curve(v: &Value) -> Curve {
    Curve {
        rank: table_ppm(&v["rank"]),
        value: table_ppm(&v["value"]),
    }
}

fn edu_group_of_level(v: &Value) -> [u8; EDU_LEVELS] {
    let groups: Vec<u8> = table(&v["edu_group_of_level"])
        .into_iter()
        .map(|g| u8::try_from(g).expect("education group"))
        .collect();
    groups.try_into().expect("one group per education level")
}

/// The earnings margin fixture (stage F).
pub fn wage_margins() -> &'static WageMargins {
    static M: OnceLock<WageMargins> = OnceLock::new();
    M.get_or_init(|| {
        let v = fixture(EARNINGS_FIXTURE);
        assert_eq!(v["sexes"], json!(["F", "M"]), "sex order");
        assert_eq!(
            len(&v["industry_groups"]),
            INDUSTRY_GROUPS,
            "industry groups"
        );
        assert_eq!(len(&v["occupations"]), OCCUPATIONS, "occupations");
        assert_eq!(len(&v["edu_groups"]), EDU_GROUPS, "education groups");
        let curve = curve(&v["quantile_curve"]);
        WageMargins {
            wage_bill: table(&v["wage_bill"]),
            minimum_wage: num(&v["minimum_wage"]),
            age_group_from: v["age_group_from"]
                .as_array()
                .expect("age groups")
                .iter()
                .map(years)
                .collect(),
            edu_group_of_level: edu_group_of_level(&v),
            rel_sex: table_ppm(&v["rel_sex"]),
            rel_occupation: table_ppm(&v["rel_occupation"]),
            rel_age: table_ppm(&v["rel_age"]),
            rel_education: table_ppm(&v["rel_education"]),
            curve_rank: curve.rank,
            curve_value: curve.value,
        }
    })
}

/// The pension margin fixture (stage G).
pub fn pension_margins() -> &'static PensionMargins {
    static M: OnceLock<PensionMargins> = OnceLock::new();
    M.get_or_init(|| {
        let v = fixture(PENSIONS_FIXTURE);
        assert_eq!(v["sexes"], json!(["F", "M"]), "sex order");
        assert_eq!(len(&v["edu_groups"]), EDU_GROUPS, "education groups");
        let curve = curve(&v["quantile_curve"]);
        PensionMargins {
            pension_bill: num(&v["pension_bill"]),
            minimum_pension: num(&v["minimum_pension"]),
            rel_sex: table_ppm(&v["rel_sex"])
                .try_into()
                .expect("one proportion per sex"),
            edu_group_of_level: edu_group_of_level(&v),
            rel_education: table_ppm(&v["rel_education"]),
            curve_rank: curve.rank,
            curve_value: curve.value,
        }
    })
}

fn borrower_rule(v: &Value) -> BorrowerRule {
    BorrowerRule {
        share_ppm: ppm(&v["share"]),
        age_from: years(&v["age_from"]),
        age_to: years(&v["age_to"]),
        income_link_ppm: ppm(&v["income_link"]),
        urban_bonus_ppm: ppm(&v["urban_bonus"]),
    }
}

/// The household wealth margin fixture (stage H).
pub fn wealth_margins() -> &'static WealthMargins {
    static M: OnceLock<WealthMargins> = OnceLock::new();
    M.get_or_init(|| {
        let v = fixture(WEALTH_FIXTURE);
        let totals = &v["totals"];
        WealthMargins {
            cash: num(&totals["cash"]),
            deposits: num(&totals["deposits"]),
            housing_loans: num(&totals["housing_loans"]),
            consumer_loans: num(&totals["consumer_loans"]),
            dwellings: num(&totals["dwellings"]),
            rel_locality: table_ppm(&v["rel_locality"]),
            liquid_curve: curve(&v["liquid_curve"]),
            dwelling_curve: curve(&v["dwelling_curve"]),
            loan_curve: curve(&v["loan_curve"]),
            mortgage: borrower_rule(&v["mortgage"]),
            consumer: borrower_rule(&v["consumer"]),
            liquid_income_link_ppm: ppm(&v["income_link"]["liquid"]),
            dwelling_income_link_ppm: ppm(&v["income_link"]["dwelling"]),
        }
    })
}

/// The income margin fixture (stage I).
pub fn income_margins() -> &'static IncomeMargins {
    static M: OnceLock<IncomeMargins> = OnceLock::new();
    M.get_or_init(|| {
        let v = fixture(INCOMES_FIXTURE);
        assert_eq!(
            len(&v["industry_groups"]),
            INDUSTRY_GROUPS,
            "industry groups"
        );
        let propensity = &v["informal_propensity"];
        IncomeMargins {
            farm_income: num(&v["farm_income"]),
            farm_in_kind: num(&v["farm_in_kind"]),
            self_employment_income: num(&v["self_employment_income"]),
            informal_income: num(&v["informal_income"]),
            minimum_wage: num(&v["minimum_wage"]),
            quantile_curve: curve(&v["quantile_curve"]),
            undeclared_wage_share: table_ppm(&v["undeclared_wage_share"]),
            fully_informal_share: ppm(&v["fully_informal_share"]),
            envelope_declared_share: ppm(&v["envelope_declared_share"]),
            low_wage_link: ppm(&v["low_wage_link"]),
            rel_kind: table_ppm(&v["rel_kind"])
                .try_into()
                .expect("employer, own-account worker, family worker"),
            self_employment_education_link: ppm(&v["self_employment_education_link"]),
            self_employment_declared: ppm(&v["self_employment_declared"]),
            farm_declared: ppm(&v["farm_declared"]),
            subsistence_household_share: ppm(&v["subsistence_household_share"]),
            subsistence_bonus_farm_worker: ppm(&v["subsistence_bonus"]["farm_worker"]),
            subsistence_bonus_retired: ppm(&v["subsistence_bonus"]["retired"]),
            subsistence_dispersion: ppm(&v["subsistence_dispersion"]),
            informal_recipient_share: ppm(&v["informal_recipient_share"]),
            informal_propensity: InformalPropensity {
                unemployed: ppm(&propensity["unemployed"]),
                inactive_other: ppm(&propensity["inactive_other"]),
                retired: ppm(&propensity["retired"]),
                student: ppm(&propensity["student"]),
                employer: ppm(&propensity["employer"]),
                own_account: ppm(&propensity["own_account"]),
                family_worker: ppm(&propensity["family_worker"]),
                employee_by_industry_group: table_ppm(&propensity["employee_by_industry_group"]),
            },
        }
    })
}

/// The firm-unit fixture (stage J): industries and units in the file's order.
pub fn firm_units() -> &'static FirmUnits {
    static M: OnceLock<FirmUnits> = OnceLock::new();
    M.get_or_init(|| {
        let v = fixture(FIRMS_FIXTURE);
        assert_eq!(
            v["sizes"],
            json!(["micro", "small", "large", "public"]),
            "size order"
        );
        let industries = v["industries"].as_array().expect("industries");
        let ids: Vec<&str> = industries
            .iter()
            .map(|i| i["id"].as_str().expect("industry id"))
            .collect();
        let industry_group = industries
            .iter()
            .map(|i| u8::try_from(num(&i["industry_group"])).expect("industry group"))
            .collect();
        let units = v["units"]
            .as_array()
            .expect("units")
            .iter()
            .map(|u| {
                let id = u["industry"].as_str().expect("industry of a unit");
                let industry = ids
                    .iter()
                    .position(|&i| i == id)
                    .unwrap_or_else(|| panic!("industry {id} is not listed"));
                let size = match u["size"].as_str().expect("size of a unit") {
                    "micro" => UnitSize::Micro,
                    "small" => UnitSize::Small,
                    "large" => UnitSize::Large,
                    "public" => UnitSize::Public,
                    other => panic!("unknown size {other}"),
                };
                let cohort = u["kind"] == "cohort";
                assert_eq!(
                    cohort,
                    size != UnitSize::Public,
                    "a unit is a cohort or public"
                );
                FirmUnit {
                    industry: u8::try_from(industry).expect("at most 255 industries"),
                    size,
                    domestic: cohort && u["controller"] == "domestic",
                    persons_employed: num(&u["persons_employed"]),
                    wages: num(&u["wages"]),
                }
            })
            .collect();
        FirmUnits {
            industry_group,
            units,
        }
    })
}

// --- the earlier stages on the fixture population ----------------------------

/// The fixture population at a cached scale.
pub fn pop(scale: u32) -> &'static Population {
    static EXTRA: OnceLock<Population> = OnceLock::new();
    if SCALES.contains(&scale) {
        population(scale)
    } else {
        slot(scale);
        EXTRA.get_or_init(|| population_at(scale).into_owned())
    }
}

/// Stage C on the fixture population at a cached scale.
pub fn attrs(scale: u32) -> &'static PersonAttributes {
    static EXTRA: OnceLock<PersonAttributes> = OnceLock::new();
    if SCALES.contains(&scale) {
        attributes(scale)
    } else {
        slot(scale);
        EXTRA.get_or_init(|| attributes_at(scale).into_owned())
    }
}

/// Stage D on the fixture population at a cached scale.
pub fn jobs(scale: u32) -> &'static PersonJobs {
    static JOBS: Cache<PersonJobs> = [const { OnceLock::new() }; 4];
    JOBS[slot(scale)].get_or_init(|| {
        assign_jobs(
            pop(scale),
            attrs(scale),
            job_margins(),
            &JobParams::new(SEED),
        )
        .expect("fixture population gets jobs")
    })
}

/// Stage E on the fixture population at a cached scale.
pub fn housing(scale: u32) -> &'static HouseholdHousing {
    static HOUSING: Cache<HouseholdHousing> = [const { OnceLock::new() }; 4];
    HOUSING[slot(scale)].get_or_init(|| {
        assign_housing(pop(scale), housing_margins(), &HousingParams::new(SEED))
            .expect("fixture population gets housing")
    })
}

/// Stage F on the fixture population at a cached scale.
pub fn wages(scale: u32) -> &'static PersonWages {
    static WAGES: Cache<PersonWages> = [const { OnceLock::new() }; 4];
    WAGES[slot(scale)].get_or_init(|| {
        assign_wages(
            pop(scale),
            attrs(scale),
            jobs(scale),
            wage_margins(),
            &WageParams::new(SEED),
        )
        .expect("fixture population gets wages")
    })
}

/// Stage G on the fixture population at a cached scale.
pub fn pensions(scale: u32) -> &'static PersonPensions {
    static PENSIONS: Cache<PersonPensions> = [const { OnceLock::new() }; 4];
    PENSIONS[slot(scale)].get_or_init(|| {
        assign_pensions(
            pop(scale),
            attrs(scale),
            pension_margins(),
            &PensionParams::new(SEED),
        )
        .expect("fixture population gets pensions")
    })
}

// --- the Python reference ----------------------------------------------------

fn run_reference(scale: u32) -> Value {
    let script = repo_root().join("python/reference/popgen_reference.py");
    let mut last = String::from("no Python found");
    for python in ["python3", "python"] {
        let mut command = Command::new(python);
        command
            .arg(&script)
            .arg(fixture_file("census2021_margins_ro.json"))
            .args(["--sample-scale", &scale.to_string()])
            .args(["--seed", &SEED.to_string()]);
        for (flag, file) in [
            ("--attributes", "census2021_edu_activity_ro.json"),
            ("--jobs", "census2021_jobs_ro.json"),
            ("--housing", "census2021_housing_ro.json"),
            ("--wages", EARNINGS_FIXTURE),
            ("--pensions", PENSIONS_FIXTURE),
            ("--wealth", WEALTH_FIXTURE),
            ("--incomes", INCOMES_FIXTURE),
            ("--firms", FIRMS_FIXTURE),
        ] {
            command.arg(flag).arg(fixture_file(file));
        }
        match command.output() {
            Ok(out) if out.status.success() => {
                return serde_json::from_slice(&out.stdout).expect("reference prints JSON");
            }
            Ok(out) => last = String::from_utf8_lossy(&out.stderr).into_owned(),
            Err(e) => last = e.to_string(),
        }
    }
    panic!(
        "the differential test needs Python 3 on PATH to run python/reference/popgen_reference.py: {last}"
    );
}

/// The Python reference on the fixtures with every stage, A to J, at a scale
/// of the differential tests (run once per scale).
pub fn reference_all(scale: u32) -> &'static Value {
    static RUNS: [OnceLock<Value>; 2] = [OnceLock::new(), OnceLock::new()];
    let i = DIFF_SCALES
        .iter()
        .position(|&s| s == scale)
        .expect("a scale of the differential tests");
    RUNS[i].get_or_init(|| run_reference(scale))
}

/// A hash of the reference's summary.
pub fn reference_hash<'a>(reference: &'a Value, name: &str) -> &'a str {
    reference["summary"][name]
        .as_str()
        .unwrap_or_else(|| panic!("reference {name}"))
}

// --- measures ----------------------------------------------------------------

/// The weight of every person: that of their household.
pub fn person_weights(pop: &Population) -> Vec<u64> {
    pop.persons
        .household_id
        .iter()
        .map(|&h| u64::from(pop.households.hh_weight[h as usize]))
        .collect()
}

/// `p / d` rounded half away from zero, as in the specs.
pub fn rdiv(p: u128, d: u128) -> u128 {
    let (q, r) = (p / d, p % d);
    if 2 * r >= d { q + 1 } else { q }
}

/// Σ weight × amount over all records.
pub fn weighted_sum(weights: &[u64], amounts: &[u64]) -> u128 {
    assert_eq!(weights.len(), amounts.len(), "one amount per record");
    weights
        .iter()
        .zip(amounts)
        .map(|(&w, &a)| u128::from(w) * u128::from(a))
        .sum()
}

/// A quotient of two totals, for the criteria that are stated as ratios.
pub fn ratio(num: u128, den: u128) -> f64 {
    assert!(den > 0, "ratio over nothing");
    num as f64 / den as f64
}

/// The value at or below which `share_ppm` millionths of the weight lie;
/// `pairs` are (value, weight).
pub fn weighted_quantile(pairs: &[(u64, u64)], share_ppm: u64) -> u64 {
    let mut pairs = pairs.to_vec();
    pairs.sort_unstable();
    let total: u128 = pairs.iter().map(|&(_, w)| u128::from(w)).sum();
    let mut seen = 0u128;
    for &(value, w) in &pairs {
        seen += u128::from(w);
        if seen * u128::from(U) >= u128::from(share_ppm) * total {
            return value;
        }
    }
    pairs.last().expect("somebody to rank").0
}

/// A bill is paid: `paid` is at least `bill` and exceeds it by less than 0.001%.
pub fn assert_pays(what: &str, paid: u128, bill: u128) {
    assert!(paid >= bill, "{what}: {paid} bani paid, the bill is {bill}");
    assert!(
        (paid - bill) * 100_000 < bill,
        "{what}: {paid} bani paid, 0.001% or more above the bill of {bill}"
    );
}

/// A total equals its target within 0.001%.
pub fn assert_total(what: &str, got: u128, target: u128) {
    assert!(
        got.abs_diff(target) * 100_000 <= target,
        "{what}: {got} bani, more than 0.001% from the target of {target}"
    );
}

/// A column one record shorter: that of another population.
pub fn shorter<T: Clone>(column: &[T]) -> Vec<T> {
    column[..column.len() - 1].to_vec()
}
