//! Scenario builder (spec `game/scenario-builder`, ADR-0018): the pipeline's
//! outputs turned into the world on the start date at one `sample_scale`, as
//! a folder of Arrow tables and small JSON files, with a consistency report.
//! `econ-cli build-scenario` calls it.
//!
//! The folder (what the acceptance tests read):
//!
//! | File | Content |
//! |---|---|
//! | `scenario.json` | `format` (1), `name`, `start_date`, `sample_scale`, `rng_seed`, `engine_version`; `stage_hashes`: `{population, attributes, jobs, housing, wages, pensions, wealth, incomes, employers}` as 16 hex digits; `opening_hash` (after rule 8, 16 hex digits); `inputs`: `{file name: SHA-256 hex}` of every input file; `files`: `{path in the folder: SHA-256 hex}` of every other file written; `law` (null); `provenance`; `consistent` |
//! | `tables/households.arrow`, `tables/persons.arrow` | the columns of stages A to J (`econ_io::popgen`) |
//! | `tables/firm_units.arrow` | [`FirmUnitTable`] |
//! | `balances.arrow` | `instrument` (u8), `sector` (u8), `balance` (i64, bani), one row per pair in matrix order |
//! | `report.json` | `checks`: one object per check with `name`, `ok`, `known_gap` and its figures; `consistent` |
//!
//! Roadmap M5's starting data, which no mechanic reads yet (specs
//! `economy/demographics`, `society/households-lifecycle`,
//! `economy/migration-remittances`, `society/education`), adds to the folder
//! without changing any of the above but the two new columns of each table:
//!
//! | File | Content |
//! |---|---|
//! | `tables/households.arrow` | also `hh_key` (u64) |
//! | `tables/persons.arrow` | also `person_key` (u64) and stage K's `inactive_kind`, `enrolled` (u8) |
//! | `tables/linked_emigrants.arrow`, `tables/diaspora_stock.arrow` | stage L (`econ_io::people`) |
//! | `demography.json` | the demographic tables, as the input file is |
//! | `scenario.json` | also `people_hashes`: `{inactivity_enrolment, linked_emigrants}`, and `next_person_key`, `next_hh_key` |

use std::collections::BTreeMap;
use std::path::Path;

use econ_ledger::{InvariantError, Ledger};
use econ_popgen::{
    HouseholdHousing, HouseholdWealth, LinkedEmigrants, OtherIncomes, PersonAttributes,
    PersonEmployers, PersonInactivityEnrolment, PersonJobs, PersonPensions, PersonWages,
    Population,
};
use serde_json::Value;

mod build;
pub mod firm_opening;
pub mod opening;
mod report;
mod sha256;
mod write;

pub use build::StageInputs;
pub use sha256::sha256_hex;

pub use firm_opening::{
    FirmOpening, FirmOpeningError, FirmOpeningMargins, FirmUnitAmounts, IndustryMargins,
    firm_opening, read_firm_opening_margins, read_firm_unit_amounts,
};
pub use opening::{
    CentralBankReserves, GovernmentDebt, OpeningBalances, OpeningError, SectorAccounts,
    SectorDetails, StatInstrument, opening_balances, read_sector_accounts, read_sector_details,
    reconcile,
};

/// The start date of every scenario.
pub const START_DATE: &str = "2021-12-01";

/// What a build depends on besides the input files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildParams {
    /// Synthetic records per real one (ADR-0003); no default.
    pub sample_scale: u32,
    /// The scenario's `rng_seed` (default 42).
    pub rng_seed: u64,
}

/// Everything read from the fixtures directory.
#[derive(Debug, Clone)]
pub struct Inputs {
    /// Every input file's name and bytes, for the SHA-256 in `scenario.json`.
    pub files: Vec<(String, Vec<u8>)>,
    /// The financial accounts (`balance_sheets2021q4_ro.json`).
    pub accounts: SectorAccounts,
    /// `sector_details2021_ro.json`.
    pub details: SectorDetails,
    /// The margin files of stages A to J.
    pub stages: StageInputs,
    /// Every input file as JSON, by its name.
    pub docs: BTreeMap<&'static str, Value>,
    /// The firm-unit table without `employees`, which needs the population.
    pub firm_units: FirmUnitTable,
}

/// Read and check every input of the spec's table.
///
/// # Errors
/// [`BuildError::Input`] naming the first file that is missing or malformed.
pub fn read_inputs(dir: &Path) -> Result<Inputs, BuildError> {
    build::read_inputs(dir)
}

/// The columns of stages C to L.
#[derive(Debug, Clone)]
pub struct StageColumns {
    /// Stage C.
    pub attributes: PersonAttributes,
    /// Stage D.
    pub jobs: PersonJobs,
    /// Stage E.
    pub housing: HouseholdHousing,
    /// Stage F.
    pub wages: PersonWages,
    /// Stage G.
    pub pensions: PersonPensions,
    /// Stage H.
    pub wealth: HouseholdWealth,
    /// Stage I.
    pub incomes: OtherIncomes,
    /// Stage J.
    pub employers: PersonEmployers,
    /// Stage K: kinds of inactivity and enrolment (roadmap M5's starting data).
    pub inactivity: PersonInactivityEnrolment,
    /// Stage L: the linked emigrants and the diaspora stock (roadmap M5).
    pub emigrants: LinkedEmigrants,
}

/// `tables/firm_units.arrow`: one row per firm unit, in the order of
/// `firm_units2021_ro.json`. Money in bani.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct FirmUnitTable {
    pub unit_id: Vec<String>,
    /// Catalogue index.
    pub industry: Vec<u8>,
    pub industry_group: Vec<u8>,
    /// Micro 0, small 1, large 2, public 3.
    pub size: Vec<u8>,
    /// Cohort 0, public 1.
    pub kind: Vec<u8>,
    /// Domestic 0, foreign 1, state 2.
    pub controller: Vec<u8>,
    pub firm_count: Vec<u32>,
    pub persons_employed: Vec<u64>,
    pub output: Vec<u64>,
    pub value_added: Vec<u64>,
    pub wages: Vec<u64>,
    /// Millionths.
    pub own_state: Vec<u32>,
    pub own_domestic_private: Vec<u32>,
    pub own_foreign: Vec<u32>,
    pub fixed_assets: Vec<u64>,
    pub currency: Vec<u64>,
    pub deposits: Vec<u64>,
    pub debt_securities_held: Vec<u64>,
    pub loans_granted: Vec<u64>,
    pub equity_held: Vec<u64>,
    pub insurance: Vec<u64>,
    pub other_receivable: Vec<u64>,
    pub debt_securities_issued: Vec<u64>,
    pub loans: Vec<u64>,
    pub equity_issued: Vec<u64>,
    pub other_payable: Vec<u64>,
    /// Weighted persons of the population whose `employer` is the unit.
    pub employees: Vec<u64>,
}

/// One check of the consistency report.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    /// The check's name in the spec's table.
    pub name: &'static str,
    /// Whether it passes.
    pub ok: bool,
    /// Why a check is expected to fail for now.
    pub known_gap: Option<&'static str>,
    /// Its figures (amounts in whole lei) and tolerance.
    pub facts: Value,
}

/// The consistency report.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// In the order of the spec's table.
    pub checks: Vec<Check>,
}

impl Report {
    /// Every check passes or is a known gap.
    #[must_use]
    pub fn consistent(&self) -> bool {
        self.checks.iter().all(|c| c.ok || c.known_gap.is_some())
    }

    /// The check of that name.
    #[must_use]
    pub fn check(&self, name: &str) -> Option<&Check> {
        self.checks.iter().find(|c| c.name == name)
    }
}

/// The built scenario, before it is written.
#[derive(Debug, Clone)]
pub struct Scenario {
    /// What it was built with.
    pub params: BuildParams,
    /// Stages A and B.
    pub population: Population,
    /// Stages C to J.
    pub stages: StageColumns,
    /// The firm-unit table.
    pub firm_units: FirmUnitTable,
    /// The opening balance sheet after rule 8.
    pub opening: OpeningBalances,
    /// A ledger after posting `opening`.
    pub ledger: Ledger,
    /// The consistency report.
    pub report: Report,
}

impl Scenario {
    /// The hashes of stages K and L (roadmap M5's starting data), under their
    /// names in `scenario.json`'s `people_hashes`.
    #[must_use]
    pub fn people_hashes(&self) -> Vec<(&'static str, u64)> {
        vec![
            ("inactivity_enrolment", self.stages.inactivity.state_hash()),
            ("linked_emigrants", self.stages.emigrants.state_hash()),
        ]
    }

    /// The hash of every stage, under its name in `scenario.json`, in stage
    /// order (stages A to J; K and L are in [`Scenario::people_hashes`]).
    #[must_use]
    pub fn stage_hashes(&self) -> Vec<(&'static str, u64)> {
        let s = &self.stages;
        vec![
            ("population", self.population.state_hash()),
            ("attributes", s.attributes.state_hash()),
            ("jobs", s.jobs.state_hash()),
            ("housing", s.housing.state_hash()),
            ("wages", s.wages.state_hash()),
            ("pensions", s.pensions.state_hash()),
            ("wealth", s.wealth.state_hash()),
            ("incomes", s.incomes.state_hash()),
            ("employers", s.employers.state_hash()),
        ]
    }
}

/// Why a build failed.
#[derive(Debug, Clone, PartialEq)]
pub enum BuildError {
    /// An input file is missing or malformed; nothing is written.
    Input {
        /// The file's name.
        file: String,
        /// What is wrong.
        why: String,
    },
    /// A stage rejected its margins.
    Stage(String),
    /// The opening balance sheet could not be built.
    Opening(OpeningError),
    /// The opening ledger fails its invariants (exit 3); nothing is written.
    Invariants(Vec<InvariantError>),
    /// A file could not be written.
    Io(String),
}

impl BuildError {
    /// The exit status of `econ-cli build-scenario` for this error: 3 for
    /// [`BuildError::Invariants`], 2 for an input that cannot be read (as a
    /// bad argument), 1 otherwise.
    #[must_use]
    pub fn exit_status(&self) -> u8 {
        match self {
            BuildError::Invariants(_) => 3,
            BuildError::Input { .. } => 2,
            _ => 1,
        }
    }
}

/// Steps 2 to 5: stages A to J, the firm-unit table, the opening balances
/// (rule 8 included), posted into a new ledger, and the report.
///
/// # Errors
/// See [`BuildError`].
pub fn build(inputs: &Inputs, params: &BuildParams) -> Result<Scenario, BuildError> {
    build_with_hook(inputs, params, &|_| {})
}

/// [`build`], with a test hook that may change the opening balances after
/// rule 8 and before they are posted and checked (AC-SCB-08).
///
/// # Errors
/// See [`BuildError`].
pub fn build_with_hook(
    inputs: &Inputs,
    params: &BuildParams,
    hook: &dyn Fn(&mut OpeningBalances),
) -> Result<Scenario, BuildError> {
    build::build_with_hook(inputs, params, hook)
}

/// Write the folder: every file into a temporary folder next to `dir`, then
/// renamed, so a failed write leaves no partial scenario and an existing
/// `dir` is replaced only after success.
///
/// # Errors
/// [`BuildError::Io`].
pub fn write(scenario: &Scenario, inputs: &Inputs, dir: &Path) -> Result<(), BuildError> {
    write::write(scenario, inputs, dir)
}

/// What `econ-cli build-scenario` does: read, build, write. The report is
/// returned whether or not it is consistent (exit 0 or 1).
///
/// # Errors
/// See [`BuildError`]; nothing is written.
pub fn run(fixtures: &Path, params: &BuildParams, out: &Path) -> Result<Report, BuildError> {
    run_with_hook(fixtures, params, out, &|_| {})
}

/// [`run`] with the test hook of [`build_with_hook`].
///
/// # Errors
/// See [`BuildError`]; nothing is written.
pub fn run_with_hook(
    fixtures: &Path,
    params: &BuildParams,
    out: &Path,
    hook: &dyn Fn(&mut OpeningBalances),
) -> Result<Report, BuildError> {
    let inputs = read_inputs(fixtures)?;
    let scenario = build_with_hook(&inputs, params, hook)?;
    write(&scenario, &inputs, out)?;
    Ok(scenario.report)
}
