//! Acceptance tests for AC-EUF-01 to 05 (spec: economy/eu-funds), the M3
//! criteria, on the `World` under `Law::law_2021()` with the feature
//! `revenue`. AC-EUF-03 and 05 also need `trade`.
//!
//! Written in a test-authoring session (ADR-0014) from the spec only: the
//! code came first, by the owner's decision of 2026-10-21. The tests read
//! these test writer's additions, declared from the spec in
//! `econ_core::world`:
//! - the month rule of the API sketch (`eu_month`);
//! - each unit's receipts of the day (`World::revenue_today`);
//! - the closed months of revenue (`World::revenue_months`).
//!
//! The yearly amounts are the spec's table. The spec places them in the
//! budget fixture's `eu_receipts`, which the fixture does not have yet.

use econ_core::world::{World, eu_month};
use econ_io::budget::Budget;
use econ_ledger::{FlowCode, Instrument, Sector};
use econ_types::Bani;

use super::support::{start_state, step, total};
use super::support_m3::{FlowSum, code, legs_of_day, legs_sum, world_2021};
use super::support_rev::{
    FARM_INDUSTRIES, assert_checks, bn, gap_ppm, industry, million_lei, received,
};

/// The spec's yearly amounts, million lei: current grants, capital grants,
/// direct payments to farmers.
const GRANTS_CURRENT: f64 = 6_550.2;
const GRANTS_CAPITAL: f64 = 16_017.5;
const AGRI_PAYMENTS: f64 = 9_806.0;

/// The exchange-rate index `e / e0` of the latest day, millionths; 1 with no
/// exchange rate on the `World`.
fn rate_index_ppm(w: &World) -> u32 {
    #[cfg(feature = "trade")]
    {
        let f = w.foreign();
        u32::try_from(
            (u64::from(f.exchange_rate_ppm) * 1_000_000 + u64::from(f.start_rate_ppm) / 2)
                / u64::from(f.start_rate_ppm),
        )
        .expect("fits")
    }
    #[cfg(not(feature = "trade"))]
    {
        let _ = w;
        1_000_000
    }
}

/// The three codes and their yearly amounts.
fn lines() -> [(FlowCode, Bani); 3] {
    [
        (
            code("transfer.eu_grants_current"),
            million_lei(GRANTS_CURRENT),
        ),
        (
            code("transfer.eu_grants_capital"),
            million_lei(GRANTS_CAPITAL),
        ),
        (code("subsidy.eu_agri"), million_lei(AGRI_PAYMENTS)),
    ]
}

/// `n / d` rounded half away from zero.
fn round_div(n: i128, d: i128) -> i128 {
    if n >= 0 {
        (n + d / 2) / d
    } else {
        -((-n + d / 2) / d)
    }
}

/// **AC-EUF-01** `[unit]` A year's amount becomes twelve monthly amounts that
/// add up exactly to it at an exchange-rate index of 1. At an index `r`, each
/// month's amount is the monthly amount times `r`, rounded half away from
/// zero.
///
/// "Monthly amounts" are budget-2021's rule (`Budget::monthly`), as step 1
/// says.
#[test]
fn ac_euf_01() {
    for per_year in [
        Bani(0),
        Bani(1),
        Bani(11),
        Bani(13),
        million_lei(GRANTS_CURRENT),
        million_lei(GRANTS_CAPITAL),
        million_lei(AGRI_PAYMENTS),
        Bani(123_456_789_017),
    ] {
        let mut sum = Bani::ZERO;
        for m in 0..12u8 {
            let v = eu_month(per_year, m, 1_000_000);
            assert_eq!(
                v,
                Budget::monthly(per_year, m),
                "{per_year:?}, month {m}: budget-2021's rule"
            );
            sum += v;
        }
        assert_eq!(sum, per_year, "{per_year:?}: the twelve add up");
        for r in [
            1_u32, 500_000, 999_999, 1_000_001, 1_100_000, 1_234_567, 2_000_000,
        ] {
            for m in 0..12u8 {
                let base = i128::from(Budget::monthly(per_year, m).get());
                let expected = round_div(base * i128::from(r), 1_000_000);
                assert_eq!(
                    i128::from(eu_month(per_year, m, r).get()),
                    expected,
                    "{per_year:?}, month {m}, index {r}"
                );
            }
        }
    }
}

/// **AC-EUF-02** `[sim]` At every month end on the Romania scenario with the
/// 2021 law:
/// - the government's receipts under `transfer.eu_grants_current` and
///   `transfer.eu_grants_capital` each equal the month's amount, through the
///   three legs;
/// - the rest of the world's deposits fall by the same;
/// - the farm units receive exactly the month's direct payments under
///   `subsidy.eu_agri`, and no other unit receives any;
/// - every ledger check holds.
///
/// The farm units' parts follow their key (their share of the farm units'
/// starting output), within one ban each.
#[test]
fn ac_euf_02() {
    let start = start_state(1000);
    let mut w = world_2021(1000);
    let farm: Vec<usize> = FARM_INDUSTRIES
        .iter()
        .map(|id| industry(&start, id))
        .collect();
    let is_farm = |u: usize| farm.contains(&usize::from(start.unit_industry[u]));
    let farm_output: i128 = (0..start.unit_industry.len())
        .filter(|&u| is_farm(u))
        .map(|u| i128::from(start.unit_output[u].get()))
        .sum();
    assert!(farm_output > 0);
    let [current, capital, agri] = lines();
    let mut month = 0u8;
    while month < 12 {
        let r = step(&mut w);
        assert_checks(&r);
        let rev = w.revenue_today();
        if !r.date.is_month_end() {
            for (c, _) in [current, capital, agri] {
                for s in Sector::ALL {
                    assert_eq!(
                        received(&w, c, s),
                        Bani::ZERO,
                        "{:?}: {} off a month end",
                        r.date,
                        c.name()
                    );
                }
            }
            assert!(
                rev.unit_eu_agri.iter().all(|&v| v == Bani::ZERO),
                "{:?}: direct payments off a month end",
                r.date
            );
            continue;
        }
        let idx = rate_index_ppm(&w);
        let legs = legs_of_day(&w, &r);
        for (c, per_year) in [current, capital] {
            let amount = eu_month(per_year, month, idx);
            assert!(amount > Bani::ZERO);
            assert_eq!(
                received(&w, c, Sector::Government),
                amount,
                "{:?}: {}",
                r.date,
                c.name()
            );
            assert_eq!(
                legs_sum(
                    &legs,
                    c,
                    Instrument::Deposits,
                    Sector::RestOfWorld,
                    Sector::Banks
                ),
                amount,
                "{:?}: {} leg 1",
                r.date,
                c.name()
            );
            assert_eq!(
                legs_sum(
                    &legs,
                    c,
                    Instrument::Reserves,
                    Sector::Banks,
                    Sector::CentralBank
                ),
                amount,
                "{:?}: {} leg 2",
                r.date,
                c.name()
            );
            assert_eq!(
                legs_sum(
                    &legs,
                    c,
                    Instrument::GovDepositsAtCb,
                    Sector::CentralBank,
                    Sector::Government
                ),
                amount,
                "{:?}: {} leg 3",
                r.date,
                c.name()
            );
            assert_eq!(
                w.ledger().flows().get(c, Sector::RestOfWorld),
                Bani::ZERO - amount,
                "{:?}: {} from the rest of the world",
                r.date,
                c.name()
            );
        }
        let (c, per_year) = agri;
        let amount = eu_month(per_year, month, idx);
        assert_eq!(
            total(&rev.unit_eu_agri),
            amount,
            "{:?}: the farm units' direct payments",
            r.date
        );
        assert_eq!(
            received(&w, c, Sector::Firms),
            amount,
            "{:?}: subsidy.eu_agri to firms",
            r.date
        );
        assert_eq!(
            w.ledger().flows().get(c, Sector::RestOfWorld),
            Bani::ZERO - amount
        );
        assert_eq!(
            legs_sum(
                &legs,
                c,
                Instrument::Deposits,
                Sector::RestOfWorld,
                Sector::Firms
            ),
            amount
        );
        for (u, &v) in rev.unit_eu_agri.iter().enumerate() {
            if !is_farm(u) {
                assert_eq!(v, Bani::ZERO, "{:?}: unit {u} is not a farm unit", r.date);
                continue;
            }
            let share =
                i128::from(amount.get()) * i128::from(start.unit_output[u].get()) / farm_output;
            assert!(
                (i128::from(v.get()) - share).abs() <= 1,
                "{:?}: farm unit {u} receives {v:?}, its key {share}",
                r.date
            );
        }
        month += 1;
    }
}

/// **AC-EUF-03** `[sim]` The trade month report's `capital_account` equals
/// the month's `transfer.eu_grants_capital`. The identity of trade-fx's
/// balance of payments, with the current account read as current and capital
/// account, closes every month end to the ban.
#[cfg(feature = "trade")]
#[test]
fn ac_euf_03() {
    let mut w = world_2021(1000);
    let capital = code("transfer.eu_grants_capital");
    let deposits = |w: &World| {
        w.ledger()
            .balance(Sector::RestOfWorld, Instrument::Deposits)
    };
    let bonds = |w: &World| w.ledger().balance(Sector::RestOfWorld, Instrument::Bonds);
    let fa = |w: &World| {
        w.ledger()
            .balance(Sector::CentralBank, Instrument::ForeignAssets)
    };
    let (mut d0, mut b0, mut f0) = (deposits(&w), bonds(&w), fa(&w));
    let mut month = FlowSum::default();
    let mut closed = 0;
    while closed < 12 {
        step(&mut w);
        month.add_day(&w);
        if w.trade_months().len() == closed {
            continue;
        }
        let m = &w.trade_months()[closed];
        let grants = month.get(capital, Sector::Government);
        assert!(grants > Bani::ZERO, "{:?}: no capital grants", m.month);
        assert_eq!(
            m.capital_account, grants,
            "{:?}: the capital account",
            m.month
        );
        let ca = month
            .of(Sector::RestOfWorld)
            .into_iter()
            .filter(|&(c, _)| !c.name().starts_with("finance."))
            .map(|(_, v)| Bani::ZERO - v)
            .fold(Bani::ZERO, |a, b| a + b);
        assert_eq!(
            m.current_account, ca,
            "{:?}: current and capital account",
            m.month
        );
        let (d1, b1, f1) = (deposits(&w), bonds(&w), fa(&w));
        assert_eq!(
            m.current_account + (d1 - d0) + (b1 - b0) - (f1 - f0),
            Bani::ZERO,
            "{:?}: the balance of payments",
            m.month
        );
        (d0, b0, f0) = (d1, b1, f1);
        month = FlowSum::default();
        closed += 1;
    }
}

/// **AC-EUF-04** `[sim]` The first twelve months' EU receipts are each
/// within 10% of 2021's (6.55, 16.02 and 9.81 billion lei); the difference is
/// the exchange rate. The fiscal report shows the grants as revenue, and the
/// net EU balance (receipts less `transfer.eu_budget`) is reported every
/// month.
#[test]
fn ac_euf_04() {
    let mut w = world_2021(1000);
    let [current, capital, agri] = lines();
    let eu_budget = code("transfer.eu_budget");
    let mut year = FlowSum::default();
    let mut month = FlowSum::default();
    let mut checked = 0;
    while checked < 12 {
        step(&mut w);
        year.add_day(&w);
        month.add_day(&w);
        if w.revenue_months().len() == checked {
            continue;
        }
        let rm = &w.revenue_months()[checked];
        let fm = &w.fiscal_months()[checked];
        assert_eq!(rm.month, fm.month);
        for (c, _) in [current, capital] {
            let got = month.get(c, Sector::Government);
            assert!(
                fm.revenue.contains(&(c, got)),
                "{:?}: {} {got:?} is not in the fiscal report's revenue",
                fm.month,
                c.name()
            );
        }
        let receipts = month.get(current.0, Sector::Government)
            + month.get(capital.0, Sector::Government)
            + month.get(agri.0, Sector::Firms);
        let own_resources = Bani::ZERO - month.get(eu_budget, Sector::Government);
        assert!(
            own_resources > Bani::ZERO,
            "{:?}: own resources paid",
            fm.month
        );
        assert_eq!(
            rm.eu_net_balance,
            receipts - own_resources,
            "{:?}: the net EU balance",
            fm.month
        );
        month = FlowSum::default();
        checked += 1;
    }
    eprintln!("AC-EUF-04, the first year, billion lei (2021):");
    for ((c, per_year), sector) in [current, capital, agri].into_iter().zip([
        Sector::Government,
        Sector::Government,
        Sector::Firms,
    ]) {
        let got = year.get(c, sector);
        eprintln!("  {}: {:.2} ({:.2})", c.name(), bn(got), bn(per_year));
        assert!(
            gap_ppm(got, per_year) <= 100_000,
            "{}: {got:?} against {per_year:?}",
            c.name()
        );
    }
}

/// **AC-EUF-05** `[sim]` A permanent 10% depreciation (the world-constants
/// edit of trade-fx's depreciation test) raises the lei value of the next
/// month's EU payments by 10%, within one ban per payment.
///
/// The edit is `StartEdit::WorldPrices` at 1.1; each payment is compared
/// with 1.1 times the same payment in the base world at the first month end.
#[cfg(feature = "trade")]
#[test]
fn ac_euf_05() {
    use super::support::params;
    use super::support_m3::world_2021_with;
    use econ_core::world::StartEdit;
    let mut base = world_2021(1000);
    let mut dear = world_2021_with(
        1000,
        vec![StartEdit::WorldPrices {
            factor_ppm: 1_100_000,
        }],
        params(),
    );
    let pay = |w: &mut World| {
        loop {
            let r = step(w);
            if r.date.is_month_end() {
                let [current, capital, agri] = lines();
                return (
                    received(w, current.0, Sector::Government),
                    received(w, capital.0, Sector::Government),
                    w.revenue_today().unit_eu_agri.clone(),
                    received(w, agri.0, Sector::Firms),
                );
            }
        }
    };
    let (bc, bk, bu, ba) = pay(&mut base);
    let (dc, dk, du, da) = pay(&mut dear);
    let tenth = |b: Bani| i128::from(b.get()) * 11;
    // One ban per payment: each grant is one, the direct payments one per
    // farm unit.
    let farm_units = i128::try_from(bu.iter().filter(|&&v| v != Bani::ZERO).count()).expect("fits");
    for (name, b, d, payments) in [
        ("current grants", bc, dc, 1),
        ("capital grants", bk, dk, 1),
        ("direct payments", ba, da, farm_units),
    ] {
        assert!(b > Bani::ZERO);
        assert!(
            (i128::from(d.get()) * 10 - tenth(b)).abs() <= 10 * payments + 1,
            "{name}: {d:?} with the depreciation, {b:?} without"
        );
    }
    for u in 0..bu.len() {
        assert!(
            (i128::from(du[u].get()) * 10 - tenth(bu[u])).abs() <= 11,
            "farm unit {u}: {:?} against {:?}",
            du[u],
            bu[u]
        );
    }
}
