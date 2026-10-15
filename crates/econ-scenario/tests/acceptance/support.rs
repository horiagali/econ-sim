//! Shared by the acceptance tests of the opening balance sheet (AC-OBS-*) and
//! the scenario builder (AC-SCB-*): the fixtures, the matrix order, the golden
//! file of the opening hash and the Python references.

// Each feature's tests use a part of this file.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use econ_ledger::{Instrument, Sector};
use econ_scenario::{OpeningBalances, SectorAccounts, opening_balances, read_sector_accounts};
use econ_types::Bani;
use serde_json::Value;

/// The financial accounts.
pub const SHEETS_FIXTURE: &str = "balance_sheets2021q4_ro.json";
/// Government debt and central bank reserves.
pub const DETAILS_FIXTURE: &str = "sector_details2021_ro.json";
/// The golden file of the opening hash: `statistics HASH` before rule 8, then
/// `SCALE HASH` after rule 8 at seed 42.
pub const OPENING_GOLDEN: &str = "opening_ro_2021.hashes";

/// The ledger's sectors in matrix column order, as the fixture and the
/// reference name them.
pub const SECTOR_KEYS: [&str; 8] = [
    "households",
    "firms",
    "banks",
    "pension_funds",
    "other_financial",
    "government",
    "central_bank",
    "rest_of_world",
];

/// The ledger's instruments in matrix row order, as the reference names them.
pub const INSTRUMENT_KEYS: [&str; 10] = [
    "cash",
    "deposits",
    "gov_deposits_at_cb",
    "reserves",
    "bonds",
    "loans",
    "cb_advances",
    "equity",
    "pension_claims",
    "foreign_assets",
];

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fixtures_dir() -> PathBuf {
    repo_root().join("python/pipeline/fixtures")
}

pub fn fixture_text(name: &str) -> String {
    let path = fixtures_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture {}: {e}", path.display()))
}

pub fn fixture_json(name: &str) -> Value {
    serde_json::from_str(&fixture_text(name)).expect("valid JSON")
}

/// The column of a sector in `[_; 8]` rows.
pub fn col(sector: Sector) -> usize {
    Sector::ALL
        .iter()
        .position(|&s| s == sector)
        .expect("a ledger sector")
}

/// The row of a ledger instrument in [`OpeningBalances::balances`].
pub fn row(instrument: Instrument) -> usize {
    Instrument::ALL
        .iter()
        .position(|&i| i == instrument)
        .expect("a ledger instrument")
}

/// One balance.
pub fn bal(o: &OpeningBalances, instrument: Instrument, sector: Sector) -> Bani {
    o.balances[row(instrument)][col(sector)]
}

/// The sum of an instrument row.
pub fn row_sum(o: &OpeningBalances, instrument: Instrument) -> Bani {
    o.balances[row(instrument)].iter().copied().sum()
}

/// Net financial assets of a sector: the sum of its column.
pub fn nfa(o: &OpeningBalances, sector: Sector) -> Bani {
    o.balances.iter().map(|r| r[col(sector)]).sum()
}

/// Lei of a fixture as bani.
pub fn lei(v: &Value) -> Bani {
    Bani(v.as_i64().expect("an amount in lei") * 100)
}

/// A figure of the financial accounts, in bani.
pub fn stat(side: &str, instrument: &str, sector: &str) -> Bani {
    lei(&statistics_json()[side][instrument][sector])
}

pub fn statistics_json() -> &'static Value {
    static V: OnceLock<Value> = OnceLock::new();
    V.get_or_init(|| fixture_json(SHEETS_FIXTURE))
}

/// The financial accounts of the fixture.
pub fn statistics() -> &'static SectorAccounts {
    static A: OnceLock<SectorAccounts> = OnceLock::new();
    A.get_or_init(|| read_sector_accounts(&fixture_text(SHEETS_FIXTURE)).expect("fixture reads"))
}

/// The opening balances of the fixture, before rule 8.
pub fn opening() -> &'static OpeningBalances {
    static O: OnceLock<OpeningBalances> = OnceLock::new();
    O.get_or_init(|| opening_balances(statistics()).expect("the fixture maps"))
}

/// The lines `KEY HASH` of the opening golden file.
pub fn opening_golden(key: &str) -> String {
    let path = repo_root().join("tests/golden").join(OPENING_GOLDEN);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("golden file {}: {e}", path.display()));
    text.lines()
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            Some((parts.next()?, parts.next()?))
        })
        .find(|&(k, _)| k == key)
        .unwrap_or_else(|| panic!("{OPENING_GOLDEN} has no line {key}"))
        .1
        .to_string()
}

pub fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// Run a Python script of the repository and parse what it prints as JSON.
/// A non-zero exit is accepted when it still prints JSON (the reference
/// reports failed checks that way).
pub fn python_json(script: &str, args: &[String]) -> Value {
    let script = repo_root().join(script);
    let mut last = String::from("no Python found");
    for python in ["python3", "python"] {
        match Command::new(python).arg(&script).args(args).output() {
            Ok(out) => match serde_json::from_slice(&out.stdout) {
                Ok(v) => return v,
                Err(e) => {
                    last = format!("{e}: {}", String::from_utf8_lossy(&out.stderr));
                }
            },
            Err(e) => last = e.to_string(),
        }
    }
    panic!(
        "the differential test needs Python 3 on PATH to run {}: {last}",
        script.display()
    );
}

/// The Python reference of the opening balance sheet on the fixtures.
pub fn opening_reference() -> &'static Value {
    static V: OnceLock<Value> = OnceLock::new();
    V.get_or_init(|| python_json("python/reference/opening_balance_reference.py", &[]))
}
