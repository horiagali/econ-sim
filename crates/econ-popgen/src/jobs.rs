//! Stage D — jobs (spec `society/population-jobs`).
//!
//! Gives every employed person a status in employment, an occupation and an
//! industry group, so that the weighted population reproduces the census tables
//! by region, sex and age. Earlier stages are not changed.
//!
//! **Skeleton.** This file fixes the API the acceptance tests are written
//! against (ADR-0014, tests-first). The stage is not implemented yet; the
//! `jobs` feature that enables its acceptance tests is switched on by the
//! implementation change. `python/reference/popgen_reference.py`
//! (`assign_jobs`) is the independent reference.

use crate::Population;
use crate::attributes::{EDU_LEVELS, PersonAttributes};

/// Statuses in employment: employee, employer, own-account worker, family worker.
pub const KINDS: usize = 4;
/// Occupations: ISCO-08 major groups, numbered by their digit (0 = armed forces).
pub const OCCUPATIONS: usize = 10;
/// Industry groups: the NACE Rev. 2 groups A, B–E, F, G–I, J, K, L, M–N, O–Q, R–U.
pub const INDUSTRY_GROUPS: usize = 10;
/// Education groups of the pattern table: ISCED 0–2, 3–4, 5–8.
pub const EDU_GROUPS: usize = 3;
/// `occupation` and `industry_group` of a person who is not employed.
pub const NOT_EMPLOYED: u8 = 255;

/// Census margins and survey patterns for jobs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobMargins {
    /// Region codes (NUTS 2), in table order.
    pub regions: Vec<String>,
    /// For each county of [`crate::Margins::counties`], its index into `regions`.
    pub region_of_county: Vec<u8>,
    /// Age bands `[from, to)` in years, as in [`crate::Margins`].
    pub age_bands: Vec<(u16, u16)>,
    /// Census persons, `[region][sex][age_band][kind][industry_group]`, row-major.
    pub by_industry_group: Vec<u64>,
    /// Census persons, `[region][sex][age_band][kind][occupation]`, row-major.
    pub by_occupation: Vec<u64>,
    /// Pattern (proportions only), `[sex][edu_group][occupation]`.
    pub pattern_edu_occupation: Vec<u64>,
    /// Pattern (proportions only), `[sex][occupation][industry_group]`.
    pub pattern_occupation_industry_group: Vec<u64>,
    /// The pattern's education group of each education level.
    pub edu_group_of_level: [u8; EDU_LEVELS],
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Persons added to every pattern cell, so no combination is impossible.
    pub pattern_floor: u64,
    /// Upper bound on alternating passes of the two-way balancing.
    pub balancing_passes: u32,
    /// Passes stop once every column is within this many millionths of the
    /// group's weight of its target.
    pub balance_stop_ppm: u32,
}

impl JobParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        JobParams {
            rng_seed,
            pattern_floor: 1,
            balancing_passes: 200,
            balance_stop_ppm: 100,
        }
    }
}

/// Job columns, aligned with the persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonJobs {
    /// 0 not employed, 1 employee, 2 employer, 3 own-account worker, 4 family worker.
    pub employment_status: Vec<u8>,
    /// ISCO-08 major group, or [`NOT_EMPLOYED`].
    pub occupation: Vec<u8>,
    /// Industry group index, or [`NOT_EMPLOYED`].
    pub industry_group: Vec<u8>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobError {
    /// No regions or age bands.
    EmptyTable,
    /// A table's length does not match its dimensions, or the attributes
    /// belong to another population.
    ShapeMismatch,
    /// A household's county has no region.
    UnknownRegion {
        /// The county index.
        county: u8,
    },
    /// The census has nobody employed in a region and sex that has employed
    /// synthetic persons.
    NoMargin {
        /// The region code.
        region: String,
    },
}

/// Assign status in employment, occupation and industry group to every employed person.
///
/// # Errors
/// See [`JobError`].
pub fn assign_jobs(
    pop: &Population,
    attrs: &PersonAttributes,
    margins: &JobMargins,
    params: &JobParams,
) -> Result<PersonJobs, JobError> {
    let _ = (pop, attrs, margins, params);
    unimplemented!("econ-popgen: stage D is written in the implementation change")
}

impl PersonJobs {
    /// Jobs hash (FNV-1a 64 over the person count, then each person's status
    /// in employment, occupation and industry group; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("econ-popgen: stage D is written in the implementation change")
    }
}
