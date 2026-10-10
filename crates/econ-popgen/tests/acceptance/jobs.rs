//! Acceptance tests for AC-POPJ-* (spec: society/population-jobs).
//! Written in a test-authoring session from the spec, the API skeleton and the
//! Python reference, before the Rust stage existed (ADR-0014).
//!
//! "The fixtures" are the normalised Census 2021 margin files of Romania:
//! the county margins, the education-and-activity margins and the jobs margins
//! (with the EU Labour Force Survey patterns).

// Column tables are indexed in parallel; index loops are the clear way to read them.
#![allow(clippy::needless_range_loop)]

use std::borrow::Cow;
use std::sync::OnceLock;

use econ_popgen::{
    Activity, EduLevel, INDUSTRY_GROUPS, JobError, JobMargins, JobParams, KINDS, NOT_EMPLOYED,
    OCCUPATIONS, PersonAttributes, PersonJobs, Population, assign_jobs,
};

use super::support::{
    ATTRIBUTES_GOLDEN, DIFF_SCALES, JOBS_GOLDEN, OTHER_SEED, SCALES, SEED, STATE_GOLDEN,
    Tolerances, assert_golden, assert_no_failures, assert_same_column, attributes, attributes_at,
    band_of_year, check_family, column, golden_hash, hex, job_margins, population, population_at,
    reference,
};

/// Tolerance in percent by expected synthetic records behind a cell.
const TOLERANCES: Tolerances = [(1000, 1), (100, 10), (30, 25)];

// Occupations (ISCO-08 major groups) and industry groups named by AC-POPJ-04.
const PROFESSIONALS: u8 = 2;
const SKILLED_AGRICULTURAL: u8 = 6;
const ELEMENTARY: u8 = 9;
const AGRICULTURE: u8 = 0;
const PUBLIC_EDUCATION_HEALTH: usize = 8;

/// Stage D on the fixture population at one of the three scales (run once).
fn jobs(scale: u32) -> &'static PersonJobs {
    static JOBS: [OnceLock<PersonJobs>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let i = SCALES
        .iter()
        .position(|&s| s == scale)
        .expect("one of the three scales");
    JOBS[i].get_or_init(|| {
        assign_jobs(
            population(scale),
            attributes(scale),
            job_margins(),
            &JobParams::new(SEED),
        )
        .expect("fixture population gets jobs")
    })
}

/// Stage D on the fixture population at any scale.
fn jobs_at(scale: u32) -> Cow<'static, PersonJobs> {
    if SCALES.contains(&scale) {
        Cow::Borrowed(jobs(scale))
    } else {
        Cow::Owned(
            assign_jobs(
                &population_at(scale),
                &attributes_at(scale),
                job_margins(),
                &JobParams::new(SEED),
            )
            .expect("fixture population gets jobs"),
        )
    }
}

/// Weighted employed persons in the shape of the two census tables.
struct Totals {
    /// `[region][sex][age_band][kind][industry_group]`
    by_industry_group: Vec<u64>,
    /// `[region][sex][age_band][kind][occupation]`
    by_occupation: Vec<u64>,
}

fn totals(m: &JobMargins, pop: &Population, jobs: &PersonJobs) -> Totals {
    let (n_r, n_b) = (m.regions.len(), m.age_bands.len());
    let mut t = Totals {
        by_industry_group: vec![0; n_r * 2 * n_b * KINDS * INDUSTRY_GROUPS],
        by_occupation: vec![0; n_r * 2 * n_b * KINDS * OCCUPATIONS],
    };
    let (hh, p) = (&pop.households, &pop.persons);
    for i in 0..p.household_id.len() {
        if jobs.employment_status[i] == 0 {
            continue;
        }
        let h = p.household_id[i] as usize;
        let w = u64::from(hh.hh_weight[h]);
        let r = usize::from(m.region_of_county[usize::from(hh.hh_county[h])]);
        let b = band_of_year(&m.age_bands, p.age[i] / 12);
        let kind = usize::from(jobs.employment_status[i]) - 1;
        let cell = ((r * 2 + p.sex[i] as usize) * n_b + b) * KINDS + kind;
        t.by_industry_group[cell * INDUSTRY_GROUPS + usize::from(jobs.industry_group[i])] += w;
        t.by_occupation[cell * OCCUPATIONS + usize::from(jobs.occupation[i])] += w;
    }
    t
}

fn group_dims(m: &JobMargins) -> [usize; 5] {
    [
        m.regions.len(),
        2,
        m.age_bands.len(),
        KINDS,
        INDUSTRY_GROUPS,
    ]
}

fn occupation_dims(m: &JobMargins) -> [usize; 5] {
    [m.regions.len(), 2, m.age_bands.len(), KINDS, OCCUPATIONS]
}

/// One family of margins of the industry-group table (which also gives the
/// census counts by status in employment) or of the occupation table.
fn check_table(
    m: &JobMargins,
    scale: u32,
    t: &Totals,
    occupation: bool,
    family: &str,
    key: &dyn Fn(&[usize]) -> Vec<usize>,
    failures: &mut Vec<String>,
) {
    if occupation {
        check_family(
            &TOLERANCES,
            family,
            scale,
            &occupation_dims(m),
            &t.by_occupation,
            &m.by_occupation,
            key,
            failures,
        );
    } else {
        check_family(
            &TOLERANCES,
            family,
            scale,
            &group_dims(m),
            &t.by_industry_group,
            &m.by_industry_group,
            key,
            failures,
        );
    }
}

/// The national totals by status in employment, occupation and industry group.
fn check_national(m: &JobMargins, scale: u32, t: &Totals, failures: &mut Vec<String>) {
    // Cells are [region, sex, age band, status in employment, industry group or occupation].
    check_table(
        m,
        scale,
        t,
        false,
        "national status",
        &|k| vec![k[3]],
        failures,
    );
    check_table(
        m,
        scale,
        t,
        false,
        "national industry group",
        &|k| vec![k[4]],
        failures,
    );
    check_table(
        m,
        scale,
        t,
        true,
        "national occupation",
        &|k| vec![k[4]],
        failures,
    );
}

/// AC-POPJ-01 [unit] Structure, at each of the three scales: one value of each column per person; a person is employed in Stage C exactly when `employment_status` is 1 to 4, and then `occupation` and `industry_group` are 0 to 9; otherwise the three values are 0, 255, 255.
#[test]
fn ac_popj_01() {
    for scale in SCALES {
        let (attrs, jobs) = (attributes(scale), jobs(scale));
        let n = population(scale).persons.household_id.len();
        assert_eq!(
            jobs.employment_status.len(),
            n,
            "1:{scale}: one status per person"
        );
        assert_eq!(
            jobs.occupation.len(),
            n,
            "1:{scale}: one occupation per person"
        );
        assert_eq!(
            jobs.industry_group.len(),
            n,
            "1:{scale}: one industry group per person"
        );
        for i in 0..n {
            let got = (
                jobs.employment_status[i],
                jobs.occupation[i],
                jobs.industry_group[i],
            );
            if attrs.activity[i] == Activity::Employed {
                assert!(
                    (1..=4).contains(&got.0)
                        && usize::from(got.1) < OCCUPATIONS
                        && usize::from(got.2) < INDUSTRY_GROUPS,
                    "1:{scale}: employed person {i} has (status, occupation, industry group) {got:?}"
                );
            } else {
                assert_eq!(
                    got,
                    (0, NOT_EMPLOYED, NOT_EMPLOYED),
                    "1:{scale}: person {i} ({:?}) is not employed",
                    attrs.activity[i]
                );
            }
        }
    }
    assert_eq!(NOT_EMPLOYED, 255);
}

/// AC-POPJ-02 [unit] The stage changes neither the population nor the Stage C attributes: the state hash and the attribute hash are those of AC-POP-06 and AC-POPA-06.
#[test]
fn ac_popj_02() {
    for scale in SCALES {
        let (pop, attrs) = (population(scale).clone(), attributes(scale).clone());
        let (pop_before, attrs_before) = (pop.clone(), attrs.clone());
        assign_jobs(&pop, &attrs, job_margins(), &JobParams::new(SEED))
            .expect("fixture population gets jobs");
        assert_eq!(pop, pop_before, "1:{scale}: population changed");
        assert_eq!(attrs, attrs_before, "1:{scale}: attributes changed");
        assert_eq!(
            hex(pop.state_hash()),
            golden_hash(STATE_GOLDEN, scale),
            "state hash at 1:{scale}"
        );
        assert_eq!(
            hex(attrs.state_hash()),
            golden_hash(ATTRIBUTES_GOLDEN, scale),
            "attribute hash at 1:{scale}"
        );
    }
}

/// AC-POPJ-03 [unit] At each of the three scales the weighted margins are within the tolerances of the table below.
#[test]
fn ac_popj_03() {
    let m = job_margins();
    let mut failures = Vec::new();
    for scale in SCALES {
        let t = totals(m, population(scale), jobs(scale));
        check_national(m, scale, &t, &mut failures);
        // Cells are [region, sex, age band, status in employment, industry group or occupation].
        let mut status = |family: &str, key: &dyn Fn(&[usize]) -> Vec<usize>| {
            check_table(m, scale, &t, false, family, key, &mut failures);
        };
        status("national sex x status", &|k| vec![k[1], k[3]]);
        status("region x status", &|k| vec![k[0], k[3]]);
        status("region x sex x age band x status", &|k| k[..4].to_vec());
        for (occupation, what) in [(false, "industry group"), (true, "occupation")] {
            let mut table = |family: &str, key: &dyn Fn(&[usize]) -> Vec<usize>| {
                let family = format!("{family} x {what}");
                check_table(m, scale, &t, occupation, &family, key, &mut failures);
            };
            table("national sex", &|k| vec![k[1], k[4]]);
            table("national status", &|k| vec![k[3], k[4]]);
            table("national age band", &|k| vec![k[2], k[4]]);
            table("region", &|k| vec![k[0], k[4]]);
            table("region x sex", &|k| vec![k[0], k[1], k[4]]);
            table("region x sex x age band x status", &|k| k.to_vec());
        }
    }
    assert_no_failures(&failures);
}

/// AC-POPJ-04 [unit] The links follow the patterns, at each of the three scales, nationally: more than 70% of professionals have `edu_level` ≥ 3; fewer than 20% of persons in elementary occupations do; more than 60% of skilled agricultural workers are in agriculture; the largest industry group of professionals is public administration, education and health.
#[test]
fn ac_popj_04() {
    for scale in SCALES {
        let (pop, attrs, jobs) = (population(scale), attributes(scale), jobs(scale));
        let (mut professionals, mut professional_graduates) = (0u64, 0u64);
        let (mut elementary, mut elementary_graduates) = (0u64, 0u64);
        let (mut farmers, mut farmers_in_agriculture) = (0u64, 0u64);
        let mut professionals_by_group = [0u64; INDUSTRY_GROUPS];
        for i in 0..pop.persons.household_id.len() {
            let w = u64::from(pop.households.hh_weight[pop.persons.household_id[i] as usize]);
            let graduate = attrs.edu_level[i] >= EduLevel::Isced5To6;
            match jobs.occupation[i] {
                PROFESSIONALS => {
                    professionals += w;
                    professional_graduates += if graduate { w } else { 0 };
                    professionals_by_group[usize::from(jobs.industry_group[i])] += w;
                }
                ELEMENTARY => {
                    elementary += w;
                    elementary_graduates += if graduate { w } else { 0 };
                }
                SKILLED_AGRICULTURAL => {
                    farmers += w;
                    farmers_in_agriculture += if jobs.industry_group[i] == AGRICULTURE {
                        w
                    } else {
                        0
                    };
                }
                _ => {}
            }
        }
        assert!(
            professionals > 0 && elementary > 0 && farmers > 0,
            "1:{scale}: an occupation is empty"
        );
        assert!(
            professional_graduates * 100 > professionals * 70,
            "1:{scale}: {professional_graduates} of {professionals} professionals have edu_level >= 3"
        );
        assert!(
            elementary_graduates * 100 < elementary * 20,
            "1:{scale}: {elementary_graduates} of {elementary} persons in elementary occupations have edu_level >= 3"
        );
        assert!(
            farmers_in_agriculture * 100 > farmers * 60,
            "1:{scale}: {farmers_in_agriculture} of {farmers} skilled agricultural workers are in agriculture"
        );
        let largest = professionals_by_group[PUBLIC_EDUCATION_HEALTH];
        for (group, &persons) in professionals_by_group.iter().enumerate() {
            assert!(
                group == PUBLIC_EDUCATION_HEALTH || persons < largest,
                "1:{scale}: professionals by industry group {professionals_by_group:?}: group {group} is not below public administration, education and health"
            );
        }
    }
}

/// AC-POPJ-05 [unit] Same inputs and seed give identical columns (equal jobs hash). A different seed changes who has which job, and the national totals by status, occupation and industry group stay within the tolerance of AC-POPJ-03.
#[test]
fn ac_popj_05() {
    let (m, scale) = (job_margins(), 1000);
    let (pop, attrs, first) = (population(scale), attributes(scale), jobs(scale));
    let again = assign_jobs(pop, attrs, m, &JobParams::new(SEED)).expect("jobs");
    assert_eq!(&again, first);
    assert_eq!(again.state_hash(), first.state_hash());

    let other = assign_jobs(pop, attrs, m, &JobParams::new(OTHER_SEED)).expect("jobs");
    assert_ne!(other.state_hash(), first.state_hash());
    assert_ne!(other.employment_status, first.employment_status);
    assert_ne!(other.occupation, first.occupation);
    assert_ne!(other.industry_group, first.industry_group);
    let mut failures = Vec::new();
    check_national(m, scale, &totals(m, pop, &other), &mut failures);
    assert_no_failures(&failures);
}

/// AC-POPJ-06 [golden] The jobs hash of the fixture population at each of the three scales equals the committed golden value.
#[test]
fn ac_popj_06() {
    assert_golden(JOBS_GOLDEN, "jobs hash", |scale| {
        jobs_at(scale).state_hash()
    });
}

/// AC-POPJ-07 [diff] On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns, person for person.
#[test]
fn ac_popj_07() {
    for scale in DIFF_SCALES {
        let jobs = jobs_at(scale);
        let r = reference(scale);
        let wide = |column: &[u8]| -> Vec<u64> { column.iter().map(|&x| u64::from(x)).collect() };
        for (name, got) in [
            ("employment_status", wide(&jobs.employment_status)),
            ("occupation", wide(&jobs.occupation)),
            ("industry_group", wide(&jobs.industry_group)),
        ] {
            assert_same_column(scale, name, &got, &column(r, name));
        }
        let want_hash = r["summary"]["jobs_hash"]
            .as_str()
            .expect("reference jobs hash");
        assert_eq!(hex(jobs.state_hash()), want_hash, "1:{scale} jobs hash");
    }
}

/// AC-POPJ-08 [unit] Margins with no regions, a table whose length does not match its dimensions, a county index without a region, attributes of a different length, and margins with nobody employed in a region and sex that has employed synthetic persons are each rejected with a named error and produce no output.
#[test]
fn ac_popj_08() {
    let m = job_margins();
    let (pop, attrs) = (population(1000), attributes(1000));
    let params = JobParams::new(SEED);

    let mut empty = m.clone();
    empty.regions.clear();
    empty.by_industry_group.clear();
    empty.by_occupation.clear();
    assert_eq!(
        assign_jobs(pop, attrs, &empty, &params),
        Err(JobError::EmptyTable)
    );

    let shortened: [fn(&mut JobMargins); 4] = [
        |m| {
            m.by_industry_group.pop();
        },
        |m| {
            m.by_occupation.pop();
        },
        |m| {
            m.pattern_edu_occupation.pop();
        },
        |m| {
            m.pattern_occupation_industry_group.pop();
        },
    ];
    for shorten in shortened {
        let mut short = m.clone();
        shorten(&mut short);
        assert_eq!(
            assign_jobs(pop, attrs, &short, &params),
            Err(JobError::ShapeMismatch)
        );
    }

    // County 5 points past the last region.
    let mut lost = m.clone();
    lost.region_of_county[5] = u8::try_from(m.regions.len()).expect("few regions");
    assert_eq!(
        assign_jobs(pop, attrs, &lost, &params),
        Err(JobError::UnknownRegion { county: 5 })
    );

    // Attributes of a population with one person fewer.
    let mut fewer: PersonAttributes = attrs.clone();
    fewer.edu_level.pop();
    fewer.activity.pop();
    assert_eq!(
        assign_jobs(pop, &fewer, m, &params),
        Err(JobError::ShapeMismatch)
    );

    // The census has no employed man in the fourth region; the population has.
    let (region, sex) = (3, 1);
    let per_sex = m.age_bands.len() * KINDS;
    let mut nobody = m.clone();
    let from = (region * 2 + sex) * per_sex;
    nobody.by_industry_group[from * INDUSTRY_GROUPS..(from + per_sex) * INDUSTRY_GROUPS].fill(0);
    nobody.by_occupation[from * OCCUPATIONS..(from + per_sex) * OCCUPATIONS].fill(0);
    assert_eq!(
        assign_jobs(pop, attrs, &nobody, &params),
        Err(JobError::NoMargin {
            region: m.regions[region].clone()
        })
    );
}
