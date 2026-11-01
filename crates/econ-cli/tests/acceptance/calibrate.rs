//! Acceptance tests for AC-CAL-04 to AC-CAL-06 and AC-CAL-08 (spec:
//! game/calibration, "The report"): `econ-cli calibrate` run as a program on
//! the Romania scenario at 1:1000, its `report.json` and `report.md`, and
//! its exit codes. AC-CAL-01 to 03 and 07 are econ-core's
//! (`crates/econ-core/tests/acceptance/calibration.rs`).
//!
//! Written before the code against the test writer's skeleton of
//! `calibrate` (ADR-0014; its readings are in the doc comment of
//! `calibrate` in `src/main.rs`): the baseline is built from the committed
//! fixtures at `--sample-scale`, seed 42, under the law of its folder;
//! `report.json` is `{"runs": [Report, …], "timing": {…}}` with
//! `econ_core::calibration::Report` serialised by serde; `--fail-on-day D`
//! stops the run on day D (`StartEdit::FailChecksOn`).

use std::path::{Path, PathBuf};
use std::process::Command;

use econ_core::calibration::{Report, RunConfig, RunInfo, Stopped, evaluate, read_targets};
use econ_core::world::{World, WorldParams};
use econ_scenario::{BuildParams, build, read_inputs, write};
use serde_json::Value;

/// The seed of the quiet baseline.
const SEED: u64 = 42;
/// A band no value leaves.
const WIDE: [i64; 2] = [-4_000_000_000_000_000_000, 4_000_000_000_000_000_000];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixtures() -> PathBuf {
    repo_root().join("python/pipeline/fixtures")
}

fn io_table() -> PathBuf {
    fixtures().join("io_table2021_ro.json")
}

/// A folder under the tests' temporary directory, emptied and created.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("econ-cli-calibrate")
        .join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clean the test folder");
    }
    std::fs::create_dir_all(&dir).expect("create the test folder");
    dir
}

/// One core target of a test file.
fn target(id: &str, indicator: &str, metric: &str, window: [u32; 2], band: [i64; 2]) -> String {
    format!(
        "[[target]]\nid = \"{id}\"\nindicator = \"{indicator}\"\nkey = \"national\"\n\
         metric = \"{metric}\"\nwindow = [{}, {}]\nband = [{}, {}]\nconfig = \"core\"\n\
         source = \"test\"\n\n",
        window[0], window[1], band[0], band[1]
    )
}

/// The test targets of a 3-year run, every band wide unless `off` gives
/// one target's band: each metric kind once, and a 50-year target the
/// 3-year report leaves out.
fn three_year_targets(off: Option<(&str, [i64; 2])>) -> String {
    let band = |id: &str| match off {
        Some((o, b)) if o == id => b,
        _ => WIDE,
    };
    let mut t = String::new();
    t += &target(
        "T-UNEMP",
        "labour.unemployment_rate",
        "mean",
        [12, 35],
        band("T-UNEMP"),
    );
    t += &target(
        "T-INFL",
        "prices.inflation_yoy",
        "at",
        [24, 35],
        band("T-INFL"),
    );
    t += &target("T-FXMIN", "fx.eur_lei", "min", [12, 35], band("T-FXMIN"));
    t += &target(
        "T-FXDRIFT",
        "fx.eur_lei",
        "drift",
        [12, 35],
        band("T-FXDRIFT"),
    );
    t += &target(
        "T-GDPTREND",
        "gdp.nominal",
        "trend",
        [12, 35],
        band("T-GDPTREND"),
    );
    t += &target(
        "T-DEFICIT",
        "budget.deficit",
        "ratio(gdp.nominal)",
        [12, 35],
        band("T-DEFICIT"),
    );
    t += &target(
        "T-DEBT",
        "debt.stock",
        "explained(budget.deficit)",
        [0, 35],
        band("T-DEBT"),
    );
    t += &target("T-RUN", "gdp.nominal", "reached", [0, 35], [1, 1]);
    t += &target("T-LONG", "gdp.nominal", "reached", [0, 599], [1, 1]);
    t
}

/// The ids of [`three_year_targets`] a 3-year report holds.
const THREE_YEAR_IDS: [&str; 8] = [
    "T-UNEMP",
    "T-INFL",
    "T-FXMIN",
    "T-FXDRIFT",
    "T-GDPTREND",
    "T-DEFICIT",
    "T-DEBT",
    "T-RUN",
];

/// What a run of `econ-cli calibrate` left.
struct Run {
    code: i32,
    json: Option<Value>,
    md: Option<Vec<u8>>,
    stderr: String,
}

/// Run `econ-cli calibrate --targets FILE --sample-scale 1000 --years Y
/// --config core --out DIR` with the test's fixtures and `extra` arguments.
fn calibrate(targets: &Path, out: &Path, years: u32, extra: &[&str]) -> Run {
    let o = Command::new(env!("CARGO_BIN_EXE_econ-cli"))
        .arg("calibrate")
        .arg("--targets")
        .arg(targets)
        .args(["--sample-scale", "1000", "--years", &years.to_string()])
        .args(["--config", "core", "--out"])
        .arg(out)
        .arg("--fixtures")
        .arg(fixtures())
        .arg("--io-table")
        .arg(io_table())
        .args(extra)
        .output()
        .expect("econ-cli runs");
    let json = std::fs::read_to_string(out.join("report.json"))
        .ok()
        .map(|t| serde_json::from_str(&t).expect("report.json is JSON"));
    Run {
        code: o.status.code().expect("an exit code, not a signal"),
        json,
        md: std::fs::read(out.join("report.md")).ok(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

/// Write a targets file into `dir`.
fn targets_file(dir: &Path, text: &str) -> PathBuf {
    let p = dir.join("targets.toml");
    std::fs::write(&p, text).expect("write the targets");
    p
}

/// The core run's report of a `report.json`.
fn core_run(json: &Value) -> &Value {
    let runs = json["runs"].as_array().expect("report.json has runs");
    assert_eq!(runs.len(), 1, "one configuration run");
    assert_eq!(runs[0]["config"], "core", "the core configuration");
    &runs[0]
}

/// The report's entry of target `id`.
fn entry<'a>(run: &'a Value, id: &str) -> &'a Value {
    run["targets"]
        .as_array()
        .expect("a run has targets")
        .iter()
        .find(|t| t["id"] == id)
        .unwrap_or_else(|| panic!("target {id}: not in report.json"))
}

/// A `report.json` without its `timing`.
fn without_timing(json: &Value) -> Value {
    let mut v = json.clone();
    v.as_object_mut()
        .expect("report.json is an object")
        .remove("timing");
    v
}

/// The quiet baseline run directly, as the report runs it: the Romania
/// scenario at 1:1000, seed 42, under the law of its folder, spec defaults,
/// no command, to the close of month 35, stopping at a failed check; its
/// targets evaluated on the world's history.
fn direct_report(targets: &Path, dir: &Path) -> Report {
    let inputs = read_inputs(&fixtures()).expect("the fixtures read");
    let scenario = build(
        &inputs,
        &BuildParams {
            sample_scale: 1000,
            rng_seed: SEED,
        },
    )
    .expect("the scenario builds");
    let folder = dir.join("scenario");
    write(&scenario, &inputs, &folder).expect("the scenario writes");
    let start = econ_io::read_start_state(&folder, &io_table()).expect("the scenario reads");
    let law =
        econ_io::commands::read_scenario_law(&folder, &econ_core::levers::Registry::committed())
            .unwrap_or_else(|(line, r)| panic!("law.json line {line}: {}", r.message));
    let mut w = World::with_law(
        start,
        law.registry,
        law.law,
        WorldParams::spec_defaults(SEED),
    )
    .expect("the world builds");
    let mut stopped = None;
    while w.months().len() < 36 {
        let r = w.step(&[]).expect("a day steps");
        if !r.checks_ok || !r.failed.is_empty() {
            stopped = Some(Stopped {
                month: u32::try_from(w.months().len()).expect("a month"),
                reason: format!("{:?}", r.failed),
            });
            break;
        }
    }
    let targets = read_targets(targets).expect("the targets read");
    evaluate(
        &targets,
        w.stats(),
        &RunInfo {
            config: RunConfig::Core,
            sample_scale: 1000,
            years: 3,
            stopped,
        },
    )
}

// --- AC-CAL-04 -------------------------------------------------------------------

/// **AC-CAL-04** `[sim]` `econ-cli calibrate --years 3 --config core` at
/// 1:1000 writes `report.json` and `report.md`; their values equal the
/// evaluation of the registry history of a direct run of the same baseline;
/// two runs give byte-identical `report.md` and equal `report.json` apart
/// from `timing`.
///
/// Readings: a test targets file with each metric kind (every band wide)
/// and one 50-year target, which a 3-year report leaves out. "Their values
/// equal": each target of `report.json` has the id, value, band, status, in
/// band and how far out of `evaluate` on a direct run's history (the same
/// targets, in the same order), and `report.md` names every target. The two
/// runs write to the same folder one after the other.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a run of years: cargo test --release -p econ-cli --features calibration --test acceptance calibrate::"
)]
fn ac_cal_04() {
    let dir = fresh_dir("ac_cal_04");
    let targets = targets_file(&dir, &three_year_targets(None));
    let out = dir.join("out");

    let first = calibrate(&targets, &out, 3, &[]);
    assert!(
        first.code == 0 || first.code == 1,
        "exit {}: {}",
        first.code,
        first.stderr
    );
    let json = first.json.expect("report.json written");
    let md = first.md.expect("report.md written");
    let md_text = String::from_utf8(md.clone()).expect("report.md is UTF-8");

    let direct = direct_report(&targets, &dir);
    let run = core_run(&json);
    let got: Vec<&Value> = run["targets"].as_array().expect("targets").iter().collect();
    assert_eq!(
        got.len(),
        direct.targets.len(),
        "the report's targets against the evaluation's"
    );
    let ids: Vec<&str> = direct.targets.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(
        ids, THREE_YEAR_IDS,
        "the 3-year targets, in the file's order"
    );
    for (j, want) in got.iter().zip(&direct.targets) {
        let id = &want.id;
        assert_eq!(j["id"], id.as_str(), "the file's order");
        assert_eq!(j["value"].as_i64(), want.value, "{id}: value");
        assert_eq!(
            j["band"],
            serde_json::json!([want.band[0], want.band[1]]),
            "{id}: band"
        );
        assert_eq!(j["in_band"].as_bool(), Some(want.in_band), "{id}: in band");
        assert_eq!(
            j["status"],
            serde_json::to_value(want.status).expect("a status"),
            "{id}: status"
        );
        match (j["how_far_out"].as_f64(), want.how_far_out) {
            (Some(a), Some(b)) => assert!(
                (a - b).abs() <= 1e-12 * b.abs().max(1.0),
                "{id}: how far out {a} against {b}"
            ),
            (None, None) => {}
            (a, b) => panic!("{id}: how far out {a:?} against {b:?}"),
        }
        assert!(md_text.contains(id.as_str()), "report.md names {id}");
    }
    assert_eq!(
        run["stopped"].is_null(),
        direct.stopped.is_none(),
        "both runs stop or neither"
    );
    assert!(
        !md_text.contains("T-LONG"),
        "the 50-year target is left out"
    );

    let second = calibrate(&targets, &out, 3, &[]);
    assert_eq!(second.code, first.code, "the same exit code");
    assert_eq!(
        second.md.as_deref(),
        Some(md.as_slice()),
        "report.md byte-identical"
    );
    assert_eq!(
        without_timing(&second.json.expect("report.json written")),
        without_timing(&json),
        "report.json equal apart from timing"
    );
}

// --- AC-CAL-05 -------------------------------------------------------------------

/// **AC-CAL-05** `[sim]` Exit codes: a targets file whose bands all contain
/// the 3-year values exits 0; moving one band off exits 1 and lists that
/// target first in `report.md`; an unreadable targets file exits 2.
///
/// Readings: the test targets of AC-CAL-04 with wide bands (and `reached`'s
/// `[1, 1]`) contain the 3-year values; "moving one band off" sets the
/// unemployment target's band to `[v + 1, v + 2]`, `v` its value in the
/// first report. "First in `report.md`": of the targets' ids, its first
/// appears before any other's. Unreadable: a file that does not exist, and
/// one that is not TOML.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a run of years: cargo test --release -p econ-cli --features calibration --test acceptance calibrate::"
)]
fn ac_cal_05() {
    let dir = fresh_dir("ac_cal_05");
    let inside = targets_file(&dir, &three_year_targets(None));
    let ok = calibrate(&inside, &dir.join("in"), 3, &[]);
    assert_eq!(ok.code, 0, "every band holds its value: {}", ok.stderr);
    let json = ok.json.expect("report.json written");
    let v = entry(core_run(&json), "T-UNEMP")["value"]
        .as_i64()
        .expect("the unemployment target has a value");

    let off_dir = fresh_dir("ac_cal_05_off");
    let off = targets_file(
        &off_dir,
        &three_year_targets(Some(("T-UNEMP", [v + 1, v + 2]))),
    );
    let out = calibrate(&off, &off_dir.join("out"), 3, &[]);
    assert_eq!(out.code, 1, "one band moved off: {}", out.stderr);
    let json = out.json.expect("report.json written");
    let moved = entry(core_run(&json), "T-UNEMP");
    assert_eq!(moved["in_band"], false, "the moved band is out");
    assert_eq!(moved["status"], "out");
    let md = String::from_utf8(out.md.expect("report.md written")).expect("UTF-8");
    let first = THREE_YEAR_IDS
        .iter()
        .filter_map(|id| md.find(id).map(|at| (at, *id)))
        .min()
        .expect("report.md names the targets");
    assert_eq!(
        first.1, "T-UNEMP",
        "the target out comes first in report.md"
    );

    let missing = calibrate(
        &dir.join("no_such_targets.toml"),
        &dir.join("missing"),
        3,
        &[],
    );
    assert_eq!(missing.code, 2, "a targets file that does not exist");
    let garbage = fresh_dir("ac_cal_05_garbage");
    let bad = targets_file(&garbage, "[[target]\nid = ");
    let bad_run = calibrate(&bad, &garbage.join("out"), 3, &[]);
    assert_eq!(bad_run.code, 2, "a targets file that is not TOML");
}

// --- AC-CAL-06 -------------------------------------------------------------------

/// **AC-CAL-06** `[sim]` A run that stops is reported: with a check made to
/// fail on a chosen day (a test edit of the start), the report names the
/// month it stopped, marks S-01 out and every target whose window it did not
/// reach "not reached", and still exits 1 with both files written.
///
/// Readings: the test edit is `--fail-on-day 45` (`StartEdit::FailChecksOn`),
/// 15 January 2022, month 1; month 0 is closed. S-01 is the spec's (any
/// indicator, `reached` over the whole run, band `[1, 1]`) in a 3-year
/// targets file; a target on month 0 is reached and in band, those whose
/// window ends in month 1 or later are "not reached". The report names the
/// month in `report.json` (`stopped.month`) and says in `report.md` that the
/// run stopped.
#[test]
fn ac_cal_06() {
    let dir = fresh_dir("ac_cal_06");
    let mut text = String::new();
    text += &target("S-01", "gdp.nominal", "reached", [0, 35], [1, 1]);
    text += &target("T-EARLY", "gdp.nominal", "at", [0, 0], WIDE);
    text += &target("T-MONTH1", "gdp.nominal", "at", [1, 1], WIDE);
    text += &target("T-LATE", "labour.unemployment_rate", "mean", [12, 35], WIDE);
    let targets = targets_file(&dir, &text);
    let out = dir.join("out");
    let r = calibrate(&targets, &out, 3, &["--fail-on-day", "45"]);
    assert_eq!(r.code, 1, "a run that stops exits 1: {}", r.stderr);
    let json = r.json.expect("report.json written");
    let md = String::from_utf8(r.md.expect("report.md written")).expect("UTF-8");

    let run = core_run(&json);
    assert_eq!(
        run["stopped"]["month"].as_u64(),
        Some(1),
        "the run stopped in month 1"
    );
    let s01 = entry(run, "S-01");
    assert_eq!(s01["status"], "out", "S-01 is out");
    assert_eq!(s01["in_band"], false);
    assert_eq!(s01["value"].as_i64(), Some(0), "reached gives 0");
    let early = entry(run, "T-EARLY");
    assert_eq!(early["status"], "in_band", "month 0 was closed");
    for id in ["T-MONTH1", "T-LATE"] {
        let t = entry(run, id);
        assert_eq!(t["status"], "not_reached", "{id}");
        assert_eq!(t["in_band"], false, "{id}");
    }
    assert!(
        md.to_lowercase().contains("stop"),
        "report.md says where the run stopped"
    );
}

// --- AC-CAL-08 -------------------------------------------------------------------

/// **AC-CAL-08** `[bench]` The 10-year core report at 1:1000 takes at most 60
/// seconds in a release build (today's run: about 7 seconds a decade).
///
/// Readings: the committed `calibration/targets.toml`, timed once from the
/// program's start to its exit (the whole report: the build of the
/// scenario, the run, the evaluation and both files). It exits 0 or 1, not
/// 2. A time budget means nothing in a debug build: the test runs in release
/// builds only.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build time budget: cargo test --release -p econ-cli --features calibration --test acceptance calibrate::"
)]
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
fn ac_cal_08() {
    let dir = fresh_dir("ac_cal_08");
    let targets = repo_root().join("calibration/targets.toml");
    let started = std::time::Instant::now();
    let r = calibrate(&targets, &dir.join("out"), 10, &[]);
    let secs = started.elapsed().as_secs_f64();
    assert!(r.code == 0 || r.code == 1, "exit {}: {}", r.code, r.stderr);
    assert!(r.json.is_some() && r.md.is_some(), "both files written");
    assert!(
        secs <= 60.0,
        "the 10-year core report took {secs:.1} s (more than 60)"
    );
}
