//! Roadmap M5's starting data on the `World` ("People over time"): loaded at
//! [`World::new`] and kept, **read by no phase yet**, and outside the state
//! hash, so that loading it changes no result and no golden value. The
//! People phase of `economy/demographics`, the household life cycle,
//! migration and education flows will read and change it, and join the hash
//! when they do.
//!
//! What it holds:
//! - per person: `sex`, `role`, `person_key` (the scenario's: the start's
//!   index), the guessed career (`contribution_months`, `career_points`, the
//!   proxy of `economy/demographics`, "At load") and stage K's `enrolled`;
//!   the age is the `World`'s own column ([`World::person_ages`]);
//! - per household: `hh_key` (the start's index);
//! - the linked emigrants (stage L), with their own `person_key`s after the
//!   residents', and the diaspora stock;
//! - the demographic tables (pre-pandemic mortality, 2019 fertility, the sex
//!   ratio at birth, the law's dated retirement ages);
//! - the school places `edu_capacity[ℓ]` (spec `society/education`, "Starting
//!   state"): the weighted persons enrolled at each level at load;
//! - the next free `person_key` and `hh_key`.

use econ_mech_demography::{
    CareerKind, CareerParams, Demography, career_proxy, extra_school_years,
};
use econ_popgen::{DiasporaCell, EduLevel, Enrolled, Role, Sex};
use econ_types::Bani;

use super::{StartState, World};

/// Stage C's activity codes.
const CHILD: u8 = 0;
const PUPIL: u8 = 1;
const STUDENT: u8 = 2;
const EMPLOYED: u8 = 3;
const UNEMPLOYED: u8 = 4;
const RETIRED: u8 = 5;
/// Stage D's status in employment.
const EMPLOYEE: u8 = 1;
const FAMILY_WORKER: u8 = 4;
/// Levels of `enrolled`, none included.
pub const ENROLMENT: usize = 8;

/// The linked emigrants of the scenario (stage L): persons of resident
/// households who live abroad (`abroad`), in their own table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct LinkedEmigrantColumns {
    /// Index into the households table.
    pub household_id: Vec<u32>,
    pub person_key: Vec<u64>,
    /// Months.
    pub age: Vec<u16>,
    pub sex: Vec<Sex>,
    pub role: Vec<Role>,
    pub edu_level: Vec<EduLevel>,
    /// `months_abroad`.
    pub months_abroad: Vec<u16>,
}

/// What the scenario gives the people of M5 (part of [`StartState`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeopleStart {
    /// Per person; empty means F for everyone (a scenario without it).
    pub sex: Vec<Sex>,
    /// Per person; empty means head for everyone.
    pub role: Vec<Role>,
    /// Per person; empty means the index.
    pub person_key: Vec<u64>,
    /// Per household; empty means the index.
    pub hh_key: Vec<u64>,
    /// Per person, stage K; empty means not enrolled.
    pub enrolled: Vec<Enrolled>,
    /// Stage L.
    pub linked_emigrants: LinkedEmigrantColumns,
    /// `diaspora_stock[k]`.
    pub diaspora_stock: Vec<DiasporaCell>,
    /// The demographic tables; `None` for a scenario without `demography.json`.
    pub demography: Option<Demography>,
    /// Moves between counties' inputs; `None` for a scenario without
    /// `county_moves.json`.
    pub county_moves: Option<econ_popgen::CountyMoves>,
    /// The next free `person_key`; 0 means after every key above.
    pub next_person_key: u64,
    /// The next free `hh_key`; 0 means after every key above.
    pub next_hh_key: u64,
}

/// Roadmap M5's people data on the `World` ([`World::people`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct People {
    /// Per person.
    pub sex: Vec<Sex>,
    /// Per person.
    pub role: Vec<Role>,
    /// Per person: `person_key`, never reused.
    pub person_key: Vec<u64>,
    /// Per person: `contribution_months`, the career proxy at load.
    pub contribution_months: Vec<u16>,
    /// Per person: `career_points`, millionths of a point.
    pub career_points: Vec<u64>,
    /// Per person: the level enrolled at (stage K).
    pub enrolled: Vec<Enrolled>,
    /// Per household: `hh_key`, never reused.
    pub hh_key: Vec<u64>,
    /// The linked emigrants.
    pub linked_emigrants: LinkedEmigrantColumns,
    /// `diaspora_stock[k]`, real persons.
    pub diaspora_stock: Vec<DiasporaCell>,
    /// The demographic tables.
    pub demography: Option<Demography>,
    /// Moves between counties' inputs.
    pub county_moves: Option<econ_popgen::CountyMoves>,
    /// `edu_capacity[ℓ]` by `Enrolled` (index 0, not enrolled, is 0):
    /// weighted persons.
    pub edu_capacity: [u64; ENROLMENT],
    /// `wage_nominal` at load: the mean gross wage of employees, a month
    /// (the career proxy's divisor).
    pub wage_nominal: Bani,
    /// The next free `person_key`.
    pub next_person_key: u64,
    /// The next free `hh_key`.
    pub next_hh_key: u64,
}

/// The career proxy's kind of a person (stage C's activity, stage D's status).
fn career_kind(activity: u8, status: u8) -> CareerKind {
    match (activity, status) {
        (EMPLOYED, EMPLOYEE) => CareerKind::Employee,
        (EMPLOYED, FAMILY_WORKER) => CareerKind::FamilyWorker,
        (EMPLOYED, _) => CareerKind::SelfEmployed,
        (UNEMPLOYED, _) => CareerKind::Unemployed,
        (PUPIL | STUDENT, _) => CareerKind::PupilStudent,
        (RETIRED, _) => CareerKind::Retired,
        (CHILD, _) => CareerKind::Child,
        _ => CareerKind::OtherInactive,
    }
}

/// Integer mean, rounded half up; 0 of nothing.
fn mean(sum: u128, weight: u128) -> Bani {
    if weight == 0 {
        return Bani::ZERO;
    }
    Bani(i64::try_from((sum + weight / 2) / weight).expect("a wage"))
}

/// The column if it has one value per record, else `none` for each.
fn filled<T: Copy>(n: usize, v: &[T], none: T) -> Vec<T> {
    if v.len() == n {
        v.to_vec()
    } else {
        vec![none; n]
    }
}

/// Build the people data at load. `person_weight` is each person's
/// household weight.
pub(super) fn load(s: &StartState, person_weight: &[u64]) -> People {
    let p = &s.people;
    let n_p = s.household_id.len();
    let n_hh = s.hh_weight.len();
    let or_index = |v: &[u64], n: usize| -> Vec<u64> {
        if v.len() == n {
            v.to_vec()
        } else {
            (0..n as u64).collect()
        }
    };
    let sex: Vec<Sex> = filled(n_p, &p.sex, Sex::F);
    let role: Vec<Role> = filled(n_p, &p.role, Role::Head);
    let enrolled: Vec<Enrolled> = filled(n_p, &p.enrolled, Enrolled::None);
    let person_key = or_index(&p.person_key, n_p);
    let hh_key = or_index(&p.hh_key, n_hh);

    // wage_nominal and the mean wage of employees by sex and education.
    let is_employee =
        |i: usize| s.employment_status.get(i) == Some(&EMPLOYEE) && s.wage[i].get() > 0;
    let (mut sum, mut weight) = (0u128, 0u128);
    let mut cell = [[(0u128, 0u128); 5]; 2];
    for i in (0..n_p).filter(|&i| is_employee(i)) {
        let (w, wage) = (
            u128::from(person_weight[i]),
            u128::try_from(s.wage[i].get()).unwrap_or(0),
        );
        sum += w * wage;
        weight += w;
        let c = &mut cell[sex[i] as usize][s.edu_level[i] as usize];
        c.0 += w * wage;
        c.1 += w;
    }
    let wage_nominal = mean(sum, weight);
    let params = CareerParams::spec_defaults();
    let share = |b: Bani| b.mul_ratio(i64::from(params.nonemployed_wage_share_ppm), 1_000_000);
    let (mut contribution_months, mut career_points) = (vec![0u16; n_p], vec![0u64; n_p]);
    for i in 0..n_p {
        let kind = career_kind(
            s.activity.get(i).copied().unwrap_or(CHILD),
            s.employment_status.get(i).copied().unwrap_or(0),
        );
        let (sum, w) = cell[sex[i] as usize][s.edu_level[i] as usize];
        let career_wage = if kind == CareerKind::Employee && s.wage[i].get() > 0 {
            s.wage[i]
        } else if w > 0 {
            share(mean(sum, w))
        } else {
            share(wage_nominal)
        };
        let c = career_proxy(
            s.age[i],
            extra_school_years(s.edu_level[i] as u8),
            params.density_ppm(kind),
            career_wage,
            wage_nominal,
        );
        contribution_months[i] = c.contribution_months;
        career_points[i] = c.career_points;
    }

    // School places: the weighted persons enrolled at each level.
    let mut edu_capacity = [0u64; ENROLMENT];
    for i in 0..n_p {
        if enrolled[i] != Enrolled::None {
            edu_capacity[enrolled[i] as usize] += person_weight[i];
        }
    }

    let linked = p.linked_emigrants.clone();
    let max_key = |keys: &[u64]| keys.iter().max().map_or(0, |&k| k + 1);
    let next_person_key = p
        .next_person_key
        .max(max_key(&person_key))
        .max(max_key(&linked.person_key));
    let next_hh_key = p.next_hh_key.max(max_key(&hh_key));
    People {
        sex,
        role,
        person_key,
        contribution_months,
        career_points,
        enrolled,
        hh_key,
        linked_emigrants: linked,
        diaspora_stock: p.diaspora_stock.clone(),
        demography: p.demography.clone(),
        county_moves: p.county_moves.clone(),
        edu_capacity,
        wage_nominal,
        next_person_key,
        next_hh_key,
    }
}

impl World {
    /// Roadmap M5's starting data: sex, role, stable identities, careers,
    /// enrolment, linked emigrants, the diaspora stock, the demographic
    /// tables and the school places (read by no phase yet).
    #[must_use]
    pub fn people(&self) -> &People {
        &self.people
    }

    /// Every person's age in months, aligned with [`World::persons`].
    #[must_use]
    pub fn person_ages(&self) -> &[u16] {
        &self.pp.age
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn career_kinds_follow_activity_and_status() {
        assert_eq!(career_kind(EMPLOYED, EMPLOYEE), CareerKind::Employee);
        assert_eq!(career_kind(EMPLOYED, 2), CareerKind::SelfEmployed);
        assert_eq!(career_kind(EMPLOYED, 3), CareerKind::SelfEmployed);
        assert_eq!(
            career_kind(EMPLOYED, FAMILY_WORKER),
            CareerKind::FamilyWorker
        );
        assert_eq!(career_kind(UNEMPLOYED, 0), CareerKind::Unemployed);
        assert_eq!(career_kind(PUPIL, 0), CareerKind::PupilStudent);
        assert_eq!(career_kind(STUDENT, 0), CareerKind::PupilStudent);
        assert_eq!(career_kind(RETIRED, 0), CareerKind::Retired);
        assert_eq!(career_kind(6, 0), CareerKind::OtherInactive);
        assert_eq!(career_kind(CHILD, 0), CareerKind::Child);
    }

    #[test]
    fn a_small_start_gets_keys_careers_and_places() {
        let s = StartState {
            hh_weight: vec![100, 50],
            household_id: vec![0, 0, 1],
            age: vec![40 * 12, 10 * 12, 30 * 12],
            edu_level: vec![EduLevel::Isced3, EduLevel::Isced0To2, EduLevel::Isced5To6],
            activity: vec![EMPLOYED, PUPIL, UNEMPLOYED],
            employment_status: vec![EMPLOYEE, 0, 0],
            wage: vec![Bani::from_lei(4_000), Bani::ZERO, Bani::ZERO],
            people: PeopleStart {
                enrolled: vec![Enrolled::None, Enrolled::Primary, Enrolled::None],
                ..PeopleStart::default()
            },
            ..StartState::default()
        };
        let p = load(&s, &[100, 100, 50]);
        assert_eq!(p.person_key, [0, 1, 2]);
        assert_eq!(p.hh_key, [0, 1]);
        assert_eq!((p.next_person_key, p.next_hh_key), (3, 2));
        assert_eq!(p.wage_nominal, Bani::from_lei(4_000));
        // The employee: 22 years at 0.9, a point a year at the mean wage.
        assert_eq!(p.contribution_months[0], 238);
        assert_eq!(p.career_points[0], 19_833_333);
        // The pupil of 10: nothing.
        assert_eq!((p.contribution_months[1], p.career_points[1]), (0, 0));
        // The unemployed graduate of 30: 8 years at 0.7, at 0.8 of the
        // overall mean (no employee of her sex and education).
        assert_eq!(p.contribution_months[2], 67);
        assert_eq!(p.career_points[2], 4_466_667);
        assert_eq!(p.edu_capacity[Enrolled::Primary as usize], 100);
        assert_eq!(p.edu_capacity.iter().sum::<u64>(), 100);
    }
}
