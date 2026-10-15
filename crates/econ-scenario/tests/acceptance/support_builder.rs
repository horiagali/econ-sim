//! Shared by the acceptance tests of the scenario builder (AC-SCB-*): builds
//! at each scale (run once), written folders, the `econ-cli` binary, Arrow and
//! JSON readers, and SHA-256.

#![allow(dead_code)]

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::OnceLock;

use arrow_array::{
    Array, Int64Array, RecordBatch, StringArray, UInt8Array, UInt16Array, UInt32Array, UInt64Array,
};
use arrow_ipc::reader::FileReader;
use econ_scenario::{BuildParams, Inputs, Scenario, build, read_inputs, write};
use serde_json::Value;

use super::support::{fixtures_dir, python_json, repo_root};

/// The seed of the scenarios of the tests.
pub const SEED: u64 = 42;
/// "A different seed" (AC-SCB-07).
pub const OTHER_SEED: u64 = 7;

/// The checks of the spec's table, in its order.
pub const CHECKS: [&str; 14] = [
    "persons",
    "wage_bill",
    "pension_bill",
    "mixed_income",
    "wages_against_input_output_table",
    "household_balance_sheet",
    "deposits",
    "employers",
    "firm_units_against_input_output_table",
    "income_against_consumption",
    "income_distribution",
    "declared_wages",
    "ledger_invariants",
    "households_against_ledger",
];
/// The two checks of the spec's table after `households_against_ledger`.
pub const CHECKS_AFTER: [&str; 2] = ["firms_against_ledger", "left_out"];
/// The only check that may fail, as a known gap, until roadmap M3.
pub const KNOWN_GAP: &str = "income_distribution";

/// Every file of the folder.
pub const FILES: [&str; 6] = [
    "scenario.json",
    "tables/households.arrow",
    "tables/persons.arrow",
    "tables/firm_units.arrow",
    "balances.arrow",
    "report.json",
];

/// The input files of the spec's table.
pub const INPUT_FILES: [&str; 13] = [
    "census2021_margins_ro.json",
    "census2021_edu_activity_ro.json",
    "census2021_jobs_ro.json",
    "census2021_housing_ro.json",
    "earnings2021_ro.json",
    "pensions2021_ro.json",
    "household_wealth2021_ro.json",
    "other_incomes2021_ro.json",
    "firm_units2021_ro.json",
    "firm_balance_sheets2021_ro.json",
    "balance_sheets2021q4_ro.json",
    "sector_details2021_ro.json",
    "io_table2021_ro.json",
];

/// Every check name of the spec's table.
pub fn all_checks() -> Vec<&'static str> {
    CHECKS.iter().chain(CHECKS_AFTER.iter()).copied().collect()
}

// --- builds ------------------------------------------------------------------

pub fn params(sample_scale: u32, rng_seed: u64) -> BuildParams {
    BuildParams {
        sample_scale,
        rng_seed,
    }
}

/// The committed fixtures, read once.
pub fn inputs() -> &'static Inputs {
    static I: OnceLock<Inputs> = OnceLock::new();
    I.get_or_init(|| read_inputs(&fixtures_dir()).expect("the fixtures read"))
}

/// The scenario at 1:1000 or 1:100, seed 42, or at 1:1000 with the other seed.
pub fn built(scale: u32, seed: u64) -> &'static Scenario {
    static B: [OnceLock<Scenario>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let slot = match (scale, seed) {
        (1000, SEED) => 0,
        (100, SEED) => 1,
        (1000, OTHER_SEED) => 2,
        _ => panic!("no cached build at 1:{scale}, seed {seed}"),
    };
    B[slot].get_or_init(|| build(inputs(), &params(scale, seed)).expect("the fixtures build"))
}

/// A folder under the tests' temporary directory, emptied: its parent exists,
/// the folder itself does not.
pub fn fresh_dir(name: &str) -> PathBuf {
    let parent = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("econ-scenario")
        .join(name);
    if parent.exists() {
        std::fs::remove_dir_all(&parent).expect("clean the test folder");
    }
    std::fs::create_dir_all(&parent).expect("create the test folder");
    parent.join("scenario")
}

/// The folder of [`built`], written once.
pub fn written(scale: u32, seed: u64) -> &'static Path {
    static W: [OnceLock<PathBuf>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let slot = match (scale, seed) {
        (1000, SEED) => 0,
        (100, SEED) => 1,
        (1000, OTHER_SEED) => 2,
        _ => panic!("no cached folder at 1:{scale}, seed {seed}"),
    };
    W[slot].get_or_init(|| {
        let dir = fresh_dir(&format!("written_{scale}_{seed}"));
        write(built(scale, seed), inputs(), &dir).expect("the scenario writes");
        dir
    })
}

// --- econ-cli ----------------------------------------------------------------

/// The `econ-cli` binary of this build profile, built once.
pub fn econ_cli() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // The test binary is target/<profile>/deps/acceptance-<hash>.
        let exe = std::env::current_exe().expect("the test binary");
        let profile = exe
            .parent()
            .and_then(Path::parent)
            .expect("target/<profile>")
            .to_path_buf();
        let mut cmd = Command::new(option_env!("CARGO").unwrap_or("cargo"));
        cmd.args(["build", "-q", "-p", "econ-cli"])
            .current_dir(repo_root());
        if profile.file_name().is_some_and(|n| n == "release") {
            cmd.arg("--release");
        }
        let status = cmd.status().expect("cargo runs");
        assert!(status.success(), "econ-cli builds");
        profile.join(format!("econ-cli{}", std::env::consts::EXE_SUFFIX))
    })
}

/// Run `econ-cli build-scenario` from the repository root (so the default
/// fixtures are the committed ones).
pub fn build_scenario_cli(args: &[&str]) -> ExitStatus {
    Command::new(econ_cli())
        .arg("build-scenario")
        .args(args)
        .current_dir(repo_root())
        .status()
        .expect("econ-cli runs")
}

// --- reading the folder ------------------------------------------------------

pub fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The checks of a `report.json`, by name, whether the file lists them as an
/// array of objects with a `name` or as an object keyed by name.
pub fn report_checks(report: &Value) -> Vec<(String, Value)> {
    match &report["checks"] {
        Value::Array(list) => list
            .iter()
            .map(|c| {
                let name = c["name"].as_str().expect("a check has a name").to_string();
                (name, c.clone())
            })
            .collect(),
        Value::Object(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        other => panic!("report.json has no checks: {other}"),
    }
}

/// An Arrow IPC file (one record batch).
pub fn read_table(path: &Path) -> RecordBatch {
    let file = File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut reader = FileReader::try_new(file, None).expect("an Arrow IPC file");
    reader
        .next()
        .expect("one record batch")
        .expect("a readable batch")
}

fn column<'a, T: 'static>(batch: &'a RecordBatch, name: &str) -> &'a T {
    batch
        .column_by_name(name)
        .unwrap_or_else(|| panic!("no column {name}"))
        .as_any()
        .downcast_ref::<T>()
        .unwrap_or_else(|| panic!("column {name} has another type"))
}

pub fn u8s(batch: &RecordBatch, name: &str) -> Vec<u8> {
    column::<UInt8Array>(batch, name).values().to_vec()
}

pub fn u16s(batch: &RecordBatch, name: &str) -> Vec<u16> {
    column::<UInt16Array>(batch, name).values().to_vec()
}

pub fn u32s(batch: &RecordBatch, name: &str) -> Vec<u32> {
    column::<UInt32Array>(batch, name).values().to_vec()
}

pub fn u64s(batch: &RecordBatch, name: &str) -> Vec<u64> {
    column::<UInt64Array>(batch, name).values().to_vec()
}

pub fn i64s(batch: &RecordBatch, name: &str) -> Vec<i64> {
    column::<Int64Array>(batch, name).values().to_vec()
}

pub fn strings(batch: &RecordBatch, name: &str) -> Vec<String> {
    let a = column::<StringArray>(batch, name);
    (0..a.len()).map(|i| a.value(i).to_string()).collect()
}

/// Sum of a household column times `hh_weight`, in bani.
pub fn weighted(hh_weight: &[u32], column: &[u64]) -> i64 {
    assert_eq!(hh_weight.len(), column.len());
    let total: u128 = hh_weight
        .iter()
        .zip(column)
        .map(|(&w, &v)| u128::from(w) * u128::from(v))
        .sum();
    i64::try_from(total).expect("fits i64 bani")
}

// --- the Python reference ----------------------------------------------------

/// `python/pipeline/build_scenario.py` at 1:1000, seed 42: `{scenario, report}`.
pub fn builder_reference() -> &'static Value {
    static V: OnceLock<Value> = OnceLock::new();
    V.get_or_init(|| {
        python_json(
            "python/pipeline/build_scenario.py",
            &[
                "--sample-scale".into(),
                "1000".into(),
                "--seed".into(),
                SEED.to_string(),
            ],
        )
    })
}

// --- SHA-256 (FIPS 180-4), to check the hashes of scenario.json ----------------

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 of `data` as 64 lowercase hex digits.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for block in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// The helper itself, on the standard test vectors.
#[test]
fn sha256_known_answers() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f2b00015"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff5412ce46f2b7bb84f8bb"
    );
}
