//! Stage H — household wealth (spec `society/population-wealth`).
//!
//! Gives every private household cash, deposits, a dwelling value if it owns
//! its home, and possibly a housing loan and a consumer loan, so that the
//! household sector holds the totals of the financial accounts. Who holds
//! what is an educated guess (no survey covers Romania); the spec says which.
//! Earlier stages are not changed.
//!
//! **Skeleton.** This file fixes the API the acceptance tests are written
//! against (ADR-0014, tests-first). The stage is not implemented yet; the
//! `wealth` feature that enables its acceptance tests is switched on by the
//! implementation change. `python/reference/popgen_reference.py`
//! (`assign_wealth`) is the independent reference.

use crate::Population;
use crate::housing::HouseholdHousing;
use crate::pensions::PersonPensions;
use crate::wages::PersonWages;

/// A quantile curve: the value at a rank over the mean, as a line through knots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Curve {
    /// Ranks of the knots, in millionths: ascending from 0 to 1,000,000.
    pub rank: Vec<u32>,
    /// Value at each knot over the mean, in millionths; not decreasing.
    pub value: Vec<u32>,
}

/// Who has a loan of one kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorrowerRule {
    /// Share of private households that have the loan, in millionths.
    pub share_ppm: u32,
    /// Youngest age (years) of a borrowing household's head.
    pub age_from: u16,
    /// First age (years) at which a head no longer borrows.
    pub age_to: u16,
    /// How closely borrowing follows income, in millionths.
    pub income_link_ppm: u32,
    /// Added to the key of an urban household, in millionths.
    pub urban_bonus_ppm: u32,
}

/// Wealth margins: totals of the household sector in lei, and the rules for
/// who holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WealthMargins {
    /// Currency held by households.
    pub cash: u64,
    /// Deposits of households.
    pub deposits: u64,
    /// Housing loans outstanding.
    pub housing_loans: u64,
    /// Consumer loans outstanding.
    pub consumer_loans: u64,
    /// Market value of owned dwellings.
    pub dwellings: u64,
    /// Mean dwelling value of each locality class, as proportions, in millionths.
    pub rel_locality: Vec<u32>,
    /// Cash and deposits together.
    pub liquid_curve: Curve,
    /// Dwelling values.
    pub dwelling_curve: Curve,
    /// Loan balances, for both kinds of loan.
    pub loan_curve: Curve,
    /// Housing loans.
    pub mortgage: BorrowerRule,
    /// Consumer loans.
    pub consumer: BorrowerRule,
    /// How closely a household's place on the liquid curve follows its income, in millionths.
    pub liquid_income_link_ppm: u32,
    /// The same for the dwelling curve.
    pub dwelling_income_link_ppm: u32,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WealthParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Passes of scaling to the proportions between locality classes.
    pub scaling_passes: u32,
}

impl WealthParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        WealthParams {
            rng_seed,
            scaling_passes: 20,
        }
    }
}

/// Wealth columns, aligned with the households table; amounts in bani.
/// Records that are not private households have zero in every column.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HouseholdWealth {
    /// Currency.
    pub hh_cash: Vec<u64>,
    /// Bank deposits.
    pub hh_deposits: Vec<u64>,
    /// Market value of the dwelling; 0 for a household that does not own its home.
    pub hh_dwelling_value: Vec<u64>,
    /// Housing loan outstanding.
    pub hh_mortgage: Vec<u64>,
    /// Consumer loans outstanding.
    pub hh_consumer_debt: Vec<u64>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WealthError {
    /// No locality classes, or a curve with fewer than two knots.
    EmptyTable,
    /// A proportion of zero, a curve that does not ascend from rank 0 to
    /// 1,000,000, a share above 1, an empty age range, a locality class the
    /// margins do not have, or columns of another population.
    ShapeMismatch,
}

/// Assign cash, deposits, dwelling value and loans to every household.
/// Without wages or pensions every income counts as zero.
///
/// # Errors
/// See [`WealthError`].
pub fn assign_wealth(
    pop: &Population,
    housing: &HouseholdHousing,
    wages: Option<&PersonWages>,
    pensions: Option<&PersonPensions>,
    margins: &WealthMargins,
    params: &WealthParams,
) -> Result<HouseholdWealth, WealthError> {
    let _ = (pop, housing, wages, pensions, margins, params);
    unimplemented!("econ-popgen: stage H is written in the implementation change")
}

impl HouseholdWealth {
    /// Wealth hash (FNV-1a 64 over the household count, then each
    /// household's five amounts; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        unimplemented!("econ-popgen: stage H is written in the implementation change")
    }
}
