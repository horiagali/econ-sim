//! The wellbeing index (spec `society/social-outcomes`, "Wellbeing index",
//! first version; feature `wellbeing`): a human-development-style index of
//! health (life expectancy), education (mean and expected years of
//! schooling) and the living standard (SoL of `society/poverty-inequality`),
//! for the nation and each development region, at each monthly close.
//! Outputs only, read by migration's expected home quality (step 5b, feature
//! `home_quality`).
//!
//! The API is the spec's sketch, written with the acceptance tests before the
//! code (ADR-0014). Ratios are millionths, years and SoL points thousandths.

use econ_types::Month;

use crate::world::World;

/// The index and its three dimension indices, each in [0, 1], millionths.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Wellbeing {
    /// `(health · education · living)^(1/3)`.
    pub index_ppm: u32,
    /// `(e0 − 20) / (85 − 20)`, clamped.
    pub health_ppm: u32,
    /// `(MYS / 15 + EYS / 18) / 2`, clamped.
    pub education_ppm: u32,
    /// `(SoL − 10) / (60 − 10)`, clamped.
    pub living_ppm: u32,
}

/// The tuning parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WellbeingParams {
    /// `schooling_years` by `edu_level`, thousandths of a year: ISCED 0–2,
    /// 3, 4, 5–6, 7–8.
    pub schooling_years_milli: [u32; 5],
}

impl WellbeingParams {
    /// The spec's defaults: 8, 12, 13, 15.5 and 17.5 years.
    #[must_use]
    pub fn spec_defaults() -> WellbeingParams {
        WellbeingParams {
            schooling_years_milli: [8_000, 12_000, 13_000, 15_500, 17_500],
        }
    }
}

/// The index of a closed month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WellbeingStats {
    /// The month.
    pub month: Month,
    /// The nation's index.
    pub national: Wellbeing,
    /// Each development region's index, in the scenario's order.
    pub by_region: Vec<Wellbeing>,
    /// `life_expectancy` at birth, thousandths of a year (national).
    pub life_expectancy_milli: u32,
    /// `mean_years_schooling` of residents aged 25 and over, thousandths of
    /// a year: by region, the nation last.
    pub mean_years_milli: Vec<u32>,
    /// `expected_years_schooling`, thousandths of a year: by region, the
    /// nation last.
    pub expected_years_milli: Vec<u32>,
}

/// AC-WB-02: period life expectancy at birth, thousandths of a year, from a
/// mortality table (`[sex][age]`, F 0 and M 1, the yearly probability of
/// dying in parts per billion, ages 0 to 100, 100 the last age): a standard
/// period life table with deaths at mid-year, the two sexes weighted by
/// `men_share_ppm`. Everyone alive at 100 dies in its year.
#[must_use]
pub fn life_expectancy_milli(yearly_q_ppb: &[[u32; 101]; 2], men_share_ppm: u32) -> u32 {
    let e0 = |q: &[u32; 101]| -> f64 {
        let (mut alive, mut years) = (1.0f64, 0.0f64);
        for (a, &x) in q.iter().enumerate() {
            let q = if a == 100 {
                1.0
            } else {
                (f64::from(x) / 1e9).min(1.0)
            };
            let next = alive * (1.0 - q);
            years += (alive + next) / 2.0;
            alive = next;
        }
        years
    };
    let m = f64::from(men_share_ppm.min(1_000_000)) / 1e6;
    let e = e0(&yearly_q_ppb[0]) * (1.0 - m) + e0(&yearly_q_ppb[1]) * m;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let v = (e * 1_000.0).round() as u32;
    v
}

/// AC-WB-01: the index from its four inputs (life expectancy, mean and
/// expected years of schooling, all thousandths of a year; the mean SoL,
/// thousandths of a point), each dimension clamped to [0, 1], millionths
/// rounded to the nearest; the index is the geometric mean of the exact
/// dimensions.
#[must_use]
pub fn wellbeing_ppm(
    life_exp_milli: u32,
    mean_years_milli: u32,
    expected_years_milli: u32,
    sol_milli: u32,
) -> Wellbeing {
    let unit = |x: f64| x.clamp(0.0, 1.0);
    let f = |x: u32| f64::from(x) / 1_000.0;
    let health = unit((f(life_exp_milli) - 20.0) / 65.0);
    let education = unit((f(mean_years_milli) / 15.0 + f(expected_years_milli) / 18.0) / 2.0);
    let living = unit((f(sol_milli) - 10.0) / 50.0);
    let product = health * education * living;
    let index = if product > 0.0 {
        econ_num::math::powf(product, 1.0 / 3.0)
    } else {
        0.0
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let ppm = |x: f64| (x * 1e6).round() as u32;
    Wellbeing {
        index_ppm: ppm(index),
        health_ppm: ppm(health),
        education_ppm: ppm(education),
        living_ppm: ppm(living),
    }
}

impl World {
    /// The wellbeing index of every closed month, in order (empty when
    /// `WorldParams::wellbeing` is `None`).
    #[must_use]
    pub fn wellbeing_stats(&self) -> &[WellbeingStats] {
        self.wb()
    }
}
