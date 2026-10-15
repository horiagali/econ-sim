//! Stage K — kinds of inactivity and enrolment (spec
//! `society/population-inactivity-enrolment`).
//!
//! Gives every person whom stage C made "other inactive" a kind of inactivity
//! (homemaker, disabled, other), and every person the level of education they
//! are enrolled at, consistent with stage C's pupils and students. Earlier
//! stages are not changed. `python/reference/popgen_reference.py`
//! (`assign_inactivity_enrolment`) is the independent reference; the two must
//! agree person for person (AC-POPK-08).
//!
//! API skeleton from the spec's API sketch, written before the code
//! (ADR-0014); compiled only with the crate feature `inactivity_enrolment`.
//! The bodies come with the implementation change.

use crate::Population;
use crate::attributes::PersonAttributes;

/// Kinds of inactivity in the margin table: homemaker, disabled, other.
pub const INACTIVE_KINDS: usize = 3;
/// Enrolment levels in the margin table (see [`Enrolled`], without `None`).
pub const ENROLMENT_LEVELS: usize = 7;

/// The kind of inactivity of a person whose activity is other inactive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum InactiveKind {
    /// The person's activity is not other inactive.
    None = 0,
    /// Homemaker.
    Homemaker = 1,
    /// Inactive for reasons of health.
    Disabled = 2,
    /// Supported by others, the state or other sources; discouraged; other situations.
    Other = 3,
}

/// The level of education a person is enrolled at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Enrolled {
    /// Not enrolled.
    None = 0,
    /// Pre-school (ISCED 0).
    PreSchool = 1,
    /// Primary (ISCED 1).
    Primary = 2,
    /// Lower secondary (ISCED 2).
    LowerSecondary = 3,
    /// Upper secondary (ISCED 3).
    UpperSecondary = 4,
    /// Post-secondary non-tertiary (ISCED 4).
    PostSecondary = 5,
    /// Short-cycle tertiary or bachelor (ISCED 5–6).
    Tertiary = 6,
    /// Master or doctorate (ISCED 7–8).
    MasterDoctorate = 7,
}

/// The margin file of the stage: integer counts of real persons, national.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InactivityEnrolmentMargins {
    /// Age bands `[from, to)` in years, those of stage C; the last is open (`to == OPEN`).
    pub age_bands: Vec<(u16, u16)>,
    /// Enrolment age groups `[first, first after)` in years; the last is open.
    pub enrolment_ages: Vec<(u16, u16)>,
    /// Persons other inactive, `[sex][age_band][kind]`, row-major.
    pub inactive_kind: Vec<u64>,
    /// Pupils and students, `[sex][enrolment_age][level]`, row-major.
    pub enrolment: Vec<u64>,
    /// Census residents, `[sex][enrolment_age]`.
    pub population: Vec<u64>,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InactivityParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// `homemaker_child_link`, in millionths.
    pub homemaker_child_link_ppm: u32,
}

impl InactivityParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        InactivityParams {
            rng_seed,
            homemaker_child_link_ppm: 500_000,
        }
    }
}

/// Kind of inactivity and enrolment level of every person, aligned with the persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonInactivityEnrolment {
    /// Kind of inactivity.
    pub inactive_kind: Vec<InactiveKind>,
    /// Level enrolled at.
    pub enrolled: Vec<Enrolled>,
}

/// Why the inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InactivityError {
    /// No age bands or no enrolment ages, or every count of a table is zero.
    EmptyTable,
    /// A table's length does not match its dimensions, or the attributes are
    /// those of another population.
    ShapeMismatch,
    /// Persons to place, and no count to place them by.
    NoMargin,
}

/// Assign the kind of inactivity and the enrolment level of every person of `pop`.
///
/// # Errors
/// See [`InactivityError`].
pub fn assign_inactivity_enrolment(
    pop: &Population,
    attrs: &PersonAttributes,
    margins: &InactivityEnrolmentMargins,
    params: &InactivityParams,
) -> Result<PersonInactivityEnrolment, InactivityError> {
    let _ = (pop, attrs, margins, params);
    unimplemented!("econ-popgen stage K: written in the implementation change")
}

impl PersonInactivityEnrolment {
    /// Inactivity and enrolment hash: FNV-1a 64 over the person count (`u32`,
    /// little-endian), then each person's kind and level (one byte each).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("econ-popgen stage K: written in the implementation change")
    }
}
