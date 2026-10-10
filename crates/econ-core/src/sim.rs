//! SIM mode: Godley & Lavoie (2007), *Monetary Economics*, chapter 3, model SIM.
//!
//! Sectors: households, firms (zero profit, pay all sales as wages), and a
//! government that buys goods and taxes income, financing its deficit by
//! issuing money (`Instrument::Cash`, a government liability).
//!
//! Per tick, with money stock `H₋₁` held by households:
//! ```text
//! Y  = (G + α2·H₋₁) / (1 − α1·(1 − θ))      national income (solved)
//! T  = θ·Y                                   taxes
//! YD = Y − T                                 disposable income
//! C  = Y − G   (= α1·YD + α2·H₋₁ up to 1 bani of rounding)
//! H  = H₋₁ + YD − C
//! ```
//! Rounding rules (shared with the Python reference): `Y` and `T` are rounded
//! half away from zero to whole bani; `C` is defined as `Y − G` so that the
//! goods market clears exactly.

use econ_ledger::{FlowCode, Instrument, Ledger, Sector, Txn};
use econ_types::Bani;

use crate::{Command, StateHasher, TickReport};

/// Parameters of model SIM.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimParams {
    /// Government spending per tick.
    pub g: Bani,
    /// Tax rate θ.
    pub theta: f64,
    /// Propensity to consume out of disposable income α1.
    pub alpha1: f64,
    /// Propensity to consume out of wealth α2.
    pub alpha2: f64,
}

impl Default for SimParams {
    /// The textbook values (G = 20, θ = 0.2, α1 = 0.6, α2 = 0.4), with G
    /// scaled to 20 million lei so rounding to bani is exercised.
    fn default() -> Self {
        SimParams {
            g: Bani::from_lei(20_000_000),
            theta: 0.2,
            alpha1: 0.6,
            alpha2: 0.4,
        }
    }
}

/// Deliberate bugs for mutation testing of the differential test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mutation {
    /// No bug.
    #[default]
    None,
    /// Taxes flow from government to households instead of the reverse.
    TaxSign,
    /// Consumption uses α1 on income Y instead of disposable income.
    ConsumeOnGrossIncome,
}

/// The SIM economy.
#[derive(Debug, Clone)]
pub struct SimModel {
    params: SimParams,
    ledger: Ledger,
    tick: u32,
    mutation: Mutation,
}

impl SimModel {
    /// A fresh economy with no money (the textbook starting point).
    #[must_use]
    pub fn new(params: SimParams) -> Self {
        let mut ledger = Ledger::new();
        ledger.allow_negative(Sector::Government, Instrument::Cash);
        SimModel {
            params,
            ledger,
            tick: 0,
            mutation: Mutation::None,
        }
    }

    /// Inject a deliberate bug (testing only).
    #[must_use]
    pub fn with_mutation(mut self, m: Mutation) -> Self {
        self.mutation = m;
        self
    }

    /// Read-only ledger access.
    #[must_use]
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// Advance one tick.
    ///
    /// # Panics
    /// If a posting is rejected (a bug) — the SIM model never overdraws.
    pub fn step(&mut self, cmds: &[Command]) -> TickReport {
        for c in cmds {
            match *c {
                Command::SetGovSpending(g) => self.params.g = g,
                Command::SetTaxRate(t) => self.params.theta = t,
            }
        }
        let tick = self.tick;
        self.ledger.begin_tick(tick);
        let p = self.params;
        let h_prev = self.ledger.balance(Sector::Households, Instrument::Cash);

        // Solve the simultaneous system for Y (one division, fixed op order).
        let denom = match self.mutation {
            Mutation::ConsumeOnGrossIncome => 1.0 - p.alpha1,
            _ => 1.0 - p.alpha1 * (1.0 - p.theta),
        };
        let y = Bani::round_f64(econ_num::finite(
            (p.g.to_f64_exact() + p.alpha2 * h_prev.to_f64_exact()) / denom,
            "SIM national income",
        ));
        let t = y.mul_rate(p.theta);
        let yd = y - t;
        let c = y - p.g;

        // Postings (each a linked transaction).
        let gov_tax_dir = if self.mutation == Mutation::TaxSign {
            (Sector::Government, Sector::Households)
        } else {
            (Sector::Households, Sector::Government)
        };
        let txn = Txn::new()
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
            )
            .leg(
                gov_tax_dir.0,
                gov_tax_dir.1,
                Instrument::Cash,
                t,
                FlowCode::TAX_INCOME,
            );
        self.ledger
            .commit(txn)
            .expect("SIM postings must never be rejected");

        let h = self.ledger.balance(Sector::Households, Instrument::Cash);
        let invariant_errors = match self.ledger.check_invariants() {
            Ok(()) => Vec::new(),
            Err(es) => es.into_iter().map(|e| format!("{e:?}")).collect(),
        };
        let mut hasher = StateHasher::default();
        hasher.write(&tick.to_le_bytes());
        self.ledger.hash_into(&mut |b| hasher.write(b));
        self.tick = tick.checked_add(1).expect("tick overflow");
        TickReport {
            tick,
            aggregates: vec![
                ("Y", y),
                ("T", t),
                ("YD", yd),
                ("C", c),
                ("G", p.g),
                ("H", h),
                ("GOV_DEFICIT", p.g - t),
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
        // Steady state of SIM: Y* = G/θ = 100 (scaled: 100 million lei), H* = α1... (H* = (1-α1)(1-θ)Y*/α2 = 80).
        let mut m = SimModel::new(SimParams::default());
        let mut last = None;
        for _ in 0..400 {
            last = Some(m.step(&[]));
        }
        let r = last.unwrap();
        let y = r.get("Y").unwrap();
        let h = r.get("H").unwrap();
        assert!(
            (y - Bani::from_lei(100_000_000)).abs() <= Bani(100),
            "Y={y}"
        );
        assert!((h - Bani::from_lei(80_000_000)).abs() <= Bani(100), "H={h}");
        assert!(r.invariant_errors.is_empty());
    }

    #[test]
    fn behavioural_consumption_matches_within_one_bani() {
        let p = SimParams::default();
        let mut m = SimModel::new(p);
        let mut h_prev = Bani::ZERO;
        for _ in 0..100 {
            let r = m.step(&[]);
            let yd = r.get("YD").unwrap();
            let c = r.get("C").unwrap();
            let behavioural = p.alpha1 * yd.to_f64_exact() + p.alpha2 * h_prev.to_f64_exact();
            assert!(
                (c.to_f64_exact() - behavioural).abs() <= 1.0,
                "C={c} vs {behavioural}"
            );
            h_prev = r.get("H").unwrap();
        }
    }

    #[test]
    fn deterministic_hashes() {
        let run = || {
            let mut m = SimModel::new(SimParams::default());
            (0..50).map(|_| m.step(&[]).state_hash).collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn invariants_hold_under_policy_changes() {
        let mut m = SimModel::new(SimParams::default());
        for i in 0..120u32 {
            let cmds: Vec<Command> = match i {
                30 => vec![Command::SetGovSpending(Bani::from_lei(25_000_000))],
                60 => vec![Command::SetTaxRate(0.25)],
                90 => vec![Command::SetGovSpending(Bani::from_lei(5_000_000))],
                _ => vec![],
            };
            let r = m.step(&cmds);
            assert!(
                r.invariant_errors.is_empty(),
                "tick {i}: {:?}",
                r.invariant_errors
            );
        }
    }
}
