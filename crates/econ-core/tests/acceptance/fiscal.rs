//! Acceptance tests for AC-FIS-02, 03 and 05 (spec: economy/fiscal-policy),
//! M3 `[sim]` criteria, on the `World` under `Law::law_2021()`. Written in a
//! test-authoring session (ADR-0014) against the test writer's additions
//! behind the feature `fiscal`: the government's debt (`World::treasury`), its
//! monthly budget report (`World::fiscal_months`) and parameters
//! (`WorldParams::fiscal`). AC-FIS-04 is in `econ-mech-fiscal`. AC-FIS-01 is
//! not written: its identity, as the spec states it, does not hold (see the
//! test-session report). The flow codes are found by the names the spec
//! gives them.

use std::collections::BTreeMap;

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_mech_fiscal::split_by_holdings;
use econ_types::Bani;

use super::support::{month_stats, step};
use super::support_m3::{FlowSum, code, gdp_2021, is_financing, ratio_ppm, world_2021};

/// Government bonds outstanding in the ledger.
fn debt(w: &econ_core::world::World) -> Bani {
    Bani::ZERO - w.ledger().balance(Sector::Government, Instrument::Bonds)
}

/// **AC-FIS-02** `[sim]` After every month end the government's deposits at
/// the central bank equal `gov_cash_target`, and no maturing bond is left
/// unpaid.
#[test]
fn ac_fis_02() {
    let mut w = world_2021(1000);
    let t = w.treasury();
    assert_eq!(
        t.book
            .by_year
            .iter()
            .copied()
            .fold(Bani::ZERO, |a, b| a + b),
        debt(&w)
    );
    let mut matured_any = false;
    for _ in 0..24 {
        let r = loop {
            let r = step(&mut w);
            if r.date.is_month_end() {
                break r;
            }
        };
        let d = r.date;
        let t = w.treasury();
        assert_eq!(
            w.ledger()
                .balance(Sector::Government, Instrument::GovDepositsAtCb),
            t.cash_target,
            "{d:?}: the government's deposits at the central bank"
        );
        assert!(
            t.cash_target > Bani::ZERO,
            "{d:?}: a target of a month of spending"
        );
        assert_eq!(t.redeemed, t.matured, "{d:?}: every maturing bond is paid");
        matured_any |= t.matured > Bani::ZERO;
        // The book is the ledger's debt.
        let book = t
            .book
            .by_year
            .iter()
            .copied()
            .fold(Bani::ZERO, |a, b| a + b);
        assert_eq!(book, debt(&w), "{d:?}: the book and the ledger's bonds");
    }
    assert!(matured_any, "no bond fell due in two years");
}

/// **AC-FIS-03** `[sim]` Coupons: each holder sector receives its share of a
/// twelfth of the coupon on the stock, the sum is the government's interest
/// under `interest.gov_debt`.
#[test]
fn ac_fis_03() {
    let mut w = world_2021(1000);
    let coupon = code("interest.gov_debt");
    let mut month_ends = 0;
    while month_ends < 6 {
        // The stock and its holders before today's payments: bonds move only
        // at month ends, last.
        let book = w.treasury().book.clone();
        let holders: Vec<Sector> = Sector::ALL
            .into_iter()
            .filter(|&s| s != Sector::Government)
            .collect();
        let holdings: Vec<Bani> = holders
            .iter()
            .map(|&s| w.ledger().balance(s, Instrument::Bonds).max(Bani::ZERO))
            .collect();
        let r = step(&mut w);
        let d = r.date;
        let f = |s| w.ledger().flows().get(coupon, s);
        if !d.is_month_end() {
            assert_eq!(
                f(Sector::Government),
                Bani::ZERO,
                "{d:?}: coupons off month end"
            );
            continue;
        }
        month_ends += 1;
        let total = book.month_coupon();
        assert!(total > Bani::ZERO);
        assert_eq!(
            f(Sector::Government),
            Bani::ZERO - total,
            "{d:?}: the government's interest"
        );
        let shares = split_by_holdings(total, &holdings);
        let mut received = Bani::ZERO;
        for (k, &s) in holders.iter().enumerate() {
            received += f(s);
            assert!(
                (f(s) - shares[k]).abs() <= Bani(1),
                "{d:?}: {s:?} holding {:?} receives {:?}, its share {:?}",
                holdings[k],
                f(s),
                shares[k]
            );
        }
        assert_eq!(
            received, total,
            "{d:?}: the holders receive the whole coupon"
        );
    }
}

/// **AC-FIS-05** `[sim]` On the Romania scenario with the 2021 law, the first
/// year's deficit is reported with its parts (revenue by tax, spending by
/// line, interest), and debt over GDP moves by the deficit less nominal
/// growth.
///
/// The parts are checked against the ledger month by month. "Moves by the
/// deficit less nominal growth" is the debt dynamics
/// `Δb = deficit / Y1 − b0 · g / (1 + g)`, with `b0` the opening debt over
/// 2021's GDP (the budget fixture), `Y1` the first year's GDP and `g` its
/// growth over 2021; it holds to within one point of GDP, the room the
/// change of the government's deposits leaves.
#[test]
fn ac_fis_05() {
    let mut w = world_2021(1000);
    let interest = code("interest.gov_debt");
    let b0_debt = debt(&w);
    let mut month = FlowSum::default();
    let mut year_deficit = Bani::ZERO;
    let mut checked = 0;
    while checked < 12 {
        step(&mut w);
        month.add_day(&w);
        if w.fiscal_months().len() == checked {
            continue;
        }
        let m = &w.fiscal_months()[checked];
        let ms = month_stats(&w, u32::try_from(checked).expect("small"));
        assert_eq!(m.month, ms.month);
        let gov: BTreeMap<FlowCode, Bani> = month
            .of(Sector::Government)
            .into_iter()
            .filter(|&(c, v)| !is_financing(c) && v != Bani::ZERO)
            .collect();
        let mut revenue = Vec::new();
        let mut spending = Vec::new();
        for (&c, &v) in &gov {
            if c == interest {
                continue;
            }
            if v > Bani::ZERO {
                revenue.push((c, v));
            } else {
                spending.push((c, Bani::ZERO - v));
            }
        }
        let paid_interest = Bani::ZERO - gov.get(&interest).copied().unwrap_or(Bani::ZERO);
        assert_eq!(m.revenue, revenue, "{:?}: revenue by code", m.month);
        assert_eq!(m.spending, spending, "{:?}: spending by code", m.month);
        assert_eq!(m.interest, paid_interest, "{:?}: interest", m.month);
        let sum = |v: &[(FlowCode, Bani)]| v.iter().map(|&(_, x)| x).fold(Bani::ZERO, |a, b| a + b);
        assert_eq!(
            m.deficit,
            sum(&m.spending) + m.interest - sum(&m.revenue),
            "{:?}: the deficit",
            m.month
        );
        for c in [
            FlowCode::TAX_INCOME,
            FlowCode::TAX_VAT,
            FlowCode::CONTRIB_PENSION,
            FlowCode::CONTRIB_HEALTH,
            FlowCode::TAX_EMPLOYER_CONTRIBUTION,
        ] {
            assert!(
                m.revenue.iter().any(|&(x, _)| x == c),
                "{:?}: no {} in revenue",
                m.month,
                c.name()
            );
        }
        assert!(
            m.interest > Bani::ZERO,
            "{:?}: interest on the debt",
            m.month
        );
        year_deficit += m.deficit;
        month = FlowSum::default();
        checked += 1;
    }
    // Debt over GDP.
    let y0 = gdp_2021();
    let y1 = (0..12u32)
        .map(|m| month_stats(&w, m).gdp_production)
        .fold(Bani::ZERO, |a, b| a + b);
    let b0 = ratio_ppm(b0_debt, y0);
    let b1 = ratio_ppm(debt(&w), y1);
    let growth = ratio_ppm(y1 - y0, y0);
    let deficit_share = ratio_ppm(year_deficit, y1);
    let predicted = deficit_share - b0 * growth / (1_000_000 + growth);
    eprintln!(
        "AC-FIS-05: deficit {} million lei ({deficit_share} ppm of GDP); debt over GDP {b0} → {b1} ppm; nominal growth {growth} ppm; predicted change {predicted} ppm",
        year_deficit.get() / 100_000_000
    );
    assert!(
        ((b1 - b0) - predicted).abs() <= 10_000,
        "debt over GDP moved {} ppm, the deficit less nominal growth gives {predicted}",
        b1 - b0
    );
}
