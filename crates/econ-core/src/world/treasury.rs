//! The government's debt and its budget report on the `World` (feature
//! `fiscal`; spec `economy/fiscal-policy`, "First version", roadmap M3): its
//! parameters and read-only state for the `[sim]` criteria AC-FIS-02, 03
//! and 05, and the month-end steps that move them.
//!
//! The treasury acts under a law with the income taxes of 2021 (the
//! placeholder law of M2 keeps M2's government, whose deposits at the central
//! bank may fall below zero) and a scenario with a debt profile. At each
//! month end, after the month's statistics and the settling of the units'
//! shortfalls, in this order: a twelfth of the budget lines the `World` does
//! not pay through its phases, the coupon to every holder, the bonds that fall
//! due, then new issues (or early redemptions) that bring the government's
//! deposits at the central bank back to `gov_cash_target`. The yield of the
//! month's issues is set first, from the policy rate and the debt over the
//! last twelve months' GDP.
//!
//! > **Simplification (M3, until trade):** the rest of the world pays for its
//! > share of new issues out of its lei deposits, as the spec has every buyer
//! > do; those deposits may go below zero (a lei overdraft at Romanian banks)
//! > until the central bank's intervention of spec `economy/trade-fx`
//! > converts them against its foreign assets.

use std::collections::BTreeMap;

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_mech_fiscal::{DebtBook, DebtParams, bond_yield_ppm, split_by_holdings};
use econ_types::{Bani, Month};

use super::World;
use super::money::{Postings, bani};

/// The debt's parameters (spec, "Tuning parameters").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FiscalParams {
    /// New issues and the yield.
    pub debt: DebtParams,
    /// `starting_coupon`, millionths a year (4.5%).
    pub starting_coupon_ppm: u32,
    /// `gov_cash_target`, in months of spending (1).
    pub cash_target_months: u32,
}

impl FiscalParams {
    /// The spec's defaults.
    #[must_use]
    pub fn spec_defaults() -> FiscalParams {
        FiscalParams {
            debt: DebtParams::spec_defaults(),
            starting_coupon_ppm: 45_000,
            cash_target_months: 1,
        }
    }
}

/// The government's debt after the latest day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Treasury {
    /// Bonds outstanding by year of maturity and their average coupon.
    pub book: DebtBook,
    /// The yield of new issues, set at the latest monthly close, millionths a
    /// year.
    pub bond_yield_ppm: i32,
    /// `gov_cash_target`: the deposits at the central bank it keeps.
    pub cash_target: Bani,
    /// Bonds that fell due at the latest month end.
    pub matured: Bani,
    /// Bonds redeemed at the latest month end, at maturity.
    pub redeemed: Bani,
    /// Bonds issued at the latest month end (negative: redeemed early).
    pub issued: Bani,
}

/// The government's budget of a closed month: every flow of the month to or
/// from the government outside its financing, by flow code, national bani.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiscalMonth {
    /// The month.
    pub month: Month,
    /// Receipts by flow code (taxes, contributions, the central bank's
    /// profit, …), ascending by code; each positive.
    pub revenue: Vec<(FlowCode, Bani)>,
    /// Payments by flow code (purchases, transfers, budget lines), ascending
    /// by code, interest excluded; each positive.
    pub spending: Vec<(FlowCode, Bani)>,
    /// Interest on the debt (`interest.gov_debt`).
    pub interest: Bani,
    /// Spending + interest − revenue.
    pub deficit: Bani,
}

/// What the `World` keeps for the treasury.
#[derive(Debug, Clone)]
pub(super) struct TreasuryState {
    /// Whether the treasury acts (see the module's documentation).
    active: bool,
    view: Treasury,
    months: Vec<FiscalMonth>,
    /// The government's flows of the month so far by code, received
    /// positive, financing excluded.
    month_flows: BTreeMap<FlowCode, i128>,
    /// Month ends passed since the start.
    month_ends: u32,
    /// Whether today is a month end the treasury closed (its report is
    /// written after the day's last phase).
    closing: Option<Month>,
    /// The budget lines it pays: code, sector paid, yearly amount.
    transfers: Vec<(FlowCode, Sector, Bani)>,
    /// For how many month ends the central bank buys the whole issue (a
    /// start edit; monetary financing).
    cb_financing_months: u32,
}

impl Default for TreasuryState {
    fn default() -> Self {
        TreasuryState {
            active: false,
            view: Treasury {
                book: DebtBook {
                    by_year: Vec::new(),
                    coupon_ppm: 0,
                },
                bond_yield_ppm: 0,
                cash_target: Bani::ZERO,
                matured: Bani::ZERO,
                redeemed: Bani::ZERO,
                issued: Bani::ZERO,
            },
            months: Vec::new(),
            month_flows: BTreeMap::new(),
            month_ends: 0,
            closing: None,
            transfers: Vec::new(),
            cb_financing_months: 0,
        }
    }
}

/// Whether a flow code is a financial transaction, outside the budget.
fn is_financing(c: FlowCode) -> bool {
    c.name().starts_with("finance.")
}

/// `x · a + y · b = r` with `x, y ≥ 0`, for coprime `a` and `b` and
/// `r ≥ (a − 1)(b − 1)`.
fn two_coins(r: i128, a: i128, b: i128) -> Option<(i128, i128)> {
    (0..a).find_map(|y| {
        let rest = r - y * b;
        (rest >= 0 && rest % a == 0).then_some((rest / a, y))
    })
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Per-record amounts `p[h] ≥ 0` with `Σ w[h] · p[h] = total` exactly, in
/// proportion to `w[h] · base[h]` as near as whole bani allow: each record its
/// share of `total` less a reserve, rounded down; one more ban to the records
/// with the largest remainders while the reserve stays; the rest, at least
/// the reserve, as `x · a + y · b` to the two records with the largest base
/// whose weights `a`, `b` are coprime. `None` when there are no such two
/// records or `total` is below the reserve.
pub(super) fn weighted_parts(total: i128, weights: &[u32], base: &[i128]) -> Option<Vec<i128>> {
    let n = weights.len();
    let mut order: Vec<usize> = (0..n).filter(|&h| base[h] > 0).collect();
    order.sort_by_key(|&h| (std::cmp::Reverse(base[h]), h));
    let &first = order.first()?;
    let a = i128::from(weights[first]);
    let &second = order
        .iter()
        .skip(1)
        .find(|&&h| gcd(a, i128::from(weights[h])) == 1)?;
    let b = i128::from(weights[second]);
    let reserve = (a - 1) * (b - 1);
    if total < reserve {
        return None;
    }
    let target = total - reserve;
    let sum: i128 = (0..n)
        .map(|h| i128::from(weights[h]) * base[h].max(0))
        .sum();
    let mut parts = vec![0i128; n];
    let mut rems: Vec<(i128, usize)> = Vec::with_capacity(order.len());
    let mut left = total;
    for &h in &order {
        let x = target * base[h];
        parts[h] = x / sum;
        rems.push((x % sum, h));
        left -= i128::from(weights[h]) * parts[h];
    }
    rems.sort_by_key(|&(r, h)| (std::cmp::Reverse(r), h));
    for (_, h) in rems {
        let w = i128::from(weights[h]);
        if left - w >= reserve {
            parts[h] += 1;
            left -= w;
        }
    }
    let (x, y) = two_coins(left, a, b)?;
    parts[first] += x;
    parts[second] += y;
    Some(parts)
}

impl World {
    /// The government's debt.
    #[must_use]
    pub fn treasury(&self) -> &Treasury {
        &self.tr.view
    }

    /// The budget of every closed month, in order.
    #[must_use]
    pub fn fiscal_months(&self) -> &[FiscalMonth] {
        &self.tr.months
    }

    /// The treasury at load: the debt book from the profile, the budget
    /// lines; active under the 2021 law with a debt profile.
    pub(super) fn start_treasury(
        &mut self,
        profile: &[(u8, u8, Bani)],
        transfers: &[(FlowCode, Sector, Bani)],
    ) {
        let active = self.law.income.is_some() && !profile.is_empty();
        let debt = Bani::ZERO - self.ledger.balance(Sector::Government, Instrument::Bonds);
        let p = self.params.fiscal;
        self.tr = TreasuryState {
            active,
            transfers: transfers.to_vec(),
            ..TreasuryState::default()
        };
        if active {
            self.tr.view.book = DebtBook::from_profile(debt, profile, p.starting_coupon_ppm);
            self.ledger
                .allow_negative(Sector::RestOfWorld, Instrument::Deposits);
        }
    }

    /// The central bank buys the whole issue for the first `months` month
    /// ends (a start edit).
    #[cfg(feature = "monetary")]
    pub(super) fn treasury_cb_financing(&mut self, months: u32) {
        self.tr.cb_financing_months = months;
    }

    /// Whether the treasury acts.
    pub(super) fn tr_active(&self) -> bool {
        self.tr.active
    }

    /// The policy rate the yield is set from: the banks' when they exist,
    /// else the starting 1.75%.
    fn treasury_policy_rate(&self) -> i32 {
        #[cfg(feature = "banking")]
        {
            if self.banks_active() {
                self.bank_policy_rate()
            } else {
                self.params.policy_rate_ppm
            }
        }
        #[cfg(not(feature = "banking"))]
        {
            17_500
        }
    }

    /// The last twelve months' GDP; fewer months closed are taken at a year's
    /// rate.
    fn gdp_year(&self) -> Bani {
        let n = self.months.len().min(12);
        if n == 0 {
            return Bani::ZERO;
        }
        let sum: i128 = self.months[self.months.len() - n..]
            .iter()
            .map(|m| i128::from(m.gdp_production.get()))
            .sum();
        bani(sum * 12 / i128::try_from(n).expect("small"))
    }

    /// Phase 12, last, at a month end: the budget lines, the coupon, the
    /// bonds that fall due and the financing (spec, "In", steps 1 to 4).
    pub(super) fn treasury_payments(&mut self) {
        if !self.tr.active {
            return;
        }
        let month = Month(u32::try_from(self.months.len().saturating_sub(1)).expect("months"));
        self.tr.closing = Some(month);
        // The yield of this month's issues (step 4).
        let debt = self.tr.view.book.total();
        self.tr.view.bond_yield_ppm = bond_yield_ppm(
            self.treasury_policy_rate(),
            debt,
            self.gdp_year(),
            &self.params.fiscal.debt,
        );
        // Step 1: a twelfth of each budget line, the remainder to the first
        // months of the year.
        let k = i64::from(self.tr.month_ends % 12);
        for (code, sector, year) in self.tr.transfers.clone() {
            let (q, r) = (year.get() / 12, year.get() % 12);
            let amount = Bani(q + i64::from(k < r));
            self.gov_transfer(sector, amount, code);
        }
        // Step 2: the coupon, to each holder by its holding.
        let (holders, holdings) = self.bond_holders();
        let coupon = self.tr.view.book.month_coupon();
        let parts = split_by_holdings(coupon, &holdings);
        for (&s, &part) in holders.iter().zip(&parts) {
            self.gov_transfer(s, part, FlowCode::INTEREST_GOV_DEBT);
        }
    }

    /// Month end, last of all (after the central bank's remittance): the
    /// bonds that fall due, then the issue that brings the government's
    /// deposits back to the target (step 3). Under a central bank financing
    /// edit, the central bank buys the whole issue (step 5).
    pub(super) fn treasury_financing(&mut self) {
        if !self.tr.active || self.tr.closing.is_none() {
            return;
        }
        let (holders, holdings) = self.bond_holders();
        let matured = self.tr.view.book.mature(self.tr.month_ends);
        let parts = split_by_holdings(matured, &holdings);
        let mut redeemed = Bani::ZERO;
        for (&s, &part) in holders.iter().zip(&parts) {
            redeemed += self.redeem(s, part);
        }
        if redeemed < matured {
            // A holder whose records could not take its part keeps it: back
            // on the book, falling due next month.
            let book = &mut self.tr.view.book.by_year;
            if book.is_empty() {
                book.push(Bani::ZERO);
            }
            book[0] += matured - redeemed;
        }
        let spending = self.month_spending_so_far();
        let target =
            bani(i128::from(spending.get()) * i128::from(self.params.fiscal.cash_target_months));
        let deposits = self
            .ledger
            .balance(Sector::Government, Instrument::GovDepositsAtCb);
        let issue = target - deposits;
        let (holders, holdings) = self.bond_holders();
        let mut net = Bani::ZERO;
        if issue > Bani::ZERO {
            // Buyers by their holdings; the central bank buys none (no
            // monetary financing unless the player chooses it, step 5), or
            // all of it while a financing edit lasts.
            let cb_finances = self.tr.month_ends < self.tr.cb_financing_months;
            let buying: Vec<Bani> = holders
                .iter()
                .zip(&holdings)
                .map(|(&s, &h)| match (s == Sector::CentralBank, cb_finances) {
                    (true, true) => Bani(1),
                    (false, false) => h,
                    _ => Bani::ZERO,
                })
                .collect();
            let parts = split_by_holdings(issue, &buying);
            for (&s, &part) in holders.iter().zip(&parts) {
                net += self.buy_bonds(s, part);
            }
        } else if issue < Bani::ZERO {
            let parts = split_by_holdings(Bani::ZERO - issue, &holdings);
            for (&s, &part) in holders.iter().zip(&parts) {
                net -= self.redeem(s, part);
            }
        }
        let yield_ppm = self.tr.view.bond_yield_ppm;
        self.tr
            .view
            .book
            .issue(net, yield_ppm, &self.params.fiscal.debt);
        self.tr.view.matured = matured;
        self.tr.view.redeemed = redeemed;
        self.tr.view.issued = net;
        self.tr.view.cash_target = target;
        self.tr.month_ends += 1;
    }

    /// After the day's last phase: the government's flows of the day into the
    /// month, and the month's report on a month end.
    pub(super) fn treasury_day_end(&mut self) {
        if !self.tr.active {
            return;
        }
        for c in FlowCode::ALL {
            if is_financing(c) {
                continue;
            }
            let v = i128::from(self.ledger.flows().get(c, Sector::Government).get());
            if v != 0 {
                *self.tr.month_flows.entry(c).or_insert(0) += v;
            }
        }
        let Some(month) = self.tr.closing.take() else {
            return;
        };
        let flows = std::mem::take(&mut self.tr.month_flows);
        let (mut revenue, mut spending, mut interest) = (Vec::new(), Vec::new(), Bani::ZERO);
        for (c, v) in flows {
            if v == 0 {
                continue;
            }
            if c == FlowCode::INTEREST_GOV_DEBT {
                interest = bani(-v);
            } else if v > 0 {
                revenue.push((c, bani(v)));
            } else {
                spending.push((c, bani(-v)));
            }
        }
        let sum = |v: &[(FlowCode, Bani)]| v.iter().fold(Bani::ZERO, |a, &(_, x)| a + x);
        let deficit = sum(&spending) + interest - sum(&revenue);
        self.tr.months.push(FiscalMonth {
            month,
            revenue,
            spending,
            interest,
            deficit,
        });
    }

    /// The month's spending and interest so far, today's postings included.
    fn month_spending_so_far(&self) -> Bani {
        let mut out = 0i128;
        for c in FlowCode::ALL {
            if is_financing(c) {
                continue;
            }
            let v = self.tr.month_flows.get(&c).copied().unwrap_or(0)
                + i128::from(self.ledger.flows().get(c, Sector::Government).get());
            if v < 0 {
                out -= v;
            }
        }
        bani(out)
    }

    /// The sectors that may hold government bonds and their holdings.
    fn bond_holders(&self) -> (Vec<Sector>, Vec<Bani>) {
        let holders: Vec<Sector> = Sector::ALL
            .into_iter()
            .filter(|&s| s != Sector::Government)
            .collect();
        let holdings = holders
            .iter()
            .map(|&s| self.ledger.balance(s, Instrument::Bonds).max(Bani::ZERO))
            .collect();
        (holders, holdings)
    }

    /// The government pays `amount` to a sector; households and firm units
    /// receive it in their records.
    fn gov_transfer(&mut self, sector: Sector, amount: Bani, code: FlowCode) {
        if amount <= Bani::ZERO {
            return;
        }
        let paid = self.credit_records(sector, amount, code);
        let mut post = Postings::default();
        post.gov_pays_any(sector, paid, code);
        post.commit(&mut self.ledger);
    }

    /// Bonds redeemed: the government pays the holder at book value;
    /// returns what was redeemed.
    fn redeem(&mut self, sector: Sector, amount: Bani) -> Bani {
        if amount <= Bani::ZERO {
            return Bani::ZERO;
        }
        let paid = self.credit_records(sector, amount, FlowCode::BONDS);
        let mut post = Postings::default();
        post.gov_pays_any(sector, paid, FlowCode::BONDS);
        post.bonds(sector, Sector::Government, paid);
        post.commit(&mut self.ledger);
        paid
    }

    /// A holder buys new bonds out of its deposits (banks out of reserves);
    /// returns what it bought.
    fn buy_bonds(&mut self, sector: Sector, amount: Bani) -> Bani {
        if amount <= Bani::ZERO {
            return Bani::ZERO;
        }
        let paid = self.debit_records(sector, amount);
        let mut post = Postings::default();
        post.pay_gov_any(sector, paid, FlowCode::BONDS);
        post.bonds(Sector::Government, sector, paid);
        post.commit(&mut self.ledger);
        paid
    }

    /// Credit `amount` to the records of the households or the firm units
    /// (other sectors have none), into their deposits; returns what was
    /// credited, the whole amount whenever the records can take it exactly.
    /// Households receive
    /// interest by their deposits and transfers equally per household, firm
    /// units interest by their deposits and transfers by their staff.
    fn credit_records(&mut self, sector: Sector, amount: Bani, code: FlowCode) -> Bani {
        let total = i128::from(amount.get());
        let financial = code == FlowCode::INTEREST_GOV_DEBT || code == FlowCode::BONDS;
        match sector {
            Sector::Households => {
                let base: Vec<i128> = if financial {
                    self.households
                        .hh_deposits
                        .iter()
                        .map(|d| i128::from(d.get()))
                        .collect()
                } else {
                    vec![1; self.households.hh_weight.len()]
                };
                let parts = weighted_parts(total, &self.households.hh_weight, &base)
                    .unwrap_or_else(|| vec![0; base.len()]);
                // Into deposits only: the month's income a household spends
                // from is its pay day's (AC-ST-05); these reach its spending
                // through its wealth.
                let mut paid = 0i128;
                for (h, &p) in parts.iter().enumerate() {
                    if p == 0 {
                        continue;
                    }
                    let hh = &mut self.households;
                    hh.hh_deposits[h] += bani(p);
                    paid += i128::from(hh.hh_weight[h]) * p;
                }
                bani(paid)
            }
            Sector::Firms => {
                let base: Vec<i128> = if financial {
                    self.units
                        .deposits
                        .iter()
                        .map(|d| i128::from(d.get()).max(0))
                        .collect()
                } else {
                    self.units.staff.iter().map(|&s| i128::from(s)).collect()
                };
                let parts = super::money::split(total, &base);
                let paid: i128 = parts.iter().sum();
                for (u, &p) in parts.iter().enumerate() {
                    self.units.deposits[u] += bani(p);
                }
                bani(paid)
            }
            _ => amount,
        }
    }

    /// Debit `amount` from the records of the households or the firm units
    /// by their deposits, never below zero; returns what was debited.
    fn debit_records(&mut self, sector: Sector, amount: Bani) -> Bani {
        let total = i128::from(amount.get());
        match sector {
            Sector::Households => {
                let base: Vec<i128> = self
                    .households
                    .hh_deposits
                    .iter()
                    .map(|d| i128::from(d.get()).max(0))
                    .collect();
                let Some(parts) = weighted_parts(total, &self.households.hh_weight, &base) else {
                    return Bani::ZERO;
                };
                if parts.iter().zip(&base).any(|(&p, &b)| p > b) {
                    return Bani::ZERO;
                }
                for (h, &p) in parts.iter().enumerate() {
                    self.households.hh_deposits[h] -= bani(p);
                }
                amount
            }
            Sector::Firms => {
                let base: Vec<i128> = self
                    .units
                    .deposits
                    .iter()
                    .map(|d| i128::from(d.get()).max(0))
                    .collect();
                let total = total.min(base.iter().sum());
                let parts = super::money::split(total, &base);
                for (u, &p) in parts.iter().enumerate() {
                    self.units.deposits[u] -= bani(p);
                }
                bani(total)
            }
            _ => amount,
        }
    }

    /// The treasury's state for the state hash.
    pub(super) fn treasury_hash(&self, write: &mut impl FnMut(u64)) {
        if !self.tr.active {
            return;
        }
        let t = &self.tr;
        write(u64::from(t.month_ends));
        write(u64::from(t.view.book.coupon_ppm));
        #[allow(clippy::cast_sign_loss)]
        for b in &t.view.book.by_year {
            write(b.get() as u64);
        }
        #[allow(clippy::cast_sign_loss)]
        write(t.view.cash_target.get() as u64);
        #[allow(clippy::cast_sign_loss)]
        for (c, v) in &t.month_flows {
            write(u64::from(c.0) << 48 ^ (*v as u64));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_parts_are_exact() {
        let weights = [202, 1003, 997, 450, 1000, 8438];
        let base = [10, 500, 400, 0, 300, 50];
        for total in [1_000_000i128, 4_900_000_017, 2_777_777] {
            let p = weighted_parts(total, &weights, &base).expect("parts");
            let sum: i128 = p
                .iter()
                .zip(&weights)
                .map(|(&a, &w)| a * i128::from(w))
                .sum();
            assert_eq!(sum, total);
            assert_eq!(p[3], 0, "no base, nothing");
        }
        assert!(
            weighted_parts(10, &weights, &base).is_none(),
            "below the reserve"
        );
    }
}
