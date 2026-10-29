//! Demographics (spec `economy/demographics`), roadmap M5.
//!
//! So far the **starting data** only, loaded into the `World` and read by no
//! mechanic yet: the demographic tables of the scenario's `demography.json`
//! (pre-pandemic mortality, 2019 fertility, the sex ratio at birth, the law's
//! dated retirement ages) and the career proxy that sets each person's
//! `contribution_months` and `career_points` at load ("2. Contributions",
//! "At load"). The People phase (ageing, deaths, births, retirement) comes
//! with its acceptance tests (AC-DEM-*). Pure functions, integer arithmetic,
//! no I/O.

use econ_types::{Bani, Date};

// The People phase's rules and the household life cycle's record operations:
// the test writer's API skeletons (ADR-0014), filled by the implementation.
pub mod records;
mod rules;
pub use rules::*;

/// Parts per billion.
pub type Ppb = u32;

/// One, in parts per billion.
pub const PPB: Ppb = 1_000_000_000;

/// Ages of the mortality table, 0 to 110 years: at 110 the monthly
/// probability is one.
pub const MORTALITY_AGES: usize = 111;

/// Sexes in the order of the tables: F 0, M 1 (`econ_popgen::Sex`).
pub const SEXES: usize = 2;

/// `mortality_rate[sex, a]`: the probability of dying within a year and the
/// monthly probability of the pipeline's rule, $m = 1 - (1 - q)^{1/12}$, by
/// sex and single year of age, parts per billion. Pre-pandemic on purpose
/// (owner, 2026-10-26): the average of 2017, 2018 and 2019, whose centre is
/// `base_year`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MortalityTable {
    /// The year the table stands for (2018, the centre of 2017–2019).
    pub base_year: u16,
    /// `[sex][age]`, a year.
    pub yearly: [[Ppb; MORTALITY_AGES]; SEXES],
    /// `[sex][age]`, a month.
    pub monthly: [[Ppb; MORTALITY_AGES]; SEXES],
}

/// `fertility_rate[a]`: births per woman a year by single year of the
/// mother's age, from `first_age`, millionths (the 2019 rates).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FertilityTable {
    /// The year of the rates.
    pub year: u16,
    /// The age of the first rate, in years.
    pub first_age: u16,
    /// Births per woman a year, millionths, one per single year from `first_age`.
    pub rates_ppm: Vec<u32>,
}

impl FertilityTable {
    /// The rate at an age in whole years; 0 outside the table.
    #[must_use]
    pub fn rate_ppm(&self, age_years: u16) -> u32 {
        age_years
            .checked_sub(self.first_age)
            .and_then(|k| self.rates_ppm.get(usize::from(k)))
            .copied()
            .unwrap_or(0)
    }

    /// The total fertility rate, millionths of a birth per woman.
    #[must_use]
    pub fn total_ppm(&self) -> u64 {
        self.rates_ppm.iter().map(|&r| u64::from(r)).sum()
    }
}

/// `retirement_age_schedule[sex]`: the law's dated standard retirement age,
/// months of age, each value in force from its date until the next.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetirementSchedule {
    /// `[sex]`: (from, months of age), dates ascending.
    pub dated: [Vec<(Date, u16)>; SEXES],
}

impl RetirementSchedule {
    /// The age in force for a sex on a date: the last value dated on or
    /// before it; `None` before the first.
    #[must_use]
    pub fn age_in_force(&self, sex: usize, on: Date) -> Option<u16> {
        self.dated
            .get(sex)?
            .iter()
            .take_while(|(from, _)| *from <= on)
            .last()
            .map(|&(_, months)| months)
    }
}

/// The demographic tables of a scenario (`demography.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Demography {
    /// Deaths.
    pub mortality: MortalityTable,
    /// Births.
    pub fertility: FertilityTable,
    /// `sex_ratio_at_birth`: the share of boys among births, millionths.
    pub sex_ratio_at_birth_ppm: u32,
    /// The law's dated retirement ages.
    pub retirement_age_schedule: RetirementSchedule,
}

/// Why a table was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableError {
    /// A probability above one.
    NotAProbability,
    /// A table of the wrong length.
    Shape,
    /// A schedule whose dates are not ascending, or that is empty.
    Schedule,
}

impl Demography {
    /// Check the tables: probabilities at most one, the monthly at most the
    /// yearly, one at 110; a non-empty fertility table; schedules ascending.
    ///
    /// # Errors
    /// See [`TableError`].
    pub fn check(&self) -> Result<(), TableError> {
        let m = &self.mortality;
        for sex in 0..SEXES {
            for a in 0..MORTALITY_AGES {
                if m.yearly[sex][a] > PPB || m.monthly[sex][a] > m.yearly[sex][a] {
                    return Err(TableError::NotAProbability);
                }
            }
            if m.monthly[sex][MORTALITY_AGES - 1] != PPB {
                return Err(TableError::NotAProbability);
            }
            let s = &self.retirement_age_schedule.dated[sex];
            if s.is_empty() || s.windows(2).any(|w| w[0].0 >= w[1].0) {
                return Err(TableError::Schedule);
            }
        }
        if self.fertility.rates_ppm.is_empty() || self.sex_ratio_at_birth_ppm > 1_000_000 {
            return Err(TableError::Shape);
        }
        Ok(())
    }
}

/// What a person did, for the career proxy (`career_density[activity]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CareerKind {
    /// Employee.
    Employee,
    /// Employer or own-account worker.
    SelfEmployed,
    /// Contributing family worker.
    FamilyWorker,
    /// Unemployed.
    Unemployed,
    /// Other inactive.
    OtherInactive,
    /// Pupil or student.
    PupilStudent,
    /// Retired (the spec gives no density; see [`CareerParams::spec_defaults`]).
    Retired,
    /// Below school age.
    Child,
}

/// The tuning parameters of the career proxy (spec, "Tuning parameters";
/// low confidence, calibrated in M7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CareerParams {
    /// `career_density` of an employee, employer or own-account worker,
    /// family worker, unemployed, other inactive, pupil or student, retired,
    /// child: millionths.
    pub career_density_ppm: [u32; 8],
    /// `nonemployed_wage_share`: the career wage of a person who is not an
    /// employee, over the mean wage of employees of the same sex and
    /// education, millionths.
    pub nonemployed_wage_share_ppm: u32,
}

impl CareerParams {
    /// The spec's defaults: 0.9, 0.6, 0.2, 0.7, 0.3, 1, and 0.8. The spec
    /// lists no density for the retired, whose pension points are already set
    /// (Stage G) and whose career no rule reads: they take the employee's 0.9.
    #[must_use]
    pub fn spec_defaults() -> Self {
        CareerParams {
            career_density_ppm: [
                900_000, 600_000, 200_000, 700_000, 300_000, 1_000_000, 900_000, 0,
            ],
            nonemployed_wage_share_ppm: 800_000,
        }
    }

    /// The density of a kind, millionths.
    #[must_use]
    pub fn density_ppm(&self, kind: CareerKind) -> u32 {
        self.career_density_ppm[kind as usize]
    }
}

/// A person's guessed work history at the start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Career {
    /// `contribution_months`.
    pub contribution_months: u16,
    /// `career_points`, millionths of a point.
    pub career_points: u64,
}

/// Years of schooling above upper secondary of an education level (codes of
/// `econ_popgen::EduLevel`: post-secondary 2, bachelor 4, master or
/// doctorate 6), as in social transfers' proxy.
#[must_use]
pub fn extra_school_years(edu_code: u8) -> u16 {
    match edu_code {
        2 => 2,
        3 => 4,
        4 => 6,
        _ => 0,
    }
}

/// `n / d` rounded half up (`n >= 0`, `d > 0`).
fn round_div(n: u128, d: u128) -> u128 {
    (n + d / 2) / d
}

/// The career proxy at load (spec, "2. Contributions", "At load"):
///
/// years = clamp(age − 18 − extra_school, 0, age − 15) × `career_density`,
///
/// in months, `contribution_months` = 12 × years and `career_points` =
/// years × career wage ÷ `wage_nominal`. `career_wage` is the person's own
/// gross wage for an employee, else the mean wage of employees of the
/// person's sex and education × `nonemployed_wage_share` (the caller's).
/// Rounded half up; no points when `wage_nominal` is not positive.
#[must_use]
pub fn career_proxy(
    age_months: u16,
    extra_school: u16,
    density_ppm: u32,
    career_wage: Bani,
    wage_nominal: Bani,
) -> Career {
    let age = u32::from(age_months);
    let upper = age.saturating_sub(15 * 12);
    let base = age
        .saturating_sub(18 * 12)
        .saturating_sub(u32::from(extra_school) * 12)
        .min(upper);
    let months = round_div(u128::from(base) * u128::from(density_ppm), 1_000_000);
    let contribution_months = u16::try_from(months).unwrap_or(u16::MAX);
    let wage = u128::try_from(career_wage.get().max(0)).unwrap_or(0);
    let nominal = u128::try_from(wage_nominal.get().max(0)).unwrap_or(0);
    let career_points = if nominal == 0 {
        0
    } else {
        let points = round_div(
            u128::from(contribution_months) * wage * 1_000_000,
            12 * nominal,
        );
        u64::try_from(points).unwrap_or(u64::MAX)
    };
    Career {
        contribution_months,
        career_points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u8) -> Date {
        Date {
            year,
            month,
            day: 1,
        }
    }

    #[test]
    fn retirement_age_follows_the_dated_schedule() {
        let s = RetirementSchedule {
            dated: [
                vec![(date(2021, 12), 741), (date(2022, 7), 742)],
                vec![(date(2021, 12), 780)],
            ],
        };
        assert_eq!(s.age_in_force(0, date(2021, 11)), None);
        assert_eq!(s.age_in_force(0, date(2021, 12)), Some(741));
        assert_eq!(s.age_in_force(0, date(2022, 6)), Some(741));
        assert_eq!(s.age_in_force(0, date(2022, 7)), Some(742));
        assert_eq!(s.age_in_force(1, date(2040, 1)), Some(780));
        assert_eq!(s.age_in_force(2, date(2040, 1)), None);
    }

    #[test]
    fn fertility_rates_by_single_year() {
        let f = FertilityTable {
            year: 2019,
            first_age: 15,
            rates_ppm: vec![10, 20, 30],
        };
        assert_eq!(f.rate_ppm(14), 0);
        assert_eq!(f.rate_ppm(16), 20);
        assert_eq!(f.rate_ppm(18), 0);
        assert_eq!(f.total_ppm(), 60);
    }

    #[test]
    fn career_proxy_counts_adult_years_at_the_density() {
        let wage = Bani::from_lei(5_000);
        let mean = Bani::from_lei(5_000);
        // 40 years old, upper secondary, employee: 22 years at 0.9 = 19.8 years.
        let c = career_proxy(40 * 12, 0, 900_000, wage, mean);
        assert_eq!(c.contribution_months, 238); // 264 × 0.9 = 237.6
        // At the mean wage, a point a year: 238 / 12 points.
        assert_eq!(c.career_points, 19_833_333);
        // Twice the mean wage, twice the points.
        let rich = career_proxy(40 * 12, 0, 900_000, wage.times(2), mean);
        assert_eq!(rich.career_points, 39_666_667);
        // A bachelor's degree: four years less.
        assert_eq!(
            career_proxy(40 * 12, 4, 1_000_000, wage, mean).contribution_months,
            216
        );
        // Under 18, nothing; a pupil of 17 has none.
        assert_eq!(
            career_proxy(17 * 12, 0, 1_000_000, wage, mean),
            Career::default()
        );
        // No wage_nominal, no points.
        assert_eq!(
            career_proxy(40 * 12, 0, 900_000, wage, Bani::ZERO).career_points,
            0
        );
    }

    #[test]
    fn spec_defaults_and_extra_school() {
        let p = CareerParams::spec_defaults();
        assert_eq!(p.density_ppm(CareerKind::Employee), 900_000);
        assert_eq!(p.density_ppm(CareerKind::FamilyWorker), 200_000);
        assert_eq!(p.density_ppm(CareerKind::PupilStudent), 1_000_000);
        assert_eq!(p.density_ppm(CareerKind::Child), 0);
        assert_eq!(p.nonemployed_wage_share_ppm, 800_000);
        assert_eq!(
            (0..5).map(extra_school_years).collect::<Vec<_>>(),
            [0, 0, 2, 4, 6]
        );
    }
}
