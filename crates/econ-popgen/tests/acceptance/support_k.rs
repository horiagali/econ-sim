//! Shared by the acceptance tests of population stage K (AC-POPK-*): its
//! margin fixture, the stage on the fixture population, and the Python
//! reference with stages A to C and K.
//!
//! The fixture population, stage C and the golden-file helpers are those of
//! `support.rs` and `support_fj.rs`. Stage K reads nothing from stages D to J.

// Helpers for the tests of one stage; not every one is used by every test.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use econ_popgen::{
    ENROLMENT_LEVELS, INACTIVE_KINDS, InactivityEnrolmentMargins, InactivityParams, OPEN,
    PersonInactivityEnrolment, assign_inactivity_enrolment,
};
use serde_json::{Value, json};

use super::support::{DIFF_SCALES, SEED};
use super::support_fj::{Cache, attrs, fixture, num, pop, slot, table};

pub const INACTIVITY_ENROLMENT_GOLDEN: &str = "popgen_ro_census2021_inactivity_enrolment.hashes";
pub const ENROLMENT_FIXTURE: &str = "inactivity_enrolment2021_ro.json";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture_file(name: &str) -> PathBuf {
    repo_root().join("python/pipeline/fixtures").join(name)
}

/// Ranges `[from, to)` in years; `null` is the open end of the last.
fn ranges(v: &Value) -> Vec<(u16, u16)> {
    v.as_array()
        .expect("array of pairs")
        .iter()
        .map(|p| {
            let year = |x: &Value| u16::try_from(num(x)).expect("age in years");
            let to = if p[1].is_null() { OPEN } else { year(&p[1]) };
            (year(&p[0]), to)
        })
        .collect()
}

/// The inactivity-and-enrolment margin fixture (stage K).
pub fn enrolment_margins() -> &'static InactivityEnrolmentMargins {
    static M: OnceLock<InactivityEnrolmentMargins> = OnceLock::new();
    M.get_or_init(|| {
        let v = fixture(ENROLMENT_FIXTURE);
        assert_eq!(v["sexes"], json!(["F", "M"]), "sex order");
        assert_eq!(
            v["inactive_kinds"],
            json!(["homemaker", "disabled", "other"]),
            "kind order"
        );
        assert_eq!(
            v["enrolment_levels"],
            json!([
                "pre_school",
                "primary",
                "lower_secondary",
                "upper_secondary",
                "post_secondary",
                "tertiary",
                "master_doctorate"
            ]),
            "level order"
        );
        let m = InactivityEnrolmentMargins {
            age_bands: ranges(&v["age_bands"]),
            enrolment_ages: ranges(&v["enrolment_ages"]),
            inactive_kind: table(&v["inactive_kind"]),
            enrolment: table(&v["enrolment"]),
            population: table(&v["population"]),
        };
        let (n_b, n_g) = (m.age_bands.len(), m.enrolment_ages.len());
        assert_eq!(
            m.inactive_kind.len(),
            2 * n_b * INACTIVE_KINDS,
            "kind table"
        );
        assert_eq!(
            m.enrolment.len(),
            2 * n_g * ENROLMENT_LEVELS,
            "enrolment table"
        );
        assert_eq!(m.population.len(), 2 * n_g, "population table");
        m
    })
}

/// Stage K on the fixture population at a cached scale, with any seed and link.
pub fn run(scale: u32, seed: u64, link_ppm: u32) -> PersonInactivityEnrolment {
    let params = InactivityParams {
        homemaker_child_link_ppm: link_ppm,
        ..InactivityParams::new(seed)
    };
    assign_inactivity_enrolment(pop(scale), attrs(scale), enrolment_margins(), &params)
        .expect("fixture population gets kinds and enrolment")
}

/// The default `homemaker_child_link`, in millionths.
pub fn default_link() -> u32 {
    InactivityParams::new(SEED).homemaker_child_link_ppm
}

/// Stage K with the default parameters at a cached scale (run once).
pub fn inactivity(scale: u32) -> &'static PersonInactivityEnrolment {
    static K: Cache<PersonInactivityEnrolment> = [const { OnceLock::new() }; 4];
    K[slot(scale)].get_or_init(|| run(scale, SEED, default_link()))
}

fn run_reference(scale: u32) -> Value {
    let script = repo_root().join("python/reference/popgen_reference.py");
    let mut last = String::from("no Python found");
    for python in ["python3", "python"] {
        let out = Command::new(python)
            .arg(&script)
            .arg(fixture_file("census2021_margins_ro.json"))
            .args(["--sample-scale", &scale.to_string()])
            .args(["--seed", &SEED.to_string()])
            .arg("--attributes")
            .arg(fixture_file("census2021_edu_activity_ro.json"))
            .arg("--enrolment")
            .arg(fixture_file(ENROLMENT_FIXTURE))
            .output();
        match out {
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

/// The Python reference on the fixtures with stages A to C and K, at a scale
/// of the differential tests (run once per scale).
pub fn reference_k(scale: u32) -> &'static Value {
    static RUNS: [OnceLock<Value>; 2] = [OnceLock::new(), OnceLock::new()];
    let i = DIFF_SCALES
        .iter()
        .position(|&s| s == scale)
        .expect("a scale of the differential tests");
    RUNS[i].get_or_init(|| run_reference(scale))
}

/// Index of the range `[from, to)` that holds `years`.
pub fn range_of(ranges: &[(u16, u16)], years: u16) -> usize {
    ranges
        .iter()
        .position(|&(lo, hi)| years >= lo && (hi == OPEN || years < hi))
        .expect("age inside a range")
}
