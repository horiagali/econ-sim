//! Phases 2 and 4 to 8 (spec `game/world-loop`): the units' plans, prices,
//! output and input purchases, the fixed final demand, the households'
//! decisions and the goods market.

use econ_ledger::{FlowCode, Sector};
use econ_mech_households::{Budget, decide};
use econ_mech_prices::{markup_ppm, step_price_ppm, target_price_ppm, unit_cost_ppm};
use econ_mech_production::{UnitState as PlanInput, plan, ration, share_sales};
use econ_mech_tax::vat::VatCategory;
use econ_num::math::powf;
use econ_types::Bani;

use super::money::{Postings, bani, ratio, split, split2};
use super::{UnitState, World};

const ONE: i128 = 1_000_000;
/// An input is essential, and its shortage cuts output the next day, when
/// it is at least this share of output (spec `economy/production`, edge
/// cases: "an essential input is short").
pub(super) const ESSENTIAL_INPUT_PPM: u32 = 10_000;
/// Days of use a unit aims to hold of each essential input, and the days
/// over which it closes a gap to that target.
pub(super) const INPUT_STOCK_DAYS: i64 = 60;
const INPUT_REFILL_DAYS: i64 = 30;
/// Romania's VAT gap in 2021, the share of VAT due that is not collected
/// (the anchor of spec `economy/firm-units-undeclared-inventories`, 36.7%).
/// Until the informal economy decides who evades (M15), households pay the
/// legal rates less this share: the input-output table's taxes on household
/// consumption (7.2% of it) are what is collected, not the legal 10.7%.
const VAT_GAP_PPM: i128 = 367_000;

/// Persons a unit needs for a month's output `y`: its starting staff per
/// unit of starting output, times the labour-requirement factor, rounded up.
pub(super) fn desired(uu: &UnitState, u: usize, y: i64) -> u64 {
    let (staff0, out0) = (i128::from(uu.staff0[u]), i128::from(uu.output0[u]));
    let factor = i128::from(uu.labour_factor_ppm);
    if out0 <= 0 {
        return u64::try_from(staff0 * factor / ONE).expect("persons");
    }
    let need = i128::from(y.max(0)) * staff0 * factor;
    let den = out0 * ONE;
    u64::try_from((need + den - 1) / den).expect("persons")
}

impl World {
    /// The day's import price index, millionths: 1,000,000 until trade acts.
    pub(super) fn import_index(&self) -> i128 {
        #[cfg(feature = "trade")]
        {
            i128::from(self.import_index_ppm())
        }
        #[cfg(not(feature = "trade"))]
        {
            ONE
        }
    }

    /// Good `j`'s excise duty per unit, millionths (0 without excise).
    fn excise_of(&self, j: usize) -> i128 {
        #[cfg(feature = "revenue")]
        {
            self.excise_duty_ppm(j)
        }
        #[cfg(not(feature = "revenue"))]
        {
            let _ = j;
            0
        }
    }

    /// An amount at the starting import price, at the day's.
    pub(super) fn at_import_price(&self, v: i128) -> i128 {
        let mi = self.import_index();
        if mi == ONE { v } else { ratio(v, mi, ONE) }
    }

    /// Industry `j`'s imported inputs worth `v` at the starting prices, at
    /// the day's import prices (oil at its own, `world.fuel_price[oil]`).
    pub(super) fn inputs_import_value(&self, j: usize, v: i128) -> i128 {
        #[cfg(feature = "trade")]
        {
            self.trade_inputs_import_value(j, v)
        }
        #[cfg(not(feature = "trade"))]
        {
            let _ = j;
            self.at_import_price(v)
        }
    }

    /// The VAT households pay on good `j`, basis points: the legal rate less
    /// the VAT gap (M2 placeholder until the informal economy, M15).
    pub(super) fn rate_bp(&self, j: usize) -> Option<u32> {
        let cat: VatCategory = self.law.vat_category[j];
        self.law
            .vat
            .rate_for(cat)
            .map(|r| u32::try_from(ratio(i128::from(r.0), ONE - VAT_GAP_PPM, ONE)).expect("a rate"))
    }

    /// Phase 2: the plan of each unit whose day it is (production, step 1).
    pub(super) fn plan(&mut self, cohort: u8) {
        let today = self.today;
        let days = self.calendar.date(today).days_in_month();
        let u_cols = &mut self.units;
        for u in 0..u_cols.industry.len() {
            if u_cols.unit_stagger_day[u] != cohort {
                continue;
            }
            let state = PlanInput {
                expected_demand: u_cols.expected_demand[u],
                inventory: u_cols.inventory[u],
                capacity: u_cols.capacity[u],
                inventory_target_ppm: u_cols.inventory_target_ppm[u],
                labour_per_million_output: 0,
            };
            let p = plan(&state, u_cols.sales_since_plan[u], &self.params.production);
            u_cols.expected_demand[u] = p.expected_demand;
            self.uu.plan_output[u] = p.planned_output;
            self.uu.plan_days[u] = days;
            self.uu.plan_done[u] = 0;
            u_cols.desired_employment[u] = desired(&self.uu, u, p.planned_output);
            u_cols.sales_since_plan[u] = 0;
            u_cols.last_plan[u] = Some(today);
        }
        // Public units under the ministry budgets staff to their target
        // (their output still follows their market sales).
        #[cfg(feature = "levers")]
        for u in 0..self.units.industry.len() {
            if self.units.unit_stagger_day[u] == cohort && self.budget_public(u) {
                self.budget_plan(u);
            }
        }
        // The month's investment budget of the same units (investment and
        // capital, step 1).
        #[cfg(feature = "investment")]
        if self.invest_active() {
            for u in 0..self.units.industry.len() {
                if self.units.unit_stagger_day[u] == cohort {
                    self.invest_budget(u, days);
                }
            }
        }
    }

    /// Expected inflation a day, millionths.
    fn expected_daily_ppm(&self) -> i32 {
        let yearly = 1.0 + f64::from(self.mac.expected_inflation_ppm) / 1e6;
        let daily = powf(yearly.max(1e-6), 1.0 / 365.0) - 1.0;
        // A few hundred millionths at most.
        #[allow(clippy::cast_possible_truncation)]
        let v = (daily * 1e6).round() as i32;
        v
    }

    /// Industry `j`'s unit cost at the prices `prices` (prices, step 1).
    pub(super) fn unit_cost_at(&self, j: usize, prices: &[u32]) -> u32 {
        let home: Vec<(u32, u32)> = self.ii.inputs[j]
            .iter()
            .map(|&(i, a)| (a, prices[i]))
            .collect();
        unit_cost_ppm(
            &home,
            u32::try_from(self.inputs_import_value(j, i128::from(self.ii.import_ppm[j])))
                .unwrap_or(u32::MAX),
            self.ii.wage_cost_ppm[j],
            self.ii.other_ppm[j],
        )
    }

    /// Good `j`'s consumer price at producer price `p` (with excise and VAT
    /// when the revenue of 2021 acts; else the producer price).
    fn consumer_price_at(&self, j: usize, p: u32) -> u32 {
        #[cfg(feature = "revenue")]
        {
            self.rev_consumer_price(j, p)
        }
        #[cfg(not(feature = "revenue"))]
        {
            let _ = j;
            p
        }
    }

    /// Phase 4: unit cost, markup, target and the day's price of each industry.
    pub(super) fn prices(&mut self) {
        let n_i = self.industries.id.len();
        let expected = self.expected_daily_ppm();
        let (mut planned, mut capacity, mut stock, mut target_stock) = (
            vec![0i128; n_i],
            vec![0i128; n_i],
            vec![0i128; n_i],
            vec![0i128; n_i],
        );
        let u = &self.units;
        for k in 0..u.industry.len() {
            let j = usize::from(u.industry[k]);
            planned[j] += i128::from(self.uu.plan_output[k]);
            capacity[j] += i128::from(u.capacity[k]);
            stock[j] += i128::from(u.inventory[k]);
            target_stock[j] +=
                i128::from(u.inventory_target_ppm[k]) * i128::from(u.expected_demand[k]) / ONE;
        }
        let old: Vec<u32> = self.industries.price_ppm.clone();
        let p = &self.params.prices;
        for j in 0..n_i {
            let uc = self.unit_cost_at(j, &old);
            let cu = if capacity[j] > 0 {
                u32::try_from(ratio(planned[j], ONE, capacity[j]).clamp(0, 10 * ONE))
                    .expect("small")
            } else {
                p.normal_utilisation_ppm
            };
            let inv = if target_stock[j] > 0 {
                u32::try_from(ratio(stock[j], ONE, target_stock[j]).clamp(0, 10 * ONE))
                    .expect("small")
            } else {
                1_000_000
            };
            let mu = markup_ppm(self.ii.markup0_ppm[j], cu, inv, p);
            let target = target_price_ppm(uc, mu);
            let next = step_price_ppm(old[j], target, self.ii.flex_daily_ppm[j], expected, p);
            self.industries.price_ppm[j] = next;
            self.industries.consumer_price_ppm[j] = self.consumer_price_at(j, next);
        }
    }

    /// Phase 5: each unit's output of the day into inventory, and its input
    /// orders: home inputs as demand on the suppliers, imported inputs paid
    /// to the rest of the world at the starting price.
    pub(super) fn output_and_inputs(&mut self) {
        let mut post = Postings::default();
        for u in 0..self.units.industry.len() {
            #[cfg(feature = "levers")]
            if self.budget_public(u) {
                self.budget_buy(u, &mut post);
            }
            let j = usize::from(self.units.industry[u]);
            let (y, days, done) = (
                self.uu.plan_output[u],
                self.uu.plan_days[u],
                self.uu.plan_done[u],
            );
            let part = if done >= days || days == 0 {
                0
            } else if done + 1 == days {
                y - (y / i64::from(days)) * i64::from(days - 1)
            } else {
                y / i64::from(days)
            };
            if done < days {
                self.uu.plan_done[u] = done + 1;
            }
            // Cut by missing staff and by the stock of its essential inputs.
            // Staff is counted in whole records, so it meets the requirement
            // when it is within half a record of it.
            let required = self.units.desired_employment[u];
            let staff = self.units.staff[u];
            let members = self.uu.members[u].len() as u64;
            let half_record = staff
                .checked_div(members)
                .unwrap_or(u64::from(self.sample_scale))
                / 2;
            let staff_ppm = if required > 0 && staff + half_record < required {
                ratio(i128::from(staff + half_record), ONE, i128::from(required))
            } else {
                ONE
            };
            let mut share = staff_ppm;
            for (k, &a) in self.ii.essential_ppm[j].iter().enumerate() {
                let need = i128::from(part) * i128::from(a) / ONE;
                if need > 0 {
                    share = share.min(i128::from(self.uu.input_stock[u][k].max(0)) * ONE / need);
                }
            }
            let out = i64::try_from(i128::from(part) * share.min(ONE) / ONE).expect("an output");
            for (k, &a) in self.ii.essential_ppm[j].iter().enumerate() {
                let a = i64::from(a);
                self.uu.input_stock[u][k] -= (out * a + 500_000) / 1_000_000;
            }
            self.units.inventory[u] += out;
            self.units.output_today[u] = out;
            self.ii.output_month[j] += out;
            let value = ratio(
                i128::from(out),
                i128::from(self.industries.price_ppm[j]),
                ONE,
            );
            self.mac.month.output_value += value;
            self.fs.add_value(u, value);
            #[cfg(feature = "revenue")]
            self.rev_output_value(u, value);
            for (&(i, a), &slot) in self.ii.inputs[j].iter().zip(&self.ii.essential_slot[j]) {
                // A day's part is far below 2^43 real units: the product fits i64.
                let mut q = (part * i64::from(a) + 500_000) / 1_000_000;
                if let Some(k) = slot {
                    // The target is days of an average day of the plan (a
                    // thirtieth of the month), so short months do not restock.
                    let target = (y / 30 * i64::from(a) + 500_000) / 1_000_000 * INPUT_STOCK_DAYS;
                    q = (q + (target - self.uu.input_stock[u][k]) / INPUT_REFILL_DAYS).max(0);
                }
                if q > 0 {
                    self.demand.units[i].push((u, q));
                }
            }
            let at_start = ratio(i128::from(part), i128::from(self.ii.import_ppm[j]), ONE);
            let imported = bani(self.inputs_import_value(j, at_start));
            if imported > Bani::ZERO {
                self.units.deposits[u] -= imported;
                post.deposits(
                    Sector::Firms,
                    Sector::RestOfWorld,
                    imported,
                    FlowCode::IMPORTS,
                );
                self.mac.month.inputs_imported += i128::from(imported.get());
                self.fs.add_inputs(u, i128::from(imported.get()));
                #[cfg(feature = "revenue")]
                {
                    self.rev_input_purchase(u, i128::from(imported.get()));
                    // Its fuel's quantity is read at the index, as before
                    // oil had its own price.
                    self.rev_imported_fuel(u, self.at_import_price(at_start));
                }
                self.mac.month.imports += i128::from(imported.get());
            }
            self.uu.inputs_served_ppm[u] = 1_000_000;
        }
        post.commit(&mut self.ledger);
    }

    /// Each unit's month of wages at today's staff and wages.
    pub(super) fn wage_reserves(&self) -> Vec<i128> {
        (0..self.units.industry.len())
            .map(|u| {
                self.uu.members[u]
                    .iter()
                    .map(|&i| {
                        let h = self.persons.household_id[i as usize] as usize;
                        i128::from(self.households.hh_weight[h])
                            * i128::from(self.persons.wage[i as usize].get())
                    })
                    .sum()
            })
            .collect()
    }

    /// What each unit can spend on investment, on its owners' income and on
    /// lending without touching a month of its wages.
    fn spare_money(&self) -> Vec<i128> {
        (0..self.units.industry.len())
            .map(|u| (i128::from(self.units.deposits[u].get()) - self.uu.wage_reserve[u]).max(0))
            .collect()
    }

    /// The end of a day's payments: a public producer's shortfall is paid by
    /// the government as non-market output at cost; cohort units repay what
    /// they owe the sector out of what they can spare, and a unit short of
    /// money borrows it from the others' spare money.
    pub(super) fn settle_credit(&mut self) {
        // The banks' credit lines first, where they exist (M3).
        #[cfg(feature = "banking")]
        self.bank_credit_lines();
        let n_u = self.units.industry.len();
        let mut post = Postings::default();
        let mut at_cost = 0i128;
        for u in 0..n_u {
            if self.uu.public[u] && self.units.deposits[u] < Bani::ZERO {
                let x = -self.units.deposits[u];
                self.units.deposits[u] += x;
                at_cost += i128::from(x.get());
                #[cfg(feature = "levers")]
                self.budget_topup(u, i128::from(x.get()));
            }
        }
        post.gov_pays(Sector::Firms, bani(at_cost), FlowCode::GOV_PURCHASES);
        self.mac.month.government += at_cost;
        self.mac.month.output_value += at_cost;
        self.mac.month.sales_value += at_cost;
        post.commit(&mut self.ledger);
        // Repayments, back to the lenders by their claims.
        let spare = self.spare_money();
        let mut pool = 0i128;
        for u in 0..n_u {
            let owes = i128::from(self.uu.owes[u].get());
            if owes > 0 && spare[u] > 0 {
                let r = owes.min(spare[u]);
                self.uu.owes[u] -= bani(r);
                self.units.deposits[u] -= bani(r);
                pool += r;
            }
        }
        if pool > 0 {
            let claims: Vec<i128> = self.uu.lent.iter().map(|b| i128::from(b.get())).collect();
            for (u, p) in split(pool, &claims).into_iter().enumerate() {
                self.uu.lent[u] -= bani(p);
                self.units.deposits[u] += bani(p);
            }
        }
        // New credit for the units short of money.
        let need: Vec<i128> = (0..n_u)
            .map(|u| {
                if self.uu.public[u] {
                    0
                } else {
                    (-i128::from(self.units.deposits[u].get())).max(0)
                }
            })
            .collect();
        let total: i128 = need.iter().sum();
        if total == 0 {
            return;
        }
        let spare = self.spare_money();
        let mut parts = Self::capped_split(total, &spare, &spare);
        let placed: i128 = parts.iter().sum();
        if placed < total {
            let room: Vec<i128> = (0..n_u)
                .map(|u| (i128::from(self.units.deposits[u].get()) - parts[u]).max(0))
                .collect();
            for (u, p) in split(total - placed, &room).into_iter().enumerate() {
                parts[u] += p;
            }
        }
        for u in 0..n_u {
            if parts[u] > 0 {
                self.units.deposits[u] -= bani(parts[u]);
                self.uu.lent[u] += bani(parts[u]);
            }
            if need[u] > 0 {
                self.units.deposits[u] += bani(need[u]);
                self.uu.owes[u] += bani(need[u]);
            }
        }
    }

    /// Split `total` over units by `weights`, no unit above its `cap`; what
    /// cannot be placed is not paid. Returns each unit's part.
    pub(super) fn capped_split(total: i128, weights: &[i128], caps: &[i128]) -> Vec<i128> {
        let mut parts = vec![0i128; weights.len()];
        let mut left = total;
        let mut active: Vec<usize> = (0..weights.len())
            .filter(|&u| weights[u] > 0 && caps[u] > 0)
            .collect();
        while left > 0 && !active.is_empty() {
            let w: Vec<i128> = active.iter().map(|&u| weights[u]).collect();
            let share = split(left, &w);
            let mut over = Vec::new();
            for (&u, &s) in active.iter().zip(&share) {
                if parts[u] + s > caps[u] {
                    over.push(u);
                }
            }
            if over.is_empty() {
                for (&u, &s) in active.iter().zip(&share) {
                    parts[u] += s;
                }
                break;
            }
            for &u in &over {
                left -= caps[u] - parts[u];
                parts[u] = caps[u];
            }
            active.retain(|u| !over.contains(u));
        }
        parts
    }

    /// Phase 6: the imported parts of the fixed government purchases and
    /// investment, paid to the rest of the world; their domestic parts and
    /// the exports are demand in the goods market.
    pub(super) fn fixed_demand(&mut self) {
        #[cfg(feature = "investment")]
        self.invest_demand_today();
        let mut post = Postings::default();
        let gov: Bani = self.ii.gov_imp.iter().copied().sum();
        let gov = bani(self.at_import_price(i128::from(gov.get())));
        post.gov_pays(Sector::RestOfWorld, gov, FlowCode::IMPORTS);
        self.mac.month.government += i128::from(gov.get());
        self.mac.month.imports += i128::from(gov.get());
        let inv: i128 = self.ii.inv_imp.iter().map(|b| i128::from(b.get())).sum();
        let inv = self.at_import_price(inv);
        // Who pays: with the investment rule, the government its share
        // (step 3) and each unit by its budget; else M2's units by surplus,
        // out of what they can spare.
        let (gov_part, parts) = match self.investment_payers() {
            Some((gov_ppm, weights)) => {
                let g = ratio(inv, gov_ppm, ONE);
                (g, split(inv - g, &weights))
            }
            None => {
                let caps = self.spare_money();
                (
                    0,
                    Self::capped_split(inv, &self.investment_weights(), &caps),
                )
            }
        };
        post.gov_pays(Sector::RestOfWorld, bani(gov_part), FlowCode::IMPORTS);
        #[cfg(feature = "investment")]
        self.invest_gov_spent(gov_part);
        self.mac.month.investment += gov_part;
        self.mac.month.imports += gov_part;
        let paid: i128 = parts.iter().sum();
        for (u, &p) in parts.iter().enumerate() {
            self.units.deposits[u] -= bani(p);
            #[cfg(feature = "investment")]
            self.invest_spent(u, p);
            #[cfg(feature = "revenue")]
            self.rev_investment_purchase(u, p);
        }
        post.deposits(
            Sector::Firms,
            Sector::RestOfWorld,
            bani(paid),
            FlowCode::IMPORTS,
        );
        self.mac.month.investment += paid;
        self.mac.month.imports += paid;
        post.commit(&mut self.ledger);
    }

    /// Who pays today's investment under the investment rule: the
    /// government's share and the units' weights; `None` under M2's
    /// placeholder.
    fn investment_payers(&self) -> Option<(i128, Vec<i128>)> {
        #[cfg(feature = "investment")]
        {
            self.invest_payers()
        }
        #[cfg(not(feature = "investment"))]
        {
            None
        }
    }

    /// Who pays the investment goods: cohort units, by their starting surplus.
    fn investment_weights(&self) -> Vec<i128> {
        self.uu
            .invest_weight
            .iter()
            .map(|&w| i128::from(w))
            .collect()
    }

    /// Phase 7: each household whose day it is decides its month's spending
    /// and splits it over the goods (households, steps 2 and 3).
    pub(super) fn household_decisions(&mut self, cohort: u8) {
        let today = self.today;
        let cpi = self.mac.cpi.last().copied().unwrap_or(1_000_000);
        let scale = self.mixed_scale_ppm();
        // The month's interest on the household's loans, known in advance:
        // disposable income is net of it (spec economy/households), so the
        // household keeps it aside for the month end (M3, with the banks).
        #[cfg(feature = "banking")]
        let dues = self.hh_interest_due();
        let h = &mut self.households;
        for k in 0..h.hh_weight.len() {
            if h.hh_stagger_day[k] != cohort {
                continue;
            }
            #[cfg(feature = "banking")]
            let due = dues.get(k).copied().unwrap_or(Bani::ZERO);
            #[cfg(not(feature = "banking"))]
            let due = Bani::ZERO;
            let per_adult = i128::from(self.hh.income_last_month[k].get()) * 1_000
                / i128::from(self.hh.eq_adults_milli[k].max(1));
            let fifth = super::close::fifth_of(per_adult, &self.mac.fifth_bounds);
            h.hh_fifth[k] = fifth;
            let b = Budget {
                income_received: (h.hh_income_received[k] - due).max(Bani::ZERO),
                money: (h.hh_cash[k] + h.hh_deposits[k] - due).max(Bani::ZERO),
                fifth,
                equivalised_adults_milli: self.hh.eq_adults_milli[k],
                subsistence_income: bani(ratio(
                    i128::from(self.hh.subsistence[k].get()),
                    scale,
                    ONE,
                )),
                price_index_ppm: cpi,
            };
            let d = decide(&b, &self.params.households);
            h.hh_hardship[k] = d.hardship;
            h.hh_income_received[k] = Bani::ZERO;
            h.last_decision[k] = Some(today);
            if d.spending > Bani::ZERO {
                self.demand.households.push((k, d.spending));
            }
        }
    }

    /// Phase 8: per industry, the day's demand against its inventories; sales
    /// shared over its units; the money booked; shortages recorded.
    #[allow(clippy::too_many_lines)]
    pub(super) fn goods_market(&mut self) {
        let n_i = self.industries.id.len();
        let mut post = Postings::default();
        let buyers = std::mem::take(&mut self.demand.households);
        let weight: Vec<i128> = buyers
            .iter()
            .map(|(k, _)| i128::from(self.households.hh_weight[*k]))
            .collect();
        // The day's household money, with weights, split over the goods by
        // the basket shares; the imported part of each good is always served,
        // the domestic part is a demand on the industry.
        let budget: i128 = buyers
            .iter()
            .zip(&weight)
            .map(|((_, s), &w)| w * i128::from(s.get()))
            .sum();
        let rates: Vec<i64> = (0..n_i)
            .map(|j| self.rate_bp(j).map_or(0, i64::from))
            .collect();
        let shares: Vec<i128> = self
            .industries
            .basket_share_ppm
            .iter()
            .map(|&s| i128::from(s))
            .collect();
        let money = split(budget, &shares);
        let mut dom_money = vec![0i128; n_i];
        let mut imp_money = vec![0i128; n_i];
        let mut hh_want_q = vec![0i64; n_i];
        for j in 0..n_i {
            imp_money[j] = money[j] * i128::from(self.ii.basket_import_ppm[j]) / ONE;
            dom_money[j] = money[j] - imp_money[j];
            let net = dom_money[j] * 10_000 / (10_000 + i128::from(rates[j]));
            let p = i128::from(self.industries.price_ppm[j].max(1));
            // With excise the unit costs the price plus the duty.
            hh_want_q[j] = i64::try_from(net * ONE / (p + self.excise_of(j))).expect("a quantity");
        }
        // What households got of each good, as a share of what they wanted.
        let mut hh_served_ppm = vec![1_000_000i128; n_i];
        let caps_inv = self.spare_money();
        let inv_weights = self.investment_weights();
        let mut units_of: Vec<Vec<usize>> = vec![Vec::new(); n_i];
        for (u, &j) in self.units.industry.iter().enumerate() {
            units_of[usize::from(j)].push(u);
        }
        let mut worst = vec![1_000_000u32; self.units.industry.len()];
        let mut investment_value = 0i128;
        for i in 0..n_i {
            let price = i128::from(self.industries.price_ppm[i]);
            let unit_buyers = std::mem::take(&mut self.demand.units[i]);
            let agents = std::mem::take(&mut self.demand.agents[i]);
            let na = agents.len();
            let hh_want = i128::from(hh_want_q[i]);
            let (g, inv, x) = (self.ii.gov_q[i], self.ii.inv_q[i], self.ii.exp_q[i]);
            let mut demands: Vec<i64> = unit_buyers.iter().map(|&(_, q)| q).collect();
            demands.extend(agents.iter().map(|&(_, q)| q));
            demands.push(i64::try_from(hh_want).expect("a demand"));
            demands.extend([g, inv, x]);
            let total: i64 = demands.iter().sum();
            let sellers = &units_of[i];
            let available: i64 = sellers
                .iter()
                .map(|&u| self.units.inventory[u].max(0))
                .sum();
            let served = ration(&demands, available);
            let nb = unit_buyers.len();
            // Households: their served total shared back, each in proportion to its want.
            let hh_sold = i128::from(served.sold[nb + na]);
            if hh_want > 0 {
                hh_served_ppm[i] = hh_sold * ONE / hh_want;
            }
            let (g_s, i_s, x_s) = (
                served.sold[nb + na + 1],
                served.sold[nb + na + 2],
                served.sold[nb + na + 3],
            );
            #[cfg(feature = "trade")]
            self.trade_exported(x_s);
            let unit_sold: i64 = served.sold[..nb + na].iter().sum();
            let sold = i128::from(unit_sold) + hh_sold + i128::from(g_s + i_s + x_s);
            let sold = i64::try_from(sold).expect("sales");
            self.industries.demand_today[i] = total;
            self.industries.shortage_today[i] = total - sold;
            self.ii.shortage_month[i] += total - sold;
            // Units' sales, by their inventories.
            let stocks: Vec<i64> = sellers
                .iter()
                .map(|&u| self.units.inventory[u].max(0))
                .collect();
            let shares = if sold > 0 {
                share_sales(sold, &stocks)
            } else {
                vec![0; sellers.len()]
            };
            for (&u, &s) in sellers.iter().zip(&shares) {
                self.units.inventory[u] -= s;
                self.units.sales_today[u] = s;
                self.units.sales_since_plan[u] += s;
            }
            // Money: what each buyer pays, VAT excluded, into the clearing account.
            // Quantities below 2^43 and prices below 2^20 millionths: i64 suffices.
            let p64 = i64::try_from(price).expect("a price");
            let value = |q: i64| i128::from((q * p64 + 500_000) / 1_000_000);
            let mut paid = 0i128;
            for (b, &(u, q)) in unit_buyers.iter().enumerate() {
                let s = served.sold[b];
                let v = value(s);
                self.units.deposits[u] -= bani(v);
                paid += v;
                self.mac.month.inputs_home += v;
                self.fs.add_inputs(u, v);
                #[cfg(feature = "levers")]
                self.budget_paid(u, v, 0);
                #[cfg(feature = "revenue")]
                {
                    self.rev_input_purchase(u, v);
                    self.rev_fuel_purchase(u, i, s);
                }
                let buyer = usize::from(self.units.industry[u]);
                if let Ok(k) = self.ii.essential[buyer].binary_search(&i) {
                    let share = if q > 0 {
                        u32::try_from(i128::from(s) * ONE / i128::from(q)).expect("a share")
                    } else {
                        1_000_000
                    };
                    self.uu.input_stock[u][k] += s;
                    worst[u] = worst[u].min(share);
                }
            }
            // Public units' investment for the government (ministry budgets).
            for (a, &(u, _)) in agents.iter().enumerate() {
                let v = value(served.sold[nb + a]);
                self.units.deposits[u] -= bani(v);
                paid += v;
                self.mac.month.investment += v;
                #[cfg(feature = "revenue")]
                self.rev_investment_purchase(u, v);
                #[cfg(feature = "levers")]
                self.budget_paid(u, 0, v);
                #[cfg(not(feature = "levers"))]
                let _ = u;
            }
            let gv = value(g_s);
            post.gov_pays(Sector::Firms, bani(gv), FlowCode::GOV_PURCHASES);
            self.mac.month.government += gv;
            let iv = value(i_s);
            investment_value += iv;
            let xv = value(x_s);
            post.deposits(
                Sector::RestOfWorld,
                Sector::Firms,
                bani(xv),
                FlowCode::EXPORTS,
            );
            self.mac.month.exports += xv;
            paid += gv + iv + xv;
            // Out of the clearing account to the selling units, by their sales.
            let w: Vec<i128> = shares.iter().map(|&s| i128::from(s)).collect();
            let revenue = split(paid, &w);
            let mut out = 0i128;
            for (&u, &r) in sellers.iter().zip(&revenue) {
                self.units.deposits[u] += bani(r);
                self.units.revenue_today[u] = bani(r);
                self.fs.add_sales(u, r);
                out += r;
            }
            let left = paid - out;
            if left != 0 {
                self.mac
                    .clearing
                    .push((FlowCode::CONSUMPTION, i, bani(left)));
            }
            self.industries.sales_value_today[i] = bani(paid);
            self.mac.month.sales_value += paid;
        }
        self.uu.inputs_served_ppm = worst;
        // Investment goods: paid by cohort units by their surplus, out of
        // what they can spare.
        let (gov_part, parts) = match self.investment_payers() {
            Some((gov_ppm, weights)) => {
                let g = ratio(investment_value, gov_ppm, ONE);
                (g, split(investment_value - g, &weights))
            }
            None => (
                0,
                Self::capped_split(investment_value, &inv_weights, &caps_inv),
            ),
        };
        post.gov_pays(Sector::Firms, bani(gov_part), FlowCode::GOV_INVESTMENT);
        #[cfg(feature = "investment")]
        self.invest_gov_spent(gov_part);
        let placed: i128 = parts.iter().sum::<i128>() + gov_part;
        for (u, &p) in parts.iter().enumerate() {
            self.units.deposits[u] -= bani(p);
            #[cfg(feature = "investment")]
            self.invest_spent(u, p);
            #[cfg(feature = "revenue")]
            self.rev_investment_purchase(u, p);
        }
        if placed < investment_value {
            // Nobody could pay the rest: the sellers' cohort pays it as a
            // whole, spread by deposits (the sector cannot leave goods unpaid).
            let rest = investment_value - placed;
            let deposits: Vec<i128> = self
                .units
                .deposits
                .iter()
                .map(|d| i128::from(d.get()).max(0))
                .collect();
            for (u, p) in split(rest, &deposits).into_iter().enumerate() {
                self.units.deposits[u] -= bani(p);
                #[cfg(feature = "investment")]
                self.invest_spent(u, p);
                #[cfg(feature = "revenue")]
                self.rev_investment_purchase(u, p);
            }
        }
        self.mac.month.investment += investment_value;
        // Households pay the served part of their spending: the same share for
        // every household, since every basket is the same.
        let mut parts = Vec::with_capacity(4 * n_i);
        for j in 0..n_i {
            let dom = dom_money[j] * hh_served_ppm[j] / ONE;
            let dom_net = dom * 10_000 / (10_000 + i128::from(rates[j]));
            let imp_net = imp_money[j] * 10_000 / (10_000 + i128::from(rates[j]));
            parts.extend([dom_net, dom - dom_net, imp_net, imp_money[j] - imp_net]);
        }
        let served: i128 = parts.iter().sum();
        let (mut by_cash, mut by_deposits, mut paid_total) = (0i128, 0i128, 0i128);
        for (b, &(k, spending)) in buyers.iter().enumerate() {
            let gross = if budget > 0 {
                bani(i128::from(spending.get()) * served / budget)
            } else {
                Bani::ZERO
            };
            if gross <= Bani::ZERO {
                continue;
            }
            let (cash, dep) = (self.households.hh_cash[k], self.households.hh_deposits[k]);
            let (pc, pd) = split2(
                i128::from(gross.get()),
                i128::from(cash.get()),
                i128::from(dep.get()),
            );
            let (pc, pd) = (bani(pc), bani(pd));
            self.households.hh_cash[k] -= pc;
            self.households.hh_deposits[k] -= pd;
            self.households.spending_to_date[k] += gross;
            let w = weight[b];
            by_cash += w * i128::from(pc.get());
            by_deposits += w * i128::from(pd.get());
            paid_total += w * i128::from(gross.get());
        }
        // What they paid, over the goods' domestic sales, VAT and imports.
        let paid_parts = split(paid_total, &parts);
        let (mut hh_vat, mut firms_imports, mut hh_excise) = (0i128, 0i128, 0i128);
        for j in 0..n_i {
            let (mut dom_net, mut imp_net) = (paid_parts[4 * j], paid_parts[4 * j + 2]);
            hh_vat += paid_parts[4 * j + 1] + paid_parts[4 * j + 3];
            // What the amounts before VAT bought, home at the price, imported
            // at the import price, each with the duty on top; the duty is the
            // government's, the rest the seller's.
            let d = self.excise_of(j);
            let p = i128::from(self.industries.price_ppm[j].max(1));
            let pm = self.import_index().max(1);
            let (q_dom, q_imp) = (dom_net * ONE / (p + d), imp_net * ONE / (pm + d));
            let duty = if d > 0 {
                let (dd, di) = (ratio(q_dom, d, ONE), ratio(q_imp, d, ONE));
                (dom_net, imp_net) = (dom_net - dd, imp_net - di);
                dd + di
            } else {
                0
            };
            hh_excise += duty;
            #[cfg(feature = "revenue")]
            self.rev_household_good(j, q_dom + q_imp, duty);
            #[cfg(not(feature = "revenue"))]
            let _ = (q_dom, q_imp);
            firms_imports += imp_net;
            if dom_net > 0 {
                let w: Vec<i128> = units_of[j]
                    .iter()
                    .map(|&u| i128::from(self.units.sales_today[u]))
                    .collect();
                let revenue = split(dom_net, &w);
                for (&u, &r) in units_of[j].iter().zip(&revenue) {
                    self.units.deposits[u] += bani(r);
                    self.units.revenue_today[u] += bani(r);
                    self.fs.add_sales(u, r);
                    #[cfg(feature = "revenue")]
                    self.rev_household_sale(j, u, r);
                }
                self.industries.sales_value_today[j] += bani(dom_net);
                self.mac.month.sales_value += dom_net;
            }
        }
        self.mac.month.consumption += paid_total;
        // Excise counts with VAT among the taxes on products.
        self.mac.month.vat += hh_vat + hh_excise;
        let firms_vat = hh_vat;
        self.mac.month.imports += firms_imports;
        post.cash(
            Sector::Households,
            Sector::Firms,
            bani(by_cash),
            FlowCode::CONSUMPTION,
        );
        post.deposits(
            Sector::Households,
            Sector::Firms,
            bani(by_deposits),
            FlowCode::CONSUMPTION,
        );
        post.deposits(
            Sector::Firms,
            Sector::RestOfWorld,
            bani(firms_imports),
            FlowCode::IMPORTS,
        );
        post.pays_gov(Sector::Firms, bani(firms_vat), FlowCode::TAX_VAT);
        post.cash_into_bank(bani(by_cash));
        post.commit(&mut self.ledger);
        #[cfg(feature = "revenue")]
        self.rev_post_products();
    }
}
