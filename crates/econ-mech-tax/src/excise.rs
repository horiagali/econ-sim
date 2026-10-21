//! Excise duties (spec `economy/excise`, "API sketch"): the duty on a
//! purchase, the price with duty and VAT, the split of a gross budget,
//! indexation and calibration. Amounts in bani, duties in millionths of a ban
//! per real unit (one real unit is what one ban bought at the starting price),
//! prices in millionths of a ban per real unit, VAT in basis points.
//!
//! Declared by the test writer from the spec's API sketch (ADR-0014, test
//! session of 2026-10-21); every body panics until the implementation fills
//! it. The `World` charges excise in its goods market; these are its pure
//! rules.

use econ_types::Bani;

/// A duty on one real unit of a good, millionths of a ban.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExciseDuty(pub u32);

/// The duty on `quantity` real units: `q · d / 10^6`, rounded half away from
/// zero to whole bani (AC-EXC-01).
#[must_use]
pub fn excise_on(quantity: i64, duty: ExciseDuty) -> Bani {
    let _ = (quantity, duty);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// A purchase split into its parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExciseSplit {
    /// Real units bought.
    pub quantity: i64,
    /// Their net value at the day's price.
    pub net: Bani,
    /// The duty on them.
    pub excise: Bani,
    /// VAT on net plus duty.
    pub vat: Bani,
}

/// The parts of a purchase of `quantity` real units at `price_ppm`: net,
/// duty, and VAT charged on net plus duty (AC-EXC-02).
#[must_use]
pub fn gross_of(quantity: i64, price_ppm: u32, duty: ExciseDuty, vat_bp: u32) -> ExciseSplit {
    let _ = (quantity, price_ppm, duty, vat_bp);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// What a gross budget buys: the largest whole number of real units whose
/// net + duty + VAT does not exceed it (AC-EXC-03).
#[must_use]
pub fn split_budget(budget: Bani, price_ppm: u32, duty: ExciseDuty, vat_bp: u32) -> ExciseSplit {
    let _ = (budget, price_ppm, duty, vat_bp);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The duty after the 1 January indexation: `round(d (1 + π12))`, unchanged
/// for `π12 ≤ 0` (AC-EXC-04).
#[must_use]
pub fn index_duty(duty: ExciseDuty, inflation_ppm: i32) -> ExciseDuty {
    let _ = (duty, inflation_ppm);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The starting duty: a group's revenue over the value at basic prices of
/// the purchases that pay it, in millionths (AC-EXC-05).
#[must_use]
pub fn calibrate_duty_ppm(target: Bani, base_value: Bani) -> ExciseDuty {
    let _ = (target, base_value);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The law's duties, per catalogue good and buying industry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExciseSchedule {
    /// Per catalogue good; 0 means none.
    pub duty: Vec<ExciseDuty>,
    /// Per good: whether firm units' input purchases pay it.
    pub firms_pay: Vec<bool>,
    /// Per buying industry: exempt from the duty on its inputs.
    pub exempt_buyer: Vec<bool>,
    /// Whether duties follow the CPI each 1 January.
    pub indexation: bool,
}

/// The households' basket with each excised good's share times `1 + d` and
/// the shares renormalised to 1,000,000 by largest remainder.
#[must_use]
pub fn basket_with_excise(basket_ppm: &[u32], duty: &[ExciseDuty]) -> Vec<u32> {
    let _ = (basket_ppm, duty);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}
