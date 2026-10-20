//! Acceptance tests for AC-MP-03 to AC-MP-05 (spec: economy/monetary-policy),
//! the M3 `[sim]` criteria, on the `World` under `Law::law_2021()`. Written
//! in a test-authoring session (ADR-0014) against the test writer's
//! additions behind the feature `monetary`: the central bank's read-only
//! state (`World::central_bank`), its parameters (`WorldParams::monetary`)
//! and a start edit (`StartEdit::CentralBankFinancing`, the deficit financed
//! by the central bank for the first months). AC-MP-01 and 02 are in
//! `econ-mech-monetary`. The flow codes are found by the names the spec
//! gives them.

use econ_core::world::StartEdit;
use econ_ledger::{Instrument, Sector};
use econ_mech_banking::month_interest;
use econ_types::Bani;

use super::support::{month_stats, params, step};
use super::support_m3::{code, flows_of, legs_of_day, legs_sum, world_2021, world_2021_with};

/// **AC-MP-03** `[sim]` Every month end, the central bank's profit is
/// remitted to the government exactly, under its flow code; the central
/// bank's net financial assets change only by a loss.
#[test]
fn ac_mp_03() {
    let mut w = world_2021(1000);
    let remit = code("transfer.cb_profit");
    let mut remitted = Bani::ZERO;
    for _ in 0..183 {
        let nfa = w.ledger().net_financial_assets(Sector::CentralBank);
        let r = step(&mut w);
        let d = r.date;
        // The day's income of the central bank: every flow but the remittance.
        let flows = flows_of(&w, Sector::CentralBank);
        let profit = flows
            .iter()
            .filter(|&(&c, _)| c != remit)
            .map(|(_, &v)| v)
            .fold(Bani::ZERO, |a, b| a + b);
        let paid = flows.get(&remit).copied().unwrap_or(Bani::ZERO);
        let gov = w.ledger().flows().get(remit, Sector::Government);
        if d.is_month_end() {
            let expected = if profit > Bani::ZERO {
                profit
            } else {
                Bani::ZERO
            };
            assert_eq!(paid, Bani::ZERO - expected, "{d:?}: the profit remitted");
            assert_eq!(gov, expected, "{d:?}: the government receives it");
            remitted += expected;
        } else {
            assert_eq!(paid, Bani::ZERO, "{d:?}: a remittance off month end");
            assert_eq!(
                profit,
                Bani::ZERO,
                "{d:?}: central bank income off month end"
            );
        }
        let loss = if profit < Bani::ZERO {
            profit
        } else {
            Bani::ZERO
        };
        assert_eq!(
            w.ledger().net_financial_assets(Sector::CentralBank) - nfa,
            loss,
            "{d:?}: the central bank's net financial assets"
        );
    }
    assert!(remitted > Bani::ZERO, "no profit in six months");
}

/// The month end's interest on reserves: (paid by the central bank, charged
/// on advances), from the day's postings.
fn reserves_interest(w: &econ_core::world::World, r: &econ_core::world::DayReport) -> (Bani, Bani) {
    let c = code("interest.reserves");
    let legs = legs_of_day(w, r);
    (
        legs_sum(
            &legs,
            c,
            Instrument::Reserves,
            Sector::CentralBank,
            Sector::Banks,
        ),
        legs_sum(
            &legs,
            c,
            Instrument::Reserves,
            Sector::Banks,
            Sector::CentralBank,
        ),
    )
}

/// **AC-MP-04** `[sim]` Reserves above the requirement earn the deposit
/// facility rate, advances cost the lending facility rate, both to the ban.
#[test]
fn ac_mp_04() {
    let p = params();
    let corridor = i32::try_from(p.monetary.corridor_ppm).expect("small");
    let mut high = p;
    // 12% of deposits: more than the banks hold, so they borrow (AC-BANK-07).
    high.banking.reserve_requirement_ppm = 120_000;
    let interest = code("interest.reserves");
    for (what, mut w) in [
        ("8%", world_2021(1000)),
        ("12%", world_2021_with(1000, Vec::new(), high)),
    ] {
        let (mut earned, mut charged) = (Bani::ZERO, Bani::ZERO);
        for _ in 0..92 {
            let policy = w.central_bank().policy_rate_ppm;
            let r = step(&mut w);
            let d = r.date;
            let (to_banks, to_cb) = reserves_interest(&w, &r);
            if !d.is_month_end() {
                assert_eq!(
                    to_banks + to_cb,
                    Bani::ZERO,
                    "{what}, {d:?}: interest off month end"
                );
                continue;
            }
            let cb = w.central_bank();
            let (excess, advances) = (cb.excess_reserves_basis, cb.advances_basis);
            assert!(
                excess >= Bani::ZERO && advances >= Bani::ZERO,
                "{what}, {d:?}"
            );
            assert!(
                advances == Bani::ZERO || excess == Bani::ZERO,
                "{what}, {d:?}: excess reserves {excess:?} with advances {advances:?}"
            );
            let deposit_facility = u32::try_from((policy - corridor).max(0)).expect("a rate");
            let lending_facility = u32::try_from(policy + corridor).expect("a rate");
            assert_eq!(
                to_banks,
                month_interest(excess, deposit_facility),
                "{what}, {d:?}: the deposit facility on {excess:?}"
            );
            assert_eq!(
                to_cb,
                month_interest(advances, lending_facility),
                "{what}, {d:?}: the lending facility on {advances:?}"
            );
            assert_eq!(
                w.ledger().flows().get(interest, Sector::Banks),
                to_banks - to_cb,
                "{what}, {d:?}: the banks' net"
            );
            earned += to_banks;
            charged += to_cb;
        }
        match what {
            "8%" => assert!(earned > Bani::ZERO, "excess reserves earn interest"),
            _ => assert!(charged > Bani::ZERO, "advances cost interest"),
        }
    }
}

/// **AC-MP-05** `[sim]` Repeated monetary financing (a deficit financed by
/// the central bank for 24 months) lowers credibility and raises expected
/// inflation.
#[test]
fn ac_mp_05() {
    let mut base = world_2021(1000);
    let mut financed = world_2021_with(
        1000,
        vec![StartEdit::CentralBankFinancing { months: 24 }],
        params(),
    );
    let mut monfin = Vec::new();
    for _ in 0..24 {
        for w in [&mut base, &mut financed] {
            let closed = w.months().len();
            while w.months().len() == closed {
                step(w);
            }
        }
        assert_eq!(
            base.central_bank().monetary_financing,
            Bani::ZERO,
            "the deficit is financed by bonds in the base"
        );
        monfin.push(financed.central_bank().monetary_financing);
    }
    eprintln!("AC-MP-05: monetary financing by month {monfin:?}");
    assert!(
        monfin[0] > Bani::ZERO,
        "the central bank financed the first month's deficit"
    );
    assert!(monfin[23] > monfin[11], "it kept financing it: {monfin:?}");
    let (cb, cf) = (
        base.central_bank().credibility_ppm,
        financed.central_bank().credibility_ppm,
    );
    assert!(
        cf < cb,
        "credibility {cf} against {cb} without monetary financing"
    );
    let (eb, ef) = (
        month_stats(&base, 23).expected_inflation_ppm,
        month_stats(&financed, 23).expected_inflation_ppm,
    );
    assert!(ef > eb, "expected inflation {ef} against {eb}");
}
