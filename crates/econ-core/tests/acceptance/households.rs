//! Acceptance tests for the `[sim]` criteria of the households mechanic on the
//! `World`: AC-HH-03, 04, 05, 07 and 08 (spec: economy/households, "First
//! version"). Written in a test-authoring session from the specs and their
//! API sketches, before the code existed (ADR-0014).

use econ_core::world::{PhaseId, StartEdit, World};
use econ_ledger::Sector;
use econ_types::{Bani, Day};

use super::support::{
    assert_day_checks, assert_staggered_today, calendar, io_table_json, money, sector_flows, step,
    units_of, weighted, world, world_with,
};

/// Per household: money, income and spending to date.
fn snapshot(w: &World) -> (Vec<Bani>, Vec<Bani>, Vec<Bani>) {
    let h = w.households();
    (
        money(w),
        h.income_to_date.clone(),
        h.spending_to_date.clone(),
    )
}

/// Every household: change in money = income received − spending, between two
/// snapshots, exactly.
fn assert_budget_identity(
    before: &(Vec<Bani>, Vec<Bani>, Vec<Bani>),
    after: &(Vec<Bani>, Vec<Bani>, Vec<Bani>),
    when: &str,
) {
    for h in 0..before.0.len() {
        let income = after.1[h] - before.1[h];
        let spending = after.2[h] - before.2[h];
        assert!(
            income >= Bani::ZERO && spending >= Bani::ZERO,
            "{when}: household {h}"
        );
        assert_eq!(
            after.0[h] - before.0[h],
            income - spending,
            "{when}: household {h}: change in cash and deposits"
        );
    }
}

/// Consumption of the first `n` closed months.
fn consumption(w: &World, n: usize) -> i128 {
    w.months()[..n]
        .iter()
        .map(|m| i128::from(m.consumption.get()))
        .sum()
}

/// **AC-HH-03** `[sim]` Budget identity: for every household and every
/// month, income received − spending = change in cash and deposits, exactly;
/// summed with weights it equals the ledger's household flows.
#[test]
fn ac_hh_03() {
    let mut w = world(1000);
    let weights = w.households().hh_weight.clone();
    let mut month_start = snapshot(&w);
    let mut yesterday = month_start.clone();
    let mut months = 0;
    for _ in 0..400 {
        let r = step(&mut w);
        let today = snapshot(&w);
        // The day: weighted income less spending is the households' column of
        // the day's flows, every flow code together.
        let income = weighted(&weights, &today.1) - weighted(&weights, &yesterday.1);
        let spending = weighted(&weights, &today.2) - weighted(&weights, &yesterday.2);
        assert_eq!(
            sector_flows(&w, Sector::Households),
            income - spending,
            "day {}: the ledger's household flows",
            r.day.0
        );
        if r.date.is_month_end() {
            assert_budget_identity(&month_start, &today, &format!("month to {:?}", r.date));
            month_start = today.clone();
            months += 1;
        }
        yesterday = today;
    }
    assert_eq!(months, 13);
}

/// **AC-HH-04** `[sim]` Each household decides exactly once in every
/// calendar month, on its own day; over a month every household has decided.
#[test]
fn ac_hh_04() {
    let mut w = world(1000);
    let cal = calendar(&w);
    let n = w.households().hh_weight.len();
    let mut decided_this_month = vec![0u32; n];
    for _ in 0..400 {
        let r = step(&mut w);
        assert_staggered_today(&w, &r);
        let h = w.households();
        for (k, &last) in decided_this_month.iter_mut().zip(&h.last_decision) {
            if last == Some(r.day) {
                *k += 1;
            }
        }
        if r.date.is_month_end() {
            let m = cal.month(r.day);
            for (i, &k) in decided_this_month.iter().enumerate() {
                assert_eq!(k, 1, "household {i} decided {k} times in month {}", m.0);
            }
            decided_this_month.iter_mut().for_each(|k| *k = 0);
        }
    }
}

/// **AC-HH-05** `[sim]` A transfer of 1,000 lei to every household of the
/// poorest fifth raises aggregate consumption over the next two months by
/// more than the same total given to the richest fifth.
#[test]
#[ignore = "waits for M3's benefits (owner 2026-10-20): without the minimum income and child allowance the poorest fifth spends a transfer less than the richest"]
fn ac_hh_05() {
    // The fifths at load, from the starting income.
    let mut base = world(1000);
    let h = base.households();
    let (weights, fifth) = (h.hh_weight.clone(), h.hh_fifth.clone());
    let weight_of = |q: u8| -> i64 {
        weights
            .iter()
            .zip(&fifth)
            .filter(|&(_, &f)| f == q)
            .map(|(&w, _)| i64::from(w))
            .sum()
    };
    let (poor, rich) = (weight_of(0), weight_of(4));
    assert!(poor > 0 && rich > 0, "both fifths have households");
    // 1,000 lei to each poorest household; the same total over the richest.
    let to_poor = Bani::from_lei(1_000);
    let total = to_poor.times(poor);
    let to_rich = total.mul_ratio(1, rich);
    let total_rich = to_rich.times(rich);
    let transfer = |q: u8, amount: Bani| -> Vec<Bani> {
        fifth
            .iter()
            .map(|&f| if f == q { amount } else { Bani::ZERO })
            .collect()
    };
    let mut to_the_poor = world_with(1000, vec![StartEdit::Transfer(transfer(0, to_poor))]);
    let mut to_the_rich = world_with(1000, vec![StartEdit::Transfer(transfer(4, to_rich))]);
    // The next two months: December and January.
    for w in [&mut base, &mut to_the_poor, &mut to_the_rich] {
        while w.months().len() < 2 {
            step(w);
        }
    }
    let c = consumption(&base, 2);
    let more_poor = consumption(&to_the_poor, 2) - c;
    let more_rich = consumption(&to_the_rich, 2) - c;
    assert!(
        more_poor > 0,
        "the transfer to the poorest raises consumption"
    );
    // Per leu given: (more_poor / total) > (more_rich / total_rich).
    assert!(
        more_poor * i128::from(total_rich.get()) > more_rich * i128::from(total.get()),
        "consumption rose by {more_poor} bani after {total:?} to the poorest, \
         by {more_rich} after {total_rich:?} to the richest"
    );
}

/// **AC-HH-07** `[sim]` When an industry has nothing to sell, the money meant
/// for its good stays with the households and the shortage is recorded; no
/// money leaves without a sale.
#[test]
fn ac_hh_07() {
    let mut base = world(1000);
    // The good households spend most on.
    let shares = &base.industries().basket_share_ppm;
    let x = (0..shares.len())
        .max_by_key(|&j| (shares[j], std::cmp::Reverse(j)))
        .expect("industries");
    assert!(shares[x] > 0);
    let mut shut = world_with(1000, vec![StartEdit::ShutIndustry(x)]);
    let units = units_of(&shut, x);
    assert!(!units.is_empty(), "the industry has units");
    for &u in &units {
        assert_eq!(shut.firm_units().inventory[u], 0, "unit {u}: no stock");
        assert_eq!(shut.firm_units().capacity[u], 0, "unit {u}: no capacity");
    }
    // Day 0: the same households decide the same budgets in both worlds;
    // where the good cannot be had, they spend less, never more.
    let rb = step(&mut base);
    let mut before = snapshot(&shut);
    let mut deposits: Vec<Bani> = units
        .iter()
        .map(|&u| shut.firm_units().deposits[u])
        .collect();
    let r = step(&mut shut);
    assert_eq!(rb.phases, r.phases);
    let (hb, hs) = (base.households(), shut.households());
    let mut deciding = 0;
    let (mut spent_base, mut spent_shut) = (0i128, 0i128);
    for i in 0..hs.hh_weight.len() {
        assert_eq!(hb.last_decision[i], hs.last_decision[i], "household {i}");
        if hs.last_decision[i] == Some(Day(0)) {
            deciding += 1;
            assert!(
                hs.spending_to_date[i] <= hb.spending_to_date[i],
                "household {i} spent more where a good was missing"
            );
            let w = i128::from(hs.hh_weight[i]);
            spent_base += w * i128::from(hb.spending_to_date[i].get());
            spent_shut += w * i128::from(hs.spending_to_date[i].get());
        }
    }
    assert!(deciding > 0, "households decide on 1 December");
    assert!(
        spent_shut < spent_base,
        "the money meant for the missing good was not kept"
    );
    // Four weeks: the industry sells nothing, receives nothing, and all its
    // demand is shortage; every household's money is income less what it
    // bought.
    for n in 0..28 {
        let r = if n == 0 { r.clone() } else { step(&mut shut) };
        assert!(r.phases.contains(&PhaseId::Households));
        let after = snapshot(&shut);
        assert_budget_identity(&before, &after, &format!("day {}", r.day.0));
        let f = shut.firm_units();
        for (k, &u) in units.iter().enumerate() {
            assert_eq!(f.sales_today[u], 0, "day {}: unit {u} sold", r.day.0);
            assert_eq!(f.revenue_today[u], Bani::ZERO, "day {}: unit {u}", r.day.0);
            assert!(
                f.deposits[u] <= deposits[k],
                "day {}: unit {u} received money without a sale",
                r.day.0
            );
            deposits[k] = f.deposits[u];
        }
        let i = shut.industries();
        assert_eq!(i.sales_value_today[x], Bani::ZERO, "day {}", r.day.0);
        assert!(
            i.demand_today[x] > 0,
            "day {}: households want the good",
            r.day.0
        );
        assert_eq!(
            i.shortage_today[x], i.demand_today[x],
            "day {}: the shortage is recorded",
            r.day.0
        );
        assert_day_checks(&shut, &r);
        before = after;
    }
    while shut.months().is_empty() {
        step(&mut shut);
    }
    assert!(
        shut.months()[0].shortages[x] > 0,
        "the month's statistics record the shortage"
    );
}

/// **AC-HH-08** `[sim]` On the Romania scenario with every other mechanic at
/// its starting values, household consumption of the first twelve months is
/// within 10% of the input-output table's 720 billion lei.
#[test]
fn ac_hh_08() {
    let table = io_table_json()["totals"]["household_consumption"]
        .as_i64()
        .expect("the table's household consumption, lei");
    // 719,713,363,386 lei in the fixture: "720 billion lei".
    assert!((715_000_000_000..725_000_000_000).contains(&table));
    let mut w = world(1000);
    while w.months().len() < 12 {
        step(&mut w);
    }
    let year = consumption(&w, 12);
    let target = i128::from(table) * 100;
    assert!(
        (year - target).abs() * 10 <= target,
        "consumption of the first twelve months: {} lei against {table}",
        year / 100
    );
}
