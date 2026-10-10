//! Acceptance tests for AC-VAT-* (spec: economy/taxation, "VAT" section).
//! Written by the test-writer pass from the spec text alone, before the
//! implementation existed (Spike 9, ADR-0014).

use econ_ledger::{FlowCode, Instrument, Ledger, Sector, Txn};
use econ_mech_tax::vat::{
    Purchase, VatCategory, VatRate, VatSchedule, VatWarning, collect_vat, vat_on,
};
use econ_types::Bani;

fn schedule(standard_pct: u32) -> VatSchedule {
    VatSchedule {
        standard: VatRate::percent(standard_pct),
        reduced: VatRate::percent(11),
    }
}

/// AC-VAT-01 [unit] VAT on a purchase is `rate × net amount`, rounded half away from zero to whole bani; the gross price is net + VAT.
#[test]
fn ac_vat_01() {
    let r = VatRate::percent(21);
    assert_eq!(vat_on(Bani(10_000), r), Bani(2_100));
    // 21% of 1 ban = 0.21 → 0; of 3 bani = 0.63 → 1; of 50 bani = 10.5 → 11 (half away from zero)
    assert_eq!(vat_on(Bani(1), r), Bani(0));
    assert_eq!(vat_on(Bani(3), r), Bani(1));
    assert_eq!(vat_on(Bani(50), r), Bani(11));
    // negative net (a refund) rounds away from zero too
    assert_eq!(vat_on(Bani(-50), r), Bani(-11));
    let net = Bani(12_345);
    assert_eq!(r.gross(net), net + vat_on(net, r));
}

/// AC-VAT-02 [unit] Each good's VAT category decides its rate: standard, reduced, zero (0%) or exempt (no VAT).
#[test]
fn ac_vat_02() {
    let s = schedule(21);
    let net = Bani(100_000);
    assert_eq!(s.vat(net, VatCategory::Standard), Bani(21_000));
    assert_eq!(s.vat(net, VatCategory::Reduced), Bani(11_000));
    assert_eq!(s.vat(net, VatCategory::Zero), Bani(0));
    assert_eq!(s.vat(net, VatCategory::Exempt), Bani(0));
    // zero-rated and exempt differ: zero-rated has a rate (0%), exempt has none
    assert_eq!(s.rate_for(VatCategory::Zero), Some(VatRate::percent(0)));
    assert_eq!(s.rate_for(VatCategory::Exempt), None);
}

/// AC-VAT-03 [ledger] Collecting VAT on a batch of household purchases posts exactly the sum of per-purchase VAT from households to government under flow code `tax.vat`, and all ledger invariants hold.
#[test]
fn ac_vat_03() {
    let s = schedule(21);
    let mut ledger = Ledger::new();
    ledger.allow_negative(Sector::Government, Instrument::Cash);
    ledger.begin_tick(0);
    ledger
        .commit(Txn::new().leg(
            Sector::Government,
            Sector::Households,
            Instrument::Cash,
            Bani::from_lei(1_000_000),
            FlowCode::OPENING,
        ))
        .unwrap();
    ledger.begin_tick(1);
    let purchases = [
        Purchase {
            net: Bani(10_050),
            category: VatCategory::Standard,
            weight: 100,
        },
        Purchase {
            net: Bani(333),
            category: VatCategory::Reduced,
            weight: 7,
        },
        Purchase {
            net: Bani(9_999),
            category: VatCategory::Zero,
            weight: 40,
        },
        Purchase {
            net: Bani(50),
            category: VatCategory::Standard,
            weight: 1,
        },
        Purchase {
            net: Bani(77_777),
            category: VatCategory::Exempt,
            weight: 3,
        },
    ];
    // Expected: per-purchase VAT (rounded per unit) × weight, summed.
    let expected = Bani(2_111 * 100 + 37 * 7 + 11);
    let hh_before = ledger.balance(Sector::Households, Instrument::Cash);
    let collected = collect_vat(&mut ledger, &s, &purchases).unwrap();
    assert_eq!(collected, expected);
    assert_eq!(
        ledger
            .flows()
            .received(FlowCode::TAX_VAT, Sector::Government),
        expected
    );
    assert_eq!(
        hh_before - ledger.balance(Sector::Households, Instrument::Cash),
        expected
    );
    assert_eq!(FlowCode::TAX_VAT.name(), "tax.vat");
    ledger.check_invariants().unwrap();
}

/// AC-VAT-04 [unit] A standard rate below the EU minimum of 15% is accepted but flagged as an EU-rule breach (ADR: eu-membership compliance), not rejected.
#[test]
fn ac_vat_04() {
    assert!(schedule(15).warnings().is_empty());
    assert!(schedule(21).warnings().is_empty());
    let low = schedule(12);
    assert_eq!(
        low.warnings(),
        vec![VatWarning::BelowEuMinimumStandardRate {
            rate: VatRate::percent(12)
        }]
    );
    // Still usable — not rejected.
    assert_eq!(low.vat(Bani(100), VatCategory::Standard), Bani(12));
}

/// AC-VAT-05 [unit] Raising the standard rate from 19% to 21% raises the gross price of a standard-rated good by exactly the VAT difference and leaves zero-rated goods unchanged.
#[test]
fn ac_vat_05() {
    let (a, b) = (schedule(19), schedule(21));
    for net in [Bani(1), Bani(99), Bani(12_345), Bani(1_000_000)] {
        let ga = a.gross(net, VatCategory::Standard);
        let gb = b.gross(net, VatCategory::Standard);
        assert_eq!(
            gb - ga,
            b.vat(net, VatCategory::Standard) - a.vat(net, VatCategory::Standard)
        );
        assert!(gb >= ga);
        assert_eq!(
            a.gross(net, VatCategory::Zero),
            b.gross(net, VatCategory::Zero)
        );
        assert_eq!(a.gross(net, VatCategory::Zero), net);
    }
}
