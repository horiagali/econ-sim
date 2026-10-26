//! Backstop: firms that cannot pay (owner, 2026-10-26; spec
//! `economy/labor-market`, "Backstop: firms that cannot pay"; roadmap M10,
//! its firm wage and tax part built in M4).
//!
//! Firm units' deposits may go negative during a day (credit inside the
//! sector and the banks' lines cover them at its end), but the firm
//! sector's deposits in the ledger may not. Every payment out of the sector
//! asks [`World::firm_payable`] how much of it the sector can pay now: all
//! of it in a normal run. When the sector is short (its money gone abroad,
//! its credit lines used up, as under heavy monetary financing), wages are
//! paid pro rata and the unit cuts its staff at its next staffing day,
//! taxes due are paid as far as they can be and the rest is reported as
//! unpaid, interest likewise, dividends are not paid.
//!
//! > **Simplification:** unpaid wages and taxes are not a debt; nobody
//! > claims them later (no arrears stock in v1).

use econ_types::Bani;

use super::World;
use super::money::{Postings, split};

const ONE: i128 = 1_000_000;

/// What firm units owed and could not pay, in bani (sector totals).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FirmUnpaid {
    /// Gross wages not paid (`wages_unpaid`).
    pub wages: Bani,
    /// Taxes and other payments to the government not paid (`taxes_unpaid`):
    /// profit and micro tax, taxes on production, royalties and the firms'
    /// other transfers to the government.
    pub taxes: Bani,
    /// Interest on bank loans not paid (the banks' loss).
    pub interest: Bani,
}

/// The kinds of payment the backstop reports.
#[derive(Debug, Clone, Copy)]
pub(super) enum Unpaid {
    Wages,
    Taxes,
    #[cfg_attr(not(feature = "banking"), allow(dead_code))]
    Interest,
}

/// The shortfalls of the month and since the start.
#[derive(Debug, Clone, Default)]
pub(super) struct UnpaidBook {
    month: [i128; 3],
    total: [i128; 3],
}

impl UnpaidBook {
    fn as_firm_unpaid(v: &[i128; 3]) -> FirmUnpaid {
        FirmUnpaid {
            wages: Bani(i64::try_from(v[0]).unwrap_or(i64::MAX)),
            taxes: Bani(i64::try_from(v[1]).unwrap_or(i64::MAX)),
            interest: Bani(i64::try_from(v[2]).unwrap_or(i64::MAX)),
        }
    }
}

impl World {
    /// The firm sector's deposits free for a payment out of it now: the
    /// ledger's balance with `post`'s legs so far, less the product taxes
    /// and duties charged today but not yet posted.
    pub(super) fn firm_room(&self, post: &Postings) -> i128 {
        self.firm_money_free() + post.firms_net()
    }

    /// Of `due` that the firm sector owes out of it, the part it can pay now
    /// with `post` pending (all of it in a normal run).
    pub(super) fn firm_payable(&self, post: &Postings, due: i128) -> i128 {
        due.min(self.firm_room(post).max(0)).max(0)
    }

    /// The units' parts of a payment out of the firm sector, cut to what
    /// the sector can pay with `post` pending: each part by the same share
    /// (largest remainder). Returns the unpaid total, recorded under `kind`
    /// when there is one; nothing changes in a normal run.
    pub(super) fn firm_fit_parts(
        &mut self,
        post: &Postings,
        parts: &mut [i128],
        kind: Option<Unpaid>,
    ) -> i128 {
        let due: i128 = parts.iter().sum();
        let payable = self.firm_payable(post, due);
        if payable >= due {
            return 0;
        }
        let cut = split(payable, parts);
        parts.copy_from_slice(&cut);
        let unpaid = due - payable;
        if let Some(k) = kind {
            self.note_unpaid(k, unpaid);
        }
        unpaid
    }

    /// Record `amount` unpaid of kind `kind`.
    pub(super) fn note_unpaid(&mut self, kind: Unpaid, amount: i128) {
        if amount <= 0 {
            return;
        }
        let k = kind as usize;
        self.uu.unpaid.month[k] += amount;
        self.uu.unpaid.total[k] += amount;
    }

    /// The month's shortfalls, taken at the close.
    pub(super) fn take_unpaid_month(&mut self) -> FirmUnpaid {
        UnpaidBook::as_firm_unpaid(&std::mem::take(&mut self.uu.unpaid.month))
    }

    /// What firm units owed since the start and could not pay (all zero in
    /// a normal run).
    #[must_use]
    pub fn firm_unpaid(&self) -> FirmUnpaid {
        UnpaidBook::as_firm_unpaid(&self.uu.unpaid.total)
    }

    /// The share of its wage bill unit `u` paid on the last pay day,
    /// millionths (1,000,000 when it paid in full).
    #[must_use]
    pub fn unit_pay_share_ppm(&self, u: usize) -> u32 {
        self.uu.pay_share_ppm.get(u).copied().unwrap_or(1_000_000)
    }

    /// Whether the government pays unit `u`'s labour cost back today (a
    /// public unit under the ministry budgets on a month end): its wages
    /// never weigh on the firm sector's money.
    fn wages_reimbursed(&self, u: usize) -> bool {
        #[cfg(feature = "levers")]
        {
            self.budget_public(u) && self.calendar.date(self.today).is_month_end()
        }
        #[cfg(not(feature = "levers"))]
        {
            let _ = u;
            false
        }
    }

    /// Pay day: the share of its wage bill each unit can pay, millionths;
    /// `None` when every unit pays in full (always in a normal run).
    ///
    /// A unit pays in full while the firm sector's free deposits cover every
    /// bill. When they do not, each unit short of its own bill first draws
    /// on its credit line; the units whose own deposits then cover their
    /// bill pay in full, and the others share what is left, each paying the
    /// same share of every employee's pay. A rounding margin of two bani per
    /// weighted payment keeps the posting within the sector's money.
    pub(super) fn wage_shares(&mut self, payments: &[(u32, u16, Bani, Bani)]) -> Option<Vec<u32>> {
        let n_u = self.units.industry.len();
        let mut gross = vec![0i128; n_u];
        let mut count = vec![0i128; n_u];
        for &(i, u, g, _) in payments {
            let w = i128::from(self.person_weight(i));
            gross[usize::from(u)] += w * i128::from(g.get());
            count[usize::from(u)] += w;
        }
        let private: Vec<bool> = (0..n_u).map(|u| !self.wages_reimbursed(u)).collect();
        let room = self.firm_money_free();
        // A cheap bound first: the employer's contribution is far below a
        // tenth of the gross.
        let bound: i128 = (0..n_u)
            .filter(|&u| private[u])
            .map(|u| gross[u] + gross[u] / 10 + 2 * count[u])
            .sum();
        if bound <= room {
            return None;
        }
        let mut cost = vec![0i128; n_u];
        for &(i, u, g, d) in payments {
            let w = i128::from(self.person_weight(i));
            let tax = self.wage_tax(i as usize, u, g, d);
            cost[usize::from(u)] += w * i128::from((g + tax.cam).get());
        }
        let need: i128 = (0..n_u).filter(|&u| private[u]).map(|u| cost[u]).sum();
        if need <= room {
            return None;
        }
        #[cfg(feature = "banking")]
        for u in 0..n_u {
            let own = i128::from(self.units.deposits[u].get()).max(0);
            if private[u] && cost[u] > own {
                let _ = self.bank_draw_now(u, super::money::bani(cost[u] - own));
            }
        }
        let room = self.firm_money_free();
        if need <= room {
            return None;
        }
        let covered =
            |w: &Self, u: usize| i128::from(w.units.deposits[u].get()) >= cost[u] + 2 * count[u];
        let (mut paid_in_full, mut short, mut margin) = (0i128, 0i128, 0i128);
        for u in (0..n_u).filter(|&u| private[u] && cost[u] > 0) {
            if covered(self, u) {
                paid_in_full += cost[u];
            } else {
                short += cost[u];
                margin += 2 * count[u];
            }
        }
        let mut shares = vec![1_000_000u32; n_u];
        let share = |left: i128, of: i128| -> u32 {
            if of <= 0 {
                return 1_000_000;
            }
            u32::try_from((left.max(0) * ONE / of).min(ONE)).expect("a share")
        };
        if paid_in_full + margin <= room {
            let s = share(room - paid_in_full - margin, short);
            for u in 0..n_u {
                if private[u] && cost[u] > 0 && !covered(self, u) {
                    shares[u] = s;
                }
            }
        } else {
            let all_margin: i128 = (0..n_u).filter(|&u| private[u]).map(|u| 2 * count[u]).sum();
            let s = share(room - all_margin, paid_in_full + short);
            for u in 0..n_u {
                if private[u] && cost[u] > 0 {
                    shares[u] = s;
                }
            }
        }
        Some(shares)
    }

    /// The staff unit `u` aims for at its staffing day: its desired staff,
    /// but no more than the staff whose pay it could meet on the last pay
    /// day (a unit that paid less than its bill lays off, within
    /// `max_layoff_share`, and does not hire).
    pub(super) fn affordable_staff(&self, u: usize, staff: u64, desired: u64) -> u64 {
        let s = self.unit_pay_share_ppm(u);
        if s >= 1_000_000 {
            return desired;
        }
        let fits = u64::try_from(i128::from(staff) * i128::from(s) / ONE).expect("persons");
        desired.min(fits)
    }
}
