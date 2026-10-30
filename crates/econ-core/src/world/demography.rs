//! Demographics and the household life cycle on the `World` (feature
//! `demography`; specs `economy/demographics` and
//! `society/households-lifecycle`, roadmap M5): the People phase's
//! parameters, the person and household columns it adds to the loaded
//! people data ([`World::people`]), the report of its
//! latest run, the retirement age in force and the year's mortality table.
//!
//! Test writer's addition (ADR-0014), written with the `[sim]` acceptance
//! tests (AC-DEM-03 to 09, 11 to 16, 18 to 20; AC-HHL-05 to 10) before the
//! code; the implementation is in `people_phase` (ageing, deaths and
//! retirement so far).
//!
//! Person columns are aligned with [`super::PersonColumns`] and household
//! columns with [`super::HouseholdColumns`]; both tables keep tombstones (a
//! dead person, an ended household) until the yearly compaction, and new
//! records are appended. Money is bani per real household unless said
//! otherwise; points are millionths of a point; ages are months.

use econ_mech_demography::records::LifeCycleParams;
use econ_mech_demography::{FertilityResponse, MortalityTable};
use econ_popgen::{Activity, Sex};
use econ_types::{Bani, Month};

use super::World;

/// The People phase's tuning parameters (the specs' tables) and the switches
/// the acceptance tests compare runs with. Millionths unless said otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemographyParams {
    /// Fertility's responses to policy and the economy (step 4).
    pub fertility: FertilityResponse,
    /// The household life cycle's rates and shares.
    pub lifecycle: LifeCycleParams,
    /// `mortality_improvement`, a year.
    pub mortality_improvement_ppm: u32,
    /// `partner_fertility_ratio`.
    pub partner_fertility_ratio_ppm: u32,
    /// `sex_ratio_at_birth`: the share of boys.
    pub sex_ratio_at_birth_ppm: u32,
    /// `fertility_scale` (calibration only).
    pub fertility_scale_ppm: u32,
    /// `mortality_scale` (calibration only).
    pub mortality_scale_ppm: u32,
    /// The three fertility multipliers fixed at one (AC-DEM-20's
    /// comparison run); `false` in a game.
    pub fixed_fertility_multipliers: bool,
    /// Compact tombstones at every monthly close instead of the yearly one
    /// (AC-DEM-15's comparison run); `false` in a game.
    pub compact_monthly: bool,
    /// Run the People phase (AC-DEM-16 times a year without it); `true` in a
    /// game.
    pub people_phase: bool,
}

impl DemographyParams {
    /// The specs' defaults: mortality improvement 1.5% a year, partner
    /// fertility ratio 3, 51.4% boys, scales one, the comparison switches
    /// off and the People phase on.
    #[must_use]
    pub fn spec_defaults() -> DemographyParams {
        DemographyParams {
            fertility: FertilityResponse::spec_defaults(),
            lifecycle: LifeCycleParams::spec_defaults(),
            mortality_improvement_ppm: 15_000,
            partner_fertility_ratio_ppm: 3_000_000,
            sex_ratio_at_birth_ppm: 514_000,
            fertility_scale_ppm: 1_000_000,
            mortality_scale_ppm: 1_000_000,
            fixed_fertility_multipliers: false,
            compact_monthly: false,
            people_phase: true,
        }
    }
}

/// The person columns the People phase adds to the loaded people data
/// ([`World::people`]: `sex`, `role`, `person_key`, `contribution_months`,
/// `career_points`; ages in [`World::person_ages`]), aligned with
/// [`super::PersonColumns`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeopleLifeColumns {
    /// `alive`: the record counts (false for a tombstone).
    pub alive: Vec<bool>,
    /// Main activity (a job seeker is `Unemployed`, a pensioner `Retired`).
    pub activity: Vec<Activity>,
    /// `pension_points` (0 for a person who draws no pension).
    pub pension_points: Vec<u64>,
    /// `survivor_points`.
    pub survivor_points: Vec<u64>,
    /// `months_since_birth`: months since the woman last gave birth; `None`
    /// if she has not in the run.
    pub months_since_birth: Vec<Option<u16>>,
    /// `origin_household`: the `hh_key` the person left when leaving home.
    pub origin_household: Vec<Option<u64>>,
}

/// The household columns of the People phase (`hh_key` is in
/// [`World::people`]), aligned with [`super::HouseholdColumns`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HouseholdLifeColumns {
    /// `hh_alive`: the record counts (false for an ended household).
    pub hh_alive: Vec<bool>,
    /// `hh_county`.
    pub hh_county: Vec<u8>,
    /// `hh_collective`.
    pub hh_collective: Vec<bool>,
    /// Tenure: owner 0, tenant 1, other 2.
    pub hh_tenure: Vec<u8>,
    /// `hh_dwelling_value`.
    pub hh_dwelling_value: Vec<Bani>,
    /// `hh_mortgage`.
    pub hh_mortgage: Vec<Bani>,
    /// `hh_consumer_debt`.
    pub hh_consumer_debt: Vec<Bani>,
    /// `hh_moved_to_date`: money moved into the household by the life cycle's
    /// operations and estates since day 0, net (AC-HH-03's identity).
    pub hh_moved_to_date: Vec<Bani>,
}

/// A birth of the month: one record, standing for `weight` real births.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Birth {
    /// The newborn's key.
    pub child_key: u64,
    /// The newborn's sex.
    pub sex: Sex,
    /// The mother's key.
    pub mother_key: u64,
    /// The mother's age at the birth, months.
    pub mother_age: u16,
    /// The mother was head or partner of a household with a partner of the
    /// other sex.
    pub mother_in_couple: bool,
    /// The household's key.
    pub hh_key: u64,
    /// The household's weight.
    pub weight: u32,
}

/// A death of the month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Death {
    /// The person's key.
    pub person_key: u64,
    /// Sex.
    pub sex: Sex,
    /// Age at death, months.
    pub age: u16,
    /// The household's key.
    pub hh_key: u64,
    /// The household's weight.
    pub weight: u32,
    /// The person was retired.
    pub retired: bool,
    /// The person's `pension_points` (0 if not retired).
    pub pension_points: u64,
    /// The partner left in the household (the other of a head-and-partner
    /// pair), if any.
    pub partner_key: Option<u64>,
}

/// A retirement of the month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retirement {
    /// The person's key.
    pub person_key: u64,
    /// Sex.
    pub sex: Sex,
    /// Age, months.
    pub age: u16,
    /// The household's weight.
    pub weight: u32,
    /// Before the standard age.
    pub early: bool,
    /// The firm unit the person worked for as an employee, or
    /// `econ_popgen::NO_EMPLOYER`.
    pub employer_before: u16,
    /// The new `pension_points`.
    pub pension_points: u64,
}

/// An estate settled this month. Amounts are national (over the weights).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EstateRecord {
    /// The ended household's key.
    pub hh_key: u64,
    /// Its weight.
    pub weight: u32,
    /// Its cash and deposits before the settlement, times its weight.
    pub money: Bani,
    /// Its mortgage and consumer debt before the settlement, times its
    /// weight.
    pub debts: Bani,
    /// Debts repaid to the banks (flow code `loan`, 7003).
    pub repaid: Bani,
    /// Debts written off by the banks (`write_off`, 7004).
    pub written_off: Bani,
    /// Paid to the government (4109 `transfer.estate`).
    pub to_government: Bani,
    /// The heir household's key, if any.
    pub heir: Option<u64>,
    /// The heir's weight (0 without an heir).
    pub heir_weight: u32,
    /// The money the heir received: its per-household rise of cash and
    /// deposits times its weight.
    pub to_heir: Bani,
    /// The dwelling value before the settlement, times the weight.
    pub dwelling: Bani,
    /// The dwelling value the heir received, times its weight.
    pub to_heir_dwelling: Bani,
}

/// A young adult who left home this month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leaver {
    /// The person's key.
    pub person_key: u64,
    /// Sex.
    pub sex: Sex,
    /// Age, months.
    pub age: u16,
    /// The weight.
    pub weight: u32,
    /// The parents' household.
    pub from_hh: u64,
    /// The new household.
    pub new_hh: u64,
}

/// A union formed this month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Union {
    /// The woman's key.
    pub woman_key: u64,
    /// The man's key.
    pub man_key: u64,
    /// The merged household.
    pub hh_key: u64,
    /// Its weight.
    pub weight: u32,
}

/// A separation this month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Separation {
    /// The original household (the woman's, with the children).
    pub hh_key: u64,
    /// The man's new household.
    pub new_hh: u64,
    /// The man's key.
    pub man_key: u64,
    /// The woman's key.
    pub woman_key: u64,
    /// The weight.
    pub weight: u32,
    /// The original household's per-household cash before the separation.
    pub cash_before: Bani,
    /// The original household's per-household deposits before the
    /// separation.
    pub deposits_before: Bani,
    /// The per-household cash the man took.
    pub moved_cash: Bani,
    /// The per-household deposits the man took.
    pub moved_deposits: Bani,
}

/// What the household life cycle did this month (step 6 and the rules after
/// a death).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LifeCycleReport {
    /// Leaving home.
    pub leavers: Vec<Leaver>,
    /// Unions.
    pub unions: Vec<Union>,
    /// Separations.
    pub separations: Vec<Separation>,
    /// Children made collective records after the death of their last
    /// adult (keys).
    pub orphans: Vec<u64>,
    /// Every household record formed this month (move out, split, make
    /// collective), by key.
    pub households_formed: Vec<u64>,
    /// Splits of records.
    pub splits: u32,
}

/// The latest People phase.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeopleReport {
    /// The month it ran in; `None` before the first.
    pub month: Option<Month>,
    /// Births.
    pub births: Vec<Birth>,
    /// Deaths.
    pub deaths: Vec<Death>,
    /// Retirements, standard and early.
    pub retirements: Vec<Retirement>,
    /// Estates settled.
    pub estates: Vec<EstateRecord>,
    /// The births' alignment targets by the mother's five-year band (15–19
    /// to 45–49), millionths of a person, carry included.
    pub birth_targets_micro: [i64; 7],
    /// The household life cycle.
    pub life: LifeCycleReport,
}

impl World {
    /// The person columns the People phase adds.
    #[must_use]
    pub fn people_life(&self) -> &PeopleLifeColumns {
        &self.dem.life
    }

    /// The household columns of the People phase.
    #[must_use]
    pub fn household_life(&self) -> &HouseholdLifeColumns {
        &self.dem.hh_life
    }

    /// The report of the latest People phase.
    #[must_use]
    pub fn people_report(&self) -> &PeopleReport {
        &self.dem.report
    }

    /// `retirement_age[sex]` in force today, months of age.
    #[must_use]
    pub fn retirement_age_in_force(&self, sex: Sex) -> u16 {
        let date = self.calendar.date(self.today);
        self.people
            .demography
            .as_ref()
            .and_then(|d| {
                d.retirement_age_schedule
                    .age_in_force(usize::from(sex != Sex::F), date)
            })
            .unwrap_or(if sex == Sex::F { 741 } else { 780 })
    }

    /// The mortality table of the current calendar year.
    #[must_use]
    pub fn mortality_table(&self) -> &MortalityTable {
        self.dem
            .table
            .as_ref()
            .expect("the scenario has demographic tables")
    }
}
