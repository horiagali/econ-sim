//! Stage J — employers (spec `society/population-employers`).
//!
//! Gives every employed person a catalogue industry and a firm unit, inside
//! the industry group that the jobs stage gave them, so that each unit has
//! its share of the group's employees and better-paid employees work in
//! better-paying units. Earlier stages are not changed.
//!
//! **Skeleton.** This file fixes the API the acceptance tests are written
//! against (ADR-0014, tests-first). The stage is not implemented yet; the
//! `employers` feature that enables its acceptance tests is switched on by
//! the implementation change. `python/reference/popgen_reference.py`
//! (`assign_employers`) is the independent reference.

use crate::Population;
use crate::jobs::PersonJobs;
use crate::wages::PersonWages;

/// `industry` of a person who is not employed.
pub const NO_INDUSTRY: u8 = 255;
/// `employer` of a person who is not employed.
pub const NO_EMPLOYER: u16 = 65535;

/// Size class of a firm unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum UnitSize {
    /// Cohort of firms with fewer than 10 persons employed.
    Micro = 0,
    /// Cohort of firms with 10 to 49.
    Small = 1,
    /// Cohort of firms with 50 or more.
    Large = 2,
    /// Non-market public producers.
    Public = 3,
}

/// What the stage needs to know of a firm unit (ADR-0016).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmUnit {
    /// Index of its industry in [`FirmUnits::industry_group`].
    pub industry: u8,
    /// Size class.
    pub size: UnitSize,
    /// The unit is controlled from Romania and is not a public unit.
    pub domestic: bool,
    /// Persons employed by the unit (all its firms together).
    pub persons_employed: u64,
    /// Its wage bill, in lei a year.
    pub wages: u64,
}

/// The firm units of the starting economy, as far as this stage reads them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmUnits {
    /// The industry group of each catalogue industry, in the catalogue's order.
    pub industry_group: Vec<u8>,
    /// The units.
    pub units: Vec<FirmUnit>,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmployerParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// How closely an employee's place among employers follows the wage, in millionths.
    pub wage_link_ppm: u32,
}

impl EmployerParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        EmployerParams {
            rng_seed,
            wage_link_ppm: 500_000,
        }
    }
}

/// The columns of the stage, aligned with the persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonEmployers {
    /// Catalogue industry, or [`NO_INDUSTRY`].
    pub industry: Vec<u8>,
    /// Firm unit, or [`NO_EMPLOYER`].
    pub employer: Vec<u16>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmployerError {
    /// No industries or no units.
    EmptyTable,
    /// A unit of an industry that is not listed, more industries or units
    /// than the columns can hold, or columns of another population.
    ShapeMismatch,
    /// Someone works in an industry group that has no unit.
    NoMargin {
        /// The industry group.
        industry_group: u8,
    },
}

/// Assign a catalogue industry and a firm unit to every employed person.
/// Without wages, employees are placed by the draw alone.
///
/// # Errors
/// See [`EmployerError`].
pub fn assign_employers(
    pop: &Population,
    jobs: &PersonJobs,
    wages: Option<&PersonWages>,
    firms: &FirmUnits,
    params: &EmployerParams,
) -> Result<PersonEmployers, EmployerError> {
    let _ = (pop, jobs, wages, firms, params);
    unimplemented!("econ-popgen: stage J is written in the implementation change")
}

impl PersonEmployers {
    /// Employer hash (FNV-1a 64 over the person count, then each person's
    /// industry and employer; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("econ-popgen: stage J is written in the implementation change")
    }
}
