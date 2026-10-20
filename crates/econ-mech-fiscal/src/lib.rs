//! Fiscal policy (spec `economy/fiscal-policy`, "First version", roadmap M3):
//! the government's debt book by year of maturity, its coupons, new issues
//! and the yield of new bonds. Pure functions, no I/O; the month-end
//! postings live with the `World`.
//!
//! Money is whole bani; rates and shares are millionths (a year for rates).
//! API skeleton from the spec's API sketch, written before the code
//! (ADR-0014). The bodies come with the implementation change.

use econ_types::Bani;

/// The debt's parameters, millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebtParams {
    /// `new_issue_years`: maturity of new bonds.
    pub new_issue_years: u8,
    /// `term_premium`: the new issues' yield over the policy rate, a year.
    pub term_premium_ppm: u32,
    /// `risk_premium` risk_0: yield per unit of (debt-to-GDP above the
    /// threshold) squared, a year.
    pub risk_ppm: u32,
    /// `debt_threshold`: debt over GDP where the risk premium starts.
    pub threshold_ppm: u32,
}

impl DebtParams {
    /// The spec's defaults: 7 years, 1.5 points, 20 points, 60% of GDP.
    #[must_use]
    pub fn spec_defaults() -> DebtParams {
        DebtParams {
            new_issue_years: 7,
            term_premium_ppm: 15_000,
            risk_ppm: 200_000,
            threshold_ppm: 600_000,
        }
    }
}

/// The yield of new bonds (step 4, AC-FIS-04): `i + term + risk_0 ·
/// max(0, B/Y − threshold)²`, with Y the last twelve months' GDP.
pub fn bond_yield_ppm(policy_ppm: i32, debt: Bani, gdp_year: Bani, p: &DebtParams) -> i32 {
    let _ = (policy_ppm, debt, gdp_year, p);
    unimplemented!("econ-mech-fiscal: written in the implementation change")
}

/// Bonds outstanding by year of maturity and the stock's average coupon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebtBook {
    /// Maturing in year k from now (k = 0 is the coming twelve months).
    pub by_year: Vec<Bani>,
    /// The stock's average coupon, millionths a year.
    pub coupon_ppm: u32,
}

impl DebtBook {
    /// The book at the start: `total` spread over years of maturity in the
    /// proportions of the profile's buckets (from year, to year, amount),
    /// each bucket spread evenly over its years.
    pub fn from_profile(total: Bani, buckets: &[(u8, u8, Bani)], coupon_ppm: u32) -> DebtBook {
        let _ = (total, buckets, coupon_ppm);
        unimplemented!("econ-mech-fiscal: written in the implementation change")
    }

    /// A month of coupon on the stock: a twelfth of the coupon, rounded once.
    pub fn month_coupon(&self) -> Bani {
        unimplemented!("econ-mech-fiscal: written in the implementation change")
    }

    /// What falls due this month, taken off the book.
    pub fn mature(&mut self, months_elapsed: u32) -> Bani {
        let _ = months_elapsed;
        unimplemented!("econ-mech-fiscal: written in the implementation change")
    }

    /// Issue new bonds maturing in `new_issue_years` at the yield (a
    /// negative amount redeems early); the coupon becomes the weighted mean.
    pub fn issue(&mut self, amount: Bani, yield_ppm: i32, p: &DebtParams) {
        let _ = (amount, yield_ppm, p);
        unimplemented!("econ-mech-fiscal: written in the implementation change")
    }
}

/// An amount split in proportion to holdings, by largest remainder: the
/// parts add up to the amount exactly.
pub fn split_by_holdings(amount: Bani, holdings: &[Bani]) -> Vec<Bani> {
    let _ = (amount, holdings);
    unimplemented!("econ-mech-fiscal: written in the implementation change")
}
