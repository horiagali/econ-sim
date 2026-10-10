//! Shared by the acceptance tests of the population stages C, D and E
//! (AC-POPA-*, AC-POPJ-*, AC-POPH-*): the margin fixtures, the tolerance check
//! by expected synthetic records, the golden files and the Python reference.

// Each stage's tests use a part of this file, and a stage can be compiled alone.
#![allow(dead_code)]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use econ_popgen::{
    AttrParams, AttributeMargins, EDU_GROUPS, EDU_LEVELS, GenParams, HousingMargins,
    INDUSTRY_GROUPS, JobMargins, KINDS, Margins, OCCUPATIONS, OPEN, PersonAttributes, Population,
    SIZE_GROUPS, TENURES, assign_attributes, generate,
};
use serde_json::{Value, json};

/// The seed of the fixture population and of every stage.
pub const SEED: u64 = 42;
/// "The three scales".
pub const SCALES: [u32; 3] = [1000, 100, 10];
/// The seed of "a different seed" (AC-*-05).
pub const OTHER_SEED: u64 = 7;
/// Scales of the differential tests (AC-*-07).
pub const DIFF_SCALES: [u32; 2] = [1000, 200];
/// Tolerance in percent by expected synthetic records behind a cell,
/// largest cells first. Smaller cells than the last entry are not checked.
pub type Tolerances = [(u64, u64); 3];

pub const STATE_GOLDEN: &str = "popgen_ro_census2021.hashes";
pub const ATTRIBUTES_GOLDEN: &str = "popgen_ro_census2021_attributes.hashes";
pub const JOBS_GOLDEN: &str = "popgen_ro_census2021_jobs.hashes";
pub const HOUSING_GOLDEN: &str = "popgen_ro_census2021_housing.hashes";

const ATTRIBUTES_FIXTURE: &str = "census2021_edu_activity_ro.json";
const JOBS_FIXTURE: &str = "census2021_jobs_ro.json";
const HOUSING_FIXTURE: &str = "census2021_housing_ro.json";

const MARGINS_FIXTURE: &str = "census2021_margins_ro.json";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture_file(name: &str) -> PathBuf {
    repo_root().join("python/pipeline/fixtures").join(name)
}

fn as_u64(v: &Value) -> u64 {
    v.as_u64().expect("non-negative integer")
}

fn flatten(v: &Value, out: &mut Vec<u64>) {
    match v {
        Value::Array(items) => items.iter().for_each(|i| flatten(i, out)),
        other => out.push(as_u64(other)),
    }
}

fn pairs(v: &Value) -> Vec<(u16, u16)> {
    v.as_array()
        .expect("array of pairs")
        .iter()
        .map(|p| {
            let lo = u16::try_from(as_u64(&p[0])).expect("small");
            let hi = if p[1].is_null() {
                OPEN
            } else {
                u16::try_from(as_u64(&p[1])).expect("small")
            };
            (lo, hi)
        })
        .collect()
}

/// The county margin fixture (stages A and B).
pub fn margins() -> &'static Margins {
    static M: OnceLock<Margins> = OnceLock::new();
    M.get_or_init(|| {
        let v = fixture(MARGINS_FIXTURE);
        assert_eq!(v["sexes"], json!(["F", "M"]), "sex order");
        Margins {
            counties: strings(&v["counties"]),
            age_bands: pairs(&v["age_bands"]),
            size_classes: pairs(&v["size_classes"]),
            persons_private: table(&v, "persons_private"),
            persons_collective: table(&v, "persons_collective"),
            households: table(&v, "households"),
        }
    })
}

/// The fixture population at one of the three scales (generated once).
pub fn population(scale: u32) -> &'static Population {
    static POPS: [OnceLock<Population>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let i = SCALES
        .iter()
        .position(|&s| s == scale)
        .expect("one of the three scales");
    POPS[i].get_or_init(|| {
        generate(margins(), &GenParams::new(scale, SEED)).expect("fixture generates")
    })
}

/// A column of the Python reference's output.
pub fn column(v: &Value, name: &str) -> Vec<u64> {
    v[name]
        .as_array()
        .unwrap_or_else(|| panic!("reference column {name}"))
        .iter()
        .map(as_u64)
        .collect()
}

fn fixture(name: &str) -> Value {
    let path = fixture_file(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("margin fixture {}: {e}", path.display()));
    serde_json::from_str(&text).expect("valid JSON")
}

fn table(v: &Value, name: &str) -> Vec<u64> {
    let mut out = Vec::new();
    flatten(&v[name], &mut out);
    out
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("array of names")
        .iter()
        .map(|s| s.as_str().expect("name").to_string())
        .collect()
}

fn len(v: &Value) -> usize {
    v.as_array().expect("array").len()
}

/// The education-and-activity margin fixture (stage C).
pub fn attribute_margins() -> &'static AttributeMargins {
    static M: OnceLock<AttributeMargins> = OnceLock::new();
    M.get_or_init(|| {
        let v = fixture(ATTRIBUTES_FIXTURE);
        assert_eq!(v["sexes"], json!(["F", "M"]), "sex order");
        assert_eq!(
            v["activities"],
            json!([
                "child",
                "in_education",
                "employed",
                "unemployed",
                "retired",
                "inactive_other"
            ]),
            "activity order"
        );
        assert_eq!(
            v["statuses"],
            json!(["employed", "unemployed", "inactive"]),
            "status order"
        );
        assert_eq!(len(&v["edu_levels"]), EDU_LEVELS, "education levels");
        let regions = strings(&v["regions"]);
        let region_of_county = margins()
            .counties
            .iter()
            .map(|county| {
                let region = v["region_of_county"][county]
                    .as_str()
                    .unwrap_or_else(|| panic!("county {county} has no region in the fixture"));
                let index = regions
                    .iter()
                    .position(|r| r == region)
                    .unwrap_or_else(|| panic!("region {region} is not listed"));
                u8::try_from(index).expect("few regions")
            })
            .collect();
        AttributeMargins {
            regions,
            region_of_county,
            age_bands: pairs(&v["age_bands"]),
            years_of_age: u16::try_from(len(&v["activity"][0][0])).expect("years of age"),
            activity: table(&v, "activity"),
            education: table(&v, "education"),
        }
    })
}

/// The jobs margin fixture (stage D). The county-to-region map is the one of
/// the education-and-activity fixture.
pub fn job_margins() -> &'static JobMargins {
    static M: OnceLock<JobMargins> = OnceLock::new();
    M.get_or_init(|| {
        let v = fixture(JOBS_FIXTURE);
        let ea = attribute_margins();
        assert_eq!(v["sexes"], json!(["F", "M"]), "sex order");
        assert_eq!(
            strings(&v["regions"]),
            ea.regions,
            "regions of both fixtures"
        );
        assert_eq!(len(&v["kinds"]), KINDS, "statuses in employment");
        assert_eq!(len(&v["occupations"]), OCCUPATIONS, "occupations");
        assert_eq!(
            len(&v["industry_groups"]),
            INDUSTRY_GROUPS,
            "industry groups"
        );
        assert_eq!(len(&v["edu_groups"]), EDU_GROUPS, "education groups");
        let groups: Vec<u8> = table(&v, "edu_group_of_level")
            .into_iter()
            .map(|g| u8::try_from(g).expect("education group"))
            .collect();
        JobMargins {
            regions: ea.regions.clone(),
            region_of_county: ea.region_of_county.clone(),
            age_bands: pairs(&v["age_bands"]),
            by_industry_group: table(&v, "by_industry_group"),
            by_occupation: table(&v, "by_occupation"),
            pattern_edu_occupation: table(&v, "pattern_edu_occupation"),
            pattern_occupation_industry_group: table(&v, "pattern_occupation_industry_group"),
            edu_group_of_level: groups.try_into().expect("one group per education level"),
        }
    })
}

/// The housing margin fixture (stage E).
pub fn housing_margins() -> &'static HousingMargins {
    static M: OnceLock<HousingMargins> = OnceLock::new();
    M.get_or_init(|| {
        let v = fixture(HOUSING_FIXTURE);
        assert_eq!(len(&v["size_groups"]), SIZE_GROUPS, "size groups");
        assert_eq!(len(&v["tenures"]), TENURES, "tenures");
        HousingMargins {
            counties: strings(&v["counties"]),
            locality_classes: u8::try_from(len(&v["locality_classes"])).expect("few classes"),
            urban_from_class: u8::try_from(as_u64(&v["urban_from_class"])).expect("a class"),
            age_group_from: table(&v, "age_group_from")
                .into_iter()
                .map(|a| u16::try_from(a).expect("age in years"))
                .collect(),
            persons_by_locality: table(&v, "persons_by_locality"),
            households_by_tenure: table(&v, "households_by_tenure"),
        }
    })
}

/// The fixture population at any scale: cached at the three scales, generated otherwise.
pub fn population_at(scale: u32) -> Cow<'static, Population> {
    if SCALES.contains(&scale) {
        Cow::Borrowed(population(scale))
    } else {
        Cow::Owned(generate(margins(), &GenParams::new(scale, SEED)).expect("fixture generates"))
    }
}

/// Stage C on the fixture population at one of the three scales (run once).
pub fn attributes(scale: u32) -> &'static PersonAttributes {
    static ATTRS: [OnceLock<PersonAttributes>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let i = SCALES
        .iter()
        .position(|&s| s == scale)
        .expect("one of the three scales");
    ATTRS[i].get_or_init(|| {
        assign_attributes(
            population(scale),
            attribute_margins(),
            &AttrParams::new(SEED),
        )
        .expect("fixture population gets attributes")
    })
}

/// Stage C on the fixture population at any scale.
pub fn attributes_at(scale: u32) -> Cow<'static, PersonAttributes> {
    if SCALES.contains(&scale) {
        Cow::Borrowed(attributes(scale))
    } else {
        Cow::Owned(
            assign_attributes(
                &population_at(scale),
                attribute_margins(),
                &AttrParams::new(SEED),
            )
            .expect("fixture population gets attributes"),
        )
    }
}

/// Index of the age band `[from, to)` that holds `years`.
pub fn band_of_year(bands: &[(u16, u16)], years: u16) -> usize {
    bands
        .iter()
        .position(|&(lo, hi)| years >= lo && (hi == OPEN || years < hi))
        .expect("age inside a band")
}

/// 0 for an age band of children (0–14), 1 for working ages (15–64), 2 for 65 or more.
pub fn broad_age(bands: &[(u16, u16)], band: usize) -> usize {
    match bands[band].0 {
        0..=14 => 0,
        15..=64 => 1,
        _ => 2,
    }
}

/// Notes a failure if `got` is further from the census count `want` than the
/// tolerance for the number of synthetic records expected behind the cell.
pub fn check(
    tolerances: &Tolerances,
    cell: &str,
    scale: u32,
    got: u64,
    want: u64,
    failures: &mut Vec<String>,
) {
    let records = want / u64::from(scale);
    let Some(&(_, pct)) = tolerances
        .iter()
        .find(|&&(min_records, _)| records >= min_records)
    else {
        return; // too few expected records: not checked
    };
    if got.abs_diff(want) * 100 > want * pct {
        failures.push(format!(
            "1:{scale} {cell}: got {got}, census {want} (tolerance {pct}%)"
        ));
    }
}

/// Checks one family of margins. `got` and `want` are row-major tables of
/// shape `dims`; `key` names the margin cell that a table cell belongs to, and
/// every margin cell (the sum of its table cells) is checked.
// One call per margin family reads best with everything spelled out.
#[allow(clippy::too_many_arguments)]
pub fn check_family(
    tolerances: &Tolerances,
    family: &str,
    scale: u32,
    dims: &[usize],
    got: &[u64],
    want: &[u64],
    key: impl Fn(&[usize]) -> Vec<usize>,
    failures: &mut Vec<String>,
) {
    let cells: usize = dims.iter().product();
    assert_eq!(want.len(), cells, "{family}: census table and its shape");
    assert_eq!(got.len(), cells, "{family}: synthetic table and its shape");
    let mut sums: BTreeMap<Vec<usize>, (u64, u64)> = BTreeMap::new();
    let mut index = vec![0usize; dims.len()];
    for flat in 0..cells {
        let mut rest = flat;
        for d in (0..dims.len()).rev() {
            index[d] = rest % dims[d];
            rest /= dims[d];
        }
        let sum = sums.entry(key(&index)).or_default();
        sum.0 += got[flat];
        sum.1 += want[flat];
    }
    for (cell, (g, w)) in sums {
        check(
            tolerances,
            &format!("{family} {cell:?}"),
            scale,
            g,
            w,
            failures,
        );
    }
}

/// Fails with the list of margins outside tolerance, if any.
pub fn assert_no_failures(failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{} margins outside tolerance:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

pub fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// The lines `scale hash` of a file in `tests/golden`.
pub fn golden(file: &str) -> Vec<(u32, String)> {
    let path = repo_root().join("tests/golden").join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("golden file {}: {e}", path.display()));
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let mut parts = line.split_whitespace();
            let scale = parts.next().expect("scale").parse().expect("scale");
            (scale, parts.next().expect("hash").to_string())
        })
        .collect()
}

/// The golden hash of one scale.
pub fn golden_hash(file: &str, scale: u32) -> String {
    golden(file)
        .into_iter()
        .find(|&(s, _)| s == scale)
        .unwrap_or_else(|| panic!("{file} has no hash for 1:{scale}"))
        .1
}

/// Every hash of a golden file is reproduced, and the file covers the three scales.
pub fn assert_golden(file: &str, what: &str, hash_at: impl Fn(u32) -> u64) {
    let lines = golden(file);
    for scale in SCALES {
        assert!(
            lines.iter().any(|&(s, _)| s == scale),
            "{file} has no {what} for 1:{scale}"
        );
    }
    for (scale, want) in lines {
        assert_eq!(hex(hash_at(scale)), want, "{what} at 1:{scale}");
    }
}

fn run_reference(scale: u32) -> Value {
    let script = repo_root().join("python/reference/popgen_reference.py");
    let mut last = String::from("no Python found");
    for python in ["python3", "python"] {
        let run = Command::new(python)
            .arg(&script)
            .arg(fixture_file(MARGINS_FIXTURE))
            .args(["--sample-scale", &scale.to_string()])
            .args(["--seed", &SEED.to_string()])
            .arg("--attributes")
            .arg(fixture_file(ATTRIBUTES_FIXTURE))
            .arg("--jobs")
            .arg(fixture_file(JOBS_FIXTURE))
            .arg("--housing")
            .arg(fixture_file(HOUSING_FIXTURE))
            .output();
        match run {
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

/// The Python reference on the fixtures, all stages, at a scale of the
/// differential tests (run once per scale).
pub fn reference(scale: u32) -> &'static Value {
    static RUNS: [OnceLock<Value>; 2] = [OnceLock::new(), OnceLock::new()];
    let i = DIFF_SCALES
        .iter()
        .position(|&s| s == scale)
        .expect("a scale of the differential tests");
    RUNS[i].get_or_init(|| run_reference(scale))
}

/// A column of the Rust result equals the reference's, record for record.
pub fn assert_same_column(scale: u32, name: &str, got: &[u64], want: &[u64]) {
    assert_eq!(got.len(), want.len(), "1:{scale} {name}: number of records");
    if let Some(i) = (0..got.len()).find(|&i| got[i] != want[i]) {
        panic!(
            "1:{scale} {name}: first difference at record {i}: Rust {} vs reference {}",
            got[i], want[i]
        );
    }
}
