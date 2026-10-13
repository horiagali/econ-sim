//! Stage J — employers (spec `society/population-employers`).
//!
//! Gives every employed person a catalogue industry and a firm unit, inside
//! the industry group that the jobs stage gave them, so that each unit has
//! its share of the group's employees and better-paid employees work in
//! better-paying units. Earlier stages are not changed.
//!
//! `python/reference/popgen_reference.py` (`assign_employers`) is the
//! independent reference; the two agree person for person (AC-POPE-07).

use econ_rng::{KeyedRng, Stream};

use crate::Population;
use crate::arith::{Int, UNIT};
use crate::assign::{column_hash, deal, split};
use crate::jobs::PersonJobs;
use crate::wages::PersonWages;
use crate::wealth::{draw_ppm, mixed_order};

/// `tick` of the draw context: the stage number.
const STAGE: u32 = 10;
/// `employment_status` codes.
const KIND_NONE: u8 = 0;
const KIND_EMPLOYEE: u8 = 1;

/// `industry` of a person who is not employed.
pub const NO_INDUSTRY: u8 = 255;
/// `employer` of a person who is not employed.
pub const NO_EMPLOYER: u16 = 65535;

/// Size class of a firm unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum UnitSize {
    /// Cohort of firms with fewer than 10 persons employed.
    Micro = 0,
    /// Cohort of firms with 10 to 49.
    Small = 1,
    /// Cohort of firms with 50 or more.
    Large = 2,
    /// Non-market public producers.
    Public = 3,
}

/// What the stage needs to know of a firm unit (ADR-0016).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirmUnit {
    /// Index of its industry in [`FirmUnits::industry_group`].
    pub industry: u8,
    /// Size class.
    pub size: UnitSize,
    /// The unit is controlled from Romania and is not a public unit.
    pub domestic: bool,
    /// Persons employed by the unit (all its firms together).
    pub persons_employed: u64,
    /// Its wage bill, in lei a year.
    pub wages: u64,
}

/// The firm units of the starting economy, as far as this stage reads them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmUnits {
    /// The industry group of each catalogue industry, in the catalogue's order.
    pub industry_group: Vec<u8>,
    /// The units.
    pub units: Vec<FirmUnit>,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmployerParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// How closely an employee's place among employers follows the wage, in millionths.
    pub wage_link_ppm: u32,
}

impl EmployerParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        EmployerParams {
            rng_seed,
            wage_link_ppm: 500_000,
        }
    }
}

/// The columns of the stage, aligned with the persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonEmployers {
    /// Catalogue industry, or [`NO_INDUSTRY`].
    pub industry: Vec<u8>,
    /// Firm unit, or [`NO_EMPLOYER`].
    pub employer: Vec<u16>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmployerError {
    /// No industries or no units.
    EmptyTable,
    /// A unit of an industry that is not listed, more industries or units
    /// than the columns can hold, or columns of another population.
    ShapeMismatch,
    /// Someone works in an industry group that has no unit.
    NoMargin {
        /// The industry group.
        industry_group: u8,
    },
}

/// Assign a catalogue industry and a firm unit to every employed person.
/// Without wages, employees are placed by the draw alone.
///
/// # Errors
/// See [`EmployerError`].
pub fn assign_employers(
    pop: &Population,
    jobs: &PersonJobs,
    wages: Option<&PersonWages>,
    firms: &FirmUnits,
    params: &EmployerParams,
) -> Result<PersonEmployers, EmployerError> {
    let (industries, units) = (&firms.industry_group, &firms.units);
    if industries.is_empty() || units.is_empty() {
        return Err(EmployerError::EmptyTable);
    }
    let p = &pop.persons;
    let n = p.household_id.len();
    let (status, group) = (&jobs.employment_status, &jobs.industry_group);
    if status.len() != n
        || group.len() != n
        || wages.is_some_and(|w| w.wage.len() != n)
        || industries.len() > usize::from(NO_INDUSTRY)
        || units.len() > usize::from(NO_EMPLOYER)
        || units
            .iter()
            .any(|u| usize::from(u.industry) >= industries.len())
    {
        return Err(EmployerError::ShapeMismatch);
    }
    let n_groups = 1 + usize::from(*industries.iter().max().expect("industries"));
    let mut units_of: Vec<Vec<usize>> = vec![Vec::new(); n_groups];
    for (k, u) in units.iter().enumerate() {
        units_of[usize::from(industries[usize::from(u.industry)])].push(k);
    }
    let pw: Vec<Int> = p
        .household_id
        .iter()
        .map(|&h| Int::from(pop.households.hh_weight[h as usize]))
        .collect();
    let pay: Vec<Int> = match wages {
        Some(w) => w.wage.iter().map(|&v| Int::from(v)).collect(),
        None => vec![0; n],
    };
    let employed: Vec<usize> = (0..n).filter(|&i| status[i] != KIND_NONE).collect();
    if let Some(&i) = employed.iter().find(|&&i| {
        units_of
            .get(usize::from(group[i]))
            .is_none_or(Vec::is_empty)
    }) {
        return Err(EmployerError::NoMargin {
            industry_group: group[i],
        });
    }
    let rng = KeyedRng::new(params.rng_seed);
    let mut prio = vec![0u64; n];
    for &i in &employed {
        prio[i] = rng.draw(Stream::PopulationGen, STAGE, i as u64).u64();
    }
    let draw = |i: usize| draw_ppm(prio[i]);
    let mut out = PersonEmployers {
        industry: vec![NO_INDUSTRY; n],
        employer: vec![NO_EMPLOYER; n],
    };
    let mut put = |i: usize, k: usize| {
        out.employer[i] = u16::try_from(k).expect("fewer units than NO_EMPLOYER");
        out.industry[i] = units[k].industry;
    };
    let persons_employed = |pool: &[usize]| -> Vec<Int> {
        let t: Vec<Int> = pool
            .iter()
            .map(|&k| Int::from(units[k].persons_employed))
            .collect();
        if t.iter().any(|&v| v != 0) {
            t
        } else {
            vec![1; pool.len()]
        }
    };
    for (g, group_units) in units_of.iter().enumerate() {
        let in_group = |i: &&usize| usize::from(group[**i]) == g;
        let employees: Vec<usize> = employed
            .iter()
            .filter(in_group)
            .copied()
            .filter(|&i| status[i] == KIND_EMPLOYEE)
            .collect();
        if !employees.is_empty() {
            // Units from the lowest to the highest pay per person.
            let mut order_u = group_units.clone();
            order_u.sort_by_key(|&k| {
                let u = &units[k];
                (
                    Int::from(u.wages) * UNIT / Int::from(u.persons_employed.max(1)),
                    k,
                )
            });
            let targets = persons_employed(&order_u);
            let order = mixed_order(&employees, &pw, &pay, &draw, params.wage_link_ppm, &|_| 0);
            let w: Vec<Int> = order.iter().map(|&i| pw[i]).collect();
            let mut carry = vec![0; targets.len()];
            for (&i, c) in order.iter().zip(split(&w, &targets, &mut carry)) {
                put(i, order_u[c]);
            }
        }
        let mut own: Vec<usize> = employed
            .iter()
            .filter(in_group)
            .copied()
            .filter(|&i| status[i] != KIND_EMPLOYEE)
            .collect();
        if !own.is_empty() {
            own.sort_by_key(|&i| (prio[i], i));
            let pick = |keep: &dyn Fn(&FirmUnit) -> bool| -> Vec<usize> {
                group_units
                    .iter()
                    .copied()
                    .filter(|&k| keep(&units[k]))
                    .collect()
            };
            let mut pool = pick(&|u| u.size == UnitSize::Micro && u.domestic);
            if pool.is_empty() {
                pool = pick(&|u| u.size != UnitSize::Public);
            }
            if pool.is_empty() {
                pool.clone_from(group_units);
            }
            let targets = persons_employed(&pool);
            let w: Vec<Int> = own.iter().map(|&i| pw[i]).collect();
            let mut carry = vec![0; targets.len()];
            for (&i, c) in own.iter().zip(deal(&w, &targets, &mut carry)) {
                put(i, pool[c]);
            }
        }
    }
    Ok(out)
}

impl PersonEmployers {
    /// Employer hash (FNV-1a 64 over the person count, then each person's
    /// industry and employer; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        column_hash(
            self.industry.len(),
            self.industry
                .iter()
                .zip(&self.employer)
                .flat_map(|(&ind, &emp)| {
                    let [a, b] = emp.to_le_bytes();
                    [ind, a, b]
                }),
        )
    }
}
