//! Value-added tax (spec `economy/vat`, AC-VAT-01..05).
//!
//! Rates are integer basis points so VAT is exact rational arithmetic
//! (`Bani::mul_ratio`), never a float multiply.

use econ_ledger::{FlowCode, Instrument, Ledger, LedgerError, Sector, Txn};
use econ_types::Bani;

/// A VAT rate in basis points (2100 = 21%).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VatRate(pub u32);

impl VatRate {
    /// EU minimum standard rate (Directive 2006/112/EC, art. 97).
    pub const EU_MIN_STANDARD: VatRate = VatRate(1500);

    /// A rate of whole percent.
    #[must_use]
    pub const fn percent(p: u32) -> VatRate {
        VatRate(p * 100)
    }

    /// Net + VAT.
    pub fn gross(self, net: Bani) -> Bani {
        net + vat_on(net, self)
    }
}

/// VAT on `net` at `rate`, rounded half away from zero to whole bani (AC-VAT-01).
pub fn vat_on(net: Bani, rate: VatRate) -> Bani {
    net.mul_ratio(i64::from(rate.0), 10_000)
}

/// Which rate applies to a good (AC-VAT-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VatCategory {
    /// Standard rate.
    Standard,
    /// Reduced rate (food, medicines, housing, …).
    Reduced,
    /// Taxable at 0% (e.g. intra-EU supplies, exports).
    Zero,
    /// Outside VAT (financial services, health, education).
    Exempt,
}

/// A configuration problem the player is told about but not prevented from making.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VatWarning {
    /// Standard rate below the EU floor of 15% (AC-VAT-04).
    BelowEuMinimumStandardRate {
        /// The configured standard rate.
        rate: VatRate,
    },
}

/// The player-set VAT schedule (lever: `vat_rate_standard`, `vat_rate_reduced`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VatSchedule {
    /// Standard rate.
    pub standard: VatRate,
    /// Reduced rate.
    pub reduced: VatRate,
}

impl VatSchedule {
    /// Rate for a category; `None` for exempt goods.
    #[must_use]
    pub fn rate_for(&self, cat: VatCategory) -> Option<VatRate> {
        match cat {
            VatCategory::Standard => Some(self.standard),
            VatCategory::Reduced => Some(self.reduced),
            VatCategory::Zero => Some(VatRate(0)),
            VatCategory::Exempt => None,
        }
    }

    /// VAT on a purchase of `net` in category `cat`.
    pub fn vat(&self, net: Bani, cat: VatCategory) -> Bani {
        self.rate_for(cat).map_or(Bani(0), |r| vat_on(net, r))
    }

    /// Price the buyer pays.
    pub fn gross(&self, net: Bani, cat: VatCategory) -> Bani {
        net + self.vat(net, cat)
    }

    /// EU-rule breaches; the schedule stays usable (AC-VAT-04).
    #[must_use]
    pub fn warnings(&self) -> Vec<VatWarning> {
        let mut w = Vec::new();
        if self.standard < VatRate::EU_MIN_STANDARD {
            w.push(VatWarning::BelowEuMinimumStandardRate {
                rate: self.standard,
            });
        }
        w
    }
}

/// One purchase record: per-unit net amount, category, and integer record weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Purchase {
    /// Net amount per represented unit.
    pub net: Bani,
    /// VAT category of the good.
    pub category: VatCategory,
    /// Record weight (`hh_weight`).
    pub weight: u32,
}

/// Post VAT on household purchases as one linked transaction, households →
/// government, flow code `tax.vat` (AC-VAT-03). Returns the total collected.
///
/// # Errors
/// The ledger rejected the transaction (nothing was posted).
pub fn collect_vat(
    ledger: &mut Ledger,
    schedule: &VatSchedule,
    purchases: &[Purchase],
) -> Result<Bani, LedgerError> {
    let mut txn = Txn::new();
    let mut total = Bani(0);
    for p in purchases {
        let v = schedule.vat(p.net, p.category);
        if v.get() == 0 || p.weight == 0 {
            continue;
        }
        total += v.times(i64::from(p.weight));
        txn = txn.weighted_leg(
            Sector::Households,
            Sector::Government,
            Instrument::Cash,
            v,
            p.weight,
            FlowCode::TAX_VAT,
        );
    }
    if total.get() != 0 {
        ledger.commit(txn)?;
    }
    Ok(total)
}
