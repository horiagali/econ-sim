//! The read-only record of the latest pay day (feature `pay_day`): what each
//! person and household was paid and what was withheld, for the `[sim]`
//! criteria of `economy/taxation` (AC-TAX-08) and `economy/social-transfers`
//! (AC-ST-05 to 08).
//!
//! Test-writer addition to the `World` (ADR-0014), written with those tests
//! before the code; filled by the pay-day phase.

use econ_types::{Bani, Day};

use super::World;

/// The latest pay day, per record. Person columns are aligned with
/// [`super::PersonColumns`], household columns with
/// [`super::HouseholdColumns`]. Amounts are per record, before the household's
/// weight; a record that received nothing holds zero.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaySlips {
    /// The day of the latest pay day; `None` before the first.
    pub day: Option<Day>,
    // Per person: wages (every payment of the day added up, the month's
    // wage and any separation pay), each split as `econ_mech_tax`'s
    // `WageTax`.
    /// Gross wage paid.
    pub gross_wage: Vec<Bani>,
    /// Its declared part.
    pub declared_wage: Vec<Bani>,
    /// The employee's pension contribution to the government.
    pub cas_public: Vec<Bani>,
    /// The part of it paid to the person's Pillar II fund.
    pub cas_pillar2: Vec<Bani>,
    /// The employee's health contribution.
    pub cass: Vec<Bani>,
    /// Income tax on the wage.
    pub income_tax: Vec<Bani>,
    /// The employer's labour insurance contribution, on top of the gross.
    pub cam: Vec<Bani>,
    /// Net wage received.
    pub net_wage: Vec<Bani>,
    /// Gross state pension.
    pub pension: Vec<Bani>,
    /// Income tax on the pension.
    pub pension_tax: Vec<Bani>,
    /// Unemployment benefit.
    pub unemployment_benefit: Vec<Bani>,
    // Per household.
    /// Self-employment and informal income paid to the household.
    pub mixed_income: Vec<Bani>,
    /// Income tax on its declared self-employment income.
    pub self_employment_tax: Vec<Bani>,
    /// Child allowance.
    pub child_allowance: Vec<Bani>,
    /// Guaranteed minimum income.
    pub minimum_income: Vec<Bani>,
    /// Everything the household received on the pay day, after taxes.
    pub received: Vec<Bani>,
}

impl PaySlips {
    /// Empty columns for a new pay day.
    pub(super) fn start(&mut self, day: Day, persons: usize, households: usize) {
        let zero = |v: &mut Vec<Bani>, n: usize| {
            v.clear();
            v.resize(n, Bani::ZERO);
        };
        self.day = Some(day);
        for v in [
            &mut self.gross_wage,
            &mut self.declared_wage,
            &mut self.cas_public,
            &mut self.cas_pillar2,
            &mut self.cass,
            &mut self.income_tax,
            &mut self.cam,
            &mut self.net_wage,
            &mut self.pension,
            &mut self.pension_tax,
            &mut self.unemployment_benefit,
        ] {
            zero(v, persons);
        }
        for v in [
            &mut self.mixed_income,
            &mut self.self_employment_tax,
            &mut self.child_allowance,
            &mut self.minimum_income,
            &mut self.received,
        ] {
            zero(v, households);
        }
    }
}

impl World {
    /// The record of the latest pay day.
    #[must_use]
    pub fn pay_slips(&self) -> &PaySlips {
        &self.slips
    }
}
