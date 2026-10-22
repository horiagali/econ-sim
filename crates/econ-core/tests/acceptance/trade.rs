//! Acceptance tests for AC-TFX-03 to AC-TFX-05 (spec: economy/trade-fx), the
//! M3 `[sim]` criteria, on the `World` under `Law::law_2021()`. Written in a
//! test-authoring session (ADR-0014) against the test writer's additions
//! behind the feature `trade`: the outside world's read-only state
//! (`World::foreign`), the monthly rest-of-world report
//! (`World::trade_months`), trade's parameters (`WorldParams::trade`) and a
//! world-constants edit (`StartEdit::WorldPrices`: every good's euro price
//! times a factor, for the whole run). AC-TFX-01 and 02 are in
//! `econ-mech-trade`.
//!
//! AC-TFX-04's "permanent 10% depreciation (a world-constants edit)": the
//! outside world's constants (spec, "First version") are the euro prices,
//! the foreign rate and inflation, the demand index and EUR/USD; the
//! exchange rate itself is state. A 10% rise of every euro price is, for
//! import prices in lei (`p* · e / e0`) and for export demand
//! (`p / (e/e0 · p*)`), the same as a permanent 10% rise of `e`, and is the
//! edit used here.

use econ_core::world::StartEdit;
use econ_ledger::{Instrument, Sector};
use econ_types::Bani;

use super::support::{month_stats, params, step};
use super::support_m3::{FlowSum, is_financing, world_2021, world_2021_with};

/// **AC-TFX-03** `[sim]` The balance of payments closes every month end:
/// current and capital account + the change in the rest of the world's lei
/// deposits + its purchases of government bonds less redemptions − the
/// change of the central bank's foreign assets = 0, to the ban (restated
/// 2026-10-21: the bonds, and the capital account of the EU's grants).
///
/// The bonds' purchases less redemptions are the change of the rest of the
/// world's holding; the foreign assets' change includes their yield.
#[test]
fn ac_tfx_03() {
    let mut w = world_2021(1000);
    let balances = |w: &econ_core::world::World| {
        let l = w.ledger();
        (
            l.balance(Sector::RestOfWorld, Instrument::Deposits),
            l.balance(Sector::RestOfWorld, Instrument::Bonds),
            l.balance(Sector::CentralBank, Instrument::ForeignAssets),
        )
    };
    let mut before = balances(&w);
    let mut month = FlowSum::default();
    let mut closed = 0;
    while closed < 12 {
        step(&mut w);
        month.add_day(&w);
        if w.trade_months().len() == closed {
            continue;
        }
        let m = &w.trade_months()[closed];
        assert_eq!(
            m.month,
            month_stats(&w, u32::try_from(closed).expect("small")).month
        );
        // The current account from the ledger: what the rest of the world
        // pays residents less what it receives, outside financial
        // transactions.
        let ca = month
            .of(Sector::RestOfWorld)
            .into_iter()
            .filter(|&(c, _)| !is_financing(c))
            .map(|(_, v)| Bani::ZERO - v)
            .fold(Bani::ZERO, |a, b| a + b);
        assert_eq!(m.current_account, ca, "{:?}: the current account", m.month);
        let after = balances(&w);
        let (deposits, bonds, foreign_assets) =
            (after.0 - before.0, after.1 - before.1, after.2 - before.2);
        assert_eq!(
            m.current_account + deposits + bonds - foreign_assets,
            Bani::ZERO,
            "{:?}: CA {:?}, change of the rest of the world's deposits {deposits:?}, its bonds {bonds:?}, the central bank's foreign assets {foreign_assets:?}",
            m.month,
            m.current_account,
        );
        before = after;
        month = FlowSum::default();
        closed += 1;
    }
}

/// **AC-TFX-04** `[sim]` A permanent 10% depreciation (a world-constants
/// edit) raises import prices at once, export volumes over the following
/// months and CPI by less than 10%.
#[test]
fn ac_tfx_04() {
    let mut base = world_2021(1000);
    let mut dear = world_2021_with(
        1000,
        vec![StartEdit::WorldPrices {
            factor_ppm: 1_100_000,
        }],
        params(),
    );
    step(&mut base);
    step(&mut dear);
    // At once: every import price 10% higher in lei.
    let (pb, pd) = (
        &base.foreign().import_price_ppm,
        &dear.foreign().import_price_ppm,
    );
    assert_eq!(pb.len(), pd.len());
    assert!(!pb.is_empty());
    for g in 0..pb.len() {
        let expected = u64::from(pb[g]) * 11 / 10;
        assert!(
            u64::from(pd[g]).abs_diff(expected) <= 1,
            "good {g}: import price {} against {} in the base",
            pd[g],
            pb[g]
        );
    }
    for w in [&mut base, &mut dear] {
        while w.months().len() < 12 {
            step(w);
        }
    }
    // Export volumes: higher in every month after the first close.
    let (tb, td) = (base.trade_months(), dear.trade_months());
    for m in 1..12 {
        assert!(
            td[m].exports_volume > tb[m].exports_volume,
            "{:?}: exports {} against {}",
            td[m].month,
            td[m].exports_volume,
            tb[m].exports_volume
        );
    }
    // CPI: higher, by less than 10%.
    for m in [5u32, 11] {
        let (cb, cd) = (
            i64::from(month_stats(&base, m).cpi_ppm),
            i64::from(month_stats(&dear, m).cpi_ppm),
        );
        assert!(cd > cb, "month {m}: CPI {cd} against {cb}");
        assert!(
            cd * 10 < cb * 11,
            "month {m}: CPI {cd} against {cb}, 10% or more"
        );
    }
}

/// **AC-TFX-05** `[sim]` Quiet baseline: on the Romania scenario with the
/// 2021 law and constant world, the exchange rate over 120 months stays
/// within 10% of its start and the central bank's reserves above their floor.
///
/// Owed to roadmap M7 (owner, 2026-10-22): without M7's calibration the leu
/// drifts about 10.3% over ten years.
#[test]
#[ignore = "waits for M7's calibration (owner 2026-10-22)"]
fn ac_tfx_05() {
    let mut w = world_2021(1000);
    let start = i64::from(w.foreign().start_rate_ppm);
    assert_eq!(i64::from(w.foreign().exchange_rate_ppm), start);
    let mut closes = 0;
    while w.months().len() < 120 {
        let r = step(&mut w);
        if w.months().len() == closes {
            continue;
        }
        closes = w.months().len();
        let f = w.foreign();
        let e = i64::from(f.exchange_rate_ppm);
        assert!(
            (e - start).abs() * 10 <= start,
            "{:?}: exchange rate {e} against {start} at the start",
            r.date
        );
        let reserves = w
            .ledger()
            .balance(Sector::CentralBank, Instrument::ForeignAssets);
        assert!(
            reserves > f.reserves_floor,
            "{:?}: reserves {reserves:?}, floor {:?}",
            r.date,
            f.reserves_floor
        );
    }
}
