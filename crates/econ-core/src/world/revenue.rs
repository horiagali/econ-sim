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
//! **Small revenue lines** (part 3): at the month end a twelfth of each
//! 2021 amount from its payers. **Other cash benefits** (part 4): on the 15th
//! (as imputed rent, so that a pay day's income stays the pay slips', the
//! owner's decision of 2026-10-21 applied here too) a twelfth of each
//! programme to its key group.
//!
//! > **Simplification (M3, until the inactivity and enrolment stage is in
//! > the scenario):** disability benefits go to persons whose activity is
//! > other inactive, education grants to pupils and students.

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_popgen::NO_EMPLOYER;
use econ_types::Bani;

use super::money::{Postings, bani, ratio};
use super::{StartState, World};

/// A million lei, in bani.
const M_LEI: i64 = 100_000_000;
/// Part 4's programmes: code, yearly amount (bani) and key.
const BENEFITS: [(FlowCode, i64, Key); 7] = [
    (
        FlowCode::BENEFIT_DISABILITY,
        71_388 * M_LEI / 10,
        Key::InactiveOther,
    ),
    (
        FlowCode::BENEFIT_DISABILITY,
        30_595 * M_LEI / 10,
        Key::DeclaredWages,
    ),
    (FlowCode::BENEFIT_FAMILY, 7_736 * M_LEI, Key::ChildrenUnder2),
    (
        FlowCode::BENEFIT_SOCIAL_ASSISTANCE,
        1_107 * M_LEI,
        Key::PoorestFifth,
    ),
    (
        FlowCode::BENEFIT_HOUSING,
        687 * M_LEI / 10,
        Key::PoorestFifth,
    ),
    (FlowCode::BENEFIT_OTHER, 15_206 * M_LEI / 10, Key::Aged65),
    (
        FlowCode::BENEFIT_STUDENT,
        29_263 * M_LEI / 10,
        Key::InEducation,
    ),
];

/// Who a held benefit goes to.
#[derive(Debug, Clone, Copy)]
enum Key {
    InactiveOther,
    DeclaredWages,
    ChildrenUnder2,
    PoorestFifth,
    Aged65,
    InEducation,
}

/// `activity` codes of stage C.
const PUPIL: u8 = 1;
const STUDENT: u8 = 2;
const INACTIVE_OTHER: u8 = 6;
/// The extraction industries that pay royalties.
const EXTRACTION: [&str; 5] = [
    "crude_oil",
    "natural_gas",
    "coal_mining",
    "metal_ores",
    "quarrying",
];

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
    /// Each person's activity (stage C).
    activity: Vec<u8>,
    /// Firm units' keys: starting value added, extraction output.
    va_key: Vec<i128>,
    royalty_key: Vec<i128>,
}

impl World {
    /// At load: the real-estate industry and the owners' keys.
    pub(super) fn start_revenue(&mut self, s: &StartState) {
        let (tenure, dwelling) = (&s.hh_tenure, &s.hh_dwelling_value);
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
        r.product_tax_ppm.clone_from(&s.product_tax_ppm);
        r.production_tax_ppm.clone_from(&s.production_tax_ppm);
        r.investment_tax_ppm = s.investment_product_tax_ppm;
        r.output_month = vec![0; n_u];
        r.activity.clone_from(&s.activity);
        r.va_key = s
            .unit_value_added
            .iter()
            .map(|b| i128::from(b.get()).max(0))
            .collect();
        let ids = &self.industries.id;
        r.royalty_key = (0..n_u)
            .map(|u| {
                let j = usize::from(self.units.industry[u]);
                if EXTRACTION.contains(&ids[j].as_str()) {
                    i128::from(s.unit_output[u].get()).max(0)
                } else {
                    0
                }
            })
            .collect();
    }

    /// A twelfth of a yearly amount for the month of the year `k` (0 to 11):
    /// the remainder to the first months (budget-2021's rule).
    fn twelfth(year: i64, k: u32) -> i128 {
        let (q, r) = (year / 12, year % 12);
        i128::from(q + i64::from(i64::from(k) < r))
    }

    /// Each household's key for a held benefit, now.
    fn benefit_key(&self, key: Key) -> Vec<i128> {
        let n_h = self.households.hh_weight.len();
        let mut k = vec![0i128; n_h];
        match key {
            Key::PoorestFifth => {
                for h in 0..n_h {
                    k[h] = i128::from(self.households.hh_fifth[h] == 0);
                }
            }
            _ => {
                let p = &self.persons;
                for i in 0..p.household_id.len() {
                    let h = p.household_id[i] as usize;
                    let age = self.pp.age[i];
                    k[h] += match key {
                        Key::InactiveOther => {
                            i128::from(self.rev.activity.get(i) == Some(&INACTIVE_OTHER))
                        }
                        Key::DeclaredWages => {
                            if p.employer[i] == NO_EMPLOYER {
                                0
                            } else {
                                i128::from(p.wage_declared[i].get()).max(0)
                            }
                        }
                        Key::ChildrenUnder2 => i128::from(age < 24),
                        Key::Aged65 => i128::from(age >= 65 * 12),
                        Key::InEducation => {
                            i128::from(matches!(self.rev.activity.get(i), Some(&PUPIL | &STUDENT)))
                        }
                        Key::PoorestFifth => 0,
                    };
                }
            }
        }
        k
    }

    /// Part 4 on the 15th: a twelfth of each held programme to its group.
    fn pay_held_benefits(&mut self) {
        let k = u32::try_from(self.months.len() % 12).expect("a month");
        for (code, year, key) in BENEFITS {
            let due = Self::twelfth(year, k);
            let keys = self.benefit_key(key);
            let Some(parts) =
                super::treasury::weighted_parts(due, &self.households.hh_weight, &keys)
            else {
                continue;
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
            let mut post = Postings::default();
            post.gov_pays(Sector::Households, bani(paid), code);
            post.commit(&mut self.ledger);
        }
    }

    /// Households pay `total` by `keys`, exactly in whole bani per record,
    /// none more than its money; returns what they paid.
    fn households_pay(&mut self, total: i128, keys: &[i128]) -> i128 {
        let Some(parts) = super::treasury::weighted_parts(total, &self.households.hh_weight, keys)
        else {
            return 0;
        };
        let mut paid = 0i128;
        for (h, &a) in parts.iter().enumerate() {
            let hh = &mut self.households;
            let x = a.min(i128::from(hh.hh_deposits[h].get()).max(0));
            if x > 0 {
                hh.hh_deposits[h] -= bani(x);
                paid += i128::from(hh.hh_weight[h]) * x;
            }
        }
        paid
    }

    /// Firm units pay `total` by `keys` (a unit short of money borrows at
    /// the day's checks).
    fn units_pay(&mut self, total: i128, keys: &[i128]) -> i128 {
        let parts = super::money::split(total, keys);
        let paid: i128 = parts.iter().sum();
        for (u, &p) in parts.iter().enumerate() {
            self.units.deposits[u] -= bani(p);
        }
        paid
    }

    /// Part 3 at the month end: a twelfth of each small revenue line.
    fn pay_held_revenue(&mut self) {
        let k = u32::try_from(self.months.len().saturating_sub(1) % 12).expect("a month");
        let n_h = self.households.hh_weight.len();
        let owners: Vec<i128> = self
            .rev
            .rent_key
            .iter()
            .map(|&x| i128::from(x > 0))
            .collect();
        let income: Vec<i128> = (0..n_h)
            .map(|h| i128::from(self.hh.income_last_month[h].get()).max(0))
            .collect();
        let equal = vec![1i128; n_h];
        let va = self.rev.va_key.clone();
        let royalty = self.rev.royalty_key.clone();
        let mut post = Postings::default();
        let property = self.households_pay(Self::twelfth(38_641 * M_LEI / 10, k), &owners);
        post.pays_gov(Sector::Households, bani(property), FlowCode::TAX_PROPERTY);
        let gambling = self.households_pay(Self::twelfth(23_763 * M_LEI / 10, k), &income);
        post.pays_gov(Sector::Households, bani(gambling), FlowCode::TAX_GAMBLING);
        let other = Self::twelfth(64_187 * M_LEI / 10, k);
        let other_hh = self.households_pay(other / 2, &equal);
        post.pays_gov(
            Sector::Households,
            bani(other_hh),
            FlowCode::TRANSFER_OTHER_RECEIVED,
        );
        let other_f = self.units_pay(other - other / 2, &va);
        post.pays_gov(
            Sector::Firms,
            bani(other_f),
            FlowCode::TRANSFER_OTHER_RECEIVED,
        );
        let capital = self.units_pay(Self::twelfth(48_572 * M_LEI / 10, k), &va);
        post.pays_gov(
            Sector::Firms,
            bani(capital),
            FlowCode::TRANSFER_CAPITAL_RECEIVED,
        );
        let royalties = self.units_pay(Self::twelfth(26_196 * M_LEI / 10, k), &royalty);
        post.pays_gov(
            Sector::Firms,
            bani(royalties),
            FlowCode::RENT_NATURAL_RESOURCES,
        );
        post.commit(&mut self.ledger);
        // Interest received: the banks pay it from their reserves.
        let interest = Self::twelfth(12_871 * M_LEI / 10, k);
        let mut post = Postings::default();
        post.leg(
            Sector::Banks,
            Sector::CentralBank,
            Instrument::Reserves,
            bani(interest),
            FlowCode::INTEREST_GOV_RECEIVED,
        );
        post.leg(
            Sector::CentralBank,
            Sector::Government,
            Instrument::GovDepositsAtCb,
            bani(interest),
            FlowCode::INTEREST_GOV_RECEIVED,
        );
        post.commit(&mut self.ledger);
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
            if !self.months.is_empty() {
                self.pay_held_benefits();
            }
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
        self.pay_held_revenue();
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
