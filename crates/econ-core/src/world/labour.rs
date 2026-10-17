//! Phases 3 and 9 to 11 (spec `game/world-loop`; rules in
//! `economy/labor-market`, "First version"): staffing and matching on a
//! unit's day, quits and pay at month end, the wage round each January.
//!
//! Records stand for many persons, so a target in weighted persons is met
//! by taking records in order while a record brings the total closer to it
//! (a record is taken when at least half of its weight is owed).

use econ_ledger::{FlowCode, Sector};
use econ_mech_labour::{fill_rate_ppm, hires, layoffs, part_month_pay, wage_growth_ppm};
use econ_popgen::NO_EMPLOYER;
use econ_rng::Stream;
use econ_types::{Bani, Day};

use super::World;
use super::load::{EMPLOYEE, qualifies, tier};
use super::money::{Postings, bani, ratio};

/// Keyed draws of the labour market: `k` of the draw context.
const DRAW_LAYOFF: u32 = 1;
const DRAW_QUIT: u32 = 2;
const DRAW_HIRE: u32 = 3;
const ONE: i128 = 1_000_000;

/// Take records, in order, until their weight is as close as it gets to
/// `target`: a record is taken while at least half of it is still owed.
fn take(order: &[u32], weight: impl Fn(u32) -> u64, target: u64) -> Vec<u32> {
    let mut taken = Vec::new();
    let mut got = 0u64;
    for &i in order {
        let w = weight(i);
        if got.saturating_add(w / 2) >= target || w == 0 {
            break;
        }
        got += w;
        taken.push(i);
    }
    taken
}

impl World {
    fn person_weight(&self, i: u32) -> u64 {
        u64::from(self.households.hh_weight[self.persons.household_id[i as usize] as usize])
    }

    /// The day of the month a person's current job started this month (1 if
    /// it started before).
    fn job_start(&self, i: usize, month_first: Day) -> u8 {
        match self.persons.hired_on[i] {
            Some(d) if d >= month_first => self.calendar.date(d).day,
            _ => 1,
        }
    }

    /// The first day of the current month.
    fn month_first(&self) -> Day {
        let date = self.calendar.date(self.today);
        Day(self.today.0 + 1 - u32::from(date.day))
    }

    /// A person leaves their unit: paid at month end for the days worked
    /// (`last_day` included), then a job seeker.
    fn separate(&mut self, i: u32, last_day: u8) {
        let p = i as usize;
        let u = self.persons.employer[p];
        let first = self.month_first();
        let start = self.job_start(p, first);
        let dim = self.calendar.date(self.today).days_in_month();
        let worked = last_day.saturating_add(1).saturating_sub(start);
        let pay = part_month_pay(self.persons.wage[p], worked, dim);
        if pay > Bani::ZERO {
            self.mac.owed.push((i, u, pay));
        }
        let w = self.person_weight(i);
        let k = usize::from(u);
        self.units.staff[k] -= w;
        if let Ok(pos) = self.uu.members[k].binary_search(&i) {
            self.uu.members[k].remove(pos);
        }
        self.persons.employer[p] = NO_EMPLOYER;
        self.persons.wage[p] = Bani::ZERO;
        self.persons.wage_declared[p] = Bani::ZERO;
        self.persons.months_unemployed[p] = 0;
        self.persons.hired_on[p] = None;
        self.pp.status[p] = 0;
        self.pp.seeking[p] = true;
    }

    /// Phase 3: layoffs, vacancies and hires of each unit whose day it is.
    #[allow(clippy::too_many_lines)]
    pub(super) fn staffing(&mut self, cohort: u8) {
        let today = self.today;
        let date = self.calendar.date(today);
        let minimum = self.law.minimum_wage;
        // Today's seekers, longest unemployed first, then by keyed priority.
        let mut seekers: Vec<(std::cmp::Reverse<u16>, u64, u32)> = (0..self.persons.employer.len())
            .filter(|&i| self.pp.seeking[i])
            .map(|i| {
                let id = u32::try_from(i).expect("a person id");
                let prio = self
                    .seed
                    .fast_u64(Stream::Labour, today.0, i as u64, DRAW_HIRE);
                (
                    std::cmp::Reverse(self.persons.months_unemployed[i]),
                    prio,
                    id,
                )
            })
            .collect();
        seekers.sort_unstable();
        let mut order: Vec<u32> = seekers.into_iter().map(|s| s.2).collect();
        for u in 0..self.units.industry.len() {
            if self.units.unit_stagger_day[u] != cohort {
                continue;
            }
            self.units.last_staffing[u] = Some(today);
            let staff = self.units.staff[u];
            let desired = self.units.desired_employment[u];
            if staff > desired {
                let n = layoffs(staff, desired, &self.params.labour);
                let mut members = self.uu.members[u].clone();
                members.sort_by_key(|&i| {
                    (
                        self.seed
                            .fast_u64(Stream::Labour, today.0, u64::from(i), DRAW_LAYOFF),
                        i,
                    )
                });
                let out = take(&members, |i| self.person_weight(i), n);
                let last = date.day.saturating_sub(1);
                for i in out {
                    self.separate(i, last);
                }
                continue;
            }
            if staff == desired {
                continue;
            }
            // Vacancies, spread over occupations by the unit's staff (its
            // starting staff if it has none), the armed forces left out.
            let vacancies = desired - staff;
            let mut mix = [0u64; 10];
            for &i in &self.uu.members[u] {
                let o = usize::from(self.persons.occupation[i as usize]);
                if let Some(m) = mix.get_mut(o) {
                    *m += self.person_weight(i);
                }
            }
            mix[0] = 0;
            if mix.iter().all(|&v| v == 0) {
                mix = self.uu.start_mix[u];
                mix[0] = 0;
            }
            if mix.iter().all(|&v| v == 0) {
                mix[9] = 1;
            }
            let weights: Vec<i128> = mix.iter().map(|&v| i128::from(v)).collect();
            let by_occ = super::money::split(i128::from(vacancies), &weights);
            let mut by_tier = [0u64; 3];
            for (o, &v) in by_occ.iter().enumerate() {
                if let Some(t) = tier(u8::try_from(o).expect("an occupation")) {
                    by_tier[t] += u64::try_from(v).expect("vacancies");
                }
            }
            let regions = self.unit_regions(u);
            let group = usize::from(self.industries.group[usize::from(self.units.industry[u])]);
            for t in 0..3 {
                let v = by_tier[t];
                if v == 0 {
                    continue;
                }
                self.uu.vacancies_month[u][t] += v;
                let reach = |w: &World, i: u32| {
                    let p = i as usize;
                    qualifies(t, w.persons.edu_level[p])
                        && (regions == 0 || regions & (1u64 << (w.pp.region[p] % 64)) != 0)
                };
                // Seekers in order until those reached cover the vacancies (and
                // a record more): the cap of the hires binds only below that.
                let mut reachable = Vec::new();
                let mut reachable_weight = 0u64;
                for &i in &order {
                    if reachable_weight
                        >= v.saturating_add(u64::from(self.sample_scale).saturating_mul(4))
                    {
                        break;
                    }
                    if reach(self, i) {
                        reachable.push(i);
                        reachable_weight += self.person_weight(i);
                    }
                }
                let q = fill_rate_ppm(
                    self.mac.tier_seekers[t],
                    self.mac.tier_vacancies[t],
                    &self.params.labour,
                );
                let h = hires(v, reachable_weight, q, &mut self.uu.hire_carry[u][t]);
                let hired = take(&reachable, |i| self.person_weight(i), h);
                // Occupations of the tier, each hire to the one still owed the most.
                let mut owed: Vec<(usize, i128)> = by_occ
                    .iter()
                    .enumerate()
                    .filter(|&(o, _)| tier(u8::try_from(o).expect("an occupation")) == Some(t))
                    .map(|(o, &v)| (o, v))
                    .collect();
                for &i in &hired {
                    let p = i as usize;
                    let k = (0..owed.len())
                        .max_by_key(|&k| (owed[k].1, std::cmp::Reverse(k)))
                        .expect("an occupation");
                    let w = i128::from(self.person_weight(i));
                    owed[k].1 -= w;
                    let o = owed[k].0;
                    let occupation = u8::try_from(o).expect("an occupation");
                    let wage = self.mac.reference_wage[group][o].max(minimum);
                    self.persons.employer[p] = u16::try_from(u).expect("a unit id");
                    self.persons.occupation[p] = occupation;
                    self.persons.wage[p] = wage;
                    self.persons.wage_declared[p] = wage;
                    self.persons.months_unemployed[p] = 0;
                    self.persons.hired_on[p] = Some(today);
                    self.pp.status[p] = EMPLOYEE;
                    self.pp.seeking[p] = false;
                    self.units.staff[u] += self.person_weight(i);
                    let pos = self.uu.members[u].binary_search(&i).unwrap_or_else(|e| e);
                    self.uu.members[u].insert(pos, i);
                }
                order.retain(|&i| self.pp.seeking[i as usize]);
            }
        }
    }

    /// The regions where a unit has staff, as bits; 0 for none.
    fn unit_regions(&self, u: usize) -> u64 {
        self.uu.members[u].iter().fold(0u64, |acc, &i| {
            acc | (1u64 << (self.pp.region[i as usize] % 64))
        })
    }

    /// Phase 9: each employee quits with probability `quit_rate` a month,
    /// aligned per industry group (the first in keyed priority quit).
    pub(super) fn quits(&mut self) {
        let today = self.today;
        let dim = self.calendar.date(today).days_in_month();
        let groups = self.mac.normal_tightness.len();
        let mut by_group: Vec<Vec<(u64, u32)>> = vec![Vec::new(); groups];
        for (u, members) in self.uu.members.iter().enumerate() {
            let g = usize::from(self.industries.group[usize::from(self.units.industry[u])]);
            for &i in members {
                by_group[g].push((
                    self.seed
                        .fast_u64(Stream::Labour, today.0, u64::from(i), DRAW_QUIT),
                    i,
                ));
            }
        }
        for mut list in by_group {
            list.sort_unstable();
            let order: Vec<u32> = list.iter().map(|&(_, i)| i).collect();
            let total: u64 = order.iter().map(|&i| self.person_weight(i)).sum();
            let target = u64::try_from(ratio(
                i128::from(total),
                i128::from(self.params.labour.quit_rate_ppm),
                ONE,
            ))
            .expect("quits");
            for i in take(&order, |i| self.person_weight(i), target) {
                self.separate(i, dim);
            }
        }
    }

    /// The month's scale of self-employment, informal and home-grown income:
    /// last month's consumption over the first month's (1 before a month closes).
    pub(super) fn mixed_scale_ppm(&self) -> i128 {
        let (first, last) = (self.mac.first_consumption, self.mac.last_consumption);
        if first > Bani::ZERO {
            ratio(i128::from(last.get()), ONE, i128::from(first.get()))
        } else {
            ONE
        }
    }

    /// Phase 10: wages for the days worked, pensions, and the households'
    /// other income; each household's `hh_income_received` grows by what it got.
    #[allow(clippy::too_many_lines)]
    pub(super) fn pay(&mut self) {
        let mut post = Postings::default();
        let first = self.month_first();
        let dim = self.calendar.date(self.today).days_in_month();
        let n_p = self.persons.employer.len();
        let mut got = vec![Bani::ZERO; self.households.hh_weight.len()];
        self.persons
            .last_pay
            .iter_mut()
            .for_each(|v| *v = Bani::ZERO);
        // Wages: employees of each unit, and the separations of the month.
        let mut unit_pays = vec![0i128; self.units.industry.len()];
        for p in 0..n_p {
            let e = self.persons.employer[p];
            if e == NO_EMPLOYER {
                continue;
            }
            let start = self.job_start(p, first);
            let pay = part_month_pay(self.persons.wage[p], dim + 1 - start, dim);
            self.persons.last_pay[p] += pay;
            unit_pays[usize::from(e)] +=
                i128::from(self.person_weight(u32::try_from(p).expect("id")))
                    * i128::from(pay.get());
        }
        for (i, u, pay) in std::mem::take(&mut self.mac.owed) {
            self.persons.last_pay[i as usize] += pay;
            unit_pays[usize::from(u)] += i128::from(self.person_weight(i)) * i128::from(pay.get());
        }
        let mut wages = 0i128;
        for p in 0..n_p {
            let pay = self.persons.last_pay[p];
            if pay > Bani::ZERO {
                let h = self.persons.household_id[p] as usize;
                got[h] += pay;
            }
        }
        for (u, &paid) in unit_pays.iter().enumerate() {
            self.units.deposits[u] -= bani(paid);
            let j = usize::from(self.units.industry[u]);
            self.ii.wages_month[j] += bani(paid);
            wages += paid;
        }
        post.deposits(
            Sector::Firms,
            Sector::Households,
            bani(wages),
            FlowCode::WAGES,
        );
        self.mac.month.wages += wages;
        // Pensions, at their starting amounts.
        let mut pensions = 0i128;
        for p in 0..n_p {
            let pension = self.pp.pension[p];
            if pension > Bani::ZERO {
                let h = self.persons.household_id[p] as usize;
                got[h] += pension;
                pensions += i128::from(self.households.hh_weight[h]) * i128::from(pension.get());
            }
        }
        post.gov_pays(
            Sector::Households,
            bani(pensions),
            FlowCode::BENEFIT_PENSION,
        );
        // Self-employment and informal income: the starting amounts, scaled,
        // paid by the domestic cohort units out of what they can spare.
        let scale = self.mixed_scale_ppm();
        let mut owed = vec![Bani::ZERO; self.households.hh_weight.len()];
        let mut want = 0i128;
        for p in 0..n_p {
            let v = self.pp.other_income[p];
            if v > Bani::ZERO {
                let h = self.persons.household_id[p] as usize;
                let amount = bani(ratio(i128::from(v.get()), scale, ONE));
                owed[h] += amount;
                want += i128::from(self.households.hh_weight[h]) * i128::from(amount.get());
            }
        }
        let caps: Vec<i128> = (0..self.units.industry.len())
            .map(|u| i128::from(self.units.deposits[u].get()).max(0))
            .collect();
        let weights: Vec<i128> = self
            .uu
            .mixed_weight
            .iter()
            .map(|&w| i128::from(w))
            .collect();
        let parts = Self::capped_split(want, &weights, &caps);
        let placed: i128 = parts.iter().sum();
        // When not all of it can be paid, every household gets the same share.
        let mut paid_out = 0i128;
        for h in 0..owed.len() {
            if placed < want {
                owed[h] = bani(i128::from(owed[h].get()) * placed / want.max(1));
            }
            if owed[h] > Bani::ZERO {
                got[h] += owed[h];
                paid_out += i128::from(self.households.hh_weight[h]) * i128::from(owed[h].get());
            }
        }
        let paid_by = if paid_out == placed {
            parts
        } else {
            super::money::split(paid_out, &parts)
        };
        for (u, &p) in paid_by.iter().enumerate() {
            self.units.deposits[u] -= bani(p);
        }
        post.deposits(
            Sector::Firms,
            Sector::Households,
            bani(paid_out),
            FlowCode::MIXED_INCOME,
        );
        for (h, &g) in got.iter().enumerate() {
            if g > Bani::ZERO {
                self.households.hh_deposits[h] += g;
                self.households.hh_income_received[h] += g;
                self.households.income_to_date[h] += g;
                self.hh.income_this_month[h] += g;
            }
        }
        post.commit(&mut self.ledger);
    }

    /// Phase 11: each January, every wage and reference wage of an industry
    /// group moves by expected inflation plus the response to its tightness;
    /// then no wage is below the minimum wage.
    pub(super) fn wage_round(&mut self) {
        let minimum = self.law.minimum_wage;
        let groups = self.mac.normal_tightness.len();
        let growth: Vec<i32> = (0..groups)
            .map(|g| {
                let history = &self.mac.tightness[g];
                let recent = &history[history.len().saturating_sub(12)..];
                let mean = if recent.is_empty() {
                    self.mac.normal_tightness[g]
                } else {
                    let s: u64 = recent.iter().map(|&v| u64::from(v)).sum();
                    u32::try_from(s / recent.len() as u64).expect("a tightness")
                };
                wage_growth_ppm(
                    self.mac.expected_inflation_ppm,
                    mean,
                    self.mac.normal_tightness[g],
                    &self.params.labour,
                )
            })
            .collect();
        let raise = |b: Bani, g: i32| bani(ratio(i128::from(b.get()), ONE + i128::from(g), ONE));
        for p in 0..self.persons.employer.len() {
            let e = self.persons.employer[p];
            if e == NO_EMPLOYER {
                continue;
            }
            let g = growth[usize::from(
                self.industries.group[usize::from(self.units.industry[usize::from(e)])],
            )];
            let w = raise(self.persons.wage[p], g).max(minimum);
            let d = raise(self.persons.wage_declared[p], g).min(w);
            self.persons.wage[p] = w;
            self.persons.wage_declared[p] = d;
        }
        for (g, row) in self.mac.reference_wage.iter_mut().enumerate() {
            for w in row.iter_mut() {
                *w = raise(*w, growth[g]).max(minimum);
            }
        }
    }
}
