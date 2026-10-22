//! Acceptance tests for AC-EXC-06 to 08 (spec: economy/excise), the M3
//! `[sim]` criteria, on the `World` under `Law::law_2021()` with the feature
//! `revenue`. AC-EXC-01 to 05 are in `econ-mech-tax` (feature `excise`).
//!
//! Written in a test-authoring session (ADR-0014) from the spec only: the
//! code came first, by the owner's decision of 2026-10-21. The tests read
//! these test writer's additions, declared from the spec in
//! `econ_core::world`:
//! - the day's duties and purchases (`World::revenue_today`);
//! - the closed months of revenue (`World::revenue_months`);
//! - each industry's unit cost (`World::unit_costs_ppm`);
//! - a world without excise (`StartEdit::WithoutRevenue`).

use econ_core::world::{RevenuePart, StartEdit};
use econ_ledger::{Instrument, Sector};
use econ_types::Bani;

use super::support::{params, start_state, step, total};
use super::support_m3::{FlowSum, code, legs_of_day, world_2021, world_2021_with};
use super::support_rev::{
    BEVERAGES_DUTY_PPM, EXEMPT_BUYERS, FUEL_DUTY_PPM, TOBACCO_DUTY_PPM, assert_checks, bn,
    by_parts, final_demand, gap_ppm, industry, million_lei, received, units_of, weight_total,
};

/// `q · d / 10^6` as a float, bani.
fn duty_on(q: i64, d: u32) -> f64 {
    q as f64 * f64::from(d) / 1e6
}

/// The excised goods and their starting duties.
fn excised(start: &econ_core::world::StartState) -> [(usize, u32); 3] {
    [
        (industry(start, "refined_petroleum"), FUEL_DUTY_PPM),
        (industry(start, "tobacco"), TOBACCO_DUTY_PPM),
        (industry(start, "beverages"), BEVERAGES_DUTY_PPM),
    ]
}

/// **AC-EXC-06** `[sim]` On every day of a year on the Romania scenario with
/// the 2021 law, the government's receipts under `tax.excise` equal the
/// sum, with weights, of the duties on households' purchases and firm units'
/// fuel purchases. No exempt industry and no export pays any, and every
/// ledger check holds.
///
/// Each purchase record's duty is rounded: households' duties are within one
/// ban per household of `q · d` by good, a unit's within one ban per
/// purchase (a supplier unit, or the import).
#[test]
fn ac_exc_06() {
    let start = start_state(1000);
    let mut w = world_2021(1000);
    let excise = code("tax.excise");
    let fuel = industry(&start, "refined_petroleum");
    let exempt: Vec<usize> = EXEMPT_BUYERS
        .iter()
        .map(|id| industry(&start, id))
        .collect();
    let n_goods = start.industry_id.len();
    let suppliers = i64::try_from(units_of(&start, fuel).len()).expect("fits") + 1;
    let weights = weight_total(&w.households().hh_weight);
    let mut paid_by_firms = Bani::ZERO;
    let mut last_duty: Option<Vec<u32>> = None;
    for _ in 0..365 {
        let r = step(&mut w);
        assert_checks(&r);
        let rev = w.revenue_today();
        assert_eq!(rev.excise_duty_ppm.len(), n_goods);
        assert_eq!(rev.hh_quantity.len(), n_goods);
        // The duties: the spec's on the first day; they change on 1 January
        // only (indexation), and never fall.
        match &last_duty {
            None => {
                for g in 0..n_goods {
                    let d = excised(&start)
                        .iter()
                        .find(|&&(x, _)| x == g)
                        .map_or(0, |&(_, d)| d);
                    assert_eq!(
                        rev.excise_duty_ppm[g], d,
                        "{:?}: good {} starts at its duty",
                        r.date, start.industry_id[g]
                    );
                }
            }
            Some(prev) => {
                if !(r.date.month == 1 && r.date.day == 1) {
                    assert_eq!(
                        &rev.excise_duty_ppm, prev,
                        "{:?}: duties change on 1 January only",
                        r.date
                    );
                }
                for (g, &before) in prev.iter().enumerate().take(n_goods) {
                    assert!(
                        rev.excise_duty_ppm[g] >= before,
                        "{:?}: a duty fell",
                        r.date
                    );
                }
            }
        }
        last_duty = Some(rev.excise_duty_ppm.clone());
        // Households, by good.
        for g in 0..n_goods {
            let d = rev.excise_duty_ppm[g];
            let e = rev.hh_excise[g];
            assert!(e >= Bani::ZERO);
            if d == 0 {
                assert_eq!(
                    e,
                    Bani::ZERO,
                    "{:?}: households pay excise on {}",
                    r.date,
                    start.industry_id[g]
                );
            }
            assert!(
                (e.get() as f64 - duty_on(rev.hh_quantity[g], d)).abs() <= weights as f64,
                "{:?}: households' duty on {} is {e:?}, q {} times d {d}",
                r.date,
                start.industry_id[g],
                rev.hh_quantity[g]
            );
        }
        // Firm units: fuel only, exempt buyers none.
        let d = rev.excise_duty_ppm[fuel];
        for u in 0..start.unit_industry.len() {
            let j = usize::from(start.unit_industry[u]);
            let e = rev.unit_excise[u];
            if exempt.contains(&j) {
                assert_eq!(
                    e,
                    Bani::ZERO,
                    "{:?}: exempt unit {u} ({}) pays excise",
                    r.date,
                    start.industry_id[j]
                );
                continue;
            }
            assert!(
                (e.get() as f64 - duty_on(rev.unit_fuel_quantity[u], d)).abs() <= suppliers as f64,
                "{:?}: unit {u} pays {e:?} on {} units of fuel",
                r.date,
                rev.unit_fuel_quantity[u]
            );
        }
        // The government receives exactly the duties.
        let duties = total(&rev.hh_excise) + total(&rev.unit_excise);
        assert_eq!(
            received(&w, excise, Sector::Government),
            duties,
            "{:?}: tax.excise and the duties",
            r.date
        );
        // Paid by the firms sector, through the three legs; never by the
        // rest of the world.
        let legs = legs_of_day(&w, &r);
        for l in legs.iter().filter(|l| l.code == excise) {
            assert_ne!(
                l.payer,
                Sector::RestOfWorld,
                "{:?}: an export pays excise",
                r.date
            );
        }
        let first_leg: Bani = legs
            .iter()
            .filter(|l| {
                l.code == excise && l.payer == Sector::Firms && l.instrument == Instrument::Deposits
            })
            .map(|l| l.amount)
            .fold(Bani::ZERO, |a, b| a + b);
        assert_eq!(
            first_leg, duties,
            "{:?}: firms pay the duties from deposits",
            r.date
        );
        paid_by_firms += first_leg;
    }
    assert!(paid_by_firms > Bani::ZERO);
}

/// **AC-EXC-07** `[sim]` At load, the starting price of every good is 1 and
/// the starting unit cost of every industry is what it is without excise (the
/// duty moved out of other costs). Households' first-day real demand for
/// each excised good is within 1% of the table's daily quantity times the
/// households' spending ratio of that day.
///
/// "The households' spending ratio of that day" is read with the basket's
/// rule: every good's day spending, net of its duty's weight in the basket
/// (`q · consumer price / (1 + d)`), over the table's purchases of it, is
/// the same ratio. So each excised good's ratio is within 1% of the ratio of
/// all other goods together.
#[test]
fn ac_exc_07() {
    let start = start_state(1000);
    let mut w = world_2021(1000);
    let without = world_2021_with(
        1000,
        vec![StartEdit::WithoutRevenue(RevenuePart::Excise)],
        params(),
    );
    for (g, &p) in w.industries().price_ppm.iter().enumerate() {
        assert_eq!(
            p, 1_000_000,
            "the starting price of {}",
            start.industry_id[g]
        );
    }
    assert_eq!(
        w.unit_costs_ppm(),
        without.unit_costs_ppm(),
        "starting unit costs with and without excise"
    );
    let r = step(&mut w);
    assert_checks(&r);
    let rev = w.revenue_today();
    let cp = &w.industries().consumer_price_ppm;
    let n = start.industry_id.len();
    let table: Vec<f64> = start
        .industry_id
        .iter()
        .map(|id| by_parts(id, |i| final_demand("households", i)))
        .collect();
    let x = |g: usize| {
        let d = f64::from(rev.excise_duty_ppm[g]) / 1e6;
        rev.hh_quantity[g] as f64 * f64::from(cp[g]) / 1e6 / (1.0 + d)
    };
    let excised: Vec<usize> = excised(&start).iter().map(|&(g, _)| g).collect();
    let (mut xs, mut ts) = (0.0, 0.0);
    for g in (0..n).filter(|g| !excised.contains(g) && table[*g] > 0.0) {
        xs += x(g);
        ts += table[g];
    }
    let ratio = xs / ts;
    for &g in &excised {
        let rg = x(g) / table[g];
        eprintln!(
            "AC-EXC-07: {}: day 1 {:.6} of the table, the others {:.6}",
            start.industry_id[g], rg, ratio
        );
        assert!(
            (rg / ratio - 1.0).abs() <= 0.01,
            "{}: {rg} of the table, the other goods {ratio}",
            start.industry_id[g]
        );
    }
}

/// **AC-EXC-08** `[sim]` The first twelve months' excise on the Romania
/// scenario with the 2021 law is within 20% of 2021's 34.66 billion lei,
/// reported by good. The tolerance is taxation's revenue check's: households
/// spend about 20% less than in 2021.
#[test]
fn ac_exc_08() {
    let start = start_state(1000);
    let mut w = world_2021(1000);
    let mut sum = FlowSum::default();
    while w.months().len() < 12 {
        step(&mut w);
        sum.add_day(&w);
    }
    let received_year = sum.get(code("tax.excise"), Sector::Government);
    let months: &[econ_core::world::RevenueMonth] = &w.revenue_months()[..12];
    let n = start.industry_id.len();
    let mut by_good = vec![Bani::ZERO; n];
    for m in months {
        for (b, &e) in by_good.iter_mut().zip(&m.excise_by_good) {
            *b += e;
        }
    }
    assert_eq!(
        total(&by_good),
        received_year,
        "the report by good and the ledger"
    );
    eprintln!(
        "AC-EXC-08: the first year's excise {:.2} billion lei (2021: 34.66)",
        bn(received_year)
    );
    for (g, &e) in by_good.iter().enumerate() {
        if e != Bani::ZERO {
            eprintln!("  {}: {:.2}", start.industry_id[g], bn(e));
        }
    }
    let target = million_lei(34_663.5);
    assert!(
        gap_ppm(received_year, target) <= 200_000,
        "excise {:.2} billion lei, 2021's 34.66",
        bn(received_year)
    );
}
