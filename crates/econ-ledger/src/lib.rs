//! The accounting ledger (ADR-0007).
//!
//! **All money moves through this crate.** Behaviour code builds a [`Txn`]
//! of one or more [`Leg`]s (each: payer, payee, instrument, amount, flow
//! code) and commits it; the ledger applies all legs or none. Balances can
//! only be read from outside, never written.
//!
//! Model: each (sector, instrument) pair has a signed balance in [`Bani`]
//! from that sector's point of view (positive = asset, negative =
//! liability). A leg moves `amount` of an instrument from payer to payee, so
//! every instrument row always sums to zero across sectors (invariant I-1).
//! Every leg is also recorded by flow code in the transaction-flow matrix
//! (TFM), whose rows sum to zero and whose columns equal each sector's change
//! in financial position (I-3). Checks use a separate code path from the
//! posting path so they catch bugs rather than restate them.

#![warn(clippy::arithmetic_side_effects)]

mod codes;
mod ledger;

pub use codes::FlowCode;
pub use ledger::{
    Instrument, InvariantError, Ledger, LedgerError, Leg, Posting, Sector, TickFlows, Txn,
};
