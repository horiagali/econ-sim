//! Acceptance test for AC-POPE-04 (spec: society/population-employers), as
//! reworded on 2026-10-13. Written in a test-authoring session from the spec,
//! the API skeleton and the Python reference, before the Rust stage existed
//! (ADR-0014). Compiled only with the crate feature `employers`.
//!
//! The other employer criteria are in `employers.rs`.

// Column tables are indexed in parallel; index loops are the clear way to read them.
#![allow(clippy::needless_range_loop)]

use econ_popgen::{
    EmployerParams, FirmUnits, INDUSTRY_GROUPS, PersonEmployers, PersonJobs, PersonWages, UnitSize,
    assign_employers,
};

use super::support::{SCALES, SEED};
use super::support_fj::{EMPLOYEE, firm_units, jobs, person_weights, pop, wages};

/// Micro, small and large: the sizes the criterion compares (public units are left out).
const SIZES: [UnitSize; 3] = [UnitSize::Micro, UnitSize::Small, UnitSize::Large];

/// Weight and weighted wage of the employees in each (size, industry group) cell.
struct Cells {
    weight: [[u128; INDUSTRY_GROUPS]; 3],
    pay: [[u128; INDUSTRY_GROUPS]; 3],
}

fn cells(
    firms: &FirmUnits,
    jobs: &PersonJobs,
    wages: &PersonWages,
    employers: &PersonEmployers,
    w: &[u64],
) -> Cells {
    let mut c = Cells {
        weight: [[0; INDUSTRY_GROUPS]; 3],
        pay: [[0; INDUSTRY_GROUPS]; 3],
    };
    for i in 0..w.len() {
        if jobs.employment_status[i] != EMPLOYEE {
            continue;
        }
        let unit = &firms.units[usize::from(employers.employer[i])];
        let Some(s) = SIZES.iter().position(|&size| size == unit.size) else {
            continue;
        };
        let g = usize::from(firms.industry_group[usize::from(unit.industry)]);
        c.weight[s][g] += u128::from(w[i]);
        c.pay[s][g] += u128::from(w[i]) * u128::from(wages.wage[i]);
    }
    c
}

/// Plain mean wage of each size, in bani.
fn plain_means(c: &Cells) -> [f64; 3] {
    let mut m = [0.0; 3];
    for s in 0..3 {
        let weight: u128 = c.weight[s].iter().sum();
        let pay: u128 = c.pay[s].iter().sum();
        assert!(weight > 0, "size {:?} has no employee", SIZES[s]);
        m[s] = pay as f64 / weight as f64;
    }
    m
}

/// Mean wage of each size with the mix of industry groups held fixed: the
/// average of the size's mean wage in each group, weighted by the group's
/// employees in micro, small and large units together, over the groups where
/// that size has employees.
fn mix_fixed_means(c: &Cells) -> [f64; 3] {
    let mut m = [0.0; 3];
    for s in 0..3 {
        let (mut num, mut den) = (0.0, 0.0);
        for g in 0..INDUSTRY_GROUPS {
            if c.weight[s][g] == 0 {
                continue;
            }
            let group: u128 = (0..3).map(|t| c.weight[t][g]).sum();
            num += group as f64 * (c.pay[s][g] as f64 / c.weight[s][g] as f64);
            den += group as f64;
        }
        assert!(den > 0.0, "size {:?} has no employee", SIZES[s]);
        m[s] = num / den;
    }
    m
}

fn spread(m: [f64; 3]) -> f64 {
    let hi = m.iter().copied().fold(f64::MIN, f64::max);
    let lo = m.iter().copied().fold(f64::MAX, f64::min);
    hi / lo - 1.0
}

/// AC-POPE-04 [unit] At each of the three scales the mean wage of employees rises from micro to small to large units. With `employer_wage_link` = 0 the three means are within 10% of each other once industry groups are held fixed: each size's mean is the average of its mean wage in each industry group, weighted by the group's employees in micro, small and large units together, over the groups where that size has employees.
#[test]
fn ac_pope_04() {
    let firms = firm_units();
    let mut failures = Vec::new();
    for scale in SCALES {
        let (p, j, wg) = (pop(scale), jobs(scale), wages(scale));
        let w = person_weights(p);

        let linked = assign_employers(p, j, Some(wg), firms, &EmployerParams::new(SEED))
            .expect("fixture population gets employers");
        let m = plain_means(&cells(firms, j, wg, &linked, &w));
        if !(m[0] < m[1] && m[1] < m[2]) {
            failures.push(format!(
                "1:{scale}: mean wage micro {:.0}, small {:.0}, large {:.0} bani does not rise",
                m[0], m[1], m[2]
            ));
        }

        let unlinked = EmployerParams {
            wage_link_ppm: 0,
            ..EmployerParams::new(SEED)
        };
        let free = assign_employers(p, j, Some(wg), firms, &unlinked)
            .expect("fixture population gets employers");
        let fixed = mix_fixed_means(&cells(firms, j, wg, &free, &w));
        let gap = spread(fixed);
        if gap > 0.10 {
            failures.push(format!(
                "1:{scale}: with no wage link the mix-fixed means micro {:.0}, small {:.0}, \
                 large {:.0} bani are {:.1}% apart, more than 10%",
                fixed[0],
                fixed[1],
                fixed[2],
                gap * 100.0
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
