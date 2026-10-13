//! Acceptance test for AC-ACC-03 (spec: economy/accounting). Written in a
//! test-authoring session from the spec and the ledger as it is (ADR-0014).

use econ_ledger::{FlowCode, Instrument, Ledger, Sector, Txn};
use econ_types::Bani;

/// A small deterministic generator for test postings (xorshift64).
struct Draws(u64);

impl Draws {
    fn next(&mut self, below: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % below
    }

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[usize::try_from(self.next(from.len() as u64)).expect("small index")]
    }
}

fn assert_sums_to_zero(ledger: &Ledger, when: &str) {
    let total: Bani = Sector::ALL
        .iter()
        .map(|&s| ledger.net_financial_assets(s))
        .sum();
    assert_eq!(total, Bani(0), "{when}: sector balances sum to {total:?}");
    for i in Instrument::ALL {
        let row: Bani = Sector::ALL.iter().map(|&s| ledger.balance(s, i)).sum();
        assert_eq!(row, Bani(0), "{when}: {i:?} sums to {row:?}");
    }
}

/// AC-ACC-03 [ledger] Sum of sector financial balances is 0 every tick.
#[test]
fn ac_acc_03() {
    let mut ledger = Ledger::new();
    // Only the government and the central bank may run negative, so that
    // some transfers are refused along the way.
    for i in Instrument::ALL {
        ledger.allow_negative(Sector::Government, i);
        ledger.allow_negative(Sector::CentralBank, i);
    }
    ledger.begin_tick(0);
    assert_sums_to_zero(&ledger, "empty ledger");
    let mut opening = Txn::new();
    for s in Sector::ALL {
        if s != Sector::Government {
            for i in Instrument::ALL {
                opening = opening.leg(
                    Sector::Government,
                    s,
                    i,
                    Bani::from_lei(1_000_000),
                    FlowCode::OPENING,
                );
            }
        }
    }
    ledger.commit(opening).expect("opening balances");
    assert_sums_to_zero(&ledger, "after the opening balances");

    let codes = [
        FlowCode::CONSUMPTION,
        FlowCode::GOV_PURCHASES,
        FlowCode::WAGES,
        FlowCode::TAX_INCOME,
        FlowCode::TAX_VAT,
    ];
    let mut draws = Draws(0x9E37_79B9_7F4A_7C15);
    let (mut posted, mut refused) = (0, 0);
    for tick in 1..=365 {
        ledger.begin_tick(tick);
        for _ in 0..20 {
            let mut txn = Txn::new();
            for _ in 0..=draws.next(4) {
                let payer = draws.pick(&Sector::ALL);
                let mut payee = draws.pick(&Sector::ALL);
                if payee == payer {
                    payee = Sector::ALL[(Sector::ALL
                        .iter()
                        .position(|&s| s == payer)
                        .expect("listed")
                        + 1)
                        % Sector::ALL.len()];
                }
                let amount = Bani(i64::try_from(1 + draws.next(5_000_000_000)).expect("fits"));
                let weight = u32::try_from(1 + draws.next(300)).expect("fits");
                txn = txn.weighted_leg(
                    payer,
                    payee,
                    draws.pick(&Instrument::ALL),
                    amount,
                    weight,
                    draws.pick(&codes),
                );
            }
            match ledger.commit(txn) {
                Ok(()) => posted += 1,
                Err(_) => refused += 1,
            }
        }
        assert_sums_to_zero(&ledger, &format!("tick {tick}"));
        ledger
            .check_invariants()
            .unwrap_or_else(|e| panic!("tick {tick}: {e:?}"));
    }
    // Both paths were exercised.
    assert!(posted > 1_000, "{posted} transactions posted");
    assert!(refused > 0, "no transaction was refused");
}
