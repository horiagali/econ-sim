//! Acceptance tests for AC-GRV-01 to 07 (spec:
//! economy/government-revenue-2021), on the `World` under `Law::law_2021()`
//! with the feature `revenue`.
//!
//! Written in a test-authoring session (ADR-0014) from the spec only: the
//! code came first, by the owner's decision of 2026-10-21. The tests read
//! the starting state's rates and these test writer's additions, declared
//! from the spec in `econ_core::world`:
//! - the day's purchases and taxes (`World::revenue_today`);
//! - the closed months of revenue (`World::revenue_months`);
//! - the held benefits of the latest 15th (`World::held_benefits`);
//! - each industry's unit cost (`World::unit_costs_ppm`);
//! - a world without parts 1 and 2 (`StartEdit::WithoutRevenue`).
//!
//! The held amounts are the spec's tables. The spec places them in the
//! budget fixture's `revenue_held` and `benefits_held`, which the fixture
//! does not have yet.

use std::collections::BTreeMap;

use econ_core::world::{RevenuePart, StartEdit};
use econ_io::budget::Budget;
use econ_ledger::{FlowCode, Sector};
use econ_types::Bani;

use super::support::{params, start_state, step, total, weighted};
use super::support_m3::{code, legs_of_day, world_2021, world_2021_with};
use super::support_rev::{
    assert_checks, bn, by_parts, final_demand, gap_ppm, inputs, million_lei, nace_count, received,
    row, starting_fuel_duty,
};

/// `|a − b| ≤ tol`, floats.
fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// **AC-GRV-01** `[unit]` The rates at load: for every industry, `t_j` times
/// its starting input purchases plus its starting fuel duty equals its
/// product taxes on inputs in the table, within one ban per million. Over
/// all industries this is 34.67 billion lei, and `t_I` times the table's
/// investment and inventories is 16.45 billion.
///
/// An industry's starting input purchases, fuel duty and product taxes on
/// inputs are its catalogue shares of its NACE industries' in the table (the
/// rates are computed at load from the table, as the input coefficients
/// are). "Within one ban per million" is a millionth of its purchases (the
/// rate's rounding), and a ban.
#[test]
fn ac_grv_01() {
    let start = start_state(1000);
    let mut sum = 0.0;
    let mut failures = Vec::new();
    for (j, id) in start.industry_id.iter().enumerate() {
        let bought = by_parts(id, inputs) * 100.0;
        let fuel = starting_fuel_duty(id) * 100.0;
        let taxes = by_parts(id, |nj| row("product_taxes", nj)) * 100.0;
        let t = f64::from(start.product_tax_ppm[j]) / 1e6;
        if !close(t * bought + fuel, taxes, bought / 1e6 + 1.0) {
            failures.push(format!(
                "{id}: t {} on {bought:.0} bani plus fuel duty {fuel:.0} is {:.0}, product taxes {taxes:.0}",
                start.product_tax_ppm[j],
                t * bought + fuel,
            ));
        }
        sum += t * bought;
    }
    assert!(failures.is_empty(), "rates at load: {failures:#?}");
    eprintln!(
        "AC-GRV-01: product taxes on inputs {:.2} billion lei",
        sum / 1e11
    );
    assert!(
        close(sum, 34.67e11, 0.005e11),
        "product taxes on inputs {} bani, the spec's 34.67 billion lei",
        sum
    );
    let mut invest = 0.0;
    for i in 0..nace_count() {
        invest += final_demand("investment", i) + final_demand("inventories", i);
    }
    let t_i = f64::from(start.investment_product_tax_ppm) / 1e6;
    assert!(
        close(t_i * invest, 16.45e9, 0.005e9),
        "t_I {} on {invest} lei",
        start.investment_product_tax_ppm
    );
    assert!(
        start.investment_product_tax_ppm.abs_diff(55_400) <= 100,
        "t_I {}",
        start.investment_product_tax_ppm
    );
}

/// **AC-GRV-02** `[sim]` Every day on the Romania scenario with the 2021
/// law: the receipts under `tax.products` equal the sum of `t_j` times each
/// unit's day's input purchases plus `t_I` times its investment purchases;
/// the government pays none on its own purchases or investment; every ledger
/// check holds.
///
/// A unit's tax rounds per purchase: within one ban per good it buys, home
/// and imported, and one for its investment.
#[test]
fn ac_grv_02() {
    let start = start_state(1000);
    let mut w = world_2021(1000);
    let products = code("tax.products");
    let goods = i128::try_from(start.industry_id.len()).expect("small");
    for _ in 0..92 {
        let r = step(&mut w);
        assert_checks(&r);
        let rev = w.revenue_today();
        for u in 0..start.unit_industry.len() {
            let j = usize::from(start.unit_industry[u]);
            let expected = i128::from(rev.unit_inputs[u].get())
                * i128::from(start.product_tax_ppm[j])
                + i128::from(rev.unit_investment[u].get())
                    * i128::from(start.investment_product_tax_ppm);
            let paid = i128::from(rev.unit_product_tax[u].get()) * 1_000_000;
            assert!(
                (paid - expected).abs() <= (2 * goods + 1) * 1_000_000,
                "{:?}: unit {u} pays {:?} on inputs {:?} and investment {:?}",
                r.date,
                rev.unit_product_tax[u],
                rev.unit_inputs[u],
                rev.unit_investment[u]
            );
        }
        assert_eq!(
            received(&w, products, Sector::Government),
            total(&rev.unit_product_tax),
            "{:?}: tax.products",
            r.date
        );
        for l in legs_of_day(&w, &r).iter().filter(|l| l.code == products) {
            assert_ne!(
                l.payer,
                Sector::Government,
                "{:?}: the government pays product taxes",
                r.date
            );
        }
    }
}

/// **AC-GRV-03** `[sim]` With parts 1 and 2 on, every industry's starting
/// unit cost and price are those of the `World` without them. After twelve
/// months, the fall in firm units' deposits plus the fall in the dividends
/// they paid plus the fall in the profit tax they paid (`tax.profit` and
/// `tax.micro`) equals the taxes paid (`tax.products` and `tax.production`),
/// all summed over each day's flows, within 10% (restated by the owner,
/// 2026-10-22).
///
/// "The taxes paid" are the government's receipts under `tax.products` and
/// `tax.production` in the world with them. "The fall in firm units'
/// deposits" is their deposits in the world without them less those in the
/// world with them. "The fall in the dividends they paid" is the firms' net
/// payments under `income.dividend` in the world without them less those in
/// the world with them; "the fall in the profit tax they paid" is the same
/// for the firms' net payments under `tax.profit` and `tax.micro`. Flows are
/// summed over each day (zero when there are none).
#[test]
fn ac_grv_03() {
    let mut with = world_2021(1000);
    let mut without = world_2021_with(
        1000,
        vec![StartEdit::WithoutRevenue(RevenuePart::ProductTaxes)],
        params(),
    );
    assert_eq!(
        with.unit_costs_ppm(),
        without.unit_costs_ppm(),
        "starting unit costs"
    );
    assert_eq!(
        with.industries().price_ppm,
        without.industries().price_ppm,
        "starting prices"
    );
    let taxes = [code("tax.products"), code("tax.production")];
    let dividend = code("income.dividend");
    let profit_taxes = [code("tax.profit"), code("tax.micro")];
    // The taxes the first world paid and each world's dividends and profit
    // tax, day by day.
    let mut paid = Bani::ZERO;
    let mut dividends = [Bani::ZERO; 2];
    let mut profit_tax = [Bani::ZERO; 2];
    for (k, w) in [&mut with, &mut without].into_iter().enumerate() {
        while w.months().len() < 12 {
            step(w);
            if k == 0 {
                for &c in &taxes {
                    paid += received(w, c, Sector::Government);
                }
            }
            dividends[k] += Bani::ZERO - w.ledger().flows().get(dividend, Sector::Firms);
            for &c in &profit_taxes {
                profit_tax[k] += Bani::ZERO - w.ledger().flows().get(c, Sector::Firms);
            }
        }
    }
    let lower = total(&without.firm_units().deposits) - total(&with.firm_units().deposits);
    let fewer_dividends = dividends[1] - dividends[0];
    let less_profit_tax = profit_tax[1] - profit_tax[0];
    eprintln!(
        "AC-GRV-03: taxes paid {:.2} billion lei, firm deposits lower by {:.2}, dividends lower by {:.2}, profit tax lower by {:.2}",
        bn(paid),
        bn(lower),
        bn(fewer_dividends),
        bn(less_profit_tax)
    );
    assert!(paid > Bani::ZERO);
    assert!(
        gap_ppm(lower + fewer_dividends + less_profit_tax, paid) <= 100_000,
        "deposits lower by {lower:?}, dividends by {fewer_dividends:?} and profit tax by {less_profit_tax:?}, taxes paid {paid:?}"
    );
}

/// **AC-GRV-04** `[sim]` At every month end, `tax.production` equals the sum
/// of `o_j` times each unit's month's output value. The first year is within
/// 20% of 8.10 billion lei.
///
/// `o_j` is the table's other taxes less subsidies over output, a NACE
/// row counting only when positive (negative rows are subsidies, paid
/// elsewhere); for a catalogue industry with several NACE parts, its positive
/// rows over its output. A unit's tax is rounded, one ban.
#[test]
fn ac_grv_04() {
    let start = start_state(1000);
    for (j, id) in start.industry_id.iter().enumerate() {
        let positive = by_parts(id, |nj| row("other_taxes_less_subsidies", nj).max(0.0));
        let o = positive / by_parts(id, |nj| row("output", nj)) * 1e6;
        assert!(
            close(f64::from(start.production_tax_ppm[j]), o, 1.0),
            "{id}: o {} against {o}",
            start.production_tax_ppm[j]
        );
    }
    let mut w = world_2021(1000);
    let production = code("tax.production");
    let mut year = Bani::ZERO;
    let mut checked = 0;
    while checked < 12 {
        let r = step(&mut w);
        assert_checks(&r);
        let got = received(&w, production, Sector::Government);
        if !r.date.is_month_end() {
            assert_eq!(
                got,
                Bani::ZERO,
                "{:?}: tax.production off a month end",
                r.date
            );
            continue;
        }
        let m = &w.revenue_months()[checked];
        let mut sum = Bani::ZERO;
        for u in 0..start.unit_industry.len() {
            let o = i128::from(start.production_tax_ppm[usize::from(start.unit_industry[u])]);
            let expected = i128::from(m.unit_output_value[u].get()) * o;
            let paid = m.unit_production_tax[u];
            assert!(
                (i128::from(paid.get()) * 1_000_000 - expected).abs() <= 1_000_000,
                "{:?}: unit {u} pays {paid:?} on output {:?}",
                m.month,
                m.unit_output_value[u]
            );
            sum += paid;
        }
        assert_eq!(
            got, sum,
            "{:?}: tax.production and the units' taxes",
            m.month
        );
        year += got;
        checked += 1;
    }
    eprintln!(
        "AC-GRV-04: the first year's other taxes on production {:.2} billion lei (8.10)",
        bn(year)
    );
    assert!(gap_ppm(year, million_lei(8_100.0)) <= 200_000, "{:?}", year);
}

/// A held revenue line of the spec's table: code, million lei a year, the
/// sectors that pay it.
fn held_lines() -> Vec<(FlowCode, f64, Vec<Sector>)> {
    vec![
        (code("tax.property"), 3_864.1, vec![Sector::Households]),
        (code("tax.gambling"), 2_376.3, vec![Sector::Households]),
        (code("rent.natural_resources"), 2_619.6, vec![Sector::Firms]),
        (code("interest.gov_received"), 1_287.1, vec![Sector::Banks]),
        (
            code("transfer.other_received"),
            6_418.7,
            vec![Sector::Households, Sector::Firms],
        ),
        (
            code("transfer.capital_received"),
            4_857.2,
            vec![Sector::Firms],
        ),
    ]
}

/// **AC-GRV-05** `[sim]` Each held revenue line is paid every month end by
/// its payers only, under its code. Twelve months add to its 2021 amount
/// exactly, less any household shortfall, which is reported. Every owner
/// household pays the same property tax and no tenant pays any.
///
/// "By its payers only": on the month end, the line's net flow is negative
/// for its payers only and zero for every other sector but the
/// government's. Owners that fall short pay less; every other owner pays the
/// same amount.
#[test]
fn ac_grv_05() {
    let start = start_state(1000);
    let mut w = world_2021(1000);
    let lines = held_lines();
    let total_year: f64 = lines.iter().map(|l| l.1).sum();
    assert!(close(total_year, 21_423.0, 0.05));
    let weights = w.households().hh_weight.clone();
    let owners: i64 = (0..weights.len())
        .filter(|&h| start.hh_tenure[h] == 0)
        .map(|h| i64::from(weights[h]))
        .sum();
    let mut paid_year: BTreeMap<FlowCode, Bani> = BTreeMap::new();
    let mut short_year: BTreeMap<FlowCode, Bani> = BTreeMap::new();
    let mut checked = 0u8;
    while checked < 12 {
        let r = step(&mut w);
        assert_checks(&r);
        if !r.date.is_month_end() {
            for (c, _, _) in &lines {
                assert_eq!(
                    received(&w, *c, Sector::Government),
                    Bani::ZERO,
                    "{:?}: {} off a month end",
                    r.date,
                    c.name()
                );
            }
            continue;
        }
        let m = &w.revenue_months()[usize::from(checked)];
        assert_eq!(m.held.len(), lines.len(), "a payment per held line");
        for (k, (c, per_year, payers)) in lines.iter().enumerate() {
            let h = &m.held[k];
            assert_eq!(h.code, *c, "the spec's order");
            assert_eq!(
                h.due,
                Budget::monthly(million_lei(*per_year), checked),
                "{:?}: {}'s twelfth",
                m.month,
                c.name()
            );
            assert_eq!(
                h.paid + h.shortfall,
                h.due,
                "{:?}: {} paid and forgiven",
                m.month,
                c.name()
            );
            assert!(h.shortfall >= Bani::ZERO);
            if !payers.contains(&Sector::Households) {
                assert_eq!(
                    h.shortfall,
                    Bani::ZERO,
                    "{:?}: {}: only households fall short",
                    m.month,
                    c.name()
                );
            }
            assert_eq!(
                received(&w, *c, Sector::Government),
                h.paid,
                "{:?}: {} received",
                m.month,
                c.name()
            );
            let mut by_payers = Bani::ZERO;
            for s in Sector::ALL {
                let net = w.ledger().flows().get(*c, s);
                if s == Sector::Government {
                    continue;
                }
                if payers.contains(&s) {
                    assert!(
                        net <= Bani::ZERO,
                        "{:?}: {} {s:?} receives it",
                        m.month,
                        c.name()
                    );
                    by_payers += net;
                } else {
                    assert_eq!(net, Bani::ZERO, "{:?}: {} paid by {s:?}", m.month, c.name());
                }
            }
            assert_eq!(
                by_payers,
                Bani::ZERO - h.paid,
                "{:?}: {} paid by its payers",
                m.month,
                c.name()
            );
            *paid_year.entry(*c).or_insert(Bani::ZERO) += h.paid;
            *short_year.entry(*c).or_insert(Bani::ZERO) += h.shortfall;
        }
        // Property tax: owners alike, tenants none.
        let tax = &m.hh_property_tax;
        let most = tax.iter().copied().max().unwrap_or(Bani::ZERO);
        let mut alike = 0i64;
        for h in 0..tax.len() {
            if start.hh_tenure[h] != 0 {
                assert_eq!(
                    tax[h],
                    Bani::ZERO,
                    "{:?}: household {h} is not an owner and pays property tax",
                    m.month
                );
            } else {
                assert!(tax[h] <= most);
                if tax[h] == most {
                    alike += i64::from(weights[h]);
                }
            }
        }
        assert!(
            alike * 100 >= owners * 99,
            "{:?}: owners paying the common amount {alike} of {owners}",
            m.month
        );
        let due = m.held[0].due;
        assert!(
            (most.get() * owners - due.get()).abs() <= owners,
            "{:?}: {most:?} per owner household, {due:?} in all",
            m.month
        );
        assert_eq!(
            weighted(&weights, tax),
            m.held[0].paid,
            "{:?}: property tax by household",
            m.month
        );
        checked += 1;
    }
    for (c, per_year, _) in &lines {
        assert_eq!(
            paid_year[c] + short_year[c],
            million_lei(*per_year),
            "{}: twelve months",
            c.name()
        );
        eprintln!(
            "AC-GRV-05: {} paid {:.3} billion lei, forgiven {:.3}",
            c.name(),
            bn(paid_year[c]),
            bn(short_year[c])
        );
    }
}

/// A held benefit of the spec's table: the programme, its code, million lei
/// a year (approximate for the residuals).
#[cfg(feature = "pay_day")]
fn held_benefits() -> Vec<(&'static str, FlowCode, f64, bool)> {
    vec![
        (
            "sickness_disability.disability",
            code("benefit.disability"),
            7_138.8,
            true,
        ),
        (
            "sickness_disability.sickness",
            code("benefit.disability"),
            3_059.5,
            true,
        ),
        ("family_children", code("benefit.family"), 7_736.0, false),
        (
            "social_exclusion",
            code("benefit.social_assistance"),
            1_107.0,
            false,
        ),
        ("housing", code("benefit.housing"), 68.7, true),
        (
            "social_protection_other",
            code("benefit.other"),
            1_520.6,
            true,
        ),
        ("education_grants", code("benefit.student"), 2_926.3, true),
    ]
}

/// **AC-GRV-06** `[sim]` On every 15th (the held benefits' payment day,
/// restated by the owner from pay day, 2026-10-22) each held benefit pays
/// exactly its monthly amount, less rounding under one ban per record, to
/// its key group only. Recipients' `hh_income_received` grows by it. Twelve
/// months add to the programme's held amount. The keys are read on the day
/// of payment: the sickness part's employees are those employed on the
/// 15th, not those at load (restated by the owner, 2026-10-22).
///
/// The codes that carry only held benefits (`benefit.disability`,
/// `benefit.housing`, `benefit.student`) pay nothing on other days. Income
/// is checked for households that did not decide on the 15th: it grows by at
/// least their held benefits; without `investment` (no dividends that day)
/// it grows by exactly their held benefits and imputed rent, which is paid
/// the same day.
///
/// "One ban per record" is per member of the key group with its weight. The
/// residual programmes' yearly amounts are fixed at load; the spec gives them
/// approximately, so they are checked within 5%. The keys the starting state
/// shows are checked: persons aged 65 or over; households with a child under
/// 2; households of the lowest fifth; employees.
///
/// "Employees on the 15th" means persons whose current declared wage
/// (`World::persons().wage_declared`) is above zero. A household whose having
/// an employee is the same before and after the 15th's step must be in the
/// key exactly when it has one; a household whose employment changed during
/// the 15th may be either (the spec does not order the payment against the
/// day's hiring and separations). In the same way "the lowest income fifth"
/// is read on the 15th: a household in the key is in the lowest
/// `hh_fifth` before or after the 15th's step (households deciding that day
/// move between fifths).
#[cfg(feature = "pay_day")]
#[test]
fn ac_grv_06() {
    let start = start_state(1000);
    let mut w = world_2021(1000);
    let programmes = held_benefits();
    let weights = w.households().hh_weight.clone();
    let n = weights.len();
    let mut members = vec![0i64; n];
    let mut aged = vec![false; n];
    let mut baby = vec![false; n];
    for (p, &h) in start.household_id.iter().enumerate() {
        let h = h as usize;
        members[h] += 1;
        aged[h] |= start.age[p] >= 65 * 12;
        baby[h] |= start.age[p] < 2 * 12;
    }
    // Households with a person whose current declared wage is above zero.
    let employed = |w: &econ_core::world::World| {
        let mut e = vec![false; n];
        let persons = w.persons();
        for (p, &h) in persons.household_id.iter().enumerate() {
            e[h as usize] |= persons.wage_declared[p] > Bani::ZERO;
        }
        e
    };
    let mut due_year = vec![Bani::ZERO; programmes.len()];
    let held_only = [
        code("benefit.disability"),
        code("benefit.housing"),
        code("benefit.student"),
    ];
    let mut pay_days = 0u8;
    while pay_days < 12 {
        let before = w.households().hh_income_received.clone();
        let employee_before = employed(&w);
        let fifth_before = w.households().hh_fifth.clone();
        let r = step(&mut w);
        assert_checks(&r);
        if r.date.day != 15 {
            for c in held_only {
                assert_eq!(
                    received(&w, c, Sector::Households),
                    Bani::ZERO,
                    "{:?}: {} off the 15th",
                    r.date,
                    c.name()
                );
            }
            continue;
        }
        let held = w.held_benefits();
        assert_eq!(
            held.len(),
            programmes.len(),
            "{:?}: a payment per held benefit",
            r.date
        );
        let mut by_code: BTreeMap<FlowCode, Bani> = BTreeMap::new();
        let mut held_hh = vec![Bani::ZERO; n];
        for (k, (name, c, _, _)) in programmes.iter().enumerate() {
            let b = &held[k];
            assert_eq!(b.programme, *name, "the spec's order");
            assert_eq!(b.code, *c, "{name}'s code");
            let paid = weighted(&weights, &b.received);
            let room: i64 = (0..n)
                .filter(|&h| b.in_key[h])
                .map(|h| i64::from(weights[h]) * members[h])
                .sum();
            assert!(
                paid <= b.due && (b.due - paid).get() <= room,
                "{:?}: {name} pays {paid:?} of {:?}",
                r.date,
                b.due
            );
            for (h, held) in held_hh.iter_mut().enumerate() {
                if b.received[h] != Bani::ZERO {
                    assert!(
                        b.in_key[h],
                        "{:?}: {name} pays household {h}, outside its key",
                        r.date
                    );
                }
                *held += b.received[h];
            }
            match *name {
                "social_protection_other" => {
                    assert_eq!(b.in_key, aged, "{name}: persons aged 65 or over")
                }
                "family_children" => {
                    assert_eq!(b.in_key, baby, "{name}: households with a child under 2")
                }
                "sickness_disability.sickness" => {
                    let employee_after = employed(&w);
                    for h in 0..n {
                        if employee_before[h] == employee_after[h] {
                            assert_eq!(
                                b.in_key[h], employee_after[h],
                                "{:?}: {name}: household {h}, employees on the 15th",
                                r.date
                            );
                        }
                    }
                }
                "housing" | "social_exclusion" => {
                    let fifth_after = &w.households().hh_fifth;
                    let lowest_before = fifth_before.iter().copied().min().unwrap_or(0);
                    let lowest_after = fifth_after.iter().copied().min().unwrap_or(0);
                    for h in 0..n {
                        if b.in_key[h] {
                            assert!(
                                fifth_before[h] == lowest_before || fifth_after[h] == lowest_after,
                                "{:?}: {name}: household {h} not in the lowest fifth on the 15th \
                                 (fifth {} before the day, {} after)",
                                r.date,
                                fifth_before[h],
                                fifth_after[h]
                            );
                        }
                    }
                }
                _ => {}
            }
            *by_code.entry(*c).or_insert(Bani::ZERO) += paid;
            due_year[k] += b.due;
        }
        // The new codes carry the held benefits only.
        for c in held_only {
            assert_eq!(
                received(&w, c, Sector::Households),
                by_code[&c],
                "{:?}: {} to households",
                r.date,
                c.name()
            );
        }
        for (&c, &v) in &by_code {
            assert!(
                received(&w, c, Sector::Households) >= v,
                "{:?}: {}",
                r.date,
                c.name()
            );
        }
        // Income: what households received on the 15th, the held benefits in
        // it.
        let rent_today = w.imputed_rent().day == Some(r.day);
        let after = &w.households().hh_income_received;
        for h in 0..n {
            if w.households().last_decision[h] == Some(r.day) {
                continue;
            }
            let grew = after[h] - before[h];
            assert!(
                grew >= held_hh[h],
                "{:?}: household {h}'s income grew by {grew:?}, its held benefits {:?}",
                r.date,
                held_hh[h]
            );
            if cfg!(not(feature = "investment")) {
                let rent = if rent_today {
                    w.imputed_rent().received[h]
                } else {
                    Bani::ZERO
                };
                assert_eq!(
                    grew,
                    held_hh[h] + rent,
                    "{:?}: household {h}'s income received: held benefits {:?}, imputed rent {rent:?}",
                    r.date,
                    held_hh[h]
                );
            }
        }
        pay_days += 1;
    }
    for (k, (name, _, per_year, exact)) in programmes.iter().enumerate() {
        let target = million_lei(*per_year);
        eprintln!(
            "AC-GRV-06: {name}: {:.3} billion lei a year",
            bn(due_year[k])
        );
        if *exact {
            assert_eq!(due_year[k], target, "{name}: twelve months");
        } else {
            assert!(
                gap_ppm(due_year[k], target) <= 50_000,
                "{name}: {:?} against about {target:?}",
                due_year[k]
            );
        }
    }
}

/// **AC-GRV-07** `[sim]` The first year's fiscal report (fiscal policy's
/// report of the deficit's parts) lists revenue by ESA line next to the
/// research note's 2021 figures, and total revenue against TR less public
/// units' sales and imputed contributions (352.8 billion lei). With excise
/// and EU funds on, the total is within 15% (a check, not a target).
#[test]
fn ac_grv_07() {
    let mut w = world_2021(1000);
    while w.fiscal_months().len() < 12 {
        step(&mut w);
    }
    let mut revenue: BTreeMap<FlowCode, Bani> = BTreeMap::new();
    for m in &w.fiscal_months()[..12] {
        for &(c, v) in &m.revenue {
            *revenue.entry(c).or_insert(Bani::ZERO) += v;
        }
    }
    // The research note's 2021 figures, million lei, by the code that
    // carries them.
    let figures: [(&str, f64, &str); 16] = [
        ("tax.vat", 76_336.1, "D211"),
        (
            "tax.products",
            6_120.2,
            "other D21 (with non-deductible VAT, in D211)",
        ),
        ("tax.excise", 34_663.5, "D214A, D2122C"),
        ("tax.production", 9_683.5, "D29"),
        ("tax.income", 28_493.9, "D51A"),
        ("tax.profit", 26_285.3, "D51B (with tax.micro)"),
        ("tax.gambling", 2_376.3, "D51D"),
        ("tax.property", 3_864.1, "D59"),
        (
            "contrib.pension",
            124_334.0,
            "D611 + D613 (all contributions)",
        ),
        ("tax.dividend", 0.0, "in D51A"),
        ("interest.gov_received", 1_287.1, "D41"),
        ("rent.natural_resources", 2_619.6, "D45"),
        ("transfer.eu_grants_current", 6_550.2, "D74"),
        ("transfer.other_received", 6_418.7, "D75, D72"),
        ("transfer.eu_grants_capital", 16_017.5, "D92 from the EU"),
        ("transfer.capital_received", 4_857.2, "D9 from residents"),
    ];
    eprintln!("AC-GRV-07, the first year's revenue, billion lei (2021, ESA):");
    for (name, v2021, esa) in figures {
        let got = revenue.get(&code(name)).copied().unwrap_or(Bani::ZERO);
        eprintln!("  {name}: {:.2} ({:.2}, {esa})", bn(got), v2021 / 1e3);
    }
    for (c, v) in &revenue {
        if !figures.iter().any(|f| code(f.0) == *c) {
            eprintln!("  {}: {:.2}", c.name(), bn(*v));
        }
    }
    let sum = revenue.values().copied().fold(Bani::ZERO, |a, b| a + b);
    let target = million_lei(388_854.5 - 26_482.3 - 9_598.1);
    eprintln!(
        "  total: {:.2} (TR less sales and imputed contributions {:.2})",
        bn(sum),
        bn(target)
    );
    assert!(
        gap_ppm(million_lei(352_774.1), target) < 10,
        "the spec's 352.8 billion"
    );
    for name in [
        "tax.excise",
        "tax.products",
        "tax.production",
        "tax.property",
        "transfer.eu_grants_current",
        "transfer.eu_grants_capital",
    ] {
        assert!(
            revenue.contains_key(&code(name)),
            "the fiscal report has no {name}"
        );
    }
    assert!(
        gap_ppm(sum, target) <= 150_000,
        "revenue {:.2} billion lei against {:.2}",
        bn(sum),
        bn(target)
    );
}
