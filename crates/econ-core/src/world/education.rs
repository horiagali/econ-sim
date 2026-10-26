//! Education's first version (spec `society/education`, "First version
//! (roadmap M5)", feature `education`): the pure rules of the June close
//! (dropout, progression, completion, continuation, leaving) and of the
//! August close (places), the yearly report, and the read-only columns.
//!
//! API skeleton from the spec's API sketch, written with the acceptance tests
//! before the code (ADR-0014); every body is `unimplemented!` until the
//! implementation fills it. Rates are millionths; levels are indexed from
//! pre-school (0) to master or doctorate (6), `enrolled` − 1.

use econ_popgen::{EduLevel, Enrolled};

use super::World;

/// The parameters of the spec's table (first version), millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EducationParams {
    /// `dropout_rate[ℓ]` a year, by level, pre-school first.
    pub dropout_ppm: [u32; 7],
    /// `continue_rate[3→4]`.
    pub continue_lower_to_upper_ppm: u32,
    /// `continue_rate[4→6]`.
    pub continue_upper_to_tertiary_ppm: u32,
    /// `continue_rate[4→5]`.
    pub continue_upper_to_post_ppm: u32,
    /// `continue_rate[6→7]`.
    pub continue_bachelor_to_master_ppm: u32,
    /// `primary_intake`.
    pub primary_intake_ppm: u32,
    /// `preschool_intake[a]` for ages 3, 4, 5.
    pub preschool_intake_ppm: [u32; 3],
    /// `leaver_seek_share[e]` by `edu_level`.
    pub leaver_seek_ppm: [u32; 5],
    /// `min_leaving_age`, years.
    pub min_leaving_age: u8,
}

impl EducationParams {
    /// The spec's defaults (first version).
    #[must_use]
    pub fn spec_defaults() -> EducationParams {
        EducationParams {
            dropout_ppm: [0, 5_000, 10_000, 20_000, 80_000, 120_000, 80_000],
            continue_lower_to_upper_ppm: 980_000,
            continue_upper_to_tertiary_ppm: 650_000,
            continue_upper_to_post_ppm: 200_000,
            continue_bachelor_to_master_ppm: 700_000,
            primary_intake_ppm: 970_000,
            preschool_intake_ppm: [660_000, 350_000, 200_000],
            leaver_seek_ppm: [400_000, 700_000, 800_000, 850_000, 900_000],
            min_leaving_age: 16,
        }
    }
}

/// Years of each level, pre-school first.
pub const LEVEL_YEARS: [u8; 7] = [3, 5, 4, 4, 2, 3, 2];
/// The typical starting age of each level, years.
pub const LEVEL_START_AGE: [u8; 7] = [3, 6, 11, 15, 19, 19, 22];

/// What becomes of a leaver at the June close (step 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaverFate {
    /// A job seeker: activity unemployed, looking for work.
    Seeking,
    /// Other inactive (`inactive_kind` other).
    Inactive,
    /// Already had a job and keeps it (activity employed).
    KeepsJob,
}

/// The June rule's result for one person.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JuneOutcome {
    /// The level the person is enrolled at after the close.
    pub enrolled: Enrolled,
    /// The year of study after the close.
    pub study_year: u8,
    /// Highest completed education after the close.
    pub edu_level: EduLevel,
    /// The person dropped out (step 1).
    pub dropped_out: bool,
    /// The person completed their level (step 2).
    pub completed: bool,
    /// `Some` when the person becomes a leaver at this close (16 or older).
    pub leaver: Option<LeaverFate>,
}

/// One level's line of the yearly report; weighted persons.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LevelYear {
    /// Enrolled before the close.
    pub before: u64,
    /// Dropouts.
    pub dropouts: u64,
    /// Completers.
    pub completers: u64,
    /// Completers continuing to each level.
    pub continuing_to: [u64; 7],
    /// Entrants given the level at this close.
    pub entrants: u64,
    /// Entrants refused for lack of places.
    pub refused: u64,
    /// Enrolled after the close.
    pub after: u64,
}

/// The report of a June or August close.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EducationYear {
    /// By level, pre-school first.
    pub levels: [LevelYear; 7],
    /// Leavers who look for work, by `edu_level`.
    pub leavers_seeking: [u64; 5],
    /// Leavers who become other inactive, by `edu_level`.
    pub leavers_inactive: [u64; 5],
    /// Leavers who keep their job, by `edu_level`.
    pub leavers_employed: [u64; 5],
}

/// The June rule for one person (AC-EDU-02). `draws` are the close's three
/// draws for the person in millionths (`u1`, `u2`, `u3`); `age_months` is the
/// person's age at the close. A person not enrolled (`Enrolled::None`) is a
/// pupil or student not enrolled: only step 3 applies.
pub fn june_step(
    enrolled: Enrolled,
    study_year: u8,
    edu_level: EduLevel,
    age_months: u16,
    has_job: bool,
    draws: [u32; 3],
    p: &EducationParams,
) -> JuneOutcome {
    let _ = (enrolled, study_year, edu_level, age_months, has_job, draws, p);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Admission against places (AC-EDU-03): for each entrant `(priority, index,
/// weight)`, in input order, whether it is admitted.
pub fn admit(places: u64, continuing: u64, entrants: &[(u64, u32, u32)]) -> Vec<bool> {
    let _ = (places, continuing, entrants);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

impl World {
    /// The latest June or August report; `None` before the first.
    #[must_use]
    pub fn education_year(&self) -> Option<&EducationYear> {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// FNV-1a 64 over the person count, then each person's `enrolled`,
    /// `study_year`, `edu_level` and `activity`, one byte each (AC-EDU-09).
    #[must_use]
    pub fn education_hash(&self) -> u64 {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The weighted share of persons of 18 to 24 with `edu_level` 0 who are
    /// not enrolled, millionths, at the latest June close.
    #[must_use]
    pub fn early_leavers_rate_ppm(&self) -> u32 {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The weighted share of persons of 25 to 34 with `edu_level` 3 or 4,
    /// millionths, at the latest June close.
    #[must_use]
    pub fn tertiary_attainment_ppm(&self) -> u32 {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// Each person's `enrolled`, aligned with [`super::PersonColumns`].
    #[must_use]
    pub fn enrolled(&self) -> &[Enrolled] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// Each person's `study_year`, aligned with [`super::PersonColumns`].
    #[must_use]
    pub fn study_year(&self) -> &[u8] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// `edu_capacity`: the places of each level, weighted persons, pre-school
    /// first.
    #[must_use]
    pub fn edu_capacity(&self) -> [u64; 7] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}
