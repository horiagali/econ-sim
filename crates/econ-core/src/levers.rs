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

use crate::world::{PhaseId, World};

/// The message of every body of this skeleton.
const LATER: &str = "ADR-0014: implemented after the acceptance tests";

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

/// The law in force: one value per lever (and key). The generator replaces
/// the body with one typed field per lever, which the mechanics read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawInForce {
    _generated: (),
}

impl LawInForce {
    /// The law of `law.json` and `world.json`: every lever of the registry
    /// set once, as start-date lines.
    ///
    /// # Errors
    /// A line rejected, or an active lever (or key) missing, named (AC-LEV-02).
    pub fn from_lines(registry: &Registry, lines: &[LogLine]) -> Result<LawInForce, Rejection> {
        let _ = (registry, lines);
        unimplemented!("{LATER}")
    }

    /// The value in force of a lever, for a key of a keyed lever.
    #[must_use]
    pub fn get(&self, lever: LeverId, key: Option<u32>) -> Option<&LeverValue> {
        let _ = (self, lever, key);
        unimplemented!("{LATER}")
    }

    /// Apply a validated command; changes nothing else.
    pub fn set(&mut self, cmd: &Command) {
        let _ = (self, cmd);
        unimplemented!("{LATER}")
    }
}

/// The earliest day a command of `meta` given on `given` can take effect.
#[must_use]
pub fn earliest_effective(meta: &LeverMeta, given: Date) -> Date {
    let _ = (meta, given);
    unimplemented!("{LATER}")
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
    let _ = (registry, order, today, pending);
    unimplemented!("{LATER}")
}

/// A value in the lever's display units, e.g. `21%` or `2,550 lei`.
#[must_use]
pub fn format_value(meta: &LeverMeta, value: &LeverValue) -> String {
    let _ = (meta, value);
    unimplemented!("{LATER}")
}

impl World {
    /// Give an order today (the current day): validate it, log it and add it
    /// to the pending list.
    ///
    /// # Errors
    /// The rejection; state and log are unchanged.
    pub fn give(&mut self, order: Order) -> Result<Accepted, Rejection> {
        let _ = (self, order);
        unimplemented!("{LATER}")
    }

    /// The law in force.
    #[must_use]
    pub fn law(&self) -> &LawInForce {
        let _ = self;
        unimplemented!("{LATER}")
    }

    /// The commands accepted and not yet in force, in the order given.
    #[must_use]
    pub fn pending(&self) -> &[Pending] {
        let _ = self;
        unimplemented!("{LATER}")
    }

    /// Every accepted order so far, with `seq` and `effective` filled.
    #[must_use]
    pub fn command_log(&self) -> &[LogLine] {
        let _ = self;
        unimplemented!("{LATER}")
    }
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
        let _ = (start, registry, law, params);
        unimplemented!("{TEST_WRITER}")
    }

    /// The registry the world validates against, its keyed levers' `keys`
    /// filled from the scenario's tables.
    #[must_use]
    pub fn registry(&self) -> &Registry {
        let _ = self;
        unimplemented!("{TEST_WRITER}")
    }

    /// The L5 events of the lever system so far, in the order emitted.
    #[must_use]
    pub fn events(&self) -> &[PolicyEvent] {
        let _ = self;
        unimplemented!("{TEST_WRITER}")
    }

    /// Every budget line in every closed month, by month then line.
    #[must_use]
    pub fn budget_months(&self) -> &[BudgetLineMonth] {
        let _ = self;
        unimplemented!("{TEST_WRITER}")
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
