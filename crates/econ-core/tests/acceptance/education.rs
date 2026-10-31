//! Acceptance tests for AC-EDU-01 to AC-EDU-09 (spec: society/education,
//! "First version (roadmap M5)"), written by the test writer from the spec
//! and its API sketch before the code (ADR-0014); behind the feature
//! `education`.
//!
//! "1:100" and "1:1000" are the Romania scenario at that scale, seed 42,
//! under the law of its folder (`law.json`: the 2021 law), with no lever
//! changed. Day 0 is 1 December 2021 (month 0); the June closes are those of
//! 30 June (months 6, 18, 30, …), the August closes those of 31 August
//! (months 8, 20, 32, …). Levels are indexed from pre-school (0) to master
//! or doctorate (6), `enrolled` − 1.
//!
//! Readings shared by the tests:
//! - A job seeker is a person whose activity is `Unemployed`. A seeker
//!   qualifies for the low tier with any education, the mid tier with
//!   ISCED 3 or more, the high tier with ISCED 5 or more (the labour
//!   market's table); an occupation's tier is high for ISCO 1 to 3, mid for
//!   4 to 8, low for 9.
//! - Persons who are no longer residents (a record of weight 0, abroad
//!   with migration, or dead with the People phase) are left out of every
//!   count and comparison.
//! - A person is the same person at two snapshots when it has the same
//!   `person_key` (`World::people`): with the People phase compiled in
//!   (features `demography` and `lifecycle`, which `migration` implies), the
//!   yearly compaction moves rows (restated 2026-10-31; the tests read rows
//!   and "age did not fall" before).
//! - A leaver is a person who was enrolled, or a pupil or student, before a
//!   close and after it is not enrolled and is neither pupil nor student.

use std::collections::{BTreeMap, BTreeSet};

use econ_core::world::World;
use econ_core::world::education::{
    EducationParams, EducationYear, JuneOutcome, LEVEL_START_AGE, LEVEL_YEARS, LeaverFate, admit,
    june_step,
};
use econ_popgen::{Activity, EduLevel, Enrolled, NO_EMPLOYER};
use econ_types::{Date, Day};

use super::support::start_state;
use super::support_lev::date;
use super::support_m5::{
    residents_mask, step_checked, step_until, weights_of_persons, world_2021_default,
    world_2021_seed,
};

/// Years, in months.
const fn years(y: u16) -> u16 {
    y * 12
}

/// The level index of an enrolment (`None` for not enrolled).
fn level(e: Enrolled) -> Option<usize> {
    match e {
        Enrolled::None => None,
        other => Some(other as usize - 1),
    }
}

/// The enrolment of a level index.
fn enrolment(l: usize) -> Enrolled {
    [
        Enrolled::PreSchool,
        Enrolled::Primary,
        Enrolled::LowerSecondary,
        Enrolled::UpperSecondary,
        Enrolled::PostSecondary,
        Enrolled::Tertiary,
        Enrolled::MasterDoctorate,
    ][l]
}

/// Whether a person's education qualifies for a tier (0 low, 1 mid, 2 high).
fn qualifies_tier(tier: usize, edu: EduLevel) -> bool {
    match tier {
        0 => true,
        1 => edu >= EduLevel::Isced3,
        _ => edu >= EduLevel::Isced5To6,
    }
}

/// The tier of an occupation (ISCO major group).
fn tier_of(occupation: u8) -> usize {
    match occupation {
        1..=3 => 2,
        4..=8 => 1,
        9 => 0,
        other => panic!("a job in occupation {other}"),
    }
}

/// The edu level of an index of the report's leaver arrays.
fn edu(e: usize) -> EduLevel {
    [
        EduLevel::Isced0To2,
        EduLevel::Isced3,
        EduLevel::Isced4,
        EduLevel::Isced5To6,
        EduLevel::Isced7To8,
    ][e]
}

/// Whether an activity is pupil or student.
fn in_school(a: Activity) -> bool {
    matches!(a, Activity::Pupil | Activity::Student)
}

/// Each person's `person_key`, aligned with the person rows now.
fn person_keys(w: &World) -> Vec<u64> {
    w.people().person_key.clone()
}

/// The row of each `person_key` now.
fn rows_by_key(w: &World) -> BTreeMap<u64, usize> {
    w.people()
        .person_key
        .iter()
        .enumerate()
        .map(|(p, &k)| (k, p))
        .collect()
}

/// Whether each person row is a resident now: `support_m5`'s residents
/// (a record of positive weight, not abroad) and, with the People phase
/// compiled in, alive (a dead person's row stays until the compaction).
fn residents_now(w: &World) -> Vec<bool> {
    #[cfg(feature = "demography")]
    let res = {
        let mut res = residents_mask(w);
        for (r, &alive) in res.iter_mut().zip(&w.people_life().alive) {
            *r &= alive;
        }
        res
    };
    #[cfg(not(feature = "demography"))]
    let res = residents_mask(w);
    res
}

/// The weighted enrolled persons of each level now.
fn enrolled_by_level(w: &World) -> [u64; 7] {
    let weights = weights_of_persons(w);
    let res = residents_now(w);
    let mut n = [0u64; 7];
    for (p, &e) in w.enrolled().iter().enumerate() {
        if let Some(l) = level(e)
            && res[p]
        {
            n[l] += u64::from(weights[p]);
        }
    }
    n
}

/// **AC-EDU-01** `[unit]` Starting `study_year`: for a person whose age on 1
/// September 2021 less the level's starting age lies within the level's
/// years, it is that difference; otherwise it is the keyed draw modulo the
/// years. `edu_capacity` equals the weighted enrolled count of each level at
/// load. Same inputs and seed give the same columns.
///
/// Readings: the age on 1 September 2021 is the age in months at day 0 less
/// 3, in whole years. The keyed draw (`draw(Education, tick 0, entity =
/// person)`, first word) uses a stream the code adds, which the test cannot
/// name: for a person outside the range the test checks that the year is
/// one of the level's years, and that the years drawn for a level's many
/// out-of-range persons are not all the same. Checked on the `World` at
/// 1:1000 before its first day: `enrolled` is Stage K's column of the
/// scenario.
#[test]
fn ac_edu_01() {
    let w = world_2021_default(1000);
    let start = start_state(1000);
    assert_eq!(w.enrolled(), &start.enrolled[..], "`enrolled` is Stage K's");
    let ages = w.person_age_months();
    let years_col = w.study_year();
    let mut outside: [Vec<u8>; 7] = Default::default();
    let mut inside = 0;
    for (p, &e) in w.enrolled().iter().enumerate() {
        let Some(l) = level(e) else {
            assert_eq!(years_col[p], 0, "person {p} not enrolled: study year 0");
            continue;
        };
        let age_sept = i32::from(ages[p].saturating_sub(3) / 12);
        let diff = age_sept - i32::from(LEVEL_START_AGE[l]);
        if (0..i32::from(LEVEL_YEARS[l])).contains(&diff) {
            assert_eq!(
                i32::from(years_col[p]),
                diff,
                "person {p}, level {l}, aged {} months",
                ages[p]
            );
            inside += 1;
        } else {
            assert!(
                years_col[p] < LEVEL_YEARS[l],
                "person {p}, level {l}: study year {} of {}",
                years_col[p],
                LEVEL_YEARS[l]
            );
            outside[l].push(years_col[p]);
        }
    }
    assert!(inside > 0);
    for (l, ys) in outside.iter().enumerate() {
        if ys.len() >= 50 && LEVEL_YEARS[l] > 1 {
            assert!(
                ys.iter().any(|&y| y != ys[0]),
                "level {l}: {} out-of-range persons all drew year {}",
                ys.len(),
                ys[0]
            );
        }
    }
    assert!(
        outside.iter().map(Vec::len).sum::<usize>() > 0,
        "the start has repeating, late or adult students"
    );
    assert_eq!(
        w.edu_capacity(),
        enrolled_by_level(&w),
        "places = enrolled at load"
    );

    let again = world_2021_default(1000);
    assert_eq!(again.enrolled(), w.enrolled());
    assert_eq!(again.study_year(), w.study_year());
    assert_eq!(again.edu_capacity(), w.edu_capacity());
}

fn p() -> EducationParams {
    let p = EducationParams::spec_defaults();
    assert_eq!(
        p.dropout_ppm,
        [0, 5_000, 10_000, 20_000, 80_000, 120_000, 80_000]
    );
    assert_eq!(p.continue_lower_to_upper_ppm, 980_000);
    assert_eq!(p.continue_upper_to_tertiary_ppm, 650_000);
    assert_eq!(p.continue_upper_to_post_ppm, 200_000);
    assert_eq!(p.continue_bachelor_to_master_ppm, 700_000);
    assert_eq!(p.primary_intake_ppm, 970_000);
    assert_eq!(p.preschool_intake_ppm, [660_000, 350_000, 200_000]);
    assert_eq!(
        p.leaver_seek_ppm,
        [400_000, 700_000, 800_000, 850_000, 900_000]
    );
    assert_eq!(p.min_leaving_age, 16);
    p
}

/// Draws: `u1` no dropout, `u2` and `u3` as given.
const STAY: u32 = 999_999;

fn outcome(
    e: Enrolled,
    year: u8,
    edu: EduLevel,
    age: u16,
    job: bool,
    draws: [u32; 3],
) -> JuneOutcome {
    june_step(e, year, edu, age, job, draws, &p())
}

/// **AC-EDU-02** `[unit]` The June rule for one person, for given draws: a
/// dropout keeps `edu_level` and leaves; a completer's `edu_level` becomes
/// the maximum of the old one and the level's completion value, never lower;
/// continuation follows the rates in the table's order; a leaver of 16 or
/// more without a job becomes a seeker or other inactive by
/// `leaver_seek_share`, one with a job stays employed; a leaver under 16
/// stays a pupil not enrolled.
///
/// Readings: the draws are in millionths; an event of rate `r` happens when
/// its draw is below `r` (`u1` < dropout rate; `u3` < the seek share: a
/// seeker). Continuation from upper secondary takes `u2` against the rates
/// in the table's order, cumulated: below 650,000 tertiary, below 850,000
/// post-secondary, else leave. `age_months` is the age at the close; a
/// pre-school child "will be 6 on 1 September" at 71 months on 30 June.
#[test]
fn ac_edu_02() {
    let adult = years(17);
    // Dropout from upper secondary: leaves, keeps ISCED 0–2, a seeker or not.
    let o = outcome(
        Enrolled::UpperSecondary,
        1,
        EduLevel::Isced0To2,
        adult,
        false,
        [0, STAY, 0],
    );
    assert!(o.dropped_out && !o.completed);
    assert_eq!(
        (o.enrolled, o.study_year, o.edu_level),
        (Enrolled::None, 0, EduLevel::Isced0To2)
    );
    assert_eq!(o.leaver, Some(LeaverFate::Seeking), "u3 0 < 400,000");
    let o = outcome(
        Enrolled::UpperSecondary,
        1,
        EduLevel::Isced0To2,
        adult,
        false,
        [19_999, STAY, 399_999],
    );
    assert!(o.dropped_out);
    assert_eq!(o.leaver, Some(LeaverFate::Seeking));
    let o = outcome(
        Enrolled::UpperSecondary,
        1,
        EduLevel::Isced0To2,
        adult,
        false,
        [0, STAY, 400_000],
    );
    assert_eq!(
        o.leaver,
        Some(LeaverFate::Inactive),
        "u3 at the share: inactive"
    );
    // A dropout with a job keeps it.
    let o = outcome(
        Enrolled::Tertiary,
        0,
        EduLevel::Isced3,
        years(25),
        true,
        [0, 0, 0],
    );
    assert_eq!(o.leaver, Some(LeaverFate::KeepsJob));
    assert_eq!(o.edu_level, EduLevel::Isced3);
    // No dropout at u1 = the rate.
    let o = outcome(
        Enrolled::UpperSecondary,
        1,
        EduLevel::Isced0To2,
        adult,
        false,
        [20_000, STAY, 0],
    );
    assert!(!o.dropped_out);
    assert_eq!(
        (o.enrolled, o.study_year, o.leaver),
        (Enrolled::UpperSecondary, 2, None),
        "progression"
    );

    // Completing upper secondary: ISCED 3, then continuation in table order.
    let done = |u2| {
        outcome(
            Enrolled::UpperSecondary,
            3,
            EduLevel::Isced0To2,
            years(19),
            false,
            [STAY, u2, 0],
        )
    };
    let o = done(0);
    assert!(o.completed && !o.dropped_out);
    assert_eq!(o.edu_level, EduLevel::Isced3);
    assert_eq!(
        (o.enrolled, o.study_year, o.leaver),
        (Enrolled::Tertiary, 0, None)
    );
    assert_eq!(done(649_999).enrolled, Enrolled::Tertiary);
    assert_eq!(done(650_000).enrolled, Enrolled::PostSecondary);
    assert_eq!(done(849_999).enrolled, Enrolled::PostSecondary);
    let o = done(850_000);
    assert_eq!((o.enrolled, o.study_year), (Enrolled::None, 0));
    assert_eq!(o.edu_level, EduLevel::Isced3);
    assert_eq!(
        o.leaver,
        Some(LeaverFate::Seeking),
        "u3 0 < 700,000 for ISCED 3"
    );
    // The completion value never lowers `edu_level`.
    let o = outcome(
        Enrolled::UpperSecondary,
        3,
        EduLevel::Isced5To6,
        years(30),
        false,
        [STAY, STAY, STAY],
    );
    assert!(o.completed);
    assert_eq!(o.edu_level, EduLevel::Isced5To6);
    assert_eq!(o.leaver, Some(LeaverFate::Inactive));

    // Primary to lower secondary with no draw.
    let o = outcome(
        Enrolled::Primary,
        4,
        EduLevel::Isced0To2,
        years(11),
        false,
        [STAY, STAY, STAY],
    );
    assert!(o.completed);
    assert_eq!(
        (o.enrolled, o.study_year, o.leaver),
        (Enrolled::LowerSecondary, 0, None)
    );
    // Lower secondary to upper secondary with 0.98.
    let o = outcome(
        Enrolled::LowerSecondary,
        3,
        EduLevel::Isced0To2,
        years(15),
        false,
        [STAY, 979_999, 0],
    );
    assert_eq!(
        (o.enrolled, o.edu_level),
        (Enrolled::UpperSecondary, EduLevel::Isced0To2)
    );
    let o = outcome(
        Enrolled::LowerSecondary,
        3,
        EduLevel::Isced0To2,
        years(15),
        false,
        [STAY, 980_000, 0],
    );
    assert_eq!(o.enrolled, Enrolled::None);
    assert_eq!(
        o.leaver, None,
        "a leaver under 16 stays a pupil not enrolled"
    );
    let o = outcome(
        Enrolled::LowerSecondary,
        3,
        EduLevel::Isced0To2,
        years(16),
        false,
        [STAY, 980_000, 0],
    );
    assert_eq!(o.leaver, Some(LeaverFate::Seeking), "16: a leaver");
    // Post-secondary: ISCED 4, leave.
    let o = outcome(
        Enrolled::PostSecondary,
        1,
        EduLevel::Isced3,
        years(21),
        false,
        [STAY, 0, STAY],
    );
    assert_eq!(
        (o.enrolled, o.edu_level),
        (Enrolled::None, EduLevel::Isced4)
    );
    assert_eq!(o.leaver, Some(LeaverFate::Inactive));
    // Bachelor: ISCED 5–6, master with 0.70.
    let o = outcome(
        Enrolled::Tertiary,
        2,
        EduLevel::Isced3,
        years(22),
        false,
        [STAY, 699_999, 0],
    );
    assert_eq!(
        (o.enrolled, o.study_year, o.edu_level),
        (Enrolled::MasterDoctorate, 0, EduLevel::Isced5To6)
    );
    let o = outcome(
        Enrolled::Tertiary,
        2,
        EduLevel::Isced3,
        years(22),
        false,
        [STAY, 700_000, 849_999],
    );
    assert_eq!(
        (o.enrolled, o.edu_level),
        (Enrolled::None, EduLevel::Isced5To6)
    );
    assert_eq!(
        o.leaver,
        Some(LeaverFate::Seeking),
        "u3 < 850,000 for ISCED 5–6"
    );
    // Master: ISCED 7–8, leave.
    let o = outcome(
        Enrolled::MasterDoctorate,
        1,
        EduLevel::Isced5To6,
        years(24),
        true,
        [STAY, 0, 0],
    );
    assert_eq!(
        (o.enrolled, o.edu_level),
        (Enrolled::None, EduLevel::Isced7To8)
    );
    assert_eq!(o.leaver, Some(LeaverFate::KeepsJob));

    // A dropout under 16 stays a pupil not enrolled.
    let o = outcome(
        Enrolled::LowerSecondary,
        1,
        EduLevel::Isced0To2,
        years(13),
        false,
        [0, 0, 0],
    );
    assert!(o.dropped_out);
    assert_eq!((o.enrolled, o.leaver), (Enrolled::None, None));

    // Pre-school: leaves at 6 on 1 September whatever the year; no leaver.
    let o = outcome(
        Enrolled::PreSchool,
        0,
        EduLevel::Isced0To2,
        71,
        false,
        [STAY, STAY, STAY],
    );
    assert_eq!((o.enrolled, o.leaver), (Enrolled::None, None));
    let o = outcome(
        Enrolled::PreSchool,
        1,
        EduLevel::Isced0To2,
        years(5),
        false,
        [0, 0, 0],
    );
    assert!(!o.dropped_out, "no dropout from pre-school (rate 0)");
    assert_eq!((o.enrolled, o.study_year), (Enrolled::PreSchool, 2));

    // A pupil not enrolled: a leaver from 16.
    let o = outcome(
        Enrolled::None,
        0,
        EduLevel::Isced0To2,
        years(16),
        false,
        [STAY, STAY, 0],
    );
    assert_eq!(
        (o.enrolled, o.leaver),
        (Enrolled::None, Some(LeaverFate::Seeking))
    );
    let o = outcome(
        Enrolled::None,
        0,
        EduLevel::Isced0To2,
        years(16) - 1,
        false,
        [STAY, STAY, 0],
    );
    assert_eq!(o.leaver, None);
}

/// **AC-EDU-03** `[unit]` Places at the August close: continuing students are
/// never removed; entrants are admitted in ascending (keyed priority, index)
/// while the level's weight stays within its places; with a cohort of
/// 6-year-olds above primary's free places, exactly the surplus (to within
/// one person's weight) is refused and stays not enrolled; with places to
/// spare nobody is refused.
///
/// Readings: `admit` answers for the entrants only (continuing students are
/// not its to remove); with continuing students above the places, every
/// entrant is refused. Entrants of equal weight make "while it stays
/// within" unambiguous.
#[test]
fn ac_edu_03() {
    // Ten entrants of 100, priorities out of order.
    let entrants: Vec<(u64, u32, u32)> = [
        (50, 0),
        (10, 1),
        (90, 2),
        (30, 3),
        (70, 4),
        (20, 5),
        (60, 6),
        (40, 7),
        (80, 8),
        (0, 9),
    ]
    .iter()
    .map(|&(prio, i)| (prio, i, 100))
    .collect();
    let a = admit(1_000, 400, &entrants);
    assert_eq!(a.len(), 10);
    // Priorities 0, 10, 20, 30, 40, 50 fit: 400 + 600 = 1,000.
    let admitted: Vec<u32> = entrants
        .iter()
        .zip(&a)
        .filter(|(_, x)| **x)
        .map(|(e, _)| e.1)
        .collect();
    let mut admitted_sorted = admitted.clone();
    admitted_sorted.sort_unstable();
    assert_eq!(admitted_sorted, vec![0, 1, 3, 5, 7, 9]);
    // Ties broken by index.
    let tied: Vec<(u64, u32, u32)> = (0..5).map(|i| (7, 4 - i, 100)).collect();
    let a = admit(300, 100, &tied);
    let got: Vec<u32> = tied
        .iter()
        .zip(&a)
        .filter(|(_, x)| **x)
        .map(|(e, _)| e.1)
        .collect();
    let mut got = got;
    got.sort_unstable();
    assert_eq!(got, vec![0, 1], "equal priorities: the lower indices");
    // Continuing above the places: nobody admitted, nobody removed.
    assert!(admit(1_000, 1_200, &entrants).iter().all(|&x| !x));
    // Places to spare.
    assert!(admit(10_000, 400, &entrants).iter().all(|&x| x));
    assert!(
        admit(1_400, 400, &entrants).iter().all(|&x| x),
        "exactly enough"
    );
    // A cohort of 6-year-olds of weight 1,000 each above the free places:
    // the surplus refused to within one weight.
    let cohort: Vec<(u64, u32, u32)> = (0..190u32)
        .map(|i| {
            (
                u64::from(i.wrapping_mul(2_654_435_761) % 1_000_003),
                i,
                1_000,
            )
        })
        .collect();
    let free = 173_400;
    let a = admit(867_000, 867_000 - free, &cohort);
    let admitted: u64 = cohort
        .iter()
        .zip(&a)
        .filter(|(_, x)| **x)
        .map(|(e, _)| u64::from(e.2))
        .sum();
    let refused = 190_000 - admitted;
    assert!(admitted <= free, "within the places");
    assert!(
        refused - (190_000 - free) < 1_000,
        "the surplus refused to within one weight: {refused}"
    );
}

/// The weighted completers of a June report by level.
fn completers(y: &EducationYear) -> [u64; 7] {
    let mut c = [0; 7];
    for (l, x) in y.levels.iter().enumerate() {
        c[l] = x.completers;
    }
    c
}

/// Step a world through the June close of a year; the report.
fn june_report(w: &mut World, year: i32) -> EducationYear {
    step_until(w, date(year, 7, 1));
    w.education_year().expect("a June report").clone()
}

/// AC-EDU-04's check on a world at 1:100.
fn assert_graduates(w: &mut World, what: &str) {
    // 2021's graduates: upper secondary, post-secondary, bachelor, master.
    let targets = [
        (3, 175_934u64, 25),
        (4, 31_705, 30),
        (5, 76_593, 25),
        (6, 54_941, 30),
    ];
    for year in [2022, 2023, 2024] {
        let c = completers(&june_report(w, year));
        for (l, target, pct) in targets {
            let lo = target * (100 - pct) / 100;
            let hi = target * (100 + pct) / 100;
            assert!(
                (lo..=hi).contains(&c[l]),
                "{what}: June {year}, level {l}: {} completers against {target} ± {pct}%",
                c[l]
            );
        }
    }
}

/// **AC-EDU-04** `[sim]` At 1:100, the weighted completers of each of the
/// June closes of 2022, 2023 and 2024 are within 25% of 2021's graduates for
/// upper secondary (175,934) and bachelor (76,593), and within 30% for
/// post-secondary (31,705) and master and doctorate (54,941).
///
/// Readings: "within 25%" is the inclusive band target × (1 ± 0.25), in
/// whole persons rounded down at both ends. Three years at 1:100: release
/// builds only.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "three years at 1:100: cargo test --release -p econ-core --features education --test acceptance education::"
)]
fn ac_edu_04() {
    let mut w = world_2021_default(100);
    assert_graduates(&mut w, "seed 42");
}

/// Weighted job seekers qualifying for each tier.
fn seekers_by_tier(w: &World) -> [u64; 3] {
    let weights = weights_of_persons(w);
    let res = residents_now(w);
    let mut s = [0u64; 3];
    for (p, &a) in w.activity().iter().enumerate() {
        if a == Activity::Unemployed && res[p] {
            for (t, x) in s.iter_mut().enumerate() {
                if qualifies_tier(t, w.persons().edu_level[p]) {
                    *x += u64::from(weights[p]);
                }
            }
        }
    }
    s
}

/// The columns education may change.
fn edu_columns(w: &World) -> (Vec<Enrolled>, Vec<u8>, Vec<EduLevel>) {
    (
        w.enrolled().to_vec(),
        w.study_year().to_vec(),
        w.persons().edu_level.clone(),
    )
}

/// Whether a day falls in July 2022.
fn in_july(d: Day, w: &World) -> bool {
    let _ = w;
    let first = 212; // 1 July 2022 is day 212 (from 1 December 2021)
    (first..first + 31).contains(&d.0)
}

/// **AC-EDU-05** `[sim]` At 1:1000: in the month after a June close, the
/// weighted job seekers of each tier rise by at least the leavers who seek
/// and qualify for it, less those hired that month; every leaver hired in a
/// tier qualifies for it with their new `edu_level`; in the months other
/// than June and August no person's `enrolled`, `study_year` or `edu_level`
/// changes.
///
/// Restated 2026-10-31 (the owner's yes for this run): persons are followed
/// by `person_key` (`World::people`), not by row and "age did not fall",
/// because with the People phase compiled in the yearly compaction shifts
/// the rows; a dead person (with the People phase) is not a resident.
///
/// Readings: the June close runs after the month's statistics, so the
/// seekers "before" are those of the June close without the leavers who
/// seek (the seekers after the close less the June report's
/// `leavers_seeking`, by the tiers their `edu_level` qualifies for); the
/// seekers "after" are those at the July close. "Those hired that month"
/// are the persons hired in July who qualify for the tier (each such hire
/// leaves the tier's pool). The third part compares the three columns at
/// every monthly close of 14 months with the close before, for the persons
/// present at both (the same `person_key`, a resident at the close).
#[test]
fn ac_edu_05() {
    let mut w = world_2021_default(1000);
    assert_eq!(w.date(), date(2021, 12, 1));
    assert_eq!(date(2022, 7, 1), Date::new(2022, 7, 1).expect("a date"));
    let mut cols = edu_columns(&w);
    let mut keys = person_keys(&w);
    let mut pre_june: Option<BTreeMap<u64, Activity>> = None;
    let mut after_june: Option<([u64; 3], [u64; 3], BTreeSet<u64>)> = None;
    while w.months().len() < 14 {
        if w.date() == date(2022, 6, 30) {
            pre_june = Some(
                w.people()
                    .person_key
                    .iter()
                    .zip(w.activity())
                    .map(|(&k, &a)| (k, a))
                    .collect(),
            );
        }
        let closes = w.months().len();
        step_checked(&mut w);
        if w.months().len() == closes {
            continue;
        }
        let t = closes; // the month just closed
        let now = edu_columns(&w);
        let res = residents_now(&w);
        let rows = rows_by_key(&w);
        if t != 6 && t != 8 {
            for (p, k) in keys.iter().enumerate() {
                let Some(&q) = rows.get(k) else { continue };
                if !res[q] {
                    continue;
                }
                assert_eq!(
                    (cols.0[p], cols.1[p], cols.2[p]),
                    (now.0[q], now.1[q], now.2[q]),
                    "month {t}: person {k}'s education changed outside June and August"
                );
            }
        }
        let keys_now = person_keys(&w);
        if t == 6 {
            let report = w.education_year().expect("the June report").clone();
            let mut leavers_seek = [0u64; 3];
            for (e, &n) in report.leavers_seeking.iter().enumerate() {
                for (tier, x) in leavers_seek.iter_mut().enumerate() {
                    if qualifies_tier(tier, edu(e)) {
                        *x += n;
                    }
                }
            }
            let pre = pre_june.as_ref().expect("the day before the June close");
            let leavers: BTreeSet<u64> = (0..keys_now.len())
                .filter(|&q| {
                    res[q]
                        && pre.get(&keys_now[q]).is_some_and(|&a| in_school(a))
                        && w.activity()[q] == Activity::Unemployed
                })
                .map(|q| keys_now[q])
                .collect();
            after_june = Some((seekers_by_tier(&w), leavers_seek, leavers));
        }
        if t == 7 {
            let (after, leavers_seek, leavers) = after_june.clone().expect("the June close ran");
            let weights = weights_of_persons(&w);
            let pp = w.persons();
            let mut hired = [0u64; 3];
            for p in 0..pp.hired_on.len() {
                let Some(d) = pp.hired_on[p] else { continue };
                if !in_july(d, &w) || !res[p] {
                    continue;
                }
                for (tier, x) in hired.iter_mut().enumerate() {
                    if qualifies_tier(tier, pp.edu_level[p]) {
                        *x += u64::from(weights[p]);
                    }
                }
                if leavers.contains(&keys_now[p]) && pp.employer[p] != NO_EMPLOYER {
                    let tier = tier_of(pp.occupation[p]);
                    assert!(
                        qualifies_tier(tier, pp.edu_level[p]),
                        "leaver {} with {:?} hired into occupation {}",
                        keys_now[p],
                        pp.edu_level[p],
                        pp.occupation[p]
                    );
                }
            }
            let july = seekers_by_tier(&w);
            for tier in 0..3 {
                let before = i128::from(after[tier]) - i128::from(leavers_seek[tier]);
                let rise = i128::from(july[tier]) - before;
                let floor = i128::from(leavers_seek[tier]) - i128::from(hired[tier]);
                assert!(
                    rise >= floor,
                    "tier {tier}: seekers rose by {rise}, less than {} leavers seeking less {} hired",
                    leavers_seek[tier],
                    hired[tier]
                );
            }
            assert!(
                leavers_seek[0] > 0,
                "the June close sent leavers to the labour market"
            );
        }
        cols = now;
        keys = keys_now;
    }
}

/// **AC-EDU-06** `[sim]` Over a 10-year run at 1:100, at each June close the
/// early leavers rate of 18 to 24 is between 10% and 22% (2021: 15.3%) and
/// the tertiary attainment of 25 to 34 between 18% and 32% (2021: 23.3%).
///
/// Readings: the June closes of 2022 to 2031; the bands are inclusive.
/// Release builds only.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "ten years at 1:100: cargo test --release -p econ-core --features education --test acceptance education::"
)]
fn ac_edu_06() {
    let mut w = world_2021_default(100);
    for year in 2022..=2031 {
        step_until(&mut w, date(year, 7, 1));
        let early = w.early_leavers_rate_ppm();
        let tertiary = w.tertiary_attainment_ppm();
        assert!(
            (100_000..=220_000).contains(&early),
            "June {year}: early leavers {early}"
        );
        assert!(
            (180_000..=320_000).contains(&tertiary),
            "June {year}: tertiary attainment {tertiary}"
        );
    }
}

/// **AC-EDU-07** `[sim]` At 1:1000, no person who is a pupil not enrolled at
/// the start is ever enrolled; each becomes a leaver at the first June close
/// at which they are 16 or older.
///
/// Restated 2026-10-31 (the owner's yes for this run): persons are followed
/// by `person_key` (`World::people`), not by row and "age did not fall",
/// because with the People phase compiled in the yearly compaction shifts
/// the rows.
///
/// Readings: "a pupil not enrolled" is a person whose activity is pupil or
/// student and who is not enrolled at load. The run covers three June closes
/// (to July 2024). At every close such a person is not enrolled; at each
/// June close, one still pupil or student before it who is 16 or older at
/// the close is not pupil nor student after it; one younger still is. A
/// person who dies or emigrates (no `person_key` row now, or not a
/// resident) is followed no further. With the People phase compiled in, a
/// person it retires (activity retired: the standard or early retirement of
/// demographics, step 5) has left school through a pension, not through
/// education's closes (the spec's edge case: a pension does not change
/// education's rule, which acts on pupils and students only), and is
/// followed no further; without it nobody retires.
#[test]
fn ac_edu_07() {
    let mut w = world_2021_default(1000);
    let start: Vec<u64> = (0..w.enrolled().len())
        .filter(|&p| w.enrolled()[p] == Enrolled::None && in_school(w.activity()[p]))
        .map(|p| w.people().person_key[p])
        .collect();
    assert!(
        !start.is_empty(),
        "the start has pupils not enrolled (Stage K)"
    );
    let mut became_16 = 0;
    let mut gone: BTreeSet<u64> = BTreeSet::new();
    while w.date() < date(2024, 7, 1) {
        let before: BTreeMap<u64, Activity> = w
            .people()
            .person_key
            .iter()
            .zip(w.activity())
            .map(|(&k, &a)| (k, a))
            .collect();
        let closes = w.months().len();
        step_checked(&mut w);
        if w.months().len() == closes {
            continue;
        }
        let june = w.date() == date(w.date().year, 7, 1);
        let res = residents_now(&w);
        let rows = rows_by_key(&w);
        let ages = w.person_age_months();
        for &k in &start {
            let row = rows.get(&k).copied().filter(|&q| res[q]);
            let Some(q) = row.filter(|_| !gone.contains(&k)) else {
                gone.insert(k);
                continue;
            };
            assert_eq!(
                w.enrolled()[q],
                Enrolled::None,
                "{:?}: person {k} enrolled",
                w.date()
            );
            let was = before
                .get(&k)
                .copied()
                .expect("followed since the last day");
            if !in_school(was) {
                continue;
            }
            let now = w.activity()[q];
            if now == Activity::Retired {
                // Retired by the People phase: a pension, not a close.
                gone.insert(k);
                continue;
            }
            let still = in_school(now);
            if june && ages[q] >= years(16) {
                assert!(
                    !still,
                    "{:?}: person {k}, {} months, still a pupil",
                    w.date(),
                    ages[q]
                );
                became_16 += 1;
            } else {
                assert!(
                    still,
                    "{:?}: person {k}, {} months, left school outside a June close at 16",
                    w.date(),
                    ages[q]
                );
            }
        }
    }
    assert!(became_16 > 0, "some reached 16 and left");
}

/// A snapshot of the persons on the day before a close: `enrolled`, the
/// activity and the row, by `person_key`.
type Snapshot = BTreeMap<u64, (Enrolled, Activity)>;

/// The snapshot of the persons now.
fn snapshot(w: &World) -> Snapshot {
    let k = &w.people().person_key;
    (0..k.len())
        .map(|p| (k[p], (w.enrolled()[p], w.activity()[p])))
        .collect()
}

/// The leavers of a close by fate, weighted.
#[derive(Debug, Default)]
struct Leavers {
    seeking: u64,
    inactive: u64,
    employed: u64,
    /// Leavers retired after the close (a pension of the People phase).
    retired: u64,
}

/// The leavers of a close by the criterion, against the persons table.
///
/// The criterion's leavers are the resident persons aged 16 or more at the
/// close among the dropouts, the completers who do not continue and the
/// refused entrants (in the persons table: enrolled before the close and not
/// enrolled after it) and, at a June close, the persons not enrolled whose
/// activity was pupil or student before it. Persons are matched by
/// `person_key` (a person born in the step is not in the snapshot and not a
/// leaver). Asserts that each of them has left school (neither enrolled nor
/// pupil or student) and that nobody else has, but for a person retired by
/// the People phase; returns their weights by fate: seeking, other
/// inactive, keeping a job, or retired.
fn leavers_by_snapshot(before: &Snapshot, w: &World, june: bool) -> Leavers {
    let weights = weights_of_persons(w);
    let res = residents_now(w);
    let ages = w.person_age_months();
    let keys = &w.people().person_key;
    let mut out = Leavers::default();
    for p in 0..keys.len() {
        let Some(&(enrolled_before, activity_before)) = before.get(&keys[p]) else {
            continue;
        };
        if !res[p] {
            continue;
        }
        let was_enrolled = enrolled_before != Enrolled::None;
        let left_level = was_enrolled && w.enrolled()[p] == Enrolled::None;
        let pupil_not_enrolled = !was_enrolled && in_school(activity_before);
        let leaver = ages[p] >= years(16) && (left_level || (june && pupil_not_enrolled));
        let was = was_enrolled || in_school(activity_before);
        let is = w.enrolled()[p] != Enrolled::None || in_school(w.activity()[p]);
        let retired = w.activity()[p] == Activity::Retired;
        if !leaver {
            assert!(
                !was || is || retired,
                "{:?}: person {} ({} months) left school without being a leaver",
                w.date(),
                keys[p],
                ages[p]
            );
            continue;
        }
        assert!(
            !is,
            "{:?}: leaver {} ({} months) is still {:?}, {:?}",
            w.date(),
            keys[p],
            ages[p],
            w.enrolled()[p],
            w.activity()[p]
        );
        let wt = u64::from(weights[p]);
        match w.activity()[p] {
            Activity::Unemployed => out.seeking += wt,
            Activity::InactiveOther => out.inactive += wt,
            Activity::Employed => out.employed += wt,
            Activity::Retired => out.retired += wt,
            other => panic!("{:?}: leaver {} became {other:?}", w.date(), keys[p]),
        }
    }
    out
}

/// **AC-EDU-08** `[unit]` The `EducationYear` report balances at every June
/// and August close of a 3-year run at 1:1000: for each level, enrolled
/// after = enrolled before − dropouts − completers + entrants admitted;
/// leavers = the persons aged 16 or more at the close among the dropouts,
/// the completers who do not continue, the refused entrants, and the persons
/// not enrolled whose activity was pupil or student before the close
/// (younger ones stay pupils, step 3); all weighted, exact. (Restated in the
/// spec 2026-10-29.)
///
/// Restated 2026-10-31 (the owner's yes for this run): persons are followed
/// by `person_key` (`World::people`), not by row and "age did not fall",
/// because with the People phase compiled in the yearly compaction shifts
/// the rows; and a leaver who is retired after the close is accepted (see
/// below) where the test of 2026-10-29 panicked.
///
/// Readings: "entrants admitted" is `entrants − refused`; "enrolled after"
/// is also the weighted enrolled count of the persons table after the
/// close (residents; with the People phase, alive). The leavers are found
/// in the persons table by snapshots of the day before and after the close
/// (see `leavers_by_snapshot`); the report's leavers, by fate (seeking,
/// other inactive, keeping their job), equal them exactly. A leaver whose
/// activity is retired after the close (the People phase's retirement: a
/// pension) is still a leaver; the spec's edge case says a pension does not
/// change education's activity rule, so the report counts such a leaver
/// under the fate the rule gave them, which the persons table no longer
/// shows: each fate is then exact up to the retired leavers (the
/// table's count ≤ the report's ≤ the table's + the retired leavers), and
/// the total exact. Without the People phase nobody retires and every fate
/// is exact. The not-enrolled pupils and students are a source at the June
/// closes only: step 3 runs at the June close, and the August close makes
/// leavers of refused entrants only. The run covers the closes of June and
/// August 2022, 2023 and 2024.
#[test]
fn ac_edu_08() {
    let mut w = world_2021_default(1000);
    let mut checked = 0;
    while w.months().len() < 33 {
        let snap = snapshot(&w);
        let closes = w.months().len();
        step_checked(&mut w);
        if w.months().len() == closes {
            continue;
        }
        let t = closes;
        if t % 12 != 6 && t % 12 != 8 {
            continue;
        }
        let june = t % 12 == 6;
        let y = w
            .education_year()
            .expect("a report at a June or August close")
            .clone();
        let counted = enrolled_by_level(&w);
        for (l, x) in y.levels.iter().enumerate() {
            assert!(
                x.refused <= x.entrants,
                "month {t}, level {l}: refused above entrants"
            );
            assert_eq!(
                i128::from(x.after),
                i128::from(x.before) - i128::from(x.dropouts) - i128::from(x.completers)
                    + i128::from(x.entrants)
                    - i128::from(x.refused),
                "month {t}, level {l}: {x:?}"
            );
            assert_eq!(x.after, counted[l], "month {t}, level {l}: enrolled after");
            assert!(
                x.continuing_to.iter().sum::<u64>() <= x.completers,
                "month {t}, level {l}: more continuing than completers"
            );
            if !june {
                assert_eq!(
                    (x.dropouts, x.completers),
                    (0, 0),
                    "month {t}: August has no dropouts or completers"
                );
            }
        }
        let found = leavers_by_snapshot(&snap, &w, june);
        for (name, reported, table) in [
            (
                "seeking",
                y.leavers_seeking.iter().sum::<u64>(),
                found.seeking,
            ),
            (
                "inactive",
                y.leavers_inactive.iter().sum::<u64>(),
                found.inactive,
            ),
            (
                "employed",
                y.leavers_employed.iter().sum::<u64>(),
                found.employed,
            ),
        ] {
            assert!(
                table <= reported && reported <= table + found.retired,
                "month {t}: leavers {name} {reported} in the report, {table} in the table (and {} retired)",
                found.retired
            );
        }
        let reported: u64 = y.leavers_seeking.iter().sum::<u64>()
            + y.leavers_inactive.iter().sum::<u64>()
            + y.leavers_employed.iter().sum::<u64>();
        assert_eq!(
            reported,
            found.seeking + found.inactive + found.employed + found.retired,
            "month {t}: the leavers are exactly the persons of 16 or more who left: {found:?}"
        );
        if june {
            assert!(reported > 0, "month {t}: the June close has leavers");
        }
        checked += 1;
    }
    assert_eq!(checked, 6, "three June and three August closes");
}

/// The education hash after three school years (the June close of 2024).
fn hash_after_three_years(mut w: World) -> u64 {
    step_until(&mut w, date(2024, 7, 1));
    w.education_hash()
}

/// **AC-EDU-09** `[unit]` Determinism: two runs with the same seed give the
/// same education hash (FNV-1a 64 over the person count, then each person's
/// `enrolled`, `study_year`, `edu_level` and `activity`, one byte each) after
/// three school years; at 1:1000 a different seed changes it, and AC-EDU-04
/// still holds at 1:100 with seed 7.
///
/// Readings: "seed" is the `World`'s seed of keyed draws (`rng_seed`) on the
/// same scenario. Three school years end with the June close of 2024.
/// Release builds only (three runs of 31 months and one of three years at
/// 1:100).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long runs: cargo test --release -p econ-core --features education --test acceptance education::"
)]
fn ac_edu_09() {
    let a = hash_after_three_years(world_2021_default(1000));
    let b = hash_after_three_years(world_2021_default(1000));
    assert_eq!(a, b, "same seed, same hash");
    let c = hash_after_three_years(world_2021_seed(1000, 7));
    assert_ne!(a, c, "another seed changes the hash");
    let mut w = world_2021_seed(100, 7);
    assert_graduates(&mut w, "seed 7");
}

/// The level index helper covers every level.
#[test]
fn levels_round_trip() {
    for l in 0..7 {
        assert_eq!(level(enrolment(l)), Some(l));
    }
}
