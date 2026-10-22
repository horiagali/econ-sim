//! Acceptance tests for AC-SCB-01 to AC-SCB-10 (spec: game/scenario-builder).
//! Written in a test-authoring session from the spec and its API sketch,
//! before the code existed (ADR-0014). `[cli]` tests run `econ-cli` on the
//! committed fixtures.

use std::collections::BTreeMap;
use std::path::Path;

use econ_ledger::{FlowCode, Instrument, Ledger, Sector};
use econ_scenario::{
    BuildError, OpeningBalances, build, opening_balances, read_inputs, reconcile, run_with_hook,
    write,
};
use econ_types::Bani;
use serde_json::Value;

use super::support::{
    bal, fixture_json, fixtures_dir, hex, opening_golden, repo_root, row_sum, statistics,
};
use super::support_builder::{
    FILES, INPUT_FILES, KNOWN_GAP, OTHER_SEED, SEED, all_checks, build_scenario_cli,
    builder_reference, built, fresh_dir, i64s, inputs, params, read_json, read_table,
    report_checks, sha256_hex, strings, u8s, u16s, u32s, u64s, weighted, written,
};

/// Each stage's name in `scenario.json` and its golden file.
const STAGE_GOLDENS: [(&str, &str); 9] = [
    ("population", "popgen_ro_census2021.hashes"),
    ("attributes", "popgen_ro_census2021_attributes.hashes"),
    ("jobs", "popgen_ro_census2021_jobs.hashes"),
    ("housing", "popgen_ro_census2021_housing.hashes"),
    ("wages", "popgen_ro_census2021_wages.hashes"),
    ("pensions", "popgen_ro_census2021_pensions.hashes"),
    ("wealth", "popgen_ro_census2021_wealth.hashes"),
    ("incomes", "popgen_ro_census2021_incomes.hashes"),
    ("employers", "popgen_ro_census2021_employers.hashes"),
];

/// The stock columns of `firm_units.arrow`, named as in `firm_balance_sheets2021_ro.json`.
const STOCKS: [&str; 12] = [
    "fixed_assets",
    "currency",
    "deposits",
    "debt_securities_held",
    "loans_granted",
    "equity_held",
    "insurance",
    "other_receivable",
    "debt_securities_issued",
    "loans",
    "equity_issued",
    "other_payable",
];

/// The hash of a stage golden file at one scale.
fn stage_golden(file: &str, scale: u32) -> String {
    let path = repo_root().join("tests/golden").join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("golden file {}: {e}", path.display()));
    text.lines()
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            Some((
                parts.next()?.parse::<u32>().ok()?,
                parts.next()?.to_string(),
            ))
        })
        .find(|&(s, _)| s == scale)
        .unwrap_or_else(|| panic!("{file} has no hash for 1:{scale}"))
        .1
}

/// Every file under `dir`, by its path relative to `dir` with `/`.
fn files_under(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).expect("a readable folder") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .expect("under the root")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(rel, std::fs::read(&path).expect("a readable file"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// What `econ-cli build-scenario` wrote at a scale passes AC-SCB-01.
fn assert_cli_scenario(scale: u32) {
    let out = fresh_dir(&format!("cli_{scale}"));
    let status = build_scenario_cli(&[
        "--sample-scale",
        &scale.to_string(),
        "--out",
        out.to_str().expect("a UTF-8 path"),
    ]);
    assert_eq!(status.code(), Some(0), "exit status at 1:{scale}");
    for f in FILES {
        assert!(out.join(f).is_file(), "1:{scale}: {f} written");
    }
    let report = read_json(&out.join("report.json"));
    let checks = report_checks(&report);
    for name in all_checks() {
        let (_, c) = checks
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("1:{scale}: report.json has no check {name}"));
        if name == KNOWN_GAP {
            assert!(
                c["known_gap"].as_str().is_some_and(|s| !s.is_empty()),
                "1:{scale}: {name} is a known gap with its reason"
            );
        } else {
            assert_eq!(c["ok"], Value::Bool(true), "1:{scale}: {name} passes: {c}");
        }
    }
    for (name, c) in &checks {
        assert!(
            c["ok"] == Value::Bool(true) || c["known_gap"].is_string(),
            "1:{scale}: {name} fails without being a known gap"
        );
    }
    let sc = read_json(&out.join("scenario.json"));
    assert_eq!(sc["format"], Value::from(1));
    assert_eq!(sc["start_date"], Value::from("2021-12-01"));
    assert_eq!(sc["sample_scale"], Value::from(scale));
    assert_eq!(sc["rng_seed"], Value::from(SEED), "the default seed");
    // Restated 2026-10-22 (owner's amendment, M4): `law` names the law's
    // two files, which the folder holds (FILES checks they exist).
    assert_eq!(
        sc["law"],
        serde_json::json!({"law": "law.json", "world": "world.json"}),
        "scenario.json names law.json and world.json"
    );
    assert_eq!(sc["consistent"], Value::Bool(true));
}

/// AC-SCB-01 [cli] `econ-cli build-scenario --sample-scale 1000 --out DIR` exits 0 and writes `scenario.json`, `tables/households.arrow`, `tables/persons.arrow`, `tables/firm_units.arrow`, `balances.arrow`, `report.json`, `law.json` and `world.json`. `report.json` has every check of the table, and all pass except the known gap `income_distribution`.
#[test]
fn ac_scb_01() {
    assert_cli_scenario(1000);
}

/// AC-SCB-02 [cli] The same at 1:100. At 1:10 likewise, marked ignored for CI time and run by `just build-scenario 10` (the test below).
#[test]
fn ac_scb_02() {
    assert_cli_scenario(100);
}

/// The 1:10 half of the criterion above, for `cargo test -- --ignored`.
#[test]
#[ignore = "minutes at 1:10; run by `just build-scenario 10` and with --ignored"]
fn ac_scb_02_at_1_in_10() {
    assert_cli_scenario(10);
}

/// AC-SCB-03 [unit] The stage hashes in `scenario.json` equal the golden files of the stages (`tests/golden/popgen_ro_census2021*.hashes`) at 1:1000 and 1:100, and the opening hash (after rule 8) equals the reference's at each scale.
#[test]
fn ac_scb_03() {
    for scale in [1000, 100] {
        let sc = read_json(&written(scale, SEED).join("scenario.json"));
        for (name, file) in STAGE_GOLDENS {
            assert_eq!(
                sc["stage_hashes"][name].as_str(),
                Some(stage_golden(file, scale).as_str()),
                "{name} at 1:{scale}"
            );
        }
        let opening = opening_golden(&scale.to_string());
        assert_eq!(
            sc["opening_hash"].as_str(),
            Some(opening.as_str()),
            "1:{scale}"
        );
        assert_eq!(hex(built(scale, SEED).opening.state_hash()), opening);
        // The library names the same hashes.
        let hashes = built(scale, SEED).stage_hashes();
        let names: Vec<&str> = hashes.iter().map(|&(n, _)| n).collect();
        assert_eq!(names, STAGE_GOLDENS.map(|(n, _)| n));
        for (name, hash) in hashes {
            assert_eq!(Value::from(hex(hash)), sc["stage_hashes"][name], "{name}");
        }
    }
}

/// AC-SCB-04 [unit] `balances.arrow` read back gives a ledger whose every instrument row sums to zero and whose balances equal the opening balance sheet's. Every balance was posted with flow code `opening`.
#[test]
fn ac_scb_04() {
    let scenario = built(1000, SEED);
    let table = read_table(&written(1000, SEED).join("balances.arrow"));
    let (instrument, sector, balance) = (
        u8s(&table, "instrument"),
        u8s(&table, "sector"),
        i64s(&table, "balance"),
    );
    let n_i = Instrument::ALL.len();
    let n_s = Sector::ALL.len();
    assert_eq!(balance.len(), n_i * n_s, "one row per (instrument, sector)");
    let mut read_back = OpeningBalances {
        balances: vec![[Bani::ZERO; 8]; n_i],
        left_out: scenario.opening.left_out.clone(),
        gaps: scenario.opening.gaps.clone(),
    };
    for k in 0..balance.len() {
        // Matrix order: instrument by instrument, sector by sector.
        assert_eq!(usize::from(instrument[k]), k / n_s, "row {k}: instrument");
        assert_eq!(usize::from(sector[k]), k % n_s, "row {k}: sector");
        read_back.balances[k / n_s][k % n_s] = Bani(balance[k]);
    }
    for i in Instrument::ALL {
        assert_eq!(row_sum(&read_back, i), Bani::ZERO, "row {i:?}");
    }
    assert_eq!(read_back.balances, scenario.opening.balances);

    // The opening balance sheet: rules 1 to 7 on the statistics, then rule 8
    // with the weighted household totals of the population.
    let hh = &scenario.population.households.hh_weight;
    let cash = Bani(weighted(hh, &scenario.stages.wealth.hh_cash));
    let deposits = Bani(weighted(hh, &scenario.stages.wealth.hh_deposits));
    let want = reconcile(
        &opening_balances(statistics()).expect("the fixture maps"),
        cash,
        deposits,
    )
    .expect("within 10 millionths");
    assert_eq!(read_back.balances, want.balances);

    let mut ledger = Ledger::new();
    read_back.post(&mut ledger).expect("the balances post");
    assert_eq!(ledger.check_invariants(), Ok(()));
    for i in Instrument::ALL {
        for s in Sector::ALL {
            assert_eq!(ledger.balance(s, i), bal(&want, i, s), "{i:?} of {s:?}");
            assert_eq!(
                scenario.ledger.balance(s, i),
                bal(&want, i, s),
                "{i:?} of {s:?}"
            );
        }
    }
    assert_eq!(scenario.ledger.check_invariants(), Ok(()));
    let postings = scenario.ledger.debug_postings();
    assert!(!postings.is_empty(), "the opening balances were posted");
    for p in postings {
        assert_eq!(p.leg.code, FlowCode::OPENING, "{p:?}");
    }
}

/// AC-SCB-05 [unit] `tables/firm_units.arrow` has one row per unit of `firm_units2021_ro.json`, in its order. Its stock columns sum to the `totals` of `firm_balance_sheets2021_ro.json`, and its `employees` sum to the weighted employed persons of the population.
#[test]
fn ac_scb_05() {
    let dir = written(1000, SEED);
    let table = read_table(&dir.join("tables/firm_units.arrow"));
    let firms = fixture_json("firm_units2021_ro.json");
    let sheets = fixture_json("firm_balance_sheets2021_ro.json");
    let units = firms["units"].as_array().expect("units");
    let ids: Vec<String> = units
        .iter()
        .map(|u| u["id"].as_str().expect("an id").to_string())
        .collect();
    assert_eq!(
        strings(&table, "unit_id"),
        ids,
        "one row per unit, in file order"
    );

    // Identity, size and flows are the file's.
    let industries: Vec<&str> = firms["industries"]
        .as_array()
        .expect("industries")
        .iter()
        .map(|i| i["id"].as_str().expect("an id"))
        .collect();
    let code = |v: &Value, names: &[&str]| -> u8 {
        let s = v.as_str().expect("a label");
        u8::try_from(names.iter().position(|&n| n == s).expect("a known label")).expect("small")
    };
    let (industry, size, kind, controller) = (
        u8s(&table, "industry"),
        u8s(&table, "size"),
        u8s(&table, "kind"),
        u8s(&table, "controller"),
    );
    let (firm_count, own_state, own_foreign) = (
        u32s(&table, "firm_count"),
        u32s(&table, "own_state"),
        u32s(&table, "own_foreign"),
    );
    for (f, u) in units.iter().enumerate() {
        assert_eq!(industry[f], code(&u["industry"], &industries), "{}", ids[f]);
        assert_eq!(
            size[f],
            code(&u["size"], &["micro", "small", "large", "public"])
        );
        assert_eq!(kind[f], code(&u["kind"], &["cohort", "public"]));
        assert_eq!(
            controller[f],
            code(&u["controller"], &["domestic", "foreign", "state"])
        );
        assert_eq!(
            u64::from(firm_count[f]),
            u["firm_count"].as_u64().expect("a count")
        );
        assert_eq!(
            u64::from(own_state[f]),
            u["ownership"]["state"].as_u64().expect("ppm")
        );
        assert_eq!(
            u64::from(own_foreign[f]),
            u["ownership"]["foreign"].as_u64().expect("ppm")
        );
    }
    let totals = &firms["totals"];
    assert_eq!(
        u64s(&table, "persons_employed").iter().sum::<u64>(),
        totals["persons_employed"].as_u64().expect("persons")
    );
    for name in ["output", "value_added", "wages"] {
        assert_eq!(
            u64s(&table, name).iter().sum::<u64>(),
            totals[name].as_u64().expect("lei") * 100,
            "{name} in bani"
        );
    }

    // Stocks: the balance sheets' totals; zero for the units without a sheet.
    let with_sheet: Vec<&str> = sheets["units"]
        .as_array()
        .expect("units")
        .iter()
        .map(|u| u["id"].as_str().expect("an id"))
        .collect();
    for name in STOCKS {
        let column = u64s(&table, name);
        assert_eq!(
            column.iter().sum::<u64>(),
            sheets["totals"][name].as_u64().expect("lei") * 100,
            "{name} in bani"
        );
        for (f, id) in ids.iter().enumerate() {
            if !with_sheet.contains(&id.as_str()) {
                assert_eq!(column[f], 0, "{name} of {id}, which has no balance sheet");
            }
        }
    }
    assert_eq!(
        ids.len() - with_sheet.len(),
        17,
        "units without a balance sheet"
    );

    // Employees: weighted persons whose employer is the unit.
    let households = read_table(&dir.join("tables/households.arrow"));
    let persons = read_table(&dir.join("tables/persons.arrow"));
    let hh_weight = u32s(&households, "hh_weight");
    let household_id = u32s(&persons, "household_id");
    let status = u8s(&persons, "employment_status");
    let employer = u16s(&persons, "employer");
    let mut want = vec![0u64; ids.len()];
    let mut employed = 0u64;
    for p in 0..household_id.len() {
        let w = u64::from(hh_weight[household_id[p] as usize]);
        if status[p] != 0 {
            employed += w;
        }
        if employer[p] != u16::MAX {
            want[usize::from(employer[p])] += w;
        }
    }
    let employees = u64s(&table, "employees");
    assert_eq!(employees, want, "employees by unit");
    assert_eq!(employees.iter().sum::<u64>(), employed);
    assert_eq!(
        built(1000, SEED).firm_units.employees,
        want,
        "the library's table"
    );
}

/// AC-SCB-06 [unit] Weighted household cash and deposits of the tables equal the ledger's household `cash` and `deposits` exactly. The firm units' currency and deposits equal the ledger's firm `cash` and `deposits` exactly.
#[test]
fn ac_scb_06() {
    for scale in [1000, 100] {
        let dir = written(scale, SEED);
        let ledger = &built(scale, SEED).ledger;
        let households = read_table(&dir.join("tables/households.arrow"));
        let w = u32s(&households, "hh_weight");
        assert_eq!(
            Bani(weighted(&w, &u64s(&households, "hh_cash"))),
            ledger.balance(Sector::Households, Instrument::Cash),
            "1:{scale}"
        );
        assert_eq!(
            Bani(weighted(&w, &u64s(&households, "hh_deposits"))),
            ledger.balance(Sector::Households, Instrument::Deposits),
            "1:{scale}"
        );
        let units = read_table(&dir.join("tables/firm_units.arrow"));
        let sum =
            |name: &str| Bani(i64::try_from(u64s(&units, name).iter().sum::<u64>()).expect("fits"));
        assert_eq!(
            sum("currency"),
            ledger.balance(Sector::Firms, Instrument::Cash),
            "1:{scale}"
        );
        assert_eq!(
            sum("deposits"),
            ledger.balance(Sector::Firms, Instrument::Deposits),
            "1:{scale}"
        );
    }
}

/// AC-SCB-07 [unit] Two builds with the same inputs, scale and seed produce byte-identical folders. A different seed changes the stage hashes and, through rule 8, only the households', the central bank's and the banks' balances of `cash` and `deposits`.
#[test]
fn ac_scb_07() {
    let first = files_under(written(1000, SEED));
    let again_inputs = read_inputs(&fixtures_dir()).expect("the fixtures read");
    let again = build(&again_inputs, &params(1000, SEED)).expect("the fixtures build");
    let dir = fresh_dir("again_1000");
    write(&again, &again_inputs, &dir).expect("the scenario writes");
    let second = files_under(&dir);
    assert_eq!(
        first.keys().collect::<Vec<_>>(),
        second.keys().collect::<Vec<_>>(),
        "the same files"
    );
    for (path, bytes) in &first {
        assert!(second[path] == *bytes, "{path} differs between two builds");
    }

    let (a, b) = (built(1000, SEED), built(1000, OTHER_SEED));
    for ((name, ha), (_, hb)) in a.stage_hashes().into_iter().zip(b.stage_hashes()) {
        assert_ne!(ha, hb, "the {name} hash with another seed");
    }
    let mut changed = false;
    for i in Instrument::ALL {
        for s in Sector::ALL {
            let may_change = matches!(i, Instrument::Cash | Instrument::Deposits)
                && matches!(s, Sector::Households | Sector::CentralBank | Sector::Banks);
            if may_change {
                changed |= bal(&a.opening, i, s) != bal(&b.opening, i, s);
            } else {
                assert_eq!(
                    bal(&a.opening, i, s),
                    bal(&b.opening, i, s),
                    "{i:?} of {s:?}"
                );
            }
        }
    }
    assert!(changed, "another seed gives other household totals");
}

/// AC-SCB-08 [unit] If the opening balances are corrupted so that a row does not sum to zero (a test hook), the build exits 3 and writes nothing. A malformed input file gives a named error and writes nothing.
#[test]
fn ac_scb_08() {
    let corrupt = |o: &mut OpeningBalances| o.balances[0][0] += Bani(1);
    let p = params(1000, SEED);

    // A new folder: nothing appears, not even a temporary one next to it.
    let out = fresh_dir("corrupt_new");
    let parent = out.parent().expect("a parent").to_path_buf();
    let r = run_with_hook(&fixtures_dir(), &p, &out, &corrupt);
    let err = r.expect_err("a corrupted opening ledger fails");
    assert!(matches!(err, BuildError::Invariants(_)), "{err:?}");
    assert_eq!(err.exit_status(), 3);
    assert!(!out.exists(), "nothing written");
    assert_eq!(
        std::fs::read_dir(&parent).expect("the parent").count(),
        0,
        "no partial folder"
    );

    // An existing folder is left as it was.
    let out = fresh_dir("corrupt_existing");
    std::fs::create_dir_all(&out).expect("the folder");
    std::fs::write(out.join("keep.txt"), b"before").expect("a file");
    let err = run_with_hook(&fixtures_dir(), &p, &out, &corrupt).expect_err("fails");
    assert_eq!(err.exit_status(), 3);
    let left = files_under(&out);
    assert_eq!(left.len(), 1, "only the file that was there");
    assert_eq!(left["keep.txt"], b"before");

    // Malformed and missing input files.
    for (name, how) in [
        ("household_wealth2021_ro.json", "malformed"),
        ("balance_sheets2021q4_ro.json", "malformed"),
        ("sector_details2021_ro.json", "missing"),
    ] {
        let fixtures = fresh_dir(&format!("bad_input_{how}_{name}"));
        std::fs::create_dir_all(&fixtures).expect("a fixtures copy");
        for entry in std::fs::read_dir(fixtures_dir()).expect("the fixtures") {
            let path = entry.expect("an entry").path();
            if path.is_file() {
                std::fs::copy(&path, fixtures.join(path.file_name().expect("a name")))
                    .expect("copy a fixture");
            }
        }
        if how == "missing" {
            std::fs::remove_file(fixtures.join(name)).expect("remove");
        } else {
            std::fs::write(fixtures.join(name), b"{ \"not\": [a fixture").expect("break");
        }
        match read_inputs(&fixtures) {
            Err(BuildError::Input { file, .. }) => {
                assert!(file.contains(name), "{how} {name}: the error names {file}");
            }
            other => panic!("{how} {name}: {other:?}"),
        }
        let out = fixtures.parent().expect("a parent").join("out");
        assert!(run_with_hook(&fixtures, &p, &out, &|_| {}).is_err());
        assert!(!out.exists(), "{how} {name}: nothing written");
    }
}

/// Every figure of the reference is in the Rust report: integers (amounts in
/// whole lei, counts) exactly, ratios and shares within 1e-4, notes not compared.
fn assert_same_figures(path: &str, rust: &Value, reference: &Value) {
    match reference {
        Value::Object(map) => {
            for (k, v) in map {
                if matches!(k.as_str(), "ok" | "known_gap" | "note" | "name") {
                    continue;
                }
                let r = rust
                    .get(k)
                    .unwrap_or_else(|| panic!("{path}: the Rust report has no {k}"));
                assert_same_figures(&format!("{path}.{k}"), r, v);
            }
        }
        Value::Array(items) => {
            let r = rust
                .as_array()
                .unwrap_or_else(|| panic!("{path}: not a list"));
            assert_eq!(r.len(), items.len(), "{path}: length");
            for (i, (a, b)) in r.iter().zip(items).enumerate() {
                assert_same_figures(&format!("{path}[{i}]"), a, b);
            }
        }
        Value::Number(n) if n.is_i64() || n.is_u64() => {
            assert_eq!(rust.as_i64(), n.as_i64(), "{path}: to the leu");
        }
        Value::Number(n) => {
            let (a, b) = (
                rust.as_f64()
                    .unwrap_or_else(|| panic!("{path}: not a number")),
                n.as_f64().expect("a number"),
            );
            assert!(
                (a - b).abs() <= 1e-4 * b.abs().max(1.0),
                "{path}: {a} vs {b}"
            );
        }
        Value::String(_) => {}
        other => assert_eq!(rust, other, "{path}"),
    }
}

/// AC-SCB-09 [diff] The report's figures equal those of the reference `python/pipeline/build_scenario.py` (extended with the four new checks) at 1:1000: the same pass or fail for every check, and amounts equal to the leu.
#[test]
fn ac_scb_09() {
    let reference = builder_reference();
    let ref_checks = reference["report"]["checks"]
        .as_object()
        .expect("the reference's checks");
    let report = &built(1000, SEED).report;
    for name in all_checks() {
        assert!(ref_checks.contains_key(name), "the reference has {name}");
        assert!(report.check(name).is_some(), "the Rust report has {name}");
    }
    let mut compared = 0;
    for (name, r) in ref_checks {
        let Some(c) = report.check(name) else {
            continue;
        };
        assert_eq!(Value::Bool(c.ok), r["ok"], "{name}: pass or fail");
        assert_eq!(
            c.known_gap.is_some(),
            r.get("known_gap").is_some(),
            "{name}: known gap"
        );
        assert_same_figures(name, &c.facts, r);
        compared += 1;
    }
    assert!(compared >= all_checks().len());
    assert_eq!(
        report.consistent(),
        reference["report"]["ok"] == Value::Bool(true)
    );
    // The scenario's hashes are the reference's too.
    let scenario = &reference["scenario"];
    assert_eq!(
        Value::from(hex(built(1000, SEED).opening.state_hash())),
        scenario["opening_hash"]
    );
    for (name, hash) in built(1000, SEED).stage_hashes() {
        assert_eq!(
            Value::from(hex(hash)),
            scenario["stage_hashes"][name],
            "{name}"
        );
    }
}

/// AC-SCB-10 [unit] `scenario.json` lists the SHA-256 of every input file and of every file it wrote, and the hashes match the files.
#[test]
fn ac_scb_10() {
    let dir = written(1000, SEED);
    let sc = read_json(&dir.join("scenario.json"));
    let listed = sc["inputs"].as_object().expect("inputs");
    for name in INPUT_FILES {
        assert!(listed.contains_key(name), "inputs lists {name}");
    }
    for (name, hash) in listed {
        let bytes = std::fs::read(fixtures_dir().join(name))
            .unwrap_or_else(|e| panic!("listed input {name}: {e}"));
        assert_eq!(hash.as_str(), Some(sha256_hex(&bytes).as_str()), "{name}");
    }
    let mut on_disk = files_under(dir);
    on_disk.remove("scenario.json");
    let files = sc["files"].as_object().expect("files");
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        on_disk.keys().collect::<Vec<_>>(),
        "every other file of the folder, and only those"
    );
    for (path, bytes) in &on_disk {
        assert_eq!(
            files[path].as_str(),
            Some(sha256_hex(bytes).as_str()),
            "{path}"
        );
    }
    for f in FILES.iter().skip(1) {
        assert!(files.contains_key(*f), "files lists {f}");
    }
    // `inputs` are the files the build read: the same as those of the library.
    let read: Vec<&str> = inputs().files.iter().map(|(n, _)| n.as_str()).collect();
    for name in listed.keys() {
        assert!(read.contains(&name.as_str()), "{name} was read");
    }
}
