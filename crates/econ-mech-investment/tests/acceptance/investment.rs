//! Acceptance tests for AC-INV-01 to 03 (spec: economy/investment-capital),
//! the `[unit]` criteria. Written in a test-authoring session (ADR-0014) from
//! the spec and its API sketch only: the code came first, by the owner's
//! decision of 2026-10-21.

use econ_mech_investment::{
    CapitalBook, Controller, InvestParams, desired_investment, dividend, financed,
    split_by_ownership, starting_growth_ppm,
};
use econ_types::Bani;

const ONE: i128 = 1_000_000;

/// The spec's tuning parameters, millionths.
fn params() -> InvestParams {
    let p = InvestParams::spec_defaults();
    assert_eq!(p.depreciation_ppm, 65_000);
    assert_eq!(p.gestation_months, 3);
    assert_eq!(p.cu_target_ppm, 800_000);
    assert_eq!(p.beta_cu_ppm, 500_000);
    assert_eq!(p.beta_r_ppm, 1_000_000);
    // 1/24 a month, rounded.
    assert_eq!(p.growth_adjust_ppm, 41_667);
    assert_eq!(p.growth_bound_ppm, 100_000);
    assert_eq!(p.payout_domestic_ppm, 300_000);
    assert_eq!(p.payout_foreign_ppm, 450_000);
    assert_eq!(p.payout_state_ppm, 500_000);
    assert_eq!(p.cash_sweep_ppm, 100_000);
    p
}

/// `a · b / c` rounded half away from zero.
fn round_div(n: i128, d: i128) -> i128 {
    assert!(d > 0);
    if n >= 0 {
        (n + d / 2) / d
    } else {
        -((-n + d / 2) / d)
    }
}

/// Capitals from a small unit to the largest (real bani: 10 thousand lei to
/// 100 billion lei).
const CAPITALS: [i64; 5] = [
    1_000_000,
    123_456_789,
    98_765_432_109,
    1_000_000_000_000,
    10_000_000_000_000,
];

/// **AC-INV-01** `[unit]` The investment budget follows step 1. With the
/// starting inputs it equals `I0_f / 12`; it is never negative; it does not
/// rise with the real rate, and does not fall with utilisation or expected
/// growth.
///
/// The starting inputs: the price index 1, utilisation at cu*, the real rate
/// at its start (gap 0) and expected growth `I0_f / K_f − δ`. That growth is
/// held in whole millionths, so "equals" allows its rounding: half a
/// millionth of `K / 12`, and a ban.
#[test]
fn ac_inv_01() {
    let p = params();
    for &k in &CAPITALS {
        for share_ppm in [65_000_i64, 100_000, 120_000, 250_000] {
            // A year of starting investment, I0_f.
            let i0_year =
                Bani(i64::try_from(i128::from(k) * i128::from(share_ppm) / ONE).expect("fits"));
            let g = starting_growth_ppm(i0_year, k, &p);
            assert_eq!(
                i64::from(g),
                i64::try_from(round_div(i128::from(i0_year.get()) * ONE, i128::from(k)))
                    .expect("fits")
                    - i64::from(p.depreciation_ppm),
                "K {k}: starting growth is I0/K − δ"
            );
            let budget = desired_investment(k, 1_000_000, p.cu_target_ppm, 0, g, &p);
            let month = round_div(i128::from(i0_year.get()), 12);
            let room = i128::from(k) / 12 / ONE + 1;
            assert!(
                (i128::from(budget.get()) - month).abs() <= room,
                "K {k}, I0 {i0_year:?}: budget {budget:?}, I0/12 {month}"
            );
        }
    }
    // Never negative; monotone in each input.
    let rates = [
        -100_000, -20_000, 0, 10_000, 20_000, 50_000, 200_000, 1_000_000,
    ];
    let utils = [0, 300_000, 600_000, 800_000, 900_000, 1_000_000];
    let growths = [-200_000, -100_000, -65_000, 0, 35_000, 100_000, 300_000];
    for &k in &CAPITALS {
        for price in [500_000_u32, 1_000_000, 1_300_000] {
            for &r in &rates {
                for &u in &utils {
                    for &g in &growths {
                        let b = desired_investment(k, price, u, r, g, &p);
                        assert!(b >= Bani::ZERO, "K {k}, rate gap {r}, cu {u}, g {g}: {b:?}");
                    }
                }
            }
            for &u in &utils {
                for &g in &growths {
                    let mut last = None;
                    for &r in &rates {
                        let b = desired_investment(k, price, u, r, g, &p);
                        if let Some(prev) = last {
                            assert!(b <= prev, "K {k}: rises with the real rate at {r}");
                        }
                        last = Some(b);
                    }
                }
            }
            for &r in &rates {
                for &g in &growths {
                    let mut last = None;
                    for &u in &utils {
                        let b = desired_investment(k, price, u, r, g, &p);
                        if let Some(prev) = last {
                            assert!(b >= prev, "K {k}: falls with utilisation at {u}");
                        }
                        last = Some(b);
                    }
                }
            }
            for &r in &rates {
                for &u in &utils {
                    let mut last = None;
                    for &g in &growths {
                        let b = desired_investment(k, price, u, r, g, &p);
                        if let Some(prev) = last {
                            assert!(b >= prev, "K {k}: falls with expected growth at {g}");
                        }
                        last = Some(b);
                    }
                }
            }
        }
    }
    // What it can finance: the budget, at most spare money plus credit room.
    for (d, s, c) in [
        (100, 30, 50),
        (100, 80, 50),
        (100, 0, 0),
        (0, 10, 10),
        (70, 100, 0),
    ] {
        let f = financed(Bani(d), Bani(s), Bani(c));
        assert_eq!(f, Bani(d.min(s + c)), "financed({d}, {s}, {c})");
    }
}

/// **AC-INV-02** `[unit]` Capital follows step 6. Each month
/// `K' = K − round(K δ / 12) +` the oldest slot. A purchase enters capital
/// exactly `gestation_months` later. With no purchases capital falls by
/// δ/12 a month.
#[test]
fn ac_inv_02() {
    let p = params();
    let months = usize::from(p.gestation_months);
    let dep = |k: i64| {
        i64::try_from(round_div(
            i128::from(k) * i128::from(p.depreciation_ppm),
            12 * ONE,
        ))
        .expect("fits")
    };
    for &k in &CAPITALS {
        // At the start each slot holds a month of the starting investment.
        let start_month = k / 150;
        let mut book = CapitalBook::new(k, start_month, &p);
        assert_eq!(book.capital, k);
        assert_eq!(book.in_progress, vec![start_month; months]);
        assert_eq!(book.depreciation(&p), dep(k));
        // A month of steady purchases, then one purchase, then none.
        let spending = [start_month, start_month + 7, 0, 0, 0, 0, 0, 0];
        let mut slots: Vec<i64> = book.in_progress.clone();
        for &spent in &spending {
            let before = book.capital;
            let (d, completed) = book.close_month(spent, &p);
            assert_eq!(d, dep(before), "K {before}: the month's depreciation");
            assert_eq!(completed, slots[0], "the oldest slot completes");
            assert_eq!(
                book.capital,
                before - dep(before) + slots[0],
                "K' = K − round(Kδ/12) + oldest"
            );
            slots.remove(0);
            slots.push(spent);
            assert_eq!(
                book.in_progress, slots,
                "the month's spending enters the newest slot"
            );
        }
        // The one purchase (month 1's extra 7) entered at the close
        // `gestation_months` after it: month 1 + 3 = month 4.
        let mut book = CapitalBook::new(k, 0, &p);
        let mut entered = Vec::new();
        for m in 0..10 {
            let spent = if m == 1 { 1_000_003 } else { 0 };
            let (_, completed) = book.close_month(spent, &p);
            if completed != 0 {
                entered.push((m, completed));
            }
        }
        assert_eq!(
            entered,
            vec![(1 + months, 1_000_003)],
            "K {k}: a purchase enters `gestation_months` later"
        );
        // No purchases: falls by δ/12 a month.
        let mut book = CapitalBook::new(k, 0, &p);
        for _ in 0..24 {
            let before = book.capital;
            book.close_month(0, &p);
            assert_eq!(book.capital, before - dep(before));
            assert!(book.capital < before || before == 0);
        }
    }
}

/// **AC-INV-03** `[unit]` Dividends follow step 5. The dividend is the
/// formula, never negative, never above spare money. The ownership split sums
/// to the dividend to the ban, each part within one ban of its share.
///
/// The formula: `min(spare, θ_c max(0, Π) + σ max(0, M − m0 C))`, `θ_c` the
/// controller's payout; rounding to whole bani allows one ban.
#[test]
fn ac_inv_03() {
    let p = params();
    let payout = |c: Controller| match c {
        Controller::Domestic => p.payout_domestic_ppm,
        Controller::Foreign => p.payout_foreign_ppm,
        Controller::State => p.payout_state_ppm,
    };
    let profits = [
        -5_000_000_000_i64,
        -1,
        0,
        1,
        999,
        7_777_777,
        3_000_000_000_000,
    ];
    let deposits = [-100, 0, 50_000, 10_000_000, 9_000_000_000_000];
    let targets = [0_i64, 40_000, 10_000_000, 5_000_000_000_000];
    let spares = [-1_000, 0, 1, 1_000_000, 10_000_000_000_000];
    for c in [Controller::Domestic, Controller::Foreign, Controller::State] {
        let theta = i128::from(payout(c));
        for &pi in &profits {
            for &m in &deposits {
                for &t in &targets {
                    for &s in &spares {
                        let d = dividend(Bani(pi), Bani(m), Bani(t), Bani(s), c, &p);
                        let formula = theta * i128::from(pi.max(0))
                            + i128::from(p.cash_sweep_ppm) * i128::from((m - t).max(0));
                        let expected = (formula as f64 / 1e6).min(s as f64).max(0.0);
                        assert!(
                            d >= Bani::ZERO,
                            "{c:?} Π {pi} M {m} target {t} spare {s}: {d:?}"
                        );
                        assert!(
                            d <= Bani(s.max(0)),
                            "{c:?} Π {pi} spare {s}: {d:?} above spare money"
                        );
                        assert!(
                            (d.get() as f64 - expected).abs() <= 1.0,
                            "{c:?} Π {pi} M {m} target {t} spare {s}: {d:?}, the formula {expected}"
                        );
                    }
                }
            }
        }
    }
    // A loss-making unit pays the excess-money part only.
    let d = dividend(
        Bani(-1_000_000),
        Bani(2_000_000),
        Bani(1_000_000),
        Bani(10_000_000),
        Controller::Foreign,
        &p,
    );
    assert_eq!(d, Bani(100_000));
    // The split.
    let shares: [[u32; 3]; 7] = [
        [0, 1_000_000, 0],
        [0, 0, 1_000_000],
        [1_000_000, 0, 0],
        [333_333, 333_333, 333_334],
        [510_000, 120_000, 370_000],
        [1, 2, 999_997],
        [250_000, 250_000, 500_000],
    ];
    for amount in [0_i64, 1, 2, 3, 101, 999_999, 123_456_789_012] {
        for sh in shares {
            let parts = split_by_ownership(Bani(amount), sh[0], sh[1], sh[2]);
            assert_eq!(
                parts.iter().copied().sum::<Bani>(),
                Bani(amount),
                "{amount} by {sh:?}"
            );
            for k in 0..3 {
                let exact = i128::from(amount) * i128::from(sh[k]);
                let diff = (i128::from(parts[k].get()) * ONE - exact).abs();
                assert!(
                    diff <= ONE,
                    "{amount} by {sh:?}: part {k} is {:?}",
                    parts[k]
                );
                assert!(parts[k] >= Bani::ZERO);
            }
        }
    }
}
