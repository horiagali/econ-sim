//! Stage C — education and activity (spec `society/population-attributes`).
//!
//! Gives every person of a generated population an education level and an
//! activity, so that the weighted population reproduces the census tables by
//! region, sex and age. The population itself is not changed.
//!
//! **Skeleton.** This file fixes the API the acceptance tests are written
//! against (ADR-0014, tests-first). The stage is not implemented yet; the
//! `attributes` feature that enables its acceptance tests is switched on by
//! the implementation change. `python/reference/popgen_reference.py`
//! (`assign_attributes`) is the independent reference.

use crate::Population;

/// Activities in the activity margin table: child (below the minimum working
/// age), in education, employed, unemployed, retired, other inactive.
pub const MARGIN_ACTIVITIES: usize = 6;
/// Labour statuses in the education margin table: employed, unemployed, inactive.
pub const STATUSES: usize = 3;
/// Education levels (see [`EduLevel`]).
pub const EDU_LEVELS: usize = 5;

/// Highest completed education, in ISCED 2011 groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum EduLevel {
    /// At most lower secondary; also everyone under 15.
    Isced0To2 = 0,
    /// Upper secondary.
    Isced3 = 1,
    /// Post-secondary non-tertiary.
    Isced4 = 2,
    /// Short-cycle tertiary or bachelor.
    Isced5To6 = 3,
    /// Master or doctorate.
    Isced7To8 = 4,
}

/// A person's main activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Activity {
    /// Below school age.
    Child = 0,
    /// At school: of school age and under the minimum working age, or in
    /// education without upper secondary completed.
    Pupil = 1,
    /// In education with upper secondary completed.
    Student = 2,
    /// Employed (employee, self-employed, employer or family worker).
    Employed = 3,
    /// Unemployed.
    Unemployed = 4,
    /// Retired, or living on capital income.
    Retired = 5,
    /// Other inactive (homemaker, disabled, other).
    InactiveOther = 6,
}

/// Census margin tables for education and activity: integer counts of real
/// persons by development region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeMargins {
    /// Region codes (NUTS 2), in table order.
    pub regions: Vec<String>,
    /// For each county of [`crate::Margins::counties`], its index into `regions`.
    pub region_of_county: Vec<u8>,
    /// Age bands `[from, to)` in years, as in [`crate::Margins`].
    pub age_bands: Vec<(u16, u16)>,
    /// Number of single years of age in `activity`; the last one is open.
    pub years_of_age: u16,
    /// `[region][sex][year_of_age][margin activity]`, row-major.
    pub activity: Vec<u64>,
    /// `[region][sex][age_band][status][edu_level]`, row-major.
    pub education: Vec<u64>,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttrParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// From this age (years) a child is a pupil.
    pub school_age: u16,
    /// Alternating passes of the two-way balancing of the activity targets.
    pub balancing_passes: u32,
}

impl AttrParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        AttrParams {
            rng_seed,
            school_age: 6,
            balancing_passes: 20,
        }
    }
}

/// Education level and activity of every person, aligned with the persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonAttributes {
    /// Highest completed education.
    pub edu_level: Vec<EduLevel>,
    /// Main activity.
    pub activity: Vec<Activity>,
}

/// Why margins were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrError {
    /// No regions, age bands or years of age.
    EmptyTable,
    /// A table's length does not match its dimensions.
    ShapeMismatch,
    /// A household's county has no region.
    UnknownRegion {
        /// The county index.
        county: u8,
    },
    /// The census has nobody in an age band that has synthetic persons.
    NoMargin {
        /// The region code.
        region: String,
    },
}

/// Assign education level and activity to every person of `pop`.
///
/// # Errors
/// See [`AttrError`].
pub fn assign_attributes(
    pop: &Population,
    margins: &AttributeMargins,
    params: &AttrParams,
) -> Result<PersonAttributes, AttrError> {
    let _ = (pop, margins, params);
    unimplemented!("econ-popgen: stage C is written in the implementation change")
}

impl PersonAttributes {
    /// Attribute hash (FNV-1a 64 over the person count, then each person's
    /// education level and activity; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("econ-popgen: stage C is written in the implementation change")
    }
}
