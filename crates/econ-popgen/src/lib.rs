//! Population generator (spec `society/population-generator`).
//!
//! Builds the starting synthetic population (households and their members,
//! with integer weights) from census margin tables, at any `sample_scale`.
//! A pure function of (margins, parameters): no I/O, integer arithmetic only.
//!
//! The public API below is the one the acceptance tests were written against
//! (ADR-0014, tests-first). Stage A (the rule-based seed) is in `seed`, Stage B
//! (the fit) in `fit`. `python/reference/popgen_reference.py` is the
//! independent reference: the two must agree record for record (AC-POP-07).

mod arith;
mod attributes;
mod fit;
mod seed;

pub use attributes::{
    Activity, AttrError, AttrParams, AttributeMargins, EDU_LEVELS, EduLevel, MARGIN_ACTIVITIES,
    PersonAttributes, STATUSES, assign_attributes,
};

use arith::Int;
use econ_rng::KeyedRng;

/// Marks the open end of the last age band or size class in [`Margins`].
pub const OPEN: u16 = u16::MAX;

/// Sex of a person. The discriminant is the index used in the margin tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Sex {
    /// Female.
    F = 0,
    /// Male.
    M = 1,
}

/// A person's role in their household.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Role {
    /// The household's reference person (exactly one per household).
    Head = 0,
    /// Partner of the head.
    Partner = 1,
    /// Child of the head.
    Child = 2,
    /// Other relative or member.
    Other = 3,
}

/// Census margin tables: integer counts of real persons and households.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Margins {
    /// County codes (NUTS 3), in table order.
    pub counties: Vec<String>,
    /// Age bands `[from, to)` in years; the last band is open (`to == OPEN`).
    pub age_bands: Vec<(u16, u16)>,
    /// Household size classes `[min, max]` members; the last class is open (`max == OPEN`).
    pub size_classes: Vec<(u16, u16)>,
    /// Persons in private households, `[county][sex][age_band]`, row-major.
    pub persons_private: Vec<u64>,
    /// Persons not in private households, same shape.
    pub persons_collective: Vec<u64>,
    /// Private households, `[county][size_class]`, row-major.
    pub households: Vec<u64>,
}

/// Generator parameters. `sample_scale` comes from the scenario (ADR-0003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenParams {
    /// Real people per synthetic person.
    pub sample_scale: u32,
    /// Master seed of the keyed draws.
    pub rng_seed: u64,
    /// Cap of the open size class in the rule-based seed.
    pub max_household_size: u16,
    /// Relative error at which raking stops, in parts per million.
    pub raking_tolerance_ppm: u32,
    /// Upper bound on raking sweeps.
    pub max_sweeps: u32,
    /// Cells with fewer expected synthetic records are not checked for convergence.
    pub min_cell_records: u32,
}

impl GenParams {
    /// The spec's default tuning parameters for a given scale and seed.
    #[must_use]
    pub fn new(sample_scale: u32, rng_seed: u64) -> Self {
        GenParams {
            sample_scale,
            rng_seed,
            max_household_size: 15,
            raking_tolerance_ppm: 1000,
            max_sweeps: 50,
            min_cell_records: 30,
        }
    }
}

/// The households table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Households {
    /// Real households (or, for collective records, persons) represented; ≥ 1.
    pub hh_weight: Vec<u32>,
    /// Index into [`Margins::counties`].
    pub hh_county: Vec<u8>,
    /// The record stands for people not living in a private household.
    pub hh_collective: Vec<bool>,
}

/// The persons table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Persons {
    /// Index into the households table.
    pub household_id: Vec<u32>,
    /// Age in months.
    pub age: Vec<u16>,
    /// Sex.
    pub sex: Vec<Sex>,
    /// Role in the household.
    pub role: Vec<Role>,
}

/// How the fit went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FitReport {
    /// Largest number of raking sweeps used by a county.
    pub sweeps: u32,
    /// Every county reached the raking tolerance.
    pub converged: bool,
    /// Largest relative error left on a checked constraint, parts per million.
    pub max_error_ppm: u32,
    /// County cells with people but no synthetic record.
    pub unfitted_cells: u32,
}

/// A generated population.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Population {
    /// Households.
    pub households: Households,
    /// Persons.
    pub persons: Persons,
    /// Fit report.
    pub report: FitReport,
}

/// Why margins or parameters were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenError {
    /// A county's private persons cannot fit its households.
    InconsistentMargins {
        /// The county code.
        county: String,
    },
    /// `sample_scale` is zero.
    ZeroScale,
    /// No counties, age bands or size classes.
    EmptyTable,
    /// A table's length does not match its dimensions.
    ShapeMismatch,
}

/// Generate the starting population.
///
/// # Errors
/// See [`GenError`]. Besides tables of the wrong length, `ShapeMismatch` also
/// covers dimensions the output columns cannot hold: more than 256 counties,
/// or an age band that is empty or does not fit `age` in months.
///
/// # Panics
/// If a weight or a table index does not fit 32 bits, which no census reaches.
pub fn generate(margins: &Margins, params: &GenParams) -> Result<Population, GenError> {
    if params.sample_scale == 0 {
        return Err(GenError::ZeroScale);
    }
    let (n_c, n_b, n_k) = (
        margins.counties.len(),
        margins.age_bands.len(),
        margins.size_classes.len(),
    );
    if n_c == 0 || n_b == 0 || n_k == 0 {
        return Err(GenError::EmptyTable);
    }
    let n_cells = 2 * n_b;
    if margins.persons_private.len() != n_c * n_cells
        || margins.persons_collective.len() != n_c * n_cells
        || margins.households.len() != n_c * n_k
        || n_c > usize::from(u8::MAX) + 1
    {
        return Err(GenError::ShapeMismatch);
    }
    // The open band is five years wide; the open class is capped.
    let mut bands = Vec::with_capacity(n_b);
    for &(from, to) in &margins.age_bands {
        let to = if to == OPEN {
            from.checked_add(5)
        } else {
            Some(to)
        };
        match to {
            Some(to) if to > from && to <= u16::MAX / 12 => bands.push((from, to)),
            _ => return Err(GenError::ShapeMismatch),
        }
    }
    let classes: Vec<(Int, Int)> = margins
        .size_classes
        .iter()
        .map(|&(min, max)| {
            let max = if max == OPEN {
                params.max_household_size
            } else {
                max
            };
            (Int::from(min), Int::from(max))
        })
        .collect();
    let ints = |table: &[u64]| -> Vec<Int> { table.iter().map(|&v| Int::from(v)).collect() };
    let (private, collective, households) = (
        ints(&margins.persons_private),
        ints(&margins.persons_collective),
        ints(&margins.households),
    );
    let private: Vec<&[Int]> = private.chunks_exact(n_cells).collect();
    let collective: Vec<&[Int]> = collective.chunks_exact(n_cells).collect();
    let households: Vec<&[Int]> = households.chunks_exact(n_k).collect();

    // Stage A: every county's seed. Person sequence numbers run over the country.
    let rng = KeyedRng::new(params.rng_seed);
    let scale = Int::from(params.sample_scale);
    let mut next_seq = 0u64;
    let mut seeds = Vec::with_capacity(n_c);
    for ci in 0..n_c {
        let county = seed::CountyMargins {
            name: &margins.counties[ci],
            households: households[ci],
            private: private[ci],
            collective: collective[ci],
            bands: &bands,
            classes: &classes,
        };
        seeds.push(seed::build(&county, scale, rng, &mut next_seq)?);
    }

    // Stage B: targets for the whole country, then weights county by county.
    let (targets, unfitted_cells) = fit::person_targets(&seeds, &private, n_b)?;
    let mut report = FitReport {
        sweeps: 0,
        converged: true,
        max_error_ppm: 0,
        unfitted_cells,
    };
    let weight = |w: Int| u32::try_from(w).expect("weight fits u32");
    let mut hh = Households::default();
    let mut persons = Persons::default();
    for (ci, seed) in seeds.iter().enumerate() {
        let county = u8::try_from(ci).expect("at most 256 counties");
        let weights = fit::private_weights(
            &margins.counties[ci],
            seed,
            &targets[ci],
            households[ci].iter().sum(),
            private[ci].iter().sum(),
            params,
            &mut report,
        )?;
        let cells: Vec<usize> = seed.collective.iter().map(|p| p.cell).collect();
        let collective_weights = fit::collective_weights(&cells, collective[ci]);
        let records = seed
            .members
            .iter()
            .map(Vec::as_slice)
            .zip(weights)
            .map(|(members, w)| (members, w, false))
            .chain(
                seed.collective
                    .iter()
                    .map(std::slice::from_ref)
                    .zip(collective_weights)
                    .map(|(members, w)| (members, w, true)),
            );
        for (members, w, is_collective) in records {
            let id = u32::try_from(hh.hh_weight.len()).expect("household id fits u32");
            hh.hh_weight.push(weight(w));
            hh.hh_county.push(county);
            hh.hh_collective.push(is_collective);
            for person in members {
                persons.household_id.push(id);
                persons.age.push(person.age);
                persons.sex.push(person.sex);
                persons.role.push(person.role);
            }
        }
    }
    Ok(Population {
        households: hh,
        persons,
        report,
    })
}

impl Population {
    /// State hash (FNV-1a 64 over the tables in the order fixed by the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        const OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
        const PRIME: u64 = 0x0000_0100_0000_01B3;
        let mut hash = OFFSET;
        let mut feed = |bytes: &[u8]| {
            for &b in bytes {
                hash = (hash ^ u64::from(b)).wrapping_mul(PRIME);
            }
        };
        let len = |n: usize| {
            u32::try_from(n)
                .expect("table length fits u32")
                .to_le_bytes()
        };
        let (hh, p) = (&self.households, &self.persons);
        feed(&len(hh.hh_weight.len()));
        for i in 0..hh.hh_weight.len() {
            feed(&hh.hh_weight[i].to_le_bytes());
            feed(&[hh.hh_county[i], u8::from(hh.hh_collective[i])]);
        }
        feed(&len(p.household_id.len()));
        for i in 0..p.household_id.len() {
            feed(&p.household_id[i].to_le_bytes());
            feed(&p.age[i].to_le_bytes());
            feed(&[p.sex[i] as u8, p.role[i] as u8]);
        }
        hash
    }
}
