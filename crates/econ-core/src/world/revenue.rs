//! The rest of 2021's revenue, benefits and imputed rent on the `World`
//! (feature `revenue`; specs `economy/imputed-rent`, `economy/excise`,
//! `economy/government-revenue-2021`, `economy/eu-funds` "First version",
//! roadmap M3, pulled forward by the owner on 2026-10-21). Built before its
//! acceptance tests by the owner's decision; the views the tests read are
//! declared in `observe.rs`.
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
//! (`income.imputed_rent`), each its share rounded down to the ban (one more
//! ban to the largest remainders); what cannot be placed stays with the
//! units. It is left out of the units' profit and taxable profit.
//!
//! **The product taxes inside firms' prices** (spec
//! `economy/government-revenue-2021`, part 1): each day a unit pays its
//! industry's rate (net of its starting fuel duty, from the starting state)
//! on its input purchases, home and imported, and
//! `investment_product_tax_ppm` on its investment purchases, to the
//! government under `tax.products`; they are an input cost. **Other taxes on
//! production** (part 2): at the monthly close each unit pays its
//! industry's rate on the month's value of output under `tax.production`.
//! **Small revenue lines** (part 3): at the month end a twelfth of each
//! 2021 amount from its payers; what households cannot pay is forgiven and
//! reported. **Other cash benefits** (part 4): on the 15th (as imputed rent,
//! the owner's decision of 2026-10-21 applied here too) a twelfth of each
//! programme to its key group.
//!
//! **EU funds** (spec `economy/eu-funds`, M3): at each month end the rest of
//! the world pays a twelfth of 2021's grants, held in euro at the day's
//! exchange rate: current and capital grants to the government, the farm
//! direct payments straight to the crop and livestock units by their
//! starting output.
//!
//! **Excise** (spec `economy/excise`): households pay each excised good's
//! duty per unit on top of the price, with VAT on the duty; non-exempt firm
//! units pay it on their fuel, home and imported; it is posted at the end of
//! the goods market under `tax.excise`, and indexed each 1 January by the
//! twelve-month inflation (never down). The consumer price is
//! `(p + d)(1 + VAT)`, scaled so that the starting CPI is one.
//!
//! **The tax levers** (spec `game/lever-system`, M4): the duties are the
//! law's `excise_duty[g]` (a change replaces the indexed duty); under the law
//! of `law.json` the property tax is `property_tax_rate` on each owner's
//! dwelling value instead of 2021's fixed total, and the wealth tax
//! (`wealth_tax_rate` above `wealth_tax_threshold`, none in 2021) is paid at
//! the month end.
//!
//! > **Simplification (M3):** the fuel duty stays inside firms' "other costs"
//! > (where the input-output table put it), so starting prices do not move
//! > and a change of the duty does not reach producer prices until the
//! > levers (M4, AC-EXC-09).
//!
//! > **Simplification (M3, until the inactivity and enrolment stage is in
//! > the scenario):** disability benefits go to persons whose activity is
//! > other inactive, education grants to pupils and students.

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_popgen::NO_EMPLOYER;
use econ_types::{Bani, Day};

use super::money::{Postings, bani, ratio};
use super::observe::{
    HeldBenefit, HeldPayment, ImputedRentPayment, RevenueDay, RevenueMonth, eu_month,
};
use super::{RevenuePart, StartState, World};

/// A million lei, in bani.
const M_LEI: i64 = 100_000_000;
/// Part 4's programmes: name, code, yearly amount (bani) and key.
const BENEFITS: [(&str, FlowCode, i64, Key); 7] = [
    (
        "sickness_disability.disability",
        FlowCode::BENEFIT_DISABILITY,
        71_388 * M_LEI / 10,
        Key::InactiveOther,
    ),
    (
        "sickness_disability.sickness",
        FlowCode::BENEFIT_DISABILITY,
        30_595 * M_LEI / 10,
        Key::DeclaredWages,
    ),
    (
        "family_children",
        FlowCode::BENEFIT_FAMILY,
        7_736 * M_LEI,
        Key::ChildrenUnder2,
    ),
    (
        "social_exclusion",
        FlowCode::BENEFIT_SOCIAL_ASSISTANCE,
        1_107 * M_LEI,
        Key::PoorestFifth,
    ),
    (
        "housing",
        FlowCode::BENEFIT_HOUSING,
        687 * M_LEI / 10,
        Key::PoorestFifth,
    ),
    (
        "social_protection_other",
        FlowCode::BENEFIT_OTHER,
        15_206 * M_LEI / 10,
        Key::Aged65,
    ),
    (
        "education_grants",
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
/// The industries whose fuel pays no duty (feedstock, jet fuel).
const FUEL_EXEMPT: [&str; 4] = [
    "refined_petroleum",
    "chemicals",
    "fertilisers",
    "air_transport",
];

/// EU receipts a year, million lei of 2021 (spec economy/eu-funds): current
/// grants, capital grants, the farm direct payments.
const EU_CURRENT: i64 = 65_502 * M_LEI / 10;
const EU_CAPITAL: i64 = 160_175 * M_LEI / 10;
const EU_AGRI: i64 = 9_806 * M_LEI;

/// Part 3's lines a year, in the spec's order.
const PROPERTY: i64 = 38_641 * M_LEI / 10;
const GAMBLING: i64 = 23_763 * M_LEI / 10;
const ROYALTIES: i64 = 26_196 * M_LEI / 10;
const INTEREST: i64 = 12_871 * M_LEI / 10;
const OTHER_CURRENT: i64 = 64_187 * M_LEI / 10;
const OTHER_CAPITAL: i64 = 48_572 * M_LEI / 10;

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
/// Households' purchases of `real_estate` over its output in the
/// input-output table (92.1 of 117.8 billion lei; research note
/// imputed-rent-romania-2021).
const HH_REAL_ESTATE_PPM: i128 = 781_834;
/// The farm units' industries: those of A01 (spec economy/eu-funds).
const FARM: [&str; 6] = [
    "agri_cereals",
    "agri_oilseeds",
    "agri_fruit_veg",
    "agri_vineyards",
    "livestock_meat",
    "livestock_dairy",
];

/// The day of the month imputed rent and the held benefits are paid.
const PAYBACK_DAY: u8 = 15;
/// `hh_tenure` of an owner household (stage E).
const OWNER: u8 = 0;

/// What the `World` keeps for the revenue of 2021.
#[derive(Debug, Clone, Default)]
pub(super) struct Revenue {
    active: bool,
    /// The parts a start edit leaves out.
    no_excise: bool,
    no_product_taxes: bool,
    no_imputed_rent: bool,
    /// The real-estate industry's catalogue index.
    real_estate: Option<usize>,
    /// Each owner household's key (its dwelling value); zero for others.
    rent_key: Vec<i128>,
    /// Each household's dwelling value (zero when it does not own its home),
    /// for its net worth.
    dwelling: Vec<i128>,
    /// Each unit's net household spending on real estate this month and last.
    rent_month: Vec<i128>,
    rent_last: Vec<i128>,
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
    /// Each industry's excise duty, millionths of the starting price per
    /// unit; whether its fuel is exempt; the fuel industry; each industry's
    /// imported fuel over its imported inputs.
    duty_ppm: Vec<u32>,
    fuel_exempt: Vec<bool>,
    fuel: Option<usize>,
    fuel_import_ppm: Vec<u32>,
    /// Excise charged today, not yet posted; this month's by good.
    excise_today: i128,
    excise_month: Vec<i128>,
    /// The consumer prices' scale, millionths: one over the starting CPI of
    /// `(p + d)(1 + VAT)`.
    cpi_scale_ppm: i128,
    /// The farm units' key to the EU's direct payments (starting output).
    farm_key: Vec<i128>,
    /// The views of `observe.rs`.
    day: RevenueDay,
    months: Vec<RevenueMonth>,
    held: Vec<HeldBenefit>,
    rent: ImputedRentPayment,
}

/// Per-record amounts `p[h] ≥ 0`, each `total · key[h] / Σ w · key` rounded
/// down, one more ban to the records with the largest remainders while the
/// total allows; returns the parts and what they add up to with weights
/// (at most `total`).
fn share_out(total: i128, weights: &[u32], keys: &[i128]) -> (Vec<i128>, i128) {
    let n = weights.len();
    let sum: i128 = (0..n)
        .map(|h| i128::from(weights[h]) * keys[h].max(0))
        .sum();
    let mut parts = vec![0i128; n];
    if total <= 0 || sum <= 0 {
        return (parts, 0);
    }
    let mut rems: Vec<(i128, usize)> = Vec::new();
    let mut paid = 0i128;
    for h in 0..n {
        if keys[h] <= 0 {
            continue;
        }
        let x = total * keys[h];
        parts[h] = x / sum;
        rems.push((x % sum, h));
        paid += i128::from(weights[h]) * parts[h];
    }
    rems.sort_by_key(|&(r, h)| (std::cmp::Reverse(r), h));
    for (_, h) in rems {
        let w = i128::from(weights[h]);
        if paid + w <= total {
            parts[h] += 1;
            paid += w;
        }
    }
    (parts, paid)
}

impl World {
    /// At load: the real-estate industry and the owners' keys, the rates,
    /// the duties, the parts a start edit leaves out.
    pub(super) fn start_revenue(&mut self, s: &StartState, without: &[RevenuePart]) {
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
        r.no_excise = without.contains(&RevenuePart::Excise);
        r.no_product_taxes = without.contains(&RevenuePart::ProductTaxes);
        r.no_imputed_rent = without.contains(&RevenuePart::ImputedRent);
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
        r.dwelling = (0..n_h)
            .map(|h| dwelling.get(h).map_or(0, |d| i128::from(d.get()).max(0)))
            .collect();
        r.rent_month = vec![0; n_u];
        r.rent_last = vec![0; n_u];
        let n_i = self.industries.id.len();
        if r.no_product_taxes {
            r.product_tax_ppm = vec![0; n_i];
            r.production_tax_ppm = vec![0; n_i];
            r.investment_tax_ppm = 0;
        } else {
            r.product_tax_ppm.clone_from(&s.product_tax_ppm);
            r.production_tax_ppm.clone_from(&s.production_tax_ppm);
            r.investment_tax_ppm = s.investment_product_tax_ppm;
        }
        r.output_month = vec![0; n_u];
        r.activity.clone_from(&s.activity);
        r.va_key = s
            .unit_value_added
            .iter()
            .map(|b| i128::from(b.get()).max(0))
            .collect();
        r.fuel_import_ppm.clone_from(&s.fuel_import_ppm);
        let ids = &self.industries.id;
        // `excise_duty[g]` of the law in force.
        r.duty_ppm = (0..n_i)
            .map(|j| {
                if r.no_excise {
                    return 0;
                }
                self.law.excise_duty_ppm.get(j).copied().unwrap_or(0)
            })
            .collect();
        r.fuel_exempt = (0..n_i)
            .map(|j| FUEL_EXEMPT.contains(&ids[j].as_str()))
            .collect();
        r.fuel = ids.iter().position(|id| id == "refined_petroleum");
        r.farm_key = (0..n_u)
            .map(|u| {
                let id = &ids[usize::from(self.units.industry[u])];
                if FARM.contains(&id.as_str()) {
                    i128::from(s.unit_output[u].get()).max(0)
                } else {
                    0
                }
            })
            .collect();
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
        r.excise_month = vec![0; n_i];
        r.rent.key = r.rent_key.iter().map(|&k| bani(k)).collect();
        r.rent.received = vec![Bani::ZERO; n_h];
        if !r.no_imputed_rent {
            self.drop_imputed_from_mixed(s);
        }
        self.reset_day();
        self.start_excise();
    }

    /// The payback gives owner-occupiers' imputed rent to owner households,
    /// so the real-estate units' weight for paying mixed income leaves out
    /// that part of their value added: θ times households' share of their
    /// output (roadmap M7, "Real estate's mixed income and surplus").
    fn drop_imputed_from_mixed(&mut self, s: &StartState) {
        let Some(j) = self.rev.real_estate else {
            return;
        };
        for u in 0..self.units.industry.len() {
            if usize::from(self.units.industry[u]) != j {
                continue;
            }
            let output = i128::from(s.unit_output[u].get()).max(0);
            let imputed = ratio(
                ratio(output, HH_REAL_ESTATE_PPM, 1_000_000),
                IMPUTED_SHARE_PPM,
                1_000_000,
            );
            let w = &mut self.uu.mixed_weight[u];
            *w = u64::try_from((i128::from(*w) - imputed).max(0)).expect("non-negative");
        }
    }

    /// Excise at load: households' basket shares of the excised goods times
    /// one plus the duty, renormalised (their real demand kept); the
    /// consumer prices' scale.
    fn start_excise(&mut self) {
        let n_i = self.industries.id.len();
        let shares: Vec<i128> = (0..n_i)
            .map(|j| {
                i128::from(self.industries.basket_share_ppm[j])
                    * (1_000_000 + i128::from(self.rev.duty_ppm[j]))
            })
            .collect();
        let renorm = super::money::split(1_000_000, &shares);
        for j in 0..n_i {
            self.industries.basket_share_ppm[j] = u32::try_from(renorm[j]).expect("a share");
        }
        self.rev.cpi_scale_ppm = 1_000_000;
        let raw: i128 = (0..n_i)
            .map(|j| {
                i128::from(self.industries.basket_share_ppm[j])
                    * self.raw_consumer_price(j, 1_000_000)
            })
            .sum::<i128>()
            / 1_000_000;
        if raw > 0 {
            self.rev.cpi_scale_ppm = 1_000_000_000_000 / raw;
        }
        for j in 0..n_i {
            let p = self.industries.price_ppm[j];
            self.industries.consumer_price_ppm[j] = self.rev_consumer_price(j, p);
        }
    }

    /// `(p + d)(1 + VAT)` of good `j` at price `p`, millionths, unscaled.
    fn raw_consumer_price(&self, j: usize, p: u32) -> i128 {
        let vat = i128::from(self.rate_ppm(j).unwrap_or(0));
        (i128::from(p) + self.excise_duty_ppm(j)) * (1_000_000 + vat) / 1_000_000
    }

    /// Good `j`'s consumer price at producer price `p` (the producer price
    /// when the revenue does not act).
    pub(super) fn rev_consumer_price(&self, j: usize, p: u32) -> u32 {
        if !self.rev.active {
            return p;
        }
        let c = self.raw_consumer_price(j, p) * self.rev.cpi_scale_ppm / 1_000_000;
        u32::try_from(c).unwrap_or(u32::MAX)
    }

    /// A consumer price from its parts: producer price `p`, duty `d` and
    /// VAT paid `vat_ppm` (the formula of [`Self::rev_consumer_price`]).
    #[cfg(feature = "levers")]
    pub(super) fn rev_consumer_price_parts(&self, p: u32, d: i128, vat_ppm: u32) -> u32 {
        if !self.rev.active {
            return p;
        }
        let raw = (i128::from(p) + d) * (1_000_000 + i128::from(vat_ppm)) / 1_000_000;
        u32::try_from(raw * self.rev.cpi_scale_ppm / 1_000_000).unwrap_or(u32::MAX)
    }

    /// Good `j`'s excise duty per unit now, millionths of the starting price
    /// (0 when the revenue does not act).
    pub(super) fn excise_duty_ppm(&self, j: usize) -> i128 {
        if !self.rev.active {
            return 0;
        }
        i128::from(self.rev.duty_ppm.get(j).copied().unwrap_or(0))
    }

    /// The day's views emptied.
    fn reset_day(&mut self) {
        let (n_i, n_u) = (self.industries.id.len(), self.units.industry.len());
        let d = &mut self.rev.day;
        d.excise_duty_ppm.clone_from(&self.rev.duty_ppm);
        d.hh_quantity = vec![0; n_i];
        d.hh_excise = vec![Bani::ZERO; n_i];
        for v in [
            &mut d.unit_excise,
            &mut d.unit_inputs,
            &mut d.unit_investment,
            &mut d.unit_product_tax,
            &mut d.unit_eu_agri,
        ] {
            *v = vec![Bani::ZERO; n_u];
        }
        d.unit_fuel_quantity = vec![0; n_u];
    }

    /// Households bought `quantity` real units of good `j` today, home and
    /// imported, and paid `duty` of excise on them.
    pub(super) fn rev_household_good(&mut self, j: usize, quantity: i128, duty: i128) {
        if !self.rev.active {
            return;
        }
        let d = &mut self.rev.day;
        d.hh_quantity[j] += i64::try_from(quantity).unwrap_or(i64::MAX);
        d.hh_excise[j] += bani(duty);
        self.rev.excise_month[j] += duty;
        self.rev.excise_today += duty;
    }

    /// A unit bought `quantity` real units of good `i` today: the fuel duty.
    pub(super) fn rev_fuel_purchase(&mut self, u: usize, i: usize, quantity: i64) {
        if !self.rev.active || self.rev.fuel != Some(i) || quantity <= 0 {
            return;
        }
        self.rev.day.unit_fuel_quantity[u] += quantity;
        let j = usize::from(self.units.industry[u]);
        if self.rev.fuel_exempt[j] {
            return;
        }
        let duty = ratio(i128::from(quantity), self.excise_duty_ppm(i), 1_000_000);
        self.charge_excise(u, i, duty);
    }

    /// A unit's imported inputs worth `value` today (at the day's import
    /// price): the duty on their fuel.
    pub(super) fn rev_imported_fuel(&mut self, u: usize, value: i128) {
        if !self.rev.active || value <= 0 {
            return;
        }
        let Some(f) = self.rev.fuel else {
            return;
        };
        let j = usize::from(self.units.industry[u]);
        let fuel_value = ratio(
            value,
            i128::from(self.rev.fuel_import_ppm.get(j).copied().unwrap_or(0)),
            1_000_000,
        );
        let quantity = ratio(fuel_value, 1_000_000, self.import_index().max(1));
        self.rev.day.unit_fuel_quantity[u] += i64::try_from(quantity).unwrap_or(i64::MAX);
        if self.rev.fuel_exempt[j] {
            return;
        }
        let duty = ratio(quantity, self.excise_duty_ppm(f), 1_000_000);
        self.charge_excise(u, f, duty);
    }

    fn charge_excise(&mut self, u: usize, good: usize, duty: i128) {
        if duty <= 0 {
            return;
        }
        self.units.deposits[u] -= bani(duty);
        self.fs.add_inputs(u, duty);
        self.rev.excise_today += duty;
        self.rev.excise_month[good] += duty;
        self.rev.day.unit_excise[u] += bani(duty);
    }

    /// A command changed the law: each good whose `excise_duty[g]` changed
    /// takes the new duty in place of its indexed one.
    #[cfg(feature = "levers")]
    pub(super) fn rev_law_changed(&mut self, old: &super::Law) {
        if !self.rev.active || self.rev.no_excise {
            return;
        }
        let new = &self.law.excise_duty_ppm;
        for j in 0..self.rev.duty_ppm.len() {
            let d = new.get(j).copied().unwrap_or(0);
            if old.excise_duty_ppm.get(j).copied().unwrap_or(0) != d {
                self.rev.duty_ppm[j] = d;
                if let Some(v) = self.rev.day.excise_duty_ppm.get_mut(j) {
                    *v = d;
                }
            }
        }
    }

    /// 1 January: every duty times one plus the twelve-month inflation of the
    /// last close, never down.
    fn index_excise(&mut self) {
        let Some(m) = self.months.last() else {
            return;
        };
        let pi = i128::from(m.inflation_ppm.max(0));
        for d in &mut self.rev.duty_ppm {
            *d =
                u32::try_from(ratio(i128::from(*d), 1_000_000 + pi, 1_000_000)).unwrap_or(u32::MAX);
        }
    }

    /// A twelfth of a yearly amount for the month of the year `k` (0 to 11):
    /// the remainder to the first months (budget-2021's rule).
    fn twelfth(year: i64, k: u32) -> i128 {
        let (q, r) = (year / 12, year % 12);
        i128::from(q + i64::from(i64::from(k) < r))
    }

    /// Each household's key for a held benefit, now: its members in the
    /// group (the employees' declared wages for sick pay, one for a
    /// household of the lowest fifth).
    fn benefit_key(&self, key: Key) -> Vec<i128> {
        let n_h = self.households.hh_weight.len();
        let mut k = vec![0i128; n_h];
        match key {
            Key::PoorestFifth => {
                let lowest = self.households.hh_fifth.iter().copied().min().unwrap_or(0);
                for h in 0..n_h {
                    k[h] = i128::from(self.households.hh_fifth[h] == lowest);
                }
            }
            _ => {
                let p = &self.persons;
                for i in 0..p.household_id.len() {
                    if !self.person_alive(i) {
                        continue;
                    }
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
        self.mask_ended(&mut k);
        k
    }

    /// Ended household records take no share of a payment by key.
    fn mask_ended(&self, keys: &mut [i128]) {
        for (h, k) in keys.iter_mut().enumerate() {
            if !self.household_alive(h) {
                *k = 0;
            }
        }
    }

    /// Part 4 on the 15th: a twelfth of each held programme to its group.
    fn pay_held_benefits(&mut self) {
        // The payment's index: the first 15th, before any month closes, is 0.
        let k = u32::try_from(self.months.len() % 12).expect("a month");
        let mut held = Vec::with_capacity(BENEFITS.len());
        for (name, code, year, key) in BENEFITS {
            let due = Self::twelfth(year, k);
            let keys = self.benefit_key(key);
            let (parts, paid) = share_out(due, &self.households.hh_weight, &keys);
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
            }
            let mut post = Postings::default();
            post.gov_pays(Sector::Households, bani(paid), code);
            post.commit(&mut self.ledger);
            held.push(HeldBenefit {
                programme: name.to_string(),
                code,
                due: bani(due),
                received: parts.iter().map(|&a| bani(a)).collect(),
                in_key: keys.iter().map(|&x| x > 0).collect(),
            });
        }
        self.rev.held = held;
    }

    /// Households pay `total` by `keys`, each its share rounded, none more
    /// than its deposits; returns each record's payment and the total paid.
    /// Equal keys are an equal amount per household, rounded down, the
    /// remainder falling short (every owner pays the same property tax).
    fn households_pay(&mut self, total: i128, keys: &[i128]) -> (Vec<i128>, i128) {
        let mut masked = keys.to_vec();
        self.mask_ended(&mut masked);
        let keys = &masked;
        let w = &self.households.hh_weight;
        let mut keyed = keys.iter().copied().filter(|&k| k > 0);
        let first = keyed.next().unwrap_or(0);
        let mut parts: Vec<i128> = if keyed.all(|k| k == first) {
            let count: i128 = (0..w.len())
                .filter(|&h| keys[h] > 0)
                .map(|h| i128::from(w[h]))
                .sum();
            let each = if count > 0 { total / count } else { 0 };
            keys.iter().map(|&k| if k > 0 { each } else { 0 }).collect()
        } else {
            share_out(total, w, keys).0
        };
        let mut paid = 0i128;
        for (h, a) in parts.iter_mut().enumerate() {
            let hh = &mut self.households;
            *a = (*a).min(i128::from(hh.hh_deposits[h].get()).max(0));
            if *a > 0 {
                hh.hh_deposits[h] -= bani(*a);
                hh.spending_to_date[h] += bani(*a);
                paid += i128::from(hh.hh_weight[h]) * *a;
            }
        }
        (parts, paid)
    }

    /// Firm units pay `total` by `keys`, exactly (a unit short of money
    /// borrows at the day's checks).
    /// The firm units pay `total` by `keys`, as far as the firm sector can
    /// with `post` pending; the rest is reported unpaid (the backstop).
    fn units_pay(&mut self, post: &Postings, total: i128, keys: &[i128]) -> i128 {
        let payable = self.firm_payable(post, total);
        self.note_unpaid(super::insolvency::Unpaid::Taxes, total - payable);
        let total = payable;
        let parts = super::money::split(total, keys);
        let paid: i128 = parts.iter().sum();
        for (u, &p) in parts.iter().enumerate() {
            self.units.deposits[u] -= bani(p);
        }
        paid
    }

    /// EU funds at the month end: a twelfth of each year's amount in euro, at
    /// the day's exchange rate. Returns what the EU paid.
    fn pay_eu_funds(&mut self, k: u8) -> i128 {
        #[cfg(feature = "trade")]
        let rate = self.trade_rate_index_ppm();
        #[cfg(not(feature = "trade"))]
        let rate = 1_000_000u32;
        let lei = |year: i64| i128::from(eu_month(Bani(year), k, rate).get());
        let (current, capital) = (lei(EU_CURRENT), lei(EU_CAPITAL));
        let mut post = Postings::default();
        // Through the banks' reserves as far as they hold them; beyond (the
        // leu so weak that a month's grants exceed them, never in a normal
        // run) the central bank takes the euros into its foreign assets and
        // credits the treasury directly (no lever may crash the World).
        let mut reserves = i128::from(
            self.ledger
                .balance(Sector::Banks, Instrument::Reserves)
                .get(),
        )
        .max(0);
        for (amount, code) in [
            (current, FlowCode::TRANSFER_EU_GRANTS_CURRENT),
            (capital, FlowCode::TRANSFER_EU_GRANTS_CAPITAL),
        ] {
            let via_banks = amount.min(reserves);
            reserves -= via_banks;
            post.pay_gov_any(Sector::RestOfWorld, bani(via_banks), code);
            let direct = bani(amount - via_banks);
            post.leg(
                Sector::RestOfWorld,
                Sector::CentralBank,
                Instrument::ForeignAssets,
                direct,
                code,
            );
            post.leg(
                Sector::CentralBank,
                Sector::Government,
                Instrument::GovDepositsAtCb,
                direct,
                code,
            );
        }
        let agri = lei(EU_AGRI);
        let parts = super::money::split(agri, &self.rev.farm_key);
        let mut paid = 0i128;
        for (u, &p) in parts.iter().enumerate() {
            if p > 0 {
                self.units.deposits[u] += bani(p);
                // Income of the unit: in its profit and taxable profit.
                self.fs.add_value(u, p);
                self.rev.day.unit_eu_agri[u] += bani(p);
                paid += p;
            }
        }
        post.deposits(
            Sector::RestOfWorld,
            Sector::Firms,
            bani(paid),
            FlowCode::SUBSIDY_EU_AGRI,
        );
        post.commit(&mut self.ledger);
        current + capital + paid
    }

    /// Part 3 at the month end: a twelfth of each small revenue line, in the
    /// spec's order; and each household's property tax.
    fn pay_held_revenue(&mut self, k: u32) -> (Vec<HeldPayment>, Vec<Bani>) {
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
        let mut held = Vec::with_capacity(6);
        let mut post = Postings::default();
        let line = |code, due: i128, paid: i128| HeldPayment {
            code,
            due: bani(due),
            paid: bani(paid),
            shortfall: bani(due - paid),
        };
        let (due, by_hh, property) = match self.law.property_tax_bp {
            Some(bp) => self.property_by_value(bp, k),
            None => {
                let due = Self::twelfth(PROPERTY, k);
                let (by_hh, paid) = self.households_pay(due, &owners);
                (due, by_hh, paid)
            }
        };
        post.pays_gov(Sector::Households, bani(property), FlowCode::TAX_PROPERTY);
        held.push(line(FlowCode::TAX_PROPERTY, due, property));
        let due = Self::twelfth(GAMBLING, k);
        let (_, gambling) = self.households_pay(due, &income);
        post.pays_gov(Sector::Households, bani(gambling), FlowCode::TAX_GAMBLING);
        held.push(line(FlowCode::TAX_GAMBLING, due, gambling));
        let due = Self::twelfth(ROYALTIES, k);
        let royalties = self.units_pay(&post, due, &royalty);
        post.pays_gov(
            Sector::Firms,
            bani(royalties),
            FlowCode::RENT_NATURAL_RESOURCES,
        );
        held.push(line(FlowCode::RENT_NATURAL_RESOURCES, due, royalties));
        // Interest received: the banks pay it from their reserves.
        let due = Self::twelfth(INTEREST, k);
        post.leg(
            Sector::Banks,
            Sector::CentralBank,
            Instrument::Reserves,
            bani(due),
            FlowCode::INTEREST_GOV_RECEIVED,
        );
        post.leg(
            Sector::CentralBank,
            Sector::Government,
            Instrument::GovDepositsAtCb,
            bani(due),
            FlowCode::INTEREST_GOV_RECEIVED,
        );
        held.push(line(FlowCode::INTEREST_GOV_RECEIVED, due, due));
        let due = Self::twelfth(OTHER_CURRENT, k);
        let (_, other_hh) = self.households_pay(due / 2, &equal);
        post.pays_gov(
            Sector::Households,
            bani(other_hh),
            FlowCode::TRANSFER_OTHER_RECEIVED,
        );
        let other_f = self.units_pay(&post, due - due / 2, &va);
        post.pays_gov(
            Sector::Firms,
            bani(other_f),
            FlowCode::TRANSFER_OTHER_RECEIVED,
        );
        held.push(line(
            FlowCode::TRANSFER_OTHER_RECEIVED,
            due,
            other_hh + other_f,
        ));
        let due = Self::twelfth(OTHER_CAPITAL, k);
        let capital = self.units_pay(&post, due, &va);
        post.pays_gov(
            Sector::Firms,
            bani(capital),
            FlowCode::TRANSFER_CAPITAL_RECEIVED,
        );
        held.push(line(FlowCode::TRANSFER_CAPITAL_RECEIVED, due, capital));
        self.bank_intraday_advance(&mut post);
        post.commit(&mut self.ledger);
        (held, by_hh.into_iter().map(bani).collect())
    }

    /// The property tax of month `k` of the year under `property_tax_rate`
    /// (`bp` a year): each owner household's yearly tax is the rate on its
    /// dwelling value (the median owner's when it has none recorded), paid a
    /// twelfth a month (the remainder in the first months); none pays more
    /// than its deposits, the rest falling short. Returns what was due in
    /// all, each household's payment and what was paid in all.
    fn property_by_value(&mut self, bp: u32, k: u32) -> (i128, Vec<i128>, i128) {
        let n_h = self.households.hh_weight.len();
        let mut parts = vec![0i128; n_h];
        let (mut due, mut paid) = (0i128, 0i128);
        for h in 0..n_h {
            let key = self.rev.rent_key[h];
            if key <= 0 {
                continue;
            }
            let year = ratio(key, i128::from(bp), 10_000);
            let a = year / 12 + i128::from(i128::from(k) < year % 12);
            let w = i128::from(self.households.hh_weight[h]);
            due += w * a;
            let hh = &mut self.households;
            let x = a.min(i128::from(hh.hh_deposits[h].get()).max(0));
            if x > 0 {
                hh.hh_deposits[h] -= bani(x);
                hh.spending_to_date[h] += bani(x);
                paid += w * x;
            }
            parts[h] = x;
        }
        (due, parts, paid)
    }

    /// The wealth tax at the month end: `wealth_tax_rate` a year on each
    /// household's net worth (cash, deposits and dwelling, less its loans)
    /// above `wealth_tax_threshold`, a twelfth a month, none paying more than
    /// its deposits (the rest is forgiven). Nothing when the rate is zero.
    fn pay_wealth_tax(&mut self) {
        let bp = i128::from(self.law.wealth_tax_bp);
        if bp <= 0 {
            return;
        }
        let threshold = i128::from(self.law.wealth_tax_threshold.get());
        let n_h = self.households.hh_weight.len();
        let mut paid = 0i128;
        for h in 0..n_h {
            let hh = &self.households;
            let mut worth = i128::from(hh.hh_cash[h].get())
                + i128::from(hh.hh_deposits[h].get())
                + self.rev.dwelling[h];
            #[cfg(feature = "banking")]
            {
                let b = self.bank();
                worth -= b.hh_mortgage.get(h).map_or(0, |x| i128::from(x.get()));
                worth -= b.hh_consumer_debt.get(h).map_or(0, |x| i128::from(x.get()));
            }
            let base = worth - threshold;
            if base <= 0 {
                continue;
            }
            let a = ratio(base, bp, 120_000);
            let hh = &mut self.households;
            let x = a.min(i128::from(hh.hh_deposits[h].get()).max(0));
            if x > 0 {
                hh.hh_deposits[h] -= bani(x);
                hh.spending_to_date[h] += bani(x);
                paid += i128::from(hh.hh_weight[h]) * x;
            }
        }
        let mut post = Postings::default();
        post.pays_gov(Sector::Households, bani(paid), FlowCode::TAX_WEALTH);
        self.bank_intraday_advance(&mut post);
        post.commit(&mut self.ledger);
    }

    /// A unit bought inputs worth `value` today: its product tax.
    pub(super) fn rev_input_purchase(&mut self, u: usize, value: i128) {
        if !self.rev.active || value <= 0 {
            return;
        }
        self.rev.day.unit_inputs[u] += bani(value);
        let j = usize::from(self.units.industry[u]);
        let rate = self.rev.product_tax_ppm.get(j).copied().unwrap_or(0);
        self.charge_product_tax(u, ratio(value, i128::from(rate), 1_000_000));
    }

    /// A unit paid `value` for investment goods today: its product tax.
    pub(super) fn rev_investment_purchase(&mut self, u: usize, value: i128) {
        if !self.rev.active || value <= 0 {
            return;
        }
        self.rev.day.unit_investment[u] += bani(value);
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
        self.rev.day.unit_product_tax[u] += bani(tax);
    }

    /// The end of the goods market: the day's product taxes and excise
    /// posted.
    pub(super) fn rev_post_products(&mut self) {
        if !self.rev.active {
            return;
        }
        let x = std::mem::take(&mut self.rev.products_today);
        let e = std::mem::take(&mut self.rev.excise_today);
        let mut post = Postings::default();
        post.pays_gov(Sector::Firms, bani(x), FlowCode::TAX_PRODUCTS);
        post.pays_gov(Sector::Firms, bani(e), FlowCode::TAX_EXCISE);
        self.bank_intraday_advance(&mut post);
        post.commit(&mut self.ledger);
    }

    /// The product tax and fuel duty taxes not yet posted today (phase 8
    /// posts them).
    pub(super) fn rev_pending_taxes(&self) -> i128 {
        if self.rev.active {
            self.rev.products_today + self.rev.excise_today
        } else {
            0
        }
    }

    /// The product tax unit `u` would owe on inputs worth `value`, as
    /// [`World::rev_input_purchase`] charges it.
    pub(super) fn rev_input_tax(&self, u: usize, value: i128) -> i128 {
        if !self.rev.active || value <= 0 {
            return 0;
        }
        let j = usize::from(self.units.industry[u]);
        let rate = self.rev.product_tax_ppm.get(j).copied().unwrap_or(0);
        ratio(value, i128::from(rate), 1_000_000).max(0)
    }

    /// The fuel duty unit `u` would owe on imported inputs whose value at
    /// the index is `value`, as [`World::rev_imported_fuel`] charges it.
    pub(super) fn rev_imported_fuel_duty(&self, u: usize, value: i128) -> i128 {
        if !self.rev.active || value <= 0 {
            return 0;
        }
        let Some(f) = self.rev.fuel else {
            return 0;
        };
        let j = usize::from(self.units.industry[u]);
        if self.rev.fuel_exempt[j] {
            return 0;
        }
        let fuel_value = ratio(
            value,
            i128::from(self.rev.fuel_import_ppm.get(j).copied().unwrap_or(0)),
            1_000_000,
        );
        let quantity = ratio(fuel_value, 1_000_000, self.import_index().max(1));
        ratio(quantity, self.excise_duty_ppm(f), 1_000_000).max(0)
    }

    /// The fuel duty unit `u` would owe on `quantity` real units of good
    /// `i`, as [`World::rev_fuel_purchase`] charges it.
    pub(super) fn rev_fuel_duty(&self, u: usize, i: usize, quantity: i64) -> i128 {
        if !self.rev.active || self.rev.fuel != Some(i) || quantity <= 0 {
            return 0;
        }
        let j = usize::from(self.units.industry[u]);
        if self.rev.fuel_exempt[j] {
            return 0;
        }
        ratio(i128::from(quantity), self.excise_duty_ppm(i), 1_000_000).max(0)
    }

    /// The rate of product tax on investment goods, millionths, as
    /// [`World::rev_investment_purchase`] charges it.
    pub(super) fn rev_investment_tax_ppm(&self) -> i128 {
        if self.rev.active {
            i128::from(self.rev.investment_tax_ppm)
        } else {
            0
        }
    }

    /// A unit's value of output today.
    pub(super) fn rev_output_value(&mut self, u: usize, value: i128) {
        if self.rev.active {
            self.rev.output_month[u] += value;
        }
    }

    /// A new household record: its imputed-rent key and dwelling (an owner's
    /// key is its dwelling value).
    #[cfg(feature = "lifecycle")]
    pub(super) fn rev_push_household(&mut self, rent_key: i128, dwelling: i128) {
        if !self.rev.active {
            return;
        }
        let r = &mut self.rev;
        r.rent_key.push(rent_key);
        r.dwelling.push(dwelling);
        if !r.rent.key.is_empty() {
            r.rent.key.push(Bani::ZERO);
        }
        if !r.rent.received.is_empty() {
            r.rent.received.push(Bani::ZERO);
        }
    }

    /// A household's imputed-rent key and dwelling, and setting them.
    #[cfg(feature = "lifecycle")]
    pub(super) fn rev_dwelling_of(&self, h: usize) -> (i128, i128) {
        (
            self.rev.rent_key.get(h).copied().unwrap_or(0),
            self.rev.dwelling.get(h).copied().unwrap_or(0),
        )
    }

    /// See [`World::rev_dwelling_of`].
    #[cfg(feature = "lifecycle")]
    pub(super) fn rev_set_dwelling(&mut self, h: usize, rent_key: i128, dwelling: i128) {
        if let Some(k) = self.rev.rent_key.get_mut(h) {
            *k = rent_key;
        }
        if let Some(d) = self.rev.dwelling.get_mut(h) {
            *d = dwelling;
        }
    }

    /// A copy of person `p`'s activity (a split record).
    #[cfg(feature = "lifecycle")]
    pub(super) fn rev_copy_person(&mut self, p: usize) {
        if let Some(&a) = self.rev.activity.get(p) {
            self.rev.activity.push(a);
        }
    }

    /// The yearly compaction: drop the removed persons and households.
    #[cfg(feature = "demography")]
    pub(super) fn rev_compact(&mut self, keep_p: &[bool], keep_h: &[bool]) {
        use super::money::retain_by;
        let r = &mut self.rev;
        retain_by(&mut r.activity, keep_p);
        retain_by(&mut r.rent_key, keep_h);
        retain_by(&mut r.dwelling, keep_h);
        retain_by(&mut r.rent.key, keep_h);
        retain_by(&mut r.rent.received, keep_h);
    }

    /// A new person's activity (Stage C's code), for the distribution keys.
    #[cfg(feature = "demography")]
    pub(super) fn rev_push_person(&mut self, activity: u8) {
        if !self.rev.activity.is_empty() {
            self.rev.activity.push(activity);
        }
    }

    /// An estate's dwelling: the ended household's key and dwelling are
    /// gone; the heir's dwelling is now `heir_dwelling` (per household), and
    /// an owner heir's imputed-rent key follows it (tenure does not change
    /// before housing, M13).
    #[cfg(feature = "demography")]
    pub(super) fn rev_estate_dwelling(&mut self, ended: usize, heir: Option<(usize, i128)>) {
        if !self.rev.active {
            return;
        }
        let r = &mut self.rev;
        if let Some(k) = r.rent_key.get_mut(ended) {
            *k = 0;
        }
        if let Some(d) = r.dwelling.get_mut(ended) {
            *d = 0;
        }
        if let Some((h, value)) = heir {
            if let Some(d) = r.dwelling.get_mut(h) {
                *d = value;
            }
            if let Some(k) = r.rent_key.get_mut(h)
                && *k > 0
            {
                *k = value.max(1);
            }
        }
    }

    /// A unit received `net` of households' spending on good `j` today.
    pub(super) fn rev_household_sale(&mut self, j: usize, u: usize, net: i128) {
        if self.rev.active && self.rev.real_estate == Some(j) {
            self.rev.rent_month[u] += net;
        }
    }

    /// The start of a day: the day's views emptied; 1 January indexes the
    /// duties; the 15th pays last month's imputed rent and the held
    /// benefits (from the first 15th: they are twelfths of a year, not last
    /// month's).
    pub(super) fn rev_begin_day(&mut self) {
        if !self.rev.active {
            return;
        }
        let date = self.calendar.date(self.today);
        if date.month == 1 && date.day == 1 && !self.months.is_empty() {
            self.index_excise();
        }
        self.reset_day();
        if date.day == PAYBACK_DAY {
            if !self.rev.no_imputed_rent && !self.months.is_empty() {
                self.pay_imputed_rent();
            }
            self.pay_held_benefits();
        }
    }

    /// Monthly close, before profit: the other taxes on production, the held
    /// revenue lines, EU funds and the month's report; this month's
    /// real-estate spending becomes what the 15th pays from.
    pub(super) fn rev_close(&mut self) {
        if !self.rev.active {
            return;
        }
        let k = u32::try_from(self.months.len().saturating_sub(1) % 12).expect("a month");
        let n_u = self.units.industry.len();
        let mut total = 0i128;
        let mut output_value = vec![Bani::ZERO; n_u];
        let mut production_tax = vec![Bani::ZERO; n_u];
        let mut due = vec![0i128; n_u];
        for u in 0..n_u {
            let value = std::mem::take(&mut self.rev.output_month[u]);
            output_value[u] = bani(value);
            let j = usize::from(self.units.industry[u]);
            let rate = i128::from(self.rev.production_tax_ppm.get(j).copied().unwrap_or(0));
            due[u] = ratio(value.max(0), rate, 1_000_000).max(0);
        }
        // What the firm sector cannot pay is reported unpaid (the backstop).
        self.firm_fit_parts(
            &Postings::default(),
            &mut due,
            Some(super::insolvency::Unpaid::Taxes),
        );
        for (u, &tax) in due.iter().enumerate() {
            if tax > 0 {
                self.units.deposits[u] -= bani(tax);
                self.fs.add_inputs(u, tax);
                production_tax[u] = bani(tax);
                total += tax;
            }
        }
        let mut post = Postings::default();
        post.pays_gov(Sector::Firms, bani(total), FlowCode::TAX_PRODUCTION);
        self.bank_intraday_advance(&mut post);
        post.commit(&mut self.ledger);
        let (held, property) = self.pay_held_revenue(k);
        self.pay_wealth_tax();
        let eu = self.pay_eu_funds(u8::try_from(k).expect("a month"));
        let own_resources = i128::from(
            self.ledger
                .flows()
                .get(FlowCode::TRANSFER_EU_BUDGET, Sector::RestOfWorld)
                .get(),
        );
        let month =
            econ_types::Month(u32::try_from(self.months.len().saturating_sub(1)).expect("months"));
        let excise = std::mem::replace(
            &mut self.rev.excise_month,
            vec![0; self.industries.id.len()],
        );
        self.rev.months.push(RevenueMonth {
            month,
            excise_by_good: excise.into_iter().map(bani).collect(),
            unit_output_value: output_value,
            unit_production_tax: production_tax,
            held,
            hh_property_tax: property,
            eu_net_balance: bani(eu - own_resources),
        });
        let r = &mut self.rev;
        r.rent_last = std::mem::replace(&mut r.rent_month, vec![0; r.rent_last.len()]);
    }

    /// Imputed rent back to owner households, each its key's share rounded;
    /// the units pay by their part of last month's spending.
    fn pay_imputed_rent(&mut self) {
        let spent: i128 = self.rev.rent_last.iter().sum();
        let due = ratio(spent, IMPUTED_SHARE_PPM, 1_000_000);
        // Never more than the firm sector holds (no lever may overdraw it;
        // in a normal run this never binds).
        let payable = self.firm_payable(&Postings::default(), due);
        let (parts, paid) = share_out(payable, &self.households.hh_weight, &self.rev.rent_key);
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
        let v = &mut self.rev.rent;
        v.day = Some(Day(self.today.0));
        v.spending = bani(spent);
        v.due = bani(due);
        v.received = parts.into_iter().map(bani).collect();
    }

    /// The views of `observe.rs`.
    pub(super) fn rev_day_view(&self) -> &RevenueDay {
        &self.rev.day
    }
    pub(super) fn rev_months_view(&self) -> &[RevenueMonth] {
        &self.rev.months
    }
    pub(super) fn rev_held_view(&self) -> &[HeldBenefit] {
        &self.rev.held
    }
    pub(super) fn rev_rent_view(&self) -> &ImputedRentPayment {
        &self.rev.rent
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
        for &d in &self.rev.duty_ppm {
            write(u64::from(d));
        }
        #[allow(clippy::cast_sign_loss)]
        for (m, l) in self.rev.rent_month.iter().zip(&self.rev.rent_last) {
            write(*m as u64 ^ (*l as u64).rotate_left(31));
        }
        #[allow(clippy::cast_sign_loss)]
        for &e in &self.rev.excise_month {
            write(e as u64);
        }
    }
}
