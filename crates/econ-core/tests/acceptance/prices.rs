//! Acceptance tests for the `[sim]` criteria of prices on the `World`:
//! AC-PRICE-04, 05, 07 and 08 (spec: economy/prices-inflation, "First
//! version"). Written in a test-authoring session from the specs and their
//! API sketches, before the code existed (ADR-0014). The VAT criterion stays
//! owed: it needs the VAT lever mid-run (M4).

use econ_core::world::{PhaseId, StartEdit, World};

use super::support::{io_table_json, month_stats, ppm, step, world, world_with};

/// Each catalogue industry's table industry code.
fn table_code(id: &str) -> String {
    for entry in io_table_json()["catalogue"]
        .as_array()
        .expect("the catalogue")
    {
        let parts = entry["parts"].as_array().expect("parts");
        if parts.iter().any(|p| p["id"] == id) {
            return entry["industry"].as_str().expect("a code").to_string();
        }
    }
    panic!("{id} is not in the catalogue");
}

/// Compensation of employees over output of a table industry, millionths.
fn wage_share_ppm(code: &str) -> i64 {
    let t = io_table_json();
    let k = t["industries"]
        .as_array()
        .expect("industries")
        .iter()
        .position(|i| i["code"] == code)
        .expect("a table industry");
    let comp = t["compensation"][k].as_i64().expect("compensation");
    let out = t["output"][k].as_i64().expect("output");
    ppm(i128::from(comp), i128::from(out))
}

/// `p_j` over the mean price of the other industries, millionths.
fn relative_price(w: &World, j: usize) -> i64 {
    let p = &w.industries().price_ppm;
    let others: i128 = p
        .iter()
        .enumerate()
        .filter(|&(k, _)| k != j)
        .map(|(_, &v)| i128::from(v))
        .sum();
    let n = i128::try_from(p.len() - 1).expect("industries");
    ppm(i128::from(p[j]) * n, others)
}

/// Mean of `edit / base` prices over some industries, millionths.
fn mean_ratio(base: &World, edit: &World, industries: &[usize]) -> i64 {
    let (pb, pe) = (&base.industries().price_ppm, &edit.industries().price_ppm);
    let sum: i64 = industries
        .iter()
        .map(|&j| ppm(i128::from(pe[j]), i128::from(pb[j])))
        .sum();
    sum / i64::try_from(industries.len()).expect("a count")
}

/// **AC-PRICE-04** `[sim]` CPI at each monthly close is the weighted mean of
/// consumer prices with the basket's weights; the weights sum to 1.
#[test]
fn ac_price_04() {
    let mut w = world(1000);
    let mut closes = 0;
    for _ in 0..400 {
        let r = step(&mut w);
        if !r.phases.contains(&PhaseId::MonthlyClose) {
            continue;
        }
        closes += 1;
        let i = w.industries();
        let weights: u64 = i.basket_share_ppm.iter().map(|&s| u64::from(s)).sum();
        assert_eq!(weights, 1_000_000, "{:?}: the weights sum to 1", r.date);
        let mean: i128 = i
            .basket_share_ppm
            .iter()
            .zip(&i.consumer_price_ppm)
            .map(|(&s, &p)| i128::from(s) * i128::from(p))
            .sum();
        let mean = i64::try_from(mean / 1_000_000).expect("an index");
        let cpi = i64::from(w.months().last().expect("closed").cpi_ppm);
        assert!(
            (cpi - mean).abs() <= 1,
            "{:?}: CPI {cpi} against the weighted mean {mean}",
            r.date
        );
    }
    assert_eq!(closes, 13);
    assert_eq!(month_stats(&w, 0).month.0, 0);
}

/// **AC-PRICE-05** `[sim]` A 10% rise in every wage raises prices over the
/// following months, more and sooner in industries with a higher wage share
/// of cost, and by less than 10% in total while imports cost the same.
///
/// The rise is on day 0, after the starting markups are set. "More and
/// sooner" is compared among manufacturing industries (table industries C
/// except C19, refined fuels), which share one price flexibility: the third
/// with the highest wage share against the third with the lowest, at the
/// close of January and of April.
#[test]
fn ac_price_05() {
    let mut base = world(1000);
    let mut rise = world_with(
        1000,
        vec![StartEdit::ScaleWages {
            factor_ppm: 1_100_000,
        }],
    );
    let ids = base.industries().id.clone();
    let mut manufacturing: Vec<(i64, usize)> = ids
        .iter()
        .enumerate()
        .filter_map(|(j, id)| {
            let code = table_code(id);
            (code.starts_with('C') && code != "C19").then(|| (wage_share_ppm(&code), j))
        })
        .collect();
    manufacturing.sort();
    let third = manufacturing.len() / 3;
    assert!(third >= 3, "enough manufacturing industries");
    let low: Vec<usize> = manufacturing[..third].iter().map(|&(_, j)| j).collect();
    let high: Vec<usize> = manufacturing[manufacturing.len() - third..]
        .iter()
        .map(|&(_, j)| j)
        .collect();
    assert!(manufacturing[third - 1].0 < manufacturing[manufacturing.len() - third].0);
    let mut cpi_ratio = Vec::new();
    for m in 0..12u32 {
        while base.months().len() <= m as usize {
            step(&mut base);
        }
        while rise.months().len() <= m as usize {
            step(&mut rise);
        }
        let (b, r) = (month_stats(&base, m), month_stats(&rise, m));
        cpi_ratio.push(ppm(i128::from(r.cpi_ppm), i128::from(b.cpi_ppm)));
        if m == 1 || m == 4 {
            let (h, l) = (
                mean_ratio(&base, &rise, &high),
                mean_ratio(&base, &rise, &low),
            );
            assert!(
                h > l,
                "month {m}: prices rose {h} ppm where wages weigh most, {l} where least"
            );
        }
    }
    assert!(
        cpi_ratio[4] > cpi_ratio[1] && cpi_ratio[1] > 1_000_000,
        "prices rise over the months: {cpi_ratio:?}"
    );
    assert!(
        cpi_ratio.iter().all(|&c| c < 1_100_000),
        "less than 10% in total: {cpi_ratio:?}"
    );
}

/// **AC-PRICE-07** `[sim]` When an industry's inventories fall below target
/// its price rises relative to the others; when they pile up it falls.
///
/// The industry with the largest starting stock starts at half its stock in
/// one world and at twice in another; after thirty days its price relative to
/// the mean of the others is compared with the unchanged world's.
#[test]
fn ac_price_07() {
    let mut base = world(1000);
    let f = base.firm_units();
    let industries = base.industries().id.len();
    let stock = |j: usize| -> i64 {
        f.industry
            .iter()
            .zip(&f.inventory)
            .filter(|&(&k, _)| usize::from(k) == j)
            .map(|(_, &v)| v)
            .sum()
    };
    let x = (0..industries)
        .max_by_key(|&j| (stock(j), std::cmp::Reverse(j)))
        .expect("industries");
    assert!(stock(x) > 0);
    let mut low = world_with(
        1000,
        vec![StartEdit::ScaleInventory {
            industry: x,
            factor_ppm: 500_000,
        }],
    );
    let mut high = world_with(
        1000,
        vec![StartEdit::ScaleInventory {
            industry: x,
            factor_ppm: 2_000_000,
        }],
    );
    for w in [&mut base, &mut low, &mut high] {
        for _ in 0..30 {
            step(w);
        }
    }
    let (rb, rl, rh) = (
        relative_price(&base, x),
        relative_price(&low, x),
        relative_price(&high, x),
    );
    assert!(rl > rb, "stock below target: {rl} ppm against {rb}");
    assert!(rh < rb, "stock piled up: {rh} ppm against {rb}");
}

/// **AC-PRICE-08** `[sim]` Quiet baseline: on the Romania scenario with no
/// change of policy, with the target as starting expectation, twelve-month
/// inflation over 120 months stays within 3 points of the target.
///
/// Twelve-month inflation is read from month 12 on, the first close with a
/// CPI twelve months back.
#[test]
fn ac_price_08() {
    let mut w = world(1000);
    let target = i64::from(w.inflation_target_ppm());
    while w.months().len() < 120 {
        step(&mut w);
    }
    for m in &w.months()[12..] {
        assert!(
            (i64::from(m.inflation_ppm) - target).abs() <= 30_000,
            "{:?}: twelve-month inflation {} ppm, target {target}",
            m.month,
            m.inflation_ppm
        );
    }
}
