//! The People phase (feature `demography`; spec `economy/demographics`,
//! "Periods"): at month end, after Pay, ageing, contributions, deaths and
//! retirement. Births, estates and the household life cycle come in later
//! steps of the implementation (roadmap M5).
//!
//! A dead person stays as a tombstone: `alive` false, with no job, no job
//! search, no pension and no income, so every phase that reads jobs,
//! pensions or incomes skips it; the phases that read ages check
//! [`World::person_alive`].

use econ_mech_demography::{
    Candidate, MORTALITY_AGES, MortalityTable, PensionLaw, align, new_pension_points,
    table_for_year,
};
use econ_popgen::{Activity, NO_EMPLOYER, Role, Sex};
use econ_types::{Bani, Month};

use super::World;
use super::demography::{Death, HouseholdLifeColumns, PeopleLifeColumns, PeopleReport, Retirement};
use super::money::ratio;

/// Five-year age bands of the death strata (0–4 … 100 and over).
const DEATH_BANDS: usize = 21;
/// Years before the standard age of early retirement's strata.
const EARLY_YEARS: usize = 10;
/// Keyed-draw purposes on `Stream::Demography`.
const PURPOSE_DEATH: u32 = 1;
const PURPOSE_EARLY: u32 = 2;
/// The law's pension parameters the transfers rule does not hold.
const FULL_YEARS: u8 = 35;
const MIN_YEARS: u8 = 15;
const EARLY_REDUCTION_PPM: u32 = 7_500;
const SURVIVOR_SHARE_PPM: u32 = 500_000;

fn sex_index(s: Sex) -> usize {
    usize::from(s != Sex::F)
}

/// The People phase's state.
#[derive(Debug, Clone, Default)]
pub(super) struct DemState {
    pub(super) active: bool,
    pub(super) life: PeopleLifeColumns,
    pub(super) hh_life: HouseholdLifeColumns,
    pub(super) report: PeopleReport,
    /// The base table (2018) and the current year's.
    base: Option<MortalityTable>,
    pub(super) table: Option<MortalityTable>,
    table_year: u16,
    /// Alignment carries, millionths of a person.
    carry_death: [[i64; DEATH_BANDS]; 2],
    carry_early: [[i64; EARLY_YEARS]; 2],
    /// Early retirement's monthly hazard by sex and whole years before the
    /// standard age (1 to 10), parts per billion.
    early_ppb: [[u32; EARLY_YEARS]; 2],
    /// `wage_nominal`: the mean gross wage of employees, a month.
    wage_nominal: Bani,
}

impl World {
    /// Load: the columns, the year's table and early retirement's hazard.
    pub(super) fn start_people_phase(&mut self, s: &super::StartState) {
        let activity = &s.activity;
        let n_p = self.persons.household_id.len();
        let n_hh = self.households.hh_weight.len();
        let Some(dem) = self.people.demography.clone() else {
            self.dem = DemState::default();
            return;
        };
        let act = |a: u8| match a {
            0 => Activity::Child,
            1 => Activity::Pupil,
            2 => Activity::Student,
            3 => Activity::Employed,
            4 => Activity::Unemployed,
            5 => Activity::Retired,
            _ => Activity::InactiveOther,
        };
        let activity: Vec<Activity> = (0..n_p).map(|p| act(activity[p])).collect();
        // Early retirement's hazard from the start's share retired R(a) at
        // each single year of age in the ten years before the standard age.
        let date = self.calendar.date(self.today);
        let mut early_ppb = [[0u32; EARLY_YEARS]; 2];
        for (sx, row) in early_ppb.iter_mut().enumerate() {
            let Some(std_age) = dem.retirement_age_schedule.age_in_force(sx, date) else {
                continue;
            };
            let std_years = usize::from(std_age / 12);
            let mut all = vec![0u128; std_years + 1];
            let mut ret = vec![0u128; std_years + 1];
            for p in 0..n_p {
                if sex_index(self.people.sex[p]) != sx {
                    continue;
                }
                let a = usize::from(self.pp.age[p] / 12);
                if a + EARLY_YEARS + 1 < std_years || a > std_years {
                    continue;
                }
                let w =
                    u128::from(self.households.hh_weight[self.persons.household_id[p] as usize]);
                all[a] += w;
                if activity[p] == Activity::Retired {
                    ret[a] += w;
                }
            }
            let share =
                |a: usize| -> u128 { (ret[a] * 1_000_000_000).checked_div(all[a]).unwrap_or(0) };
            for (k, v) in row.iter_mut().enumerate() {
                // k + 1 whole years before the standard age.
                let Some(a) = std_years.checked_sub(k + 1) else {
                    continue;
                };
                let (r0, r1) = (share(a), share(a + 1).max(share(a)));
                let h = if r0 >= 1_000_000_000 {
                    0
                } else {
                    (r1 - r0) * 1_000_000_000 / (1_000_000_000 - r0)
                };
                *v = u32::try_from(h / 12).unwrap_or(u32::MAX);
            }
        }
        let year = u16::try_from(date.year).expect("a year");
        let table = table_for_year(
            &dem.mortality,
            year,
            self.params.demography.mortality_improvement_ppm,
        );
        self.dem = DemState {
            active: self.params.demography.people_phase,
            life: PeopleLifeColumns {
                alive: vec![true; n_p],
                activity,
                pension_points: self.fs_points().to_vec(),
                survivor_points: vec![0; n_p],
                months_since_birth: vec![None; n_p],
                origin_household: vec![None; n_p],
            },
            hh_life: HouseholdLifeColumns {
                hh_alive: vec![true; n_hh],
                hh_county: s.hh_county.clone(),
                hh_collective: self.fs_collective().to_vec(),
                hh_tenure: s.hh_tenure.clone(),
                hh_dwelling_value: s.hh_dwelling_value.clone(),
                hh_mortgage: s.hh_mortgage.clone(),
                hh_consumer_debt: s.hh_consumer_debt.clone(),
                hh_moved_to_date: vec![Bani::ZERO; n_hh],
            },
            report: PeopleReport::default(),
            base: Some(dem.mortality),
            table: Some(table),
            table_year: year,
            carry_death: [[0; DEATH_BANDS]; 2],
            carry_early: [[0; EARLY_YEARS]; 2],
            early_ppb,
            wage_nominal: self.people.wage_nominal,
        };
    }

    /// The People phase, at month end after Pay.
    pub(super) fn people_phase(&mut self) {
        if !self.dem.active {
            return;
        }
        let t = u32::try_from(self.months.len()).expect("a month");
        self.dem.report = PeopleReport {
            month: Some(Month(t)),
            ..PeopleReport::default()
        };
        self.people_ageing();
        self.people_contributions();
        self.people_deaths();
        self.people_retirement();
    }

    /// The close of December: next year's mortality table and wage_nominal.
    pub(super) fn people_year_end(&mut self) {
        if !self.dem.active {
            return;
        }
        let year = u16::try_from(self.calendar.date(self.today).year + 1).expect("a year");
        if let Some(base) = &self.dem.base {
            self.dem.table = Some(table_for_year(
                base,
                year,
                self.params.demography.mortality_improvement_ppm,
            ));
            self.dem.table_year = year;
        }
        // wage_nominal: the mean gross wage of employees over the year.
        let n = self.months.len();
        let year_months = &self.months[n.saturating_sub(12)..];
        let wages: i128 = year_months
            .iter()
            .map(|m| i128::from(m.wage_bill.get()))
            .sum();
        let employed: i128 = year_months.iter().map(|m| i128::from(m.employed)).sum();
        if employed > 0 {
            self.dem.wage_nominal = Bani(i64::try_from(wages / employed).expect("a wage"));
        }
    }

    fn weight_of(&self, p: usize) -> u32 {
        self.households.hh_weight[self.persons.household_id[p] as usize]
    }

    /// Step 1: every live person a month older.
    fn people_ageing(&mut self) {
        for p in 0..self.pp.age.len() {
            if self.dem.life.alive[p] {
                self.pp.age[p] = self.pp.age[p].saturating_add(1);
            }
        }
    }

    /// Step 2: the month's contribution months and career points.
    fn people_contributions(&mut self) {
        let wn = i128::from(self.dem.wage_nominal.get().max(1));
        for p in 0..self.pp.age.len() {
            if !self.dem.life.alive[p] || self.dem.life.activity[p] == Activity::Retired {
                continue;
            }
            let declared = if self.persons.employer[p] != NO_EMPLOYER {
                self.persons.wage_declared[p]
            } else {
                self.slips
                    .unemployment_benefit
                    .get(p)
                    .copied()
                    .unwrap_or(Bani::ZERO)
            };
            if declared <= Bani::ZERO {
                continue;
            }
            self.people.contribution_months[p] =
                self.people.contribution_months[p].saturating_add(1);
            let pts = ratio(i128::from(declared.get()), 1_000_000, 12 * wn);
            self.people.career_points[p] += u64::try_from(pts.max(0)).expect("points");
        }
    }

    /// Step 3: deaths, aligned by sex and five-year band.
    fn people_deaths(&mut self) {
        let scale = i128::from(self.params.demography.mortality_scale_ppm);
        let Some(table) = self.dem.table.clone() else {
            return;
        };
        let mut strata: Vec<Vec<(usize, Candidate)>> = vec![Vec::new(); 2 * DEATH_BANDS];
        for p in 0..self.pp.age.len() {
            let w = self.weight_of(p);
            if !self.dem.life.alive[p] || w == 0 {
                continue;
            }
            let s = sex_index(self.people.sex[p]);
            let years = usize::from(self.pp.age[p] / 12);
            let m = table.monthly[s][years.min(MORTALITY_AGES - 1)];
            let m = u32::try_from((i128::from(m) * scale / 1_000_000).min(1_000_000_000))
                .expect("a probability");
            let c = Candidate {
                person_key: self.people.person_key[p],
                weight: w,
                p: m,
                rank_p: m,
            };
            strata[s * DEATH_BANDS + (years / 5).min(DEATH_BANDS - 1)].push((p, c));
        }
        let day = self.today;
        for (k, stratum) in strata.iter().enumerate() {
            if stratum.is_empty() {
                continue;
            }
            let (s, b) = (k / DEATH_BANDS, k % DEATH_BANDS);
            let cands: Vec<Candidate> = stratum.iter().map(|&(_, c)| c).collect();
            let sel = align(
                &cands,
                self.dem.carry_death[s][b],
                &self.seed,
                day,
                PURPOSE_DEATH,
            );
            self.dem.carry_death[s][b] = sel.carry_micro;
            let by_key: std::collections::BTreeMap<u64, usize> =
                stratum.iter().map(|&(p, c)| (c.person_key, p)).collect();
            for key in sel.chosen {
                self.die(by_key[&key]);
            }
        }
    }

    /// What the deceased would draw as a pension: points if retired, else
    /// career points over the full contribution years.
    fn pension_points_of(&self, p: usize) -> u64 {
        if self.dem.life.activity[p] == Activity::Retired {
            self.dem.life.pension_points[p]
        } else {
            self.people.career_points[p] / u64::from(FULL_YEARS)
        }
    }

    /// The partner of `p` in its household (the other of a head and partner
    /// pair), if alive.
    fn partner_of(&self, p: usize) -> Option<usize> {
        let want = match self.people.role[p] {
            Role::Head => Role::Partner,
            Role::Partner => Role::Head,
            _ => return None,
        };
        let h = self.persons.household_id[p];
        (0..self.persons.household_id.len()).find(|&q| {
            q != p
                && self.persons.household_id[q] == h
                && self.dem.life.alive[q]
                && self.people.role[q] == want
        })
    }

    /// A death: job, benefits and pension end; the survivor right; the record
    /// becomes a tombstone.
    fn die(&mut self, p: usize) {
        let partner = self.partner_of(p);
        let points = self.pension_points_of(p);
        if let Some(q) = partner {
            let sv = u64::try_from(u128::from(points) * u128::from(SURVIVOR_SHARE_PPM) / 1_000_000)
                .expect("points");
            let own = &mut self.dem.life.survivor_points[q];
            *own = (*own).max(sv);
            // A retired survivor draws max(own points, survivor points) from
            // the next pay day; its own points stay as they are.
            if self.dem.life.activity[q] == Activity::Retired {
                let drawn = self.dem.life.pension_points[q].max(*own);
                self.fs_set_points(q, drawn);
            }
        }
        let retired = self.dem.life.activity[p] == Activity::Retired;
        self.dem.report.deaths.push(Death {
            person_key: self.people.person_key[p],
            sex: self.people.sex[p],
            age: self.pp.age[p],
            hh_key: self.people.hh_key[self.persons.household_id[p] as usize],
            weight: self.weight_of(p),
            retired,
            pension_points: points,
            partner_key: partner.map(|q| self.people.person_key[q]),
        });
        self.end_work(p);
        self.pp.pension[p] = Bani::ZERO;
        self.pp.other_income[p] = Bani::ZERO;
        self.fs_set_points(p, 0);
        self.fs_end_claim(p);
        self.dem.life.alive[p] = false;
        // A new head if the head died: the partner, else the oldest adult.
        if self.people.role[p] == Role::Head {
            let h = self.persons.household_id[p];
            let next = partner.or_else(|| {
                (0..self.persons.household_id.len())
                    .filter(|&q| {
                        self.persons.household_id[q] == h
                            && self.dem.life.alive[q]
                            && self.pp.age[q] >= 18 * 12
                    })
                    .max_by_key(|&q| (self.pp.age[q], std::cmp::Reverse(self.people.person_key[q])))
            });
            if let Some(q) = next {
                self.people.role[q] = Role::Head;
            }
        }
        self.people_recount(self.persons.household_id[p] as usize);
    }

    /// A person's job or self-employment ends for good (death, retirement):
    /// no pay owed (the month's pay was paid on pay day), no claim, not
    /// seeking.
    fn end_work(&mut self, p: usize) {
        let u = self.persons.employer[p];
        if u != NO_EMPLOYER {
            let i = u32::try_from(p).expect("a person");
            let w = self.person_weight(i);
            let k = usize::from(u);
            self.units.staff[k] -= w;
            if let Ok(pos) = self.uu.members[k].binary_search(&i) {
                self.uu.members[k].remove(pos);
            }
        }
        self.persons.employer[p] = NO_EMPLOYER;
        self.persons.wage[p] = Bani::ZERO;
        self.persons.wage_declared[p] = Bani::ZERO;
        self.persons.months_unemployed[p] = 0;
        self.persons.hired_on[p] = None;
        self.pp.status[p] = 0;
        self.pp.seeking[p] = false;
    }

    /// A household's equivalised adults and its transfers' counts, from its
    /// live members.
    fn people_recount(&mut self, h: usize) {
        let members: Vec<usize> = (0..self.persons.household_id.len())
            .filter(|&q| self.persons.household_id[q] as usize == h && self.dem.life.alive[q])
            .collect();
        let mut eq = 0u32;
        let mut adults = 0u32;
        for &q in &members {
            if self.pp.age[q] >= 14 * 12 {
                eq += if adults == 0 { 1_000 } else { 500 };
                adults += 1;
            } else {
                eq += 300;
            }
        }
        self.hh.eq_adults_milli[h] = eq;
        let ages: Vec<(usize, u16)> = members.iter().map(|&q| (q, self.pp.age[q])).collect();
        self.fs_recount(h, &ages);
    }

    /// Step 5: standard retirement, then early.
    fn people_retirement(&mut self) {
        let Some(law) = self.pension_law() else {
            return;
        };
        let date = self.calendar.date(self.today);
        let ages = [
            self.retirement_age_in_force(Sex::F),
            self.retirement_age_in_force(Sex::M),
        ];
        let min_months = u16::from(MIN_YEARS) * 12;
        let mut early: Vec<Vec<(usize, Candidate)>> = vec![Vec::new(); 2 * EARLY_YEARS];
        for p in 0..self.pp.age.len() {
            let w = self.weight_of(p);
            if !self.dem.life.alive[p]
                || w == 0
                || self.dem.life.activity[p] == Activity::Retired
                || self.people.contribution_months[p] < min_months
            {
                continue;
            }
            let s = sex_index(self.people.sex[p]);
            let age = self.pp.age[p];
            if age >= ages[s] {
                self.retire(p, 0, &law);
            } else {
                let before = (ages[s] - age).div_ceil(12);
                if (1..=EARLY_YEARS as u16).contains(&before) {
                    let k = usize::from(before - 1);
                    let h = self.dem.early_ppb[s][k];
                    let c = Candidate {
                        person_key: self.people.person_key[p],
                        weight: w,
                        p: h,
                        rank_p: h,
                    };
                    early[s * EARLY_YEARS + k].push((p, c));
                }
            }
        }
        let _ = date;
        let day = self.today;
        for (k, stratum) in early.iter().enumerate() {
            if stratum.is_empty() {
                continue;
            }
            let (s, y) = (k / EARLY_YEARS, k % EARLY_YEARS);
            let cands: Vec<Candidate> = stratum.iter().map(|&(_, c)| c).collect();
            let sel = align(
                &cands,
                self.dem.carry_early[s][y],
                &self.seed,
                day,
                PURPOSE_EARLY,
            );
            self.dem.carry_early[s][y] = sel.carry_micro;
            let by_key: std::collections::BTreeMap<u64, usize> =
                stratum.iter().map(|&(p, c)| (c.person_key, p)).collect();
            for key in sel.chosen {
                let p = by_key[&key];
                let months_early = ages[s].saturating_sub(self.pp.age[p]);
                self.retire(p, months_early, &law);
            }
        }
    }

    /// The pension law a new pension reads, when the law has transfers.
    fn pension_law(&self) -> Option<PensionLaw> {
        let t = self.law.transfers.as_ref()?;
        Some(PensionLaw {
            full_years: FULL_YEARS,
            min_years: MIN_YEARS,
            early_reduction_ppm_per_month: EARLY_REDUCTION_PPM,
            survivor_share_ppm: SURVIVOR_SHARE_PPM,
            point_value: t.pension.point_value,
            minimum: t.pension.minimum,
        })
    }

    /// Retire `p`: activity retired, work ends, the new pension's points.
    fn retire(&mut self, p: usize, months_early: u16, law: &PensionLaw) {
        let employer_before = self.persons.employer[p];
        let Some(points) = new_pension_points(
            self.people.career_points[p],
            self.dem.life.survivor_points[p],
            months_early,
            self.people.contribution_months[p],
            law,
        ) else {
            return;
        };
        self.end_work(p);
        self.fs_end_claim(p);
        self.pp.other_income[p] = Bani::ZERO;
        self.dem.life.activity[p] = Activity::Retired;
        self.dem.life.pension_points[p] = points;
        self.fs_set_points(p, points);
        // A positive starting pension marks a pensioner; the amount is the
        // points' (social transfers).
        self.pp.pension[p] = law.minimum.max(Bani(1));
        self.dem.report.retirements.push(Retirement {
            person_key: self.people.person_key[p],
            sex: self.people.sex[p],
            age: self.pp.age[p],
            weight: self.weight_of(p),
            early: months_early > 0,
            employer_before,
            pension_points: points,
        });
    }
}
