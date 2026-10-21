//! Acceptance tests for AC-BANK-01 to AC-BANK-08 (spec: economy/money-banking),
//! the M3 `[sim]` criteria, on the `World` under `Law::law_2021()`. Written in
//! a test-authoring session (ADR-0014) against the test writer's additions
//! behind the feature `banking`: the banks' read-only state
//! (`World::bank`), their parameters (`WorldParams::banking`,
//! `WorldParams::policy_rate_ppm`) and two start edits (`StartEdit::PolicyRate`,
//! a change of the policy rate from a given day, and `StartEdit::ExtraLoan`,
//! a loan drawn on day 0). The flow codes are found by the names the spec
//! gives them.

use econ_core::world::{StartEdit, World};
use econ_ledger::{Instrument, Sector};
use econ_mech_banking::month_interest;
use econ_types::{Bani, Day};

use super::support::{assert_day_checks, params, start_state, step, total, weighted};
use super::support_m3::{code, flow, legs_of_day, legs_sum, world_2021, world_2021_with};

/// One point, in millionths.
const POINT: u32 = 10_000;

fn sum(v: &[Bani]) -> Bani {
    total(v)
}

/// **AC-BANK-01** `[sim]` A credit draw raises the unit's loans and deposits
/// by the same amount, and the banks' loans and deposit liabilities by the
/// same amount (money creation); a repayment does the reverse.
#[test]
#[allow(clippy::needless_range_loop)]
fn ac_bank_01() {
    let mut w = world_2021(1000);
    let loan = code("finance.loan");
    let (mut drawn_total, mut repaid_total) = (Bani::ZERO, Bani::ZERO);
    for _ in 0..92 {
        let before = w.bank().unit_loans.clone();
        let r = step(&mut w);
        let b = w.bank();
        let d = r.date;
        // Each unit: its loans move by its draws less its repayments and the
        // banks' loss on a write-off, nothing else.
        for u in 0..before.len() {
            assert_eq!(
                b.unit_loans[u],
                before[u] + b.drawn_today[u] - b.repaid_today[u] - b.bank_loss_today[u],
                "{d:?}, unit {u}: loans"
            );
            assert!(b.drawn_today[u] >= Bani::ZERO && b.repaid_today[u] >= Bani::ZERO);
        }
        let (drawn, repaid) = (sum(&b.drawn_today), sum(&b.repaid_today));
        drawn_total += drawn;
        repaid_total += repaid;
        // The postings: a draw is a loan from the banks to firms and a deposit
        // of the same amount from the banks to firms; a repayment the reverse.
        let legs = legs_of_day(&w, &r);
        let net_loans = legs_sum(&legs, loan, Instrument::Loans, Sector::Firms, Sector::Banks)
            - legs_sum(&legs, loan, Instrument::Loans, Sector::Banks, Sector::Firms);
        let net_deposits = legs_sum(
            &legs,
            loan,
            Instrument::Deposits,
            Sector::Banks,
            Sector::Firms,
        ) - legs_sum(
            &legs,
            loan,
            Instrument::Deposits,
            Sector::Firms,
            Sector::Banks,
        );
        assert_eq!(net_loans, drawn - repaid, "{d:?}: the banks' new loans");
        assert_eq!(
            net_deposits,
            drawn - repaid,
            "{d:?}: the deposits they create"
        );
        assert_eq!(
            flow(&w, loan, Sector::Banks),
            Bani::ZERO,
            "{d:?}: a loan changes no one's net worth"
        );
        assert_eq!(flow(&w, loan, Sector::Firms), Bani::ZERO);
        // Other sectors do not draw on credit lines in M3.
        for s in [Sector::Households, Sector::Government, Sector::RestOfWorld] {
            assert!(
                legs.iter()
                    .all(|l| l.code != loan || (l.payer != s && l.payee != s)),
                "{d:?}: {s:?} in a credit-line posting"
            );
        }
    }
    assert!(
        drawn_total > Bani::ZERO,
        "no unit drew on its credit line in three months"
    );
    assert!(repaid_total > Bani::ZERO, "no unit repaid in three months");
}

/// **AC-BANK-02** `[sim]` Every month end, interest paid by borrowers less
/// interest paid to depositors equals the banks' interest income in the
/// ledger, under its flow codes, to the ban.
#[test]
fn ac_bank_02() {
    let mut w = world_2021(1000);
    let (on_loans, on_deposits) = (code("interest.loan"), code("interest.deposit"));
    let mut month_ends = 0;
    while month_ends < 3 {
        // The rates of the month: set at the last close, before today's.
        let rates = w.bank().rates;
        let r = step(&mut w);
        let d = r.date;
        if !d.is_month_end() {
            for s in [Sector::Households, Sector::Firms, Sector::Banks] {
                assert_eq!(
                    flow(&w, on_loans, s),
                    Bani::ZERO,
                    "{d:?}: interest off month end"
                );
                assert_eq!(flow(&w, on_deposits, s), Bani::ZERO);
            }
            continue;
        }
        month_ends += 1;
        let b = w.bank();
        let h = w.households();
        // Households' loans are constant in M3: a month at each loan's rate.
        for k in 0..h.hh_weight.len() {
            let due = month_interest(b.hh_mortgage[k], rates.mortgage_ppm)
                + month_interest(b.hh_consumer_debt[k], rates.consumer_ppm);
            assert!(
                (b.hh_interest_paid[k] - due).abs() <= Bani(1),
                "{d:?}, household {k}: interest {:?} on its loans, due {due:?}",
                b.hh_interest_paid[k]
            );
        }
        // Per record, weighted, equals the ledger by sector.
        let hh_paid = weighted(&h.hh_weight, &b.hh_interest_paid);
        let hh_got = weighted(&h.hh_weight, &b.hh_interest_received);
        let units_paid = sum(&b.unit_interest_paid);
        let units_got = sum(&b.unit_interest_received);
        assert_eq!(
            flow(&w, on_loans, Sector::Households),
            Bani::ZERO - hh_paid,
            "{d:?}"
        );
        assert_eq!(
            flow(&w, on_loans, Sector::Firms),
            Bani::ZERO - units_paid,
            "{d:?}"
        );
        assert_eq!(flow(&w, on_deposits, Sector::Households), hh_got, "{d:?}");
        assert_eq!(flow(&w, on_deposits, Sector::Firms), units_got, "{d:?}");
        assert!(
            units_paid > Bani::ZERO && hh_got > Bani::ZERO,
            "{d:?}: interest moved"
        );
        // Borrowers' interest less depositors' = the banks' interest income.
        let others: Vec<Sector> = Sector::ALL
            .into_iter()
            .filter(|&s| s != Sector::Banks)
            .collect();
        let by_borrowers = others
            .iter()
            .map(|&s| Bani::ZERO - flow(&w, on_loans, s))
            .fold(Bani::ZERO, |a, b| a + b);
        let to_depositors = others
            .iter()
            .map(|&s| flow(&w, on_deposits, s))
            .fold(Bani::ZERO, |a, b| a + b);
        let income = flow(&w, on_loans, Sector::Banks) + flow(&w, on_deposits, Sector::Banks);
        assert_eq!(
            by_borrowers - to_depositors,
            income,
            "{d:?}: the banks' interest income"
        );
        assert!(by_borrowers >= hh_paid + units_paid, "{d:?}");
        for &s in &others {
            assert!(
                flow(&w, on_loans, s) <= Bani::ZERO,
                "{d:?}: {s:?} is paid loan interest"
            );
            assert!(
                flow(&w, on_deposits, s) >= Bani::ZERO,
                "{d:?}: {s:?} pays deposit interest"
            );
        }
    }
}

/// **AC-BANK-03** `[sim]` For every firm unit and day, its loans in its column
/// equal its share of the ledger's firm loans: the units' sum equals the
/// banks' claim on firms (I-4).
#[test]
fn ac_bank_03() {
    let mut w = world_2021(1000);
    let l = |w: &World, s, i| w.ledger().balance(s, i);
    let at_load = w.bank().clone();
    assert_eq!(sum(&at_load.unit_loans), at_load.firm_loans, "at load");
    assert!(
        at_load.firm_loans > Bani::ZERO,
        "the units start with bank loans"
    );
    assert!(
        at_load.firm_loans <= l(&w, Sector::Banks, Instrument::Loans),
        "the banks' claim on firms is part of their loans"
    );
    assert_eq!(at_load.unit_loans.len(), w.firm_units().industry.len());
    for _ in 0..92 {
        let (firm_loans, banks, firms) = (
            w.bank().firm_loans,
            l(&w, Sector::Banks, Instrument::Loans),
            l(&w, Sector::Firms, Instrument::Loans),
        );
        let r = step(&mut w);
        let b = w.bank();
        let d = r.date;
        assert_eq!(sum(&b.unit_loans), b.firm_loans, "{d:?}: the units' sum");
        assert!(
            b.unit_loans.iter().all(|&v| v >= Bani::ZERO),
            "{d:?}: negative loans"
        );
        // Households' loans are constant in M3, the other lenders' too: every
        // change of the ledger's loans is a change of the units' bank loans.
        let moved = b.firm_loans - firm_loans;
        assert_eq!(
            l(&w, Sector::Banks, Instrument::Loans) - banks,
            moved,
            "{d:?}: banks"
        );
        assert_eq!(
            l(&w, Sector::Firms, Instrument::Loans) - firms,
            Bani::ZERO - moved,
            "{d:?}: firms"
        );
        assert_eq!(b.hh_mortgage, at_load.hh_mortgage, "{d:?}: mortgages");
        assert_eq!(
            b.hh_consumer_debt, at_load.hh_consumer_debt,
            "{d:?}: consumer debt"
        );
    }
}

/// **AC-BANK-04** `[sim]` No firm unit's deposits are negative at the end of
/// any day, with no credit inside the firm sector.
///
/// Owed to the M1 item "refine the 64 into the 90" (the data-centre unit's
/// wage bill), by the owner's decision of 2026-10-21.
#[test]
#[ignore = "owed to the M1 item: refine the 64 into the 90 (the data-centre unit's wage bill)"]
fn ac_bank_04() {
    let mut w = world_2021(1000);
    for _ in 0..183 {
        let r = step(&mut w);
        assert_day_checks(&w, &r);
        if let Some(u) = w.firm_units().deposits.iter().position(|&v| v < Bani::ZERO) {
            panic!("{:?}: unit {u}'s deposits are negative", r.date);
        }
        assert_eq!(
            w.bank().firm_sector_credit,
            Bani::ZERO,
            "{:?}: credit inside the firm sector",
            r.date
        );
    }
}

/// **AC-BANK-05** `[sim]` A one-point rise of the policy rate raises new and
/// floating loan rates by one point at the next monthly close.
#[test]
fn ac_bank_05() {
    let p = params();
    let start = p.policy_rate_ppm;
    let mut base = world_2021(1000);
    let mut raised = world_2021_with(
        1000,
        vec![StartEdit::PolicyRate {
            from: Day(10),
            rate_ppm: start + 10_000,
        }],
        p,
    );
    // Before the first close (day 30, 31 December) the rates have not moved.
    for n in 0..30u32 {
        step(&mut base);
        step(&mut raised);
        assert_eq!(
            raised.bank().policy_rate_ppm,
            if n >= 10 { start + 10_000 } else { start },
            "day {n}: the policy rate"
        );
        assert_eq!(
            raised.bank().rates,
            base.bank().rates,
            "day {n}: rates before the close"
        );
    }
    let r = step(&mut base);
    step(&mut raised);
    assert!(r.date.is_month_end(), "day 30 closes December");
    let (b, x) = (base.bank().rates, raised.bank().rates);
    assert_eq!(x.firms_ppm, b.firms_ppm + POINT, "firm loans");
    assert_eq!(x.mortgage_ppm, b.mortgage_ppm + POINT, "mortgages");
    assert_eq!(x.consumer_ppm, b.consumer_ppm + POINT, "consumer loans");
}

/// **AC-BANK-06** `[sim]` A write-off lowers bank equity by (1 − recovery) of
/// it; banks below the minimum capital ratio open no new credit line.
#[test]
fn ac_bank_06() {
    let p = params();
    let recovery = i128::from(p.banking.recovery_ppm);
    let write_off = code("finance.write_off");
    // A cohort unit that starts owing three years of its output: far above
    // its credit limit of twelve months of revenue.
    let start = start_state(1000);
    let unit = (0..start.unit_cohort.len())
        .find(|&u| start.unit_cohort[u] && start.unit_output[u] > Bani::ZERO)
        .expect("a cohort unit");
    let amount = start.unit_output[unit].times(3);
    let mut w = world_2021_with(1000, vec![StartEdit::ExtraLoan { unit, amount }], p);
    let mut written_any = false;
    while w.months().is_empty() {
        let r = step(&mut w);
        let b = w.bank();
        let (written, loss) = (sum(&b.written_off_today), sum(&b.bank_loss_today));
        for u in 0..b.written_off_today.len() {
            let (x, l) = (b.written_off_today[u], b.bank_loss_today[u]);
            let due = i128::from(x.get()) * (1_000_000 - recovery);
            assert!(
                (i128::from(l.get()) * 1_000_000 - due).abs() <= 1_000_000,
                "{:?}, unit {u}: a loss of {l:?} on a write-off of {x:?}",
                r.date
            );
        }
        // The banks' net worth falls by the loss, under its flow code.
        assert_eq!(
            flow(&w, write_off, Sector::Banks),
            Bani::ZERO - loss,
            "{:?}",
            r.date
        );
        assert_eq!(flow(&w, write_off, Sector::Firms), loss, "{:?}", r.date);
        if written > Bani::ZERO {
            assert!(r.date.is_month_end(), "write-offs at the monthly close");
            written_any = true;
        }
    }
    assert!(written_any, "nothing was written off");
    assert!(
        w.bank().written_off_today[unit] > Bani::ZERO,
        "unit {unit}'s excess debt was written off at the first close"
    );
    // Banks below the minimum: no new credit line, while the same economy
    // draws with banks above it.
    let mut strict = params();
    strict.banking.car_min_ppm = 1_000_000;
    let mut base = world_2021(1000);
    let mut below = world_2021_with(1000, Vec::new(), strict);
    let mut drawn_base = Bani::ZERO;
    for _ in 0..61 {
        step(&mut base);
        let r = step(&mut below);
        drawn_base += sum(&base.bank().drawn_today);
        assert!(
            below.bank().car_ppm < 1_000_000,
            "{:?}: banks below the minimum",
            r.date
        );
        assert_eq!(
            sum(&below.bank().drawn_today),
            Bani::ZERO,
            "{:?}: a draw",
            r.date
        );
    }
    assert!(
        drawn_base > Bani::ZERO,
        "the same economy draws on credit when banks may lend"
    );
}

/// Reserves at least the requirement (to the ban); advances only when
/// reserves are at the requirement.
fn assert_reserves(w: &World, requirement_ppm: u32, what: &str) -> Bani {
    let l = w.ledger();
    let reserves = i128::from(l.balance(Sector::Banks, Instrument::Reserves).get());
    let deposits = -i128::from(l.balance(Sector::Banks, Instrument::Deposits).get());
    let advances = -l.balance(Sector::Banks, Instrument::CbAdvances).get();
    let need = deposits * i128::from(requirement_ppm);
    assert!(
        reserves * 1_000_000 >= need - 1_000_000,
        "{what}: reserves {reserves} below {requirement_ppm} ppm of deposits {deposits}"
    );
    assert!(advances >= 0, "{what}: advances {advances}");
    if advances > 0 {
        assert!(
            reserves * 1_000_000 <= need + 1_000_000,
            "{what}: advances of {advances} outstanding with reserves {reserves} above the requirement"
        );
    }
    Bani(advances)
}

/// **AC-BANK-07** `[sim]` Banks' reserves are never below the requirement at
/// the end of a day; when they would be, central bank advances fill the gap
/// and are repaid when reserves allow.
#[test]
fn ac_bank_07() {
    let p = params();
    let mut base = world_2021(1000);
    // 12% of deposits is more than the banks' reserves at the start: they
    // borrow from the central bank on day 0.
    let mut high = p;
    high.banking.reserve_requirement_ppm = 120_000;
    let mut tight = world_2021_with(1000, Vec::new(), high);
    let mut advanced = false;
    for n in 0..92u32 {
        step(&mut base);
        step(&mut tight);
        let _ = assert_reserves(
            &base,
            p.banking.reserve_requirement_ppm,
            &format!("day {n}, 8%"),
        );
        let a = assert_reserves(&tight, 120_000, &format!("day {n}, 12%"));
        if n == 0 {
            assert!(a > Bani::ZERO, "the banks borrow the gap on day 0");
        }
        advanced |= a > Bani::ZERO;
    }
    assert!(advanced);
}

/// **AC-BANK-08** `[sim]` Quiet baseline: on the Romania scenario with the
/// 2021 law, over 120 months total bank loans to firms stay within 30% of
/// their start and capital adequacy above `car_min`.
#[test]
fn ac_bank_08() {
    let p = params();
    let mut w = world_2021(1000);
    let start = i128::from(w.bank().firm_loans.get());
    assert!(start > 0);
    let mut closes = 0;
    while w.months().len() < 120 {
        let r = step(&mut w);
        if w.months().len() > closes {
            closes = w.months().len();
            let b = w.bank();
            let loans = i128::from(b.firm_loans.get());
            assert!(
                (loans - start).abs() * 10 <= start * 3,
                "{:?}: loans to firms {loans} against {start} at the start",
                r.date
            );
            assert!(
                b.car_ppm >= p.banking.car_min_ppm,
                "{:?}: capital adequacy {} ppm",
                r.date,
                b.car_ppm
            );
        }
    }
}
