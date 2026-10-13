//! Stage F — wages (spec `society/population-wages`).
//!
//! Gives every employee a gross monthly wage, so that each industry group
//! pays the wage bill of the national accounts and, inside a group, wages
//! differ by sex, occupation, age and education as in the earnings survey.
//! Earlier stages are not changed.
//!
//! `python/reference/popgen_reference.py` (`assign_wages`) is the independent
//! reference; the two agree person for person (AC-POPW-08).

use econ_rng::{KeyedRng, Stream};

use crate::Population;
use crate::arith::{Int, UNIT, rdiv};
use crate::assign::column_hash;
use crate::attributes::{EDU_LEVELS, PersonAttributes};
use crate::jobs::{EDU_GROUPS, INDUSTRY_GROUPS, OCCUPATIONS, PersonJobs};

/// Earnings margins: levels from the national accounts, proportions from the
/// earnings survey. Tables by industry group have [`crate::INDUSTRY_GROUPS`]
/// rows, by occupation [`crate::OCCUPATIONS`] columns, by education group
/// [`crate::EDU_GROUPS`] columns. Proportions are in millionths of the mean
/// wage of the industry group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WageMargins {
    /// Wages and salaries paid per month, in lei, `[industry_group]`.
    pub wage_bill: Vec<u64>,
    /// Gross minimum wage per month, in lei.
    pub minimum_wage: u64,
    /// First age (years) of each age group of `rel_age`, ascending from 0.
    pub age_group_from: Vec<u16>,
    /// The education group of each education level.
    pub edu_group_of_level: [u8; EDU_LEVELS],
    /// `[industry_group][sex]`.
    pub rel_sex: Vec<u32>,
    /// `[industry_group][sex][occupation]`, row-major.
    pub rel_occupation: Vec<u32>,
    /// `[industry_group][sex][age_group]`, row-major.
    pub rel_age: Vec<u32>,
    /// `[industry_group][sex][edu_group]`, row-major.
    pub rel_education: Vec<u32>,
    /// Ranks of the knots of the quantile curve, in millionths: ascending
    /// from 0 to 1,000,000.
    pub curve_rank: Vec<u32>,
    /// Earnings at each knot over mean earnings, in millionths; not decreasing.
    pub curve_value: Vec<u32>,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WageParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Share of the quantile curve's spread that is left inside a cell, in
    /// millionths.
    pub dispersion_ppm: u32,
    /// The floor of an industry group is the minimum wage, or this share of
    /// the group's mean wage if that is lower; in millionths.
    pub floor_share_ppm: u32,
    /// Passes of scaling to the survey's proportions.
    pub scaling_passes: u32,
}

impl WageParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        WageParams {
            rng_seed,
            dispersion_ppm: 500_000,
            floor_share_ppm: 800_000,
            scaling_passes: 20,
        }
    }
}

/// The wage column, aligned with the persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonWages {
    /// Gross monthly wage in bani; 0 for everyone who is not an employee.
    pub wage: Vec<u64>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WageError {
    /// No age groups, or a quantile curve with fewer than two knots.
    EmptyTable,
    /// A table's length does not match its dimensions, a proportion is zero,
    /// the quantile curve is not ascending from rank 0 to 1,000,000, or the
    /// attributes or jobs belong to another population.
    ShapeMismatch,
    /// An industry group has employees but no wage bill.
    NoMargin {
        /// The industry group index.
        industry_group: u8,
    },
}

/// Assign a gross monthly wage to every employee.
///
/// # Errors
/// See [`WageError`].
pub fn assign_wages(
    pop: &Population,
    attrs: &PersonAttributes,
    jobs: &PersonJobs,
    margins: &WageMargins,
    params: &WageParams,
) -> Result<PersonWages, WageError> {
    check_margins(margins)?;
    let p = &pop.persons;
    let n = p.household_id.len();
    if attrs.edu_level.len() != n
        || jobs.employment_status.len() != n
        || jobs.occupation.len() != n
        || jobs.industry_group.len() != n
    {
        return Err(WageError::ShapeMismatch);
    }
    let (n_age, n_edu) = (margins.age_group_from.len(), EDU_GROUPS);
    let mut by_group: Vec<Vec<usize>> = vec![Vec::new(); INDUSTRY_GROUPS];
    for i in 0..n {
        if jobs.employment_status[i] == KIND_EMPLOYEE {
            let g = usize::from(jobs.industry_group[i]);
            if g >= INDUSTRY_GROUPS || usize::from(jobs.occupation[i]) >= OCCUPATIONS {
                return Err(WageError::ShapeMismatch);
            }
            by_group[g].push(i);
        }
    }
    let rng = KeyedRng::new(params.rng_seed);
    let mut wage = vec![0u64; n];
    for (g, members) in by_group.iter_mut().enumerate() {
        if members.is_empty() {
            continue;
        }
        let bill = Int::from(margins.wage_bill[g]) * BANI_PER_LEU;
        if bill <= 0 {
            return Err(WageError::NoMargin {
                industry_group: u8::try_from(g).expect("industry group fits u8"),
            });
        }
        members
            .sort_by_cached_key(|&i| (rng.draw(Stream::PopulationGen, STAGE, i as u64).u64(), i));
        let w: Vec<Int> = members
            .iter()
            .map(|&i| Int::from(pop.households.hh_weight[p.household_id[i] as usize]))
            .collect();
        let total_w: Int = w.iter().sum();
        let sex: Vec<usize> = members.iter().map(|&i| p.sex[i] as usize).collect();
        let age_group = |i: usize| {
            let years = p.age[i] / MONTHS;
            margins
                .age_group_from
                .iter()
                .rposition(|&from| from <= years)
                .expect("the first age group starts at 0")
        };
        let cells = |n_cells: usize, cell_of: &dyn Fn(usize) -> usize| -> Vec<usize> {
            members
                .iter()
                .zip(&sex)
                .map(|(&i, &s)| s * n_cells + cell_of(i))
                .collect()
        };
        let dims = [
            (
                cells(OCCUPATIONS, &|i| usize::from(jobs.occupation[i])),
                group_table(&margins.rel_occupation, g, OCCUPATIONS),
            ),
            (
                cells(n_age, &age_group),
                group_table(&margins.rel_age, g, n_age),
            ),
            (
                cells(n_edu, &|i| {
                    usize::from(margins.edu_group_of_level[attrs.edu_level[i] as usize])
                }),
                group_table(&margins.rel_education, g, n_edu),
            ),
        ];
        let floor = (Int::from(margins.minimum_wage) * BANI_PER_LEU).min(rdiv(
            Int::from(params.floor_share_ppm) * bill,
            total_w * UNIT,
        ));
        let group = Group {
            w: &w,
            sex: &sex,
            rel_sex: [
                Int::from(margins.rel_sex[2 * g]),
                Int::from(margins.rel_sex[2 * g + 1]),
            ],
            curve_rank: &margins.curve_rank,
            curve_value: &margins.curve_value,
            dispersion_ppm: params.dispersion_ppm,
            scaling_passes: params.scaling_passes,
            bill,
            floor,
        };
        let amounts = spread(&group, &dims);
        for (&i, a) in members.iter().zip(amounts) {
            wage[i] = u64::try_from(a).expect("wage fits u64");
        }
    }
    Ok(PersonWages { wage })
}

/// `tick` of the draw context: the stage number.
const STAGE: u32 = 5;
/// `employment_status` of an employee.
const KIND_EMPLOYEE: u8 = 1;
const BANI_PER_LEU: Int = 100;
const MONTHS: u16 = 12;

fn check_margins(m: &WageMargins) -> Result<(), WageError> {
    let n_age = m.age_group_from.len();
    if n_age == 0 || m.curve_rank.len() < 2 {
        return Err(WageError::EmptyTable);
    }
    let n_g = INDUSTRY_GROUPS;
    let table_ok = |t: &[u32], inner: usize| t.len() == n_g * 2 * inner && t.iter().all(|&v| v > 0);
    let ok = m.wage_bill.len() == n_g
        && table_ok(&m.rel_sex, 1)
        && table_ok(&m.rel_occupation, OCCUPATIONS)
        && table_ok(&m.rel_age, n_age)
        && table_ok(&m.rel_education, EDU_GROUPS)
        && curve_ok(&m.curve_rank, &m.curve_value)
        && m.edu_group_of_level
            .iter()
            .all(|&e| usize::from(e) < EDU_GROUPS)
        && m.age_group_from[0] == 0;
    if ok {
        Ok(())
    } else {
        Err(WageError::ShapeMismatch)
    }
}

/// One table of industry group `g` (`[sex][inner]`, row-major).
fn group_table(t: &[u32], g: usize, inner: usize) -> Vec<Int> {
    t[g * 2 * inner..(g + 1) * 2 * inner]
        .iter()
        .map(|&v| Int::from(v))
        .collect()
}

/// A quantile curve of at least two knots ascends from rank 0 to `UNIT`, starts
/// above zero and does not decrease.
pub(crate) fn curve_ok(rank: &[u32], value: &[u32]) -> bool {
    value.len() == rank.len()
        && rank[0] == 0
        && Int::from(rank[rank.len() - 1]) == UNIT
        && rank.windows(2).all(|r| r[0] < r[1])
        && value[0] > 0
        && value.windows(2).all(|v| v[0] <= v[1])
}

/// The quantile curve at `rank` (millionths, `0 <= rank < UNIT`): a line through its knots.
fn curve_at(ranks: &[u32], values: &[u32], rank: Int) -> Int {
    let k = (0..ranks.len() - 1)
        .rev()
        .find(|&i| Int::from(ranks[i]) <= rank)
        .expect("the curve starts at rank 0");
    let (r0, r1) = (Int::from(ranks[k]), Int::from(ranks[k + 1]));
    let (v0, v1) = (Int::from(values[k]), Int::from(values[k + 1]));
    v0 + ((v1 - v0) * (rank - r0)).div_euclid(r1 - r0)
}

/// One group to spread an amount over (an industry group's employees, or the
/// retired): its members in keyed-random order and what they are to be paid.
pub(crate) struct Group<'a> {
    /// Members' weights.
    pub w: &'a [Int],
    /// Members' sexes (0 F, 1 M).
    pub sex: &'a [usize],
    /// Mean of each sex, as proportions.
    pub rel_sex: [Int; 2],
    /// Knots of the quantile curve.
    pub curve_rank: &'a [u32],
    pub curve_value: &'a [u32],
    /// Share of the curve's spread that is used, in millionths.
    pub dispersion_ppm: u32,
    /// Passes of scaling to the proportions.
    pub scaling_passes: u32,
    /// What the group is paid, in bani.
    pub bill: Int,
    /// Nobody gets less, in bani.
    pub floor: Int,
}

/// Amounts for the members of one group, in bani: the group pays `bill`,
/// nobody gets less than `floor` (spec `society/population-wages`, steps 2 to
/// 4). `dims` are the tables of proportions inside each sex: each member's
/// cell (`sex * width + category`) and the proportion of each cell.
pub(crate) fn spread(group: &Group<'_>, dims: &[(Vec<usize>, Vec<Int>)]) -> Vec<Int> {
    let (w, sex, floor) = (group.w, group.sex, group.floor);
    let total_w: Int = w.iter().sum();
    let dispersion = Int::from(group.dispersion_ppm);
    // A place on the quantile curve from the rank (the midpoint of the member's own weight).
    let mut x = Vec::with_capacity(w.len());
    let mut before = 0;
    for &wi in w {
        let rank = (2 * before + wi) * UNIT / (2 * total_w);
        x.push(
            (UNIT * UNIT
                + dispersion * (curve_at(group.curve_rank, group.curve_value, rank) - UNIT))
                .div_euclid(UNIT),
        );
        before += wi;
    }

    // The mean each sex is to have (the group's mean is UNIT) is fixed first,
    // so that every table asks for the same thing.
    let mut sex_w = [0; 2];
    for (&s, &wi) in sex.iter().zip(w) {
        sex_w[s] += wi;
    }
    let sex_norm: Int = sex_w[0] * group.rel_sex[0] + sex_w[1] * group.rel_sex[1];
    let sex_mean = group.rel_sex.map(|r| rdiv(r * total_w * UNIT, sex_norm));
    for _ in 0..group.scaling_passes {
        for (cells, targets) in dims {
            let mut weight_of = vec![0; targets.len()];
            let mut sum_of = vec![0; targets.len()];
            for ((&c, &wi), &xi) in cells.iter().zip(w).zip(&x) {
                weight_of[c] += wi;
                sum_of[c] += wi * xi;
            }
            let width = targets.len() / 2;
            let norm: [Int; 2] = std::array::from_fn(|k| {
                weight_of[k * width..(k + 1) * width]
                    .iter()
                    .zip(&targets[k * width..])
                    .map(|(wc, t)| wc * t)
                    .sum()
            });
            for ((&c, &s), xi) in cells.iter().zip(sex).zip(x.iter_mut()) {
                if sum_of[c] != 0 {
                    *xi = rdiv(
                        *xi * targets[c] * weight_of[c] * sex_w[s] * sex_mean[s],
                        sum_of[c] * norm[s],
                    );
                }
            }
        }
    }

    // The level: the group pays its bill, nobody is below the floor.
    let paid = |level: Int| -> Int {
        w.iter()
            .zip(&x)
            .map(|(wi, &xi)| wi * rdiv(level * xi, UNIT).max(floor))
            .sum()
    };
    let bill = group.bill;
    let (mut lo, mut hi) = (0, rdiv(2 * bill, total_w) + 1);
    while paid(hi) < bill {
        hi *= 2;
    }
    while lo < hi {
        let mid = (lo + hi) / 2;
        if paid(mid) >= bill {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    x.iter().map(|&xi| rdiv(lo * xi, UNIT).max(floor)).collect()
}

/// Spread `bill` over members of weights `w` (in this order) without a split
/// by sex and with no floor; `dims` as in [`spread`], all cells of sex 0.
pub(crate) fn spread_unsplit(
    w: &[Int],
    dims: &[(Vec<usize>, Vec<Int>)],
    curve: (&[u32], &[u32]),
    dispersion_ppm: u32,
    scaling_passes: u32,
    bill: Int,
) -> Vec<Int> {
    let sex = vec![0; w.len()];
    let group = Group {
        w,
        sex: &sex,
        rel_sex: [UNIT, UNIT],
        curve_rank: curve.0,
        curve_value: curve.1,
        dispersion_ppm,
        scaling_passes,
        bill,
        floor: 0,
    };
    spread(&group, dims)
}

impl PersonWages {
    /// Wage hash (FNV-1a 64 over the person count, then each person's wage;
    /// see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        column_hash(
            self.wage.len(),
            self.wage.iter().flat_map(|w| w.to_le_bytes()),
        )
    }
}
