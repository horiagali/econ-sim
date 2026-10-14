//! Labour market (spec `economy/labor-market`, "First version"): fill rate,
//! hires, layoffs, part-month pay and wage growth. Pure functions, no I/O; the
//! phases that walk today's units and the month-end pay live with the `World`.
//!
//! Counts are weighted persons, shares and rates millionths; the one power
//! goes through `econ_num::math` (ADR-0006).

use econ_num::math::powf;
use econ_types::Bani;

/// One, in millionths.
const ONE: u64 = 1_000_000;

/// `n / d` rounded half away from zero (`d > 0`).
fn rdiv(n: i128, d: i128) -> i128 {
    let (q, r) = (n / d, n % d);
    if 2 * r.abs() >= d { q + n.signum() } else { q }
}

/// Tuning parameters, in millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabourParams {
    /// `matching_efficiency` (mu).
    pub matching_efficiency_ppm: u32,
    /// `matching_elasticity` (alpha).
    pub matching_elasticity_ppm: u32,
    /// `max_layoff_share`: of staff, a month.
    pub max_layoff_share_ppm: u32,
    /// `quit_rate`: a month.
    pub quit_rate_ppm: u32,
    /// `wage_tightness_response` (beta 1).
    pub wage_tightness_response_ppm: u32,
}

/// Fill rate of a tier, `min(1, mu * (U / V)^alpha)`, in millionths; 1 when
/// the tier has no vacancies (AC-LM-01). The power goes through
/// `econ_num::math` (ADR-0006).
#[must_use]
pub fn fill_rate_ppm(tier_seekers: u64, tier_vacancies: u64, p: &LabourParams) -> u32 {
    if tier_vacancies == 0 {
        return 1_000_000;
    }
    if tier_seekers == 0 {
        return 0;
    }
    // Weighted persons of a tier are far below 2^53: exact in f64.
    #[allow(clippy::cast_precision_loss)]
    let ratio = tier_seekers as f64 / tier_vacancies as f64;
    let q = f64::from(p.matching_efficiency_ppm)
        * powf(ratio, f64::from(p.matching_elasticity_ppm) / 1e6);
    // q is between 0 and mu * ratio^alpha; capped at 1 before the cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let q = q.round().min(1e6) as u32;
    q
}

/// A unit's hires this month, `min(V_f, U_f, floor(q * V_f + carry))`, in
/// weighted persons. `carry_ppm` is the fraction of a hire carried to the
/// unit's next month, in millionths of a hire (AC-LM-01).
pub fn hires(
    unit_vacancies: u64,
    reachable_seekers: u64,
    fill_rate_ppm: u32,
    carry_ppm: &mut u32,
) -> u64 {
    let total = u128::from(fill_rate_ppm.min(1_000_000)) * u128::from(unit_vacancies)
        + u128::from(*carry_ppm % 1_000_000);
    let whole = u64::try_from(total / u128::from(ONE)).expect("hires fit u64");
    *carry_ppm = u32::try_from(total % u128::from(ONE)).expect("below a million");
    whole.min(unit_vacancies).min(reachable_seekers)
}

/// Layoffs: the excess of staff over desired employment, at most
/// `max_layoff_share` of staff (AC-LM-02).
#[must_use]
pub fn layoffs(staff: u64, desired: u64, p: &LabourParams) -> u64 {
    let at_most =
        u128::from(staff) * u128::from(p.max_layoff_share_ppm.min(1_000_000)) / u128::from(ONE);
    let at_most = u64::try_from(at_most).expect("at most the staff");
    staff.saturating_sub(desired).min(at_most)
}

/// Pay for the days worked in a month, rounded to whole bani (AC-LM-04).
///
/// # Panics
/// If `days_in_month` is zero.
pub fn part_month_pay(wage: Bani, days_worked: u8, days_in_month: u8) -> Bani {
    let days = days_worked.min(days_in_month);
    wage.mul_ratio(i64::from(days), i64::from(days_in_month))
}

/// Yearly wage growth of an industry group, `pi_e + beta_1 * (theta - theta*)`,
/// in millionths.
#[must_use]
pub fn wage_growth_ppm(
    expected_inflation_ppm: i32,
    tightness_ppm: u32,
    normal_ppm: u32,
    p: &LabourParams,
) -> i32 {
    let gap = i128::from(tightness_ppm) - i128::from(normal_ppm);
    let push = rdiv(
        i128::from(p.wage_tightness_response_ppm) * gap,
        i128::from(ONE),
    );
    i32::try_from(i128::from(expected_inflation_ppm) + push).unwrap_or(if push < 0 {
        i32::MIN
    } else {
        i32::MAX
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> LabourParams {
        LabourParams {
            matching_efficiency_ppm: 500_000,
            matching_elasticity_ppm: 500_000,
            max_layoff_share_ppm: 100_000,
            quit_rate_ppm: 10_000,
            wage_tightness_response_ppm: 100_000,
        }
    }

    #[test]
    fn half_a_month_is_half_the_wage() {
        // Hired on the 16th of a 30-day month: half the wage.
        assert_eq!(
            part_month_pay(Bani::from_lei(5_000), 15, 30),
            Bani::from_lei(2_500)
        );
        assert_eq!(part_month_pay(Bani(101), 1, 2), Bani(51));
        assert_eq!(part_month_pay(Bani(100), 40, 30), Bani(100));
    }

    #[test]
    fn wages_follow_expectations_and_tightness() {
        let p = params();
        // 3% expected, tightness 0.2 above normal: 3% + 0.1 * 0.2 = 5%.
        assert_eq!(wage_growth_ppm(30_000, 1_200_000, 1_000_000, &p), 50_000);
        assert_eq!(wage_growth_ppm(30_000, 800_000, 1_000_000, &p), 10_000);
        assert_eq!(wage_growth_ppm(25_000, 700_000, 700_000, &p), 25_000);
    }
}
