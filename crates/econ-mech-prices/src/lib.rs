//! Prices (spec `economy/prices-inflation`, "First version"): unit cost,
//! markup, target price, the daily price step and expectations. Pure
//! functions, no I/O; the daily phase over industries and the monthly CPI
//! live with the `World`.
//!
//! Prices, shares and rates are millionths (a price of 1,000,000 is the
//! starting price). Sums and products are exact integers; the two powers of
//! the daily step and the daily share go through `econ_num::math` (ADR-0006)
//! and are rounded to whole millionths.

use econ_num::math::powf;

/// One, in millionths.
const ONE: i128 = 1_000_000;

/// The lowest markup: a price cannot fall below a tenth of unit cost (spec,
/// open questions).
const MIN_MARKUP_PPM: i32 = -900_000;

/// Tuning parameters, in millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceParams {
    /// `expectation_indexation` (chi).
    pub indexation_ppm: u32,
    /// `markup_utilisation_response` (eta 1).
    pub eta_utilisation_ppm: u32,
    /// `markup_inventory_response` (eta 2).
    pub eta_inventory_ppm: u32,
    /// `normal_utilisation` (cu*).
    pub normal_utilisation_ppm: u32,
    /// `max_daily_price_move`.
    pub max_daily_move_ppm: u32,
}

/// `n / d` rounded half away from zero (`d > 0`).
fn rdiv(n: i128, d: i128) -> i128 {
    let (q, r) = econ_types::div_rem_i128(n, d);
    if 2 * r.abs() >= d { q + n.signum() } else { q }
}

/// A non-negative millionths value as `u32`, saturating.
fn to_u32(v: i128) -> u32 {
    u32::try_from(v.max(0)).unwrap_or(u32::MAX)
}

/// A millionths value as `i32`, saturating.
fn to_i32(v: i128) -> i32 {
    i32::try_from(v).unwrap_or(if v < 0 { i32::MIN } else { i32::MAX })
}

/// Unit cost of an industry: home inputs at their prices (coefficient, price),
/// imported inputs, wage cost and other costs per unit of output.
#[must_use]
pub fn unit_cost_ppm(
    home_inputs: &[(u32, u32)],
    imported_ppm: u32,
    wage_cost_ppm: u32,
    other_ppm: u32,
) -> u32 {
    let home: i128 = home_inputs
        .iter()
        .map(|&(a, p)| i128::from(a) * i128::from(p))
        .sum();
    to_u32(
        rdiv(home, ONE)
            + i128::from(imported_ppm)
            + i128::from(wage_cost_ppm)
            + i128::from(other_ppm),
    )
}

/// Markup, `mu0 + eta1 (cu - cu*) - eta2 (V / (v* De) - 1)`; the inventory
/// ratio `V / (v* De)` is passed in millionths.
#[must_use]
pub fn markup_ppm(
    base_ppm: i32,
    utilisation_ppm: u32,
    inventory_ratio_ppm: u32,
    p: &PriceParams,
) -> i32 {
    let pressure = i128::from(p.eta_utilisation_ppm)
        * (i128::from(utilisation_ppm) - i128::from(p.normal_utilisation_ppm))
        - i128::from(p.eta_inventory_ppm) * (i128::from(inventory_ratio_ppm) - ONE);
    to_i32(i128::from(base_ppm) + rdiv(pressure, ONE))
}

/// Target price, `(1 + mu) * uc`, with the markup no lower than −0.9.
#[must_use]
pub fn target_price_ppm(unit_cost_ppm: u32, markup_ppm: i32) -> u32 {
    let mu = i128::from(markup_ppm.max(MIN_MARKUP_PPM));
    to_u32(rdiv(i128::from(unit_cost_ppm) * (ONE + mu), ONE))
}

/// The daily equivalent of a monthly flexibility, `1 - (1 - rho)^(1/30)`.
#[must_use]
pub fn daily_share_ppm(monthly_share_ppm: u32) -> u32 {
    let rho = f64::from(monthly_share_ppm.min(1_000_000)) / 1e6;
    let daily = 1.0 - powf(1.0 - rho, 1.0 / 30.0);
    to_u32(round_ppm(daily))
}

/// A share as whole millionths, rounded half away from zero.
fn round_ppm(x: f64) -> i128 {
    // At most a few million: the cast cannot overflow.
    #[allow(clippy::cast_possible_truncation)]
    let v = (x * 1e6).round() as i128;
    v
}

/// One day's price, `p (1 + pi_e)^chi (p* / p)^rho`, its move capped at
/// `max_daily_move` of the price (AC-PRICE-02). A price never reaches zero.
#[must_use]
pub fn step_price_ppm(
    price: u32,
    target: u32,
    daily_flexibility_ppm: u32,
    expected_daily_ppm: i32,
    p: &PriceParams,
) -> u32 {
    let price = price.max(1);
    let (p0, t) = (f64::from(price), f64::from(target.max(1)));
    let drift = powf(
        1.0 + f64::from(expected_daily_ppm) / 1e6,
        f64::from(p.indexation_ppm) / 1e6,
    );
    let pull = powf(t / p0, f64::from(daily_flexibility_ppm) / 1e6);
    let next = round_ppm(p0 * drift * pull / 1e6);
    let cap = i128::from(price) * i128::from(p.max_daily_move_ppm) / ONE;
    let old = i128::from(price);
    to_u32(next.clamp(old - cap, old + cap).max(1))
}

/// Anchored-adaptive expectations,
/// `cred * target + (1 - cred) * (previous + lambda * (actual - previous))`
/// (AC-PRICE-03), rounded once.
#[must_use]
pub fn expectations_ppm(
    previous: i32,
    actual: i32,
    target: i32,
    credibility_ppm: u32,
    learning_ppm: u32,
) -> i32 {
    let cred = i128::from(credibility_ppm.min(1_000_000));
    let lambda = i128::from(learning_ppm.min(1_000_000));
    let (prev, act) = (i128::from(previous), i128::from(actual));
    let adaptive = prev * ONE + lambda * (act - prev); // in millionths of millionths
    let num = cred * i128::from(target) * ONE + (ONE - cred) * adaptive;
    to_i32(rdiv(num, ONE * ONE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_is_half_away_from_zero() {
        assert_eq!(rdiv(5, 2), 3);
        assert_eq!(rdiv(-5, 2), -3);
        assert_eq!(rdiv(4, 3), 1);
        assert_eq!(rdiv(-4, 3), -1);
    }

    #[test]
    fn markup_never_below_minus_ninety_percent_in_the_target() {
        assert_eq!(target_price_ppm(1_000_000, -2_000_000), 100_000);
    }

    #[test]
    fn a_price_never_reaches_zero() {
        let p = PriceParams {
            indexation_ppm: 500_000,
            eta_utilisation_ppm: 200_000,
            eta_inventory_ppm: 100_000,
            normal_utilisation_ppm: 800_000,
            max_daily_move_ppm: 1_000_000,
        };
        assert!(step_price_ppm(1, 0, 1_000_000, -999_999, &p) >= 1);
    }
}
