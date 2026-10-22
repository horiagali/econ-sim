//! Acceptance tests for AC-IMP-01 to 04 (spec: economy/imputed-rent), on the
//! `World` under `Law::law_2021()` with the feature `revenue`; AC-IMP-03
//! also needs `investment`, for profit.
//!
//! Written in a test-authoring session (ADR-0014) from the spec only: the
//! code came first, by the owner's decision of 2026-10-21. The payback is on
//! the 15th, for the month before (restated by the owner, 2026-10-21), so
//! AC-IMP-01's "every month end" is read as every payment day. The tests
//! read these test writer's additions, declared from the spec in
//! `econ_core::world`:
//! - the latest payment (`World::imputed_rent`);
//! - a world without the payback (`StartEdit::WithoutRevenue`).

use econ_core::world::{RevenuePart, StartEdit, World};
use econ_ledger::{Instrument, Sector};
use econ_types::Bani;

use super::support::{params, start_state, step, weighted};
use super::support_m3::{code, gdp_2021, legs_of_day, world_2021, world_2021_with};
use super::support_rev::{assert_checks, received};

/// θ, millionths (spec, "The share").
const SHARE_PPM: i128 = 836_500;
/// Owner tenure in the starting state.
const OWNER: u8 = 0;

/// Whether the latest day paid imputed rent.
fn paid_today(w: &World, r: &econ_core::world::DayReport) -> bool {
    w.imputed_rent().day == Some(r.day)
}

/// **AC-IMP-01** `[sim]` At every month end on the Romania scenario with the
/// 2021 law:
/// - the real-estate units' payments under `income.imputed_rent` equal θ
///   times households' month's spending on `real_estate`, less the rounding
///   remainder (under one ban per owner record);
/// - every leu reaches an owner household and none reaches a tenant;
/// - the clearing account nets to zero and every ledger check holds.
///
/// "Under one ban per owner record" is per owner household, with its
/// weight: each record's part is whole bani.
#[test]
fn ac_imp_01() {
    let start = start_state(1000);
    let mut w = world_2021(1000);
    let rent = code("income.imputed_rent");
    let weights = w.households().hh_weight.clone();
    let owner_weight: i64 = (0..weights.len())
        .filter(|&h| start.hh_tenure[h] == OWNER)
        .map(|h| i64::from(weights[h]))
        .sum();
    let mut payments = 0;
    while w.months().len() < 13 {
        let r = step(&mut w);
        assert_checks(&r);
        let to_households = received(&w, rent, Sector::Households);
        if !paid_today(&w, &r) {
            assert_eq!(
                to_households,
                Bani::ZERO,
                "{:?}: imputed rent on another day",
                r.date
            );
            continue;
        }
        payments += 1;
        assert_eq!(r.date.day, 15, "{:?}: paid on the 15th", r.date);
        let p = w.imputed_rent();
        assert!(
            p.spending > Bani::ZERO,
            "{:?}: no spending on real estate",
            r.date
        );
        let due = (i128::from(p.spending.get()) * SHARE_PPM + 500_000) / 1_000_000;
        assert!(
            (i128::from(p.due.get()) - due).abs() <= 1,
            "{:?}: due {:?}, θ of {:?}",
            r.date,
            p.due,
            p.spending
        );
        let paid = weighted(&weights, &p.received);
        let remainder = p.due - paid;
        assert!(
            remainder >= Bani::ZERO && remainder.get() < owner_weight,
            "{:?}: paid {paid:?} of {:?}",
            r.date,
            p.due
        );
        assert_eq!(
            to_households, paid,
            "{:?}: income.imputed_rent to households",
            r.date
        );
        // From the firms, in deposits.
        let legs = legs_of_day(&w, &r);
        let moved: Bani = legs
            .iter()
            .filter(|l| l.code == rent)
            .map(|l| {
                assert_eq!(
                    (l.payer, l.instrument),
                    (Sector::Firms, Instrument::Deposits),
                    "{:?}: a leg of imputed rent",
                    r.date
                );
                l.amount
            })
            .fold(Bani::ZERO, |a, b| a + b);
        assert!(
            moved >= paid,
            "{:?}: legs {moved:?}, households {paid:?}",
            r.date
        );
        assert_eq!(
            w.ledger().flows().get(rent, Sector::Firms),
            Bani::ZERO - paid,
            "{:?}: paid by the firms",
            r.date
        );
        for (h, &v) in p.received.iter().enumerate() {
            assert!(v >= Bani::ZERO);
            if start.hh_tenure[h] != OWNER {
                assert_eq!(v, Bani::ZERO, "{:?}: household {h} is not an owner", r.date);
            }
        }
    }
    assert!(payments >= 12, "{payments} payments in 13 months");
}

/// **AC-IMP-02** `[sim]` Each owner household's receipt is within one ban
/// per record of its key's share. Each household's `hh_income_received`
/// grows by it.
///
/// The key: the household's dwelling value if an owner, else 0; owners with
/// no dwelling value share one value, within the owners' range. Income is
/// checked for households that did not decide on the day. With `investment`
/// on, dividends are paid the same day, so the income check runs without it.
#[test]
fn ac_imp_02() {
    let start = start_state(1000);
    let mut w = world_2021(1000);
    let weights = w.households().hh_weight.clone();
    let n = weights.len();
    let mut payments = 0;
    while payments < 6 {
        let before = w.households().hh_income_received.clone();
        let r = step(&mut w);
        if !paid_today(&w, &r) {
            continue;
        }
        payments += 1;
        let p = w.imputed_rent();
        assert_eq!(p.key.len(), n);
        let owner_values: Vec<Bani> = (0..n)
            .filter(|&h| start.hh_tenure[h] == OWNER && start.hh_dwelling_value[h] > Bani::ZERO)
            .map(|h| start.hh_dwelling_value[h])
            .collect();
        let (lo, hi) = (
            owner_values.iter().min().copied().unwrap_or(Bani::ZERO),
            owner_values.iter().max().copied().unwrap_or(Bani::ZERO),
        );
        let mut fill = None;
        for h in 0..n {
            let k = p.key[h];
            if start.hh_tenure[h] != OWNER {
                assert_eq!(k, Bani::ZERO, "household {h}: not an owner, key {k:?}");
            } else if start.hh_dwelling_value[h] > Bani::ZERO {
                assert_eq!(
                    k, start.hh_dwelling_value[h],
                    "household {h}: its dwelling value"
                );
            } else {
                assert!(
                    k >= lo && k <= hi,
                    "household {h}: owner with no value gets {k:?}"
                );
                assert_eq!(
                    *fill.get_or_insert(k),
                    k,
                    "owners with no value share one key"
                );
            }
        }
        let keys: i128 = (0..n)
            .map(|h| i128::from(p.key[h].get()) * i128::from(weights[h]))
            .sum();
        let paid = weighted(&weights, &p.received);
        for h in 0..n {
            let share = i128::from(paid.get()) * i128::from(p.key[h].get()) / keys;
            assert!(
                (i128::from(p.received[h].get()) - share).abs() <= 1,
                "{:?}: household {h} receives {:?}, its key's share {share}",
                r.date,
                p.received[h]
            );
        }
        #[cfg(not(feature = "investment"))]
        for h in 0..n {
            if w.households().last_decision[h] == Some(r.day) {
                continue;
            }
            assert_eq!(
                w.households().hh_income_received[h] - before[h],
                p.received[h],
                "{:?}: household {h}'s income received",
                r.date
            );
        }
        let _ = &before;
    }
}

/// **AC-IMP-03** `[sim]` The real-estate units' profit and taxable profit
/// exclude the payback. Their first-year surplus net of depreciation is
/// within 30% of −15.1 billion lei (13.1 billion before depreciation, less
/// their depreciation of about 28.2 billion), and prices of `real_estate`
/// are those of a world without the payback on day 1 (restated by the owner,
/// 2026-10-22: 13.1 billion is gross of depreciation, and the units' profit
/// is net of it).
///
/// The surplus is read as the units' profit before tax `Π^pre` summed over
/// the first twelve closed months (the per-unit figure the `World` shows).
/// "Within 30%" is of the target's size: within 4.53 billion lei of it.
///
/// "Exclude the payback": over the first year, the real-estate units' profit
/// before tax is lower than in the world without it by the payback they paid,
/// within 30% (the worlds' demand differs). Taxable profit is not shown by
/// unit; the profit tax is not checked here.
///
/// Owed to roadmap M7 (owner, 2026-10-22): it waits for real estate's mixed
/// income and surplus.
#[cfg(feature = "investment")]
#[test]
#[ignore = "waits for M7: real estate's mixed income and surplus (owner 2026-10-22)"]
fn ac_imp_03() {
    use super::support_rev::{bn, industry, units_of};
    let start = start_state(1000);
    let re = units_of(&start, industry(&start, "real_estate"));
    let j = industry(&start, "real_estate");
    let rent = code("income.imputed_rent");
    let mut with = world_2021(1000);
    let mut without = world_2021_with(
        1000,
        vec![StartEdit::WithoutRevenue(RevenuePart::ImputedRent)],
        params(),
    );
    step(&mut with);
    step(&mut without);
    assert_eq!(
        with.industries().price_ppm[j],
        without.industries().price_ppm[j],
        "day 1: the price of real estate"
    );
    let mut paid = Bani::ZERO;
    while with.months().len() < 12 {
        step(&mut with);
        paid += Bani::ZERO - with.ledger().flows().get(rent, Sector::Firms);
    }
    while without.months().len() < 12 {
        step(&mut without);
    }
    let surplus = |w: &World| -> Bani {
        w.investment_months()[..12]
            .iter()
            .map(|m| {
                re.iter()
                    .map(|&u| m.profit_before_tax[u])
                    .fold(Bani::ZERO, |a, b| a + b)
            })
            .fold(Bani::ZERO, |a, b| a + b)
    };
    let (s_with, s_without) = (surplus(&with), surplus(&without));
    eprintln!(
        "AC-IMP-03: real-estate surplus net of depreciation {:.2} billion lei (-15.1), {:.2} without the payback; paid {:.2}",
        bn(s_with),
        bn(s_without),
        bn(paid)
    );
    let target = Bani::from_lei(-15_100_000_000);
    assert!(
        (s_with - target).abs().get() * 10 <= target.abs().get() * 3,
        "surplus net of depreciation {s_with:?}, the spec's {target:?}"
    );
    let lower = s_without - s_with;
    assert!(
        (lower - paid).abs().get() * 10 <= paid.get() * 3,
        "profit lower by {lower:?}, payback {paid:?}"
    );
}

/// **AC-IMP-04** `[sim]` Compare two worlds on the Romania scenario with the
/// 2021 law, with and without the payback (a start edit). Over the first
/// twelve months, households' consumption over GDP is higher with the
/// payback, and closer to the table's share.
///
/// The table's share is its households' consumption over 2021's GDP (the
/// budget fixture's).
#[test]
fn ac_imp_04() {
    let mut with = world_2021(1000);
    let mut without = world_2021_with(
        1000,
        vec![StartEdit::WithoutRevenue(RevenuePart::ImputedRent)],
        params(),
    );
    for w in [&mut with, &mut without] {
        while w.months().len() < 12 {
            step(w);
        }
    }
    let share = |w: &World| {
        let (c, y) = w.months()[..12].iter().fold((0.0, 0.0), |(c, y), m| {
            (
                c + m.consumption.get() as f64,
                y + m.gdp_production.get() as f64,
            )
        });
        c / y
    };
    let table = super::support::io_table_json()["totals"]["household_consumption"]
        .as_f64()
        .expect("a number")
        * 100.0
        / gdp_2021().get() as f64;
    let (a, b) = (share(&with), share(&without));
    eprintln!(
        "AC-IMP-04: consumption over GDP {a:.4} with the payback, {b:.4} without; the table {table:.4}"
    );
    assert!(
        a > b,
        "consumption over GDP {a} with the payback, {b} without"
    );
    assert!(
        (a - table).abs() < (b - table).abs(),
        "{a} is not closer to the table's {table} than {b}"
    );
}
