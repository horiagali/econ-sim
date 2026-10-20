//! The central bank on the `World` (feature `monetary`; spec
//! `economy/monetary-policy`, "First version", roadmap M3): its parameters
//! and read-only state for the `[sim]` criteria AC-MP-03 to 05.
//!
//! API skeleton written by the test writer with those tests, before the code
//! (ADR-0014). The bodies come with the implementation change.

use econ_mech_monetary::{CredibilityParams, RuleParams};
use econ_types::Bani;

use super::World;

/// The central bank's parameters (spec, "Tuning parameters"), millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonetaryParams {
    /// `corridor`: the facilities around the policy rate (±1 point).
    pub corridor_ppm: u32,
    /// `foreign_yield`: what the foreign assets earn a year (0.5%).
    pub foreign_yield_ppm: u32,
    /// `target_band` (±1 point).
    pub target_band_ppm: u32,
    /// Credibility's update.
    pub credibility: CredibilityParams,
    /// The advisory rule.
    pub rule: RuleParams,
}

impl MonetaryParams {
    /// The spec's defaults.
    #[must_use]
    pub fn spec_defaults() -> MonetaryParams {
        MonetaryParams {
            corridor_ppm: 10_000,
            foreign_yield_ppm: 5_000,
            target_band_ppm: 10_000,
            credibility: CredibilityParams::spec_defaults(),
            rule: RuleParams::spec_defaults(),
        }
    }
}

/// The central bank's state after the latest day.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CentralBankState {
    /// The policy rate, millionths a year.
    pub policy_rate_ppm: i32,
    /// `cb_credibility`, millionths.
    pub credibility_ppm: u32,
    /// `monetary_financing`: the central bank's claim on the government
    /// (its deposits below zero, bonds bought from it).
    pub monetary_financing: Bani,
    /// The advisory rule's rate at the latest close, millionths a year.
    pub advisory_rate_ppm: i32,
    /// The reserves above the requirement on which the latest month end's
    /// interest at the deposit facility was paid.
    pub excess_reserves_basis: Bani,
    /// The advances on which the latest month end's interest at the lending
    /// facility was charged.
    pub advances_basis: Bani,
}

impl World {
    /// The central bank.
    #[must_use]
    pub fn central_bank(&self) -> &CentralBankState {
        unimplemented!("econ-core monetary: written in the implementation change")
    }
}
