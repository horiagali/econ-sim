//! Stage G — pensions (spec `society/population-pensions`).
//!
//! Gives every retired person a gross monthly pension, so that the pension
//! bill of the year is paid, women's and men's pensions stand as the gender
//! pension gap says, and about as many pensioners are at the minimum pension
//! as in reality. Earlier stages are not changed.
//!
//! The retired are one group spread like the employees of an industry group
//! in stage F ([`crate::wages`]). `python/reference/popgen_reference.py`
//! (`assign_pensions`) is the independent reference; the two agree person for
//! person (AC-POPP-08).

use econ_rng::{KeyedRng, Stream};

use crate::Population;
use crate::arith::{Int, UNIT};
use crate::assign::column_hash;
use crate::attributes::{Activity, EDU_LEVELS, PersonAttributes};
use crate::jobs::EDU_GROUPS;
use crate::wages::{Group, curve_ok, spread};

/// `tick` of the draw context: the stage number.
const STAGE: u32 = 6;
const BANI_PER_LEU: Int = 100;

/// Pension margins. Proportions are in millionths. Tables by education group
/// have [`crate::EDU_GROUPS`] columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PensionMargins {
    /// Pensions paid per month, in lei.
    pub pension_bill: u64,
    /// The minimum pension per month, in lei.
    pub minimum_pension: u64,
    /// Mean pension of each sex, as proportions, `[sex]`.
    pub rel_sex: [u32; 2],
    /// The education group of each education level.
    pub edu_group_of_level: [u8; EDU_LEVELS],
    /// Mean earnings of each education group over the mean of its sex,
    /// `[sex][edu_group]`, row-major.
    pub rel_education: Vec<u32>,
    /// Ranks of the knots of the pension quantile curve, in millionths:
    /// ascending from 0 to 1,000,000.
    pub curve_rank: Vec<u32>,
    /// Pension at each knot over the mean pension, in millionths; not decreasing.
    pub curve_value: Vec<u32>,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PensionParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Share of the quantile curve's spread that is used, in millionths.
    pub dispersion_ppm: u32,
    /// How much of the earnings gap between education groups pensions keep,
    /// in millionths.
    pub earnings_link_ppm: u32,
    /// Passes of scaling to the proportions.
    pub scaling_passes: u32,
}

impl PensionParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        PensionParams {
            rng_seed,
            dispersion_ppm: 1_000_000,
            earnings_link_ppm: 500_000,
            scaling_passes: 20,
        }
    }
}

/// The pension column, aligned with the persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonPensions {
    /// Gross monthly pension in bani; 0 for everyone who is not retired.
    pub pension: Vec<u64>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PensionError {
    /// No education groups, or a quantile curve with fewer than two knots.
    EmptyTable,
    /// A table's length does not match its dimensions, a proportion is zero,
    /// the quantile curve is not ascending from rank 0 to 1,000,000, or the
    /// attributes belong to another population.
    ShapeMismatch,
    /// There are retired persons but no pension bill.
    NoMargin,
}

/// Assign a gross monthly pension to every retired person.
///
/// # Errors
/// See [`PensionError`].
pub fn assign_pensions(
    pop: &Population,
    attrs: &PersonAttributes,
    margins: &PensionMargins,
    params: &PensionParams,
) -> Result<PersonPensions, PensionError> {
    let m = margins;
    if m.rel_education.is_empty() || m.curve_rank.len() < 2 {
        return Err(PensionError::EmptyTable);
    }
    let shape_ok = m.rel_sex.iter().all(|&r| r > 0)
        && m.rel_education.len() == 2 * EDU_GROUPS
        && m.rel_education.iter().all(|&v| v > 0)
        && curve_ok(&m.curve_rank, &m.curve_value)
        && m.edu_group_of_level
            .iter()
            .all(|&e| usize::from(e) < EDU_GROUPS);
    let p = &pop.persons;
    let n = p.household_id.len();
    if !shape_ok || attrs.activity.len() != n || attrs.edu_level.len() != n {
        return Err(PensionError::ShapeMismatch);
    }
    let mut pension = vec![0u64; n];
    let mut members: Vec<usize> = (0..n)
        .filter(|&i| attrs.activity[i] == Activity::Retired)
        .collect();
    if members.is_empty() {
        return Ok(PersonPensions { pension });
    }
    let bill = Int::from(m.pension_bill) * BANI_PER_LEU;
    if bill <= 0 {
        return Err(PensionError::NoMargin);
    }
    let rng = KeyedRng::new(params.rng_seed);
    members.sort_by_cached_key(|&i| (rng.draw(Stream::PopulationGen, STAGE, i as u64).u64(), i));
    let w: Vec<Int> = members
        .iter()
        .map(|&i| Int::from(pop.households.hh_weight[p.household_id[i] as usize]))
        .collect();
    let sex: Vec<usize> = members.iter().map(|&i| p.sex[i] as usize).collect();
    // The proportions between education groups: the earnings proportions pulled towards 1.
    let link = Int::from(params.earnings_link_ppm);
    let targets: Vec<Int> = m
        .rel_education
        .iter()
        .map(|&v| (UNIT * UNIT + link * (Int::from(v) - UNIT)).div_euclid(UNIT))
        .collect();
    let cells: Vec<usize> = members
        .iter()
        .zip(&sex)
        .map(|(&i, &s)| {
            s * EDU_GROUPS + usize::from(m.edu_group_of_level[attrs.edu_level[i] as usize])
        })
        .collect();
    // The minimum pension, or the mean pension if the bill cannot pay even that to everyone.
    let total_w: Int = w.iter().sum();
    let floor = (Int::from(m.minimum_pension) * BANI_PER_LEU).min(bill / total_w);
    let group = Group {
        w: &w,
        sex: &sex,
        rel_sex: m.rel_sex.map(Int::from),
        curve_rank: &m.curve_rank,
        curve_value: &m.curve_value,
        dispersion_ppm: params.dispersion_ppm,
        scaling_passes: params.scaling_passes,
        bill,
        floor,
    };
    for (&i, a) in members.iter().zip(spread(&group, &[(cells, targets)])) {
        pension[i] = u64::try_from(a).expect("pension fits u64");
    }
    Ok(PersonPensions { pension })
}

impl PersonPensions {
    /// Pension hash (FNV-1a 64 over the person count, then each person's
    /// pension; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        column_hash(
            self.pension.len(),
            self.pension.iter().flat_map(|v| v.to_le_bytes()),
        )
    }
}
