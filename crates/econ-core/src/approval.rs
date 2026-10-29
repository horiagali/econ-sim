//! Approval's first version (spec `society/opinion-approval`, "First version
//! (roadmap M5)", feature `approval`): the pocketbook parts of a person's
//! approval (income, unemployment, inflation, hardship), their smoothing and
//! bounds, and the per-person approval and group means on the `World`.
//! Display only: nothing in the economy reads it.
//!
//! Points are thousandths (millipoints), ratios millionths, rounding half
//! away from zero; no random draw. API skeleton from the spec's API sketch,
//! written with the acceptance tests before the code (ADR-0014); every body
//! is `unimplemented!` until the implementation fills it.

use econ_types::{Bani, Month};

use crate::world::World;

/// The parameters of the spec's table (first version).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApprovalParams {
    /// α_y, millipoints per unit (1.0) of relative change of real income.
    pub income_weight_milli: i32,
    /// κ, loss aversion, millionths.
    pub loss_aversion_ppm: u32,
    /// c^max, millipoints.
    pub income_cap_milli: i32,
    /// λ_r a month, millionths.
    pub reference_speed_ppm: u32,
    /// u_self, millipoints.
    pub unemployed_self_milli: i32,
    /// u_hh per other member looking for work (at most two), millipoints.
    pub unemployed_household_milli: i32,
    /// α_π, millipoints per percentage point of inflation above π0.
    pub inflation_weight_milli: i32,
    /// π0, a year, millionths.
    pub inflation_neutral_ppm: i32,
    /// h, millipoints.
    pub hardship_milli: i32,
    /// λ_A a month, millionths.
    pub smoothing_ppm: u32,
}

impl ApprovalParams {
    /// The spec's defaults: α_y 60 points, κ 2, c^max 25 points, λ_r 1/24,
    /// u_self 10, u_hh 4, α_π 1 point per point, π0 3%, h 8, λ_A 0.5.
    #[must_use]
    pub fn spec_defaults() -> ApprovalParams {
        ApprovalParams {
            income_weight_milli: 60_000,
            loss_aversion_ppm: 2_000_000,
            income_cap_milli: 25_000,
            reference_speed_ppm: 41_667,
            unemployed_self_milli: 10_000,
            unemployed_household_milli: 4_000,
            inflation_weight_milli: 1_000,
            inflation_neutral_ppm: 30_000,
            hardship_milli: 8_000,
            smoothing_ppm: 500_000,
        }
    }
}

/// The five parts of a person's approval, in the order of the arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// Own real income against the reference.
    Income = 0,
    /// The person or members of the household looking for work.
    Unemployment = 1,
    /// Inflation above π0.
    Inflation = 2,
    /// The household in hardship.
    Hardship = 3,
    /// What the clip to 0–100 took or added.
    Bounds = 4,
}

/// What steps 1 to 4 read for one adult.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersonInputs {
    /// The household's recent real income, `ỹ_h`.
    pub recent: Bani,
    /// The household's reference real income before this close, `r_{h,t−1}`.
    pub reference: Bani,
    /// The person is looking for work.
    pub seeking: bool,
    /// Other members of the household looking for work.
    pub others_seeking: u8,
    /// Twelve-month inflation at the close, millionths a year.
    pub inflation_ppm: i32,
    /// The household was in hardship at its last decision.
    pub hardship: bool,
}

/// The targets of steps 1 to 4 (AC-APP-01), millipoints: income,
/// unemployment, inflation, hardship.
pub fn targets(i: &PersonInputs, p: &ApprovalParams) -> [i32; 4] {
    let _ = (i, p);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Steps 5 and 6 (AC-APP-02): the smoothed parts from the previous ones
/// (`None` for a person new to approval: the parts start at their targets),
/// then `A_i` and the bounds part. Returns the five parts and `A_i`,
/// millipoints.
pub fn step(prev: Option<[i32; 4]>, targets: [i32; 4], p: &ApprovalParams) -> ([i32; 5], i32) {
    let _ = (prev, targets, p);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The household's reference after the close: `r + λ_r (ỹ − r)`, whole bani.
pub fn next_reference(reference: Bani, recent: Bani, p: &ApprovalParams) -> Bani {
    let _ = (reference, recent, p);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The approval of a group of adults at a close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupApproval {
    /// Adults (18 or more), weighted.
    pub adults: u64,
    /// Weighted mean of `A_i`, millipoints.
    pub mean_milli: i32,
    /// Weighted means of the five parts, millipoints.
    pub parts_milli: [i32; 5],
    /// The composition part of the change since the last close: what persons
    /// who entered or left the group changed in its mean, millipoints (0 at
    /// the first close).
    pub composition_milli: i32,
}

/// The approval of a close, nationally and by built-in group. A group with
/// no adult has none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalStats {
    /// The month closed.
    pub month: Month,
    /// Every adult.
    pub national: Option<GroupApproval>,
    /// By activity, indexed by `econ_popgen::Activity` as its code.
    pub by_activity: Vec<Option<GroupApproval>>,
    /// By income fifth (`hh_fifth`).
    pub by_fifth: [Option<GroupApproval>; 5],
    /// By development region.
    pub by_region: Vec<Option<GroupApproval>>,
    /// By county.
    pub by_county: Vec<Option<GroupApproval>>,
    /// By age band: 18–24, 25–34, …, 55–64, 65 or more.
    pub by_age: Vec<Option<GroupApproval>>,
    /// By `edu_level`.
    pub by_edu: [Option<GroupApproval>; 5],
    /// By sex (`econ_popgen::Sex` as its index).
    pub by_sex: [Option<GroupApproval>; 2],
}

impl World {
    /// A person's approval and its five parts, millipoints; `None` for a
    /// person under 18.
    #[must_use]
    pub fn approval(&self, person: u32) -> Option<(i32, [i32; 5])> {
        let _ = person;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The approval of every close, in order.
    #[must_use]
    pub fn approval_stats(&self) -> &[ApprovalStats] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}
