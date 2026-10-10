//! Stage E — locality size and tenure (spec `society/population-housing`).
//!
//! Gives every household the size class of its locality (and from it the
//! urban flag) and, for private households, a tenure, so that the weighted
//! population reproduces the census tables county by county. The population
//! is not changed.
//!
//! The public API is the one the acceptance tests were written against
//! (ADR-0014, tests-first). `python/reference/popgen_reference.py`
//! (`assign_housing`) is the independent reference: the two must agree
//! household for household (AC-POPH-07).

use econ_rng::{KeyedRng, Stream};

use crate::arith::Int;
use crate::assign::{Balancing, column_hash, column_sums, deal, deal_table, ints};
use crate::{Population, Role};

/// `tick` of the draw context: the stage number (3 is taken by the scale world).
const STAGE: u32 = 4;
const MONTHS: u16 = 12;

/// Tenures: owner, tenant, other.
pub const TENURES: usize = 3;
/// Household size groups of the tenure table: one person, larger.
pub const SIZE_GROUPS: usize = 2;
/// `hh_tenure` of a record that is not a private household.
pub const NO_TENURE: u8 = 255;

/// Census margin tables for locality size and tenure, by county.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HousingMargins {
    /// County codes; must equal [`crate::Margins::counties`].
    pub counties: Vec<String>,
    /// Number of locality size classes.
    pub locality_classes: u8,
    /// A locality of this class or above is urban.
    pub urban_from_class: u8,
    /// First age (years) of each age group, ascending from 0.
    pub age_group_from: Vec<u16>,
    /// Residents, `[county][age_group][locality_class]`, row-major.
    pub persons_by_locality: Vec<u64>,
    /// Private households, `[county][size_group][tenure]`, row-major.
    pub households_by_tenure: Vec<u64>,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HousingParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
    /// Persons added to every cell of the county's pattern table.
    pub pattern_floor: u64,
    /// Upper bound on alternating passes of the two-way balancing.
    pub balancing_passes: u32,
    /// Passes stop once every column is within this many millionths of the
    /// county's weight of its target.
    pub balance_stop_ppm: u32,
}

impl HousingParams {
    /// The spec's default parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        HousingParams {
            rng_seed,
            pattern_floor: 1,
            balancing_passes: 200,
            balance_stop_ppm: 100,
        }
    }
}

/// Housing columns, aligned with the households table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HouseholdHousing {
    /// Locality size class, `0..locality_classes`.
    pub hh_locality_size: Vec<u8>,
    /// The locality is urban.
    pub hh_urban: Vec<bool>,
    /// 0 owner, 1 tenant, 2 other, or [`NO_TENURE`].
    pub hh_tenure: Vec<u8>,
}

/// Why margins were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HousingError {
    /// No counties, locality classes or age groups.
    EmptyTable,
    /// A table's length does not match its dimensions, or the counties are
    /// not those of the population.
    ShapeMismatch,
    /// A county has households but the census table has nobody there.
    NoMargin {
        /// The county code.
        county: String,
    },
}

/// Assign locality size, urban flag and tenure to every household.
///
/// # Errors
/// See [`HousingError`]. `ShapeMismatch` also covers a household head younger
/// than the first age group.
pub fn assign_housing(
    pop: &Population,
    margins: &HousingMargins,
    params: &HousingParams,
) -> Result<HouseholdHousing, HousingError> {
    let (n_c, n_k, n_g) = (
        margins.counties.len(),
        usize::from(margins.locality_classes),
        margins.age_group_from.len(),
    );
    if n_c == 0 || n_k == 0 || n_g == 0 {
        return Err(HousingError::EmptyTable);
    }
    let (hh, p) = (&pop.households, &pop.persons);
    if margins.persons_by_locality.len() != n_c * n_g * n_k
        || margins.households_by_tenure.len() != n_c * SIZE_GROUPS * TENURES
        || hh.hh_county.iter().any(|&c| usize::from(c) >= n_c)
    {
        return Err(HousingError::ShapeMismatch);
    }
    let no_margin = |county: usize| HousingError::NoMargin {
        county: margins.counties[county].clone(),
    };
    let by_locality = ints(&margins.persons_by_locality);
    let by_tenure = ints(&margins.households_by_tenure);
    let balancing = Balancing {
        pattern_floor: params.pattern_floor,
        passes: params.balancing_passes,
        stop_ppm: params.balance_stop_ppm,
    };

    // Per household: members, the age group of its head, priority.
    let n_hh = hh.hh_weight.len();
    let mut size: Vec<Int> = vec![0; n_hh];
    let mut head_age = vec![0u16; n_hh];
    for i in 0..p.household_id.len() {
        let h = p.household_id[i] as usize;
        size[h] += 1;
        if p.role[i] == Role::Head {
            head_age[h] = p.age[i] / MONTHS;
        }
    }
    let mut group = Vec::with_capacity(n_hh);
    for &age in &head_age {
        let g = margins.age_group_from.iter().rposition(|&from| from <= age);
        group.push(g.ok_or(HousingError::ShapeMismatch)?);
    }
    let rng = KeyedRng::new(params.rng_seed);
    let priority: Vec<u64> = (0..n_hh)
        .map(|h| rng.draw(Stream::PopulationGen, STAGE, h as u64).u64())
        .collect();
    let mut by_county: Vec<Vec<usize>> = vec![Vec::new(); n_c];
    for h in 0..n_hh {
        by_county[usize::from(hh.hh_county[h])].push(h);
    }
    let weight = |h: usize| Int::from(hh.hh_weight[h]);

    let mut locality = vec![0u8; n_hh];
    let mut tenure = vec![NO_TENURE; n_hh];
    for (ci, households) in by_county.iter_mut().enumerate() {
        if households.is_empty() {
            continue;
        }
        households.sort_unstable_by_key(|&h| (priority[h], h));

        // Step 1: locality size. A household counts as the persons it stands
        // for; its row is the age group of its head, so that old households
        // lean to the classes where the old live.
        let census = &by_locality[ci * n_g * n_k..][..n_g * n_k];
        let classes = column_sums(census, n_k);
        if classes.iter().all(|&v| v == 0) {
            return Err(no_margin(ci));
        }
        let mut rows = vec![Vec::new(); n_g];
        for &h in households.iter() {
            rows[group[h]].push(h);
        }
        let mut carry = vec![0; n_k];
        deal_table(
            &rows,
            |h| weight(h) * size[h],
            census.chunks_exact(n_k).map(<[Int]>::to_vec),
            &classes,
            balancing,
            &mut carry,
            |h, k| locality[h] = u8::try_from(k).expect("a class index fits u8"),
        );

        // Step 2: tenure of private households, one-person households first.
        let census = &by_tenure[ci * SIZE_GROUPS * TENURES..][..SIZE_GROUPS * TENURES];
        let mut carry = [0; TENURES];
        for size_group in 0..SIZE_GROUPS {
            let members: Vec<usize> = households
                .iter()
                .copied()
                .filter(|&h| !hh.hh_collective[h] && (size[h] == 1) == (size_group == 0))
                .collect();
            if members.is_empty() {
                continue;
            }
            let mut targets = census[size_group * TENURES..][..TENURES].to_vec();
            if targets.iter().all(|&v| v == 0) {
                // No household of that size group in the census: both groups.
                targets = column_sums(census, TENURES);
            }
            if targets.iter().all(|&v| v == 0) {
                return Err(no_margin(ci));
            }
            let weights: Vec<Int> = members.iter().map(|&h| weight(h)).collect();
            for (&h, k) in members.iter().zip(deal(&weights, &targets, &mut carry)) {
                tenure[h] = u8::try_from(k).expect("a tenure index fits u8");
            }
        }
    }
    Ok(HouseholdHousing {
        hh_urban: locality
            .iter()
            .map(|&k| k >= margins.urban_from_class)
            .collect(),
        hh_locality_size: locality,
        hh_tenure: tenure,
    })
}

impl HouseholdHousing {
    /// Housing hash (FNV-1a 64 over the household count, then each
    /// household's locality size class, urban flag and tenure; see the spec).
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let n = self.hh_locality_size.len();
        column_hash(
            n,
            (0..n).flat_map(|h| {
                [
                    self.hh_locality_size[h],
                    u8::from(self.hh_urban[h]),
                    self.hh_tenure[h],
                ]
            }),
        )
    }
}
