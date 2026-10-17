//! Social transfers (spec `economy/social-transfers`, "First version"): the
//! state pension, the unemployment benefit, the child allowance and the
//! guaranteed minimum income, at their 2021 rules. Pure functions, no I/O; the
//! pay-day phase that walks persons and households, applies take-up and posts
//! through the ledger lives with the `World`.
//!
//! Amounts are whole bani; shares are millionths; pension points are
//! millionths of a point. Each amount is rounded once, half away from zero
//! (ADR-0007).

use econ_types::Bani;

/// Millionths in one.
const PPM: i128 = 1_000_000;

/// `num / den` rounded half away from zero (`den > 0`).
// Callers pass bani times millionths, far inside i128.
#[allow(clippy::arithmetic_side_effects)]
fn round_div(num: i128, den: i128) -> i128 {
    let (q, r) = (num / den, (num % den).abs());
    if 2 * r >= den { q + num.signum() } else { q }
}

/// Bani × millionths as bani, rounded once.
fn from_ppm(sum: i128) -> Bani {
    Bani(i64::try_from(round_div(sum, PPM)).expect("a benefit fits i64"))
}

/// The value of the last entry whose threshold is at most `years`.
fn by_years<T: Copy>(table: &[(u8, T)], years: u8) -> Option<T> {
    table
        .iter()
        .rev()
        .find(|&&(from, _)| from <= years)
        .map(|&(_, v)| v)
}

/// The state pension's rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PensionRule {
    /// `pension_point_value`, a month.
    pub point_value: Bani,
    /// `minimum_pension`, a month.
    pub minimum: Bani,
}

/// `pension_points` from a starting pension, set once at load: pension ÷
/// point value, in millionths of a point.
#[must_use]
// i64 × 10^6 fits i128.
#[allow(clippy::arithmetic_side_effects)]
pub fn pension_points(starting_pension: Bani, point_value: Bani) -> u64 {
    let points = round_div(
        i128::from(starting_pension.get().max(0)) * PPM,
        i128::from(point_value.get().max(1)),
    );
    u64::try_from(points).expect("points fit u64")
}

/// The month's pension: points × point value, not below the minimum (AC-ST-01).
// u64 × i64 fits i128.
#[allow(clippy::arithmetic_side_effects)]
pub fn pension(points: u64, r: &PensionRule) -> Bani {
    from_ppm(i128::from(points) * i128::from(r.point_value.get())).max(r.minimum)
}

/// The unemployment benefit's rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnemploymentRule {
    /// `reference_social_indicator`, a month.
    pub isr: Bani,
    /// `ui_isr_share`, in millionths.
    pub isr_share_ppm: u32,
    /// Share of `last_wage` by contribution years: (from years, share in
    /// millionths), ascending; none below the first entry.
    pub wage_share_by_years: Vec<(u8, u32)>,
    /// Months of benefit by contribution years: (from years, months),
    /// ascending; none below the first entry.
    pub months_by_years: Vec<(u8, u8)>,
}

/// `benefit_months_left` when a job is lost, from the contribution years.
#[must_use]
pub fn benefit_months(contribution_years: u8, r: &UnemploymentRule) -> u8 {
    by_years(&r.months_by_years, contribution_years).unwrap_or(0)
}

/// The benefit a month: `ui_isr_share` × ISR + the share of `last_wage` for the
/// contribution years; nothing under the first entry of `months_by_years`
/// (AC-ST-02).
// i64 × u32 fits i128.
#[allow(clippy::arithmetic_side_effects)]
pub fn unemployment_benefit(last_wage: Bani, contribution_years: u8, r: &UnemploymentRule) -> Bani {
    if benefit_months(contribution_years, r) == 0 {
        return Bani::ZERO;
    }
    let share = by_years(&r.wage_share_by_years, contribution_years).unwrap_or(0);
    from_ppm(
        i128::from(r.isr.get()) * i128::from(r.isr_share_ppm)
            + i128::from(last_wage.get().max(0)) * i128::from(share),
    )
}

/// How a person lost a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Separation {
    /// The employer laid the person off.
    Layoff,
    /// The person quit.
    Quit,
}

/// A running unemployment benefit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnemploymentClaim {
    /// The benefit a month.
    pub amount: Bani,
    /// `benefit_months_left`.
    pub months_left: u8,
}

/// The claim a lost job opens: `None` after a quit or with no right to the
/// benefit (under one year of contributions) (AC-ST-02).
#[must_use]
pub fn start_claim(
    separation: Separation,
    last_wage: Bani,
    contribution_years: u8,
    r: &UnemploymentRule,
) -> Option<UnemploymentClaim> {
    let months_left = benefit_months(contribution_years, r);
    if separation == Separation::Quit || months_left == 0 {
        return None;
    }
    Some(UnemploymentClaim {
        amount: unemployment_benefit(last_wage, contribution_years, r),
        months_left,
    })
}

/// The month's payment of a claim, on pay day: nothing, and no months left,
/// if the person was hired during the month; otherwise the amount while
/// months are left, one month fewer each time (AC-ST-02).
pub fn claim_month(claim: &mut UnemploymentClaim, hired_this_month: bool) -> Bani {
    if hired_this_month {
        claim.months_left = 0;
    }
    if claim.months_left == 0 {
        return Bani::ZERO;
    }
    claim.months_left = claim.months_left.saturating_sub(1);
    claim.amount
}

/// The child allowance's rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildRule {
    /// `child_allowance_under2`, a month.
    pub under2: Bani,
    /// `child_allowance`, from 2 to 18, a month.
    pub to18: Bani,
}

/// The child allowance for a child of `age_months`, paid to the child's
/// household (AC-ST-03).
pub fn child_allowance(age_months: u16, r: &ChildRule) -> Bani {
    match age_months {
        0..24 => r.under2,
        24..216 => r.to18,
        _ => Bani::ZERO,
    }
}

/// The guaranteed minimum income's rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimumIncomeRule {
    /// The floor a month for 1 to 5 members.
    pub floor_by_size: Vec<Bani>,
    /// Added to the floor for each member beyond five.
    pub per_extra: Bani,
    /// `gmi_take_up`, in millionths (applied by the pay-day phase).
    pub take_up_ppm: u32,
}

/// The minimum income of a household: the floor for its size less its
/// month's income after taxes and the other programmes, not below zero
/// (AC-ST-04).
pub fn minimum_income(size: u16, month_income: Bani, r: &MinimumIncomeRule) -> Bani {
    let n = usize::from(size);
    let floor = match (n, r.floor_by_size.last()) {
        (0, _) | (_, None) => Bani::ZERO,
        (n, Some(&last)) if n > r.floor_by_size.len() => {
            let extra = i64::try_from(n.saturating_sub(r.floor_by_size.len())).unwrap_or(i64::MAX);
            last + r.per_extra.times(extra)
        }
        (n, Some(_)) => r.floor_by_size[n.saturating_sub(1)],
    };
    (floor - month_income).max(Bani::ZERO)
}
