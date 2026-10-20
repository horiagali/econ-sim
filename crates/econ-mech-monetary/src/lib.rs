//! Monetary policy (spec `economy/monetary-policy`, "First version", roadmap
//! M3): the central bank's credibility and the advisory (Taylor-type) rule.
//! Pure functions, no I/O; the month-end interest on reserves and advances,
//! the profit remitted to the government and the close live with the
//! `World`.
//!
//! Rates, inflation and shares are millionths (a year for rates and
//! inflation). API from the spec's API sketch, written before the code
//! (ADR-0014).
//!
//! Each function works in exact `i128` integer arithmetic and rounds once, at
//! the end, half away from zero (the repo's rule), so results are monotone in
//! every input and need no floats.

/// One million: the millionths scale.
const PPM: i128 = 1_000_000;

/// `num / den` rounded half away from zero; `den > 0`.
fn div_round(num: i128, den: i128) -> i128 {
    debug_assert!(den > 0);
    let half = den / 2;
    if num >= 0 {
        (num + half) / den
    } else {
        -((-num + half) / den)
    }
}

/// The credibility update's parameters, millionths: `credibility_speed`
/// λ_c (a month) and the weights k1 to k4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredibilityParams {
    /// λ_c.
    pub speed_ppm: u32,
    /// k1, the weight of the track record.
    pub k1_ppm: u32,
    /// k2, the weight of the miss.
    pub k2_ppm: u32,
    /// k3, the weight of monetary financing over GDP.
    pub k3_ppm: u32,
    /// k4, the weight of each change of the target.
    pub k4_ppm: u32,
}

impl CredibilityParams {
    /// The spec's defaults: λ_c 0.05 a month; k1, k2, k3, k4 = 1, 10, 20, 1.
    #[must_use]
    pub fn spec_defaults() -> CredibilityParams {
        CredibilityParams {
            speed_ppm: 50_000,
            k1_ppm: 1_000_000,
            k2_ppm: 10_000_000,
            k3_ppm: 20_000_000,
            k4_ppm: 1_000_000,
        }
    }
}

/// Credibility after a monthly close (AC-MP-01):
/// `cred + λ_c [k1 · track record − k2 · |π − target| − k3 · monfin/GDP − k4 · target changes]`,
/// clipped to [0, 1,000,000]; the track record is `months_in_band_of_12` / 12.
///
/// Choices where the spec is open: `months_in_band_of_12` above 12 counts as
/// 12; `previous` above 1,000,000 is clipped like the result; the bracket is
/// kept exact (in twelfths of a millionth squared) and the step
/// `λ_c · bracket` is rounded once, half away from zero, before the clip.
#[must_use]
pub fn credibility_ppm(
    previous: u32,
    months_in_band_of_12: u8,
    miss_ppm: u32,
    monfin_over_gdp_ppm: u32,
    target_changes: u8,
    p: &CredibilityParams,
) -> u32 {
    let months = i128::from(months_in_band_of_12.min(12));
    // The bracket times 12 · 10^12: each weight (ppm) times its term (ppm).
    let bracket = i128::from(p.k1_ppm) * months * PPM
        - 12 * (i128::from(p.k2_ppm) * i128::from(miss_ppm)
            + i128::from(p.k3_ppm) * i128::from(monfin_over_gdp_ppm)
            + i128::from(p.k4_ppm) * i128::from(target_changes) * PPM);
    let step = div_round(i128::from(p.speed_ppm) * bracket, 12 * PPM * PPM);
    let next = (i128::from(previous) + step).clamp(0, PPM);
    // In [0, 10^6] after the clamp.
    u32::try_from(next).unwrap_or(0)
}

/// The advisory rule's parameters: ρ, r*, φ_π, φ_y in millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleParams {
    /// ρ, the smoothing.
    pub smoothing_ppm: u32,
    /// r*, the neutral real rate, a year.
    pub neutral_ppm: i32,
    /// φ_π.
    pub phi_pi_ppm: u32,
    /// φ_y.
    pub phi_y_ppm: u32,
}

impl RuleParams {
    /// The spec's defaults: ρ 0.8, r* 1%, φ_π 1.5, φ_y 0.5.
    #[must_use]
    pub fn spec_defaults() -> RuleParams {
        RuleParams {
            smoothing_ppm: 800_000,
            neutral_ppm: 10_000,
            phi_pi_ppm: 1_500_000,
            phi_y_ppm: 500_000,
        }
    }
}

/// The advisory rate (AC-MP-02):
/// `ρ i_{t-1} + (1 − ρ) [r* + π^e + φ_π (π − π^target) + φ_y gap]`.
///
/// Choices where the spec is open: no floor (the floor on the rate is
/// AC-MP-08, roadmap M4); ρ above 1,000,000 counts as 1 (pure smoothing); the
/// whole expression is kept exact and rounded once, half away from zero, and
/// saturates at the `i32` range.
#[must_use]
pub fn advisory_rate_ppm(
    previous: i32,
    expected_ppm: i32,
    inflation_ppm: i32,
    target_ppm: i32,
    gap_ppm: i32,
    p: &RuleParams,
) -> i32 {
    let rho = i128::from(p.smoothing_ppm).min(PPM);
    // The bracket times 10^6.
    let bracket = (i128::from(p.neutral_ppm) + i128::from(expected_ppm)) * PPM
        + i128::from(p.phi_pi_ppm) * (i128::from(inflation_ppm) - i128::from(target_ppm))
        + i128::from(p.phi_y_ppm) * i128::from(gap_ppm);
    let total = rho * i128::from(previous) * PPM + (PPM - rho) * bracket;
    let rate = div_round(total, PPM * PPM).clamp(i128::from(i32::MIN), i128::from(i32::MAX));
    i32::try_from(rate).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn div_round_is_half_away_from_zero() {
        assert_eq!(div_round(5, 10), 1);
        assert_eq!(div_round(4, 10), 0);
        assert_eq!(div_round(-5, 10), -1);
        assert_eq!(div_round(-4, 10), 0);
    }

    #[test]
    fn credibility_rounds_the_step_once() {
        let p = CredibilityParams::spec_defaults();
        // 0.05 · (1/12) = 4,166.67 ppm, rounded to 4,167.
        assert_eq!(credibility_ppm(0, 1, 0, 0, 0, &p), 4_167);
        // A record above 12 months counts as 12.
        assert_eq!(
            credibility_ppm(500_000, 200, 0, 0, 0, &p),
            credibility_ppm(500_000, 12, 0, 0, 0, &p)
        );
        // A previous value above 1 is clipped.
        assert_eq!(credibility_ppm(2_000_000, 0, 0, 0, 0, &p), 1_000_000);
    }

    #[test]
    fn credibility_extremes_do_not_overflow() {
        let p = CredibilityParams {
            speed_ppm: u32::MAX,
            k1_ppm: u32::MAX,
            k2_ppm: u32::MAX,
            k3_ppm: u32::MAX,
            k4_ppm: u32::MAX,
        };
        assert_eq!(credibility_ppm(u32::MAX, 12, 0, 0, 0, &p), 1_000_000);
        assert_eq!(credibility_ppm(0, 0, u32::MAX, u32::MAX, u8::MAX, &p), 0);
    }

    #[test]
    fn advisory_rule_smoothing_and_saturation() {
        let mut p = RuleParams::spec_defaults();
        // Full smoothing keeps the previous rate.
        p.smoothing_ppm = 1_000_000;
        assert_eq!(
            advisory_rate_ppm(17_500, 90_000, 90_000, 0, 50_000, &p),
            17_500
        );
        // No smoothing is the bracket itself; it may go negative (no floor).
        p.smoothing_ppm = 0;
        assert_eq!(
            advisory_rate_ppm(17_500, 0, -20_000, 0, -20_000, &p),
            -30_000
        );
        // Saturates at the i32 range.
        p.phi_pi_ppm = u32::MAX;
        assert_eq!(advisory_rate_ppm(0, 0, i32::MAX, i32::MIN, 0, &p), i32::MAX);
    }
}
