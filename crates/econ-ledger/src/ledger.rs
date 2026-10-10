use std::collections::BTreeMap;

use econ_types::Bani;

use crate::FlowCode;

/// An institutional sector (columns of the SFC matrices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sector {
    /// Households (sum of synthetic households × weights).
    Households,
    /// Firms (sum of firm units × counts).
    Firms,
    /// Commercial banks.
    Banks,
    /// General government.
    Government,
    /// Central bank.
    CentralBank,
    /// Rest of world.
    RestOfWorld,
}

impl Sector {
    /// All sectors in matrix column order.
    pub const ALL: [Sector; 6] = [
        Sector::Households,
        Sector::Firms,
        Sector::Banks,
        Sector::Government,
        Sector::CentralBank,
        Sector::RestOfWorld,
    ];
}

/// A financial instrument (rows of the balance-sheet matrix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Instrument {
    /// Government-issued money (the "H" of the SIM model) / cash.
    Cash,
    /// Bank deposits.
    Deposits,
    /// Bank loans.
    Loans,
    /// Government bonds.
    Bonds,
}

impl Instrument {
    /// All instruments in matrix row order.
    pub const ALL: [Instrument; 4] = [
        Instrument::Cash,
        Instrument::Deposits,
        Instrument::Loans,
        Instrument::Bonds,
    ];
}

/// One leg: `amount` of `instrument` moves from `payer` to `payee`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leg {
    /// Sector whose balance decreases.
    pub payer: Sector,
    /// Sector whose balance increases.
    pub payee: Sector,
    /// What is moved.
    pub instrument: Instrument,
    /// How much; must be > 0.
    pub amount: Bani,
    /// Why.
    pub code: FlowCode,
}

/// A recorded leg (kept in a short debug buffer; ADR-0007 storage rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Posting {
    /// Tick in which it was posted.
    pub tick: u32,
    /// The leg.
    pub leg: Leg,
}

/// A linked multi-leg transaction: applied completely or not at all.
#[derive(Debug, Default, Clone)]
#[must_use = "a Txn does nothing until committed"]
pub struct Txn {
    legs: Vec<Leg>,
}

impl Txn {
    /// Empty transaction.
    pub fn new() -> Self {
        Txn { legs: Vec::new() }
    }

    /// Add a leg.
    pub fn leg(
        mut self,
        payer: Sector,
        payee: Sector,
        instrument: Instrument,
        amount: Bani,
        code: FlowCode,
    ) -> Self {
        self.legs.push(Leg {
            payer,
            payee,
            instrument,
            amount,
            code,
        });
        self
    }

    /// Add a leg whose amount is a per-unit amount times an integer record
    /// weight (`hh_weight` or `firm_count`), computed exactly (ADR-0007
    /// weighted-posting rule).
    pub fn weighted_leg(
        self,
        payer: Sector,
        payee: Sector,
        instrument: Instrument,
        per_unit: Bani,
        weight: u32,
        code: FlowCode,
    ) -> Self {
        self.leg(
            payer,
            payee,
            instrument,
            per_unit.times(i64::from(weight)),
            code,
        )
    }
}

/// Why a transaction was rejected (nothing was applied).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    /// A leg had amount ≤ 0.
    NonPositiveAmount(Leg),
    /// A leg had the same payer and payee.
    SelfTransfer(Leg),
    /// The transaction would leave a balance negative where that is not allowed.
    WouldGoNegative {
        /// Sector.
        sector: Sector,
        /// Instrument.
        instrument: Instrument,
        /// Balance after the transaction.
        balance: Bani,
    },
}

/// A failed invariant check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvariantError {
    /// I-1: an instrument row does not sum to zero across sectors.
    RowNotZero {
        /// Instrument.
        instrument: Instrument,
        /// The non-zero sum.
        sum: Bani,
    },
    /// I-2: a balance is negative where it may not be.
    NegativeBalance {
        /// Sector.
        sector: Sector,
        /// Instrument.
        instrument: Instrument,
        /// Balance.
        balance: Bani,
    },
    /// I-3: a TFM row (flow code) does not sum to zero.
    FlowRowNotZero {
        /// Flow code.
        code: FlowCode,
        /// Sum.
        sum: Bani,
    },
    /// I-3: a sector's TFM column does not equal its change in net financial position.
    ColumnMismatch {
        /// Sector.
        sector: Sector,
        /// Sum of the sector's flows this tick.
        flows: Bani,
        /// Change in the sector's net financial assets.
        delta_nfa: Bani,
    },
}

/// Flows of one tick by flow code and sector (the TFM / tag accumulators).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TickFlows {
    /// Signed flow per (code, sector): received positive, paid negative.
    pub by_code_sector: BTreeMap<(FlowCode, Sector), Bani>,
}

impl TickFlows {
    /// The signed flow of `code` for `sector` this tick.
    pub fn get(&self, code: FlowCode, sector: Sector) -> Bani {
        self.by_code_sector
            .get(&(code, sector))
            .copied()
            .unwrap_or(Bani::ZERO)
    }

    /// Total received by `sector` under `code` (absolute value of inflows).
    pub fn received(&self, code: FlowCode, sector: Sector) -> Bani {
        let v = self.get(code, sector);
        if v > Bani::ZERO { v } else { Bani::ZERO }
    }
}

/// The ledger.
#[derive(Debug, Clone)]
pub struct Ledger {
    balances: BTreeMap<(Sector, Instrument), Bani>,
    may_be_negative: BTreeMap<(Sector, Instrument), bool>,
    tick: u32,
    flows: TickFlows,
    nfa_at_tick_start: BTreeMap<Sector, Bani>,
    debug_buffer: Vec<Posting>,
    /// How many ticks of raw postings to keep (ADR-0007: 1–3).
    debug_ticks: u32,
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

impl Ledger {
    /// An empty ledger. By default no balance may go negative; declare
    /// liabilities with [`Ledger::allow_negative`].
    #[must_use]
    pub fn new() -> Self {
        let mut l = Ledger {
            balances: BTreeMap::new(),
            may_be_negative: BTreeMap::new(),
            tick: 0,
            flows: TickFlows::default(),
            nfa_at_tick_start: BTreeMap::new(),
            debug_buffer: Vec::new(),
            debug_ticks: 2,
        };
        for s in Sector::ALL {
            for i in Instrument::ALL {
                l.balances.insert((s, i), Bani::ZERO);
                l.may_be_negative.insert((s, i), false);
            }
        }
        l.begin_tick(0);
        l
    }

    /// Declare that `sector` may hold a negative balance of `instrument`
    /// (i.e. it is the issuer / debtor), e.g. government for `Cash`.
    pub fn allow_negative(&mut self, sector: Sector, instrument: Instrument) {
        self.may_be_negative.insert((sector, instrument), true);
    }

    /// Current balance.
    pub fn balance(&self, sector: Sector, instrument: Instrument) -> Bani {
        self.balances
            .get(&(sector, instrument))
            .copied()
            .unwrap_or(Bani::ZERO)
    }

    /// Net financial assets of a sector (sum over instruments).
    pub fn net_financial_assets(&self, sector: Sector) -> Bani {
        Instrument::ALL
            .iter()
            .map(|&i| self.balance(sector, i))
            .sum()
    }

    /// Start a new tick: reset the flow accumulators and remember opening positions.
    pub fn begin_tick(&mut self, tick: u32) {
        self.tick = tick;
        self.flows = TickFlows::default();
        self.nfa_at_tick_start = Sector::ALL
            .iter()
            .map(|&s| (s, self.net_financial_assets(s)))
            .collect();
        let keep_from = tick.saturating_sub(self.debug_ticks.saturating_sub(1));
        self.debug_buffer.retain(|p| p.tick >= keep_from);
    }

    /// Flows recorded so far this tick.
    #[must_use]
    pub fn flows(&self) -> &TickFlows {
        &self.flows
    }

    /// Raw postings of the last few ticks (debug only).
    #[must_use]
    pub fn debug_postings(&self) -> &[Posting] {
        &self.debug_buffer
    }

    /// Validate and apply a transaction atomically.
    ///
    /// # Errors
    /// If any leg is invalid or a balance would go negative where not
    /// allowed; in that case **nothing** is applied.
    pub fn commit(&mut self, txn: Txn) -> Result<(), LedgerError> {
        // 1. Validate legs and compute resulting balances without mutating.
        let mut pending: BTreeMap<(Sector, Instrument), Bani> = BTreeMap::new();
        for leg in &txn.legs {
            if leg.amount <= Bani::ZERO {
                return Err(LedgerError::NonPositiveAmount(*leg));
            }
            if leg.payer == leg.payee {
                return Err(LedgerError::SelfTransfer(*leg));
            }
            let pk = (leg.payer, leg.instrument);
            let qk = (leg.payee, leg.instrument);
            let pb = *pending.get(&pk).unwrap_or(&self.balance(pk.0, pk.1));
            pending.insert(pk, pb - leg.amount);
            let qb = *pending.get(&qk).unwrap_or(&self.balance(qk.0, qk.1));
            pending.insert(qk, qb + leg.amount);
        }
        for (&(s, i), &b) in &pending {
            if b < Bani::ZERO && !self.may_be_negative[&(s, i)] {
                return Err(LedgerError::WouldGoNegative {
                    sector: s,
                    instrument: i,
                    balance: b,
                });
            }
        }
        // 2. Apply.
        for (k, b) in pending {
            self.balances.insert(k, b);
        }
        for leg in txn.legs {
            if leg.code.is_flow() {
                let e = self
                    .flows
                    .by_code_sector
                    .entry((leg.code, leg.payer))
                    .or_insert(Bani::ZERO);
                *e -= leg.amount;
                let e = self
                    .flows
                    .by_code_sector
                    .entry((leg.code, leg.payee))
                    .or_insert(Bani::ZERO);
                *e += leg.amount;
            }
            self.debug_buffer.push(Posting {
                tick: self.tick,
                leg,
            });
        }
        Ok(())
    }

    /// Check invariants I-1, I-2 and I-3 for the current tick.
    ///
    /// # Errors
    /// The list of every failed check.
    pub fn check_invariants(&self) -> Result<(), Vec<InvariantError>> {
        let mut errs = Vec::new();
        // I-1: every instrument row sums to zero (independent re-summation).
        for i in Instrument::ALL {
            let sum: Bani = Sector::ALL.iter().map(|&s| self.balance(s, i)).sum();
            if !sum.is_zero() {
                errs.push(InvariantError::RowNotZero { instrument: i, sum });
            }
        }
        // I-2: sign constraints.
        for (&(s, i), &b) in &self.balances {
            if b < Bani::ZERO && !self.may_be_negative[&(s, i)] {
                errs.push(InvariantError::NegativeBalance {
                    sector: s,
                    instrument: i,
                    balance: b,
                });
            }
        }
        // I-3a: TFM rows sum to zero.
        let mut row: BTreeMap<FlowCode, Bani> = BTreeMap::new();
        for (&(c, _), &v) in &self.flows.by_code_sector {
            *row.entry(c).or_insert(Bani::ZERO) += v;
        }
        for (c, sum) in row {
            if !sum.is_zero() {
                errs.push(InvariantError::FlowRowNotZero { code: c, sum });
            }
        }
        // I-3b: each column = change in net financial assets.
        for s in Sector::ALL {
            let flows: Bani = self
                .flows
                .by_code_sector
                .iter()
                .filter(|((_, sec), _)| *sec == s)
                .map(|(_, v)| *v)
                .sum();
            let delta_nfa = self.net_financial_assets(s) - self.nfa_at_tick_start[&s];
            if flows != delta_nfa {
                errs.push(InvariantError::ColumnMismatch {
                    sector: s,
                    flows,
                    delta_nfa,
                });
            }
        }
        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }

    /// Feed every balance into a hasher in a fixed order (for state hashes).
    pub fn hash_into(&self, h: &mut impl FnMut(&[u8])) {
        for (&(s, i), b) in &self.balances {
            h(&[s as u8, i as u8]);
            h(&b.get().to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn gov_money_ledger() -> Ledger {
        let mut l = Ledger::new();
        l.allow_negative(Sector::Government, Instrument::Cash);
        l
    }

    #[test]
    fn simple_payment_balances() {
        let mut l = gov_money_ledger();
        l.commit(Txn::new().leg(
            Sector::Government,
            Sector::Firms,
            Instrument::Cash,
            Bani(100),
            FlowCode::GOV_PURCHASES,
        ))
        .unwrap();
        assert_eq!(l.balance(Sector::Firms, Instrument::Cash), Bani(100));
        assert_eq!(l.balance(Sector::Government, Instrument::Cash), Bani(-100));
        l.check_invariants().unwrap();
    }

    #[test]
    fn atomic_rejection_applies_nothing() {
        let mut l = gov_money_ledger();
        let before = l.clone();
        // Second leg overdraws households → whole txn rejected.
        let r = l.commit(
            Txn::new()
                .leg(
                    Sector::Government,
                    Sector::Households,
                    Instrument::Cash,
                    Bani(50),
                    FlowCode::WAGES,
                )
                .leg(
                    Sector::Households,
                    Sector::Firms,
                    Instrument::Cash,
                    Bani(80),
                    FlowCode::CONSUMPTION,
                ),
        );
        assert!(matches!(r, Err(LedgerError::WouldGoNegative { .. })));
        assert_eq!(l.balances, before.balances);
        assert_eq!(l.flows, before.flows);
    }

    #[test]
    fn rejects_bad_legs() {
        let mut l = gov_money_ledger();
        assert!(matches!(
            l.commit(Txn::new().leg(
                Sector::Firms,
                Sector::Firms,
                Instrument::Cash,
                Bani(1),
                FlowCode::WAGES
            )),
            Err(LedgerError::SelfTransfer(_))
        ));
        assert!(matches!(
            l.commit(Txn::new().leg(
                Sector::Government,
                Sector::Firms,
                Instrument::Cash,
                Bani(0),
                FlowCode::WAGES
            )),
            Err(LedgerError::NonPositiveAmount(_))
        ));
    }

    #[test]
    fn weighted_leg_multiplies_exactly() {
        let mut l = gov_money_ledger();
        l.commit(Txn::new().weighted_leg(
            Sector::Government,
            Sector::Households,
            Instrument::Cash,
            Bani(1234),
            100,
            FlowCode::WAGES,
        ))
        .unwrap();
        assert_eq!(
            l.balance(Sector::Households, Instrument::Cash),
            Bani(123_400)
        );
    }

    #[test]
    fn invariants_detect_corruption() {
        let mut l = gov_money_ledger();
        l.commit(Txn::new().leg(
            Sector::Government,
            Sector::Firms,
            Instrument::Cash,
            Bani(10),
            FlowCode::GOV_PURCHASES,
        ))
        .unwrap();
        // Simulate a bug that writes a balance directly (only possible inside this crate).
        l.balances
            .insert((Sector::Firms, Instrument::Cash), Bani(11));
        let errs = l.check_invariants().unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, InvariantError::RowNotZero { .. }))
        );
        assert!(
            errs.iter()
                .any(|e| matches!(e, InvariantError::ColumnMismatch { .. }))
        );
    }

    proptest! {
        /// Any sequence of valid payments keeps every invariant.
        #[test]
        fn random_payments_keep_invariants(ops in prop::collection::vec((0usize..6, 0usize..6, 1i64..1_000_000), 1..200)) {
            let mut l = Ledger::new();
            for s in Sector::ALL { l.allow_negative(s, Instrument::Cash); }
            for (a, b, amt) in ops {
                if a == b { continue; }
                l.commit(Txn::new().leg(Sector::ALL[a], Sector::ALL[b], Instrument::Cash, Bani(amt), FlowCode::WAGES)).unwrap();
            }
            prop_assert!(l.check_invariants().is_ok());
        }
    }
}
