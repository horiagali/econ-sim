//! The poverty and inequality indicators on the `World` (feature `poverty`;
//! spec `society/poverty-inequality`): computed at each monthly close from
//! the households' money income, outside the state hash (outputs only).

use econ_types::{Bani, Month};

use super::World;
use crate::poverty::{
    Distribution, IncomeMeasures, PovertyStats, class_of, eq_adults_milli, measures, recent_income,
};

/// The indicators' state: per household, aligned with the households table.
#[derive(Debug, Clone, Default)]
pub(crate) struct PovState {
    pub(crate) stats: Vec<PovertyStats>,
    pub(crate) anchor: Bani,
    anchor_milli: i128,
    pub(crate) recent: Vec<Bani>,
    pub(crate) year: Vec<Bani>,
    /// `income_to_date` at the last close: the month's income is its growth,
    /// less the month's imputed rent.
    to_date_prev: Vec<Bani>,
    /// Imputed rent received since the last close (left out, step 1).
    imputed: Vec<Bani>,
    /// The class and the persons counted of each household at the last close.
    class_prev: Vec<Option<u8>>,
    persons_prev: Vec<u64>,
    /// Each household's county, region and collective flag at load (new
    /// households take the People phase's, when it acts).
    county: Vec<u8>,
    region: Vec<u8>,
    collective: Vec<bool>,
    n_counties: usize,
    n_regions: usize,
}

/// `n / d` to the nearest, half away from zero (`d` > 0).
fn rdiv(n: i128, d: i128) -> i128 {
    if n >= 0 {
        (2 * n + d) / (2 * d)
    } else {
        -((-2 * n + d) / (2 * d))
    }
}

/// The measures of a group with the national median's line.
fn with_national_line(d: &Distribution, median_milli: i128, line_ppm: u32) -> IncomeMeasures {
    let mut m = measures(d, line_ppm);
    let total: i128 = d.weight.iter().map(|&v| i128::from(v)).sum();
    if total > 0 {
        let poor: i128 = d
            .income_milli
            .iter()
            .zip(d.weight)
            .filter(|&(&x, _)| x * 1_000_000 < i128::from(line_ppm) * median_milli)
            .map(|(_, &v)| i128::from(v))
            .sum();
        m.poverty_rate_ppm = u32::try_from(poor * 1_000_000 / total).expect("a share");
    }
    m
}

impl World {
    /// A household's county (from load, or the People phase's).
    #[cfg_attr(not(feature = "approval"), allow(dead_code))]
    pub(crate) fn pov_county(&self, h: usize) -> u8 {
        self.pov.county.get(h).copied().unwrap_or(0)
    }

    pub(crate) fn pov(&self) -> &PovState {
        &self.pov
    }

    /// Load: the households' counties, regions and collective flags.
    pub(super) fn start_poverty(&mut self, s: &super::StartState) {
        let n_c = s
            .hh_county
            .iter()
            .copied()
            .max()
            .map_or(0, |c| usize::from(c) + 1);
        let n_r = s
            .hh_region
            .iter()
            .copied()
            .max()
            .map_or(0, |r| usize::from(r) + 1);
        self.pov = PovState {
            county: s.hh_county.clone(),
            region: s.hh_region.clone(),
            collective: s.hh_collective.clone(),
            n_counties: n_c,
            n_regions: n_r,
            ..PovState::default()
        };
    }

    /// After November's pay on day 0: it is income received before December,
    /// so month 0's income is what `income_to_date` grows by from here.
    pub(super) fn pov_after_start_pay(&mut self) {
        self.pov
            .to_date_prev
            .clone_from(&self.households.income_to_date);
    }

    /// Household `h` moved (migration's moves between counties).
    #[cfg(feature = "migration")]
    pub(crate) fn pov_set_place(&mut self, h: usize, county: u8, region: u8) {
        self.pov_grow();
        self.pov.county[h] = county;
        self.pov.region[h] = region;
    }

    /// Imputed rent paid to household `h` (per household).
    pub(crate) fn pov_imputed_rent(&mut self, h: usize, x: Bani) {
        if self.pov.imputed.len() <= h {
            self.pov.imputed.resize(h + 1, Bani::ZERO);
        }
        self.pov.imputed[h] += x;
    }

    /// The yearly compaction of the People phase: keep the households kept.
    #[cfg(feature = "demography")]
    pub(super) fn pov_compact(&mut self, keep_h: &[bool]) {
        use super::money::retain_by;
        let p = &mut self.pov;
        retain_by(&mut p.recent, keep_h);
        retain_by(&mut p.year, keep_h);
        retain_by(&mut p.to_date_prev, keep_h);
        retain_by(&mut p.imputed, keep_h);
        retain_by(&mut p.class_prev, keep_h);
        retain_by(&mut p.persons_prev, keep_h);
        retain_by(&mut p.county, keep_h);
        retain_by(&mut p.region, keep_h);
        retain_by(&mut p.collective, keep_h);
    }

    /// Grow the per-household columns to the households table (new records).
    fn pov_grow(&mut self) {
        let n = self.households.hh_weight.len();
        while self.pov.county.len() < n {
            let h = self.pov.county.len();
            #[cfg(feature = "demography")]
            let (c, coll) = (
                self.dem.hh_life.hh_county.get(h).copied().unwrap_or(0),
                self.dem
                    .hh_life
                    .hh_collective
                    .get(h)
                    .copied()
                    .unwrap_or(false),
            );
            #[cfg(not(feature = "demography"))]
            let (c, coll) = (0, false);
            let r = (0..self.persons.household_id.len())
                .find(|&p| self.persons.household_id[p] as usize == h)
                .map_or(0, |p| self.pp.region[p]);
            self.pov.county.push(c);
            self.pov.region.push(r);
            self.pov.collective.push(coll);
        }
        self.pov.recent.resize(n, Bani::ZERO);
        self.pov.year.resize(n, Bani::ZERO);
        self.pov.to_date_prev.resize(n, Bani::ZERO);
        self.pov.imputed.resize(n, Bani::ZERO);
        self.pov.class_prev.resize(n, None);
        self.pov.persons_prev.resize(n, 0);
    }

    /// The monthly close: the month's indicators (steps 1 to 8).
    #[allow(clippy::too_many_lines)]
    pub(super) fn poverty_close(&mut self) {
        let Some(p) = self.params.poverty else {
            return;
        };
        self.pov_grow();
        let n_hh = self.households.hh_weight.len();
        let t = self.months.len() - 1;
        let cpi = self.months.last().map_or(1_000_000, |m| m.cpi_ppm);
        // Members (live persons) and their ages.
        let mut ages: Vec<Vec<u16>> = vec![Vec::new(); n_hh];
        for q in 0..self.persons.household_id.len() {
            if self.person_resident(q) {
                ages[self.persons.household_id[q] as usize].push(self.pp.age[q]);
            }
        }
        let counted = |w: &World, h: usize| {
            !ages[h].is_empty() && !w.pov.collective[h] && w.household_alive(h)
        };
        let mut x = vec![0i128; n_hh];
        let mut v = vec![0u64; n_hh];
        for h in 0..n_hh {
            // The month's money income: what `income_to_date` grew by, less
            // imputed rent (step 1: money received, as the survey's).
            let y = self.households.income_to_date[h]
                - self.pov.to_date_prev[h]
                - std::mem::replace(&mut self.pov.imputed[h], Bani::ZERO);
            self.pov.to_date_prev[h] = self.households.income_to_date[h];
            // Step 2: recent real income (a new household starts at its
            // first month's).
            let first =
                t == 0 || self.pov.class_prev[h].is_none() && self.pov.recent[h] == Bani::ZERO;
            self.pov.recent[h] = if first {
                recent_income(Bani::ZERO, y, cpi, 1_000_000)
            } else {
                recent_income(self.pov.recent[h], y, cpi, p.income_recent_speed_ppm)
            };
            self.pov.year[h] += y;
            if counted(self, h) {
                let e = i128::from(eq_adults_milli(&ages[h]).max(1));
                x[h] = rdiv(i128::from(self.pov.recent[h].get()) * 1_000_000, e);
                v[h] = u64::from(self.households.hh_weight[h])
                    * u64::try_from(ages[h].len()).expect("members");
            }
        }
        let d = Distribution {
            income_milli: &x,
            weight: &v,
        };
        let national = measures(&d, p.poverty_line_ppm);
        if t == 0 {
            self.pov.anchor_milli = national.median_milli;
            self.pov.anchor =
                Bani(i64::try_from(national.median_milli / 1_000).expect("an amount"));
        }
        let pop = i128::from(national.population.max(1));
        // Hardship.
        let hard: i128 = (0..n_hh)
            .filter(|&h| v[h] > 0 && self.households.hh_hardship[h])
            .map(|h| i128::from(v[h]))
            .sum();
        // Step 6 and 7: classes and their flows.
        let mut s = PovertyStats {
            month: Month(u32::try_from(t).expect("a month")),
            national,
            hardship_rate_ppm: u32::try_from(hard * 1_000_000 / pop).expect("a share"),
            class_share_ppm: [0; 4],
            class_persons: [0; 4],
            class_entered: [0; 4],
            class_left: [0; 4],
            class_added: [0; 4],
            class_removed: [0; 4],
            by_county: Vec::new(),
            by_region: Vec::new(),
            by_age: [national; 3],
            annual: None,
        };
        for h in 0..n_hh {
            let now = (v[h] > 0).then(|| class_of(x[h], self.pov.anchor_milli, p.class_lines_ppm));
            let prev = self.pov.class_prev[h];
            let (vp, vn) = (self.pov.persons_prev[h], v[h]);
            match (prev, now) {
                (None, Some(k)) => s.class_added[usize::from(k)] += vn,
                (Some(k), None) => s.class_removed[usize::from(k)] += vp,
                (Some(a), Some(b)) if a == b => {
                    if vn > vp {
                        s.class_added[usize::from(b)] += vn - vp;
                    } else {
                        s.class_removed[usize::from(b)] += vp - vn;
                    }
                }
                (Some(a), Some(b)) => {
                    s.class_left[usize::from(a)] += vp;
                    s.class_entered[usize::from(b)] += vn;
                }
                (None, None) => {}
            }
            if let Some(k) = now {
                s.class_persons[usize::from(k)] += vn;
            }
            self.pov.class_prev[h] = now;
            self.pov.persons_prev[h] = vn;
        }
        for k in 0..4 {
            s.class_share_ppm[k] =
                u32::try_from(i128::from(s.class_persons[k]) * 1_000_000 / pop).expect("a share");
        }
        // Groups, with the national median's line.
        let med = national.median_milli;
        let group = |key: &dyn Fn(usize) -> bool| -> IncomeMeasures {
            let vv: Vec<u64> = (0..n_hh).map(|h| if key(h) { v[h] } else { 0 }).collect();
            with_national_line(
                &Distribution {
                    income_milli: &x,
                    weight: &vv,
                },
                med,
                p.poverty_line_ppm,
            )
        };
        s.by_county = (0..self.pov.n_counties)
            .map(|c| group(&|h| usize::from(self.pov.county[h]) == c))
            .collect();
        s.by_region = (0..self.pov.n_regions)
            .map(|r| group(&|h| usize::from(self.pov.region[h]) == r))
            .collect();
        // By age: each person with the household's income and weight.
        for (g, (lo, hi)) in [(0u16, 18u16), (18, 65), (65, u16::MAX)]
            .into_iter()
            .enumerate()
        {
            let mut xs = Vec::new();
            let mut ws = Vec::new();
            for h in 0..n_hh {
                if v[h] == 0 {
                    continue;
                }
                let n = ages[h]
                    .iter()
                    .filter(|&&a| a / 12 >= lo && a / 12 < hi)
                    .count();
                if n > 0 {
                    xs.push(x[h]);
                    ws.push(u64::from(self.households.hh_weight[h]) * u64::try_from(n).expect("n"));
                }
            }
            s.by_age[g] = with_national_line(
                &Distribution {
                    income_milli: &xs,
                    weight: &ws,
                },
                med,
                p.poverty_line_ppm,
            );
        }
        // Step 8: the calendar year, at December closes.
        if self.calendar.date(self.today).month == 12 {
            let ya: Vec<i128> = (0..n_hh)
                .map(|h| {
                    if v[h] == 0 {
                        return 0;
                    }
                    // The first December has one month: × 12.
                    let year = if t == 0 {
                        i128::from(self.pov.year[h].get()) * 12
                    } else {
                        i128::from(self.pov.year[h].get())
                    };
                    rdiv(
                        year * 1_000_000,
                        i128::from(eq_adults_milli(&ages[h]).max(1)),
                    )
                })
                .collect();
            s.annual = Some(measures(
                &Distribution {
                    income_milli: &ya,
                    weight: &v,
                },
                p.poverty_line_ppm,
            ));
            self.pov.year.iter_mut().for_each(|y| *y = Bani::ZERO);
        }
        self.pov.stats.push(s);
    }
}
