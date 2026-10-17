//! How money moves without banks (spec `game/world-loop`): the legs of each
//! kind of payment, posted at sector level. The records' own columns are
//! moved by the phases; the ledger holds the sector totals they must add up
//! to (I-4).

use econ_ledger::{FlowCode, Instrument, Ledger, Sector, Txn};
use econ_types::Bani;

use super::WorldError;

/// One linked transaction, flow code `opening`, that sets every balance of a
/// new ledger: each row is walked in sector order with its running sum, so
/// each sector ends at its balance (as the scenario builder posts it).
pub(super) fn post_opening(ledger: &mut Ledger, balances: &[[Bani; 8]]) -> Result<(), WorldError> {
    if balances.len() != Instrument::ALL.len() {
        return Err(WorldError::Shape(
            "the opening balances have another number of instruments",
        ));
    }
    let mut txn = Txn::new();
    for (instrument, row) in Instrument::ALL.into_iter().zip(balances) {
        let mut carried = Bani::ZERO;
        for (s, &balance) in row.iter().enumerate() {
            if balance < Bani::ZERO {
                ledger.allow_negative(Sector::ALL[s], instrument);
            }
            carried += balance;
            let Some(&next) = Sector::ALL.get(s.saturating_add(1)) else {
                break;
            };
            let here = Sector::ALL[s];
            if carried > Bani::ZERO {
                txn = txn.leg(next, here, instrument, carried, FlowCode::OPENING);
            } else if carried < Bani::ZERO {
                txn = txn.leg(here, next, instrument, -carried, FlowCode::OPENING);
            }
        }
        if !carried.is_zero() {
            return Err(WorldError::TablesDisagreeWithLedger {
                what: "an opening balance row that does not sum to zero",
            });
        }
    }
    ledger
        .commit(txn)
        .map_err(|_| WorldError::Shape("the opening balances do not post"))
}

/// A batch of postings of one phase, committed together.
#[derive(Debug, Default)]
pub(super) struct Postings {
    txn: Option<Txn>,
}

impl Postings {
    fn leg(
        &mut self,
        payer: Sector,
        payee: Sector,
        instrument: Instrument,
        amount: Bani,
        code: FlowCode,
    ) {
        if amount <= Bani::ZERO {
            return;
        }
        let txn = self.txn.take().unwrap_or_default();
        self.txn = Some(txn.leg(payer, payee, instrument, amount, code));
    }

    /// `payer` pays `payee` in bank deposits.
    pub(super) fn deposits(&mut self, payer: Sector, payee: Sector, amount: Bani, code: FlowCode) {
        self.leg(payer, payee, Instrument::Deposits, amount, code);
    }

    /// `payer` pays `payee` in currency.
    pub(super) fn cash(&mut self, payer: Sector, payee: Sector, amount: Bani, code: FlowCode) {
        self.leg(payer, payee, Instrument::Cash, amount, code);
    }

    /// The government pays a household, a firm or the rest of the world: its
    /// deposits at the central bank, the central bank's reserves to the
    /// banks, the banks' deposits to the payee.
    pub(super) fn gov_pays(&mut self, payee: Sector, amount: Bani, code: FlowCode) {
        self.leg(
            Sector::Government,
            Sector::CentralBank,
            Instrument::GovDepositsAtCb,
            amount,
            code,
        );
        self.leg(
            Sector::CentralBank,
            Sector::Banks,
            Instrument::Reserves,
            amount,
            code,
        );
        self.leg(Sector::Banks, payee, Instrument::Deposits, amount, code);
    }

    /// A household or a firm pays the government: the same legs in reverse.
    pub(super) fn pays_gov(&mut self, payer: Sector, amount: Bani, code: FlowCode) {
        self.leg(payer, Sector::Banks, Instrument::Deposits, amount, code);
        self.leg(
            Sector::Banks,
            Sector::CentralBank,
            Instrument::Reserves,
            amount,
            code,
        );
        self.leg(
            Sector::CentralBank,
            Sector::Government,
            Instrument::GovDepositsAtCb,
            amount,
            code,
        );
    }

    /// Firms pay their cash takings into the bank.
    pub(super) fn cash_into_bank(&mut self, amount: Bani) {
        self.leg(
            Sector::Firms,
            Sector::Banks,
            Instrument::Cash,
            amount,
            FlowCode::CASH_DEPOSIT,
        );
        self.leg(
            Sector::Banks,
            Sector::Firms,
            Instrument::Deposits,
            amount,
            FlowCode::CASH_DEPOSIT,
        );
    }

    /// Commit everything; a rejection is a bug of the phase that built it.
    pub(super) fn commit(self, ledger: &mut Ledger) {
        if let Some(txn) = self.txn {
            ledger
                .commit(txn)
                .expect("the World's postings are balanced and within the allowed signs");
        }
    }
}

/// `a × b / d`, rounded half away from zero, in i128.
pub(super) fn ratio(a: i128, b: i128, d: i128) -> i128 {
    assert!(d > 0, "a ratio over zero");
    let n = a.saturating_mul(b);
    let (q, r) = (n / d, n % d);
    if r.saturating_mul(2).abs() >= d {
        q + n.signum()
    } else {
        q
    }
}

/// An i128 amount as bani.
pub(super) fn bani(v: i128) -> Bani {
    Bani(i64::try_from(v).expect("an amount that fits i64 bani"))
}

/// Split `total` (≥ 0) over `weights` by largest remainder, ties to the
/// lower index; nothing to anyone if every weight is zero.
pub(super) fn split(total: i128, weights: &[i128]) -> Vec<i128> {
    let sum: i128 = weights.iter().map(|&w| w.max(0)).sum();
    if sum == 0 || total <= 0 {
        return vec![0; weights.len()];
    }
    let mut parts: Vec<i128> = weights.iter().map(|&w| total * w.max(0) / sum).collect();
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(total * weights[i].max(0) % sum), i));
    let left = usize::try_from(total - parts.iter().sum::<i128>()).unwrap_or(0);
    for &i in order.iter().take(left) {
        parts[i] += 1;
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_split_is_exact_and_proportional() {
        assert_eq!(split(10, &[1, 1, 1]), vec![4, 3, 3]);
        assert_eq!(split(7, &[0, 0]), vec![0, 0]);
        assert_eq!(split(100, &[3, 1, 0]), vec![75, 25, 0]);
    }

    #[test]
    fn ratios_round_half_away_from_zero() {
        assert_eq!(ratio(5, 1, 2), 3);
        assert_eq!(ratio(-5, 1, 2), -3);
        assert_eq!(ratio(4, 1, 3), 1);
    }
}
