//! Demographics and the household life cycle (specs `economy/demographics`
//! and `society/households-lifecycle`, roadmap M5): the mortality table of a
//! year, the alignment of a transition over weighted records, the points of
//! a new pension, fertility's responses to policy and the economy, and the
//! operations on weighted records ([`records`]). Pure functions, no I/O;
//! the `World`'s People phase walks its tables and posts the ledger legs.
//!
//! Probabilities are parts per billion ([`Ppb`]); multipliers, shares and
//! rates are millionths (ppm); pension points are millionths of a point;
//! alignment targets and carries are millionths of a person.
//!
//! API skeleton from the specs' API sketches, written with the acceptance
//! tests before the code (ADR-0014): the bodies come with the implementation
//! change.

pub mod records;

use econ_rng::KeyedRng;
use econ_types::{Bani, Day};

/// Parts per billion.
pub type Ppb = u32;

/// A probability of one, in parts per billion.
pub const PPB_ONE: Ppb = 1_000_000_000;

/// The oldest age of the mortality table, in years: at this age or more the
/// monthly probability of dying is one.
pub const MAX_AGE: usize = 110;

/// Monthly probabilities of dying by sex (`econ_popgen::Sex as usize`: F 0,
/// M 1) and age in whole years, 0 to [`MAX_AGE`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MortalityTable {
    /// The calendar year the probabilities are for (2018 for the base table,
    /// the centre of 2017–2019).
    pub year: u16,
    /// `[sex][age in years]`, parts per billion a month.
    pub monthly: [[Ppb; MAX_AGE + 1]; 2],
}

/// The pipeline's rule (AC-DEM-01): the monthly probability of a yearly one,
/// `m = 1 − (1 − q)^(1/12)`, in parts per billion.
#[must_use]
pub fn monthly_from_yearly(q_ppb: Ppb) -> Ppb {
    let _ = q_ppb;
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The table of calendar year `year` (AC-DEM-01): `round(m_base · (1 −
/// improvement)^(year − base year))`, half away from zero, by repeated
/// integer multiplication; one at [`MAX_AGE`]. `improvement_ppm` is
/// `mortality_improvement` a year.
#[must_use]
pub fn table_for_year(base: &MortalityTable, year: u16, improvement_ppm: u32) -> MortalityTable {
    let _ = (base, year, improvement_ppm);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// One eligible person of a transition's stratum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    /// The person's stable key: the entity of the keyed draw and the
    /// tiebreak of the ranking.
    pub person_key: u64,
    /// The household's weight.
    pub weight: u32,
    /// The monthly probability that sets the target.
    pub p: Ppb,
    /// The probability that ranks (the partner ratio of births moves who,
    /// not how many); `p` for the other transitions.
    pub rank_p: Ppb,
}

/// The persons a stratum selects this month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// The selected persons' keys, in ranking order.
    pub chosen: Vec<u64>,
    /// The carry to next month: target − selected weight, millionths of a
    /// person.
    pub carry_micro: i64,
}

/// Alignment of one stratum for one month (spec `economy/demographics`,
/// "Alignment"; AC-DEM-02): target `T = Σ w·p + carry` in millionths of a
/// person; persons ranked by `⌊u / rank_p⌋` ascending (ties by key), `u`
/// the first `u64` of `draw(Demography, day, person_key)` for `purpose`;
/// a person is taken while `S + w/2 ≤ T`.
#[must_use]
pub fn align(
    candidates: &[Candidate],
    carry_micro: i64,
    rng: &KeyedRng,
    day: Day,
    purpose: u32,
) -> Selection {
    let _ = (candidates, carry_micro, rng, day, purpose);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The pension law a new pension reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PensionLaw {
    /// `full_contribution_years` (35).
    pub full_years: u8,
    /// `min_contribution_years` (15).
    pub min_years: u8,
    /// `early_retirement_reduction`, millionths per month early (7,500).
    pub early_reduction_ppm_per_month: u32,
    /// `survivor_share`, millionths (500,000).
    pub survivor_share_ppm: u32,
    /// `pension_point_value`, a month.
    pub point_value: Bani,
    /// `minimum_pension`, a month.
    pub minimum: Bani,
}

/// The pension points of a new retiree (AC-DEM-10): `max(career_points ÷
/// full_years × (1 − ρ), survivor_points)`, with `ρ` the reduction per month
/// early × `months_early` (at most 60), only for fewer than `full_years` +
/// 8 years of contribution; `None` below `min_years` of contribution.
/// Points in millionths.
#[must_use]
pub fn new_pension_points(
    career_points: u64,
    survivor_points: u64,
    months_early: u16,
    contribution_months: u16,
    law: &PensionLaw,
) -> Option<u64> {
    let _ = (
        career_points,
        survivor_points,
        months_early,
        contribution_months,
        law,
    );
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Fertility's responses to policy and the economy (spec
/// `economy/demographics`, step 4), millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FertilityResponse {
    /// β_b, `fertility_benefit_response`.
    pub benefit_ppm: u32,
    /// `unemployed_fertility_ratio`.
    pub unemployed_ratio_ppm: u32,
    /// β_y, `fertility_cycle_response`.
    pub cycle_ppm: u32,
    /// g^ref, `normal_income_growth`, a year.
    pub normal_growth_ppm: i32,
    /// `fertility_lag`, months.
    pub lag_months: u8,
    /// `fertility_income_floor`, millionths of the national median.
    pub income_floor_ppm: u32,
}

impl FertilityResponse {
    /// The spec's defaults: β_b 0.1, unemployed ratio 0.85, β_y 0.2, normal
    /// growth 2% a year, lag 9 months, income floor 25% of the median.
    #[must_use]
    pub fn spec_defaults() -> FertilityResponse {
        FertilityResponse {
            benefit_ppm: 100_000,
            unemployed_ratio_ppm: 850_000,
            cycle_ppm: 200_000,
            normal_growth_ppm: 20_000,
            lag_months: 9,
            income_floor_ppm: 250_000,
        }
    }
}

/// The child support of a law, a month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildSupport {
    /// `child_allowance_under2`.
    pub under2: Bani,
    /// `child_allowance` (2 to 18).
    pub to18: Bani,
    /// `child_tax_credit`.
    pub tax_credit: Bani,
}

/// ΔB, the extra support a newborn brings a month averaged over its
/// childhood, against the 2021 law: `(2 Δunder2 + 16 Δto18) / 18 + Δcredit`.
pub fn benefit_gap(law_then: &ChildSupport, law_2021: &ChildSupport) -> Bani {
    let _ = (law_then, law_2021);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// M^pol, the household's benefit multiplier: `clamp(1 + β_b ΔB / Y, 0.8,
/// 1.3)`, with `Y` the income per equivalised adult, not below the floor
/// share of the national `median`. Millionths, 800,000 to 1,300,000.
#[must_use]
pub fn m_pol(delta_b: Bani, income_per_adult: Bani, median: Bani, r: &FertilityResponse) -> u32 {
    let _ = (delta_b, income_per_adult, median, r);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// M^job, the woman's multiplier: the unemployed ratio if she or her partner
/// is unemployed, else one. Millionths.
#[must_use]
pub fn m_job(woman_or_partner_unemployed: bool, r: &FertilityResponse) -> u32 {
    let _ = (woman_or_partner_unemployed, r);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// M^cyc, the national multiplier: `clamp(1 + β_y (g − g^ref), 0.97, 1.03)`,
/// one while `g` (real income growth over 12 months, millionths, measured
/// `fertility_lag` months earlier) is not known. Millionths, 970,000 to
/// 1,030,000.
#[must_use]
pub fn m_cyc(real_growth_ppm: Option<i32>, r: &FertilityResponse) -> u32 {
    let _ = (real_growth_ppm, r);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// An eligible woman of a birth stratum at load, for the normalisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FertileWoman {
    /// Her household's weight.
    pub weight: u32,
    /// `fertility_rate` at her age, births per woman a year, parts per
    /// billion.
    pub fertility_ppb: Ppb,
    /// Her M^job, millionths.
    pub m_job_ppm: u32,
}

/// `k_s = Σ w f / Σ w f M^job` over a stratum's women at load, millionths:
/// the starting population then gives exactly the base-rate births
/// (AC-DEM-17). One for an empty stratum.
#[must_use]
pub fn job_normalisation(women: &[FertileWoman]) -> u32 {
    let _ = women;
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}
