//! The government's debt and its budget report on the `World` (feature
//! `fiscal`; spec `economy/fiscal-policy`, "First version", roadmap M3): its
//! parameters and read-only state for the `[sim]` criteria AC-FIS-02, 03
//! and 05.
//!
//! API skeleton written by the test writer with those tests, before the code
//! (ADR-0014). The bodies come with the implementation change.

use econ_ledger::FlowCode;
use econ_mech_fiscal::{DebtBook, DebtParams};
use econ_types::{Bani, Month};

use super::World;

/// The debt's parameters (spec, "Tuning parameters").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FiscalParams {
    /// New issues and the yield.
    pub debt: DebtParams,
    /// `starting_coupon`, millionths a year (4.5%).
    pub starting_coupon_ppm: u32,
    /// `gov_cash_target`, in months of spending (1).
    pub cash_target_months: u32,
}

impl FiscalParams {
    /// The spec's defaults.
    #[must_use]
    pub fn spec_defaults() -> FiscalParams {
        FiscalParams {
            debt: DebtParams::spec_defaults(),
            starting_coupon_ppm: 45_000,
            cash_target_months: 1,
        }
    }
}

/// The government's debt after the latest day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Treasury {
    /// Bonds outstanding by year of maturity and their average coupon.
    pub book: DebtBook,
    /// The yield of new issues, set at the latest monthly close, millionths a
    /// year.
    pub bond_yield_ppm: i32,
    /// `gov_cash_target`: the deposits at the central bank it keeps.
    pub cash_target: Bani,
    /// Bonds that fell due at the latest month end.
    pub matured: Bani,
    /// Bonds redeemed at the latest month end, at maturity.
    pub redeemed: Bani,
    /// Bonds issued at the latest month end (negative: redeemed early).
    pub issued: Bani,
}

/// The government's budget of a closed month: every flow of the month to or
/// from the government outside its financing, by flow code, national bani.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiscalMonth {
    /// The month.
    pub month: Month,
    /// Receipts by flow code (taxes, contributions, the central bank's
    /// profit, …), ascending by code; each positive.
    pub revenue: Vec<(FlowCode, Bani)>,
    /// Payments by flow code (purchases, transfers, budget lines), ascending
    /// by code, interest excluded; each positive.
    pub spending: Vec<(FlowCode, Bani)>,
    /// Interest on the debt (`interest.gov_debt`).
    pub interest: Bani,
    /// Spending + interest − revenue.
    pub deficit: Bani,
}

impl World {
    /// The government's debt.
    #[must_use]
    pub fn treasury(&self) -> &Treasury {
        unimplemented!("econ-core fiscal: written in the implementation change")
    }

    /// The budget of every closed month, in order.
    #[must_use]
    pub fn fiscal_months(&self) -> &[FiscalMonth] {
        unimplemented!("econ-core fiscal: written in the implementation change")
    }
}
