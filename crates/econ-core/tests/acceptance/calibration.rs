//! Acceptance tests for AC-CAL-01, 02, 03 and 07 (spec: game/calibration,
//! the targets file, the metrics and the registry series of L-09 and L-10).
//! AC-CAL-04 to 06 and 08 are econ-cli's (`crates/econ-cli/tests/acceptance/
//! calibrate.rs`).
//!
//! Written before the code against the test writer's API skeleton
//! `econ_core::calibration` (ADR-0014), behind the feature `calibration`.
//! The metrics are checked on plain monthly series (`metric_value`) and the
//! verdicts on hand-built histories (`StatsHistory::with_months`), so no
//! test here but AC-CAL-07 runs a world.

use std::collections::BTreeSet;

use econ_core::calibration::{
    Metric, NoValue, Points, Report, RunConfig, RunInfo, Status, Target, TargetConfig,
    TargetResult, evaluate, metric_value, parse_targets,
};
use econ_core::stats::StatsHistory;

use super::support::repo_root;
use super::support_m6::{NATIONAL, computed, id, meta, run_closes, world_2021_default};

// --- helpers -------------------------------------------------------------------

/// The features econ-core's `Cargo.toml` turns on from `roots`, recursively
/// (dependency features `dep:x` and `crate/feature` left out).
fn feature_closure(roots: &[&str]) -> BTreeSet<String> {
    let path = repo_root().join("crates/econ-core/Cargo.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let manifest: toml::Table = text.parse().expect("econ-core's Cargo.toml is TOML");
    let features = manifest["features"].as_table().expect("a [features] table");
    let mut on = BTreeSet::new();
    let mut todo: Vec<String> = roots.iter().map(|s| (*s).to_string()).collect();
    while let Some(f) = todo.pop() {
        if f.starts_with("dep:") || f.contains('/') || !on.insert(f.clone()) {
            continue;
        }
        let Some(list) = features.get(&f).and_then(|v| v.as_array()) else {
            panic!("feature {f}: not in econ-core's [features]");
        };
        todo.extend(list.iter().filter_map(|v| v.as_str()).map(str::to_string));
    }
    on
}

/// The features of the spec's configurations: core is econ-core's default
/// features, people adds `home_quality`, `education` and `approval`.
fn config_features(c: RunConfig) -> BTreeSet<String> {
    match c {
        RunConfig::Core => feature_closure(&["default"]),
        RunConfig::People => feature_closure(&["default", "home_quality", "education", "approval"]),
    }
}

/// The run configurations a target's `config` names.
fn runs_of(c: TargetConfig) -> Vec<RunConfig> {
    match c {
        TargetConfig::Core => vec![RunConfig::Core],
        TargetConfig::People => vec![RunConfig::People],
        TargetConfig::Both => vec![RunConfig::Core, RunConfig::People],
    }
}

/// Whether the registry has an indicator named `name`.
fn in_registry(name: &str) -> bool {
    econ_core::stats::indicators()
        .iter()
        .any(|m| m.name == name)
}

/// The targets of a TOML text; panics when it does not parse.
fn targets(text: &str) -> Vec<Target> {
    parse_targets(text).unwrap_or_else(|e| panic!("the targets parse: {}", e.message))
}

/// The points of a series from month `first`.
fn pts(first: u32, months: &[i64]) -> Points<'_> {
    Points {
        first_month: first,
        months,
    }
}

/// The value of a metric on a series from month 0 with the `others` from
/// month 0.
fn value(metric: &Metric, window: [u32; 2], series: &[i64], others: &[&[i64]]) -> i64 {
    let o: Vec<Points<'_>> = others.iter().map(|s| pts(0, s)).collect();
    metric_value(metric, window, pts(0, series), &o)
        .unwrap_or_else(|e| panic!("{metric:?} over {window:?} of {series:?}: {e:?}"))
}

/// A core run of `years` that did not stop.
fn run(years: u32) -> RunInfo {
    RunInfo {
        config: RunConfig::Core,
        sample_scale: 1000,
        years,
        stopped: None,
    }
}

/// The report's result of target `id`.
fn result<'a>(r: &'a Report, id: &str) -> &'a TargetResult {
    r.targets
        .iter()
        .find(|t| t.id == id)
        .unwrap_or_else(|| panic!("target {id}: not in the report"))
}

/// One target of a test file.
fn target_toml(
    id: &str,
    indicator: &str,
    metric: &str,
    window: [u32; 2],
    band: [i64; 2],
) -> String {
    format!(
        "[[target]]\nid = \"{id}\"\nindicator = \"{indicator}\"\nkey = \"national\"\n\
         metric = \"{metric}\"\nwindow = [{}, {}]\nband = [{}, {}]\nconfig = \"core\"\n\
         source = \"test\"\n\n",
        window[0], window[1], band[0], band[1]
    )
}

fn assert_close(got: Option<f64>, want: f64, what: &str) {
    let got = got.unwrap_or_else(|| panic!("{what}: no distance"));
    assert!(
        (got - want).abs() < 1e-9,
        "{what}: how far out is {got}, not {want}"
    );
}

// --- AC-CAL-01 -------------------------------------------------------------------

/// The spec's proposed targets (its tables, with S-05 split into S-05a and
/// S-05b on 2026-11-01).
const SPEC_TARGETS: [&str; 19] = [
    "L-01", "L-02", "L-03", "L-04", "L-05", "L-06", "L-07", "L-08", "L-09", "L-10", "S-01", "S-02",
    "S-03", "S-04", "S-05a", "S-05b", "S-06", "S-07", "S-08",
];

/// **AC-CAL-01** `[unit]` The committed `calibration/targets.toml` parses;
/// every target names a registry indicator that is active in its
/// configuration's features, a key the indicator has (or national), a known
/// metric and a band with low ≤ high; ids are unique.
///
/// Readings: "active in its configuration's features" means the indicator's
/// status is active and its `feature` (if any) is among the features of each
/// configuration the target is for (`both`: core and people), core being
/// econ-core's default features and people those plus `home_quality`,
/// `education` and `approval`, each closed over econ-core's `Cargo.toml`.
/// The series a metric names (`ratio`'s other, `explained`'s flows) are held
/// to the same rule. A known metric is one `parse_targets` accepts (the
/// file parses). Every target names a registry indicator, `reached` ones
/// included (S-01's "any" is read as "any registry indicator"). The file
/// holds the spec's proposed targets: each id of its tables starts at least
/// one target's id (a target split in parts, `L-03` into a min and a max or
/// `S-03` by decade, may carry a suffix).
#[test]
fn ac_cal_01() {
    let path = repo_root().join("calibration/targets.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let all = targets(&text);
    assert!(!all.is_empty(), "the targets file has targets");

    let mut ids = BTreeSet::new();
    for t in &all {
        assert!(ids.insert(t.id.clone()), "target id {} appears twice", t.id);
        assert!(
            t.band[0] <= t.band[1],
            "{}: band {:?} has low above high",
            t.id,
            t.band
        );
        assert!(
            t.window[0] <= t.window[1],
            "{}: window {:?} ends before it starts",
            t.id,
            t.window
        );

        let mut names = vec![t.indicator.clone()];
        match &t.metric {
            Metric::Ratio(other) => names.push(other.clone()),
            Metric::Explained(flows) => {
                assert!(!flows.is_empty(), "{}: explained names no flow", t.id);
                names.extend(flows.iter().cloned());
            }
            _ => {}
        }
        for name in &names {
            assert!(
                in_registry(name),
                "{}: {name} is not a registry indicator",
                t.id
            );
            let m = meta(name);
            assert!(
                m.status == econ_core::stats::Status::Active,
                "{}: {name} is planned, not active",
                t.id
            );
            for c in runs_of(t.config) {
                if let Some(f) = m.feature {
                    assert!(
                        config_features(c).contains(f),
                        "{}: {name} needs feature {f}, not in the {c:?} configuration",
                        t.id
                    );
                }
            }
        }

        let m = meta(&t.indicator);
        if let Some(b) = t.key.breakdown {
            assert!(
                m.breakdowns.contains(&b),
                "{}: {} has no breakdown {}",
                t.id,
                t.indicator,
                b.name()
            );
        }
    }

    for spec_id in SPEC_TARGETS {
        assert!(
            all.iter().any(|t| t.id.starts_with(spec_id)),
            "the spec's target {spec_id} is not in the targets file"
        );
    }
}

// --- AC-CAL-02 -------------------------------------------------------------------

/// **AC-CAL-02** `[unit]` Each metric on hand-built histories: `mean`, `at`,
/// `min`, `max`, `drift`, `trend`, `ratio` and `explained` give the stated
/// values (including negative values, a one-point window and rounding half
/// away from zero); a window past the history's end makes the target fail
/// with "not reached", never panics.
///
/// Readings: the metrics on plain monthly series (`metric_value`), the
/// verdicts on a hand-built `StatsHistory`. `trend` is the least-squares
/// slope against the month, times 12; a one-point window's trend is 0.
/// `explained(flows)` is (the window's last point − its first point) ×
/// 1,000,000 over the sum of the flows' points in the window; a zero sum has
/// no value, and the target is out (not "not reached"), without a panic.
/// `reached` gives 0 past the history's end, never "not reached".
#[test]
fn ac_cal_02() {
    let s = [5, -9, 3, 12, -2];
    let all = [0, 4];
    // mean: 9 / 5 = 1.8 → 2.
    assert_eq!(value(&Metric::Mean, all, &s, &[]), 2, "mean");
    assert_eq!(value(&Metric::At, all, &s, &[]), -2, "at");
    assert_eq!(value(&Metric::Min, all, &s, &[]), -9, "min");
    assert_eq!(value(&Metric::Max, all, &s, &[]), 12, "max");
    // drift: |12 − 5| = 7 against |−9 − 5| = 14.
    assert_eq!(value(&Metric::Drift, all, &s, &[]), 14, "drift");
    assert_eq!(
        value(&Metric::Drift, [2, 4], &s, &[]),
        9,
        "drift from month 2"
    );
    assert_eq!(
        value(&Metric::Drift, [0, 2], &[-5, -9, -2], &[]),
        4,
        "drift of negative values"
    );

    // Rounding half away from zero.
    assert_eq!(value(&Metric::Mean, [0, 1], &[1, 2], &[]), 2, "1.5 → 2");
    assert_eq!(
        value(&Metric::Mean, [0, 1], &[-1, -2], &[]),
        -2,
        "−1.5 → −2"
    );
    assert_eq!(value(&Metric::Mean, [0, 1], &[-3, 2], &[]), -1, "−0.5 → −1");
    assert_eq!(value(&Metric::Mean, [0, 2], &[1, 1, 2], &[]), 1, "1.33 → 1");

    // trend, per year.
    assert_eq!(
        value(&Metric::Trend, [0, 2], &[0, 12, 24], &[]),
        144,
        "12 a month"
    );
    assert_eq!(
        value(&Metric::Trend, [0, 3], &[10, 7, 4, 1], &[]),
        -36,
        "−3 a month"
    );
    // slope 0.3 a month: 3.6 a year → 4; and −3.6 → −4.
    assert_eq!(
        value(&Metric::Trend, [0, 3], &[0, 0, 0, 1], &[]),
        4,
        "3.6 → 4"
    );
    assert_eq!(
        value(&Metric::Trend, [0, 3], &[0, 0, 0, -1], &[]),
        -4,
        "−3.6 → −4"
    );
    assert_eq!(value(&Metric::Trend, [0, 3], &[7, 7, 7, 7], &[]), 0, "flat");

    // ratio, millionths: means 1 over 128 → 7,812.5 → 7,813.
    let gdp = "gdp.nominal".to_string();
    let ratio = Metric::Ratio(gdp.clone());
    assert_eq!(
        value(&ratio, [0, 1], &[1, 1], &[&[100, 156]]),
        7_813,
        "7,812.5 → 7,813"
    );
    assert_eq!(
        value(&ratio, [0, 1], &[-1, -1], &[&[100, 156]]),
        -7_813,
        "−7,812.5 → −7,813"
    );
    assert_eq!(
        value(&ratio, [0, 1], &[30, 50], &[&[100, 300]]),
        200_000,
        "40 / 200"
    );

    // explained, millionths.
    let one = Metric::Explained(vec!["budget.deficit".to_string()]);
    let two = Metric::Explained(vec![
        "budget.deficit".to_string(),
        "budget.interest".to_string(),
    ]);
    assert_eq!(
        value(&one, [0, 2], &[100, 150, 300], &[&[50, 50, 100]]),
        1_000_000,
        "a change of 200 over flows of 200"
    );
    assert_eq!(
        value(
            &two,
            [0, 2],
            &[100, 150, 300],
            &[&[10, 10, 10], &[40, 40, 90]]
        ),
        1_000_000,
        "two flows add up"
    );
    assert_eq!(
        value(&one, [0, 1], &[300, 100], &[&[-100, -300]]),
        500_000,
        "a fall of 200 over flows of −400"
    );
    assert_eq!(value(&one, [0, 1], &[0, 1], &[&[1, 2]]), 333_333, "1 / 3");
    assert_eq!(
        value(&one, [0, 1], &[0, 1], &[&[-1, -2]]),
        -333_333,
        "1 / −3"
    );
    assert_eq!(
        value(&one, [0, 1], &[0, 1], &[&[1_000_000, 1_000_000]]),
        1,
        "0.5 → 1"
    );
    assert_eq!(
        value(&one, [0, 1], &[0, -1], &[&[1_000_000, 1_000_000]]),
        -1,
        "−0.5 → −1"
    );
    assert_eq!(
        metric_value(&one, [0, 1], pts(0, &[0, 5]), &[pts(0, &[5, -5])]),
        Err(NoValue::Undefined),
        "a zero flow sum has no value"
    );

    // A one-point window.
    let s = [5, 6, 7, 8];
    let w = [2, 2];
    for (m, want) in [
        (Metric::Mean, 7),
        (Metric::At, 7),
        (Metric::Min, 7),
        (Metric::Max, 7),
        (Metric::Drift, 0),
        (Metric::Trend, 0),
        (Metric::Reached, 1),
    ] {
        assert_eq!(value(&m, w, &s, &[]), want, "{m:?} over a one-point window");
    }
    assert_eq!(
        value(&ratio, w, &s, &[&[1, 1, 14, 1]]),
        500_000,
        "ratio over a one-point window"
    );

    // A series that starts later: months 3 to 6.
    let late = pts(3, &[10, 20, 30, 40]);
    assert_eq!(
        metric_value(&Metric::Mean, [4, 5], late, &[]),
        Ok(25),
        "months 4 and 5"
    );
    assert_eq!(
        metric_value(&Metric::At, [3, 6], late, &[]),
        Ok(40),
        "month 6"
    );

    // Past the history's end.
    for m in [
        Metric::Mean,
        Metric::At,
        Metric::Min,
        Metric::Max,
        Metric::Drift,
        Metric::Trend,
        ratio.clone(),
    ] {
        let others: Vec<Points<'_>> = if matches!(m, Metric::Ratio(_)) {
            vec![pts(0, &[1, 1, 1, 1])]
        } else {
            Vec::new()
        };
        assert_eq!(
            metric_value(&m, [2, 10], pts(0, &s), &others),
            Err(NoValue::NotReached),
            "{m:?} past the end"
        );
    }
    assert_eq!(
        metric_value(&one, [2, 10], pts(0, &s), &[pts(0, &[1, 1, 1, 1])]),
        Err(NoValue::NotReached),
        "explained past the end"
    );
    assert_eq!(
        metric_value(&Metric::Reached, [0, 10], pts(0, &s), &[]),
        Ok(0),
        "reached past the end"
    );
    assert_eq!(
        metric_value(&Metric::Reached, [0, 3], pts(0, &s), &[]),
        Ok(1),
        "reached at the end"
    );

    // The verdicts on a hand-built history.
    let h = StatsHistory::default()
        .with_months(id("gdp.nominal"), NATIONAL, 0, &[100, 110, 120, 130])
        .with_months(id("budget.deficit"), NATIONAL, 0, &[5, -5, 5, -5]);
    let mut text = String::new();
    text += &target_toml("T-REACHED", "gdp.nominal", "mean", [0, 3], [0, 1_000]);
    text += &target_toml("T-PAST", "gdp.nominal", "mean", [2, 10], [0, 1_000]);
    text += &target_toml("T-RUN", "gdp.nominal", "reached", [0, 10], [1, 1]);
    text += &target_toml(
        "T-ZERO",
        "gdp.nominal",
        "explained(budget.deficit)",
        [0, 3],
        [750_000, 1_250_000],
    );
    let r = evaluate(&targets(&text), &h, &run(1));
    let reached = result(&r, "T-REACHED");
    assert_eq!(reached.value, Some(115), "mean of the four months");
    assert_eq!(reached.status, Status::InBand);
    assert!(reached.in_band);
    let past = result(&r, "T-PAST");
    assert_eq!(past.status, Status::NotReached, "a window past the end");
    assert!(!past.in_band, "a target not reached is not in band");
    assert_eq!(past.value, None, "no value past the end");
    let ran = result(&r, "T-RUN");
    assert_eq!(ran.value, Some(0), "the history ends at month 3");
    assert_eq!(ran.status, Status::Out, "reached 0 is out of [1, 1]");
    let zero = result(&r, "T-ZERO");
    assert_eq!(zero.status, Status::Out, "a zero flow sum is out");
    assert!(!zero.in_band);
    assert_eq!(zero.value, None, "no value over a zero flow sum");
    assert!(!r.all_in_band(), "the report has targets out");
}

// --- AC-CAL-03 -------------------------------------------------------------------

/// **AC-CAL-03** `[unit]` A target exactly on either end of its band is in
/// band; one ban (or ppm) outside is out; "how far out" is the distance to
/// the nearer end over the band's width.
///
/// Readings: targets with the metric `at` on a hand-built history, one in
/// bani (`gdp.nominal`) and one in ppm (`labour.unemployment_rate`), each
/// month's point judged by a target whose window is that month. In band,
/// how far out is 0.
#[test]
fn ac_cal_03() {
    let money = [100, 200, 99, 201, 450, -100];
    let rate = [45_000, 75_000, 44_999, 75_001, 15_000];
    let h = StatsHistory::default()
        .with_months(id("gdp.nominal"), NATIONAL, 0, &money)
        .with_months(id("labour.unemployment_rate"), NATIONAL, 0, &rate);
    let mut text = String::new();
    for t in 0..u32::try_from(money.len()).expect("months") {
        text += &target_toml(
            &format!("BANI-{t}"),
            "gdp.nominal",
            "at",
            [t, t],
            [100, 200],
        );
    }
    for t in 0..u32::try_from(rate.len()).expect("months") {
        text += &target_toml(
            &format!("PPM-{t}"),
            "labour.unemployment_rate",
            "at",
            [t, t],
            [45_000, 75_000],
        );
    }
    let r = evaluate(&targets(&text), &h, &run(1));
    assert_eq!(
        r.targets.len(),
        money.len() + rate.len(),
        "every target evaluated"
    );

    let expect = |id: &str, value: i64, inside: bool, far: f64| {
        let t = result(&r, id);
        assert_eq!(t.value, Some(value), "{id}: value");
        assert_eq!(t.in_band, inside, "{id}: in band");
        assert_eq!(
            t.status,
            if inside { Status::InBand } else { Status::Out },
            "{id}: status"
        );
        assert_close(t.how_far_out, far, id);
    };
    expect("BANI-0", 100, true, 0.0);
    expect("BANI-1", 200, true, 0.0);
    expect("BANI-2", 99, false, 0.01);
    expect("BANI-3", 201, false, 0.01);
    expect("BANI-4", 450, false, 2.5);
    expect("BANI-5", -100, false, 2.0);
    expect("PPM-0", 45_000, true, 0.0);
    expect("PPM-1", 75_000, true, 0.0);
    expect("PPM-2", 44_999, false, 1.0 / 30_000.0);
    expect("PPM-3", 75_001, false, 1.0 / 30_000.0);
    expect("PPM-4", 15_000, false, 1.0);
    assert!(!r.all_in_band());

    let inside = evaluate(
        &targets(&target_toml("IN", "gdp.nominal", "at", [0, 0], [100, 200])),
        &h,
        &run(1),
    );
    assert!(inside.all_in_band(), "one target on its band's end");
}

// --- AC-CAL-07 -------------------------------------------------------------------

/// **AC-CAL-07** `[sim]` `gdp.consumption` and `wages.bill` are in the
/// registry and equal `months()[t].consumption` and `months()[t].wage_bill`
/// at every close.
///
/// Readings: the Romania scenario at 1:1000 under the 2021 law of its
/// folder, seed 42, three closes; both indicators are active, in bani, and
/// computed in the default build; their national series start at month 0.
#[test]
fn ac_cal_07() {
    for name in ["gdp.consumption", "wages.bill"] {
        assert!(in_registry(name), "the registry has no indicator {name}");
        let m = meta(name);
        assert!(computed(m), "{name} is computed in this build");
        assert_eq!(m.unit, econ_core::stats::Unit::Bani, "{name}: unit");
    }
    let mut w = world_2021_default(1000);
    run_closes(&mut w, 3);
    let months = w.months();
    for (name, field) in [
        (
            "gdp.consumption",
            months
                .iter()
                .map(|m| m.consumption.get())
                .collect::<Vec<_>>(),
        ),
        (
            "wages.bill",
            months.iter().map(|m| m.wage_bill.get()).collect::<Vec<_>>(),
        ),
    ] {
        let s = w
            .stats()
            .series(id(name), NATIONAL)
            .unwrap_or_else(|| panic!("{name}: no national series"));
        assert_eq!(s.first_month.0, 0, "{name} starts at month 0");
        assert_eq!(s.months, field.as_slice(), "{name} at every close");
    }
}
