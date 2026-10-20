//! Trade and the exchange rate (spec `economy/trade-fx`, "First version",
//! roadmap M3): the Armington import share, export demand and the monthly
//! step of the managed float. Pure functions, no I/O; powers through
//! `econ_num::math` (ADR-0006). The monthly close and the settlement live
//! with the `World`.
//!
//! Prices, shares and rates are millionths; the exchange rate is lei per
//! euro in millionths. API skeleton from the spec's API sketch, written
//! before the code (ADR-0014). The bodies come with the implementation
//! change.

use econ_types::Bani;

/// The import share a use of a good moves toward (step 2, AC-TFX-01):
/// `s0 · pM^(1−σ) / (s0 · pM^(1−σ) + (1 − s0) · p^(1−σ))`.
pub fn armington_share_ppm(
    start_share_ppm: u32,
    import_price_ppm: u32,
    domestic_price_ppm: u32,
    sigma_ppm: u32,
) -> u32 {
    let _ = (
        start_share_ppm,
        import_price_ppm,
        domestic_price_ppm,
        sigma_ppm,
    );
    unimplemented!("econ-mech-trade: written in the implementation change")
}

/// A day's export demand on a good (step 3): `base · (p / (e/e0 · p*))^(−ε)
/// · demand index`, in real units.
pub fn export_demand(
    base: i64,
    domestic_price_ppm: u32,
    rate_index_ppm: u32,
    world_price_ppm: u32,
    demand_index_ppm: u32,
    elasticity_ppm: u32,
) -> i64 {
    let _ = (
        base,
        domestic_price_ppm,
        rate_index_ppm,
        world_price_ppm,
        demand_index_ppm,
        elasticity_ppm,
    );
    unimplemented!("econ-mech-trade: written in the implementation change")
}

/// The exchange rate's parameters, millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FxParams {
    /// γ1: the move a month per point (1/100) of real-rate gap.
    pub gamma1_ppm: u32,
    /// γ2: the move a month per unit of current account over GDP.
    pub gamma2_ppm: u32,
    /// rp, Romania's risk premium, a year.
    pub risk_premium_ppm: i32,
    /// `intervention_share`: the part of a move the central bank offsets.
    pub intervention_ppm: u32,
}

impl FxParams {
    /// The spec's defaults: γ1 0.02, γ2 0.05, rp 3 points, intervention 0.8.
    #[must_use]
    pub fn spec_defaults() -> FxParams {
        FxParams {
            gamma1_ppm: 20_000,
            gamma2_ppm: 50_000,
            risk_premium_ppm: 30_000,
            intervention_ppm: 800_000,
        }
    }
}

/// The rate after a monthly close and the central bank's intervention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FxStep {
    /// Lei per euro, millionths.
    pub rate_ppm: u32,
    /// Lei of foreign assets the central bank sells (> 0) or buys (< 0).
    pub intervention: Bani,
}

/// The monthly step of the exchange rate (step 4, AC-TFX-02):
/// `Δe/e = −γ1 [(i − π^e) − (i* − π*^e) − rp] − γ2 · CA/GDP`, of which the
/// central bank offsets `intervention_share` while its reserves are above
/// the floor, and nothing below it.
pub fn fx_step(
    rate_ppm: u32,
    real_gap_ppm: i32,
    ca_over_gdp_ppm: i32,
    reserves_above_floor: bool,
    gdp_month: Bani,
    p: &FxParams,
) -> FxStep {
    let _ = (
        rate_ppm,
        real_gap_ppm,
        ca_over_gdp_ppm,
        reserves_above_floor,
        gdp_month,
        p,
    );
    unimplemented!("econ-mech-trade: written in the implementation change")
}
