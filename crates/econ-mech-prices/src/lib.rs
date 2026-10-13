//! Prices (spec `economy/prices-inflation`, "First version"): unit cost,
//! markup, target price, the daily price step and expectations. Pure
//! functions, no I/O; the daily phase over industries and the monthly CPI
//! live with the `World`.
//!
//! Prices, shares and rates are millionths (a price of 1,000,000 is the
//! starting price). Powers go through `econ_num::math` (ADR-0006). API
//! skeleton from the spec's API sketch, written before the code (ADR-0014);
//! `target_price_ppm` was added by the test writer for AC-PRICE-01. The bodies
//! come with the implementation change.

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

/// Unit cost of an industry: home inputs at their prices (coefficient, price),
/// imported inputs, wage cost and other costs per unit of output.
#[must_use]
pub fn unit_cost_ppm(
    home_inputs: &[(u32, u32)],
    imported_ppm: u32,
    wage_cost_ppm: u32,
    other_ppm: u32,
) -> u32 {
    let _ = (home_inputs, imported_ppm, wage_cost_ppm, other_ppm);
    unimplemented!("econ-mech-prices: written in the implementation change")
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
    let _ = (base_ppm, utilisation_ppm, inventory_ratio_ppm, p);
    unimplemented!("econ-mech-prices: written in the implementation change")
}

/// Target price, `(1 + mu) * uc`.
#[must_use]
pub fn target_price_ppm(unit_cost_ppm: u32, markup_ppm: i32) -> u32 {
    let _ = (unit_cost_ppm, markup_ppm);
    unimplemented!("econ-mech-prices: written in the implementation change")
}

/// The daily equivalent of a monthly flexibility, `1 - (1 - rho)^(1/30)`.
#[must_use]
pub fn daily_share_ppm(monthly_share_ppm: u32) -> u32 {
    let _ = monthly_share_ppm;
    unimplemented!("econ-mech-prices: written in the implementation change")
}

/// One day's price, `p (1 + pi_e)^chi (p* / p)^rho`, its move capped at
/// `max_daily_move` (AC-PRICE-02).
#[must_use]
pub fn step_price_ppm(
    price: u32,
    target: u32,
    daily_flexibility_ppm: u32,
    expected_daily_ppm: i32,
    p: &PriceParams,
) -> u32 {
    let _ = (price, target, daily_flexibility_ppm, expected_daily_ppm, p);
    unimplemented!("econ-mech-prices: written in the implementation change")
}

/// Anchored-adaptive expectations,
/// `cred * target + (1 - cred) * (previous + lambda * (actual - previous))`
/// (AC-PRICE-03).
#[must_use]
pub fn expectations_ppm(
    previous: i32,
    actual: i32,
    target: i32,
    credibility_ppm: u32,
    learning_ppm: u32,
) -> i32 {
    let _ = (previous, actual, target, credibility_ppm, learning_ppm);
    unimplemented!("econ-mech-prices: written in the implementation change")
}
