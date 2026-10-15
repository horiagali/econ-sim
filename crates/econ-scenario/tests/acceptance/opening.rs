//! Acceptance tests for AC-OBS-01 to AC-OBS-10 (spec:
//! economy/opening-balance-sheet). Written in a test-authoring session from
//! the spec and its API sketch, before the code existed (ADR-0014).

use econ_ledger::{FlowCode, Instrument, Ledger, Sector};
use econ_scenario::{
    OpeningBalances, OpeningError, StatInstrument, opening_balances, read_sector_accounts,
    read_sector_details, reconcile,
};
use econ_types::Bani;
use serde_json::Value;

use super::support::{
    DETAILS_FIXTURE, INSTRUMENT_KEYS, SECTOR_KEYS, bal, col, fixture_json, fixture_text, hex, nfa,
    opening, opening_golden, opening_reference, row, row_sum, stat, statistics, statistics_json,
};

/// Every (instrument, sector) pair of the ledger.
fn cells() -> impl Iterator<Item = (Instrument, Sector)> {
    Instrument::ALL
        .into_iter()
        .flat_map(|i| Sector::ALL.into_iter().map(move |s| (i, s)))
}

/// The opening balances of changed financial accounts, read from JSON as the
/// builder reads the fixture.
fn map_json(v: &Value) -> Result<OpeningBalances, OpeningError> {
    read_sector_accounts(&v.to_string()).and_then(|a| opening_balances(&a))
}

/// AC-OBS-01 [unit] On the 2021 fixtures, every ledger instrument row of the opening balances sums to exactly 0 bani, and `cb_advances` is 0 for every sector.
#[test]
fn ac_obs_01() {
    // The ledger's rows and columns are those of the accounting matrix, in its order.
    assert_eq!(
        Instrument::ALL,
        [
            Instrument::Cash,
            Instrument::Deposits,
            Instrument::GovDepositsAtCb,
            Instrument::Reserves,
            Instrument::Bonds,
            Instrument::Loans,
            Instrument::CbAdvances,
            Instrument::Equity,
            Instrument::PensionClaims,
            Instrument::ForeignAssets,
        ]
    );
    assert_eq!(
        Sector::ALL,
        [
            Sector::Households,
            Sector::Firms,
            Sector::Banks,
            Sector::PensionFunds,
            Sector::OtherFinancial,
            Sector::Government,
            Sector::CentralBank,
            Sector::RestOfWorld,
        ]
    );
    let o = opening();
    assert_eq!(
        o.balances.len(),
        Instrument::ALL.len(),
        "one row per instrument"
    );
    for i in Instrument::ALL {
        assert_eq!(row_sum(o, i), Bani::ZERO, "row {i:?} sums to zero");
    }
    for s in Sector::ALL {
        assert_eq!(
            bal(o, Instrument::CbAdvances, s),
            Bani::ZERO,
            "cb_advances of {s:?}"
        );
    }
    // Not an empty matrix: the spec's table, rounded.
    assert!(bal(o, Instrument::Cash, Sector::CentralBank) < Bani::from_lei(-100_000_000_000));
    assert!(bal(o, Instrument::Bonds, Sector::Government) < Bani::from_lei(-400_000_000_000));
}

/// AC-OBS-02 [unit] Before rule 8, household `cash` and `deposits` equal the currency and deposits households hold in the financial accounts, to the ban. After rule 8, with any household totals within 10 millionths of those, they equal the given totals, every row still sums to zero, and only the central bank's `cash` and the banks' `deposits` change. Totals further off are rejected with a named error.
#[test]
fn ac_obs_02() {
    let o = opening();
    let cash = stat("assets", "currency", "households");
    let deposits = stat("assets", "deposits", "households");
    assert_eq!(bal(o, Instrument::Cash, Sector::Households), cash);
    assert_eq!(bal(o, Instrument::Deposits, Sector::Households), deposits);

    // 10 millionths of each figure, exactly: 57,326,000 and 294,173,000 bani.
    let (cash_tol, dep_tol) = (Bani(cash.get() / 100_000), Bani(deposits.get() / 100_000));
    assert_eq!(
        cash_tol.get() * 100_000,
        cash.get(),
        "the tolerance is whole bani"
    );
    assert_eq!(
        dep_tol.get() * 100_000,
        deposits.get(),
        "the tolerance is whole bani"
    );

    // The spec's differences at 1:1000, seed 42, then the edges of the tolerance.
    let accepted = [
        (cash + Bani(-2_726_837), deposits + Bani(9_575_511)),
        (cash + cash_tol, deposits - dep_tol),
        (cash - cash_tol, deposits + dep_tol),
        (cash, deposits),
    ];
    for (hh_cash, hh_deposits) in accepted {
        let r = reconcile(o, hh_cash, hh_deposits).expect("within 10 millionths");
        assert_eq!(bal(&r, Instrument::Cash, Sector::Households), hh_cash);
        assert_eq!(
            bal(&r, Instrument::Deposits, Sector::Households),
            hh_deposits
        );
        for i in Instrument::ALL {
            assert_eq!(row_sum(&r, i), Bani::ZERO, "row {i:?} after rule 8");
        }
        for (i, s) in cells() {
            let absorbs = matches!(
                (i, s),
                (Instrument::Cash, Sector::Households | Sector::CentralBank)
                    | (Instrument::Deposits, Sector::Households | Sector::Banks)
            );
            if !absorbs {
                assert_eq!(
                    bal(&r, i, s),
                    bal(o, i, s),
                    "{i:?} of {s:?} unchanged by rule 8"
                );
            }
        }
        assert_eq!(
            bal(&r, Instrument::Cash, Sector::CentralBank),
            bal(o, Instrument::Cash, Sector::CentralBank) - (hh_cash - cash)
        );
        assert_eq!(
            bal(&r, Instrument::Deposits, Sector::Banks),
            bal(o, Instrument::Deposits, Sector::Banks) - (hh_deposits - deposits)
        );
        assert_eq!(r.left_out, o.left_out, "rule 8 leaves the rest alone");
        assert_eq!(r.gaps, o.gaps);
    }

    for (hh_cash, hh_deposits, instrument) in [
        (cash + cash_tol + Bani(1), deposits, Instrument::Cash),
        (cash - cash_tol - Bani(1), deposits, Instrument::Cash),
        (cash, deposits + dep_tol + Bani(1), Instrument::Deposits),
        (cash, deposits - dep_tol - Bani(1), Instrument::Deposits),
    ] {
        assert_eq!(
            reconcile(o, hh_cash, hh_deposits),
            Err(OpeningError::HouseholdsTooFar { instrument }),
            "{hh_cash:?} {hh_deposits:?}"
        );
    }
}

/// AC-OBS-03 [unit] Each sector's net financial assets in the ledger, plus its left-out net derivatives and other accounts, equals its financial net worth in the statistics, within the closed gaps (at most 1 million lei in all).
#[test]
fn ac_obs_03() {
    let o = opening();
    let sheets = statistics_json();
    assert_eq!(
        o.gaps.len(),
        StatInstrument::ALL.len(),
        "one gap per instrument"
    );
    assert_eq!(o.left_out.len(), StatInstrument::LEFT_OUT.len());
    let mut closed = Bani::ZERO;
    for (k, &gap) in StatInstrument::ALL.iter().zip(&o.gaps) {
        let sum = |side: &str| -> Bani { SECTOR_KEYS.iter().map(|s| stat(side, k.key(), s)).sum() };
        assert_eq!(gap, sum("assets") - sum("liabilities"), "gap of {k:?}");
        closed += gap.abs();
    }
    assert!(
        closed <= Bani::from_lei(1_000_000),
        "closed gaps {closed:?}"
    );
    for (s, key) in Sector::ALL.into_iter().zip(SECTOR_KEYS) {
        let left_out: Bani = o.left_out.iter().map(|r| r[col(s)]).sum();
        let net_worth = Bani(sheets["financial_net_worth"][key].as_i64().expect("lei") * 100);
        let diff = nfa(o, s) + left_out - net_worth;
        assert!(diff.abs() <= closed, "{s:?}: off by {diff:?}");
    }
    // The left-out instruments are each sector's net position in them.
    for (k, r) in StatInstrument::LEFT_OUT.iter().zip(&o.left_out) {
        for (s, key) in Sector::ALL.into_iter().zip(SECTOR_KEYS) {
            let net = stat("assets", k.key(), key) - stat("liabilities", k.key(), key);
            if s != Sector::RestOfWorld {
                assert_eq!(r[col(s)], net, "{k:?} of {s:?}");
            }
        }
    }
}

/// AC-OBS-04 [unit] Deposits close by rule 4: the central bank's deposits abroad plus the banks' remainder equal the rest of the world's deposit liabilities exactly. Data where they do not are rejected with a named error and no output.
#[test]
fn ac_obs_04() {
    let o = opening();
    let a = |s: &str| stat("assets", "deposits", s);
    let l = |s: &str| stat("liabilities", "deposits", s);
    // Rule 4d: what banks owe, less what households, firms, pension funds,
    // other financial firms and the rest of the world hold, is held by banks
    // themselves and nets out: the banks' `deposits` balance is minus the rest.
    let held_by_others: Bani = [
        "households",
        "firms",
        "pension_funds",
        "other_financial",
        "rest_of_world",
    ]
    .iter()
    .map(|s| a(s))
    .sum();
    let interbank = l("banks") - held_by_others;
    assert_eq!(
        bal(o, Instrument::Deposits, Sector::Banks),
        -l("banks") + interbank
    );
    // Rule 4e: the banks' remainder, after reserves and their own deposits.
    let remainder = a("banks") - bal(o, Instrument::Reserves, Sector::Banks) - interbank;
    assert_eq!(a("central_bank") + remainder, l("rest_of_world"));
    // 4a to 4c: the government's deposits are at the central bank, and the
    // central bank's other deposit liabilities are the banks' reserves.
    let gov_at_cb = bal(o, Instrument::GovDepositsAtCb, Sector::Government);
    assert_eq!(
        gov_at_cb,
        a("government") - l("government").min(a("government"))
    );
    assert_eq!(
        bal(o, Instrument::GovDepositsAtCb, Sector::CentralBank),
        -gov_at_cb
    );
    assert_eq!(
        bal(o, Instrument::Reserves, Sector::CentralBank),
        -(l("central_bank") - gov_at_cb)
    );
    assert_eq!(
        bal(o, Instrument::Deposits, Sector::RestOfWorld),
        a("rest_of_world")
    );

    // Households issue deposits that firms hold: what residents hold abroad no
    // longer equals what the rest of the world owes.
    let mut v = statistics_json().clone();
    v["liabilities"]["deposits"]["households"] = Value::from(1_000_000);
    let firms = v["assets"]["deposits"]["firms"].as_i64().expect("lei");
    v["assets"]["deposits"]["firms"] = Value::from(firms + 1_000_000);
    assert_eq!(
        map_json(&v),
        Err(OpeningError::RuleDoesNotClose {
            rule: 4,
            instrument: StatInstrument::Deposits
        })
    );
}

/// AC-OBS-05 [unit] The rest of the world's `bonds` balance equals its holdings of debt securities in the statistics, and the central bank's `foreign_assets` include all its debt securities, deposits, gold and SDR (net of the SDR allocation).
#[test]
fn ac_obs_05() {
    let o = opening();
    assert_eq!(
        bal(o, Instrument::Bonds, Sector::RestOfWorld),
        stat("assets", "debt_securities", "rest_of_world")
    );
    let cb = "central_bank";
    let reserves = stat("assets", "debt_securities", cb)
        + stat("assets", "deposits", cb)
        + stat("assets", "gold_and_sdr", cb)
        - stat("liabilities", "gold_and_sdr", cb);
    let held = bal(o, Instrument::ForeignAssets, Sector::CentralBank);
    assert!(held >= reserves, "{held:?} < {reserves:?}");
    // Its securities are all foreign (rule 5b): it holds no government bonds.
    assert_eq!(bal(o, Instrument::Bonds, Sector::CentralBank), Bani::ZERO);
}

/// AC-OBS-06 [unit] Every balance of the opening ledger is posted with flow code `opening`, and the ledger's invariant check passes after posting.
#[test]
fn ac_obs_06() {
    let o = opening();
    let mut ledger = Ledger::new();
    o.post(&mut ledger).expect("the opening balances post");
    for (i, s) in cells() {
        assert_eq!(ledger.balance(s, i), bal(o, i, s), "{i:?} of {s:?}");
    }
    let postings = ledger.debug_postings();
    assert!(!postings.is_empty());
    for p in postings {
        assert_eq!(p.leg.code, FlowCode::OPENING, "{p:?}");
    }
    // Every non-zero balance was moved by some posting of its row and sector.
    for (i, s) in cells() {
        if !bal(o, i, s).is_zero() {
            assert!(
                postings
                    .iter()
                    .any(|p| p.leg.instrument == i && (p.leg.payer == s || p.leg.payee == s)),
                "{i:?} of {s:?} was not posted"
            );
        }
    }
    assert_eq!(ledger.check_invariants(), Ok(()));
    // What one sector holds another owes.
    assert_eq!(
        Sector::ALL
            .iter()
            .map(|&s| ledger.net_financial_assets(s))
            .sum::<Bani>(),
        Bani::ZERO
    );
}

/// AC-OBS-07 [golden] The opening hash on the 2021 fixtures, before rule 8, equals the committed golden value (`3e6e8302f2eded33` from the reference).
#[test]
fn ac_obs_07() {
    assert_eq!(opening_golden("statistics"), "3e6e8302f2eded33");
    assert_eq!(hex(opening().state_hash()), opening_golden("statistics"));
    // The same inputs give the same hash.
    let again = opening_balances(statistics()).expect("the fixture maps");
    assert_eq!(again.state_hash(), opening().state_hash());
}

/// AC-OBS-08 [diff] The Rust opening balances equal the Python reference's (`python/reference/opening_balance_reference.py`) for every instrument and sector.
#[test]
fn ac_obs_08() {
    let o = opening();
    let r = opening_reference();
    let amount = |v: &Value| Bani(v.as_i64().expect("bani"));
    for (i, ikey) in Instrument::ALL.into_iter().zip(INSTRUMENT_KEYS) {
        for (s, skey) in Sector::ALL.into_iter().zip(SECTOR_KEYS) {
            assert_eq!(
                bal(o, i, s),
                amount(&r["balances"][ikey][skey]),
                "{ikey} of {skey}"
            );
        }
    }
    for (k, lo) in StatInstrument::LEFT_OUT.iter().zip(&o.left_out) {
        for (s, skey) in Sector::ALL.into_iter().zip(SECTOR_KEYS) {
            assert_eq!(
                lo[col(s)],
                amount(&r["left_out"][k.key()][skey]),
                "{k:?} of {skey}"
            );
        }
    }
    for (k, &gap) in StatInstrument::ALL.iter().zip(&o.gaps) {
        assert_eq!(gap, amount(&r["gaps"][k.key()]), "gap of {k:?}");
    }
    assert_eq!(
        Value::from(hex(o.state_hash())),
        r["opening_hash"],
        "the hash of the same matrix"
    );
    // The row helper agrees with the reference's order of instruments.
    assert_eq!(row(Instrument::ForeignAssets), INSTRUMENT_KEYS.len() - 1);
}

/// AC-OBS-09 [unit] The sector details: the maturity buckets and the currencies each sum to the government debt total, the first bucket equals the measured short-term debt, and gold plus SDR holdings equal the central bank's gold and SDR in the financial accounts.
#[test]
fn ac_obs_09() {
    let d = read_sector_details(&fixture_text(DETAILS_FIXTURE)).expect("the fixture reads");
    let debt = &d.government_debt;
    let raw = fixture_json(DETAILS_FIXTURE);
    assert_eq!(
        debt.total,
        Bani(raw["government_debt"]["total"].as_i64().expect("lei") * 100)
    );
    assert_eq!(debt.total, Bani::from_lei(576_600_000_000));
    assert_eq!(
        debt.by_residual_maturity.iter().copied().sum::<Bani>(),
        debt.total
    );
    assert_eq!(debt.by_currency.iter().copied().sum::<Bani>(), debt.total);
    assert_eq!(debt.by_residual_maturity[0], debt.short_term);
    assert_eq!(debt.short_term, Bani::from_lei(29_380_000_000));
    let r = &d.central_bank_reserves;
    assert_eq!(
        r.gold + r.sdr_holdings,
        stat("assets", "gold_and_sdr", "central_bank")
    );
    assert_eq!(r.fx_deposits, stat("assets", "deposits", "central_bank"));
    assert_eq!(
        r.fx_securities,
        stat("assets", "debt_securities", "central_bank")
    );
    // Malformed details are a named error.
    assert!(read_sector_details("{\"government_debt\": 1}").is_err());
}

/// AC-OBS-10 [unit] Removing a sector or an instrument from the statistics, or making foreign notes exceed the banks' vault cash, gives a named error and no output.
#[test]
fn ac_obs_10() {
    let base = statistics_json();
    for (s, key) in Sector::ALL.into_iter().zip(SECTOR_KEYS) {
        let mut v = base.clone();
        v["sectors"]
            .as_array_mut()
            .expect("sectors")
            .retain(|x| x != key);
        for side in ["assets", "liabilities"] {
            for row in v[side].as_object_mut().expect("rows").values_mut() {
                row.as_object_mut().expect("a row").remove(key);
            }
        }
        assert_eq!(
            map_json(&v),
            Err(OpeningError::MissingSector(s)),
            "without {key}"
        );
    }
    for k in StatInstrument::ALL {
        let mut v = base.clone();
        v["instruments"]
            .as_array_mut()
            .expect("instruments")
            .retain(|x| x != k.key());
        for side in ["assets", "liabilities", "gap"] {
            v[side].as_object_mut().expect("rows").remove(k.key());
        }
        assert_eq!(
            map_json(&v),
            Err(OpeningError::MissingInstrument(k)),
            "without {k:?}"
        );
    }
    // Foreign notes one leu above what the central bank and the banks hold;
    // households hold the extra notes so that the instrument still balances.
    let mut v = base.clone();
    let held = ["banks", "central_bank"]
        .iter()
        .map(|s| v["assets"]["currency"][*s].as_i64().expect("lei"))
        .sum::<i64>();
    let extra = held + 1;
    v["liabilities"]["currency"]["rest_of_world"] = Value::from(extra);
    let hh = v["assets"]["currency"]["households"].as_i64().expect("lei");
    v["assets"]["currency"]["households"] = Value::from(hh + extra);
    assert_eq!(
        map_json(&v),
        Err(OpeningError::RuleDoesNotClose {
            rule: 2,
            instrument: StatInstrument::Currency
        })
    );
    // Not a financial-accounts file at all.
    assert!(matches!(
        read_sector_accounts("[1, 2, 3]"),
        Err(OpeningError::Parse(_))
    ));
}
