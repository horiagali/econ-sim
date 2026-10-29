//! Acceptance tests for the `[unit]` criteria of society/households-lifecycle:
//! AC-HHL-01 to 04, the operations on weighted records (split, merge, move
//! out, make collective) as pure functions on small tables. Written in a
//! test-authoring session from the spec and its API sketch, before the code
//! existed (ADR-0014).
//!
//! Balances are per real household; "Σ weight × balance" is over live
//! records only (a tombstone counts nowhere), and so are the weighted
//! persons (live persons of live records).

use std::collections::{BTreeMap, BTreeSet};

use econ_mech_demography::records::{
    CASH, CONSUMER_DEBT, DEPOSITS, DWELLING, HouseholdTable, MORTGAGE, OWNER, PersonTable, TENANT,
    Tables, leave_home_amount, make_collective, merge_singles, move_out, split, unite,
};
use econ_popgen::{Activity, EduLevel, NO_EMPLOYER, Role, Sex};
use econ_rng::KeyedRng;
use econ_types::{Bani, Day};

const DAY: Day = Day(59);

/// One household of the test tables.
struct Hh {
    key: u64,
    weight: u32,
    county: u8,
    locality: u8,
    tenure: u8,
    /// Cash, deposits, mortgage, consumer debt, dwelling.
    balances: [i64; 5],
    /// (key, age in years, sex, role, activity, employer).
    members: Vec<(u64, u16, Sex, Role, Activity, u16)>,
}

/// Five households: a family of four (0), a woman alone (1), a man alone of
/// another weight who owns his home (2), a man alone of the woman's weight
/// (3), three children without an adult (4).
fn households() -> Vec<Hh> {
    use Activity::{Child, Employed, Pupil, Retired, Student};
    use Role::{Child as Kid, Head, Partner};
    use Sex::{F, M};
    vec![
        Hh {
            key: 100,
            weight: 1_000,
            county: 5,
            locality: 3,
            tenure: OWNER,
            balances: [10_001, 50_003, 2_000_000, 300_000, 30_000_000],
            members: vec![
                (1, 50, M, Head, Employed, 3),
                (2, 48, F, Partner, Retired, NO_EMPLOYER),
                (3, 20, F, Kid, Employed, 3),
                (4, 16, M, Kid, Pupil, NO_EMPLOYER),
            ],
        },
        Hh {
            key: 101,
            weight: 1_000,
            county: 5,
            locality: 3,
            tenure: TENANT,
            balances: [5_000, 7_000, 0, 1_000, 0],
            members: vec![(5, 30, F, Head, Employed, 7)],
        },
        Hh {
            key: 102,
            weight: 1_003,
            county: 5,
            locality: 1,
            tenure: OWNER,
            balances: [2_000, 1_000, 500_000, 0, 20_000_000],
            members: vec![(6, 32, M, Head, Employed, 3)],
        },
        Hh {
            key: 103,
            weight: 1_000,
            county: 5,
            locality: 3,
            tenure: TENANT,
            balances: [300, 900, 0, 0, 0],
            members: vec![(7, 33, M, Head, Student, NO_EMPLOYER)],
        },
        Hh {
            key: 104,
            weight: 997,
            county: 9,
            locality: 2,
            tenure: OWNER,
            balances: [1_000, 1_001, 0, 0, 50_000],
            members: vec![
                (30, 5, F, Head, Child, NO_EMPLOYER),
                (10, 10, M, Kid, Pupil, NO_EMPLOYER),
                (20, 16, F, Kid, Pupil, NO_EMPLOYER),
            ],
        },
    ]
}

fn tables() -> Tables {
    let mut h = HouseholdTable {
        balances: vec![Vec::new(); 5],
        ..HouseholdTable::default()
    };
    let mut p = PersonTable::default();
    for (i, x) in households().into_iter().enumerate() {
        h.hh_key.push(x.key);
        h.weight.push(x.weight);
        h.county.push(x.county);
        h.region.push(x.county / 4);
        h.locality.push(x.locality);
        h.collective.push(false);
        h.tenure.push(x.tenure);
        h.alive.push(true);
        h.stagger_day.push(u8::try_from(3 + i).expect("a day"));
        h.last_decision.push(Some(Day(40)));
        h.income_to_date.push(Bani(1_000));
        h.spending_to_date.push(Bani(900));
        h.moved_to_date.push(Bani::ZERO);
        for (col, &v) in x.balances.iter().enumerate() {
            h.balances[col].push(Bani(v));
        }
        for (key, years, sex, role, activity, employer) in x.members {
            p.person_key.push(key);
            p.household.push(u32::try_from(i).expect("an index"));
            p.alive.push(true);
            p.age.push(12 * years + 5);
            p.sex.push(sex);
            p.role.push(role);
            p.activity.push(activity);
            p.edu_level.push(if years >= 20 {
                EduLevel::Isced3
            } else {
                EduLevel::Isced0To2
            });
            p.employer.push(employer);
            p.origin_household
                .push(if key == 6 { Some(77) } else { None });
        }
    }
    Tables {
        households: h,
        persons: p,
        next_hh_key: 200,
        next_person_key: 1_000,
        rng: KeyedRng::new(42),
    }
}

fn idx(i: u32) -> usize {
    usize::try_from(i).expect("an index")
}

/// Live persons of a household.
fn members(t: &Tables, hh: u32) -> Vec<usize> {
    (0..t.persons.person_key.len())
        .filter(|&p| t.persons.alive[p] && t.persons.household[p] == hh)
        .collect()
}

/// Live households.
fn live_households(t: &Tables) -> usize {
    t.households.alive.iter().filter(|&&a| a).count()
}

/// Live persons of live households.
fn live_persons(t: &Tables) -> Vec<usize> {
    (0..t.persons.person_key.len())
        .filter(|&p| t.persons.alive[p] && t.households.alive[idx(t.persons.household[p])])
        .collect()
}

/// Σ weight × balance of every column, over live records.
fn totals(t: &Tables) -> Vec<i128> {
    let h = &t.households;
    h.balances
        .iter()
        .map(|col| {
            (0..h.hh_key.len())
                .filter(|&i| h.alive[i])
                .map(|i| i128::from(h.weight[i]) * i128::from(col[i].get()))
                .sum()
        })
        .collect()
}

/// The weighted live persons by (age, sex, activity).
fn weighted_persons(t: &Tables) -> BTreeMap<(u16, u8, u8), u64> {
    let mut m = BTreeMap::new();
    for p in live_persons(t) {
        let w = u64::from(t.households.weight[idx(t.persons.household[p])]);
        let k = (
            t.persons.age[p],
            t.persons.sex[p] as u8,
            t.persons.activity[p] as u8,
        );
        *m.entry(k).or_insert(0) += w;
    }
    m
}

/// Each employer's weighted staff.
fn staff(t: &Tables) -> BTreeMap<u16, u64> {
    let mut m = BTreeMap::new();
    for p in live_persons(t) {
        let e = t.persons.employer[p];
        if e != NO_EMPLOYER {
            *m.entry(e).or_insert(0) += u64::from(t.households.weight[idx(t.persons.household[p])]);
        }
    }
    m
}

/// A person's attributes, for comparing copies.
fn attrs(t: &Tables, p: usize) -> (u16, u8, u8, u8, u8, u16, Option<u64>, bool) {
    let q = &t.persons;
    (
        q.age[p],
        q.sex[p] as u8,
        q.role[p] as u8,
        q.activity[p] as u8,
        q.edu_level[p] as u8,
        q.employer[p],
        q.origin_household[p],
        q.alive[p],
    )
}

fn person(t: &Tables, key: u64) -> usize {
    t.persons
        .person_key
        .iter()
        .position(|&k| k == key)
        .unwrap_or_else(|| panic!("no person {key}"))
}

fn balance(t: &Tables, col: usize, hh: usize) -> i64 {
    t.households.balances[col][hh].get()
}

/// The keys in use, persons and households.
fn keys(t: &Tables) -> (BTreeSet<u64>, BTreeSet<u64>) {
    (
        t.persons.person_key.iter().copied().collect(),
        t.households.hh_key.iter().copied().collect(),
    )
}

/// **AC-HHL-01** `[unit]` Split: a record of weight $w$ split into $w-k$ and $k$
/// has every member copied with new keys and identical attributes; Σ weight ×
/// balance of every column and the weighted persons by age, sex and activity
/// are unchanged exactly; an employer's weighted staff is unchanged.
///
/// Also checked, from the spec's table of operations: every balance is copied
/// per household, the copy keeps the county, region, locality and tenure,
/// and gets a new `hh_key` and a stagger day from 1 to 28 (a new household).
#[test]
fn ac_hhl_01() {
    let mut t = tables();
    let (old_persons, old_households) = keys(&t);
    let (bal, people, jobs) = (totals(&t), weighted_persons(&t), staff(&t));
    let before: Vec<_> = members(&t, 0).iter().map(|&p| attrs(&t, p)).collect();
    let c = split(&mut t, 0, 400, DAY);
    let ci = idx(c);
    assert_ne!(ci, 0, "the copy is a new record");
    assert_eq!(t.households.weight[0], 600, "the original keeps w − k");
    assert_eq!(t.households.weight[ci], 400, "the copy has k");
    assert!(t.households.alive[ci]);
    let copies = members(&t, c);
    assert_eq!(copies.len(), before.len(), "every member is copied");
    let mut got: Vec<_> = copies.iter().map(|&p| attrs(&t, p)).collect();
    let mut want = before.clone();
    got.sort_unstable();
    want.sort_unstable();
    assert_eq!(got, want, "the copies' attributes are identical");
    let after_orig: Vec<_> = members(&t, 0).iter().map(|&p| attrs(&t, p)).collect();
    assert_eq!(after_orig, before, "the original keeps its members");
    let mut new_keys = BTreeSet::new();
    for &p in &copies {
        let k = t.persons.person_key[p];
        assert!(!old_persons.contains(&k), "copy {p} reuses key {k}");
        assert!(new_keys.insert(k), "two copies share key {k}");
    }
    assert!(
        !old_households.contains(&t.households.hh_key[ci]),
        "a new hh_key"
    );
    assert!(
        (1..=28).contains(&t.households.stagger_day[ci]),
        "a stagger day from 1 to 28"
    );
    for col in [CASH, DEPOSITS, MORTGAGE, CONSUMER_DEBT, DWELLING] {
        assert_eq!(
            balance(&t, col, ci),
            balance(&t, col, 0),
            "column {col} copied per household"
        );
    }
    let h = &t.households;
    assert_eq!(
        (
            h.county[ci],
            h.region[ci],
            h.locality[ci],
            h.tenure[ci],
            h.collective[ci]
        ),
        (
            h.county[0],
            h.region[0],
            h.locality[0],
            h.tenure[0],
            h.collective[0]
        )
    );
    assert_eq!(totals(&t), bal, "Σ weight × balance of every column");
    assert_eq!(
        weighted_persons(&t),
        people,
        "weighted persons by age, sex and activity"
    );
    assert_eq!(staff(&t), jobs, "employers' weighted staff");
}

/// **AC-HHL-02** `[unit]` Merge: two one-person records of equal weight become
/// one record with both persons and summed balances, totals unchanged; with
/// unequal weights the larger is split first and the leftover keeps its
/// person and balances; records change by −1 household (equal) or 0
/// households and +1 person (unequal).
///
/// Readings: the equal case is `merge_singles`, the unequal one `unite`
/// (split, then merge: the union's merge), tried with either record as the
/// one kept; "records" are live households and live persons.
#[test]
fn ac_hhl_02() {
    // Equal weights: the woman of record 1 and the man of record 3.
    let mut t = tables();
    let (bal, people) = (totals(&t), weighted_persons(&t));
    let (n_h, n_p) = (live_households(&t), live_persons(&t).len());
    let sums: Vec<i64> = (0..5)
        .map(|c| balance(&t, c, 1) + balance(&t, c, 3))
        .collect();
    merge_singles(&mut t, 1, 3).expect("two one-person records of one weight merge");
    assert_eq!(live_households(&t), n_h - 1, "one household fewer");
    assert_eq!(live_persons(&t).len(), n_p, "no person added or lost");
    let both: BTreeSet<u64> = members(&t, 1)
        .iter()
        .map(|&p| t.persons.person_key[p])
        .collect();
    assert_eq!(
        both,
        BTreeSet::from([5, 7]),
        "both persons in the record kept"
    );
    assert!(!t.households.alive[3], "the other record ended");
    for (c, &s) in sums.iter().enumerate() {
        assert_eq!(
            balance(&t, c, 1),
            s,
            "column {c}: balances added per household"
        );
    }
    assert_eq!(totals(&t), bal, "totals unchanged");
    assert_eq!(weighted_persons(&t), people);

    // Unequal weights: the woman of record 1 (1,000) and the man of record 2
    // (1,003), either kept.
    for (keep, other) in [(1u32, 2u32), (2, 1)] {
        let mut t = tables();
        let (bal, people) = (totals(&t), weighted_persons(&t));
        let (n_h, n_p) = (live_households(&t), live_persons(&t).len());
        let man_attrs = attrs(&t, person(&t, 6));
        let man_bal: Vec<i64> = (0..5).map(|c| balance(&t, c, 2)).collect();
        let sums: Vec<i64> = (0..5)
            .map(|c| balance(&t, c, 1) + balance(&t, c, 2))
            .collect();
        let m = unite(&mut t, keep, other, DAY).expect("a union of unequal weights");
        let mi = idx(m);
        assert_eq!(live_households(&t), n_h, "keep {keep}: as many households");
        assert_eq!(
            live_persons(&t).len(),
            n_p + 1,
            "keep {keep}: one person more (the copy)"
        );
        assert_eq!(
            t.households.weight[mi], 1_000,
            "keep {keep}: the merged record has the smaller weight"
        );
        let inside = members(&t, m);
        assert_eq!(
            inside.len(),
            2,
            "keep {keep}: both persons in the merged record"
        );
        for (c, &s) in sums.iter().enumerate() {
            assert_eq!(
                balance(&t, c, mi),
                s,
                "keep {keep}, column {c}: summed balances"
            );
        }
        let leftover: Vec<usize> = (0..t.households.hh_key.len())
            .filter(|&h| t.households.alive[h] && t.households.weight[h] == 3)
            .collect();
        assert_eq!(leftover.len(), 1, "keep {keep}: one leftover of weight 3");
        let l = leftover[0];
        let left_members = members(&t, u32::try_from(l).expect("an index"));
        assert_eq!(
            left_members.len(),
            1,
            "keep {keep}: the leftover keeps its person"
        );
        assert_eq!(
            attrs(&t, left_members[0]),
            man_attrs,
            "keep {keep}: the leftover's person"
        );
        let left_bal: Vec<i64> = (0..5).map(|c| balance(&t, c, l)).collect();
        assert_eq!(
            left_bal, man_bal,
            "keep {keep}: the leftover keeps its balances"
        );
        assert_eq!(totals(&t), bal, "keep {keep}: totals unchanged");
        assert_eq!(
            weighted_persons(&t),
            people,
            "keep {keep}: weighted persons unchanged"
        );
    }
}

/// **AC-HHL-03** `[unit]` Move out: the new record has the old one's weight,
/// county and locality, tenure tenant, no dwelling and no debt; the moved
/// amount is the rule's, rounded down; Σ weight × balance is unchanged;
/// `origin_household` is the old record's key.
///
/// Readings: the rule is leaving home's, $a$ = `leave_home_share` (0.25) ×
/// the per-household cash (and, separately, deposits) ÷ the members aged 18
/// or more, rounded down, nothing when the balance is zero or negative (spec,
/// edge cases). The leaver is the new record's head, and `hh_moved_to_date`
/// records the move on both sides.
#[test]
fn ac_hhl_03() {
    // The rule, rounded down.
    assert_eq!(
        leave_home_amount(Bani(10_001), 3, 250_000),
        Bani(833),
        "833.42 → 833"
    );
    assert_eq!(
        leave_home_amount(Bani(50_003), 3, 250_000),
        Bani(4_166),
        "4,166.92 → 4,166"
    );
    assert_eq!(leave_home_amount(Bani(999), 3, 250_000), Bani(83));
    assert_eq!(leave_home_amount(Bani(1_000), 1, 250_000), Bani(250));
    assert_eq!(
        leave_home_amount(Bani::ZERO, 2, 250_000),
        Bani::ZERO,
        "nothing to take"
    );
    assert_eq!(
        leave_home_amount(Bani(-5_000), 2, 250_000),
        Bani::ZERO,
        "negative money: nothing"
    );

    // The daughter of 20 leaves her parents (three adults in the household).
    let mut t = tables();
    let (_, old_households) = keys(&t);
    let (bal, people) = (totals(&t), weighted_persons(&t));
    let d = person(&t, 3);
    let adults = members(&t, 0)
        .iter()
        .filter(|&&p| t.persons.age[p] >= 18 * 12)
        .count();
    assert_eq!(adults, 3);
    let a_cash = leave_home_amount(t.households.balances[CASH][0], 3, 250_000);
    let a_dep = leave_home_amount(t.households.balances[DEPOSITS][0], 3, 250_000);
    let before: Vec<i64> = (0..5).map(|c| balance(&t, c, 0)).collect();
    let n = move_out(
        &mut t,
        u32::try_from(d).expect("an index"),
        a_cash,
        a_dep,
        DAY,
    );
    let ni = idx(n);
    assert_ne!(ni, 0);
    assert_eq!(person(&t, 3), d, "the person keeps her record");
    assert_eq!(t.persons.household[d], n, "she lives in the new record");
    assert_eq!(members(&t, n), vec![d], "alone");
    assert_eq!(t.persons.role[d], Role::Head, "its head");
    assert_eq!(
        t.persons.origin_household[d],
        Some(100),
        "the old record's key"
    );
    assert!(
        !old_households.contains(&t.households.hh_key[ni]),
        "a new hh_key"
    );
    let h = &t.households;
    assert_eq!(h.weight[ni], h.weight[0], "the old record's weight");
    assert_eq!(h.county[ni], h.county[0], "its county");
    assert_eq!(h.region[ni], h.region[0], "its region");
    assert_eq!(h.locality[ni], h.locality[0], "its locality");
    assert_eq!(h.tenure[ni], TENANT, "a tenant");
    assert!(!h.collective[ni] && h.alive[ni]);
    assert!((1..=28).contains(&h.stagger_day[ni]));
    assert_eq!(balance(&t, DWELLING, ni), 0, "no dwelling");
    assert_eq!(balance(&t, MORTGAGE, ni), 0, "no mortgage");
    assert_eq!(balance(&t, CONSUMER_DEBT, ni), 0, "no consumer debt");
    assert_eq!(balance(&t, CASH, ni), 833, "the moved cash");
    assert_eq!(balance(&t, DEPOSITS, ni), 4_166, "the moved deposits");
    assert_eq!(balance(&t, CASH, 0), before[CASH] - 833);
    assert_eq!(balance(&t, DEPOSITS, 0), before[DEPOSITS] - 4_166);
    for c in [MORTGAGE, CONSUMER_DEBT, DWELLING] {
        assert_eq!(
            balance(&t, c, 0),
            before[c],
            "column {c} stays with the parents"
        );
    }
    assert_eq!(
        t.households.moved_to_date[ni],
        Bani(4_999),
        "what the new record received"
    );
    assert_eq!(
        t.households.moved_to_date[0],
        Bani(-4_999),
        "what the old record gave"
    );
    assert_eq!(totals(&t), bal, "Σ weight × balance of every column");
    assert_eq!(weighted_persons(&t), people);
}

/// **AC-HHL-04** `[unit]` Make collective: a household with no member aged 18
/// or more becomes one collective record per member, of the same weight, with
/// the money split exactly (the remainder to the first child).
///
/// Readings: "the first child" is the first by `person_key` (spec, step 1);
/// cash and deposits are split separately, each floor-divided with the
/// remainder to that child, who also gets the dwelling (spec, step 1).
#[test]
fn ac_hhl_04() {
    let mut t = tables();
    let bal = totals(&t);
    let people = weighted_persons(&t);
    let n_h = live_households(&t);
    let records = make_collective(&mut t, 4, DAY);
    assert_eq!(records.len(), 3, "one record per child");
    let distinct: BTreeSet<u32> = records.iter().copied().collect();
    assert_eq!(distinct.len(), 3, "three distinct records");
    assert_eq!(live_households(&t), n_h - 1 + 3);
    let mut by_key = BTreeMap::new();
    for &r in &records {
        let ri = idx(r);
        let h = &t.households;
        assert!(
            h.alive[ri] && h.collective[ri],
            "record {r}: a live collective record"
        );
        assert_eq!(h.weight[ri], 997, "record {r}: the household's weight");
        let m = members(&t, r);
        assert_eq!(m.len(), 1, "record {r}: one member");
        by_key.insert(t.persons.person_key[m[0]], ri);
    }
    assert_eq!(
        by_key.keys().copied().collect::<Vec<_>>(),
        vec![10, 20, 30],
        "each child"
    );
    // The first child by key (10) gets the remainders and the dwelling.
    let cash: Vec<i64> = by_key.values().map(|&r| balance(&t, CASH, r)).collect();
    let deposits: Vec<i64> = by_key.values().map(|&r| balance(&t, DEPOSITS, r)).collect();
    let dwelling: Vec<i64> = by_key.values().map(|&r| balance(&t, DWELLING, r)).collect();
    assert_eq!(
        cash,
        vec![334, 333, 333],
        "1,000 bani: 333 each, the remainder to the first"
    );
    assert_eq!(
        deposits,
        vec![335, 333, 333],
        "1,001 bani: 333 each, the remainder to the first"
    );
    assert_eq!(
        dwelling,
        vec![50_000, 0, 0],
        "the dwelling to the first child"
    );
    assert_eq!(totals(&t), bal, "the money split exactly");
    assert_eq!(weighted_persons(&t), people);
}
