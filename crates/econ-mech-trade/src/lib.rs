//! Trade and the exchange rate (spec `economy/trade-fx`, "First version",
//! roadmap M3): the Armington import share, export demand and the monthly
//! step of the managed float. Pure functions, no I/O; powers through
//! `econ_num::math` (ADR-0006). The monthly close and the settlement live
//! with the `World`.
//!
//! Prices, shares and rates are millionths; the exchange rate is lei per
//! euro in millionths. Every rounding to an integer is half away from zero.
//! A price of zero is read as one millionth (the smallest representable
//! price), so no step divides by zero.

use econ_num::math::powf;
use econ_types::Bani;

/// One, in millionths.
const ONE: u32 = 1_000_000;
const ONE_F: f64 = 1_000_000.0;
const ONE_I: i128 = 1_000_000;

/// `n / d` rounded half away from zero (`d > 0`).
#[allow(clippy::arithmetic_side_effects)] // d > 0; callers keep n within i128
fn round_div(n: i128, d: i128) -> i128 {
    debug_assert!(d > 0);
    let q = n / d;
    let r = (n % d).abs();
    if 2 * r >= d { q + n.signum() } else { q }
}

/// The import share a use of a good moves toward (step 2, AC-TFX-01):
/// `s0 · pM^(1−σ) / (s0 · pM^(1−σ) + (1 − s0) · p^(1−σ))`.
///
/// Computed as `s0 / (s0 + (1 − s0) · (p / pM)^(1−σ))`, which is the same
/// expression divided through by `pM^(1−σ)`. At equal prices, at σ = 1 and
/// at a starting share of 0 or 1 it returns the starting share exactly. A
/// starting share above one is read as one.
#[must_use]
pub fn armington_share_ppm(
    start_share_ppm: u32,
    import_price_ppm: u32,
    domestic_price_ppm: u32,
    sigma_ppm: u32,
) -> u32 {
    let s0 = start_share_ppm.min(ONE);
    let pm = import_price_ppm.max(1);
    let pd = domestic_price_ppm.max(1);
    if s0 == 0 || s0 == ONE || pm == pd || sigma_ppm == ONE {
        return s0;
    }
    let exponent = (ONE_F - f64::from(sigma_ppm)) / ONE_F;
    let relative = powf(f64::from(pd) / f64::from(pm), exponent);
    let s0f = f64::from(s0);
    let share = s0f * ONE_F / (s0f + (ONE_F - s0f) * relative);
    let share = econ_num::finite(share, "armington share").round();
    // The share lies in [0, ONE] by construction; the clamp only guards the
    // float's extremes.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let out = share.clamp(0.0, ONE_F) as u32;
    out
}

/// A day's export demand on a good (step 3): `base · (p / (e/e0 · p*))^(−ε)
/// · demand index`, in real units.
///
/// `rate_index_ppm` is `e / e0`. At unit relative price (or ε = 0) the result
/// is exactly `base · demand index`. A result beyond `i64` saturates.
#[must_use]
pub fn export_demand(
    base: i64,
    domestic_price_ppm: u32,
    rate_index_ppm: u32,
    world_price_ppm: u32,
    demand_index_ppm: u32,
    elasticity_ppm: u32,
) -> i64 {
    let p = f64::from(domestic_price_ppm.max(1));
    let foreign = f64::from(rate_index_ppm.max(1)) * f64::from(world_price_ppm.max(1)) / ONE_F;
    let elasticity = f64::from(elasticity_ppm) / ONE_F;
    #[allow(clippy::float_cmp)] // exact equality is the intended shortcut
    let factor = if p == foreign || elasticity_ppm == 0 {
        1.0
    } else {
        powf(p / foreign, -elasticity)
    };
    // `base` is exact in f64 up to 2^53, which every day's real flow of a
    // good satisfies.
    #[allow(clippy::cast_precision_loss)]
    let x = base as f64 * factor * f64::from(demand_index_ppm) / ONE_F;
    let x = econ_num::finite(x, "export demand").round();
    #[allow(clippy::cast_possible_truncation)]
    let out = x.clamp(-9.2e18, 9.2e18) as i64;
    out
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
    /// The spec's defaults: γ1 0.02, γ2 0.05, rp the starting real-rate gap
    /// (1.75 points: the policy rate 1.75% less expected inflation 2.5%,
    /// against the euro area's −0.5% less 2%; restated in the spec on
    /// 2026-10-21 from a constant 3 points), intervention 0.8.
    #[must_use]
    pub fn spec_defaults() -> FxParams {
        FxParams {
            gamma1_ppm: 20_000,
            gamma2_ppm: 50_000,
            risk_premium_ppm: 17_500,
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
///
/// `real_gap_ppm` is `(i − π^e) − (i* − π*^e)`, a year; γ1 applies per point
/// (10 000 ppm) of `gap − rp`. The move is computed exactly in `i128` and
/// the new rate rounded once (never below one millionth, never above
/// `u32::MAX`).
///
/// **The intervention's size** (the spec leaves it open): the central bank
/// sells foreign assets worth the offset part of the move times the month's
/// GDP, `intervention_share · (Δe/e)_free · gdp_month`, rounded to the ban.
/// It sells (> 0) against depreciation pressure and buys (< 0) against
/// appreciation; it is zero below the floor or with no pressure.
#[must_use]
#[allow(clippy::arithmetic_side_effects)] // every product below is bounded well inside i128
pub fn fx_step(
    rate_ppm: u32,
    real_gap_ppm: i32,
    ca_over_gdp_ppm: i32,
    reserves_above_floor: bool,
    gdp_month: Bani,
    p: &FxParams,
) -> FxStep {
    // The free move Δe/e in units of 1e-12:
    // −γ1 · (gap − rp) / 10 000 / 1e6 − γ2 · CA/GDP / 1e6 / 1e6.
    let gap_less_rp = i128::from(real_gap_ppm) - i128::from(p.risk_premium_ppm);
    let move_e12 = -i128::from(p.gamma1_ppm) * gap_less_rp * 100
        - i128::from(p.gamma2_ppm) * i128::from(ca_over_gdp_ppm);
    let offset_ppm = if reserves_above_floor {
        i128::from(p.intervention_ppm.min(ONE))
    } else {
        0
    };
    let kept_ppm = ONE_I - offset_ppm;
    let e = i128::from(rate_ppm);
    // Δe = e · move · kept, over 1e12 · 1e6.
    let delta = round_div(e * move_e12 * kept_ppm, ONE_I * ONE_I * ONE_I);
    let rate = (e + delta).clamp(1, i128::from(u32::MAX));
    let rate_ppm = u32::try_from(rate).expect("clamped to u32");
    let intervention = round_div(
        i128::from(gdp_month.get()) * move_e12 * offset_ppm,
        ONE_I * ONE_I * ONE_I,
    );
    let intervention = Bani(i64::try_from(intervention).expect("an intervention within i64"));
    FxStep {
        rate_ppm,
        intervention,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_demand_at_unit_relative_price_is_base_times_demand() {
        assert_eq!(export_demand(1_000, ONE, ONE, ONE, ONE, ONE), 1_000);
        assert_eq!(export_demand(1_000, ONE, ONE, ONE, 1_100_000, ONE), 1_100);
        // Equal p and e/e0 · p* at another level: still unit relative price.
        assert_eq!(
            export_demand(1_000, 1_200_000, 1_200_000, ONE, ONE, 1_500_000),
            1_000
        );
    }

    #[test]
    fn export_demand_follows_the_relative_price() {
        // ε = 1: a 10% depreciation raises demand by 10%.
        assert_eq!(
            export_demand(1_000_000, ONE, 1_100_000, ONE, ONE, ONE),
            1_100_000
        );
        // Dearer domestic good: (1.25)^(−1) = 0.8.
        assert_eq!(
            export_demand(1_000_000, 1_250_000, ONE, ONE, ONE, ONE),
            800_000
        );
        // ε = 2: (1/1.1)^(−2) = 1.21.
        assert_eq!(
            export_demand(1_000_000, ONE, ONE, 1_100_000, ONE, 2_000_000),
            1_210_000
        );
        // ε = 0: prices do not matter.
        assert_eq!(export_demand(1_000, 2_000_000, ONE, ONE, ONE, 0), 1_000);
        // A zero price does not divide by zero.
        assert!(export_demand(1_000, 0, ONE, ONE, ONE, ONE) > 1_000);
    }

    #[test]
    fn armington_edges() {
        // A share above one is read as one; zero prices do not panic.
        assert_eq!(
            armington_share_ppm(2_000_000, ONE, 1_100_000, 1_500_000),
            ONE
        );
        let s = armington_share_ppm(300_000, 0, ONE, 1_500_000);
        assert!(s > 300_000 && s < ONE);
    }

    #[test]
    fn fx_spec_defaults_hold_the_rate_at_the_starting_gap() {
        let p = FxParams::spec_defaults();
        let gdp = Bani::from_lei(100_000_000_000);
        for floor in [false, true] {
            let step = fx_step(4_921_500, 17_500, 0, floor, gdp, &p);
            assert_eq!(step.rate_ppm, 4_921_500);
            assert_eq!(step.intervention, Bani::ZERO);
        }
    }

    #[test]
    fn fx_intervention_is_the_offset_move_times_gdp() {
        let p = FxParams {
            gamma1_ppm: 20_000,
            gamma2_ppm: 50_000,
            risk_premium_ppm: 30_000,
            intervention_ppm: 800_000,
        };
        let gdp = Bani::from_lei(100_000_000_000);
        // Free move +4%, 80% offset: 3.2% of the month's GDP.
        let step = fx_step(4_921_500, 10_000, 0, true, gdp, &p);
        assert_eq!(step.intervention, Bani::from_lei(3_200_000_000));
        // Extreme pressure cannot push the rate below one millionth.
        let step = fx_step(10, i32::MAX, i32::MAX, false, gdp, &p);
        assert_eq!(step.rate_ppm, 1);
    }
}
