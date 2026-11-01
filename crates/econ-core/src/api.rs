//! The core API (spec `game/core-api`, roadmap M6, fifth slice): one typed
//! surface of queries that read and commands that act, each a `serde` type,
//! for the Tauri host and the UI (feature `api`, off by default; needs
//! `stats`, `groups` and `explain`). `ui/src/generated/api.ts` holds the
//! TypeScript types, generated from these by `just codegen`.
//!
//! The API is the spec's sketch, written with the acceptance tests before the
//! code (ADR-0014): a test writer's skeleton whose bodies are
//! `unimplemented!` until the code exists. The `serde` impls of the types the
//! API carries from other modules (`Filter`, `GroupSummary`, `Node`) are
//! skeletons too; the implementation may replace them with derives.
//!
//! Choices the sketch left open, stated by the test writer:
//! - Indicators are named by their dotted registry names, breakdowns by the
//!   registry's spelling (`region`, `county`, `age_band`, …) in [`KeyDto`],
//!   levers by their dotted ids and keys by their text ids in [`OrderDto`];
//!   dates are [`DateDto`]; units, periods and the like are the registry
//!   files' spellings (`bani`, `month`, `active`, …).
//! - `Series { from, to }` is the months `from..=to` that the series has;
//!   with `daily`, a day indicator's points of the days of those months.
//! - `Watched { name }` of a name never watched answers `Watched(None)`.
//! - Errors: an unknown indicator name (`UnknownIndicator`), a key the
//!   indicator lacks (`BadKey`), an order the lever system rejects
//!   (`Rejected` with its `Rejection`; an unknown lever id or key text is
//!   the lever system's `unknown_lever` or `bad_key`), an I/O failure
//!   (`Io`), and a malformed query (`Malformed`: a month range with
//!   `from > to`).

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::explain::Node;
use crate::groups::{Attribute, Filter, GroupSummary};
use crate::levers::Rejection;
use crate::world::World;

/// A key of an indicator: `breakdown` `None` is the national value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct KeyDto {
    /// The breakdown's registry spelling, `None` for national.
    pub breakdown: Option<String>,
    /// The member's index (0 for national).
    pub index: u32,
}

/// A calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DateDto {
    /// The year.
    pub year: i32,
    /// The month, 1 to 12.
    pub month: u8,
    /// The day, 1 to 31.
    pub day: u8,
}

/// A lever's value, as `levers::LeverValue`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueDto {
    /// Basis points.
    Rate(i32),
    /// Bani.
    Money(i64),
    /// A count or an index.
    Number(i64),
    /// Index into the lever's options.
    Choice(u16),
    /// On or off.
    Flag(bool),
    /// (threshold in bani, rate in basis points) rows.
    Schedule(Vec<(i64, i32)>),
}

/// A lever order, as `levers::Order`, with the lever and key by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderDto {
    /// Change a lever.
    Set {
        /// The lever's dotted id.
        lever: String,
        /// For a keyed lever, the key's text id.
        key: Option<String>,
        /// The new value.
        value: ValueDto,
        /// `None`: as soon as the lever allows.
        effective: Option<DateDto>,
    },
    /// Cancel a pending command.
    Cancel {
        /// Its sequence number.
        seq: u64,
    },
}

/// A query: reads, never changes the `World`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Query {
    /// The date, day and month index, `sample_scale`, the law's name, the
    /// features compiled.
    Status,
    /// The registry's metadata.
    Indicators,
    /// The points of a series in a month range.
    Series {
        /// The indicator's dotted name.
        id: String,
        /// The key.
        key: KeyDto,
        /// The first month.
        from: u32,
        /// The last month (included).
        to: u32,
        /// Daily points too, for a day indicator.
        daily: bool,
    },
    /// The keys an indicator has.
    Keys {
        /// The indicator's dotted name.
        id: String,
    },
    /// A group's summary.
    Group {
        /// The filter.
        filter: Filter,
    },
    /// A group's breakdown.
    GroupBreakdown {
        /// The filter.
        filter: Filter,
        /// The attribute.
        by: Attribute,
    },
    /// A watched group's summaries.
    Watched {
        /// Its name.
        name: String,
    },
    /// A tree of explanations.
    Explain {
        /// The indicator's dotted name.
        id: String,
        /// The key.
        key: KeyDto,
        /// The first month.
        from: u32,
        /// The last month.
        to: u32,
    },
    /// The indicators that have a tree.
    Explained,
    /// The lever registry, the law in force and the pending orders.
    Levers,
}

/// A command: acts, and answers with what it did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// Step `days` days.
    Step {
        /// Days to step.
        days: u32,
    },
    /// Step until `months` more months are closed.
    StepMonths {
        /// Closes to make.
        months: u32,
    },
    /// A lever order.
    Order {
        /// The order.
        order: OrderDto,
    },
    /// Watch a group from the next close.
    Watch {
        /// Its name.
        name: String,
        /// The filter.
        filter: Filter,
    },
    /// Write the closed years' history files into a folder.
    WriteHistory {
        /// The folder.
        dir: String,
    },
}

/// `Status`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusDto {
    /// Today's date (the day the next step runs).
    pub date: DateDto,
    /// Today's day index.
    pub day: u32,
    /// The month index today falls in.
    pub month: u32,
    /// The scenario's `sample_scale`.
    pub sample_scale: u32,
    /// The name of the law in force.
    pub law: String,
    /// The cargo features of econ-core compiled.
    pub features: Vec<String>,
}

/// One registry entry, in the registry file's spellings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndicatorDto {
    /// The dotted name.
    pub name: String,
    /// The glossary `code_name` (empty for none).
    pub glossary: String,
    /// `bani`, `real_bani`, `ppm`, `persons`, `index_ppm`, `milli`, `count`.
    pub unit: String,
    /// `month` or `day`.
    pub period: String,
    /// The breakdowns' spellings.
    pub breakdowns: Vec<String>,
    /// `sum`, `mean` or `none`.
    pub aggregate: String,
    /// The formula as the registry writes it (`real(gdp.nominal)`), if
    /// derived.
    pub derive: Option<String>,
    /// The feature that computes it, if any.
    pub feature: Option<String>,
    /// The views' spellings.
    pub views: Vec<String>,
    /// A headline indicator.
    pub headline: bool,
    /// `active` or `planned`.
    pub status: String,
}

/// `Series`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeriesDto {
    /// The month of `months[0]`.
    pub first_month: u32,
    /// One point per month of the range the series has.
    pub months: Vec<i64>,
    /// With `daily` and a day indicator: the points of the range's days.
    pub days: Option<Vec<i64>>,
}

/// One lever of `Levers`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeverDto {
    /// The dotted id.
    pub id: String,
    /// `active`, `planned` or `retired`.
    pub status: String,
    /// The keys' text ids (empty for an unkeyed lever).
    pub keys: Vec<String>,
    /// The value in force per key (one for an unkeyed lever).
    pub in_force: Vec<Option<ValueDto>>,
}

/// A pending order of `Levers`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingDto {
    /// Its sequence number.
    pub seq: u64,
    /// The day it was given.
    pub given: DateDto,
    /// The lever's dotted id.
    pub lever: String,
    /// The key's text id.
    pub key: Option<String>,
    /// The value.
    pub value: ValueDto,
    /// The day it takes effect.
    pub effective: DateDto,
}

/// `Levers`'s answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeversDto {
    /// Every lever of the registry, in its order, with the law in force.
    pub levers: Vec<LeverDto>,
    /// The pending orders, in sequence order.
    pub pending: Vec<PendingDto>,
}

/// What a step did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteppedDto {
    /// Days stepped.
    pub days: u32,
    /// Closes made.
    pub closes: u32,
}

/// An accepted order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderedDto {
    /// Its sequence number.
    pub seq: u64,
    /// The day it takes effect.
    pub effective: DateDto,
    /// Its warnings' messages.
    pub warnings: Vec<String>,
}

/// One history file written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDto {
    /// The year.
    pub year: i32,
    /// The file name.
    pub file: String,
    /// Its SHA-256, hex.
    pub sha256: String,
    /// Its rows.
    pub rows: u64,
}

/// An answer: one variant per query and command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Answer {
    /// `Query::Status`.
    Status(StatusDto),
    /// `Query::Indicators`.
    Indicators(Vec<IndicatorDto>),
    /// `Query::Series`.
    Series(SeriesDto),
    /// `Query::Keys`.
    Keys(Vec<KeyDto>),
    /// `Query::Group`.
    Group(GroupSummary),
    /// `Query::GroupBreakdown`.
    GroupBreakdown(Vec<(u32, GroupSummary)>),
    /// `Query::Watched`: `None` for a name never watched.
    Watched(Option<Vec<GroupSummary>>),
    /// `Query::Explain`: `None` when the indicator has no tree.
    Explain(Option<Node>),
    /// `Query::Explained`: the indicators' dotted names.
    Explained(Vec<String>),
    /// `Query::Levers`.
    Levers(LeversDto),
    /// `Command::Step` and `Command::StepMonths`.
    Stepped(SteppedDto),
    /// `Command::Order`, accepted.
    Ordered(OrderedDto),
    /// `Command::Watch`: the name watched.
    Watching(String),
    /// `Command::WriteHistory`: the files written.
    Written(Vec<BlockDto>),
}

/// Why a query or command failed: a value, never a panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// No indicator (or lever) of that name.
    UnknownIndicator(String),
    /// A key the indicator lacks.
    BadKey,
    /// The lever system rejected the order.
    Rejected(Rejection),
    /// An I/O failure.
    Io(String),
    /// A malformed query or command.
    Malformed(String),
}

impl World {
    /// Answer a query; the `World` does not change.
    ///
    /// # Errors
    /// See [`ApiError`].
    pub fn query(&self, q: &Query) -> Result<Answer, ApiError> {
        let _ = q;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// Carry out a command.
    ///
    /// # Errors
    /// See [`ApiError`].
    pub fn command(&mut self, c: &Command) -> Result<Answer, ApiError> {
        let _ = c;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}

// The serde form of the types the API carries from `groups` and `explain`.

impl Serialize for Filter {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let _ = serializer;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}

impl<'de> Deserialize<'de> for Filter {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let _ = deserializer;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}

impl Serialize for GroupSummary {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let _ = serializer;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}

impl<'de> Deserialize<'de> for GroupSummary {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let _ = deserializer;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}

impl Serialize for Node {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let _ = serializer;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let _ = deserializer;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}
