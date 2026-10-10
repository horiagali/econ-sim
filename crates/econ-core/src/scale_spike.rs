//! Spike 4: performance and market clearing at variable population scale.
//!
//! **Throwaway-quality spike code** (see docs/03-architecture/spikes/). It
//! builds a fake weighted population of Romania at any `sample_scale`, then
//! runs a monthly tick with the expensive parts of the real design:
//!
//! 1. job separations (keyed RNG per person),
//! 2. labour matching per (region × skill) with alignment-style selection,
//! 3. wages via per-industry wage clearing accounts (integer bani, weighted),
//! 4. household consumption over 83 goods, with VAT carved out of what
//!    households spend and posted to government through the ledger
//!    (`tax.vat`, spec `economy/vat`),
//! 5. a 90×90 Leontief solve for gross output,
//! 6. an aggregation cube (county × activity × age band × education).
//!
//! Nothing here hardcodes the scale: record counts and weights come from
//! `sample_scale` and the (fake) totals.
//!
//! [`ScaleWorld::step_day`] runs the same parts with a daily clock, each at
//! its own period (ADR-0017; results in docs/03-architecture/spikes/spike-10-daily-clock.md).
//!
//! [`ScaleWorld::from_population`] builds the same world on a real starting
//! population (spec `society/population-generator`): households, weights,
//! counties and ages come from the census; education, jobs, wages and
//! deposits are still invented.

use econ_ledger::{FlowCode, Instrument, Ledger, Sector, Txn};
use econ_mech_tax::vat::{VatCategory, VatRate, VatSchedule};
use econ_num::lu::leontief_output;
use econ_rng::{KeyedRng, Stream};
use econ_types::{Bani, Date, split_largest_remainder};

/// Fake country totals (stand-ins for scenario data).
pub const REAL_POPULATION: u64 = 19_000_000;
/// Number of industries in the catalogue.
pub const N_INDUSTRIES: usize = 90;
/// Number of consumer goods.
pub const N_GOODS: usize = 83;
/// Counties.
pub const N_COUNTIES: usize = 42;
/// Labour-market regions.
pub const N_REGIONS: usize = 8;
/// Skill tiers.
pub const N_SKILLS: usize = 3;
/// Fake VAT schedule (stand-in for scenario data): 21% standard, 11% reduced.
pub const VAT_SCHEDULE: VatSchedule = VatSchedule {
    standard: VatRate(2100),
    reduced: VatRate(1100),
};
/// VAT groups of the consumption basket: standard, reduced, untaxed (zero-rated or exempt).
const N_VAT_GROUPS: usize = 3;
const N_STATUS: usize = 4; // 0 child/student, 1 employed, 2 unemployed, 3 retired
const N_AGE_BANDS: usize = 8;
const N_EDU: usize = 5;

/// Column tables of the fake world.
#[derive(Debug, Clone)]
pub struct ScaleWorld {
    /// Real people per synthetic person.
    pub sample_scale: u32,
    // persons
    age: Vec<u8>,
    county: Vec<u8>,
    edu: Vec<u8>,
    status: Vec<u8>,
    industry: Vec<u16>,
    wage: Vec<Bani>,
    household: Vec<u32>,
    // households
    hh_weight: Vec<u32>,
    hh_deposits: Vec<Bani>,
    hh_income: Vec<Bani>,
    /// VAT collected so far (the government's cash in the spike).
    gov_vat: Bani,
    /// Sector-level ledger: aggregated postings per flow code (ADR-0007).
    ledger: Ledger,
    // industries / firms (aggregated per industry for the spike)
    io: Vec<f64>,
    consumption_shares: Vec<f64>,
    seed_value: u64,
    seed: KeyedRng,
    tick: u32,
    /// Monthly job-separation probability (a player-adjustable parameter in the spike).
    separation_rate: f64,
    params: ScaleParams,
}

/// Behaviour parameters of the spike world (the calibration targets of Spike 5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleParams {
    /// Matching efficiency μ in hires = μ·√(U·V).
    pub matching_efficiency: f64,
    /// Vacancies per job seeker (stand-in for firm labour demand).
    pub vacancy_ratio: f64,
    /// Marginal propensity to consume out of income.
    pub mpc_income: f64,
    /// Monthly propensity to consume out of deposits.
    pub mpc_wealth: f64,
}

impl Default for ScaleParams {
    fn default() -> Self {
        ScaleParams {
            matching_efficiency: 0.25,
            vacancy_ratio: 0.8,
            mpc_income: 0.85,
            mpc_wealth: 0.005,
        }
    }
}

/// A starting population as plain columns: what the population generator
/// (crate `econ-popgen`) produces. `econ-core` does not depend on that crate.
#[derive(Debug, Clone, Copy)]
pub struct SeedPopulation<'a> {
    /// Real households represented by each synthetic household.
    pub hh_weight: &'a [u32],
    /// County index of each household.
    pub hh_county: &'a [u8],
    /// Household of each person.
    pub household_id: &'a [u32],
    /// Age of each person in months.
    pub age_months: &'a [u16],
}

/// Why a [`SeedPopulation`] cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedError {
    /// Columns of one table differ in length.
    ColumnLengths,
    /// A person points at a household that does not exist.
    UnknownHousehold {
        /// The person.
        person: usize,
    },
    /// A household is in a county the world does not have.
    UnknownCounty {
        /// The household.
        household: usize,
    },
    /// A household has weight 0.
    ZeroWeight {
        /// The household.
        household: usize,
    },
}

/// Commands accepted by the spike world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScaleCommand {
    /// Change the monthly job-separation rate.
    SetSeparationRate(f64),
}

/// All state of a [`ScaleWorld`] as plain columns (for saves).
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleColumns {
    /// Real people per synthetic person.
    pub sample_scale: u32,
    /// Master seed.
    pub seed: u64,
    /// Next tick to run.
    pub tick: u32,
    /// Monthly separation rate.
    pub separation_rate: f64,
    /// Behaviour parameters.
    pub params: ScaleParams,
    /// Person columns.
    pub age: Vec<u8>,
    /// Person columns.
    pub county: Vec<u8>,
    /// Person columns.
    pub edu: Vec<u8>,
    /// Person columns.
    pub status: Vec<u8>,
    /// Person columns.
    pub industry: Vec<u16>,
    /// Person wage (per represented person), bani.
    pub wage: Vec<i64>,
    /// Person → household.
    pub household: Vec<u32>,
    /// Household weights.
    pub hh_weight: Vec<u32>,
    /// Household deposits (per represented household), bani.
    pub hh_deposits: Vec<i64>,
    /// Household income last tick, bani.
    pub hh_income: Vec<i64>,
    /// VAT collected by government so far, bani (weighted total).
    pub gov_vat: i64,
    /// Input-output matrix (row-major).
    pub io: Vec<f64>,
    /// Consumption shares per good.
    pub consumption_shares: Vec<f64>,
}

/// Results of one tick.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaleReport {
    /// Weighted employment.
    pub employed: u64,
    /// Weighted unemployment.
    pub unemployed: u64,
    /// Weighted wage bill.
    pub wage_bill: Bani,
    /// Weighted household consumption at purchaser prices (VAT included).
    pub consumption: Bani,
    /// Weighted VAT collected this tick (part of `consumption`).
    pub vat: Bani,
    /// Sum of gross output (float, for the IO solve check).
    pub gross_output: f64,
    /// Number of non-empty cube cells.
    pub cube_cells: usize,
    /// Every wage clearing account netted to zero (I-8).
    pub clearing_ok: bool,
    /// Ledger invariants I-1 to I-3 hold, and the ledger's household and
    /// government balances equal the weighted column totals (I-4).
    pub ledger_ok: bool,
}

fn region_of(county: u8) -> usize {
    usize::from(county) % N_REGIONS
}

/// VAT group of a good (fake assignment, stand-in for the goods catalogue):
/// index into `[standard, reduced, untaxed]`.
fn vat_group_of(good: usize) -> usize {
    match good % 10 {
        0..=5 => 0,
        6..=8 => 1,
        _ => 2,
    }
}

/// VAT categories of the taxed groups, in group order.
const TAXED_GROUPS: [VatCategory; 2] = [VatCategory::Standard, VatCategory::Reduced];

/// Split what a household pays for goods of one VAT category into the net
/// amount and the VAT on it, so that VAT is exactly `rate × net` rounded to
/// whole bani (AC-VAT-01) and `net + vat <= gross`. At most one ban of
/// `gross` is left unspent by rounding.
fn split_gross(gross: Bani, cat: VatCategory) -> (Bani, Bani) {
    let Some(rate) = VAT_SCHEDULE.rate_for(cat) else {
        return (gross, Bani::ZERO);
    };
    let bp = i64::from(rate.0);
    let mut net = gross.mul_ratio(10_000, 10_000 + bp);
    let mut vat = VAT_SCHEDULE.vat(net, cat);
    while net + vat > gross {
        net -= Bani(1);
        vat = VAT_SCHEDULE.vat(net, cat);
    }
    (net, vat)
}

fn skill_of(edu: u8) -> usize {
    match edu {
        0 | 1 => 0,
        2 | 3 => 1,
        _ => 2,
    }
}

impl ScaleWorld {
    /// Build a fake population at `sample_scale` (e.g. 100 for 1:100).
    ///
    /// # Panics
    /// If `sample_scale` is zero.
    #[must_use]
    pub fn generate(sample_scale: u32, seed: u64) -> Self {
        assert!(sample_scale > 0, "sample_scale must be > 0");
        let rng = KeyedRng::new(seed);
        let n_persons = usize::try_from(REAL_POPULATION / u64::from(sample_scale)).expect("size");
        let mut w = ScaleWorld {
            sample_scale,
            age: Vec::with_capacity(n_persons),
            county: Vec::with_capacity(n_persons),
            edu: Vec::with_capacity(n_persons),
            status: Vec::with_capacity(n_persons),
            industry: Vec::with_capacity(n_persons),
            wage: Vec::with_capacity(n_persons),
            household: Vec::with_capacity(n_persons),
            hh_weight: Vec::new(),
            hh_deposits: Vec::new(),
            hh_income: Vec::new(),
            gov_vat: Bani::ZERO,
            ledger: Ledger::new(),
            io: Vec::new(),
            consumption_shares: Vec::new(),
            seed_value: seed,
            seed: rng,
            tick: 0,
            separation_rate: 0.015,
            params: ScaleParams::default(),
        };
        let mut hh: u32 = 0;
        let mut members_left = 0u64;
        for i in 0..n_persons {
            let mut d = rng.draw(Stream::PopulationGen, 0, i as u64);
            if members_left == 0 {
                members_left = 1 + d.below(4);
                w.hh_weight.push(sample_scale);
                w.hh_deposits.push(Bani::from_lei(
                    i64::try_from(d.below(50_000)).expect("small"),
                ));
                w.hh_income.push(Bani::ZERO);
                hh = u32::try_from(w.hh_weight.len() - 1).expect("hh id");
            }
            members_left -= 1;
            let age = u8::try_from(d.below(90)).expect("age");
            let edu = u8::try_from(d.below(5)).expect("edu");
            let status = if age < 20 {
                0
            } else if age >= 65 {
                3
            } else if d.bernoulli(0.92) {
                1
            } else {
                2
            };
            w.age.push(age);
            w.county
                .push(u8::try_from(d.below(N_COUNTIES as u64)).expect("county"));
            w.edu.push(edu);
            w.status.push(status);
            w.industry
                .push(u16::try_from(d.below(N_INDUSTRIES as u64)).expect("ind"));
            let base = 3_000 + 2_000 * i64::from(edu) + i64::try_from(d.below(2_000)).expect("w");
            w.wage.push(Bani::from_lei(base));
            w.household.push(hh);
        }
        w.fake_economy();
        w.ledger = w.opening_ledger();
        w
    }

    /// Build the world on a real starting population. Households, their
    /// weights and counties, and every person's age come from `pop`.
    /// Education, employment, industry, wages and deposits are invented by
    /// the same rules as in [`ScaleWorld::generate`], until the generator
    /// increments and mechanics that own them exist.
    ///
    /// `sample_scale` is the scale `pop` was generated at (it is recorded, not
    /// used to size anything: weights come from `pop`).
    ///
    /// # Errors
    /// See [`SeedError`].
    ///
    /// # Panics
    /// If `sample_scale` is zero.
    pub fn from_population(
        sample_scale: u32,
        seed: u64,
        pop: &SeedPopulation<'_>,
    ) -> Result<Self, SeedError> {
        assert!(sample_scale > 0, "sample_scale must be > 0");
        let (n_hh, n_persons) = (pop.hh_weight.len(), pop.household_id.len());
        if pop.hh_county.len() != n_hh || pop.age_months.len() != n_persons {
            return Err(SeedError::ColumnLengths);
        }
        if let Some(household) = pop.hh_weight.iter().position(|&w| w == 0) {
            return Err(SeedError::ZeroWeight { household });
        }
        if let Some(household) = pop
            .hh_county
            .iter()
            .position(|&c| usize::from(c) >= N_COUNTIES)
        {
            return Err(SeedError::UnknownCounty { household });
        }
        if let Some(person) = pop.household_id.iter().position(|&h| h as usize >= n_hh) {
            return Err(SeedError::UnknownHousehold { person });
        }
        let rng = KeyedRng::new(seed);
        let mut w = ScaleWorld {
            sample_scale,
            age: Vec::with_capacity(n_persons),
            county: Vec::with_capacity(n_persons),
            edu: Vec::with_capacity(n_persons),
            status: Vec::with_capacity(n_persons),
            industry: Vec::with_capacity(n_persons),
            wage: Vec::with_capacity(n_persons),
            household: pop.household_id.to_vec(),
            hh_weight: pop.hh_weight.to_vec(),
            hh_deposits: Vec::with_capacity(n_hh),
            hh_income: vec![Bani::ZERO; n_hh],
            gov_vat: Bani::ZERO,
            ledger: Ledger::new(),
            io: Vec::new(),
            consumption_shares: Vec::new(),
            seed_value: seed,
            seed: rng,
            tick: 0,
            separation_rate: 0.015,
            params: ScaleParams::default(),
        };
        // Draw contexts: tick 0 = per person, 3 = per household (1 and 2 are
        // the IO matrix and the consumption shares).
        for h in 0..n_hh {
            let lei = rng.draw(Stream::PopulationGen, 3, h as u64).below(50_000);
            w.hh_deposits
                .push(Bani::from_lei(i64::try_from(lei).expect("small")));
        }
        for i in 0..n_persons {
            let mut d = rng.draw(Stream::PopulationGen, 0, i as u64);
            let age = u8::try_from(pop.age_months[i] / 12).unwrap_or(u8::MAX);
            let edu = u8::try_from(d.below(5)).expect("edu");
            let status = if age < 20 {
                0
            } else if age >= 65 {
                3
            } else if d.bernoulli(0.92) {
                1
            } else {
                2
            };
            w.age.push(age);
            w.county.push(pop.hh_county[pop.household_id[i] as usize]);
            w.edu.push(edu);
            w.status.push(status);
            w.industry
                .push(u16::try_from(d.below(N_INDUSTRIES as u64)).expect("ind"));
            let base = 3_000 + 2_000 * i64::from(edu) + i64::try_from(d.below(2_000)).expect("w");
            w.wage.push(Bani::from_lei(base));
        }
        w.fake_economy();
        w.ledger = w.opening_ledger();
        Ok(w)
    }

    /// Invented input-output matrix (productive) and consumption shares.
    fn fake_economy(&mut self) {
        let rng = self.seed;
        let mut io = vec![0.0; N_INDUSTRIES * N_INDUSTRIES];
        for (k, v) in io.iter_mut().enumerate() {
            *v = rng.draw(Stream::PopulationGen, 1, k as u64).uniform() * 0.5 / N_INDUSTRIES as f64;
        }
        self.io = io;
        let raw: Vec<f64> = (0..N_GOODS)
            .map(|g| 0.5 + rng.draw(Stream::PopulationGen, 2, g as u64).uniform())
            .collect();
        let tot: f64 = raw.iter().sum();
        self.consumption_shares = raw.iter().map(|x| x / tot).collect();
    }

    /// Real persons represented: the sum of household weights over persons.
    #[must_use]
    pub fn real_persons(&self) -> u64 {
        self.household
            .iter()
            .map(|&h| u64::from(self.hh_weight[h as usize]))
            .sum()
    }

    /// Weighted total of household deposits.
    fn weighted_deposits(&self) -> Bani {
        self.hh_deposits
            .iter()
            .zip(&self.hh_weight)
            .map(|(d, &w)| d.times(i64::from(w)))
            .sum()
    }

    /// The sector ledger implied by the columns: households hold their
    /// weighted deposits, government holds the VAT collected so far.
    ///
    /// Spike simplification: firms have no balance sheet here, so the firm
    /// sector is the counterparty of the opening balances and may be negative.
    fn opening_ledger(&self) -> Ledger {
        let mut l = Ledger::new();
        l.allow_negative(Sector::Firms, Instrument::Cash);
        let mut txn = Txn::new();
        for (payee, amount) in [
            (Sector::Households, self.weighted_deposits()),
            (Sector::Government, self.gov_vat),
        ] {
            if amount > Bani::ZERO {
                txn = txn.leg(
                    Sector::Firms,
                    payee,
                    Instrument::Cash,
                    amount,
                    FlowCode::OPENING,
                );
            }
        }
        l.commit(txn).expect("opening balances");
        l.begin_tick(self.tick);
        l
    }

    /// VAT collected by government so far (weighted total).
    pub fn gov_vat(&self) -> Bani {
        self.gov_vat
    }

    /// Number of synthetic person records.
    #[must_use]
    pub fn n_persons(&self) -> usize {
        self.age.len()
    }

    /// Number of synthetic households.
    #[must_use]
    pub fn n_households(&self) -> usize {
        self.hh_weight.len()
    }

    /// Set behaviour parameters and the separation rate (calibration runs).
    pub fn set_params(&mut self, params: ScaleParams, separation_rate: f64) {
        self.params = params;
        self.separation_rate = separation_rate;
    }

    /// The next tick to run.
    #[must_use]
    pub fn tick(&self) -> u32 {
        self.tick
    }

    /// Export all state as columns.
    #[must_use]
    pub fn to_columns(&self) -> ScaleColumns {
        ScaleColumns {
            sample_scale: self.sample_scale,
            seed: self.seed_value,
            tick: self.tick,
            separation_rate: self.separation_rate,
            params: self.params,
            age: self.age.clone(),
            county: self.county.clone(),
            edu: self.edu.clone(),
            status: self.status.clone(),
            industry: self.industry.clone(),
            wage: self.wage.iter().map(|b| b.get()).collect(),
            household: self.household.clone(),
            hh_weight: self.hh_weight.clone(),
            hh_deposits: self.hh_deposits.iter().map(|b| b.get()).collect(),
            hh_income: self.hh_income.iter().map(|b| b.get()).collect(),
            gov_vat: self.gov_vat.get(),
            io: self.io.clone(),
            consumption_shares: self.consumption_shares.clone(),
        }
    }

    /// Rebuild from columns.
    #[must_use]
    pub fn from_columns(c: ScaleColumns) -> Self {
        let mut w = ScaleWorld {
            sample_scale: c.sample_scale,
            age: c.age,
            county: c.county,
            edu: c.edu,
            status: c.status,
            industry: c.industry,
            wage: c.wage.into_iter().map(Bani).collect(),
            household: c.household,
            hh_weight: c.hh_weight,
            hh_deposits: c.hh_deposits.into_iter().map(Bani).collect(),
            hh_income: c.hh_income.into_iter().map(Bani).collect(),
            gov_vat: Bani(c.gov_vat),
            ledger: Ledger::new(),
            io: c.io,
            consumption_shares: c.consumption_shares,
            seed_value: c.seed,
            seed: KeyedRng::new(c.seed),
            tick: c.tick,
            separation_rate: c.separation_rate,
            params: c.params,
        };
        w.ledger = w.opening_ledger();
        w
    }

    /// Hash of the full state (FNV-1a over every column in a fixed order).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let mut h = crate::StateHasher::default();
        let c = self.to_columns();
        h.write(&c.sample_scale.to_le_bytes());
        h.write(&c.seed.to_le_bytes());
        h.write(&c.tick.to_le_bytes());
        h.write_f64(c.separation_rate);
        h.write_f64(c.params.matching_efficiency);
        h.write_f64(c.params.vacancy_ratio);
        h.write_f64(c.params.mpc_income);
        h.write_f64(c.params.mpc_wealth);
        h.write(&c.age);
        h.write(&c.county);
        h.write(&c.edu);
        h.write(&c.status);
        for v in &c.industry {
            h.write(&v.to_le_bytes());
        }
        for v in &c.wage {
            h.write(&v.to_le_bytes());
        }
        for v in &c.household {
            h.write(&v.to_le_bytes());
        }
        for v in &c.hh_weight {
            h.write(&v.to_le_bytes());
        }
        for v in &c.hh_deposits {
            h.write(&v.to_le_bytes());
        }
        for v in &c.hh_income {
            h.write(&v.to_le_bytes());
        }
        h.write(&c.gov_vat.to_le_bytes());
        for v in &c.io {
            h.write_f64(*v);
        }
        for v in &c.consumption_shares {
            h.write_f64(*v);
        }
        h.finish()
    }

    /// Apply player commands (at the start of a tick).
    pub fn apply(&mut self, cmds: &[ScaleCommand]) {
        for c in cmds {
            match *c {
                ScaleCommand::SetSeparationRate(r) => {
                    self.separation_rate = econ_num::finite(r, "separation rate").clamp(0.0, 1.0);
                }
            }
        }
    }

    /// Run one monthly tick.
    ///
    /// # Panics
    /// If the IO system is singular (it is constructed to be productive).
    pub fn step(&mut self) -> ScaleReport {
        let tick = self.tick;
        self.ledger.begin_tick(tick);
        let sep = self.separation_rate;
        let n = self.n_persons();
        // 1. Separations (separation_rate per month, default 1.5%).
        for i in 0..n {
            if self.status[i] == 1
                && self
                    .seed
                    .fast_bernoulli(Stream::Labour, tick, i as u64, 0, sep)
            {
                self.status[i] = 2;
            }
        }
        // 2. Matching per (region, skill): hires = μ·U^0.5·V^0.5, aligned.
        let cells = N_REGIONS * N_SKILLS;
        let mut seekers: Vec<Vec<(u64, u32)>> = vec![Vec::new(); cells];
        for i in 0..n {
            if self.status[i] == 2 {
                let c = region_of(self.county[i]) * N_SKILLS + skill_of(self.edu[i]);
                let prio = self.seed.fast_u64(Stream::Labour, tick, i as u64, 1);
                seekers[c].push((prio, u32::try_from(i).expect("id")));
            }
        }
        for s in &mut seekers {
            // Vacancies: fake, proportional to seekers (stand-in for firm demand).
            let u = s.len() as f64;
            let v = self.params.vacancy_ratio * u;
            let hires_f = self.params.matching_efficiency * (u * v).sqrt();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let hires = (hires_f.floor() as usize).min(s.len());
            // Alignment: exactly `hires` people, chosen by keyed priority (ties by id).
            s.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
            for &(_, id) in s.iter().take(hires) {
                self.status[id as usize] = 1;
            }
        }
        // 3. Wages through per-industry wage clearing accounts (I-8).
        for x in &mut self.hh_income {
            *x = Bani::ZERO;
        }
        let mut clearing = vec![Bani::ZERO; N_INDUSTRIES];
        let mut wage_bill = Bani::ZERO;
        for i in 0..n {
            if self.status[i] == 1 {
                let h = self.household[i] as usize;
                let wgt = i64::from(self.hh_weight[h]);
                let total = self.wage[i].times(wgt);
                let j = usize::from(self.industry[i]);
                clearing[j] += total; // firms pay into the clearing account…
                clearing[j] -= total; // …households receive from it
                self.hh_income[h] += self.wage[i];
                wage_bill += total;
            }
        }
        let clearing_ok = clearing.iter().all(|c| c.is_zero());
        // 4. Consumption: 85% of income + 0.5% of deposits at purchaser prices.
        //    VAT is carved out per household and VAT category (exact per
        //    purchase), then posted as aggregates through the ledger.
        let weights_ppm: Vec<u64> = self
            .consumption_shares
            .iter()
            .map(|s| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let v = (s * 1e6).round() as u64;
                v
            })
            .collect();
        let mut group_ppm = [0i64; N_VAT_GROUPS];
        for (g, &w) in weights_ppm.iter().enumerate() {
            group_ppm[vat_group_of(g)] += i64::try_from(w).expect("share");
        }
        let all_ppm: i64 = group_ppm.iter().sum();
        let mut consumption = Bani::ZERO;
        let mut vat_total = Bani::ZERO;
        // What sellers receive, per VAT group (basic prices).
        let mut net_by_group = [Bani::ZERO; N_VAT_GROUPS];
        for h in 0..self.n_households() {
            let c = self.hh_income[h].mul_rate(self.params.mpc_income)
                + self.hh_deposits[h].mul_rate(self.params.mpc_wealth);
            let c = if c > self.hh_deposits[h] + self.hh_income[h] {
                self.hh_deposits[h] + self.hh_income[h]
            } else {
                c
            };
            let wgt = i64::from(self.hh_weight[h]);
            let mut untaxed = c;
            let mut spent = Bani::ZERO;
            for (k, &cat) in TAXED_GROUPS.iter().enumerate() {
                let gross = c.mul_ratio(group_ppm[k], all_ppm);
                untaxed -= gross;
                let (net, vat) = split_gross(gross, cat);
                spent += net + vat;
                net_by_group[k] += net.times(wgt);
                vat_total += vat.times(wgt);
            }
            spent += untaxed;
            net_by_group[N_VAT_GROUPS - 1] += untaxed.times(wgt);
            self.hh_deposits[h] = self.hh_deposits[h] + self.hh_income[h] - spent;
            consumption += spent.times(wgt);
        }
        let mut txn = Txn::new();
        for (payer, payee, amount, code) in [
            (
                Sector::Firms,
                Sector::Households,
                wage_bill,
                FlowCode::WAGES,
            ),
            (
                Sector::Households,
                Sector::Firms,
                consumption - vat_total,
                FlowCode::CONSUMPTION,
            ),
            (
                Sector::Households,
                Sector::Government,
                vat_total,
                FlowCode::TAX_VAT,
            ),
        ] {
            if amount > Bani::ZERO {
                txn = txn.leg(payer, payee, Instrument::Cash, amount, code);
            }
        }
        self.ledger
            .commit(txn)
            .expect("household spending is capped by deposits and income");
        self.gov_vat += vat_total;
        let ledger_ok = self.ledger.check_invariants().is_ok()
            && self.ledger.balance(Sector::Households, Instrument::Cash)
                == self.weighted_deposits()
            && self.ledger.balance(Sector::Government, Instrument::Cash) == self.gov_vat;
        // Spike simplification: one basket for everyone, so split each VAT
        // group's total once over its goods (the real model splits per basket
        // class, a handful of splits per tick). Demand is at basic prices.
        let mut demand = vec![Bani::ZERO; N_GOODS];
        for (k, &total) in net_by_group.iter().enumerate() {
            let w: Vec<u64> = weights_ppm
                .iter()
                .enumerate()
                .map(|(g, &w)| if vat_group_of(g) == k { w } else { 0 })
                .collect();
            for (g, part) in split_largest_remainder(total, &w).into_iter().enumerate() {
                demand[g] += part;
            }
        }
        // 5. Leontief solve for gross output.
        let mut final_demand = vec![0.0; N_INDUSTRIES];
        for (g, d) in demand.iter().enumerate() {
            final_demand[g] = d.to_f64_exact() / 1e8;
        }
        let x = leontief_output(N_INDUSTRIES, &self.io, &final_demand).expect("productive IO");
        let gross_output = econ_num::det_sum(&x);
        // 6. Aggregation cube (weighted counts).
        let mut cube = vec![0u64; N_COUNTIES * N_STATUS * N_AGE_BANDS * N_EDU];
        let mut employed = 0u64;
        let mut unemployed = 0u64;
        for i in 0..n {
            let wgt = u64::from(self.hh_weight[self.household[i] as usize]);
            let band = (usize::from(self.age[i]) / 12).min(N_AGE_BANDS - 1);
            let idx = ((usize::from(self.county[i]) * N_STATUS + usize::from(self.status[i]))
                * N_AGE_BANDS
                + band)
                * N_EDU
                + usize::from(self.edu[i]);
            cube[idx] += wgt;
            match self.status[i] {
                1 => employed += wgt,
                2 => unemployed += wgt,
                _ => {}
            }
        }
        self.tick += 1;
        ScaleReport {
            employed,
            unemployed,
            wage_bill,
            consumption,
            vat: vat_total,
            gross_output,
            cube_cells: cube.iter().filter(|&&c| c > 0).count(),
            clearing_ok,
            ledger_ok,
        }
    }
}

/// Days of the month on which households take their monthly decisions
/// (ADR-0017, staggering): 1 to 28, which every month has.
pub const STAGGER_DAYS: u8 = 28;

/// Who takes their monthly decisions on which day of the month (ADR-0017).
#[derive(Debug, Clone)]
pub struct DaySchedule {
    /// Households of each day; index 0 is the first of the month.
    households: Vec<Vec<u32>>,
    /// The members of those households.
    persons: Vec<Vec<u32>>,
    /// Hires owed to each (region, skill) cell from earlier days: the part of
    /// a hire that a day's few job seekers were too few for. Without it a
    /// cell with four seekers a day would never hire anyone.
    hire_carry: Vec<f64>,
}

/// What the month-end phases of [`ScaleWorld::step_day`] report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthClose {
    /// Weighted employment.
    pub employed: u64,
    /// Weighted unemployment.
    pub unemployed: u64,
    /// Weighted wage bill of the month.
    pub wage_bill: Bani,
    /// Number of non-empty cube cells.
    pub cube_cells: usize,
    /// Every wage clearing account netted to zero (I-8).
    pub clearing_ok: bool,
}

/// Results of one day.
#[derive(Debug, Clone, PartialEq)]
pub struct DayReport {
    /// Weighted consumption of the households that shopped today (VAT included).
    pub consumption: Bani,
    /// Weighted VAT collected today (part of `consumption`).
    pub vat: Bani,
    /// Sum of gross output for today's demand.
    pub gross_output: f64,
    /// Ledger invariants hold and the ledger agrees with the columns (I-1 to I-4).
    pub ledger_ok: bool,
    /// The month-end phases, on the last day of a month.
    pub month: Option<MonthClose>,
}

impl ScaleWorld {
    /// Give every household a day of the month, 1 to [`STAGGER_DAYS`], by a keyed draw.
    #[must_use]
    pub fn day_schedule(&self) -> DaySchedule {
        let days = usize::from(STAGGER_DAYS);
        let mut households = vec![Vec::new(); days];
        let mut persons = vec![Vec::new(); days];
        let day_of = |h: usize| {
            // Draw index 7 of the consumption stream at tick 0 is used for nothing else.
            let draw = self.seed.fast_u64(Stream::Consumption, 0, h as u64, 7);
            usize::try_from(draw % u64::from(STAGGER_DAYS)).expect("below 28")
        };
        for h in 0..self.n_households() {
            households[day_of(h)].push(u32::try_from(h).expect("id"));
        }
        for i in 0..self.n_persons() {
            persons[day_of(self.household[i] as usize)].push(u32::try_from(i).expect("id"));
        }
        DaySchedule {
            households,
            persons,
            hire_carry: vec![0.0; N_REGIONS * N_SKILLS],
        }
    }

    /// Run one daily tick (ADR-0017): the same work as [`ScaleWorld::step`],
    /// each part at its own period.
    ///
    /// - **Staggered:** the households whose day it is, and their members, do
    ///   their month's separations, job search and spending.
    /// - **Daily:** the goods market: demand to gross output through the
    ///   input-output system, the ledger and its checks.
    /// - **Month end:** wages for everyone employed, and the statistics cube.
    ///
    /// A world is stepped either with this or with `step`, not both.
    ///
    /// # Panics
    /// If the IO system is singular (it is constructed to be productive).
    pub fn step_day(&mut self, schedule: &mut DaySchedule, date: Date) -> DayReport {
        let tick = self.tick;
        self.ledger.begin_tick(tick);
        let weights_ppm: Vec<u64> = self
            .consumption_shares
            .iter()
            .map(|s| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let v = (s * 1e6).round() as u64;
                v
            })
            .collect();
        let mut consumption = Bani::ZERO;
        let mut vat_total = Bani::ZERO;
        let mut net_by_group = [Bani::ZERO; N_VAT_GROUPS];
        if date.day <= STAGGER_DAYS {
            let cohort = usize::from(date.day - 1);
            // Separations and matching among today's persons.
            let sep = self.separation_rate;
            let mut seekers: Vec<Vec<(u64, u32)>> = vec![Vec::new(); N_REGIONS * N_SKILLS];
            for &id in &schedule.persons[cohort] {
                let i = id as usize;
                if self.status[i] == 1
                    && self
                        .seed
                        .fast_bernoulli(Stream::Labour, tick, u64::from(id), 0, sep)
                {
                    self.status[i] = 2;
                }
                if self.status[i] == 2 {
                    let c = region_of(self.county[i]) * N_SKILLS + skill_of(self.edu[i]);
                    let prio = self.seed.fast_u64(Stream::Labour, tick, u64::from(id), 1);
                    seekers[c].push((prio, id));
                }
            }
            for (s, carry) in seekers.iter_mut().zip(&mut schedule.hire_carry) {
                let u = s.len() as f64;
                let v = self.params.vacancy_ratio * u;
                let owed = self.params.matching_efficiency * (u * v).sqrt() + *carry;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let hires = (owed.floor() as usize).min(s.len());
                *carry = (owed - hires as f64).min(1.0);
                s.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
                for &(_, id) in s.iter().take(hires) {
                    self.status[id as usize] = 1;
                }
            }
            // Spending of today's households, out of last pay day's income and deposits.
            let mut group_ppm = [0i64; N_VAT_GROUPS];
            for (g, &w) in weights_ppm.iter().enumerate() {
                group_ppm[vat_group_of(g)] += i64::try_from(w).expect("share");
            }
            let all_ppm: i64 = group_ppm.iter().sum();
            for &id in &schedule.households[cohort] {
                let h = id as usize;
                let c = self.hh_income[h].mul_rate(self.params.mpc_income)
                    + self.hh_deposits[h].mul_rate(self.params.mpc_wealth);
                let c = if c > self.hh_deposits[h] {
                    self.hh_deposits[h]
                } else {
                    c
                };
                let wgt = i64::from(self.hh_weight[h]);
                let mut untaxed = c;
                let mut spent = Bani::ZERO;
                for (k, &cat) in TAXED_GROUPS.iter().enumerate() {
                    let gross = c.mul_ratio(group_ppm[k], all_ppm);
                    untaxed -= gross;
                    let (net, vat) = split_gross(gross, cat);
                    spent += net + vat;
                    net_by_group[k] += net.times(wgt);
                    vat_total += vat.times(wgt);
                }
                spent += untaxed;
                net_by_group[N_VAT_GROUPS - 1] += untaxed.times(wgt);
                self.hh_deposits[h] -= spent;
                consumption += spent.times(wgt);
            }
        }

        // Month end: wages for everyone employed, through the clearing accounts.
        let month_end = date.is_month_end();
        let mut wage_bill = Bani::ZERO;
        let mut clearing_ok = true;
        if month_end {
            for x in &mut self.hh_income {
                *x = Bani::ZERO;
            }
            let mut clearing = vec![Bani::ZERO; N_INDUSTRIES];
            for i in 0..self.n_persons() {
                if self.status[i] == 1 {
                    let h = self.household[i] as usize;
                    let total = self.wage[i].times(i64::from(self.hh_weight[h]));
                    let j = usize::from(self.industry[i]);
                    clearing[j] += total;
                    clearing[j] -= total;
                    self.hh_income[h] += self.wage[i];
                    self.hh_deposits[h] += self.wage[i];
                    wage_bill += total;
                }
            }
            clearing_ok = clearing.iter().all(|c| c.is_zero());
        }

        // Daily: the ledger and its checks.
        let mut txn = Txn::new();
        let mut any = false;
        for (payer, payee, amount, code) in [
            (
                Sector::Firms,
                Sector::Households,
                wage_bill,
                FlowCode::WAGES,
            ),
            (
                Sector::Households,
                Sector::Firms,
                consumption - vat_total,
                FlowCode::CONSUMPTION,
            ),
            (
                Sector::Households,
                Sector::Government,
                vat_total,
                FlowCode::TAX_VAT,
            ),
        ] {
            if amount > Bani::ZERO {
                txn = txn.leg(payer, payee, Instrument::Cash, amount, code);
                any = true;
            }
        }
        if any {
            self.ledger
                .commit(txn)
                .expect("household spending is capped by deposits");
        }
        self.gov_vat += vat_total;
        let ledger_ok = self.ledger.check_invariants().is_ok()
            && self.ledger.balance(Sector::Households, Instrument::Cash)
                == self.weighted_deposits()
            && self.ledger.balance(Sector::Government, Instrument::Cash) == self.gov_vat;

        // Daily: the goods market, from today's demand to gross output.
        let mut demand = vec![Bani::ZERO; N_GOODS];
        for (k, &total) in net_by_group.iter().enumerate() {
            let w: Vec<u64> = weights_ppm
                .iter()
                .enumerate()
                .map(|(g, &w)| if vat_group_of(g) == k { w } else { 0 })
                .collect();
            for (g, part) in split_largest_remainder(total, &w).into_iter().enumerate() {
                demand[g] += part;
            }
        }
        let mut final_demand = vec![0.0; N_INDUSTRIES];
        for (g, d) in demand.iter().enumerate() {
            final_demand[g] = d.to_f64_exact() / 1e8;
        }
        let x = leontief_output(N_INDUSTRIES, &self.io, &final_demand).expect("productive IO");
        let gross_output = econ_num::det_sum(&x);

        // Month end: the statistics cube.
        let month = month_end.then(|| {
            let mut cube = vec![0u64; N_COUNTIES * N_STATUS * N_AGE_BANDS * N_EDU];
            let (mut employed, mut unemployed) = (0u64, 0u64);
            for i in 0..self.n_persons() {
                let wgt = u64::from(self.hh_weight[self.household[i] as usize]);
                let band = (usize::from(self.age[i]) / 12).min(N_AGE_BANDS - 1);
                let idx = ((usize::from(self.county[i]) * N_STATUS + usize::from(self.status[i]))
                    * N_AGE_BANDS
                    + band)
                    * N_EDU
                    + usize::from(self.edu[i]);
                cube[idx] += wgt;
                match self.status[i] {
                    1 => employed += wgt,
                    2 => unemployed += wgt,
                    _ => {}
                }
            }
            MonthClose {
                employed,
                unemployed,
                wage_bill,
                cube_cells: cube.iter().filter(|&&c| c > 0).count(),
                clearing_ok,
            }
        });
        self.tick += 1;
        DayReport {
            consumption,
            vat: vat_total,
            gross_output,
            ledger_ok,
            month,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 3-scale test (ADR-0010): invariants hold and per-capita aggregates
    /// agree within sampling error at three population scales.
    #[test]
    fn three_scale_per_capita_agreement() {
        let mut results = Vec::new();
        for scale in [2000u32, 1000, 500] {
            let mut w = ScaleWorld::generate(scale, 11);
            let mut last = None;
            for _ in 0..6 {
                last = Some(w.step());
            }
            let r = last.unwrap();
            assert!(r.clearing_ok);
            assert!(r.ledger_ok);
            let pop = REAL_POPULATION as f64;
            let u_rate = r.unemployed as f64 / (r.employed + r.unemployed) as f64;
            results.push((
                scale,
                r.wage_bill.to_f64_exact() / pop,
                u_rate,
                r.vat.to_f64_exact() / pop,
            ));
        }
        let (_, w0, u0, v0) = results[2];
        for &(scale, w, u, v) in &results {
            assert!(
                ((v - v0) / v0).abs() < 0.06,
                "VAT per capita at 1:{scale} = {v} vs {v0}"
            );
            assert!(
                ((w - w0) / w0).abs() < 0.06,
                "wage per capita at 1:{scale} = {w} vs {w0}"
            );
            assert!(
                (u - u0).abs() < 0.02,
                "unemployment at 1:{scale} = {u} vs {u0}"
            );
        }
    }

    /// AC-VAT-01 at the point of use: the VAT carved out of a gross amount is
    /// exactly `rate × net`, never more than the household pays, and rounding
    /// leaves at most one ban unspent.
    #[test]
    fn split_gross_is_exact_vat_on_net() {
        for cat in TAXED_GROUPS {
            let rate = VAT_SCHEDULE.rate_for(cat).unwrap();
            for g in (0..5_000).chain((1_000_003..50_000_000).step_by(999_983)) {
                let gross = Bani(g);
                let (net, vat) = split_gross(gross, cat);
                assert_eq!(vat, econ_mech_tax::vat::vat_on(net, rate), "gross {g}");
                let unspent = gross - net - vat;
                assert!(
                    (0..=1).contains(&unspent.get()),
                    "gross {g}: unspent {unspent:?}"
                );
            }
        }
        assert_eq!(
            split_gross(Bani(12_100), VatCategory::Standard),
            (Bani(10_000), Bani(2_100))
        );
        assert_eq!(
            split_gross(Bani(777), VatCategory::Exempt),
            (Bani(777), Bani::ZERO)
        );
    }

    /// VAT is posted through the ledger (flow code `tax.vat`), stock-flow
    /// consistently: government's balance is the running total, households'
    /// balance equals their weighted deposits, and firms receive the rest.
    #[test]
    fn vat_goes_from_households_to_government_through_the_ledger() {
        let mut w = ScaleWorld::generate(2000, 3);
        let deposits_before = w.weighted_deposits();
        let mut collected = Bani::ZERO;
        let mut wages = Bani::ZERO;
        let mut spent = Bani::ZERO;
        for _ in 0..4 {
            let r = w.step();
            assert!(r.ledger_ok && r.clearing_ok);
            assert!(r.vat > Bani::ZERO && r.vat < r.consumption);
            let f = w.ledger.flows();
            assert_eq!(f.received(FlowCode::TAX_VAT, Sector::Government), r.vat);
            assert_eq!(f.get(FlowCode::TAX_VAT, Sector::Households), -r.vat);
            assert_eq!(
                f.received(FlowCode::CONSUMPTION, Sector::Firms),
                r.consumption - r.vat
            );
            // The basket mixes 21%, 11% and untaxed goods: the VAT share of
            // spending lies between zero and 21/121.
            let share = r.vat.to_f64_exact() / r.consumption.to_f64_exact();
            assert!(share > 0.05 && share < 21.0 / 121.0, "VAT share {share}");
            collected += r.vat;
            wages += r.wage_bill;
            spent += r.consumption;
        }
        assert_eq!(w.gov_vat(), collected);
        assert_eq!(
            w.ledger.balance(Sector::Government, Instrument::Cash),
            collected
        );
        assert_eq!(w.weighted_deposits(), deposits_before + wages - spent);
    }

    /// The ledger is rebuilt from the columns, so a saved and reloaded world
    /// continues exactly like the original.
    #[test]
    fn columns_round_trip_keeps_ledger_and_vat() {
        let mut a = ScaleWorld::generate(5000, 9);
        a.step();
        a.step();
        let mut b = ScaleWorld::from_columns(a.to_columns());
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(a.step(), b.step());
        assert_eq!(a.gov_vat(), b.gov_vat());
        assert_eq!(a.state_hash(), b.state_hash());
    }

    /// Golden run of the scale world: 12 ticks at 1:1000, seed 42. A change
    /// here is a re-golden event (DETERMINISM.md): explain it in
    /// CHANGELOG-sim.md and update the constants in the same change.
    #[test]
    fn golden_12_ticks_at_1_in_1000() {
        let mut w = ScaleWorld::generate(1000, 42);
        let mut last = None;
        for _ in 0..12 {
            last = Some(w.step());
        }
        let r = last.unwrap();
        let got = (
            w.state_hash(),
            r.employed,
            r.wage_bill.get(),
            r.consumption.get(),
            r.vat.get(),
            w.gov_vat().get(),
        );
        assert_eq!(got, GOLDEN_12, "scale-world golden changed: {got:?}");
    }

    /// (state hash, employed, wage bill, consumption, VAT in tick 12, VAT collected in 12 ticks).
    const GOLDEN_12: (u64, u64, i64, i64, i64, i64) = (
        0x8557_a49a_52c1_e86a,
        8_890_000,
        7_129_664_400_000,
        6_207_483_617_000,
        823_228_305_000,
        9_769_553_421_000,
    );

    /// A small hand-made population: households of 1 to 4 members with
    /// unequal weights, spread over the counties.
    fn seed_columns() -> (Vec<u32>, Vec<u8>, Vec<u32>, Vec<u16>) {
        let (mut weight, mut county, mut household, mut age) = (vec![], vec![], vec![], vec![]);
        for h in 0..600u32 {
            weight.push(700 + (h % 7) * 100);
            county.push(u8::try_from(h as usize % N_COUNTIES).unwrap());
            for m in 0..=(h % 4) {
                household.push(h);
                age.push(u16::try_from((h * 37 + m * 211) % (95 * 12)).unwrap());
            }
        }
        (weight, county, household, age)
    }

    #[test]
    fn world_from_a_population_keeps_its_people_and_runs() {
        let (weight, county, household, age) = seed_columns();
        let pop = SeedPopulation {
            hh_weight: &weight,
            hh_county: &county,
            household_id: &household,
            age_months: &age,
        };
        let mut w = ScaleWorld::from_population(1000, 42, &pop).unwrap();
        // Households, weights, counties and ages are the population's own.
        assert_eq!(w.n_households(), weight.len());
        assert_eq!(w.n_persons(), household.len());
        let real: u64 = household
            .iter()
            .map(|&h| u64::from(weight[h as usize]))
            .sum();
        assert_eq!(w.real_persons(), real);
        let c = w.to_columns();
        assert_eq!(c.hh_weight, weight);
        assert_eq!(c.household, household);
        for i in 0..household.len() {
            assert_eq!(u16::from(c.age[i]), age[i] / 12);
            assert_eq!(c.county[i], county[household[i] as usize]);
        }
        // It runs, with unequal weights, and money stays accounted for.
        let mut last = None;
        for _ in 0..6 {
            let r = w.step();
            assert!(r.ledger_ok && r.clearing_ok);
            last = Some(r);
        }
        let r = last.unwrap();
        assert!(r.employed > 0 && r.employed + r.unemployed < real);
        assert!(r.vat > Bani::ZERO);
        // Same inputs, same world; and it survives a save round trip.
        let again = ScaleWorld::from_population(1000, 42, &pop).unwrap();
        assert_ne!(again.state_hash(), w.state_hash());
        let mut again = again;
        for _ in 0..6 {
            again.step();
        }
        assert_eq!(again.state_hash(), w.state_hash());
        assert_eq!(
            ScaleWorld::from_columns(w.to_columns()).state_hash(),
            w.state_hash()
        );
    }

    #[test]
    fn bad_populations_are_rejected() {
        let (weight, county, household, age) = seed_columns();
        let pop = SeedPopulation {
            hh_weight: &weight,
            hh_county: &county,
            household_id: &household,
            age_months: &age,
        };
        let build = |p: &SeedPopulation<'_>| ScaleWorld::from_population(1000, 1, p).map(|_| ());
        assert_eq!(
            build(&SeedPopulation {
                age_months: &age[1..],
                ..pop
            }),
            Err(SeedError::ColumnLengths)
        );
        let mut zero = weight.clone();
        zero[5] = 0;
        assert_eq!(
            build(&SeedPopulation {
                hh_weight: &zero,
                ..pop
            }),
            Err(SeedError::ZeroWeight { household: 5 })
        );
        let mut far = county.clone();
        far[9] = u8::try_from(N_COUNTIES).unwrap();
        assert_eq!(
            build(&SeedPopulation {
                hh_county: &far,
                ..pop
            }),
            Err(SeedError::UnknownCounty { household: 9 })
        );
        let mut lost = household.clone();
        lost[3] = 600;
        assert_eq!(
            build(&SeedPopulation {
                household_id: &lost,
                ..pop
            }),
            Err(SeedError::UnknownHousehold { person: 3 })
        );
    }

    #[test]
    fn deterministic_at_fixed_scale() {
        let run = || {
            let mut w = ScaleWorld::generate(5000, 5);
            (0..3).map(|_| w.step()).collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    /// The daily tick (ADR-0017) does the same economy as the monthly one:
    /// run both for two years and compare the second year.
    #[test]
    fn daily_clock_agrees_with_the_monthly_tick_in_aggregate() {
        use econ_types::{Calendar, Date, Day};
        let mut monthly = ScaleWorld::generate(1000, 42);
        let (mut m_consumption, mut m_wages, mut m_employed) = (Bani::ZERO, Bani::ZERO, 0u64);
        for month in 0..24 {
            let r = monthly.step();
            assert!(r.ledger_ok && r.clearing_ok);
            if month >= 12 {
                m_consumption += r.consumption;
                m_wages += r.wage_bill;
                m_employed += r.employed;
            }
        }

        let mut daily = ScaleWorld::generate(1000, 42);
        let mut schedule = daily.day_schedule();
        // Every household has a day, and the days are about equally full.
        let per_day: Vec<usize> = schedule.households.iter().map(Vec::len).collect();
        assert_eq!(per_day.iter().sum::<usize>(), daily.n_households());
        let mean = daily.n_households() / usize::from(STAGGER_DAYS);
        assert!(
            per_day
                .iter()
                .all(|&n| n > mean * 3 / 4 && n < mean * 5 / 4)
        );
        assert_eq!(
            schedule.persons.iter().map(Vec::len).sum::<usize>(),
            daily.n_persons()
        );

        let cal = Calendar::new(Date::new(2021, 12, 1).unwrap());
        let (mut d_consumption, mut d_wages, mut d_employed) = (Bani::ZERO, Bani::ZERO, 0u64);
        let (mut months, mut day) = (0, Day(0));
        while months < 24 {
            let r = daily.step_day(&mut schedule, cal.date(day));
            assert!(r.ledger_ok, "ledger on day {}", day.0);
            if months >= 12 {
                d_consumption += r.consumption;
            }
            if let Some(close) = r.month {
                assert!(close.clearing_ok);
                if months >= 12 {
                    d_wages += close.wage_bill;
                    d_employed += close.employed;
                }
                months += 1;
            }
            day = day.next();
        }
        // Two years of days: December 2021 to November 2023.
        assert_eq!(day.0, 730);

        let close = |a: i64, b: i64, pct: i64| (a - b).abs() * 100 <= b * pct;
        assert!(
            close(d_wages.get(), m_wages.get(), 3),
            "wage bill: daily {d_wages:?}, monthly {m_wages:?}"
        );
        assert!(
            close(d_consumption.get(), m_consumption.get(), 3),
            "consumption: daily {d_consumption:?}, monthly {m_consumption:?}"
        );
        assert!(
            close(
                i64::try_from(d_employed).unwrap(),
                i64::try_from(m_employed).unwrap(),
                3
            ),
            "employment: daily {d_employed}, monthly {m_employed}"
        );
    }

    #[test]
    fn daily_clock_is_deterministic() {
        use econ_types::{Calendar, Date, Day};
        let cal = Calendar::new(Date::new(2021, 12, 1).unwrap());
        let run = || {
            let mut w = ScaleWorld::generate(1000, 42);
            let mut schedule = w.day_schedule();
            for d in 0..62 {
                w.step_day(&mut schedule, cal.date(Day(d)));
            }
            w.state_hash()
        };
        assert_eq!(run(), run());
    }
}
