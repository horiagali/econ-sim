//! Education's first version (spec `society/education`, "First version
//! (roadmap M5)", feature `education`): the pure rules of the June close
//! (dropout, progression, completion, continuation, leaving) and of the
//! August close (places), the yearly report, and the read-only columns.
//!
//! The API is the spec's sketch, written with the acceptance tests before the
//! code (ADR-0014). Rates are millionths; levels are indexed from pre-school
//! (0) to master or doctorate (6), `enrolled` − 1. The draws are keyed by
//! the day of the close and the person's `person_key` (stream `Education`).

use econ_popgen::{Activity, EduLevel, Enrolled, NO_EMPLOYER};
use econ_rng::Stream;

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
///
/// A pre-school child who will be 6 on 1 September (two months after the
/// close) leaves it and counts as completing it; a person enrolled at
/// post-secondary or above with `edu_level` 0 is treated as upper secondary
/// in their last year (the spec's edge case).
pub fn june_step(
    enrolled: Enrolled,
    study_year: u8,
    edu_level: EduLevel,
    age_months: u16,
    has_job: bool,
    draws: [u32; 3],
    p: &EducationParams,
) -> JuneOutcome {
    let leaver = |e: EduLevel| leaver_fate(e, age_months, has_job, draws[2], p);
    let out = |enrolled, study_year, edu_level, dropped_out, completed, leaver| JuneOutcome {
        enrolled,
        study_year,
        edu_level,
        dropped_out,
        completed,
        leaver,
    };
    let Some(mut l) = level_of(enrolled) else {
        return out(
            Enrolled::None,
            0,
            edu_level,
            false,
            false,
            leaver(edu_level),
        );
    };
    let mut year = study_year;
    if l >= 4 && edu_level == EduLevel::Isced0To2 {
        (l, year) = (3, LEVEL_YEARS[3] - 1);
    }
    // Step 1: dropout.
    if draws[0] < p.dropout_ppm[l] {
        return out(Enrolled::None, 0, edu_level, true, false, leaver(edu_level));
    }
    // Pre-school: leaves at 6 on 1 September, no leaver (under 16).
    if l == 0 {
        if (age_months + 2) / 12 >= 6 {
            return out(Enrolled::None, 0, edu_level, false, true, None);
        }
        let y = (year + 1).min(LEVEL_YEARS[0] - 1);
        return out(Enrolled::PreSchool, y, edu_level, false, false, None);
    }
    // Step 2: progression and completion.
    if year + 1 < LEVEL_YEARS[l] {
        return out(enrolment_of(l), year + 1, edu_level, false, false, None);
    }
    let edu = edu_level.max(COMPLETION[l]);
    let u2 = draws[1];
    let next = match l {
        1 => Some(2),
        2 => (u2 < p.continue_lower_to_upper_ppm).then_some(3),
        3 if u2 < p.continue_upper_to_tertiary_ppm => Some(5),
        3 if u2 - p.continue_upper_to_tertiary_ppm < p.continue_upper_to_post_ppm => Some(4),
        5 => (u2 < p.continue_bachelor_to_master_ppm).then_some(6),
        _ => None,
    };
    match next {
        Some(n) => out(enrolment_of(n), 0, edu, false, true, None),
        None => out(Enrolled::None, 0, edu, false, true, leaver(edu)),
    }
}

/// The level a completer reaches at least, by level (pre-school and primary:
/// none, lower secondary: ISCED 0–2).
const COMPLETION: [EduLevel; 7] = [
    EduLevel::Isced0To2,
    EduLevel::Isced0To2,
    EduLevel::Isced0To2,
    EduLevel::Isced3,
    EduLevel::Isced4,
    EduLevel::Isced5To6,
    EduLevel::Isced7To8,
];

/// Step 3: what becomes of a person leaving school; `None` under the minimum
/// leaving age.
fn leaver_fate(
    edu: EduLevel,
    age_months: u16,
    has_job: bool,
    u3: u32,
    p: &EducationParams,
) -> Option<LeaverFate> {
    if age_months < u16::from(p.min_leaving_age) * 12 {
        None
    } else if has_job {
        Some(LeaverFate::KeepsJob)
    } else if u3 < p.leaver_seek_ppm[edu as usize] {
        Some(LeaverFate::Seeking)
    } else {
        Some(LeaverFate::Inactive)
    }
}

/// The level index of an enrolment (`None` for not enrolled).
fn level_of(e: Enrolled) -> Option<usize> {
    match e {
        Enrolled::None => None,
        other => Some(other as usize - 1),
    }
}

/// The enrolment of a level index.
fn enrolment_of(l: usize) -> Enrolled {
    [
        Enrolled::PreSchool,
        Enrolled::Primary,
        Enrolled::LowerSecondary,
        Enrolled::UpperSecondary,
        Enrolled::PostSecondary,
        Enrolled::Tertiary,
        Enrolled::MasterDoctorate,
    ][l]
}

/// Admission against places (AC-EDU-03): for each entrant `(priority, index,
/// weight)`, in input order, whether it is admitted. Entrants are taken in
/// ascending (priority, index) while the continuing and admitted weight stays
/// within the places; the first that does not fit and all after it are
/// refused.
pub fn admit(places: u64, continuing: u64, entrants: &[(u64, u32, u32)]) -> Vec<bool> {
    let mut order: Vec<usize> = (0..entrants.len()).collect();
    order.sort_unstable_by_key(|&i| (entrants[i].0, entrants[i].1));
    let mut out = vec![false; entrants.len()];
    let mut total = continuing;
    for i in order {
        let w = u64::from(entrants[i].2);
        if total + w > places {
            break;
        }
        total += w;
        out[i] = true;
    }
    out
}

/// Draw words of a close.
const DRAW_DROPOUT: u32 = 0;
const DRAW_CONTINUE: u32 = 1;
const DRAW_SEEK: u32 = 2;
const DRAW_INTAKE: u32 = 3;
const DRAW_PRIORITY: u32 = 4;

/// A person given a level at the June close (continuing from the level
/// below): an entrant of the August close.
const ENTRANT: u8 = 1;
/// A child who left pre-school at the June close: among the primary intake.
const FROM_PRESCHOOL: u8 = 2;

/// Education's state on the `World`, per person aligned with the persons
/// table (`enrolled` is [`super::people::People::enrolled`]).
#[derive(Debug, Clone, Default)]
pub(crate) struct EduState {
    study_year: Vec<u8>,
    /// [`ENTRANT`], [`FROM_PRESCHOOL`] or 0, from a June close to the
    /// August close after it.
    pending: Vec<u8>,
    year: Option<EducationYear>,
    early_leavers_ppm: u32,
    tertiary_ppm: u32,
}

impl World {
    /// The latest June or August report; `None` before the first.
    #[must_use]
    pub fn education_year(&self) -> Option<&EducationYear> {
        self.edu.year.as_ref()
    }

    /// FNV-1a 64 over the person count, then each person's `enrolled`,
    /// `study_year`, `edu_level` and `activity`, one byte each (AC-EDU-09).
    #[must_use]
    pub fn education_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |b: u8| {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        };
        let n = self.pp.age.len();
        for b in (n as u64).to_le_bytes() {
            eat(b);
        }
        let act = self.activity();
        for p in 0..n {
            eat(self.people.enrolled[p] as u8);
            eat(self.edu.study_year.get(p).copied().unwrap_or(0));
            eat(self.persons.edu_level[p] as u8);
            eat(act.get(p).copied().unwrap_or(Activity::Child) as u8);
        }
        h
    }

    /// The weighted share of persons of 18 to 24 with `edu_level` 0 who are
    /// not enrolled, millionths, at the latest June close.
    #[must_use]
    pub fn early_leavers_rate_ppm(&self) -> u32 {
        self.edu.early_leavers_ppm
    }

    /// The weighted share of persons of 25 to 34 with `edu_level` 3 or 4,
    /// millionths, at the latest June close.
    #[must_use]
    pub fn tertiary_attainment_ppm(&self) -> u32 {
        self.edu.tertiary_ppm
    }

    /// Each person's `enrolled`, aligned with [`super::PersonColumns`].
    #[must_use]
    pub fn enrolled(&self) -> &[Enrolled] {
        &self.people.enrolled
    }

    /// Each person's `study_year`, aligned with [`super::PersonColumns`].
    #[must_use]
    pub fn study_year(&self) -> &[u8] {
        &self.edu.study_year
    }

    /// `edu_capacity`: the places of each level, weighted persons, pre-school
    /// first.
    #[must_use]
    pub fn edu_capacity(&self) -> [u64; 7] {
        let mut c = [0; 7];
        c.copy_from_slice(&self.people.edu_capacity[1..]);
        c
    }

    /// Load: each enrolled person's starting `study_year` (the age on
    /// 1 September 2021 less the level's starting age, else a keyed draw).
    pub(super) fn start_education(&mut self) {
        let n = self.pp.age.len();
        let mut years = vec![0u8; n];
        for (p, y) in years.iter_mut().enumerate() {
            let Some(l) = level_of(self.people.enrolled[p]) else {
                continue;
            };
            let age_sept = i32::from(self.pp.age[p].saturating_sub(3) / 12);
            let diff = age_sept - i32::from(LEVEL_START_AGE[l]);
            *y = if (0..i32::from(LEVEL_YEARS[l])).contains(&diff) {
                u8::try_from(diff).expect("a year")
            } else {
                let d = self
                    .seed
                    .fast_u64(Stream::Education, 0, self.people.person_key[p], 0);
                u8::try_from(d % u64::from(LEVEL_YEARS[l])).expect("a year")
            };
        }
        self.edu = EduState {
            study_year: years,
            pending: vec![0; n],
            ..EduState::default()
        };
    }

    /// Append education's columns for a new person row: a copy of person
    /// `from`'s, or a newborn's.
    #[cfg(feature = "demography")]
    pub(super) fn edu_push_person(&mut self, from: Option<usize>) {
        let n = self.pp.age.len();
        let e = &mut self.edu;
        let (y, k) = from.map_or((0, 0), |p| {
            (
                e.study_year.get(p).copied().unwrap_or(0),
                e.pending.get(p).copied().unwrap_or(0),
            )
        });
        e.study_year.resize(n - 1, 0);
        e.pending.resize(n - 1, 0);
        e.study_year.push(y);
        e.pending.push(k);
    }

    /// The yearly compaction of the People phase.
    #[cfg(feature = "demography")]
    pub(super) fn edu_compact(&mut self, keep_p: &[bool]) {
        super::money::retain_by(&mut self.edu.study_year, keep_p);
        super::money::retain_by(&mut self.edu.pending, keep_p);
    }

    /// The monthly close, after the month's statistics: the June close (end
    /// of the school year) and the August close (intake).
    pub(super) fn education_close(&mut self) {
        let n = self.pp.age.len();
        self.edu.study_year.resize(n, 0);
        self.edu.pending.resize(n, 0);
        match self.calendar.date(self.today).month {
            6 => self.edu_june(),
            8 => self.edu_august(),
            _ => {}
        }
    }

    /// A resident person's weight (0 for a tombstone or a freed record).
    fn edu_weight(&self, p: usize) -> u64 {
        if self.person_resident(p) {
            u64::from(self.households.hh_weight[self.persons.household_id[p] as usize])
        } else {
            0
        }
    }

    /// A draw of the close for person `p`, millionths.
    fn edu_draw(&self, p: usize, k: u32) -> u32 {
        let d = self.seed.fast_u64(
            Stream::Education,
            self.today.0,
            self.people.person_key[p],
            k,
        );
        u32::try_from(d % 1_000_000).expect("millionths")
    }

    fn edu_has_job(&self, p: usize) -> bool {
        self.persons.employer[p] != NO_EMPLOYER || (2..=4).contains(&self.pp.status[p])
    }

    /// Step 3 for a leaver: the activity and job search of their fate.
    fn edu_leave(&mut self, p: usize, fate: LeaverFate, w: u64, y: &mut EducationYear) {
        let e = self.persons.edu_level[p] as usize;
        match fate {
            LeaverFate::Seeking => {
                self.pp.seeking[p] = true;
                self.persons.months_unemployed[p] = 0;
                y.leavers_seeking[e] += w;
            }
            LeaverFate::Inactive => {
                self.pp.seeking[p] = false;
                y.leavers_inactive[e] += w;
            }
            LeaverFate::KeepsJob => y.leavers_employed[e] += w,
        }
        self.set_base_activity(p, Activity::InactiveOther);
    }

    /// The June close: dropout, progression, completion, continuation and
    /// leaving.
    fn edu_june(&mut self) {
        let params = self.params.education;
        let mut y = EducationYear::default();
        let n = self.pp.age.len();
        for p in 0..n {
            let w = self.edu_weight(p);
            if w == 0 {
                continue;
            }
            let e = self.people.enrolled[p];
            let before = level_of(e);
            let act = self.activity().get(p).copied().unwrap_or(Activity::Child);
            if before.is_none() && !matches!(act, Activity::Pupil | Activity::Student) {
                continue;
            }
            let draws = [
                self.edu_draw(p, DRAW_DROPOUT),
                self.edu_draw(p, DRAW_CONTINUE),
                self.edu_draw(p, DRAW_SEEK),
            ];
            let o = june_step(
                e,
                self.edu.study_year[p],
                self.persons.edu_level[p],
                self.pp.age[p],
                self.edu_has_job(p),
                draws,
                &params,
            );
            self.people.enrolled[p] = o.enrolled;
            self.edu.study_year[p] = o.study_year;
            self.persons.edu_level[p] = o.edu_level;
            if let Some(l) = before {
                let x = &mut y.levels[l];
                x.before += w;
                if o.dropped_out {
                    x.dropouts += w;
                }
                if o.completed {
                    x.completers += w;
                    if let Some(next) = level_of(o.enrolled) {
                        x.continuing_to[next] += w;
                        y.levels[next].entrants += w;
                        self.edu.pending[p] = ENTRANT;
                    }
                }
            }
            match o.leaver {
                Some(fate) => self.edu_leave(p, fate, w, &mut y),
                None if before.is_some() && o.enrolled == Enrolled::None => {
                    // Under 16: a pupil not enrolled; from pre-school, among
                    // the primary intake.
                    self.set_base_activity(p, Activity::Pupil);
                    if before == Some(0) {
                        self.edu.pending[p] = FROM_PRESCHOOL;
                    }
                }
                None => {}
            }
        }
        self.edu_after(&mut y);
        // The derived indicators.
        let (mut young, mut early, mut adults, mut tertiary) = (0u64, 0u64, 0u64, 0u64);
        for p in 0..n {
            let w = self.edu_weight(p);
            let age = self.pp.age[p] / 12;
            let edu = self.persons.edu_level[p];
            if (18..=24).contains(&age) {
                young += w;
                if edu == EduLevel::Isced0To2 && self.people.enrolled[p] == Enrolled::None {
                    early += w;
                }
            } else if (25..=34).contains(&age) {
                adults += w;
                if edu >= EduLevel::Isced5To6 {
                    tertiary += w;
                }
            }
        }
        let share = |a: u64, b: u64| {
            u32::try_from(u128::from(a) * 1_000_000 / u128::from(b.max(1))).expect("a share")
        };
        self.edu.early_leavers_ppm = share(early, young);
        self.edu.tertiary_ppm = share(tertiary, adults);
        self.edu.year = Some(y);
    }

    /// The August close: the pre-school and primary intake, then each
    /// level's entrants against its places. The report's `before` is the
    /// continuing students: June's continuers are entrants here again.
    fn edu_august(&mut self) {
        let params = self.params.education;
        let mut y = EducationYear::default();
        let n = self.pp.age.len();
        // Steps 1 and 2: the intake.
        for p in 0..n {
            let w = self.edu_weight(p);
            if w == 0 || self.people.enrolled[p] != Enrolled::None {
                continue;
            }
            let child =
                self.activity().get(p).copied().unwrap_or(Activity::Child) == Activity::Child;
            let age = self.pp.age[p] / 12;
            let rate = if self.edu.pending[p] == FROM_PRESCHOOL || child && age == 6 {
                (params.primary_intake_ppm, Enrolled::Primary)
            } else if child && (3..=5).contains(&age) {
                (
                    params.preschool_intake_ppm[usize::from(age - 3)],
                    Enrolled::PreSchool,
                )
            } else {
                continue;
            };
            if self.edu_draw(p, DRAW_INTAKE) < rate.0 {
                self.people.enrolled[p] = rate.1;
                self.edu.study_year[p] = 0;
                self.edu.pending[p] = ENTRANT;
            } else if rate.1 == Enrolled::Primary {
                self.set_base_activity(p, Activity::Pupil);
            }
        }
        // Step 3: places.
        let mut continuing = [0u64; 7];
        let mut entrants: [Vec<(u64, u32, u32)>; 7] = Default::default();
        for p in 0..n {
            let w = self.edu_weight(p);
            let Some(l) = level_of(self.people.enrolled[p]) else {
                continue;
            };
            if w == 0 {
                continue;
            }
            if self.edu.pending[p] == ENTRANT {
                let prio = self.seed.fast_u64(
                    Stream::Education,
                    self.today.0,
                    self.people.person_key[p],
                    DRAW_PRIORITY,
                );
                let i = u32::try_from(p).expect("a person");
                entrants[l].push((prio, i, u32::try_from(w).expect("a weight")));
            } else {
                continuing[l] += w;
            }
        }
        let caps = self.edu_capacity();
        for l in 0..7 {
            y.levels[l].before = continuing[l];
            let ok = admit(caps[l], continuing[l], &entrants[l]);
            for (&(_, i, w), admitted) in entrants[l].iter().zip(ok) {
                let (p, w) = (i as usize, u64::from(w));
                y.levels[l].entrants += w;
                if admitted {
                    continue;
                }
                y.levels[l].refused += w;
                self.people.enrolled[p] = Enrolled::None;
                self.edu.study_year[p] = 0;
                let fate = leaver_fate(
                    self.persons.edu_level[p],
                    self.pp.age[p],
                    self.edu_has_job(p),
                    self.edu_draw(p, DRAW_SEEK),
                    &params,
                );
                match fate {
                    Some(f) => self.edu_leave(p, f, w, &mut y),
                    None if l > 0 => self.set_base_activity(p, Activity::Pupil),
                    None => {}
                }
            }
        }
        self.edu.pending.iter_mut().for_each(|k| *k = 0);
        self.edu_after(&mut y);
        self.edu.year = Some(y);
    }

    /// The report's enrolled after the close, by level.
    fn edu_after(&self, y: &mut EducationYear) {
        for p in 0..self.pp.age.len() {
            if let Some(l) = level_of(self.people.enrolled[p]) {
                y.levels[l].after += self.edu_weight(p);
            }
        }
    }
}
