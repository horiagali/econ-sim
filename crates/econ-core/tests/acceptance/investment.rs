//! Acceptance tests for AC-INV-04 to 10 (spec: economy/investment-capital),
//! the M3 `[sim]` criteria, on the `World` under `Law::law_2021()` with the
//! feature `investment`. AC-INV-01 to 03 are in `econ-mech-investment`.
//!
//! Written in a test-authoring session (ADR-0014) from the spec and its API
//! sketch only: the code came first, by the owner's decision of 2026-10-21.
//! The tests read each unit's capital and dividends (`World::investment`).
//! They also read two test writer's additions declared from the spec in
//! `econ_core::world`: the closed months of investment
//! (`World::investment_months`) and the latest budgets
//! (`World::investment_plans`).
//!
//! Dividends are paid on the 15th of the next month (step 5, decided
//! 2026-10-20), so AC-INV-04's "every month end" is read as every day
//! dividends are paid.

use econ_core::world::World;
use econ_ledger::{Instrument, Sector};
use econ_mech_investment::{CapitalBook, InvestParams, split_by_ownership};
use econ_types::Bani;

use super::support::{month_stats, start_state, step, total};
use super::support_m3::{FlowSum, code, legs_of_day, world_2021};
use super::support_rev::{
    assert_checks, bn, gap_ppm, is_public, million_lei, received, weight_total,
};

/// The index of a sector in [`Sector::ALL`].
fn sector_index(s: Sector) -> usize {
    Sector::ALL.iter().position(|&x| x == s).expect("a sector")
}

/// The index of an instrument in [`Instrument::ALL`].
fn instrument_index(i: Instrument) -> usize {
    Instrument::ALL
        .iter()
        .position(|&x| x == i)
        .expect("an instrument")
}

/// φ_PF: the pension funds' share of the households' and pension funds'
/// equity in the opening balance sheet (spec, "Pension funds' share").
fn pension_fund_share(start: &econ_core::world::StartState) -> f64 {
    let equity = start.balances[instrument_index(Instrument::Equity)];
    let hh = equity[sector_index(Sector::Households)].get() as f64;
    let pf = equity[sector_index(Sector::PensionFunds)].get() as f64;
    pf / (hh + pf)
}

/// Step to the next monthly close; returns the reports of the days.
fn run_to_close(w: &mut World) -> Vec<econ_core::world::DayReport> {
    let closed = w.months().len();
    let mut days = Vec::new();
    while w.months().len() == closed {
        days.push(step(w));
    }
    days
}

/// `round(K δ / 12)`, the month's depreciation of step 6.
fn month_depreciation(k: i64, p: &InvestParams) -> i64 {
    CapitalBook::new(k, 0, p).depreciation(p)
}

/// **AC-INV-04** `[sim]` The month's dividends add up, every month end on
/// the Romania scenario with the 2021 law:
/// - the `income.dividend` postings to households, pension funds, government
///   and the rest of the world sum to the units' dividends, less the
///   households' rounding remainder;
/// - each sector receives its ownership share;
/// - public producers pay none;
/// - the dividend clearing account nets to zero (I-8);
/// - `tax.dividend` is 5% of the households' gross dividends, to the ban.
///
/// The households' remainder is under one ban per household record times its
/// weight (whole bani per record). "To the ban" for the tax: one ban per
/// household (each record's tax is rounded, then counted with its weight).
#[test]
fn ac_inv_04() {
    let start = start_state(1000);
    let mut w = world_2021(1000);
    let dividend = code("income.dividend");
    let tax = code("tax.dividend");
    let phi = pension_fund_share(&start);
    assert!((phi - 0.089).abs() < 0.002, "φ_PF {phi}, the spec's 8.9%");
    let weights = weight_total(&w.households().hh_weight);
    let mut paid_days = 0;
    while w.months().len() < 14 {
        let r = step(&mut w);
        assert_checks(&r);
        let inv = w.investment();
        let divs = &inv.dividends_today;
        assert_eq!(divs.len(), start.unit_industry.len(), "a dividend per unit");
        for (u, &d) in divs.iter().enumerate() {
            assert!(d >= Bani::ZERO, "{:?}: unit {u} pays {d:?}", r.date);
            if is_public(&start, u) {
                assert_eq!(
                    d,
                    Bani::ZERO,
                    "{:?}: public producer {u} pays a dividend",
                    r.date
                );
            }
        }
        let paid = total(divs);
        if paid == Bani::ZERO {
            for s in [
                Sector::Households,
                Sector::PensionFunds,
                Sector::Government,
                Sector::RestOfWorld,
            ] {
                assert_eq!(
                    received(&w, dividend, s),
                    Bani::ZERO,
                    "{:?}: {s:?} dividends on no dividend day",
                    r.date
                );
            }
            continue;
        }
        paid_days += 1;
        assert_eq!(
            r.date.day, 15,
            "{:?}: dividends are paid on the 15th",
            r.date
        );
        // The ownership split, unit by unit.
        let mut parts = [Bani::ZERO; 3];
        for (u, &d) in divs.iter().enumerate() {
            let own = start.unit_ownership_ppm[u];
            let p = split_by_ownership(d, own[0], own[1], own[2]);
            for k in 0..3 {
                parts[k] += p[k];
            }
        }
        let [state, domestic, foreign] = parts;
        let gov = received(&w, dividend, Sector::Government);
        let row = received(&w, dividend, Sector::RestOfWorld);
        let pf = received(&w, dividend, Sector::PensionFunds);
        assert_eq!(gov, state, "{:?}: the state's share", r.date);
        assert_eq!(row, foreign, "{:?}: the foreign share", r.date);
        // The households' gross dividends: what they received, plus the tax
        // if it was withheld before it reached them.
        let legs = legs_of_day(&w, &r);
        let withheld_by_firms = legs
            .iter()
            .filter(|l| l.code == tax && l.payer == Sector::Firms)
            .map(|l| l.amount)
            .fold(Bani::ZERO, |a, b| a + b);
        let hh_gross = received(&w, dividend, Sector::Households) + withheld_by_firms;
        let units = i64::try_from(divs.len()).expect("fits");
        let pf_expected = domestic.get() as f64 * phi;
        assert!(
            (pf.get() as f64 - pf_expected).abs() <= pf_expected * 1e-5 + units as f64,
            "{:?}: pension funds {pf:?}, φ_PF of the domestic share {pf_expected}",
            r.date
        );
        let remainder = domestic - pf - hh_gross;
        assert!(
            remainder >= Bani::ZERO && remainder.get() < weights.max(1),
            "{:?}: the households' rounding remainder {remainder:?}",
            r.date
        );
        assert_eq!(
            gov + row + pf + hh_gross,
            paid - remainder,
            "{:?}: the postings and the units' dividends",
            r.date
        );
        // By receiving sector, as the view reports it.
        let by = inv.dividends_by_sector;
        assert_eq!(
            by[sector_index(Sector::Government)],
            gov,
            "{:?}: the view's government",
            r.date
        );
        assert_eq!(
            by[sector_index(Sector::RestOfWorld)],
            row,
            "{:?}: the view's rest of the world",
            r.date
        );
        assert_eq!(
            by[sector_index(Sector::PensionFunds)],
            pf,
            "{:?}: the view's pension funds",
            r.date
        );
        // The dividend tax.
        let t = received(&w, tax, Sector::Government);
        assert_eq!(
            inv.dividend_tax_today, t,
            "{:?}: the view's dividend tax",
            r.date
        );
        let expected = (i128::from(hh_gross.get()) * 5 + 50) / 100;
        assert!(
            (i128::from(t.get()) - expected).abs() <= i128::from(weights),
            "{:?}: dividend tax {t:?}, 5% of {hh_gross:?}",
            r.date
        );
        assert!(
            hh_gross > Bani::ZERO,
            "{:?}: households receive dividends",
            r.date
        );
    }
    assert!(
        paid_days >= 12,
        "dividends were paid on {paid_days} days in 14 months"
    );
}

/// **AC-INV-05** `[sim]` Foreign units' dividends leave the country. They
/// reach the rest of the world's deposits, appear as primary income paid
/// abroad in the month's current account, and the balance of payments still
/// closes (the restated identity of trade-fx).
#[cfg(feature = "trade")]
#[test]
fn ac_inv_05() {
    let mut w = world_2021(1000);
    let dividend = code("income.dividend");
    let mut month = FlowSum::default();
    let mut row_deposits = Bani::ZERO;
    let mut checked = 0;
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
    while checked < 13 {
        let r = step(&mut w);
        month.add_day(&w);
        // Dividends to the rest of the world reach its lei deposits.
        let legs = legs_of_day(&w, &r);
        let to_row: Bani = legs
            .iter()
            .filter(|l| l.code == dividend && l.payee == Sector::RestOfWorld)
            .map(|l| {
                assert_eq!(
                    l.instrument,
                    Instrument::Deposits,
                    "{:?}: dividends abroad in deposits",
                    r.date
                );
                l.amount
            })
            .fold(Bani::ZERO, |a, b| a + b);
        assert_eq!(to_row, received(&w, dividend, Sector::RestOfWorld));
        row_deposits += to_row;
        if w.trade_months().len() == checked {
            continue;
        }
        let m = &w.trade_months()[checked];
        let paid_abroad = month.get(dividend, Sector::RestOfWorld);
        if checked >= 1 {
            assert!(
                paid_abroad > Bani::ZERO,
                "{:?}: no dividends paid abroad",
                m.month
            );
        }
        // In the current account: every non-financing flow of the rest of
        // the world, the dividends among them.
        let ca = month
            .of(Sector::RestOfWorld)
            .into_iter()
            .filter(|&(c, _)| !c.name().starts_with("finance."))
            .map(|(_, v)| Bani::ZERO - v)
            .fold(Bani::ZERO, |a, b| a + b);
        assert_eq!(
            m.current_account, ca,
            "{:?}: the current account with dividends paid abroad",
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
        checked += 1;
    }
    assert!(row_deposits > Bani::ZERO);
}

/// **AC-INV-06** `[sim]` Capital and capacity. At every monthly close, each
/// unit's capital follows AC-INV-02 with its month's real spending, and its
/// capacity is `A K / κ`. The month's private and government investment are
/// the investment of GDP by expenditure, and I-7 holds.
///
/// The first `gestation_months` closes complete the starting work in
/// progress: a month of the starting investment each, the same in each.
/// `A` is read as one value across units: `capacity · κ / K`, with
/// `κ = K0 / capacity0`; it grows by `tfp_base_growth / 12` (1% a year) a
/// month (step 8). A unit with no capital keeps its capacity.
///
/// Owed to roadmap M7 (owner, 2026-10-22): it needs the investment rule,
/// which is off until M7's calibration.
#[test]
#[ignore = "waits for the investment rule (M7, owner 2026-10-22)"]
fn ac_inv_06() {
    let start = start_state(1000);
    let p = InvestParams::spec_defaults();
    let gestation = usize::from(p.gestation_months);
    let mut w = world_2021(1000);
    let k0: Vec<i64> = w.investment().capital.clone();
    let n = k0.len();
    for u in 0..n {
        assert_eq!(
            k0[u],
            start.unit_fixed_assets[u].get(),
            "unit {u}: starting capital is its fixed assets"
        );
    }
    let cap0: Vec<i64> = w.firm_units().capacity.clone();
    let mut first_completed: Vec<Option<i64>> = vec![None; n];
    let mut before = k0.clone();
    let mut a_last = 1.0;
    for m in 0..24usize {
        for r in run_to_close(&mut w) {
            assert_checks(&r);
        }
        let inv = w.investment();
        let after = inv.capital.clone();
        let months = w.investment_months();
        assert_eq!(months.len(), m + 1, "an investment month per close");
        let im = &months[m];
        assert_eq!(im.spent.len(), n);
        assert_eq!(im.spent_real.len(), n);
        let mut a_values = Vec::new();
        for u in 0..n {
            let dep = month_depreciation(before[u], &p);
            assert_eq!(
                inv.depreciation[u], dep,
                "month {m}, unit {u}: depreciation round(K δ / 12)"
            );
            let completed = after[u] - before[u] + dep;
            if m < gestation {
                // The starting slots: a month of the starting investment.
                match first_completed[u] {
                    None => first_completed[u] = Some(completed),
                    Some(c) => assert_eq!(
                        completed, c,
                        "month {m}, unit {u}: the starting slots are equal"
                    ),
                }
                if k0[u] > 0 {
                    assert!(
                        completed > 0,
                        "month {m}, unit {u}: no starting work in progress"
                    );
                } else {
                    assert_eq!(completed, 0, "unit {u} has no capital");
                }
            } else {
                assert_eq!(
                    completed,
                    months[m - gestation].spent_real[u],
                    "month {m}, unit {u}: completes month {}'s real spending",
                    m - gestation
                );
            }
            // Capacity A K / κ.
            let cap = w.firm_units().capacity[u];
            if k0[u] == 0 {
                assert_eq!(after[u], 0, "unit {u}: capital without investment");
                assert_eq!(
                    cap, cap0[u],
                    "month {m}, unit {u}: no capital keeps its capacity"
                );
            } else if cap0[u] > 1_000_000 && after[u] > 0 {
                a_values.push(cap as f64 * k0[u] as f64 / (cap0[u] as f64 * after[u] as f64));
            }
            // The month's spending at starting prices.
            let real = im.spent[u].get() as f64 * 1e6 / f64::from(im.price_index_ppm);
            assert!(
                (im.spent_real[u] as f64 - real).abs() <= real.abs() * 0.01 + 1.0,
                "month {m}, unit {u}: real spending {} against {:?} at p^I {}",
                im.spent_real[u],
                im.spent[u],
                im.price_index_ppm
            );
            assert!(
                im.spent[u] >= Bani::ZERO,
                "month {m}, unit {u}: negative investment"
            );
            if is_public(&start, u) || k0[u] == 0 {
                assert_eq!(
                    im.spent[u],
                    Bani::ZERO,
                    "month {m}, unit {u} invests with no capital"
                );
            }
        }
        // One A across units.
        assert!(!a_values.is_empty());
        let a = a_values.iter().sum::<f64>() / a_values.len() as f64;
        for &x in &a_values {
            assert!(
                (x - a).abs() <= 1e-4 * a,
                "month {m}: a unit's A {x} against {a}"
            );
        }
        let expected_a = (1.0 + 0.01 / 12.0_f64).powi(i32::try_from(m + 1).expect("small"));
        assert!(
            (a - expected_a).abs() <= 1e-3,
            "month {m}: A {a}, 1% a year gives {expected_a}"
        );
        assert!(a > a_last - 1e-9, "month {m}: A fell");
        a_last = a;
        // Investment in GDP by expenditure; I-7.
        assert_eq!(
            im.private,
            total(&im.spent),
            "month {m}: private investment"
        );
        assert!(
            im.government > Bani::ZERO,
            "month {m}: government investment"
        );
        assert_eq!(
            im.private + im.government,
            im.gdp_investment,
            "month {m}: investment in GDP"
        );
        let s = month_stats(&w, u32::try_from(m).expect("small"));
        assert_eq!(s.gdp_production, s.gdp_expenditure, "month {m}: I-7");
        assert_eq!(s.gdp_expenditure, s.gdp_income, "month {m}: I-7");
        before = after;
    }
}

/// **AC-INV-07** `[sim]` Financing. No unit's monthly budget exceeds its
/// spare money plus credit room on its plan day. Spending beyond its deposits
/// is a `finance.loan` draw, and money and banking's criterion that no firm
/// unit's deposits are negative at a day's end holds.
#[cfg(feature = "banking")]
#[test]
fn ac_inv_07() {
    use econ_mech_investment::financed;
    let mut w = world_2021(1000);
    let loan = code("finance.loan");
    let mut planned = 0;
    while w.months().len() < 12 {
        let r = step(&mut w);
        let units = w.firm_units();
        let plans = w.investment_plans();
        for u in 0..units.industry.len() {
            if units.last_plan[u] == Some(r.day) {
                planned += 1;
                let (d, b, s, c) = (
                    plans.desired[u],
                    plans.budget[u],
                    plans.spare[u],
                    plans.credit_room[u],
                );
                assert!(b >= Bani::ZERO, "{:?}, unit {u}: budget {b:?}", r.date);
                assert!(
                    b <= s.max(Bani::ZERO) + c.max(Bani::ZERO),
                    "{:?}, unit {u}: budget {b:?} above spare {s:?} plus credit room {c:?}",
                    r.date
                );
                assert_eq!(
                    b,
                    financed(d, s, c),
                    "{:?}, unit {u}: the budget is what it can finance",
                    r.date
                );
            }
            assert!(
                units.deposits[u] >= Bani::ZERO,
                "{:?}, unit {u}: deposits {:?} at the day's end",
                r.date,
                units.deposits[u]
            );
        }
        // What units drew today is the day's `finance.loan` to firms.
        let drawn = total(&w.bank().drawn_today);
        let legs = legs_of_day(&w, &r);
        let lent: Bani = legs
            .iter()
            .filter(|l| {
                l.code == loan && l.payee == Sector::Firms && l.instrument == Instrument::Deposits
            })
            .map(|l| l.amount)
            .fold(Bani::ZERO, |a, b| a + b);
        assert_eq!(
            drawn, lent,
            "{:?}: credit-line draws and finance.loan",
            r.date
        );
    }
    assert!(planned > 0, "no plan day in a year");
}

/// Firms' investment in each closed month.
#[cfg(feature = "banking")]
fn private_by_month(w: &World) -> Vec<Bani> {
    w.investment_months().iter().map(|m| m.private).collect()
}

/// **AC-INV-08** `[sim]` The policy rate matters. Compare two worlds, one
/// with the policy rate 2 points higher from month 3
/// (`StartEdit::PolicyRate`). Firms' investment is lower in every month from
/// month 6 to month 24. The fall is larger, relative to the baseline, for the
/// third of units with the most capital per unit of output than for the third
/// with the least.
///
/// Owed to roadmap M7: the investment rule is switched off until then
/// (`INVESTMENT_RULE`).
#[cfg(feature = "banking")]
#[test]
#[ignore = "the investment rule is off until M7 (INVESTMENT_RULE); owed to roadmap M7"]
fn ac_inv_08() {
    use super::support::params;
    use super::support_m3::world_2021_with;
    use econ_core::world::StartEdit;
    // The first day of month 3: after two monthly closes.
    let mut probe = world_2021(1000);
    while probe.months().len() < 2 {
        step(&mut probe);
    }
    let from = probe.day();
    let p = params();
    let mut base = world_2021(1000);
    let mut high = world_2021_with(
        1000,
        vec![StartEdit::PolicyRate {
            from,
            rate_ppm: p.policy_rate_ppm + 20_000,
        }],
        p,
    );
    for w in [&mut base, &mut high] {
        while w.months().len() < 24 {
            step(w);
        }
    }
    let (b, h) = (private_by_month(&base), private_by_month(&high));
    for m in 5..24 {
        assert!(
            h[m] < b[m],
            "month {}: investment {:?} with the higher rate, {:?} without",
            m + 1,
            h[m],
            b[m]
        );
    }
    // Thirds of units by capital per unit of output at the start.
    let start = start_state(1000);
    let mut by_intensity: Vec<(f64, usize)> = (0..start.unit_industry.len())
        .filter(|&u| start.unit_fixed_assets[u] > Bani::ZERO && start.unit_output[u] > Bani::ZERO)
        .map(|u| {
            (
                start.unit_fixed_assets[u].get() as f64 / start.unit_output[u].get() as f64,
                u,
            )
        })
        .collect();
    by_intensity.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
    let third = by_intensity.len() / 3;
    let fall = |units: &[(f64, usize)]| {
        let sum = |w: &World| -> f64 {
            w.investment_months()[5..24]
                .iter()
                .map(|m| {
                    units
                        .iter()
                        .map(|&(_, u)| m.spent[u].get() as f64)
                        .sum::<f64>()
                })
                .sum()
        };
        let (sb, sh) = (sum(&base), sum(&high));
        (sb - sh) / sb
    };
    let least = fall(&by_intensity[..third]);
    let most = fall(&by_intensity[by_intensity.len() - third..]);
    eprintln!("AC-INV-08: relative fall, most capital-intensive third {most:.4}, least {least:.4}");
    assert!(
        most > least,
        "the most capital-intensive third falls {most}, the least {least}"
    );
}

/// **AC-INV-09** `[sim]` Quiet baseline. Run the Romania scenario at 1:100
/// with the 2021 law and no command, over 120 months. At every annual close:
/// firm units' deposits over the last twelve months' nominal GDP are within
/// 30% of the first year's ratio; gross investment is between 15% and 30% of
/// GDP; total capital stays between 0.9 and 1.6 times its start.
///
/// Owed to roadmap M7: the investment rule is switched off until then
/// (`INVESTMENT_RULE`).
#[test]
#[ignore = "the investment rule is off until M7 (INVESTMENT_RULE); owed to roadmap M7"]
fn ac_inv_09() {
    let mut w = world_2021(100);
    let k0: i128 = w.investment().capital.iter().map(|&k| i128::from(k)).sum();
    let mut first_ratio = None;
    for year in 1..=10usize {
        while w.months().len() < 12 * year {
            step(&mut w);
        }
        let months = &w.months()[12 * (year - 1)..12 * year];
        let gdp = months
            .iter()
            .map(|m| m.gdp_production)
            .fold(Bani::ZERO, |a, b| a + b);
        let deposits = total(&w.firm_units().deposits);
        let ratio = deposits.get() as f64 / gdp.get() as f64;
        let first = *first_ratio.get_or_insert(ratio);
        assert!(
            (ratio - first).abs() <= 0.3 * first,
            "year {year}: deposits over GDP {ratio}, first year {first}"
        );
        let inv = &w.investment_months()[12 * (year - 1)..12 * year];
        let gross = inv
            .iter()
            .map(|m| m.private + m.government)
            .fold(Bani::ZERO, |a, b| a + b);
        let share = gross.get() as f64 / gdp.get() as f64;
        assert!(
            (0.15..=0.30).contains(&share),
            "year {year}: investment {share} of GDP"
        );
        let k: i128 = w.investment().capital.iter().map(|&k| i128::from(k)).sum();
        let kr = k as f64 / k0 as f64;
        assert!(
            (0.9..=1.6).contains(&kr),
            "year {year}: capital {kr} times its start"
        );
    }
}

/// **AC-INV-10** `[sim]` The first year is reported next to the research
/// note's targets, on the Romania scenario with the 2021 law: consumption of
/// fixed capital (163 bn lei), private investment (249), dividends paid
/// abroad (18.6), dividends to residents, profit tax (26.3), household
/// consumption (720). Within: consumption of fixed capital ±15%, private
/// investment ±25%, dividends paid abroad ±50%, profit tax ±50%, household
/// consumption ±10%.
///
/// Consumption of fixed capital is each close's depreciation at that
/// close's `p^I`. Profit tax is `tax.profit` and `tax.micro`, as in
/// taxation's revenue check.
///
/// Owed to roadmap M7 (owner, 2026-10-22): it needs the investment rule,
/// which is off until M7's calibration.
#[test]
#[ignore = "waits for the investment rule (M7, owner 2026-10-22)"]
fn ac_inv_10() {
    let mut w = world_2021(1000);
    let mut sum = FlowSum::default();
    let mut cfc = 0.0_f64;
    for m in 0..12usize {
        let closed = w.months().len();
        while w.months().len() == closed {
            step(&mut w);
            sum.add_day(&w);
        }
        let p = f64::from(w.investment_months()[m].price_index_ppm) / 1e6;
        cfc += w
            .investment()
            .depreciation
            .iter()
            .map(|&d| d as f64)
            .sum::<f64>()
            * p;
    }
    let cfc = Bani(cfc.round() as i64);
    let private = w.investment_months()[..12]
        .iter()
        .map(|m| m.private)
        .fold(Bani::ZERO, |a, b| a + b);
    let dividend = code("income.dividend");
    let abroad = sum.get(dividend, Sector::RestOfWorld);
    let residents = sum.get(dividend, Sector::Households)
        + sum.get(dividend, Sector::PensionFunds)
        + sum.get(dividend, Sector::Government);
    let profit_tax = sum.get(code("tax.profit"), Sector::Government)
        + sum.get(code("tax.micro"), Sector::Government);
    let consumption = w.months()[..12]
        .iter()
        .map(|m| m.consumption)
        .fold(Bani::ZERO, |a, b| a + b);
    let rows = [
        ("consumption of fixed capital", cfc, 163_400.0, 150_000),
        ("private investment", private, 249_000.0, 250_000),
        ("dividends paid abroad", abroad, 18_600.0, 500_000),
        ("profit tax", profit_tax, 26_285.3, 500_000),
        ("household consumption", consumption, 719_713.4, 100_000),
    ];
    eprintln!("AC-INV-10, the first year, billion lei (target):");
    let mut failures = Vec::new();
    for (name, sim, target, tol) in rows {
        let target = million_lei(target);
        let gap = gap_ppm(sim, target);
        eprintln!(
            "  {name}: {:.1} ({:.1}), {:+.1}%",
            bn(sim),
            bn(target),
            (sim.get() as f64 / target.get() as f64 - 1.0) * 100.0
        );
        if gap > tol {
            failures.push(format!("{name} {:.1} against {:.1}", bn(sim), bn(target)));
        }
    }
    eprintln!(
        "  dividends to residents: {:.1} (households 0.1, government 2.9 in 2021)",
        bn(residents)
    );
    assert!(failures.is_empty(), "outside the tolerance: {failures:?}");
}
