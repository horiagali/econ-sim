//! Acceptance tests for AC-ST-05 to AC-ST-08 (spec: economy/social-transfers),
//! the M3 `[sim]` criteria, on the `World`'s pay day under `Law::law_2021()`.
//! Written in a test-authoring session (ADR-0014) against the `World` and the
//! test writer's additions behind the feature `pay_day`: the read-only record
//! of the pay day (`World::pay_slips`) and a layoff applied at load
//! (`StartEdit::Layoff`), by which AC-ST-08 compares two worlds.

use econ_core::world::{Law, StartEdit, World};
use econ_ledger::{FlowCode, Sector};
use econ_mech_transfers::unemployment_benefit;
use econ_popgen::{EduLevel, NO_EMPLOYER};
use econ_types::{Bani, Day};

use super::support::{industry, params, person_weights, start_state, step, units_of, weighted};
use super::support_m3::{
    FlowSum, budget_programme_cash, flow, within, world_2021, world_2021_with,
};

/// Step to the end of the next pay day (the month end).
fn to_pay_day(w: &mut World) -> econ_core::world::DayReport {
    loop {
        let r = step(w);
        if r.date.is_month_end() {
            return r;
        }
    }
}

/// Weighted sum of a per-person column.
fn per_person(w: &World, column: &[Bani]) -> Bani {
    weighted(&person_weights(w), column)
}

/// Weighted sum of a per-household column.
fn per_household(w: &World, column: &[Bani]) -> Bani {
    weighted(&w.households().hh_weight, column)
}

/// **AC-ST-05** `[sim]` On every pay day, for each programme, the
/// government's payment equals the weighted sum of the persons' or
/// households' amounts, and the ledger records it under the programme's flow
/// code; each household's `hh_income_received` grows by its share.
#[test]
fn ac_st_05() {
    let mut w = world_2021(1000);
    let mut paid = [Bani::ZERO; 4];
    for _ in 0..3 {
        let before = w.households().hh_income_received.clone();
        let r = to_pay_day(&mut w);
        let s = w.pay_slips().clone();
        assert_eq!(s.day, Some(r.day));
        let d = r.date;
        for (k, (c, amount)) in [
            (FlowCode::BENEFIT_PENSION, per_person(&w, &s.pension)),
            (
                FlowCode::BENEFIT_UNEMPLOYMENT,
                per_person(&w, &s.unemployment_benefit),
            ),
            (
                FlowCode::BENEFIT_FAMILY,
                per_household(&w, &s.child_allowance),
            ),
            (
                FlowCode::BENEFIT_SOCIAL_ASSISTANCE,
                per_household(&w, &s.minimum_income),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                flow(&w, c, Sector::Government),
                Bani::ZERO - amount,
                "{d:?}: the government's payment under {}",
                c.name()
            );
            assert_eq!(
                flow(&w, c, Sector::Households),
                amount,
                "{d:?}: {}",
                c.name()
            );
            paid[k] += amount;
        }
        // Each household's share: what its members and it were paid today.
        let h = w.households();
        let persons = w.persons();
        let mut share = vec![Bani::ZERO; h.hh_weight.len()];
        for (p, &hh) in persons.household_id.iter().enumerate() {
            share[hh as usize] +=
                s.net_wage[p] + s.pension[p] - s.pension_tax[p] + s.unemployment_benefit[p];
        }
        for k in 0..share.len() {
            share[k] += s.mixed_income[k] - s.self_employment_tax[k]
                + s.child_allowance[k]
                + s.minimum_income[k];
            assert_eq!(
                s.received[k], share[k],
                "{d:?}: household {k}'s pay-day total"
            );
            // Interest is income of the month too, once the banks pay it.
            #[cfg(feature = "banking")]
            let extra = w.bank().hh_interest_received[k];
            #[cfg(not(feature = "banking"))]
            let extra = Bani::ZERO;
            // A household that decided today started counting again today
            // (its decision comes before pay).
            let start = if h.last_decision[k] == Some(r.day) {
                Bani::ZERO
            } else {
                before[k]
            };
            assert_eq!(
                h.hh_income_received[k],
                start + s.received[k] + extra,
                "{d:?}: household {k}'s hh_income_received"
            );
        }
    }
    for (k, v) in paid.iter().enumerate() {
        assert!(
            *v > Bani::ZERO,
            "programme {k} paid nothing in three months"
        );
    }
}

/// **AC-ST-06** `[sim]` On the Romania scenario, the first month's pensions
/// equal stage G's pension column person by person, except the persons raised
/// to the minimum; the year's pension bill is within 2% of 2021's
/// (budget-2021, old-age and survivors).
#[test]
fn ac_st_06() {
    let start = start_state(1000);
    let minimum = Law::law_2021()
        .transfers
        .expect("the 2021 law has transfers")
        .pension
        .minimum;
    assert_eq!(minimum, Bani::from_lei(800));
    let mut w = world_2021(1000);
    let mut sum = FlowSum::default();
    let r = loop {
        let r = step(&mut w);
        sum.add_day(&w);
        if r.date.is_month_end() {
            break r;
        }
    };
    let s = w.pay_slips();
    assert_eq!(s.day, Some(r.day));
    let mut raised = 0;
    for (p, &g) in start.pension.iter().enumerate() {
        let expected = if g <= Bani::ZERO {
            Bani::ZERO
        } else if g < minimum {
            raised += 1;
            minimum
        } else {
            g
        };
        assert_eq!(
            s.pension[p], expected,
            "person {p}: stage G's pension {g:?}"
        );
    }
    eprintln!("AC-ST-06: {raised} pensions raised to the minimum at 1:1000");
    while w.months().len() < 12 {
        step(&mut w);
        sum.add_day(&w);
    }
    let bill = Bani::ZERO - sum.get(FlowCode::BENEFIT_PENSION, Sector::Government);
    let budget = budget_programme_cash("old_age") + budget_programme_cash("survivors");
    eprintln!(
        "AC-ST-06: pension bill of the first year {} million lei, 2021 {} million lei",
        bill.get() / 100_000_000,
        budget.get() / 100_000_000
    );
    assert!(
        within(bill, budget, 20_000),
        "the year's pension bill {bill:?} against 2021's {budget:?}"
    );
}

/// Children under 18 in each household at load.
fn children(n_hh: usize) -> Vec<u8> {
    let start = start_state(1000);
    let mut c = vec![0u8; n_hh];
    for (p, &h) in start.household_id.iter().enumerate() {
        if start.age[p] < 18 * 12 {
            c[h as usize] = c[h as usize].saturating_add(1);
        }
    }
    c
}

/// **AC-ST-07** `[sim]` Doubling the child allowance lowers the number of
/// households with children in hardship (households) within two months, and
/// raises their consumption.
#[test]
fn ac_st_07() {
    let mut law = Law::law_2021();
    let child = &mut law.transfers.as_mut().expect("transfers").child;
    child.under2 = child.under2.times(2);
    child.to18 = child.to18.times(2);
    let mut base = world_2021(1000);
    let mut doubled = World::new(start_state(1000), law, params()).expect("the world builds");
    for w in [&mut base, &mut doubled] {
        while w.months().len() < 2 {
            step(w);
        }
    }
    let kids = children(base.households().hh_weight.len());
    let in_hardship = |w: &World| -> u64 {
        let h = w.households();
        (0..kids.len())
            .filter(|&k| kids[k] > 0 && h.hh_hardship[k])
            .map(|k| u64::from(h.hh_weight[k]))
            .sum()
    };
    let spending = |w: &World| -> i128 {
        let h = w.households();
        (0..kids.len())
            .filter(|&k| kids[k] > 0)
            .map(|k| i128::from(h.hh_weight[k]) * i128::from(h.spending_to_date[k].get()))
            .sum()
    };
    let (hb, hd) = (in_hardship(&base), in_hardship(&doubled));
    eprintln!("AC-ST-07: households with children in hardship {hb} → {hd}");
    assert!(
        hb > 0,
        "no household with children in hardship to begin with"
    );
    assert!(
        hd < hb,
        "doubling the allowance: {hd} in hardship against {hb}"
    );
    let (sb, sd) = (spending(&base), spending(&doubled));
    assert!(sd > sb, "their consumption: {sd} against {sb}");
}

/// Contribution years (social transfers, simplification): years since 18
/// less the years of schooling above upper secondary.
fn contribution_years(age_months: u16, edu: EduLevel) -> u8 {
    let schooling = match edu {
        EduLevel::Isced4 => 2,
        EduLevel::Isced5To6 => 4,
        EduLevel::Isced7To8 => 6,
        _ => 0,
    };
    u8::try_from(
        ((age_months / 12).saturating_sub(18))
            .saturating_sub(schooling)
            .min(60),
    )
    .expect("small")
}

/// **AC-ST-08** `[sim]` Laying off 10% of one industry's staff raises the
/// unemployment benefit bill in the following month by about the laid-off
/// persons' benefit, and the bill falls as they are hired back.
#[test]
#[allow(clippy::needless_range_loop)]
fn ac_st_08() {
    let mut base = world_2021(1000);
    let retail = industry(&base, "retail_trade");
    let mut laid = world_2021_with(
        1000,
        vec![StartEdit::Layoff {
            industry: retail,
            share_ppm: 100_000,
        }],
        params(),
    );
    // Who was laid off: employed by the industry in the base, not in the
    // edited world, at load.
    let units = units_of(&base, retail);
    let start = start_state(1000);
    let weights = person_weights(&base);
    let rule = Law::law_2021().transfers.expect("transfers").unemployment;
    let (b, l) = (base.persons(), laid.persons());
    let mut group = Vec::new();
    let mut expected = 0i128;
    let mut staff = 0u64;
    for p in 0..b.employer.len() {
        let e = b.employer[p];
        if e == NO_EMPLOYER || !units.contains(&usize::from(e)) {
            continue;
        }
        staff += u64::from(weights[p]);
        if l.employer[p] == NO_EMPLOYER {
            group.push(p);
            let years = contribution_years(start.age[p], start.edu_level[p]);
            let benefit = unemployment_benefit(b.wage[p], years, &rule);
            expected += i128::from(weights[p]) * i128::from(benefit.get());
        }
    }
    let laid_off: u64 = group.iter().map(|&p| u64::from(weights[p])).sum();
    assert!(!group.is_empty(), "nobody was laid off");
    assert!(
        laid_off * 100 >= staff * 8 && laid_off * 100 <= staff * 12,
        "laid off {laid_off} of {staff}: about 10%"
    );
    let expected = Bani(i64::try_from(expected).expect("fits"));
    // Month by month: the group's benefits and the difference in the bill.
    let mut group_bill = Vec::new();
    let mut excess = Vec::new();
    let mut rehired = Vec::new();
    for _ in 0..6 {
        let rb = to_pay_day(&mut base);
        let rl = to_pay_day(&mut laid);
        assert_eq!(rb.day, rl.day);
        let sl = laid.pay_slips();
        let wl = person_weights(&laid);
        let g: i128 = group
            .iter()
            .map(|&p| i128::from(wl[p]) * i128::from(sl.unemployment_benefit[p].get()))
            .sum();
        group_bill.push(g);
        let bill =
            |w: &World| Bani::ZERO - flow(w, FlowCode::BENEFIT_UNEMPLOYMENT, Sector::Government);
        excess.push(i128::from((bill(&laid) - bill(&base)).get()));
        rehired.push(
            group
                .iter()
                .filter(|&&p| laid.persons().employer[p] != NO_EMPLOYER)
                .count(),
        );
    }
    eprintln!(
        "AC-ST-08: expected {} lei; group's benefits {group_bill:?} bani; excess bill {excess:?} bani; rehired {rehired:?} of {}",
        expected.get() / 100,
        group.len()
    );
    // The following month: about the laid-off persons' benefit.
    let first = Bani(i64::try_from(group_bill[0]).expect("fits"));
    assert!(
        within(first, expected, 50_000),
        "the group's benefits in the following month {first:?} against {expected:?}"
    );
    let first_excess = Bani(i64::try_from(excess[0]).expect("fits"));
    assert!(
        within(first_excess, expected, 200_000),
        "the rise of the bill {first_excess:?} against {expected:?}"
    );
    // Then it falls as they are hired back.
    assert!(rehired[5] > 0, "nobody was hired back in six months");
    assert!(
        group_bill[5] < group_bill[0],
        "the group's benefits do not fall: {group_bill:?}"
    );
    assert!(
        excess[5] < excess[0],
        "the excess bill does not fall: {excess:?}"
    );
    assert_eq!(laid.day(), Day(base.day().0));
}
