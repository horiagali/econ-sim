//! Acceptance tests for AC-FUI-01 to AC-FUI-10 (spec:
//! economy/firm-units-undeclared-inventories). Written in a test-authoring
//! session from the spec, its API sketch and the Python reference, before the
//! Rust code existed (ADR-0014).

// The columns are parallel vectors indexed by unit, as in the spec.
#![allow(clippy::needless_range_loop)]

use econ_popgen::{FirmUnits, UnitSize};
use econ_scenario::{FirmOpening, FirmOpeningError, FirmOpeningMargins, firm_opening};
use serde_json::Value;

use super::support_fui::{
    FIRMS_FIXTURE, MARGINS_FIXTURE, NO_EMPLOYER, U, amounts, changed, class, columns, firms,
    firms_json, fixture_path, golden, group, hex, industry_ids, margins, margins_json,
    parse_margins, reference, variants, workers,
};

/// Industry groups of the fixture, by their place in `industry_groups`.
const AGRICULTURE: u8 = 0;
const TRADE_TRANSPORT_HOSPITALITY: u8 = 3;
const INFORMATION: u8 = 4;
const FINANCE: u8 = 5;
const PROFESSIONAL: u8 = 7;
const PUBLIC_EDUCATION_HEALTH: u8 = 8;

fn n_units() -> usize {
    firms().units.len()
}

fn is_cohort(f: usize) -> bool {
    firms().units[f].size != UnitSize::Public
}

fn industry_id(f: usize) -> &'static str {
    industry_ids()[usize::from(firms().units[f].industry)]
}

/// A unit's stock: own output plus materials.
fn stock(c: &FirmOpening, f: usize) -> u64 {
    c.inventory[f] + c.inventory_materials[f]
}

/// The spec's apportion: largest remainder on integer weights, ties to the
/// lower index; a total of zero gives zeros.
fn apportion(total: u64, weights: &[u128]) -> Vec<u64> {
    let total = u128::from(total);
    let sum: u128 = weights.iter().sum();
    if total == 0 {
        return vec![0; weights.len()];
    }
    assert!(sum > 0, "weights to apportion over");
    let mut parts: Vec<u128> = weights.iter().map(|w| total * w / sum).collect();
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(total * weights[i] % sum), i));
    let left = total - parts.iter().sum::<u128>();
    for &i in order.iter().take(usize::try_from(left).expect("few")) {
        parts[i] += 1;
    }
    parts
        .into_iter()
        .map(|p| u64::try_from(p).expect("fits"))
        .collect()
}

/// The unit's room: `max_undeclared_share` of what it can hide (0 for a public unit).
fn room(f: usize) -> u64 {
    if !is_cohort(f) {
        return 0;
    }
    let a = amounts()[f];
    let m = margins();
    let va = u64::try_from(a.value_added.max(0)).expect("non-negative");
    let full_surplus =
        u64::try_from((a.value_added - i64::try_from(a.wages).expect("fits")).max(0))
            .expect("non-negative");
    let surplus = full_surplus * u64::from(m.surplus_from_sales_ppm[usize::from(group(f))]) / U;
    let hideable = (va + surplus).saturating_sub(full_surplus);
    u64::try_from(u128::from(hideable) * u128::from(m.max_undeclared_share_ppm) / u128::from(U))
        .expect("fits")
}

/// `num / den` as millionths, rounded down; 0 when `den` is 0.
fn ppm(num: u64, den: u64) -> u32 {
    if den == 0 {
        0
    } else {
        u32::try_from(u128::from(num) * u128::from(U) / u128::from(den)).expect("a share")
    }
}

/// `a / b` and `c / d` differ by at most `points` hundredths.
fn within_points(a: u128, b: u128, c: u128, d: u128, points: u128) -> bool {
    let (x, y) = (a * d, c * b);
    x.abs_diff(y) * 100 <= points * b * d
}

/// AC-FUI-01 [unit] Structure: for every unit the bounds of the Outputs table hold; public units have every column zero; only domestic units carry household income; `inventory` + `inventory_materials` is zero for units of industries with zero days.
#[test]
fn ac_fui_01() {
    let c = columns();
    let n = n_units();
    for col in [
        &c.undeclared_wages,
        &c.undeclared_value_added,
        &c.undeclared_sales,
        &c.inventory,
        &c.inventory_materials,
    ] {
        assert_eq!(col.len(), n, "one entry per unit");
    }
    for f in 0..n {
        let a = amounts()[f];
        let id = &firms_json()["units"][f]["id"];
        assert!(c.undeclared_wages[f] <= a.wages, "{id}: undeclared wages");
        assert!(c.undeclared_value_added[f] >= c.undeclared_wages[f], "{id}");
        assert!(
            c.undeclared_value_added[f] <= room(f),
            "{id}: at most the room"
        );
        assert!(c.undeclared_sales[f] <= a.output, "{id}: undeclared sales");
        assert_eq!(
            c.undeclared_wage_share_ppm[f],
            ppm(c.undeclared_wages[f], a.wages),
            "{id}"
        );
        assert_eq!(
            c.undeclared_sales_share_ppm[f],
            ppm(c.undeclared_sales[f], a.output),
            "{id}"
        );
        if !is_cohort(f) {
            for (name, v) in [
                ("undeclared_wages", c.undeclared_wages[f]),
                ("undeclared_value_added", c.undeclared_value_added[f]),
                ("undeclared_sales", c.undeclared_sales[f]),
                ("inventory", c.inventory[f]),
                ("inventory_materials", c.inventory_materials[f]),
            ] {
                assert_eq!(v, 0, "{id}: public units have {name} zero");
            }
            assert_eq!(c.undeclared_wage_share_ppm[f], 0);
            assert_eq!(c.undeclared_sales_share_ppm[f], 0);
        }
        let days = margins().industries[usize::from(firms().units[f].industry)].inventory_days;
        if days == 0 {
            assert_eq!(
                stock(c, f),
                0,
                "{id}: an industry with zero days holds no stock"
            );
        }
    }
    // Household income goes to domestic units only. Without undeclared profit
    // in foreign-controlled units, what they hide is their undeclared wages.
    let mut m = margins().clone();
    m.profit_tilt_ppm[3] = 0;
    let c = firm_opening(firms(), amounts(), &m).expect("still places");
    for f in (0..n).filter(|&f| class(f) == Some(3)) {
        assert_eq!(
            c.undeclared_value_added[f],
            c.undeclared_wages[f],
            "{}: no household income in a foreign-controlled unit",
            firms_json()["units"][f]["id"]
        );
    }
}

/// AC-FUI-02 [unit] Totals hit exactly: undeclared value added equals the anchor's target; each group's undeclared wages equal ⌊group wage bill × Stage I's share / U⌋; the household part equals the three amounts of the margins; inventories equal `corporations` + `household_business`, and the household part lies in domestic micro units only.
#[test]
fn ac_fui_02() {
    let c = columns();
    let m = margins();
    let n = n_units();
    let cohort_va: u128 = (0..n)
        .filter(|&f| is_cohort(f))
        .map(|f| u128::from(u64::try_from(amounts()[f].value_added.max(0)).expect("fits")))
        .sum();
    let target = cohort_va * u128::from(m.undeclared_value_added_share_ppm) / u128::from(U);
    let total_uva: u128 = c
        .undeclared_value_added
        .iter()
        .map(|&x| u128::from(x))
        .sum();
    assert_eq!(total_uva, target, "the anchor's target, to the leu");

    let groups = m.undeclared_wage_share_ppm.len();
    for g in 0..groups {
        let members: Vec<usize> = (0..n)
            .filter(|&f| is_cohort(f) && usize::from(group(f)) == g)
            .collect();
        let bill: u128 = members
            .iter()
            .map(|&f| u128::from(amounts()[f].wages))
            .sum();
        let want = bill * u128::from(m.undeclared_wage_share_ppm[g]) / u128::from(U);
        let got: u128 = members
            .iter()
            .map(|&f| u128::from(c.undeclared_wages[f]))
            .sum();
        assert_eq!(got, want, "undeclared wages of group {g}");
    }
    // The household part: Step 3 adds only undeclared profit, which is never
    // negative, so what is hidden besides wages covers the three amounts.
    let household = u128::from(m.undeclared_farm_income)
        + u128::from(m.undeclared_self_employment_income)
        + u128::from(m.informal_income);
    let total_uw: u128 = c.undeclared_wages.iter().map(|&x| u128::from(x)).sum();
    assert!(
        total_uva - total_uw >= household,
        "the household part is placed"
    );

    // Inventories: the corporations' part over every cohort unit by output ×
    // days; the rest of each unit's stock is households' and lies in domestic
    // micro units only.
    let key: Vec<u128> = (0..n)
        .map(|f| {
            if is_cohort(f) {
                let days =
                    margins().industries[usize::from(firms().units[f].industry)].inventory_days;
                u128::from(amounts()[f].output) * u128::from(days)
            } else {
                0
            }
        })
        .collect();
    let corporations = apportion(m.inventories_corporations, &key);
    let mut household_stock = 0u64;
    for f in 0..n {
        let rest = stock(c, f)
            .checked_sub(corporations[f])
            .expect("a unit holds at least its corporations' part");
        let domestic_micro = class(f) == Some(0);
        if !domestic_micro {
            assert_eq!(
                rest,
                0,
                "{}: households' stock only in domestic micro units",
                firms_json()["units"][f]["id"]
            );
        }
        household_stock += rest;
    }
    assert_eq!(household_stock, m.inventories_household_business);
    let total: u64 = (0..n).map(|f| stock(c, f)).sum();
    assert_eq!(
        total,
        m.inventories_corporations + m.inventories_household_business
    );
}

/// AC-FUI-03 [unit] Consistency with Stage I: the undeclared share of each industry group's wage bill equals Stage I's `undeclared_wage_share` within one millionth (rounding), and undeclared value added is at least undeclared wages plus households' undeclared income.
#[test]
fn ac_fui_03() {
    let c = columns();
    let m = margins();
    let n = n_units();
    for g in 0..m.undeclared_wage_share_ppm.len() {
        let members: Vec<usize> = (0..n)
            .filter(|&f| is_cohort(f) && usize::from(group(f)) == g)
            .collect();
        let bill: u128 = members
            .iter()
            .map(|&f| u128::from(amounts()[f].wages))
            .sum();
        let hidden: u128 = members
            .iter()
            .map(|&f| u128::from(c.undeclared_wages[f]))
            .sum();
        let share = u128::from(m.undeclared_wage_share_ppm[g]);
        // |hidden / bill − share / U| ≤ 1 / U
        assert!(
            (hidden * u128::from(U)).abs_diff(share * bill) <= bill,
            "group {g}: undeclared share of the wage bill"
        );
    }
    let total_uva: u64 = c.undeclared_value_added.iter().sum();
    let total_uw: u64 = c.undeclared_wages.iter().sum();
    assert!(
        total_uva
            >= total_uw
                + m.undeclared_farm_income
                + m.undeclared_self_employment_income
                + m.informal_income
    );
}

/// AC-FUI-04 [unit] Consistency with the population, at 1:1000 and 1:100, seed 42: for micro, small, large and foreign-controlled units, the undeclared share of the class's wage bill in Stages I and J (employees' `wage` − `wage_declared`, weighted, by their employer's class) is within 2 points of the firm table's.
#[test]
fn ac_fui_04() {
    let c = columns();
    let n = n_units();
    let mut table_paid = [0u128; 4];
    let mut table_hidden = [0u128; 4];
    for f in 0..n {
        if let Some(k) = class(f) {
            table_paid[k] += u128::from(amounts()[f].wages);
            table_hidden[k] += u128::from(c.undeclared_wages[f]);
        }
    }
    for scale in [1000, 100] {
        let w = workers(scale);
        let mut paid = [0u128; 4];
        let mut hidden = [0u128; 4];
        for p in 0..w.employer.len() {
            if w.employer[p] == NO_EMPLOYER || w.wage[p] == 0 {
                continue;
            }
            if let Some(k) = class(usize::from(w.employer[p])) {
                paid[k] += u128::from(w.weight[p]) * u128::from(w.wage[p]);
                hidden[k] += u128::from(w.weight[p]) * u128::from(w.wage[p] - w.wage_declared[p]);
            }
        }
        for (k, name) in ["micro", "small", "large", "foreign-controlled"]
            .iter()
            .enumerate()
        {
            assert!(paid[k] > 0, "1:{scale}: {name} units have employees");
            assert!(
                within_points(hidden[k], paid[k], table_hidden[k], table_paid[k], 2),
                "1:{scale} {name}: population {hidden:?}/{paid:?} against the table {table_hidden:?}/{table_paid:?}"
            );
        }
    }
}

/// AC-FUI-05 [unit] Anchors and shape on the fixtures: the undeclared share of private value added is 27.1% to the leu; the VAT gap from hidden sales is between 15% and 36.7%; the undeclared sales share falls from micro to small to large to foreign-controlled.
#[test]
fn ac_fui_05() {
    let c = columns();
    let n = n_units();
    assert_eq!(margins().undeclared_value_added_share_ppm, 271_000);
    let private_va: u128 = (0..n)
        .filter(|&f| is_cohort(f))
        .map(|f| u128::from(u64::try_from(amounts()[f].value_added.max(0)).expect("fits")))
        .sum();
    let hidden: u128 = c
        .undeclared_value_added
        .iter()
        .map(|&x| u128::from(x))
        .sum();
    assert_eq!(
        hidden,
        private_va * 271_000 / u128::from(U),
        "27.1% to the leu"
    );

    // The VAT gap from hidden sales: the share of households' purchases bought
    // undeclared, each unit taking its industry's purchases in proportion to
    // its output (the reference's proxy).
    let industries = margins().industries.len();
    let mut out_by_industry = vec![0u128; industries];
    for f in 0..n {
        out_by_industry[usize::from(firms().units[f].industry)] += u128::from(amounts()[f].output);
    }
    let (mut lost, mut purchases) = (0f64, 0f64);
    for (i, im) in margins().industries.iter().enumerate() {
        purchases += im.household_purchases as f64;
        if out_by_industry[i] == 0 {
            continue;
        }
        for f in (0..n).filter(|&f| usize::from(firms().units[f].industry) == i) {
            if amounts()[f].output > 0 {
                let share_of_industry = amounts()[f].output as f64 / out_by_industry[i] as f64;
                let hidden_share = c.undeclared_sales[f] as f64 / amounts()[f].output as f64;
                lost += im.household_purchases as f64 * share_of_industry * hidden_share;
            }
        }
    }
    let gap = lost / purchases;
    assert!(
        (0.15..=0.367).contains(&gap),
        "VAT gap from hidden sales {gap}"
    );

    let mut sales = [0u128; 4];
    let mut output = [0u128; 4];
    for f in 0..n {
        if let Some(k) = class(f) {
            sales[k] += u128::from(c.undeclared_sales[f]);
            output[k] += u128::from(amounts()[f].output);
        }
    }
    for k in 0..3 {
        // sales[k] / output[k] > sales[k+1] / output[k+1]
        assert!(
            sales[k] * output[k + 1] > sales[k + 1] * output[k],
            "the undeclared sales share falls from class {k} to {}",
            k + 1
        );
    }
}

/// AC-FUI-06 [unit] Inventories shape on the fixtures: days of output held are highest in agriculture and trade and below 15 in information, finance, professional services and public services; units of motor, wholesale and retail trade hold less than 10% of their stock as own output.
#[test]
fn ac_fui_06() {
    let c = columns();
    let n = n_units();
    let groups = margins().undeclared_wage_share_ppm.len();
    let mut held = vec![0u128; groups];
    let mut output = vec![0u128; groups];
    for f in 0..n {
        let g = usize::from(group(f));
        held[g] += u128::from(stock(c, f));
        output[g] += u128::from(amounts()[f].output);
    }
    // Days held: stock × 365 / a year's output.
    let days = |g: u8| -> f64 {
        let g = usize::from(g);
        held[g] as f64 * 365.0 / output[g] as f64
    };
    for g in 0..u8::try_from(groups).expect("few groups") {
        if g != AGRICULTURE && g != TRADE_TRANSPORT_HOSPITALITY {
            for top in [AGRICULTURE, TRADE_TRANSPORT_HOSPITALITY] {
                assert!(
                    days(top) > days(g),
                    "group {top} holds more days than group {g}"
                );
            }
        }
    }
    for g in [INFORMATION, FINANCE, PROFESSIONAL, PUBLIC_EDUCATION_HEALTH] {
        assert!(days(g) < 15.0, "group {g}: {} days", days(g));
    }
    let mut trade_units = 0;
    for f in 0..n {
        if ["vehicle_trade_repair", "wholesale_trade", "retail_trade"].contains(&industry_id(f))
            && stock(c, f) > 0
        {
            assert!(
                c.inventory[f] * 10 < stock(c, f),
                "{}: own output under 10% of the stock",
                firms_json()["units"][f]["id"]
            );
            trade_units += 1;
        }
    }
    assert!(trade_units > 0, "trade units hold stock");
}

/// AC-FUI-07 [unit] Purity: the same inputs give the same hash, and the input files are unchanged. With every undeclared share, amount and the anchor at zero, every undeclared column is zero and the inventories are unchanged. An anchor of 15% gives `AnchorsInconsistent`; `max_undeclared_share` of 50% gives `NoRoom`.
#[test]
fn ac_fui_07() {
    let (f0, a0, m0) = (firms().clone(), amounts().clone(), margins().clone());
    let before = (
        std::fs::read(fixture_path(FIRMS_FIXTURE)).expect("firm file"),
        std::fs::read(fixture_path(MARGINS_FIXTURE)).expect("margin file"),
    );
    let first = firm_opening(&f0, &a0, &m0).expect("places");
    let second = firm_opening(&f0, &a0, &m0).expect("places");
    assert_eq!(first.state_hash(), second.state_hash());
    assert_eq!(first, second);
    assert_eq!(
        (&f0, &a0, &m0),
        (firms(), amounts(), margins()),
        "the inputs are unchanged"
    );
    let after = (
        std::fs::read(fixture_path(FIRMS_FIXTURE)).expect("firm file"),
        std::fs::read(fixture_path(MARGINS_FIXTURE)).expect("margin file"),
    );
    assert!(before == after, "the input files are unchanged");

    let mut zero = m0.clone();
    zero.undeclared_wage_share_ppm
        .iter_mut()
        .for_each(|s| *s = 0);
    zero.undeclared_farm_income = 0;
    zero.undeclared_self_employment_income = 0;
    zero.informal_income = 0;
    zero.undeclared_value_added_share_ppm = 0;
    let z = firm_opening(&f0, &a0, &zero).expect("nothing to hide places");
    for col in [
        &z.undeclared_wages,
        &z.undeclared_value_added,
        &z.undeclared_sales,
    ] {
        assert!(col.iter().all(|&x| x == 0));
    }
    assert!(z.undeclared_wage_share_ppm.iter().all(|&x| x == 0));
    assert!(z.undeclared_sales_share_ppm.iter().all(|&x| x == 0));
    assert_eq!(z.inventory, first.inventory);
    assert_eq!(z.inventory_materials, first.inventory_materials);

    let anchor_15 = with(&m0, |m| m.undeclared_value_added_share_ppm = 150_000);
    assert_eq!(
        firm_opening(&f0, &a0, &anchor_15),
        Err(FirmOpeningError::AnchorsInconsistent)
    );
    let max_50 = with(&m0, |m| m.max_undeclared_share_ppm = 500_000);
    assert_eq!(
        firm_opening(&f0, &a0, &max_50),
        Err(FirmOpeningError::NoRoom)
    );
}

/// A changed copy of margins.
fn with(
    m: &FirmOpeningMargins,
    change: impl FnOnce(&mut FirmOpeningMargins),
) -> FirmOpeningMargins {
    let mut m = m.clone();
    change(&mut m);
    m
}

/// AC-FUI-08 [golden] The hash of the fixtures equals the committed golden value.
#[test]
fn ac_fui_08() {
    assert_eq!(golden("fixtures"), "fc869fc9686d1dc1", "the spec's value");
    assert_eq!(hex(columns().state_hash()), golden("fixtures"));
}

/// AC-FUI-09 [diff] The Rust function and the Python reference produce identical columns on the fixtures and on three changed margin files: the anchor at 20%, every tilt at 1, `max_undeclared_share` at 75%.
#[test]
fn ac_fui_09() {
    let mut cases = vec![("fixtures", margins_json().clone())];
    cases.extend(variants());
    for (name, v) in cases {
        let rust = firm_opening(firms(), amounts(), &parse_margins(&v).expect("reads"))
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let r = reference(name, &v);
        let rows = r["columns"].as_array().expect("the reference's columns");
        assert_eq!(rows.len(), n_units(), "{name}: units");
        let num = |row: &Value, k: &str| row[k].as_u64().expect("a non-negative integer");
        for (f, row) in rows.iter().enumerate() {
            let id = &row["id"];
            assert_eq!(
                rust.undeclared_wages[f],
                num(row, "undeclared_wages"),
                "{name} {id}"
            );
            assert_eq!(
                rust.undeclared_value_added[f],
                num(row, "undeclared_value_added"),
                "{name} {id}"
            );
            assert_eq!(
                rust.undeclared_sales[f],
                num(row, "undeclared_sales"),
                "{name} {id}"
            );
            assert_eq!(
                u64::from(rust.undeclared_wage_share_ppm[f]),
                num(row, "undeclared_wage_share"),
                "{name} {id}"
            );
            assert_eq!(
                u64::from(rust.undeclared_sales_share_ppm[f]),
                num(row, "undeclared_sales_share"),
                "{name} {id}"
            );
            assert_eq!(rust.inventory[f], num(row, "inventory"), "{name} {id}");
            assert_eq!(
                rust.inventory_materials[f],
                num(row, "inventory_materials"),
                "{name} {id}"
            );
        }
        assert_eq!(
            Value::from(hex(rust.state_hash())),
            r["hash"],
            "{name}: hash"
        );
        assert_eq!(hex(rust.state_hash()), golden(name), "{name}: golden");
    }
}

/// AC-FUI-10 [unit] Empty units, industries that differ from the firm file's, a unit of an unlisted industry, a tilt table of the wrong length and a share above one are each rejected with a named error and produce no output.
#[test]
fn ac_fui_10() {
    let (f0, a0, m0) = (firms(), amounts(), margins());

    // Empty units.
    let empty = FirmUnits {
        industry_group: f0.industry_group.clone(),
        units: Vec::new(),
    };
    assert_eq!(
        firm_opening(&empty, &[], m0),
        Err(FirmOpeningError::EmptyTable)
    );

    // Industries that differ from the firm file's: one renamed, one missing.
    let renamed = changed(|v| v["industries"][0]["id"] = Value::from("not_an_industry"));
    assert_eq!(
        parse_margins(&renamed),
        Err(FirmOpeningError::ShapeMismatch)
    );
    let dropped = changed(|v| {
        v["industries"].as_array_mut().expect("industries").pop();
    });
    assert_eq!(
        parse_margins(&dropped),
        Err(FirmOpeningError::ShapeMismatch)
    );
    let mut short = m0.clone();
    short.industries.pop();
    assert_eq!(
        firm_opening(f0, a0, &short),
        Err(FirmOpeningError::ShapeMismatch)
    );

    // A unit of an unlisted industry.
    let mut unlisted = f0.clone();
    unlisted.units[0].industry = u8::try_from(f0.industry_group.len()).expect("few industries");
    assert_eq!(
        firm_opening(&unlisted, a0, m0),
        Err(FirmOpeningError::ShapeMismatch)
    );

    // Amounts that do not match the units, and group tables of the wrong length.
    assert_eq!(
        firm_opening(f0, &a0[1..], m0),
        Err(FirmOpeningError::ShapeMismatch)
    );
    let mut groups = m0.clone();
    groups.undeclared_wage_share_ppm.pop();
    assert_eq!(
        firm_opening(f0, a0, &groups),
        Err(FirmOpeningError::ShapeMismatch)
    );

    // A tilt table of the wrong length.
    for k in ["wage_tilt", "informal_tilt", "profit_tilt"] {
        let three = changed(|v| v[k] = Value::from(vec![1_000_000; 3]));
        assert_eq!(
            parse_margins(&three),
            Err(FirmOpeningError::ShapeMismatch),
            "{k}"
        );
        let five = changed(|v| v[k] = Value::from(vec![1_000_000; 5]));
        assert_eq!(
            parse_margins(&five),
            Err(FirmOpeningError::ShapeMismatch),
            "{k}"
        );
    }

    // A share above one.
    let mut above = m0.clone();
    above.undeclared_wage_share_ppm[0] = 1_000_001;
    assert_eq!(
        firm_opening(f0, a0, &above),
        Err(FirmOpeningError::ShareAboveOne)
    );
    let mut above = m0.clone();
    above.max_undeclared_share_ppm = 1_000_001;
    assert_eq!(
        firm_opening(f0, a0, &above),
        Err(FirmOpeningError::ShareAboveOne)
    );
    let mut above = m0.clone();
    above.industries[0].finished_share_ppm = 1_000_001;
    assert_eq!(
        firm_opening(f0, a0, &above),
        Err(FirmOpeningError::ShareAboveOne)
    );

    // Not a margin file at all.
    assert!(matches!(
        parse_margins(&Value::from(vec![1, 2, 3])),
        Err(FirmOpeningError::Parse(_))
    ));
}
