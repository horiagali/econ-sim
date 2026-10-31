//! The wellbeing index on the `World` (feature `wellbeing`; spec
//! `society/social-outcomes`): computed at each monthly close after the
//! poverty indicators, outside the state hash (outputs only).

use econ_types::Month;

use super::World;
use crate::wellbeing::{WellbeingStats, life_expectancy_milli, wellbeing_ppm};

impl World {
    pub(crate) fn wb(&self) -> &[WellbeingStats] {
        &self.wb
    }

    fn wb_push(&mut self, s: WellbeingStats) {
        self.wb.push(s);
    }

    /// The monthly close, after the poverty indicators (it reads SoL).
    #[allow(clippy::needless_range_loop)]
    pub(super) fn wellbeing_close(&mut self) {
        let Some(p) = self.params.wellbeing else {
            return;
        };
        let Some(sol) = self.poverty_stats().last().map(|s| s.sol.clone()) else {
            return;
        };
        let n_r = usize::from(self.pp.region.iter().copied().max().unwrap_or(0)) + 1;
        // Residents: men, adults' schooling, and enrolment by age, by region
        // (the nation last).
        let mut men = (0u64, 0u64);
        let mut years = vec![(0u128, 0u64); n_r + 1];
        let mut by_age = vec![[(0u64, 0u64); 30]; n_r + 1];
        for q in 0..self.pp.age.len() {
            if !self.person_resident(q) {
                continue;
            }
            let w = u64::from(self.households.hh_weight[self.persons.household_id[q] as usize]);
            if w == 0 {
                continue;
            }
            let r = usize::from(self.pp.region[q]);
            men.1 += w;
            if self.people.sex.get(q) == Some(&econ_popgen::Sex::M) {
                men.0 += w;
            }
            let a = usize::from(self.pp.age[q] / 12);
            let enrolled = self
                .people
                .enrolled
                .get(q)
                .copied()
                .unwrap_or(econ_popgen::Enrolled::None)
                != econ_popgen::Enrolled::None;
            for g in [r, n_r] {
                if a >= 25 {
                    years[g].0 += u128::from(w)
                        * u128::from(p.schooling_years_milli[self.persons.edu_level[q] as usize]);
                    years[g].1 += w;
                }
                if (5..=29).contains(&a) {
                    by_age[g][a].1 += w;
                    if enrolled {
                        by_age[g][a].0 += w;
                    }
                }
            }
        }
        let mean_years: Vec<u32> = years
            .iter()
            .map(|&(y, n)| {
                if n == 0 {
                    0
                } else {
                    u32::try_from((y + u128::from(n) / 2) / u128::from(n)).expect("years")
                }
            })
            .collect();
        let expected_years: Vec<u32> = by_age
            .iter()
            .map(|ages| {
                #[allow(clippy::cast_precision_loss)]
                let e: f64 = (5..=29)
                    .filter(|&a| ages[a].1 > 0)
                    .map(|a| ages[a].0 as f64 / ages[a].1 as f64)
                    .sum();
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let v = (e * 1_000.0).round() as u32;
                v
            })
            .collect();
        // Life expectancy: the year's mortality table, ages 0 to 100.
        let men_ppm = u32::try_from(u128::from(men.0) * 1_000_000 / u128::from(men.1.max(1)))
            .expect("a share");
        let life = self
            .wb_table()
            .map_or(0, |t| life_expectancy_milli(&t, men_ppm));
        let by_region = (0..n_r)
            .map(|r| {
                wellbeing_ppm(
                    life,
                    mean_years[r],
                    expected_years[r],
                    sol.by_region.get(r).copied().unwrap_or(sol.national_milli),
                )
            })
            .collect();
        let t = u32::try_from(self.months().len() - 1).expect("a month");
        self.wb_push(WellbeingStats {
            month: Month(t),
            national: wellbeing_ppm(
                life,
                mean_years[n_r],
                expected_years[n_r],
                sol.national_milli,
            ),
            by_region,
            life_expectancy_milli: life,
            mean_years_milli: mean_years,
            expected_years_milli: expected_years,
        });
    }

    /// The year's yearly mortality probabilities, ages 0 to 100.
    fn wb_table(&self) -> Option<[[u32; 101]; 2]> {
        #[cfg(feature = "demography")]
        let full = self
            .dem_mortality_yearly()
            .or_else(|| self.people.demography.as_ref().map(|d| d.mortality.yearly))?;
        #[cfg(not(feature = "demography"))]
        let full = self.people.demography.as_ref()?.mortality.yearly;
        let mut t = [[0u32; 101]; 2];
        for s in 0..2 {
            t[s].copy_from_slice(&full[s][..101]);
        }
        Some(t)
    }
}
