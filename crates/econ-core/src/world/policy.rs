//! The money levers on the `World` (spec `game/lever-system`, "The alpha
//! levers"; feature `levers`): the policy rate, the reserve requirement and
//! the inflation target read from the law in force, and the central bank's
//! month-end bond operations, QE and monetary financing.

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_types::Bani;

use super::World;
use super::money::Postings;
use crate::levers::{LeverId, LeverValue};

impl World {
    /// The parameters the money levers set, from the law in force: at load
    /// and whenever a command takes effect.
    pub(crate) fn levers_refresh_params(&mut self) {
        let Some(st) = self.lever_state() else {
            return;
        };
        let rate = |l| match st.law.get(l, None) {
            Some(LeverValue::Rate(v)) => Some(*v),
            _ => None,
        };
        let (policy, reserves, target) = (
            rate(LeverId::MoneyPolicyRate),
            rate(LeverId::MoneyReserveRequirement),
            rate(LeverId::MoneyInflationTarget),
        );
        // Basis points to millionths.
        #[cfg(feature = "banking")]
        {
            if let Some(v) = policy {
                self.bank_set_policy_rate(v * 100);
            }
            if let Some(v) = reserves {
                self.params.banking.reserve_requirement_ppm =
                    u32::try_from(v.max(0) * 100).unwrap_or(u32::MAX);
            }
        }
        #[cfg(not(feature = "banking"))]
        let _ = (policy, reserves);
        if let Some(v) = target {
            self.mac.target_ppm = v * 100;
        }
    }

    /// A money lever's amount a month, bani.
    fn lever_money(&self, lever: LeverId) -> i64 {
        match self.lever_state().and_then(|s| s.law.get(lever, None)) {
            Some(LeverValue::Money(b)) => b.get(),
            _ => 0,
        }
    }

    /// The month end, before the treasury's issue: monetary financing (the
    /// central bank buys that much of new bonds) and QE (it buys the banks'
    /// bonds, or sells them its own when negative, against reserves).
    /// Returns the new bonds the central bank bought.
    pub(super) fn policy_bond_operations(&mut self) -> Bani {
        if self.lever_state().is_none() {
            return Bani::ZERO;
        }
        let financing = Bani(self.lever_money(LeverId::MoneyMonetaryFinancing).max(0));
        let bought = self.treasury_sell_to_central_bank(financing);
        let qe = self.lever_money(LeverId::MoneyQe);
        let l = &self.ledger;
        let (from, to, amount) = if qe > 0 {
            let held = l.balance(Sector::Banks, Instrument::Bonds).get().max(0);
            (Sector::Banks, Sector::CentralBank, qe.min(held))
        } else {
            let held = l
                .balance(Sector::CentralBank, Instrument::Bonds)
                .get()
                .max(0);
            let reserves = l.balance(Sector::Banks, Instrument::Reserves).get().max(0);
            (
                Sector::CentralBank,
                Sector::Banks,
                (-qe).min(held).min(reserves),
            )
        };
        if amount > 0 {
            let mut post = Postings::default();
            post.bonds(from, to, Bani(amount));
            post.leg(
                to,
                from,
                Instrument::Reserves,
                Bani(amount),
                FlowCode::BONDS,
            );
            post.commit(&mut self.ledger);
        }
        bought
    }
}
