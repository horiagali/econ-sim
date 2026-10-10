//! Stage F — wages (spec `society/population-wages`).
//!
//! Gives every employee a gross monthly wage, so that each industry group
//! pays the wage bill of the national accounts and, inside a group, wages
//! differ by sex, occupation, age and education as in the earnings survey.
//! Earlier stages are not changed.
//!
//! **Skeleton.** This file fixes the API the acceptance tests are written
//! against (ADR-0014, tests-first). The stage is not implemented yet; the
//! `wages` feature that enables its acceptance tests is switched on by the
//! implementation change. `python/reference/popgen_reference.py`
//! (`assign_wages`) is the independent reference.

use crate::Population;
use crate::attributes::{EDU_LEVELS, PersonAttributes};
use crate::jobs::PersonJobs;

/// Earnings margins: levels from the national accounts, proportions from the
/// earnings survey. Tables by industry group have [`crate::INDUSTRY_GROUPS`]
/// rows, by occupation [`crate::OCCUPATIONS`] columns, by education group
/// [`crate::EDU_GROUPS`] columns. Proportions are in millionths of the mean
/// wage of the industry group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WageMargins {
    /// Wages and salaries paid per month, in lei, `[industry_group]`.
    pub wage_bill: Vec<u64>,
    /// Gross minimum wage per month, in lei.
    pub minimum_wage: u64,
    /// First age (years) of each age group of `rel_age`, ascending from 0.
    pub age_group_from: Vec<u16>,
    /// The education group of each education level.
    pub edu_group_of_level: [u8; EDU_LEVELS],
    /// `[industry_group][sex]`.
    pub rel_sex: Vec<u32>,
    /// `[industry_group][sex][occupation]`, row-major.
    pub rel_occupation: Vec<u32>,
    /// `[industry_group][sex][age_group]`, row-major.
    pub rel_age: Vec<u32>,
    /// `[industry_group][sex][edu_group]`, row-major.
    pub rel_education: Vec<u32>,
    /// Ranks of the knots of the quantile curve, in millionths: ascending
    /// from 0 to 1,000,000.
    pub curve_rank: Vec<u32>,
    /// Earnings at each knot over mean earnings, in millionths; not decreasing.
    pub curve_value: Vec<u32>,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WageParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Share of the quantile curve's spread that is left inside a cell, in
    /// millionths.
    pub dispersion_ppm: u32,
    /// The floor of an industry group is the minimum wage, or this share of
    /// the group's mean wage if that is lower; in millionths.
    pub floor_share_ppm: u32,
    /// Passes of scaling to the survey's proportions.
    pub scaling_passes: u32,
}

impl WageParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        WageParams {
            rng_seed,
            dispersion_ppm: 500_000,
            floor_share_ppm: 800_000,
            scaling_passes: 20,
        }
    }
}

/// The wage column, aligned with the persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonWages {
    /// Gross monthly wage in bani; 0 for everyone who is not an employee.
    pub wage: Vec<u64>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WageError {
    /// No age groups, or a quantile curve with fewer than two knots.
    EmptyTable,
    /// A table's length does not match its dimensions, a proportion is zero,
    /// the quantile curve is not ascending from rank 0 to 1,000,000, or the
    /// attributes or jobs belong to another population.
    ShapeMismatch,
    /// An industry group has employees but no wage bill.
    NoMargin {
        /// The industry group index.
        industry_group: u8,
    },
}

/// Assign a gross monthly wage to every employee.
///
/// # Errors
/// See [`WageError`].
pub fn assign_wages(
    pop: &Population,
    attrs: &PersonAttributes,
    jobs: &PersonJobs,
    margins: &WageMargins,
    params: &WageParams,
) -> Result<PersonWages, WageError> {
    let _ = (pop, attrs, jobs, margins, params);
    unimplemented!("econ-popgen: stage F is written in the implementation change")
}

impl PersonWages {
    /// Wage hash (FNV-1a 64 over the person count, then each person's wage;
    /// see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("econ-popgen: stage F is written in the implementation change")
    }
}
