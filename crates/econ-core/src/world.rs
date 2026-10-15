//! The real `World` (spec `game/world-loop`, roadmap M2): a scenario loaded
//! into column tables, advanced one day per [`World::step`], the phases of
//! the day run in their fixed order when their period is due
//! ([`crate::schedule`]), every leu moved through the ledger and the
//! invariants checked every day.
//!
//! **Skeleton** (ADR-0014): the API of the spec's sketch, written with the
//! acceptance tests (`tests/acceptance/`, feature `world`); the bodies come
//! with the implementation. Beyond the sketch, for the tests: the read-only
//! column tables ([`HouseholdColumns`], [`PersonColumns`],
//! [`FirmUnitColumns`], [`IndustryColumns`]), [`WorldParams::spec_defaults`],
//! [`World::inflation_target_ppm`], and [`StartEdit`], the edits of a
//! starting state by which two worlds are compared where a criterion needs a
//! change that M2 has no lever for.
//!
//! Money columns are whole bani per record (a household's amounts before its
//! `hh_weight`; a firm unit's are already totals). Quantities are real units
//! (lei at starting prices); prices and shares are millionths.

use econ_ledger::{InvariantError, Ledger};
use econ_mech_households::HouseholdParams;
use econ_mech_labour::LabourParams;
use econ_mech_prices::PriceParams;
use econ_mech_production::ProductionParams;
use econ_mech_tax::vat::{VatCategory, VatSchedule};
use econ_popgen::EduLevel;
use econ_types::{Bani, Date, Day, Month};

use crate::Command;

/// What a `World` starts from: the population and stage columns, the firm-unit
/// table, the opening balances, the input coefficients, the basket shares and
/// the placeholder final demand. Read from a scenario folder by
/// `econ_io::read_start_state`.
#[derive(Debug, Clone)]
pub struct StartState {
    // The implementation holds the tables here.
    _unbuilt: (),
}

/// A change to a starting state, applied by [`World::new`] after it has
/// derived the starting values of the spec ("Starting the clock" and the
/// mechanics' starting states), so that a test can compare two worlds that
/// differ only in it. Industry indices are catalogue indices, as in
/// [`IndustryColumns`]; household indices are those of [`HouseholdColumns`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartEdit {
    /// A one-off transfer: each household receives this amount (per record,
    /// before its weight) on day 0 before the first phase, paid by the
    /// government as pensions are (the three legs of the spec) and counted in
    /// its `hh_income_received` (AC-HH-05).
    Transfer(Vec<Bani>),
    /// Every unit of the industry starts with no inventory and no capacity:
    /// it neither produces nor sells (AC-HH-07).
    ShutIndustry(usize),
    /// Every unit of the industry starts with its inventory times
    /// `factor_ppm` / 1,000,000; its inventory target ratio is unchanged
    /// (AC-PRICE-07).
    ScaleInventory {
        /// Catalogue index.
        industry: usize,
        /// Millionths.
        factor_ppm: u32,
    },
    /// Every unit's labour requirement per unit of output times
    /// `factor_ppm` / 1,000,000, for the whole run: its desired employment
    /// rises in that proportion (AC-LM-06).
    ScaleLabourRequirement {
        /// Millionths.
        factor_ppm: u32,
    },
    /// The household basket shares by catalogue industry, in millionths
    /// summing to 1,000,000, in place of the input-output table's (AC-PROD-07).
    BasketShares(Vec<u32>),
    /// Every person's wage and declared wage, and every reference wage, times
    /// `factor_ppm` / 1,000,000, after the starting markups and unit costs are
    /// set: a wage rise on day 0 (AC-PRICE-05).
    ScaleWages {
        /// Millionths.
        factor_ppm: u32,
    },
}

impl StartState {
    /// Record an edit; [`World::new`] applies the edits in the order given
    /// and refuses an edit that does not fit the tables with
    /// [`WorldError::Shape`].
    pub fn apply(&mut self, edit: StartEdit) {
        let _ = edit;
        unimplemented!("game/world-loop: edits of a starting state")
    }
}

/// The law in force. Until `law.json` exists (M4) the M2 tests pass
/// [`Law::placeholder_2021`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Law {
    /// The VAT rates.
    pub vat: VatSchedule,
    /// Each catalogue industry's good's VAT category.
    pub vat_category: Vec<VatCategory>,
    /// The gross minimum wage a month.
    pub minimum_wage: Bani,
}

impl Law {
    /// The law of 1 December 2021: VAT 19%, 9% and 5% with each good's category
    /// from the law in force; a minimum wage of 2,300 lei.
    #[must_use]
    pub fn placeholder_2021() -> Law {
        unimplemented!("game/world-loop: the placeholder law")
    }
}

/// The four mechanics' parameters and the seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldParams {
    /// Households.
    pub households: HouseholdParams,
    /// Labour market.
    pub labour: LabourParams,
    /// Production.
    pub production: ProductionParams,
    /// Prices.
    pub prices: PriceParams,
    /// Master seed of the keyed draws (the scenario's `rng_seed`).
    pub rng_seed: u64,
}

impl WorldParams {
    /// Every parameter at its spec's default, with this seed.
    #[must_use]
    pub fn spec_defaults(rng_seed: u64) -> WorldParams {
        let _ = rng_seed;
        unimplemented!("game/world-loop: the mechanics' default parameters")
    }
}

/// The phases of a day, in the order they run (the spec's table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum PhaseId {
    Commands,
    Plan,
    Staffing,
    Prices,
    OutputAndInputs,
    FixedDemand,
    Households,
    GoodsMarket,
    Quits,
    Pay,
    WageRound,
    MonthlyClose,
    Checks,
}

/// What a day returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayReport {
    /// The day that ran.
    pub day: Day,
    /// Its date.
    pub date: Date,
    /// The phases that ran, in order.
    pub phases: Vec<PhaseId>,
    /// Every check of the checks phase held.
    pub checks_ok: bool,
    /// The checks that failed.
    pub failed: Vec<InvariantError>,
    /// The state hash after the day.
    pub state_hash: u64,
}

/// The statistics of a closed month. Money is national (weighted) bani.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthStats {
    /// The month.
    pub month: Month,
    /// Consumer price index, millionths (1,000,000 at the start).
    pub cpi_ppm: u32,
    /// Twelve-month inflation (over the starting CPI, annualised, in the first
    /// year), millionths.
    pub inflation_ppm: i32,
    /// Expected inflation after the close, millionths a year.
    pub expected_inflation_ppm: i32,
    /// Employed persons, weighted.
    pub employed: u64,
    /// Unemployed persons, weighted.
    pub unemployed: u64,
    /// Wages paid.
    pub wage_bill: Bani,
    /// Household consumption, VAT included.
    pub consumption: Bani,
    /// GDP by production.
    pub gdp_production: Bani,
    /// GDP by expenditure.
    pub gdp_expenditure: Bani,
    /// GDP by income.
    pub gdp_income: Bani,
    /// Shortages of the month by catalogue industry, real units.
    pub shortages: Vec<i64>,
    /// Households in hardship, weighted.
    pub hardship_households: u64,
}

/// Why a world was not built or a day was refused. Nothing changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldError {
    /// The tables disagree with the opening ledger (I-4 at day 0).
    TablesDisagreeWithLedger {
        /// Which balance.
        what: &'static str,
    },
    /// Commands are not accepted before the lever registry (M4).
    CommandsNotYet,
    /// The tables do not fit together.
    Shape(&'static str),
}

/// The households table, aligned with the scenario's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HouseholdColumns {
    /// Real households represented.
    pub hh_weight: Vec<u32>,
    /// Currency.
    pub hh_cash: Vec<Bani>,
    /// Bank deposits.
    pub hh_deposits: Vec<Bani>,
    /// Money received since the last decision day.
    pub hh_income_received: Vec<Bani>,
    /// The day of the month of the household's decision, 1 to 28.
    pub hh_stagger_day: Vec<u8>,
    /// The household could not pay for the subsistence basket at its last decision.
    pub hh_hardship: Vec<bool>,
    /// Income fifth by last month's income per equivalised adult (0 = poorest).
    pub hh_fifth: Vec<u8>,
    /// The day of its last decision; `None` before the first.
    pub last_decision: Vec<Option<Day>>,
    /// Everything received since day 0 (AC-HH-03).
    pub income_to_date: Vec<Bani>,
    /// Everything spent since day 0, VAT included (AC-HH-03).
    pub spending_to_date: Vec<Bani>,
}

/// The persons table, aligned with the scenario's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonColumns {
    /// Index into the households table.
    pub household_id: Vec<u32>,
    /// Highest completed education.
    pub edu_level: Vec<EduLevel>,
    /// Firm unit, or `econ_popgen::NO_EMPLOYER`.
    pub employer: Vec<u16>,
    /// ISCO-08 major group, or `econ_popgen::NOT_EMPLOYED`.
    pub occupation: Vec<u8>,
    /// Gross wage a month; 0 for a person without a job.
    pub wage: Vec<Bani>,
    /// Its declared part.
    pub wage_declared: Vec<Bani>,
    /// Months since the person lost a job or started looking.
    pub months_unemployed: Vec<u16>,
    /// The day the person's current job started; `None` for a job held since
    /// day 0 or no job.
    pub hired_on: Vec<Option<Day>>,
    /// The wage paid to the person at the latest pay day.
    pub last_pay: Vec<Bani>,
}

/// The firm units, aligned with the scenario's firm-unit table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FirmUnitColumns {
    /// Catalogue industry.
    pub industry: Vec<u8>,
    /// The day of the month of the unit's plan and staffing, 1 to 28.
    pub unit_stagger_day: Vec<u8>,
    /// Weighted persons whose `employer` is the unit.
    pub staff: Vec<u64>,
    /// Persons wanted by the last plan.
    pub desired_employment: Vec<u64>,
    /// Expected sales a month.
    pub expected_demand: Vec<i64>,
    /// Maximum output a month.
    pub capacity: Vec<i64>,
    /// Finished-goods inventory.
    pub inventory: Vec<i64>,
    /// Months of expected demand the unit aims to hold, millionths.
    pub inventory_target_ppm: Vec<u32>,
    /// Sold since the last plan.
    pub sales_since_plan: Vec<i64>,
    /// Produced on the latest day.
    pub output_today: Vec<i64>,
    /// Sold on the latest day.
    pub sales_today: Vec<i64>,
    /// Received out of its industry's sales clearing account on the latest day.
    pub revenue_today: Vec<Bani>,
    /// Currency.
    pub currency: Vec<Bani>,
    /// Bank deposits.
    pub deposits: Vec<Bani>,
    /// The day of its last plan; `None` before the first.
    pub last_plan: Vec<Option<Day>>,
    /// The day of its last staffing; `None` before the first.
    pub last_staffing: Vec<Option<Day>>,
}

/// The catalogue industries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndustryColumns {
    /// Catalogue id (`agri_cereals`, `cars`, …).
    pub id: Vec<String>,
    /// Industry group (the ten NACE groups of the population's jobs).
    pub group: Vec<u8>,
    /// Household basket share, millionths; also the CPI weight.
    pub basket_share_ppm: Vec<u32>,
    /// Price, millionths of the starting price.
    pub price_ppm: Vec<u32>,
    /// Consumer price, VAT included, millionths of the starting consumer price.
    pub consumer_price_ppm: Vec<u32>,
    /// Demand on the latest day, real units.
    pub demand_today: Vec<i64>,
    /// Demand not served on the latest day, real units.
    pub shortage_today: Vec<i64>,
    /// What the latest day's buyers paid for the industry's goods, VAT
    /// excluded, into its sales clearing account.
    pub sales_value_today: Vec<Bani>,
}

/// The simulated economy.
#[derive(Debug, Clone)]
pub struct World {
    // The implementation holds the calendar, today, the column tables, the
    // ledger and the macro state here.
    _unbuilt: (),
}

impl World {
    /// Load a starting state under a law (AC-WLD-08: refuses tables that
    /// disagree with the opening ledger, building nothing).
    ///
    /// # Errors
    /// [`WorldError::TablesDisagreeWithLedger`], [`WorldError::Shape`].
    pub fn new(start: StartState, law: Law, params: WorldParams) -> Result<World, WorldError> {
        let _ = (start, law, params);
        unimplemented!("game/world-loop: load a world")
    }

    /// Run today's phases and move to the next day. `commands` must be empty
    /// in M2 (AC-WLD-09).
    ///
    /// # Errors
    /// [`WorldError::CommandsNotYet`]; nothing changes.
    pub fn step(&mut self, commands: &[Command]) -> Result<DayReport, WorldError> {
        let _ = commands;
        unimplemented!("game/world-loop: one day")
    }

    /// The date of [`World::day`].
    #[must_use]
    pub fn date(&self) -> Date {
        unimplemented!("game/world-loop: today's date")
    }

    /// The day the next [`World::step`] runs (day 0 after [`World::new`]).
    #[must_use]
    pub fn day(&self) -> Day {
        unimplemented!("game/world-loop: today")
    }

    /// The hash of the whole state.
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("game/world-loop: the state hash")
    }

    /// The ledger.
    #[must_use]
    pub fn ledger(&self) -> &Ledger {
        unimplemented!("game/world-loop: the ledger")
    }

    /// The statistics of every closed month, in order.
    #[must_use]
    pub fn months(&self) -> &[MonthStats] {
        unimplemented!("game/world-loop: the monthly statistics")
    }

    /// The households table.
    #[must_use]
    pub fn households(&self) -> &HouseholdColumns {
        unimplemented!("game/world-loop: the households table")
    }

    /// The persons table.
    #[must_use]
    pub fn persons(&self) -> &PersonColumns {
        unimplemented!("game/world-loop: the persons table")
    }

    /// The firm units.
    #[must_use]
    pub fn firm_units(&self) -> &FirmUnitColumns {
        unimplemented!("game/world-loop: the firm units")
    }

    /// The industries.
    #[must_use]
    pub fn industries(&self) -> &IndustryColumns {
        unimplemented!("game/world-loop: the industries")
    }

    /// The inflation target, millionths a year (the starting expected
    /// inflation).
    #[must_use]
    pub fn inflation_target_ppm(&self) -> i32 {
        unimplemented!("game/world-loop: the inflation target")
    }
}
