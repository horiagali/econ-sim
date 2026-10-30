//! The People phase's rules (spec `economy/demographics`, roadmap M5): the
//! mortality table of a year, the alignment of a transition over weighted
//! records, the points of a new pension and fertility's responses to policy
//! and the economy. Pure functions, no I/O. Probabilities are parts per
//! billion; multipliers and shares millionths; pension points millionths of
//! a point; alignment targets and carries millionths of a person.
//!
//! API skeleton from the spec's API sketch, written with the acceptance
//! tests (AC-DEM-01, 02, 10, 17) before the code (ADR-0014): the bodies come
//! with the implementation change.

use econ_rng::{KeyedRng, Stream};
use econ_types::{Bani, Day};

use crate::{MORTALITY_AGES, MortalityTable, PPB, Ppb};

/// `n / d` to the nearest, half away from zero (`d` > 0).
fn rdiv(n: i128, d: i128) -> i128 {
    if n >= 0 {
        (2 * n + d) / (2 * d)
    } else {
        -((-2 * n + d) / (2 * d))
    }
}

/// The pipeline's rule (AC-DEM-01): the monthly probability of a yearly one,
/// `m = 1 − (1 − q)^(1/12)`, in parts per billion.
#[must_use]
pub fn monthly_from_yearly(q_ppb: Ppb) -> Ppb {
    if q_ppb >= PPB {
        return PPB;
    }
    let q = f64::from(q_ppb) / 1e9;
    let m = 1.0 - econ_num::math::powf(1.0 - q, 1.0 / 12.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let v = (m * 1e9 + 0.5) as u32;
    v.min(PPB)
}

/// The table of calendar year `year` (AC-DEM-01): `round(m_base · (1 −
/// improvement)^(year − base year))`, half away from zero, by repeated
/// integer multiplication; one at 110 years. The result's `base_year` is
/// `year` and its `yearly` column is the base's. `improvement_ppm` is
/// `mortality_improvement` a year.
#[must_use]
pub fn table_for_year(base: &MortalityTable, year: u16, improvement_ppm: u32) -> MortalityTable {
    let mut monthly = base.monthly;
    let keep = u128::from(1_000_000u32.saturating_sub(improvement_ppm));
    for _ in base.base_year..year {
        for row in &mut monthly {
            for v in row.iter_mut() {
                // round(v · keep / 10^6), half away from zero (v ≥ 0).
                let n = u128::from(*v) * keep;
                *v = u32::try_from((2 * n + 1_000_000) / 2_000_000).expect("below one");
            }
        }
    }
    for row in &mut monthly {
        row[MORTALITY_AGES - 1] = PPB;
    }
    MortalityTable {
        base_year: year,
        yearly: base.yearly,
        monthly,
    }
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
    // T = Σ w·p + carry, millionths of a person (p in parts per billion).
    let target: i128 = candidates
        .iter()
        .map(|c| i128::from(c.weight) * i128::from(c.p) / 1_000)
        .sum::<i128>()
        + i128::from(carry_micro);
    let mut ranked: Vec<(u64, u64, u32)> = candidates
        .iter()
        .filter(|c| c.weight > 0)
        .map(|c| {
            let u = rng.fast_u64(Stream::Demography, day.0, c.person_key, purpose);
            let rank = if c.rank_p == 0 {
                u64::MAX
            } else {
                u / u64::from(c.rank_p)
            };
            (rank, c.person_key, c.weight)
        })
        .collect();
    ranked.sort_unstable();
    let mut taken: i128 = 0;
    let mut chosen = Vec::new();
    for (rank, key, w) in ranked {
        let w = i128::from(w) * 1_000_000;
        if rank == u64::MAX || taken + w / 2 > target {
            break;
        }
        taken += w;
        chosen.push(key);
    }
    Selection {
        chosen,
        carry_micro: i64::try_from(target - taken).expect("a carry"),
    }
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
    if u32::from(contribution_months) < u32::from(law.min_years) * 12 {
        return None;
    }
    let own = u128::from(career_points) / u128::from(law.full_years.max(1));
    let reduced = if u32::from(contribution_months) < (u32::from(law.full_years) + 8) * 12 {
        let rho = u128::from(law.early_reduction_ppm_per_month) * u128::from(months_early.min(60));
        let keep = 1_000_000u128.saturating_sub(rho);
        (own * keep + 500_000) / 1_000_000
    } else {
        own
    };
    let own = u64::try_from(reduced).expect("points");
    Some(own.max(survivor_points))
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
    let d = |a: Bani, b: Bani| i128::from(a.get()) - i128::from(b.get());
    let childhood = 2 * d(law_then.under2, law_2021.under2) + 16 * d(law_then.to18, law_2021.to18);
    let avg = rdiv(childhood, 18);
    Bani(i64::try_from(avg + d(law_then.tax_credit, law_2021.tax_credit)).expect("a gap"))
}

/// M^pol, the household's benefit multiplier: `clamp(1 + β_b ΔB / Y, 0.8,
/// 1.3)`, with `Y` the income per equivalised adult, not below the floor
/// share of the national `median`. Millionths, 800,000 to 1,300,000.
#[must_use]
pub fn m_pol(delta_b: Bani, income_per_adult: Bani, median: Bani, r: &FertilityResponse) -> u32 {
    let floor = i128::from(median.get()) * i128::from(r.income_floor_ppm) / 1_000_000;
    let y = i128::from(income_per_adult.get()).max(floor);
    if y <= 0 {
        return 1_000_000;
    }
    let m = 1_000_000 + rdiv(i128::from(r.benefit_ppm) * i128::from(delta_b.get()), y);
    u32::try_from(m.clamp(800_000, 1_300_000)).expect("clamped")
}

/// M^job, the woman's multiplier: the unemployed ratio if she or her partner
/// is unemployed, else one. Millionths.
#[must_use]
pub fn m_job(woman_or_partner_unemployed: bool, r: &FertilityResponse) -> u32 {
    if woman_or_partner_unemployed {
        r.unemployed_ratio_ppm
    } else {
        1_000_000
    }
}

/// M^cyc, the national multiplier: `clamp(1 + β_y (g − g^ref), 0.97, 1.03)`,
/// one while `g` (real income growth over 12 months, millionths, measured
/// `fertility_lag` months earlier) is not known. Millionths, 970,000 to
/// 1,030,000.
#[must_use]
pub fn m_cyc(real_growth_ppm: Option<i32>, r: &FertilityResponse) -> u32 {
    let Some(g) = real_growth_ppm else {
        return 1_000_000;
    };
    let gap = i128::from(g) - i128::from(r.normal_growth_ppm);
    let m = 1_000_000 + rdiv(i128::from(r.cycle_ppm) * gap, 1_000_000);
    u32::try_from(m.clamp(970_000, 1_030_000)).expect("clamped")
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
    let base: u128 = women
        .iter()
        .map(|w| u128::from(w.weight) * u128::from(w.fertility_ppb))
        .sum();
    let with_job: u128 = women
        .iter()
        .map(|w| u128::from(w.weight) * u128::from(w.fertility_ppb) * u128::from(w.m_job_ppm))
        .sum();
    if base == 0 || with_job == 0 {
        return 1_000_000;
    }
    // base · 10^12 / with_job, to the nearest millionth.
    let k = (2 * base * 1_000_000_000_000 + with_job) / (2 * with_job);
    u32::try_from(k).expect("a normalisation")
}
