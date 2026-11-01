//! Acceptance tests for AC-GRP-01 to AC-GRP-06 (spec: game/group-queries,
//! draft of 2026-11-01), written by the test writer from the spec and its API
//! sketch before the code (ADR-0014); behind the feature `groups`, against
//! the test writer's API skeleton `econ_core::groups` (`Condition`,
//! `Filter`, `Attribute`, `GroupSummary`, `MIN_RECORDS`, `World::group`,
//! `group_breakdown`, `watch_group`, `watched`).
//!
//! "1:1000" is the Romania scenario at that scale, seed 42, under the law of
//! its folder (`law.json`: the 2021 law), every parameter at its spec
//! default. Month 0 is December 2021.
//!
//! Readings shared by the tests:
//! - A resident is a person alive, in a record of positive weight and not
//!   abroad; a person's weight is its household's.
//! - The money fields are `wages_gross`, `wages_net`, `pensions`,
//!   `unemployment_benefits`, `household_transfers`, `taxes_paid`, `money`
//!   and `debt`.
//! - "One summary per value of the attribute": a breakdown has a member for
//!   every value, empty ones included (two sexes, five edu levels, seven
//!   activities, five fifths, urban and rural, every region and county of
//!   the scenario, every catalogue industry and "none").

use std::collections::BTreeSet;
use std::sync::OnceLock;

use econ_core::groups::{Attribute, Condition, Filter, GroupSummary, MIN_RECORDS};
use econ_core::world::World;
use econ_popgen::{EduLevel, Sex};

use super::support::{params, start_state};
use super::support_m6::{
    NATIONAL, id, person_weights, residents_mask, run_closes, step_checked, world_2021,
    world_2021_default,
};

/// The world after 3 closes at 1:1000, built once.
fn three_closes() -> &'static World {
    static W: OnceLock<World> = OnceLock::new();
    W.get_or_init(|| {
        let mut w = world_2021_default(1000);
        run_closes(&mut w, 3);
        w
    })
}

/// The empty filter: every resident.
fn everyone() -> Filter {
    Filter::default()
}

/// A filter of these conditions.
fn filter(conditions: Vec<Condition>) -> Filter {
    Filter { conditions }
}

/// The money fields of a summary, by name, in bani.
fn money_fields(s: &GroupSummary) -> [(&'static str, i64); 8] {
    [
        ("wages_gross", s.wages_gross.get()),
        ("wages_net", s.wages_net.get()),
        ("pensions", s.pensions.get()),
        ("unemployment_benefits", s.unemployment_benefits.get()),
        ("household_transfers", s.household_transfers.get()),
        ("taxes_paid", s.taxes_paid.get()),
        ("money", s.money.get()),
        ("debt", s.debt.get()),
    ]
}

/// The summary of an empty group: every count and money field zero, the
/// sample flagged, no mean.
fn empty_summary() -> GroupSummary {
    GroupSummary {
        small_sample: true,
        ..GroupSummary::default()
    }
}

/// The national weighted sum of a per-person pay-slip column.
fn slips_sum(w: &World, column: &[econ_types::Bani]) -> i64 {
    let weights = person_weights(w);
    assert_eq!(
        column.len(),
        weights.len(),
        "the pay slips are aligned with the persons"
    );
    let total: i128 = weights
        .iter()
        .zip(column)
        .map(|(&wt, v)| i128::from(wt) * i128::from(v.get()))
        .sum();
    i64::try_from(total).expect("fits i64 bani")
}

/// The number of development regions and counties of the scenario.
fn regions_and_counties() -> (usize, usize) {
    let s = start_state(1000);
    let regions = s
        .hh_region
        .iter()
        .copied()
        .max()
        .map_or(0, |r| usize::from(r) + 1);
    let counties = s
        .hh_county
        .iter()
        .copied()
        .max()
        .map_or(0, |c| usize::from(c) + 1);
    (regions, counties)
}

/// Weighted residents whose age in whole years is in `[from, to)`, and their
/// person rows.
fn residents_aged(w: &World, from: u16, to: u16) -> (u64, u32) {
    let mask = residents_mask(w);
    let weights = person_weights(w);
    let ages = w.person_ages();
    let mut persons = 0u64;
    let mut rows = 0u32;
    for p in 0..mask.len() {
        let years = ages[p] / 12;
        if mask[p] && years >= from && years < to {
            persons += u64::from(weights[p]);
            rows += 1;
        }
    }
    (persons, rows)
}

// --- AC-GRP-01 ------------------------------------------------------------------

/// **AC-GRP-01** `[sim]` At 1:1000 under the 2021 law, after 3 closes: the
/// empty filter's `persons` equals `people.population` of the registry, its
/// `employed` and `unemployed` equal the month's `employed` and
/// `unemployed`, and its wages, pensions and benefits equal the pay slips'
/// weighted national sums of the month exactly.
///
/// Readings: `people.population` is the registry's latest national point
/// (month 2); the month's `employed` and `unemployed` are `months()[2]`'s.
/// The pay slips are those of the latest pay day (the close of month 2),
/// each person's amounts times its household's weight, over every person
/// row: `wages_gross` = Σ `gross_wage`, `wages_net` = Σ `net_wage`,
/// `pensions` = Σ `pension` (gross), `unemployment_benefits` = Σ
/// `unemployment_benefit`. Under this test's features no person row stops
/// being a resident between the pay day and the close (no People phase), so
/// "the group's persons" and "every row" are the same.
#[test]
fn ac_grp_01() {
    let w = three_closes();
    assert_eq!(w.months().len(), 3);
    let s = w.group(&everyone());
    let population = w
        .stats()
        .latest(id("people.population"), NATIONAL)
        .expect("people.population has a point");
    assert_eq!(
        i64::try_from(s.persons).expect("fits"),
        population,
        "persons against people.population"
    );
    let month = &w.months()[2];
    assert_eq!(s.employed, month.employed, "employed against the month's");
    assert_eq!(
        s.unemployed, month.unemployed,
        "unemployed against the month's"
    );
    let slips = w.pay_slips();
    assert_eq!(
        s.wages_gross.get(),
        slips_sum(w, &slips.gross_wage),
        "gross wages against the pay slips"
    );
    assert_eq!(
        s.wages_net.get(),
        slips_sum(w, &slips.net_wage),
        "net wages against the pay slips"
    );
    assert_eq!(
        s.pensions.get(),
        slips_sum(w, &slips.pension),
        "pensions against the pay slips"
    );
    assert_eq!(
        s.unemployment_benefits.get(),
        slips_sum(w, &slips.unemployment_benefit),
        "unemployment benefits against the pay slips"
    );
    assert!(
        s.persons > 0 && s.employed > 0,
        "a populated economy: {s:?}"
    );
}

// --- AC-GRP-02 ------------------------------------------------------------------

/// **AC-GRP-02** `[sim]` Partitions: breaking the empty filter down by
/// `region`, `county`, `sex`, `edu`, `activity`, `fifth`, `urban` and
/// `industry`, the members' `persons`, `employed`, `unemployed` and every
/// money field add up to the whole group's exactly; for `age` bands and
/// `sex` the money fields add up to within one ban per household of the
/// group.
///
/// Readings: AC-GRP-01's world (3 closes at 1:1000). `persons`, `employed`
/// and `unemployed` add up exactly for every attribute listed, age bands
/// included. The money fields add up exactly for `region`, `county`, `edu`,
/// `activity`, `fifth`, `urban` and `industry`; for `sex` and age bands,
/// to within the whole group's `households` (weighted households, as money
/// is weighted) in bani. Each breakdown has one member per value (see the
/// file's readings), the age bands at least two, and no value twice.
#[test]
fn ac_grp_02() {
    let w = three_closes();
    let whole = w.group(&everyone());
    let (regions, counties) = regions_and_counties();
    let industries = w.industries().id.len();
    let cases: [(Attribute, bool, Option<usize>); 9] = [
        (Attribute::Region, true, Some(regions)),
        (Attribute::County, true, Some(counties)),
        (Attribute::Sex, false, Some(2)),
        (Attribute::Edu, true, Some(5)),
        (Attribute::Activity, true, Some(7)),
        (Attribute::Fifth, true, Some(5)),
        (Attribute::Urban, true, Some(2)),
        (Attribute::Industry, true, Some(industries + 1)),
        (Attribute::AgeBand, false, None),
    ];
    for (by, money_exact, members) in cases {
        let parts = w.group_breakdown(&everyone(), by);
        match members {
            Some(n) => assert_eq!(parts.len(), n, "{by:?}: one member per value"),
            None => assert!(
                parts.len() >= 2,
                "{by:?}: a partition, {} members",
                parts.len()
            ),
        }
        let values: BTreeSet<u32> = parts.iter().map(|(v, _)| *v).collect();
        assert_eq!(values.len(), parts.len(), "{by:?}: a value listed twice");
        let sum =
            |f: &dyn Fn(&GroupSummary) -> u64| -> u64 { parts.iter().map(|(_, s)| f(s)).sum() };
        assert_eq!(sum(&|s| s.persons), whole.persons, "{by:?}: persons");
        assert_eq!(sum(&|s| s.employed), whole.employed, "{by:?}: employed");
        assert_eq!(
            sum(&|s| s.unemployed),
            whole.unemployed,
            "{by:?}: unemployed"
        );
        let whole_money = money_fields(&whole);
        for (k, (name, total)) in whole_money.iter().enumerate() {
            let parts_total: i128 = parts
                .iter()
                .map(|(_, s)| i128::from(money_fields(s)[k].1))
                .sum();
            let diff = (parts_total - i128::from(*total)).abs();
            if money_exact {
                assert_eq!(
                    diff, 0,
                    "{by:?}: {name} adds up to {parts_total}, the whole has {total}"
                );
            } else {
                assert!(
                    diff <= i128::from(whole.households),
                    "{by:?}: {name} adds up to {parts_total}, the whole has {total}: more than one ban per household ({})",
                    whole.households
                );
            }
        }
    }
}

// --- AC-GRP-03 ------------------------------------------------------------------

/// **AC-GRP-03** `[sim]` Consistency with the M5 indicators: with `poverty`
/// and `approval` compiled, a group by one region has the `sol_milli` and
/// `approval_milli` of that region in the poverty and approval stats, to
/// within one thousandth of a point; its persons equal the registry's
/// `people.population` for the region.
///
/// Readings: AC-GRP-01's world (3 closes at 1:1000); for each region `r`,
/// the filter `Region([r])`. The poverty stats' value is
/// `poverty_stats()` of the latest close, `sol.by_region[r]`; the approval
/// stats' is `approval_stats()` of the latest close, `by_region[r]`'s
/// `mean_milli` (a region without adults: `None` on both sides). "Within one
/// thousandth of a point" is within 1 in thousandths. `people.population`
/// for the region is its latest point at the key (region, `r`).
#[test]
#[cfg(all(feature = "poverty", feature = "approval"))]
fn ac_grp_03() {
    use econ_core::stats::{Breakdown, Key};
    let w = three_closes();
    let (regions, _) = regions_and_counties();
    let pov = w
        .poverty_stats()
        .last()
        .expect("poverty stats at the close");
    let app = w
        .approval_stats()
        .last()
        .expect("approval stats at the close");
    assert_eq!(pov.sol.by_region.len(), regions);
    for r in 0..regions {
        let g = w.group(&filter(vec![Condition::Region(vec![
            u8::try_from(r).expect("a region"),
        ])]));
        let sol = g.sol_milli.unwrap_or_else(|| panic!("region {r}: a SoL"));
        assert!(
            (i64::from(sol) - i64::from(pov.sol.by_region[r])).abs() <= 1,
            "region {r}: SoL {sol} against the poverty stats' {}",
            pov.sol.by_region[r]
        );
        match (&app.by_region[r], g.approval_milli) {
            (Some(a), Some(m)) => assert!(
                (i64::from(m) - i64::from(a.mean_milli)).abs() <= 1,
                "region {r}: approval {m} against the approval stats' {}",
                a.mean_milli
            ),
            (None, None) => {}
            (a, m) => panic!("region {r}: approval {m:?} against the approval stats' {a:?}"),
        }
        let pop = w
            .stats()
            .latest(id("people.population"), Key::of(Breakdown::Region, r))
            .unwrap_or_else(|| panic!("region {r}: people.population has a point"));
        assert_eq!(
            i64::try_from(g.persons).expect("fits"),
            pop,
            "region {r}: persons against people.population"
        );
    }
}

// --- AC-GRP-04 ------------------------------------------------------------------

/// **AC-GRP-04** `[unit]` Filters: two conditions on the same attribute
/// intersect; a condition with no value gives an empty group (all fields
/// zero, `small_sample` true); a group of fewer than `min_records` person
/// records is flagged; an age range `[from, to)` includes `from` and not
/// `to`.
///
/// Readings: on AC-GRP-01's world. "Intersect": `Sex([F]) ∧ Sex([M])` is
/// empty; `Age[20, 40) ∧ Age[30, 50)` is `Age[30, 40)`; `Edu([0–2, 3]) ∧
/// Edu([3, 4])` is `Edu([3])` (whole summaries equal). "No value": every
/// list condition with an empty list, and the empty range `Age[40, 40)`,
/// give the summary whose counts and money are zero, `small_sample` true and
/// whose means are `None` (a mean over no one). "Flagged": `small_sample` is
/// `person_records < MIN_RECORDS` (30, the spec's default), checked on an
/// age with 1 to 29 person rows among the old (searched from 85 up) and on
/// the empty filter (not flagged). The age range: `Age[a, a + 1)` holds the
/// residents whose age in whole years is `a` (persons and person rows), for
/// `a` = 0, 18, 30 and 65, and `[a, a + 1)` and `[a + 1, a + 2)` add up to
/// `[a, a + 2)`.
#[test]
fn ac_grp_04() {
    let w = three_closes();
    // Intersection.
    assert_eq!(
        w.group(&filter(vec![
            Condition::Sex(vec![Sex::F]),
            Condition::Sex(vec![Sex::M])
        ])),
        empty_summary(),
        "F and M"
    );
    assert_eq!(
        w.group(&filter(vec![
            Condition::Age { from: 20, to: 40 },
            Condition::Age { from: 30, to: 50 },
        ])),
        w.group(&filter(vec![Condition::Age { from: 30, to: 40 }])),
        "[20, 40) and [30, 50)"
    );
    assert_eq!(
        w.group(&filter(vec![
            Condition::Edu(vec![EduLevel::Isced0To2, EduLevel::Isced3]),
            Condition::Edu(vec![EduLevel::Isced3, EduLevel::Isced4]),
        ])),
        w.group(&filter(vec![Condition::Edu(vec![EduLevel::Isced3])])),
        "two edu conditions"
    );
    // No value.
    let none: Vec<(&str, Condition)> = vec![
        ("county", Condition::County(vec![])),
        ("region", Condition::Region(vec![])),
        ("age [40, 40)", Condition::Age { from: 40, to: 40 }),
        ("sex", Condition::Sex(vec![])),
        ("edu", Condition::Edu(vec![])),
        ("activity", Condition::Activity(vec![])),
        ("occupation", Condition::Occupation(vec![])),
        ("industry", Condition::Industry(vec![])),
        ("fifth", Condition::Fifth(vec![])),
        ("class", Condition::Class(vec![])),
        ("enrolled", Condition::Enrolled(vec![])),
    ];
    for (name, c) in none {
        assert_eq!(
            w.group(&filter(vec![c])),
            empty_summary(),
            "{name}: no value"
        );
    }
    // Small samples.
    assert_eq!(MIN_RECORDS, 30, "the spec's default");
    let all = w.group(&everyone());
    assert!(
        all.person_records >= MIN_RECORDS && !all.small_sample,
        "everyone: {all:?}"
    );
    let small = (85..=120u16)
        .find(|&a| {
            let (_, rows) = residents_aged(w, a, a + 1);
            rows > 0 && rows < MIN_RECORDS
        })
        .expect("an old age with 1 to 29 person rows at 1:1000");
    let (_, rows) = residents_aged(w, small, small + 1);
    let g = w.group(&filter(vec![Condition::Age {
        from: small,
        to: small + 1,
    }]));
    assert_eq!(g.person_records, rows, "age {small}: person rows");
    assert!(g.small_sample, "age {small}: {rows} rows, flagged");
    // The age range.
    for a in [0u16, 18, 30, 65] {
        let (persons, rows) = residents_aged(w, a, a + 1);
        let one = w.group(&filter(vec![Condition::Age { from: a, to: a + 1 }]));
        assert_eq!(one.persons, persons, "age {a}: persons");
        assert_eq!(one.person_records, rows, "age {a}: person rows");
        assert!(persons > 0, "age {a}: somebody");
        let next = w.group(&filter(vec![Condition::Age {
            from: a + 1,
            to: a + 2,
        }]));
        let two = w.group(&filter(vec![Condition::Age { from: a, to: a + 2 }]));
        assert_eq!(
            one.persons + next.persons,
            two.persons,
            "[{a}, {}) in two",
            a + 2
        );
        assert_eq!(
            one.person_records + next.person_records,
            two.person_records,
            "[{a}, {}) in two: rows",
            a + 2
        );
    }
}

// --- AC-GRP-05 ------------------------------------------------------------------

/// **AC-GRP-05** `[sim]` Watched groups: a filter watched on day 0 has one
/// summary per close, each equal to the group's summary at that close;
/// watching changes no state (the state hash after 6 closes is the same with
/// and without).
///
/// Readings: two filters (women of 18 to 64 in region 0; the unemployed)
/// watched on day 0 of a 1:1000 world, run for 6 closes beside an unwatched
/// one; after each close's day, `watched(name)` has as many summaries as
/// closes, the last equal to `group(filter)` then. The state hash is
/// compared every day, which includes the one after 6 closes. A name never
/// watched has none.
#[test]
fn ac_grp_05() {
    let women = filter(vec![
        Condition::Sex(vec![Sex::F]),
        Condition::Age { from: 18, to: 65 },
        Condition::Region(vec![0]),
    ]);
    let unemployed = filter(vec![Condition::Activity(vec![
        econ_popgen::Activity::Unemployed,
    ])]);
    let mut a = world_2021_default(1000);
    let mut b = world_2021(1000, params());
    a.watch_group("women", women.clone());
    a.watch_group("unemployed", unemployed.clone());
    assert_eq!(a.state_hash(), b.state_hash(), "watching on day 0");
    assert!(a.watched("nobody").is_none(), "a name never watched");
    let mut seen: Vec<(GroupSummary, GroupSummary)> = Vec::new();
    while a.months().len() < 6 {
        let before = a.months().len();
        let ra = step_checked(&mut a);
        let rb = step_checked(&mut b);
        assert_eq!(
            ra.state_hash, rb.state_hash,
            "{:?}: the state hashes differ",
            ra.date
        );
        if a.months().len() > before {
            seen.push((a.group(&women), a.group(&unemployed)));
        }
        for (name, k) in [("women", 0usize), ("unemployed", 1)] {
            let got = a.watched(name).unwrap_or_else(|| panic!("{name}: watched"));
            assert_eq!(got.len(), seen.len(), "{name}: one summary per close");
            if let Some(last) = seen.last() {
                let want = if k == 0 { &last.0 } else { &last.1 };
                assert_eq!(
                    got.last(),
                    Some(want),
                    "{name}: the summary at close {}",
                    seen.len()
                );
            }
        }
    }
    assert_eq!(a.state_hash(), b.state_hash(), "after 6 closes");
    assert_eq!(a.watched("women").map(<[GroupSummary]>::len), Some(6));
}

// --- AC-GRP-06 ------------------------------------------------------------------

/// The fastest of three timings of `f`, milliseconds.
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
fn best_millis(mut f: impl FnMut()) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..3 {
        let started = std::time::Instant::now();
        f();
        best = best.min(started.elapsed().as_millis());
    }
    best
}

/// **AC-GRP-06** `[bench]` A summary of the empty filter at 1:100 takes at
/// most 50 ms in a release build, and a breakdown by county at most 200 ms.
///
/// Readings: the Romania scenario at 1:100 under the 2021 law after its
/// first close (a summary needs a close), each query timed three times and
/// the fastest kept (one slow moment of the machine does not decide). A time
/// budget means nothing in a debug build: the test runs in release builds
/// only.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build time budget: cargo test --release -p econ-core --features groups --test acceptance groups::"
)]
fn ac_grp_06() {
    let mut w = world_2021_default(100);
    run_closes(&mut w, 1);
    let mut persons = 0;
    let summary = best_millis(|| persons = w.group(&everyone()).persons);
    assert!(persons > 0, "a populated group");
    let mut members = 0;
    let breakdown =
        best_millis(|| members = w.group_breakdown(&everyone(), Attribute::County).len());
    assert!(members > 1, "counties");
    assert!(
        summary <= 50,
        "the empty filter's summary took {summary} ms (more than 50)"
    );
    assert!(
        breakdown <= 200,
        "the breakdown by county took {breakdown} ms (more than 200)"
    );
}
