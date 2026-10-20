//! The banks on the `World` (feature `banking`; spec `economy/money-banking`,
//! "First version", roadmap M3): read-only state for the `[sim]` criteria
//! AC-BANK-01 to 08.
//!
//! API skeleton written by the test writer with those tests, before the code
//! (ADR-0014). The bodies come with the implementation change.

use econ_mech_banking::Rates;
use econ_types::Bani;

use super::World;

/// The banks' state after the latest day. Money in bani; per-unit columns
/// aligned with [`super::FirmUnitColumns`] (a unit's amounts are already
/// totals of its firms), per-household columns with
/// [`super::HouseholdColumns`] (per record, before the weight).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BankState {
    /// The policy rate the banks price from, millionths a year.
    pub policy_rate_ppm: i32,
    /// The rates of the month, set at the latest monthly close (at load: the
    /// starting policy rate plus the spreads).
    pub rates: Rates,
    /// Each firm unit's bank loans, `unit_loans`.
    pub unit_loans: Vec<Bani>,
    /// What each unit drew on its credit line on the latest day.
    pub drawn_today: Vec<Bani>,
    /// What each unit repaid on the latest day.
    pub repaid_today: Vec<Bani>,
    /// What was written off each unit's loans on the latest day (the excess
    /// over its credit limit; the monthly close).
    pub written_off_today: Vec<Bani>,
    /// The banks' loss on those write-offs, by unit: by this much the unit's
    /// loans fell; it keeps owing the rest.
    pub bank_loss_today: Vec<Bani>,
    /// The banks' claim on firms: the sum of `unit_loans`.
    pub firm_loans: Bani,
    /// Each household's mortgage and consumer debt (stage H; constant in M3).
    pub hh_mortgage: Vec<Bani>,
    /// Consumer debt.
    pub hh_consumer_debt: Vec<Bani>,
    /// Interest each household paid on its loans at the latest month end.
    pub hh_interest_paid: Vec<Bani>,
    /// Interest each household received on its deposits at the latest month end.
    pub hh_interest_received: Vec<Bani>,
    /// Interest each unit paid on its loans at the latest month end.
    pub unit_interest_paid: Vec<Bani>,
    /// Interest each unit received on its deposits at the latest month end.
    pub unit_interest_received: Vec<Bani>,
    /// The banks' equity after the latest monthly close.
    pub equity: Bani,
    /// Capital adequacy after the latest monthly close, millionths.
    pub car_ppm: u32,
    /// Credit inside the firm sector (M2's placeholder): what the units owe
    /// each other. Zero once the banks lend.
    pub firm_sector_credit: Bani,
}

impl World {
    /// The banks.
    #[must_use]
    pub fn bank(&self) -> &BankState {
        unimplemented!("econ-core banking: written in the implementation change")
    }
}
