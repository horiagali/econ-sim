//! The banks on the `World` (feature `banking`; spec `economy/money-banking`,
//! "First version", roadmap M3): read-only state for the `[sim]` criteria
//! AC-BANK-01 to 08, and the steps that move it.
//!
//! The banks act under a law with the income taxes of 2021, as the treasury
//! does (the placeholder law keeps M2's credit inside the firm sector and
//! money that earns no interest, and its goldens). One aggregate bank:
//! - **Daily, at the checks:** a cohort unit's shortfall is drawn on its
//!   credit line while the line is open; what the line cannot cover falls
//!   back on M2's credit inside the firm sector. Then reserves are brought up
//!   to the requirement with central bank advances, which are repaid as soon
//!   as reserves exceed it.
//! - **Month end:** interest on loans and deposits, per record; cohort units
//!   repay a share of their loans out of deposits beyond a month of wages.
//! - **Monthly close:** the excess of a unit's loans over its credit limit is
//!   written off; the rates of the next month; the banks' equity and capital
//!   adequacy.

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_mech_banking::{Rates, capital_ratio_ppm, credit_room, month_interest, rates, write_off};
use econ_types::{Bani, Day};

use super::World;
use super::money::{Postings, bani, ratio};

/// The banks' state after the latest day. Money in bani; per-unit columns
/// aligned with [`super::FirmUnitColumns`] (a unit's amounts are already
/// totals of its firms), per-household columns with
/// [`super::HouseholdColumns`] (per record, before the weight).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BankState {
    /// The policy rate the banks price from, millionths a year.
    pub policy_rate_ppm: i32,
    /// The rates of the month, set at the latest monthly close (at load: the
    /// starting policy rate plus the spreads).
    pub rates: Rates,
    /// Each firm unit's bank loans, `unit_loans`.
    pub unit_loans: Vec<Bani>,
    /// What each unit drew on its credit line on the latest day.
    pub drawn_today: Vec<Bani>,
    /// What each unit repaid on the latest day.
    pub repaid_today: Vec<Bani>,
    /// What was written off each unit's loans on the latest day (the excess
    /// over its credit limit; the monthly close).
    pub written_off_today: Vec<Bani>,
    /// The banks' loss on those write-offs, by unit: by this much the unit's
    /// loans fell; it keeps owing the rest.
    pub bank_loss_today: Vec<Bani>,
    /// The banks' claim on firms: the sum of `unit_loans`.
    pub firm_loans: Bani,
    /// Each household's mortgage and consumer debt (stage H; constant in M3).
    pub hh_mortgage: Vec<Bani>,
    /// Consumer debt.
    pub hh_consumer_debt: Vec<Bani>,
    /// Interest each household paid on its loans at the latest month end.
    pub hh_interest_paid: Vec<Bani>,
    /// Interest each household received on its deposits at the latest month end.
    pub hh_interest_received: Vec<Bani>,
    /// Interest each unit paid on its loans at the latest month end.
    pub unit_interest_paid: Vec<Bani>,
    /// Interest each unit received on its deposits at the latest month end.
    pub unit_interest_received: Vec<Bani>,
    /// The banks' equity after the latest monthly close.
    pub equity: Bani,
    /// Capital adequacy after the latest monthly close, millionths.
    pub car_ppm: u32,
    /// Credit inside the firm sector (M2's placeholder): what the units owe
    /// each other. Zero once the banks lend.
    pub firm_sector_credit: Bani,
}

/// What the `World` keeps for the banks.
#[derive(Debug, Clone, Default)]
pub(super) struct Banks {
    active: bool,
    view: BankState,
    /// A change of the policy rate from a day (a start edit).
    policy_change: Option<(Day, i32)>,
    /// Each unit's revenue this month so far, and last month's.
    revenue_month: Vec<i128>,
    revenue_last: Vec<i128>,
}

impl World {
    /// The banks.
    #[must_use]
    pub fn bank(&self) -> &BankState {
        &self.bk.view
    }

    /// Whether the banks act.
    pub(super) fn banks_active(&self) -> bool {
        self.bk.active
    }

    /// The banks at load: each unit's share of the banks' claim on firms (the
    /// banks' loans less the households'), households' loans, the starting
    /// rates and capital.
    pub(super) fn start_banks(
        &mut self,
        unit_loans: &[Bani],
        mortgage: &[Bani],
        consumer: &[Bani],
        unit_output: &[Bani],
    ) {
        let active = self.law.income.is_some() && !unit_loans.is_empty();
        let n_u = self.units.industry.len();
        let n_h = self.households.hh_weight.len();
        self.bk = Banks {
            active,
            ..Banks::default()
        };
        if !active {
            return;
        }
        let w = &self.households.hh_weight;
        let weighted = |v: &[Bani]| -> i128 {
            v.iter()
                .zip(w)
                .map(|(b, &w)| i128::from(b.get()) * i128::from(w))
                .sum()
        };
        let hh_loans = weighted(mortgage) + weighted(consumer);
        let banks = i128::from(self.ledger.balance(Sector::Banks, Instrument::Loans).get());
        let firm_claim = (banks - hh_loans).max(0);
        let table: Vec<i128> = unit_loans.iter().map(|b| i128::from(b.get())).collect();
        let parts = super::money::split(firm_claim, &table);
        let v = &mut self.bk.view;
        v.unit_loans = parts.into_iter().map(bani).collect();
        v.firm_loans = bani(firm_claim);
        v.hh_mortgage = mortgage.to_vec();
        v.hh_consumer_debt = consumer.to_vec();
        v.policy_rate_ppm = self.params.policy_rate_ppm;
        for col in [
            &mut v.drawn_today,
            &mut v.repaid_today,
            &mut v.written_off_today,
            &mut v.bank_loss_today,
            &mut v.unit_interest_paid,
            &mut v.unit_interest_received,
        ] {
            *col = vec![Bani::ZERO; n_u];
        }
        v.hh_interest_paid = vec![Bani::ZERO; n_h];
        v.hh_interest_received = vec![Bani::ZERO; n_h];
        // A month's revenue at the start: a twelfth of the unit's output.
        self.bk.revenue_last = unit_output
            .iter()
            .map(|b| i128::from(b.get()) / 12)
            .collect();
        self.bk.revenue_month = vec![0; n_u];
        self.bank_capital();
        let car = self.bk.view.car_ppm;
        self.bk.view.rates = rates(self.params.policy_rate_ppm, car, &self.params.banking);
        self.ledger
            .allow_negative(Sector::Banks, Instrument::CbAdvances);
    }

    /// A change of the policy rate from a day (start edit).
    pub(super) fn bank_policy_change(&mut self, from: Day, rate_ppm: i32) {
        self.bk.policy_change = Some((from, rate_ppm));
    }

    /// A loan drawn on day 0 (start edit), as a credit-line draw.
    pub(super) fn bank_extra_loan(&mut self, unit: usize, amount: Bani) {
        if !self.bk.active || amount <= Bani::ZERO {
            return;
        }
        self.draw(unit, amount);
    }

    /// The start of a day: the day's columns emptied, the policy rate of the
    /// day.
    pub(super) fn bank_begin_day(&mut self) {
        if !self.bk.active {
            return;
        }
        let today = self.today;
        if let Some((from, rate)) = self.bk.policy_change
            && today >= from
        {
            self.bk.view.policy_rate_ppm = rate;
        }
        let v = &mut self.bk.view;
        for col in [
            &mut v.drawn_today,
            &mut v.repaid_today,
            &mut v.written_off_today,
            &mut v.bank_loss_today,
        ] {
            col.iter_mut().for_each(|x| *x = Bani::ZERO);
        }
    }

    /// Each household's interest due at the coming month end, at the
    /// month's rates; empty when the banks do not act.
    /// Only the households of `cohort` (the stagger day deciding today)
    /// are computed; the others' entries are zero.
    pub(super) fn hh_interest_due(&self, cohort: u8) -> Vec<Bani> {
        if !self.bk.active {
            return Vec::new();
        }
        let (v, r) = (&self.bk.view, self.bk.view.rates);
        let day = &self.households.hh_stagger_day;
        v.hh_mortgage
            .iter()
            .zip(&v.hh_consumer_debt)
            .zip(day)
            .map(|((&m, &c), &d)| {
                if d == cohort {
                    month_interest(m, r.mortgage_ppm) + month_interest(c, r.consumer_ppm)
                } else {
                    Bani::ZERO
                }
            })
            .collect()
    }

    /// The policy rate in force from today (`money.policy_rate`); the banks
    /// reprice at the next monthly close.
    #[cfg(feature = "levers")]
    pub(super) fn bank_set_policy_rate(&mut self, rate_ppm: i32) {
        self.bk.view.policy_rate_ppm = rate_ppm;
    }

    /// The policy rate today.
    pub(super) fn bank_policy_rate(&self) -> i32 {
        self.bk.view.policy_rate_ppm
    }

    /// A unit draws `amount` on its credit line: its loans and deposits grow.
    fn draw(&mut self, u: usize, amount: Bani) {
        self.units.deposits[u] += amount;
        let v = &mut self.bk.view;
        v.unit_loans[u] += amount;
        v.firm_loans += amount;
        v.drawn_today[u] += amount;
        let mut post = Postings::default();
        post.leg(
            Sector::Firms,
            Sector::Banks,
            Instrument::Loans,
            amount,
            FlowCode::LOAN,
        );
        post.leg(
            Sector::Banks,
            Sector::Firms,
            Instrument::Deposits,
            amount,
            FlowCode::LOAN,
        );
        post.commit(&mut self.ledger);
    }

    /// A cohort unit draws up to `amount` on its credit line now, as far as
    /// the line is open; returns what it drew. For a payment the firm
    /// sector's deposits cannot cover before the day's checks (an input bill
    /// beyond what the sector holds); never for a public unit.
    pub(super) fn bank_draw_now(&mut self, u: usize, amount: Bani) -> Bani {
        if !self.bk.active || self.uu.public[u] || amount <= Bani::ZERO {
            return Bani::ZERO;
        }
        let x = amount.min(self.bank_credit_room(u));
        if x > Bani::ZERO {
            self.draw(u, x);
        }
        x.max(Bani::ZERO)
    }

    /// A unit's room on its credit line now; none when the banks do not act.
    pub(super) fn bank_credit_room(&self, u: usize) -> Bani {
        if !self.bk.active {
            return Bani::ZERO;
        }
        credit_room(
            self.bk.view.unit_loans[u],
            bani(self.bk.revenue_last[u]),
            self.bk.view.car_ppm,
            &self.params.banking,
        )
    }

    /// The units' shortfalls drawn on their credit lines, as far as each
    /// line is open (step 1); before M2's credit inside the firm sector takes
    /// the rest.
    pub(super) fn bank_credit_lines(&mut self) {
        if !self.bk.active {
            return;
        }
        let car = self.bk.view.car_ppm;
        for u in 0..self.units.industry.len() {
            if self.uu.public[u] || self.units.deposits[u] >= Bani::ZERO {
                continue;
            }
            let need = Bani::ZERO - self.units.deposits[u];
            let room = credit_room(
                self.bk.view.unit_loans[u],
                bani(self.bk.revenue_last[u]),
                car,
                &self.params.banking,
            );
            let amount = need.min(room);
            if amount > Bani::ZERO {
                self.draw(u, amount);
            }
        }
    }

    /// After the day's payments: reserves at least the requirement, with
    /// advances from the central bank; advances repaid when reserves allow
    /// (step 5). The day's revenue into the month's.
    pub(super) fn bank_day_end(&mut self) {
        if !self.bk.active {
            return;
        }
        for (r, &t) in self
            .bk
            .revenue_month
            .iter_mut()
            .zip(&self.units.revenue_today)
        {
            *r += i128::from(t.get());
        }
        self.bank_rebalance_reserves();
    }

    /// Reserves brought up to the requirement with central bank advances, or
    /// advances repaid out of reserves above it (step 5).
    pub(super) fn bank_rebalance_reserves(&mut self) {
        if !self.bk.active {
            return;
        }
        let l = &self.ledger;
        let reserves = i128::from(l.balance(Sector::Banks, Instrument::Reserves).get());
        let deposits = -i128::from(l.balance(Sector::Banks, Instrument::Deposits).get());
        let advances = -i128::from(l.balance(Sector::Banks, Instrument::CbAdvances).get());
        let need = (deposits.max(0) * i128::from(self.params.banking.reserve_requirement_ppm)
            + 999_999)
            / 1_000_000;
        let mut post = Postings::default();
        if reserves < need {
            let x = bani(need - reserves);
            post.leg(
                Sector::CentralBank,
                Sector::Banks,
                Instrument::Reserves,
                x,
                FlowCode::CB_ADVANCE,
            );
            post.leg(
                Sector::Banks,
                Sector::CentralBank,
                Instrument::CbAdvances,
                x,
                FlowCode::CB_ADVANCE,
            );
        } else if advances > 0 && reserves > need {
            let x = bani((reserves - need).min(advances));
            post.leg(
                Sector::Banks,
                Sector::CentralBank,
                Instrument::Reserves,
                x,
                FlowCode::CB_ADVANCE,
            );
            post.leg(
                Sector::CentralBank,
                Sector::Banks,
                Instrument::CbAdvances,
                x,
                FlowCode::CB_ADVANCE,
            );
        }
        post.commit(&mut self.ledger);
    }

    /// An intraday advance (step 5): when a batch would take the banks'
    /// reserves below zero (a day's taxes above their whole stock, only in
    /// an economy gone bad), the central bank advances the shortfall in the
    /// same batch, with the legs of [`World::bank_rebalance_reserves`]; the
    /// day's check repays it out of any excess. A batch that leaves the
    /// reserves at or above zero is untouched.
    pub(super) fn bank_intraday_advance(&self, post: &mut Postings) {
        if !self.bk.active {
            return;
        }
        let reserves = i128::from(
            self.ledger
                .balance(Sector::Banks, Instrument::Reserves)
                .get(),
        );
        let after = reserves + post.banks_reserves_net();
        if after >= 0 {
            return;
        }
        let x = bani(-after);
        post.leg(
            Sector::CentralBank,
            Sector::Banks,
            Instrument::Reserves,
            x,
            FlowCode::CB_ADVANCE,
        );
        post.leg(
            Sector::Banks,
            Sector::CentralBank,
            Instrument::CbAdvances,
            x,
            FlowCode::CB_ADVANCE,
        );
    }

    /// Month end: interest on loans and deposits, per record (step 2), then
    /// repayments (step 3).
    #[allow(clippy::too_many_lines)]
    pub(super) fn bank_month_end(&mut self) {
        if !self.bk.active {
            return;
        }
        let r = self.bk.view.rates;
        let mut post = Postings::default();
        // Households: interest on their loans, out of deposits then cash;
        // on their deposits, as income of the month, less the tax the banks
        // withhold at `interest_tax_rate` (spec economy/taxation step 10).
        let tax_bp = self
            .law
            .income
            .as_ref()
            .map_or(0, |l| i128::from(l.interest_bp));
        let (mut hh_paid_dep, mut hh_paid_cash, mut hh_got) = (0i128, 0i128, 0i128);
        let mut hh_tax = 0i128;
        for h in 0..self.households.hh_weight.len() {
            let w = i128::from(self.households.hh_weight[h]);
            let v = &mut self.bk.view;
            let due = month_interest(v.hh_mortgage[h], r.mortgage_ppm)
                + month_interest(v.hh_consumer_debt[h], r.consumer_ppm);
            let gross = month_interest(
                self.households.hh_deposits[h].max(Bani::ZERO),
                r.deposit_ppm,
            );
            let tax = if tax_bp > 0 && gross > Bani::ZERO {
                ratio(i128::from(gross.get()), tax_bp, 10_000)
            } else {
                0
            };
            hh_tax += w * tax;
            let got = gross - bani(tax);
            v.hh_interest_received[h] = got;
            let hh = &mut self.households;
            hh.hh_deposits[h] += got;
            hh.hh_income_received[h] += got;
            hh.income_to_date[h] += got;
            self.hh.income_this_month[h] += got;
            hh_got += w * i128::from(got.get());
            let from_dep = due.min(hh.hh_deposits[h].max(Bani::ZERO));
            let from_cash = (due - from_dep).min(hh.hh_cash[h].max(Bani::ZERO));
            hh.hh_deposits[h] -= from_dep;
            hh.hh_cash[h] -= from_cash;
            self.bk.view.hh_interest_paid[h] = from_dep + from_cash;
            hh_paid_dep += w * i128::from(from_dep.get());
            hh_paid_cash += w * i128::from(from_cash.get());
        }
        post.leg(
            Sector::Banks,
            Sector::Households,
            Instrument::Deposits,
            bani(hh_got),
            FlowCode::INTEREST_DEPOSIT,
        );
        // The tax withheld: the banks pay it to the government's account.
        post.leg(
            Sector::Banks,
            Sector::CentralBank,
            Instrument::Reserves,
            bani(hh_tax),
            FlowCode::TAX_INTEREST,
        );
        post.leg(
            Sector::CentralBank,
            Sector::Government,
            Instrument::GovDepositsAtCb,
            bani(hh_tax),
            FlowCode::TAX_INTEREST,
        );
        post.leg(
            Sector::Households,
            Sector::Banks,
            Instrument::Deposits,
            bani(hh_paid_dep),
            FlowCode::INTEREST_LOAN,
        );
        post.leg(
            Sector::Households,
            Sector::Banks,
            Instrument::Cash,
            bani(hh_paid_cash),
            FlowCode::INTEREST_LOAN,
        );
        // Firm units: interest on their bank loans and on their deposits; a
        // unit short of deposits draws the rest at the day's checks.
        let n_u = self.units.industry.len();
        let mut due: Vec<i128> = (0..n_u)
            .map(|u| i128::from(month_interest(self.bk.view.unit_loans[u], r.firms_ppm).get()))
            .collect();
        let mut u_got = 0i128;
        for u in 0..n_u {
            let got = month_interest(self.units.deposits[u].max(Bani::ZERO), r.deposit_ppm);
            self.bk.view.unit_interest_received[u] = got;
            self.units.deposits[u] += got;
            u_got += i128::from(got.get());
        }
        post.leg(
            Sector::Banks,
            Sector::Firms,
            Instrument::Deposits,
            bani(u_got),
            FlowCode::INTEREST_DEPOSIT,
        );
        // The backstop: interest the firm sector cannot pay is not paid
        // (the banks' loss; never in a normal run).
        self.firm_fit_parts(&post, &mut due, Some(super::insolvency::Unpaid::Interest));
        let mut u_paid = 0i128;
        for u in 0..n_u {
            self.bk.view.unit_interest_paid[u] = bani(due[u]);
            self.units.deposits[u] -= bani(due[u]);
            u_paid += due[u];
        }
        post.leg(
            Sector::Firms,
            Sector::Banks,
            Instrument::Deposits,
            bani(u_paid),
            FlowCode::INTEREST_LOAN,
        );
        // The other depositors: interest on what they hold at the banks.
        for s in [
            Sector::PensionFunds,
            Sector::OtherFinancial,
            Sector::RestOfWorld,
        ] {
            let d = self.ledger.balance(s, Instrument::Deposits).max(Bani::ZERO);
            post.leg(
                Sector::Banks,
                s,
                Instrument::Deposits,
                month_interest(d, r.deposit_ppm),
                FlowCode::INTEREST_DEPOSIT,
            );
        }
        self.bank_intraday_advance(&mut post);
        post.commit(&mut self.ledger);
        // Repayments: a share of the loans, out of deposits beyond a month of
        // wages.
        let reserve = self.wage_reserves();
        let mut repaid = 0i128;
        let mut parts = vec![0i128; self.units.industry.len()];
        for u in 0..self.units.industry.len() {
            if self.uu.public[u] {
                continue;
            }
            let loans = self.bk.view.unit_loans[u];
            let share = bani(
                i128::from(loans.get()) * i128::from(self.params.banking.firm_repayment_ppm)
                    / 1_000_000,
            );
            let spare = bani((i128::from(self.units.deposits[u].get()) - reserve[u]).max(0));
            parts[u] = i128::from(share.min(spare).min(loans).get()).max(0);
        }
        // Never more than the firm sector holds (the backstop).
        self.firm_fit_parts(&Postings::default(), &mut parts, None);
        for u in 0..self.units.industry.len() {
            let x = bani(parts[u]);
            if x > Bani::ZERO {
                self.units.deposits[u] -= x;
                let v = &mut self.bk.view;
                v.unit_loans[u] -= x;
                v.firm_loans -= x;
                v.repaid_today[u] += x;
                repaid += i128::from(x.get());
            }
        }
        let mut post = Postings::default();
        post.leg(
            Sector::Banks,
            Sector::Firms,
            Instrument::Loans,
            bani(repaid),
            FlowCode::LOAN,
        );
        post.leg(
            Sector::Firms,
            Sector::Banks,
            Instrument::Deposits,
            bani(repaid),
            FlowCode::LOAN,
        );
        post.commit(&mut self.ledger);
    }

    /// Monthly close: write-offs above the credit limit (step 4), the
    /// banks' capital (step 6) and the next month's rates.
    pub(super) fn bank_close(&mut self) {
        if !self.bk.active {
            return;
        }
        // This month's revenue, today's included, is the next month's measure.
        let month: Vec<i128> = self
            .bk
            .revenue_month
            .iter()
            .zip(&self.units.revenue_today)
            .map(|(&m, t)| m + i128::from(t.get()))
            .collect();
        let mut lost = 0i128;
        for u in 0..self.units.industry.len() {
            if self.uu.public[u] {
                continue;
            }
            let (written, loss) = write_off(
                self.bk.view.unit_loans[u],
                bani(month[u]),
                &self.params.banking,
            );
            if loss > Bani::ZERO || written > Bani::ZERO {
                let v = &mut self.bk.view;
                v.written_off_today[u] = written;
                v.bank_loss_today[u] = loss;
                v.unit_loans[u] -= loss;
                v.firm_loans -= loss;
                lost += i128::from(loss.get());
            }
        }
        let mut post = Postings::default();
        post.leg(
            Sector::Banks,
            Sector::Firms,
            Instrument::Loans,
            bani(lost),
            FlowCode::WRITE_OFF,
        );
        post.commit(&mut self.ledger);
        self.bk.revenue_last = month;
        // Today's revenue is already in `revenue_last`: the day's end adds it
        // to the new month, so take it out here.
        self.bk.revenue_month = self
            .units
            .revenue_today
            .iter()
            .map(|t| -i128::from(t.get()))
            .collect();
        self.bank_capital();
        let car = self.bk.view.car_ppm;
        self.bk.view.rates = rates(self.bk.view.policy_rate_ppm, car, &self.params.banking);
    }

    /// The banks' equity (every balance but the shares they issued) and
    /// capital adequacy.
    fn bank_capital(&mut self) {
        let equity: i128 = Instrument::ALL
            .into_iter()
            .filter(|&i| i != Instrument::Equity)
            .map(|i| i128::from(self.ledger.balance(Sector::Banks, i).get()))
            .sum();
        let w = &self.households.hh_weight;
        let weighted = |v: &[Bani]| -> Bani {
            bani(
                v.iter()
                    .zip(w)
                    .map(|(b, &w)| i128::from(b.get()) * i128::from(w))
                    .sum(),
            )
        };
        let v = &self.bk.view;
        let car = capital_ratio_ppm(
            bani(equity),
            v.firm_loans,
            weighted(&v.hh_consumer_debt),
            weighted(&v.hh_mortgage),
        );
        let v = &mut self.bk.view;
        v.equity = bani(equity);
        v.car_ppm = car;
    }

    /// The firm sector's own credit, for the view.
    pub(super) fn bank_note_firm_credit(&mut self) {
        if !self.bk.active {
            return;
        }
        let owed: i128 = self.uu.owes.iter().map(|b| i128::from(b.get())).sum();
        self.bk.view.firm_sector_credit = bani(owed);
    }

    /// The banks' state for the state hash.
    pub(super) fn bank_hash(&self, write: &mut impl FnMut(u64)) {
        if !self.bk.active {
            return;
        }
        let v = &self.bk.view;
        #[allow(clippy::cast_sign_loss)]
        {
            write(v.policy_rate_ppm as u64);
            for b in &v.unit_loans {
                write(b.get() as u64);
            }
            for r in &self.bk.revenue_month {
                write(*r as u64);
            }
            for r in &self.bk.revenue_last {
                write(*r as u64);
            }
        }
        write(u64::from(v.car_ppm));
        write(u64::from(v.rates.firms_ppm));
    }
}
