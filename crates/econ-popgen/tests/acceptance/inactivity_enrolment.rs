//! Acceptance tests for AC-POPK-* (spec: society/population-inactivity-enrolment).
//! Written in a test-authoring session from the spec, the API skeleton and the
//! Python reference, before the Rust stage existed (ADR-0014).
//!
//! "The fixtures" are the margin files committed with the pipeline: the
//! Census 2021 margins of stages A to C and the inactivity-and-enrolment
//! margins. "Targets" for enrolment are the school statistics at each sex and
//! age group's enrolment rate, on the synthetic population: count × weight of
//! the group ÷ census population of the group.

// Column tables are indexed in parallel; index loops are the clear way to read them.
#![allow(clippy::needless_range_loop)]

use econ_popgen::{
    Activity, ENROLMENT_LEVELS, EduLevel, Enrolled, INACTIVE_KINDS, InactiveKind,
    InactivityEnrolmentMargins, InactivityError, InactivityParams, PersonAttributes,
    PersonInactivityEnrolment, Population, assign_inactivity_enrolment,
};

use super::support::{
    ATTRIBUTES_GOLDEN, DIFF_SCALES, OTHER_SEED, SCALES, SEED, STATE_GOLDEN, assert_golden,
    assert_no_failures, assert_same_column, column, golden_hash, hex,
};
use super::support_fj::{attrs, person_weights, pop, reference_hash, shorter};
use super::support_k::{
    INACTIVITY_ENROLMENT_GOLDEN, default_link, enrolment_margins, inactivity, range_of,
    reference_k, run,
};

/// Months in a year.
const MONTHS: u16 = 12;
/// A household member younger than this (years) is a young child.
const YOUNG_CHILD_AGE: u16 = 7;
/// Below this age (years) a person is not of working age.
const MIN_WORKING_AGE: u16 = 15;
/// Cells with fewer expected synthetic persons are not checked (AC-POPK-03, 04).
const MIN_EXPECTED: u64 = 50;
/// Index of upper secondary among the levels of the margin table.
const UPPER_SECONDARY: usize = Enrolled::UpperSecondary as usize - 1;
/// Enrolment ages whose level shares are not checked (AC-POPK-04): the pupils of 19.
const UNCHECKED_AGES: [u16; 2] = [19, 20];

const KIND_NAMES: [&str; INACTIVE_KINDS] = ["homemaker", "disabled", "other"];
const LEVEL_NAMES: [&str; ENROLMENT_LEVELS] = [
    "pre-school",
    "primary",
    "lower secondary",
    "upper secondary",
    "post-secondary",
    "tertiary",
    "master and doctorate",
];

/// `|a / a_tot − b / b_tot|` is more than `points` percentage points; an empty
/// side counts as off.
fn share_off(a: u128, a_tot: u128, b: u128, b_tot: u128, points: u128) -> bool {
    if a_tot == 0 || b_tot == 0 {
        return true;
    }
    (a * b_tot).abs_diff(b * a_tot) * 100 > points * a_tot * b_tot
}

/// The households where someone is younger than 7 years.
fn with_young_child(pop: &Population) -> Vec<bool> {
    let mut young = vec![false; pop.households.hh_weight.len()];
    for i in 0..pop.persons.age.len() {
        if pop.persons.age[i] < YOUNG_CHILD_AGE * MONTHS {
            young[pop.persons.household_id[i] as usize] = true;
        }
    }
    young
}

/// Weighted persons other inactive by `[sex][age_band][kind]`.
fn kind_table(
    m: &InactivityEnrolmentMargins,
    pop: &Population,
    k: &PersonInactivityEnrolment,
) -> Vec<u128> {
    let n_b = m.age_bands.len();
    let w = person_weights(pop);
    let mut t = vec![0u128; 2 * n_b * INACTIVE_KINDS];
    for i in 0..w.len() {
        if k.inactive_kind[i] == InactiveKind::None {
            continue;
        }
        let band = range_of(&m.age_bands, pop.persons.age[i] / MONTHS);
        let s = pop.persons.sex[i] as usize;
        t[(s * n_b + band) * INACTIVE_KINDS + k.inactive_kind[i] as usize - 1] += u128::from(w[i]);
    }
    t
}

/// AC-POPK-03 on one result: kind totals by sex within 2%, and kind shares in
/// every sex and age band with at least 50 expected synthetic persons other
/// inactive within 3 points.
fn check_kinds(
    what: &str,
    scale: u32,
    pop: &Population,
    k: &PersonInactivityEnrolment,
    failures: &mut Vec<String>,
) {
    let m = enrolment_margins();
    let n_b = m.age_bands.len();
    let got = kind_table(m, pop, k);
    let want: Vec<u128> = m.inactive_kind.iter().map(|&v| u128::from(v)).collect();
    let cell = |t: &[u128], s: usize, b: usize, kd: usize| t[(s * n_b + b) * INACTIVE_KINDS + kd];
    for s in 0..2 {
        for kd in 0..INACTIVE_KINDS {
            let g: u128 = (0..n_b).map(|b| cell(&got, s, b, kd)).sum();
            let t: u128 = (0..n_b).map(|b| cell(&want, s, b, kd)).sum();
            if g.abs_diff(t) * 100 > 2 * t {
                failures.push(format!(
                    "{what} 1:{scale} sex {s} {}: {g} persons, margin file {t} (2%)",
                    KIND_NAMES[kd]
                ));
            }
        }
        for b in 0..n_b {
            let w_tot: u128 = (0..INACTIVE_KINDS).map(|kd| cell(&want, s, b, kd)).sum();
            if w_tot / u128::from(scale) < u128::from(MIN_EXPECTED) {
                continue;
            }
            let g_tot: u128 = (0..INACTIVE_KINDS).map(|kd| cell(&got, s, b, kd)).sum();
            for kd in 0..INACTIVE_KINDS {
                let (g, w) = (cell(&got, s, b, kd), cell(&want, s, b, kd));
                if share_off(g, g_tot, w, w_tot, 3) {
                    failures.push(format!(
                        "{what} 1:{scale} sex {s} ages {:?} {}: {g} of {g_tot}, margin file {w} of {w_tot} (3 points)",
                        m.age_bands[b], KIND_NAMES[kd]
                    ));
                }
            }
        }
    }
}

/// The weight of every sex and enrolment age group, `[sex][group]`.
fn group_weights(m: &InactivityEnrolmentMargins, pop: &Population) -> Vec<u128> {
    let n_g = m.enrolment_ages.len();
    let w = person_weights(pop);
    let mut out = vec![0u128; 2 * n_g];
    for i in 0..w.len() {
        let g = range_of(&m.enrolment_ages, pop.persons.age[i] / MONTHS);
        out[pop.persons.sex[i] as usize * n_g + g] += u128::from(w[i]);
    }
    out
}

/// Weighted enrolled persons by `[sex][group][level]`.
fn level_table(
    m: &InactivityEnrolmentMargins,
    pop: &Population,
    k: &PersonInactivityEnrolment,
) -> Vec<u128> {
    let n_g = m.enrolment_ages.len();
    let w = person_weights(pop);
    let mut t = vec![0u128; 2 * n_g * ENROLMENT_LEVELS];
    for i in 0..w.len() {
        if k.enrolled[i] == Enrolled::None {
            continue;
        }
        let g = range_of(&m.enrolment_ages, pop.persons.age[i] / MONTHS);
        let s = pop.persons.sex[i] as usize;
        t[(s * n_g + g) * ENROLMENT_LEVELS + k.enrolled[i] as usize - 1] += u128::from(w[i]);
    }
    t
}

/// AC-POPK-04 on one result: national counts by level (all but upper
/// secondary) within 6% of their targets, and level shares in every sex and
/// age group with at least 50 expected synthetic enrolled persons, except
/// ages 19 and 20, within 3 points of the school statistics.
fn check_levels(
    what: &str,
    scale: u32,
    pop: &Population,
    k: &PersonInactivityEnrolment,
    failures: &mut Vec<String>,
) {
    let m = enrolment_margins();
    let n_g = m.enrolment_ages.len();
    let got = level_table(m, pop, k);
    let weight = group_weights(m, pop);
    let count = |s: usize, g: usize, lv: usize| {
        u128::from(m.enrolment[(s * n_g + g) * ENROLMENT_LEVELS + lv])
    };
    let have = |s: usize, g: usize, lv: usize| got[(s * n_g + g) * ENROLMENT_LEVELS + lv];
    for lv in 0..ENROLMENT_LEVELS {
        if lv == UPPER_SECONDARY {
            continue;
        }
        let mut target = 0f64;
        let mut g_total = 0u128;
        for s in 0..2 {
            for g in 0..n_g {
                g_total += have(s, g, lv);
                let p = m.population[s * n_g + g];
                if p > 0 {
                    target += count(s, g, lv) as f64 * weight[s * n_g + g] as f64 / p as f64;
                }
            }
        }
        if (g_total as f64 - target).abs() > 0.06 * target {
            failures.push(format!(
                "{what} 1:{scale} {}: {g_total} enrolled, target {target:.0} (6%)",
                LEVEL_NAMES[lv]
            ));
        }
    }
    for s in 0..2 {
        for g in 0..n_g {
            if UNCHECKED_AGES.contains(&m.enrolment_ages[g].0) {
                continue;
            }
            let w_tot: u128 = (0..ENROLMENT_LEVELS).map(|lv| count(s, g, lv)).sum();
            if w_tot / u128::from(scale) < u128::from(MIN_EXPECTED) {
                continue;
            }
            let g_tot: u128 = (0..ENROLMENT_LEVELS).map(|lv| have(s, g, lv)).sum();
            for lv in 0..ENROLMENT_LEVELS {
                if share_off(have(s, g, lv), g_tot, count(s, g, lv), w_tot, 3) {
                    failures.push(format!(
                        "{what} 1:{scale} sex {s} ages {:?} {}: {} of {g_tot}, school statistics {} of {w_tot} (3 points)",
                        m.enrolment_ages[g], LEVEL_NAMES[lv], have(s, g, lv), count(s, g, lv)
                    ));
                }
            }
        }
    }
}

/// AC-POPK-01 [unit] Structure, at each of the three scales: a person has a kind exactly when their activity is other inactive; every student is enrolled; a pupil is left not enrolled only in a sex and enrolment age group whose enrolled weight is at least its targets less one person's weight, and at 1:1000 at least one pupil is left not enrolled; nobody retired is enrolled; a child (activity) is enrolled only in pre-school; everyone enrolled at post-secondary or above has `edu_level` ≥ 1; everyone enrolled whose activity is employed, unemployed or other inactive is 15 or older.
#[test]
fn ac_popk_01() {
    let m = enrolment_margins();
    let n_g = m.enrolment_ages.len();
    for scale in SCALES {
        let (pop, a, k) = (pop(scale), attrs(scale), inactivity(scale));
        let n = pop.persons.household_id.len();
        assert_eq!(k.inactive_kind.len(), n, "1:{scale}: one kind per person");
        assert_eq!(k.enrolled.len(), n, "1:{scale}: one level per person");
        let w = person_weights(pop);
        let weight = group_weights(m, pop);
        // Enrolled weight of every sex and enrolment age group.
        let mut enrolled = vec![0u128; 2 * n_g];
        for i in 0..n {
            if k.enrolled[i] != Enrolled::None {
                let g = range_of(&m.enrolment_ages, pop.persons.age[i] / MONTHS);
                enrolled[pop.persons.sex[i] as usize * n_g + g] += u128::from(w[i]);
            }
        }
        let mut pupils_left = 0usize;
        for i in 0..n {
            let (age, act, edu, kind, lv) = (
                pop.persons.age[i],
                a.activity[i],
                a.edu_level[i],
                k.inactive_kind[i],
                k.enrolled[i],
            );
            let who =
                format!("1:{scale}: person {i} ({age} months, {act:?}, {edu:?}, {kind:?}, {lv:?})");
            assert_eq!(
                kind != InactiveKind::None,
                act == Activity::InactiveOther,
                "{who}: a kind exactly when other inactive"
            );
            match act {
                Activity::Student => assert_ne!(lv, Enrolled::None, "{who}: a student is enrolled"),
                Activity::Retired => assert_eq!(lv, Enrolled::None, "{who}: retired"),
                Activity::Child => assert!(
                    matches!(lv, Enrolled::None | Enrolled::PreSchool),
                    "{who}: a child is enrolled only in pre-school"
                ),
                Activity::Employed | Activity::Unemployed | Activity::InactiveOther => {
                    if lv != Enrolled::None {
                        assert!(age >= MIN_WORKING_AGE * MONTHS, "{who}: enrolled under 15");
                    }
                }
                Activity::Pupil => {
                    if lv == Enrolled::None {
                        pupils_left += 1;
                        // Enrolled weight ≥ Σ count × W / P − this pupil's weight.
                        let s = pop.persons.sex[i] as usize;
                        let g = range_of(&m.enrolment_ages, age / MONTHS);
                        let sg = s * n_g + g;
                        let counts: u128 = (0..ENROLMENT_LEVELS)
                            .map(|lv| u128::from(m.enrolment[sg * ENROLMENT_LEVELS + lv]))
                            .sum();
                        let p = u128::from(m.population[sg]);
                        assert!(
                            (enrolled[sg] + u128::from(w[i])) * p >= counts * weight[sg],
                            "{who}: a pupil left out of a group whose enrolled weight {} is below its targets {counts} × {} / {p} by more than one person",
                            enrolled[sg],
                            weight[sg]
                        );
                    }
                }
            }
            if lv >= Enrolled::PostSecondary {
                assert!(
                    edu >= EduLevel::Isced3,
                    "{who}: post-secondary or above without upper secondary"
                );
            }
        }
        if scale == 1000 {
            assert!(pupils_left > 0, "1:{scale}: every pupil is enrolled");
        }
    }
}

/// AC-POPK-02 [unit] The stage changes nothing before it: the population's state hash and the attribute hash are those of their own golden tests.
#[test]
fn ac_popk_02() {
    for scale in SCALES {
        let (pop, a) = (pop(scale).clone(), attrs(scale).clone());
        let before = (pop.clone(), a.clone());
        assign_inactivity_enrolment(&pop, &a, enrolment_margins(), &InactivityParams::new(SEED))
            .expect("fixture population gets kinds and enrolment");
        assert_eq!(pop, before.0, "1:{scale}: population changed");
        assert_eq!(a, before.1, "1:{scale}: attributes changed");
        for (what, hash, file) in [
            ("state", pop.state_hash(), STATE_GOLDEN),
            ("attribute", a.state_hash(), ATTRIBUTES_GOLDEN),
        ] {
            assert_eq!(
                hex(hash),
                golden_hash(file, scale),
                "{what} hash at 1:{scale}"
            );
        }
    }
}

/// AC-POPK-03 [unit] At each of the three scales, each kind's national weighted total by sex is within 2% of the margin file's total, and in every sex and age band with at least 50 expected synthetic persons other inactive, each kind's share is within 3 points of its share in the margin file.
#[test]
fn ac_popk_03() {
    let mut failures = Vec::new();
    for scale in SCALES {
        check_kinds(
            "default",
            scale,
            pop(scale),
            inactivity(scale),
            &mut failures,
        );
    }
    assert_no_failures(&failures);
}

/// AC-POPK-04 [unit] At each of the three scales, the national weighted count of pre-school, primary, lower secondary, post-secondary, tertiary, and master and doctorate is each within 6% of its target (upper secondary is not checked: the pupils of 19, see Edge cases); and in every sex and enrolment age group with at least 50 expected synthetic enrolled persons, except ages 19 and 20, each level's share of the enrolled is within 3 points of its share in the school statistics.
#[test]
fn ac_popk_04() {
    let mut failures = Vec::new();
    for scale in SCALES {
        check_levels(
            "default",
            scale,
            pop(scale),
            inactivity(scale),
            &mut failures,
        );
    }
    assert_no_failures(&failures);
}

/// AC-POPK-05 [unit] With `homemaker_child_link` = 1 the share of homemakers who live with a child younger than 7 is at least 10 points above that with 0 (1:1000 and 1:100); with 0 it is within 3 points of the same share among all persons other inactive. AC-POPK-03 holds for 0, 0.5 and 1.
///
/// AC-POPK-03 is checked for 0 and 1 at the criterion's two scales; for 0.5
/// (the default) `ac_popk_03` checks it at the three scales.
#[test]
fn ac_popk_05() {
    let mut failures = Vec::new();
    for scale in [1000, 100] {
        let pop = pop(scale);
        let young = with_young_child(pop);
        let w = person_weights(pop);
        // (weight with a young child, weight) of homemakers, and of everyone other inactive.
        let shares = |k: &PersonInactivityEnrolment| {
            let (mut hm, mut all) = ((0u128, 0u128), (0u128, 0u128));
            for i in 0..w.len() {
                if k.inactive_kind[i] == InactiveKind::None {
                    continue;
                }
                let (wi, y) = (
                    u128::from(w[i]),
                    young[pop.persons.household_id[i] as usize],
                );
                all = (all.0 + if y { wi } else { 0 }, all.1 + wi);
                if k.inactive_kind[i] == InactiveKind::Homemaker {
                    hm = (hm.0 + if y { wi } else { 0 }, hm.1 + wi);
                }
            }
            (hm, all)
        };
        let (none, full) = (run(scale, SEED, 0), run(scale, SEED, 1_000_000));
        let ((h0, t0), all) = shares(&none);
        let ((h1, t1), _) = shares(&full);
        assert!(t0 > 0 && t1 > 0, "1:{scale}: no homemaker");
        // h1 / t1 ≥ h0 / t0 + 10 / 100.
        assert!(
            h1 * t0 * 100 >= h0 * t1 * 100 + 10 * t0 * t1,
            "1:{scale}: homemakers with a young child: {h1} of {t1} with link 1, {h0} of {t0} with link 0; not 10 points apart"
        );
        assert!(
            !share_off(h0, t0, all.0, all.1, 3),
            "1:{scale}: with link 0, {h0} of {t0} homemakers live with a young child; {} of {} of all other inactive",
            all.0,
            all.1
        );
        check_kinds("link 0", scale, pop, &none, &mut failures);
        check_kinds("link 1", scale, pop, &full, &mut failures);
    }
    assert_no_failures(&failures);
}

/// AC-POPK-06 [unit] Same inputs and seed give identical columns (equal hash). A different seed changes who has which kind and level; AC-POPK-03 and AC-POPK-04 still hold.
#[test]
fn ac_popk_06() {
    let scale = 1000;
    let (pop, first) = (pop(scale), inactivity(scale));
    let again = run(scale, SEED, default_link());
    assert_eq!(&again, first);
    assert_eq!(again.state_hash(), first.state_hash());

    let other = run(scale, OTHER_SEED, default_link());
    assert_ne!(other.state_hash(), first.state_hash());
    assert_ne!(
        other.inactive_kind, first.inactive_kind,
        "kinds are the same with another seed"
    );
    assert_ne!(
        other.enrolled, first.enrolled,
        "levels are the same with another seed"
    );
    let mut failures = Vec::new();
    check_kinds("another seed", scale, pop, &other, &mut failures);
    check_levels("another seed", scale, pop, &other, &mut failures);
    assert_no_failures(&failures);
}

/// AC-POPK-07 [golden] The inactivity and enrolment hash of the fixture population at each of the three scales equals the committed golden value.
#[test]
fn ac_popk_07() {
    assert_golden(
        INACTIVITY_ENROLMENT_GOLDEN,
        "inactivity and enrolment hash",
        |scale| inactivity(scale).state_hash(),
    );
}

/// AC-POPK-08 [diff] On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns.
#[test]
fn ac_popk_08() {
    for scale in DIFF_SCALES {
        let k = inactivity(scale);
        let r = reference_k(scale);
        assert_eq!(
            column(r, "household_id").len(),
            pop(scale).persons.household_id.len(),
            "1:{scale}: persons of the reference population"
        );
        let kind: Vec<u64> = k.inactive_kind.iter().map(|&v| v as u64).collect();
        let level: Vec<u64> = k.enrolled.iter().map(|&v| v as u64).collect();
        assert_same_column(scale, "inactive_kind", &kind, &column(r, "inactive_kind"));
        assert_same_column(scale, "enrolled", &level, &column(r, "enrolled"));
        assert_eq!(
            hex(k.state_hash()),
            reference_hash(r, "inactivity_enrolment_hash"),
            "1:{scale} inactivity and enrolment hash"
        );
    }
    assert_eq!(InactiveKind::Other as u8, 3);
    assert_eq!(Enrolled::MasterDoctorate as u8, 7);
}

/// AC-POPK-09 [unit] Tables of the wrong length, the wrong sexes, kinds or levels, no age bands, all counts zero, and attributes of a different length are each rejected with a named error and produce no output.
#[test]
fn ac_popk_09() {
    let m = enrolment_margins();
    let (pop, a) = (pop(1000), attrs(1000));
    let params = InactivityParams::new(SEED);
    let run = |m: &InactivityEnrolmentMargins| assign_inactivity_enrolment(pop, a, m, &params);
    let (n_b, n_g) = (m.age_bands.len(), m.enrolment_ages.len());

    // Tables of the wrong length.
    let tables: [fn(&mut InactivityEnrolmentMargins) -> &mut Vec<u64>; 3] = [
        |m| &mut m.inactive_kind,
        |m| &mut m.enrolment,
        |m| &mut m.population,
    ];
    for t in tables {
        let mut short = m.clone();
        t(&mut short).pop();
        assert_eq!(run(&short), Err(InactivityError::ShapeMismatch));
    }

    // The wrong sexes: one sex, or three.
    let mut one = m.clone();
    one.inactive_kind.truncate(n_b * INACTIVE_KINDS);
    one.enrolment.truncate(n_g * ENROLMENT_LEVELS);
    one.population.truncate(n_g);
    assert_eq!(run(&one), Err(InactivityError::ShapeMismatch));
    let mut three = m.clone();
    three
        .inactive_kind
        .extend_from_within(..n_b * INACTIVE_KINDS);
    three.enrolment.extend_from_within(..n_g * ENROLMENT_LEVELS);
    three.population.extend_from_within(..n_g);
    assert_eq!(run(&three), Err(InactivityError::ShapeMismatch));

    // The wrong kinds (four per row) and the wrong levels (six per row).
    let mut kinds = m.clone();
    kinds.inactive_kind = m
        .inactive_kind
        .chunks_exact(INACTIVE_KINDS)
        .flat_map(|row| row.iter().copied().chain([0]))
        .collect();
    assert_eq!(run(&kinds), Err(InactivityError::ShapeMismatch));
    let mut levels = m.clone();
    levels.enrolment = m
        .enrolment
        .chunks_exact(ENROLMENT_LEVELS)
        .flat_map(|row| row[..ENROLMENT_LEVELS - 1].to_vec())
        .collect();
    assert_eq!(run(&levels), Err(InactivityError::ShapeMismatch));

    // No age bands, no enrolment ages.
    let mut bandless = m.clone();
    bandless.age_bands.clear();
    bandless.inactive_kind.clear();
    assert_eq!(run(&bandless), Err(InactivityError::EmptyTable));
    let mut ageless = m.clone();
    ageless.enrolment_ages.clear();
    ageless.enrolment.clear();
    ageless.population.clear();
    assert_eq!(run(&ageless), Err(InactivityError::EmptyTable));

    // All counts zero.
    let mut no_kinds = m.clone();
    no_kinds.inactive_kind.fill(0);
    assert_eq!(run(&no_kinds), Err(InactivityError::EmptyTable));
    let mut no_pupils = m.clone();
    no_pupils.enrolment.fill(0);
    assert_eq!(run(&no_pupils), Err(InactivityError::EmptyTable));
    // Women other inactive, and no count for any woman (step 1.2).
    let mut no_women = m.clone();
    no_women.inactive_kind[..n_b * INACTIVE_KINDS].fill(0);
    assert_eq!(run(&no_women), Err(InactivityError::NoMargin));

    // Attributes of a different length.
    let other = PersonAttributes {
        edu_level: shorter(&a.edu_level),
        activity: shorter(&a.activity),
    };
    assert_eq!(
        assign_inactivity_enrolment(pop, &other, m, &params),
        Err(InactivityError::ShapeMismatch)
    );
}
