//! Production (spec `economy/production`, "First version"): a unit's monthly
//! plan, and rationing when demand on an industry exceeds its inventories.
//! Pure functions, no I/O; the daily phases live with the `World`.
//!
//! Quantities are whole real units (lei at starting prices); shares are
//! millionths. Every step is rounded once, half away from zero; splits are
//! exact by largest remainder, ties to the lower index.

/// One, in millionths.
const ONE: i128 = 1_000_000;

/// `n / d` rounded half away from zero (`d > 0`).
fn rdiv(n: i128, d: i128) -> i128 {
    let (q, r) = (n / d, n % d);
    if 2 * r.abs() >= d { q + n.signum() } else { q }
}

/// An `i128` quantity as `i64`.
fn qty(v: i128) -> i64 {
    i64::try_from(v).expect("quantity fits i64")
}

/// Split `total` (≥ 0) over `weights` (negative ones count as 0) in proportion,
/// exactly, by largest remainder with ties to the lower index. All-zero
/// weights give nothing to anyone, so `total` must then be 0.
fn apportion(total: i64, weights: &[i64]) -> Vec<i64> {
    let w: Vec<i128> = weights.iter().map(|&x| i128::from(x.max(0))).collect();
    let sum: i128 = w.iter().sum();
    if sum == 0 {
        return vec![0; weights.len()];
    }
    let t = i128::from(total.max(0));
    let mut parts: Vec<i128> = w.iter().map(|&x| t * x / sum).collect();
    let mut order: Vec<usize> = (0..w.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(t * w[i] % sum), i));
    let missing = usize::try_from(t - parts.iter().sum::<i128>()).expect("fewer than the parts");
    for &i in order.iter().take(missing) {
        parts[i] += 1;
    }
    parts.into_iter().map(qty).collect()
}

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
    // D_e <- D_e + theta (S - D_e).
    let de = i128::from(u.expected_demand);
    let de = de
        + rdiv(
            i128::from(p.theta_ppm) * (i128::from(sales_last_month) - de),
            ONE,
        );
    // Y = D_e + phi (v* D_e - V), between 0 and capacity.
    let gap = i128::from(u.inventory_target_ppm) * de - i128::from(u.inventory) * ONE;
    let y = de + rdiv(i128::from(p.phi_ppm) * gap, ONE * ONE);
    let y = y.clamp(0, i128::from(u.capacity.max(0)));
    // Persons: ceil(Y * labour / 1e6).
    let need = y * i128::from(u.labour_per_million_output);
    let persons = (need + ONE - 1) / ONE;
    Plan {
        expected_demand: qty(de),
        planned_output: qty(y),
        desired_employment: u64::try_from(persons).expect("persons fit u64"),
    }
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
    let asked: i64 = demands.iter().map(|&d| d.max(0)).sum();
    let sold = if asked <= available {
        demands.iter().map(|&d| d.max(0)).collect()
    } else {
        apportion(available.max(0), demands)
    };
    let shortage = asked - sold.iter().sum::<i64>();
    Served { sold, shortage }
}

/// Split an industry's sales over its units in proportion to their
/// inventories, exactly.
#[must_use]
///
/// # Panics
/// If `sold` is above zero and no unit has inventory.
pub fn share_sales(sold: i64, inventories: &[i64]) -> Vec<i64> {
    assert!(
        sold <= 0 || inventories.iter().any(|&v| v > 0),
        "share_sales: sales without inventory"
    );
    apportion(sold, inventories)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sales_are_shared_by_inventories_exactly() {
        assert_eq!(share_sales(10, &[1, 1, 1]), vec![4, 3, 3]);
        assert_eq!(share_sales(100, &[300, 100, 0]), vec![75, 25, 0]);
        assert_eq!(share_sales(0, &[0, 0]), vec![0, 0]);
        let s = share_sales(999_999, &[7, 13, 1_000_003, 5]);
        assert_eq!(s.iter().sum::<i64>(), 999_999);
    }

    #[test]
    fn a_rationed_buyer_never_gets_more_than_asked() {
        let s = ration(&[1, 2, 3], 5);
        assert_eq!(s.sold.iter().sum::<i64>(), 5);
        assert!(s.sold.iter().zip([1, 2, 3]).all(|(&g, d)| g <= d));
        assert_eq!(s.shortage, 1);
    }
}
