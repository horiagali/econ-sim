//! Taxes on income (spec `economy/taxation`, "First version"): the income tax
//! schedule, the personal deduction, contributions and income tax on a wage,
//! income tax on a pension, and the quarterly corporate and micro-enterprise
//! taxes. Pure functions, no I/O; the pay-day and quarter-end phases that call
//! them and post through the ledger live with the `World`.
//!
//! Amounts are whole bani; rates are basis points (as VAT's). Each amount is
//! rounded once, half away from zero (ADR-0007).

use econ_types::Bani;

/// Basis points in one.
const BP: i128 = 10_000;

/// `num / den` rounded half away from zero (`den > 0`).
// Callers pass bani times basis points, far inside i128.
#[allow(clippy::arithmetic_side_effects)]
fn round_div(num: i128, den: i128) -> i128 {
    let (q, r) = (num / den, (num % den).abs());
    if 2 * r >= den { q + num.signum() } else { q }
}

/// A sum of bani × basis points as bani, rounded once.
fn from_bp(sum: i128) -> Bani {
    Bani(i64::try_from(round_div(sum, BP)).expect("a tax fits i64"))
}

/// `amount × rate_bp` in bani × basis points, not yet rounded.
// i64 × u32 fits i128.
#[allow(clippy::arithmetic_side_effects)]
fn times_bp(amount: Bani, rate_bp: u32) -> i128 {
    i128::from(amount.get()) * i128::from(rate_bp)
}

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
        Schedule {
            thresholds: vec![Bani::ZERO],
            rates_bp: vec![rate_bp],
        }
    }

    /// Tax on a month's taxable income: each bracket's rate on the part of
    /// `taxable` inside it, summed and rounded once (AC-TAX-03).
    pub fn tax(&self, taxable: Bani) -> Bani {
        from_bp(self.tax_bp(taxable))
    }

    /// [`Schedule::tax`] before rounding, in bani × basis points.
    fn tax_bp(&self, taxable: Bani) -> i128 {
        let mut sum = 0i128;
        for (k, (&from, &rate)) in self.thresholds.iter().zip(&self.rates_bp).enumerate() {
            if taxable <= from {
                break;
            }
            let to = self
                .thresholds
                .get(k.saturating_add(1))
                .map_or(taxable, |&next| next.min(taxable));
            sum = sum.saturating_add(times_bp(to - from, rate));
        }
        sum
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
// The span and the base are a few thousand lei: products fit i128.
#[allow(clippy::arithmetic_side_effects)]
pub fn personal_deduction(gross: Bani, dependants: u8, r: &DeductionRule) -> Bani {
    let base = r.base[usize::from(dependants.min(4))];
    if gross <= r.full_until {
        return base;
    }
    if gross >= r.zero_at {
        return Bani::ZERO;
    }
    // The straight line between the two ends, in whole lei.
    let left = i128::from((r.zero_at - gross).get());
    let span = i128::from((r.zero_at - r.full_until).get());
    let lei = round_div(i128::from(base.get()) * left, span * 100);
    Bani::from_lei(i64::try_from(lei).expect("a deduction fits i64"))
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
///
/// With an override, its rates apply to the declared wage up to its ceiling
/// and the general rates to the rest. The personal deduction is taken from
/// the income taxed by the general schedule first, then from the rest.
pub fn tax_wage(
    wage: Bani,
    declared: Bani,
    dependants: u8,
    t: &LabourTaxes,
    over: Option<&IndustryOverride>,
) -> WageTax {
    tax_wage_with(wage, declared, dependants, t, over, &WageLevers::NONE)
}

/// The levers of the law in force on a wage beyond [`LabourTaxes`] (spec
/// `game/lever-system`, the tax levers of M4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WageLevers {
    /// `contribution_ceiling`: the declared wage a month above which no CAS
    /// or CASS is due; zero for none.
    pub contribution_ceiling: Bani,
    /// The income tax credit of the wage: `child_tax_credit` times the
    /// children the earner claims; it lowers the income tax, never below
    /// zero.
    pub tax_credit: Bani,
}

impl WageLevers {
    /// No ceiling and no credit (the law of 2021).
    pub const NONE: WageLevers = WageLevers {
        contribution_ceiling: Bani::ZERO,
        tax_credit: Bani::ZERO,
    };
}

/// [`tax_wage`] under the wage levers: CAS and CASS on the declared wage up
/// to the contribution ceiling (the override's part first), the income tax
/// less the credit. With [`WageLevers::NONE`] it is [`tax_wage`].
#[must_use]
pub fn tax_wage_with(
    wage: Bani,
    declared: Bani,
    dependants: u8,
    t: &LabourTaxes,
    over: Option<&IndustryOverride>,
    levers: &WageLevers,
) -> WageTax {
    if declared <= Bani::ZERO {
        return WageTax {
            cas_public: Bani::ZERO,
            cas_pillar2: Bani::ZERO,
            cass: Bani::ZERO,
            income_tax: Bani::ZERO,
            cam: Bani::ZERO,
            net: wage,
        };
    }
    // The part under the override's rates and the part under the general ones.
    let (own, general) = match over {
        Some(o) => (
            declared.min(o.ceiling),
            (declared - o.ceiling).max(Bani::ZERO),
        ),
        None => (Bani::ZERO, declared),
    };
    // Each part's base for CAS and CASS: the ceiling takes the override's
    // part first.
    let ceiling = levers.contribution_ceiling;
    let (own_base, general_base) = if ceiling > Bani::ZERO {
        let o = own.min(ceiling);
        (o, general.min(ceiling - o))
    } else {
        (own, general)
    };
    let (own_cas, own_cass) = over.map_or((0, 0), |o| (o.cas_bp, o.cass_bp));
    let cas = from_bp(times_bp(own_base, own_cas).saturating_add(times_bp(general_base, t.cas_bp)));
    let pillar2_base = if ceiling > Bani::ZERO {
        declared.min(ceiling)
    } else {
        declared
    };
    let cas_pillar2 = from_bp(times_bp(pillar2_base, t.pillar2_bp)).min(cas);
    let cass =
        from_bp(times_bp(own_base, own_cass).saturating_add(times_bp(general_base, t.cass_bp)));
    // Income after each part's own contributions (rounded per part only to
    // place the deduction; the tax is rounded once).
    let after = |part: Bani, base: Bani, cas_bp: u32, cass_bp: u32| {
        part - from_bp(times_bp(base, cas_bp)) - from_bp(times_bp(base, cass_bp))
    };
    let own_rate = over.and_then(|o| o.income_rate_bp);
    let (mut by_schedule, mut flat_part) = (
        after(general, general_base, t.cas_bp, t.cass_bp),
        Bani::ZERO,
    );
    if own_rate.is_some() {
        flat_part = after(own, own_base, own_cas, own_cass);
    } else {
        by_schedule += after(own, own_base, own_cas, own_cass);
    }
    let deduction = personal_deduction(declared, dependants, &t.deduction);
    let used = deduction.min(by_schedule.max(Bani::ZERO));
    by_schedule = (by_schedule - used).max(Bani::ZERO);
    flat_part = (flat_part - (deduction - used)).max(Bani::ZERO);
    let income_tax = from_bp(
        t.income
            .tax_bp(by_schedule)
            .saturating_add(times_bp(flat_part, own_rate.unwrap_or(0))),
    );
    let income_tax = (income_tax - levers.tax_credit.max(Bani::ZERO)).max(Bani::ZERO);
    WageTax {
        cas_public: cas - cas_pillar2,
        cas_pillar2,
        cass,
        income_tax,
        cam: from_bp(times_bp(declared, t.cam_bp)),
        net: wage - cas - cass - income_tax,
    }
}

/// Income tax on a pension: the schedule on its part above `tax_free` (AC-TAX-06).
pub fn tax_pension(pension: Bani, tax_free: Bani, s: &Schedule) -> Bani {
    s.tax((pension - tax_free).max(Bani::ZERO))
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
/// turnover instead. Whether a unit is in the micro regime (its declared
/// turnover per firm in the last four quarters under `micro_threshold`) is
/// the caller's to decide; the losses of a micro unit are left as they are.
pub fn tax_quarter(
    profit: Bani,
    losses_carried: &mut Bani,
    micro: Option<(Bani, bool)>,
    t: &ProfitTaxes,
) -> Bani {
    if let Some((turnover, staff)) = micro {
        let rate = if staff {
            t.micro_with_staff_bp
        } else {
            t.micro_no_staff_bp
        };
        return from_bp(times_bp(turnover.max(Bani::ZERO), rate));
    }
    if profit < Bani::ZERO {
        *losses_carried += -profit;
        return Bani::ZERO;
    }
    let used = (*losses_carried).min(profit);
    *losses_carried -= used;
    from_bp(times_bp(profit - used, t.profit_bp))
}
