//! The test writer's additions for M3's investment and revenue (ADR-0014),
//! from the test session of 2026-10-21. The specs are
//! economy/investment-capital, excise, eu-funds, government-revenue-2021 and
//! imputed-rent. This file holds the read-only views their acceptance
//! criteria read, plus one rule of an API sketch that the `World` does not
//! export.
//!
//! Each item is declared from its spec only, because the code came before its
//! tests and the test writer does not read it. Every body panics until the
//! implementation fills it in. The implementation may move each item next to
//! the code it reads, but must keep its name and shape.

#[cfg(feature = "revenue")]
use econ_ledger::FlowCode;
#[cfg(feature = "revenue")]
use econ_types::Day;
use econ_types::{Bani, Month};

use super::World;

/// A closed month of investment (spec economy/investment-capital,
/// "Outputs"), national bani; per-unit columns align with
/// [`super::FirmUnitColumns`].
#[cfg(feature = "investment")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvestmentMonth {
    /// The month.
    pub month: Month,
    /// Each firm unit's investment purchases in the month, home and
    /// imported, at the prices paid (product taxes not included).
    pub spent: Vec<Bani>,
    /// The same divided by `p^I`, real bani: what entered the unit's newest
    /// slot of work in progress at the close (step 6).
    pub spent_real: Vec<i64>,
    /// Firms' investment in the month: the sum of `spent`.
    pub private: Bani,
    /// The government's investment in the month (step 3: `gov.investment`
    /// and its imported part).
    pub government: Bani,
    /// The investment in the month's GDP by expenditure.
    pub gdp_investment: Bani,
    /// `p^I` at the close, millionths.
    pub price_index_ppm: u32,
    /// Each unit's profit before tax `Π^pre` of the month (step 4).
    pub profit_before_tax: Vec<Bani>,
}

/// Each firm unit's investment budget on its latest plan day and what bounded
/// it (spec economy/investment-capital, step 1), aligned with
/// [`super::FirmUnitColumns`].
#[cfg(feature = "investment")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InvestmentPlans {
    /// `I*`, the desired budget.
    pub desired: Vec<Bani>,
    /// `I`, the budget it can finance.
    pub budget: Vec<Bani>,
    /// Its spare money on that day (deposits beyond a month of wages).
    pub spare: Vec<Bani>,
    /// Its credit room on that day (`econ_mech_banking::credit_room`).
    pub credit_room: Vec<Bani>,
}

/// The latest day's taxes on purchases and the EU's payments to units (specs
/// economy/excise, government-revenue-2021 part 1, eu-funds step 4). Goods
/// and industries are catalogue indices, units align with
/// [`super::FirmUnitColumns`].
#[cfg(feature = "revenue")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevenueDay {
    /// Each good's excise duty in force, millionths of a ban per real unit;
    /// 0 for a good with none.
    pub excise_duty_ppm: Vec<u32>,
    /// Households' purchases of each good, real units, with weights, home
    /// and imported.
    pub hh_quantity: Vec<i64>,
    /// The excise on them, by good.
    pub hh_excise: Vec<Bani>,
    /// Each firm unit's purchases of `refined_petroleum`, real units, home
    /// and imported.
    pub unit_fuel_quantity: Vec<i64>,
    /// The excise each firm unit paid.
    pub unit_excise: Vec<Bani>,
    /// Each firm unit's input purchases, home and imported, at the prices
    /// paid, excise not included.
    pub unit_inputs: Vec<Bani>,
    /// Each firm unit's investment purchases, at the prices paid.
    pub unit_investment: Vec<Bani>,
    /// The product taxes each firm unit paid (`tax.products`).
    pub unit_product_tax: Vec<Bani>,
    /// What each firm unit received under `subsidy.eu_agri`.
    pub unit_eu_agri: Vec<Bani>,
}

/// A held revenue line's payment at a month end (spec
/// economy/government-revenue-2021, part 3).
#[cfg(feature = "revenue")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldPayment {
    /// The line's flow code.
    pub code: FlowCode,
    /// The month's twelfth.
    pub due: Bani,
    /// What the government received.
    pub paid: Bani,
    /// What households could not pay and was forgiven.
    pub shortfall: Bani,
}

/// A closed month of revenue (specs economy/excise, eu-funds,
/// government-revenue-2021).
#[cfg(feature = "revenue")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevenueMonth {
    /// The month.
    pub month: Month,
    /// Excise by good (catalogue index).
    pub excise_by_good: Vec<Bani>,
    /// Each firm unit's output value in the month (part 2's base).
    pub unit_output_value: Vec<Bani>,
    /// The other taxes on production each unit paid at the month end.
    pub unit_production_tax: Vec<Bani>,
    /// The held revenue lines paid at the month end, in the spec's order.
    pub held: Vec<HeldPayment>,
    /// Each household's property tax paid at the month end, per record
    /// (before its weight).
    pub hh_property_tax: Vec<Bani>,
    /// The month's receipts from the EU (the three lines) less own resources
    /// paid (`transfer.eu_budget`).
    pub eu_net_balance: Bani,
}

/// A held benefit's payment on the latest pay day (spec
/// economy/government-revenue-2021, part 4).
#[cfg(feature = "revenue")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldBenefit {
    /// The programme as the spec's table names it, for example
    /// `sickness_disability.disability` or `education_grants`.
    pub programme: String,
    /// Its flow code.
    pub code: FlowCode,
    /// The month's twelfth.
    pub due: Bani,
    /// What each household received, per record (before its weight).
    pub received: Vec<Bani>,
    /// Whether the household is in the key group, or has a member in it.
    pub in_key: Vec<bool>,
}

/// The latest payment of imputed rent (spec economy/imputed-rent, step 2).
#[cfg(feature = "revenue")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImputedRentPayment {
    /// The day it was paid; `None` before the first.
    pub day: Option<Day>,
    /// Households' net spending on `real_estate` in the month paid for, with
    /// weights.
    pub spending: Bani,
    /// `θ` times that spending.
    pub due: Bani,
    /// `hh_imputed_rent_key`, fixed at load.
    pub key: Vec<Bani>,
    /// What each household received, per record (before its weight).
    pub received: Vec<Bani>,
}

#[cfg(feature = "investment")]
impl World {
    /// The closed months of investment, oldest first.
    #[must_use]
    pub fn investment_months(&self) -> &[InvestmentMonth] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// Each unit's latest investment budget.
    #[must_use]
    pub fn investment_plans(&self) -> &InvestmentPlans {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}

#[cfg(feature = "revenue")]
impl World {
    /// The latest day's taxes on purchases and the EU's payments to units.
    #[must_use]
    pub fn revenue_today(&self) -> &RevenueDay {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The closed months of revenue, oldest first.
    #[must_use]
    pub fn revenue_months(&self) -> &[RevenueMonth] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The held benefits of the latest pay day, in the spec's order.
    #[must_use]
    pub fn held_benefits(&self) -> &[HeldBenefit] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The latest payment of imputed rent.
    #[must_use]
    pub fn imputed_rent(&self) -> &ImputedRentPayment {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// Each industry's unit cost today, millionths of a ban per real unit
    /// (spec economy/prices-inflation, step 1).
    #[must_use]
    pub fn unit_costs_ppm(&self) -> Vec<u32> {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}

/// A month's EU payment (spec economy/eu-funds, step 1, AC-EUF-01): the
/// year's amount split into twelve by budget-2021's rule, times the exchange
/// rate index `rate_index_ppm` / 1,000,000, rounded half away from zero.
/// `month` is 0 to 11, counted from the start month, as in
/// `econ_io::budget::Budget::monthly`.
#[cfg(feature = "revenue")]
#[must_use]
pub fn eu_month(per_year: Bani, month: u8, rate_index_ppm: u32) -> Bani {
    let _ = (per_year, month, rate_index_ppm);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}
