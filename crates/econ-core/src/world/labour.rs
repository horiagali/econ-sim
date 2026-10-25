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
/// Whether an underpaid quitter looks for work while employed (pay and job
/// choice, step 4).
#[cfg(feature = "levers")]
const DRAW_JOB_TO_JOB: u32 = 4;
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
    pub(super) fn person_weight(&self, i: u32) -> u64 {
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
    /// (`last_day` included), then a job seeker; a layoff opens an
    /// unemployment claim when the law has the benefit.
    fn separate(&mut self, i: u32, last_day: u8, layoff: bool) {
        let p = i as usize;
        let u = self.persons.employer[p];
        let first = self.month_first();
        let start = self.job_start(p, first);
        let dim = self.calendar.date(self.today).days_in_month();
        let worked = last_day.saturating_add(1).saturating_sub(start);
        let pay = part_month_pay(self.persons.wage[p], worked, dim);
        let declared = part_month_pay(self.persons.wage_declared[p], worked, dim);
        if pay > Bani::ZERO {
            self.mac.owed.push((i, u, pay, declared));
        }
        let last_wage = self.persons.wage[p];
        self.open_claim(p, last_wage, layoff);
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

    /// A person leaves its unit for another job on the same day (pay and job
    /// choice, step 4): paid for the days worked, no unemployment claim, not
    /// seeking.
    #[cfg(feature = "levers")]
    fn leave_for_job(&mut self, i: u32) {
        let p = i as usize;
        let u = self.persons.employer[p];
        let first = self.month_first();
        let start = self.job_start(p, first);
        let date = self.calendar.date(self.today);
        let dim = date.days_in_month();
        let worked = date.day.saturating_sub(start);
        let pay = part_month_pay(self.persons.wage[p], worked, dim);
        let declared = part_month_pay(self.persons.wage_declared[p], worked, dim);
        if pay > Bani::ZERO {
            self.mac.owed.push((i, u, pay, declared));
        }
        let k = usize::from(u);
        self.units.staff[k] -= self.person_weight(i);
        if let Ok(pos) = self.uu.members[k].binary_search(&i) {
            self.uu.members[k].remove(pos);
        }
        self.persons.employer[p] = NO_EMPLOYER;
    }

    /// Every unit of an industry lays off `share_ppm` of its employees (a
    /// starting edit): the first in keyed priority, rounded per unit.
    pub(super) fn lay_off_share(&mut self, industry: usize, share_ppm: u32) {
        let today = self.today;
        for u in 0..self.units.industry.len() {
            if usize::from(self.units.industry[u]) != industry {
                continue;
            }
            let target =
                u64::try_from(i128::from(self.units.staff[u]) * i128::from(share_ppm) / ONE)
                    .expect("persons");
            let mut members = self.uu.members[u].clone();
            members.sort_by_key(|&i| {
                (
                    self.seed
                        .fast_u64(Stream::Labour, today.0, u64::from(i), DRAW_LAYOFF),
                    i,
                )
            });
            for i in take(&members, |i| self.person_weight(i), target) {
                self.separate(i, 0, true);
            }
        }
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
        // Pay and job choice: yesterday's underpaid quitters who look while
        // employed go first, for better-paid vacancies only.
        #[cfg(feature = "levers")]
        let job_seekers: Vec<u32> = std::mem::take(&mut self.bud.job_seekers)
            .into_iter()
            .filter(|&i| self.persons.employer[i as usize] != NO_EMPLOYER)
            .collect();
        #[cfg(feature = "levers")]
        order.splice(0..0, job_seekers.iter().copied());
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
                    self.separate(i, last, true);
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
                // The lowest wage the unit offers in this tier.
                let offered = |w: &World, o: usize| {
                    let base = w.mac.reference_wage[group][o];
                    #[cfg(feature = "levers")]
                    let base = w.budget_hire_wage(u, group, o).unwrap_or(base);
                    base.max(minimum)
                };
                let lowest_offer = (0..10)
                    .filter(|&o| {
                        by_occ[o] > 0 && tier(u8::try_from(o).expect("an occupation")) == Some(t)
                    })
                    .map(|o| offered(self, o))
                    .min()
                    .unwrap_or(minimum);
                let reach = |w: &World, i: u32| {
                    let p = i as usize;
                    let employed = w.persons.employer[p] != NO_EMPLOYER;
                    qualifies(t, w.persons.edu_level[p])
                        && (regions == 0 || regions & (1u64 << (w.pp.region[p] % 64)) != 0)
                        && (!employed
                            || (usize::from(w.persons.employer[p]) != u
                                && w.persons.wage[p] < lowest_offer))
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
                // Pay and job choice, step 2: relative pay speeds or slows hiring.
                #[cfg(feature = "levers")]
                let q = if self.pay_choice_active() {
                    econ_mech_labour::pay_choice::fill_rate_with_pay_ppm(
                        q,
                        self.unit_rho_ppm(u, t, lowest_offer),
                        &econ_mech_labour::pay_choice::PayParams::spec_defaults(),
                    )
                } else {
                    q
                };
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
                    let wage = offered(self, o);
                    // A job-to-job move: the person leaves its unit today.
                    #[cfg(feature = "levers")]
                    if self.persons.employer[p] != NO_EMPLOYER {
                        self.leave_for_job(i);
                    }
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
        #[cfg(feature = "levers")]
        if self.pay_choice_active() {
            self.quits_by_pay();
            return;
        }
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
                self.separate(i, dim, false);
            }
        }
    }

    /// Pay and job choice, steps 3 and 4: each employee's quit probability
    /// from its relative pay; the quitters are taken in keyed order weighted
    /// by it (M2's order when every relative pay is 1), the month's total is
    /// the sum of the probabilities; a share of the underpaid quitters look
    /// for work while employed instead.
    #[cfg(feature = "levers")]
    fn quits_by_pay(&mut self) {
        use econ_mech_labour::pay_choice::{PayParams, quit_probability_ppm};
        let pay = PayParams::spec_defaults();
        let today = self.today;
        let dim = self.calendar.date(today).days_in_month();
        let rate = self.params.labour.quit_rate_ppm;
        let groups = self.mac.normal_tightness.len();
        let mut by_group: Vec<Vec<(u128, u32, u32)>> = vec![Vec::new(); groups];
        for (u, members) in self.uu.members.iter().enumerate() {
            let g = usize::from(self.industries.group[usize::from(self.units.industry[u])]);
            for &i in members {
                let rho = self.person_rho_ppm(i as usize);
                let prob = quit_probability_ppm(rate, rho, &pay).max(1);
                let draw = self
                    .seed
                    .fast_u64(Stream::Labour, today.0, u64::from(i), DRAW_QUIT);
                let key = u128::from(draw >> 24) * 1_000_000 / u128::from(prob);
                by_group[g].push((key, i, prob));
            }
        }
        for mut list in by_group {
            list.sort_unstable();
            let order: Vec<u32> = list.iter().map(|&(_, i, _)| i).collect();
            let expected: u128 = list
                .iter()
                .map(|&(_, i, prob)| u128::from(self.person_weight(i)) * u128::from(prob))
                .sum();
            let target = u64::try_from((expected + 500_000) / 1_000_000).expect("quits");
            for i in take(&order, |i| self.person_weight(i), target) {
                let p = i as usize;
                let draw =
                    self.seed
                        .fast_u64(Stream::Labour, today.0, u64::from(i), DRAW_JOB_TO_JOB);
                if self.person_rho_ppm(p) < 1_000_000
                    && draw % 1_000_000 < u64::from(pay.job_to_job_share_ppm)
                {
                    self.bud.job_seekers.push(i);
                } else {
                    self.separate(i, dim, false);
                }
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
        // Wages: employees of each unit, and the separations of the month,
        // each payment taxed under the law in force.
        let mut payments: Vec<(u32, u16, Bani, Bani)> = Vec::new();
        for p in 0..n_p {
            let e = self.persons.employer[p];
            if e == NO_EMPLOYER {
                continue;
            }
            let days = dim + 1 - self.job_start(p, first);
            let gross = part_month_pay(self.persons.wage[p], days, dim);
            let declared = part_month_pay(self.persons.wage_declared[p], days, dim);
            payments.push((u32::try_from(p).expect("id"), e, gross, declared));
        }
        payments.extend(std::mem::take(&mut self.mac.owed));
        let today = self.today;
        let n_hh = self.households.hh_weight.len();
        self.slips.start(today, n_p, n_hh);
        let mut unit_pays = vec![0i128; self.units.industry.len()];
        let mut withheld = super::fiscal::Withheld::default();
        let (mut gross_total, mut net_total) = (0i128, 0i128);
        for &(i, u, gross, declared) in &payments {
            let p = i as usize;
            let tax = self.wage_tax(p, u, gross, declared);
            let w = i128::from(self.person_weight(i));
            let s = &mut self.slips;
            s.gross_wage[p] += gross;
            s.declared_wage[p] += declared;
            s.cas_public[p] += tax.cas_public;
            s.cas_pillar2[p] += tax.cas_pillar2;
            s.cass[p] += tax.cass;
            s.income_tax[p] += tax.income_tax;
            s.cam[p] += tax.cam;
            s.net_wage[p] += tax.net;
            self.persons.last_pay[p] += gross;
            let h = self.persons.household_id[p] as usize;
            got[h] += tax.net;
            let cost = w * i128::from((gross + tax.cam).get());
            unit_pays[usize::from(u)] += cost;
            self.fs.add_labour(usize::from(u), cost);
            gross_total += w * i128::from(gross.get());
            net_total += w * i128::from(tax.net.get());
            withheld.cas_public += w * i128::from(tax.cas_public.get());
            withheld.cas_pillar2 += w * i128::from(tax.cas_pillar2.get());
            withheld.cass += w * i128::from(tax.cass.get());
            withheld.income_tax += w * i128::from(tax.income_tax.get());
            withheld.cam += w * i128::from(tax.cam.get());
        }
        for (u, &paid) in unit_pays.iter().enumerate() {
            self.units.deposits[u] -= bani(paid);
            let j = usize::from(self.units.industry[u]);
            self.ii.wages_month[j] += bani(paid);
        }
        // The ministry budgets reimburse the public units and pay the lines.
        #[cfg(feature = "levers")]
        self.budget_month_end(&unit_pays);
        post.deposits(
            Sector::Firms,
            Sector::Households,
            bani(net_total),
            FlowCode::WAGES,
        );
        Self::post_withheld(&mut post, &withheld);
        self.mac.month.wages += gross_total;
        self.mac.month.employer_contributions += withheld.cam;
        // Pensions: at their starting amounts, or by points under a law with
        // transfers; taxed above the tax-free part under a law with income tax.
        let (mut pensions, mut pension_tax) = (0i128, 0i128);
        for p in 0..n_p {
            let (gross, tax) = self.pension_of(p);
            self.slips.pension[p] = gross;
            self.slips.pension_tax[p] = tax;
            if gross > Bani::ZERO {
                let h = self.persons.household_id[p] as usize;
                got[h] += gross - tax;
                let w = i128::from(self.households.hh_weight[h]);
                pensions += w * i128::from(gross.get());
                pension_tax += w * i128::from(tax.get());
            }
        }
        post.gov_pays(
            Sector::Households,
            bani(pensions),
            FlowCode::BENEFIT_PENSION,
        );
        post.pays_gov(Sector::Households, bani(pension_tax), FlowCode::TAX_INCOME);
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
            self.slips.mixed_income[h] = owed[h];
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
            self.fs.add_mixed(u, p);
        }
        post.deposits(
            Sector::Firms,
            Sector::Households,
            bani(paid_out),
            FlowCode::MIXED_INCOME,
        );
        // Income tax on declared self-employment income.
        let mut self_tax = 0i128;
        for p in 0..n_p {
            let tax = self.self_employment_tax(p, scale);
            if tax > Bani::ZERO {
                let h = self.persons.household_id[p] as usize;
                let tax = tax.min(got[h]);
                got[h] -= tax;
                self.slips.self_employment_tax[h] += tax;
                self_tax += i128::from(self.households.hh_weight[h]) * i128::from(tax.get());
            }
        }
        post.pays_gov(Sector::Households, bani(self_tax), FlowCode::TAX_INCOME);
        // Unemployment benefit, child allowance, minimum income.
        self.pay_transfers(&mut got, &mut post);
        // The quarter's taxes of the firm units.
        let date = self.calendar.date(self.today);
        if date.month.is_multiple_of(3) {
            self.quarter_taxes(&mut post);
        }
        for (h, &g) in got.iter().enumerate() {
            self.slips.received[h] = g;
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
            // Public pay is a share of the start (`spend.public_pay`): no round.
            #[cfg(feature = "levers")]
            if self.budget_public(usize::from(e)) {
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
        // `transfer.pension.indexation` under `law.json`: by the year's
        // prices or the round's mean wage growth across industry groups.
        #[cfg(feature = "levers")]
        {
            let n = i64::try_from(growth.len().max(1)).unwrap_or(1);
            let mean = growth.iter().map(|&g| i64::from(g)).sum::<i64>() / n;
            self.pension_indexation(i32::try_from(mean).unwrap_or(0));
        }
    }
}
