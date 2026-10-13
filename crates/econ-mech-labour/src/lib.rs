//! Labour market (spec `economy/labor-market`, "First version"): fill rate,
//! hires, layoffs, part-month pay and wage growth. Pure functions, no I/O; the
//! phases that walk today's units and the month-end pay live with the `World`.
//!
//! API skeleton from the spec's API sketch, written before the code
//! (ADR-0014). The bodies come with the implementation change.

use econ_types::Bani;

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
    let _ = (tier_seekers, tier_vacancies, p);
    unimplemented!("econ-mech-labour: written in the implementation change")
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
    let _ = (unit_vacancies, reachable_seekers, fill_rate_ppm, carry_ppm);
    unimplemented!("econ-mech-labour: written in the implementation change")
}

/// Layoffs: the excess of staff over desired employment, at most
/// `max_layoff_share` of staff (AC-LM-02).
#[must_use]
pub fn layoffs(staff: u64, desired: u64, p: &LabourParams) -> u64 {
    let _ = (staff, desired, p);
    unimplemented!("econ-mech-labour: written in the implementation change")
}

/// Pay for the days worked in a month, rounded to whole bani (AC-LM-04).
pub fn part_month_pay(wage: Bani, days_worked: u8, days_in_month: u8) -> Bani {
    let _ = (wage, days_worked, days_in_month);
    unimplemented!("econ-mech-labour: written in the implementation change")
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
    let _ = (expected_inflation_ppm, tightness_ppm, normal_ppm, p);
    unimplemented!("econ-mech-labour: written in the implementation change")
}
