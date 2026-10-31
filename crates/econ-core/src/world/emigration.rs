//! Emigration at the monthly close (spec `economy/migration-remittances`,
//! steps 1 and 2; feature `migration`): the pull of abroad by skill tier,
//! each eligible resident's monthly probability, whole-record alignment by
//! cell (region, age band, tier), and the departures: a person alone (a
//! linked emigrant) or the whole household, whose net deposits go to the
//! rest of the world under `transfer.migrant_capital`.
//!
//! The base rates `p0` by age band are fitted at load so that, with every
//! factor at its starting value, the first year's expected emigrants aged 15
//! to 64 equal 2021's by band, children and companions who leave with their
//! households included.

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_mech_migration::{Pick, emigration_ppm, pull_ppm, take_whole_records};
use econ_popgen::{Activity, EduLevel, Enrolled, NO_EMPLOYER, Role};
use econ_rng::Stream;
use econ_types::Bani;

use super::World;
use super::migration::{MIGRATION_AGE_BANDS, StockCell};
use super::money::{Postings, bani};

/// The keyed draws' purposes.
const PURPOSE_PRIORITY: u32 = 0;
const PURPOSE_WHOLE: u32 = 1;
/// The age bands a person may leave alone in: 15–24 to 55–64
/// ([`MIGRATION_AGE_BANDS`] 1 to 5), from 18.
const BANDS: usize = 5;
/// Skill tiers: low, mid, high.
const TIERS: usize = 3;

/// The band of [`MIGRATION_AGE_BANDS`] of an age in months.
pub(crate) fn age_band(age_months: u16) -> usize {
    let y = age_months / 12;
    MIGRATION_AGE_BANDS
        .iter()
        .position(|&(a, b)| y >= a && y < b)
        .unwrap_or(MIGRATION_AGE_BANDS.len() - 1)
}

/// The skill tier of an education: low 0, mid 1, high 2.
pub(crate) fn edu_tier(e: EduLevel) -> usize {
    match e {
        EduLevel::Isced0To2 => 0,
        EduLevel::Isced3 | EduLevel::Isced4 => 1,
        EduLevel::Isced5To6 | EduLevel::Isced7To8 => 2,
    }
}

/// What the month's emigration did, for the month's report.
#[derive(Debug, Default)]
pub(super) struct Departures {
    pub(super) emigrants: u64,
    pub(super) by_age: [u64; 7],
    pub(super) households: Vec<(u32, u32, Bani)>,
}

impl World {
    /// A person's skill tier: its job's (high ISCO 1–3, mid 4–8, low 9),
    /// else the tier its education qualifies for.
    fn mig_tier(&self, p: usize) -> usize {
        if self.persons.employer[p] != NO_EMPLOYER
            && let Some(t) = super::load::tier(self.persons.occupation[p])
        {
            return 2 - t;
        }
        edu_tier(self.persons.edu_level[p])
    }

    /// A pensioner: a pension or retired.
    fn mig_pensioner(&self, p: usize) -> bool {
        self.pp.pension[p] > Bani::ZERO || self.dem.life.activity[p] == Activity::Retired
    }

    /// Whether a resident may leave alone: 18 to 64, not a pensioner, in a
    /// live record of positive weight.
    fn mig_eligible(&self, p: usize) -> bool {
        let h = self.persons.household_id[p] as usize;
        let y = self.pp.age[p] / 12;
        self.person_resident(p)
            && (18..65).contains(&y)
            && !self.mig_pensioner(p)
            && self.household_alive(h)
            && self.households.hh_weight[h] > 0
    }

    /// Per household: whether it has a linked emigrant, its resident
    /// members, and whether one is a pensioner.
    fn mig_households(&self) -> (Vec<bool>, Vec<Vec<usize>>, Vec<bool>) {
        let n_hh = self.households.hh_weight.len();
        let (mut linked, mut members, mut pensioner) =
            (vec![false; n_hh], vec![Vec::new(); n_hh], vec![false; n_hh]);
        for p in 0..self.pp.age.len() {
            let h = self.persons.household_id[p] as usize;
            if self.mig_abroad(p) {
                linked[h] = true;
            } else if self.person_resident(p) {
                members[h].push(p);
                pensioner[h] |= self.mig_pensioner(p);
            }
        }
        (linked, members, pensioner)
    }

    /// Whether a chosen person leaves with the whole household for sure
    /// (`Some(true)`: alone in it, or its only adult with minors), never
    /// (`Some(false)`: a pensioner in it), or by the draw (`None`).
    fn mig_whole_rule(&self, p: usize, members: &[usize], pensioner: bool) -> Option<bool> {
        let adults = members
            .iter()
            .filter(|&&q| self.pp.age[q] >= 18 * 12)
            .count();
        if members.len() == 1 || (adults == 1 && members.len() > 1 && self.pp.age[p] >= 18 * 12) {
            Some(true)
        } else if pensioner {
            Some(false)
        } else {
            None
        }
    }

    /// Load: fit `p0` by band (2021's emigrants aged 15 to 64, companions
    /// included) on the starting population.
    pub(super) fn mig_fit(&mut self) {
        let q = self.params.migration;
        let r = q.rules;
        let (linked, members, pensioner) = self.mig_households();
        let share = f64::from(r.household_share_ppm) / 1e6;
        // m[a][b]: a year's expected emigrants of band a per unit of p0 of
        // band b (ages 15–64: bands 1 to 5).
        let mut m = [[0f64; BANDS]; BANDS];
        for p in 0..self.pp.age.len() {
            if !self.mig_eligible(p) {
                continue;
            }
            let h = self.persons.household_id[p] as usize;
            let w = f64::from(self.households.hh_weight[h]);
            let mut f = 1.0;
            if linked[h] {
                f *= 1.0 + f64::from(r.network_effect_ppm) / 1e6;
            }
            if self.pp.seeking[p] {
                f *= f64::from(r.unemployed_factor_ppm) / 1e6;
            }
            let b = age_band(self.pp.age[p]) - 1;
            m[b][b] += 12.0 * w * f;
            let whole = match self.mig_whole_rule(p, &members[h], pensioner[h]) {
                Some(true) => 1.0,
                Some(false) => 0.0,
                None => share,
            };
            for &j in &members[h] {
                let a = age_band(self.pp.age[j]);
                if j != p && (1..=BANDS).contains(&a) {
                    m[a - 1][b] += 12.0 * w * f * whole;
                }
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let t: [f64; BANDS] = q.emigrants_2021.map(|x| x as f64);
        let x = solve(m, t);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            self.mig.p0_ppm = x.map(|v| (v.max(0.0) * 1e6).round() as u32);
        }
    }

    /// The month's wages abroad, basis points of the start.
    pub(super) fn mig_omega_bp(&self) -> u32 {
        use crate::levers::{LeverId, LeverValue};
        self.lever_state()
            .and_then(|st| match st.law.get(LeverId::WorldWagesAbroad, None) {
                Some(LeverValue::Rate(v)) => u32::try_from(*v).ok(),
                _ => None,
            })
            .unwrap_or(10_000)
    }

    /// Step 1: the pull by tier, millionths; the start's at the first close.
    fn mig_pull(&mut self, omega_bp: u32) -> [u32; TIERS] {
        let (mut sum, mut wt) = ([0i128; TIERS], [0i128; TIERS]);
        for p in 0..self.pp.age.len() {
            let net = self.slips.net_wage.get(p).copied().unwrap_or(Bani::ZERO);
            if self.persons.employer[p] == NO_EMPLOYER
                || net <= Bani::ZERO
                || !self.person_resident(p)
            {
                continue;
            }
            let s = self.mig_tier(p);
            let w = i128::from(self.person_weight(u32::try_from(p).expect("a person")));
            sum[s] += w * i128::from(net.get());
            wt[s] += w;
        }
        let e = i128::from(self.foreign().exchange_rate_ppm.max(1));
        // The net wage in euro, ×1e12 for precision.
        let now: [i128; TIERS] = std::array::from_fn(|s| {
            if wt[s] == 0 {
                0
            } else {
                sum[s] * 1_000_000_000_000 / wt[s] / e
            }
        });
        let (sum_all, wt_all) = (sum.iter().sum::<i128>(), wt.iter().sum::<i128>());
        let all = if wt_all == 0 {
            0
        } else {
            sum_all * 1_000_000_000_000 / wt_all / e
        };
        let base = *self.mig.base_wage_eur.get_or_insert(now);
        let base_all = *self.mig.base_wage_all.get_or_insert(all);
        let index = |now: i128, base: i128| {
            if base > 0 && now > 0 {
                u32::try_from(now * 1_000_000 / base).unwrap_or(u32::MAX)
            } else {
                1_000_000
            }
        };
        self.mig.h_all_ppm = index(all, base_all);
        let pull = std::array::from_fn(|s| pull_ppm(omega_bp, index(now[s], base[s])));
        self.mig.pull = pull;
        pull
    }

    /// The unemployment rate of each region now, millionths (with the
    /// scripted shocks), and the start's (at the first close).
    pub(super) fn mig_unemployment(&mut self) -> (Vec<u32>, Vec<u32>) {
        let n_r = self
            .pp
            .region
            .iter()
            .copied()
            .max()
            .map_or(0, |r| usize::from(r) + 1);
        let (mut un, mut lf) = (vec![0u64; n_r], vec![0u64; n_r]);
        for p in 0..self.pp.age.len() {
            if !self.person_resident(p) {
                continue;
            }
            let w = self.person_weight(u32::try_from(p).expect("a person"));
            let r = usize::from(self.pp.region[p]);
            if self.pp.seeking[p] {
                un[r] += w;
                lf[r] += w;
            } else if self.persons.employer[p] != NO_EMPLOYER
                || (2..=4).contains(&self.pp.status[p])
            {
                lf[r] += w;
            }
        }
        let measured: Vec<u32> = (0..n_r)
            .map(|r| u32::try_from(un[r] * 1_000_000 / lf[r].max(1)).expect("a rate"))
            .collect();
        if self.mig.base_unemp.is_empty() {
            self.mig.base_unemp.clone_from(&measured);
        }
        let mut now = measured;
        for &(r, pts) in &self.mig.shocks {
            if let Some(u) = now.get_mut(usize::from(r)) {
                *u = (*u + pts).min(1_000_000);
            }
        }
        (now, self.mig.base_unemp.clone())
    }

    /// Step 2: the month's emigration.
    pub(super) fn mig_emigration(&mut self) -> Departures {
        let r = self.params.migration.rules;
        let omega = self.mig_omega_bp();
        let pull = self.mig_pull(omega);
        let (u, u0) = self.mig_unemployment();
        self.mig.unemp_now.clone_from(&u);
        let n_r = u.len();
        let (linked, members, pensioner) = self.mig_households();
        let n_cells = n_r * BANDS * TIERS;
        if self.mig.carry.len() != n_cells {
            self.mig.carry = vec![0; n_cells];
        }
        let day = self.today.0;
        let mut cells: Vec<Vec<Pick>> = vec![Vec::new(); n_cells];
        let mut expected = vec![0i128; n_cells];
        for p in 0..self.pp.age.len() {
            if !self.mig_eligible(p) {
                continue;
            }
            let h = self.persons.household_id[p] as usize;
            let b = age_band(self.pp.age[p]) - 1;
            let s = self.mig_tier(p);
            let reg = usize::from(self.pp.region[p]);
            let prob = emigration_ppm(
                self.mig.p0_ppm[b],
                pull[s],
                u[reg],
                u0[reg],
                linked[h],
                self.pp.seeking[p],
                &r,
            );
            // Step 5b: the other side of the choice, capped at p^max.
            #[cfg(feature = "home_quality")]
            let prob = {
                let q = self.mig.home_quality.get(reg).copied().unwrap_or(1_000_000);
                let e = -i64::from(self.params.migration.quality_emigration_elasticity_ppm);
                let f = econ_mech_migration::quality_factor_ppm(q, e);
                u32::try_from((u64::from(prob) * u64::from(f) + 500_000) / 1_000_000)
                    .unwrap_or(u32::MAX)
                    .min(r.max_emigration_ppm)
            };
            if prob == 0 {
                continue;
            }
            let w = self.households.hh_weight[h];
            let k = (reg * BANDS + b) * TIERS + s;
            expected[k] += i128::from(w) * i128::from(prob);
            let draw = self.seed.fast_u64(
                Stream::Migration,
                day,
                self.people.person_key[p],
                PURPOSE_PRIORITY,
            );
            let priority = u64::try_from(
                (u128::from(draw >> 20) * 1_000_000 / u128::from(prob)).min(u128::from(u64::MAX)),
            )
            .expect("a priority");
            cells[k].push(Pick {
                record: u32::try_from(p).expect("a person"),
                weight: w,
                priority,
            });
        }
        let mut out = Departures::default();
        let mut gone = vec![false; self.households.hh_weight.len()];
        for k in 0..n_cells {
            let target = i64::try_from((expected[k] + 500_000) / 1_000_000).expect("a target");
            let mut carry = self.mig.carry[k];
            let chosen = take_whole_records(target, &mut carry, &mut cells[k]);
            self.mig.carry[k] = carry;
            for i in chosen {
                let p = i as usize;
                let h = self.persons.household_id[p] as usize;
                // A record formed today (the life cycle) stays this month.
                if gone[h] || h >= self.mig.hh_before || !self.mig_eligible(p) {
                    continue;
                }
                let whole = self
                    .mig_whole_rule(p, &members[h], pensioner[h])
                    .unwrap_or_else(|| {
                        let d = self.seed.fast_u64(
                            Stream::Migration,
                            day,
                            self.people.person_key[p],
                            PURPOSE_WHOLE,
                        );
                        d % 1_000_000 < u64::from(r.household_share_ppm)
                    });
                if whole {
                    gone[h] = true;
                    self.mig_leave_whole(h, &mut out);
                } else {
                    self.mig_leave_alone(p, &mut out);
                }
            }
        }
        out
    }

    /// The person's job, claims and income end: inert.
    fn mig_make_inert(&mut self, p: usize) {
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
        self.pp.pension[p] = Bani::ZERO;
        self.pp.other_income[p] = Bani::ZERO;
        self.fs_end_claim(p);
        self.people.enrolled[p] = Enrolled::None;
    }

    /// A person leaves alone: a linked emigrant of its household.
    fn mig_leave_alone(&mut self, p: usize, out: &mut Departures) {
        let h = self.persons.household_id[p] as usize;
        let w = u64::from(self.households.hh_weight[h]);
        out.emigrants += w;
        out.by_age[age_band(self.pp.age[p])] += w;
        self.mig_make_inert(p);
        if self.people.role[p] == Role::Head {
            let partner = (0..self.pp.age.len()).find(|&q| {
                self.persons.household_id[q] as usize == h
                    && q != p
                    && self.person_resident(q)
                    && self.people.role[q] == Role::Partner
            });
            let next = partner.or_else(|| {
                (0..self.pp.age.len())
                    .filter(|&q| {
                        self.persons.household_id[q] as usize == h
                            && q != p
                            && self.person_resident(q)
                            && self.pp.age[q] >= 18 * 12
                    })
                    .max_by_key(|&q| (self.pp.age[q], std::cmp::Reverse(self.people.person_key[q])))
            });
            if let Some(q) = next {
                self.people.role[q] = Role::Head;
            }
        }
        if matches!(self.people.role[p], Role::Head | Role::Partner) {
            self.people.role[p] = Role::Other;
        }
        self.dem.life.activity[p] = Activity::InactiveOther;
        self.set_base_activity(p, Activity::InactiveOther);
        self.mig.abroad[p] = true;
        self.mig.months_abroad[p] = 0;
        self.people_recount(h);
    }

    /// The whole household leaves: its money settles (cash deposited, loans
    /// repaid, the rest written off, the remaining deposits to the rest of
    /// the world), its persons join the stock, and the record ends.
    fn mig_leave_whole(&mut self, h: usize, out: &mut Departures) {
        let wd = i128::from(self.households.hh_weight[h]);
        let cash = i128::from(self.households.hh_cash[h].get().max(0));
        let deposits = i128::from(self.households.hh_deposits[h].get().max(0));
        let money = cash + deposits;
        let debt = |v: &[Bani]| i128::from(v.get(h).copied().unwrap_or(Bani::ZERO).get().max(0));
        let debts = debt(&self.bank().hh_mortgage) + debt(&self.bank().hh_consumer_debt);
        let repaid = money.min(debts);
        let net = money - repaid;
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
        if net > 0 {
            post.deposits(
                Sector::Households,
                Sector::RestOfWorld,
                bani(wd * net),
                FlowCode::TRANSFER_MIGRANT_CAPITAL,
            );
        }
        self.bank_intraday_advance(&mut post);
        post.commit(&mut self.ledger);
        // The persons: residents emigrate to the stock; linked emigrants
        // join it (`mig_household_ends`).
        let w = u64::from(self.households.hh_weight[h]);
        let year = self.calendar.date(self.today).year;
        for p in 0..self.pp.age.len() {
            if self.persons.household_id[p] as usize != h || !self.person_resident(p) {
                continue;
            }
            out.emigrants += w;
            out.by_age[age_band(self.pp.age[p])] += w;
            let birth_year = u16::try_from(year - i32::from(self.pp.age[p] / 12)).unwrap_or(0);
            self.mig_stock_add(StockCell {
                region: self.pp.region[p],
                sex: self.people.sex[p] as u8,
                birth_year,
                tier: u8::try_from(edu_tier(self.persons.edu_level[p])).expect("a tier"),
                persons: w,
            });
            self.mig_make_inert(p);
            self.dem.life.alive[p] = false;
        }
        self.mig_household_ends(h);
        // The record ends, holding nothing.
        self.households.hh_cash[h] = Bani::ZERO;
        self.households.hh_deposits[h] = Bani::ZERO;
        self.households.hh_hardship[h] = false;
        self.bank_household_debts_settled(h);
        let life = &mut self.dem.hh_life;
        life.hh_mortgage[h] = Bani::ZERO;
        life.hh_consumer_debt[h] = Bani::ZERO;
        life.hh_dwelling_value[h] = Bani::ZERO;
        life.hh_alive[h] = false;
        life.hh_moved_to_date[h] -= bani(net);
        self.rev_estate_dwelling(h, None);
        #[cfg(feature = "investment")]
        self.invest_estate(h, None);
        self.fs_end_household(h);
        out.households.push((
            u32::try_from(h).expect("a household"),
            self.households.hh_weight[h],
            bani(net),
        ));
    }
}

/// Solve `m x = t` by Gaussian elimination with partial pivoting (fixed
/// order, so deterministic); a singular column gives 0.
fn solve(mut m: [[f64; BANDS]; BANDS], mut t: [f64; BANDS]) -> [f64; BANDS] {
    for c in 0..BANDS {
        let pivot = (c..BANDS)
            .max_by(|&a, &b| m[a][c].abs().total_cmp(&m[b][c].abs()))
            .unwrap_or(c);
        m.swap(c, pivot);
        t.swap(c, pivot);
        if m[c][c] == 0.0 {
            continue;
        }
        for r in 0..BANDS {
            if r != c {
                let f = m[r][c] / m[c][c];
                let pivot_row = m[c];
                for (x, &y) in m[r][c..].iter_mut().zip(&pivot_row[c..]) {
                    *x -= f * y;
                }
                t[r] -= f * t[c];
            }
        }
    }
    std::array::from_fn(|c| if m[c][c] == 0.0 { 0.0 } else { t[c] / m[c][c] })
}
