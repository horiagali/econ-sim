//! Stage I — the income split (spec `society/population-incomes`).
//!
//! Splits every wage into a declared and an undeclared part, gives the
//! self-employed their income and its declared part, gives some people
//! informal income outside a job and some rural households what they grow for
//! themselves, so that households receive the mixed income of the national
//! accounts. Every rule about who gets what is an educated guess; the spec
//! says which. Earlier stages are not changed.
//!
//! **Skeleton.** This file fixes the API the acceptance tests are written
//! against (ADR-0014, tests-first). The stage is not implemented yet; the
//! `incomes` feature that enables its acceptance tests is switched on by the
//! implementation change. `python/reference/popgen_reference.py`
//! (`assign_other_incomes`) is the independent reference.

use crate::Population;
use crate::attributes::PersonAttributes;
use crate::housing::HouseholdHousing;
use crate::jobs::PersonJobs;
use crate::wages::PersonWages;
use crate::wealth::Curve;

/// How likely each kind of person is to have informal income outside a job,
/// relative to each other, in millionths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformalPropensity {
    /// Unemployed persons.
    pub unemployed: u32,
    /// Other inactive persons.
    pub inactive_other: u32,
    /// Retired persons.
    pub retired: u32,
    /// Pupils and students.
    pub student: u32,
    /// Employers.
    pub employer: u32,
    /// Own-account workers.
    pub own_account: u32,
    /// Family workers.
    pub family_worker: u32,
    /// Employees, `[industry_group]`.
    pub employee_by_industry_group: Vec<u32>,
}

/// Income margins: amounts in lei per month, shares in millionths. Tables by
/// industry group have [`crate::INDUSTRY_GROUPS`] entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomeMargins {
    /// Farm income of households, in cash and in kind.
    pub farm_income: u64,
    /// The part of it that households grow and consume themselves.
    pub farm_in_kind: u64,
    /// Income of employers, own-account and family workers outside farming.
    pub self_employment_income: u64,
    /// Informal income outside a job.
    pub informal_income: u64,
    /// Gross minimum wage: what an envelope job declares at least.
    pub minimum_wage: u64,
    /// The earnings curve, for the spread of every amount.
    pub quantile_curve: Curve,
    /// Share of each industry group's wage bill paid outside contracts.
    pub undeclared_wage_share: Vec<u32>,
    /// Of the undeclared wages, the part earned in jobs with no contract.
    pub fully_informal_share: u32,
    /// What an envelope job declares of the true wage, if above the minimum wage.
    pub envelope_declared_share: u32,
    /// How strongly low wages come first among jobs with no contract.
    pub low_wage_link: u32,
    /// Mean income of an employer, an own-account worker and a family worker, as proportions.
    pub rel_kind: [u32; 3],
    /// How closely a self-employed person's place follows education.
    pub self_employment_education_link: u32,
    /// Declared share of self-employment income outside farming.
    pub self_employment_declared: u32,
    /// Declared share of farm income in cash.
    pub farm_declared: u32,
    /// Share of rural private households that grow food for themselves.
    pub subsistence_household_share: u32,
    /// Added to the key of a household with a farm worker.
    pub subsistence_bonus_farm_worker: u32,
    /// Added to the key of a household with a retired member and no farm worker.
    pub subsistence_bonus_retired: u32,
    /// Share of the curve's spread used for what households grow.
    pub subsistence_dispersion: u32,
    /// Share of persons aged 15 to 74 with informal income outside a job.
    pub informal_recipient_share: u32,
    /// Who they are.
    pub informal_propensity: InformalPropensity,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncomeParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Passes of scaling to the proportions between kinds of self-employed.
    pub scaling_passes: u32,
}

impl IncomeParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        IncomeParams {
            rng_seed,
            scaling_passes: 20,
        }
    }
}

/// The columns of the stage; amounts in bani per month.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OtherIncomes {
    /// Per person: the part of the wage that is on the contract (the whole
    /// wage, a part, or 0).
    pub wage_declared: Vec<u64>,
    /// Per person: income as an employer, own-account or family worker.
    pub self_employment_income: Vec<u64>,
    /// Per person: its declared part.
    pub self_employment_declared: Vec<u64>,
    /// Per person: informal income outside a job.
    pub informal_income: Vec<u64>,
    /// Per household: value of what it grows and consumes itself.
    pub hh_subsistence_income: Vec<u64>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncomeError {
    /// No industry groups, or a curve with fewer than two knots.
    EmptyTable,
    /// A table's length does not match its dimensions, a share above 1, a
    /// proportion of zero, farm income in kind above farm income, a curve
    /// that does not ascend from rank 0 to 1,000,000, or columns of another
    /// population.
    ShapeMismatch,
}

/// Split wages and assign the other incomes.
///
/// # Errors
/// See [`IncomeError`].
pub fn assign_other_incomes(
    pop: &Population,
    attrs: &PersonAttributes,
    jobs: &PersonJobs,
    housing: &HouseholdHousing,
    wages: &PersonWages,
    margins: &IncomeMargins,
    params: &IncomeParams,
) -> Result<OtherIncomes, IncomeError> {
    let _ = (pop, attrs, jobs, housing, wages, margins, params);
    unimplemented!("econ-popgen: stage I is written in the implementation change")
}

impl OtherIncomes {
    /// Income hash (FNV-1a 64 over the person count and each person's four
    /// amounts, then the household count and each household's amount; see
    /// the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("econ-popgen: stage I is written in the implementation change")
    }
}
