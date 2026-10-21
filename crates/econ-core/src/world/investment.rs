//! Investment and capital on the `World` (feature `investment`; spec
//! `economy/investment-capital`, "First version", roadmap M3): the capital
//! book of each firm unit, its investment budget, its monthly profit and its
//! dividends, and the government's investment.
//! - **Each unit's plan day (step 1):** the month's budget by
//!   `desired_investment`, as far as spare money and credit room finance it;
//!   public producers and units without capital invest nothing.
//! - **Each day (steps 2, 3):** the units' budgets and the government's
//!   investment (its budget lines, at fixed real levels) are spent over the
//!   goods by the investment column's starting shares; the domestic part is
//!   demand in the goods market, the imported part paid to the rest of the
//!   world; each unit pays by its budget, the government under
//!   `gov.investment` and `imports`.
//!
//! It acts under a law with the income taxes of 2021, as the treasury does.
//! - **Each day:** what each unit pays for investment goods is recorded.
//! - **Monthly close (steps 4, 6, 7, 8):** each market unit's profit of the
//!   month (value of output less inputs, labour cost, depreciation, net
//!   interest, the mixed income it paid and the accrued tax); its capital
//!   book closes the month; the depreciation goes into the quarter's
//!   profit-tax base; capacity follows capital; expected growth follows
//!   realised sales; productivity grows by `TFP_GROWTH_PPM`.
//! - **The 15th (step 5):** last month's profit paid out by
//!   `econ_mech_investment::dividend` and split by ownership: the state's
//!   share to the government, the foreign share to the rest of the world,
//!   the domestic share to pension funds (their share of the domestic
//!   equity at the start) and households (by their money at load), less the
//!   dividend tax, which the unit withholds and pays the government.
//!
//! > **Simplification (M3):** the starting work in progress is a month of
//! > depreciation per slot; the price of investment goods is taken as one,
//! > so real capital is the value bought.

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_mech_investment::{CapitalBook, Controller, InvestParams, dividend, split_by_ownership};
use econ_types::Bani;

use super::World;
use super::money::{Postings, bani, ratio};
use super::observe::{InvestmentMonth, InvestmentPlans};

/// The dividend tax, millionths (5% in 2021, spec economy/taxation step 10).
const DIVIDEND_TAX_PPM: i128 = 50_000;
/// The day of the month dividends are paid.
const DIVIDEND_DAY: u8 = 15;
/// `tfp_base_growth`, a year (production's 1%; spec open question 7: run
/// at 1% and at 0%, keep 1% if the quiet baseline holds).
const TFP_GROWTH_PPM: i128 = 0;
/// Whether the investment rule (steps 1 to 3, 6 to 8) acts. **Off until
/// the calibration of M7** (owner, 2026-10-21): in the `World` under the 2021
/// law it deepened the slump (unemployment 14-16% against 7-10% without it),
/// because units run below the utilisation the rule takes as normal and cut
/// investment, which cuts demand. While it is off the units buy M2's
/// investment at fixed real levels (and pay the government's part), and
/// capital, profit and dividends work as with it. Switch it on and tune it
/// in M7 (roadmap).
pub(super) const INVESTMENT_RULE: bool = false;

/// The firms' loan rate when there are no banks: the starting policy rate
/// plus money and banking's spread for firms.
const LOAN_RATE_NO_BANKS_PPM: i32 = 47_500;

/// Each firm unit's capital and what it paid out, after the latest day.
/// Money in bani, capital in real bani; aligned with
/// [`super::FirmUnitColumns`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InvestmentState {
    /// Capital in place.
    pub capital: Vec<i64>,
    /// Depreciation of the latest monthly close.
    pub depreciation: Vec<i64>,
    /// Profit after the accrued tax of the latest closed month: what the next
    /// dividends are paid from.
    pub profit_after_tax: Vec<Bani>,
    /// Dividends each unit paid on the latest day.
    pub dividends_today: Vec<Bani>,
    /// The latest day's dividends by receiving sector, in matrix order of
    /// [`Sector::ALL`].
    pub dividends_by_sector: [Bani; 8],
    /// The dividend tax withheld on the latest day.
    pub dividend_tax_today: Bani,
}

/// What the `World` keeps for investment and capital.
#[derive(Debug, Clone, Default)]
pub(super) struct Investment {
    active: bool,
    view: InvestmentState,
    books: Vec<CapitalBook>,
    /// What each unit paid for investment goods this month.
    spent_month: Vec<i128>,
    /// Each unit's money over its monthly costs at the start, millionths.
    cash_ratio_ppm: Vec<i128>,
    /// Each unit's inputs and labour cost of the latest closed month.
    costs_last: Vec<i128>,
    controller: Vec<Controller>,
    ownership: Vec<[u32; 3]>,
    /// Households' key to the domestic dividends (their money at load).
    hh_key: Vec<i128>,
    /// Pension funds' share of the domestic dividends, millionths.
    pf_share_ppm: i128,
    /// The investment rule.
    rule: Rule,
    /// The views of `observe.rs`: each closed month, each unit's latest plan.
    months: Vec<InvestmentMonth>,
    plans: InvestmentPlans,
    /// The month's investment in GDP by expenditure (from the close), and
    /// the government's investment of the month.
    gdp_month: i128,
    gov_month: i128,
}

/// What the investment rule keeps (steps 1 to 3, 6 to 8).
#[derive(Debug, Clone, Default)]
struct Rule {
    /// Each unit's spending a day this month.
    budget_day: Vec<i128>,
    /// Each unit's expected growth, millionths a year.
    growth: Vec<i32>,
    /// Each unit's capital per unit of capacity a month, millionths.
    kappa: Vec<i128>,
    /// Each unit's utilisation at the last three closes, millionths.
    cu: Vec<Vec<i64>>,
    /// Each unit's output and sales this month, real.
    out_month: Vec<i64>,
    sales_month: Vec<i64>,
    /// Each unit's real sales of the last thirteen months.
    sales_year: Vec<Vec<i64>>,
    /// The investment column's starting shares by good, millionths of the
    /// day's value: domestic and imported.
    dom_share: Vec<i128>,
    imp_share: Vec<i128>,
    /// The government's investment a day.
    gov_day: i128,
    /// The real loan rate of the start.
    rho0: i32,
    /// The productivity index, millionths.
    tfp: i128,
    /// Today's spending and its government part.
    today_total: i128,
    today_gov: i128,
}

impl World {
    /// Each firm unit's capital and dividends.
    #[must_use]
    pub fn investment(&self) -> &InvestmentState {
        &self.inv.view
    }

    /// The parameters of investment and capital.
    fn invest_params(&self) -> InvestParams {
        InvestParams::spec_defaults()
    }

    /// At load: the capital books, the cash ratios, the owners.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn start_investment(
        &mut self,
        fixed_assets: &[Bani],
        controller: &[u8],
        ownership: &[[u32; 3]],
        output: &[Bani],
        value_added: &[Bani],
        wages: &[Bani],
        gov_investment_year: Bani,
    ) {
        let n_u = self.units.industry.len();
        let active = self.law.income.is_some() && fixed_assets.len() == n_u;
        self.inv = Investment {
            active,
            ..Investment::default()
        };
        if !active {
            return;
        }
        let p = self.invest_params();
        let mut books = Vec::with_capacity(n_u);
        let mut cash_ratio = Vec::with_capacity(n_u);
        for u in 0..n_u {
            let k = fixed_assets[u].get();
            let start = CapitalBook::new(k, 0, &p);
            let month_dep = start.depreciation(&p);
            books.push(CapitalBook::new(k, month_dep, &p));
            let costs = (i128::from(output[u].get()) - i128::from(value_added[u].get())
                + i128::from(wages[u].get()))
                / 12;
            cash_ratio.push(if costs > 0 {
                i128::from(self.units.deposits[u].get()) * 1_000_000 / costs
            } else {
                -1
            });
        }
        let l = &self.ledger;
        let (hh_eq, pf_eq) = (
            i128::from(
                l.balance(Sector::Households, Instrument::Equity)
                    .get()
                    .max(0),
            ),
            i128::from(
                l.balance(Sector::PensionFunds, Instrument::Equity)
                    .get()
                    .max(0),
            ),
        );
        let i = &mut self.inv;
        i.books = books;
        i.cash_ratio_ppm = cash_ratio;
        i.spent_month = vec![0; n_u];
        i.costs_last = vec![0; n_u];
        i.controller = controller
            .iter()
            .map(|&c| match c {
                1 => Controller::Foreign,
                2 => Controller::State,
                _ => Controller::Domestic,
            })
            .collect();
        i.ownership = ownership.to_vec();
        i.hh_key = (0..self.households.hh_weight.len())
            .map(|h| {
                i128::from(
                    (self.households.hh_cash[h] + self.households.hh_deposits[h])
                        .get()
                        .max(0),
                )
            })
            .collect();
        i.pf_share_ppm = if hh_eq + pf_eq > 0 {
            pf_eq * 1_000_000 / (hh_eq + pf_eq)
        } else {
            0
        };
        let v = &mut i.view;
        v.capital = i.books.iter().map(|b| b.capital).collect();
        v.depreciation = vec![0; n_u];
        v.profit_after_tax = vec![Bani::ZERO; n_u];
        v.dividends_today = vec![Bani::ZERO; n_u];
        i.plans = InvestmentPlans {
            desired: vec![Bani::ZERO; n_u],
            budget: vec![Bani::ZERO; n_u],
            spare: vec![Bani::ZERO; n_u],
            credit_room: vec![Bani::ZERO; n_u],
        };
        self.start_rule(gov_investment_year);
    }

    /// The rule at load: the column's shares, the government's part, each
    /// unit's starting investment and expected growth (spec, "Starting
    /// state").
    fn start_rule(&mut self, gov_investment_year: Bani) {
        let p = self.invest_params();
        let n_u = self.units.industry.len();
        let dom: Vec<i128> = self.ii.inv_q.iter().map(|&q| i128::from(q)).collect();
        let imp: Vec<i128> = self
            .ii
            .inv_imp
            .iter()
            .map(|b| i128::from(b.get()))
            .collect();
        let v0: i128 = dom.iter().sum::<i128>() + imp.iter().sum::<i128>();
        let gov_day = (i128::from(gov_investment_year.get()) / 365).clamp(0, v0);
        let firms_year = (v0 - gov_day) * 365;
        let market: Vec<bool> = (0..n_u).map(|u| !self.uu.public[u]).collect();
        let total_k: i128 = (0..n_u)
            .filter(|&u| market[u])
            .map(|u| i128::from(self.inv.books[u].capital.max(0)))
            .sum();
        let r = &mut self.inv.rule;
        r.dom_share = dom
            .iter()
            .map(|&q| if v0 > 0 { q * 1_000_000 / v0 } else { 0 })
            .collect();
        r.imp_share = imp
            .iter()
            .map(|&q| if v0 > 0 { q * 1_000_000 / v0 } else { 0 })
            .collect();
        r.gov_day = gov_day;
        r.tfp = 1_000_000;
        r.growth = vec![0; n_u];
        r.kappa = vec![0; n_u];
        r.budget_day = vec![0; n_u];
        r.cu = vec![Vec::new(); n_u];
        r.out_month = vec![0; n_u];
        r.sales_month = vec![0; n_u];
        r.sales_year = vec![Vec::new(); n_u];
        for u in 0..n_u {
            let k = i128::from(self.inv.books[u].capital.max(0));
            if !market[u] || k == 0 || total_k == 0 {
                continue;
            }
            let i0_year = firms_year * k / total_k;
            r.growth[u] = econ_mech_investment::starting_growth_ppm(
                bani(i0_year),
                self.inv.books[u].capital,
                &p,
            );
            r.budget_day[u] = i0_year / 365;
            let cap = i128::from(self.units.capacity[u]);
            if cap > 0 {
                r.kappa[u] = k * 1_000_000 / cap;
            }
        }
        let rho0 = self.invest_loan_rate() - self.mac.expected_inflation_ppm;
        self.inv.rule.rho0 = rho0;
    }

    /// The firms' loan rate: the banks' when they act.
    fn invest_loan_rate(&self) -> i32 {
        #[cfg(feature = "banking")]
        if self.banks_active() {
            return i32::try_from(self.bank().rates.firms_ppm).unwrap_or(i32::MAX);
        }
        LOAN_RATE_NO_BANKS_PPM
    }

    /// Whether the investment rule acts (see [`INVESTMENT_RULE`]).
    pub(super) fn invest_rule_active(&self) -> bool {
        self.inv.active && INVESTMENT_RULE
    }

    /// Step 1, on the unit's plan day: the month's budget.
    /// The plan is computed and reported whether or not the rule acts; only
    /// the rule spends it.
    pub(super) fn invest_budget(&mut self, u: usize, days: u8) {
        if !self.inv.active || self.uu.public[u] || self.inv.books[u].capital <= 0 {
            return;
        }
        let p = self.invest_params();
        let r = &self.inv.rule;
        let cu = if r.cu[u].is_empty() {
            i64::from(p.cu_target_ppm)
        } else {
            r.cu[u].iter().sum::<i64>() / i64::try_from(r.cu[u].len()).expect("3")
        };
        let gap = (self.invest_loan_rate() - self.mac.expected_inflation_ppm) - r.rho0;
        let desired = econ_mech_investment::desired_investment(
            self.inv.books[u].capital,
            1_000_000,
            u32::try_from(cu.clamp(0, 2_000_000)).expect("a share"),
            gap,
            r.growth[u],
            &p,
        );
        let spare =
            bani((i128::from(self.units.deposits[u].get()) - self.uu.wage_reserve[u]).max(0));
        #[cfg(feature = "banking")]
        let room = self.bank_credit_room(u);
        #[cfg(not(feature = "banking"))]
        let room = Bani::ZERO;
        let month = econ_mech_investment::financed(desired, spare, room);
        let v = &mut self.inv.plans;
        v.desired[u] = desired;
        v.budget[u] = month;
        v.spare[u] = spare;
        v.credit_room[u] = room;
        if self.invest_rule_active() {
            self.inv.rule.budget_day[u] = i128::from(month.get()) / i128::from(days.max(1));
        }
    }

    /// Whether investment and capital act (plans are reported even with the
    /// rule off).
    pub(super) fn invest_active(&self) -> bool {
        self.inv.active
    }

    /// The month's investment in GDP by expenditure, at the close.
    pub(super) fn invest_note_gdp(&mut self, investment: i128) {
        self.inv.gdp_month = investment;
    }

    /// The government paid `amount` of its own investment today.
    pub(super) fn invest_gov_spent(&mut self, amount: i128) {
        if self.inv.active {
            self.inv.gov_month += amount;
        }
    }

    /// The views of `observe.rs`.
    pub(super) fn invest_months_view(&self) -> &[InvestmentMonth] {
        &self.inv.months
    }
    pub(super) fn invest_plans_view(&self) -> &InvestmentPlans {
        &self.inv.plans
    }

    /// Steps 2 and 3, before the day's goods market: the day's investment
    /// demand by good, the units' budgets and the government's part.
    pub(super) fn invest_demand_today(&mut self) {
        if !self.invest_rule_active() {
            return;
        }
        let r = &mut self.inv.rule;
        let units: i128 = r.budget_day.iter().sum();
        let total = units + r.gov_day;
        r.today_total = total;
        r.today_gov = r.gov_day;
        for g in 0..self.ii.inv_q.len() {
            let value = total * r.dom_share[g] / 1_000_000;
            let price = i128::from(self.industries.price_ppm[g].max(1));
            self.ii.inv_q[g] = i64::try_from(value * 1_000_000 / price).expect("a quantity");
            self.ii.inv_imp[g] = bani(total * r.imp_share[g] / 1_000_000);
        }
    }

    /// Who pays today's investment: the government's share, millionths, and
    /// the units' weights (their budgets); `None` when the rule does not act.
    pub(super) fn invest_payers(&self) -> Option<(i128, Vec<i128>)> {
        if !self.invest_rule_active() {
            return None;
        }
        let r = &self.inv.rule;
        let gov = if r.today_total > 0 {
            r.today_gov * 1_000_000 / r.today_total
        } else {
            0
        };
        Some((gov, r.budget_day.clone()))
    }

    /// After the day's last phase: each unit's real output and sales into
    /// the month's.
    pub(super) fn invest_day_end(&mut self) {
        if !self.inv.active {
            return;
        }
        let r = &mut self.inv.rule;
        for u in 0..r.out_month.len() {
            r.out_month[u] += self.units.output_today[u];
            r.sales_month[u] += self.units.sales_today[u];
        }
    }

    /// Steps 6 to 8 at the close: utilisation, capacity from capital,
    /// expected growth, productivity.
    fn rule_close(&mut self) {
        let p = self.invest_params();
        self.inv.rule.tfp += self.inv.rule.tfp * TFP_GROWTH_PPM / 12 / 1_000_000;
        let tfp = self.inv.rule.tfp;
        for u in 0..self.units.industry.len() {
            let r = &mut self.inv.rule;
            let out = std::mem::take(&mut r.out_month[u]);
            let sold = std::mem::take(&mut r.sales_month[u]);
            if self.uu.public[u] {
                continue;
            }
            let cap = self.units.capacity[u];
            if cap > 0 {
                let cu = i64::try_from(i128::from(out) * 1_000_000 / i128::from(cap))
                    .unwrap_or(i64::MAX);
                r.cu[u].push(cu);
                if r.cu[u].len() > 3 {
                    let _ = r.cu[u].remove(0);
                }
            }
            r.sales_year[u].push(sold);
            if r.sales_year[u].len() > 13 {
                let _ = r.sales_year[u].remove(0);
            }
            if r.sales_year[u].len() == 13 && r.sales_year[u][0] > 0 {
                let realised = (i128::from(r.sales_year[u][12]) - i128::from(r.sales_year[u][0]))
                    * 1_000_000
                    / i128::from(r.sales_year[u][0]);
                r.growth[u] = econ_mech_investment::next_expected_growth_ppm(
                    r.growth[u],
                    i32::try_from(realised).unwrap_or(0),
                    &p,
                );
            }
            if r.kappa[u] > 0 {
                let k = i128::from(self.inv.books[u].capital.max(0));
                self.units.capacity[u] = i64::try_from(tfp * k / r.kappa[u]).unwrap_or(i64::MAX);
            }
        }
        self.uu.labour_factor_ppm =
            u32::try_from(1_000_000_000_000 / tfp.max(1)).unwrap_or(u32::MAX);
    }

    /// A unit paid `amount` for investment goods today.
    pub(super) fn invest_spent(&mut self, u: usize, amount: i128) {
        if self.inv.active {
            self.inv.spent_month[u] += amount;
        }
    }

    /// The start of a day: the day's columns emptied; on the 15th, last
    /// month's profit paid out (step 5).
    pub(super) fn invest_begin_day(&mut self) {
        if !self.inv.active {
            return;
        }
        let v = &mut self.inv.view;
        v.dividends_today.iter_mut().for_each(|d| *d = Bani::ZERO);
        v.dividends_by_sector = [Bani::ZERO; 8];
        v.dividend_tax_today = Bani::ZERO;
        if self.calendar.date(self.today).day == DIVIDEND_DAY && !self.months.is_empty() {
            self.pay_dividends();
        }
    }

    /// Monthly close: each market unit's profit and capital (steps 4, 6).
    pub(super) fn invest_close(&mut self) {
        if !self.inv.active {
            return;
        }
        let p = self.invest_params();
        let threshold = self
            .law
            .income
            .as_ref()
            .map(|l| (l.profit.micro_threshold, l.profit));
        let n_u = self.units.industry.len();
        let mut spent_view = vec![Bani::ZERO; n_u];
        let mut profit_view = vec![Bani::ZERO; n_u];
        for u in 0..n_u {
            let [value, inputs, labour, sales, mixed] = self.fs.take_month(u);
            let spent = std::mem::take(&mut self.inv.spent_month[u]);
            spent_view[u] = bani(spent);
            let dep = self.inv.books[u].depreciation(&p);
            let (dep_month, _) = self.inv.books[u]
                .close_month(i64::try_from(spent).expect("a month of investment"), &p);
            debug_assert_eq!(dep, dep_month);
            self.fs.add_depreciation(u, i128::from(dep));
            self.inv.view.capital[u] = self.inv.books[u].capital;
            self.inv.view.depreciation[u] = dep;
            self.inv.costs_last[u] = inputs + labour;
            if self.uu.public[u] {
                self.inv.view.profit_after_tax[u] = Bani::ZERO;
                continue;
            }
            #[cfg(feature = "banking")]
            let interest = if self.banks_active() {
                let b = self.bank();
                i128::from((b.unit_interest_paid[u] - b.unit_interest_received[u]).get())
            } else {
                0
            };
            #[cfg(not(feature = "banking"))]
            let interest = 0i128;
            let pre = value - inputs - labour - i128::from(dep) - interest - mixed;
            profit_view[u] = bani(pre);
            // The accrued tax: the profit rate on a profit, the micro rate on
            // the month's sales of a micro unit.
            let tax = match &threshold {
                Some((limit, law)) if self.fs.is_micro(u, *limit) => {
                    let bp = if self.units.staff[u] > 0 {
                        law.micro_with_staff_bp
                    } else {
                        law.micro_no_staff_bp
                    };
                    ratio(sales.max(0), i128::from(bp), 10_000)
                }
                Some((_, law)) => ratio(pre.max(0), i128::from(law.profit_bp), 10_000),
                None => 0,
            };
            self.inv.view.profit_after_tax[u] = bani(pre - tax);
        }
        if self.invest_rule_active() {
            self.rule_close();
        }
        let private: Bani = spent_view.iter().copied().fold(Bani::ZERO, |a, b| a + b);
        let month =
            econ_types::Month(u32::try_from(self.months.len().saturating_sub(1)).expect("months"));
        let government = bani(std::mem::take(&mut self.inv.gov_month));
        self.inv.months.push(InvestmentMonth {
            month,
            spent_real: spent_view.iter().map(|b| b.get()).collect(),
            spent: spent_view,
            private,
            government,
            gdp_investment: bani(self.inv.gdp_month),
            price_index_ppm: 1_000_000,
            profit_before_tax: profit_view,
        });
    }

    /// Step 5: every market unit's dividend, split by its owners.
    fn pay_dividends(&mut self) {
        let p = self.invest_params();
        let reserve = self.wage_reserves();
        let n_u = self.units.industry.len();
        let (mut to_gov, mut to_row, mut to_pf, mut to_hh) = (0i128, 0i128, 0i128, 0i128);
        for u in 0..n_u {
            if self.uu.public[u] {
                continue;
            }
            let deposits = self.units.deposits[u];
            let ratio_ppm = self.inv.cash_ratio_ppm[u];
            let target = if ratio_ppm < 0 {
                deposits
            } else {
                bani(ratio(self.inv.costs_last[u], ratio_ppm, 1_000_000))
            };
            let spare = bani((i128::from(deposits.get()) - reserve[u]).max(0));
            let d = dividend(
                self.inv.view.profit_after_tax[u],
                deposits,
                target,
                spare,
                self.inv.controller[u],
                &p,
            );
            if d <= Bani::ZERO {
                continue;
            }
            let [state, domestic, foreign] = {
                let o = self.inv.ownership[u];
                split_by_ownership(d, o[0], o[1], o[2])
            };
            let pf = ratio(i128::from(domestic.get()), self.inv.pf_share_ppm, 1_000_000);
            to_gov += i128::from(state.get());
            to_row += i128::from(foreign.get());
            to_pf += pf;
            to_hh += i128::from(domestic.get()) - pf;
            self.units.deposits[u] -= d;
            self.inv.view.dividends_today[u] = d;
        }
        // Households: their share by their key, exactly in whole bani per
        // record; what they cannot take in whole bani goes to the pension
        // funds.
        let mut hh_net = 0i128;
        let mut tax = 0i128;
        let mut hh_gross = 0i128;
        if to_hh > 0
            && let Some(parts) =
                super::treasury::weighted_parts(to_hh, &self.households.hh_weight, &self.inv.hh_key)
        {
            for (h, &a) in parts.iter().enumerate() {
                if a == 0 {
                    continue;
                }
                let t = ratio(a, DIVIDEND_TAX_PPM, 1_000_000);
                let net = bani(a - t);
                let w = i128::from(self.households.hh_weight[h]);
                let hh = &mut self.households;
                hh.hh_deposits[h] += net;
                hh.hh_income_received[h] += net;
                hh.income_to_date[h] += net;
                self.hh.income_this_month[h] += net;
                hh_net += w * i128::from(net.get());
                tax += w * t;
                hh_gross += w * a;
            }
        }
        to_pf += to_hh - hh_gross;
        let mut post = Postings::default();
        let code = FlowCode::INCOME_DIVIDEND;
        post.pays_gov(Sector::Firms, bani(to_gov), code);
        post.deposits(Sector::Firms, Sector::RestOfWorld, bani(to_row), code);
        post.deposits(Sector::Firms, Sector::PensionFunds, bani(to_pf), code);
        post.deposits(Sector::Firms, Sector::Households, bani(hh_net), code);
        post.pays_gov(Sector::Firms, bani(tax), FlowCode::TAX_DIVIDEND);
        post.commit(&mut self.ledger);
        let v = &mut self.inv.view;
        v.dividends_by_sector[Sector::Government as usize] = bani(to_gov);
        v.dividends_by_sector[Sector::RestOfWorld as usize] = bani(to_row);
        v.dividends_by_sector[Sector::PensionFunds as usize] = bani(to_pf);
        v.dividends_by_sector[Sector::Households as usize] = bani(hh_net);
        v.dividend_tax_today = bani(tax);
    }

    /// Investment's state for the state hash.
    pub(super) fn invest_hash(&self, write: &mut impl FnMut(u64)) {
        if !self.inv.active {
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        for (b, s) in self.inv.books.iter().zip(&self.inv.spent_month) {
            write(b.capital as u64);
            for &x in &b.in_progress {
                write(x as u64);
            }
            write(*s as u64);
        }
        #[allow(clippy::cast_sign_loss)]
        {
            let r = &self.inv.rule;
            write(r.tfp as u64);
            for u in 0..r.budget_day.len() {
                write(r.budget_day[u] as u64 ^ (r.growth[u] as u64).rotate_left(32));
                write(r.out_month[u] as u64 ^ (r.sales_month[u] as u64).rotate_left(21));
            }
        }
        #[allow(clippy::cast_sign_loss)]
        for (p, c) in self
            .inv
            .view
            .profit_after_tax
            .iter()
            .zip(&self.inv.costs_last)
        {
            write(p.get() as u64 ^ (*c as u64).rotate_left(29));
        }
    }
}
