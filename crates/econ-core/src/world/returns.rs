//! Returns and foreign immigration at the monthly close (spec
//! `economy/migration-remittances`, steps 4 and 5; feature `migration`), and
//! the diaspora stock's deaths each January.
//!
//! - A linked emigrant returns with `q0[g] · link_return · R_s^(−ε_r)`
//!   (aligned by age group and tier): it rejoins its household, looking for
//!   work if 15 to 64, with `return_savings`.
//! - The stock returns with `q0[g] · R_s^(−ε_r)` per real person (aligned by
//!   origin region, age group and tier, adults only): each return is a new
//!   one-person record of weight `sample_scale`, in a county of the region by
//!   its residents, cloned from a resident donor of the county with the same
//!   sex, age band and tier, with `return_savings`.
//! - `q0` is fitted at load to 2021's returns of citizens. Nobody under 15 is
//!   abroad at the start (stage L's links and stock are adults), so the
//!   returns of children (born abroad, returning with their parents) are
//!   folded into the 15–64 group's target.
//! - Foreign immigrants: `(I_2021 / 12) · H^ε_i` a month, `H` the all-tier
//!   home wage index in euro, as one-person records of weight
//!   `sample_scale` aged as 2021's emigrants of 18 to 64, in counties by
//!   residents (the vacancies have no county in the `World`), cloned from a
//!   donor, looking for work, with no money.

use econ_ledger::{FlowCode, Sector};
use econ_mech_migration::{Pick, eur_to_lei, take_whole_records};
use econ_num::math;
use econ_popgen::{Activity, Enrolled, NO_EMPLOYER, Role, Sex};
use econ_rng::Stream;
use econ_types::Bani;

use super::World;
use super::emigration::{age_band, edu_tier};
use super::migration::MIGRATION_AGE_BANDS;
use super::money::{Postings, bani};

/// The keyed draws' purposes (stream `Migration`).
const PURPOSE_RETURN: u32 = 10;
const PURPOSE_STOCK_CELL: u32 = 11;
const PURPOSE_COUNTY: u32 = 12;
const PURPOSE_DONOR: u32 = 13;
const PURPOSE_IMMIGRANT: u32 = 14;
/// Age groups of returns: under 15, 15–64, 65 and over.
const GROUPS: usize = 3;
const TIERS: usize = 3;

/// The return group of an age in years.
fn group_of(years: i32) -> usize {
    if years < 15 {
        0
    } else if years < 65 {
        1
    } else {
        2
    }
}

/// A new resident to create: where, who, and what it brings.
struct Arrival {
    county: u8,
    sex: Sex,
    age_months: u16,
    tier: usize,
    savings: Bani,
}

impl World {
    /// The current calendar year.
    fn mig_year(&self) -> i32 {
        self.calendar.date(self.today).year
    }

    /// Load: fit `q0` (after the links are folded and the stock is known).
    pub(super) fn mig_fit_returns(&mut self) {
        let q = self.params.migration;
        let lr = f64::from(q.rules.link_return_ppm) / 1e6;
        let mut mass = [0f64; GROUPS];
        for p in 0..self.pp.age.len() {
            if self.mig_abroad(p) {
                let w = f64::from(self.households.hh_weight[self.persons.household_id[p] as usize]);
                mass[group_of(i32::from(self.pp.age[p] / 12))] += lr * w;
            }
        }
        let year = self.mig_year();
        for c in &self.mig.stock {
            let age = year - i32::from(c.birth_year);
            if age >= 18 {
                #[allow(clippy::cast_precision_loss)]
                {
                    mass[group_of(age)] += c.persons as f64;
                }
            }
        }
        let t = q.returns_2021;
        #[allow(clippy::cast_precision_loss)]
        let target = [0.0, (t[0] + t[1]) as f64, t[2] as f64];
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            self.mig.q0_ppm = std::array::from_fn(|g| {
                if mass[g] > 0.0 {
                    (target[g] / 12.0 / mass[g] * 1e6).round() as u32
                } else {
                    0
                }
            });
        }
    }

    /// `q0[g] · R_s^(−ε_r)` (times `link_return` for a link), millionths.
    fn mig_return_ppm(&self, g: usize, tier: usize, link: bool) -> u32 {
        let r = self.params.migration.rules;
        let pull = f64::from(self.mig.pull[tier].max(1)) / 1e6;
        let mut x = f64::from(self.mig.q0_ppm[g])
            * math::powf(pull, -f64::from(r.return_elasticity_ppm) / 1e6);
        if link {
            x *= f64::from(r.link_return_ppm) / 1e6;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let v = x.round().min(1e6) as u32;
        v
    }

    /// `return_savings` at today's rate, per returning adult.
    fn mig_savings(&self) -> Bani {
        let e = u64::from(self.foreign().exchange_rate_ppm);
        eur_to_lei(self.params.migration.return_savings_eur_cents, e)
    }

    /// Money from the rest of the world into household `h`'s deposits
    /// (per household), under `transfer.migrant_capital`.
    fn mig_bring(&mut self, h: usize, amount: Bani) {
        if amount <= Bani::ZERO {
            return;
        }
        let w = i128::from(self.households.hh_weight[h]);
        let mut post = Postings::default();
        post.deposits(
            Sector::RestOfWorld,
            Sector::Households,
            bani(w * i128::from(amount.get())),
            FlowCode::TRANSFER_MIGRANT_CAPITAL,
        );
        self.bank_intraday_advance(&mut post);
        post.commit(&mut self.ledger);
        self.households.hh_deposits[h] += amount;
        self.dem.hh_life.hh_moved_to_date[h] += amount;
    }

    /// Step 4 for the links: those chosen rejoin their households.
    pub(super) fn mig_link_returns(&mut self) -> u64 {
        let day = self.today.0;
        let n_cells = GROUPS * TIERS;
        if self.mig.link_carry.len() != n_cells {
            self.mig.link_carry = vec![0; n_cells];
        }
        let mut cells: Vec<Vec<Pick>> = vec![Vec::new(); n_cells];
        let mut expected = vec![0i128; n_cells];
        for p in 0..self.pp.age.len() {
            let h = self.persons.household_id[p] as usize;
            if !self.mig_abroad(p) || !self.household_alive(h) || self.households.hh_weight[h] == 0
            {
                continue;
            }
            let g = group_of(i32::from(self.pp.age[p] / 12));
            let s = edu_tier(self.persons.edu_level[p]);
            let q = self.mig_return_ppm(g, s, true);
            if q == 0 {
                continue;
            }
            let w = self.households.hh_weight[h];
            let k = g * TIERS + s;
            expected[k] += i128::from(w) * i128::from(q);
            let draw = self.seed.fast_u64(
                Stream::Migration,
                day,
                self.people.person_key[p],
                PURPOSE_RETURN,
            );
            let priority = u64::try_from(
                (u128::from(draw >> 20) * 1_000_000 / u128::from(q)).min(u128::from(u64::MAX)),
            )
            .expect("a priority");
            cells[k].push(Pick {
                record: u32::try_from(p).expect("a person"),
                weight: w,
                priority,
            });
        }
        let savings = self.mig_savings();
        let mut returns = 0u64;
        for k in 0..n_cells {
            let target = i64::try_from((expected[k] + 500_000) / 1_000_000).expect("a target");
            let mut carry = self.mig.link_carry[k];
            let chosen = take_whole_records(target, &mut carry, &mut cells[k]);
            self.mig.link_carry[k] = carry;
            for i in chosen {
                let p = i as usize;
                let h = self.persons.household_id[p] as usize;
                returns += u64::from(self.households.hh_weight[h]);
                self.mig.abroad[p] = false;
                self.mig.months_abroad[p] = 0;
                let years = self.pp.age[p] / 12;
                if (15..65).contains(&years) {
                    self.pp.seeking[p] = true;
                    self.persons.months_unemployed[p] = 0;
                }
                self.dem.life.activity[p] = Activity::InactiveOther;
                self.set_base_activity(p, Activity::InactiveOther);
                self.mig_bring(h, savings);
                self.people_recount(h);
            }
        }
        returns
    }

    /// Each county's region (from its residents) and residents (weighted).
    fn mig_counties(&self) -> (Vec<u8>, Vec<u64>) {
        let n_c = usize::from(
            self.dem
                .hh_life
                .hh_county
                .iter()
                .copied()
                .max()
                .unwrap_or(0),
        ) + 1;
        let (mut region, mut residents) = (vec![u8::MAX; n_c], vec![0u64; n_c]);
        for p in 0..self.pp.age.len() {
            if !self.person_resident(p) {
                continue;
            }
            let h = self.persons.household_id[p] as usize;
            let c = usize::from(self.dem.hh_life.hh_county[h]);
            region[c] = self.pp.region[p];
            residents[c] += u64::from(self.households.hh_weight[h]);
        }
        (region, residents)
    }

    /// A county drawn by residents among those `ok`, keyed by `key`.
    fn mig_draw_county(
        &self,
        residents: &[u64],
        ok: impl Fn(usize) -> bool,
        key: u64,
    ) -> Option<u8> {
        let total: u64 = (0..residents.len())
            .filter(|&c| ok(c))
            .map(|c| residents[c])
            .sum();
        if total == 0 {
            return None;
        }
        let mut x = self
            .seed
            .fast_u64(Stream::Migration, self.today.0, key, PURPOSE_COUNTY)
            % total;
        for (c, &r) in residents.iter().enumerate() {
            if !ok(c) {
                continue;
            }
            if x < r {
                return u8::try_from(c).ok();
            }
            x -= r;
        }
        None
    }

    /// The resident donor for an arrival: in its county, of its sex, age
    /// band and tier, the lowest keyed draw; failing that, dropping the tier,
    /// then the sex, then the age band.
    fn mig_donor(&self, a: &Arrival, key: u64) -> Option<usize> {
        let band = age_band(a.age_months);
        let draw = |p: usize| {
            self.seed.fast_u64(
                Stream::Migration,
                self.today.0,
                key ^ self.people.person_key[p].rotate_left(32),
                PURPOSE_DONOR,
            )
        };
        let base = |p: usize| {
            let h = self.persons.household_id[p] as usize;
            self.person_resident(p)
                && self.household_alive(h)
                && self.households.hh_weight[h] > 0
                && !self.dem.hh_life.hh_collective[h]
                && self.dem.hh_life.hh_county[h] == a.county
        };
        let tests: [&dyn Fn(usize) -> bool; 4] = [
            &|p| {
                self.people.sex[p] == a.sex
                    && age_band(self.pp.age[p]) == band
                    && edu_tier(self.persons.edu_level[p]) == a.tier
            },
            &|p| self.people.sex[p] == a.sex && age_band(self.pp.age[p]) == band,
            &|p| age_band(self.pp.age[p]) == band,
            &|_| true,
        ];
        for t in tests {
            if let Some(p) = (0..self.pp.age.len())
                .filter(|&p| base(p) && t(p))
                .min_by_key(|&p| (draw(p), p))
            {
                return Some(p);
            }
        }
        None
    }

    /// A new one-person record of weight `sample_scale` for an arrival,
    /// cloned from its donor: a resident head with no job, no pension, no
    /// claims, looking for work if 15 to 64. `false` if there is no donor.
    fn mig_arrive(&mut self, a: &Arrival, key: u64) -> bool {
        let Some(donor) = self.mig_donor(a, key) else {
            return false;
        };
        let dh = self.persons.household_id[donor] as usize;
        let h = self.push_household(dh, self.sample_scale, false);
        self.dem.report.life.households_formed.pop();
        let p = self.copy_person(donor, h);
        // The job is not cloned (copy_person put it in the unit's members).
        let u = self.persons.employer[p];
        if u != NO_EMPLOYER {
            let i = u32::try_from(p).expect("a person");
            let m = &mut self.uu.members[usize::from(u)];
            if let Ok(pos) = m.binary_search(&i) {
                m.remove(pos);
            }
        }
        self.persons.employer[p] = NO_EMPLOYER;
        self.persons.occupation[p] = econ_popgen::NOT_EMPLOYED;
        self.persons.wage[p] = Bani::ZERO;
        self.persons.wage_declared[p] = Bani::ZERO;
        self.persons.hired_on[p] = None;
        self.persons.months_unemployed[p] = 0;
        self.pp.status[p] = 0;
        self.pp.pension[p] = Bani::ZERO;
        self.pp.other_income[p] = Bani::ZERO;
        self.pp.age[p] = a.age_months;
        self.pp.seeking[p] = (15..65).contains(&(a.age_months / 12));
        self.people.sex[p] = a.sex;
        self.people.role[p] = Role::Head;
        self.people.enrolled[p] = Enrolled::None;
        self.people.contribution_months[p] = 0;
        self.people.career_points[p] = 0;
        let l = &mut self.dem.life;
        l.activity[p] = Activity::InactiveOther;
        l.pension_points[p] = 0;
        l.survivor_points[p] = 0;
        l.months_since_birth[p] = None;
        l.origin_household[p] = None;
        self.mig.abroad[p] = false;
        self.mig.months_abroad[p] = 0;
        self.fs_set_points(p, 0);
        self.fs_end_claim(p);
        self.set_base_activity(p, Activity::InactiveOther);
        self.people_recount(h);
        self.mig_bring(h, a.savings);
        true
    }

    /// Step 4 for the stock: the month's returns, as new records.
    pub(super) fn mig_stock_returns(&mut self) -> u64 {
        let scale = i64::from(self.sample_scale.max(1));
        let (region_of, residents) = self.mig_counties();
        let n_r = usize::from(self.pp.region.iter().copied().max().unwrap_or(0)) + 1;
        let n_cells = n_r * GROUPS * TIERS;
        if self.mig.stock_carry.len() != n_cells {
            self.mig.stock_carry = vec![0; n_cells];
        }
        let year = self.mig_year();
        let cell_of = |c: &super::migration::StockCell| -> Option<usize> {
            let age = year - i32::from(c.birth_year);
            (age >= 18 && usize::from(c.region) < n_r).then(|| {
                (usize::from(c.region) * GROUPS + group_of(age)) * TIERS
                    + usize::from(c.tier.min(2))
            })
        };
        let mut expected = vec![0i128; n_cells];
        for c in &self.mig.stock {
            if let Some(k) = cell_of(c) {
                let g = (k / TIERS) % GROUPS;
                let q = self.mig_return_ppm(g, k % TIERS, false);
                expected[k] += i128::from(c.persons) * i128::from(q);
            }
        }
        let savings = self.mig_savings();
        let mut returns = 0u64;
        for (k, &e) in expected.iter().enumerate() {
            let goal = i64::try_from(e / 1_000_000).expect("a target") + self.mig.stock_carry[k];
            let n = (goal + scale / 2).div_euclid(scale).max(0);
            self.mig.stock_carry[k] = goal - n * scale;
            let (reg, tier) = (k / (GROUPS * TIERS), k % TIERS);
            for j in 0..n {
                // A stock cell of this cell with a record's persons, by
                // persons.
                let key =
                    (u64::try_from(k).expect("a cell") << 32) | u64::try_from(j).expect("a record");
                let fits: Vec<usize> = (0..self.mig.stock.len())
                    .filter(|&i| {
                        cell_of(&self.mig.stock[i]) == Some(k)
                            && self.mig.stock[i].persons >= u64::try_from(scale).expect("a scale")
                    })
                    .collect();
                let total: u64 = fits.iter().map(|&i| self.mig.stock[i].persons).sum();
                if total == 0 {
                    self.mig.stock_carry[k] = 0;
                    break;
                }
                let mut x =
                    self.seed
                        .fast_u64(Stream::Migration, self.today.0, key, PURPOSE_STOCK_CELL)
                        % total;
                let mut pick = fits[0];
                for &i in &fits {
                    if x < self.mig.stock[i].persons {
                        pick = i;
                        break;
                    }
                    x -= self.mig.stock[i].persons;
                }
                let c = self.mig.stock[pick];
                let Some(county) =
                    self.mig_draw_county(&residents, |cc| usize::from(region_of[cc]) == reg, key)
                else {
                    break;
                };
                let age = u16::try_from(year - i32::from(c.birth_year)).unwrap_or(0);
                let arrival = Arrival {
                    county,
                    sex: if c.sex == Sex::M as u8 {
                        Sex::M
                    } else {
                        Sex::F
                    },
                    age_months: age * 12 + u16::try_from(j % 12).expect("a month"),
                    tier,
                    savings: if age >= 18 { savings } else { Bani::ZERO },
                };
                if self.mig_arrive(&arrival, key) {
                    self.mig.stock[pick].persons -= u64::try_from(scale).expect("a scale");
                    returns += u64::try_from(scale).expect("a scale");
                }
            }
        }
        returns
    }

    /// Step 5: the month's foreign immigrants, as new records.
    pub(super) fn mig_immigration(&mut self) -> u64 {
        let q = self.params.migration;
        let scale = i64::from(self.sample_scale.max(1));
        let h = f64::from(self.mig.h_all_ppm.max(1)) / 1e6;
        #[allow(clippy::cast_precision_loss)]
        let month = q.immigrants_2021 as f64 / 12.0
            * math::powf(h, f64::from(q.immigration_elasticity_ppm) / 1e6);
        #[allow(clippy::cast_possible_truncation)]
        let goal = month.round() as i64 + self.mig.imm_carry;
        let n = (goal + scale / 2).div_euclid(scale).max(0);
        self.mig.imm_carry = goal - n * scale;
        let (_, residents) = self.mig_counties();
        let bands: u64 = q.emigrants_2021.iter().sum();
        let mut arrived = 0u64;
        for j in 0..n {
            let key = (1u64 << 63) | u64::try_from(j).expect("a record");
            let d = |k: u32| {
                self.seed.fast_u64(
                    Stream::Migration,
                    self.today.0,
                    key,
                    PURPOSE_IMMIGRANT + 100 * k,
                )
            };
            // Age band by 2021's emigrants of 15–64, then a year in it (18+).
            let mut x = d(0) % bands.max(1);
            let mut b = 0;
            for (i, &v) in q.emigrants_2021.iter().enumerate() {
                if x < v {
                    b = i;
                    break;
                }
                x -= v;
            }
            let (from, _) = MIGRATION_AGE_BANDS[b + 1];
            let from = from.max(18);
            let span = if b == 0 { 7 } else { 10 };
            let years = from + u16::try_from(d(1) % span).expect("a year");
            let sex = if d(2) % 1_000_000 < u64::from(q.immigrant_men_ppm) {
                Sex::M
            } else {
                Sex::F
            };
            let Some(county) = self.mig_draw_county(&residents, |_| true, key) else {
                break;
            };
            let arrival = Arrival {
                county,
                sex,
                age_months: years * 12 + u16::try_from(d(3) % 12).expect("a month"),
                tier: usize::try_from(d(4) % 3).expect("a tier"),
                savings: Bani::ZERO,
            };
            if self.mig_arrive(&arrival, key) {
                arrived += u64::try_from(scale).expect("a scale");
            }
        }
        arrived
    }

    /// January: the stock's deaths of the year past, by the year's
    /// mortality table (weighted persons).
    pub(super) fn mig_stock_deaths(&mut self) -> u64 {
        let Some(table) = self.dem.table.clone() else {
            return 0;
        };
        let year = self.mig_year();
        let mut deaths = 0u64;
        for c in &mut self.mig.stock {
            let age = usize::try_from((year - i32::from(c.birth_year)).max(0)).unwrap_or(0);
            let s = usize::from(c.sex == Sex::M as u8);
            let m = u64::from(table.yearly[s][age.min(table.yearly[s].len() - 1)]);
            let d = (u128::from(c.persons) * u128::from(m) + 500_000_000) / 1_000_000_000;
            let d = u64::try_from(d).expect("deaths").min(c.persons);
            c.persons -= d;
            deaths += d;
        }
        deaths
    }
}
