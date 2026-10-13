//! Acceptance tests for AC-POPE-* (spec: society/population-employers).
//! Written in a test-authoring session from the spec, the API skeleton and the
//! Python reference, before the Rust stage existed (ADR-0014).
//!
//! "The fixtures" are the margin files and the firm-unit file committed with
//! the pipeline: the Census 2021 margins of stages A to D, the earnings
//! margins and the firm units.
//!
//! AC-POPE-04 has no test yet. As written it does not hold on the Python
//! reference: with `employer_wage_link` = 0 the mean wages in micro, small and
//! large units are about 13% apart at 1:1000 and 1:100, not within 10%
//! (measured on 2026-10-11). The criterion is with the owner.

// Column tables are indexed in parallel; index loops are the clear way to read them.
#![allow(clippy::needless_range_loop)]

use std::sync::OnceLock;

use econ_popgen::{
    EmployerError, EmployerParams, FirmUnits, INDUSTRY_GROUPS, NO_EMPLOYER, NO_INDUSTRY,
    PersonEmployers, PersonJobs, PersonWages, UnitSize, assign_employers,
};

use super::support::{
    DIFF_SCALES, JOBS_GOLDEN, OTHER_SEED, SCALES, SEED, STATE_GOLDEN, assert_golden,
    assert_no_failures, assert_same_column, column, golden_hash, hex,
};
use super::support_fj::{
    Cache, EMPLOYEE, EMPLOYERS_GOLDEN, NO_JOB, WAGES_GOLDEN, firm_units, jobs, person_weights, pop,
    ratio, reference_all, reference_hash, shorter, slot, wages,
};

/// Stage J on the fixture population at a cached scale, with the wages of stage F.
fn employers(scale: u32) -> &'static PersonEmployers {
    static EMPLOYERS: Cache<PersonEmployers> = [const { OnceLock::new() }; 4];
    EMPLOYERS[slot(scale)].get_or_init(|| run(scale, Some(wages(scale)), SEED))
}

fn run(scale: u32, wages: Option<&PersonWages>, seed: u64) -> PersonEmployers {
    assign_employers(
        pop(scale),
        jobs(scale),
        wages,
        firm_units(),
        &EmployerParams::new(seed),
    )
    .expect("fixture population gets employers")
}

/// AC-POPE-03: in every industry group, each unit's share of the group's
/// employees is within 3 points of its share of the group's persons employed.
fn check_unit_shares(
    what: &str,
    firms: &FirmUnits,
    jobs: &PersonJobs,
    employers: &PersonEmployers,
    w: &[u64],
    failures: &mut Vec<String>,
) {
    let mut employees = vec![0u128; firms.units.len()];
    for i in 0..w.len() {
        if jobs.employment_status[i] == EMPLOYEE {
            employees[usize::from(employers.employer[i])] += u128::from(w[i]);
        }
    }
    for group in 0..INDUSTRY_GROUPS {
        let units: Vec<usize> = (0..firms.units.len())
            .filter(|&k| {
                usize::from(firms.industry_group[usize::from(firms.units[k].industry)]) == group
            })
            .collect();
        let got: u128 = units.iter().map(|&k| employees[k]).sum();
        let want: u128 = units
            .iter()
            .map(|&k| u128::from(firms.units[k].persons_employed))
            .sum();
        assert!(got > 0, "{what}: industry group {group} has no employee");
        assert!(want > 0, "{what}: industry group {group} has no unit");
        for &k in &units {
            let (share, target) = (
                ratio(employees[k], got),
                ratio(u128::from(firms.units[k].persons_employed), want),
            );
            if (share - target).abs() > 0.03 {
                failures.push(format!(
                    "{what} unit {k} of industry group {group}: {share:.4} of the group's employees, {target:.4} of its persons employed"
                ));
            }
        }
    }
}

/// AC-POPE-01 [unit] Structure, at each of the three scales: a person has an industry and an employer exactly when employed; the employer's industry is the person's industry; that industry's group is the person's `industry_group`; employers, own-account and family workers are in micro units.
#[test]
fn ac_pope_01() {
    let firms = firm_units();
    for scale in SCALES {
        let (jobs, employers) = (jobs(scale), employers(scale));
        let n = pop(scale).persons.household_id.len();
        assert_eq!(
            employers.industry.len(),
            n,
            "1:{scale}: one industry per person"
        );
        assert_eq!(
            employers.employer.len(),
            n,
            "1:{scale}: one employer per person"
        );
        for i in 0..n {
            let (status, industry, employer) = (
                jobs.employment_status[i],
                employers.industry[i],
                employers.employer[i],
            );
            if status == NO_JOB {
                assert_eq!(
                    (industry, employer),
                    (NO_INDUSTRY, NO_EMPLOYER),
                    "1:{scale}: person {i} is not employed"
                );
                continue;
            }
            assert!(
                usize::from(industry) < firms.industry_group.len(),
                "1:{scale}: employed person {i} has industry {industry}"
            );
            assert!(
                usize::from(employer) < firms.units.len(),
                "1:{scale}: employed person {i} has employer {employer}"
            );
            let unit = &firms.units[usize::from(employer)];
            assert_eq!(
                unit.industry, industry,
                "1:{scale}: person {i} works in industry {industry} for unit {employer}"
            );
            assert_eq!(
                firms.industry_group[usize::from(industry)],
                jobs.industry_group[i],
                "1:{scale}: person {i} works in industry {industry}"
            );
            if status != EMPLOYEE {
                assert_eq!(
                    unit.size,
                    UnitSize::Micro,
                    "1:{scale}: person {i} with employment status {status} is in unit {employer}"
                );
            }
        }
    }
    assert_eq!((NO_INDUSTRY, NO_EMPLOYER), (255, 65535));
}

/// AC-POPE-02 [unit] The stage changes nothing before it: the earlier hashes are those of their own golden tests.
#[test]
fn ac_pope_02() {
    for scale in SCALES {
        let (pop, jobs, wages) = (
            pop(scale).clone(),
            jobs(scale).clone(),
            wages(scale).clone(),
        );
        let before = (pop.clone(), jobs.clone(), wages.clone());
        assign_employers(
            &pop,
            &jobs,
            Some(&wages),
            firm_units(),
            &EmployerParams::new(SEED),
        )
        .expect("fixture population gets employers");
        assert_eq!(pop, before.0, "1:{scale}: population changed");
        assert_eq!(jobs, before.1, "1:{scale}: jobs changed");
        assert_eq!(wages, before.2, "1:{scale}: wages changed");
        for (what, hash, file) in [
            ("state", pop.state_hash(), STATE_GOLDEN),
            ("jobs", jobs.state_hash(), JOBS_GOLDEN),
            ("wage", wages.state_hash(), WAGES_GOLDEN),
        ] {
            assert_eq!(
                hex(hash),
                golden_hash(file, scale),
                "{what} hash at 1:{scale}"
            );
        }
    }
}

/// AC-POPE-03 [unit] At each of the three scales, in every industry group, each unit's share of the group's employees is within 3 points of its share of the group's persons employed in the firm-unit file.
#[test]
fn ac_pope_03() {
    let mut failures = Vec::new();
    for scale in SCALES {
        check_unit_shares(
            &format!("1:{scale}"),
            firm_units(),
            jobs(scale),
            employers(scale),
            &person_weights(pop(scale)),
            &mut failures,
        );
    }
    assert_no_failures(&failures);
}

/// AC-POPE-05 [unit] Same inputs and seed give identical columns (equal employer hash). A different seed changes who works where; AC-POPE-03 still holds. Without wages the stage runs and AC-POPE-03 holds.
#[test]
fn ac_pope_05() {
    let (firms, scale) = (firm_units(), 1000);
    let (jobs, first) = (jobs(scale), employers(scale));
    let w = person_weights(pop(scale));
    let again = run(scale, Some(wages(scale)), SEED);
    assert_eq!(&again, first);
    assert_eq!(again.state_hash(), first.state_hash());

    let mut failures = Vec::new();
    let other = run(scale, Some(wages(scale)), OTHER_SEED);
    assert_ne!(other.state_hash(), first.state_hash());
    assert_ne!(other.employer, first.employer);
    check_unit_shares("another seed:", firms, jobs, &other, &w, &mut failures);

    let without = run(scale, None, SEED);
    check_unit_shares("without wages:", firms, jobs, &without, &w, &mut failures);
    assert_no_failures(&failures);
}

/// AC-POPE-06 [golden] The employer hash of the fixture population at each of the three scales equals the committed golden value.
#[test]
fn ac_pope_06() {
    assert_golden(EMPLOYERS_GOLDEN, "employer hash", |scale| {
        employers(scale).state_hash()
    });
}

/// AC-POPE-07 [diff] On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns.
#[test]
fn ac_pope_07() {
    for scale in DIFF_SCALES {
        let employers = employers(scale);
        let r = reference_all(scale);
        let industry: Vec<u64> = employers.industry.iter().map(|&x| u64::from(x)).collect();
        let employer: Vec<u64> = employers.employer.iter().map(|&x| u64::from(x)).collect();
        assert_same_column(scale, "industry", &industry, &column(r, "industry"));
        assert_same_column(scale, "employer", &employer, &column(r, "employer"));
        assert_eq!(
            hex(employers.state_hash()),
            reference_hash(r, "employers_hash"),
            "1:{scale} employer hash"
        );
    }
}

/// AC-POPE-08 [unit] No industries, no units, a unit of an unlisted industry, jobs of a different length, and a worker in an industry group without a unit are each rejected with a named error and produce no output.
#[test]
fn ac_pope_08() {
    let firms = firm_units();
    let (pop, jobs, wages) = (pop(1000), jobs(1000), wages(1000));
    let params = EmployerParams::new(SEED);
    let run = |firms: &FirmUnits| assign_employers(pop, jobs, Some(wages), firms, &params);

    let mut no_industries = firms.clone();
    no_industries.industry_group.clear();
    assert_eq!(run(&no_industries), Err(EmployerError::EmptyTable));

    let mut no_units = firms.clone();
    no_units.units.clear();
    assert_eq!(run(&no_units), Err(EmployerError::EmptyTable));

    let mut unlisted = firms.clone();
    unlisted.units[0].industry =
        u8::try_from(firms.industry_group.len()).expect("fewer than 256 industries");
    assert_eq!(run(&unlisted), Err(EmployerError::ShapeMismatch));

    let other = PersonJobs {
        employment_status: shorter(&jobs.employment_status),
        occupation: shorter(&jobs.occupation),
        industry_group: shorter(&jobs.industry_group),
    };
    assert_eq!(
        assign_employers(pop, &other, Some(wages), firms, &params),
        Err(EmployerError::ShapeMismatch)
    );

    // Trade, transport and hospitality has workers at every scale; no other group loses its units.
    let group = 3;
    assert!(
        (0..jobs.employment_status.len())
            .any(|i| jobs.employment_status[i] != NO_JOB && jobs.industry_group[i] == group),
        "industry group {group} has workers"
    );
    let mut unitless = firms.clone();
    unitless
        .units
        .retain(|unit| firms.industry_group[usize::from(unit.industry)] != group);
    assert_eq!(
        run(&unitless),
        Err(EmployerError::NoMargin {
            industry_group: group
        })
    );
}
