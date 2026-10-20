//! Money and banking (spec `economy/money-banking`, "First version", roadmap
//! M3): the rates of loans and deposits, a month of interest, the room a firm
//! unit has on its credit line, the write-off of excess debt and the banks'
//! capital adequacy. Pure functions, no I/O; the daily credit draws, the
//! month-end interest and repayments and the reserve check live with the
//! `World`.
//!
//! Money is whole bani; rates and shares are millionths (a year for rates).
//! API skeleton from the spec's API sketch, written before the code
//! (ADR-0014). The bodies come with the implementation change.

use econ_types::Bani;

/// The banks' parameters (spec, "Tuning parameters"). Rates and shares in
/// millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankParams {
    /// `spread_firms`: firm loans over the policy rate.
    pub spread_firms_ppm: u32,
    /// `spread_mortgage`.
    pub spread_mortgage_ppm: u32,
    /// `spread_consumer`.
    pub spread_consumer_ppm: u32,
    /// `deposit_spread`: deposits under the policy rate, the rate not below 0.
    pub deposit_spread_ppm: u32,
    /// `reserve_requirement`: reserves banks must hold, of deposits.
    pub reserve_requirement_ppm: u32,
    /// `car_min`: capital adequacy below which new credit stops.
    pub car_min_ppm: u32,
    /// `credit_limit_months`: how many months of revenue a unit may owe.
    pub credit_limit_months: u32,
    /// `firm_repayment_share`, a month.
    pub firm_repayment_ppm: u32,
    /// `recovery_rate`: what the banks recover of a written-off loan.
    pub recovery_ppm: u32,
}

impl BankParams {
    /// The spec's defaults: spreads 3.0, 2.75 and 7.25 points, deposits 1
    /// point under the policy rate, reserves 8%, `car_min` 10%, 12 months of
    /// revenue, 1/60 a month, recovery 40%.
    #[must_use]
    pub fn spec_defaults() -> BankParams {
        BankParams {
            spread_firms_ppm: 30_000,
            spread_mortgage_ppm: 27_500,
            spread_consumer_ppm: 72_500,
            deposit_spread_ppm: 10_000,
            reserve_requirement_ppm: 80_000,
            car_min_ppm: 100_000,
            credit_limit_months: 12,
            firm_repayment_ppm: 16_667,
            recovery_ppm: 400_000,
        }
    }
}

/// The rates of the month, millionths a year.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rates {
    /// Loans to firm units.
    pub firms_ppm: u32,
    /// Mortgages.
    pub mortgage_ppm: u32,
    /// Consumer loans.
    pub consumer_ppm: u32,
    /// Deposits.
    pub deposit_ppm: u32,
}

/// The rates for a policy rate and the banks' capital adequacy (AC-BANK-05):
/// the policy rate plus each spread; deposits the policy rate less the
/// deposit spread, not below 0.
pub fn rates(policy_ppm: i32, car_ppm: u32, p: &BankParams) -> Rates {
    let _ = (policy_ppm, car_ppm, p);
    unimplemented!("econ-mech-banking: written in the implementation change")
}

/// A month of interest on a balance: a twelfth of a year at the rate,
/// rounded once.
pub fn month_interest(balance: Bani, rate_ppm: u32) -> Bani {
    let _ = (balance, rate_ppm);
    unimplemented!("econ-mech-banking: written in the implementation change")
}

/// What a unit may still draw on its credit line: up to
/// `credit_limit_months` of its revenue, nothing while the banks' capital
/// adequacy is below `car_min`.
pub fn credit_room(loans: Bani, revenue_month: Bani, car_ppm: u32, p: &BankParams) -> Bani {
    let _ = (loans, revenue_month, car_ppm, p);
    unimplemented!("econ-mech-banking: written in the implementation change")
}

/// The write-off of a unit's loans above `credit_limit_months` of its
/// revenue: (written off, the banks' loss, `1 − recovery` of it).
pub fn write_off(loans: Bani, revenue_month: Bani, p: &BankParams) -> (Bani, Bani) {
    let _ = (loans, revenue_month, p);
    unimplemented!("econ-mech-banking: written in the implementation change")
}

/// Capital adequacy, millionths: equity over risk-weighted loans (firms and
/// consumer loans 100%, mortgages 35%).
pub fn capital_ratio_ppm(equity: Bani, firm_loans: Bani, consumer: Bani, mortgages: Bani) -> u32 {
    let _ = (equity, firm_loans, consumer, mortgages);
    unimplemented!("econ-mech-banking: written in the implementation change")
}
