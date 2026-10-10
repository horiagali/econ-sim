//! Spike 4: performance and market clearing at variable population scale.
//!
//! **Throwaway-quality spike code** (see docs/03-architecture/spikes/). It
//! builds a fake weighted population of Romania at any `sample_scale`, then
//! runs a monthly tick with the expensive parts of the real design:
//!
//! 1. job separations (keyed RNG per person),
//! 2. labour matching per (region × skill) with alignment-style selection,
//! 3. wages via per-industry wage clearing accounts (integer bani, weighted),
//! 4. household consumption over 83 goods,
//! 5. a 90×90 Leontief solve for gross output,
//! 6. an aggregation cube (county × activity × age band × education).
//!
//! Nothing here hardcodes the scale: record counts and weights come from
//! `sample_scale` and the (fake) totals.

use econ_num::lu::leontief_output;
use econ_rng::{KeyedRng, Stream};
use econ_types::{Bani, split_largest_remainder};

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
    /// Weighted household consumption.
    pub consumption: Bani,
    /// Sum of gross output (float, for the IO solve check).
    pub gross_output: f64,
    /// Number of non-empty cube cells.
    pub cube_cells: usize,
    /// Every wage clearing account netted to zero (I-8).
    pub clearing_ok: bool,
}

fn region_of(county: u8) -> usize {
    usize::from(county) % N_REGIONS
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
        // Productive IO matrix and consumption shares.
        let mut io = vec![0.0; N_INDUSTRIES * N_INDUSTRIES];
        for (k, v) in io.iter_mut().enumerate() {
            *v = rng.draw(Stream::PopulationGen, 1, k as u64).uniform() * 0.5 / N_INDUSTRIES as f64;
        }
        w.io = io;
        let raw: Vec<f64> = (0..N_GOODS)
            .map(|g| 0.5 + rng.draw(Stream::PopulationGen, 2, g as u64).uniform())
            .collect();
        let tot: f64 = raw.iter().sum();
        w.consumption_shares = raw.iter().map(|x| x / tot).collect();
        w
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
            io: self.io.clone(),
            consumption_shares: self.consumption_shares.clone(),
        }
    }

    /// Rebuild from columns.
    #[must_use]
    pub fn from_columns(c: ScaleColumns) -> Self {
        ScaleWorld {
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
            io: c.io,
            consumption_shares: c.consumption_shares,
            seed_value: c.seed,
            seed: KeyedRng::new(c.seed),
            tick: c.tick,
            separation_rate: c.separation_rate,
            params: c.params,
        }
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
        // 4. Consumption: 85% of income + 0.5% of deposits, split over goods.
        let mut demand = vec![Bani::ZERO; N_GOODS];
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
        for h in 0..self.n_households() {
            let c = self.hh_income[h].mul_rate(self.params.mpc_income)
                + self.hh_deposits[h].mul_rate(self.params.mpc_wealth);
            let c = if c > self.hh_deposits[h] + self.hh_income[h] {
                self.hh_deposits[h] + self.hh_income[h]
            } else {
                c
            };
            self.hh_deposits[h] = self.hh_deposits[h] + self.hh_income[h] - c;
            consumption += c.times(i64::from(self.hh_weight[h]));
        }
        // Spike simplification: one basket for everyone, so split the total once
        // (the real model splits per basket class, a handful of splits per tick).
        for (g, part) in split_largest_remainder(consumption, &weights_ppm)
            .into_iter()
            .enumerate()
        {
            demand[g] += part;
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
            gross_output,
            cube_cells: cube.iter().filter(|&&c| c > 0).count(),
            clearing_ok,
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
            let pop = REAL_POPULATION as f64;
            let u_rate = r.unemployed as f64 / (r.employed + r.unemployed) as f64;
            results.push((scale, r.wage_bill.to_f64_exact() / pop, u_rate));
        }
        let (_, w0, u0) = results[2];
        for &(scale, w, u) in &results {
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

    #[test]
    fn deterministic_at_fixed_scale() {
        let run = || {
            let mut w = ScaleWorld::generate(5000, 5);
            (0..3).map(|_| w.step()).collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }
}
