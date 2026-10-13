//! Households (spec `economy/households`, "First version"): how much a
//! household spends in a month, and the split over goods. Pure functions, no
//! I/O; the phase that walks today's households lives with the `World`.
//!
//! API skeleton from the spec's API sketch, written before the code
//! (ADR-0014). The bodies come with the implementation change.

use econ_types::Bani;

/// Tuning parameters of the decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HouseholdParams {
    /// `mpc[q]`: share of the month's income spent, in millionths, by income
    /// fifth (index 0 = poorest).
    pub mpc: [u32; 5],
    /// `mpc_wealth`: share of cash and deposits spent each month, in millionths.
    pub mpc_wealth_ppm: u32,
    /// `subsistence_basket`: the floor per equivalised adult a month, at starting prices.
    pub subsistence_basket: Bani,
}

/// What a household decides with, on its day of the month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// `hh_income_received`: money received since the last decision day.
    pub income_received: Bani,
    /// `hh_cash` + `hh_deposits`, after income.
    pub money: Bani,
    /// Income fifth by last month's income per equivalised adult (0 = poorest).
    pub fifth: u8,
    /// Equivalised adults, in thousandths (1 for the first adult, 0.5 per
    /// further adult, 0.3 per child under 14).
    pub equivalised_adults_milli: u32,
    /// `hh_subsistence_income`: food grown at home, a month.
    pub subsistence_income: Bani,
    /// Consumer price index, in millionths (1,000,000 at the start).
    pub price_index_ppm: u32,
}

/// The month's decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    /// Spending this month, VAT included.
    pub spending: Bani,
    /// `hh_hardship`: the household's money is below its floor.
    pub hardship: bool,
}

/// The decision of step 2 (AC-HH-01, AC-HH-06).
#[must_use]
pub fn decide(b: &Budget, p: &HouseholdParams) -> Decision {
    let _ = (b, p);
    unimplemented!("econ-mech-households: written in the implementation change")
}

/// Split `spending` over goods by `basket_share_ppm` (millionths, summing to
/// 1,000,000), largest remainder to whole bani (AC-HH-02).
#[must_use]
pub fn split_basket(spending: Bani, basket_share_ppm: &[u32]) -> Vec<Bani> {
    let _ = (spending, basket_share_ppm);
    unimplemented!("econ-mech-households: written in the implementation change")
}
