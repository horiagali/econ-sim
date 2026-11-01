//! Calibration targets and their evaluation (spec `game/calibration`, roadmap
//! M7, first step; ADR-0009's targets as code): the targets file
//! `calibration/targets.toml`, the metrics, and the report of a quiet
//! baseline run (feature `calibration`, off by default; needs `stats`). The
//! evaluation is a pure function of a [`StatsHistory`] and the targets:
//! nothing here runs a world.
//!
//! The API is the spec's sketch, written with the acceptance tests before the
//! code (ADR-0014).
//!
//! Choices the sketch left open (the test writer's readings):
//! - **Two layers.** [`metric_value`] is a metric over plain monthly series
//!   ([`Points`]), so the unit tests need no history; [`evaluate`] reads the
//!   targets' series from a [`StatsHistory`] (built in tests with
//!   [`StatsHistory::with_months`]) and judges them against their bands.
//! - **The targets file.** `[[target]]` tables with the spec's keys. The
//!   indicator and the series a metric names are kept as registry names
//!   (`String`); [`parse_targets`] checks the syntax (known metric, key,
//!   config, two-element window and band), not the registry. `key` is
//!   `"national"` or `"<breakdown>:<index>"` with the registry's breakdown
//!   spellings (`region:3`). `metric` is `mean`, `at`, `min`, `max`,
//!   `drift`, `trend`, `reached`, `ratio(<name>)` or
//!   `explained(<name>, <name>, …)`. `source` and `note` may be absent
//!   (empty).
//! - **Windows.** `window = [first, last]`, months from the start (month 0
//!   is December 2021), both included. A metric reads the points of the
//!   series whose months lie in the window. When the series (or a series the
//!   metric names) has no point at the window's last month the metric has no
//!   value: [`NoValue::NotReached`], except `reached`, which then gives 0.
//! - **`trend`** of a one-point window is 0 (no slope).
//! - **`explained(flows)`**: (the point at the window's last month − the
//!   point at its first month) × 1,000,000 over the sum of every named flow
//!   series's points in the window, rounded half away from zero; a zero sum
//!   has no value ([`NoValue::Undefined`]), and so has `ratio` over a zero
//!   mean.
//! - **Judging.** A value `v` is in band when `low ≤ v ≤ high`. "How far
//!   out" is the distance to the nearer end over the band's width
//!   `high − low` (a width of 0 counts as 1, so `reached`'s band `[1, 1]`
//!   gives 1.0 for a run that stopped); 0 in band; none without a value.
//!   A target whose metric has no value is out; with `NotReached` its status
//!   is [`Status::NotReached`].
//! - **Which targets a run evaluates.** Those of its configuration (or
//!   `both`) whose window's last month is before the run's planned length
//!   (`years × 12`); the others belong to another length and are left out of
//!   the report. They keep the file's order in [`Report::targets`].
//! - **The report file.** `report.json` is `{"runs": [Report, …], "timing":
//!   {…}}`, one [`Report`] per configuration run, serialised by serde as the
//!   types below are (statuses in snake case: `in_band`, `out`,
//!   `not_reached`); only `timing` may differ between two runs.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::stats::{Breakdown, IndicatorId, Key, StatsHistory};

/// The configurations of the quiet baseline (spec, "The quiet baseline").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunConfig {
    /// econ-core's default features.
    Core,
    /// With `home_quality`, `education` and `approval`.
    People,
}

/// The configurations a target is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetConfig {
    /// Core only.
    Core,
    /// People only.
    People,
    /// Both.
    Both,
}

/// A metric (spec, "Metrics").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Metric {
    /// The mean of the points.
    Mean,
    /// The point of the window's last month.
    At,
    /// The smallest point.
    Min,
    /// The largest point.
    Max,
    /// The largest absolute difference between a point and the window's
    /// first point.
    Drift,
    /// The least-squares slope of the points, per year.
    Trend,
    /// The mean of the indicator over the mean of the named series,
    /// millionths.
    Ratio(String),
    /// 1 if the run has a point at the window's last month, else 0.
    Reached,
    /// The change of the indicator over the window over the sum of the named
    /// flow series over the window, millionths.
    Explained(Vec<String>),
}

/// One target of the targets file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Its stable id (`L-01`).
    pub id: String,
    /// The registry name of its indicator.
    pub indicator: String,
    /// The key (national or a breakdown member).
    pub key: Key,
    /// The metric.
    pub metric: Metric,
    /// The first and last months, both included.
    pub window: [u32; 2],
    /// The band, both ends included, in the indicator's unit.
    pub band: [i64; 2],
    /// The configurations it is for.
    pub config: TargetConfig,
    /// Where the band comes from.
    pub source: String,
    /// A note.
    pub note: String,
}

/// Why a targets file was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetsError {
    /// What is wrong, naming the target (or the file) it is about.
    pub message: String,
}

/// Parse the text of a targets file.
///
/// # Errors
/// A text that is not TOML, or a target with a missing or malformed field.
pub fn parse_targets(text: &str) -> Result<Vec<Target>, TargetsError> {
    let file: RawFile = toml::from_str(text).map_err(|e| TargetsError {
        message: format!("the targets file is not a targets file: {e}"),
    })?;
    file.target.into_iter().map(Target::from_raw).collect()
}

/// The file as TOML gives it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    #[serde(default)]
    target: Vec<RawTarget>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTarget {
    id: String,
    indicator: String,
    key: String,
    metric: String,
    window: [u32; 2],
    band: [i64; 2],
    config: TargetConfig,
    #[serde(default)]
    source: String,
    #[serde(default)]
    note: String,
}

impl Target {
    fn from_raw(r: RawTarget) -> Result<Target, TargetsError> {
        let err = |what: String| TargetsError {
            message: format!("target {}: {what}", r.id),
        };
        let key = parse_key(&r.key).ok_or_else(|| err(format!("unknown key {:?}", r.key)))?;
        let metric =
            parse_metric(&r.metric).ok_or_else(|| err(format!("unknown metric {:?}", r.metric)))?;
        Ok(Target {
            id: r.id,
            indicator: r.indicator,
            key,
            metric,
            window: r.window,
            band: r.band,
            config: r.config,
            source: r.source,
            note: r.note,
        })
    }
}

/// `national` or `<breakdown>:<index>`.
fn parse_key(s: &str) -> Option<Key> {
    if s == "national" {
        return Some(Key::NATIONAL);
    }
    let (name, index) = s.split_once(':')?;
    let index: u32 = index.trim().parse().ok()?;
    BREAKDOWNS
        .iter()
        .find(|b| b.name() == name.trim())
        .map(|&b| Key {
            breakdown: Some(b),
            index,
        })
}

const BREAKDOWNS: [Breakdown; 13] = [
    Breakdown::Region,
    Breakdown::County,
    Breakdown::Industry,
    Breakdown::Class,
    Breakdown::Fifth,
    Breakdown::AgeBand,
    Breakdown::Sex,
    Breakdown::Edu,
    Breakdown::Activity,
    Breakdown::Level,
    Breakdown::Flow,
    Breakdown::Line,
    Breakdown::Good,
];

/// A metric's spelling: a name, or `ratio(x)` and `explained(a, b, …)`.
fn parse_metric(s: &str) -> Option<Metric> {
    let s = s.trim();
    let args = |prefix: &str| -> Option<Vec<String>> {
        let inner = s.strip_prefix(prefix)?.strip_suffix(')')?;
        let names: Vec<String> = inner.split(',').map(|x| x.trim().to_string()).collect();
        (!names.iter().any(String::is_empty)).then_some(names)
    };
    Some(match s {
        "mean" => Metric::Mean,
        "at" => Metric::At,
        "min" => Metric::Min,
        "max" => Metric::Max,
        "drift" => Metric::Drift,
        "trend" => Metric::Trend,
        "reached" => Metric::Reached,
        _ => {
            if let Some(mut a) = args("ratio(") {
                if a.len() != 1 {
                    return None;
                }
                Metric::Ratio(a.remove(0))
            } else {
                Metric::Explained(args("explained(")?)
            }
        }
    })
}

/// Read and parse a targets file.
///
/// # Errors
/// A file that cannot be read, or [`parse_targets`]'s errors.
pub fn read_targets(path: &Path) -> Result<Vec<Target>, TargetsError> {
    let text = std::fs::read_to_string(path).map_err(|e| TargetsError {
        message: format!("{}: {e}", path.display()),
    })?;
    parse_targets(&text)
}

/// A series's monthly points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Points<'a> {
    /// The month of `months[0]`.
    pub first_month: u32,
    /// One point per month.
    pub months: &'a [i64],
}

/// Why a metric has no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoValue {
    /// A series has no point at the window's last month.
    NotReached,
    /// A zero denominator (`ratio`, `explained`).
    Undefined,
}

/// The value of `metric` over `window` of `points`; `others` are the series
/// the metric names, in its order (one for `ratio`, the flows of
/// `explained`, none for the rest).
///
/// # Errors
/// [`NoValue`] when the metric has no value.
pub fn metric_value(
    metric: &Metric,
    window: [u32; 2],
    points: Points<'_>,
    others: &[Points<'_>],
) -> Result<i64, NoValue> {
    if *metric == Metric::Reached {
        return Ok(i64::from(in_window(window, points).is_some()));
    }
    let p = in_window(window, points).ok_or(NoValue::NotReached)?;
    let o: Vec<&[i64]> = others
        .iter()
        .map(|&x| in_window(window, x).ok_or(NoValue::NotReached))
        .collect::<Result<_, _>>()?;
    let n = i128::try_from(p.len()).expect("a length");
    let sum = |v: &[i64]| v.iter().map(|&x| i128::from(x)).sum::<i128>();
    let first = p[0];
    let last = p[p.len() - 1];
    let v = match metric {
        Metric::Mean => round_div(sum(p), n),
        Metric::At => i128::from(last),
        Metric::Min => i128::from(p.iter().copied().min().expect("a point")),
        Metric::Max => i128::from(p.iter().copied().max().expect("a point")),
        Metric::Drift => p
            .iter()
            .map(|&x| (i128::from(x) - i128::from(first)).abs())
            .max()
            .expect("a point"),
        Metric::Trend => {
            // The least-squares slope, exactly: (n Σxy − Σx Σy) / (n Σx² − (Σx)²).
            let (mut sx, mut sy, mut sxy, mut sxx) = (0i128, 0i128, 0i128, 0i128);
            for (k, &y) in p.iter().enumerate() {
                let x = i128::try_from(k).expect("a month");
                let y = i128::from(y);
                sx += x;
                sy += y;
                sxy += x * y;
                sxx += x * x;
            }
            let d = n * sxx - sx * sx;
            if d == 0 {
                0
            } else {
                round_div(12 * (n * sxy - sx * sy), d)
            }
        }
        Metric::Ratio(_) => {
            let b = o.first().ok_or(NoValue::Undefined)?;
            let nb = i128::try_from(b.len()).expect("a length");
            let d = sum(b) * n;
            if d == 0 {
                return Err(NoValue::Undefined);
            }
            round_div(sum(p) * nb * 1_000_000, d)
        }
        Metric::Explained(_) => {
            let flows: i128 = o.iter().map(|f| sum(f)).sum();
            if flows == 0 {
                return Err(NoValue::Undefined);
            }
            round_div((i128::from(last) - i128::from(first)) * 1_000_000, flows)
        }
        Metric::Reached => unreachable!("handled above"),
    };
    i64::try_from(v).map_err(|_| NoValue::Undefined)
}

/// The points of `points` whose months lie in `window`; `None` when it has
/// no point at the window's last month.
fn in_window(window: [u32; 2], points: Points<'_>) -> Option<&[i64]> {
    let [from, to] = window;
    let end = usize::try_from(to.checked_sub(points.first_month)?).ok()?;
    if end >= points.months.len() || from > to {
        return None;
    }
    let start = usize::try_from(from.saturating_sub(points.first_month)).ok()?;
    Some(&points.months[start..=end])
}

/// `n / d` rounded half away from zero (`d` ≠ 0).
fn round_div(n: i128, d: i128) -> i128 {
    let (an, ad) = (n.abs(), d.abs());
    let q = (2 * an + ad) / (2 * ad);
    if (n < 0) == (d < 0) { q } else { -q }
}

/// Where a run stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stopped {
    /// The month the run stopped in (from 0).
    pub month: u32,
    /// Why: the failed check or the panic's message.
    pub reason: String,
}

/// A run, as the report needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunInfo {
    /// Its configuration.
    pub config: RunConfig,
    /// Its scale (1:N).
    pub sample_scale: u32,
    /// Its planned length.
    pub years: u32,
    /// Where it stopped, if it did.
    pub stopped: Option<Stopped>,
}

/// A target's verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// In its band.
    InBand,
    /// Out of its band, or without a value.
    Out,
    /// The run has no point at the window's last month.
    NotReached,
}

/// One target in the report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetResult {
    /// The target's id.
    pub id: String,
    /// Its value, if the metric has one.
    pub value: Option<i64>,
    /// Its band.
    pub band: [i64; 2],
    /// Whether it is in its band.
    pub in_band: bool,
    /// Its verdict.
    pub status: Status,
    /// The distance to the band's nearer end in band widths (0 in band;
    /// none without a value).
    pub how_far_out: Option<f64>,
}

/// The report of one configuration's run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// The configuration.
    pub config: RunConfig,
    /// The scale.
    pub sample_scale: u32,
    /// The planned length.
    pub years: u32,
    /// Where the run stopped, if it did.
    pub stopped: Option<Stopped>,
    /// Every target evaluated, in the file's order.
    pub targets: Vec<TargetResult>,
}

impl Report {
    /// Whether every target is in its band (the exit code 0).
    #[must_use]
    pub fn all_in_band(&self) -> bool {
        self.stopped.is_none() && self.targets.iter().all(|t| t.in_band)
    }
}

/// Evaluate the targets of the run's configuration and length on its
/// history.
#[must_use]
pub fn evaluate(targets: &[Target], history: &StatsHistory, run: &RunInfo) -> Report {
    let months = run.years.saturating_mul(12);
    let results = targets
        .iter()
        .filter(|t| t.config.covers(run.config) && t.window[1] < months)
        .map(|t| judge(t, history))
        .collect();
    Report {
        config: run.config,
        sample_scale: run.sample_scale,
        years: run.years,
        stopped: run.stopped.clone(),
        targets: results,
    }
}

impl TargetConfig {
    /// Whether a target of this configuration is evaluated on a run of `c`.
    #[must_use]
    pub fn covers(self, c: RunConfig) -> bool {
        matches!(
            (self, c),
            (TargetConfig::Both, _)
                | (TargetConfig::Core, RunConfig::Core)
                | (TargetConfig::People, RunConfig::People)
        )
    }
}

/// The points of a registry indicator at a key (none when the registry has
/// no such indicator or the run did not compute it).
fn points_of<'a>(history: &'a StatsHistory, name: &str, key: Key) -> Points<'a> {
    let series = crate::stats::indicators()
        .iter()
        .find(|m| m.name == name)
        .and_then(|m| history.series(m.id, key));
    match series {
        Some(s) => Points {
            first_month: s.first_month.0,
            months: s.months,
        },
        None => Points {
            first_month: 0,
            months: &[],
        },
    }
}

/// One target's verdict.
fn judge(t: &Target, history: &StatsHistory) -> TargetResult {
    let points = points_of(history, &t.indicator, t.key);
    let named: &[String] = match &t.metric {
        Metric::Ratio(x) => std::slice::from_ref(x),
        Metric::Explained(xs) => xs,
        _ => &[],
    };
    let others: Vec<Points<'_>> = named
        .iter()
        .map(|n| points_of(history, n, Key::NATIONAL))
        .collect();
    let [low, high] = t.band;
    let (value, status, how_far_out) = match metric_value(&t.metric, t.window, points, &others) {
        Ok(v) => {
            if (low..=high).contains(&v) {
                (Some(v), Status::InBand, Some(0.0))
            } else {
                let dist = if v < low {
                    i128::from(low) - i128::from(v)
                } else {
                    i128::from(v) - i128::from(high)
                };
                let width = (i128::from(high) - i128::from(low)).max(1);
                #[allow(clippy::cast_precision_loss)]
                let far = dist as f64 / width as f64;
                (Some(v), Status::Out, Some(far))
            }
        }
        Err(NoValue::NotReached) => (None, Status::NotReached, None),
        Err(NoValue::Undefined) => (None, Status::Out, None),
    };
    TargetResult {
        id: t.id.clone(),
        value,
        band: t.band,
        in_band: status == Status::InBand,
        status,
        how_far_out,
    }
}

impl StatsHistory {
    /// The history with the monthly points of `id` at `key` from
    /// `first_month` (replacing any it had): a built history for the tests
    /// of the evaluation (test writer's addition, ADR-0014).
    #[must_use]
    pub fn with_months(
        mut self,
        id: IndicatorId,
        key: Key,
        first_month: u32,
        points: &[i64],
    ) -> Self {
        self.set_months(id, key, first_month, points);
        self
    }
}
