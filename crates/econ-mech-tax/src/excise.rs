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

use econ_types::{Bani, split_largest_remainder};

/// One, in millionths.
const ONE: i128 = 1_000_000;
/// Basis points in one.
const BP: i128 = 10_000;
/// The spec's bound on a purchase's real units ("Edge cases": `q < 2^43`).
const MAX_QUANTITY: i64 = (1 << 43) - 1;

/// `num / den` rounded half away from zero (`den > 0`).
fn round_div(num: i128, den: i128) -> i128 {
    let (q, r) = (num / den, (num % den).abs());
    if 2 * r >= den { q + num.signum() } else { q }
}

/// An `i128` amount as bani; beyond `i64` it saturates.
fn to_bani(v: i128) -> Bani {
    Bani(i64::try_from(v).unwrap_or(if v < 0 { i64::MIN } else { i64::MAX }))
}

/// Bani × millionths as bani, rounded once.
fn from_ppm(v: i128) -> Bani {
    to_bani(round_div(v, ONE))
}

/// A duty on one real unit of a good, millionths of a ban.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExciseDuty(pub u32);

/// The duty on `quantity` real units: `q · d / 10^6`, rounded half away from
/// zero to whole bani (AC-EXC-01).
pub fn excise_on(quantity: i64, duty: ExciseDuty) -> Bani {
    from_ppm(i128::from(quantity) * i128::from(duty.0))
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
    let net = from_ppm(i128::from(quantity) * i128::from(price_ppm));
    let excise = excise_on(quantity, duty);
    let vat = to_bani(round_div(
        (i128::from(net.get()) + i128::from(excise.get())) * i128::from(vat_bp),
        BP,
    ));
    ExciseSplit {
        quantity,
        net,
        excise,
        vat,
    }
}

/// What a gross budget buys: the largest whole number of real units whose
/// net + duty + VAT does not exceed it (AC-EXC-03).
#[must_use]
pub fn split_budget(budget: Bani, price_ppm: u32, duty: ExciseDuty, vat_bp: u32) -> ExciseSplit {
    let fits = |q: i64| {
        let s = gross_of(q, price_ppm, duty, vat_bp);
        i128::from(s.net.get()) + i128::from(s.excise.get()) + i128::from(s.vat.get())
            <= i128::from(budget.get())
    };
    // A unit cheaper than half a ban rounds to nothing: even a zero budget
    // may buy one.
    if !fits(1) {
        return gross_of(0, price_ppm, duty, vat_bp);
    }
    // The gross price is non-decreasing in the quantity: find a quantity that
    // does not fit by doubling, then bisect. A free unit (zero price and duty)
    // stops at the spec's bound on quantities.
    let (mut lo, mut hi) = (1_i64, 2_i64);
    while hi < MAX_QUANTITY && fits(hi) {
        lo = hi;
        hi = hi.saturating_mul(2).min(MAX_QUANTITY);
    }
    if fits(hi) {
        return gross_of(hi, price_ppm, duty, vat_bp);
    }
    // Invariant: `lo` fits, `hi` does not.
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if fits(mid) { lo = mid } else { hi = mid }
    }
    gross_of(lo, price_ppm, duty, vat_bp)
}

/// The duty after the 1 January indexation: `round(d (1 + π12))`, unchanged
/// for `π12 ≤ 0` (AC-EXC-04).
#[must_use]
pub fn index_duty(duty: ExciseDuty, inflation_ppm: i32) -> ExciseDuty {
    if inflation_ppm <= 0 {
        return duty;
    }
    let d = round_div(i128::from(duty.0) * (ONE + i128::from(inflation_ppm)), ONE);
    ExciseDuty(u32::try_from(d).unwrap_or(u32::MAX))
}

/// The starting duty: a group's revenue over the value at basic prices of
/// the purchases that pay it, in millionths (AC-EXC-05).
#[must_use]
pub fn calibrate_duty_ppm(target: Bani, base_value: Bani) -> ExciseDuty {
    if base_value <= Bani::ZERO || target <= Bani::ZERO {
        return ExciseDuty(0);
    }
    let d = round_div(i128::from(target.get()) * ONE, i128::from(base_value.get()));
    ExciseDuty(u32::try_from(d).unwrap_or(u32::MAX))
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
    assert_eq!(basket_ppm.len(), duty.len(), "a duty per basket good");
    if basket_ppm.is_empty() {
        return Vec::new();
    }
    // Share × (10^6 + d): below 2^20 · 2^33, far inside u64.
    let weights: Vec<u64> = basket_ppm
        .iter()
        .zip(duty)
        .map(|(&b, d)| u64::from(b) * (1_000_000 + u64::from(d.0)))
        .collect();
    split_largest_remainder(Bani(1_000_000), &weights)
        .into_iter()
        .map(|b| u32::try_from(b.get()).expect("a share of 10^6 fits u32"))
        .collect()
}
