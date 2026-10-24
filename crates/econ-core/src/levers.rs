//! The lever system (spec `game/lever-system`, ADR-0018): the registry, the
//! commands the player gives, their validation, effective dates, the law in
//! force and the command log.
//!
//! **API skeleton only** (ADR-0014): the types and signatures of the spec's
//! "API sketch", written before the acceptance tests; every body is
//! `unimplemented!` until those tests exist. Behind the feature `levers`, off by
//! default.
//!
//! The registry's data is `crates/econ-core/levers.toml`; the starting values
//! are `python/pipeline/fixtures/law2021_ro.json`. [`LeverId`] is generated
//! from the registry by `scripts/codegen_levers.py` into `levers_gen.rs`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use econ_types::{Bani, Date};

use crate::world::{Law, PhaseId, World};

/// When a lever's change can take effect (spec, "Effective date").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// The first day of the month after the day given, then `min_lag`.
    Law,
    /// The day after the day given (central-bank operations).
    Market,
    /// The day after: a project starts; its duration is its mechanic's.
    Project,
    /// The day after: a world setting (roadmap D8).
    World,
}

/// The form of a lever's value (ADR-0018: six shapes, integers only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shape {
    /// Basis points.
    Rate,
    /// Bani.
    Money,
    /// A count or an index.
    Number,
    /// One of the lever's `options`.
    Choice,
    /// On or off.
    Flag,
    /// Ordered (threshold in bani, rate in basis points) rows.
    Schedule,
}

/// The lever groups of counterfactual attribution (ADR-0008 L4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum Group {
    Taxes,
    Spending,
    Transfers,
    Money,
    Trade,
    World,
}

/// Whether commands of a lever are accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Wired to a mechanic; commands accepted.
    Active,
    /// Registered, not wired yet: commands rejected with `not_yet`.
    Planned,
    /// Removed: commands rejected with `retired`; never deleted from the registry.
    Retired,
}

/// The lever ids, generated from `levers.toml` by `scripts/codegen_levers.py`.
#[path = "levers_gen.rs"]
mod levers_gen;
pub use levers_gen::LeverId;

/// A gradual change: the value in force moves toward the commanded one by at
/// most `per_month` (in the shape's integer) on the first of each month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseIn {
    /// The largest step a month.
    pub per_month: i64,
}

/// One registry entry (spec, "What a lever is").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeverMeta {
    /// The lever.
    pub id: LeverId,
    /// Earliest effective date rule.
    pub kind: Kind,
    /// Form of the value.
    pub shape: Shape,
    /// Display unit.
    pub unit: String,
    /// Inclusive bounds in the shape's integer; for a schedule, of each rate.
    pub range: Option<(i64, i64)>,
    /// For a schedule: the inclusive bounds of the number of rows.
    pub rows: Option<(u32, u32)>,
    /// For a keyed lever: the name of its key table.
    pub key: Option<String>,
    /// For a keyed lever: the allowed keys (the subset, or the whole table).
    pub keys: Vec<String>,
    /// For a choice: the allowed values, in order.
    pub options: Vec<String>,
    /// Extra delay beyond the kind's, in months.
    pub min_lag_months: u32,
    /// Gradual change, if any.
    pub phase_in: Option<PhaseIn>,
    /// The spec that owns the effect.
    pub spec: String,
    /// The glossary `code_name` the lever sets.
    pub glossary: String,
    /// The phase of the `World` that reads it.
    pub reader: PhaseId,
    /// The counterfactual group.
    pub group: Group,
    /// The catalogue's milestone tag, e.g. `alpha`.
    pub milestone: String,
    /// Active, planned or retired.
    pub status: Status,
    /// String-table key of its name and help text.
    pub label: String,
}

/// A fault of the registry file, naming the entry when there is one (AC-LEV-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryError {
    /// The id of the faulty entry, as written in the file.
    pub lever: Option<String>,
    /// What is wrong.
    pub message: String,
}

/// The parsed `levers.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registry {
    /// Every entry, in file order.
    pub levers: Vec<LeverMeta>,
    /// The small key tables the file defines (`[key_tables]`); scenario
    /// tables (industries, budget lines, public units) come from the scenario.
    pub key_tables: BTreeMap<String, Vec<String>>,
}

impl Registry {
    /// Parse the text of `levers.toml`.
    ///
    /// # Errors
    /// A malformed entry, an unknown kind, shape, group, status or reader.
    pub fn parse(text: &str) -> Result<Registry, RegistryError> {
        let file: RawFile = toml::from_str(text).map_err(|e| RegistryError {
            lever: None,
            message: format!("levers.toml: {e}"),
        })?;
        let mut levers = Vec::with_capacity(file.lever.len());
        for (n, entry) in file.lever.into_iter().enumerate() {
            let id = entry
                .get("id")
                .and_then(toml::Value::as_str)
                .map(str::to_string);
            let fault = |message: String| RegistryError {
                lever: id.clone(),
                message,
            };
            let raw: RawLever = entry
                .try_into()
                .map_err(|e| fault(format!("entry {}: {e}", n + 1)))?;
            levers.push(meta_of(raw, &file.key_tables).map_err(fault)?);
        }
        Ok(Registry {
            levers,
            key_tables: file.key_tables,
        })
    }

    /// The registry committed with this engine (`crates/econ-core/levers.toml`).
    ///
    /// # Panics
    /// The committed file does not parse (`just check` keeps it parsing).
    #[must_use]
    pub fn committed() -> Registry {
        Registry::parse(include_str!("../levers.toml")).expect("the committed levers.toml parses")
    }

    /// Read and parse `levers.toml`.
    ///
    /// # Errors
    /// The file cannot be read, or [`Registry::parse`] fails.
    pub fn load(path: &Path) -> Result<Registry, RegistryError> {
        let text = std::fs::read_to_string(path).map_err(|e| RegistryError {
            lever: None,
            message: format!("{}: {e}", path.display()),
        })?;
        Registry::parse(&text)
    }

    /// The entry of a lever.
    #[must_use]
    ///
    /// # Panics
    /// The registry has no entry for the lever (a registry not parsed from
    /// the file the ids were generated from).
    pub fn get(&self, lever: LeverId) -> &LeverMeta {
        self.levers
            .iter()
            .find(|m| m.id == lever)
            .unwrap_or_else(|| panic!("the registry has no entry {}", lever.as_str()))
    }

    /// The entry of a dotted id, if any.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&LeverMeta> {
        let lever = LeverId::from_id(id)?;
        self.levers.iter().find(|m| m.id == lever)
    }

    /// The registry checks of `just check` (spec, "Checks"): unique dotted
    /// ids, the fields of each shape, `spec` among `spec_ids` (the docs
    /// index), `glossary` among `glossary_names`, a known key table.
    ///
    /// # Errors
    /// Every fault found, each naming its entry.
    pub fn check(
        &self,
        spec_ids: &[&str],
        glossary_names: &[&str],
        scenario_key_tables: &[&str],
    ) -> Result<(), Vec<RegistryError>> {
        let mut errors = Vec::new();
        let mut seen = BTreeSet::new();
        for m in &self.levers {
            let id = m.id.as_str();
            let mut faults = Vec::new();
            if !seen.insert(m.id) {
                faults.push("duplicate id".to_string());
            }
            if !is_dotted(id) {
                faults.push("the id is not lower-case dotted words".to_string());
            }
            match m.shape {
                Shape::Rate | Shape::Money | Shape::Number if m.range.is_none() => {
                    faults.push("a range is needed".to_string());
                }
                Shape::Schedule if m.range.is_none() || m.rows.is_none() => {
                    faults.push("a schedule needs a range and rows".to_string());
                }
                Shape::Choice if m.options.is_empty() => {
                    faults.push("a choice needs options".to_string());
                }
                _ => {}
            }
            if m.range.is_some_and(|(lo, hi)| lo > hi)
                || m.rows.is_some_and(|(lo, hi)| lo > hi || lo == 0)
            {
                faults.push("a range runs backwards or allows no rows".to_string());
            }
            if !spec_ids.contains(&m.spec.as_str()) {
                faults.push(format!("spec `{}` is not in the docs index", m.spec));
            }
            if !glossary_names.contains(&m.glossary.as_str()) {
                faults.push(format!("glossary `{}` is not a glossary name", m.glossary));
            }
            if let Some(table) = &m.key {
                match self.key_tables.get(table) {
                    Some(all) => {
                        if let Some(k) = m.keys.iter().find(|k| !all.contains(k)) {
                            faults.push(format!("key `{k}` is not in the table `{table}`"));
                        }
                    }
                    None if scenario_key_tables.contains(&table.as_str()) => {}
                    None => faults.push(format!("no key table `{table}`")),
                }
            } else if !m.keys.is_empty() {
                faults.push("keys without a key table".to_string());
            }
            if m.label.is_empty() {
                faults.push("no label".to_string());
            }
            errors.extend(faults.into_iter().map(|f| RegistryError {
                lever: Some(id.to_string()),
                message: format!("{id}: {f}"),
            }));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// `levers.toml` as written.
#[derive(serde::Deserialize)]
struct RawFile {
    #[serde(default)]
    key_tables: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    lever: Vec<toml::Value>,
}

/// One `[[lever]]` table as written.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLever {
    id: String,
    kind: String,
    shape: String,
    unit: String,
    range: Option<[i64; 2]>,
    rows: Option<[u32; 2]>,
    key: Option<String>,
    keys: Option<Vec<String>>,
    options: Option<Vec<String>>,
    /// `"1 month"`, `"3 months"`.
    min_lag: Option<String>,
    phase_in: Option<RawPhaseIn>,
    spec: String,
    glossary: String,
    reader: String,
    group: String,
    milestone: String,
    status: String,
    label: String,
}

/// `phase_in = { per_month = … }`.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPhaseIn {
    per_month: i64,
}

/// A registry entry from its table; the fault's text otherwise.
fn meta_of(raw: RawLever, tables: &BTreeMap<String, Vec<String>>) -> Result<LeverMeta, String> {
    let id = LeverId::from_id(&raw.id).ok_or_else(|| {
        format!(
            "`{}` is not among the generated ids: run scripts/codegen_levers.py",
            raw.id
        )
    })?;
    let kind = match raw.kind.as_str() {
        "law" => Kind::Law,
        "market" => Kind::Market,
        "project" => Kind::Project,
        "world" => Kind::World,
        k => return Err(format!("unknown kind `{k}`")),
    };
    let shape = match raw.shape.as_str() {
        "rate" => Shape::Rate,
        "money" => Shape::Money,
        "number" => Shape::Number,
        "choice" => Shape::Choice,
        "flag" => Shape::Flag,
        "schedule" => Shape::Schedule,
        k => return Err(format!("unknown shape `{k}`")),
    };
    let group = match raw.group.as_str() {
        "taxes" => Group::Taxes,
        "spending" => Group::Spending,
        "transfers" => Group::Transfers,
        "money" => Group::Money,
        "trade" => Group::Trade,
        "world" => Group::World,
        k => return Err(format!("unknown group `{k}`")),
    };
    let status = match raw.status.as_str() {
        "active" => Status::Active,
        "planned" => Status::Planned,
        "retired" => Status::Retired,
        k => return Err(format!("unknown status `{k}`")),
    };
    let reader = phase_of(&raw.reader).ok_or_else(|| format!("unknown reader `{}`", raw.reader))?;
    let min_lag_months = match &raw.min_lag {
        None => 0,
        Some(lag) => lag
            .trim()
            .strip_suffix("months")
            .or_else(|| lag.trim().strip_suffix("month"))
            .and_then(|n| n.trim().parse().ok())
            .ok_or_else(|| format!("min_lag `{lag}` is not \"N months\""))?,
    };
    let keys = match (&raw.key, raw.keys) {
        (_, Some(subset)) => subset,
        (Some(table), None) => tables.get(table).cloned().unwrap_or_default(),
        (None, None) => Vec::new(),
    };
    Ok(LeverMeta {
        id,
        kind,
        shape,
        unit: raw.unit,
        range: raw.range.map(|[lo, hi]| (lo, hi)),
        rows: raw.rows.map(|[lo, hi]| (lo, hi)),
        key: raw.key,
        keys,
        options: raw.options.unwrap_or_default(),
        min_lag_months,
        phase_in: raw.phase_in.map(|p| PhaseIn {
            per_month: p.per_month,
        }),
        spec: raw.spec,
        glossary: raw.glossary,
        reader,
        group,
        milestone: raw.milestone,
        status,
        label: raw.label,
    })
}

/// The phase of a `reader` name (the `PhaseId` variant's name).
fn phase_of(name: &str) -> Option<PhaseId> {
    Some(match name {
        "Commands" => PhaseId::Commands,
        "Plan" => PhaseId::Plan,
        "Staffing" => PhaseId::Staffing,
        "Prices" => PhaseId::Prices,
        "OutputAndInputs" => PhaseId::OutputAndInputs,
        "FixedDemand" => PhaseId::FixedDemand,
        "Households" => PhaseId::Households,
        "GoodsMarket" => PhaseId::GoodsMarket,
        "Quits" => PhaseId::Quits,
        "Pay" => PhaseId::Pay,
        "WageRound" => PhaseId::WageRound,
        "MonthlyClose" => PhaseId::MonthlyClose,
        "Checks" => PhaseId::Checks,
        _ => return None,
    })
}

/// Lower-case words of `a-z`, `0-9` and `_`, at least two, joined by dots.
fn is_dotted(id: &str) -> bool {
    let words: Vec<&str> = id.split('.').collect();
    words.len() >= 2
        && words.iter().all(|w| {
            !w.is_empty()
                && w.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
}

/// A lever's value, in one of the six shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeverValue {
    /// Basis points.
    Rate(i32),
    /// Bani.
    Money(Bani),
    /// A count or an index.
    Number(i64),
    /// Index into the lever's `options`.
    Choice(u16),
    /// On or off.
    Flag(bool),
    /// (threshold, rate in basis points) rows, thresholds strictly ascending from 0.
    Schedule(Vec<(Bani, i32)>),
}

/// A change of one lever (ADR-0018).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// The lever.
    pub lever: LeverId,
    /// For a keyed lever: the index of the key in the lever's `keys`.
    pub key: Option<u32>,
    /// The new value.
    pub value: LeverValue,
    /// `None`: as soon as the lever's kind and lag allow.
    pub effective: Option<Date>,
}

/// What the player gives: a change, or the cancel of a pending one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Order {
    /// Change a lever.
    Set(Command),
    /// Cancel the pending command with this sequence number.
    Cancel {
        /// The sequence number of the command to cancel.
        seq: u64,
    },
}

/// Why a command is refused (spec, "Commands and validation").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RejectCode {
    /// `unknown_lever`: the id is not in the registry.
    UnknownLever,
    /// `retired`: the lever is retired.
    Retired,
    /// `not_yet`: the lever is planned.
    NotYet,
    /// `wrong_shape`: the value is not of the lever's shape.
    WrongShape,
    /// `out_of_range`: a value, a schedule rate or the row count is out of range.
    OutOfRange,
    /// `bad_key`: key missing, unexpected, unknown or not allowed.
    BadKey,
    /// `bad_option`: a choice not among the options.
    BadOption,
    /// `bad_schedule`: thresholds not strictly ascending from 0.
    BadSchedule,
    /// `too_early`: the requested date is before the earliest allowed.
    TooEarly,
    /// `nothing_to_cancel`: the cancel names no pending command.
    NothingToCancel,
}

impl RejectCode {
    /// The code as the spec writes it, e.g. `out_of_range`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RejectCode::UnknownLever => "unknown_lever",
            RejectCode::Retired => "retired",
            RejectCode::NotYet => "not_yet",
            RejectCode::WrongShape => "wrong_shape",
            RejectCode::OutOfRange => "out_of_range",
            RejectCode::BadKey => "bad_key",
            RejectCode::BadOption => "bad_option",
            RejectCode::BadSchedule => "bad_schedule",
            RejectCode::TooEarly => "too_early",
            RejectCode::NothingToCancel => "nothing_to_cancel",
        }
    }
}

/// A refused command: nothing changes and nothing is logged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    /// Why.
    pub code: RejectCode,
    /// The lever, when the command named a known one.
    pub lever: Option<LeverId>,
    /// The English message of the string table, values in display units.
    pub message: String,
}

/// Why an accepted command carries a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WarningCode {
    /// It breaks an EU rule (VAT standard rate below 15%, a reduced rate below 5%).
    EuRule,
    /// It breaks a consistency the law usually keeps (a reduced rate above the standard one).
    Inconsistent,
}

/// A warning on an accepted command (an L5 `PolicyWarning`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// Why.
    pub code: WarningCode,
    /// The lever.
    pub lever: LeverId,
    /// The English message.
    pub message: String,
}

/// An accepted order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    /// Its sequence number in the log.
    pub seq: u64,
    /// The day it takes effect (for a cancel, the day given).
    pub effective: Date,
    /// EU and consistency warnings.
    pub warnings: Vec<Warning>,
}

/// A command accepted and not yet in force (part of the state).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// Its sequence number.
    pub seq: u64,
    /// The day it was given.
    pub given: Date,
    /// The command.
    pub command: Command,
    /// The day it takes effect.
    pub effective: Date,
}

/// One line of the command log, of a commands file, of `law.json` or of
/// `world.json` (JSON lines; spec, "Logging and deterministic replay").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// Sequence number; optional in a commands file and absent in `law.json`.
    pub seq: Option<u64>,
    /// The day it was given.
    pub given: Date,
    /// The change or the cancel.
    pub order: Order,
    /// The day it took or takes effect; optional in a commands file.
    pub effective: Option<Date>,
}

/// The L5 event of a command taking effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyChanged {
    /// The lever.
    pub lever: LeverId,
    /// The key, for a keyed lever.
    pub key: Option<u32>,
    /// The value before.
    pub old: LeverValue,
    /// The value after.
    pub new: LeverValue,
    /// The day the command was given.
    pub given: Date,
    /// The day it took effect.
    pub effective: Date,
    /// Its sequence number.
    pub seq: u64,
}

/// The law in force: one value per lever (and key). The mechanics read it
/// through the hand-written rule structs of [`Law`], filled from it by
/// [`law_of`] whenever a command takes effect (spec, "API sketch").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawInForce {
    /// By lever, in [`LeverId::ALL`] order; by key (one slot for an unkeyed
    /// lever).
    values: Vec<Vec<Option<LeverValue>>>,
}

impl LawInForce {
    /// The law of `law.json` and `world.json`: every lever of the registry
    /// set once, as start-date lines.
    ///
    /// # Errors
    /// A line rejected, or an active lever (or key) missing, named (AC-LEV-02).
    pub fn from_lines(registry: &Registry, lines: &[LogLine]) -> Result<LawInForce, Rejection> {
        let mut law = LawInForce {
            values: LeverId::ALL
                .iter()
                .map(|&l| {
                    let n = registry.levers.iter().find(|m| m.id == l).map_or(1, slots);
                    vec![None; n]
                })
                .collect(),
        };
        for line in lines {
            match &line.order {
                Order::Set(cmd) => {
                    check_value(registry.get(cmd.lever), cmd)?;
                    law.set(cmd);
                }
                Order::Cancel { .. } => {
                    return Err(Rejection {
                        code: RejectCode::NothingToCancel,
                        lever: None,
                        message: "The law of a scenario has no cancels.".to_string(),
                    });
                }
            }
        }
        // Every active lever, and every key of a keyed one, is set. No code of
        // the spec's table fits a missing line; it is reported as `bad_key`.
        for meta in registry
            .levers
            .iter()
            .filter(|m| m.status == Status::Active)
        {
            for k in 0..slots(meta) {
                let key = u32::try_from(k).ok().filter(|_| meta.key.is_some());
                if law.get(meta.id, key).is_none() {
                    let what = match key {
                        Some(_) => format!(
                            "`{}` ({}) for `{}`",
                            meta.label,
                            meta.id.as_str(),
                            meta.keys[k]
                        ),
                        None => format!("`{}` ({})", meta.label, meta.id.as_str()),
                    };
                    return Err(Rejection {
                        code: RejectCode::BadKey,
                        lever: Some(meta.id),
                        message: format!(
                            "{what} is not set by the scenario's law.json or world.json."
                        ),
                    });
                }
            }
        }
        Ok(law)
    }

    /// The value in force of a lever, for a key of a keyed lever.
    #[must_use]
    pub fn get(&self, lever: LeverId, key: Option<u32>) -> Option<&LeverValue> {
        let slot = usize::try_from(key.unwrap_or(0)).ok()?;
        self.values.get(lever as usize)?.get(slot)?.as_ref()
    }

    /// Apply a validated command; changes nothing else.
    pub fn set(&mut self, cmd: &Command) {
        let l = cmd.lever as usize;
        if self.values.len() <= l {
            self.values.resize(LeverId::ALL.len(), Vec::new());
        }
        let slot = usize::try_from(cmd.key.unwrap_or(0)).expect("a key index fits usize");
        let values = &mut self.values[l];
        if values.len() <= slot {
            values.resize(slot + 1, None);
        }
        values[slot] = Some(cmd.value.clone());
    }
}

/// The value slots of a lever: one per key, or one.
fn slots(meta: &LeverMeta) -> usize {
    if meta.key.is_some() {
        meta.keys.len()
    } else {
        1
    }
}

/// The earliest day a command of `meta` given on `given` can take effect.
#[must_use]
pub fn earliest_effective(meta: &LeverMeta, given: Date) -> Date {
    match meta.kind {
        Kind::Law => first_of_month_after(given, 1 + meta.min_lag_months),
        Kind::Market | Kind::Project | Kind::World => next_day(given),
    }
}

/// The day after.
fn next_day(d: Date) -> Date {
    if d.day < d.days_in_month() {
        Date::new(d.year, d.month, d.day + 1).expect("a day of the month")
    } else {
        first_of_month_after(d, 1)
    }
}

/// The 1st of the month `months` after `d`'s.
fn first_of_month_after(d: Date, months: u32) -> Date {
    let index = i64::from(d.year) * 12 + i64::from(d.month) - 1 + i64::from(months);
    let year = i32::try_from(index.div_euclid(12)).expect("a year in range");
    let month = u8::try_from(index.rem_euclid(12) + 1).expect("a month");
    Date::new(year, month, 1).expect("the 1st exists")
}

/// Validate an order against the registry, the day and the pending list only
/// (never the economy's state), and give its effective day and warnings.
///
/// # Errors
/// The first rejection of the spec's table that applies.
pub fn validate(
    registry: &Registry,
    order: &Order,
    today: Date,
    pending: &[Pending],
) -> Result<(Date, Vec<Warning>), Rejection> {
    let cmd = match order {
        Order::Cancel { seq } => {
            return if pending.iter().any(|p| p.seq == *seq) {
                Ok((today, Vec::new()))
            } else {
                Err(Rejection {
                    code: RejectCode::NothingToCancel,
                    lever: None,
                    message: "That change has already taken effect or does not exist.".to_string(),
                })
            };
        }
        Order::Set(cmd) => cmd,
    };
    let meta = registry.get(cmd.lever);
    let label = &meta.label;
    let reject = |code, message: String| Rejection {
        code,
        lever: Some(meta.id),
        message,
    };
    match meta.status {
        Status::Retired => {
            return Err(reject(
                RejectCode::Retired,
                format!("`{label}` was removed."),
            ));
        }
        Status::Planned => {
            return Err(reject(
                RejectCode::NotYet,
                format!("`{label}` arrives with {}.", meta.milestone),
            ));
        }
        Status::Active => {}
    }
    check_value(meta, cmd)?;
    let earliest = earliest_effective(meta, today);
    let effective = match cmd.effective {
        Some(d) if d < earliest => {
            return Err(reject(
                RejectCode::TooEarly,
                format!(
                    "`{label}` can change from {} at the soonest.",
                    iso(earliest)
                ),
            ));
        }
        Some(d) => d,
        None => earliest,
    };
    Ok((effective, eu_warnings(meta, cmd)))
}

/// The checks of a command's key and value against its lever, without its
/// status or dates (also those of `law.json`'s lines).
fn check_value(meta: &LeverMeta, cmd: &Command) -> Result<(), Rejection> {
    let label = &meta.label;
    let reject = |code, message: String| {
        Err(Rejection {
            code,
            lever: Some(meta.id),
            message,
        })
    };
    let keys = || meta.keys.join(", ");
    match (&meta.key, cmd.key) {
        (Some(_), None) => {
            return reject(
                RejectCode::BadKey,
                format!("`{label}` needs a key; choose one of {}.", keys()),
            );
        }
        (None, Some(_)) => {
            return reject(RejectCode::BadKey, format!("`{label}` takes no key."));
        }
        (Some(_), Some(k)) if usize::try_from(k).map_or(true, |k| k >= meta.keys.len()) => {
            return reject(
                RejectCode::BadKey,
                format!("`{label}` has no key {k}; choose one of {}.", keys()),
            );
        }
        _ => {}
    }
    let given = shape_of(&cmd.value);
    if given != meta.shape {
        return reject(
            RejectCode::WrongShape,
            format!(
                "`{label}` takes {}, not {}.",
                shape_name(meta.shape),
                shape_name(given)
            ),
        );
    }
    match &cmd.value {
        LeverValue::Rate(v) => in_range(meta, i64::from(*v)),
        LeverValue::Money(b) => in_range(meta, b.get()),
        LeverValue::Number(n) => in_range(meta, *n),
        LeverValue::Choice(i) if usize::from(*i) >= meta.options.len() => reject(
            RejectCode::BadOption,
            format!("`{label}` can be {}.", meta.options.join(", ")),
        ),
        LeverValue::Choice(_) | LeverValue::Flag(_) => Ok(()),
        LeverValue::Schedule(rows) => {
            let n = u32::try_from(rows.len()).unwrap_or(u32::MAX);
            let (lo, hi) = meta.rows.unwrap_or((1, u32::MAX));
            if n < lo.max(1) || n > hi {
                return reject(
                    RejectCode::OutOfRange,
                    format!("`{label}` must have between {lo} and {hi} rows; you asked {n}."),
                );
            }
            for &(_, rate) in rows {
                in_range(meta, i64::from(rate))?;
            }
            let rises = rows.windows(2).all(|w| w[0].0 < w[1].0);
            if rows.first().map(|r| r.0) != Some(Bani::ZERO) || !rises {
                return reject(
                    RejectCode::BadSchedule,
                    format!("Brackets of `{label}` must start at 0 and rise."),
                );
            }
            Ok(())
        }
    }
}

/// `out_of_range` unless `v` is within the lever's range.
fn in_range(meta: &LeverMeta, v: i64) -> Result<(), Rejection> {
    match meta.range {
        Some((lo, hi)) if v < lo || v > hi => Err(Rejection {
            code: RejectCode::OutOfRange,
            lever: Some(meta.id),
            message: format!(
                "`{}` must be between {} and {}; you asked {}.",
                meta.label,
                show(meta.shape, lo),
                show(meta.shape, hi),
                show(meta.shape, v)
            ),
        }),
        _ => Ok(()),
    }
}

/// The EU rules a valid command may break (spec, "Warnings, not rejections").
fn eu_warnings(meta: &LeverMeta, cmd: &Command) -> Vec<Warning> {
    let floor = match cmd.lever {
        LeverId::TaxVatRateStandard => 1_500,
        LeverId::TaxVatRateReduced | LeverId::TaxVatRateReduced2 => 500,
        _ => return Vec::new(),
    };
    match cmd.value {
        LeverValue::Rate(v) if v < floor => vec![Warning {
            code: WarningCode::EuRule,
            lever: cmd.lever,
            message: format!(
                "`{}` below {} breaks the EU VAT directive's floor.",
                meta.label,
                show(Shape::Rate, i64::from(floor))
            ),
        }],
        _ => Vec::new(),
    }
}

/// A reduced VAT rate above the standard one, against the law in force.
fn consistency_warnings(meta: &LeverMeta, cmd: &Command, law: &LawInForce) -> Vec<Warning> {
    let rate = |l| match law.get(l, None) {
        Some(LeverValue::Rate(v)) => *v,
        _ => 0,
    };
    let LeverValue::Rate(v) = cmd.value else {
        return Vec::new();
    };
    let broken = match cmd.lever {
        LeverId::TaxVatRateStandard => {
            v < rate(LeverId::TaxVatRateReduced) || v < rate(LeverId::TaxVatRateReduced2)
        }
        LeverId::TaxVatRateReduced | LeverId::TaxVatRateReduced2 => {
            v > rate(LeverId::TaxVatRateStandard)
        }
        _ => false,
    };
    if broken {
        vec![Warning {
            code: WarningCode::Inconsistent,
            lever: cmd.lever,
            message: format!(
                "`{}` leaves a reduced VAT rate above the standard one.",
                meta.label
            ),
        }]
    } else {
        Vec::new()
    }
}

/// The shape a value has.
fn shape_of(v: &LeverValue) -> Shape {
    match v {
        LeverValue::Rate(_) => Shape::Rate,
        LeverValue::Money(_) => Shape::Money,
        LeverValue::Number(_) => Shape::Number,
        LeverValue::Choice(_) => Shape::Choice,
        LeverValue::Flag(_) => Shape::Flag,
        LeverValue::Schedule(_) => Shape::Schedule,
    }
}

/// A shape in words, for messages.
fn shape_name(shape: Shape) -> &'static str {
    match shape {
        Shape::Rate => "a rate",
        Shape::Money => "an amount of money",
        Shape::Number => "a number",
        Shape::Choice => "a choice",
        Shape::Flag => "on or off",
        Shape::Schedule => "a schedule",
    }
}

/// An integer of a shape in display units (a schedule's integers are rates).
fn show(shape: Shape, v: i64) -> String {
    match shape {
        Shape::Rate | Shape::Schedule => percent(v),
        Shape::Money => lei(v),
        _ => grouped(v),
    }
}

/// Basis points as a percentage: `2100` → `21%`, `375` → `3.75%`.
fn percent(bp: i64) -> String {
    let sign = if bp < 0 { "-" } else { "" };
    let a = bp.unsigned_abs();
    let (whole, frac) = (a / 100, a % 100);
    if frac == 0 {
        format!("{sign}{whole}%")
    } else if frac % 10 == 0 {
        format!("{sign}{whole}.{}%", frac / 10)
    } else {
        format!("{sign}{whole}.{frac:02}%")
    }
}

/// Bani as lei: `255000` → `2,550 lei`, `255050` → `2,550.50 lei`.
fn lei(bani: i64) -> String {
    let sign = if bani < 0 { "-" } else { "" };
    let a = bani.unsigned_abs();
    let (whole, frac) = (a / 100, a % 100);
    let digits = grouped_u(whole);
    if frac == 0 {
        format!("{sign}{digits} lei")
    } else {
        format!("{sign}{digits}.{frac:02} lei")
    }
}

/// An integer with thousands separators.
fn grouped(v: i64) -> String {
    let sign = if v < 0 { "-" } else { "" };
    format!("{sign}{}", grouped_u(v.unsigned_abs()))
}

fn grouped_u(v: u64) -> String {
    let digits = v.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A date as `2022-03-01`.
fn iso(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

/// A value in the lever's display units, e.g. `21%` or `2,550 lei`.
#[must_use]
pub fn format_value(meta: &LeverMeta, value: &LeverValue) -> String {
    match value {
        LeverValue::Rate(v) => percent(i64::from(*v)),
        LeverValue::Money(b) => lei(b.get()),
        LeverValue::Number(n) => grouped(*n),
        LeverValue::Choice(i) => meta
            .options
            .get(usize::from(*i))
            .cloned()
            .unwrap_or_else(|| format!("option {i}")),
        LeverValue::Flag(on) => if *on { "on" } else { "off" }.to_string(),
        LeverValue::Schedule(rows) => rows
            .iter()
            .map(|(t, r)| format!("{} from {}", percent(i64::from(*r)), lei(t.get())))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

impl World {
    /// Give an order today (the current day): validate it, log it and add it
    /// to the pending list.
    ///
    /// # Errors
    /// The rejection; state and log are unchanged.
    ///
    /// # Panics
    /// A world built without `law.json` ([`World::new`], not
    /// [`World::with_law`]).
    pub fn give(&mut self, order: Order) -> Result<Accepted, Rejection> {
        let today = self.date();
        let st = self.lever_state_mut().expect(NO_LAW);
        let (effective, mut warnings) = validate(&st.registry, &order, today, &st.pending)?;
        let seq = st.next_seq;
        st.next_seq += 1;
        // Pending and logged with the effective day filled (spec: the log,
        // every line with seq and effective filled), so a replay of the log
        // gives the same log.
        let order = match order {
            Order::Set(cmd) => Order::Set(Command {
                effective: Some(effective),
                ..cmd
            }),
            cancel @ Order::Cancel { .. } => cancel,
        };
        match &order {
            Order::Set(cmd) => {
                warnings.extend(consistency_warnings(
                    st.registry.get(cmd.lever),
                    cmd,
                    &st.law,
                ));
                st.pending.push(Pending {
                    seq,
                    given: today,
                    command: cmd.clone(),
                    effective,
                });
                st.events.push(PolicyEvent::Given {
                    seq,
                    lever: cmd.lever,
                    key: cmd.key,
                    given: today,
                    effective,
                });
                for w in &warnings {
                    st.events.push(PolicyEvent::Warning {
                        seq,
                        given: today,
                        warning: w.clone(),
                    });
                }
            }
            Order::Cancel { seq: target } => st.pending.retain(|p| p.seq != *target),
        }
        st.log.push(LogLine {
            seq: Some(seq),
            given: today,
            order,
            effective: Some(effective),
        });
        Ok(Accepted {
            seq,
            effective,
            warnings,
        })
    }

    /// The law in force.
    #[must_use]
    ///
    /// # Panics
    /// A world built without `law.json`.
    pub fn law(&self) -> &LawInForce {
        &self.lever_state().expect(NO_LAW).law
    }

    /// The commands accepted and not yet in force, in the order given.
    #[must_use]
    pub fn pending(&self) -> &[Pending] {
        self.lever_state().map_or(&[], |s| s.pending.as_slice())
    }

    /// Every accepted order so far, with `seq` and `effective` filled.
    #[must_use]
    pub fn command_log(&self) -> &[LogLine] {
        self.lever_state().map_or(&[], |s| s.log.as_slice())
    }
}

/// The message of a lever call on a world built without `law.json`.
const NO_LAW: &str = "a world built with World::with_law (under law.json)";

/// The lever system's part of a `World`'s state.
#[derive(Debug, Clone)]
pub(crate) struct LeverState {
    /// The registry, keyed levers' keys from the scenario.
    pub(crate) registry: Registry,
    /// The law in force.
    pub(crate) law: LawInForce,
    /// Accepted commands not yet in force, in the order given.
    pub(crate) pending: Vec<Pending>,
    /// Every accepted order.
    pub(crate) log: Vec<LogLine>,
    /// The L5 events.
    pub(crate) events: Vec<PolicyEvent>,
    /// The next order's sequence number (from 1).
    pub(crate) next_seq: u64,
}

/// The `Commands` phase: the commands due today take effect, in the order
/// given (for one lever and key the last given wins), and the law the
/// mechanics read is rebuilt.
pub(crate) fn apply_due(world: &mut World, today: Date) {
    let industries = world.industries().id.clone();
    let Some(st) = world.lever_state_mut() else {
        return;
    };
    if !st.pending.iter().any(|p| p.effective <= today) {
        return;
    }
    let mut due = Vec::new();
    st.pending.retain(|p| {
        if p.effective <= today {
            due.push(p.clone());
            false
        } else {
            true
        }
    });
    due.sort_by_key(|p| p.seq);
    for p in due {
        let c = &p.command;
        let old = st.law.get(c.lever, c.key).cloned();
        st.law.set(c);
        st.events.push(PolicyEvent::Changed(PolicyChanged {
            lever: c.lever,
            key: c.key,
            old: old.unwrap_or_else(|| c.value.clone()),
            new: c.value.clone(),
            given: p.given,
            effective: p.effective,
            seq: p.seq,
        }));
    }
    let law = law_of(&st.law, &st.registry, &industries);
    world.set_law(law);
    world.budget_refresh_pay();
    world.levers_refresh_params();
}

/// The hand-written rule structs the mechanics read, filled from the law in
/// force (`industries`: the catalogue ids, in the world's order). What no
/// lever sets (the industry overrides, the minimum income's take-up) stays
/// as in [`Law::law_2021`].
#[must_use]
pub(crate) fn law_of(f: &LawInForce, reg: &Registry, industries: &[String]) -> Law {
    use LeverId as L;
    let rate = |l| match f.get(l, None) {
        Some(LeverValue::Rate(v)) => Some(u32::try_from(*v).unwrap_or(0)),
        _ => None,
    };
    let money = |l, k| match f.get(l, k) {
        Some(LeverValue::Money(b)) => Some(*b),
        _ => None,
    };
    // A keyed lever's values with their key texts, in the registry's order.
    let keyed = |l: LeverId| -> Vec<(&str, &LeverValue)> {
        let meta = reg.get(l);
        meta.keys
            .iter()
            .enumerate()
            .filter_map(|(k, key)| {
                let v = f.get(l, u32::try_from(k).ok())?;
                Some((key.as_str(), v))
            })
            .collect()
    };
    let years = |key: &str| key.trim_end_matches('y').parse::<u8>().unwrap_or(0);

    let mut law = Law::law_2021();
    if let Some(v) = rate(L::TaxVatRateStandard) {
        law.vat.standard = econ_mech_tax::vat::VatRate(v);
    }
    if let Some(v) = rate(L::TaxVatRateReduced) {
        law.vat.reduced = econ_mech_tax::vat::VatRate(v);
    }
    if let Some(v) = rate(L::TaxVatRateReduced2) {
        law.vat.reduced_2 = econ_mech_tax::vat::VatRate(v);
    }
    let category = reg.get(L::TaxVatCategory);
    for (j, id) in industries.iter().enumerate() {
        let Some(k) = category.keys.iter().position(|k| k == id) else {
            continue;
        };
        if let Some(LeverValue::Choice(i)) = f.get(L::TaxVatCategory, u32::try_from(k).ok()) {
            use econ_mech_tax::vat::VatCategory as C;
            let c = match category.options.get(usize::from(*i)).map(String::as_str) {
                Some("standard") => C::Standard,
                Some("reduced") => C::Reduced,
                Some("reduced_2") => C::Reduced2,
                Some("zero") => C::Zero,
                Some("exempt") => C::Exempt,
                _ => continue,
            };
            if let Some(slot) = law.vat_category.get_mut(j) {
                *slot = c;
            }
        }
    }
    if let Some(v) = money(L::LabourMinimumWage, None) {
        law.minimum_wage = v;
    }
    if let Some(income) = law.income.as_mut() {
        let t = &mut income.labour;
        t.cas_bp = rate(L::TaxContribCas).unwrap_or(t.cas_bp);
        t.pillar2_bp = rate(L::TaxContribPillar2Share).unwrap_or(t.pillar2_bp);
        t.cass_bp = rate(L::TaxContribCass).unwrap_or(t.cass_bp);
        t.cam_bp = rate(L::TaxContribCam).unwrap_or(t.cam_bp);
        if let Some(LeverValue::Schedule(rows)) = f.get(L::TaxIncomeSchedule, None) {
            t.income = econ_mech_tax::income::Schedule {
                thresholds: rows.iter().map(|r| r.0).collect(),
                rates_bp: rows
                    .iter()
                    .map(|r| u32::try_from(r.1).unwrap_or(0))
                    .collect(),
            };
        }
        for (i, (_, v)) in keyed(L::TaxIncomeDeductionBase).into_iter().enumerate() {
            if let (Some(slot), LeverValue::Money(b)) = (t.deduction.base.get_mut(i), v) {
                *slot = *b;
            }
        }
        t.deduction.full_until =
            money(L::TaxIncomeDeductionFullUntil, None).unwrap_or(t.deduction.full_until);
        t.deduction.zero_at =
            money(L::TaxIncomeDeductionZeroAt, None).unwrap_or(t.deduction.zero_at);
        income.pension_tax_free =
            money(L::TaxIncomePensionTaxFree, None).unwrap_or(income.pension_tax_free);
        let p = &mut income.profit;
        p.profit_bp = rate(L::TaxProfitRate).unwrap_or(p.profit_bp);
        p.micro_threshold = money(L::TaxProfitMicroThreshold, None).unwrap_or(p.micro_threshold);
        p.micro_with_staff_bp = rate(L::TaxProfitMicroRateStaff).unwrap_or(p.micro_with_staff_bp);
        p.micro_no_staff_bp = rate(L::TaxProfitMicroRateNoStaff).unwrap_or(p.micro_no_staff_bp);
        income.dividend_bp = rate(L::TaxCapitalDividendRate).unwrap_or(income.dividend_bp);
        income.interest_bp = rate(L::TaxCapitalInterestRate).unwrap_or(income.interest_bp);
        income.contribution_ceiling =
            money(L::TaxContribCeiling, None).unwrap_or(income.contribution_ceiling);
        income.child_credit =
            money(L::TaxIncomeCreditPerChild, None).unwrap_or(income.child_credit);
    }
    if let Some(v) = rate(L::TaxPropertyRate) {
        law.property_tax_bp = Some(v);
    }
    law.wealth_tax_threshold =
        money(L::TaxWealthThreshold, None).unwrap_or(law.wealth_tax_threshold);
    law.wealth_tax_bp = rate(L::TaxWealthRate).unwrap_or(law.wealth_tax_bp);
    // `excise_duty[g]`: bani per 1,000 units (1,000 lei of the good at
    // starting prices) in the registry, millionths of the starting price per
    // unit in the law (one ban per 1,000 units is ten millionths).
    for (key, v) in keyed(L::TaxExciseDuty) {
        let (Some(j), LeverValue::Money(b)) = (industries.iter().position(|id| id == key), v)
        else {
            continue;
        };
        if let Some(slot) = law.excise_duty_ppm.get_mut(j) {
            *slot = u32::try_from(b.get().max(0).saturating_mul(10)).unwrap_or(u32::MAX);
        }
    }
    if let Some(tr) = law.transfers.as_mut() {
        tr.pension.point_value =
            money(L::TransferPensionPointValue, None).unwrap_or(tr.pension.point_value);
        tr.pension.minimum = money(L::TransferPensionMinimum, None).unwrap_or(tr.pension.minimum);
        let u = &mut tr.unemployment;
        u.isr = money(L::TransferUnemploymentIsr, None).unwrap_or(u.isr);
        u.isr_share_ppm =
            rate(L::TransferUnemploymentIsrShare).map_or(u.isr_share_ppm, |bp| bp * 100);
        let shares: Vec<(u8, u32)> = keyed(L::TransferUnemploymentWageShareByYears)
            .into_iter()
            .filter_map(|(k, v)| match v {
                LeverValue::Rate(bp) => Some((years(k), u32::try_from(*bp).unwrap_or(0) * 100)),
                _ => None,
            })
            .collect();
        if !shares.is_empty() {
            u.wage_share_by_years = shares;
        }
        let months: Vec<(u8, u8)> = keyed(L::TransferUnemploymentMonthsByYears)
            .into_iter()
            .filter_map(|(k, v)| match v {
                LeverValue::Number(n) => Some((years(k), u8::try_from(*n).unwrap_or(u8::MAX))),
                _ => None,
            })
            .collect();
        if !months.is_empty() {
            u.months_by_years = months;
        }
        tr.child.under2 = money(L::TransferChildUnder2, None).unwrap_or(tr.child.under2);
        tr.child.to18 = money(L::TransferChildTo18, None).unwrap_or(tr.child.to18);
        let floors: Vec<Bani> = keyed(L::TransferMinIncomeFloor)
            .into_iter()
            .filter_map(|(_, v)| match v {
                LeverValue::Money(b) => Some(*b),
                _ => None,
            })
            .collect();
        if !floors.is_empty() {
            tr.minimum_income.floor_by_size = floors;
        }
        tr.minimum_income.per_extra =
            money(L::TransferMinIncomePerExtra, None).unwrap_or(tr.minimum_income.per_extra);
    }
    law
}

// --- Test writer's additions (ADR-0014) ----------------------------------------
// What the acceptance tests of game/lever-system and of the labour market's
// "Pay and job choice" read that the skeleton above did not have: a world
// built under the law of `law.json` and `world.json`, the registry with the
// scenario's key tables, the L5 events, the ministry budgets of a month, the
// price "why" terms and the L4 counterfactual of the lever groups.

/// The message of every body the test writer added.
const TEST_WRITER: &str = "test writer's addition (ADR-0014): the implementation fills it";

/// An L5 event of the lever system (spec, "How a lever change is explained",
/// and "Ministry budgets").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyEvent {
    /// `PolicyGiven`: a command accepted and pending.
    Given {
        /// Its sequence number.
        seq: u64,
        /// The lever.
        lever: LeverId,
        /// The key, for a keyed lever.
        key: Option<u32>,
        /// The day it was given.
        given: Date,
        /// The day it takes effect.
        effective: Date,
    },
    /// `PolicyChanged`: a command took effect.
    Changed(PolicyChanged),
    /// `PolicyWarning`: an accepted command breaks an EU rule or a
    /// consistency the law usually keeps.
    Warning {
        /// The sequence number of the command.
        seq: u64,
        /// The day it was given.
        given: Date,
        /// The warning.
        warning: Warning,
    },
    /// A budget line's wage bill above its running allowance (spec, "Wage
    /// bill above the running allowance"): the hiring freeze's warning,
    /// naming the line, with what the treasury paid beyond the allowance in
    /// the month.
    BudgetOverrun {
        /// The day of the payment.
        date: Date,
        /// The budget line's id (`education`, `health`, …).
        line: String,
        /// Paid by the treasury beyond the allowance.
        overrun: Bani,
    },
}

/// One budget line in one closed month (spec, "Ministry budgets"), national
/// bani.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetLineMonth {
    /// The month.
    pub month: econ_types::Month,
    /// The budget line's id.
    pub line: String,
    /// $R_l$: the running allowance of the budget year to the end of this
    /// month (the sum of its monthly twelfths).
    pub running_allowance_to_date: Bani,
    /// What the line's running part has paid in the budget year to the end
    /// of this month (wages and inputs).
    pub running_paid_to_date: Bani,
    /// The wage bill paid in the month (employees × pay), overrun included.
    pub wages: Bani,
    /// Inputs bought in the month.
    pub inputs: Bani,
    /// The investment part paid in the month (`gov.investment` purchases).
    pub investment: Bani,
    /// The wage bill above the allowance paid by the treasury in the month.
    pub overrun: Bani,
}

/// One term of a "why" tree (ADR-0008 L2), flattened.
#[derive(Debug, Clone, PartialEq)]
pub struct WhyTerm {
    /// The term's label.
    pub label: String,
    /// The lever the term's input is a field of, if any.
    pub lever: Option<LeverId>,
    /// Its contribution to the change of the explained value.
    pub contribution: f64,
}

impl World {
    /// A world on a starting state under the law in force of `law.json` and
    /// `world.json`, with the registry whose keyed levers carry the
    /// scenario's key tables.
    ///
    /// # Errors
    /// As [`World::new`].
    pub fn with_law(
        start: crate::world::StartState,
        registry: Registry,
        law: LawInForce,
        params: crate::world::WorldParams,
    ) -> Result<World, crate::world::WorldError> {
        let industries = start.industry_id.clone();
        let budget = crate::world::BudgetStart {
            wage_shares: start.budget_wage_shares.clone(),
            investment_domestic: start.investment_domestic.clone(),
            investment_imported: start.investment_imported.clone(),
            gov_investment_year: start.gov_investment_year,
        };
        let mut world = World::new(start, law_of(&law, &registry, &industries), params)?;
        world.set_lever_state(LeverState {
            registry,
            law,
            pending: Vec::new(),
            log: Vec::new(),
            events: Vec::new(),
            next_seq: 1,
        });
        world.start_budget(&budget);
        world.levers_refresh_params();
        Ok(world)
    }

    /// The registry the world validates against, its keyed levers' `keys`
    /// filled from the scenario's tables.
    #[must_use]
    ///
    /// # Panics
    /// A world built without `law.json`.
    pub fn registry(&self) -> &Registry {
        &self.lever_state().expect(NO_LAW).registry
    }

    /// The L5 events of the lever system so far, in the order emitted.
    #[must_use]
    pub fn events(&self) -> &[PolicyEvent] {
        self.lever_state().map_or(&[], |s| s.events.as_slice())
    }

    /// Every budget line in every closed month, by month then line.
    #[must_use]
    pub fn budget_months(&self) -> &[BudgetLineMonth] {
        self.budget_rows()
    }

    /// The terms of the "why" tree of a catalogue industry's consumer price
    /// over a closed month.
    #[must_use]
    pub fn price_why(&self, industry: usize, month: econ_types::Month) -> Vec<WhyTerm> {
        let _ = (self, industry, month);
        unimplemented!("{TEST_WRITER}")
    }
}

/// The L4 counterfactual of the lever groups (spec, "How a lever change is
/// explained"; ADR-0008): from `fork`, run to the end of `until` once with
/// every order of `log` given on its day and once without the orders of
/// each group in `groups`; a group's effect on `indicator` is the value of
/// the full run less that of the run without its orders.
///
/// # Errors
/// An order of the log rejected.
pub fn attribute_groups(
    fork: &World,
    log: &[LogLine],
    groups: &[Group],
    until: Date,
    indicator: fn(&World) -> i64,
) -> Result<Vec<(Group, i64)>, Rejection> {
    let _ = (fork, log, groups, until, indicator);
    unimplemented!("{TEST_WRITER}")
}
