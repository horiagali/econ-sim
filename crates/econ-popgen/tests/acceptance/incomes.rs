//! Acceptance tests for AC-POPI-* (spec: society/population-incomes).
//! Written in a test-authoring session from the spec, the API skeleton and the
//! Python reference, before the Rust stage existed (ADR-0014).
//!
//! "The fixtures" are the normalised margin files committed with the pipeline:
//! the Census 2021 margins of stages A to E, the earnings margins and the
//! income margins.

// Column tables are indexed in parallel; index loops are the clear way to read them.
#![allow(clippy::needless_range_loop)]

use std::sync::OnceLock;

use econ_popgen::{
    INDUSTRY_GROUPS, IncomeError, IncomeMargins, IncomeParams, OtherIncomes, PersonJobs,
    PersonWages, Population, assign_other_incomes,
};

use super::support::{
    ATTRIBUTES_GOLDEN, DIFF_SCALES, HOUSING_GOLDEN, JOBS_GOLDEN, OTHER_SEED, SCALES, SEED,
    STATE_GOLDEN, assert_golden, assert_same_column, column, golden_hash, hex,
};
use super::support_fj::{
    BANI_PER_LEU, Cache, EMPLOYEE, INCOMES_FIXTURE, INCOMES_GOLDEN, U, WAGES_GOLDEN, assert_total,
    attrs, fixture, housing, income_margins, jobs, num, person_weights, pop, ratio, reference_all,
    reference_hash, shorter, slot, wages, weighted_sum,
};

/// Statuses in employment of the self-employed: employer, own-account worker, family worker.
const SELF_EMPLOYED: std::ops::RangeInclusive<u8> = 2..=4;
/// Informal income outside a job goes to persons aged 15 to 74 only; in months.
const INFORMAL_AGES: std::ops::Range<u16> = 15 * 12..75 * 12;
/// The industry group whose wages are not private labour income (AC-POPI-05).
const PUBLIC_GROUP: &str = "public_education_health";

/// Stage I on the fixture population at a cached scale.
fn incomes(scale: u32) -> &'static OtherIncomes {
    static INCOMES: Cache<OtherIncomes> = [const { OnceLock::new() }; 4];
    INCOMES[slot(scale)].get_or_init(|| run(scale, income_margins(), SEED))
}

fn run(scale: u32, m: &IncomeMargins, seed: u64) -> OtherIncomes {
    assign_other_incomes(
        pop(scale),
        attrs(scale),
        jobs(scale),
        housing(scale),
        wages(scale),
        m,
        &IncomeParams::new(seed),
    )
    .expect("fixture population gets its other incomes")
}

fn household_weights(pop: &Population) -> Vec<u64> {
    pop.households
        .hh_weight
        .iter()
        .map(|&w| u64::from(w))
        .collect()
}

/// AC-POPI-03: self-employment, informal and subsistence income add up to the
/// mixed income of the margin file.
fn assert_mixed_income(what: &str, m: &IncomeMargins, pop: &Population, incomes: &OtherIncomes) {
    let mixed_income = num(&fixture(INCOMES_FIXTURE)["mixed_income"]);
    assert_eq!(
        mixed_income,
        m.farm_income + m.self_employment_income + m.informal_income,
        "mixed income is farm income, self-employment income and informal income"
    );
    let w = person_weights(pop);
    let got = weighted_sum(&w, &incomes.self_employment_income)
        + weighted_sum(&w, &incomes.informal_income)
        + weighted_sum(&household_weights(pop), &incomes.hh_subsistence_income);
    assert_total(
        &format!("{what} mixed income"),
        got,
        u128::from(mixed_income) * u128::from(BANI_PER_LEU),
    );
}

/// Wage bill and undeclared wages of the employees of each industry group, in bani.
fn wage_split(
    jobs: &PersonJobs,
    wages: &PersonWages,
    incomes: &OtherIncomes,
    w: &[u64],
) -> (Vec<u128>, Vec<u128>) {
    let (mut bill, mut undeclared) = (vec![0u128; INDUSTRY_GROUPS], vec![0u128; INDUSTRY_GROUPS]);
    for i in 0..w.len() {
        if jobs.employment_status[i] == EMPLOYEE {
            let g = usize::from(jobs.industry_group[i]);
            bill[g] += u128::from(w[i]) * u128::from(wages.wage[i]);
            undeclared[g] +=
                u128::from(w[i]) * u128::from(wages.wage[i] - incomes.wage_declared[i]);
        }
    }
    (bill, undeclared)
}

/// AC-POPI-01 [unit] Structure, at each of the three scales: the bounds of the Outputs table hold for every person and household; nobody who is not an employee has a declared wage.
#[test]
fn ac_popi_01() {
    let minimum_wage = income_margins().minimum_wage * BANI_PER_LEU;
    for scale in SCALES {
        let (pop, jobs, housing, wages, incomes) = (
            pop(scale),
            jobs(scale),
            housing(scale),
            wages(scale),
            incomes(scale),
        );
        let (n, n_hh) = (
            pop.persons.household_id.len(),
            pop.households.hh_weight.len(),
        );
        for (name, column) in [
            ("wage_declared", &incomes.wage_declared),
            ("self_employment_income", &incomes.self_employment_income),
            (
                "self_employment_declared",
                &incomes.self_employment_declared,
            ),
            ("informal_income", &incomes.informal_income),
        ] {
            assert_eq!(column.len(), n, "1:{scale}: one {name} per person");
        }
        assert_eq!(
            incomes.hh_subsistence_income.len(),
            n_hh,
            "1:{scale}: one hh_subsistence_income per household"
        );
        for i in 0..n {
            let (wage, declared) = (wages.wage[i], incomes.wage_declared[i]);
            assert!(
                declared <= wage,
                "1:{scale}: person {i} declares {declared} of a wage of {wage}"
            );
            assert!(
                declared == 0 || declared == wage || declared >= minimum_wage,
                "1:{scale}: person {i} declares {declared} of a wage of {wage}, less than the minimum wage"
            );
            assert!(
                jobs.employment_status[i] == EMPLOYEE || declared == 0,
                "1:{scale}: person {i} is not an employee and has a declared wage of {declared}"
            );
            assert_eq!(
                incomes.self_employment_income[i] > 0,
                SELF_EMPLOYED.contains(&jobs.employment_status[i]),
                "1:{scale}: person {i} with employment status {} has self-employment income {}",
                jobs.employment_status[i],
                incomes.self_employment_income[i]
            );
            assert!(
                incomes.self_employment_declared[i] <= incomes.self_employment_income[i],
                "1:{scale}: person {i} declares more self-employment income than they have"
            );
            assert!(
                incomes.informal_income[i] == 0 || INFORMAL_AGES.contains(&pop.persons.age[i]),
                "1:{scale}: person {i} aged {} months has informal income",
                pop.persons.age[i]
            );
        }
        for h in 0..n_hh {
            assert!(
                incomes.hh_subsistence_income[h] == 0
                    || (!pop.households.hh_collective[h] && !housing.hh_urban[h]),
                "1:{scale}: household {h} has subsistence income and is not a rural private household"
            );
        }
    }
}

/// AC-POPI-02 [unit] The stage changes nothing before it: the earlier hashes are those of their own golden tests.
#[test]
fn ac_popi_02() {
    for scale in SCALES {
        let (pop, attrs, jobs, housing, wages) = (
            pop(scale).clone(),
            attrs(scale).clone(),
            jobs(scale).clone(),
            housing(scale).clone(),
            wages(scale).clone(),
        );
        let before = (
            pop.clone(),
            attrs.clone(),
            jobs.clone(),
            housing.clone(),
            wages.clone(),
        );
        assign_other_incomes(
            &pop,
            &attrs,
            &jobs,
            &housing,
            &wages,
            income_margins(),
            &IncomeParams::new(SEED),
        )
        .expect("fixture population gets its other incomes");
        assert_eq!(pop, before.0, "1:{scale}: population changed");
        assert_eq!(attrs, before.1, "1:{scale}: attributes changed");
        assert_eq!(jobs, before.2, "1:{scale}: jobs changed");
        assert_eq!(housing, before.3, "1:{scale}: housing changed");
        assert_eq!(wages, before.4, "1:{scale}: wages changed");
        for (what, hash, file) in [
            ("state", pop.state_hash(), STATE_GOLDEN),
            ("attribute", attrs.state_hash(), ATTRIBUTES_GOLDEN),
            ("jobs", jobs.state_hash(), JOBS_GOLDEN),
            ("housing", housing.state_hash(), HOUSING_GOLDEN),
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

/// AC-POPI-03 [unit] At each of the three scales, weighted self-employment income plus informal income plus subsistence income equals `mixed_income` within 0.001%.
#[test]
fn ac_popi_03() {
    for scale in SCALES {
        assert_mixed_income(
            &format!("1:{scale}"),
            income_margins(),
            pop(scale),
            incomes(scale),
        );
    }
}

/// AC-POPI-04 [unit] At each of the three scales the undeclared share of each industry group's wage bill is within 3 points of its target, and exactly zero where the target is zero.
#[test]
fn ac_popi_04() {
    let m = income_margins();
    assert!(
        m.undeclared_wage_share.contains(&0),
        "the fixture has an industry group with no undeclared wages"
    );
    for scale in SCALES {
        let w = person_weights(pop(scale));
        let (bill, undeclared) = wage_split(jobs(scale), wages(scale), incomes(scale), &w);
        for g in 0..INDUSTRY_GROUPS {
            assert!(bill[g] > 0, "1:{scale}: industry group {g} pays no wages");
            if m.undeclared_wage_share[g] == 0 {
                assert_eq!(
                    undeclared[g], 0,
                    "1:{scale}: industry group {g} has undeclared wages and a target of zero"
                );
                continue;
            }
            let (got, want) = (
                ratio(undeclared[g], bill[g]),
                f64::from(m.undeclared_wage_share[g]) / U as f64,
            );
            assert!(
                (got - want).abs() <= 0.03,
                "1:{scale} industry group {g}: {got:.4} of the wage bill is undeclared, the target is {want:.4}"
            );
        }
    }
}

/// AC-POPI-05 [unit] The shape, at each of the three scales: more than 65% of employees are fully declared; more are partly declared than have no contract; the share of persons aged 15 to 74 with informal income is within half a point of its target, and of rural households growing food within one point; the undeclared share of private labour income is between 20% and 45%.
#[test]
fn ac_popi_05() {
    let m = income_margins();
    let public = fixture(INCOMES_FIXTURE)["industry_groups"]
        .as_array()
        .expect("industry groups")
        .iter()
        .position(|g| g == PUBLIC_GROUP)
        .expect("the public industry group");
    for scale in SCALES {
        let (pop, jobs, housing, wages, incomes) = (
            pop(scale),
            jobs(scale),
            housing(scale),
            wages(scale),
            incomes(scale),
        );
        let w = person_weights(pop);

        let (mut full, mut partly, mut none) = (0u128, 0u128, 0u128);
        let (mut adults, mut recipients) = (0u128, 0u128);
        for i in 0..w.len() {
            let weight = u128::from(w[i]);
            if jobs.employment_status[i] == EMPLOYEE {
                if incomes.wage_declared[i] == wages.wage[i] {
                    full += weight;
                } else if incomes.wage_declared[i] == 0 {
                    none += weight;
                } else {
                    partly += weight;
                }
            }
            if INFORMAL_AGES.contains(&pop.persons.age[i]) {
                adults += weight;
            }
            if incomes.informal_income[i] > 0 {
                recipients += weight;
            }
        }
        let fully_declared = ratio(full, full + partly + none);
        assert!(
            fully_declared > 0.65,
            "1:{scale}: {fully_declared:.4} of employees are fully declared"
        );
        assert!(
            partly > none,
            "1:{scale}: {partly} employees are partly declared, {none} have no contract"
        );

        let (got, want) = (
            ratio(recipients, adults),
            f64::from(m.informal_recipient_share) / U as f64,
        );
        assert!(
            (got - want).abs() <= 0.005,
            "1:{scale}: {got:.4} of persons aged 15 to 74 have informal income, the target is {want:.4}"
        );

        let hh = &pop.households;
        let (mut rural, mut growing) = (0u128, 0u128);
        for h in 0..hh.hh_weight.len() {
            if !hh.hh_collective[h] && !housing.hh_urban[h] {
                rural += u128::from(hh.hh_weight[h]);
                if incomes.hh_subsistence_income[h] > 0 {
                    growing += u128::from(hh.hh_weight[h]);
                }
            }
        }
        let (got, want) = (
            ratio(growing, rural),
            f64::from(m.subsistence_household_share) / U as f64,
        );
        assert!(
            (got - want).abs() <= 0.01,
            "1:{scale}: {got:.4} of rural households grow food, the target is {want:.4}"
        );

        // Private labour income: wages outside the public group, self-employment
        // income and informal income. Undeclared: the undeclared part of wages
        // and of self-employment income, and all informal income.
        let (bill, undeclared) = wage_split(jobs, wages, incomes, &w);
        let private_wages: u128 = (0..INDUSTRY_GROUPS)
            .filter(|&g| g != public)
            .map(|g| bill[g])
            .sum();
        let self_employment = weighted_sum(&w, &incomes.self_employment_income);
        let informal = weighted_sum(&w, &incomes.informal_income);
        let hidden = undeclared.iter().sum::<u128>() + self_employment
            - weighted_sum(&w, &incomes.self_employment_declared)
            + informal;
        let share = ratio(hidden, private_wages + self_employment + informal);
        assert!(
            (0.20..=0.45).contains(&share),
            "1:{scale}: {share:.4} of private labour income is undeclared"
        );
    }
}

/// AC-POPI-06 [unit] Same inputs and seed give identical columns (equal income hash). A different seed changes who gets what; AC-POPI-03 still holds. With every undeclared wage share at zero, `wage_declared` equals `wage` for everyone.
#[test]
fn ac_popi_06() {
    let (m, scale) = (income_margins(), 1000);
    let first = incomes(scale);
    let again = run(scale, m, SEED);
    assert_eq!(&again, first);
    assert_eq!(again.state_hash(), first.state_hash());

    let other = run(scale, m, OTHER_SEED);
    assert_ne!(other.state_hash(), first.state_hash());
    for (name, one, two) in [
        ("wage_declared", &other.wage_declared, &first.wage_declared),
        (
            "self_employment_income",
            &other.self_employment_income,
            &first.self_employment_income,
        ),
        (
            "informal_income",
            &other.informal_income,
            &first.informal_income,
        ),
        (
            "hh_subsistence_income",
            &other.hh_subsistence_income,
            &first.hh_subsistence_income,
        ),
    ] {
        assert_ne!(one, two, "{name} is the same with another seed");
    }
    assert_mixed_income("another seed:", m, pop(scale), &other);

    let mut declared = m.clone();
    declared.undeclared_wage_share.fill(0);
    assert_eq!(
        run(scale, &declared, SEED).wage_declared,
        wages(scale).wage,
        "with no undeclared wages every wage is declared in full"
    );
}

/// AC-POPI-07 [golden] The income hash of the fixture population at each of the three scales equals the committed golden value.
#[test]
fn ac_popi_07() {
    assert_golden(INCOMES_GOLDEN, "income hash", |scale| {
        incomes(scale).state_hash()
    });
}

/// AC-POPI-08 [diff] On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns.
#[test]
fn ac_popi_08() {
    for scale in DIFF_SCALES {
        let incomes = incomes(scale);
        let r = reference_all(scale);
        for (name, got) in [
            ("wage_declared", &incomes.wage_declared),
            ("self_employment_income", &incomes.self_employment_income),
            (
                "self_employment_declared",
                &incomes.self_employment_declared,
            ),
            ("informal_income", &incomes.informal_income),
            ("hh_subsistence_income", &incomes.hh_subsistence_income),
        ] {
            assert_same_column(scale, name, got, &column(r, name));
        }
        assert_eq!(
            hex(incomes.state_hash()),
            reference_hash(r, "incomes_hash"),
            "1:{scale} income hash"
        );
    }
}

/// AC-POPI-09 [unit] Margins with no industry groups, a table of the wrong length, a share above 1 and wages of a different length are each rejected with a named error and produce no output.
#[test]
fn ac_popi_09() {
    let m = income_margins();
    let (pop, attrs, jobs, housing, wages) = (
        pop(1000),
        attrs(1000),
        jobs(1000),
        housing(1000),
        wages(1000),
    );
    let params = IncomeParams::new(SEED);
    let run =
        |m: &IncomeMargins| assign_other_incomes(pop, attrs, jobs, housing, wages, m, &params);

    // The two tables by industry group are the only place the margins name the groups.
    let mut groupless = m.clone();
    groupless.undeclared_wage_share.clear();
    groupless
        .informal_propensity
        .employee_by_industry_group
        .clear();
    assert_eq!(run(&groupless), Err(IncomeError::EmptyTable));

    let mut short = m.clone();
    short.undeclared_wage_share.pop();
    assert_eq!(run(&short), Err(IncomeError::ShapeMismatch));
    let mut short = m.clone();
    short.informal_propensity.employee_by_industry_group.pop();
    assert_eq!(run(&short), Err(IncomeError::ShapeMismatch));

    let over_one = u32::try_from(U).expect("a million") + 1;
    let shares: [fn(&mut IncomeMargins) -> &mut u32; 4] = [
        |m| &mut m.undeclared_wage_share[0],
        |m| &mut m.fully_informal_share,
        |m| &mut m.informal_recipient_share,
        |m| &mut m.subsistence_household_share,
    ];
    for share in shares {
        let mut too_much = m.clone();
        *share(&mut too_much) = over_one;
        assert_eq!(run(&too_much), Err(IncomeError::ShapeMismatch));
    }

    let other = PersonWages {
        wage: shorter(&wages.wage),
    };
    assert_eq!(
        assign_other_incomes(pop, attrs, jobs, housing, &other, m, &params),
        Err(IncomeError::ShapeMismatch)
    );
}
