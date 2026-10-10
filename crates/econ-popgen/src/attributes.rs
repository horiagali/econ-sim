//! Stage C — education and activity (spec `society/population-attributes`).
//!
//! Gives every person of a generated population an education level and an
//! activity, so that the weighted population reproduces the census tables by
//! region, sex and age. The population itself is not changed.
//!
//! The public API is the one the acceptance tests were written against
//! (ADR-0014, tests-first). `python/reference/popgen_reference.py`
//! (`assign_attributes`) is the independent reference: the two must agree
//! person for person (AC-POPA-07).

use econ_rng::{KeyedRng, Stream};

use crate::arith::Int;
use crate::assign::{balance, band_of, column_hash, column_sums, household_regions, ints, split};
use crate::{OPEN, Population};

/// `tick` of the draw context: the stage number (0 is the seed stage).
const STAGE: u32 = 1;
const MONTHS: u16 = 12;
/// Margin activities, in table order.
const CHILD: usize = 0;
const IN_EDUCATION: usize = 1;
/// Labour status (employed, unemployed, inactive) of each margin activity.
const STATUS_OF_MARGIN: [usize; MARGIN_ACTIVITIES] = [2, 2, 0, 1, 2, 2];
/// Activity of each margin activity; child and in education are split afterwards.
const ACTIVITY_OF_MARGIN: [Activity; MARGIN_ACTIVITIES] = [
    Activity::Child,
    Activity::Pupil,
    Activity::Employed,
    Activity::Unemployed,
    Activity::Retired,
    Activity::InactiveOther,
];
const LEVELS: [EduLevel; EDU_LEVELS] = [
    EduLevel::Isced0To2,
    EduLevel::Isced3,
    EduLevel::Isced4,
    EduLevel::Isced5To6,
    EduLevel::Isced7To8,
];

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
/// See [`AttrError`]. `ShapeMismatch` also covers a person whose age is in no
/// age band of the margins.
pub fn assign_attributes(
    pop: &Population,
    margins: &AttributeMargins,
    params: &AttrParams,
) -> Result<PersonAttributes, AttrError> {
    let (n_r, n_b, n_y) = (
        margins.regions.len(),
        margins.age_bands.len(),
        usize::from(margins.years_of_age),
    );
    if n_r == 0 || n_b == 0 || n_y == 0 {
        return Err(AttrError::EmptyTable);
    }
    if margins.activity.len() != n_r * 2 * n_y * MARGIN_ACTIVITIES
        || margins.education.len() != n_r * 2 * n_b * STATUSES * EDU_LEVELS
    {
        return Err(AttrError::ShapeMismatch);
    }
    let (hh, p) = (&pop.households, &pop.persons);
    let hh_region = household_regions(&hh.hh_county, &margins.region_of_county, n_r)
        .map_err(|county| AttrError::UnknownRegion { county })?;
    let no_margin = |region_sex: usize| AttrError::NoMargin {
        region: margins.regions[region_sex / 2].clone(),
    };
    let activity = ints(&margins.activity);
    let education = ints(&margins.education);

    // Per person: weight, (region, sex) index, year of age, age band, priority.
    let n = p.household_id.len();
    let rng = KeyedRng::new(params.rng_seed);
    let last_year = margins.years_of_age - 1;
    let mut weight: Vec<Int> = Vec::with_capacity(n);
    let mut region_sex = Vec::with_capacity(n);
    let mut year = Vec::with_capacity(n);
    let mut band = Vec::with_capacity(n);
    let mut priority = Vec::with_capacity(n);
    for i in 0..n {
        let h = p.household_id[i] as usize;
        let years = (p.age[i] / MONTHS).min(last_year);
        weight.push(Int::from(hh.hh_weight[h]));
        region_sex.push(hh_region[h] * 2 + p.sex[i] as usize);
        year.push(usize::from(years));
        band.push(band_of(&margins.age_bands, years).ok_or(AttrError::ShapeMismatch)?);
        priority.push(rng.draw(Stream::PopulationGen, STAGE, i as u64).u64());
    }
    let weights_of =
        |members: &[usize]| -> Vec<Int> { members.iter().map(|&i| weight[i]).collect() };

    // Step 1: activity, by region, sex and year of age. Targets come from
    // balancing each age band; the carry passes from one year to the next.
    let mut by_year: Vec<Vec<usize>> = vec![Vec::new(); n_r * 2 * n_y];
    for i in 0..n {
        by_year[region_sex[i] * n_y + year[i]].push(i);
    }
    let mut margin_activity = vec![CHILD; n];
    for rs in 0..n_r * 2 {
        let mut carry = [0; MARGIN_ACTIVITIES];
        for &(from, to) in &margins.age_bands {
            let to = if to == OPEN {
                n_y
            } else {
                usize::from(to).min(n_y)
            };
            let years = usize::from(from).min(to)..to;
            let weights: Vec<Int> = years
                .clone()
                .map(|y| by_year[rs * n_y + y].iter().map(|&i| weight[i]).sum())
                .collect();
            if weights.iter().all(|&w| w == 0) {
                continue;
            }
            let census = &activity[(rs * n_y + years.start) * MARGIN_ACTIVITIES
                ..(rs * n_y + years.end) * MARGIN_ACTIVITIES];
            let band_totals = column_sums(census, MARGIN_ACTIVITIES);
            if band_totals.iter().all(|&v| v == 0) {
                return Err(no_margin(rs));
            }
            // A year the census has nobody in, but a synthetic person is,
            // uses the band's totals.
            let pattern: Vec<Vec<Int>> = census
                .chunks_exact(MARGIN_ACTIVITIES)
                .map(|row| {
                    if row.iter().any(|&v| v != 0) {
                        row.to_vec()
                    } else {
                        band_totals.clone()
                    }
                })
                .collect();
            let targets = balance(
                &pattern,
                &weights,
                &band_totals,
                params.balancing_passes,
                None,
            );
            for (y, targets) in years.zip(&targets) {
                let members = &mut by_year[rs * n_y + y];
                if members.is_empty() {
                    continue;
                }
                members.sort_unstable_by_key(|&i| (priority[i], i));
                let dealt = split(&weights_of(members), targets, &mut carry);
                for (&i, k) in members.iter().zip(dealt) {
                    margin_activity[i] = k;
                }
            }
        }
    }

    // Step 2: education level, by region, sex, labour status and age band; the
    // carry passes from one band to the next. Among the inactive, persons in
    // education come last, so they take the highest levels.
    let mut by_band: Vec<Vec<usize>> = vec![Vec::new(); n_r * 2 * STATUSES * n_b];
    for i in 0..n {
        let status = STATUS_OF_MARGIN[margin_activity[i]];
        by_band[(region_sex[i] * STATUSES + status) * n_b + band[i]].push(i);
    }
    let mut level = vec![0; n];
    for sequence in 0..n_r * 2 * STATUSES {
        let (rs, status) = (sequence / STATUSES, sequence % STATUSES);
        let mut carry = [0; EDU_LEVELS];
        for b in 0..n_b {
            let members = &mut by_band[sequence * n_b + b];
            if members.is_empty() {
                continue;
            }
            let cell =
                &education[(rs * n_b + b) * STATUSES * EDU_LEVELS..][..STATUSES * EDU_LEVELS];
            let mut targets = cell[status * EDU_LEVELS..][..EDU_LEVELS].to_vec();
            if targets.iter().all(|&v| v == 0) {
                // Nobody of that status in the census: the whole age band.
                targets = column_sums(cell, EDU_LEVELS);
            }
            if targets.iter().all(|&v| v == 0) {
                return Err(no_margin(rs));
            }
            members.sort_unstable_by_key(|&i| (margin_activity[i] == IN_EDUCATION, priority[i], i));
            let dealt = split(&weights_of(members), &targets, &mut carry);
            for (&i, k) in members.iter().zip(dealt) {
                level[i] = k;
            }
        }
    }

    // Step 3: the census has one category for everyone below the minimum
    // working age and one for everyone in education; the population model
    // splits both.
    let school_age_months = u32::from(params.school_age) * u32::from(MONTHS);
    let activity = (0..n)
        .map(|i| match margin_activity[i] {
            CHILD if u32::from(p.age[i]) >= school_age_months => Activity::Pupil,
            IN_EDUCATION if level[i] >= EduLevel::Isced3 as usize => Activity::Student,
            k => ACTIVITY_OF_MARGIN[k],
        })
        .collect();
    Ok(PersonAttributes {
        edu_level: level.into_iter().map(|k| LEVELS[k]).collect(),
        activity,
    })
}

impl PersonAttributes {
    /// Attribute hash (FNV-1a 64 over the person count, then each person's
    /// education level and activity; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let pairs = self.edu_level.iter().zip(&self.activity);
        column_hash(
            self.edu_level.len(),
            pairs.flat_map(|(&level, &activity)| [level as u8, activity as u8]),
        )
    }
}
