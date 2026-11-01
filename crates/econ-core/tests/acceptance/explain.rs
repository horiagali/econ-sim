//! Acceptance tests for AC-WHY-01 to AC-WHY-07 (spec: game/explanations,
//! draft of 2026-11-01), written by the test writer from the spec and its API
//! sketch before the code (ADR-0014); behind the feature `explain`, against
//! the test writer's API skeleton `econ_core::explain` (`Node`,
//! `World::explain`, `World::explained`, `lmdi`, `round_to_sum`).
//!
//! "1:1000" is the Romania scenario at that scale, seed 42, under the law of
//! its folder (`law.json`: the 2021 law), every parameter at its spec
//! default. Month 0 is December 2021. Indicators are named by their dotted
//! registry names.
//!
//! Readings shared by the tests:
//! - `explain(id, key, from, to)` is the change from month `from`'s point to
//!   month `to`'s; "the change over months 2 to 4" is the window `(1, 4)`
//!   (the monthly trees of months 2, 3 and 4), and a lever "from month 2"
//!   takes effect on its first day, 1 February 2022.
//! - For the year-on-year rates (`gdp.growth_yoy`, `prices.inflation_yoy`),
//!   whose value is itself the change over the year, "the window of the year
//!   to month 12" is `(0, 12)` and its root is their value at month 12.
//! - The trees are of the national value (`Key { breakdown: None, index: 0
//!   }`) unless a criterion names a key.
//! - The children are in the order of the spec's table ("Children of a
//!   month's change") where a criterion compares them with a record.

use econ_core::explain::{Node, lmdi, round_to_sum};
use econ_core::stats::{IndicatorId, Key, Series};
use econ_core::world::World;
use econ_types::Month;

use super::support_m6::{
    NATIONAL, computed, id, meta, name_of, run_closes, step_checked, world_2021_default,
};

/// The indicators the spec's table gives a tree.
const TREES: [&str; 19] = [
    "gdp.nominal",
    "gdp.real",
    "wages.mean_real",
    "gdp.per_capita",
    "gdp.growth_yoy",
    "prices.cpi",
    "prices.inflation_yoy",
    "labour.unemployment_rate",
    "labour.employed",
    "wages.mean_nominal",
    "budget.balance",
    "debt.gdp_ratio",
    "money.policy_rate",
    "fx.eur_lei",
    "approval.national",
    "people.population",
    "people.sol",
    "people.poverty_rate_anchored",
    "people.wellbeing",
];

/// The headline indicators without a tree in M6.
const NO_TREE: [&str; 2] = ["people.gini", "trade.balance"];

/// The year-on-year rates.
const YOY: [&str; 2] = ["gdp.growth_yoy", "prices.inflation_yoy"];

/// The monthly point of a series at month `t`, if it has one.
fn point(s: &Series<'_>, t: u32) -> Option<i64> {
    t.checked_sub(s.first_month.0)
        .and_then(|i| s.months.get(usize::try_from(i).expect("an index")).copied())
}

/// The point of `id` at `key` and month `t`; panics when there is none.
fn value(w: &World, id: IndicatorId, key: Key, t: u32) -> i64 {
    let s = w
        .stats()
        .series(id, key)
        .unwrap_or_else(|| panic!("{} {key:?}: no series", name_of(id)));
    point(&s, t).unwrap_or_else(|| panic!("{} {key:?}: no point at month {t}", name_of(id)))
}

/// The tree of a change; panics when there is none.
fn tree(w: &World, id: IndicatorId, key: Key, from: u32, to: u32) -> Node {
    w.explain(id, key, Month(from), Month(to))
        .unwrap_or_else(|| panic!("{} {key:?} ({from}, {to}): no tree", name_of(id)))
}

/// Every node with children has children that sum to its value exactly.
fn assert_sums(n: &Node, path: &str) {
    if n.children.is_empty() {
        return;
    }
    let sum: i128 = n.children.iter().map(|c| i128::from(c.value)).sum();
    assert_eq!(
        sum,
        i128::from(n.value),
        "{path}: the children of {:?} sum to {sum}, not {}",
        n.label,
        n.value
    );
    for c in &n.children {
        assert_sums(c, &format!("{path} / {}", c.label));
    }
}

/// The children's values.
#[cfg(any(feature = "poverty", feature = "migration"))]
fn child_values(n: &Node) -> Vec<i64> {
    n.children.iter().map(|c| c.value).collect()
}

fn is_yoy(id: IndicatorId) -> bool {
    YOY.contains(&name_of(id))
}

// --- AC-WHY-01 ------------------------------------------------------------------

/// A deterministic sequence in [0, 1) (no randomness in tests).
fn sequence(n: usize, seed: u64) -> Vec<f64> {
    let mut x = seed;
    (0..n)
        .map(|_| {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            #[allow(clippy::cast_precision_loss)]
            let v = (x >> 11) as f64 / (1u64 << 53) as f64;
            v
        })
        .collect()
}

fn assert_rounded(values: &[f64], total: i64) {
    let r = round_to_sum(values, total);
    assert_eq!(r.len(), values.len(), "{values:?}: one integer per value");
    assert_eq!(
        r.iter().map(|&x| i128::from(x)).sum::<i128>(),
        i128::from(total),
        "{values:?} → {r:?}: the total {total}"
    );
    for (x, v) in r.iter().zip(values) {
        #[allow(clippy::cast_precision_loss)]
        let d = (*x as f64 - v).abs();
        assert!(d <= 1.0, "{values:?} → {r:?}: {x} is not within one of {v}");
    }
}

fn assert_lmdi(y1: f64, y0: f64, parts: &[(f64, f64)]) -> Vec<f64> {
    let c = lmdi(y1, y0, parts);
    assert_eq!(c.len(), parts.len(), "one contribution per part");
    let sum: f64 = c.iter().sum();
    let tol = 1e-9 * y1.abs().max(y0.abs()).max((y1 - y0).abs());
    assert!(
        (sum - (y1 - y0)).abs() <= tol,
        "lmdi({y1}, {y0}, {parts:?}) = {c:?}: sums to {sum}, not {}",
        y1 - y0
    );
    c
}

/// **AC-WHY-01** `[unit]` `round_to_sum` returns integers summing to the
/// total, each within one of its float; `lmdi` contributions sum to
/// `y1 − y0` within 1e-9 relative; a node's children sum to its value.
///
/// Readings: `round_to_sum` on given floats whose sum is the total (halves,
/// thirds, negatives, mixed signs, and 200 deterministic cases of 1 to 40
/// values). `lmdi(y1, y0, parts)` has `y = Π x_k` with `parts[k] = (x_k at
/// 1, x_k at 0)`, so its contribution `k` is `L(y1, y0) · ln(x_k1 / x_k0)`
/// (checked on a case with one factor still, and on a ratio); "within 1e-9
/// relative" is relative to the largest of `|y1|`, `|y0|` and `|y1 − y0|`.
/// "A node's children sum to its value": every node of every tree of
/// `explained()` over the first month, at every depth, after 2 closes at
/// 1:1000 (the year-on-year rates, which have no point yet, left out).
#[test]
fn ac_why_01() {
    assert_rounded(&[1.5, 2.5, 3.0], 7);
    assert_rounded(&[1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0], 1);
    assert_rounded(&[-1.5, -2.5, 4.0], 0);
    assert_rounded(&[-0.6, -0.6, -0.8], -2);
    assert_rounded(&[1e9 + 0.25, -3e8 + 0.5, 0.25], 700_000_001);
    assert_rounded(&[], 0);
    assert_rounded(&[42.0], 42);
    for case in 0..200u64 {
        let n = usize::try_from(case % 40 + 1).expect("a count");
        let raw = sequence(n, case + 1);
        // Values in [−5,000, 5,000) with fractional parts, scaled so that
        // they sum to an integer.
        let mut v: Vec<f64> = raw.iter().map(|x| x * 10_000.0 - 5_000.0).collect();
        let s: f64 = v.iter().sum();
        let fix = s.round() - s;
        v[0] += fix;
        #[allow(clippy::cast_possible_truncation)]
        let total = v.iter().sum::<f64>().round() as i64;
        assert_rounded(&v, total);
    }

    let c = assert_lmdi(150.0, 100.0, &[(15.0, 10.0), (10.0, 10.0)]);
    assert!(
        (c[0] - 50.0).abs() <= 1e-9 && c[1].abs() <= 1e-9,
        "one factor still: {c:?}"
    );
    // value = nominal / deflator: (330 / 1.1) − (200 / 1.0) = 100.
    let c = assert_lmdi(300.0, 200.0, &[(330.0, 200.0), (1.0 / 1.1, 1.0)]);
    let l = 100.0 / econ_num::math::ln(1.5);
    assert!(
        (c[0] - l * econ_num::math::ln(1.65)).abs() <= 1e-9,
        "the nominal part: {c:?}"
    );
    assert!(
        (c[1] - l * econ_num::math::ln(1.0 / 1.1)).abs() <= 1e-9,
        "the deflator's part: {c:?}"
    );
    // No change of the value, with moving parts.
    let c = assert_lmdi(10.0, 10.0, &[(4.0, 2.0), (2.5, 5.0)]);
    assert!(
        (c[0] - 10.0 * econ_num::math::ln(2.0)).abs() <= 1e-9,
        "{c:?}"
    );
    for case in 0..100u64 {
        let k = usize::try_from(case % 5 + 1).expect("a count");
        let a = sequence(k, 1000 + case);
        let b = sequence(k, 2000 + case);
        let parts: Vec<(f64, f64)> = a
            .iter()
            .zip(&b)
            .map(|(x, y)| (0.5 + 3.0 * x, 0.5 + 3.0 * y))
            .collect();
        let y1: f64 = parts.iter().map(|p| p.0).product();
        let y0: f64 = parts.iter().map(|p| p.1).product();
        assert_lmdi(y1, y0, &parts);
    }

    let mut w = world_2021_default(1000);
    run_closes(&mut w, 2);
    let explained = w.explained();
    assert!(!explained.is_empty(), "some indicator has a tree");
    for &i in explained {
        if is_yoy(i) {
            continue;
        }
        let n = tree(&w, i, NATIONAL, 0, 1);
        assert_sums(&n, name_of(i));
    }
}

// --- AC-WHY-02 ------------------------------------------------------------------

/// **AC-WHY-02** `[sim]` At 1:1000, for every indicator of `explained()` and
/// every month `t` of 1 to 12: the tree of (t − 1, t) has the root value
/// `indicator(t) − indicator(t − 1)` exactly and its children sum to it
/// exactly; the same for the window (0, 12) (for `gdp.growth_yoy` and
/// `prices.inflation_yoy`, the window of the year to month 12). With the M5
/// features compiled, the people trees too.
///
/// Readings: 13 closes (months 0 to 12) at 1:1000. `explained()` holds every
/// indicator of the spec's table that is active with its feature compiled
/// (so the people trees exactly when their features are), and neither
/// `people.gini` nor `trade.balance` (whose `explain` is `None`). For each
/// indicator of `explained()`, other than the year-on-year rates: the
/// monthly trees of months 1 to 12 and the window (0, 12) have the root
/// value of the change and children summing to it at every depth; the
/// window's children are the monthly trees' added child by child ("a window
/// of several months adds the monthly trees child by child, the same
/// children each month": same count and labels, values summed). For the two
/// year-on-year rates: the window (0, 12) has the root value of the rate at
/// month 12 and children summing to it.
#[test]
fn ac_why_02() {
    let mut w = world_2021_default(1000);
    run_closes(&mut w, 13);
    let explained = w.explained();
    for name in TREES {
        let m = meta(name);
        assert_eq!(
            explained.contains(&m.id),
            computed(m),
            "{name}: in explained() exactly when computed"
        );
    }
    for name in NO_TREE {
        let i = id(name);
        assert!(!explained.contains(&i), "{name}: no tree in M6");
        assert!(
            w.explain(i, NATIONAL, Month(11), Month(12)).is_none(),
            "{name}: explain is None"
        );
    }
    for &i in explained {
        let name = name_of(i);
        assert!(computed(meta(name)), "{name}: explained but not computed");
        if is_yoy(i) {
            let n = tree(&w, i, NATIONAL, 0, 12);
            assert_eq!(
                n.value,
                value(&w, i, NATIONAL, 12),
                "{name}: the year to month 12"
            );
            assert!(!n.children.is_empty(), "{name}: children");
            assert_sums(&n, name);
            continue;
        }
        let mut monthly = Vec::new();
        for t in 1..=12u32 {
            let n = tree(&w, i, NATIONAL, t - 1, t);
            let change = value(&w, i, NATIONAL, t) - value(&w, i, NATIONAL, t - 1);
            assert_eq!(n.value, change, "{name}, month {t}: the root is the change");
            assert!(!n.children.is_empty(), "{name}, month {t}: children");
            assert_sums(&n, &format!("{name}, month {t}"));
            monthly.push(n);
        }
        let n = tree(&w, i, NATIONAL, 0, 12);
        let change = value(&w, i, NATIONAL, 12) - value(&w, i, NATIONAL, 0);
        assert_eq!(n.value, change, "{name}, (0, 12): the root is the change");
        assert_sums(&n, &format!("{name}, (0, 12)"));
        for (t, m) in monthly.iter().enumerate() {
            let labels: Vec<&str> = m.children.iter().map(|c| c.label.as_str()).collect();
            let window: Vec<&str> = n.children.iter().map(|c| c.label.as_str()).collect();
            assert_eq!(
                labels,
                window,
                "{name}: month {}'s children are the window's",
                t + 1
            );
        }
        for (k, c) in n.children.iter().enumerate() {
            let sum: i64 = monthly.iter().map(|m| m.children[k].value).sum();
            assert_eq!(
                c.value, sum,
                "{name}, (0, 12): {:?} is the monthly ones added",
                c.label
            );
        }
    }
}

// --- AC-WHY-03 ------------------------------------------------------------------

/// A 1:1000 world with one order given on day 0, effective 1 February 2022
/// (month 2), run for 5 closes (months 0 to 4).
#[cfg(all(feature = "fiscal", feature = "trade"))]
fn run_with_order(
    lever: &str,
    key: Option<&str>,
    value: impl Fn(&econ_core::levers::LeverValue) -> econ_core::levers::LeverValue,
) -> World {
    use super::support_lev::{date, give, key_of, set_on};
    let mut w = world_2021_default(1000);
    let l = w
        .registry()
        .find(lever)
        .unwrap_or_else(|| panic!("no lever {lever}"))
        .id;
    let k = key.map(|k| key_of(&w, l, k));
    let now = w
        .law()
        .get(l, k)
        .unwrap_or_else(|| panic!("{lever}: a value in force"))
        .clone();
    give(&mut w, set_on(l, k, value(&now), date(2022, 2, 1)));
    run_closes(&mut w, 5);
    w
}

/// **AC-WHY-03** `[sim]` Causes: raising `world.fuel_price[oil]` by 50% from
/// month 2 makes the oil goods' children the largest positive contribution
/// to the CPI's change over months 2 to 4; raising the standard VAT rate by
/// 2 points from month 2 makes VAT the largest positive child of the budget
/// balance's change over months 2 to 4.
///
/// Readings: two runs at 1:1000, each with one order given on day 0 and
/// effective 1 February 2022: `world.fuel_price[oil]` at 150% of its value
/// in force; `tax.vat.rate_standard` 200 basis points higher. The window is
/// `(1, 4)` (the file's readings). The oil goods are `crude_oil` and
/// `refined_petroleum`, whose euro price the oil key scales (spec
/// economy/trade-fx); a good's child of the CPI names the good's key
/// (`Breakdown::Good`, the catalogue index) in `indicator`. "The oil goods'
/// children the largest positive contribution": their values added are
/// positive and larger than any other child's. VAT's child of the budget
/// balance is the one whose label names VAT (contains `vat`, any case): it
/// is positive and larger than every other child.
#[test]
#[cfg(all(feature = "fiscal", feature = "trade"))]
fn ac_why_03() {
    use econ_core::levers::LeverValue;
    use econ_core::stats::Breakdown;

    use super::support::industry;

    let w = run_with_order("world.fuel_price", Some("oil"), |v| match v {
        LeverValue::Money(b) => LeverValue::Money(econ_types::Bani(b.get() * 3 / 2)),
        other => panic!("world.fuel_price: money, got {other:?}"),
    });
    let oil: Vec<Key> = ["crude_oil", "refined_petroleum"]
        .iter()
        .map(|g| Key::of(Breakdown::Good, industry(&w, g)))
        .collect();
    let cpi = tree(&w, id("prices.cpi"), NATIONAL, 1, 4);
    assert_sums(&cpi, "prices.cpi (1, 4)");
    let is_oil = |c: &Node| c.indicator.is_some_and(|(_, k)| oil.contains(&k));
    let oil_total: i64 = cpi
        .children
        .iter()
        .filter(|c| is_oil(c))
        .map(|c| c.value)
        .sum();
    assert!(
        cpi.children.iter().any(is_oil),
        "the CPI's tree has the oil goods' children: {:?}",
        cpi.children.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        oil_total > 0,
        "the oil goods' contribution {oil_total} is positive"
    );
    for c in cpi.children.iter().filter(|c| !is_oil(c)) {
        assert!(
            oil_total > c.value,
            "the oil goods' contribution {oil_total} against {:?}'s {}",
            c.label,
            c.value
        );
    }

    let w = run_with_order("tax.vat.rate_standard", None, |v| match v {
        LeverValue::Rate(r) => LeverValue::Rate(r + 200),
        other => panic!("tax.vat.rate_standard: a rate, got {other:?}"),
    });
    let balance = tree(&w, id("budget.balance"), NATIONAL, 1, 4);
    assert_sums(&balance, "budget.balance (1, 4)");
    let vat: Vec<&Node> = balance
        .children
        .iter()
        .filter(|c| c.label.to_lowercase().contains("vat"))
        .collect();
    assert_eq!(
        vat.len(),
        1,
        "one VAT child: {:?}",
        balance
            .children
            .iter()
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
    let v = vat[0].value;
    assert!(v > 0, "VAT's contribution {v} is positive");
    for c in balance
        .children
        .iter()
        .filter(|c| !std::ptr::eq(*c, vat[0]))
    {
        assert!(
            v > c.value,
            "VAT's contribution {v} against {:?}'s {}",
            c.label,
            c.value
        );
    }
}

// --- AC-WHY-04 ------------------------------------------------------------------

/// **AC-WHY-04** `[sim]` The trees agree with their records: the SoL tree's
/// children are the stats' `parts_change_milli`; the approval tree's
/// children are the change of the national group's `parts_milli`; the
/// population tree's children are the migration month's flows.
///
/// Readings: 6 closes at 1:1000, months `t` of 1 to 5, each feature's part
/// when it is compiled. `people.sol` (feature `poverty`): the tree of
/// `(t − 1, t)` has five children (purchases, home-grown food, imputed rent,
/// health, education) equal to `poverty_stats()[t].sol.parts_change_milli`
/// in order. `approval.national` (feature `approval`): five children
/// (income, unemployment, inflation, hardship, bounds) equal to
/// `approval_stats()[t].national.parts_milli[k]` less month `t − 1`'s.
/// `people.population` (feature `migration`): five children (births,
/// resident deaths, emigrants, returns, immigrants) equal to
/// `migration_months()[t]`'s `births`, `−deaths_resident`, `−emigrants`,
/// `returns`, `immigrants` (signed as parts of the change).
#[test]
#[cfg(any(feature = "poverty", feature = "migration"))]
fn ac_why_04() {
    let mut w = world_2021_default(1000);
    run_closes(&mut w, 6);
    for t in 1..=5u32 {
        let ti = usize::try_from(t).expect("an index");
        #[cfg(feature = "poverty")]
        {
            let n = tree(&w, id("people.sol"), NATIONAL, t - 1, t);
            let want = w.poverty_stats()[ti].sol.parts_change_milli.to_vec();
            assert_eq!(
                child_values(&n),
                want,
                "people.sol, month {t}: the parts' change"
            );
        }
        #[cfg(feature = "approval")]
        {
            let s = w.approval_stats();
            let now = s[ti].national.as_ref().expect("adults approve").parts_milli;
            let before = s[ti - 1]
                .national
                .as_ref()
                .expect("adults approve")
                .parts_milli;
            let want: Vec<i64> = now
                .iter()
                .zip(&before)
                .map(|(a, b)| i64::from(*a) - i64::from(*b))
                .collect();
            let n = tree(&w, id("approval.national"), NATIONAL, t - 1, t);
            assert_eq!(
                child_values(&n),
                want,
                "approval.national, month {t}: the parts' change"
            );
        }
        #[cfg(feature = "migration")]
        {
            let m = &w.migration_months()[ti];
            let i = |x: u64| i64::try_from(x).expect("fits");
            let want = vec![
                i(m.births),
                -i(m.deaths_resident),
                -i(m.emigrants),
                i(m.returns),
                i(m.immigrants),
            ];
            let n = tree(&w, id("people.population"), NATIONAL, t - 1, t);
            assert_eq!(
                child_values(&n),
                want,
                "people.population, month {t}: the flows"
            );
        }
        let _ = ti;
    }
}

// --- AC-WHY-05 ------------------------------------------------------------------

/// **AC-WHY-05** `[sim]` A policy rate set by an order: the month's tree of
/// `money.policy_rate` has the child "player's orders" equal to the change.
///
/// Readings: at 1:1000, on 10 February 2022 (month 2) the player sets
/// `money.policy_rate` 150 basis points above the rate of that day; it takes
/// effect the next day and holds (spec game/lever-system, group 8). The
/// month's tree is that of `(1, 2)`; the change is not zero. The child
/// "player's orders" is the one child whose label names the orders
/// (contains `order`, any case).
#[test]
#[cfg(feature = "monetary")]
fn ac_why_05() {
    use econ_core::levers::LeverValue;

    use super::support_lev::{date, give, set, step_to};
    let mut w = world_2021_default(1000);
    step_to(&mut w, date(2022, 2, 10));
    let lever = w
        .registry()
        .find("money.policy_rate")
        .expect("the lever")
        .id;
    let bp = w.central_bank().policy_rate_ppm / 100;
    give(&mut w, set(lever, None, LeverValue::Rate(bp + 150)));
    run_closes(&mut w, 3);
    let rate = id("money.policy_rate");
    let change = value(&w, rate, NATIONAL, 2) - value(&w, rate, NATIONAL, 1);
    assert_ne!(change, 0, "the order changed the month's rate");
    let n = tree(&w, rate, NATIONAL, 1, 2);
    assert_eq!(n.value, change);
    assert_sums(&n, "money.policy_rate (1, 2)");
    let orders: Vec<&Node> = n
        .children
        .iter()
        .filter(|c| c.label.to_lowercase().contains("order"))
        .collect();
    assert_eq!(
        orders.len(),
        1,
        "one child of the player's orders: {:?}",
        n.children.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert_eq!(
        orders[0].value, change,
        "the player's orders are the change"
    );
}

// --- AC-WHY-06 ------------------------------------------------------------------

/// Check a node's children's references, recursively.
fn assert_drill(w: &World, root: IndicatorId, n: &Node, from: u32, to: u32, path: &str) {
    for c in &n.children {
        let here = format!("{path} / {}", c.label);
        if let Some((i, key)) = c.indicator {
            let m = meta(name_of(i));
            assert!(computed(m), "{here}: names {} which is not active", m.name);
            assert!(
                w.stats().keys(i).contains(&key),
                "{here}: names {} {key:?}, a key it does not have",
                m.name
            );
            let change = || value(w, i, key, to) - value(w, i, key, from);
            let root_name = name_of(root);
            if i == root {
                assert_eq!(
                    c.value,
                    change(),
                    "{here}: the indicator itself, {key:?}'s change"
                );
            } else if root_name == "budget.balance" && m.name == "budget.revenue" {
                assert_eq!(c.value, change(), "{here}: the revenue member's change");
            } else if root_name == "budget.balance"
                && (m.name == "budget.spending" || m.name == "budget.interest")
            {
                assert_eq!(
                    c.value,
                    -change(),
                    "{here}: less the spending member's change"
                );
            }
        }
        assert_drill(w, root, c, from, to, &here);
    }
}

/// **AC-WHY-06** `[unit]` Drill-down: every child that names a registry
/// indicator names an active one with that key, and its value equals that
/// indicator's change over the window when the child is the indicator
/// itself (for example the VAT child of `budget.balance` equals
/// `budget.revenue`'s VAT member's change).
///
/// Readings: 3 closes at 1:1000; the trees of every indicator of
/// `explained()` (the year-on-year rates, which have no point yet, left out)
/// over `(0, 1)`, `(1, 2)` and `(0, 2)`, at every depth. "Active" is active
/// with its feature compiled; "with that key" is a key the history has for
/// it. "The child is the indicator itself" is read as: a child naming the
/// root's own indicator at another key (a region's change of
/// `labour.employed`), and, in `budget.balance`'s tree, a child naming a
/// member of `budget.revenue` (its change) or of `budget.spending` or
/// `budget.interest` (less its change, as spending lowers the balance). The
/// budget balance's tree names `budget.revenue`'s members (with `fiscal`).
#[test]
fn ac_why_06() {
    let mut w = world_2021_default(1000);
    run_closes(&mut w, 3);
    for &i in w.explained() {
        if is_yoy(i) {
            continue;
        }
        for (from, to) in [(0, 1), (1, 2), (0, 2)] {
            let n = tree(&w, i, NATIONAL, from, to);
            assert_drill(
                &w,
                i,
                &n,
                from,
                to,
                &format!("{} ({from}, {to})", name_of(i)),
            );
        }
    }
    #[cfg(feature = "fiscal")]
    {
        let revenue = id("budget.revenue");
        let n = tree(&w, id("budget.balance"), NATIONAL, 1, 2);
        assert!(
            n.children
                .iter()
                .any(|c| c.indicator.is_some_and(|(i, _)| i == revenue)),
            "budget.balance's children name budget.revenue's members: {:?}",
            n.children.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }
}

// --- AC-WHY-07 ------------------------------------------------------------------

/// **AC-WHY-07** `[sim]` Isolation: computing trees changes no state (the
/// state hash after 6 closes is the same whether `explain` is called at
/// every close or never).
///
/// Readings: two 1:1000 worlds stepped side by side for 6 closes; in one,
/// after each close's day, the tree of the month's change of every indicator
/// of `explained()` that has points is computed (the year-on-year rates
/// none, before month 12), and the window from month 0. The state hash is
/// compared every day, which includes the one after 6 closes.
#[test]
fn ac_why_07() {
    let mut a = world_2021_default(1000);
    let mut b = world_2021_default(1000);
    let mut trees = 0usize;
    while a.months().len() < 6 {
        let before = a.months().len();
        let ra = step_checked(&mut a);
        let rb = step_checked(&mut b);
        if a.months().len() > before {
            let t = u32::try_from(before).expect("a month");
            if t >= 1 {
                for &i in a.explained() {
                    if is_yoy(i) {
                        continue;
                    }
                    let _ = a.explain(i, NATIONAL, Month(t - 1), Month(t));
                    let _ = a.explain(i, NATIONAL, Month(0), Month(t));
                    trees += 1;
                }
            }
        }
        assert_eq!(
            ra.state_hash, rb.state_hash,
            "{:?}: the state hashes differ",
            ra.date
        );
        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "{:?}: after the trees",
            ra.date
        );
    }
    assert_eq!(a.state_hash(), b.state_hash(), "after 6 closes");
    assert!(trees > 0, "trees were computed");
}
