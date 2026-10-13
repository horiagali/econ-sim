//! Acceptance tests for AC-POPH2-* (spec: society/population-wealth).
//! Written in a test-authoring session from the spec, the API skeleton and the
//! Python reference, before the Rust stage existed (ADR-0014).
//!
//! "The fixtures" are the normalised margin files committed with the pipeline:
//! the Census 2021 margins of the earlier stages, the earnings and pension
//! margins, and the household wealth margins.

// Column tables are indexed in parallel; index loops are the clear way to read them.
#![allow(clippy::needless_range_loop)]

use std::sync::OnceLock;

use econ_popgen::{
    BorrowerRule, HouseholdHousing, HouseholdWealth, PersonPensions, PersonWages, Population, Role,
    WealthError, WealthMargins, WealthParams, assign_wealth,
};

use super::support::{
    DIFF_SCALES, HOUSING_GOLDEN, OTHER_SEED, SCALES, SEED, STATE_GOLDEN, assert_golden,
    assert_same_column, column, golden_hash, hex,
};
use super::support_fj::{
    BANI_PER_LEU, Cache, PENSIONS_GOLDEN, U, WAGES_GOLDEN, WEALTH_FIXTURE, WEALTH_GOLDEN,
    assert_total, fixture, housing, pensions, pop, ratio, reference_all, reference_hash, shorter,
    slot, table, wages, wealth_margins, weighted_sum,
};

/// `hh_tenure` of an owner household.
const OWNER: u8 = 0;

/// Stage H on the fixture population at a cached scale, after every earlier stage.
fn wealth(scale: u32) -> &'static HouseholdWealth {
    static WEALTH: Cache<HouseholdWealth> = [const { OnceLock::new() }; 4];
    WEALTH[slot(scale)].get_or_init(|| {
        assign_wealth(
            pop(scale),
            housing(scale),
            Some(wages(scale)),
            Some(pensions(scale)),
            wealth_margins(),
            &WealthParams::new(SEED),
        )
        .expect("fixture population gets wealth")
    })
}

/// The five columns with the name of their total in the margin file.
fn columns(wealth: &HouseholdWealth) -> [(&'static str, &[u64]); 5] {
    [
        ("cash", &wealth.hh_cash),
        ("deposits", &wealth.hh_deposits),
        ("dwellings", &wealth.hh_dwelling_value),
        ("housing_loans", &wealth.hh_mortgage),
        ("consumer_loans", &wealth.hh_consumer_debt),
    ]
}

/// The five totals of the margin file, in lei, in the order of [`columns`].
fn targets(m: &WealthMargins) -> [u64; 5] {
    [
        m.cash,
        m.deposits,
        m.dwellings,
        m.housing_loans,
        m.consumer_loans,
    ]
}

/// AC-POPH2-03: each weighted total equals its target.
fn assert_totals(what: &str, m: &WealthMargins, pop: &Population, wealth: &HouseholdWealth) {
    let weights: Vec<u64> = pop
        .households
        .hh_weight
        .iter()
        .map(|&w| u64::from(w))
        .collect();
    for ((name, column), target) in columns(wealth).into_iter().zip(targets(m)) {
        assert_total(
            &format!("{what} {name}"),
            weighted_sum(&weights, column),
            u128::from(target) * u128::from(BANI_PER_LEU),
        );
    }
}

/// What the criteria need to know of the households.
struct Households {
    /// Weight of every record.
    weight: Vec<u64>,
    /// The private households.
    private: Vec<usize>,
    /// Their weight.
    private_weight: u128,
    /// Wages and pensions of the members, in bani.
    income: Vec<u64>,
    /// Age of the head, in years.
    head_years: Vec<u16>,
}

fn households(
    pop: &Population,
    wages: Option<&PersonWages>,
    pensions: Option<&PersonPensions>,
) -> Households {
    let hh = &pop.households;
    let n = hh.hh_weight.len();
    let (mut income, mut head_years) = (vec![0u64; n], vec![0u16; n]);
    for i in 0..pop.persons.household_id.len() {
        let h = pop.persons.household_id[i] as usize;
        income[h] += wages.map_or(0, |w| w.wage[i]) + pensions.map_or(0, |p| p.pension[i]);
        if pop.persons.role[i] == Role::Head {
            head_years[h] = pop.persons.age[i] / 12;
        }
    }
    let weight: Vec<u64> = hh.hh_weight.iter().map(|&w| u64::from(w)).collect();
    let private: Vec<usize> = (0..n).filter(|&h| !hh.hh_collective[h]).collect();
    Households {
        private_weight: private.iter().map(|&h| u128::from(weight[h])).sum(),
        weight,
        private,
        income,
        head_years,
    }
}

/// Cash and deposits of a household.
fn money(wealth: &HouseholdWealth, h: usize) -> u64 {
    wealth.hh_cash[h] + wealth.hh_deposits[h]
}

/// Money plus home less loans.
fn net_wealth(wealth: &HouseholdWealth, h: usize) -> i128 {
    i128::from(money(wealth, h)) + i128::from(wealth.hh_dwelling_value[h])
        - i128::from(wealth.hh_mortgage[h])
        - i128::from(wealth.hh_consumer_debt[h])
}

/// Share of the amount held by each fifth of the weight; `ordered` are
/// (weight, amount) from the lowest to the highest rank. A record that
/// straddles two fifths is divided between them.
fn fifth_shares(ordered: &[(u64, i128)]) -> [f64; 5] {
    let total_w: u128 = ordered.iter().map(|&(w, _)| u128::from(w)).sum();
    let total: i128 = ordered.iter().map(|&(w, a)| i128::from(w) * a).sum();
    assert!(total > 0, "something to share");
    let (mut held, mut seen) = ([0i128; 5], 0u128);
    for &(w, amount) in ordered {
        let mut left = u128::from(w);
        while left > 0 {
            let fifth = usize::try_from((seen * 5 / total_w).min(4)).expect("a fifth");
            let room = if fifth < 4 {
                (fifth as u128 + 1) * total_w / 5 - seen
            } else {
                left
            };
            let part = left.min(room.max(1));
            held[fifth] += i128::try_from(part).expect("a weight") * amount;
            seen += part;
            left -= part;
        }
    }
    held.map(|h| h as f64 / total as f64)
}

/// Net wealth by fifth of households, ranked by `key` (ties by household index).
fn net_wealth_by_fifth<K: Ord>(
    hh: &Households,
    wealth: &HouseholdWealth,
    key: impl Fn(usize) -> K,
) -> [f64; 5] {
    let mut ranked: Vec<(K, usize)> = hh.private.iter().map(|&h| (key(h), h)).collect();
    ranked.sort();
    let ordered: Vec<(u64, i128)> = ranked
        .iter()
        .map(|&(_, h)| (hh.weight[h], net_wealth(wealth, h)))
        .collect();
    fifth_shares(&ordered)
}

/// AC-POPH2-01 [unit] Structure, at each of the three scales: five values per household; a record that is not a private household has five zeros; every private household has money; `hh_dwelling_value` is above zero exactly for owners; only owners have `hh_mortgage`.
#[test]
fn ac_poph2_01() {
    for scale in SCALES {
        let (pop, housing, wealth) = (pop(scale), housing(scale), wealth(scale));
        let n = pop.households.hh_weight.len();
        for (name, column) in columns(wealth) {
            assert_eq!(
                column.len(),
                n,
                "1:{scale}: one value of {name} per household"
            );
        }
        for h in 0..n {
            let owner = housing.hh_tenure[h] == OWNER;
            if pop.households.hh_collective[h] {
                for (name, column) in columns(wealth) {
                    assert_eq!(column[h], 0, "1:{scale}: collective record {h} has {name}");
                }
                assert!(!owner, "1:{scale}: collective record {h} owns a home");
            } else {
                assert!(
                    money(wealth, h) > 0,
                    "1:{scale}: private household {h} has no money"
                );
            }
            assert_eq!(
                wealth.hh_dwelling_value[h] > 0,
                owner,
                "1:{scale}: household {h} with tenure {} has a dwelling worth {}",
                housing.hh_tenure[h],
                wealth.hh_dwelling_value[h]
            );
            assert!(
                owner || wealth.hh_mortgage[h] == 0,
                "1:{scale}: household {h} has a housing loan and does not own its home"
            );
        }
    }
}

/// AC-POPH2-02 [unit] The stage changes nothing before it: the state, housing, wage and pension hashes are those of their own golden tests.
#[test]
fn ac_poph2_02() {
    for scale in SCALES {
        let (pop, housing, wages, pensions) = (
            pop(scale).clone(),
            housing(scale).clone(),
            wages(scale).clone(),
            pensions(scale).clone(),
        );
        let before = (
            pop.clone(),
            housing.clone(),
            wages.clone(),
            pensions.clone(),
        );
        assign_wealth(
            &pop,
            &housing,
            Some(&wages),
            Some(&pensions),
            wealth_margins(),
            &WealthParams::new(SEED),
        )
        .expect("fixture population gets wealth");
        assert_eq!(pop, before.0, "1:{scale}: population changed");
        assert_eq!(housing, before.1, "1:{scale}: housing changed");
        assert_eq!(wages, before.2, "1:{scale}: wages changed");
        assert_eq!(pensions, before.3, "1:{scale}: pensions changed");
        for (what, hash, file) in [
            ("state", pop.state_hash(), STATE_GOLDEN),
            ("housing", housing.state_hash(), HOUSING_GOLDEN),
            ("wage", wages.state_hash(), WAGES_GOLDEN),
            ("pension", pensions.state_hash(), PENSIONS_GOLDEN),
        ] {
            assert_eq!(
                hex(hash),
                golden_hash(file, scale),
                "{what} hash at 1:{scale}"
            );
        }
    }
}

/// AC-POPH2-03 [unit] At each of the three scales each of the five weighted totals equals its target in the margin file within 0.001%.
#[test]
fn ac_poph2_03() {
    for scale in SCALES {
        assert_totals(
            &format!("1:{scale}"),
            wealth_margins(),
            pop(scale),
            wealth(scale),
        );
    }
}

/// AC-POPH2-04 [unit] At each of the three scales: the share of private households with a housing loan is within half a point of the rule's share, likewise for consumer loans; every borrower's head is in the rule's age range; the mean dwelling value rises from each locality class to the next, and the largest class's mean over the smallest's is within 0.3 of the ratio of their proportions.
#[test]
fn ac_poph2_04() {
    let m = wealth_margins();
    for scale in SCALES {
        let (pop, housing, wealth) = (pop(scale), housing(scale), wealth(scale));
        let hh = households(pop, None, None);
        let loans: [(&str, &BorrowerRule, &[u64]); 2] = [
            ("housing", &m.mortgage, &wealth.hh_mortgage),
            ("consumer", &m.consumer, &wealth.hh_consumer_debt),
        ];
        for (name, rule, column) in loans {
            let mut borrowers = 0u128;
            for &h in &hh.private {
                if column[h] == 0 {
                    continue;
                }
                borrowers += u128::from(hh.weight[h]);
                assert!(
                    (rule.age_from..rule.age_to).contains(&hh.head_years[h]),
                    "1:{scale}: household {h} has a {name} loan and a head aged {}",
                    hh.head_years[h]
                );
            }
            let share = ratio(borrowers, hh.private_weight);
            let want = f64::from(rule.share_ppm) / U as f64;
            assert!(
                (share - want).abs() <= 0.005,
                "1:{scale}: {share:.4} of private households have a {name} loan, the rule says {want:.4}"
            );
        }

        let classes = m.rel_locality.len();
        let (mut owners, mut value) = (vec![0u128; classes], vec![0u128; classes]);
        for &h in &hh.private {
            if wealth.hh_dwelling_value[h] > 0 {
                let class = usize::from(housing.hh_locality_size[h]);
                owners[class] += u128::from(hh.weight[h]);
                value[class] += u128::from(hh.weight[h]) * u128::from(wealth.hh_dwelling_value[h]);
            }
        }
        let means: Vec<f64> = (0..classes).map(|k| ratio(value[k], owners[k])).collect();
        for k in 1..classes {
            assert!(
                means[k] > means[k - 1],
                "1:{scale}: mean dwelling value {:.0} in locality class {k}, {:.0} in the class below",
                means[k],
                means[k - 1]
            );
        }
        let (got, want) = (
            means[classes - 1] / means[0],
            f64::from(m.rel_locality[classes - 1]) / f64::from(m.rel_locality[0]),
        );
        assert!(
            (got - want).abs() <= 0.3,
            "1:{scale}: the largest class's mean dwelling value is {got:.3} times the smallest's, the proportions say {want:.3}"
        );
    }
}

/// AC-POPH2-05 [unit] The shape, at each of the three scales: the top tenth of households by money holds between 60% and 75% of it; the richest fifth by net wealth (money plus home less loans) holds more than 45% and the poorest fifth less than 5%; net wealth by income fifth is within 6 points of the pattern in each fifth; fewer than 6% of households have negative net wealth.
#[test]
fn ac_poph2_05() {
    let pattern: Vec<f64> =
        table(&fixture(WEALTH_FIXTURE)["pattern"]["net_wealth_share_by_income_quintile"])
            .into_iter()
            .map(|share| share as f64 / U as f64)
            .collect();
    assert_eq!(pattern.len(), 5, "the pattern has five fifths");
    for scale in SCALES {
        let (pop, wealth) = (pop(scale), wealth(scale));
        let hh = households(pop, Some(wages(scale)), Some(pensions(scale)));

        // From the richest in money down, until a tenth of the households is reached.
        let mut by_money: Vec<(u64, usize)> =
            hh.private.iter().map(|&h| (money(wealth, h), h)).collect();
        by_money.sort_unstable_by(|a, b| b.cmp(a));
        let (mut top, mut all, mut seen) = (0u128, 0u128, 0u128);
        for &(amount, h) in &by_money {
            let held = u128::from(hh.weight[h]) * u128::from(amount);
            if seen * 10 < hh.private_weight {
                top += held;
                seen += u128::from(hh.weight[h]);
            }
            all += held;
        }
        let top_tenth = ratio(top, all);
        assert!(
            (0.60..=0.75).contains(&top_tenth),
            "1:{scale}: the top tenth by money holds {top_tenth:.4} of it"
        );

        let by_wealth = net_wealth_by_fifth(&hh, wealth, |h| net_wealth(wealth, h));
        assert!(
            by_wealth[4] > 0.45,
            "1:{scale}: the richest fifth holds {:.4} of net wealth",
            by_wealth[4]
        );
        assert!(
            by_wealth[0] < 0.05,
            "1:{scale}: the poorest fifth holds {:.4} of net wealth",
            by_wealth[0]
        );

        let by_income = net_wealth_by_fifth(&hh, wealth, |h| hh.income[h]);
        for fifth in 0..5 {
            assert!(
                (by_income[fifth] - pattern[fifth]).abs() <= 0.06,
                "1:{scale}: income fifth {fifth} holds {:.4} of net wealth, the pattern says {:.4}",
                by_income[fifth],
                pattern[fifth]
            );
        }

        let negative: u128 = hh
            .private
            .iter()
            .filter(|&&h| net_wealth(wealth, h) < 0)
            .map(|&h| u128::from(hh.weight[h]))
            .sum();
        let share = ratio(negative, hh.private_weight);
        assert!(
            share < 0.06,
            "1:{scale}: {share:.4} of households have negative net wealth"
        );
    }
}

/// AC-POPH2-06 [unit] Same inputs and seed give identical columns (equal wealth hash). A different seed changes who holds what; AC-POPH2-03 still holds. Without wages and pensions the stage runs and AC-POPH2-03 holds.
#[test]
fn ac_poph2_06() {
    let (m, scale) = (wealth_margins(), 1000);
    let (pop, housing, wages, pensions, first) = (
        pop(scale),
        housing(scale),
        Some(wages(scale)),
        Some(pensions(scale)),
        wealth(scale),
    );
    let again =
        assign_wealth(pop, housing, wages, pensions, m, &WealthParams::new(SEED)).expect("wealth");
    assert_eq!(&again, first);
    assert_eq!(again.state_hash(), first.state_hash());

    let other = assign_wealth(
        pop,
        housing,
        wages,
        pensions,
        m,
        &WealthParams::new(OTHER_SEED),
    )
    .expect("wealth");
    assert_ne!(other.state_hash(), first.state_hash());
    for ((name, one), (_, two)) in columns(&other).into_iter().zip(columns(first)) {
        assert_ne!(one, two, "{name} is the same with another seed");
    }
    assert_totals("another seed:", m, pop, &other);

    let without = assign_wealth(pop, housing, None, None, m, &WealthParams::new(SEED))
        .expect("wealth without wages and pensions");
    assert_totals("without wages and pensions:", m, pop, &without);
}

/// AC-POPH2-07 [golden] The wealth hash of the fixture population at each of the three scales equals the committed golden value.
#[test]
fn ac_poph2_07() {
    assert_golden(WEALTH_GOLDEN, "wealth hash", |scale| {
        wealth(scale).state_hash()
    });
}

/// AC-POPH2-08 [diff] On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns, household for household.
#[test]
fn ac_poph2_08() {
    for scale in DIFF_SCALES {
        let wealth = wealth(scale);
        let r = reference_all(scale);
        for (name, got) in [
            ("hh_cash", &wealth.hh_cash),
            ("hh_deposits", &wealth.hh_deposits),
            ("hh_dwelling_value", &wealth.hh_dwelling_value),
            ("hh_mortgage", &wealth.hh_mortgage),
            ("hh_consumer_debt", &wealth.hh_consumer_debt),
        ] {
            assert_same_column(scale, name, got, &column(r, name));
        }
        assert_eq!(
            hex(wealth.state_hash()),
            reference_hash(r, "wealth_hash"),
            "1:{scale} wealth hash"
        );
    }
}

/// AC-POPH2-09 [unit] Margins with no locality classes, a curve that does not end at rank 1, a proportion of zero, a share above 1, housing columns of a different length and wages of a different length are each rejected with a named error and produce no output.
#[test]
fn ac_poph2_09() {
    let m = wealth_margins();
    let (pop, housing, wages, pensions) = (pop(1000), housing(1000), wages(1000), pensions(1000));
    let params = WealthParams::new(SEED);
    let run =
        |m: &WealthMargins| assign_wealth(pop, housing, Some(wages), Some(pensions), m, &params);

    let mut classless = m.clone();
    classless.rel_locality.clear();
    assert_eq!(run(&classless), Err(WealthError::EmptyTable));

    let curves: [fn(&mut WealthMargins) -> &mut Vec<u32>; 3] = [
        |m| &mut m.liquid_curve.rank,
        |m| &mut m.dwelling_curve.rank,
        |m| &mut m.loan_curve.rank,
    ];
    for ranks in curves {
        let mut unfinished = m.clone();
        *ranks(&mut unfinished).last_mut().expect("knots") -= 1;
        assert_eq!(run(&unfinished), Err(WealthError::ShapeMismatch));
    }

    let mut zero = m.clone();
    zero.rel_locality[2] = 0;
    assert_eq!(run(&zero), Err(WealthError::ShapeMismatch));

    let over_one = u32::try_from(U).expect("a million") + 1;
    let mut everyone = m.clone();
    everyone.mortgage.share_ppm = over_one;
    assert_eq!(run(&everyone), Err(WealthError::ShapeMismatch));
    let mut everyone = m.clone();
    everyone.consumer.share_ppm = over_one;
    assert_eq!(run(&everyone), Err(WealthError::ShapeMismatch));

    let other = HouseholdHousing {
        hh_locality_size: shorter(&housing.hh_locality_size),
        hh_urban: shorter(&housing.hh_urban),
        hh_tenure: shorter(&housing.hh_tenure),
    };
    assert_eq!(
        assign_wealth(pop, &other, Some(wages), Some(pensions), m, &params),
        Err(WealthError::ShapeMismatch)
    );

    let other = PersonWages {
        wage: shorter(&wages.wage),
    };
    assert_eq!(
        assign_wealth(pop, housing, Some(&other), Some(pensions), m, &params),
        Err(WealthError::ShapeMismatch)
    );
}
