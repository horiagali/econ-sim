//! Monetary policy (spec `economy/monetary-policy`, "First version", roadmap
//! M3): the central bank's credibility and the advisory (Taylor-type) rule.
//! Pure functions, no I/O; the month-end interest on reserves and advances,
//! the profit remitted to the government and the close live with the
//! `World`.
//!
//! Rates, inflation and shares are millionths (a year for rates and
//! inflation). API skeleton from the spec's API sketch, written before the
//! code (ADR-0014). The bodies come with the implementation change.

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
pub fn credibility_ppm(
    previous: u32,
    months_in_band_of_12: u8,
    miss_ppm: u32,
    monfin_over_gdp_ppm: u32,
    target_changes: u8,
    p: &CredibilityParams,
) -> u32 {
    let _ = (
        previous,
        months_in_band_of_12,
        miss_ppm,
        monfin_over_gdp_ppm,
        target_changes,
        p,
    );
    unimplemented!("econ-mech-monetary: written in the implementation change")
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
pub fn advisory_rate_ppm(
    previous: i32,
    expected_ppm: i32,
    inflation_ppm: i32,
    target_ppm: i32,
    gap_ppm: i32,
    p: &RuleParams,
) -> i32 {
    let _ = (
        previous,
        expected_ppm,
        inflation_ppm,
        target_ppm,
        gap_ppm,
        p,
    );
    unimplemented!("econ-mech-monetary: written in the implementation change")
}
