//! PC mode: Godley & Lavoie (2007), *Monetary Economics*, chapter 4, model PC
//! ("portfolio choice"), the follow-up of Spike 2.
//!
//! Sectors: households, firms (zero profit, pay all sales as wages), a
//! government that buys goods, taxes income and pays interest on its bills
//! (`Instrument::Bonds`), and a central bank that sets the bill rate, buys the
//! bills households do not want and issues cash (`Instrument::Cash`, a central
//! bank liability), remitting its interest income to the government.
//!
//! Per tick, with `r₋₁` the previous tick's rate and stocks of the previous tick:
//! ```text
//! iH  = r₋₁·Bh₋₁,  iCB = r₋₁·Bcb₋₁                     interest on bills
//! Y   = (G + α2·V₋₁ + α1·(1 − θ)·iH) / (1 − α1·(1 − θ)) national income (solved)
//! T   = θ·(Y + iH)                                      taxes
//! YD  = Y + iH − T                                      disposable income
//! C   = Y − G   (= α1·YD + α2·V₋₁ up to 1 bano of rounding)
//! V   = V₋₁ + YD − C                                    household wealth
//! Bh  = λ0·V + λ1·r·V − λ2·YD                           bills demanded
//! Hh  = V − Bh                                          cash held
//! Bs  = Bs₋₁ + G + iH − T                               bills issued
//! Bcb = Bs − Bh                                         bills the central bank buys
//! ```
//! Rounding rules (shared with `python/reference/pc_reference.py`): `Y`, `T`,
//! `Bh` and each interest payment are rounded half away from zero to whole
//! bani; `C` is `Y − G` so that the goods market clears exactly.

use econ_ledger::{FlowCode, Instrument, Ledger, Sector, Txn};
use econ_types::Bani;

use crate::{StateHasher, TickReport};

/// Parameters of model PC.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcParams {
    /// Government spending per tick.
    pub g: Bani,
    /// Tax rate θ.
    pub theta: f64,
    /// Propensity to consume out of disposable income α1.
    pub alpha1: f64,
    /// Propensity to consume out of wealth α2.
    pub alpha2: f64,
    /// Share of wealth held in bills at a zero rate λ0.
    pub lambda0: f64,
    /// Response of the bill share to the rate λ1.
    pub lambda1: f64,
    /// Response of the bill share to income over wealth (transactions demand
    /// for cash) λ2.
    pub lambda2: f64,
    /// Bill rate r set by the central bank.
    pub r: f64,
}

impl Default for PcParams {
    /// The textbook values (G = 20, θ = 0.2, α1 = 0.6, α2 = 0.4, λ0 = 0.635,
    /// λ1 = 5, λ2 = 0.01, r = 2.5%), with G scaled to 20 million lei so
    /// rounding to bani is exercised.
    fn default() -> Self {
        PcParams {
            g: Bani::from_lei(20_000_000),
            theta: 0.2,
            alpha1: 0.6,
            alpha2: 0.4,
            lambda0: 0.635,
            lambda1: 5.0,
            lambda2: 0.01,
            r: 0.025,
        }
    }
}

/// Deliberate bugs for mutation testing of the differential test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PcMutation {
    /// No bug.
    #[default]
    None,
    /// The government's interest leaves disposable income out (taxed, never received).
    InterestOutOfIncome,
    /// The demand for bills falls with the rate (λ1 with the wrong sign).
    PortfolioSign,
}

/// The PC economy.
#[derive(Debug, Clone)]
pub struct PcModel {
    params: PcParams,
    /// The rate in force in the previous tick (interest is paid at it).
    r_prev: f64,
    ledger: Ledger,
    tick: u32,
    mutation: PcMutation,
}

/// Add a leg only when its amount is positive, swapping the direction of a
/// negative one.
fn leg_signed(
    txn: Txn,
    payer: Sector,
    payee: Sector,
    instrument: Instrument,
    amount: Bani,
    code: FlowCode,
) -> Txn {
    if amount > Bani::ZERO {
        txn.leg(payer, payee, instrument, amount, code)
    } else if amount < Bani::ZERO {
        txn.leg(payee, payer, instrument, -amount, code)
    } else {
        txn
    }
}

impl PcModel {
    /// A fresh economy with no money and no bills (the textbook starting point).
    #[must_use]
    pub fn new(params: PcParams) -> Self {
        let mut ledger = Ledger::new();
        ledger.allow_negative(Sector::CentralBank, Instrument::Cash);
        ledger.allow_negative(Sector::Government, Instrument::Bonds);
        PcModel {
            params,
            r_prev: params.r,
            ledger,
            tick: 0,
            mutation: PcMutation::None,
        }
    }

    /// Inject a deliberate bug (testing only).
    #[must_use]
    pub fn with_mutation(mut self, m: PcMutation) -> Self {
        self.mutation = m;
        self
    }

    /// Set the bill rate from this tick on (interest at the new rate is paid
    /// from the next tick).
    pub fn set_rate(&mut self, r: f64) {
        self.params.r = r;
    }

    /// Read-only ledger access.
    #[must_use]
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// Advance one tick.
    ///
    /// # Panics
    /// If a posting is rejected (a bug): the PC model never overdraws.
    pub fn step(&mut self) -> TickReport {
        let tick = self.tick;
        self.ledger.begin_tick(tick);
        let p = self.params;
        let bh_prev = self.ledger.balance(Sector::Households, Instrument::Bonds);
        let bcb_prev = self.ledger.balance(Sector::CentralBank, Instrument::Bonds);
        let v_prev = self.ledger.net_financial_assets(Sector::Households);

        let ih = bh_prev.mul_rate(self.r_prev);
        let icb = bcb_prev.mul_rate(self.r_prev);
        let ih_income = match self.mutation {
            PcMutation::InterestOutOfIncome => Bani::ZERO,
            _ => ih,
        };
        // Solve the simultaneous system for Y (fixed op order).
        let y = Bani::round_f64(econ_num::finite(
            (p.g.to_f64_exact()
                + p.alpha2 * v_prev.to_f64_exact()
                + p.alpha1 * (1.0 - p.theta) * ih_income.to_f64_exact())
                / (1.0 - p.alpha1 * (1.0 - p.theta)),
            "PC national income",
        ));
        let t = (y + ih).mul_rate(p.theta);
        let yd = y + ih - t;
        let c = y - p.g;
        let v = v_prev + yd - c;
        let lambda1 = match self.mutation {
            PcMutation::PortfolioSign => -p.lambda1,
            _ => p.lambda1,
        };
        let bh = Bani::round_f64(econ_num::finite(
            p.lambda0 * v.to_f64_exact() + lambda1 * p.r * v.to_f64_exact()
                - p.lambda2 * yd.to_f64_exact(),
            "PC bills demanded",
        ));
        let d_bh = bh - bh_prev;
        // The government sells the central bank what households do not buy.
        let d_bs = p.g + ih - t;
        let d_bcb = d_bs - d_bh;

        let mut txn = Txn::new()
            .leg(
                Sector::Government,
                Sector::Firms,
                Instrument::Cash,
                p.g,
                FlowCode::GOV_PURCHASES,
            )
            .leg(
                Sector::Firms,
                Sector::Households,
                Instrument::Cash,
                y,
                FlowCode::WAGES,
            )
            .leg(
                Sector::Households,
                Sector::Firms,
                Instrument::Cash,
                c,
                FlowCode::CONSUMPTION,
            );
        txn = leg_signed(
            txn,
            Sector::Government,
            Sector::Households,
            Instrument::Cash,
            ih,
            FlowCode::INTEREST_GOV_DEBT,
        );
        txn = leg_signed(
            txn,
            Sector::Government,
            Sector::CentralBank,
            Instrument::Cash,
            icb,
            FlowCode::INTEREST_GOV_DEBT,
        );
        txn = leg_signed(
            txn,
            Sector::CentralBank,
            Sector::Government,
            Instrument::Cash,
            icb,
            FlowCode::TRANSFER_CB_PROFIT,
        );
        txn = leg_signed(
            txn,
            Sector::Households,
            Sector::Government,
            Instrument::Cash,
            t,
            FlowCode::TAX_INCOME,
        );
        // Bills change hands for cash at book value.
        txn = leg_signed(
            txn,
            Sector::Government,
            Sector::Households,
            Instrument::Bonds,
            d_bh,
            FlowCode::BONDS,
        );
        txn = leg_signed(
            txn,
            Sector::Households,
            Sector::Government,
            Instrument::Cash,
            d_bh,
            FlowCode::BONDS,
        );
        txn = leg_signed(
            txn,
            Sector::Government,
            Sector::CentralBank,
            Instrument::Bonds,
            d_bcb,
            FlowCode::BONDS,
        );
        txn = leg_signed(
            txn,
            Sector::CentralBank,
            Sector::Government,
            Instrument::Cash,
            d_bcb,
            FlowCode::BONDS,
        );
        self.ledger
            .commit(txn)
            .expect("PC postings must never be rejected");

        let hh = self.ledger.balance(Sector::Households, Instrument::Cash);
        let bs = -self.ledger.balance(Sector::Government, Instrument::Bonds);
        let bcb = self.ledger.balance(Sector::CentralBank, Instrument::Bonds);
        let invariant_errors = match self.ledger.check_invariants() {
            Ok(()) => Vec::new(),
            Err(es) => es.into_iter().map(|e| format!("{e:?}")).collect(),
        };
        let mut hasher = StateHasher::default();
        hasher.write(&tick.to_le_bytes());
        self.ledger.hash_into(&mut |b| hasher.write(b));
        self.r_prev = p.r;
        self.tick = tick.checked_add(1).expect("tick overflow");
        TickReport {
            tick,
            aggregates: vec![
                ("Y", y),
                ("T", t),
                ("YD", yd),
                ("C", c),
                ("G", p.g),
                ("V", v),
                ("BH", bh),
                ("HH", hh),
                ("BS", bs),
                ("BCB", bcb),
                ("IH", ih),
                ("ICB", icb),
            ],
            state_hash: hasher.finish(),
            invariant_errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converges_to_textbook_steady_state() {
        // Steady state of PC at r = 2.5%: Y* ≈ 106.5, V* ≈ 86.5 (scaled by a million lei).
        let mut m = PcModel::new(PcParams::default());
        let mut last = None;
        for _ in 0..400 {
            last = Some(m.step());
            assert!(last.as_ref().unwrap().invariant_errors.is_empty());
        }
        let r = last.unwrap();
        let y = r.get("Y").unwrap();
        let v = r.get("V").unwrap();
        assert!(
            (y - Bani::from_lei(106_486_486)).abs() <= Bani::from_lei(10),
            "Y={y}"
        );
        assert!(
            (v - Bani::from_lei(86_486_486)).abs() <= Bani::from_lei(10),
            "V={v}"
        );
        // Cash supplied by the central bank equals cash held (the redundant equation).
        assert_eq!(
            -m.ledger().balance(Sector::CentralBank, Instrument::Cash),
            r.get("HH").unwrap()
        );
    }

    #[test]
    fn a_higher_rate_raises_income_in_the_long_run() {
        let run = |rate_change: bool| {
            let mut m = PcModel::new(PcParams::default());
            let mut y = Bani::ZERO;
            for i in 0..400 {
                if rate_change && i == 100 {
                    m.set_rate(0.035);
                }
                y = m.step().get("Y").unwrap();
            }
            y
        };
        assert!(run(true) > run(false));
    }
}
