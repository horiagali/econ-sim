//! Taxes on income (spec `economy/taxation`, "First version"): the income tax
//! schedule, the personal deduction, contributions and income tax on a wage,
//! income tax on a pension, and the quarterly corporate and micro-enterprise
//! taxes. Pure functions, no I/O; the pay-day and quarter-end phases that call
//! them and post through the ledger live with the `World`.
//!
//! Amounts are whole bani; rates are basis points (as VAT's). Each amount is
//! rounded once, half away from zero (ADR-0007). API skeleton from the spec's
//! API sketch, written before the code (ADR-0014); the bodies come with the
//! implementation change.

use econ_types::Bani;

/// The income tax schedule, a month: thresholds `0 = b_0 < b_1 < …` and the
/// marginal rate of each bracket. A flat tax is a one-bracket schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    /// Lower bound of each bracket, ascending; the first is 0.
    pub thresholds: Vec<Bani>,
    /// Marginal rate of each bracket, in basis points (same length as `thresholds`).
    pub rates_bp: Vec<u32>,
}

impl Schedule {
    /// A one-bracket schedule: `rate_bp` on all taxable income.
    #[must_use]
    pub fn flat(rate_bp: u32) -> Schedule {
        let _ = rate_bp;
        unimplemented!("econ-mech-tax income: written in the implementation change")
    }

    /// Tax on a month's taxable income: each bracket's rate on the part of
    /// `taxable` inside it, summed and rounded once (AC-TAX-03).
    pub fn tax(&self, taxable: Bani) -> Bani {
        let _ = taxable;
        unimplemented!("econ-mech-tax income: written in the implementation change")
    }
}

/// The personal deduction's rule, a month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeductionRule {
    /// `deduction_base[d]` for 0 to 4 dependants.
    pub base: [Bani; 5],
    /// `deduction_full_until`: the full base applies up to this gross wage.
    pub full_until: Bani,
    /// `deduction_zero_at`: no deduction from this gross wage.
    pub zero_at: Bani,
}

/// The personal deduction for a gross declared wage and a number of
/// dependants (at most four counted), in whole lei (AC-TAX-02).
pub fn personal_deduction(gross: Bani, dependants: u8, r: &DeductionRule) -> Bani {
    let _ = (gross, dependants, r);
    unimplemented!("econ-mech-tax income: written in the implementation change")
}

/// The general taxes on a wage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabourTaxes {
    /// `cas_rate`: employee's pension contribution, Pillar II share included.
    pub cas_bp: u32,
    /// `pillar2_share`: the part of CAS that goes to the person's Pillar II fund.
    pub pillar2_bp: u32,
    /// `cass_rate`: employee's health contribution.
    pub cass_bp: u32,
    /// `cam_rate`: employer's labour insurance contribution, on top of the gross wage.
    pub cam_bp: u32,
    /// The income tax schedule.
    pub income: Schedule,
    /// The personal deduction.
    pub deduction: DeductionRule,
}

/// An industry's own rates (data of the law), up to `ceiling` of gross wage a
/// month; the general rates apply above it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndustryOverride {
    /// Income tax rate; `None` keeps the general schedule.
    pub income_rate_bp: Option<u32>,
    /// CAS rate.
    pub cas_bp: u32,
    /// CASS rate.
    pub cass_bp: u32,
    /// Gross wage a month up to which the override applies.
    pub ceiling: Bani,
}

/// Contributions and income tax on one wage, a month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WageTax {
    /// CAS to the government.
    pub cas_public: Bani,
    /// CAS to the person's Pillar II pension fund.
    pub cas_pillar2: Bani,
    /// CASS.
    pub cass: Bani,
    /// Income tax.
    pub income_tax: Bani,
    /// CAM, paid by the employer on top of the gross wage.
    pub cam: Bani,
    /// Net wage received: `wage` − contributions − income tax.
    pub net: Bani,
}

/// Contributions and income tax on a wage of which `declared` is declared
/// (AC-TAX-01, 04, 05). Only the declared part is taxed.
#[must_use]
pub fn tax_wage(
    wage: Bani,
    declared: Bani,
    dependants: u8,
    t: &LabourTaxes,
    over: Option<&IndustryOverride>,
) -> WageTax {
    let _ = (wage, declared, dependants, t, over);
    unimplemented!("econ-mech-tax income: written in the implementation change")
}

/// Income tax on a pension: the schedule on its part above `tax_free` (AC-TAX-06).
pub fn tax_pension(pension: Bani, tax_free: Bani, s: &Schedule) -> Bani {
    let _ = (pension, tax_free, s);
    unimplemented!("econ-mech-tax income: written in the implementation change")
}

/// The quarterly taxes on firm units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfitTaxes {
    /// `profit_tax_rate`.
    pub profit_bp: u32,
    /// `micro_threshold`: declared turnover per firm in the last four quarters.
    pub micro_threshold: Bani,
    /// `micro_rate_with_staff`.
    pub micro_with_staff_bp: u32,
    /// `micro_rate_no_staff`.
    pub micro_no_staff_bp: u32,
}

/// The quarter's tax of a firm unit (AC-TAX-07): profit tax on `profit` after
/// the losses carried forward (`losses_carried`, a non-negative amount, is
/// used up first and grows with a loss), or, for a micro unit
/// (`micro` = declared turnover, has staff), the micro-enterprise tax on
/// turnover instead.
pub fn tax_quarter(
    profit: Bani,
    losses_carried: &mut Bani,
    micro: Option<(Bani, bool)>,
    t: &ProfitTaxes,
) -> Bani {
    let _ = (profit, losses_carried, micro, t);
    unimplemented!("econ-mech-tax income: written in the implementation change")
}
