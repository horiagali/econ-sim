//! Acceptance tests for the `[sim]` criteria of production on the `World`:
//! AC-PROD-03 to AC-PROD-07 and AC-PROD-09 (spec: economy/production, "First
//! version"). Written in a test-authoring session from the specs and their
//! API sketches, before the code existed (ADR-0014). The staff cut of the
//! bottleneck criterion stays owed: M2 has no way to hold a unit's staff down
//! (at load the labour market hires the gap back within the month).

use econ_core::world::{PhaseId, StartEdit, World};
use econ_types::{Bani, Calendar};

use super::support::{
    assert_day_checks, calendar, industry, industry_output_today, step, total, units_of, world,
    world_with,
};

/// Output of an industry in each calendar month, over the first `months`
/// months.
fn monthly_output(w: &mut World, cal: Calendar, industry: usize, months: u32) -> Vec<i64> {
    let mut out = vec![0i64; months as usize];
    loop {
        let r = step(w);
        let m = cal.month(r.day).0;
        if m >= months {
            return out;
        }
        out[m as usize] += industry_output_today(w, industry);
        if m + 1 == months && r.date.is_month_end() {
            return out;
        }
    }
}

/// The basket with one good's share times `num / den`, the others scaled
/// down in proportion, summing to 1,000,000 exactly.
fn basket_with_more(shares: &[u32], good: usize, num: u64, den: u64) -> Vec<u32> {
    let old = u64::from(shares[good]);
    let new = old * num / den;
    let rest_old = 1_000_000 - old;
    let rest_new = 1_000_000 - new;
    let mut out: Vec<u32> = shares
        .iter()
        .enumerate()
        .map(|(j, &s)| {
            if j == good {
                u32::try_from(new).expect("a share")
            } else {
                u32::try_from(u64::from(s) * rest_new / rest_old).expect("a share")
            }
        })
        .collect();
    let sum: u64 = out.iter().map(|&s| u64::from(s)).sum();
    let largest = (0..out.len())
        .filter(|&j| j != good)
        .max_by_key(|&j| out[j])
        .expect("other goods");
    out[largest] += u32::try_from(1_000_000 - sum).expect("a remainder");
    out
}

/// **AC-PROD-03** `[sim]` Inventories: for every unit and day, inventory
/// change = output − sales, exactly; no inventory is negative.
#[test]
#[allow(clippy::needless_range_loop)]
fn ac_prod_03() {
    let mut w = world(1000);
    let mut before = w.firm_units().inventory.clone();
    assert!(before.iter().all(|&v| v >= 0), "at load");
    for _ in 0..120 {
        let r = step(&mut w);
        let f = w.firm_units();
        for u in 0..before.len() {
            assert_eq!(
                f.inventory[u] - before[u],
                f.output_today[u] - f.sales_today[u],
                "day {}: unit {u}",
                r.day.0
            );
            assert!(f.inventory[u] >= 0, "day {}: unit {u}", r.day.0);
            assert!(f.output_today[u] >= 0 && f.sales_today[u] >= 0);
        }
        before = f.inventory.clone();
    }
}

/// **AC-PROD-04** `[sim]` Money: what buyers pay for an industry's goods in a
/// day equals what its units receive, through a sales clearing account that
/// nets to zero.
#[test]
fn ac_prod_04() {
    let mut w = world(1000);
    let industries = w.industries().id.len();
    let mut paid = Bani::ZERO;
    for _ in 0..120 {
        let r = step(&mut w);
        // The clearing accounts net to zero (I-8) among the day's checks.
        assert_day_checks(&w, &r);
        let (f, i) = (w.firm_units(), w.industries());
        for j in 0..industries {
            let units = units_of(&w, j);
            let received = total(
                &units
                    .iter()
                    .map(|&u| f.revenue_today[u])
                    .collect::<Vec<_>>(),
            );
            assert_eq!(
                received, i.sales_value_today[j],
                "day {}: industry {}",
                r.day.0, i.id[j]
            );
            let sold: i64 = units.iter().map(|&u| f.sales_today[u]).sum();
            assert_eq!(
                sold,
                i.demand_today[j] - i.shortage_today[j],
                "day {}: industry {}: sales are demand served",
                r.day.0,
                i.id[j]
            );
            assert!(i.shortage_today[j] >= 0 && i.shortage_today[j] <= i.demand_today[j]);
            if sold == 0 {
                assert_eq!(received, Bani::ZERO, "day {}: {}", r.day.0, i.id[j]);
            }
            paid += received;
        }
    }
    assert!(paid > Bani::ZERO, "goods were sold");
}

/// **AC-PROD-05** `[sim]` GDP by production, by expenditure and by income
/// agree exactly at every monthly close.
#[test]
fn ac_prod_05() {
    let mut w = world(100);
    while w.months().len() < 3 {
        let r = step(&mut w);
        if r.phases.contains(&PhaseId::MonthlyClose) {
            let m = w.months().last().expect("a closed month");
            assert_eq!(m.gdp_production, m.gdp_expenditure, "{:?}", m.month);
            assert_eq!(m.gdp_production, m.gdp_income, "{:?}", m.month);
        }
    }
}

/// **AC-PROD-06** `[sim]` Each unit plans exactly once in every calendar
/// month, on its own day, before its staffing.
#[test]
fn ac_prod_06() {
    let mut w = world(1000);
    let cal = calendar(&w);
    let units = w.firm_units().industry.len();
    let mut plans = vec![0u32; units];
    for _ in 0..400 {
        let r = step(&mut w);
        let f = w.firm_units();
        let due_today = r.date.day <= 28;
        if due_today {
            let plan = r.phases.iter().position(|&p| p == PhaseId::Plan);
            let staffing = r.phases.iter().position(|&p| p == PhaseId::Staffing);
            assert!(
                matches!((plan, staffing), (Some(a), Some(b)) if a < b),
                "{:?}: the plan runs before the staffing: {:?}",
                r.date,
                r.phases
            );
        }
        for (u, n) in plans.iter_mut().enumerate() {
            let due = due_today && f.unit_stagger_day[u] == r.date.day;
            assert_eq!(f.last_plan[u] == Some(r.day), due, "{:?}: unit {u}", r.date);
            if due {
                assert_eq!(
                    f.last_staffing[u],
                    Some(r.day),
                    "unit {u} staffs the same day"
                );
                *n += 1;
            }
        }
        if r.date.is_month_end() {
            for (u, n) in plans.iter_mut().enumerate() {
                assert_eq!(*n, 1, "unit {u}: plans in month {}", cal.month(r.day).0);
                *n = 0;
            }
        }
    }
}

/// **AC-PROD-07** `[sim]` A permanent rise in household demand for
/// `food_bakery_milling` raises the output of `agri_cereals` in the
/// following months (input-output propagation).
#[test]
fn ac_prod_07() {
    let mut base = world(1000);
    let cal = calendar(&base);
    let bakery = industry(&base, "food_bakery_milling");
    let cereals = industry(&base, "agri_cereals");
    let shares = base.industries().basket_share_ppm.clone();
    assert!(shares[bakery] > 0, "households buy bread");
    // Half as much again of the basket goes to bread and flour.
    let mut more = world_with(
        1000,
        vec![StartEdit::BasketShares(basket_with_more(
            &shares, bakery, 3, 2,
        ))],
    );
    assert!(more.industries().basket_share_ppm[bakery] > shares[bakery]);
    let base_out = monthly_output(&mut base, cal, cereals, 7);
    let more_out = monthly_output(&mut more, cal, cereals, 7);
    let sum = |v: &[i64]| v[1..].iter().sum::<i64>();
    assert!(
        sum(&more_out) > sum(&base_out),
        "cereals over months 1 to 6: {more_out:?} against {base_out:?}"
    );
    for m in 2..7 {
        assert!(
            more_out[m] > base_out[m],
            "month {m}: cereals {} against {}",
            more_out[m],
            base_out[m]
        );
    }
}

/// **AC-PROD-09** `[sim]` Quiet baseline: on the Romania scenario with no
/// change of policy, output of every industry group over 120 months stays
/// within 15% of its starting level, and inventories neither run out nor
/// grow without limit.
///
/// The starting level of a group is its units' starting expected demand (a
/// month of starting output). A calendar month's output is put on a month of
/// 365/12 days before the comparison, since plans spread a month's output
/// over the days of the plan's month. Inventories by group: positive at every
/// month end where they start positive, and never above three times the
/// start.
#[test]
fn ac_prod_09() {
    let mut w = world(1000);
    let cal = calendar(&w);
    let groups = usize::from(*w.industries().group.iter().max().expect("industries")) + 1;
    let group_of_unit: Vec<usize> = w
        .firm_units()
        .industry
        .iter()
        .map(|&j| usize::from(w.industries().group[usize::from(j)]))
        .collect();
    let by_group = |col: &[i64]| -> Vec<i64> {
        let mut g = vec![0i64; groups];
        for (u, &v) in col.iter().enumerate() {
            g[group_of_unit[u]] += v;
        }
        g
    };
    let start_output = by_group(&w.firm_units().expected_demand);
    let start_stock = by_group(&w.firm_units().inventory);
    let mut output = vec![0i64; groups];
    let mut closes = 0;
    while closes < 120 {
        let r = step(&mut w);
        let today = by_group(&w.firm_units().output_today);
        for g in 0..groups {
            output[g] += today[g];
        }
        if !r.date.is_month_end() {
            continue;
        }
        let m = cal.month(r.day).0;
        let days = i128::from(r.date.days_in_month());
        let stock = by_group(&w.firm_units().inventory);
        for g in 0..groups {
            if start_output[g] == 0 {
                continue;
            }
            // Output on a month of 365/12 days, against the start, in millionths.
            let level =
                i128::from(output[g]) * 365 * 1_000_000 / (12 * days * i128::from(start_output[g]));
            assert!(
                (850_000..=1_150_000).contains(&level),
                "month {m}: group {g} output at {level} ppm of its start"
            );
            if start_stock[g] > 0 {
                assert!(stock[g] > 0, "month {m}: group {g} ran out of stock");
                assert!(
                    stock[g] <= 3 * start_stock[g],
                    "month {m}: group {g} stock {} against {} at the start",
                    stock[g],
                    start_stock[g]
                );
            }
        }
        output.iter_mut().for_each(|v| *v = 0);
        closes += 1;
    }
    assert_eq!(w.months().len(), 120);
}
