//! Stage G — pensions (spec `society/population-pensions`).
//!
//! Gives every retired person a gross monthly pension, so that the pension
//! bill of the year is paid, women's and men's pensions stand as the gender
//! pension gap says, and about as many pensioners are at the minimum pension
//! as in reality. Earlier stages are not changed.
//!
//! **Skeleton.** This file fixes the API the acceptance tests are written
//! against (ADR-0014, tests-first). The stage is not implemented yet; the
//! `pensions` feature that enables its acceptance tests is switched on by the
//! implementation change. `python/reference/popgen_reference.py`
//! (`assign_pensions`) is the independent reference.

use crate::Population;
use crate::attributes::{EDU_LEVELS, PersonAttributes};

/// Pension margins. Proportions are in millionths. Tables by education group
/// have [`crate::EDU_GROUPS`] columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PensionMargins {
    /// Pensions paid per month, in lei.
    pub pension_bill: u64,
    /// The minimum pension per month, in lei.
    pub minimum_pension: u64,
    /// Mean pension of each sex, as proportions, `[sex]`.
    pub rel_sex: [u32; 2],
    /// The education group of each education level.
    pub edu_group_of_level: [u8; EDU_LEVELS],
    /// Mean earnings of each education group over the mean of its sex,
    /// `[sex][edu_group]`, row-major.
    pub rel_education: Vec<u32>,
    /// Ranks of the knots of the pension quantile curve, in millionths:
    /// ascending from 0 to 1,000,000.
    pub curve_rank: Vec<u32>,
    /// Pension at each knot over the mean pension, in millionths; not decreasing.
    pub curve_value: Vec<u32>,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PensionParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Share of the quantile curve's spread that is used, in millionths.
    pub dispersion_ppm: u32,
    /// How much of the earnings gap between education groups pensions keep,
    /// in millionths.
    pub earnings_link_ppm: u32,
    /// Passes of scaling to the proportions.
    pub scaling_passes: u32,
}

impl PensionParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        PensionParams {
            rng_seed,
            dispersion_ppm: 1_000_000,
            earnings_link_ppm: 500_000,
            scaling_passes: 20,
        }
    }
}

/// The pension column, aligned with the persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonPensions {
    /// Gross monthly pension in bani; 0 for everyone who is not retired.
    pub pension: Vec<u64>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PensionError {
    /// No education groups, or a quantile curve with fewer than two knots.
    EmptyTable,
    /// A table's length does not match its dimensions, a proportion is zero,
    /// the quantile curve is not ascending from rank 0 to 1,000,000, or the
    /// attributes belong to another population.
    ShapeMismatch,
    /// There are retired persons but no pension bill.
    NoMargin,
}

/// Assign a gross monthly pension to every retired person.
///
/// # Errors
/// See [`PensionError`].
pub fn assign_pensions(
    pop: &Population,
    attrs: &PersonAttributes,
    margins: &PensionMargins,
    params: &PensionParams,
) -> Result<PersonPensions, PensionError> {
    let _ = (pop, attrs, margins, params);
    unimplemented!("econ-popgen: stage G is written in the implementation change")
}

impl PersonPensions {
    /// Pension hash (FNV-1a 64 over the person count, then each person's
    /// pension; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("econ-popgen: stage G is written in the implementation change")
    }
}
