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
//! are `python/pipeline/fixtures/law2021_ro.json`. [`LeverId`] and the typed
//! fields of [`LawInForce`] are to be generated from the registry by
//! `scripts/codegen_levers.py` into `levers_gen.rs`; until the generator
//! exists, [`LeverId`] is written here by hand, one variant per registry entry
//! in registry order.

use std::collections::BTreeMap;
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

/// The lever ids, one per registry entry, in registry order. To be generated
/// from `levers.toml` (`levers_gen.rs`); written by hand until the generator
/// exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LeverId {
    /// `tax.income.schedule`
    TaxIncomeSchedule,
    /// `tax.income.deduction_base`
    TaxIncomeDeductionBase,
    /// `tax.income.deduction_full_until`
    TaxIncomeDeductionFullUntil,
    /// `tax.income.deduction_zero_at`
    TaxIncomeDeductionZeroAt,
    /// `tax.income.credit_per_child`
    TaxIncomeCreditPerChild,
    /// `tax.income.pension_tax_free`
    TaxIncomePensionTaxFree,
    /// `tax.capital.dividend_rate`
    TaxCapitalDividendRate,
    /// `tax.capital.interest_rate`
    TaxCapitalInterestRate,
    /// `tax.profit.rate`
    TaxProfitRate,
    /// `tax.profit.micro_rate_staff`
    TaxProfitMicroRateStaff,
    /// `tax.profit.micro_rate_no_staff`
    TaxProfitMicroRateNoStaff,
    /// `tax.profit.micro_threshold`
    TaxProfitMicroThreshold,
    /// `tax.vat.rate_standard`
    TaxVatRateStandard,
    /// `tax.vat.rate_reduced`
    TaxVatRateReduced,
    /// `tax.vat.rate_reduced_2`
    TaxVatRateReduced2,
    /// `tax.vat.category`
    TaxVatCategory,
    /// `tax.contrib.cas`
    TaxContribCas,
    /// `tax.contrib.pillar2_share`
    TaxContribPillar2Share,
    /// `tax.contrib.cass`
    TaxContribCass,
    /// `tax.contrib.cam`
    TaxContribCam,
    /// `tax.contrib.ceiling`
    TaxContribCeiling,
    /// `tax.property.rate`
    TaxPropertyRate,
    /// `tax.wealth.threshold`
    TaxWealthThreshold,
    /// `tax.wealth.rate`
    TaxWealthRate,
    /// `tax.excise.duty`
    TaxExciseDuty,
    /// `spend.ministry_budget`
    SpendMinistryBudget,
    /// `spend.investment_share`
    SpendInvestmentShare,
    /// `spend.public_pay`
    SpendPublicPay,
    /// `spend.public_headcount`
    SpendPublicHeadcount,
    /// `spend.new_issue_years`
    SpendNewIssueYears,
    /// `transfer.pension.point_value`
    TransferPensionPointValue,
    /// `transfer.pension.minimum`
    TransferPensionMinimum,
    /// `transfer.pension.retirement_age`
    TransferPensionRetirementAge,
    /// `transfer.pension.indexation`
    TransferPensionIndexation,
    /// `transfer.unemployment.isr`
    TransferUnemploymentIsr,
    /// `transfer.unemployment.isr_share`
    TransferUnemploymentIsrShare,
    /// `transfer.unemployment.wage_share_by_years`
    TransferUnemploymentWageShareByYears,
    /// `transfer.unemployment.months_by_years`
    TransferUnemploymentMonthsByYears,
    /// `transfer.child.under2`
    TransferChildUnder2,
    /// `transfer.child.to18`
    TransferChildTo18,
    /// `transfer.min_income.floor`
    TransferMinIncomeFloor,
    /// `transfer.min_income.per_extra`
    TransferMinIncomePerExtra,
    /// `labour.minimum_wage`
    LabourMinimumWage,
    /// `money.inflation_target`
    MoneyInflationTarget,
    /// `money.policy_rate`
    MoneyPolicyRate,
    /// `money.reserve_requirement`
    MoneyReserveRequirement,
    /// `money.qe`
    MoneyQe,
    /// `money.monetary_financing`
    MoneyMonetaryFinancing,
    /// `fx.regime`
    FxRegime,
    /// `fx.intervention`
    FxIntervention,
    /// `informal.inspection_budget`
    InformalInspectionBudget,
    /// `informal.audit_budget`
    InformalAuditBudget,
    /// `informal.penalty_multiple`
    InformalPenaltyMultiple,
    /// `world.foreign_price_growth`
    WorldForeignPriceGrowth,
    /// `world.fuel_price`
    WorldFuelPrice,
    /// `world.foreign_demand_growth`
    WorldForeignDemandGrowth,
    /// `world.foreign_rate`
    WorldForeignRate,
    /// `world.wages_abroad`
    WorldWagesAbroad,
}

impl LeverId {
    /// Every lever, in registry order.
    pub const ALL: [LeverId; 58] = [
        LeverId::TaxIncomeSchedule,
        LeverId::TaxIncomeDeductionBase,
        LeverId::TaxIncomeDeductionFullUntil,
        LeverId::TaxIncomeDeductionZeroAt,
        LeverId::TaxIncomeCreditPerChild,
        LeverId::TaxIncomePensionTaxFree,
        LeverId::TaxCapitalDividendRate,
        LeverId::TaxCapitalInterestRate,
        LeverId::TaxProfitRate,
        LeverId::TaxProfitMicroRateStaff,
        LeverId::TaxProfitMicroRateNoStaff,
        LeverId::TaxProfitMicroThreshold,
        LeverId::TaxVatRateStandard,
        LeverId::TaxVatRateReduced,
        LeverId::TaxVatRateReduced2,
        LeverId::TaxVatCategory,
        LeverId::TaxContribCas,
        LeverId::TaxContribPillar2Share,
        LeverId::TaxContribCass,
        LeverId::TaxContribCam,
        LeverId::TaxContribCeiling,
        LeverId::TaxPropertyRate,
        LeverId::TaxWealthThreshold,
        LeverId::TaxWealthRate,
        LeverId::TaxExciseDuty,
        LeverId::SpendMinistryBudget,
        LeverId::SpendInvestmentShare,
        LeverId::SpendPublicPay,
        LeverId::SpendPublicHeadcount,
        LeverId::SpendNewIssueYears,
        LeverId::TransferPensionPointValue,
        LeverId::TransferPensionMinimum,
        LeverId::TransferPensionRetirementAge,
        LeverId::TransferPensionIndexation,
        LeverId::TransferUnemploymentIsr,
        LeverId::TransferUnemploymentIsrShare,
        LeverId::TransferUnemploymentWageShareByYears,
        LeverId::TransferUnemploymentMonthsByYears,
        LeverId::TransferChildUnder2,
        LeverId::TransferChildTo18,
        LeverId::TransferMinIncomeFloor,
        LeverId::TransferMinIncomePerExtra,
        LeverId::LabourMinimumWage,
        LeverId::MoneyInflationTarget,
        LeverId::MoneyPolicyRate,
        LeverId::MoneyReserveRequirement,
        LeverId::MoneyQe,
        LeverId::MoneyMonetaryFinancing,
        LeverId::FxRegime,
        LeverId::FxIntervention,
        LeverId::InformalInspectionBudget,
        LeverId::InformalAuditBudget,
        LeverId::InformalPenaltyMultiple,
        LeverId::WorldForeignPriceGrowth,
        LeverId::WorldFuelPrice,
        LeverId::WorldForeignDemandGrowth,
        LeverId::WorldForeignRate,
        LeverId::WorldWagesAbroad,
    ];

    /// The stable dotted id, e.g. `tax.vat.rate_standard`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        let _ = self;
        unimplemented!("{LATER}")
    }

    /// The lever of a dotted id, if the registry has it.
    #[must_use]
    pub fn from_id(id: &str) -> Option<LeverId> {
        let _ = id;
        unimplemented!("{LATER}")
    }
}

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
        let _ = text;
        unimplemented!("{LATER}")
    }

    /// Read and parse `levers.toml`.
    ///
    /// # Errors
    /// The file cannot be read, or [`Registry::parse`] fails.
    pub fn load(path: &Path) -> Result<Registry, RegistryError> {
        let _ = path;
        unimplemented!("{LATER}")
    }

    /// The entry of a lever.
    #[must_use]
    pub fn get(&self, lever: LeverId) -> &LeverMeta {
        let _ = (self, lever);
        unimplemented!("{LATER}")
    }

    /// The entry of a dotted id, if any.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&LeverMeta> {
        let _ = (self, id);
        unimplemented!("{LATER}")
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
        let _ = (self, spec_ids, glossary_names, scenario_key_tables);
        unimplemented!("{LATER}")
    }
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
        let _ = self;
        unimplemented!("{LATER}")
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
