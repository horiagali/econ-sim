//! History files (spec `game/history-files`, roadmap M6, third slice): the
//! statistics registry's history and the watched groups on disk, as one
//! immutable Arrow IPC block (zstd-compressed) per closed calendar year,
//! `history/<year>.arrow`, and an `index.json` (year → file name, SHA-256,
//! rows) (ADR-0011). Behind the feature `history`.
//!
//! The API is the spec's sketch, written with the acceptance tests before the
//! code (ADR-0014): a test writer's skeleton whose bodies are
//! `unimplemented!` until the code exists.
//!
//! The test writer's additions to the sketch, for the unit and bench
//! criteria (AC-HIST-03 to 05), which need histories that no run gives (other
//! format versions and scales, 50 years): a [`History`] carries its
//! metadata (format version, scenario id, engine hash, start date, each
//! indicator's unit and period), and [`write_history`] writes any `History`
//! as blocks. [`FORMAT_VERSION`] is the version written, and
//! [`READABLE_VERSIONS`] the versions [`read_history`] accepts.
//!
//! Choices the sketch left open, stated by the test writer:
//! - `dir` is the history folder itself: blocks are `dir/<year>.arrow` and
//!   the index `dir/index.json`.
//! - A point's year is the calendar year of its month (month 0 is the start
//!   date's month).
//! - Units and periods are the registry file's spellings (`bani`, `ppm`, …;
//!   `month`, `day`), breakdowns too (`region`, `age_band`, …).
//! - Every error names the file it is about in its message.

use std::path::Path;

use econ_core::groups::GroupSummary;
use econ_core::world::World;
use econ_types::Date;

use crate::SaveError;

/// The format version this engine writes.
pub const FORMAT_VERSION: u32 = 1;
/// The format versions [`read_history`] reads.
pub const READABLE_VERSIONS: &[u32] = &[1];

/// One block written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockInfo {
    /// Its calendar year.
    pub year: i32,
    /// Its file name in the folder (`2022.arrow`).
    pub file: String,
    /// The SHA-256 of its bytes, lowercase hex.
    pub sha256: String,
    /// Its rows.
    pub rows: u64,
}

/// An indicator's metadata as a block carries it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IndicatorInfo {
    /// The registry's dotted name.
    pub name: String,
    /// The unit's spelling.
    pub unit: String,
    /// The period's spelling.
    pub period: String,
}

/// One point of a series.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    /// The registry's dotted name.
    pub indicator: String,
    /// The breakdown's name, `None` for national.
    pub breakdown: Option<String>,
    /// The member's index (0 for national).
    pub index: u32,
    /// The month index from the start (for a daily point, the month the day
    /// is in).
    pub month: u32,
    /// For a daily point, the day index from the start.
    pub day: Option<u32>,
    /// The value, in the indicator's unit.
    pub value: i64,
}

/// A history read from (or to be written to) a folder of blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History {
    /// The format version of the blocks.
    pub format_version: u32,
    /// The scenario's `sample_scale`.
    pub sample_scale: u32,
    /// The scenario's id.
    pub scenario: String,
    /// The engine's git hash.
    pub engine: String,
    /// The date of month 0's first day.
    pub start: Date,
    /// Each indicator's unit and period, in the writer's registry order.
    pub indicators: Vec<IndicatorInfo>,
    /// Every point.
    pub points: Vec<Point>,
    /// The watched groups' summaries: (name, month, summary).
    pub watched: Vec<(String, u32, GroupSummary)>,
}

/// Write a closed year's block of `world`'s history into `dir`.
///
/// # Errors
/// I/O and encoding failures, or a year not closed.
pub fn write_year(dir: &Path, world: &World, year: i32) -> Result<BlockInfo, SaveError> {
    let _ = (dir, world, year);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Write every closed year's block of `world`'s history and `index.json`.
///
/// # Errors
/// I/O and encoding failures.
pub fn write_all(dir: &Path, world: &World) -> Result<Vec<BlockInfo>, SaveError> {
    let _ = (dir, world);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Write a whole history, one block per calendar year with a point, and
/// `index.json`; the blocks carry `h.format_version` (test writer's
/// addition, ADR-0014).
///
/// # Errors
/// I/O and encoding failures.
pub fn write_history(dir: &Path, h: &History) -> Result<Vec<BlockInfo>, SaveError> {
    let _ = (dir, h);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Read every block of `dir`, checked against `index.json`: each block's
/// hash, one `sample_scale`, no block missing, a readable format version.
///
/// # Errors
/// A check failed (naming the file), or I/O and decoding failures.
pub fn read_history(dir: &Path) -> Result<History, SaveError> {
    let _ = dir;
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}
