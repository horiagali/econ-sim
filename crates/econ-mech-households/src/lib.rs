//! Households (spec `economy/households`, "First version"): how much a
//! household spends in a month, and the split over goods. Pure functions, no
//! I/O; the phase that walks today's households lives with the `World`.
//!
//! Money is whole bani; every product is computed in `i128` and rounded once,
//! half away from zero (ADR-0007).

use econ_types::{Bani, split_largest_remainder};

/// `n / d` rounded half away from zero (`d > 0`).
fn rdiv(n: i128, d: i128) -> i128 {
    let (q, r) = econ_types::div_rem_i128(n, d);
    if 2 * r.abs() >= d { q + n.signum() } else { q }
}

/// An `i128` amount of bani as `Bani`.
fn bani(v: i128) -> Bani {
    Bani(i64::try_from(v).expect("amount fits i64 bani"))
}

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
    let money = i128::from(b.money.get()).max(0);
    // floor = subsistence basket x equivalised adults x CPI, less food grown at home, not below 0.
    let basket = rdiv(
        i128::from(p.subsistence_basket.get())
            * i128::from(b.equivalised_adults_milli)
            * i128::from(b.price_index_ppm),
        1_000 * 1_000_000,
    );
    let floor = (basket - i128::from(b.subsistence_income.get())).max(0);
    // c_y(q) * Y + c_w * M, rounded once.
    let mpc = i128::from(p.mpc[usize::from(b.fifth.min(4))]);
    let wanted = rdiv(
        mpc * i128::from(b.income_received.get()) + i128::from(p.mpc_wealth_ppm) * money,
        1_000_000,
    );
    Decision {
        spending: bani(wanted.max(floor).min(money).max(0)),
        hardship: money < floor,
    }
}

/// Split `spending` over goods by `basket_share_ppm` (millionths, summing to
/// 1,000,000), largest remainder to whole bani (AC-HH-02).
#[must_use]
pub fn split_basket(spending: Bani, basket_share_ppm: &[u32]) -> Vec<Bani> {
    if basket_share_ppm.is_empty() {
        return Vec::new();
    }
    let weights: Vec<u64> = basket_share_ppm.iter().map(|&s| u64::from(s)).collect();
    split_largest_remainder(spending, &weights)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_is_half_away_from_zero() {
        assert_eq!(rdiv(7, 2), 4);
        assert_eq!(rdiv(-7, 2), -4);
        assert_eq!(rdiv(5, 3), 2);
    }

    #[test]
    fn an_empty_basket_splits_into_nothing() {
        assert!(split_basket(Bani(100), &[]).is_empty());
    }
}
