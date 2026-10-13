//! Acceptance tests for AC-POPP-* (spec: society/population-pensions).
//! Written in a test-authoring session from the spec, the API skeleton and the
//! Python reference, before the Rust stage existed (ADR-0014).
//!
//! "The fixtures" are the normalised margin files committed with the pipeline:
//! the Census 2021 margins of stages A to C and the pension margins.

// Column tables are indexed in parallel; index loops are the clear way to read them.
#![allow(clippy::needless_range_loop)]

use econ_popgen::{
    Activity, EDU_GROUPS, PensionError, PensionMargins, PensionParams, PersonAttributes,
    PersonPensions, Population, assign_pensions,
};

use super::support::{
    ATTRIBUTES_GOLDEN, DIFF_SCALES, OTHER_SEED, SCALES, SEED, STATE_GOLDEN, assert_golden,
    assert_no_failures, assert_same_column, column, golden_hash, hex,
};
use super::support_fj::{
    BANI_PER_LEU, PENSIONS_GOLDEN, U, assert_pays, attrs, pension_margins, pensions,
    person_weights, pop, ratio, reference_all, reference_hash, shorter, weighted_quantile,
    weighted_sum,
};

/// Education groups of the margins: at most lower secondary, and tertiary.
const LOW_EDUCATION: usize = 0;
const TERTIARY: usize = EDU_GROUPS - 1;
/// Education groups with fewer synthetic records are not checked (AC-POPP-04).
const MIN_GROUP_RECORDS: u64 = 100;

fn minimum_pension(m: &PensionMargins) -> u64 {
    m.minimum_pension * BANI_PER_LEU
}

fn bill(m: &PensionMargins) -> u128 {
    u128::from(m.pension_bill) * u128::from(BANI_PER_LEU)
}

/// Weight and pensions paid of the retired, by sex and by (sex, education group).
struct Paid {
    sex_w: [u64; 2],
    sex_paid: [u128; 2],
    /// `[sex][edu_group]`, row-major.
    cell_w: Vec<u64>,
    cell_paid: Vec<u128>,
}

impl Paid {
    /// Mean pension of an education group, both sexes together.
    fn mean_of_group(&self, group: usize) -> f64 {
        ratio(
            self.cell_paid[group] + self.cell_paid[EDU_GROUPS + group],
            u128::from(self.cell_w[group] + self.cell_w[EDU_GROUPS + group]),
        )
    }

    /// Mean pension of a cell over the mean of its sex.
    fn cell_over_sex(&self, c: usize) -> f64 {
        let s = c / EDU_GROUPS;
        ratio(self.cell_paid[c], u128::from(self.cell_w[c]))
            / ratio(self.sex_paid[s], u128::from(self.sex_w[s]))
    }
}

fn paid(
    m: &PensionMargins,
    pop: &Population,
    attrs: &PersonAttributes,
    pensions: &PersonPensions,
) -> Paid {
    let w = person_weights(pop);
    let mut p = Paid {
        sex_w: [0; 2],
        sex_paid: [0; 2],
        cell_w: vec![0; 2 * EDU_GROUPS],
        cell_paid: vec![0; 2 * EDU_GROUPS],
    };
    for i in 0..w.len() {
        if attrs.activity[i] != Activity::Retired {
            continue;
        }
        let s = pop.persons.sex[i] as usize;
        let c = s * EDU_GROUPS + usize::from(m.edu_group_of_level[attrs.edu_level[i] as usize]);
        let amount = u128::from(w[i]) * u128::from(pensions.pension[i]);
        p.sex_w[s] += w[i];
        p.sex_paid[s] += amount;
        p.cell_w[c] += w[i];
        p.cell_paid[c] += amount;
    }
    p
}

/// AC-POPP-03: the pension bill is paid.
fn assert_bill_paid(what: &str, m: &PensionMargins, pop: &Population, pensions: &PersonPensions) {
    assert_pays(
        &format!("{what} pensions"),
        weighted_sum(&person_weights(pop), &pensions.pension),
        bill(m),
    );
}

/// AC-POPP-01 [unit] Structure, at each of the three scales: one pension per person; it is above zero exactly when `activity` is retired; none is below the minimum pension.
#[test]
fn ac_popp_01() {
    let m = pension_margins();
    for scale in SCALES {
        let (attrs, pensions) = (attrs(scale), pensions(scale));
        let n = pop(scale).persons.household_id.len();
        assert_eq!(
            pensions.pension.len(),
            n,
            "1:{scale}: one pension per person"
        );
        for i in 0..n {
            let retired = attrs.activity[i] == Activity::Retired;
            assert_eq!(
                pensions.pension[i] > 0,
                retired,
                "1:{scale}: person {i} with activity {:?} has pension {}",
                attrs.activity[i],
                pensions.pension[i]
            );
            assert!(
                !retired || pensions.pension[i] >= minimum_pension(m),
                "1:{scale}: person {i} has a pension of {}, below the minimum pension",
                pensions.pension[i]
            );
        }
    }
}

/// AC-POPP-02 [unit] The stage changes nothing before it: the state hash and the attribute hash are those of AC-POP-06 and AC-POPA-06.
#[test]
fn ac_popp_02() {
    for scale in SCALES {
        let (pop, attrs) = (pop(scale).clone(), attrs(scale).clone());
        let before = (pop.clone(), attrs.clone());
        assign_pensions(&pop, &attrs, pension_margins(), &PensionParams::new(SEED))
            .expect("fixture population gets pensions");
        assert_eq!(pop, before.0, "1:{scale}: population changed");
        assert_eq!(attrs, before.1, "1:{scale}: attributes changed");
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

/// AC-POPP-03 [unit] At each of the three scales Σ weight × pension is at least the pension bill and exceeds it by less than 0.001%.
#[test]
fn ac_popp_03() {
    for scale in SCALES {
        assert_bill_paid(
            &format!("1:{scale}"),
            pension_margins(),
            pop(scale),
            pensions(scale),
        );
    }
}

/// AC-POPP-04 [unit] At each of the three scales: the mean pension of women over that of men is within 0.03 of `rel_sex` (0.769); inside each sex, the mean pension of each education group with at least 100 synthetic records, over the mean of the sex, is within 4% of the proportion of the update rule, renormalised to the persons present as in Stage F.
#[test]
fn ac_popp_04() {
    let m = pension_margins();
    let link = u64::from(PensionParams::new(SEED).earnings_link_ppm);
    // The earnings proportions pulled towards 1: ⌊(U² + k · (rel − U)) / U⌋.
    let targets: Vec<u64> = m
        .rel_education
        .iter()
        .map(|&rel| (U * U + link * u64::from(rel) - link * U) / U)
        .collect();
    let rel_sex = f64::from(m.rel_sex[0]) / f64::from(m.rel_sex[1]);
    assert!((rel_sex - 0.769).abs() < 0.0005, "rel_sex of the fixture");
    let mut failures = Vec::new();
    for scale in SCALES {
        let p = paid(m, pop(scale), attrs(scale), pensions(scale));
        let women_over_men = ratio(p.sex_paid[0], u128::from(p.sex_w[0]))
            / ratio(p.sex_paid[1], u128::from(p.sex_w[1]));
        if (women_over_men - rel_sex).abs() > 0.03 {
            failures.push(format!(
                "1:{scale}: women's mean pension over men's is {women_over_men:.4}, rel_sex {rel_sex:.4}"
            ));
        }
        let mut checked = 0;
        for c in 0..2 * EDU_GROUPS {
            let records = p.cell_w[c] / u64::from(scale);
            if records < MIN_GROUP_RECORDS {
                continue;
            }
            let s = c / EDU_GROUPS;
            let norm: u128 = (0..EDU_GROUPS)
                .map(|e| {
                    u128::from(p.cell_w[s * EDU_GROUPS + e])
                        * u128::from(targets[s * EDU_GROUPS + e])
                })
                .sum();
            let want = ratio(u128::from(targets[c]) * u128::from(p.sex_w[s]), norm);
            let got = p.cell_over_sex(c);
            checked += 1;
            if (got - want).abs() > 0.04 * want {
                failures.push(format!(
                    "1:{scale} sex {s}, education group {}: mean over the sex's mean is {got:.4}, the rule's proportion {want:.4} ({records} records)",
                    c % EDU_GROUPS
                ));
            }
        }
        assert!(
            checked > 0,
            "1:{scale}: no education group large enough to check"
        );
    }
    assert_no_failures(&failures);
}

/// AC-POPP-05 [unit] At each of the three scales: between 15% and 24% of the retired are at the minimum pension; the median pension is between 60% and 80% of the mean; retired persons with tertiary education have a mean pension at least 1.3 times that of those with at most lower secondary education.
#[test]
fn ac_popp_05() {
    let m = pension_margins();
    for scale in SCALES {
        let (pop, attrs, pensions) = (pop(scale), attrs(scale), pensions(scale));
        let w = person_weights(pop);
        let pairs: Vec<(u64, u64)> = (0..w.len())
            .filter(|&i| attrs.activity[i] == Activity::Retired)
            .map(|i| (pensions.pension[i], w[i]))
            .collect();
        let weight: u128 = pairs.iter().map(|&(_, wt)| u128::from(wt)).sum();
        let at_minimum: u128 = pairs
            .iter()
            .filter(|&&(pension, _)| pension <= minimum_pension(m))
            .map(|&(_, wt)| u128::from(wt))
            .sum();
        let share = ratio(at_minimum, weight);
        assert!(
            (0.15..=0.24).contains(&share),
            "1:{scale}: {share:.4} of the retired are at the minimum pension"
        );

        let median = weighted_quantile(&pairs, 500_000);
        let median_over_mean = ratio(
            u128::from(median) * weight,
            weighted_sum(&w, &pensions.pension),
        );
        assert!(
            (0.60..=0.80).contains(&median_over_mean),
            "1:{scale}: the median pension is {median_over_mean:.4} of the mean"
        );

        let p = paid(m, pop, attrs, pensions);
        let (tertiary, low) = (p.mean_of_group(TERTIARY), p.mean_of_group(LOW_EDUCATION));
        assert!(
            tertiary >= 1.3 * low,
            "1:{scale}: mean pension with tertiary education {tertiary:.0}, with at most lower secondary {low:.0}"
        );
    }
}

/// AC-POPP-06 [unit] Same inputs and seed give identical pensions (equal pension hash). A different seed changes who gets what; AC-POPP-03 still holds. With `pension_earnings_link` = 0 the mean pensions of the education groups of a sex are within 8% of each other.
#[test]
fn ac_popp_06() {
    let (m, scale) = (pension_margins(), 1000);
    let (pop, attrs, first) = (pop(scale), attrs(scale), pensions(scale));
    let again = assign_pensions(pop, attrs, m, &PensionParams::new(SEED)).expect("pensions");
    assert_eq!(&again, first);
    assert_eq!(again.state_hash(), first.state_hash());

    let other = assign_pensions(pop, attrs, m, &PensionParams::new(OTHER_SEED)).expect("pensions");
    assert_ne!(other.state_hash(), first.state_hash());
    assert_ne!(other.pension, first.pension);
    assert_bill_paid("another seed:", m, pop, &other);

    let unlinked = PensionParams {
        earnings_link_ppm: 0,
        ..PensionParams::new(SEED)
    };
    let p = paid(
        m,
        pop,
        attrs,
        &assign_pensions(pop, attrs, m, &unlinked).expect("pensions"),
    );
    for s in 0..2 {
        let means: Vec<f64> = (s * EDU_GROUPS..(s + 1) * EDU_GROUPS)
            .filter(|&c| p.cell_w[c] > 0)
            .map(|c| ratio(p.cell_paid[c], u128::from(p.cell_w[c])))
            .collect();
        assert!(means.len() > 1, "sex {s}: education groups to compare");
        let (lowest, highest) = (
            means.iter().copied().fold(f64::INFINITY, f64::min),
            means.iter().copied().fold(0.0, f64::max),
        );
        assert!(
            highest <= 1.08 * lowest,
            "sex {s}: with no link to earnings the education groups' mean pensions run from {lowest:.0} to {highest:.0}"
        );
    }
}

/// AC-POPP-07 [golden] The pension hash of the fixture population at each of the three scales equals the committed golden value.
#[test]
fn ac_popp_07() {
    assert_golden(PENSIONS_GOLDEN, "pension hash", |scale| {
        pensions(scale).state_hash()
    });
}

/// AC-POPP-08 [diff] On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical pensions, person for person.
#[test]
fn ac_popp_08() {
    for scale in DIFF_SCALES {
        let pensions = pensions(scale);
        let r = reference_all(scale);
        assert_same_column(scale, "pension", &pensions.pension, &column(r, "pension"));
        assert_eq!(
            hex(pensions.state_hash()),
            reference_hash(r, "pensions_hash"),
            "1:{scale} pension hash"
        );
    }
}

/// AC-POPP-09 [unit] Margins with no education groups, a table whose length does not match its dimensions, a proportion of zero, a quantile curve that does not end at rank 1, attributes of a different length, and a pension bill of zero while someone is retired are each rejected with a named error and produce no output.
#[test]
fn ac_popp_09() {
    let m = pension_margins();
    let (pop, attrs) = (pop(1000), attrs(1000));
    let params = PensionParams::new(SEED);
    let run = |m: &PensionMargins| assign_pensions(pop, attrs, m, &params);

    // The table by education group is the only place the margins name the groups.
    let mut groupless = m.clone();
    groupless.rel_education.clear();
    assert_eq!(run(&groupless), Err(PensionError::EmptyTable));

    let mut short = m.clone();
    short.rel_education.pop();
    assert_eq!(run(&short), Err(PensionError::ShapeMismatch));
    let mut short = m.clone();
    short.curve_value.pop();
    assert_eq!(run(&short), Err(PensionError::ShapeMismatch));

    let mut zero = m.clone();
    zero.rel_education[1] = 0;
    assert_eq!(run(&zero), Err(PensionError::ShapeMismatch));
    let mut zero = m.clone();
    zero.rel_sex[0] = 0;
    assert_eq!(run(&zero), Err(PensionError::ShapeMismatch));

    let mut unfinished = m.clone();
    *unfinished.curve_rank.last_mut().expect("knots") -= 1;
    assert_eq!(run(&unfinished), Err(PensionError::ShapeMismatch));

    let other = PersonAttributes {
        edu_level: shorter(&attrs.edu_level),
        activity: shorter(&attrs.activity),
    };
    assert_eq!(
        assign_pensions(pop, &other, m, &params),
        Err(PensionError::ShapeMismatch)
    );

    assert!(
        attrs.activity.contains(&Activity::Retired),
        "someone is retired"
    );
    let mut unpaid = m.clone();
    unpaid.pension_bill = 0;
    assert_eq!(run(&unpaid), Err(PensionError::NoMargin));
}
