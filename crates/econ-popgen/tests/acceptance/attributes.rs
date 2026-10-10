//! Acceptance tests for AC-POPA-* (spec: society/population-attributes).
//! Written in a test-authoring session from the spec, the API skeleton and the
//! Python reference, before the Rust stage existed (ADR-0014).
//!
//! "The fixtures" are the normalised Census 2021 margin files of Romania:
//! the county margins and the education-and-activity margins.

// Column tables are indexed in parallel; index loops are the clear way to read them.
#![allow(clippy::needless_range_loop)]

use econ_popgen::{
    Activity, AttrError, AttrParams, AttributeMargins, EDU_LEVELS, EduLevel, MARGIN_ACTIVITIES,
    PersonAttributes, Population, STATUSES, assign_attributes,
};

use super::support::{
    ATTRIBUTES_GOLDEN, DIFF_SCALES, OTHER_SEED, SCALES, SEED, STATE_GOLDEN, Tolerances,
    assert_golden, assert_no_failures, assert_same_column, attribute_margins, attributes,
    attributes_at, band_of_year, broad_age, check_family, column, golden_hash, hex, population,
    population_at, reference,
};

/// Tolerance in percent by expected synthetic records behind a cell.
const TOLERANCES: Tolerances = [(1000, 1), (100, 6), (30, 15)];
/// Below this age (years) the census gives no activity and no education level.
const MIN_WORKING_AGE: u16 = 15;

// The census activities, in the order of the activity margin table.
const M_CHILD: usize = 0;
const M_IN_EDUCATION: usize = 1;
const M_EMPLOYED: usize = 2;
const M_UNEMPLOYED: usize = 3;
const M_RETIRED: usize = 4;
const M_INACTIVE_OTHER: usize = 5;
/// Labour status (employed, unemployed, inactive) of each census activity.
const STATUS_OF_MARGIN: [usize; MARGIN_ACTIVITIES] = [2, 2, 0, 1, 2, 2];

/// The census activity of a person: a `pupil` under 15 counts as child;
/// `pupil` and `student` of 15 or more count as in education.
fn margin_activity(activity: Activity, age_months: u16) -> usize {
    match activity {
        Activity::Child => M_CHILD,
        Activity::Pupil if age_months < MIN_WORKING_AGE * 12 => M_CHILD,
        Activity::Pupil | Activity::Student => M_IN_EDUCATION,
        Activity::Employed => M_EMPLOYED,
        Activity::Unemployed => M_UNEMPLOYED,
        Activity::Retired => M_RETIRED,
        Activity::InactiveOther => M_INACTIVE_OTHER,
    }
}

/// Weighted persons in the shape of the two margin tables.
struct Totals {
    /// `[region][sex][year_of_age][margin activity]`
    activity: Vec<u64>,
    /// `[region][sex][age_band][status][edu_level]`
    education: Vec<u64>,
}

fn totals(m: &AttributeMargins, pop: &Population, attrs: &PersonAttributes) -> Totals {
    let (n_r, n_y, n_b) = (
        m.regions.len(),
        usize::from(m.years_of_age),
        m.age_bands.len(),
    );
    let mut t = Totals {
        activity: vec![0; n_r * 2 * n_y * MARGIN_ACTIVITIES],
        education: vec![0; n_r * 2 * n_b * STATUSES * EDU_LEVELS],
    };
    let (hh, p) = (&pop.households, &pop.persons);
    for i in 0..p.household_id.len() {
        let h = p.household_id[i] as usize;
        let w = u64::from(hh.hh_weight[h]);
        let r = usize::from(m.region_of_county[usize::from(hh.hh_county[h])]);
        let rs = r * 2 + p.sex[i] as usize;
        let year = (p.age[i] / 12).min(m.years_of_age - 1);
        let k = margin_activity(attrs.activity[i], p.age[i]);
        t.activity[(rs * n_y + usize::from(year)) * MARGIN_ACTIVITIES + k] += w;
        let b = band_of_year(&m.age_bands, year);
        let cell = (rs * n_b + b) * STATUSES + STATUS_OF_MARGIN[k];
        t.education[cell * EDU_LEVELS + attrs.edu_level[i] as usize] += w;
    }
    t
}

fn activity_dims(m: &AttributeMargins) -> [usize; 4] {
    [
        m.regions.len(),
        2,
        usize::from(m.years_of_age),
        MARGIN_ACTIVITIES,
    ]
}

fn education_dims(m: &AttributeMargins) -> [usize; 5] {
    [m.regions.len(), 2, m.age_bands.len(), STATUSES, EDU_LEVELS]
}

/// The national totals by census activity and by education level.
fn check_national(m: &AttributeMargins, scale: u32, t: &Totals, failures: &mut Vec<String>) {
    check_family(
        &TOLERANCES,
        "national activity",
        scale,
        &activity_dims(m),
        &t.activity,
        &m.activity,
        |k| vec![k[3]],
        failures,
    );
    check_family(
        &TOLERANCES,
        "national education",
        scale,
        &education_dims(m),
        &t.education,
        &m.education,
        |k| vec![k[4]],
        failures,
    );
}

/// AC-POPA-01 [unit] Structure, at each of the three scales: one `edu_level` and one `activity` per person; everyone under 15 is `child` or `pupil` with `edu_level` 0, `child` below `school_age` and `pupil` from it; nobody of 15 or more is `child`; a `student` has `edu_level` ≥ 1; a `pupil` of 15 or more has `edu_level` 0.
#[test]
fn ac_popa_01() {
    let school_age_months = AttrParams::new(SEED).school_age * 12;
    for scale in SCALES {
        let (pop, attrs) = (population(scale), attributes(scale));
        let n = pop.persons.household_id.len();
        assert_eq!(
            attrs.edu_level.len(),
            n,
            "1:{scale}: one edu_level per person"
        );
        assert_eq!(
            attrs.activity.len(),
            n,
            "1:{scale}: one activity per person"
        );
        for i in 0..n {
            let (age, edu, act) = (pop.persons.age[i], attrs.edu_level[i], attrs.activity[i]);
            let who = format!("1:{scale}: person {i} ({age} months, {act:?}, {edu:?})");
            if age < MIN_WORKING_AGE * 12 {
                let expected = if age < school_age_months {
                    Activity::Child
                } else {
                    Activity::Pupil
                };
                assert_eq!(act, expected, "{who}: under 15");
                assert_eq!(edu, EduLevel::Isced0To2, "{who}: under 15");
            } else {
                assert_ne!(act, Activity::Child, "{who}: 15 or more");
            }
            if act == Activity::Student {
                assert!(edu >= EduLevel::Isced3, "{who}: a student");
            }
            if act == Activity::Pupil {
                assert_eq!(edu, EduLevel::Isced0To2, "{who}: a pupil");
            }
        }
    }
}

/// AC-POPA-02 [unit] The stage does not change the population: households and persons are equal before and after, and the state hash still equals the golden value of AC-POP-06.
#[test]
fn ac_popa_02() {
    for scale in SCALES {
        let pop = population(scale).clone();
        let before = pop.clone();
        assign_attributes(&pop, attribute_margins(), &AttrParams::new(SEED))
            .expect("fixture population gets attributes");
        assert_eq!(
            pop.households, before.households,
            "1:{scale}: households changed"
        );
        assert_eq!(pop.persons, before.persons, "1:{scale}: persons changed");
        assert_eq!(
            hex(pop.state_hash()),
            golden_hash(STATE_GOLDEN, scale),
            "state hash at 1:{scale}"
        );
    }
}

/// AC-POPA-03 [unit] At each of the three scales the weighted margins are within the tolerances of the table below.
#[test]
fn ac_popa_03() {
    let m = attribute_margins();
    let bands = &m.age_bands;
    let band = |year: usize| band_of_year(bands, u16::try_from(year).expect("year of age"));
    let mut failures = Vec::new();
    for scale in SCALES {
        let t = totals(m, population(scale), attributes(scale));
        check_national(m, scale, &t, &mut failures);

        let mut activity = |family: &str, key: &dyn Fn(&[usize]) -> Vec<usize>| {
            check_family(
                &TOLERANCES,
                family,
                scale,
                &activity_dims(m),
                &t.activity,
                &m.activity,
                key,
                &mut failures,
            );
        };
        // Cells are [region, sex, year of age, activity].
        activity("national sex x activity", &|k| vec![k[1], k[3]]);
        activity("national age band x activity", &|k| vec![band(k[2]), k[3]]);
        activity("region x activity", &|k| vec![k[0], k[3]]);
        activity("region x sex x broad age x activity", &|k| {
            vec![k[0], k[1], broad_age(bands, band(k[2])), k[3]]
        });
        activity("region x sex x age band x activity", &|k| {
            vec![k[0], k[1], band(k[2]), k[3]]
        });

        let mut education = |family: &str, key: &dyn Fn(&[usize]) -> Vec<usize>| {
            check_family(
                &TOLERANCES,
                family,
                scale,
                &education_dims(m),
                &t.education,
                &m.education,
                key,
                &mut failures,
            );
        };
        // Cells are [region, sex, age band, status, education level].
        education("national sex x education", &|k| vec![k[1], k[4]]);
        education("national age band x education", &|k| vec![k[2], k[4]]);
        education("national status x education", &|k| vec![k[3], k[4]]);
        education("region x education", &|k| vec![k[0], k[4]]);
        education("region x status x education", &|k| vec![k[0], k[3], k[4]]);
        education("region x sex x broad age x education", &|k| {
            vec![k[0], k[1], broad_age(bands, k[2]), k[4]]
        });
        education("region x sex x age band x status x education", &|k| {
            k.to_vec()
        });
    }
    assert_no_failures(&failures);
}

/// AC-POPA-04 [unit] At each of the three scales, for every year of age with at least 100 expected synthetic persons nationally, the share of each census activity among persons of that age is within 5 percentage points of the census share. In particular the share in education is higher at age 15 than at 19, and at 19 than at 24; the share retired is higher at age 69 than at 64, and at 64 than at 59.
#[test]
fn ac_popa_04() {
    const ACTIVITY_NAMES: [&str; MARGIN_ACTIVITIES] = [
        "child",
        "in education",
        "employed",
        "unemployed",
        "retired",
        "other inactive",
    ];
    let m = attribute_margins();
    let n_y = usize::from(m.years_of_age);
    // National persons as [year of age][activity].
    let by_year = |table: &[u64]| {
        let mut out = vec![[0u64; MARGIN_ACTIVITIES]; n_y];
        for (flat, &v) in table.iter().enumerate() {
            out[(flat / MARGIN_ACTIVITIES) % n_y][flat % MARGIN_ACTIVITIES] += v;
        }
        out
    };
    let want = by_year(&m.activity);
    let mut failures = Vec::new();
    for scale in SCALES {
        let t = totals(m, population(scale), attributes(scale));
        let got = by_year(&t.activity);
        let persons = |table: &[[u64; MARGIN_ACTIVITIES]], y: usize| -> u128 {
            u128::from(table[y].iter().sum::<u64>())
        };
        for y in 0..n_y {
            let (g_tot, w_tot) = (persons(&got, y), persons(&want, y));
            // Checked from 100 expected synthetic persons of that age.
            if w_tot / u128::from(scale) < 100 || g_tot == 0 {
                continue;
            }
            for k in 0..MARGIN_ACTIVITIES {
                // |got / g_tot − want / w_tot| ≤ 5 / 100, without division.
                let (g, w) = (u128::from(got[y][k]), u128::from(want[y][k]));
                if (g * w_tot).abs_diff(w * g_tot) * 100 > 5 * g_tot * w_tot {
                    failures.push(format!(
                        "1:{scale} age {y}, {}: {g} of {g_tot} synthetic, {w} of {w_tot} in the census",
                        ACTIVITY_NAMES[k]
                    ));
                }
            }
        }
        // The share of activity `k` is higher at age `a` than at age `b`.
        let mut higher = |k: usize, a: usize, b: usize| {
            let (tot_a, tot_b) = (persons(&got, a), persons(&got, b));
            assert!(
                tot_a > 0 && tot_b > 0,
                "1:{scale}: nobody aged {a} or nobody aged {b}"
            );
            if u128::from(got[a][k]) * tot_b <= u128::from(got[b][k]) * tot_a {
                failures.push(format!(
                    "1:{scale} share {}: {} of {tot_a} at age {a} is not above {} of {tot_b} at age {b}",
                    ACTIVITY_NAMES[k], got[a][k], got[b][k]
                ));
            }
        };
        higher(M_IN_EDUCATION, 15, 19);
        higher(M_IN_EDUCATION, 19, 24);
        higher(M_RETIRED, 69, 64);
        higher(M_RETIRED, 64, 59);
    }
    assert!(
        failures.is_empty(),
        "{} activity shares by year of age are off:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// AC-POPA-05 [unit] Same population, margins and seed give identical attributes (equal attribute hash). A different seed changes who has which attribute, and the national weighted totals by activity and by level stay within the tolerance of AC-POPA-03.
#[test]
fn ac_popa_05() {
    let (m, scale) = (attribute_margins(), 1000);
    let (pop, first) = (population(scale), attributes(scale));
    let again = assign_attributes(pop, m, &AttrParams::new(SEED)).expect("attributes");
    assert_eq!(&again, first);
    assert_eq!(again.state_hash(), first.state_hash());

    let other = assign_attributes(pop, m, &AttrParams::new(OTHER_SEED)).expect("attributes");
    assert_ne!(other.state_hash(), first.state_hash());
    assert_ne!(other.activity, first.activity);
    assert_ne!(other.edu_level, first.edu_level);
    let mut failures = Vec::new();
    check_national(m, scale, &totals(m, pop, &other), &mut failures);
    assert_no_failures(&failures);
}

/// AC-POPA-06 [golden] The attribute hash of the fixture population at each of the three scales equals the committed golden value.
#[test]
fn ac_popa_06() {
    assert_golden(ATTRIBUTES_GOLDEN, "attribute hash", |scale| {
        attributes_at(scale).state_hash()
    });
}

/// AC-POPA-07 [diff] On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical attributes, person for person.
#[test]
fn ac_popa_07() {
    for scale in DIFF_SCALES {
        let attrs = attributes_at(scale);
        let r = reference(scale);
        assert_eq!(
            column(r, "household_id").len(),
            population_at(scale).persons.household_id.len(),
            "1:{scale}: persons of the reference population"
        );
        let edu: Vec<u64> = attrs.edu_level.iter().map(|&e| e as u64).collect();
        let activity: Vec<u64> = attrs.activity.iter().map(|&a| a as u64).collect();
        assert_same_column(scale, "edu_level", &edu, &column(r, "edu_level"));
        assert_same_column(scale, "activity", &activity, &column(r, "activity"));
        let want_hash = r["summary"]["attributes_hash"]
            .as_str()
            .expect("reference attribute hash");
        assert_eq!(
            hex(attrs.state_hash()),
            want_hash,
            "1:{scale} attribute hash"
        );
    }
    assert_eq!(EduLevel::Isced7To8 as u8, 4);
    assert_eq!(Activity::Child as u8, 0);
    assert_eq!(Activity::InactiveOther as u8, 6);
}

/// AC-POPA-08 [unit] Margins with no regions, a table whose length does not match its dimensions, a county index without a region, and margins with nobody in an age band that has synthetic persons are each rejected with a named error and produce no output.
#[test]
fn ac_popa_08() {
    let m = attribute_margins();
    let pop = population(1000);
    let params = AttrParams::new(SEED);

    let mut empty = m.clone();
    empty.regions.clear();
    empty.activity.clear();
    empty.education.clear();
    assert_eq!(
        assign_attributes(pop, &empty, &params),
        Err(AttrError::EmptyTable)
    );

    let mut short = m.clone();
    short.education.pop();
    assert_eq!(
        assign_attributes(pop, &short, &params),
        Err(AttrError::ShapeMismatch)
    );
    let mut short = m.clone();
    short.activity.pop();
    assert_eq!(
        assign_attributes(pop, &short, &params),
        Err(AttrError::ShapeMismatch)
    );

    // County 5 points past the last region.
    let mut lost = m.clone();
    lost.region_of_county[5] = u8::try_from(m.regions.len()).expect("few regions");
    assert_eq!(
        assign_attributes(pop, &lost, &params),
        Err(AttrError::UnknownRegion { county: 5 })
    );

    // The census has no man aged 30 to 34 in the fourth region; the population has.
    let (region, sex) = (3, 1);
    let band = band_of_year(&m.age_bands, 30);
    let (from, to) = m.age_bands[band];
    let (n_y, n_b) = (usize::from(m.years_of_age), m.age_bands.len());
    let rs = region * 2 + sex;
    let mut nobody = m.clone();
    for year in from..to {
        let row = (rs * n_y + usize::from(year)) * MARGIN_ACTIVITIES;
        nobody.activity[row..row + MARGIN_ACTIVITIES].fill(0);
    }
    let cell = (rs * n_b + band) * STATUSES * EDU_LEVELS;
    nobody.education[cell..cell + STATUSES * EDU_LEVELS].fill(0);
    assert_eq!(
        assign_attributes(pop, &nobody, &params),
        Err(AttrError::NoMargin {
            region: m.regions[region].clone()
        })
    );
}
