//! The rest of 2021's revenue, benefits and imputed rent on the `World`
//! (feature `revenue`; specs `economy/imputed-rent`, `economy/excise`,
//! `economy/government-revenue-2021`, `economy/eu-funds` "First version",
//! roadmap M3, pulled forward by the owner on 2026-10-21). Built before its
//! acceptance tests by the owner's decision; they come from the specs.
//!
//! It acts under a law with the income taxes of 2021, as the treasury does.
//!
//! **Imputed rent** (spec `economy/imputed-rent`): households buy
//! `real_estate` through their basket as before; each day their net spending
//! on it, and each real-estate unit's part of it, is recorded. On the 15th
//! (restated from pay day by the owner on 2026-10-21, so that a pay day's
//! income stays what the pay slips show, AC-ST-05), last month's
//! `IMPUTED_SHARE_PPM` of that spending is paid back by the units, each by
//! its part, to owner households by the value of their dwellings, as income
//! (`income.imputed_rent`). It is left out of the units' profit and taxable
//! profit.
//!
//! **The product taxes inside firms' prices** (spec
//! `economy/government-revenue-2021`, part 1): each day a unit pays its
//! industry's rate on its input purchases, home and imported, and
//! `investment_product_tax_ppm` on its investment purchases, to the
//! government under `tax.products`; they are an input cost. **Other taxes on
//! production** (part 2): at the monthly close each unit pays its
//! industry's rate on the month's value of output under `tax.production`.

use econ_ledger::{FlowCode, Sector};
use econ_types::Bani;

use super::World;
use super::money::{Postings, bani, ratio};

/// θ: the share of households' real-estate spending that is owner-occupiers'
/// imputed rent less the industry's costs on it (spec, "Starting state").
const IMPUTED_SHARE_PPM: i128 = 836_500;
/// The day of the month imputed rent is paid back.
const PAYBACK_DAY: u8 = 15;
/// `hh_tenure` of an owner household (stage E).
const OWNER: u8 = 0;

/// What the `World` keeps for the revenue of 2021.
#[derive(Debug, Clone, Default)]
pub(super) struct Revenue {
    active: bool,
    /// The real-estate industry's catalogue index.
    real_estate: Option<usize>,
    /// Each owner household's key (its dwelling value); zero for others.
    rent_key: Vec<i128>,
    /// Each unit's net household spending on real estate this month and last.
    rent_month: Vec<i128>,
    rent_last: Vec<i128>,
    /// Imputed rent paid back on the latest day, national bani.
    pub(super) imputed_today: Bani,
    /// Each industry's product-tax rate on inputs, its production-tax rate,
    /// and the rate on investment, millionths.
    product_tax_ppm: Vec<u32>,
    production_tax_ppm: Vec<u32>,
    investment_tax_ppm: u32,
    /// Product taxes charged today, not yet posted.
    products_today: i128,
    /// Each unit's value of output this month.
    output_month: Vec<i128>,
}

impl World {
    /// At load: the real-estate industry and the owners' keys.
    pub(super) fn start_revenue(
        &mut self,
        tenure: &[u8],
        dwelling: &[Bani],
        product_tax_ppm: &[u32],
        production_tax_ppm: &[u32],
        investment_tax_ppm: u32,
    ) {
        let n_h = self.households.hh_weight.len();
        let active = self.law.income.is_some() && tenure.len() == n_h;
        self.rev = Revenue {
            active,
            ..Revenue::default()
        };
        if !active {
            return;
        }
        let n_u = self.units.industry.len();
        let mut owners: Vec<i128> = (0..n_h)
            .filter(|&h| tenure[h] == OWNER && dwelling.get(h).is_some_and(|d| d.get() > 0))
            .map(|h| i128::from(dwelling[h].get()))
            .collect();
        owners.sort_unstable();
        let median = owners.get(owners.len() / 2).copied().unwrap_or(1);
        let r = &mut self.rev;
        r.real_estate = self.industries.id.iter().position(|id| id == "real_estate");
        r.rent_key = (0..n_h)
            .map(|h| {
                if tenure[h] != OWNER {
                    0
                } else {
                    dwelling
                        .get(h)
                        .map(|d| i128::from(d.get()))
                        .filter(|&d| d > 0)
                        .unwrap_or(median)
                }
            })
            .collect();
        r.rent_month = vec![0; n_u];
        r.rent_last = vec![0; n_u];
        r.product_tax_ppm = product_tax_ppm.to_vec();
        r.production_tax_ppm = production_tax_ppm.to_vec();
        r.investment_tax_ppm = investment_tax_ppm;
        r.output_month = vec![0; n_u];
    }

    /// A unit bought inputs worth `value` today: its product tax.
    pub(super) fn rev_input_purchase(&mut self, u: usize, value: i128) {
        if !self.rev.active || value <= 0 {
            return;
        }
        let j = usize::from(self.units.industry[u]);
        let rate = self.rev.product_tax_ppm.get(j).copied().unwrap_or(0);
        self.charge_product_tax(u, ratio(value, i128::from(rate), 1_000_000));
    }

    /// A unit paid `value` for investment goods today: its product tax.
    pub(super) fn rev_investment_purchase(&mut self, u: usize, value: i128) {
        if !self.rev.active || value <= 0 {
            return;
        }
        let rate = i128::from(self.rev.investment_tax_ppm);
        self.charge_product_tax(u, ratio(value, rate, 1_000_000));
    }

    fn charge_product_tax(&mut self, u: usize, tax: i128) {
        if tax <= 0 {
            return;
        }
        self.units.deposits[u] -= bani(tax);
        self.fs.add_inputs(u, tax);
        self.rev.products_today += tax;
    }

    /// The end of the goods market: the day's product taxes posted.
    pub(super) fn rev_post_products(&mut self) {
        if !self.rev.active {
            return;
        }
        let x = std::mem::take(&mut self.rev.products_today);
        let mut post = Postings::default();
        post.pays_gov(Sector::Firms, bani(x), FlowCode::TAX_PRODUCTS);
        post.commit(&mut self.ledger);
    }

    /// A unit's value of output today.
    pub(super) fn rev_output_value(&mut self, u: usize, value: i128) {
        if self.rev.active {
            self.rev.output_month[u] += value;
        }
    }

    /// A unit received `net` of households' spending on good `j` today.
    pub(super) fn rev_household_sale(&mut self, j: usize, u: usize, net: i128) {
        if self.rev.active && self.rev.real_estate == Some(j) {
            self.rev.rent_month[u] += net;
        }
    }

    /// The start of a day: the 15th pays last month's imputed rent back.
    pub(super) fn rev_begin_day(&mut self) {
        if !self.rev.active {
            return;
        }
        self.rev.imputed_today = Bani::ZERO;
        if self.calendar.date(self.today).day == PAYBACK_DAY {
            self.pay_imputed_rent();
        }
    }

    /// Monthly close, before profit: the other taxes on production; this
    /// month's real-estate spending becomes what the 15th pays from.
    pub(super) fn rev_close(&mut self) {
        if !self.rev.active {
            return;
        }
        let mut total = 0i128;
        for u in 0..self.units.industry.len() {
            let value = std::mem::take(&mut self.rev.output_month[u]);
            let j = usize::from(self.units.industry[u]);
            let rate = i128::from(self.rev.production_tax_ppm.get(j).copied().unwrap_or(0));
            let tax = ratio(value.max(0), rate, 1_000_000);
            if tax > 0 {
                self.units.deposits[u] -= bani(tax);
                self.fs.add_inputs(u, tax);
                total += tax;
            }
        }
        let mut post = Postings::default();
        post.pays_gov(Sector::Firms, bani(total), FlowCode::TAX_PRODUCTION);
        post.commit(&mut self.ledger);
        let r = &mut self.rev;
        r.rent_last = std::mem::replace(&mut r.rent_month, vec![0; r.rent_last.len()]);
    }

    /// Imputed rent back to owner households, exactly in whole bani per
    /// record; the units pay by their part of last month's spending.
    fn pay_imputed_rent(&mut self) {
        let spent: i128 = self.rev.rent_last.iter().sum();
        let due = ratio(spent, IMPUTED_SHARE_PPM, 1_000_000);
        if due <= 0 {
            return;
        }
        let Some(parts) =
            super::treasury::weighted_parts(due, &self.households.hh_weight, &self.rev.rent_key)
        else {
            return;
        };
        let mut paid = 0i128;
        for (h, &a) in parts.iter().enumerate() {
            if a == 0 {
                continue;
            }
            let x = bani(a);
            let hh = &mut self.households;
            hh.hh_deposits[h] += x;
            hh.hh_income_received[h] += x;
            hh.income_to_date[h] += x;
            self.hh.income_this_month[h] += x;
            paid += i128::from(hh.hh_weight[h]) * a;
        }
        let by_unit = super::money::split(paid, &self.rev.rent_last);
        for (u, &p) in by_unit.iter().enumerate() {
            if p != 0 {
                self.units.deposits[u] -= bani(p);
                // Left out of profit and taxable profit, as a distribution.
                self.fs.add_mixed(u, p);
            }
        }
        let mut post = Postings::default();
        post.deposits(
            Sector::Firms,
            Sector::Households,
            bani(paid),
            FlowCode::INCOME_IMPUTED_RENT,
        );
        post.commit(&mut self.ledger);
        self.rev.imputed_today = bani(paid);
    }

    /// The revenue's state for the state hash.
    pub(super) fn rev_hash(&self, write: &mut impl FnMut(u64)) {
        if !self.rev.active {
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        for v in &self.rev.output_month {
            write(*v as u64);
        }
        #[allow(clippy::cast_sign_loss)]
        for (m, l) in self.rev.rent_month.iter().zip(&self.rev.rent_last) {
            write(*m as u64 ^ (*l as u64).rotate_left(31));
        }
    }
}
