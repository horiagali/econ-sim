//! Acceptance tests for AC-MIG-05 to AC-MIG-17 (spec:
//! economy/migration-remittances), the `[sim]` criteria, on the `World`.
//! Written by the test writer from the spec and its API sketch before the
//! code (ADR-0014), against the test writer's additions behind the feature
//! `migration`: the month's flows (`World::migration_months`), the diaspora
//! (`World::diaspora`), each person's `abroad` flag (`World::abroad`), and
//! the start edits `StartEdit::NoDiaspora` (the counterfactual of AC-MIG-07
//! and 14) and `StartEdit::MigrationUnemploymentShock` (AC-MIG-15).
//! AC-MIG-01 to 04 (`[unit]`) are in `econ-mech-migration`.
//!
//! "1:1000" and "1:100" are the Romania scenario at that scale, seed 42,
//! under the law of its folder (`law.json`, `world.json`: the 2021 law), as
//! in the M4 suite. Day 0 is 1 December 2021 (month 0); "the first year" is
//! the first 12 monthly closes (months 0 to 11).
//!
//! Readings shared by the tests:
//! - Persons are weighted (real persons). Residents are the persons of
//!   records of positive weight without `abroad`; the diaspora is the
//!   weighted persons with `abroad` (`DiasporaView::linked`) plus the stock.
//! - `world.wages_abroad` is a rate in basis points of the start (10,000 =
//!   100%, the spec's ω = 1 at the start): +20% is 12,000, −20% 8,000, 130%
//!   13,000. An order given on day 0 takes effect on the day the lever
//!   system returns; "the next 24 months" are the 24 closes from the month of
//!   that day.
//! - Runs of more than a year at 1:1000, and every run at 1:100, are release
//!   builds only (ignored under `debug_assertions`, the form of AC-WLD-10).

use econ_core::levers::{LeverId, LeverValue};
use econ_core::world::{MIGRATION_AGE_BANDS, MigrationMonth, StartEdit, World};
use econ_ledger::{Instrument, Sector};
use econ_types::{Bani, Date};

use super::support::params;
use super::support_lev::{give, set};
use super::support_m3::{FlowSum, code, is_financing, legs_of_day, legs_sum};
use super::support_m5::{
    county, rate_in_force, region_of_county, residents, step_checked, weights_of_persons,
    world_2021,
};

/// A world at a scale with these start edits, spec defaults.
fn world(scale: u32, edits: Vec<StartEdit>) -> World {
    world_2021(scale, params(), edits)
}

/// The month index (0 = December 2021) of a date.
fn month_of(d: Date) -> usize {
    let m = (d.year - 2021) * 12 + i32::from(d.month) - 12;
    usize::try_from(m).expect("a date on or after December 2021")
}

/// Step, checking every day, until `months` months are closed.
fn run(w: &mut World, months: usize) {
    while w.months().len() < months {
        step_checked(w);
    }
}

/// The closed months `from..to` of a world's migration report.
fn span(w: &World, from: usize, to: usize) -> &[MigrationMonth] {
    let m = w.migration_months();
    assert!(
        m.len() >= to,
        "{} months of migration, {to} needed",
        m.len()
    );
    for (i, x) in m.iter().enumerate() {
        assert_eq!(
            x.month.0 as usize, i,
            "one migration report per close, in order"
        );
    }
    &m[from..to]
}

/// The sum of a field over months.
fn sum(months: &[MigrationMonth], f: impl Fn(&MigrationMonth) -> u64) -> u64 {
    months.iter().map(f).sum()
}

/// The diaspora now: linked plus the stock.
fn diaspora_total(w: &World) -> u64 {
    let d = w.diaspora();
    d.linked + d.stock.iter().map(|c| c.persons).sum::<u64>()
}

/// Weighted persons with `abroad` in records of positive weight.
fn linked_from_persons(w: &World) -> u64 {
    let weights = weights_of_persons(w);
    w.abroad()
        .iter()
        .zip(&weights)
        .filter(|(a, wt)| **a && **wt > 0)
        .map(|(_, &wt)| u64::from(wt))
        .sum()
}

/// `world.wages_abroad` at the start must be 100% (10,000 basis points).
fn wages_abroad_at_start(w: &World) -> i32 {
    let now = rate_in_force(w, LeverId::WorldWagesAbroad, None);
    assert_eq!(now, 10_000, "world.wages_abroad is 100% at the start");
    now
}

/// A world at 1:1000 with an order given on day 0; the month it takes
/// effect.
fn with_order(lever: LeverId, value: LeverValue) -> (World, usize) {
    let mut w = world(1000, Vec::new());
    let a = give(&mut w, set(lever, None, value));
    (w, month_of(a.effective))
}

/// `|a − b| ≤ pct % of b`.
fn within_pct(a: u64, b: u64, pct: u64) -> bool {
    a.abs_diff(b) * 100 <= b * pct
}

/// **AC-MIG-05** `[sim]` First year on the Romania scenario under the 2021
/// law: weighted emigrants are 216,861 ± 10%, returns of Romanian citizens
/// 149,513 ± 10%, foreign immigrants 45,043 ± 10%; each age band's share of
/// emigrants from 15 to 64 is within 3 points of the research note's table.
///
/// Readings: at 1:1000, the first 12 closes. The research note's table
/// (Eurostat `migr_emi2`, 2021) gives 39,259, 62,148, 45,638, 31,050 and
/// 13,111 emigrants aged 15–24, 25–34, 35–44, 45–54 and 55–64 out of 216,858
/// (the table's sum); a band's share is its emigrants over all emigrants of
/// the year, and "within 3 points" is within 30,000 millionths, inclusive.
#[test]
fn ac_mig_05() {
    let mut w = world(1000, Vec::new());
    run(&mut w, 12);
    let year = span(&w, 0, 12);
    let emigrants = sum(year, |m| m.emigrants);
    let returns = sum(year, |m| m.returns);
    let immigrants = sum(year, |m| m.immigrants);
    assert!(
        within_pct(emigrants, 216_861, 10),
        "emigrants {emigrants} against 216,861 ± 10%"
    );
    assert!(
        within_pct(returns, 149_513, 10),
        "returns {returns} against 149,513 ± 10%"
    );
    assert!(
        within_pct(immigrants, 45_043, 10),
        "foreign immigrants {immigrants} against 45,043 ± 10%"
    );

    let mut by_age = [0u64; 7];
    for m in year {
        assert_eq!(
            m.emigrants_by_age.iter().sum::<u64>(),
            m.emigrants,
            "{:?}: emigrants by age add up to the emigrants",
            m.month
        );
        for (b, &n) in m.emigrants_by_age.iter().enumerate() {
            by_age[b] += n;
        }
    }
    // Bands 1 to 5 of MIGRATION_AGE_BANDS: 15–24 … 55–64.
    let table: [(usize, u64); 5] = [
        (1, 39_259),
        (2, 62_148),
        (3, 45_638),
        (4, 31_050),
        (5, 13_111),
    ];
    let table_total: u64 = 216_858;
    for (b, n) in table {
        assert_eq!(
            MIGRATION_AGE_BANDS[b].0,
            5 + 10 * b as u16,
            "band {b} starts at {}",
            5 + 10 * b
        );
        let share = i128::from(by_age[b]) * 1_000_000 / i128::from(emigrants);
        let want = i128::from(n) * 1_000_000 / i128::from(table_total);
        assert!(
            (share - want).abs() <= 30_000,
            "ages {:?}: share {share} millionths against {want}",
            MIGRATION_AGE_BANDS[b]
        );
    }
}

/// **AC-MIG-06** `[sim]` First 12 months: remittances received by
/// households total 17,248 million lei ± 3% (the exchange rate's drift
/// aside: the test divides each month by e_t / e0); every lei of them is
/// posted from the rest of the world to households under
/// `transfer.remittances`, and the month report counts them in the current
/// account.
///
/// Readings: at 1:1000. `e_t` is the exchange rate in force on the month's
/// last day, when remittances are paid (`Foreign::exchange_rate_ppm` before
/// that day runs), and `e0` its starting value. "Every lei": over each
/// month, the households' flows under `transfer.remittances` are the month's
/// remittances of the report and the rest of the world's are their negative,
/// with no other sector; every leg of the code moves deposits from the rest
/// of the world to households. "Counted in the current account": the code is
/// not a financial transaction, and the month's `current_account` is what
/// the rest of the world pays residents less what it receives outside
/// financial transactions (the identity of AC-TFX-03), so it holds the
/// remittances.
#[test]
fn ac_mig_06() {
    let remit = code("transfer.remittances");
    assert!(
        !is_financing(remit),
        "remittances are secondary income, not a financial transaction"
    );
    let mut w = world(1000, Vec::new());
    let e0 = i128::from(w.foreign().start_rate_ppm);
    let mut month = FlowSum::default();
    let mut rate_on_pay_day: Option<i128> = None;
    let mut deflated: i128 = 0;
    while w.months().len() < 12 {
        if w.date().is_month_end() {
            rate_on_pay_day = Some(i128::from(w.foreign().exchange_rate_ppm));
        }
        let closes = w.months().len();
        let r = step_checked(&mut w);
        month.add_day(&w);
        for leg in legs_of_day(&w, &r).iter().filter(|l| l.code == remit) {
            assert_eq!(
                (leg.payer, leg.payee, leg.instrument),
                (
                    Sector::RestOfWorld,
                    Sector::Households,
                    Instrument::Deposits
                ),
                "{:?}: a leg of transfer.remittances",
                r.date
            );
        }
        if w.months().len() == closes {
            continue;
        }
        let m = &w.migration_months()[closes];
        assert!(
            m.remittances > Bani::ZERO,
            "{:?}: remittances are paid",
            m.month
        );
        assert_eq!(
            month.get(remit, Sector::Households),
            m.remittances,
            "{:?}: households",
            m.month
        );
        assert_eq!(
            month.get(remit, Sector::RestOfWorld),
            Bani::ZERO - m.remittances,
            "{:?}: the rest of the world",
            m.month
        );
        for s in Sector::ALL {
            if s != Sector::Households && s != Sector::RestOfWorld {
                assert_eq!(
                    month.get(remit, s),
                    Bani::ZERO,
                    "{:?}: {s:?} under transfer.remittances",
                    m.month
                );
            }
        }
        let ca = month
            .of(Sector::RestOfWorld)
            .into_iter()
            .filter(|&(c, _)| !is_financing(c))
            .map(|(_, v)| Bani::ZERO - v)
            .fold(Bani::ZERO, |a, b| a + b);
        assert_eq!(
            w.trade_months()[closes].current_account,
            ca,
            "{:?}: the current account",
            m.month
        );
        let e = rate_on_pay_day.take().expect("the month's last day ran");
        deflated += i128::from(m.remittances.get()) * e0 / e;
        month = FlowSum::default();
    }
    let target: i128 = 17_248_000_000 * 100; // bani
    assert!(
        (deflated - target).abs() * 100 <= target * 3,
        "remittances of the first year at the starting rate: {deflated} bani against {target} ± 3%"
    );
}

/// What households received in a day: their positive flows by code, outside
/// financial transactions.
fn households_received_today(w: &World) -> i128 {
    w.ledger()
        .flows()
        .by_code_sector
        .iter()
        .filter(|((c, s), v)| {
            *s == Sector::Households && c.is_flow() && !is_financing(*c) && v.get() > 0
        })
        .map(|(_, v)| i128::from(v.get()))
        .sum()
}

/// The households' income and consumption of the first year.
fn first_year_income(edits: Vec<StartEdit>) -> (i128, i128) {
    let mut w = world(1000, edits);
    let mut income = 0;
    while w.months().len() < 12 {
        step_checked(&mut w);
        income += households_received_today(&w);
    }
    let consumption = w
        .months()
        .iter()
        .map(|m| i128::from(m.consumption.get()))
        .sum();
    (income, consumption)
}

/// **AC-MIG-07** `[sim]` Remittances close part of the households' income
/// gap: over the first year, households' total income with migration on
/// exceeds a counterfactual with the diaspora tables removed by at least 16
/// billion lei, and their consumption is higher.
///
/// Readings: at 1:1000. The counterfactual is `StartEdit::NoDiaspora` (the
/// scenario without its diaspora tables: emigration and moves still run).
/// Households' income is what they received: each day, the sum of their
/// positive flows by code outside financial transactions (wages, pensions,
/// benefits, interest, remittances, …), over the first 12 closes;
/// consumption is the months' `consumption`.
#[test]
fn ac_mig_07() {
    let (income, consumption) = first_year_income(Vec::new());
    let (income_cf, consumption_cf) = first_year_income(vec![StartEdit::NoDiaspora]);
    let gap = income - income_cf;
    assert!(
        gap >= 16_000_000_000 * 100,
        "households' income exceeds the counterfactual's by {gap} bani, below 16 billion lei"
    );
    assert!(
        consumption > consumption_cf,
        "consumption {consumption} against {consumption_cf}"
    );
}

/// AC-MIG-08's balances on a world through `months` closes.
fn assert_persons_balance(scale: u32, months: usize) {
    let mut w = world(scale, Vec::new());
    let mut resident = residents(&w);
    let mut diaspora = diaspora_total(&w);
    assert_eq!(
        w.diaspora().linked,
        linked_from_persons(&w),
        "1:{scale}, day 0: linked emigrants"
    );
    while w.months().len() < months {
        let closes = w.months().len();
        step_checked(&mut w);
        if w.months().len() == closes {
            continue;
        }
        let m = &w.migration_months()[closes];
        let what = format!("1:{scale}, {:?}", m.month);
        let r = residents(&w);
        let d = diaspora_total(&w);
        let dr = i128::from(r) - i128::from(resident);
        let want_r = i128::from(m.births) - i128::from(m.deaths_resident) - i128::from(m.emigrants)
            + i128::from(m.returns)
            + i128::from(m.immigrants);
        assert_eq!(
            dr, want_r,
            "{what}: residents change by births − deaths − emigrants + returns + immigrants"
        );
        let dd = i128::from(d) - i128::from(diaspora);
        let want_d = i128::from(m.emigrants) - i128::from(m.returns) - i128::from(m.deaths_abroad);
        assert_eq!(
            dd, want_d,
            "{what}: the diaspora changes by emigrants − returns − deaths abroad"
        );
        assert_eq!(
            w.diaspora().linked,
            linked_from_persons(&w),
            "{what}: linked emigrants"
        );
        assert_eq!(
            m.moves_in.len(),
            m.moves_out.len(),
            "{what}: moves by county"
        );
        assert_eq!(
            m.moves_in.iter().sum::<u64>(),
            m.moves_out.iter().sum::<u64>(),
            "{what}: moves between counties sum to zero nationally"
        );
        resident = r;
        diaspora = d;
    }
}

/// **AC-MIG-08** `[sim]` Conservation of persons, every month, at 1:1000 and
/// 1:100: resident persons change by exactly births − resident deaths −
/// emigrants + returns + foreign immigrants, and the diaspora (linked plus
/// stock) by exactly emigrants − returns − deaths abroad; moves between
/// counties sum to zero nationally; no weight or count is negative.
///
/// Readings: every close of 24 months at 1:1000 and of 12 months at 1:100,
/// against the close before (day 0 for month 0). Weights and counts are
/// unsigned, so "no negative" is that every balance above holds in signed
/// arithmetic and the linked count of the diaspora is the weighted `abroad`
/// persons of the persons table. Release builds only.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "24 months at 1:1000 and a year at 1:100: cargo test --release -p econ-core --features migration --test acceptance migration::"
)]
fn ac_mig_08() {
    assert_persons_balance(1000, 24);
    assert_persons_balance(100, 12);
}

/// AC-MIG-09's checks on a world through a year.
fn assert_money_with_migration(scale: u32) {
    let capital = code("transfer.migrant_capital");
    let mut w = world(scale, Vec::new());
    let mut whole = 0;
    while w.months().len() < 12 {
        let weights_before = w.households().hh_weight.clone();
        let closes = w.months().len();
        // Every ledger invariant, households' deposits against the ledger
        // (I-4), no negative money: the checks of the day.
        let r = step_checked(&mut w);
        if w.months().len() == closes {
            continue;
        }
        let m = &w.migration_months()[closes];
        let mut moved: i128 = 0;
        for &(h, wt, net) in &m.emigrated_households {
            assert_eq!(
                weights_before.get(h as usize).copied(),
                Some(wt),
                "1:{scale}, {:?}: household {h} left with its whole weight",
                m.month
            );
            assert!(
                wt > 0 && net >= Bani::ZERO,
                "1:{scale}, {:?}: household {h}: {wt}, {net:?}",
                m.month
            );
            moved += i128::from(wt) * i128::from(net.get());
            whole += 1;
        }
        let legs = legs_of_day(&w, &r);
        let posted = legs_sum(
            &legs,
            capital,
            Instrument::Deposits,
            Sector::Households,
            Sector::RestOfWorld,
        );
        assert_eq!(
            i128::from(posted.get()),
            moved,
            "1:{scale}, {:?}: the emigrating households' net deposits go to the rest of the world under transfer.migrant_capital",
            m.month
        );
    }
    assert!(
        whole > 0,
        "1:{scale}: some households emigrated whole in a year"
    );
}

/// **AC-MIG-09** `[sim]` Conservation of money with migration on, every day,
/// at 1:1000 and 1:100: every ledger invariant holds (I-1 to I-8 of
/// ADR-0007); a whole household's emigration moves exactly its net deposits
/// to the rest of the world under `transfer.migrant_capital`; the weighted
/// sum of household deposits equals the banks' deposit liabilities to
/// households.
///
/// Readings: every day of the first year at each scale. The invariants are
/// the ledger's `check_invariants` and the day's checks; the banks' deposit
/// liabilities to households are the ledger's households' deposits (the
/// counterpart the ledger keeps, I-1), against the weighted column (I-4).
/// "Its net deposits" are those the month report gives for each household
/// that left whole (after its cash was deposited and its loans repaid or
/// written off): the legs of `transfer.migrant_capital` from households to
/// the rest of the world on the close's day are exactly their weighted sum.
/// Release builds only.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a year at 1:1000 and at 1:100: cargo test --release -p econ-core --features migration --test acceptance migration::"
)]
fn ac_mig_09() {
    assert_money_with_migration(1000);
    assert_money_with_migration(100);
}

/// A comparison of 24 months after a change of `world.wages_abroad` with the
/// baseline: (emigrants, returns, remittances in euro cents) of each run and
/// the diaspora at the end.
struct Compare {
    changed: (u64, u64, i64, u64),
    base: (u64, u64, i64, u64),
}

fn compare_wages_abroad(bp: i32) -> Compare {
    let mut base = world(1000, Vec::new());
    wages_abroad_at_start(&base);
    let (mut w, from) = with_order(LeverId::WorldWagesAbroad, LeverValue::Rate(bp));
    let to = from + 24;
    run(&mut w, to);
    run(&mut base, to);
    let totals = |w: &World| {
        let s = span(w, from, to);
        (
            sum(s, |m| m.emigrants),
            sum(s, |m| m.returns),
            s.iter().map(|m| m.remittances_eur_cents).sum::<i64>(),
            diaspora_total(w),
        )
    };
    Compare {
        changed: totals(&w),
        base: totals(&base),
    }
}

/// **AC-MIG-10** `[sim]` Raising `world.wages_abroad` by 20% at the start:
/// over the next 24 months emigrants are at least 10% above a baseline run,
/// returns are lower, the diaspora is larger, and remittances in euro are
/// higher.
///
/// Readings: at 1:1000, the lever set to 12,000 on day 0. Emigrants,
/// returns and remittances in euro are summed over the 24 closes from the
/// month it takes effect; the diaspora is compared at the last of them.
#[test]
fn ac_mig_10() {
    let c = compare_wages_abroad(12_000);
    let (e, r, rem, d) = c.changed;
    let (e0, r0, rem0, d0) = c.base;
    assert!(
        e * 10 >= e0 * 11,
        "emigrants {e} against {e0} in the baseline: not 10% above"
    );
    assert!(r < r0, "returns {r} against {r0}");
    assert!(d > d0, "the diaspora {d} against {d0}");
    assert!(rem > rem0, "remittances {rem} euro cents against {rem0}");
}

/// **AC-MIG-11** `[sim]` Lowering `world.wages_abroad` by 20%: over the next
/// 24 months returns are higher and emigrants lower than in the baseline.
///
/// Readings: at 1:1000, the lever set to 8,000 on day 0; sums over the 24
/// closes from the month it takes effect.
#[test]
fn ac_mig_11() {
    let c = compare_wages_abroad(8_000);
    let (e, r, _, _) = c.changed;
    let (e0, r0, _, _) = c.base;
    assert!(r > r0, "returns {r} against {r0}");
    assert!(e < e0, "emigrants {e} against {e0}");
}

/// Emigrants aged 15 to 64 (bands 1 to 5) over months.
fn working_age_emigrants(months: &[MigrationMonth]) -> u64 {
    months
        .iter()
        .map(|m| m.emigrants_by_age[1..=5].iter().sum::<u64>())
        .sum()
}

/// **AC-MIG-12** `[sim]` A cut of the employee's pension contribution by 5
/// points (higher net wages): over the next 24 months, emigrants of working
/// age are fewer than in the baseline.
///
/// Readings: at 1:1000, `tax.contrib.cas` (the employee's pension
/// contribution, CAS) set on day 0 to its rate in force less 500 basis
/// points. Working age is 15 to 64 (the bands of the emigrants by age);
/// emigrants are summed over the 24 closes from the month the cut takes
/// effect.
#[test]
fn ac_mig_12() {
    let probe = world(1000, Vec::new());
    let cas = rate_in_force(&probe, LeverId::TaxContribCas, None);
    let (mut w, from) = with_order(LeverId::TaxContribCas, LeverValue::Rate(cas - 500));
    let mut base = probe;
    let to = from + 24;
    run(&mut w, to);
    run(&mut base, to);
    let cut = working_age_emigrants(span(&w, from, to));
    let b = working_age_emigrants(span(&base, from, to));
    assert!(
        cut < b,
        "emigrants of working age {cut} against {b} in the baseline"
    );
}

/// A flow code's amount in a fiscal month's receipts or payments.
fn fiscal_line(lines: &[(econ_ledger::FlowCode, Bani)], name: &str) -> i128 {
    let c = code(name);
    lines
        .iter()
        .filter(|(x, _)| *x == c)
        .map(|(_, v)| i128::from(v.get()))
        .sum()
}

/// **AC-MIG-13** `[sim]` Emigration takes jobs and contributors: after 5
/// years with `world.wages_abroad` at 130%, the labour force, the employed
/// and the pension contributions (`contrib.pension`) are lower than in the
/// baseline, and pension spending is not.
///
/// Readings: at 1:1000, the lever set to 13,000 on day 0, both runs through
/// 60 closes. The labour force (employed plus unemployed) and the employed
/// are those of the 60th close; the pension contributions are the
/// government's receipts under `contrib.pension` over the fifth year (the
/// last 12 closes), and pension spending its payments under
/// `benefit.pension` over the same months, not lower than the baseline's.
/// Release builds only.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "two five-year runs: cargo test --release -p econ-core --features migration --test acceptance migration::"
)]
fn ac_mig_13() {
    let mut base = world(1000, Vec::new());
    wages_abroad_at_start(&base);
    let (mut w, _) = with_order(LeverId::WorldWagesAbroad, LeverValue::Rate(13_000));
    run(&mut w, 60);
    run(&mut base, 60);
    let last = |w: &World| {
        let m = &w.months()[59];
        (m.employed + m.unemployed, m.employed)
    };
    let fifth_year = |w: &World, contributions: bool| -> i128 {
        w.fiscal_months()[48..60]
            .iter()
            .map(|f| {
                if contributions {
                    fiscal_line(&f.revenue, "contrib.pension")
                } else {
                    fiscal_line(&f.spending, "benefit.pension")
                }
            })
            .sum()
    };
    let (lf, emp) = last(&w);
    let (lf0, emp0) = last(&base);
    assert!(lf < lf0, "the labour force {lf} against {lf0}");
    assert!(emp < emp0, "the employed {emp} against {emp0}");
    let (c, c0) = (fifth_year(&w, true), fifth_year(&base, true));
    assert!(c0 > 0);
    assert!(
        c < c0,
        "pension contributions of the fifth year {c} against {c0}"
    );
    let (p, p0) = (fifth_year(&w, false), fifth_year(&base, false));
    assert!(p0 > 0);
    assert!(
        p >= p0,
        "pension spending of the fifth year {p} against {p0}: lower"
    );
}

/// **AC-MIG-14** `[sim]` Remittances support the leu: in the counterfactual
/// without the diaspora tables, the exchange rate after 24 months is higher
/// (the leu weaker) than with them, and the current account lower: in the
/// first month (remittances arrive on its last day, before they are spent)
/// by at least 95% of the month's remittances, and in every later month by
/// more than zero and at most the month's remittances (spending on imports
/// and the weaker leu narrow the gap; owner, 2026-10-29).
///
/// Restated with the spec on 2026-10-29 (owner's yes, recorded in the
/// spec): the 95% floor held in every month before; it now holds in the
/// first month, and later months are bounded on both sides.
///
/// Readings: at 1:1000, the counterfactual is `StartEdit::NoDiaspora`. The
/// exchange rate is `Foreign::exchange_rate_ppm` after the 24th close. The
/// gap of a month is the current account with the diaspora less that of the
/// counterfactual, at each of the 24 closes; "the month's remittances" are
/// those of the run with the diaspora.
#[test]
fn ac_mig_14() {
    let mut with = world(1000, Vec::new());
    let mut without = world(1000, vec![StartEdit::NoDiaspora]);
    run(&mut with, 24);
    run(&mut without, 24);
    assert!(
        without.foreign().exchange_rate_ppm > with.foreign().exchange_rate_ppm,
        "the leu after 24 months: {} lei per euro (millionths) without the diaspora, {} with it",
        without.foreign().exchange_rate_ppm,
        with.foreign().exchange_rate_ppm
    );
    let remittances = span(&with, 0, 24);
    for (t, m) in remittances.iter().enumerate() {
        let gap = i128::from(with.trade_months()[t].current_account.get())
            - i128::from(without.trade_months()[t].current_account.get());
        let r = i128::from(m.remittances.get());
        assert!(r > 0, "month {t}: remittances are paid");
        if t == 0 {
            assert!(
                gap * 100 >= r * 95,
                "month 0: the current account is {gap} bani lower without the diaspora, less than 95% of {r}"
            );
        } else {
            assert!(
                gap > 0,
                "month {t}: the current account is not lower without the diaspora (gap {gap} bani)"
            );
            assert!(
                gap <= r,
                "month {t}: the current account is {gap} bani lower without the diaspora, more than the month's remittances {r}"
            );
        }
    }
}

/// Moves into and out of a county over months.
fn county_moves(months: &[MigrationMonth], c: usize) -> (u64, u64) {
    (
        sum(months, |m| m.moves_in[c]),
        sum(months, |m| m.moves_out[c]),
    )
}

/// **AC-MIG-15** `[sim]` Moves between counties: in the first year Ilfov,
/// Cluj and Timiș gain residents and Teleorman, Vaslui and Bacău lose them;
/// a county whose region's unemployment rises 5 points (a scripted shock)
/// gains fewer movers and sends more over the next 24 months.
///
/// Readings: at 1:100 (at 1:1000 a year has a few dozen moving records), so
/// release builds only. "Gain residents" is a positive net balance of moves
/// between counties over the first 12 closes (moves in less moves out),
/// "lose" a negative one. The shock is `StartEdit::MigrationUnemploymentShock`
/// for Cluj's development region (Nord-Vest), 50,000 millionths (5 points)
/// for the whole run; Cluj's moves in over the first 24 closes are fewer, and
/// its moves out more, than in the baseline. Step 6's origin push factor
/// (`move_unemployment`, added to the spec 2026-10-29) is what makes a
/// county send more movers when its region's unemployment rises; the shock
/// is read by it as by the destination choice.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "two two-year runs at 1:100: cargo test --release -p econ-core --features migration --test acceptance migration::"
)]
fn ac_mig_15() {
    let mut base = world(100, Vec::new());
    run(&mut base, 24);
    let year = span(&base, 0, 12);
    for (code, name, gains) in [
        ("RO322", "Ilfov", true),
        ("RO113", "Cluj", true),
        ("RO424", "Timiș", true),
        ("RO317", "Teleorman", false),
        ("RO216", "Vaslui", false),
        ("RO211", "Bacău", false),
    ] {
        let (i, o) = county_moves(year, county(code));
        let net = i128::from(i) - i128::from(o);
        if gains {
            assert!(
                net > 0,
                "{name}: net moves of the first year {net} ({i} in, {o} out)"
            );
        } else {
            assert!(
                net < 0,
                "{name}: net moves of the first year {net} ({i} in, {o} out)"
            );
        }
    }
    let cluj = county("RO113");
    let region = region_of_county(100, cluj);
    let mut shocked = world(
        100,
        vec![StartEdit::MigrationUnemploymentShock {
            region,
            points_ppm: 50_000,
        }],
    );
    run(&mut shocked, 24);
    let (i0, o0) = county_moves(span(&base, 0, 24), cluj);
    let (i, o) = county_moves(span(&shocked, 0, 24), cluj);
    assert!(
        i < i0,
        "Cluj gains fewer movers with the shock: {i} against {i0}"
    );
    assert!(
        o > o0,
        "Cluj sends more movers with the shock: {o} against {o0}"
    );
}

/// The ledger's balances, every sector and instrument.
fn balances(w: &World) -> Vec<Bani> {
    let l = w.ledger();
    Sector::ALL
        .iter()
        .flat_map(|&s| Instrument::ALL.iter().map(move |&i| (s, i)))
        .map(|(s, i)| l.balance(s, i))
        .collect()
}

/// The first year's emigrants over the residents at the start, millionths.
fn first_year_emigration_rate(scale: u32) -> i128 {
    let mut w = world(scale, Vec::new());
    let start = residents(&w);
    assert!(start > 0);
    run(&mut w, 12);
    let e = sum(span(&w, 0, 12), |m| m.emigrants);
    i128::from(e) * 1_000_000 / i128::from(start)
}

/// **AC-MIG-16** `[sim]` Determinism and scale: two runs with the same seed
/// give identical persons, weights and ledger; at 1:1000 and 1:100 the first
/// year's emigration rate per resident differs by less than 10% of its
/// value.
///
/// Readings: two runs of a year at 1:1000 with seed 42, compared every day
/// (state hash) and at the end: the persons table, the households table
/// (weights included), `abroad`, the diaspora, the migration reports and
/// every ledger balance. The rate is the first 12 closes' emigrants over the
/// residents at the start; "of its value" is of the 1:100 rate. Release
/// builds only.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "two years at 1:1000 and one at 1:100: cargo test --release -p econ-core --features migration --test acceptance migration::"
)]
fn ac_mig_16() {
    let mut a = world(1000, Vec::new());
    let mut b = world(1000, Vec::new());
    while a.months().len() < 12 {
        let ra = step_checked(&mut a);
        let rb = step_checked(&mut b);
        assert_eq!(
            ra.state_hash, rb.state_hash,
            "{:?}: the state hashes differ",
            ra.date
        );
    }
    assert_eq!(a.persons(), b.persons(), "the persons");
    assert_eq!(
        a.households(),
        b.households(),
        "the households and their weights"
    );
    assert_eq!(a.abroad(), b.abroad(), "abroad");
    assert_eq!(a.diaspora(), b.diaspora(), "the diaspora");
    assert_eq!(
        a.migration_months(),
        b.migration_months(),
        "the migration reports"
    );
    assert_eq!(balances(&a), balances(&b), "the ledger");

    let coarse = first_year_emigration_rate(1000);
    let fine = first_year_emigration_rate(100);
    assert!(fine > 0);
    assert!(
        (coarse - fine).abs() * 10 < fine,
        "the first year's emigration rate: {coarse} millionths at 1:1000, {fine} at 1:100"
    );
}

/// **AC-MIG-17** `[sim]` 50-year quiet run: the diaspora stays between 2 and
/// 7 million, the resident population shrinks with net emigration (the
/// direction of the official projection, M5's exit), and no table outgrows
/// its starting size by more than half (free slots reused).
///
/// Readings: at 1:1000, 600 closes with no lever changed. At every close the
/// diaspora (linked plus stock) is between 2,000,000 and 7,000,000
/// inclusive, and the households and persons tables have at most 1.5 times
/// their rows at the start. At the end the residents are fewer than at the
/// start, and net migration over the run (returns and foreign immigrants
/// less emigrants) is negative. Release builds only.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "fifty years: cargo test --release -p econ-core --features migration --test acceptance migration::"
)]
fn ac_mig_17() {
    let mut w = world(1000, Vec::new());
    let residents0 = residents(&w);
    let households0 = w.households().hh_weight.len();
    let persons0 = w.persons().household_id.len();
    while w.months().len() < 600 {
        let closes = w.months().len();
        let r = step_checked(&mut w);
        if w.months().len() == closes {
            continue;
        }
        let d = diaspora_total(&w);
        assert!(
            (2_000_000..=7_000_000).contains(&d),
            "{:?}: the diaspora is {d}",
            r.date
        );
        let (h, p) = (
            w.households().hh_weight.len(),
            w.persons().household_id.len(),
        );
        assert!(
            h * 2 <= households0 * 3,
            "{:?}: {h} household rows against {households0} at the start",
            r.date
        );
        assert!(
            p * 2 <= persons0 * 3,
            "{:?}: {p} person rows against {persons0} at the start",
            r.date
        );
    }
    let all = span(&w, 0, 600);
    let net: i128 = all
        .iter()
        .map(|m| i128::from(m.returns) + i128::from(m.immigrants) - i128::from(m.emigrants))
        .sum();
    assert!(net < 0, "net migration over fifty years is {net}");
    let end = residents(&w);
    assert!(
        end < residents0,
        "residents {end} after fifty years against {residents0}"
    );
}
