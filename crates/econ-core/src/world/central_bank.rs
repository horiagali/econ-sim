//! The central bank on the `World` (feature `monetary`; spec
//! `economy/monetary-policy`, "First version", roadmap M3): its parameters
//! and read-only state for the `[sim]` criteria AC-MP-03 to 05.
//!
//! The central bank acts with the banks (under the 2021 law). Month end,
//! after the treasury's payments and the banks' month end: interest at the
//! deposit facility on reserves above the requirement and at the lending
//! facility on advances; its foreign assets' yield, paid by the rest of the
//! world in foreign assets; then the day's profit remitted to the government
//! before the treasury's financing (a loss is borne by its equity). Monthly
//! close, after the financing: monetary financing, credibility and the
//! advisory rule. Expectations then use this credibility.

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_mech_banking::month_interest;
use econ_mech_monetary::{CredibilityParams, RuleParams, advisory_rate_ppm, credibility_ppm};
use econ_types::Bani;

use super::World;
use super::money::{Postings, bani};

/// The central bank's parameters (spec, "Tuning parameters"), millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonetaryParams {
    /// `corridor`: the facilities around the policy rate (±1 point).
    pub corridor_ppm: u32,
    /// `foreign_yield`: what the foreign assets earn a year (0.5%).
    pub foreign_yield_ppm: u32,
    /// `target_band` (±1 point).
    pub target_band_ppm: u32,
    /// Credibility's update.
    pub credibility: CredibilityParams,
    /// The advisory rule.
    pub rule: RuleParams,
    /// `monfin_pass`: expected inflation per unit of the year's monetary
    /// financing over GDP (0.5).
    pub monfin_pass_ppm: u32,
}

impl MonetaryParams {
    /// The spec's defaults.
    #[must_use]
    pub fn spec_defaults() -> MonetaryParams {
        MonetaryParams {
            corridor_ppm: 10_000,
            foreign_yield_ppm: 5_000,
            target_band_ppm: 10_000,
            credibility: CredibilityParams::spec_defaults(),
            rule: RuleParams::spec_defaults(),
            monfin_pass_ppm: 500_000,
        }
    }
}

/// The central bank's state after the latest day.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CentralBankState {
    /// The policy rate, millionths a year.
    pub policy_rate_ppm: i32,
    /// `cb_credibility`, millionths.
    pub credibility_ppm: u32,
    /// `monetary_financing`: the central bank's claim on the government
    /// (its deposits below zero, bonds bought from it).
    pub monetary_financing: Bani,
    /// The advisory rule's rate at the latest close, millionths a year.
    pub advisory_rate_ppm: i32,
    /// The reserves above the requirement on which the latest month end's
    /// interest at the deposit facility was paid.
    pub excess_reserves_basis: Bani,
    /// The advances on which the latest month end's interest at the lending
    /// facility was charged.
    pub advances_basis: Bani,
}

/// The credibility of the start (the `World`'s expectations used it as a
/// constant before the central bank existed).
pub(super) const START_CREDIBILITY_PPM: u32 = 700_000;

/// What the `World` keeps for the central bank.
#[derive(Debug, Clone, Default)]
pub(super) struct CentralBank {
    active: bool,
    view: CentralBankState,
    /// Its bonds at the start: monetary financing is what it holds above them.
    bonds0: Bani,
    /// Monetary financing at each of the last thirteen closes.
    monfin_year: Vec<Bani>,
    /// Whether each of the last twelve closes had inflation in the band.
    in_band: Vec<bool>,
    /// The year's increase of monetary financing over GDP at the last close.
    monfin_ppm: u32,
    /// What it added to expected inflation at the last close.
    push_ppm: i32,
}

impl World {
    /// The central bank.
    #[must_use]
    pub fn central_bank(&self) -> &CentralBankState {
        &self.cb.view
    }

    /// The central bank at load: active with the banks.
    pub(super) fn start_central_bank(&mut self) {
        let active = self.banks_active();
        self.cb = CentralBank {
            active,
            bonds0: self.ledger.balance(Sector::CentralBank, Instrument::Bonds),
            ..CentralBank::default()
        };
        if active {
            let policy = self.bank_policy_rate();
            let v = &mut self.cb.view;
            v.policy_rate_ppm = policy;
            v.credibility_ppm = START_CREDIBILITY_PPM;
            v.advisory_rate_ppm = v.policy_rate_ppm;
        }
    }

    /// What monetary financing added to expected inflation at the last
    /// close (step 5), millionths a year; none when the central bank does not
    /// act.
    pub(super) fn cb_expectations_push(&self) -> i32 {
        if self.cb.active { self.cb.push_ppm } else { 0 }
    }

    /// The push of this close, from the year's monetary financing over GDP
    /// at the last close; remembered so the next close can take it out.
    pub(super) fn cb_next_push(&mut self) -> i32 {
        if !self.cb.active {
            return 0;
        }
        let push = i128::from(self.params.monetary.monfin_pass_ppm)
            * i128::from(self.cb.monfin_ppm)
            / 1_000_000;
        self.cb.push_ppm = i32::try_from(push).unwrap_or(i32::MAX);
        self.cb.push_ppm
    }

    /// The credibility expectations use.
    pub(super) fn cb_credibility(&self) -> Option<u32> {
        self.cb.active.then_some(self.cb.view.credibility_ppm)
    }

    /// Each day: the policy rate the banks price from.
    pub(super) fn cb_begin_day(&mut self) {
        if self.cb.active {
            self.cb.view.policy_rate_ppm = self.bank_policy_rate();
        }
    }

    /// Month end: interest on reserves and advances, the foreign assets'
    /// yield, and the day's profit remitted to the government.
    pub(super) fn cb_month_end(&mut self) {
        if !self.cb.active {
            return;
        }
        // Advances repaid where reserves allow, before the interest on either.
        self.bank_rebalance_reserves();
        let policy = self.cb.view.policy_rate_ppm;
        let p = self.params.monetary;
        let corridor = i32::try_from(p.corridor_ppm).unwrap_or(i32::MAX);
        let l = &self.ledger;
        let reserves = l.balance(Sector::Banks, Instrument::Reserves);
        let deposits = Bani::ZERO - l.balance(Sector::Banks, Instrument::Deposits);
        let advances = Bani::ZERO - l.balance(Sector::Banks, Instrument::CbAdvances);
        let need = bani(
            (i128::from(deposits.get().max(0))
                * i128::from(self.params.banking.reserve_requirement_ppm)
                + 999_999)
                / 1_000_000,
        );
        let excess = (reserves - need).max(Bani::ZERO);
        let advances = advances.max(Bani::ZERO);
        let deposit_facility = u32::try_from((policy - corridor).max(0)).unwrap_or(0);
        let lending_facility = u32::try_from((policy + corridor).max(0)).unwrap_or(0);
        let foreign = l
            .balance(Sector::CentralBank, Instrument::ForeignAssets)
            .max(Bani::ZERO);
        let mut post = Postings::default();
        let code = FlowCode::INTEREST_RESERVES;
        post.leg(
            Sector::CentralBank,
            Sector::Banks,
            Instrument::Reserves,
            month_interest(excess, deposit_facility),
            code,
        );
        post.leg(
            Sector::Banks,
            Sector::CentralBank,
            Instrument::Reserves,
            month_interest(advances, lending_facility),
            code,
        );
        post.leg(
            Sector::RestOfWorld,
            Sector::CentralBank,
            Instrument::ForeignAssets,
            month_interest(foreign, p.foreign_yield_ppm),
            FlowCode::INTEREST_CB_FOREIGN,
        );
        post.commit(&mut self.ledger);
        self.cb.view.excess_reserves_basis = excess;
        self.cb.view.advances_basis = advances;
        // The day's profit: every flow of the central bank today.
        let profit: i128 = FlowCode::ALL
            .into_iter()
            .map(|c| i128::from(self.ledger.flows().get(c, Sector::CentralBank).get()))
            .sum();
        if profit > 0 {
            let mut post = Postings::default();
            post.leg(
                Sector::CentralBank,
                Sector::Government,
                Instrument::GovDepositsAtCb,
                bani(profit),
                FlowCode::TRANSFER_CB_PROFIT,
            );
            post.commit(&mut self.ledger);
        }
    }

    /// Monthly close, after the treasury's financing: monetary financing,
    /// credibility and the advisory rule.
    pub(super) fn cb_close(&mut self) {
        if !self.cb.active {
            return;
        }
        let l = &self.ledger;
        let overdraft = (Bani::ZERO - l.balance(Sector::Government, Instrument::GovDepositsAtCb))
            .max(Bani::ZERO);
        let bought =
            (l.balance(Sector::CentralBank, Instrument::Bonds) - self.cb.bonds0).max(Bani::ZERO);
        let monfin = overdraft + bought;
        self.cb.view.monetary_financing = monfin;
        let Some(m) = self.months.last() else {
            return;
        };
        let (inflation, expected, gdp) =
            (m.inflation_ppm, m.expected_inflation_ppm, m.gdp_production);
        let target = self.mac.target_ppm;
        let p = self.params.monetary;
        let miss = (i64::from(inflation) - i64::from(target)).unsigned_abs();
        self.cb.in_band.push(miss <= u64::from(p.target_band_ppm));
        if self.cb.in_band.len() > 12 {
            self.cb.in_band.remove(0);
        }
        // The year's increase of monetary financing over the year's GDP.
        let year_ago = if self.cb.monfin_year.len() >= 12 {
            self.cb.monfin_year[self.cb.monfin_year.len() - 12]
        } else {
            Bani::ZERO
        };
        self.cb.monfin_year.push(monfin);
        if self.cb.monfin_year.len() > 13 {
            let _ = self.cb.monfin_year.remove(0);
        }
        let n = self.months.len().min(12);
        let gdp_year: i128 = self.months[self.months.len() - n..]
            .iter()
            .map(|m| i128::from(m.gdp_production.get()))
            .sum::<i128>()
            * 12
            / i128::try_from(n).expect("small");
        let rise = i128::from((monfin - year_ago).get().max(0));
        let monfin_ppm = if gdp_year > 0 {
            u32::try_from(rise * 1_000_000 / gdp_year).unwrap_or(u32::MAX)
        } else {
            0
        };
        self.cb.monfin_ppm = monfin_ppm;
        let in_band = u8::try_from(self.cb.in_band.iter().filter(|&&b| b).count()).expect("12");
        self.cb.view.credibility_ppm = credibility_ppm(
            self.cb.view.credibility_ppm,
            in_band,
            u32::try_from(miss).unwrap_or(u32::MAX),
            monfin_ppm,
            0,
            &p.credibility,
        );
        // The advisory rule: the output gap against the last 24 months.
        let k = self.months.len().min(24);
        let trend: i128 = self.months[self.months.len() - k..]
            .iter()
            .map(|m| i128::from(m.gdp_production.get()))
            .sum::<i128>()
            / i128::try_from(k).expect("small");
        let gap = if trend > 0 {
            i32::try_from((i128::from(gdp.get()) - trend) * 1_000_000 / trend).unwrap_or(0)
        } else {
            0
        };
        self.cb.view.advisory_rate_ppm = advisory_rate_ppm(
            self.cb.view.advisory_rate_ppm,
            expected,
            inflation,
            target,
            gap,
            &p.rule,
        );
    }

    /// The central bank's state for the state hash.
    pub(super) fn cb_hash(&self, write: &mut impl FnMut(u64)) {
        if !self.cb.active {
            return;
        }
        write(u64::from(self.cb.view.credibility_ppm));
        #[allow(clippy::cast_sign_loss)]
        write(self.cb.view.advisory_rate_ppm as u64);
        for &b in &self.cb.in_band {
            write(u64::from(b));
        }
        #[allow(clippy::cast_sign_loss)]
        for b in &self.cb.monfin_year {
            write(b.get() as u64);
        }
    }
}
