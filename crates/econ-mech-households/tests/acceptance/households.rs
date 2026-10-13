//! Acceptance tests for AC-HH-01, AC-HH-02 and AC-HH-06 (spec:
//! economy/households, "First version", steps 2 and 3). Written in a
//! test-authoring session from the spec and its API sketch, before the code
//! existed (ADR-0014). The other AC-HH criteria need the `World`.

use econ_mech_households::{Budget, Decision, HouseholdParams, decide, split_basket};
use econ_types::Bani;

/// The spec's defaults: `mpc` 0.95 … 0.60 by fifth, `mpc_wealth` 0.5% a month,
/// `subsistence_basket` 700 lei.
fn params() -> HouseholdParams {
    HouseholdParams {
        mpc: [950_000, 900_000, 850_000, 750_000, 600_000],
        mpc_wealth_ppm: 5_000,
        subsistence_basket: Bani::from_lei(700),
    }
}

/// One adult at starting prices, no food grown at home.
fn budget(income: Bani, money: Bani, fifth: u8) -> Budget {
    Budget {
        income_received: income,
        money,
        fifth,
        equivalised_adults_milli: 1_000,
        subsistence_income: Bani(0),
        price_index_ppm: 1_000_000,
    }
}

fn spend(d: Decision) -> Bani {
    d.spending
}

/// AC-HH-01 [unit] The decision rule: for given income, money, fifth, floor and parameters, spending is the formula of step 2, in whole bani; it never exceeds the household's money.
#[test]
fn ac_hh_01() {
    let p = params();

    // c_y(q) * Y + c_w * M: 0.85 * 4,000 + 0.005 * 10,000 = 3,450 lei, above
    // the floor of 1.5 * 700 = 1,050 lei.
    let b = Budget {
        equivalised_adults_milli: 1_500,
        ..budget(Bani::from_lei(4_000), Bani::from_lei(10_000), 2)
    };
    assert_eq!(
        decide(&b, &p),
        Decision {
            spending: Bani::from_lei(3_450),
            hardship: false
        }
    );

    // Each fifth uses its own propensity: c_y(q) * 2,000 + 0.005 * 10,000.
    for (q, lei) in [(0, 1_900), (1, 1_800), (2, 1_700), (3, 1_500), (4, 1_200)] {
        let b = budget(Bani::from_lei(2_000), Bani::from_lei(10_000), q);
        assert_eq!(spend(decide(&b, &p)), Bani::from_lei(lei + 50), "fifth {q}");
    }

    // Whole bani, rounded half away from zero (no floor here).
    let no_floor = HouseholdParams {
        subsistence_basket: Bani(0),
        ..p
    };
    // 0.6 * 3 + 0.005 * 100 = 2.3 bani -> 2.
    assert_eq!(
        spend(decide(&budget(Bani(3), Bani(100), 4), &no_floor)),
        Bani(2)
    );
    // 0.6 * 5 + 0.005 * 100 = 3.5 bani -> 4.
    assert_eq!(
        spend(decide(&budget(Bani(5), Bani(100), 4), &no_floor)),
        Bani(4)
    );

    // The floor binds: 0.95 * 0 + 0.005 * 5,000 = 25 lei < 700 lei.
    assert_eq!(
        decide(&budget(Bani(0), Bani::from_lei(5_000), 0), &p),
        Decision {
            spending: Bani::from_lei(700),
            hardship: false
        }
    );
    // The floor scales with equivalised adults and with the price index:
    // two adults and a child (1.8) at prices up 10%: 700 * 1.8 * 1.1 = 1,386 lei.
    let b = Budget {
        equivalised_adults_milli: 1_800,
        price_index_ppm: 1_100_000,
        ..budget(Bani(0), Bani::from_lei(5_000), 0)
    };
    assert_eq!(spend(decide(&b, &p)), Bani::from_lei(1_386));

    // Never more than the household's money.
    for income_lei in [0, 1, 700, 3_000, 20_000] {
        for money_lei in [0, 1, 500, 699, 700, 701, 2_500, 100_000] {
            for fifth in 0..5 {
                let b = budget(Bani::from_lei(income_lei), Bani::from_lei(money_lei), fifth);
                let d = decide(&b, &p);
                assert!(
                    d.spending <= b.money,
                    "income {income_lei}, money {money_lei}, fifth {fifth}: spends {:?}",
                    d.spending
                );
                assert!(d.spending >= Bani(0));
            }
        }
    }
    // Spending all of it when the formula asks for more than there is.
    assert_eq!(
        spend(decide(
            &budget(Bani::from_lei(10_000), Bani::from_lei(3_000), 0),
            &p
        )),
        Bani::from_lei(3_000)
    );
}

/// AC-HH-02 [unit] The basket split gives every good its share and sums to the household's spending exactly.
#[test]
fn ac_hh_02() {
    assert_eq!(
        split_basket(Bani(100_000), &[500_000, 300_000, 200_000]),
        vec![Bani(50_000), Bani(30_000), Bani(20_000)]
    );
    // 10 bani over three near-equal shares: 3.33334, 3.33333, 3.33333; the
    // ban left over goes to the largest remainder.
    assert_eq!(
        split_basket(Bani(10), &[333_334, 333_333, 333_333]),
        vec![Bani(4), Bani(3), Bani(3)]
    );
    assert_eq!(
        split_basket(Bani(0), &[600_000, 400_000]),
        vec![Bani(0), Bani(0)]
    );

    let shares = [
        212_345, 150_000, 120_001, 99_999, 87_655, 80_000, 70_000, 60_000, 50_000, 40_000, 30_000,
    ];
    assert_eq!(shares.iter().sum::<u32>(), 1_000_000);
    for spending in [1, 7, 999, 123_457, 34_500_001, 987_654_321] {
        let parts = split_basket(Bani(spending), &shares);
        assert_eq!(parts.len(), shares.len());
        assert_eq!(
            parts.iter().map(|b| b.get()).sum::<i64>(),
            spending,
            "spending {spending}"
        );
        for (part, &share) in parts.iter().zip(&shares) {
            // Within one ban of the exact share.
            let exact = i128::from(spending) * i128::from(share);
            let got = i128::from(part.get()) * 1_000_000;
            assert!(
                (got - exact).abs() < 1_000_000,
                "spending {spending}, share {share}: got {part:?}"
            );
        }
    }
}

/// AC-HH-06 [sim] A household whose money is below its floor spends all of it and is flagged in hardship; one with food grown at home has a lower floor.
///
/// Tested on the decision rule, which is where the floor and the flag are set.
#[test]
fn ac_hh_06() {
    let p = params();

    // 500 lei against a floor of 700: all of it, in hardship.
    assert_eq!(
        decide(&budget(Bani(0), Bani::from_lei(500), 0), &p),
        Decision {
            spending: Bani::from_lei(500),
            hardship: true
        }
    );
    // No income and no money: buys nothing, in hardship.
    assert_eq!(
        decide(&budget(Bani(0), Bani(0), 0), &p),
        Decision {
            spending: Bani(0),
            hardship: true
        }
    );

    // Food grown at home worth 200 lei lowers the floor to 500 lei: the same
    // 500 lei is no longer hardship.
    let grower = Budget {
        subsistence_income: Bani::from_lei(200),
        ..budget(Bani(0), Bani::from_lei(500), 0)
    };
    assert_eq!(
        decide(&grower, &p),
        Decision {
            spending: Bani::from_lei(500),
            hardship: false
        }
    );
    // With money to spare, the grower's floor is 500 lei and the other's 700.
    let buyer = budget(Bani(0), Bani::from_lei(2_000), 0);
    let grower = Budget {
        subsistence_income: Bani::from_lei(200),
        ..buyer
    };
    assert_eq!(spend(decide(&buyer, &p)), Bani::from_lei(700));
    assert_eq!(spend(decide(&grower, &p)), Bani::from_lei(500));
    // The floor is not below zero: food at home worth more than the basket.
    let farmer = Budget {
        subsistence_income: Bani::from_lei(1_000),
        ..budget(Bani(0), Bani(0), 0)
    };
    assert_eq!(
        decide(&farmer, &p),
        Decision {
            spending: Bani(0),
            hardship: false
        }
    );
}
