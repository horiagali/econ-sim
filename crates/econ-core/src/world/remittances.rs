//! Remittances from linked emigrants (spec `economy/migration-remittances`,
//! step 3; feature `trade`, active with the outside world).
//!
//! Each linked emigrant sends, in euro, r̄ · ω^η · (1 − δ)^(months abroad / 12)
//! on the month's last day, paid in lei at the day's rate from the rest of the
//! world's lei deposits to its household's deposits (flow code
//! `transfer.remittances`, secondary income). r̄ is fitted at load so that the
//! start's links send the 2021 net flow at e0. Emigration, returns and the
//! diaspora stock come with the rest of the spec; until then the links are
//! the starting ones and `world.wages_abroad` (ω) stays at its starting 100%.

use econ_ledger::{FlowCode, Sector};
use econ_types::{Bani, Month};

use super::World;
use super::money::{Postings, bani, ratio};

/// Remittances' parameters (spec, "Tuning parameters").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemittanceParams {
    /// The 2021 net flow r̄ is fitted to, a year (17,248 million lei).
    pub annual_2021: Bani,
    /// `remittance_decay` δ, a year, millionths (5%).
    pub decay_ppm: u32,
    /// `remittance_elasticity` η to wages abroad, millionths (0.8).
    pub elasticity_ppm: u32,
}

impl RemittanceParams {
    /// The spec's defaults.
    #[must_use]
    pub fn spec_defaults() -> RemittanceParams {
        RemittanceParams {
            annual_2021: Bani::from_lei(17_248_000_000),
            decay_ppm: 50_000,
            elasticity_ppm: 800_000,
        }
    }
}

/// A month's remittances.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RemittanceMonth {
    /// The month they were paid in (its last day).
    pub month: Month,
    /// Paid to households, lei (national).
    pub lei: Bani,
    /// The same in euro cents.
    pub eur_cents: i64,
}

/// Remittances' state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Remittances {
    active: bool,
    /// r̄ in millionths of a euro cent a month.
    rbar_micro_cents: i128,
    /// Each linked emigrant's months abroad at the start.
    months_abroad0: Vec<u16>,
    /// The months paid, in order.
    pub(super) months: Vec<RemittanceMonth>,
    /// The same as migration's month reports (the other flows come with the
    /// rest of the spec).
    #[cfg(feature = "migration")]
    pub(super) migration_months: Vec<super::migration::MigrationMonth>,
}

/// (1 − δ)^(months / 12), millionths (pinned `libm`, ADR-0006).
fn decay_ppm(decay_ppm: u32, months: u32) -> i128 {
    let keep = 1.0 - f64::from(decay_ppm) / 1e6;
    let f = econ_num::math::powf(keep, f64::from(months) / 12.0);
    #[allow(clippy::cast_possible_truncation)]
    let v = (f * 1e6).round() as i128;
    v
}

impl World {
    /// Every month's remittances, in order.
    #[must_use]
    pub fn remittance_months(&self) -> &[RemittanceMonth] {
        &self.rem.months
    }

    /// Load: fit r̄ so that the start's links send the 2021 flow at e0.
    pub(super) fn start_remittances(&mut self) {
        let p = self.params.remittances;
        let links = &self.people.linked_emigrants;
        let mut weighted: i128 = 0;
        for (i, &h) in links.household_id.iter().enumerate() {
            let w = i128::from(self.households.hh_weight[h as usize]);
            weighted += w * decay_ppm(p.decay_ppm, u32::from(links.months_abroad[i]));
        }
        // Zero when trade does not act (no outside world).
        let e0 = i128::from(self.foreign().start_rate_ppm);
        self.rem = Remittances {
            active: weighted > 0 && e0 > 0,
            months_abroad0: links.months_abroad.clone(),
            ..Remittances::default()
        };
        if !self.rem.active {
            return;
        }
        // A month's flow in bani; a euro cent is e bani (e lei per euro).
        let month = i128::from(p.annual_2021.get()) / 12;
        // Σ w · decay/1e6 · r̄/1e6 · e0/1e6 = month.
        self.rem.rbar_micro_cents = ratio(month, 1_000_000_000_000_000_000, weighted * e0);
    }

    /// The month's last day, after pay: each link's remittance to its
    /// household, in lei at the day's rate.
    pub(super) fn pay_remittances(&mut self) {
        if !self.rem.active {
            return;
        }
        let p = self.params.remittances;
        let t = u32::try_from(self.months.len()).expect("a month");
        let e = i128::from(self.foreign().exchange_rate_ppm);
        // ω^η with ω at its starting 100% until `world.wages_abroad` is wired.
        let omega_ppm: i128 = 1_000_000;
        let n_hh = self.households.hh_weight.len();
        let mut per_hh = vec![0i128; n_hh];
        let mut cents: i128 = 0;
        let links = &self.people.linked_emigrants;
        for (i, &h) in links.household_id.iter().enumerate() {
            let h = h as usize;
            let w = i128::from(self.households.hh_weight[h]);
            if w == 0 {
                continue;
            }
            let d = decay_ppm(p.decay_ppm, u32::from(self.rem.months_abroad0[i]) + t);
            // Millionths of a cent → cents, then cents → bani at e.
            let c = ratio(
                self.rem.rbar_micro_cents * omega_ppm / 1_000_000,
                d,
                1_000_000_000_000,
            );
            per_hh[h] += ratio(c, e, 1_000_000);
            cents += w * c;
        }
        let mut total: i128 = 0;
        for (h, &a) in per_hh.iter().enumerate() {
            if a <= 0 {
                continue;
            }
            let a = bani(a);
            let hh = &mut self.households;
            hh.hh_deposits[h] += a;
            hh.hh_income_received[h] += a;
            hh.income_to_date[h] += a;
            self.hh.income_this_month[h] += a;
            total += i128::from(hh.hh_weight[h]) * i128::from(a.get());
        }
        let lei = bani(total);
        if lei > Bani::ZERO {
            let mut post = Postings::default();
            post.deposits(
                Sector::RestOfWorld,
                Sector::Households,
                lei,
                FlowCode::TRANSFER_REMITTANCES,
            );
            #[cfg(feature = "banking")]
            self.bank_intraday_advance(&mut post);
            post.commit(&mut self.ledger);
        }
        let eur_cents = i64::try_from(cents).expect("cents fit i64");
        self.rem.months.push(RemittanceMonth {
            month: Month(t),
            lei,
            eur_cents,
        });
        #[cfg(feature = "migration")]
        self.rem
            .migration_months
            .push(super::migration::MigrationMonth {
                month: Month(t),
                remittances: lei,
                remittances_eur_cents: eur_cents,
                ..super::migration::MigrationMonth::default()
            });
    }
}
