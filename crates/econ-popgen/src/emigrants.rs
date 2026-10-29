//! Stage L — linked emigrants and the diaspora stock (spec
//! `economy/migration-remittances`, "Starting state"; roadmap M5).
//!
//! Chooses the private households that have one member abroad, about 1.2
//! million real households, more often rural, in Moldavia, Maramureș and the
//! south and with older heads, fewer in Bucharest; gives each one linked
//! emigrant with an age, sex and education drawn like 2021's emigrants and
//! the months since they left; and spreads the rest of the diaspora (the
//! diaspora total less the linked emigrants) over a table of counts by origin
//! region, sex, year of birth and skill tier. Earlier stages are not changed:
//! the linked emigrants are a table of their own, not rows of the persons
//! table, so no stage before this one, and no mechanic, sees them.
//!
//! The rules (choices of this first version, where the spec leaves room):
//! - **Households.** A private household's chance is the product of its
//!   region's factor, the rural factor (if not urban) and its head's age
//!   factor. Households are ranked by `⌊u / chance⌋` (u the first word of the
//!   household's draw) and taken in that order while the weight taken stays
//!   within half a record of `linked_households` (the alignment of
//!   `economy/demographics`): the count is exact to half a record at every
//!   scale.
//! - **The emigrant.** From the household's draw, in order: (sex, age at
//!   departure) by 2021's emigrants (18 to 69), the departure year by its
//!   weights, the month of age at departure, the months of that year before
//!   the start, the education by `edu_shares_ppm`. `months_abroad` = 12 ×
//!   (start year − departure year) + the months; `age` = the age at
//!   departure + `months_abroad`. The role is child when the head is at least
//!   15 years older, else other.
//! - **The stock.** Its persons are the diaspora total less the weight of the
//!   linked emigrants, split by largest remainder over (sex, year of birth)
//!   by the departure ages times the departure years, then each over the
//!   regions by their residents times their factor, then over the tiers
//!   (high: ISCED 5–8, mid: 3–4, low: 0–2) by the education shares. Deaths
//!   abroad since departure are not taken out.
//!
//! No Python twin yet: the stage has no acceptance criteria of its own, and
//! is not one of the stages AC-POP-07 compares (owed with them).

use std::collections::BTreeMap;

use econ_rng::{KeyedRng, Stream};

use crate::arith::{Int, UNIT, apportion};
use crate::assign::{band_of, column_hash, household_regions};
use crate::attributes::{EDU_LEVELS, EduLevel};
use crate::housing::HouseholdHousing;
use crate::{Population, Role, Sex};

/// `tick` of the draw context: the stage number.
const STAGE: u32 = 12;
/// A head at least this many months older than the emigrant is a parent.
const PARENT_GAP_MONTHS: u16 = 15 * 12;

/// The margin file of the stage (`linked_emigrants2021_ro.json`). Counts are
/// real households and persons; weights are relative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedEmigrantMargins {
    /// The year of the start date: a departure that year is 0 to 11 months before it.
    pub start_year: u16,
    /// Real private households with a linked emigrant.
    pub linked_households: u64,
    /// People born in Romania living abroad: the linked emigrants and the stock.
    pub diaspora_total: u64,
    /// The age at departure of the first column of `departure_age_weights`, years.
    pub first_departure_age: u16,
    /// Emigrants by `[sex][age at departure − first]`, row-major (2021's).
    pub departure_age_weights: Vec<u64>,
    /// (departure year, relative weight).
    pub departure_years: Vec<(u16, u64)>,
    /// The emigrants' education, by `EduLevel`, millionths summing to one.
    pub edu_shares_ppm: [u32; EDU_LEVELS],
    /// Each region's factor (index of stage C's regions), millionths.
    pub region_factor_ppm: Vec<u32>,
    /// The factor of a rural household, millionths (urban: one).
    pub rural_factor_ppm: u32,
    /// By the head's age in years `[from, to)` (the last open), millionths.
    pub head_age_factor: Vec<(u16, u16, u32)>,
}

/// Parameters of the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedEmigrantParams {
    /// Master seed of the keyed draws (the scenario's seed).
    pub rng_seed: u64,
}

impl LinkedEmigrantParams {
    /// The parameters for a given seed.
    #[must_use]
    pub fn new(rng_seed: u64) -> Self {
        LinkedEmigrantParams { rng_seed }
    }
}

/// One cell of `diaspora_stock[k]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiasporaCell {
    /// Origin region (index of stage C's regions).
    pub region: u8,
    /// Sex.
    pub sex: Sex,
    /// Year of birth.
    pub birth_year: u16,
    /// Skill tier: high 0, mid 1, low 2 (as the labour market's).
    pub tier: u8,
    /// Real persons.
    pub persons: u64,
}

/// The linked emigrants (one row each, in household order) and the stock.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkedEmigrants {
    /// The household (index into the households table) the emigrant belongs to.
    pub household_id: Vec<u32>,
    /// Age in months.
    pub age: Vec<u16>,
    /// Sex.
    pub sex: Vec<Sex>,
    /// Role in the household.
    pub role: Vec<Role>,
    /// Highest completed education.
    pub edu_level: Vec<EduLevel>,
    /// `months_abroad`.
    pub months_abroad: Vec<u16>,
    /// `diaspora_stock[k]`, cells with persons only, by (region, sex, year of birth, tier).
    pub stock: Vec<DiasporaCell>,
}

/// Why the inputs were rejected. Nothing is produced in that case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmigrantError {
    /// A weight table sums to zero, or the education shares do not sum to one.
    EmptyTable,
    /// A table's length does not match its dimensions, a county has no
    /// region, or the housing columns are those of another population.
    ShapeMismatch,
}

const EDU: [EduLevel; EDU_LEVELS] = [
    EduLevel::Isced0To2,
    EduLevel::Isced3,
    EduLevel::Isced4,
    EduLevel::Isced5To6,
    EduLevel::Isced7To8,
];

/// The skill tier an education qualifies for: high 0, mid 1, low 2.
fn tier_of(edu: usize) -> usize {
    match edu {
        3 | 4 => 0,
        1 | 2 => 1,
        _ => 2,
    }
}

/// The index `k` of the cumulative weights `cum` (ascending, last = total)
/// that holds the draw `x < total`.
fn pick(cum: &[u64], x: u64) -> usize {
    cum.partition_point(|&c| c <= x)
}

fn cumulative(weights: impl Iterator<Item = u64>) -> Vec<u64> {
    let mut sum = 0u64;
    weights
        .map(|w| {
            sum = sum.saturating_add(w);
            sum
        })
        .collect()
}

/// Run stage L on `pop`. `region_of_county` and the number of regions are
/// stage C's; `housing` is stage E's.
///
/// # Errors
/// See [`EmigrantError`].
// Weights and counts are i128 (`Int`) far from its ends.
#[allow(clippy::arithmetic_side_effects, clippy::too_many_lines)]
pub fn assign_linked_emigrants(
    pop: &Population,
    region_of_county: &[u8],
    n_regions: usize,
    housing: &HouseholdHousing,
    margins: &LinkedEmigrantMargins,
    params: &LinkedEmigrantParams,
) -> Result<LinkedEmigrants, EmigrantError> {
    let (hh, p) = (&pop.households, &pop.persons);
    let n_h = hh.hh_weight.len();
    let n_a = margins.departure_age_weights.len() / 2;
    if housing.hh_urban.len() != n_h
        || margins.region_factor_ppm.len() != n_regions
        || margins.departure_age_weights.len() != 2 * n_a
        || n_a == 0
    {
        return Err(EmigrantError::ShapeMismatch);
    }
    if margins
        .edu_shares_ppm
        .iter()
        .map(|&s| u64::from(s))
        .sum::<u64>()
        != 1_000_000
        || margins.departure_age_weights.iter().sum::<u64>() == 0
        || margins.departure_years.iter().map(|&(_, w)| w).sum::<u64>() == 0
        || margins
            .departure_years
            .iter()
            .any(|&(y, _)| y > margins.start_year)
    {
        return Err(EmigrantError::EmptyTable);
    }
    let region = household_regions(&hh.hh_county, region_of_county, n_regions)
        .map_err(|_| EmigrantError::ShapeMismatch)?;
    let mut head_age = vec![None; n_h];
    for i in 0..p.household_id.len() {
        if p.role[i] == Role::Head {
            head_age[p.household_id[i] as usize] = Some(p.age[i]);
        }
    }
    let head_bands: Vec<(u16, u16)> = margins
        .head_age_factor
        .iter()
        .map(|&(from, to, _)| (from, to))
        .collect();

    // The households: ranked by u / chance, taken while within half a record of the target.
    let rng = KeyedRng::new(params.rng_seed);
    let mut ranked: Vec<(u64, usize)> = Vec::new();
    for h in 0..n_h {
        let Some(age) = head_age[h] else { continue };
        if hh.hh_collective[h] {
            continue;
        }
        let head = band_of(&head_bands, age / 12).map_or(0, |b| margins.head_age_factor[b].2);
        let rural = if housing.hh_urban[h] {
            1_000_000
        } else {
            u64::from(margins.rural_factor_ppm)
        };
        let chance = u64::from(margins.region_factor_ppm[region[h]]) * rural / 1_000_000
            * u64::from(head)
            / 1_000_000;
        if chance == 0 {
            continue;
        }
        let u = rng.draw(Stream::PopulationGen, STAGE, h as u64).u64();
        ranked.push((u / chance, h));
    }
    ranked.sort_unstable();
    let target = Int::from(margins.linked_households);
    let mut taken: Int = 0;
    let mut chosen = Vec::new();
    for &(_, h) in &ranked {
        let w = Int::from(hh.hh_weight[h]);
        if 2 * taken + w <= 2 * target {
            taken += w;
            chosen.push(h);
        }
    }
    chosen.sort_unstable();

    // One emigrant per chosen household.
    let age_cum = cumulative(margins.departure_age_weights.iter().copied());
    let year_cum = cumulative(margins.departure_years.iter().map(|&(_, w)| w));
    let edu_cum = cumulative(margins.edu_shares_ppm.iter().map(|&s| u64::from(s)));
    let mut out = LinkedEmigrants::default();
    for &h in &chosen {
        let mut d = rng.draw(Stream::PopulationGen, STAGE, h as u64);
        let _rank = d.u64();
        let cell = pick(&age_cum, d.below(*age_cum.last().expect("non-empty")));
        let year = margins.departure_years
            [pick(&year_cum, d.below(*year_cum.last().expect("non-empty")))]
        .0;
        let month_of_age = d.below(12);
        let months_before = d.below(12);
        let edu = pick(&edu_cum, d.below(1_000_000));
        let (sex, at) = (cell / n_a, cell % n_a);
        let departure_age =
            (u64::from(margins.first_departure_age) + at as u64) * 12 + month_of_age;
        let months_abroad = u64::from(margins.start_year - year) * 12 + months_before;
        let age = u16::try_from(departure_age + months_abroad).expect("an age in months");
        let head = head_age[h].expect("a chosen household has a head");
        out.household_id
            .push(u32::try_from(h).expect("household id fits u32"));
        out.age.push(age);
        out.sex.push(if sex == 0 { Sex::F } else { Sex::M });
        out.role
            .push(if head >= age.saturating_add(PARENT_GAP_MONTHS) {
                Role::Child
            } else {
                Role::Other
            });
        out.edu_level.push(EDU[edu]);
        out.months_abroad
            .push(u16::try_from(months_abroad).expect("months abroad fit u16"));
    }

    // The stock: the rest of the diaspora by (sex, year of birth), region and tier.
    let stock_total = (Int::from(margins.diaspora_total) - taken).max(0);
    let mut by_birth: BTreeMap<(usize, u16), Int> = BTreeMap::new();
    for sex in 0..2 {
        for at in 0..n_a {
            let w_age = Int::from(margins.departure_age_weights[sex * n_a + at]);
            for &(year, w_year) in &margins.departure_years {
                let age = margins.first_departure_age + u16::try_from(at).expect("an age");
                let birth = year.saturating_sub(age);
                *by_birth.entry((sex, birth)).or_insert(0) += w_age * Int::from(w_year);
            }
        }
    }
    let mut residents = vec![0 as Int; n_regions];
    for i in 0..p.household_id.len() {
        let h = p.household_id[i] as usize;
        residents[region[h]] += Int::from(hh.hh_weight[h]);
    }
    let region_w: Vec<Int> = (0..n_regions)
        .map(|r| residents[r] * Int::from(margins.region_factor_ppm[r]) / UNIT)
        .collect();
    let mut tier_w = [0 as Int; 3];
    for (e, &s) in margins.edu_shares_ppm.iter().enumerate() {
        tier_w[tier_of(e)] += Int::from(s);
    }
    let cells: Vec<(usize, u16)> = by_birth.keys().copied().collect();
    let parts = apportion(stock_total, &by_birth.values().copied().collect::<Vec<_>>());
    for (&(sex, birth_year), &n) in cells.iter().zip(&parts) {
        for (r, &nr) in apportion(n, &region_w).iter().enumerate() {
            for (t, &nt) in apportion(nr, &tier_w).iter().enumerate() {
                if nt > 0 {
                    out.stock.push(DiasporaCell {
                        region: u8::try_from(r).expect("at most 256 regions"),
                        sex: if sex == 0 { Sex::F } else { Sex::M },
                        birth_year,
                        tier: u8::try_from(t).expect("three tiers"),
                        persons: u64::try_from(nt).expect("a count"),
                    });
                }
            }
        }
    }
    out.stock.sort_unstable();
    Ok(out)
}

impl LinkedEmigrants {
    /// Stage L's hash: FNV-1a 64 over the emigrant count (`u32`,
    /// little-endian), then each emigrant's household id (`u32`), age
    /// (`u16`), sex, role, education (a byte each) and months abroad
    /// (`u16`), then each stock cell's region, sex, tier (a byte each), year
    /// of birth (`u16`) and persons (`u64`), little-endian.
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let emigrants = (0..self.household_id.len()).flat_map(|k| {
            let mut b = Vec::with_capacity(11);
            b.extend_from_slice(&self.household_id[k].to_le_bytes());
            b.extend_from_slice(&self.age[k].to_le_bytes());
            b.extend_from_slice(&[
                self.sex[k] as u8,
                self.role[k] as u8,
                self.edu_level[k] as u8,
            ]);
            b.extend_from_slice(&self.months_abroad[k].to_le_bytes());
            b
        });
        let stock = self.stock.iter().flat_map(|c| {
            let mut b = vec![c.region, c.sex as u8, c.tier];
            b.extend_from_slice(&c.birth_year.to_le_bytes());
            b.extend_from_slice(&c.persons.to_le_bytes());
            b
        });
        column_hash(self.household_id.len(), emigrants.chain(stock))
    }

    /// The real persons the linked emigrants stand for (each weighs its
    /// household's `hh_weight`).
    #[must_use]
    pub fn weighted(&self, hh_weight: &[u32]) -> u64 {
        self.household_id
            .iter()
            .map(|&h| u64::from(hh_weight[h as usize]))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Households, Persons};

    /// Ten one-person households of weight 10 in two regions, half rural.
    fn small() -> (Population, HouseholdHousing) {
        let mut pop = Population::default();
        for h in 0..10u32 {
            pop.households = Households {
                hh_weight: [pop.households.hh_weight.clone(), vec![10]].concat(),
                hh_county: [pop.households.hh_county.clone(), vec![(h % 2) as u8]].concat(),
                hh_collective: [pop.households.hh_collective.clone(), vec![false]].concat(),
            };
            pop.persons = Persons {
                household_id: [pop.persons.household_id.clone(), vec![h]].concat(),
                age: [pop.persons.age.clone(), vec![600]].concat(),
                sex: [pop.persons.sex.clone(), vec![Sex::F]].concat(),
                role: [pop.persons.role.clone(), vec![Role::Head]].concat(),
            };
        }
        let housing = HouseholdHousing {
            hh_locality_size: vec![0; 10],
            hh_urban: (0..10).map(|h| h < 5).collect(),
            hh_tenure: vec![0; 10],
        };
        (pop, housing)
    }

    fn margins() -> LinkedEmigrantMargins {
        LinkedEmigrantMargins {
            start_year: 2021,
            linked_households: 40,
            diaspora_total: 1_000,
            first_departure_age: 18,
            departure_age_weights: vec![1, 2, 3, 4],
            departure_years: vec![(2010, 1), (2021, 1)],
            edu_shares_ppm: [300_000, 430_000, 70_000, 160_000, 40_000],
            region_factor_ppm: vec![1_000_000, 2_000_000],
            rural_factor_ppm: 1_500_000,
            head_age_factor: vec![(0, 30, 300_000), (30, crate::OPEN, 1_000_000)],
        }
    }

    #[test]
    fn count_is_within_half_a_record_and_the_stock_takes_the_rest() {
        let (pop, housing) = small();
        let got = assign_linked_emigrants(
            &pop,
            &[0, 1],
            2,
            &housing,
            &margins(),
            &LinkedEmigrantParams::new(42),
        )
        .expect("the stage runs");
        // 40 real households: four records of weight 10.
        assert_eq!(got.household_id.len(), 4);
        assert_eq!(got.weighted(&pop.households.hh_weight), 40);
        let stock: u64 = got.stock.iter().map(|c| c.persons).sum();
        assert_eq!(stock, 960);
        for k in 0..4 {
            // Departures in 2010 or 2021, at 18 or 19.
            let m = got.months_abroad[k];
            assert!(m < 12 || (132..144).contains(&m), "{m}");
            let at_departure = got.age[k] - m;
            assert!((18 * 12..20 * 12).contains(&at_departure));
            // A head of 50 and an emigrant under 33: a child.
            assert_eq!(got.role[k], Role::Child);
        }
        // Same seed, same result; another seed, another draw.
        let again = assign_linked_emigrants(
            &pop,
            &[0, 1],
            2,
            &housing,
            &margins(),
            &LinkedEmigrantParams::new(42),
        )
        .expect("the stage runs");
        assert_eq!(again.state_hash(), got.state_hash());
    }

    #[test]
    fn bad_inputs_are_refused() {
        let (pop, housing) = small();
        let mut m = margins();
        m.edu_shares_ppm[0] += 1;
        let p = LinkedEmigrantParams::new(42);
        assert_eq!(
            assign_linked_emigrants(&pop, &[0, 1], 2, &housing, &m, &p),
            Err(EmigrantError::EmptyTable)
        );
        assert_eq!(
            assign_linked_emigrants(&pop, &[0, 1], 3, &housing, &margins(), &p),
            Err(EmigrantError::ShapeMismatch)
        );
    }

    #[test]
    fn pick_finds_the_cell_of_a_draw() {
        let cum = cumulative([2u64, 0, 3].into_iter());
        assert_eq!(cum, [2, 2, 5]);
        assert_eq!(pick(&cum, 0), 0);
        assert_eq!(pick(&cum, 1), 0);
        assert_eq!(pick(&cum, 2), 2);
        assert_eq!(pick(&cum, 4), 2);
    }
}
