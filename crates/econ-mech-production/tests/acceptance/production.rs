//! Acceptance tests for AC-PROD-01 and AC-PROD-02 (spec: economy/production,
//! "First version", steps 1 and 2). Written in a test-authoring session from
//! the spec and its API sketch, before the code existed (ADR-0014). The other
//! AC-PROD criteria need the `World`.

use econ_mech_production::{Plan, ProductionParams, Served, UnitState, plan, ration};

/// The spec's defaults: theta 0.3 a month, phi 0.25 a month.
fn params() -> ProductionParams {
    ProductionParams {
        theta_ppm: 300_000,
        phi_ppm: 250_000,
    }
}

/// Expected demand 1,000 a month, one month of it in stock, capacity 5,000,
/// four persons per thousand units of output.
fn unit() -> UnitState {
    UnitState {
        expected_demand: 1_000,
        inventory: 1_000,
        capacity: 5_000,
        inventory_target_ppm: 1_000_000,
        labour_per_million_output: 4_000,
    }
}

/// AC-PROD-01 [unit] The plan: expected demand, planned output and desired employment are the formulas of step 1, in integers; planned output is never negative and never above capacity.
#[test]
fn ac_prod_01() {
    let p = params();

    // D_e <- 1,000 + 0.3 * (2,000 - 1,000) = 1,300;
    // Y = 1,300 + 0.25 * (1 * 1,300 - 1,000) = 1,375;
    // employment = ceil(1,375 * 4 / 1,000) = ceil(5.5) = 6.
    assert_eq!(
        plan(&unit(), 2_000, &p),
        Plan {
            expected_demand: 1_300,
            planned_output: 1_375,
            desired_employment: 6
        }
    );
    // In balance: sales as expected, stock at target. 1,000 * 4 / 1,000 = 4
    // persons exactly, not rounded up.
    assert_eq!(
        plan(&unit(), 1_000, &p),
        Plan {
            expected_demand: 1_000,
            planned_output: 1_000,
            desired_employment: 4
        }
    );
    // Rounding half away from zero: 0.3 * 5 = 1.5 -> 2, then
    // 0.25 * (1,002 - 1,000) = 0.5 -> 1; ceil(4.012) = 5.
    assert_eq!(
        plan(&unit(), 1_005, &p),
        Plan {
            expected_demand: 1_002,
            planned_output: 1_003,
            desired_employment: 5
        }
    );
    // And below: 0.3 * -5 = -1.5 -> -2, then 0.25 * (998 - 1,000) = -0.5 -> -1.
    assert_eq!(
        plan(&unit(), 995, &p),
        Plan {
            expected_demand: 998,
            planned_output: 997,
            desired_employment: 4
        }
    );
    // A services unit keeps a quarter of a month: 1,000 + 0.25 * (250 - 0) = 1,062.5 -> 1,063.
    let services = UnitState {
        inventory: 0,
        inventory_target_ppm: 250_000,
        ..unit()
    };
    assert_eq!(plan(&services, 1_000, &p).planned_output, 1_063);

    // Never negative: far too much stock.
    let glut = UnitState {
        inventory: 100_000,
        ..unit()
    };
    let g = plan(&glut, 1_000, &p);
    assert_eq!(g.planned_output, 0);
    assert_eq!(g.desired_employment, 0);
    // Never above capacity: 3,700 + 675 = 4,375 is cut to 4,000.
    let small = UnitState {
        capacity: 4_000,
        ..unit()
    };
    assert_eq!(
        plan(&small, 10_000, &p),
        Plan {
            expected_demand: 3_700,
            planned_output: 4_000,
            desired_employment: 16
        }
    );

    for sales in [0, 1, 999, 1_000, 3_000, 1_000_000] {
        for inventory in [0, 500, 1_000, 5_000, 1_000_000] {
            let u = UnitState {
                inventory,
                ..unit()
            };
            let pl = plan(&u, sales, &p);
            assert!(
                (0..=u.capacity).contains(&pl.planned_output),
                "sales {sales}, inventory {inventory}: {pl:?}"
            );
        }
    }
}

fn check_rationed(demands: &[i64], available: i64, s: &Served) {
    let asked: i64 = demands.iter().sum();
    let sold: i64 = s.sold.iter().sum();
    assert_eq!(s.sold.len(), demands.len());
    assert!(
        sold <= available,
        "{demands:?} from {available}: sold {sold}"
    );
    assert_eq!(s.shortage, asked - sold, "{demands:?} from {available}");
    let n = i64::try_from(demands.len()).expect("few buyers");
    assert!(
        sold >= available - n,
        "{demands:?} from {available}: only {sold} sold"
    );
    for (&d, &got) in demands.iter().zip(&s.sold) {
        assert!((0..=d).contains(&got));
        // The same share for every buyer, within one unit.
        let exact = i128::from(d) * i128::from(available);
        let have = i128::from(got) * i128::from(asked);
        assert!(
            (have - exact).abs() < i128::from(asked),
            "{demands:?} from {available}: buyer of {d} got {got}"
        );
    }
}

/// AC-PROD-02 [unit] Rationing: when demand on an industry exceeds its inventories every buyer gets the same share, nothing more than the inventories is sold, and the shortage is demand less sales.
#[test]
fn ac_prod_02() {
    // Enough stock: everyone is served.
    assert_eq!(
        ration(&[100, 200, 300], 1_000),
        Served {
            sold: vec![100, 200, 300],
            shortage: 0
        }
    );
    assert_eq!(
        ration(&[100, 200, 300], 600),
        Served {
            sold: vec![100, 200, 300],
            shortage: 0
        }
    );
    // Half the demand in stock: everyone gets half.
    assert_eq!(
        ration(&[100, 200, 300], 300),
        Served {
            sold: vec![50, 100, 150],
            shortage: 300
        }
    );
    // Nothing in stock.
    assert_eq!(
        ration(&[100, 200, 300], 0),
        Served {
            sold: vec![0, 0, 0],
            shortage: 600
        }
    );
    assert_eq!(
        ration(&[], 500),
        Served {
            sold: vec![],
            shortage: 0
        }
    );

    for demands in [
        vec![1, 1, 1],
        vec![7, 0, 13, 1],
        vec![1_000_000, 1, 333_333],
        vec![5; 17],
    ] {
        let asked: i64 = demands.iter().sum();
        for available in [0, 1, 2, asked / 3, asked - 1] {
            check_rationed(&demands, available, &ration(&demands, available));
        }
    }
}
