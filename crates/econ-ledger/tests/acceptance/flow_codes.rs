//! Acceptance test for AC-ACC-04 (spec: economy/accounting). Written in a
//! test-authoring session before the check exists (ADR-0014); compiled only
//! with the crate feature `flow-code-check`.
//!
//! "Fails at test time": committing a leg whose flow code is not one of the
//! ledger's codes either panics or is refused with an error, and in the second
//! case nothing is posted. The test accepts both, so the implementation may
//! choose a debug assertion or a `LedgerError` variant.

use std::panic::{AssertUnwindSafe, catch_unwind};

use econ_ledger::{FlowCode, Instrument, Ledger, Sector, Txn};
use econ_types::Bani;

fn ledger() -> Ledger {
    let mut ledger = Ledger::new();
    ledger.allow_negative(Sector::Government, Instrument::Cash);
    ledger.begin_tick(0);
    ledger
        .commit(Txn::new().leg(
            Sector::Government,
            Sector::Households,
            Instrument::Cash,
            Bani::from_lei(1_000),
            FlowCode::OPENING,
        ))
        .expect("opening balance");
    ledger.begin_tick(1);
    ledger
}

fn pay(code: FlowCode) -> Txn {
    Txn::new().leg(
        Sector::Households,
        Sector::Firms,
        Instrument::Cash,
        Bani::from_lei(10),
        code,
    )
}

/// AC-ACC-04 [unit] A transfer with an unknown reason code fails at test time.
#[test]
fn ac_acc_04() {
    // Every known code is accepted.
    for code in [
        FlowCode::CONSUMPTION,
        FlowCode::GOV_PURCHASES,
        FlowCode::WAGES,
        FlowCode::TAX_INCOME,
        FlowCode::TAX_VAT,
    ] {
        let mut l = ledger();
        assert!(l.commit(pay(code)).is_ok(), "{code:?} refused");
    }

    // Unknown codes, alone or next to a valid leg, fail.
    for unknown in [
        FlowCode(0),
        FlowCode(1999),
        FlowCode(7777),
        FlowCode(u16::MAX),
    ] {
        for with_valid_leg in [false, true] {
            let mut l = ledger();
            let before = l.balance(Sector::Households, Instrument::Cash);
            let txn = if with_valid_leg {
                pay(FlowCode::CONSUMPTION).leg(
                    Sector::Households,
                    Sector::Government,
                    Instrument::Cash,
                    Bani::from_lei(1),
                    unknown,
                )
            } else {
                pay(unknown)
            };
            let outcome = catch_unwind(AssertUnwindSafe(|| l.commit(txn)));
            match outcome {
                Err(_) => {} // a panic: failed at test time
                Ok(Err(_)) => assert_eq!(
                    l.balance(Sector::Households, Instrument::Cash),
                    before,
                    "{unknown:?}: refused but something was posted"
                ),
                Ok(Ok(())) => panic!("{unknown:?} (with a valid leg: {with_valid_leg}) was posted"),
            }
        }
    }
}
