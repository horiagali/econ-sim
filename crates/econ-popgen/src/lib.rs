//! Population generator (spec `society/population-generator`).
//!
//! Builds the starting synthetic population (households and their members,
//! with integer weights) from census margin tables, at any `sample_scale`.
//! A pure function of (margins, parameters): no I/O, integer arithmetic only.
//!
//! **Skeleton.** This file fixes the API the acceptance tests are written
//! against (ADR-0014, tests-first). The generator itself is not implemented
//! yet; the `generator` feature that enables the acceptance tests is switched
//! on by the implementation change.

/// Marks the open end of the last age band or size class in [`Margins`].
pub const OPEN: u16 = u16::MAX;

/// Sex of a person. The discriminant is the index used in the margin tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Sex {
    /// Female.
    F = 0,
    /// Male.
    M = 1,
}

/// A person's role in their household.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Role {
    /// The household's reference person (exactly one per household).
    Head = 0,
    /// Partner of the head.
    Partner = 1,
    /// Child of the head.
    Child = 2,
    /// Other relative or member.
    Other = 3,
}

/// Census margin tables: integer counts of real persons and households.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Margins {
    /// County codes (NUTS 3), in table order.
    pub counties: Vec<String>,
    /// Age bands `[from, to)` in years; the last band is open (`to == OPEN`).
    pub age_bands: Vec<(u16, u16)>,
    /// Household size classes `[min, max]` members; the last class is open (`max == OPEN`).
    pub size_classes: Vec<(u16, u16)>,
    /// Persons in private households, `[county][sex][age_band]`, row-major.
    pub persons_private: Vec<u64>,
    /// Persons not in private households, same shape.
    pub persons_collective: Vec<u64>,
    /// Private households, `[county][size_class]`, row-major.
    pub households: Vec<u64>,
}

/// Generator parameters. `sample_scale` comes from the scenario (ADR-0003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenParams {
    /// Real people per synthetic person.
    pub sample_scale: u32,
    /// Master seed of the keyed draws.
    pub rng_seed: u64,
    /// Cap of the open size class in the rule-based seed.
    pub max_household_size: u16,
    /// Relative error at which raking stops, in parts per million.
    pub raking_tolerance_ppm: u32,
    /// Upper bound on raking sweeps.
    pub max_sweeps: u32,
    /// Cells with fewer expected synthetic records are not checked for convergence.
    pub min_cell_records: u32,
}

impl GenParams {
    /// The spec's default tuning parameters for a given scale and seed.
    #[must_use]
    pub fn new(sample_scale: u32, rng_seed: u64) -> Self {
        GenParams {
            sample_scale,
            rng_seed,
            max_household_size: 15,
            raking_tolerance_ppm: 1000,
            max_sweeps: 50,
            min_cell_records: 30,
        }
    }
}

/// The households table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Households {
    /// Real households (or, for collective records, persons) represented; ≥ 1.
    pub hh_weight: Vec<u32>,
    /// Index into [`Margins::counties`].
    pub hh_county: Vec<u8>,
    /// The record stands for people not living in a private household.
    pub hh_collective: Vec<bool>,
}

/// The persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Persons {
    /// Index into the households table.
    pub household_id: Vec<u32>,
    /// Age in months.
    pub age: Vec<u16>,
    /// Sex.
    pub sex: Vec<Sex>,
    /// Role in the household.
    pub role: Vec<Role>,
}

/// How the fit went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FitReport {
    /// Largest number of raking sweeps used by a county.
    pub sweeps: u32,
    /// Every county reached the raking tolerance.
    pub converged: bool,
    /// Largest relative error left on a checked constraint, parts per million.
    pub max_error_ppm: u32,
    /// County cells with people but no synthetic record.
    pub unfitted_cells: u32,
}

/// A generated population.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Population {
    /// Households.
    pub households: Households,
    /// Persons.
    pub persons: Persons,
    /// Fit report.
    pub report: FitReport,
}

/// Why margins or parameters were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenError {
    /// A county's private persons cannot fit its households.
    InconsistentMargins {
        /// The county code.
        county: String,
    },
    /// `sample_scale` is zero.
    ZeroScale,
    /// No counties, age bands or size classes.
    EmptyTable,
    /// A table's length does not match its dimensions.
    ShapeMismatch,
}

/// Generate the starting population.
///
/// # Errors
/// See [`GenError`].
pub fn generate(margins: &Margins, params: &GenParams) -> Result<Population, GenError> {
    let _ = (margins, params);
    unimplemented!("econ-popgen: the generator is written in the implementation change")
}

impl Population {
    /// State hash (FNV-1a 64 over the tables in the order fixed by the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("econ-popgen: the generator is written in the implementation change")
    }
}
