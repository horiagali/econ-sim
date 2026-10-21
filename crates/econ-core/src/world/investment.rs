//! Investment and capital on the `World` (feature `investment`; spec
//! `economy/investment-capital`, "First version", roadmap M3): the capital
//! book of each firm unit, its monthly profit and its dividends. The
//! investment rule (steps 1, 2, 7 and 8) comes later; until then the units
//! keep buying M2's investment at fixed real levels, and what each one buys
//! goes into its capital after the gestation.
//!
//! It acts under a law with the income taxes of 2021, as the treasury does.
//! - **Each day:** what each unit pays for investment goods is recorded.
//! - **Monthly close (step 4 and 6):** each market unit's profit of the
//!   month (value of output less inputs, labour cost, depreciation, net
//!   interest, the mixed income it paid and the accrued tax); its capital
//!   book closes the month; the depreciation goes into the quarter's
//!   profit-tax base.
//! - **The 15th (step 5):** last month's profit paid out by
//!   `econ_mech_investment::dividend` and split by ownership: the state's
//!   share to the government, the foreign share to the rest of the world,
//!   the domestic share to pension funds (their share of the domestic
//!   equity at the start) and households (by their money at load), less the
//!   dividend tax, which the unit withholds and pays the government.
//!
//! > **Simplification (M3, until the investment rule):** the starting work in
//! > progress is a month of depreciation per slot; the price of investment
//! > goods is taken as one, so real capital is the value bought.

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_mech_investment::{CapitalBook, Controller, InvestParams, dividend, split_by_ownership};
use econ_types::Bani;

use super::World;
use super::money::{Postings, bani, ratio};

/// The dividend tax, millionths (5% in 2021, spec economy/taxation step 10).
const DIVIDEND_TAX_PPM: i128 = 50_000;
/// The day of the month dividends are paid.
const DIVIDEND_DAY: u8 = 15;

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
    pub(super) fn start_investment(
        &mut self,
        fixed_assets: &[Bani],
        controller: &[u8],
        ownership: &[[u32; 3]],
        output: &[Bani],
        value_added: &[Bani],
        wages: &[Bani],
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
        for u in 0..self.units.industry.len() {
            let [value, inputs, labour, sales, mixed] = self.fs.take_month(u);
            let spent = std::mem::take(&mut self.inv.spent_month[u]);
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
