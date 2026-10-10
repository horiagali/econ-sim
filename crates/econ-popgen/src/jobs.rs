//! Stage D — jobs (spec `society/population-jobs`).
//!
//! Gives every employed person a status in employment, an occupation and an
//! industry group, so that the weighted population reproduces the census tables
//! by region, sex and age. Earlier stages are not changed.
//!
//! The public API is the one the acceptance tests were written against
//! (ADR-0014, tests-first). `python/reference/popgen_reference.py`
//! (`assign_jobs`) is the independent reference: the two must agree person
//! for person (AC-POPJ-07).

use econ_rng::{KeyedRng, Stream};

use crate::Population;
use crate::arith::Int;
use crate::assign::{
    Balancing, band_of, column_hash, column_sums, deal, deal_table, household_regions, ints,
};
use crate::attributes::{Activity, EDU_LEVELS, PersonAttributes};

/// `tick` of the draw context: the stage number.
const STAGE: u32 = 2;
const MONTHS: u16 = 12;
/// `employment_status` of a person who is not employed; the kinds are 1 to [`KINDS`].
const NO_STATUS: u8 = 0;

/// Statuses in employment: employee, employer, own-account worker, family worker.
pub const KINDS: usize = 4;
/// Occupations: ISCO-08 major groups, numbered by their digit (0 = armed forces).
pub const OCCUPATIONS: usize = 10;
/// Industry groups: the NACE Rev. 2 groups A, B–E, F, G–I, J, K, L, M–N, O–Q, R–U.
pub const INDUSTRY_GROUPS: usize = 10;
/// Education groups of the pattern table: ISCED 0–2, 3–4, 5–8.
pub const EDU_GROUPS: usize = 3;
/// `occupation` and `industry_group` of a person who is not employed.
pub const NOT_EMPLOYED: u8 = 255;

/// Census margins and survey patterns for jobs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobMargins {
    /// Region codes (NUTS 2), in table order.
    pub regions: Vec<String>,
    /// For each county of [`crate::Margins::counties`], its index into `regions`.
    pub region_of_county: Vec<u8>,
    /// Age bands `[from, to)` in years, as in [`crate::Margins`].
    pub age_bands: Vec<(u16, u16)>,
    /// Census persons, `[region][sex][age_band][kind][industry_group]`, row-major.
    pub by_industry_group: Vec<u64>,
    /// Census persons, `[region][sex][age_band][kind][occupation]`, row-major.
    pub by_occupation: Vec<u64>,
    /// Pattern (proportions only), `[sex][edu_group][occupation]`.
    pub pattern_edu_occupation: Vec<u64>,
    /// Pattern (proportions only), `[sex][occupation][industry_group]`.
    pub pattern_occupation_industry_group: Vec<u64>,
    /// The pattern's education group of each education level.
    pub edu_group_of_level: [u8; EDU_LEVELS],
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Persons added to every pattern cell, so no combination is impossible.
    pub pattern_floor: u64,
    /// Upper bound on alternating passes of the two-way balancing.
    pub balancing_passes: u32,
    /// Passes stop once every column is within this many millionths of the
    /// group's weight of its target.
    pub balance_stop_ppm: u32,
}

impl JobParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        JobParams {
            rng_seed,
            pattern_floor: 1,
            balancing_passes: 200,
            balance_stop_ppm: 100,
        }
    }
}

/// Job columns, aligned with the persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersonJobs {
    /// 0 not employed, 1 employee, 2 employer, 3 own-account worker, 4 family worker.
    pub employment_status: Vec<u8>,
    /// ISCO-08 major group, or [`NOT_EMPLOYED`].
    pub occupation: Vec<u8>,
    /// Industry group index, or [`NOT_EMPLOYED`].
    pub industry_group: Vec<u8>,
}

/// Why inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobError {
    /// No regions or age bands.
    EmptyTable,
    /// A table's length does not match its dimensions, or the attributes
    /// belong to another population.
    ShapeMismatch,
    /// A household's county has no region.
    UnknownRegion {
        /// The county index.
        county: u8,
    },
    /// The census has nobody employed in a region and sex that has employed
    /// synthetic persons.
    NoMargin {
        /// The region code.
        region: String,
    },
}

/// One census table, `[region][sex][age_band][kind][column]`, row-major.
struct Census {
    table: Vec<Int>,
    n_bands: usize,
    width: usize,
}

impl Census {
    /// All kinds of one region, sex and age band: `KINDS` rows.
    fn band(&self, region_sex: usize, band: usize) -> &[Int] {
        &self.table[(region_sex * self.n_bands + band) * KINDS * self.width..][..KINDS * self.width]
    }

    /// All age bands of one region and sex.
    fn all_bands(&self, region_sex: usize) -> &[Int] {
        let per_sex = self.n_bands * KINDS * self.width;
        &self.table[region_sex * per_sex..][..per_sex]
    }

    /// Persons of each kind in one region, sex and age band; if the census
    /// has nobody there, in all age bands.
    fn by_kind(&self, region_sex: usize, band: usize) -> Option<Vec<Int>> {
        let kinds = |rows: &[Int]| -> Vec<Int> {
            let mut out = vec![0; KINDS];
            for (i, row) in rows.chunks_exact(self.width).enumerate() {
                out[i % KINDS] += row.iter().sum::<Int>();
            }
            out
        };
        some_nonzero(kinds(self.band(region_sex, band)))
            .or_else(|| some_nonzero(kinds(self.all_bands(region_sex))))
    }

    /// Census counts by column of one region, sex, age band and kind. If the
    /// census has nobody there: the same kind over all age bands, then all
    /// kinds too.
    fn targets(&self, region_sex: usize, band: usize, kind: usize) -> Option<Vec<Int>> {
        let row = &self.band(region_sex, band)[kind * self.width..][..self.width];
        let all = self.all_bands(region_sex);
        some_nonzero(row.to_vec())
            .or_else(|| {
                let mut out = vec![0; self.width];
                for rows in all.chunks_exact(KINDS * self.width) {
                    for (sum, v) in out.iter_mut().zip(&rows[kind * self.width..]) {
                        *sum += v;
                    }
                }
                some_nonzero(out)
            })
            .or_else(|| some_nonzero(column_sums(all, self.width)))
    }
}

/// The counts, unless they are all zero.
fn some_nonzero(counts: Vec<Int>) -> Option<Vec<Int>> {
    counts.iter().any(|&v| v != 0).then_some(counts)
}

/// Assign status in employment, occupation and industry group to every employed person.
///
/// # Errors
/// See [`JobError`]. `ShapeMismatch` also covers an education group outside
/// the pattern table and an employed person whose age is in no age band of
/// the margins.
pub fn assign_jobs(
    pop: &Population,
    attrs: &PersonAttributes,
    margins: &JobMargins,
    params: &JobParams,
) -> Result<PersonJobs, JobError> {
    let (n_r, n_b) = (margins.regions.len(), margins.age_bands.len());
    if n_r == 0 || n_b == 0 {
        return Err(JobError::EmptyTable);
    }
    let cells = n_r * 2 * n_b * KINDS;
    if margins.by_industry_group.len() != cells * INDUSTRY_GROUPS
        || margins.by_occupation.len() != cells * OCCUPATIONS
        || margins.pattern_edu_occupation.len() != 2 * EDU_GROUPS * OCCUPATIONS
        || margins.pattern_occupation_industry_group.len() != 2 * OCCUPATIONS * INDUSTRY_GROUPS
        || margins
            .edu_group_of_level
            .iter()
            .any(|&g| usize::from(g) >= EDU_GROUPS)
    {
        return Err(JobError::ShapeMismatch);
    }
    let (hh, p) = (&pop.households, &pop.persons);
    let hh_region = household_regions(&hh.hh_county, &margins.region_of_county, n_r)
        .map_err(|county| JobError::UnknownRegion { county })?;
    let n = p.household_id.len();
    if attrs.activity.len() != n || attrs.edu_level.len() != n {
        return Err(JobError::ShapeMismatch);
    }
    let no_margin = |region_sex: usize| JobError::NoMargin {
        region: margins.regions[region_sex / 2].clone(),
    };
    let by_group = Census {
        table: ints(&margins.by_industry_group),
        n_bands: n_b,
        width: INDUSTRY_GROUPS,
    };
    let by_occupation = Census {
        table: ints(&margins.by_occupation),
        n_bands: n_b,
        width: OCCUPATIONS,
    };
    let edu_pattern = ints(&margins.pattern_edu_occupation);
    let occupation_pattern = ints(&margins.pattern_occupation_industry_group);
    let balancing = Balancing {
        pattern_floor: params.pattern_floor,
        passes: params.balancing_passes,
        stop_ppm: params.balance_stop_ppm,
    };

    // The employed of each (region, sex, age band), in (priority, index) order.
    let rng = KeyedRng::new(params.rng_seed);
    let mut weight: Vec<Int> = vec![0; n];
    let mut priority = vec![0u64; n];
    let mut cell: Vec<Vec<usize>> = vec![Vec::new(); n_r * 2 * n_b];
    for i in 0..n {
        if attrs.activity[i] != Activity::Employed {
            continue;
        }
        let h = p.household_id[i] as usize;
        let band = band_of(&margins.age_bands, p.age[i] / MONTHS).ok_or(JobError::ShapeMismatch)?;
        weight[i] = Int::from(hh.hh_weight[h]);
        priority[i] = rng.draw(Stream::PopulationGen, STAGE, i as u64).u64();
        cell[(hh_region[h] * 2 + p.sex[i] as usize) * n_b + band].push(i);
    }
    for members in &mut cell {
        members.sort_unstable_by_key(|&i| (priority[i], i));
    }
    let code = |k: usize| u8::try_from(k).expect("a category index is small");

    // Step 1: status in employment, by region, sex and age band.
    let mut status = vec![NO_STATUS; n];
    for rs in 0..n_r * 2 {
        let mut carry = [0; KINDS];
        for b in 0..n_b {
            let members = &cell[rs * n_b + b];
            if members.is_empty() {
                continue;
            }
            let targets = by_group.by_kind(rs, b).ok_or_else(|| no_margin(rs))?;
            let weights: Vec<Int> = members.iter().map(|&i| weight[i]).collect();
            for (&i, k) in members.iter().zip(deal(&weights, &targets, &mut carry)) {
                status[i] = code(k + 1);
            }
        }
    }

    // Steps 2 and 3, by region, sex and status in employment; the carry runs
    // through the age bands, and inside a band through the rows of the table.
    let mut occupation = vec![NOT_EMPLOYED; n];
    let mut industry_group = vec![NOT_EMPLOYED; n];
    for rs in 0..n_r * 2 {
        let sex = rs % 2;
        for kind in 0..KINDS {
            let mut carry_occupation = [0; OCCUPATIONS];
            let mut carry_group = [0; INDUSTRY_GROUPS];
            for b in 0..n_b {
                let members: Vec<usize> = cell[rs * n_b + b]
                    .iter()
                    .copied()
                    .filter(|&i| status[i] == code(kind + 1))
                    .collect();
                if members.is_empty() {
                    continue;
                }
                // Step 2: occupation, from education. Rows are education levels.
                let mut rows = vec![Vec::new(); EDU_LEVELS];
                for &i in &members {
                    rows[attrs.edu_level[i] as usize].push(i);
                }
                let targets = by_occupation
                    .targets(rs, b, kind)
                    .ok_or_else(|| no_margin(rs))?;
                let pattern = margins.edu_group_of_level.iter().map(|&g| {
                    edu_pattern[(sex * EDU_GROUPS + usize::from(g)) * OCCUPATIONS..][..OCCUPATIONS]
                        .to_vec()
                });
                deal_table(
                    &rows,
                    |i| weight[i],
                    pattern,
                    &targets,
                    balancing,
                    &mut carry_occupation,
                    |i, k| occupation[i] = code(k),
                );
                // Step 3: industry group, from occupation. Rows are occupations.
                let mut rows = vec![Vec::new(); OCCUPATIONS];
                for &i in &members {
                    rows[usize::from(occupation[i])].push(i);
                }
                let targets = by_group.targets(rs, b, kind).ok_or_else(|| no_margin(rs))?;
                let pattern = occupation_pattern[sex * OCCUPATIONS * INDUSTRY_GROUPS..]
                    [..OCCUPATIONS * INDUSTRY_GROUPS]
                    .chunks_exact(INDUSTRY_GROUPS)
                    .map(<[Int]>::to_vec);
                deal_table(
                    &rows,
                    |i| weight[i],
                    pattern,
                    &targets,
                    balancing,
                    &mut carry_group,
                    |i, k| industry_group[i] = code(k),
                );
            }
        }
    }
    Ok(PersonJobs {
        employment_status: status,
        occupation,
        industry_group,
    })
}

impl PersonJobs {
    /// Jobs hash (FNV-1a 64 over the person count, then each person's status
    /// in employment, occupation and industry group; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let n = self.employment_status.len();
        column_hash(
            n,
            (0..n).flat_map(|i| {
                [
                    self.employment_status[i],
                    self.occupation[i],
                    self.industry_group[i],
                ]
            }),
        )
    }
}
