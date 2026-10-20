//! Stage K — kinds of inactivity and enrolment (spec
//! `society/population-inactivity-enrolment`).
//!
//! Gives every person whom stage C made "other inactive" a kind of inactivity
//! (homemaker, disabled, other), and every person the level of education they
//! are enrolled at, consistent with stage C's pupils and students. Earlier
//! stages are not changed. `python/reference/popgen_reference.py`
//! (`assign_inactivity_enrolment`) is the independent reference; the two must
//! agree person for person (AC-POPK-08).

use econ_rng::{KeyedRng, Stream};

use crate::Population;
use crate::arith::{Int, UNIT, apportion};
use crate::assign::{band_of, column_hash, column_sums, ints, split};
use crate::attributes::{Activity, EduLevel, PersonAttributes};

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

/// `tick` of the draw context: the stage number.
const STAGE: u32 = 11;
/// A household member younger than this many years is a young child.
const YOUNG_CHILD_YEARS: u16 = 7;
/// Index of the first enrolment level that needs upper secondary completed
/// (post-secondary).
const AFTER_SECONDARY: usize = 4;

/// Assign the kind of inactivity and the enrolment level of every person of `pop`.
///
/// Kinds: per sex and age band, the band's other inactive persons are split
/// over the kinds, homemaker first; persons living with a young child come
/// first with probability `homemaker_child_link`. Enrolment: per sex and
/// enrolment age group, every student is enrolled, pupils up to the places
/// the students leave, and the group's enrolment above them is taken from
/// its children, employed, unemployed and other inactive persons; the
/// enrolled are split over the levels from the lowest education and youngest
/// up. A level after upper secondary that falls to someone without it
/// becomes upper secondary, and the place goes to the next age group.
///
/// # Errors
/// See [`InactivityError`].
// Weights and counts are i128 (`Int`) far from its ends.
#[allow(clippy::arithmetic_side_effects, clippy::too_many_lines)]
pub fn assign_inactivity_enrolment(
    pop: &Population,
    attrs: &PersonAttributes,
    margins: &InactivityEnrolmentMargins,
    params: &InactivityParams,
) -> Result<PersonInactivityEnrolment, InactivityError> {
    let m = margins;
    let (n_b, n_g) = (m.age_bands.len(), m.enrolment_ages.len());
    if n_b == 0 || n_g == 0 {
        return Err(InactivityError::EmptyTable);
    }
    if m.inactive_kind.len() != 2 * n_b * INACTIVE_KINDS
        || m.enrolment.len() != 2 * n_g * ENROLMENT_LEVELS
        || m.population.len() != 2 * n_g
    {
        return Err(InactivityError::ShapeMismatch);
    }
    if m.inactive_kind.iter().all(|&v| v == 0) || m.enrolment.iter().all(|&v| v == 0) {
        return Err(InactivityError::EmptyTable);
    }
    let p = &pop.persons;
    let n = p.household_id.len();
    let (edu, activity) = (&attrs.edu_level, &attrs.activity);
    if edu.len() != n || activity.len() != n {
        return Err(InactivityError::ShapeMismatch);
    }
    let years: Vec<u16> = p.age.iter().map(|&a| a / 12).collect();
    let band: Vec<usize> = years
        .iter()
        .map(|&y| band_of(&m.age_bands, y).ok_or(InactivityError::ShapeMismatch))
        .collect::<Result<_, _>>()?;
    let group: Vec<usize> = years
        .iter()
        .map(|&y| band_of(&m.enrolment_ages, y).ok_or(InactivityError::ShapeMismatch))
        .collect::<Result<_, _>>()?;
    let weight: Vec<Int> = p
        .household_id
        .iter()
        .map(|&h| Int::from(pop.households.hh_weight[h as usize]))
        .collect();
    let sex: Vec<usize> = p.sex.iter().map(|&s| s as usize).collect();
    let rng = KeyedRng::new(params.rng_seed);
    let (mut prio, mut link) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for i in 0..n {
        let mut d = rng.draw(Stream::PopulationGen, STAGE, i as u64);
        prio.push(d.u64());
        link.push((Int::from(d.u64() >> 44) * UNIT) >> 20);
    }
    let mut young_child = vec![false; pop.households.hh_weight.len()];
    for i in 0..n {
        if years[i] < YOUNG_CHILD_YEARS {
            young_child[p.household_id[i] as usize] = true;
        }
    }

    // Step 1: kinds of inactivity, by sex and age band; rounding is carried
    // to the next band.
    let mut kind = vec![InactiveKind::None; n];
    let mut members_of: Vec<Vec<usize>> = vec![Vec::new(); 2 * n_b];
    for i in 0..n {
        if activity[i] == Activity::InactiveOther {
            members_of[sex[i] * n_b + band[i]].push(i);
        }
    }
    let link_ppm = Int::from(params.homemaker_child_link_ppm);
    for s in 0..2 {
        let rows = &m.inactive_kind[s * n_b * INACTIVE_KINDS..(s + 1) * n_b * INACTIVE_KINDS];
        let sex_totals = column_sums(&ints(rows), INACTIVE_KINDS);
        let mut carry = vec![0; INACTIVE_KINDS];
        for b in 0..n_b {
            let members = &mut members_of[s * n_b + b];
            if members.is_empty() {
                continue;
            }
            let mut targets = ints(&rows[b * INACTIVE_KINDS..(b + 1) * INACTIVE_KINDS]);
            if targets.iter().all(|&v| v == 0) {
                targets.clone_from(&sex_totals);
            }
            if targets.iter().all(|&v| v == 0) {
                return Err(InactivityError::NoMargin);
            }
            members.sort_by_key(|&i| {
                let first = young_child[p.household_id[i] as usize] && link[i] < link_ppm;
                (u8::from(!first), prio[i], i)
            });
            let w: Vec<Int> = members.iter().map(|&i| weight[i]).collect();
            for (&i, k) in members.iter().zip(split(&w, &targets, &mut carry)) {
                kind[i] = KINDS[k];
            }
        }
    }

    // Step 2: enrolment, by sex and enrolment age group; rounding is carried
    // to the next group.
    let mut enrolled = vec![Enrolled::None; n];
    let mut by_group: Vec<Vec<usize>> = vec![Vec::new(); 2 * n_g];
    for i in 0..n {
        by_group[sex[i] * n_g + group[i]].push(i);
    }
    let in_pool = |a: Activity| {
        matches!(
            a,
            Activity::Child | Activity::Employed | Activity::Unemployed | Activity::InactiveOther
        )
    };
    let upper = |i: usize| edu[i] >= EduLevel::Isced3;
    let sum_w = |v: &[usize]| v.iter().map(|&i| weight[i]).sum::<Int>();
    // Who in an ordered pool is taken for `take` of its weight `w_pool`.
    let taken = |pool: &[usize], w_pool: Int, take: Int| -> Vec<usize> {
        let w: Vec<Int> = pool.iter().map(|&i| weight[i]).collect();
        pool.iter()
            .zip(split(&w, &[take, w_pool - take], &mut [0, 0]))
            .filter(|&(_, c)| c == 0)
            .map(|(&i, _)| i)
            .collect()
    };
    for s in 0..2 {
        let mut carry = vec![0; ENROLMENT_LEVELS];
        let mut owed: [Int; ENROLMENT_LEVELS] = [0; ENROLMENT_LEVELS];
        for g in 0..n_g {
            let members = &by_group[s * n_g + g];
            if members.is_empty() {
                continue;
            }
            let at = (s * n_g + g) * ENROLMENT_LEVELS;
            let counts = ints(&m.enrolment[at..at + ENROLMENT_LEVELS]);
            let rest = (Int::from(m.population[s * n_g + g]) - counts.iter().sum::<Int>()).max(0);
            let total = sum_w(members);
            let mut t: Vec<Int> = if counts.iter().sum::<Int>() + rest != 0 {
                let mut targets = counts.clone();
                targets.push(rest);
                let mut parts = apportion(total, &targets);
                parts.truncate(ENROLMENT_LEVELS);
                parts
            } else {
                vec![0; ENROLMENT_LEVELS]
            };
            for (v, o) in t.iter_mut().zip(owed.iter_mut()) {
                *v += *o;
                *o = 0;
            }
            let by_prio = |keep: &dyn Fn(usize) -> bool| -> Vec<usize> {
                let mut v: Vec<usize> = members.iter().copied().filter(|&i| keep(i)).collect();
                v.sort_by_key(|&i| (prio[i], i));
                v
            };
            let students: Vec<usize> = members
                .iter()
                .copied()
                .filter(|&i| activity[i] == Activity::Student)
                .collect();
            let mut pupils = by_prio(&|i| activity[i] == Activity::Pupil);
            let w_must_hi = sum_w(&students);
            let pool_hi = by_prio(&|i| in_pool(activity[i]) && upper(i));
            let pool_lo = by_prio(&|i| in_pool(activity[i]) && !upper(i));
            let (w_hi, w_lo) = (sum_w(&pool_hi), sum_w(&pool_lo));
            let later: Int = t[AFTER_SECONDARY..].iter().sum();
            let earlier: Int = t[..AFTER_SECONDARY].iter().sum();
            // The places students leave empty after upper secondary, taken
            // from the upper pool.
            let take_hi = (later - w_must_hi).max(0).min(w_hi);
            // Pupils are enrolled only up to the places the students and the
            // upper pool leave; the surplus, last in priority, is not enrolled.
            let room = (earlier + later - w_must_hi - take_hi).max(0);
            let w_pupils = sum_w(&pupils);
            if w_pupils > room {
                pupils = taken(&pupils, w_pupils, room);
            }
            let mut chosen = students;
            chosen.extend(&pupils);
            let w_must = sum_w(&chosen);
            // The places pupils and students leave empty before upper
            // secondary; students beyond the later levels' places take earlier ones.
            let spare_hi = (w_must_hi - later).max(0);
            let take_lo = (earlier - (w_must - w_must_hi) - spare_hi).max(0).min(w_lo);
            for (pool, w_pool, take) in [(&pool_hi, w_hi, take_hi), (&pool_lo, w_lo, take_lo)] {
                if take != 0 {
                    chosen.extend(taken(pool, w_pool, take));
                }
            }
            if chosen.is_empty() {
                continue;
            }
            if t.iter().all(|&v| v == 0) {
                if counts.iter().all(|&v| v == 0) {
                    return Err(InactivityError::NoMargin);
                }
                t = counts;
            }
            chosen.sort_by_key(|&i| (edu[i], p.age[i], prio[i], i));
            let w: Vec<Int> = chosen.iter().map(|&i| weight[i]).collect();
            for (&i, k) in chosen.iter().zip(split(&w, &t, &mut carry)) {
                enrolled[i] = if k >= AFTER_SECONDARY && !upper(i) {
                    // Not eligible: upper secondary instead; the place goes to
                    // the next age group.
                    owed[k] += weight[i];
                    Enrolled::UpperSecondary
                } else {
                    LEVELS[k]
                };
            }
        }
    }
    Ok(PersonInactivityEnrolment {
        inactive_kind: kind,
        enrolled,
    })
}

/// The kinds, in the margin table's order.
const KINDS: [InactiveKind; INACTIVE_KINDS] = [
    InactiveKind::Homemaker,
    InactiveKind::Disabled,
    InactiveKind::Other,
];

/// The levels, in the margin table's order.
const LEVELS: [Enrolled; ENROLMENT_LEVELS] = [
    Enrolled::PreSchool,
    Enrolled::Primary,
    Enrolled::LowerSecondary,
    Enrolled::UpperSecondary,
    Enrolled::PostSecondary,
    Enrolled::Tertiary,
    Enrolled::MasterDoctorate,
];

impl PersonInactivityEnrolment {
    /// Inactivity and enrolment hash: FNV-1a 64 over the person count (`u32`,
    /// little-endian), then each person's kind and level (one byte each).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        column_hash(
            self.inactive_kind.len(),
            self.inactive_kind
                .iter()
                .zip(&self.enrolled)
                .flat_map(|(&k, &e)| [k as u8, e as u8]),
        )
    }
}
