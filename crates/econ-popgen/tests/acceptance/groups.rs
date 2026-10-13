//! Acceptance test for AC-PG-01 (spec: society/population-groups): the
//! starting population is the generator stages run together. Written in a
//! test-authoring session (ADR-0014). Each stage's own criteria are in its
//! file; this test runs stages A to E in order on the same population and
//! checks that the chain gives each stage's golden hash and consistent shapes.

use econ_popgen::{HousingParams, JobParams, assign_housing, assign_jobs};

use super::support::{
    ATTRIBUTES_GOLDEN, HOUSING_GOLDEN, JOBS_GOLDEN, SCALES, SEED, STATE_GOLDEN, attributes,
    golden_hash, hex, housing_margins, job_margins, population,
};

/// AC-PG-01 [unit] The starting population meets the criteria of its generator specs: AC-POP ([population-generator](population-generator.md)), AC-POPA ([population-attributes](population-attributes.md)), AC-POPJ ([population-jobs](population-jobs.md)) and AC-POPH ([population-housing](population-housing.md)). This criterion adds no test of its own.
#[test]
fn ac_pg_01() {
    for scale in SCALES {
        let pop = population(scale);
        let attrs = attributes(scale);
        let jobs = assign_jobs(pop, attrs, job_margins(), &JobParams::new(SEED))
            .expect("fixture population gets jobs");
        let housing = assign_housing(pop, housing_margins(), &HousingParams::new(SEED))
            .expect("fixture population gets housing");

        let persons = pop.persons.household_id.len();
        let households = pop.households.hh_weight.len();
        assert!(persons > 0 && households > 0, "1:{scale}: empty population");
        assert_eq!(attrs.activity.len(), persons, "1:{scale}: attributes");
        assert_eq!(attrs.edu_level.len(), persons, "1:{scale}: attributes");
        assert_eq!(jobs.employment_status.len(), persons, "1:{scale}: jobs");
        assert_eq!(jobs.occupation.len(), persons, "1:{scale}: jobs");
        assert_eq!(jobs.industry_group.len(), persons, "1:{scale}: jobs");
        assert_eq!(housing.hh_tenure.len(), households, "1:{scale}: housing");
        assert_eq!(housing.hh_urban.len(), households, "1:{scale}: housing");

        for (what, hash, file) in [
            ("state", pop.state_hash(), STATE_GOLDEN),
            ("attribute", attrs.state_hash(), ATTRIBUTES_GOLDEN),
            ("jobs", jobs.state_hash(), JOBS_GOLDEN),
            ("housing", housing.state_hash(), HOUSING_GOLDEN),
        ] {
            assert_eq!(
                hex(hash),
                golden_hash(file, scale),
                "{what} hash at 1:{scale}"
            );
        }
    }
}
