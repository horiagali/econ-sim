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
            // The law's rate is the rate of the start date and of a command
            // in force; while the advisory rule sets the rate, a change of
            // another lever leaves it (owner, 2026-10-29).
            #[cfg(feature = "monetary")]
            let rule = self.cb_follows_rule();
            #[cfg(not(feature = "monetary"))]
            let rule = false;
            if let Some(v) = policy
                && !rule
            {
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
        // `spend.new_issue_years`: the maturity of the treasury's next issues
        // (bonds already issued keep theirs).
        #[cfg(feature = "fiscal")]
        if let Some(LeverValue::Number(n)) = self
            .lever_state()
            .and_then(|s| s.law.get(LeverId::SpendNewIssueYears, None))
        {
            self.params.fiscal.debt.new_issue_years = u8::try_from((*n).clamp(1, 30)).unwrap_or(7);
        }
        #[cfg(feature = "trade")]
        self.fx_refresh_regime();
    }

    /// `fx.regime`: the share of the exchange rate's monthly move the central
    /// bank offsets while its reserves are above the floor: none for a
    /// float, trade's parameter for a managed float (kept from load), all of
    /// it for a peg (the owner's decision of 2026-10-22).
    #[cfg(feature = "trade")]
    fn fx_refresh_regime(&mut self) {
        let managed = *self
            .bud
            .fx_managed_ppm
            .get_or_insert(self.params.trade.fx.intervention_ppm);
        let Some(LeverValue::Choice(i)) = self
            .lever_state()
            .and_then(|s| s.law.get(LeverId::FxRegime, None))
            .cloned()
        else {
            return;
        };
        let option = self.lever_state().and_then(|s| {
            s.registry
                .get(LeverId::FxRegime)
                .options
                .get(usize::from(i))
                .cloned()
        });
        self.params.trade.fx.intervention_ppm = match option.as_deref() {
            Some("float") => 0,
            Some("peg") => 1_000_000,
            _ => managed,
        };
    }

    /// `fx.intervention` added to the rule's at the month end: the player's
    /// purchase of foreign assets (> 0) or sale (< 0), in lei; sales never
    /// take reserves below the floor. Returns the intervention in trade's
    /// sign (sales > 0).
    #[cfg(feature = "trade")]
    pub(super) fn fx_with_player(&self, rule: Bani, above_floor: Bani) -> Bani {
        let player = self.lever_money(LeverId::FxIntervention);
        let total = rule - Bani(player);
        total.min(above_floor.max(Bani::ZERO))
    }

    /// `transfer.pension.indexation`, at the wage round (1 January): the
    /// pension point in force grows by the twelve-month inflation of the last
    /// close (`prices`), by the round's mean wage growth (`wages`, given) or
    /// by their mean (`mix`); `none` keeps it frozen. Neither rate lowers the
    /// point. The new point becomes the value of
    /// `transfer.pension.point_value` in force, as the law's own rule (no
    /// command is logged; a replay recomputes it).
    pub(super) fn pension_indexation(&mut self, wage_growth_ppm: i32) {
        let Some(st) = self.lever_state() else {
            return;
        };
        let Some(LeverValue::Choice(i)) = st.law.get(LeverId::TransferPensionIndexation, None)
        else {
            return;
        };
        let option = st
            .registry
            .get(LeverId::TransferPensionIndexation)
            .options
            .get(usize::from(*i))
            .map(String::as_str);
        let prices = self.months.last().map_or(0, |m| m.inflation_ppm);
        let g = match option {
            Some("prices") => prices,
            Some("wages") => wage_growth_ppm,
            Some("mix") => {
                i32::try_from((i64::from(prices) + i64::from(wage_growth_ppm)) / 2).unwrap_or(0)
            }
            _ => return,
        };
        let Some(LeverValue::Money(point)) = st.law.get(LeverId::TransferPensionPointValue, None)
        else {
            return;
        };
        let g = i128::from(g.max(0));
        let new = Bani(
            i64::try_from(i128::from(point.get()) * (1_000_000 + g) / 1_000_000)
                .unwrap_or(point.get()),
        );
        if new == *point {
            return;
        }
        let industries = self.industries.id.clone();
        if let Some(st) = self.lever_state_mut() {
            st.law.set(&crate::levers::Command {
                lever: LeverId::TransferPensionPointValue,
                key: None,
                value: LeverValue::Money(new),
                effective: None,
            });
            let law = crate::levers::law_of(&st.law, &st.registry, &industries);
            self.set_law(law);
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
