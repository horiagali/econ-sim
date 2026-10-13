//! Production (spec `economy/production`, "First version"): a unit's monthly
//! plan, and rationing when demand on an industry exceeds its inventories.
//! Pure functions, no I/O; the daily phases live with the `World`.
//!
//! Quantities are whole real units (lei at starting prices); shares are
//! millionths. API skeleton from the spec's API sketch, written before the
//! code (ADR-0014). The bodies come with the implementation change.

/// Tuning parameters, in millionths a month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductionParams {
    /// `demand_expectation_theta`.
    pub theta_ppm: u32,
    /// `inventory_adjust_phi`.
    pub phi_ppm: u32,
}

/// What a unit plans with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitState {
    /// Expected demand a month, before this plan.
    pub expected_demand: i64,
    /// Inventory.
    pub inventory: i64,
    /// Capacity a month.
    pub capacity: i64,
    /// `inventory_target_ratio`: months of expected demand, in millionths.
    pub inventory_target_ppm: u32,
    /// Persons needed per million units of output.
    pub labour_per_million_output: u64,
}

/// The month's plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    /// Expected demand after this month's update.
    pub expected_demand: i64,
    /// Planned output, between 0 and capacity.
    pub planned_output: i64,
    /// `desired_employment`, persons.
    pub desired_employment: u64,
}

/// The plan of step 1 (AC-PROD-01).
#[must_use]
pub fn plan(u: &UnitState, sales_last_month: i64, p: &ProductionParams) -> Plan {
    let _ = (u, sales_last_month, p);
    unimplemented!("econ-mech-production: written in the implementation change")
}

/// What each buyer got, and what was not served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Served {
    /// Sold to each buyer, in the order of the demands.
    pub sold: Vec<i64>,
    /// Demand less sales.
    pub shortage: i64,
}

/// Serve `demands` from `available`; when demand exceeds it every buyer gets
/// the same share of what they asked for (AC-PROD-02).
#[must_use]
pub fn ration(demands: &[i64], available: i64) -> Served {
    let _ = (demands, available);
    unimplemented!("econ-mech-production: written in the implementation change")
}

/// Split an industry's sales over its units in proportion to their
/// inventories, exactly.
#[must_use]
pub fn share_sales(sold: i64, inventories: &[i64]) -> Vec<i64> {
    let _ = (sold, inventories);
    unimplemented!("econ-mech-production: written in the implementation change")
}
