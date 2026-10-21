//! Acceptance tests for AC-EXC-01 to 05 (spec: economy/excise), the `[unit]`
//! criteria, on the module of the spec's API sketch (`econ_mech_tax::excise`).
//! Written in a test-authoring session (ADR-0014) from the spec only: the
//! `World`'s excise came first, by the owner's decision of 2026-10-21.
//! Compiled with the crate feature `excise`.

use std::path::PathBuf;

use econ_mech_tax::excise::{
    ExciseDuty, basket_with_excise, calibrate_duty_ppm, excise_on, gross_of, index_duty,
    split_budget,
};
use econ_types::Bani;
use serde_json::Value;

const ONE: i128 = 1_000_000;

/// `n / d` rounded half away from zero, `d > 0`.
fn round_div(n: i128, d: i128) -> i128 {
    if n >= 0 {
        (n + d / 2) / d
    } else {
        -((-n + d / 2) / d)
    }
}

/// The starting duties of the spec's table, millionths of a ban per real unit.
const FUEL: ExciseDuty = ExciseDuty(512_500);
const TOBACCO: ExciseDuty = ExciseDuty(2_030_400);
const BEVERAGES: ExciseDuty = ExciseDuty(71_200);

/// **AC-EXC-01** `[unit]` The duty on `q` units at duty `d` is `q · d / 10^6`
/// rounded half away from zero to whole bani. It is zero for a zero duty or
/// quantity, and never negative.
#[test]
fn ac_exc_01() {
    let quantities: [i64; 9] = [0, 1, 2, 3, 499, 10_000, 123_456_789, 1 << 40, (1 << 43) - 1];
    let duties = [
        0_u32,
        1,
        499_999,
        500_000,
        512_500,
        1_000_000,
        2_030_400,
        71_200,
        u32::MAX,
    ];
    for &q in &quantities {
        for &d in &duties {
            let e = excise_on(q, ExciseDuty(d));
            let expected = round_div(i128::from(q) * i128::from(d), ONE);
            assert_eq!(i128::from(e.get()), expected, "q {q}, d {d}");
            assert!(e >= Bani::ZERO, "q {q}, d {d}: negative");
            if q == 0 || d == 0 {
                assert_eq!(e, Bani::ZERO, "q {q}, d {d}");
            }
        }
    }
    // Half a ban rounds up: 1 unit at 500,000 ppm.
    assert_eq!(excise_on(1, ExciseDuty(500_000)), Bani(1));
    assert_eq!(excise_on(1, ExciseDuty(499_999)), Bani(0));
}

/// **AC-EXC-02** `[unit]` The worked example: 10,000 units of tobacco at
/// price 1 and duty 2,030,400 ppm, with VAT at 19%, give net 100.00, duty
/// 203.04, VAT 57.58 and gross 360.62 lei, to the ban. VAT is charged on the
/// duty.
#[test]
fn ac_exc_02() {
    let s = gross_of(10_000, 1_000_000, TOBACCO, 1_900);
    assert_eq!(s.quantity, 10_000);
    assert_eq!(s.net, Bani(10_000), "net 100.00 lei");
    assert_eq!(s.excise, Bani(20_304), "duty 203.04 lei");
    assert_eq!(s.vat, Bani(5_758), "VAT 57.58 lei, on 303.04");
    assert_eq!(s.net + s.excise + s.vat, Bani(36_062), "gross 360.62 lei");
    // The fuel of the worked example: 1,000.00 lei of fuel to a firm unit,
    // no VAT on its input: duty 512.50 lei.
    let f = gross_of(100_000, 1_000_000, FUEL, 0);
    assert_eq!(f.excise, Bani(51_250));
    assert_eq!(f.net + f.excise + f.vat, Bani(151_250));
}

/// **AC-EXC-03** `[unit]` From a gross budget, net + duty + VAT never exceeds
/// the budget, and the rest is less than one unit's gross price. A higher
/// duty never buys more units from the same budget.
///
/// "The rest is less than one unit's gross price": one more unit would not
/// fit, `gross_of(q + 1)` exceeds the budget.
#[test]
fn ac_exc_03() {
    let budgets = [0_i64, 1, 99, 360, 36_062, 1_000_000, 987_654_321];
    let prices = [400_000_u32, 1_000_000, 1_234_567, 3_000_000];
    let duties = [0_u32, 71_200, 512_500, 2_030_400, 10_152_000];
    let vats = [0_u32, 500, 900, 1_900];
    for &b in &budgets {
        for &p in &prices {
            for &v in &vats {
                let mut last_q = i64::MAX;
                for &d in &duties {
                    let s = split_budget(Bani(b), p, ExciseDuty(d), v);
                    let total = s.net + s.excise + s.vat;
                    assert!(s.quantity >= 0);
                    assert!(
                        total <= Bani(b),
                        "budget {b}, p {p}, d {d}, VAT {v}: spends {total:?}"
                    );
                    assert_eq!(
                        gross_of(s.quantity, p, ExciseDuty(d), v),
                        s,
                        "the split is the purchase's parts"
                    );
                    let next = gross_of(s.quantity + 1, p, ExciseDuty(d), v);
                    assert!(
                        next.net + next.excise + next.vat > Bani(b),
                        "budget {b}, p {p}, d {d}, VAT {v}: one more unit fits"
                    );
                    assert!(
                        s.quantity <= last_q,
                        "budget {b}: a higher duty {d} buys more"
                    );
                    last_q = s.quantity;
                }
            }
        }
    }
}

/// **AC-EXC-04** `[unit]` Indexation multiplies every duty by one plus the
/// twelve-month inflation, rounded. With inflation of zero or below, or with
/// indexation off, duties are unchanged.
///
/// The rule is `index_duty`; whether the law indexes at all
/// (`ExciseSchedule::indexation`) is the `World`'s, on 1 January.
#[test]
fn ac_exc_04() {
    for d in [0_u32, 1, 71_200, 512_500, 2_030_400, 1_000_003] {
        for pi in [1_i32, 50_000, 51_000, 82_000, 137_999, 1_000_000] {
            let expected = round_div(i128::from(d) * (ONE + i128::from(pi)), ONE);
            assert_eq!(
                i128::from(index_duty(ExciseDuty(d), pi).0),
                expected,
                "d {d}, π {pi}"
            );
        }
        for pi in [0_i32, -1, -20_000, -500_000] {
            assert_eq!(
                index_duty(ExciseDuty(d), pi),
                ExciseDuty(d),
                "d {d}, π {pi}: deflation"
            );
        }
    }
}

// --- the input-output table ----------------------------------------------------

fn io_table() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../python/pipeline/fixtures/io_table2021_ro.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {}: {e}", path.display()));
    serde_json::from_str(&text).expect("valid JSON")
}

fn lei(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

/// The NACE row of a code.
fn nace(t: &Value, code: &str) -> usize {
    t["industries"]
        .as_array()
        .expect("industries")
        .iter()
        .position(|i| i["code"] == code)
        .unwrap_or_else(|| panic!("no industry {code}"))
}

/// A catalogue industry's share of its NACE industry, and that industry.
fn part(t: &Value, id: &str) -> (usize, f64) {
    for c in t["catalogue"].as_array().expect("catalogue") {
        for p in c["parts"].as_array().expect("parts") {
            if p["id"] == id {
                let code = c["industry"].as_str().expect("code");
                return (nace(t, code), lei(&p["share"]) / 1e6);
            }
        }
    }
    panic!("no catalogue industry {id}")
}

/// Households' purchases of a NACE product, home and imported, lei.
fn households(t: &Value, i: usize) -> f64 {
    let h = &t["final"]["households"];
    lei(&h["domestic"][i]) + lei(&h["imports_eu"][i]) + lei(&h["imports_non_eu"][i])
}

/// The bases of the spec's table, lei a year: households' fuel, firms'
/// fuel less the exempt buyers', households' tobacco and beverages.
fn bases(t: &Value) -> (f64, f64, f64, f64) {
    let (c19, fuel_share) = part(t, "refined_petroleum");
    let n = t["industries"].as_array().expect("industries").len();
    let exempt = [
        "refined_petroleum",
        "chemicals",
        "fertilisers",
        "air_transport",
    ];
    let mut exempt_share = vec![0.0; n];
    for id in exempt {
        let (j, s) = part(t, id);
        exempt_share[j] += s;
    }
    let mut firms = 0.0;
    for j in 0..n {
        let bought = lei(&t["domestic"][c19][j])
            + lei(&t["imports_eu"][c19][j])
            + lei(&t["imports_non_eu"][c19][j]);
        firms += bought * fuel_share * (1.0 - exempt_share[j]);
    }
    let hh_fuel = households(t, c19) * fuel_share;
    let (food, tob) = part(t, "tobacco");
    let (food2, bev) = part(t, "beverages");
    (
        hh_fuel,
        firms,
        households(t, food) * tob,
        households(t, food2) * bev,
    )
}

/// **AC-EXC-05** `[unit]` Calibration: on the committed
/// `io_table2021_ro.json`, the starting duties times the starting bases give
/// 2021's revenue by group within 1%: fuel 17.47, tobacco 15.81 and beverages
/// 1.39 billion lei. `calibrate_duty_ppm` reproduces the defaults.
///
/// "Reproduces the defaults": the spec's defaults are rounded to the
/// hundred millionths; the calibration is within 0.1% of each.
#[test]
fn ac_exc_05() {
    let t = io_table();
    let (hh_fuel, firm_fuel, tobacco, beverages) = bases(&t);
    eprintln!(
        "AC-EXC-05 bases, billion lei: fuel {:.2} + {:.2}, tobacco {:.2}, beverages {:.2}",
        hh_fuel / 1e9,
        firm_fuel / 1e9,
        tobacco / 1e9,
        beverages / 1e9
    );
    // The bases of the spec's table.
    for (name, got, spec) in [
        ("households' fuel", hh_fuel, 11.84e9),
        ("firms' fuel", firm_fuel, 22.24e9),
        ("tobacco", tobacco, 7.79e9),
        ("beverages", beverages, 19.46e9),
    ] {
        assert!(
            (got - spec).abs() <= spec * 0.01,
            "{name}: base {got} lei against the spec's {spec}"
        );
    }
    let bani = |lei: f64| Bani(i64::try_from(lei.round() as i128 * 100).expect("fits"));
    for (name, duty, base, target_million) in [
        ("fuel", FUEL, hh_fuel + firm_fuel, 17_470.0),
        ("tobacco", TOBACCO, tobacco, 15_807.0),
        ("beverages", BEVERAGES, beverages, 1_387.0),
    ] {
        let target = target_million * 1e6;
        let revenue = base * f64::from(duty.0) / 1e6;
        assert!(
            (revenue - target).abs() <= target * 0.01,
            "{name}: duty {} on a base of {base} lei raises {revenue} lei, 2021's {target}",
            duty.0
        );
        let calibrated = calibrate_duty_ppm(bani(target), bani(base));
        assert!(
            (f64::from(calibrated.0) - f64::from(duty.0)).abs() <= f64::from(duty.0) * 0.001,
            "{name}: calibrated {} ppm, the default {}",
            calibrated.0,
            duty.0
        );
        // Exactly the ratio, in millionths.
        let exact = round_div(
            i128::from(bani(target).get()) * ONE,
            i128::from(bani(base).get()),
        );
        assert_eq!(i128::from(calibrated.0), exact, "{name}: target over base");
    }
}

/// The basket reweighting of the starting state ("Households' basket"): each
/// excised good's share times `1 + d`, renormalised to 1,000,000 by largest
/// remainder; goods with no duty keep their relative shares. Part of
/// AC-EXC-07's starting state, checked here on the rule.
#[test]
fn ac_exc_07_basket_rule() {
    let basket = [100_000_u32, 250_000, 50_000, 400_000, 200_000];
    let duty = [
        ExciseDuty(0),
        ExciseDuty(2_030_400),
        ExciseDuty(0),
        ExciseDuty(71_200),
        ExciseDuty(512_500),
    ];
    let w = basket_with_excise(&basket, &duty);
    assert_eq!(w.iter().map(|&x| u64::from(x)).sum::<u64>(), 1_000_000);
    let total: f64 = basket
        .iter()
        .zip(&duty)
        .map(|(&b, d)| f64::from(b) * (1.0 + f64::from(d.0) / 1e6))
        .sum();
    for g in 0..basket.len() {
        let exact = f64::from(basket[g]) * (1.0 + f64::from(duty[g].0) / 1e6) / total * 1e6;
        assert!(
            (f64::from(w[g]) - exact).abs() < 1.0,
            "good {g}: {} against {exact}",
            w[g]
        );
    }
}
