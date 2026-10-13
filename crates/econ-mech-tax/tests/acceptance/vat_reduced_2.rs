//! Acceptance test for AC-VAT-06 (spec: economy/vat), the second reduced rate.
//! Written in a test-authoring session from the spec and its API sketch, before
//! the code existed (ADR-0014). Compiled only with the crate feature
//! `reduced-2`, which the implementation adds to `default`.

use econ_mech_tax::vat::{VatCategory, VatRate, VatSchedule};
use econ_types::Bani;

fn schedule(standard_pct: u32, reduced_pct: u32, reduced_2_pct: u32) -> VatSchedule {
    VatSchedule {
        standard: VatRate::percent(standard_pct),
        reduced: VatRate::percent(reduced_pct),
        reduced_2: VatRate::percent(reduced_2_pct),
    }
}

/// AC-VAT-06 [unit] A good in the second reduced category is taxed at `vat_rate_reduced_2`. With the schedule 19%, 9%, 5%, a net purchase of 1,000.00 lei carries 190.00, 90.00 and 50.00 lei of VAT in the three taxed categories; changing one reduced rate leaves the VAT of the other category unchanged.
#[test]
fn ac_vat_06() {
    // Romania on 1 December 2021.
    let s = schedule(19, 9, 5);
    let net = Bani::from_lei(1_000);
    assert_eq!(s.rate_for(VatCategory::Reduced2), Some(VatRate::percent(5)));
    assert_eq!(s.vat(net, VatCategory::Standard), Bani::from_lei(190));
    assert_eq!(s.vat(net, VatCategory::Reduced), Bani::from_lei(90));
    assert_eq!(s.vat(net, VatCategory::Reduced2), Bani::from_lei(50));
    assert_eq!(s.gross(net, VatCategory::Reduced2), Bani::from_lei(1_050));

    // Changing the first reduced rate leaves the second reduced category alone,
    // and the other way round. The standard rate is untouched by both.
    for net in [Bani(1), Bani(50), Bani(12_345), net] {
        let first_moved = schedule(19, 11, 5);
        assert_eq!(
            first_moved.vat(net, VatCategory::Reduced2),
            s.vat(net, VatCategory::Reduced2)
        );
        assert_eq!(
            first_moved.vat(net, VatCategory::Standard),
            s.vat(net, VatCategory::Standard)
        );
        let second_moved = schedule(19, 9, 7);
        assert_eq!(
            second_moved.vat(net, VatCategory::Reduced),
            s.vat(net, VatCategory::Reduced)
        );
        assert_eq!(
            second_moved.vat(net, VatCategory::Standard),
            s.vat(net, VatCategory::Standard)
        );
    }
    assert_eq!(
        schedule(19, 11, 5).vat(Bani::from_lei(1_000), VatCategory::Reduced),
        Bani::from_lei(110)
    );
    assert_eq!(
        schedule(19, 9, 7).vat(Bani::from_lei(1_000), VatCategory::Reduced2),
        Bani::from_lei(70)
    );

    // Neither reduced rate has to be the lower one.
    let swapped = schedule(19, 5, 9);
    assert_eq!(
        swapped.vat(Bani::from_lei(1_000), VatCategory::Reduced),
        Bani::from_lei(50)
    );
    assert_eq!(
        swapped.vat(Bani::from_lei(1_000), VatCategory::Reduced2),
        Bani::from_lei(90)
    );
    // A second reduced rate is not an EU-rule breach in itself.
    assert!(s.warnings().is_empty());
}
