//! Stage E — locality size and tenure (spec `society/population-housing`).
//!
//! Gives every household the size class of its locality (and from it the
//! urban flag) and, for private households, a tenure, so that the weighted
//! population reproduces the census tables county by county. The population
//! is not changed.
//!
//! **Skeleton.** This file fixes the API the acceptance tests are written
//! against (ADR-0014, tests-first). The stage is not implemented yet; the
//! `housing` feature that enables its acceptance tests is switched on by the
//! implementation change. `python/reference/popgen_reference.py`
//! (`assign_housing`) is the independent reference.

use crate::Population;

/// Tenures: owner, tenant, other.
pub const TENURES: usize = 3;
/// Household size groups of the tenure table: one person, larger.
pub const SIZE_GROUPS: usize = 2;
/// `hh_tenure` of a record that is not a private household.
pub const NO_TENURE: u8 = 255;

/// Census margin tables for locality size and tenure, by county.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HousingMargins {
    /// County codes; must equal [`crate::Margins::counties`].
    pub counties: Vec<String>,
    /// Number of locality size classes.
    pub locality_classes: u8,
    /// A locality of this class or above is urban.
    pub urban_from_class: u8,
    /// First age (years) of each age group, ascending from 0.
    pub age_group_from: Vec<u16>,
    /// Residents, `[county][age_group][locality_class]`, row-major.
    pub persons_by_locality: Vec<u64>,
    /// Private households, `[county][size_group][tenure]`, row-major.
    pub households_by_tenure: Vec<u64>,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HousingParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Persons added to every cell of the county's pattern table.
    pub pattern_floor: u64,
    /// Upper bound on alternating passes of the two-way balancing.
    pub balancing_passes: u32,
    /// Passes stop once every column is within this many millionths of the
    /// county's weight of its target.
    pub balance_stop_ppm: u32,
}

impl HousingParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        HousingParams {
            rng_seed,
            pattern_floor: 1,
            balancing_passes: 200,
            balance_stop_ppm: 100,
        }
    }
}

/// Housing columns, aligned with the households table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HouseholdHousing {
    /// Locality size class, `0..locality_classes`.
    pub hh_locality_size: Vec<u8>,
    /// The locality is urban.
    pub hh_urban: Vec<bool>,
    /// 0 owner, 1 tenant, 2 other, or [`NO_TENURE`].
    pub hh_tenure: Vec<u8>,
}

/// Why margins were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HousingError {
    /// No counties, locality classes or age groups.
    EmptyTable,
    /// A table's length does not match its dimensions, or the counties are
    /// not those of the population.
    ShapeMismatch,
    /// A county has households but the census table has nobody there.
    NoMargin {
        /// The county code.
        county: String,
    },
}

/// Assign locality size, urban flag and tenure to every household.
///
/// # Errors
/// See [`HousingError`].
pub fn assign_housing(
    pop: &Population,
    margins: &HousingMargins,
    params: &HousingParams,
) -> Result<HouseholdHousing, HousingError> {
    let _ = (pop, margins, params);
    unimplemented!("econ-popgen: stage E is written in the implementation change")
}

impl HouseholdHousing {
    /// Housing hash (FNV-1a 64 over the household count, then each
    /// household's locality size class, urban flag and tenure; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("econ-popgen: stage E is written in the implementation change")
    }
}
