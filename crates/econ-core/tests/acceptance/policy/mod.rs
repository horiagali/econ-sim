//! The policy-scenario suite (spec game/policy-scenarios, M4): nine dated
//! policy changes, each run against the same world without it, with the
//! response the simulation must show. One file per scenario, its commands
//! file under `commands/`, the harness in `harness.rs`.
//!
//! Readings shared by every scenario criterion (AC-POL-01 to 09):
//! - "The scenario's commands file is accepted": `econ_io::commands::
//!   read_commands` reads it against the registry of the scenario's folder
//!   (keys from its tables), and the `World` accepts each line on its
//!   `given` day with its logged effective day. A rejection fails the test
//!   with the lever system's code and message.
//! - Commands are given on 15 February 2022. `law` levers carry no
//!   `effective` and so take effect on 1 March 2022 (T0); `market` and
//!   `world` levers name 1 March 2022 as their `effective` day (spec, "Shock
//!   date"). S5's return to zero is given on 15 February 2024 with
//!   `effective` 1 March 2024 (month 25).
//! - A scenario criterion runs at 1:1000, the scale of its own test; the
//!   1:100 and 1:10 runs are AC-POL-10's (spec, "Three scales").
//! - Every checked expectation of the scenario's table holds: the expected
//!   sign, inside the inclusive band, and at least twice the noise floor ν
//!   in absolute value; a band that contains zero skips the sign and the 2ν
//!   rule. Watched expectations are printed (`--nocapture`), never asserted.
//! - Every run (baseline, placebo, scenario) keeps the checks of the day and
//!   person conservation on every day (see `harness.rs`).
//!
//! Features: the suite is behind `policy`. A scenario whose lever is not
//! wired yet has its test behind that lever's feature as well
//! (`lever_policy_rate` S1, `lever_monetary_financing` S5,
//! `lever_fx_devaluation` S6, `lever_fuel_price` S9), and is part of AC-POL-10
//! and 11 only when the feature is on ([`wired`]).

pub mod harness;

pub mod deficit_spending;
pub mod devaluation;
pub mod income_tax_cut;
pub mod minimum_wage;
pub mod monetary_financing;
pub mod oil_shock;
pub mod pension_rise;
pub mod rate_hike;
pub mod vat_rise;

mod mutation;
mod scales;

use harness::Scenario;

/// The nine scenarios, S1 to S9.
pub fn all() -> [&'static Scenario; 9] {
    [
        &rate_hike::S1,
        &income_tax_cut::S2,
        &vat_rise::S3,
        &deficit_spending::S4,
        &monetary_financing::S5,
        &devaluation::S6,
        &minimum_wage::S7,
        &pension_rise::S8,
        &oil_shock::S9,
    ]
}

/// Whether a scenario's lever is wired in this build: the five whose levers
/// were active when the suite was written, and the four others when their
/// lever's feature is on.
pub fn is_wired(s: &Scenario) -> bool {
    const BLOCKED: [(&str, bool); 4] = [
        ("S1", cfg!(feature = "lever_policy_rate")),
        ("S5", cfg!(feature = "lever_monetary_financing")),
        ("S6", cfg!(feature = "lever_fx_devaluation")),
        ("S9", cfg!(feature = "lever_fuel_price")),
    ];
    BLOCKED
        .iter()
        .find(|(id, _)| *id == s.id)
        .is_none_or(|&(_, on)| on)
}

/// The scenarios run by AC-POL-10 and 11 in this build.
pub fn wired() -> Vec<&'static Scenario> {
    let w: Vec<_> = all().into_iter().filter(|s| is_wired(s)).collect();
    for s in all().into_iter().filter(|s| !is_wired(s)) {
        println!(
            "{} ({}) is not run: its lever's feature is off in this build",
            s.id, s.ac
        );
    }
    w
}
