//! The People phase (feature `demography`; spec `economy/demographics`,
//! "Periods"): at month end, after Pay, ageing, contributions, deaths and
//! retirement, and the estates of households left empty. Births and the
//! household life cycle come in later
//! steps of the implementation (roadmap M5).
//!
//! A dead person stays as a tombstone: `alive` false, with no job, no job
//! search, no pension and no income, so every phase that reads jobs,
//! pensions or incomes skips it; the phases that read ages check
//! [`World::person_alive`].

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_mech_demography::{
    Candidate, ChildSupport, FertileWoman, MORTALITY_AGES, MortalityTable, PensionLaw, align,
    benefit_gap, job_normalisation, m_cyc, m_job, m_pol, new_pension_points, table_for_year,
};
use econ_popgen::{Activity, EduLevel, Enrolled, NO_EMPLOYER, Role, Sex};
use econ_rng::Stream;
use econ_types::{Bani, Month};

use super::World;
use super::demography::{
    Birth, Death, EstateRecord, HouseholdLifeColumns, PeopleLifeColumns, PeopleReport, Retirement,
};
use super::money::{Postings, bani, ratio};

/// Five-year age bands of the death strata (0–4 … 100 and over).
const DEATH_BANDS: usize = 21;
/// Years before the standard age of early retirement's strata.
const EARLY_YEARS: usize = 10;
/// Keyed-draw purposes on `Stream::Demography`.
const PURPOSE_DEATH: u32 = 1;
const PURPOSE_EARLY: u32 = 2;
const PURPOSE_HEIR: u32 = 3;
const PURPOSE_BIRTH: u32 = 4;
const PURPOSE_BIRTH_SEX: u32 = 5;
/// Five-year bands of the mother's age, 15–19 to 45–49.
const BIRTH_BANDS: usize = 7;
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
    /// Births: the carries, the normalisation k_s of M^job by band
    /// (millionths), the child support of the 2021 law and of each closed
    /// month, and real income per equivalised adult at each month (for
    /// M^cyc).
    carry_birth: [i64; BIRTH_BANDS],
    k_birth: [u32; BIRTH_BANDS],
    support_2021: Option<ChildSupport>,
    support_history: Vec<ChildSupport>,
    real_income: Vec<i128>,
    /// The life cycle's carries: leaving home by sex × band, unions by the
    /// woman's band, separations.
    pub(super) carry_leave: [i64; 8],
    pub(super) carry_union: [i64; 5],
    pub(super) carry_separation: i64,
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
            carry_birth: [0; BIRTH_BANDS],
            k_birth: [1_000_000; BIRTH_BANDS],
            support_2021: self.child_support(),
            support_history: Vec::new(),
            real_income: Vec::new(),
            carry_leave: [0; 8],
            carry_union: [0; 5],
            carry_separation: 0,
        };
        self.dem.k_birth = self.birth_normalisation();
    }

    /// The child support of the law in force, when the law has transfers.
    fn child_support(&self) -> Option<ChildSupport> {
        let t = self.law.transfers.as_ref()?;
        let credit = self
            .law
            .income
            .as_ref()
            .map_or(Bani::ZERO, |l| l.child_credit);
        Some(ChildSupport {
            under2: t.child.under2,
            to18: t.child.to18,
            tax_credit: credit,
        })
    }

    /// Each live person's partner (the other of a head and partner pair).
    fn partners(&self) -> Vec<Option<usize>> {
        let n_hh = self.households.hh_weight.len();
        let (mut head, mut partner) = (vec![None; n_hh], vec![None; n_hh]);
        for p in 0..self.pp.age.len() {
            if !self.dem.life.alive[p] {
                continue;
            }
            let h = self.persons.household_id[p] as usize;
            match self.people.role[p] {
                Role::Head => head[h] = Some(p),
                Role::Partner => partner[h] = Some(p),
                _ => {}
            }
        }
        (0..self.pp.age.len())
            .map(|p| {
                let h = self.persons.household_id[p] as usize;
                match self.people.role[p] {
                    Role::Head => partner[h],
                    Role::Partner => head[h],
                    _ => None,
                }
            })
            .collect()
    }

    /// The women who may give birth this month, by band: `(person, band)`.
    /// `(person, band, recent)`: `recent` if she gave birth in the last 12
    /// months (she counts in the band's target but cannot be chosen).
    fn fertile_women(&self) -> Vec<(usize, usize, bool)> {
        (0..self.pp.age.len())
            .filter_map(|p| {
                let h = self.persons.household_id[p] as usize;
                let years = self.pp.age[p] / 12;
                let recent = self.dem.life.months_since_birth[p].is_some_and(|m| m < 12);
                (self.dem.life.alive[p]
                    && self.people.sex[p] == Sex::F
                    && (15..=49).contains(&years)
                    && !self.dem.hh_life.hh_collective[h]
                    && self.households.hh_weight[h] > 0)
                    .then(|| (p, usize::from((years - 15) / 5), recent))
            })
            .collect()
    }

    /// k_s at load: the starting population gives the base-rate births.
    fn birth_normalisation(&self) -> [u32; BIRTH_BANDS] {
        let Some(dem) = self.people.demography.as_ref() else {
            return [1_000_000; BIRTH_BANDS];
        };
        let partners = self.partners();
        let r = &self.params.demography.fertility;
        let mut by_band: Vec<Vec<FertileWoman>> = vec![Vec::new(); BIRTH_BANDS];
        for (p, b, _) in self.fertile_women() {
            let unemployed = self.pp.seeking[p] || partners[p].is_some_and(|q| self.pp.seeking[q]);
            by_band[b].push(FertileWoman {
                weight: self.weight_of(p),
                fertility_ppb: dem.fertility.rate_ppm(self.pp.age[p] / 12) * 1_000,
                m_job_ppm: m_job(unemployed, r),
            });
        }
        let mut k = [1_000_000; BIRTH_BANDS];
        for (b, women) in by_band.iter().enumerate() {
            k[b] = job_normalisation(women);
        }
        k
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
        self.people_births();
        self.people_retirement();
        #[cfg(feature = "lifecycle")]
        {
            let before = self.households.hh_weight.len();
            self.people_lifecycle();
            // A record formed and ended in the same phase (the part of a
            // split merged away) never formed a household.
            let alive: std::collections::BTreeSet<u64> = (before..self.households.hh_weight.len())
                .filter(|&h| self.dem.hh_life.hh_alive[h])
                .map(|h| self.people.hh_key[h])
                .collect();
            let formed_before: std::collections::BTreeSet<u64> =
                (0..before).map(|h| self.people.hh_key[h]).collect();
            self.dem
                .report
                .life
                .households_formed
                .retain(|k| alive.contains(k) || formed_before.contains(k));
        }
    }

    /// The compaction (spec step 6, "Records"): tombstones and ended
    /// households are removed, the tables keep their order, and every stored
    /// index (households' members, firm units' members, the links' households)
    /// is remapped. Keyed draws use keys, never indices, so no result changes.
    pub(super) fn compact_people(&mut self) {
        use super::money::retain_by;
        if !self.dem.active {
            return;
        }
        let keep_p = self.dem.life.alive.clone();
        let keep_h = self.dem.hh_life.hh_alive.clone();
        if keep_p.iter().all(|&k| k) && keep_h.iter().all(|&k| k) {
            return;
        }
        debug_assert!(self.mac.owed.is_empty(), "no pay owed at a close");
        // Old index → new index.
        let remap = |keep: &[bool]| -> Vec<u32> {
            let mut next = 0u32;
            keep.iter()
                .map(|&k| {
                    let i = next;
                    if k {
                        next += 1;
                    }
                    if k { i } else { u32::MAX }
                })
                .collect()
        };
        let (map_p, map_h) = (remap(&keep_p), remap(&keep_h));
        // Persons.
        for h in &mut self.persons.household_id {
            *h = map_h[*h as usize];
        }
        let q = &mut self.persons;
        retain_by(&mut q.household_id, &keep_p);
        retain_by(&mut q.edu_level, &keep_p);
        retain_by(&mut q.employer, &keep_p);
        retain_by(&mut q.occupation, &keep_p);
        retain_by(&mut q.wage, &keep_p);
        retain_by(&mut q.wage_declared, &keep_p);
        retain_by(&mut q.months_unemployed, &keep_p);
        retain_by(&mut q.hired_on, &keep_p);
        retain_by(&mut q.last_pay, &keep_p);
        let s = &mut self.pp;
        retain_by(&mut s.status, &keep_p);
        retain_by(&mut s.seeking, &keep_p);
        retain_by(&mut s.region, &keep_p);
        retain_by(&mut s.pension, &keep_p);
        retain_by(&mut s.age, &keep_p);
        retain_by(&mut s.other_income, &keep_p);
        let pe = &mut self.people;
        retain_by(&mut pe.sex, &keep_p);
        retain_by(&mut pe.role, &keep_p);
        retain_by(&mut pe.person_key, &keep_p);
        retain_by(&mut pe.contribution_months, &keep_p);
        retain_by(&mut pe.career_points, &keep_p);
        retain_by(&mut pe.enrolled, &keep_p);
        retain_by(&mut pe.hh_key, &keep_h);
        let l = &mut self.dem.life;
        retain_by(&mut l.alive, &keep_p);
        retain_by(&mut l.activity, &keep_p);
        retain_by(&mut l.pension_points, &keep_p);
        retain_by(&mut l.survivor_points, &keep_p);
        retain_by(&mut l.months_since_birth, &keep_p);
        retain_by(&mut l.origin_household, &keep_p);
        // The firm units' members: live persons only, remapped (the order of
        // indices is kept, so the lists stay sorted).
        for members in &mut self.uu.members {
            members.retain(|&i| map_p[i as usize] != u32::MAX);
            for i in members.iter_mut() {
                *i = map_p[*i as usize];
            }
        }
        // Yesterday's job seekers of pay and job choice.
        #[cfg(feature = "levers")]
        {
            let js = &mut self.bud.job_seekers;
            js.retain(|&i| map_p[i as usize] != u32::MAX);
            for i in js.iter_mut() {
                *i = map_p[*i as usize];
            }
        }
        // Households.
        let h = &mut self.households;
        retain_by(&mut h.hh_weight, &keep_h);
        retain_by(&mut h.hh_cash, &keep_h);
        retain_by(&mut h.hh_deposits, &keep_h);
        retain_by(&mut h.hh_income_received, &keep_h);
        retain_by(&mut h.hh_stagger_day, &keep_h);
        retain_by(&mut h.hh_hardship, &keep_h);
        retain_by(&mut h.hh_fifth, &keep_h);
        retain_by(&mut h.last_decision, &keep_h);
        retain_by(&mut h.income_to_date, &keep_h);
        retain_by(&mut h.spending_to_date, &keep_h);
        let hs = &mut self.hh;
        retain_by(&mut hs.eq_adults_milli, &keep_h);
        retain_by(&mut hs.subsistence, &keep_h);
        retain_by(&mut hs.income_this_month, &keep_h);
        retain_by(&mut hs.income_last_month, &keep_h);
        let hl = &mut self.dem.hh_life;
        retain_by(&mut hl.hh_alive, &keep_h);
        retain_by(&mut hl.hh_county, &keep_h);
        retain_by(&mut hl.hh_collective, &keep_h);
        retain_by(&mut hl.hh_tenure, &keep_h);
        retain_by(&mut hl.hh_dwelling_value, &keep_h);
        retain_by(&mut hl.hh_mortgage, &keep_h);
        retain_by(&mut hl.hh_consumer_debt, &keep_h);
        retain_by(&mut hl.hh_moved_to_date, &keep_h);
        // The links: those of ended households go (their remittances had
        // stopped); the rest point at their household's new index.
        let le = &mut self.people.linked_emigrants;
        let keep_link: Vec<bool> = le
            .household_id
            .iter()
            .map(|&g| keep_h[g as usize])
            .collect();
        for g in &mut le.household_id {
            *g = map_h[*g as usize];
        }
        retain_by(&mut le.household_id, &keep_link);
        retain_by(&mut le.person_key, &keep_link);
        retain_by(&mut le.age, &keep_link);
        retain_by(&mut le.sex, &keep_link);
        retain_by(&mut le.role, &keep_link);
        retain_by(&mut le.edu_level, &keep_link);
        retain_by(&mut le.months_abroad, &keep_link);
        #[cfg(feature = "trade")]
        self.rem_compact(&keep_link);
        self.fs_compact(&keep_p, &keep_h);
        self.bank_compact(&keep_h);
        #[cfg(feature = "investment")]
        self.invest_compact(&keep_h);
        #[cfg(feature = "revenue")]
        self.rev_compact(&keep_p, &keep_h);
        #[cfg(feature = "poverty")]
        self.pov_compact(&keep_h);
        #[cfg(feature = "approval")]
        self.app_compact(&keep_p, &keep_h);
        #[cfg(any(
            feature = "migration",
            feature = "education",
            feature = "approval",
            feature = "poverty"
        ))]
        self.act_compact(&keep_p);
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
                if let Some(m) = &mut self.dem.life.months_since_birth[p] {
                    *m = m.saturating_add(1);
                }
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

    /// The household's money income per equivalised adult, last month.
    fn income_per_adult(&self, h: usize) -> i128 {
        i128::from(self.hh.income_last_month[h].get()) * 1_000
            / i128::from(self.hh.eq_adults_milli[h].max(1))
    }

    /// Step 4: births, aligned by the mother's five-year band.
    #[allow(clippy::too_many_lines)]
    fn people_births(&mut self) {
        let Some(dem) = self.people.demography.clone() else {
            return;
        };
        let r = self.params.demography.fertility;
        let fixed = self.params.demography.fixed_fertility_multipliers;
        let scale = i128::from(self.params.demography.fertility_scale_ppm);
        let t = self.months.len();
        // The law of fertility_lag months ago (the start's before then).
        let lag = usize::from(r.lag_months);
        let support_then = if t >= lag {
            self.dem.support_history.get(t - lag).copied()
        } else {
            None
        }
        .or(self.dem.support_2021);
        let delta_b = match (support_then, self.dem.support_2021) {
            (Some(then), Some(base)) => benefit_gap(&then, &base),
            _ => Bani::ZERO,
        };
        // M^cyc from real income growth measured `lag` months ago.
        let series = &self.dem.real_income;
        let growth = (series.len() > lag + 12)
            .then(|| {
                let (now, before) = (
                    series[series.len() - 1 - lag],
                    series[series.len() - 13 - lag],
                );
                (before > 0)
                    .then(|| i32::try_from((now - before) * 1_000_000 / before).unwrap_or(0))
            })
            .flatten();
        let m_cycle = if fixed { 1_000_000 } else { m_cyc(growth, &r) };
        // The national median income per adult (for M^pol's floor).
        let n_hh = self.households.hh_weight.len();
        let mut incomes: Vec<(i128, u32)> = (0..n_hh)
            .filter(|&h| self.household_alive(h) && self.households.hh_weight[h] > 0)
            .map(|h| (self.income_per_adult(h), self.households.hh_weight[h]))
            .collect();
        incomes.sort_unstable();
        let total: u64 = incomes.iter().map(|&(_, w)| u64::from(w)).sum();
        let mut acc = 0u64;
        let median = incomes
            .iter()
            .find(|&&(_, w)| {
                acc += u64::from(w);
                2 * acc >= total
            })
            .map_or(0, |&(y, _)| y);
        let median = Bani(i64::try_from(median).unwrap_or(0));
        let partners = self.partners();
        let couple_ratio = i128::from(self.params.demography.partner_fertility_ratio_ppm);
        let mut strata: Vec<Vec<(usize, Candidate)>> = vec![Vec::new(); BIRTH_BANDS];
        // The expected births of women who gave birth in the last 12 months:
        // in the band's target, so the exclusion moves who gives birth, not
        // how many (millionths of a person).
        let mut recent_micro = [0i128; BIRTH_BANDS];
        for (p, b, recent) in self.fertile_women() {
            let h = self.persons.household_id[p] as usize;
            let f = i128::from(dem.fertility.rate_ppm(self.pp.age[p] / 12)) * 1_000 / 12;
            let (mp, mj) = if fixed {
                (1_000_000, 1_000_000)
            } else {
                let y = Bani(i64::try_from(self.income_per_adult(h)).unwrap_or(0));
                let unemployed =
                    self.pp.seeking[p] || partners[p].is_some_and(|q| self.pp.seeking[q]);
                (m_pol(delta_b, y, median, &r), m_job(unemployed, &r))
            };
            let k = if fixed {
                1_000_000
            } else {
                self.dem.k_birth[b]
            };
            let mut prob = f * scale / 1_000_000;
            for m in [
                i128::from(k),
                i128::from(mp),
                i128::from(mj),
                i128::from(m_cycle),
            ] {
                prob = prob * m / 1_000_000;
            }
            let prob = u32::try_from(prob.clamp(0, 1_000_000_000)).expect("a probability");
            if recent {
                recent_micro[b] += i128::from(self.weight_of(p)) * i128::from(prob) / 1_000;
                continue;
            }
            let in_couple = partners[p].is_some_and(|q| self.people.sex[q] == Sex::M);
            let rank = if in_couple {
                u32::try_from((i128::from(prob) * couple_ratio / 1_000_000).min(1_000_000_000))
                    .expect("a probability")
            } else {
                prob
            };
            strata[b].push((
                p,
                Candidate {
                    person_key: self.people.person_key[p],
                    weight: self.weight_of(p),
                    p: prob,
                    rank_p: rank.max(u32::from(prob > 0)),
                },
            ));
        }
        let day = self.today;
        for (b, stratum) in strata.iter().enumerate() {
            let cands: Vec<Candidate> = stratum.iter().map(|&(_, c)| c).collect();
            let target: i128 = cands
                .iter()
                .map(|c| i128::from(c.weight) * i128::from(c.p) / 1_000)
                .sum::<i128>()
                + i128::from(self.dem.carry_birth[b])
                + recent_micro[b];
            self.dem.report.birth_targets_micro[b] = i64::try_from(target).unwrap_or(i64::MAX);
            let carry = self.dem.carry_birth[b] + i64::try_from(recent_micro[b]).expect("a carry");
            if stratum.is_empty() {
                self.dem.carry_birth[b] = carry;
                continue;
            }
            let sel = align(&cands, carry, &self.seed, day, PURPOSE_BIRTH);
            self.dem.carry_birth[b] = sel.carry_micro;
            let by_key: std::collections::BTreeMap<u64, usize> =
                stratum.iter().map(|&(p, c)| (c.person_key, p)).collect();
            for key in sel.chosen {
                let mother = by_key[&key];
                let in_couple = partners[mother].is_some_and(|q| self.people.sex[q] == Sex::M);
                self.give_birth(mother, in_couple);
            }
        }
        // This month's child support and real income per adult, for later
        // months' lags.
        if let Some(cs) = self.child_support() {
            self.dem.support_history.push(cs);
        }
        let (mut inc, mut eq) = (0i128, 0i128);
        for h in 0..n_hh {
            if !self.household_alive(h) {
                continue;
            }
            let w = i128::from(self.households.hh_weight[h]);
            inc += w * i128::from(self.hh.income_last_month[h].get());
            eq += w * i128::from(self.hh.eq_adults_milli[h]);
        }
        let cpi = i128::from(self.mac.cpi.last().copied().unwrap_or(1_000_000).max(1));
        self.dem.real_income.push(if eq > 0 {
            inc * 1_000 * 1_000_000 / eq / cpi
        } else {
            0
        });
    }

    /// A birth: a newborn in the mother's household.
    fn give_birth(&mut self, mother: usize, in_couple: bool) {
        let key = self.people.person_key[mother];
        let u = self
            .seed
            .fast_u64(Stream::Demography, self.today.0, key, PURPOSE_BIRTH_SEX)
            % 1_000_000;
        let boy = u < u64::from(self.params.demography.sex_ratio_at_birth_ppm);
        let sex = if boy { Sex::M } else { Sex::F };
        let h = self.persons.household_id[mother];
        let child = self.push_person(h, sex, mother);
        self.dem.life.months_since_birth[mother] = Some(0);
        self.dem.report.births.push(Birth {
            child_key: self.people.person_key[child],
            sex,
            mother_key: key,
            mother_age: self.pp.age[mother],
            mother_in_couple: in_couple,
            hh_key: self.people.hh_key[h as usize],
            weight: self.weight_of(mother),
        });
        self.people_recount(h as usize);
    }

    /// Append a person to every per-person column: a newborn in household
    /// `h` (age 0, role child, no job, no pension, a new key), in the
    /// region of `like`.
    fn push_person(&mut self, h: u32, sex: Sex, like: usize) -> usize {
        let p = self.pp.age.len();
        let key = self.people.next_person_key;
        self.people.next_person_key += 1;
        let q = &mut self.persons;
        q.household_id.push(h);
        q.edu_level.push(EduLevel::Isced0To2);
        q.employer.push(NO_EMPLOYER);
        q.occupation.push(econ_popgen::NOT_EMPLOYED);
        q.wage.push(Bani::ZERO);
        q.wage_declared.push(Bani::ZERO);
        q.months_unemployed.push(0);
        q.hired_on.push(None);
        q.last_pay.push(Bani::ZERO);
        let s = &mut self.pp;
        s.status.push(0);
        s.seeking.push(false);
        let region = s.region[like];
        s.region.push(region);
        s.pension.push(Bani::ZERO);
        s.age.push(0);
        s.other_income.push(Bani::ZERO);
        let pe = &mut self.people;
        pe.sex.push(sex);
        pe.role.push(Role::Child);
        pe.person_key.push(key);
        pe.contribution_months.push(0);
        pe.career_points.push(0);
        pe.enrolled.push(Enrolled::None);
        let l = &mut self.dem.life;
        l.alive.push(true);
        l.activity.push(Activity::Child);
        l.pension_points.push(0);
        l.survivor_points.push(0);
        l.months_since_birth.push(None);
        l.origin_household.push(None);
        self.fs_push_person();
        #[cfg(feature = "revenue")]
        self.rev_push_person(Activity::Child as u8);
        p
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
        let h = self.persons.household_id[p] as usize;
        self.people_recount(h);
        let empty = !(0..self.persons.household_id.len())
            .any(|q| self.persons.household_id[q] as usize == h && self.dem.life.alive[q]);
        if empty {
            self.settle_estate(h, self.pp.age[p]);
        } else {
            #[cfg(feature = "lifecycle")]
            if self.orphaned(h) {
                self.make_collective(h);
            }
        }
    }

    /// The heir of an ended household (step 3, "Estate settlement"): the
    /// household of the first live person who left this one; else a keyed
    /// draw among private households of the county with a member 18 to 45
    /// years younger than the deceased; else any private household of the
    /// county; else none.
    fn find_heir(&self, h: usize, deceased_age: u16) -> Option<usize> {
        let key = self.people.hh_key[h];
        let n_p = self.persons.household_id.len();
        let child = (0..n_p)
            .filter(|&q| {
                let g = self.persons.household_id[q] as usize;
                self.dem.life.alive[q]
                    && self.dem.life.origin_household[q] == Some(key)
                    && g != h
                    && self.household_alive(g)
            })
            .min_by_key(|&q| self.people.person_key[q]);
        if let Some(q) = child {
            return Some(self.persons.household_id[q] as usize);
        }
        let county = self.dem.hh_life.hh_county[h];
        let (lo, hi) = (
            deceased_age.saturating_sub(45 * 12),
            deceased_age.saturating_sub(18 * 12),
        );
        let n_hh = self.households.hh_weight.len();
        let mut younger = vec![false; n_hh];
        for q in 0..n_p {
            let a = self.pp.age[q];
            if self.dem.life.alive[q] && a >= lo && a <= hi {
                younger[self.persons.household_id[q] as usize] = true;
            }
        }
        let eligible = |g: usize| {
            g != h
                && self.household_alive(g)
                && !self.dem.hh_life.hh_collective[g]
                && self.households.hh_weight[g] > 0
                && self.dem.hh_life.hh_county[g] == county
        };
        let draw = |g: usize| {
            self.seed.fast_u64(
                Stream::Demography,
                self.today.0,
                self.people.hh_key[g],
                PURPOSE_HEIR,
            )
        };
        (0..n_hh)
            .filter(|&g| eligible(g) && younger[g])
            .min_by_key(|&g| (draw(g), g))
            .or_else(|| {
                (0..n_hh)
                    .filter(|&g| eligible(g))
                    .min_by_key(|&g| (draw(g), g))
            })
    }

    /// A household left with no live member: its money repays its debts
    /// (the rest written off), what remains and its dwelling pass to the
    /// heir (the remainders and, with no heir, everything to the
    /// government under `transfer.estate`), and the record ends.
    fn settle_estate(&mut self, h: usize, deceased_age: u16) {
        let wd = i128::from(self.households.hh_weight[h]);
        let cash = i128::from(self.households.hh_cash[h].get().max(0));
        let deposits = i128::from(self.households.hh_deposits[h].get().max(0));
        let money = cash + deposits;
        let (mortgage, consumer) = (
            self.bank()
                .hh_mortgage
                .get(h)
                .copied()
                .unwrap_or(Bani::ZERO),
            self.bank()
                .hh_consumer_debt
                .get(h)
                .copied()
                .unwrap_or(Bani::ZERO),
        );
        let debts = i128::from(mortgage.get().max(0)) + i128::from(consumer.get().max(0));
        let repaid = money.min(debts);
        let left = money - repaid;
        let dwelling = i128::from(self.dem.hh_life.hh_dwelling_value[h].get().max(0));
        let heir = self.find_heir(h, deceased_age);
        let wh = heir.map_or(0, |g| i128::from(self.households.hh_weight[g]));
        // Per household of the heir: ⌊w_d · a / w_h⌋; the remainder (money)
        // to the government, the dwelling's dropped.
        let (to_heir_per, to_heir_dw_per) = if wh > 0 {
            (wd * left / wh, wd * dwelling / wh)
        } else {
            (0, 0)
        };
        let to_heir = to_heir_per * wh;
        let to_government = wd * left - to_heir;
        let mut post = Postings::default();
        if cash > 0 {
            let x = bani(wd * cash);
            post.leg(
                Sector::Households,
                Sector::Banks,
                Instrument::Cash,
                x,
                FlowCode::CASH_DEPOSIT,
            );
            post.leg(
                Sector::Banks,
                Sector::Households,
                Instrument::Deposits,
                x,
                FlowCode::CASH_DEPOSIT,
            );
        }
        if repaid > 0 {
            let x = bani(wd * repaid);
            post.leg(
                Sector::Banks,
                Sector::Households,
                Instrument::Loans,
                x,
                FlowCode::LOAN,
            );
            post.leg(
                Sector::Households,
                Sector::Banks,
                Instrument::Deposits,
                x,
                FlowCode::LOAN,
            );
        }
        if debts > repaid {
            let x = bani(wd * (debts - repaid));
            post.leg(
                Sector::Banks,
                Sector::Households,
                Instrument::Loans,
                x,
                FlowCode::WRITE_OFF,
            );
        }
        if to_government > 0 {
            post.pays_gov(
                Sector::Households,
                bani(to_government),
                FlowCode::TRANSFER_ESTATE,
            );
        }
        self.bank_intraday_advance(&mut post);
        post.commit(&mut self.ledger);
        // The tables: the ended record holds nothing; the heir receives.
        self.households.hh_cash[h] = Bani::ZERO;
        self.households.hh_deposits[h] = Bani::ZERO;
        self.households.hh_hardship[h] = false;
        self.bank_household_debts_settled(h);
        let life = &mut self.dem.hh_life;
        life.hh_mortgage[h] = Bani::ZERO;
        life.hh_consumer_debt[h] = Bani::ZERO;
        life.hh_dwelling_value[h] = Bani::ZERO;
        life.hh_alive[h] = false;
        life.hh_moved_to_date[h] -= bani(left);
        let mut heir_dwelling = None;
        if let Some(g) = heir {
            let a = bani(to_heir_per);
            self.households.hh_deposits[g] += a;
            life.hh_moved_to_date[g] += a;
            life.hh_dwelling_value[g] += bani(to_heir_dw_per);
            heir_dwelling = Some((g, i128::from(life.hh_dwelling_value[g].get())));
        }
        self.rev_estate_dwelling(h, heir_dwelling);
        #[cfg(feature = "investment")]
        self.invest_estate(h, heir);
        self.fs_end_household(h);
        self.dem.report.estates.push(EstateRecord {
            hh_key: self.people.hh_key[h],
            weight: u32::try_from(wd).expect("a weight"),
            money: bani(wd * money),
            debts: bani(wd * debts),
            repaid: bani(wd * repaid),
            written_off: bani(wd * (debts - repaid)),
            to_government: bani(to_government),
            heir: heir.map(|g| self.people.hh_key[g]),
            heir_weight: u32::try_from(wh).expect("a weight"),
            to_heir: bani(to_heir),
            dwelling: bani(wd * dwelling),
            to_heir_dwelling: bani(to_heir_dw_per * wh),
        });
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
    pub(super) fn people_recount(&mut self, h: usize) {
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
