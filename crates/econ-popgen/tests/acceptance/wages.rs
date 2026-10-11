//! Acceptance tests for AC-POPW-* (spec: society/population-wages).
//! Written in a test-authoring session from the spec, the API skeleton and the
//! Python reference, before the Rust stage existed (ADR-0014).
//!
//! "The fixtures" are the normalised margin files committed with the pipeline:
//! the Census 2021 margins of stages A to D and the earnings margins.

// Column tables are indexed in parallel; index loops are the clear way to read them.
#![allow(clippy::needless_range_loop)]

use econ_popgen::{
    EDU_GROUPS, INDUSTRY_GROUPS, OCCUPATIONS, PersonAttributes, PersonJobs, PersonWages,
    Population, WageError, WageMargins, WageParams, assign_wages,
};

use super::support::{
    ATTRIBUTES_GOLDEN, DIFF_SCALES, JOBS_GOLDEN, OTHER_SEED, SCALES, SEED, STATE_GOLDEN,
    assert_golden, assert_no_failures, assert_same_column, column, golden_hash, hex,
};
use super::support_fj::{
    BANI_PER_LEU, EMPLOYEE, U, WAGES_GOLDEN, assert_pays, attrs, jobs, person_weights, pop, ratio,
    rdiv, reference_all, reference_hash, shorter, wage_margins, wages, weighted_quantile,
};

/// Tolerance of AC-POPW-04 by synthetic records in the cell, largest cells
/// first. Smaller cells than the last entry are not checked.
const TOLERANCES: [(u64, f64); 3] = [(1000, 0.05), (100, 0.06), (30, 0.08)];
// The survey's distribution (AC-POPW-05).
const SURVEY_P10_OVER_MEDIAN: f64 = 0.578;
const SURVEY_P90_OVER_MEDIAN: f64 = 2.362;
const SURVEY_MEDIAN_OVER_MEAN: f64 = 0.753;

/// The employees of one industry group.
struct Group {
    /// Indices into the persons table.
    members: Vec<usize>,
    /// Their weight.
    weight: u64,
    /// The group's wage bill, in bani.
    bill: u128,
    /// The lowest wage the group may pay, in bani (step 3 of the update rule).
    floor: u64,
    /// The floor is below the minimum wage.
    lowered: bool,
}

fn minimum_wage(m: &WageMargins) -> u64 {
    m.minimum_wage * BANI_PER_LEU
}

fn groups(m: &WageMargins, params: &WageParams, jobs: &PersonJobs, w: &[u64]) -> Vec<Group> {
    let mut members = vec![Vec::new(); INDUSTRY_GROUPS];
    for i in 0..w.len() {
        if jobs.employment_status[i] == EMPLOYEE {
            members[usize::from(jobs.industry_group[i])].push(i);
        }
    }
    members
        .into_iter()
        .enumerate()
        .map(|(g, members)| {
            let weight: u64 = members.iter().map(|&i| w[i]).sum();
            let bill = u128::from(m.wage_bill[g]) * u128::from(BANI_PER_LEU);
            // The minimum wage, or `wage_floor_share` of the group's mean wage if that is lower.
            let share_of_mean = if weight == 0 {
                u128::from(minimum_wage(m))
            } else {
                rdiv(
                    u128::from(params.floor_share_ppm) * bill,
                    u128::from(weight) * u128::from(U),
                )
            };
            let lowered = share_of_mean < u128::from(minimum_wage(m));
            Group {
                members,
                weight,
                bill,
                floor: if lowered {
                    u64::try_from(share_of_mean).expect("a wage in bani")
                } else {
                    minimum_wage(m)
                },
                lowered,
            }
        })
        .collect()
}

fn paid(group: &Group, w: &[u64], wages: &PersonWages) -> u128 {
    group
        .members
        .iter()
        .map(|&i| u128::from(w[i]) * u128::from(wages.wage[i]))
        .sum()
}

/// AC-POPW-03: every industry group pays its wage bill, and so does the country.
fn assert_bills_paid(
    what: &str,
    m: &WageMargins,
    jobs: &PersonJobs,
    w: &[u64],
    wages: &PersonWages,
) {
    let (mut paid_all, mut bill_all) = (0u128, 0u128);
    for (g, group) in groups(m, &WageParams::new(SEED), jobs, w)
        .iter()
        .enumerate()
    {
        assert!(
            !group.members.is_empty(),
            "{what}: industry group {g} has no employee"
        );
        let paid = paid(group, w, wages);
        assert_pays(&format!("{what} industry group {g}"), paid, group.bill);
        paid_all += paid;
        bill_all += group.bill;
    }
    assert_pays(&format!("{what} all industry groups"), paid_all, bill_all);
}

/// The category of an employee in each of the three tables of proportions:
/// occupation, age group, education group.
fn categories(
    m: &WageMargins,
    pop: &Population,
    attrs: &PersonAttributes,
    jobs: &PersonJobs,
    i: usize,
) -> [usize; 3] {
    let years = pop.persons.age[i] / 12;
    let age_group = m
        .age_group_from
        .iter()
        .rposition(|&from| from <= years)
        .expect("age groups start at 0");
    [
        usize::from(jobs.occupation[i]),
        age_group,
        usize::from(m.edu_group_of_level[attrs.edu_level[i] as usize]),
    ]
}

/// AC-POPW-01 [unit] Structure, at each of the three scales: one wage per person; it is above zero exactly when `employment_status` is employee; no wage is below the floor of its industry group, and outside the groups with a lowered floor none is below the minimum wage.
#[test]
fn ac_popw_01() {
    let m = wage_margins();
    for scale in SCALES {
        let (jobs, wages) = (jobs(scale), wages(scale));
        let w = person_weights(pop(scale));
        assert_eq!(wages.wage.len(), w.len(), "1:{scale}: one wage per person");
        for i in 0..w.len() {
            assert_eq!(
                wages.wage[i] > 0,
                jobs.employment_status[i] == EMPLOYEE,
                "1:{scale}: person {i} with employment status {} has wage {}",
                jobs.employment_status[i],
                wages.wage[i]
            );
        }
        for (g, group) in groups(m, &WageParams::new(SEED), jobs, &w)
            .iter()
            .enumerate()
        {
            assert!(
                group.floor <= minimum_wage(m),
                "1:{scale}: floor of group {g}"
            );
            for &i in &group.members {
                assert!(
                    wages.wage[i] >= group.floor,
                    "1:{scale}: person {i} of industry group {g} earns {}, below the group's floor of {}",
                    wages.wage[i],
                    group.floor
                );
                if !group.lowered {
                    assert!(
                        wages.wage[i] >= minimum_wage(m),
                        "1:{scale}: person {i} of industry group {g} earns {}, below the minimum wage",
                        wages.wage[i]
                    );
                }
            }
        }
    }
}

/// AC-POPW-02 [unit] The stage changes nothing before it: the state hash, the attribute hash and the jobs hash are those of AC-POP-06, AC-POPA-06 and AC-POPJ-06.
#[test]
fn ac_popw_02() {
    for scale in SCALES {
        let (pop, attrs, jobs) = (
            pop(scale).clone(),
            attrs(scale).clone(),
            jobs(scale).clone(),
        );
        let before = (pop.clone(), attrs.clone(), jobs.clone());
        assign_wages(&pop, &attrs, &jobs, wage_margins(), &WageParams::new(SEED))
            .expect("fixture population gets wages");
        assert_eq!(pop, before.0, "1:{scale}: population changed");
        assert_eq!(attrs, before.1, "1:{scale}: attributes changed");
        assert_eq!(jobs, before.2, "1:{scale}: jobs changed");
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
        assert_eq!(
            hex(jobs.state_hash()),
            golden_hash(JOBS_GOLDEN, scale),
            "jobs hash at 1:{scale}"
        );
    }
}

/// AC-POPW-03 [unit] At each of the three scales every industry group pays its wage bill: Σ weight × wage is at least the bill and exceeds it by less than 0.001%. The national total follows.
#[test]
fn ac_popw_03() {
    for scale in SCALES {
        let w = person_weights(pop(scale));
        assert_bills_paid(
            &format!("1:{scale}"),
            wage_margins(),
            jobs(scale),
            &w,
            wages(scale),
        );
    }
}

/// AC-POPW-04 [unit] At each of the three scales, in every industry group whose floor is the minimum wage, the mean wage of each (sex, occupation), (sex, age group) and (sex, education group) cell over the mean of the group is within the tolerances of the table below of the survey's proportion, renormalised as in step 2.
#[test]
fn ac_popw_04() {
    let m = wage_margins();
    let tables: [(&str, usize, &[u32]); 3] = [
        ("occupation", OCCUPATIONS, &m.rel_occupation),
        ("age group", m.age_group_from.len(), &m.rel_age),
        ("education group", EDU_GROUPS, &m.rel_education),
    ];
    let mut failures = Vec::new();
    let mut checked = 0;
    for scale in SCALES {
        let (pop, attrs, jobs, wages) = (pop(scale), attrs(scale), jobs(scale), wages(scale));
        let w = person_weights(pop);
        for (g, group) in groups(m, &WageParams::new(SEED), jobs, &w)
            .iter()
            .enumerate()
        {
            if group.lowered || group.members.is_empty() {
                continue;
            }
            let group_paid = paid(group, &w, wages);
            let rel_sex = &m.rel_sex[g * 2..g * 2 + 2];
            for (t, &(name, width, rel)) in tables.iter().enumerate() {
                // The survey's proportions of this group, [sex][category].
                let targets = &rel[g * 2 * width..(g + 1) * 2 * width];
                let (mut cell_w, mut cell_paid) = (vec![0u64; 2 * width], vec![0u128; 2 * width]);
                for &i in &group.members {
                    let c =
                        pop.persons.sex[i] as usize * width + categories(m, pop, attrs, jobs, i)[t];
                    cell_w[c] += w[i];
                    cell_paid[c] += u128::from(w[i]) * u128::from(wages.wage[i]);
                }
                // Renormalised to the persons the group has, as in step 2: the
                // mean of each sex, then the cell inside its sex.
                let sex_w: Vec<u64> = cell_w.chunks(width).map(|c| c.iter().sum()).collect();
                let sex_norm = ratio(
                    u128::from(sex_w[0]) * u128::from(rel_sex[0])
                        + u128::from(sex_w[1]) * u128::from(rel_sex[1]),
                    1,
                );
                for c in 0..2 * width {
                    let records = cell_w[c] / u64::from(scale);
                    let Some(&(_, tolerance)) = TOLERANCES
                        .iter()
                        .find(|&&(min_records, _)| records >= min_records)
                    else {
                        continue; // too few synthetic records: not checked
                    };
                    let s = c / width;
                    let norm: u128 = (0..width)
                        .map(|k| {
                            u128::from(cell_w[s * width + k]) * u128::from(targets[s * width + k])
                        })
                        .sum();
                    let want = f64::from(rel_sex[s]) * ratio(u128::from(group.weight), 1)
                        / sex_norm
                        * f64::from(targets[c])
                        * ratio(u128::from(sex_w[s]), norm);
                    let got = ratio(cell_paid[c], u128::from(cell_w[c]))
                        / ratio(group_paid, u128::from(group.weight));
                    checked += 1;
                    if (got - want).abs() > tolerance * want {
                        failures.push(format!(
                            "1:{scale} industry group {g}, sex {s}, {name} {}: mean over the group's mean is {got:.4}, the survey's proportion {want:.4} (tolerance {tolerance}, {records} records)",
                            c % width
                        ));
                    }
                }
            }
        }
    }
    assert!(checked > 0, "no cell was large enough to check");
    assert_no_failures(&failures);
}

/// AC-POPW-05 [unit] The national distribution at each of the three scales: the first decile over the median is within 0.06 of the survey's (0.578), the ninth decile over the median within 0.25 of the survey's (2.362), the median over the mean within 0.05 of the survey's (0.753); between 3% and 15% of employees are at or below the minimum wage.
#[test]
fn ac_popw_05() {
    let m = wage_margins();
    for scale in SCALES {
        let (jobs, wages) = (jobs(scale), wages(scale));
        let w = person_weights(pop(scale));
        let pairs: Vec<(u64, u64)> = (0..w.len())
            .filter(|&i| jobs.employment_status[i] == EMPLOYEE)
            .map(|i| (wages.wage[i], w[i]))
            .collect();
        let weight: u128 = pairs.iter().map(|&(_, wt)| u128::from(wt)).sum();
        let paid: u128 = pairs
            .iter()
            .map(|&(wage, wt)| u128::from(wage) * u128::from(wt))
            .sum();
        let [p10, median, p90] =
            [100_000, 500_000, 900_000].map(|share| weighted_quantile(&pairs, share));
        let over_median = |v: u64| ratio(u128::from(v), u128::from(median));
        assert!(
            (over_median(p10) - SURVEY_P10_OVER_MEDIAN).abs() <= 0.06,
            "1:{scale}: first decile {p10} over median {median}"
        );
        assert!(
            (over_median(p90) - SURVEY_P90_OVER_MEDIAN).abs() <= 0.25,
            "1:{scale}: ninth decile {p90} over median {median}"
        );
        let median_over_mean = ratio(u128::from(median) * weight, paid);
        assert!(
            (median_over_mean - SURVEY_MEDIAN_OVER_MEAN).abs() <= 0.05,
            "1:{scale}: median over mean is {median_over_mean:.4}"
        );
        let low: u128 = pairs
            .iter()
            .filter(|&&(wage, _)| wage <= minimum_wage(m))
            .map(|&(_, wt)| u128::from(wt))
            .sum();
        let share = ratio(low, weight);
        assert!(
            (0.03..=0.15).contains(&share),
            "1:{scale}: {share:.4} of employees are at or below the minimum wage"
        );
    }
}

/// AC-POPW-06 [unit] Same inputs and seed give identical wages (equal wage hash). A different seed changes who earns what; AC-POPW-03 still holds.
#[test]
fn ac_popw_06() {
    let (m, scale) = (wage_margins(), 1000);
    let (pop, attrs, jobs, first) = (pop(scale), attrs(scale), jobs(scale), wages(scale));
    let again = assign_wages(pop, attrs, jobs, m, &WageParams::new(SEED)).expect("wages");
    assert_eq!(&again, first);
    assert_eq!(again.state_hash(), first.state_hash());

    let other = assign_wages(pop, attrs, jobs, m, &WageParams::new(OTHER_SEED)).expect("wages");
    assert_ne!(other.state_hash(), first.state_hash());
    assert_ne!(other.wage, first.wage);
    assert_bills_paid("another seed:", m, jobs, &person_weights(pop), &other);
}

/// AC-POPW-07 [golden] The wage hash of the fixture population at each of the three scales equals the committed golden value.
#[test]
fn ac_popw_07() {
    assert_golden(WAGES_GOLDEN, "wage hash", |scale| wages(scale).state_hash());
}

/// AC-POPW-08 [diff] On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical wages, person for person.
#[test]
fn ac_popw_08() {
    for scale in DIFF_SCALES {
        let wages = wages(scale);
        let r = reference_all(scale);
        assert_same_column(scale, "wage", &wages.wage, &column(r, "wage"));
        assert_eq!(
            hex(wages.state_hash()),
            reference_hash(r, "wages_hash"),
            "1:{scale} wage hash"
        );
    }
}

/// AC-POPW-09 [unit] Margins with no age groups, a table whose length does not match its dimensions, a proportion of zero, a quantile curve that does not end at rank 1, jobs of a different length, and a wage bill of zero for a group that has employees are each rejected with a named error and produce no output.
#[test]
fn ac_popw_09() {
    let m = wage_margins();
    let (pop, attrs, jobs) = (pop(1000), attrs(1000), jobs(1000));
    let params = WageParams::new(SEED);
    let run = |m: &WageMargins| assign_wages(pop, attrs, jobs, m, &params);

    let mut ageless = m.clone();
    ageless.age_group_from.clear();
    ageless.rel_age.clear();
    assert_eq!(run(&ageless), Err(WageError::EmptyTable));

    let tables: [fn(&mut WageMargins) -> &mut Vec<u32>; 4] = [
        |m| &mut m.rel_sex,
        |m| &mut m.rel_occupation,
        |m| &mut m.rel_age,
        |m| &mut m.rel_education,
    ];
    for table in tables {
        let mut short = m.clone();
        table(&mut short).pop();
        assert_eq!(run(&short), Err(WageError::ShapeMismatch));

        let mut zero = m.clone();
        table(&mut zero)[0] = 0;
        assert_eq!(run(&zero), Err(WageError::ShapeMismatch));
    }
    let mut short = m.clone();
    short.wage_bill.pop();
    assert_eq!(run(&short), Err(WageError::ShapeMismatch));

    let mut unfinished = m.clone();
    *unfinished.curve_rank.last_mut().expect("knots") -= 1;
    assert_eq!(run(&unfinished), Err(WageError::ShapeMismatch));

    let other = PersonJobs {
        employment_status: shorter(&jobs.employment_status),
        occupation: shorter(&jobs.occupation),
        industry_group: shorter(&jobs.industry_group),
    };
    assert_eq!(
        assign_wages(pop, attrs, &other, m, &params),
        Err(WageError::ShapeMismatch)
    );

    // Trade, transport and hospitality has employees at every scale.
    let group = 3;
    assert!(
        (0..jobs.employment_status.len()).any(|i| {
            jobs.employment_status[i] == EMPLOYEE && usize::from(jobs.industry_group[i]) == group
        }),
        "industry group {group} has employees"
    );
    let mut unpaid = m.clone();
    unpaid.wage_bill[group] = 0;
    assert_eq!(
        run(&unpaid),
        Err(WageError::NoMargin {
            industry_group: u8::try_from(group).expect("few groups"),
        })
    );
}
