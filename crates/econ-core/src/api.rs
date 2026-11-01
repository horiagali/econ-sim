//! The core API (spec `game/core-api`, roadmap M6, fifth slice): one typed
//! surface of queries that read and commands that act, each a `serde` type,
//! for the Tauri host and the UI (feature `api`, off by default; needs
//! `stats`, `groups` and `explain`). `ui/src/generated/api.ts` holds the
//! TypeScript types, generated from these by `just codegen`.
//!
//! The API is the spec's sketch, written with the acceptance tests before the
//! code (ADR-0014). The types the API carries from other modules (`Filter`,
//! `GroupSummary`, `Node`) pass through private mirror types with the same
//! fields (enums of persons' attributes by their variant names, money as
//! bani, an indicator by its dotted name), so that econ-popgen and
//! econ-types need no `serde`.
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

use econ_popgen::{Activity, EduLevel, Enrolled, Sex};
use econ_types::{Bani, Calendar, Date, Month};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::explain::Node;
use crate::groups::{Attribute, Condition, Filter, GroupSummary};
use crate::levers::{LeverValue, Order, RejectCode, Rejection};
use crate::stats::{
    Aggregate, Breakdown, Derive, IndicatorId, IndicatorMeta, Key, Status, View, indicators,
};
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

/// econ-core's features compiled, of those the API reports.
const FEATURES: [(&str, bool); 26] = [
    ("world", cfg!(feature = "world")),
    ("pay_day", cfg!(feature = "pay_day")),
    ("banking", cfg!(feature = "banking")),
    ("fiscal", cfg!(feature = "fiscal")),
    ("monetary", cfg!(feature = "monetary")),
    ("trade", cfg!(feature = "trade")),
    ("investment", cfg!(feature = "investment")),
    ("revenue", cfg!(feature = "revenue")),
    ("demography", cfg!(feature = "demography")),
    ("lifecycle", cfg!(feature = "lifecycle")),
    ("levers", cfg!(feature = "levers")),
    ("policy", cfg!(feature = "policy")),
    ("lever_policy_rate", cfg!(feature = "lever_policy_rate")),
    (
        "lever_monetary_financing",
        cfg!(feature = "lever_monetary_financing"),
    ),
    (
        "lever_fx_devaluation",
        cfg!(feature = "lever_fx_devaluation"),
    ),
    ("lever_fuel_price", cfg!(feature = "lever_fuel_price")),
    ("migration", cfg!(feature = "migration")),
    ("education", cfg!(feature = "education")),
    ("approval", cfg!(feature = "approval")),
    ("poverty", cfg!(feature = "poverty")),
    ("wellbeing", cfg!(feature = "wellbeing")),
    ("home_quality", cfg!(feature = "home_quality")),
    ("stats", cfg!(feature = "stats")),
    ("groups", cfg!(feature = "groups")),
    ("explain", cfg!(feature = "explain")),
    ("api", cfg!(feature = "api")),
];

/// Every breakdown, to read a spelling back.
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

fn aggregate_name(a: Aggregate) -> &'static str {
    match a {
        Aggregate::Sum => "sum",
        Aggregate::Mean => "mean",
        Aggregate::None => "none",
    }
}

fn view_name(v: View) -> &'static str {
    match v {
        View::Overview => "overview",
        View::Budget => "budget",
        View::Prices => "prices",
        View::Finance => "finance",
        View::Trade => "trade",
        View::People => "people",
        View::Informal => "informal",
        View::Map => "map",
    }
}

fn name_of(id: IndicatorId) -> &'static str {
    indicators()
        .iter()
        .find(|m| m.id == id)
        .map_or("", |m| m.name)
}

fn derive_text(d: Derive) -> String {
    match d {
        Derive::Ratio(a, b) => format!("ratio({}, {})", name_of(a), name_of(b)),
        Derive::Real(a) => format!("real({})", name_of(a)),
        Derive::GrowthYoy(a) => format!("growth_yoy({})", name_of(a)),
        Derive::GrowthMom(a) => format!("growth_mom({})", name_of(a)),
        Derive::PerCapita(a) => format!("per_capita({})", name_of(a)),
    }
}

fn indicator_dto(m: &IndicatorMeta) -> IndicatorDto {
    IndicatorDto {
        name: m.name.to_owned(),
        glossary: m.glossary.to_owned(),
        unit: m.unit.name().to_owned(),
        period: m.period.name().to_owned(),
        breakdowns: m.breakdowns.iter().map(|b| b.name().to_owned()).collect(),
        aggregate: aggregate_name(m.aggregate).to_owned(),
        derive: m.derive.map(derive_text),
        feature: m.feature.map(str::to_owned),
        views: m.views.iter().map(|&v| view_name(v).to_owned()).collect(),
        headline: m.headline,
        status: match m.status {
            Status::Active => "active",
            Status::Planned => "planned",
        }
        .to_owned(),
    }
}

impl From<Key> for KeyDto {
    fn from(k: Key) -> Self {
        KeyDto {
            breakdown: k.breakdown.map(|b| b.name().to_owned()),
            index: k.index,
        }
    }
}

impl KeyDto {
    /// The key, if the breakdown's spelling is the registry's.
    fn key(&self) -> Option<Key> {
        let breakdown = match &self.breakdown {
            None => None,
            Some(name) => Some(*BREAKDOWNS.iter().find(|b| b.name() == name)?),
        };
        Some(Key {
            breakdown,
            index: self.index,
        })
    }
}

impl From<Date> for DateDto {
    fn from(d: Date) -> Self {
        DateDto {
            year: d.year,
            month: d.month,
            day: d.day,
        }
    }
}

impl From<DateDto> for Date {
    fn from(d: DateDto) -> Self {
        Date {
            year: d.year,
            month: d.month,
            day: d.day,
        }
    }
}

impl From<&LeverValue> for ValueDto {
    fn from(v: &LeverValue) -> Self {
        match v {
            LeverValue::Rate(r) => ValueDto::Rate(*r),
            LeverValue::Money(b) => ValueDto::Money(b.get()),
            LeverValue::Number(n) => ValueDto::Number(*n),
            LeverValue::Choice(c) => ValueDto::Choice(*c),
            LeverValue::Flag(f) => ValueDto::Flag(*f),
            LeverValue::Schedule(rows) => {
                ValueDto::Schedule(rows.iter().map(|(t, r)| (t.get(), *r)).collect())
            }
        }
    }
}

impl From<&ValueDto> for LeverValue {
    fn from(v: &ValueDto) -> Self {
        match v {
            ValueDto::Rate(r) => LeverValue::Rate(*r),
            ValueDto::Money(b) => LeverValue::Money(Bani(*b)),
            ValueDto::Number(n) => LeverValue::Number(*n),
            ValueDto::Choice(c) => LeverValue::Choice(*c),
            ValueDto::Flag(f) => LeverValue::Flag(*f),
            ValueDto::Schedule(rows) => {
                LeverValue::Schedule(rows.iter().map(|&(t, r)| (Bani(t), r)).collect())
            }
        }
    }
}

/// The lever system's rejection of an order it cannot read.
fn rejected(code: RejectCode, message: String) -> ApiError {
    ApiError::Rejected(Rejection {
        code,
        lever: None,
        message,
    })
}

impl World {
    /// Answer a query; the `World` does not change.
    ///
    /// # Errors
    /// See [`ApiError`].
    pub fn query(&self, q: &Query) -> Result<Answer, ApiError> {
        Ok(match q {
            Query::Status => Answer::Status(self.api_status()),
            Query::Indicators => {
                Answer::Indicators(indicators().iter().map(indicator_dto).collect())
            }
            Query::Series {
                id,
                key,
                from,
                to,
                daily,
            } => Answer::Series(self.api_series(id, key, *from, *to, *daily)?),
            Query::Keys { id } => {
                let id = api_id(id)?;
                Answer::Keys(self.stats().keys(id).iter().map(|&k| k.into()).collect())
            }
            Query::Group { filter } => Answer::Group(self.group(filter)),
            Query::GroupBreakdown { filter, by } => {
                Answer::GroupBreakdown(self.group_breakdown(filter, *by))
            }
            Query::Watched { name } => Answer::Watched(self.watched(name).map(<[_]>::to_vec)),
            Query::Explain { id, key, from, to } => {
                let id = api_id(id)?;
                let key = self.api_key(id, key)?;
                if from > to {
                    return Err(ApiError::Malformed(format!("months {from} to {to}")));
                }
                Answer::Explain(self.explain(id, key, Month(*from), Month(*to)))
            }
            Query::Explained => Answer::Explained(
                self.explained()
                    .iter()
                    .map(|&i| name_of(i).to_owned())
                    .collect(),
            ),
            Query::Levers => Answer::Levers(self.api_levers()?),
        })
    }

    /// Carry out a command.
    ///
    /// # Errors
    /// See [`ApiError`].
    pub fn command(&mut self, c: &Command) -> Result<Answer, ApiError> {
        match c {
            Command::Step { days } => {
                let (day, closes) = (self.day().0, self.months().len());
                for _ in 0..*days {
                    self.api_step()?;
                }
                Ok(self.stepped(day, closes))
            }
            Command::StepMonths { months } => {
                let (day, closes) = (self.day().0, self.months().len());
                let target = closes + usize::try_from(*months).unwrap_or(usize::MAX);
                while self.months().len() < target {
                    self.api_step()?;
                }
                Ok(self.stepped(day, closes))
            }
            Command::Order { order } => {
                let order = self.api_order(order)?;
                let a = self.give(order).map_err(ApiError::Rejected)?;
                Ok(Answer::Ordered(OrderedDto {
                    seq: a.seq,
                    effective: a.effective.into(),
                    warnings: a.warnings.into_iter().map(|w| w.message).collect(),
                }))
            }
            Command::Watch { name, filter } => {
                self.watch_group(name, filter.clone());
                Ok(Answer::Watching(name.clone()))
            }
            Command::WriteHistory { dir } => Err(ApiError::Io(format!(
                "{dir}: the history files are written by the host (econ_io::history), not the core"
            ))),
        }
    }

    fn api_status(&self) -> StatusDto {
        let start = self.start_date();
        StatusDto {
            date: self.date().into(),
            day: self.day().0,
            month: Calendar::new(start).month(self.day()).0,
            sample_scale: self.sample_scale(),
            law: format!("the law of {}", start.year),
            features: FEATURES
                .iter()
                .filter(|f| f.1)
                .map(|f| f.0.to_owned())
                .collect(),
        }
    }

    /// The key `dto` of indicator `id`: national, or a member it has points
    /// for.
    fn api_key(&self, id: IndicatorId, dto: &KeyDto) -> Result<Key, ApiError> {
        let key = dto.key().ok_or(ApiError::BadKey)?;
        if key == Key::NATIONAL || self.stats().keys(id).contains(&key) {
            Ok(key)
        } else {
            Err(ApiError::BadKey)
        }
    }

    fn api_series(
        &self,
        id: &str,
        key: &KeyDto,
        from: u32,
        to: u32,
        daily: bool,
    ) -> Result<SeriesDto, ApiError> {
        let id = api_id(id)?;
        if from > to {
            return Err(ApiError::Malformed(format!("months {from} to {to}")));
        }
        let key = self.api_key(id, key)?;
        let Some(s) = self.stats().series(id, key) else {
            // No point yet (a national value before its first).
            return Ok(SeriesDto {
                first_month: from,
                months: Vec::new(),
                days: None,
            });
        };
        let first = s.first_month.0;
        let start = from.max(first);
        let index = |m: u32| usize::try_from(m.saturating_sub(first)).unwrap_or(usize::MAX);
        let months = if start > to {
            Vec::new()
        } else {
            let (a, b) = (index(start), index(to).saturating_add(1));
            s.months
                .get(a.min(s.months.len())..b.min(s.months.len()))
                .unwrap_or_default()
                .to_vec()
        };
        let days = match (daily, s.days) {
            (true, Some(d)) => {
                let base = self.first_day_of(first);
                let a = self.first_day_of(start).saturating_sub(base);
                let b = self.first_day_of(to.saturating_add(1)).saturating_sub(base);
                Some(
                    d.get(a.min(d.len())..b.min(d.len()))
                        .unwrap_or_default()
                        .to_vec(),
                )
            }
            _ => None,
        };
        Ok(SeriesDto {
            first_month: start,
            months,
            days,
        })
    }

    /// The day index of month `m`'s first day (days counted from the start).
    fn first_day_of(&self, m: u32) -> usize {
        let cal = Calendar::new(self.start_date());
        let mut d = Date {
            day: 1,
            ..self.start_date()
        };
        for _ in 0..m {
            d = if d.month == 12 {
                Date {
                    year: d.year + 1,
                    month: 1,
                    day: 1,
                }
            } else {
                Date {
                    month: d.month + 1,
                    ..d
                }
            };
        }
        if m == 0 {
            return 0;
        }
        cal.day(d)
            .map_or(usize::MAX, |x| usize::try_from(x.0).unwrap_or(usize::MAX))
    }

    fn api_levers(&self) -> Result<LeversDto, ApiError> {
        if self.lever_state().is_none() {
            return Err(ApiError::Malformed("the world has no lever system".into()));
        }
        let law = self.law();
        let registry = self.registry();
        let levers = registry
            .levers
            .iter()
            .map(|m| LeverDto {
                id: m.id.as_str().to_owned(),
                status: match m.status {
                    crate::levers::Status::Active => "active",
                    crate::levers::Status::Planned => "planned",
                    crate::levers::Status::Retired => "retired",
                }
                .to_owned(),
                keys: m.keys.clone(),
                in_force: if m.keys.is_empty() {
                    vec![law.get(m.id, None).map(Into::into)]
                } else {
                    (0..m.keys.len())
                        .map(|k| {
                            let k = u32::try_from(k).expect("a key");
                            law.get(m.id, Some(k)).map(Into::into)
                        })
                        .collect()
                },
            })
            .collect();
        let pending = self
            .pending()
            .iter()
            .map(|p| PendingDto {
                seq: p.seq,
                given: p.given.into(),
                lever: p.command.lever.as_str().to_owned(),
                key: p.command.key.and_then(|k| {
                    registry
                        .get(p.command.lever)
                        .keys
                        .get(usize::try_from(k).ok()?)
                        .cloned()
                }),
                value: (&p.command.value).into(),
                effective: p.effective.into(),
            })
            .collect();
        Ok(LeversDto { levers, pending })
    }

    /// The lever system's order for an order by name.
    fn api_order(&self, order: &OrderDto) -> Result<Order, ApiError> {
        match order {
            OrderDto::Cancel { seq } => Ok(Order::Cancel { seq: *seq }),
            OrderDto::Set {
                lever,
                key,
                value,
                effective,
            } => {
                if self.lever_state().is_none() {
                    return Err(ApiError::Malformed("the world has no lever system".into()));
                }
                let meta = self.registry().find(lever).ok_or_else(|| {
                    rejected(
                        RejectCode::UnknownLever,
                        format!("There is no lever {lever}."),
                    )
                })?;
                let key = match key {
                    None => None,
                    Some(k) => Some(
                        meta.keys
                            .iter()
                            .position(|x| x == k)
                            .and_then(|i| u32::try_from(i).ok())
                            .ok_or_else(|| {
                                rejected(RejectCode::BadKey, format!("{lever} has no key {k}."))
                            })?,
                    ),
                };
                Ok(Order::Set(crate::levers::Command {
                    lever: meta.id,
                    key,
                    value: value.into(),
                    effective: effective.map(Into::into),
                }))
            }
        }
    }

    fn api_step(&mut self) -> Result<(), ApiError> {
        self.step(&[])
            .map(|_| ())
            .map_err(|e| ApiError::Malformed(format!("day {}: {e:?}", self.day().0)))
    }

    fn stepped(&self, day: u32, closes: usize) -> Answer {
        Answer::Stepped(SteppedDto {
            days: self.day().0 - day,
            closes: u32::try_from(self.months().len() - closes).unwrap_or(u32::MAX),
        })
    }
}

/// The indicator of a dotted name.
fn api_id(name: &str) -> Result<IndicatorId, ApiError> {
    indicators()
        .iter()
        .find(|m| m.name == name)
        .map(|m| m.id)
        .ok_or_else(|| ApiError::UnknownIndicator(name.to_owned()))
}

// The serde form of the types the API carries from `groups` and `explain`:
// mirror types with the same fields.

#[derive(Serialize, Deserialize)]
enum SexDto {
    F,
    M,
}

#[derive(Serialize, Deserialize)]
enum EduDto {
    Isced0To2,
    Isced3,
    Isced4,
    Isced5To6,
    Isced7To8,
}

#[derive(Serialize, Deserialize)]
enum ActivityDto {
    Child,
    Pupil,
    Student,
    Employed,
    Unemployed,
    Retired,
    InactiveOther,
}

#[derive(Serialize, Deserialize)]
enum EnrolledDto {
    None,
    PreSchool,
    Primary,
    LowerSecondary,
    UpperSecondary,
    PostSecondary,
    Tertiary,
    MasterDoctorate,
}

/// Conversions both ways between an attribute's enum and its mirror, by
/// variant name.
macro_rules! mirror {
    ($a:ident, $b:ident, [$($v:ident),*]) => {
        impl From<$a> for $b {
            fn from(x: $a) -> Self {
                match x { $($a::$v => $b::$v),* }
            }
        }
        impl From<$b> for $a {
            fn from(x: $b) -> Self {
                match x { $($b::$v => $a::$v),* }
            }
        }
    };
}
mirror!(Sex, SexDto, [F, M]);
mirror!(
    EduLevel,
    EduDto,
    [Isced0To2, Isced3, Isced4, Isced5To6, Isced7To8]
);
mirror!(
    Activity,
    ActivityDto,
    [
        Child,
        Pupil,
        Student,
        Employed,
        Unemployed,
        Retired,
        InactiveOther
    ]
);
mirror!(
    Enrolled,
    EnrolledDto,
    [
        None,
        PreSchool,
        Primary,
        LowerSecondary,
        UpperSecondary,
        PostSecondary,
        Tertiary,
        MasterDoctorate
    ]
);

fn map<A: Copy, B: From<A>>(v: &[A]) -> Vec<B> {
    v.iter().map(|&x| x.into()).collect()
}

fn unmap<A, B: From<A>>(v: Vec<A>) -> Vec<B> {
    v.into_iter().map(Into::into).collect()
}

#[derive(Serialize, Deserialize)]
enum ConditionDto {
    County(Vec<u8>),
    Region(Vec<u8>),
    Urban(bool),
    Age { from: u16, to: u16 },
    Sex(Vec<SexDto>),
    Edu(Vec<EduDto>),
    Activity(Vec<ActivityDto>),
    Occupation(Vec<u8>),
    Industry(Vec<Option<u16>>),
    Fifth(Vec<u8>),
    Class(Vec<u8>),
    Enrolled(Vec<EnrolledDto>),
}

#[derive(Serialize, Deserialize)]
struct FilterDto {
    conditions: Vec<ConditionDto>,
}

impl From<&Condition> for ConditionDto {
    fn from(c: &Condition) -> Self {
        match c {
            Condition::County(v) => ConditionDto::County(v.clone()),
            Condition::Region(v) => ConditionDto::Region(v.clone()),
            Condition::Urban(u) => ConditionDto::Urban(*u),
            Condition::Age { from, to } => ConditionDto::Age {
                from: *from,
                to: *to,
            },
            Condition::Sex(v) => ConditionDto::Sex(map(v)),
            Condition::Edu(v) => ConditionDto::Edu(map(v)),
            Condition::Activity(v) => ConditionDto::Activity(map(v)),
            Condition::Occupation(v) => ConditionDto::Occupation(v.clone()),
            Condition::Industry(v) => ConditionDto::Industry(v.clone()),
            Condition::Fifth(v) => ConditionDto::Fifth(v.clone()),
            Condition::Class(v) => ConditionDto::Class(v.clone()),
            Condition::Enrolled(v) => ConditionDto::Enrolled(map(v)),
        }
    }
}

impl From<ConditionDto> for Condition {
    fn from(c: ConditionDto) -> Self {
        match c {
            ConditionDto::County(v) => Condition::County(v),
            ConditionDto::Region(v) => Condition::Region(v),
            ConditionDto::Urban(u) => Condition::Urban(u),
            ConditionDto::Age { from, to } => Condition::Age { from, to },
            ConditionDto::Sex(v) => Condition::Sex(unmap(v)),
            ConditionDto::Edu(v) => Condition::Edu(unmap(v)),
            ConditionDto::Activity(v) => Condition::Activity(unmap(v)),
            ConditionDto::Occupation(v) => Condition::Occupation(v),
            ConditionDto::Industry(v) => Condition::Industry(v),
            ConditionDto::Fifth(v) => Condition::Fifth(v),
            ConditionDto::Class(v) => Condition::Class(v),
            ConditionDto::Enrolled(v) => Condition::Enrolled(unmap(v)),
        }
    }
}

impl Serialize for Filter {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        FilterDto {
            conditions: self.conditions.iter().map(Into::into).collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Filter {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let f = FilterDto::deserialize(deserializer)?;
        Ok(Filter {
            conditions: unmap(f.conditions),
        })
    }
}

#[derive(Serialize, Deserialize)]
struct SummaryDto {
    persons: u64,
    households: u64,
    person_records: u32,
    household_records: u32,
    small_sample: bool,
    employed: u64,
    unemployed: u64,
    unemployment_rate_ppm: u32,
    wages_gross: i64,
    wages_net: i64,
    pensions: i64,
    unemployment_benefits: i64,
    household_transfers: i64,
    taxes_paid: i64,
    money: i64,
    debt: i64,
    income_per_adult: Option<i64>,
    sol_milli: Option<u32>,
    approval_milli: Option<i32>,
}

impl Serialize for GroupSummary {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SummaryDto {
            persons: self.persons,
            households: self.households,
            person_records: self.person_records,
            household_records: self.household_records,
            small_sample: self.small_sample,
            employed: self.employed,
            unemployed: self.unemployed,
            unemployment_rate_ppm: self.unemployment_rate_ppm,
            wages_gross: self.wages_gross.get(),
            wages_net: self.wages_net.get(),
            pensions: self.pensions.get(),
            unemployment_benefits: self.unemployment_benefits.get(),
            household_transfers: self.household_transfers.get(),
            taxes_paid: self.taxes_paid.get(),
            money: self.money.get(),
            debt: self.debt.get(),
            income_per_adult: self.income_per_adult.map(Bani::get),
            sol_milli: self.sol_milli,
            approval_milli: self.approval_milli,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GroupSummary {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = SummaryDto::deserialize(deserializer)?;
        Ok(GroupSummary {
            persons: s.persons,
            households: s.households,
            person_records: s.person_records,
            household_records: s.household_records,
            small_sample: s.small_sample,
            employed: s.employed,
            unemployed: s.unemployed,
            unemployment_rate_ppm: s.unemployment_rate_ppm,
            wages_gross: Bani(s.wages_gross),
            wages_net: Bani(s.wages_net),
            pensions: Bani(s.pensions),
            unemployment_benefits: Bani(s.unemployment_benefits),
            household_transfers: Bani(s.household_transfers),
            taxes_paid: Bani(s.taxes_paid),
            money: Bani(s.money),
            debt: Bani(s.debt),
            income_per_adult: s.income_per_adult.map(Bani),
            sol_milli: s.sol_milli,
            approval_milli: s.approval_milli,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct NodeDto {
    label: String,
    value: i64,
    /// The indicator's dotted name and the key.
    indicator: Option<(String, KeyDto)>,
    children: Vec<NodeDto>,
}

impl From<&Node> for NodeDto {
    fn from(n: &Node) -> Self {
        NodeDto {
            label: n.label.clone(),
            value: n.value,
            indicator: n.indicator.map(|(i, k)| (name_of(i).to_owned(), k.into())),
            children: n.children.iter().map(Into::into).collect(),
        }
    }
}

impl NodeDto {
    fn node(self) -> Result<Node, String> {
        let indicator = match self.indicator {
            None => None,
            Some((name, key)) => Some((
                api_id(&name).map_err(|_| format!("no indicator {name}"))?,
                key.key()
                    .ok_or_else(|| format!("no breakdown {:?}", key.breakdown))?,
            )),
        };
        Ok(Node {
            label: self.label,
            value: self.value,
            indicator,
            children: self
                .children
                .into_iter()
                .map(NodeDto::node)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl Serialize for Node {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        NodeDto::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        NodeDto::deserialize(deserializer)?
            .node()
            .map_err(serde::de::Error::custom)
    }
}
