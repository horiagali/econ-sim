//! Social transfers (spec `economy/social-transfers`, "First version"): the
//! state pension, the unemployment benefit, the child allowance and the
//! guaranteed minimum income, at their 2021 rules. Pure functions, no I/O; the
//! pay-day phase that walks persons and households, applies take-up and posts
//! through the ledger lives with the `World`.
//!
//! Amounts are whole bani; shares are millionths; pension points are
//! millionths of a point. Each amount is rounded once, half away from zero
//! (ADR-0007). API skeleton from the spec's API sketch, written before the
//! code (ADR-0014); `Separation`, `UnemploymentClaim`, `start_claim` and
//! `claim_month` were added by the test writer for AC-ST-02 (no benefit after
//! a quit; the benefit stops when the person is hired or the months run out).
//! The bodies come with the implementation change.

use econ_types::Bani;

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
pub fn pension_points(starting_pension: Bani, point_value: Bani) -> u64 {
    let _ = (starting_pension, point_value);
    unimplemented!("econ-mech-transfers: written in the implementation change")
}

/// The month's pension: points × point value, not below the minimum (AC-ST-01).
pub fn pension(points: u64, r: &PensionRule) -> Bani {
    let _ = (points, r);
    unimplemented!("econ-mech-transfers: written in the implementation change")
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
    let _ = (contribution_years, r);
    unimplemented!("econ-mech-transfers: written in the implementation change")
}

/// The benefit a month: `ui_isr_share` × ISR + the share of `last_wage` for the
/// contribution years; nothing under the first entry of `months_by_years`
/// (AC-ST-02).
pub fn unemployment_benefit(last_wage: Bani, contribution_years: u8, r: &UnemploymentRule) -> Bani {
    let _ = (last_wage, contribution_years, r);
    unimplemented!("econ-mech-transfers: written in the implementation change")
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
    let _ = (separation, last_wage, contribution_years, r);
    unimplemented!("econ-mech-transfers: written in the implementation change")
}

/// The month's payment of a claim, on pay day: nothing, and no months left,
/// if the person was hired during the month; otherwise the amount while
/// months are left, one month fewer each time (AC-ST-02).
pub fn claim_month(claim: &mut UnemploymentClaim, hired_this_month: bool) -> Bani {
    let _ = (claim, hired_this_month);
    unimplemented!("econ-mech-transfers: written in the implementation change")
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
    let _ = (age_months, r);
    unimplemented!("econ-mech-transfers: written in the implementation change")
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
    let _ = (size, month_income, r);
    unimplemented!("econ-mech-transfers: written in the implementation change")
}
