//! Simulation core (ADR-0005).
//!
//! Spike 2 implements **SIM mode**: the Godley–Lavoie "SIM" model (one
//! household sector, one firm sector, a government that issues money) on the
//! real ledger, with integer money, a per-tick state hash and invariant checks.
//! It exists to prove the accounting backbone against an independent Python
//! reference (`python/reference/`), not as game logic.

// Money arithmetic is checked by `Bani` itself; index maths here is not linted for side effects.

mod hash;
pub mod scale_spike;
pub mod sim;

pub use hash::StateHasher;

/// A player command, applied at the start of the tick it is submitted for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Set government spending per tick (SIM mode).
    SetGovSpending(econ_types::Bani),
    /// Set the income tax rate θ (SIM mode), in [0, 1).
    SetTaxRate(f64),
}

/// What a tick returns: aggregates, invariant results and the state hash.
#[derive(Debug, Clone, PartialEq)]
pub struct TickReport {
    /// Tick number (month index).
    pub tick: u32,
    /// Named aggregates in a fixed order.
    pub aggregates: Vec<(&'static str, econ_types::Bani)>,
    /// Hash of the full state after the tick.
    pub state_hash: u64,
    /// Invariant failures (empty when all hold).
    pub invariant_errors: Vec<String>,
}

impl TickReport {
    /// Look up an aggregate by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<econ_types::Bani> {
        self.aggregates
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
    }
}
