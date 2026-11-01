//! Calibration targets and their evaluation (spec `game/calibration`, roadmap
//! M7, first step; ADR-0009's targets as code): the targets file
//! `calibration/targets.toml`, the metrics, and the report of a quiet
//! baseline run (feature `calibration`, off by default; needs `stats`). The
//! evaluation is a pure function of a [`StatsHistory`] and the targets:
//! nothing here runs a world.
//!
//! The API is the spec's sketch, written with the acceptance tests before the
//! code (ADR-0014): a test writer's skeleton whose bodies are
//! `unimplemented!` until the code exists.
//!
//! Choices the sketch left open, stated by the test writer:
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

use crate::stats::{IndicatorId, Key, StatsHistory};

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
    let _ = text;
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Read and parse a targets file.
///
/// # Errors
/// A file that cannot be read, or [`parse_targets`]'s errors.
pub fn read_targets(path: &Path) -> Result<Vec<Target>, TargetsError> {
    let _ = path;
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
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
    let _ = (metric, window, points, others);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
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
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}

/// Evaluate the targets of the run's configuration and length on its
/// history.
#[must_use]
pub fn evaluate(targets: &[Target], history: &StatsHistory, run: &RunInfo) -> Report {
    let _ = (targets, history, run);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

impl StatsHistory {
    /// The history with the monthly points of `id` at `key` from
    /// `first_month` (replacing any it had): a built history for the tests
    /// of the evaluation (test writer's addition, ADR-0014).
    #[must_use]
    pub fn with_months(self, id: IndicatorId, key: Key, first_month: u32, points: &[i64]) -> Self {
        let _ = (id, key, first_month, points);
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}
