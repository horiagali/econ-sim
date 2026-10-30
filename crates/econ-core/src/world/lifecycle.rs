//! The household life cycle on the `World` (feature `lifecycle`; spec
//! `society/households-lifecycle`): the record operations on the World's
//! columns (a new household, a copied person, move out, split, merge, make
//! collective) and the People phase's step 6 (leaving home, unions,
//! separations) and the orphans of step 3.
//!
//! Balances are per real household. Moves between households are not ledger
//! postings: they are recorded in `hh_moved_to_date` on both sides.

use econ_mech_demography::records::leave_home_amount;
use econ_mech_demography::{Candidate, align};
use econ_popgen::{NO_EMPLOYER, Role, Sex};
use econ_rng::Stream;
use econ_types::Bani;

use super::World;
use super::demography::{Leaver, Separation, Union};
use crate::schedule::{Staggered, stagger_day};

/// Keyed-draw purposes on `Stream::Demography` (the People phase's are 1
/// to 5).
const PURPOSE_LEAVE: u32 = 6;
const PURPOSE_UNION: u32 = 7;
const PURPOSE_UNION_TIE: u32 = 8;
const PURPOSE_SEPARATION: u32 = 9;
/// Stage E's tenure codes.
const OWNER: u8 = 0;
const TENANT: u8 = 1;

impl World {
    /// The live members of household `h`, in index order.
    fn members_of(&self, h: usize) -> Vec<usize> {
        (0..self.persons.household_id.len())
            .filter(|&p| self.persons.household_id[p] as usize == h && self.dem.life.alive[p])
            .collect()
    }

    /// Append a household record like `like` (county, region, locality;
    /// fifth) with `weight`, no members, no balances, a new `hh_key` and
    /// stagger day, no decision yet. Returns its index.
    fn push_household(&mut self, like: usize, weight: u32, collective: bool) -> usize {
        let n = self.households.hh_weight.len();
        let key = self.people.next_hh_key;
        self.people.next_hh_key += 1;
        let h = &mut self.households;
        h.hh_weight.push(weight);
        h.hh_cash.push(Bani::ZERO);
        h.hh_deposits.push(Bani::ZERO);
        h.hh_income_received.push(Bani::ZERO);
        h.hh_stagger_day
            .push(stagger_day(self.seed, Staggered::Household, key));
        h.hh_hardship.push(false);
        let fifth = h.hh_fifth[like];
        h.hh_fifth.push(fifth);
        h.last_decision.push(None);
        h.income_to_date.push(Bani::ZERO);
        h.spending_to_date.push(Bani::ZERO);
        let s = &mut self.hh;
        s.eq_adults_milli.push(0);
        s.subsistence.push(Bani::ZERO);
        s.income_this_month.push(Bani::ZERO);
        s.income_last_month.push(Bani::ZERO);
        self.people.hh_key.push(key);
        let l = &mut self.dem.hh_life;
        l.hh_alive.push(true);
        let county = l.hh_county[like];
        l.hh_county.push(county);
        l.hh_collective.push(collective);
        l.hh_tenure.push(TENANT);
        l.hh_dwelling_value.push(Bani::ZERO);
        l.hh_mortgage.push(Bani::ZERO);
        l.hh_consumer_debt.push(Bani::ZERO);
        l.hh_moved_to_date.push(Bani::ZERO);
        self.fs_push_household(key, collective);
        self.bank_push_household(Bani::ZERO, Bani::ZERO);
        #[cfg(feature = "revenue")]
        self.rev_push_household(0, 0);
        #[cfg(feature = "investment")]
        self.invest_push_household(0);
        self.dem.report.life.households_formed.push(key);
        n
    }

    /// Append a copy of person `p` in household `h` with a new key: every
    /// attribute, the job included (the unit's weighted staff does not
    /// change: the records' weights were split).
    fn copy_person(&mut self, p: usize, h: usize) -> usize {
        let c = self.pp.age.len();
        let key = self.people.next_person_key;
        self.people.next_person_key += 1;
        let q = &mut self.persons;
        q.household_id.push(u32::try_from(h).expect("a household"));
        q.edu_level.push(q.edu_level[p]);
        q.employer.push(q.employer[p]);
        q.occupation.push(q.occupation[p]);
        q.wage.push(q.wage[p]);
        q.wage_declared.push(q.wage_declared[p]);
        q.months_unemployed.push(q.months_unemployed[p]);
        q.hired_on.push(q.hired_on[p]);
        q.last_pay.push(q.last_pay[p]);
        let s = &mut self.pp;
        s.status.push(s.status[p]);
        s.seeking.push(s.seeking[p]);
        s.region.push(s.region[p]);
        s.pension.push(s.pension[p]);
        s.age.push(s.age[p]);
        s.other_income.push(s.other_income[p]);
        let pe = &mut self.people;
        pe.sex.push(pe.sex[p]);
        pe.role.push(pe.role[p]);
        pe.person_key.push(key);
        pe.contribution_months.push(pe.contribution_months[p]);
        pe.career_points.push(pe.career_points[p]);
        pe.enrolled.push(pe.enrolled[p]);
        let l = &mut self.dem.life;
        l.alive.push(true);
        l.activity.push(l.activity[p]);
        l.pension_points.push(l.pension_points[p]);
        l.survivor_points.push(l.survivor_points[p]);
        l.months_since_birth.push(l.months_since_birth[p]);
        l.origin_household.push(l.origin_household[p]);
        self.fs_copy_person(p);
        #[cfg(feature = "revenue")]
        self.rev_copy_person(p);
        let u = self.persons.employer[p];
        if u != NO_EMPLOYER {
            let i = u32::try_from(c).expect("a person");
            let m = &mut self.uu.members[usize::from(u)];
            if let Err(pos) = m.binary_search(&i) {
                m.insert(pos, i);
            }
        }
        c
    }

    /// Move `amount` of cash or deposits (per household) from `from` to `to`.
    fn move_money(&mut self, from: usize, to: usize, cash: Bani, deposits: Bani) {
        let h = &mut self.households;
        h.hh_cash[from] -= cash;
        h.hh_cash[to] += cash;
        h.hh_deposits[from] -= deposits;
        h.hh_deposits[to] += deposits;
        let l = &mut self.dem.hh_life;
        l.hh_moved_to_date[from] -= cash + deposits;
        l.hh_moved_to_date[to] += cash + deposits;
    }

    /// End a household record that no longer has members: it holds nothing.
    fn end_record(&mut self, h: usize) {
        self.households.hh_cash[h] = Bani::ZERO;
        self.households.hh_deposits[h] = Bani::ZERO;
        self.households.hh_hardship[h] = false;
        let l = &mut self.dem.hh_life;
        l.hh_alive[h] = false;
        l.hh_dwelling_value[h] = Bani::ZERO;
        l.hh_mortgage[h] = Bani::ZERO;
        l.hh_consumer_debt[h] = Bani::ZERO;
        self.bank_set_debts(h, Bani::ZERO, Bani::ZERO);
        #[cfg(feature = "revenue")]
        self.rev_set_dwelling(h, 0, 0);
        #[cfg(feature = "investment")]
        self.invest_set_key(h, 0);
        self.fs_end_household(h);
    }

    /// Move out: person `p` heads a new household of the same weight and
    /// county, a tenant with no dwelling and no debt, with `cash` and
    /// `deposits` (per household) from the old one. Returns the new index.
    fn move_out(&mut self, p: usize, cash: Bani, deposits: Bani) -> usize {
        let h = self.persons.household_id[p] as usize;
        let n = self.push_household(h, self.households.hh_weight[h], false);
        self.move_money(h, n, cash, deposits);
        self.persons.household_id[p] = u32::try_from(n).expect("a household");
        self.people.role[p] = Role::Head;
        self.dem.life.origin_household[p] = Some(self.people.hh_key[h]);
        self.people_recount(h);
        self.people_recount(n);
        n
    }

    /// Split: record `h` keeps `w − k` and a copy of weight `k` gets copies
    /// of every member and the same balances per household. Returns the copy.
    fn split_record(&mut self, h: usize, k: u32) -> usize {
        let w = self.households.hh_weight[h];
        debug_assert!(k > 0 && k < w);
        self.households.hh_weight[h] = w - k;
        let collective = self.dem.hh_life.hh_collective[h];
        let c = self.push_household(h, k, collective);
        // The balances, per household.
        let (cash, deposits) = (self.households.hh_cash[h], self.households.hh_deposits[h]);
        self.households.hh_cash[c] = cash;
        self.households.hh_deposits[c] = deposits;
        self.dem.hh_life.hh_moved_to_date[c] = cash + deposits;
        let l = &mut self.dem.hh_life;
        l.hh_tenure[c] = l.hh_tenure[h];
        l.hh_dwelling_value[c] = l.hh_dwelling_value[h];
        l.hh_mortgage[c] = l.hh_mortgage[h];
        l.hh_consumer_debt[c] = l.hh_consumer_debt[h];
        let (m, cd) = (
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
        self.bank_set_debts(c, m, cd);
        #[cfg(feature = "revenue")]
        {
            let (rk, dv) = self.rev_dwelling_of(h);
            self.rev_set_dwelling(c, rk, dv);
        }
        #[cfg(feature = "investment")]
        {
            let key = self.invest_key_of(h);
            self.invest_set_key(c, key);
        }
        let s = &mut self.hh;
        s.subsistence[c] = s.subsistence[h];
        s.income_last_month[c] = s.income_last_month[h];
        for p in self.members_of(h) {
            self.copy_person(p, c);
        }
        self.people_recount(c);
        self.dem.report.life.splits += 1;
        c
    }

    /// Merge two one-person private records of the same weight: `other`'s
    /// person moves into `keep` as partner, the balances add, `other` ends.
    fn merge_records(&mut self, keep: usize, other: usize) {
        for p in self.members_of(other) {
            self.persons.household_id[p] = u32::try_from(keep).expect("a household");
            self.people.role[p] = Role::Partner;
        }
        let (cash, deposits) = (
            self.households.hh_cash[other],
            self.households.hh_deposits[other],
        );
        self.move_money(other, keep, cash, deposits);
        let l = &mut self.dem.hh_life;
        let owner = l.hh_tenure[keep] == OWNER || l.hh_tenure[other] == OWNER;
        let dwelling = l.hh_dwelling_value[other];
        l.hh_dwelling_value[keep] += dwelling;
        if owner {
            l.hh_tenure[keep] = OWNER;
        }
        let (m, cd) = (
            self.bank()
                .hh_mortgage
                .get(keep)
                .copied()
                .unwrap_or(Bani::ZERO)
                + self
                    .bank()
                    .hh_mortgage
                    .get(other)
                    .copied()
                    .unwrap_or(Bani::ZERO),
            self.bank()
                .hh_consumer_debt
                .get(keep)
                .copied()
                .unwrap_or(Bani::ZERO)
                + self
                    .bank()
                    .hh_consumer_debt
                    .get(other)
                    .copied()
                    .unwrap_or(Bani::ZERO),
        );
        self.bank_set_debts(keep, m, cd);
        self.dem.hh_life.hh_mortgage[keep] = m;
        self.dem.hh_life.hh_consumer_debt[keep] = cd;
        #[cfg(feature = "revenue")]
        {
            let ((rk, dk), (ro, d_o)) = (self.rev_dwelling_of(keep), self.rev_dwelling_of(other));
            let dwelling = dk + d_o;
            let key = if rk > 0 || ro > 0 { dwelling.max(1) } else { 0 };
            self.rev_set_dwelling(keep, key, dwelling);
        }
        #[cfg(feature = "investment")]
        {
            let key = self.invest_key_of(keep) + self.invest_key_of(other);
            self.invest_set_key(keep, key);
        }
        self.end_record(other);
        self.people_recount(keep);
        self.people_recount(other);
    }

    /// Orphans (step 1): a private household with live members but none
    /// aged 18 or more becomes one collective record per member; cash and
    /// deposits split equally (floor, the remainder to the first by
    /// `person_key`), the dwelling and any debt to the first.
    pub(super) fn make_collective(&mut self, h: usize) {
        let mut members = self.members_of(h);
        if members.is_empty() {
            return;
        }
        members.sort_by_key(|&p| self.people.person_key[p]);
        let w = self.households.hh_weight[h];
        let n = i64::try_from(members.len()).expect("members");
        let (cash, deposits) = (self.households.hh_cash[h], self.households.hh_deposits[h]);
        let split = |x: Bani| {
            (
                Bani(x.get().div_euclid(n)),
                x - Bani(x.get().div_euclid(n) * (n - 1)),
            )
        };
        let ((c_each, c_first), (d_each, d_first)) = (split(cash), split(deposits));
        for (i, &p) in members.iter().enumerate() {
            let r = self.push_household(h, w, true);
            let (c, d) = if i == 0 {
                (c_first, d_first)
            } else {
                (c_each, d_each)
            };
            self.move_money(h, r, c, d);
            if i == 0 {
                let l = &mut self.dem.hh_life;
                l.hh_dwelling_value[r] = l.hh_dwelling_value[h];
                l.hh_mortgage[r] = l.hh_mortgage[h];
                l.hh_consumer_debt[r] = l.hh_consumer_debt[h];
                let (m, cd) = (
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
                self.bank_set_debts(r, m, cd);
                #[cfg(feature = "revenue")]
                {
                    let (rk, dv) = self.rev_dwelling_of(h);
                    self.rev_set_dwelling(r, rk, dv);
                }
                #[cfg(feature = "investment")]
                {
                    let key = self.invest_key_of(h);
                    self.invest_set_key(r, key);
                }
            }
            self.persons.household_id[p] = u32::try_from(r).expect("a household");
            self.people.role[p] = Role::Head;
            self.dem.report.life.orphans.push(self.people.person_key[p]);
            self.people_recount(r);
        }
        self.end_record(h);
        self.people_recount(h);
    }

    /// Whether a live private household has live members but no adult.
    pub(super) fn orphaned(&self, h: usize) -> bool {
        let m = self.members_of(h);
        !m.is_empty()
            && !self.dem.hh_life.hh_collective[h]
            && m.iter().all(|&p| self.pp.age[p] < 18 * 12)
    }

    /// Step 6: leaving home, unions, separations.
    pub(super) fn people_lifecycle(&mut self) {
        self.leaving_home();
        self.unions();
        self.separations();
    }

    /// Step 2: young adults of 18 to 34 leave their parents' household.
    fn leaving_home(&mut self) {
        let lc = self.params.demography.lifecycle;
        let n_hh = self.households.hh_weight.len();
        // The oldest head or partner of each household (the parent proxy).
        let mut elder = vec![0u16; n_hh];
        for p in 0..self.pp.age.len() {
            if self.dem.life.alive[p] && matches!(self.people.role[p], Role::Head | Role::Partner) {
                let h = self.persons.household_id[p] as usize;
                elder[h] = elder[h].max(self.pp.age[p]);
            }
        }
        let mut strata: Vec<Vec<(usize, Candidate)>> = vec![Vec::new(); 8];
        for p in 0..self.pp.age.len() {
            let h = self.persons.household_id[p] as usize;
            let a = self.pp.age[p];
            if !self.dem.life.alive[p]
                || !(18 * 12..35 * 12).contains(&a)
                || !matches!(self.people.role[p], Role::Child | Role::Other)
                || self.dem.hh_life.hh_collective[h]
                || elder[h] < a + 15 * 12
                || self.households.hh_weight[h] == 0
                // A mother of a child born in the run stays with it (there
                // are no parent links to move the child with her).
                || self.dem.life.months_since_birth[p].is_some_and(|m| m < 18 * 12)
            {
                continue;
            }
            let band = match a / 12 {
                18..=19 => 0,
                20..=24 => 1,
                25..=29 => 2,
                _ => 3,
            };
            let s = usize::from(self.people.sex[p] != Sex::F);
            let rate = u64::from(lc.leave_home_rate_ppm[s][band]);
            let prob = u32::try_from(rate * 1_000 / 12).expect("a probability");
            let rank = if self.persons.employer[p] != NO_EMPLOYER {
                u64::from(prob) * u64::from(lc.leave_home_job_ratio_ppm) / 1_000_000
            } else if self.pp.seeking[p] {
                u64::from(prob) / 2
            } else {
                u64::from(prob)
            };
            strata[s * 4 + band].push((
                p,
                Candidate {
                    person_key: self.people.person_key[p],
                    weight: self.households.hh_weight[h],
                    p: prob,
                    rank_p: u32::try_from(rank.clamp(1, 1_000_000_000)).expect("a probability"),
                },
            ));
        }
        let day = self.today;
        for (k, stratum) in strata.iter().enumerate() {
            if stratum.is_empty() {
                continue;
            }
            let cands: Vec<Candidate> = stratum.iter().map(|&(_, c)| c).collect();
            let sel = align(
                &cands,
                self.dem.carry_leave[k],
                &self.seed,
                day,
                PURPOSE_LEAVE,
            );
            self.dem.carry_leave[k] = sel.carry_micro;
            let by_key: std::collections::BTreeMap<u64, usize> =
                stratum.iter().map(|&(p, c)| (c.person_key, p)).collect();
            for key in sel.chosen {
                let p = by_key[&key];
                let h = self.persons.household_id[p] as usize;
                let adults = u32::try_from(
                    self.members_of(h)
                        .iter()
                        .filter(|&&q| self.pp.age[q] >= 18 * 12)
                        .count(),
                )
                .expect("adults");
                let share = lc.leave_home_share_ppm;
                let cash = leave_home_amount(self.households.hh_cash[h], adults, share);
                let deposits = leave_home_amount(self.households.hh_deposits[h], adults, share);
                let n = self.move_out(p, cash, deposits);
                self.dem.report.life.leavers.push(Leaver {
                    person_key: key,
                    sex: self.people.sex[p],
                    age: self.pp.age[p],
                    weight: self.households.hh_weight[n],
                    from_hh: self.people.hh_key[h],
                    new_hh: self.people.hh_key[n],
                });
            }
        }
    }

    /// Step 3: unions of women and men who live alone.
    #[allow(clippy::too_many_lines)]
    fn unions(&mut self) {
        let lc = self.params.demography.lifecycle;
        let n_hh = self.households.hh_weight.len();
        let mut women: Vec<Vec<(usize, Candidate)>> = vec![Vec::new(); 5];
        // Eligible men by county: (person, age).
        let mut men: std::collections::BTreeMap<u8, Vec<(usize, u16)>> =
            std::collections::BTreeMap::new();
        // Each household's live members, counted in one pass.
        let mut count = vec![0u32; n_hh];
        let mut only = vec![usize::MAX; n_hh];
        for p in 0..self.pp.age.len() {
            if self.dem.life.alive[p] {
                let h = self.persons.household_id[p] as usize;
                count[h] += 1;
                only[h] = p;
            }
        }
        for h in 0..n_hh {
            if count[h] != 1 || !self.household_alive(h) || self.dem.hh_life.hh_collective[h] {
                continue;
            }
            let p = only[h];
            let a = self.pp.age[p];
            let county = self.dem.hh_life.hh_county[h];
            match self.people.sex[p] {
                Sex::F if (18 * 12..50 * 12).contains(&a) => {
                    let band = match a / 12 {
                        18..=24 => 0,
                        25..=29 => 1,
                        30..=34 => 2,
                        35..=39 => 3,
                        _ => 4,
                    };
                    let prob = u32::try_from(u64::from(lc.union_rate_ppm[band]) * 1_000 / 12)
                        .expect("a probability");
                    women[band].push((
                        p,
                        Candidate {
                            person_key: self.people.person_key[p],
                            weight: self.households.hh_weight[h],
                            p: prob,
                            rank_p: prob.max(1),
                        },
                    ));
                }
                Sex::M if (18 * 12..60 * 12).contains(&a) => {
                    men.entry(county).or_default().push((p, a));
                }
                _ => {}
            }
        }
        let day = self.today;
        let gap = lc.union_age_gap_months;
        for (b, stratum) in women.iter().enumerate() {
            if stratum.is_empty() {
                continue;
            }
            let cands: Vec<Candidate> = stratum.iter().map(|&(_, c)| c).collect();
            let sel = align(
                &cands,
                self.dem.carry_union[b],
                &self.seed,
                day,
                PURPOSE_UNION,
            );
            let mut carry = sel.carry_micro;
            let by_key: std::collections::BTreeMap<u64, usize> =
                stratum.iter().map(|&(p, c)| (c.person_key, p)).collect();
            for key in sel.chosen {
                let f = by_key[&key];
                let hf = self.persons.household_id[f] as usize;
                let county = self.dem.hh_life.hh_county[hf];
                let af = self.pp.age[f];
                let want = i32::from(af) + i32::from(gap);
                let (lo, hi) = (i32::from(af) - 36, i32::from(af) + 120);
                let pick = men.get(&county).and_then(|list| {
                    list.iter()
                        .enumerate()
                        .filter(|&(_, &(_, a))| (lo..=hi).contains(&i32::from(a)))
                        .min_by_key(|&(_, &(m, a))| {
                            (
                                (i32::from(a) - want).abs(),
                                self.seed.fast_u64(
                                    Stream::Demography,
                                    day.0,
                                    self.people.person_key[m],
                                    PURPOSE_UNION_TIE,
                                ),
                            )
                        })
                        .map(|(i, _)| i)
                });
                let Some(i) = pick else {
                    // No man: her weight is not selected; the carry keeps it.
                    carry += i64::from(self.households.hh_weight[hf]) * 1_000_000;
                    continue;
                };
                let (m, _) = men.get_mut(&county).expect("a county").remove(i);
                self.unite(f, m);
            }
            self.dem.carry_union[b] = carry;
        }
    }

    /// A union of woman `f` and man `m`, each alone in a private record: the
    /// larger record is split to the smaller weight, then the two merge into
    /// the record of whoever owns a dwelling (the woman's if both or
    /// neither do).
    fn unite(&mut self, f: usize, m: usize) {
        let (mut hf, mut hm) = (
            self.persons.household_id[f] as usize,
            self.persons.household_id[m] as usize,
        );
        let (mut pf, mut pm) = (f, m);
        let (wf, wm) = (self.households.hh_weight[hf], self.households.hh_weight[hm]);
        if wf > wm {
            hf = self.split_record(hf, wm);
            pf = self.members_of(hf)[0];
        } else if wm > wf {
            hm = self.split_record(hm, wf);
            pm = self.members_of(hm)[0];
        }
        let l = &self.dem.hh_life;
        let (keep, other) = if l.hh_tenure[hm] == OWNER && l.hh_tenure[hf] != OWNER {
            (hm, hf)
        } else {
            (hf, hm)
        };
        self.merge_records(keep, other);
        // The woman heads the merged record unless she moved into his.
        let (head, partner) = if keep == hf { (pf, pm) } else { (pm, pf) };
        self.people.role[head] = Role::Head;
        self.people.role[partner] = Role::Partner;
        self.dem.report.life.unions.push(Union {
            woman_key: self.people.person_key[pf],
            man_key: self.people.person_key[pm],
            hh_key: self.people.hh_key[keep],
            weight: self.households.hh_weight[keep],
        });
    }

    /// Step 4: separations of couples with the woman under 60; the man moves
    /// out with `separation_share` of the cash and of the deposits.
    fn separations(&mut self) {
        let lc = self.params.demography.lifecycle;
        let n_hh = self.households.hh_weight.len();
        let mut head = vec![None; n_hh];
        let mut partner = vec![None; n_hh];
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
        let prob =
            u32::try_from(u64::from(lc.separation_rate_ppm) * 1_000 / 12).expect("a probability");
        let mut couples: Vec<(usize, usize, usize, Candidate)> = Vec::new();
        for h in 0..n_hh {
            let (Some(a), Some(b)) = (head[h], partner[h]) else {
                continue;
            };
            if !self.household_alive(h) || self.dem.hh_life.hh_collective[h] {
                continue;
            }
            let (woman, man) = match (self.people.sex[a], self.people.sex[b]) {
                (Sex::F, Sex::M) => (a, b),
                (Sex::M, Sex::F) => (b, a),
                _ => continue,
            };
            if self.pp.age[woman] >= 60 * 12 {
                continue;
            }
            couples.push((
                h,
                woman,
                man,
                Candidate {
                    person_key: self.people.person_key[woman],
                    weight: self.households.hh_weight[h],
                    p: prob,
                    rank_p: prob.max(1),
                },
            ));
        }
        if couples.is_empty() {
            return;
        }
        let cands: Vec<Candidate> = couples.iter().map(|c| c.3).collect();
        let sel = align(
            &cands,
            self.dem.carry_separation,
            &self.seed,
            self.today,
            PURPOSE_SEPARATION,
        );
        self.dem.carry_separation = sel.carry_micro;
        let by_key: std::collections::BTreeMap<u64, (usize, usize, usize)> = couples
            .iter()
            .map(|&(h, f, m, c)| (c.person_key, (h, f, m)))
            .collect();
        let share = lc.separation_share_ppm;
        for key in sel.chosen {
            let (h, f, m) = by_key[&key];
            let (cash, deposits) = (self.households.hh_cash[h], self.households.hh_deposits[h]);
            let part = |x: Bani| {
                if x <= Bani::ZERO {
                    Bani::ZERO
                } else {
                    Bani(
                        i64::try_from(i128::from(x.get()) * i128::from(share) / 1_000_000)
                            .expect("an amount"),
                    )
                }
            };
            let (mc, md) = (part(cash), part(deposits));
            let n = self.move_out(m, mc, md);
            self.dem.life.origin_household[m] = None;
            self.people.role[f] = Role::Head;
            self.dem.report.life.separations.push(Separation {
                hh_key: self.people.hh_key[h],
                new_hh: self.people.hh_key[n],
                man_key: self.people.person_key[m],
                woman_key: self.people.person_key[f],
                weight: self.households.hh_weight[h],
                cash_before: cash,
                deposits_before: deposits,
                moved_cash: mc,
                moved_deposits: md,
            });
        }
    }
}
