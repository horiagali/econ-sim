//! Moves between counties at the monthly close (spec
//! `economy/migration-remittances`, step 6; feature `migration`).
//!
//! Each resident household moves with `m0 · (1 + β u_r) / (1 + β u_r0)`,
//! aligned by county on whole records (the first version's `m0` has no age
//! profile, so cells by age band would only split the alignment), and goes to a destination by the logit shares of
//! `V_cd = α_d + β_w ln(W_d / W_c) − β_u (u_d − u_c) − β_t t_cd`, allocated
//! as whole records with a carry per (origin, destination): each record to
//! the destination most owed, so that each pair's moves follow its expected
//! share over months (at 1:100 a keyed draw per record left a county's
//! yearly balance within noise of a few records). `m0` is fitted at load so that a year's moves are 2021's changes of
//! domicile times `between_county_share`; `α` is fitted at load, by
//! iteration on the starting population, so that each county's expected net
//! balance is its 2021 balance (the spec's Python fit, done at load as `p0`
//! and `q0` are). `W` is the county's mean gross wage of the persons with a
//! job (the same measure at load and in the run); `t` the travel time between
//! county seats (straight-line distance × 1.3 at 70 km/h, until the transport
//! network of M11).

use econ_mech_migration::{Pick, destination_ppm, take_whole_records};
use econ_popgen::NO_EMPLOYER;
use econ_rng::Stream;
use econ_types::Bani;

use super::World;

/// The keyed draws' purposes (stream `Migration`).
const PURPOSE_MOVE: u32 = 20;

/// What the moves of a month did: persons in and out of each county.
pub(super) struct Moves {
    pub(super) moves_in: Vec<u64>,
    pub(super) moves_out: Vec<u64>,
}

/// Distance in km between two points in thousandths of a degree (the
/// equirectangular approximation, within a fraction of a percent at
/// Romania's scale).
fn distance_km(lat1: i32, lon1: i32, lat2: i32, lon2: i32) -> f64 {
    let r = |x: i32| f64::from(x) / 1000.0 * std::f64::consts::PI / 180.0;
    let mid = (r(lat1) + r(lat2)) / 2.0;
    let x = (r(lon2) - r(lon1)) * econ_num::math::cos(mid);
    let y = r(lat2) - r(lat1);
    6371.0 * econ_num::math::sqrt(x * x + y * y)
}

impl World {
    /// Each county's region (from its households' members) and resident
    /// persons and households, weighted.
    fn moves_counties(&self, n_c: usize) -> (Vec<u8>, Vec<u64>) {
        let mut region = vec![0u8; n_c];
        let mut persons = vec![0u64; n_c];
        for p in 0..self.pp.age.len() {
            if !self.person_resident(p) {
                continue;
            }
            let h = self.persons.household_id[p] as usize;
            let c = usize::from(self.dem.hh_life.hh_county[h]);
            if c < n_c {
                region[c] = self.pp.region[p];
                persons[c] += u64::from(self.households.hh_weight[h]);
            }
        }
        (region, persons)
    }

    /// Each county's mean gross wage of the persons with a job, lei (the
    /// national mean for a county with none).
    fn moves_wages(&self, n_c: usize) -> Vec<f64> {
        let (mut sum, mut wt) = (vec![0f64; n_c], vec![0f64; n_c]);
        for p in 0..self.pp.age.len() {
            if self.persons.employer[p] == NO_EMPLOYER || !self.person_resident(p) {
                continue;
            }
            let h = self.persons.household_id[p] as usize;
            let c = usize::from(self.dem.hh_life.hh_county[h]);
            if c < n_c {
                let w = f64::from(self.households.hh_weight[h]);
                #[allow(clippy::cast_precision_loss)]
                {
                    sum[c] += w * self.persons.wage[p].get() as f64;
                }
                wt[c] += w;
            }
        }
        let national = sum.iter().sum::<f64>() / wt.iter().sum::<f64>().max(1.0);
        (0..n_c)
            .map(|c| {
                if wt[c] > 0.0 {
                    sum[c] / wt[c]
                } else {
                    national.max(1.0)
                }
            })
            .collect()
    }

    /// The region unemployment rates, millionths, measured now (no shock).
    fn moves_unemployment_measured(&self) -> Vec<u32> {
        let n_r = usize::from(self.pp.region.iter().copied().max().unwrap_or(0)) + 1;
        let (mut un, mut lf) = (vec![0u64; n_r], vec![0u64; n_r]);
        for p in 0..self.pp.age.len() {
            if !self.person_resident(p) {
                continue;
            }
            let w = u64::from(self.households.hh_weight[self.persons.household_id[p] as usize]);
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
        (0..n_r)
            .map(|r| u32::try_from(un[r] * 1_000_000 / lf[r].max(1)).expect("a rate"))
            .collect()
    }

    /// The utility of each destination from county `c`, millionths (the
    /// origin's own entry is unused).
    fn moves_utility(&self, c: usize, wages: &[f64], u_county: &[f64]) -> Vec<i64> {
        let q = self.params.migration;
        let n_c = wages.len();
        let (bw, bu, bt) = (
            f64::from(q.move_wage_ppm) / 1e6,
            f64::from(q.move_unemployment_dest_ppm) / 1e6,
            f64::from(q.move_time_ppm) / 1e6,
        );
        (0..n_c)
            .map(|d| {
                let v = self.mig.alpha.get(d).copied().unwrap_or(0.0)
                    + bw * econ_num::math::ln(wages[d] / wages[c])
                    - bu * (u_county[d] - u_county[c])
                    - bt * self.mig.travel_hours[c * n_c + d];
                #[allow(clippy::cast_possible_truncation)]
                let x = (v * 1e6).round() as i64;
                x
            })
            .collect()
    }

    /// Load: the travel times, `m0` and `α` (after the links are folded).
    #[allow(clippy::needless_range_loop)]
    pub(super) fn mig_fit_moves(&mut self) {
        let Some(cm) = self.people.county_moves.clone() else {
            return;
        };
        let q = self.params.migration;
        let n_c = cm.net_2021.len();
        self.mig.travel_hours = (0..n_c * n_c)
            .map(|k| {
                let (a, b) = (k / n_c, k % n_c);
                distance_km(
                    cm.seat_lat_milli[a],
                    cm.seat_lon_milli[a],
                    cm.seat_lat_milli[b],
                    cm.seat_lon_milli[b],
                ) * f64::from(q.road_factor_ppm)
                    / 1e6
                    / f64::from(q.road_speed_kmh)
            })
            .collect();
        let (region, persons) = self.moves_counties(n_c);
        self.mig.county_region = region.clone();
        // m0: a year's moves are 2021's changes of domicile times the share.
        let total: u64 = persons.iter().sum();
        #[allow(clippy::cast_precision_loss)]
        let target = cm.settlements_2021 as f64 * f64::from(cm.between_county_share_ppm) / 1e6;
        #[allow(clippy::cast_precision_loss)]
        let m0 = target / 12.0 / (total.max(1) as f64);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            self.mig.m0_ppb = (m0 * 1e9).round() as u32;
        }
        // α: each county's expected inflow less outflow is its 2021 balance.
        let wages = self.moves_wages(n_c);
        let ur = self.moves_unemployment_measured();
        let u_c: Vec<f64> = (0..n_c)
            .map(|c| f64::from(ur.get(usize::from(region[c])).copied().unwrap_or(0)) / 1e6)
            .collect();
        #[allow(clippy::cast_precision_loss)]
        let out: Vec<f64> = persons.iter().map(|&p| p as f64 * m0 * 12.0).collect();
        #[allow(clippy::cast_precision_loss)]
        let want: Vec<f64> = (0..n_c)
            .map(|d| (out[d] + cm.net_2021[d] as f64).max(out[d] * 0.05).max(1.0))
            .collect();
        self.mig.alpha = vec![0.0; n_c];
        for _ in 0..400 {
            let mut inflow = vec![0f64; n_c];
            for c in 0..n_c {
                let v = self.moves_utility(c, &wages, &u_c);
                let shares = destination_ppm(u8::try_from(c).expect("a county"), &v);
                for d in 0..n_c {
                    inflow[d] += out[c] * f64::from(shares[d]) / 1e6;
                }
            }
            for d in 0..n_c {
                self.mig.alpha[d] += econ_num::math::ln(want[d] / inflow[d].max(1e-9));
            }
            #[allow(clippy::cast_precision_loss)]
            let mean = self.mig.alpha.iter().sum::<f64>() / n_c as f64;
            self.mig.alpha.iter_mut().for_each(|a| *a -= mean);
        }
    }

    /// Step 6: the month's moves between counties.
    #[allow(clippy::needless_range_loop, clippy::too_many_lines)]
    pub(super) fn mig_moves(&mut self, n_c: usize) -> Moves {
        let mut m = Moves {
            moves_in: vec![0; n_c],
            moves_out: vec![0; n_c],
        };
        if self.mig.alpha.len() != n_c || n_c < 2 {
            return m;
        }
        let q = self.params.migration;
        let (u, u0) = self.mig_unemployment();
        let u_c: Vec<f64> = (0..n_c)
            .map(|c| {
                f64::from(
                    u.get(usize::from(self.mig.county_region[c]))
                        .copied()
                        .unwrap_or(0),
                ) / 1e6
            })
            .collect();
        let wages = self.moves_wages(n_c);
        // The households: members, oldest working-age member's band.
        let n_hh = self.households.hh_weight.len();
        let mut members = vec![0u64; n_hh];
        for p in 0..self.pp.age.len() {
            if !self.person_resident(p) {
                continue;
            }
            let h = self.persons.household_id[p] as usize;
            members[h] += 1;
        }
        let beta = f64::from(q.move_unemployment_ppm) / 1e6;
        let day = self.today.0;
        let mut cells: Vec<Vec<Pick>> = vec![Vec::new(); n_c];
        let mut expected = vec![0i128; n_c];
        for h in 0..n_hh.min(self.mig.hh_before) {
            let w = self.households.hh_weight[h];
            let c = usize::from(self.dem.hh_life.hh_county[h]);
            if members[h] == 0
                || w == 0
                || c >= n_c
                || !self.household_alive(h)
                || self.dem.hh_life.hh_collective[h]
            {
                continue;
            }
            let r = usize::from(self.mig.county_region[c]);
            let push = (1.0 + beta * f64::from(u.get(r).copied().unwrap_or(0)) / 1e6)
                / (1.0 + beta * f64::from(u0.get(r).copied().unwrap_or(0)) / 1e6);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let prob = (f64::from(self.mig.m0_ppb) * push).round().min(1e9) as u64;
            if prob == 0 {
                continue;
            }
            let k = c;
            expected[k] += i128::from(w) * i128::from(prob);
            let draw =
                self.seed
                    .fast_u64(Stream::Migration, day, self.people.hh_key[h], PURPOSE_MOVE);
            cells[k].push(Pick {
                record: u32::try_from(h).expect("a household"),
                weight: w,
                priority: u64::try_from(
                    (u128::from(draw >> 20) * 1_000_000_000 / u128::from(prob))
                        .min(u128::from(u64::MAX)),
                )
                .expect("a priority"),
            });
        }
        if self.mig.move_carry.len() != n_c {
            self.mig.move_carry = vec![0; n_c];
        }
        for k in 0..n_c {
            let target =
                i64::try_from((expected[k] + 500_000_000) / 1_000_000_000).expect("a target");
            let mut carry = self.mig.move_carry[k];
            let chosen = take_whole_records(target, &mut carry, &mut cells[k]);
            self.mig.move_carry[k] = carry;
            let c = k;
            if chosen.is_empty() {
                continue;
            }
            let shares = destination_ppm(
                u8::try_from(c).expect("a county"),
                &self.moves_utility(c, &wages, &u_c),
            );
            if self.mig.dest_carry.len() != n_c * n_c {
                self.mig.dest_carry = vec![0; n_c * n_c];
            }
            for i in chosen {
                let h = i as usize;
                // Whole records to the destination most owed: each pair's
                // moves follow its expected share exactly over months.
                let w = i64::from(self.households.hh_weight[h]);
                let row = &mut self.mig.dest_carry[c * n_c..(c + 1) * n_c];
                for (x, &sh) in row.iter_mut().zip(&shares) {
                    *x += w * i64::from(sh);
                }
                let d = (0..n_c)
                    .filter(|&d| d != c)
                    .max_by_key(|&d| (row[d], std::cmp::Reverse(d)))
                    .expect("another county");
                row[d] -= w * 1_000_000;
                let persons = u64::from(self.households.hh_weight[h]) * members[h];
                m.moves_out[c] += persons;
                m.moves_in[d] += persons;
                self.mig_move_household(h, d);
            }
        }
        m
    }

    /// Household `h` moves to county `d`: its county and region change;
    /// employees quit and look for work if the region changes.
    fn mig_move_household(&mut self, h: usize, d: usize) {
        let to_region = self.mig.county_region[d];
        self.dem.hh_life.hh_county[h] = u8::try_from(d).expect("a county");
        #[cfg(feature = "poverty")]
        self.pov_set_place(h, self.dem.hh_life.hh_county[h], to_region);
        for p in 0..self.pp.age.len() {
            if self.persons.household_id[p] as usize != h || !self.dem.life.alive[p] {
                continue;
            }
            let from_region = self.pp.region[p];
            self.pp.region[p] = to_region;
            let u = self.persons.employer[p];
            if from_region != to_region && u != NO_EMPLOYER && self.person_resident(p) {
                let i = u32::try_from(p).expect("a person");
                let w = self.person_weight(i);
                let k = usize::from(u);
                self.units.staff[k] -= w;
                if let Ok(pos) = self.uu.members[k].binary_search(&i) {
                    self.uu.members[k].remove(pos);
                }
                self.persons.employer[p] = NO_EMPLOYER;
                self.persons.wage[p] = Bani::ZERO;
                self.persons.wage_declared[p] = Bani::ZERO;
                self.persons.hired_on[p] = None;
                self.persons.months_unemployed[p] = 0;
                self.pp.status[p] = 0;
                self.pp.seeking[p] = true;
            }
        }
    }
}
