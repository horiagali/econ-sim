//! Acceptance tests of the lever system's `[unit]` criteria (spec
//! game/lever-system): the registry and its checks (AC-LEV-01), the starting
//! law of the scenario (AC-LEV-02), validation and its messages (AC-LEV-03),
//! effective dates (AC-LEV-04) and warnings (AC-LEV-10). Written from the
//! spec against the API skeleton `econ_core::levers`, before the code
//! (ADR-0014); behind the feature `levers`.
//!
//! The spec names `crates/econ-core/tests/generated_lever_list.rs` as the
//! list the coverage tests iterate; until the generator writes it, these
//! tests iterate the committed registry (`levers.toml`) and `LeverId::ALL`.

use std::collections::BTreeMap;
use std::process::Command as Process;

use econ_core::levers::{
    Kind, LawInForce, LeverId, LeverValue, Order, PolicyEvent, Registry, RegistryError, RejectCode,
    Rejection, Shape, Status, Warning, WarningCode, earliest_effective, format_value, validate,
};
use econ_io::commands::read_commands;
use econ_types::{Bani, Date};
use serde_json::Value;

use super::support::{fresh_dir, repo_root, scenario_dir};
use super::support_lev::{
    date, give, is_active, lever_world, python, registry, registry_path, registry_text,
    scenario_law, set, set_on, start_date,
};

/// The key tables a scenario holds (spec, "The registry").
const SCENARIO_TABLES: [&str; 6] = [
    "industries",
    "budget_lines",
    "public_units",
    "counties",
    "blocs",
    "fuels",
];

/// Every doc id of the docs index (`docs/INDEX.md`).
fn spec_ids() -> Vec<String> {
    let path = repo_root().join("docs/INDEX.md");
    let text = std::fs::read_to_string(&path).expect("docs/INDEX.md");
    text.lines()
        .filter(|l| l.starts_with("| ["))
        .filter_map(|l| l.split('|').nth(2))
        .map(|c| c.trim().trim_matches('`').to_string())
        .filter(|id| !id.is_empty())
        .collect()
}

/// Every glossary `code_name`.
fn glossary_names() -> Vec<&'static str> {
    econ_types::glossary::VARIABLES
        .iter()
        .map(|v| v.code_name)
        .collect()
}

/// Run the registry checks with the docs index, the glossary and the
/// scenario's key tables.
fn run_check(r: &Registry) -> Result<(), Vec<RegistryError>> {
    let specs = spec_ids();
    let specs: Vec<&str> = specs.iter().map(String::as_str).collect();
    r.check(&specs, &glossary_names(), &SCENARIO_TABLES)
}

/// The registry text with one field of one entry replaced.
fn edit_entry(text: &str, id: &str, field: &str, value: &str) -> String {
    let marker = format!("id = \"{id}\"");
    let mut found = false;
    let chunks: Vec<String> = text
        .split("[[lever]]")
        .map(|chunk| {
            if !chunk.contains(&marker) {
                return chunk.to_string();
            }
            found = true;
            chunk
                .lines()
                .map(|l| {
                    if l.starts_with(&format!("{field} =")) {
                        format!("{field} = {value}")
                    } else {
                        l.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        })
        .collect();
    assert!(found, "the registry has an entry {id}");
    let out = chunks.join("[[lever]]");
    assert_ne!(out, text, "{id}: field {field} replaced");
    out
}

/// The faults name the entry.
fn assert_names(errors: &[RegistryError], id: &str, what: &str) {
    assert!(
        errors.iter().any(|e| e.lever.as_deref() == Some(id)),
        "{what}: no error names {id}: {errors:?}"
    );
}

/// Run `scripts/codegen_levers.py` with these arguments; whether it succeeded.
fn codegen_levers(args: &[&str]) -> (bool, String) {
    let out = Process::new(python())
        .arg(repo_root().join("scripts/codegen_levers.py"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("Python runs");
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

/// **AC-LEV-01** `[unit]` The registry checks pass on the committed
/// `levers.toml`, and each kind of fault (duplicate id, unknown shape, `spec`
/// not in the index, `glossary` not in the glossary, unknown `reader`,
/// missing key table) fails with the entry named; `just codegen-check` fails
/// when the generated files are stale.
///
/// Readings: "the index" is `docs/INDEX.md`; "the glossary" is its generated
/// list `econ_types::glossary::VARIABLES`; the key tables a scenario holds
/// are the spec's six (industries, budget lines, public units, counties,
/// blocs, fuels). An unknown shape or reader cannot be held by a
/// `LeverMeta`, so it fails `Registry::parse`; the other faults fail
/// `Registry::check`. "The entry named" means the error's `lever` is the
/// faulty entry's id. `just codegen-check` runs
/// `scripts/codegen_levers.py --check` beside the glossary's generator;
/// "stale" is shown by running the check against a copy of the registry
/// with one label changed, passed with `--registry PATH` (the flag is this
/// test's choice: the spec does not say how the check is pointed at another
/// file).
#[test]
fn ac_lev_01() {
    // The committed registry parses, in the order of `LeverId::ALL`, and
    // passes its checks.
    let text = registry_text();
    let reg = Registry::parse(&text).expect("the committed registry parses");
    assert_eq!(&reg, registry(), "Registry::load reads the same file");
    assert_eq!(
        reg.levers.len(),
        LeverId::ALL.len(),
        "one entry per lever id"
    );
    for (meta, id) in reg.levers.iter().zip(LeverId::ALL) {
        assert_eq!(meta.id, id, "registry order");
        assert_eq!(LeverId::from_id(id.as_str()), Some(id));
        assert_eq!(reg.find(id.as_str()), Some(meta));
        assert_eq!(reg.get(id), meta);
    }
    assert_eq!(reg.get(LeverId::TaxVatRateStandard).shape, Shape::Rate);
    assert_eq!(reg.get(LeverId::TaxVatRateStandard).range, Some((0, 4000)));
    assert_eq!(LeverId::from_id("tax.vat.no_such_rate"), None);
    if let Err(e) = run_check(&reg) {
        panic!("the committed registry fails its checks: {e:?}");
    }

    // Unknown shape and unknown reader: the parse fails, naming the entry.
    let vat = "tax.vat.rate_standard";
    for (field, bad) in [("shape", "\"percentage\""), ("reader", "\"Pricing\"")] {
        let t = edit_entry(&text, vat, field, bad);
        let e = Registry::parse(&t).expect_err(&format!("an unknown {field} is refused"));
        assert_eq!(e.lever.as_deref(), Some(vat), "unknown {field}: {e:?}");
    }

    // Duplicate id.
    let mut dup = reg.clone();
    let i = dup
        .levers
        .iter()
        .position(|m| m.id == LeverId::TaxVatRateStandard)
        .expect("VAT standard rate");
    dup.levers.push(dup.levers[i].clone());
    assert_names(
        &run_check(&dup).expect_err("a duplicate id fails"),
        vat,
        "duplicate id",
    );

    // `spec` not in the index.
    let mut bad = reg.clone();
    bad.levers[i].spec = "economy/no-such-spec".to_string();
    assert_names(
        &run_check(&bad).expect_err("an unknown spec fails"),
        vat,
        "spec not in the index",
    );

    // `glossary` not in the glossary.
    let mut bad = reg.clone();
    bad.levers[i].glossary = "no_such_variable".to_string();
    assert_names(
        &run_check(&bad).expect_err("an unknown glossary name fails"),
        vat,
        "glossary not in the glossary",
    );

    // A key table that exists neither in the file nor in a scenario.
    let mut bad = reg.clone();
    let k = bad
        .levers
        .iter()
        .position(|m| m.id == LeverId::TaxIncomeDeductionBase)
        .expect("deduction base");
    bad.levers[k].key = Some("no_such_table".to_string());
    assert_names(
        &run_check(&bad).expect_err("a missing key table fails"),
        "tax.income.deduction_base",
        "missing key table",
    );

    // `just codegen-check`: up to date on the committed files, failing on a
    // registry the generated files do not match.
    let (ok, out) = codegen_levers(&["--check"]);
    assert!(ok, "codegen-check passes on the committed registry: {out}");
    let stale = edit_entry(
        &text,
        vat,
        "label",
        "\"lever.tax.vat.rate_standard_renamed\"",
    );
    let dir = fresh_dir("lev01_codegen");
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("levers.toml");
    std::fs::write(&path, stale).expect("the edited registry writes");
    let (ok, out) = codegen_levers(&["--check", "--registry", path.to_str().expect("UTF-8")]);
    assert!(
        !ok,
        "codegen-check fails when the generated files are stale: {out}"
    );
    assert!(registry_path().is_file());
}

/// **AC-LEV-02** `[unit]` Every `active` lever is set exactly once in the
/// Romania scenario's `law.json` or `world.json`; a scenario missing one
/// fails to load, naming it.
///
/// Readings: "set exactly once" is per key for a keyed lever (one line for
/// each of its keys); lines are those of the files as JSON lines, keys as
/// text ids, with `given` and `effective` the start date and no `seq`
/// (spec, "Logging and deterministic replay"). "Fails to load" is
/// `LawInForce::from_lines` refusing the scenario's lines less those of one
/// lever (or one key of a keyed lever); "naming it" means the rejection's
/// `lever` is that lever and its message holds its dotted id or its label.
#[test]
fn ac_lev_02() {
    let dir = scenario_dir(1000);
    let law = scenario_law(1000);
    let mut count: BTreeMap<(String, Option<String>), usize> = BTreeMap::new();
    for file in ["law.json", "world.json"] {
        let path = dir.join(file);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("the scenario has {file}: {e}"));
        for (n, line) in text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
        {
            let v: Value =
                serde_json::from_str(line).unwrap_or_else(|e| panic!("{file} line {}: {e}", n + 1));
            let lever = v["lever"]
                .as_str()
                .unwrap_or_else(|| panic!("{file} line {}: a lever", n + 1))
                .to_string();
            assert_eq!(
                v["given"],
                Value::from("2021-12-01"),
                "{file} line {}",
                n + 1
            );
            assert_eq!(
                v["effective"],
                Value::from("2021-12-01"),
                "{file} line {}",
                n + 1
            );
            assert!(v.get("seq").is_none(), "{file} line {}: no seq", n + 1);
            let meta = registry()
                .find(&lever)
                .unwrap_or_else(|| panic!("{file} line {}: {lever} is in the registry", n + 1));
            assert_eq!(
                meta.kind == Kind::World,
                file == "world.json",
                "{lever}: world settings in world.json, law in law.json"
            );
            let key = v.get("key").and_then(Value::as_str).map(str::to_string);
            *count.entry((lever, key)).or_insert(0) += 1;
        }
    }
    for meta in law.registry.levers.iter().filter(|m| is_active(m)) {
        let id = meta.id.as_str();
        if meta.key.is_some() {
            assert!(!meta.keys.is_empty(), "{id}: its keys from the scenario");
            for k in &meta.keys {
                let n = count.get(&(id.to_string(), Some(k.clone()))).copied();
                assert_eq!(n, Some(1), "{id}[{k}] is set exactly once");
            }
        } else {
            let n = count.get(&(id.to_string(), None)).copied();
            assert_eq!(n, Some(1), "{id} is set exactly once");
        }
    }

    // The whole law loads; less one lever, or one key, it does not.
    let full = LawInForce::from_lines(&law.registry, &law.lines).expect("the scenario's law loads");
    assert_eq!(full, law.law);
    let cars = u32::try_from(
        law.registry
            .get(LeverId::TaxVatCategory)
            .keys
            .iter()
            .position(|k| k == "cars")
            .expect("cars is a key of the VAT category"),
    )
    .expect("fits");
    for (lever, key) in [
        (LeverId::TaxVatRateStandard, None),
        (LeverId::LabourMinimumWage, None),
        (LeverId::TaxVatCategory, Some(cars)),
    ] {
        let lines: Vec<_> = law
            .lines
            .iter()
            .filter(|l| {
                !matches!(&l.order, Order::Set(c) if c.lever == lever && (key.is_none() || c.key == key))
            })
            .cloned()
            .collect();
        assert!(
            lines.len() < law.lines.len(),
            "{} was in the law",
            lever.as_str()
        );
        let r = LawInForce::from_lines(&law.registry, &lines)
            .expect_err(&format!("a law without {} fails to load", lever.as_str()));
        assert_eq!(r.lever, Some(lever), "the rejection names the lever: {r:?}");
        let label = &law.registry.get(lever).label;
        assert!(
            r.message.contains(lever.as_str()) || r.message.contains(label.as_str()),
            "the message names {}: {}",
            lever.as_str(),
            r.message
        );
    }
}

/// A rejection of a case, its code and its lever.
fn assert_rejected(
    r: Result<(Date, Vec<Warning>), Rejection>,
    code: RejectCode,
    lever: Option<LeverId>,
    what: &str,
) -> String {
    match r {
        Ok(ok) => panic!("{what}: accepted {ok:?}, wanted {}", code.as_str()),
        Err(r) => {
            assert_eq!(r.code, code, "{what}: {}", r.message);
            assert_eq!(r.lever, lever, "{what}: the lever");
            if let Some(l) = lever {
                let label = &registry().get(l).label;
                assert!(
                    r.message.contains(label.as_str()),
                    "{what}: the message names the label {label}: {}",
                    r.message
                );
            }
            r.message
        }
    }
}

/// **AC-LEV-03** `[unit]` Each rejection code of the table is returned for a
/// command built to trigger it, with an English message that names the
/// lever's label and, for `out_of_range`, the bounds and the value in
/// display units; state and log are unchanged.
///
/// Readings: "names the lever's label" means the message holds the
/// registry's `label` of the lever (the `{label}` of the spec's templates;
/// when the string table of M8 gives labels their English text, this is
/// restated). `unknown_lever` cannot be built as a `Command` (its lever is a
/// `LeverId`), so it is triggered by a line of a commands file read by
/// `econ_io::commands::read_commands`, and its message names the unknown id;
/// `nothing_to_cancel` names no lever. Display units: rates as `40%`, money
/// as `2,550 lei` (the spec's examples), so the bounds of the VAT standard
/// rate are `0%` and `40%`, and those of the minimum wage `0 lei` and
/// `20,000 lei`. No committed lever is `retired`, and the choice levers are
/// keyed by scenario tables or planned, so `retired` and `bad_option` use
/// the committed registry with one lever's status changed (a `Registry` is
/// plain data). "State and log are unchanged": on a world, each rejected
/// order leaves the state hash, the pending list and the command log as
/// they were.
#[test]
fn ac_lev_03() {
    use LeverId as L;
    use LeverValue as V;
    let base = registry();
    let today = date(2021, 12, 10);
    let mut reg = base.clone();
    for m in &mut reg.levers {
        if m.id == L::TaxProfitRate {
            m.status = Status::Retired;
        }
        if m.id == L::TransferPensionIndexation {
            m.status = Status::Active;
        }
    }
    let check = |order: Order, code: RejectCode, lever: Option<LeverId>, what: &str| {
        assert_rejected(validate(&reg, &order, today, &[]), code, lever, what)
    };

    // The codes as the spec writes them.
    for (code, text) in [
        (RejectCode::UnknownLever, "unknown_lever"),
        (RejectCode::Retired, "retired"),
        (RejectCode::NotYet, "not_yet"),
        (RejectCode::WrongShape, "wrong_shape"),
        (RejectCode::OutOfRange, "out_of_range"),
        (RejectCode::BadKey, "bad_key"),
        (RejectCode::BadOption, "bad_option"),
        (RejectCode::BadSchedule, "bad_schedule"),
        (RejectCode::TooEarly, "too_early"),
        (RejectCode::NothingToCancel, "nothing_to_cancel"),
    ] {
        assert_eq!(code.as_str(), text);
    }

    // unknown_lever: a commands file naming no lever of the registry.
    let dir = fresh_dir("lev03_unknown");
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("commands.jsonl");
    std::fs::write(
        &file,
        "{\"given\":\"2021-12-10\",\"lever\":\"tax.vat.rate_standard\",\"value\":2000}\n\
         {\"given\":\"2021-12-10\",\"lever\":\"tax.vat.rate_mega\",\"value\":2100}\n",
    )
    .expect("the commands file writes");
    match read_commands(&file, base) {
        Ok(lines) => panic!("an unknown lever is refused, got {lines:?}"),
        Err((line, r)) => {
            assert_eq!(line, 2, "the line of the unknown lever");
            assert_eq!(r.code, RejectCode::UnknownLever);
            assert_eq!(r.lever, None);
            assert!(r.message.contains("tax.vat.rate_mega"), "{}", r.message);
        }
    }

    // retired, not_yet.
    check(
        set(L::TaxProfitRate, None, V::Rate(1_500)),
        RejectCode::Retired,
        Some(L::TaxProfitRate),
        "a retired lever",
    );
    assert_eq!(base.get(L::MoneyPolicyRate).status, Status::Planned);
    check(
        set(L::MoneyPolicyRate, None, V::Rate(200)),
        RejectCode::NotYet,
        Some(L::MoneyPolicyRate),
        "a planned lever",
    );

    // wrong_shape.
    check(
        set(L::TaxVatRateStandard, None, V::Money(Bani(2_100))),
        RejectCode::WrongShape,
        Some(L::TaxVatRateStandard),
        "money for a rate",
    );

    // out_of_range: the bounds and the value in display units.
    let vat = base.get(L::TaxVatRateStandard);
    assert_eq!(format_value(vat, &V::Rate(2_100)), "21%");
    let m = check(
        set(L::TaxVatRateStandard, None, V::Rate(4_100)),
        RejectCode::OutOfRange,
        Some(L::TaxVatRateStandard),
        "VAT at 41%",
    );
    for shown in ["0%", "40%", "41%"] {
        assert!(m.contains(shown), "the message shows {shown}: {m}");
    }
    let mw = base.get(L::LabourMinimumWage);
    assert_eq!(format_value(mw, &V::Money(Bani(255_000))), "2,550 lei");
    let m = check(
        set(L::LabourMinimumWage, None, V::Money(Bani(2_000_100))),
        RejectCode::OutOfRange,
        Some(L::LabourMinimumWage),
        "a minimum wage of 20,001 lei",
    );
    for shown in ["0 lei", "20,000 lei", "20,001 lei"] {
        assert!(m.contains(shown), "the message shows {shown}: {m}");
    }
    check(
        set(L::LabourMinimumWage, None, V::Money(Bani(-100))),
        RejectCode::OutOfRange,
        Some(L::LabourMinimumWage),
        "a negative minimum wage",
    );
    let sched = |rows: Vec<(i64, i32)>| {
        set(
            L::TaxIncomeSchedule,
            None,
            V::Schedule(rows.into_iter().map(|(t, r)| (Bani(t), r)).collect()),
        )
    };
    check(
        sched(vec![(0, 1_000), (500_000, 9_001)]),
        RejectCode::OutOfRange,
        Some(L::TaxIncomeSchedule),
        "a schedule rate above its range",
    );
    check(
        sched((0..11).map(|i| (i * 100_000, 1_000)).collect()),
        RejectCode::OutOfRange,
        Some(L::TaxIncomeSchedule),
        "eleven rows",
    );
    check(
        sched(Vec::new()),
        RejectCode::OutOfRange,
        Some(L::TaxIncomeSchedule),
        "an empty schedule",
    );

    // bad_key: missing, given on an unkeyed lever, unknown.
    let dependants = base.get(L::TaxIncomeDeductionBase).keys.len();
    assert_eq!(dependants, 5, "the dependants table");
    for (order, what) in [
        (
            set(L::TaxIncomeDeductionBase, None, V::Money(Bani(51_000))),
            "a keyed lever without a key",
        ),
        (
            set(L::TaxVatRateStandard, Some(0), V::Rate(2_000)),
            "a key on an unkeyed lever",
        ),
        (
            set(L::TaxIncomeDeductionBase, Some(5), V::Money(Bani(51_000))),
            "an unknown key",
        ),
    ] {
        let lever = order_lever(&order);
        check(order, RejectCode::BadKey, Some(lever), what);
    }

    // bad_option.
    let indexation = reg.get(L::TransferPensionIndexation);
    let n = u16::try_from(indexation.options.len()).expect("fits");
    check(
        set(L::TransferPensionIndexation, None, V::Choice(n)),
        RejectCode::BadOption,
        Some(L::TransferPensionIndexation),
        "an option past the list",
    );

    // bad_schedule: not strictly ascending, or not from 0.
    for (rows, what) in [
        (vec![(0, 1_000), (0, 2_000)], "two rows at 0"),
        (
            vec![(0, 1_000), (500_000, 1_600), (400_000, 2_000)],
            "a falling threshold",
        ),
        (vec![(100_000, 1_000)], "a first threshold above 0"),
    ] {
        check(
            sched(rows),
            RejectCode::BadSchedule,
            Some(L::TaxIncomeSchedule),
            what,
        );
    }

    // too_early: a law before the 1st of the next month.
    check(
        set_on(
            L::TaxVatRateStandard,
            None,
            V::Rate(2_000),
            date(2021, 12, 31),
        ),
        RejectCode::TooEarly,
        Some(L::TaxVatRateStandard),
        "VAT from 31 December",
    );

    // nothing_to_cancel.
    check(
        Order::Cancel { seq: 7 },
        RejectCode::NothingToCancel,
        None,
        "a cancel of nothing",
    );

    // On a world: state and log unchanged by every rejection.
    let mut w = lever_world(1000);
    assert_eq!(w.date(), start_date());
    let accepted = give(&mut w, set(L::TaxVatRateStandard, None, V::Rate(2_000)));
    let hash = w.state_hash();
    let pending = w.pending().to_vec();
    let log = w.command_log().to_vec();
    let events = w.events().len();
    for (order, code) in [
        (
            set(L::MoneyPolicyRate, None, V::Rate(200)),
            RejectCode::NotYet,
        ),
        (
            set(L::TaxVatRateStandard, None, V::Money(Bani(1))),
            RejectCode::WrongShape,
        ),
        (
            set(L::TaxVatRateStandard, None, V::Rate(4_100)),
            RejectCode::OutOfRange,
        ),
        (
            set(L::TaxIncomeDeductionBase, None, V::Money(Bani(1))),
            RejectCode::BadKey,
        ),
        (sched(vec![(100_000, 1_000)]), RejectCode::BadSchedule),
        (
            set_on(
                L::TaxVatRateStandard,
                None,
                V::Rate(2_000),
                date(2021, 12, 2),
            ),
            RejectCode::TooEarly,
        ),
        (
            Order::Cancel {
                seq: accepted.seq + 100,
            },
            RejectCode::NothingToCancel,
        ),
    ] {
        let r = w.give(order).expect_err("rejected on the world");
        assert_eq!(r.code, code, "{}", r.message);
        assert_eq!(
            w.state_hash(),
            hash,
            "{}: the state is unchanged",
            code.as_str()
        );
        assert_eq!(
            w.pending(),
            pending.as_slice(),
            "{}: pending unchanged",
            code.as_str()
        );
        assert_eq!(
            w.command_log(),
            log.as_slice(),
            "{}: the log is unchanged",
            code.as_str()
        );
        assert_eq!(w.events().len(), events, "{}: no event", code.as_str());
    }
}

/// The lever of a `Set`.
fn order_lever(order: &Order) -> LeverId {
    match order {
        Order::Set(c) => c.lever,
        Order::Cancel { .. } => panic!("a cancel has no lever"),
    }
}

/// **AC-LEV-04** `[unit]` Effective dates: a `law` command given on any day
/// of a month takes effect on the 1st of the next month (plus `min_lag`);
/// `market` and `world` the next day; a later requested date is kept; an
/// earlier one is `too_early`.
///
/// Readings: `min_lag` is shown on a copy of the VAT standard rate's entry
/// with `min_lag_months = 1` (no committed lever has one). The committed
/// `market` lever (`money.policy_rate`) is planned, so the validation part
/// uses the registry with it made active.
#[test]
fn ac_lev_04() {
    use LeverId as L;
    use LeverValue as V;
    let reg = registry();
    let vat = reg.get(L::TaxVatRateStandard);
    assert_eq!(vat.kind, Kind::Law);
    assert_eq!(vat.min_lag_months, 0);

    // Law: every day of a month, to the 1st of the next.
    for (year, month, next) in [
        (2021, 12, date(2022, 1, 1)),
        (2022, 2, date(2022, 3, 1)),
        (2024, 2, date(2024, 3, 1)),
        (2022, 6, date(2022, 7, 1)),
    ] {
        let days = date(year, month, 1).days_in_month();
        for d in 1..=days {
            assert_eq!(
                earliest_effective(vat, date(year, month, d)),
                next,
                "law given {year}-{month}-{d}"
            );
        }
    }

    // Law with a month's lag.
    let mut lagged = vat.clone();
    lagged.min_lag_months = 1;
    assert_eq!(
        earliest_effective(&lagged, date(2022, 2, 14)),
        date(2022, 4, 1)
    );
    assert_eq!(
        earliest_effective(&lagged, date(2022, 11, 30)),
        date(2023, 1, 1)
    );
    assert_eq!(
        earliest_effective(&lagged, date(2022, 12, 1)),
        date(2023, 2, 1)
    );

    // Market and world: the next day.
    let policy = reg.get(L::MoneyPolicyRate);
    assert_eq!(policy.kind, Kind::Market);
    let foreign = reg.get(L::WorldForeignRate);
    assert_eq!(foreign.kind, Kind::World);
    for meta in [policy, foreign] {
        for (given, next) in [
            (date(2021, 12, 1), date(2021, 12, 2)),
            (date(2021, 12, 31), date(2022, 1, 1)),
            (date(2022, 2, 28), date(2022, 3, 1)),
            (date(2024, 2, 28), date(2024, 2, 29)),
        ] {
            assert_eq!(
                earliest_effective(meta, given),
                next,
                "{} given {given:?}",
                meta.id.as_str()
            );
        }
    }

    // Validation: none = the earliest; later kept; earlier too early.
    let mut active = reg.clone();
    for m in &mut active.levers {
        if m.id == L::MoneyPolicyRate {
            m.status = Status::Active;
        }
    }
    let given = date(2022, 2, 14);
    let ok = |order: Order| match validate(&active, &order, given, &[]) {
        Ok((d, _)) => d,
        Err(r) => panic!("{order:?}: rejected {:?}: {}", r.code, r.message),
    };
    assert_eq!(
        ok(set(L::TaxVatRateStandard, None, V::Rate(2_000))),
        date(2022, 3, 1)
    );
    assert_eq!(
        ok(set_on(
            L::TaxVatRateStandard,
            None,
            V::Rate(2_000),
            date(2022, 3, 1)
        )),
        date(2022, 3, 1),
        "the earliest day itself"
    );
    assert_eq!(
        ok(set_on(
            L::TaxVatRateStandard,
            None,
            V::Rate(2_000),
            date(2022, 6, 1)
        )),
        date(2022, 6, 1),
        "a later day is kept"
    );
    assert_eq!(
        ok(set(L::MoneyPolicyRate, None, V::Rate(200))),
        date(2022, 2, 15)
    );
    assert_eq!(
        ok(set_on(
            L::MoneyPolicyRate,
            None,
            V::Rate(200),
            date(2022, 3, 10)
        )),
        date(2022, 3, 10),
        "a later day is kept"
    );
    for (order, what) in [
        (
            set_on(
                L::TaxVatRateStandard,
                None,
                V::Rate(2_000),
                date(2022, 2, 28),
            ),
            "a law before the 1st",
        ),
        (
            set_on(L::TaxVatRateStandard, None, V::Rate(2_000), given),
            "a law on the day given",
        ),
        (
            set_on(L::MoneyPolicyRate, None, V::Rate(200), given),
            "a market change on the day given",
        ),
        (
            set_on(L::MoneyPolicyRate, None, V::Rate(200), date(2022, 2, 1)),
            "a market change in the past",
        ),
    ] {
        match validate(&active, &order, given, &[]) {
            Ok(ok) => panic!("{what}: accepted {ok:?}"),
            Err(r) => assert_eq!(r.code, RejectCode::TooEarly, "{what}: {}", r.message),
        }
    }
}

/// **AC-LEV-10** `[unit]` A VAT standard rate of 14% is accepted with a
/// `PolicyWarning` (EU floor), not rejected.
///
/// Readings: the warning is the `EuRule` warning of the VAT standard rate,
/// returned by `validate` and by `World::give`, and the world's events hold
/// it as a `PolicyEvent::Warning`. 15% is the floor itself (the spec warns
/// "below 15%"), so 15% carries no EU warning.
#[test]
fn ac_lev_10() {
    use LeverId as L;
    use LeverValue as V;
    let reg = registry();
    let today = date(2021, 12, 10);
    let order = set(L::TaxVatRateStandard, None, V::Rate(1_400));
    let (effective, warnings) = validate(reg, &order, today, &[])
        .unwrap_or_else(|r| panic!("14% is accepted: {:?} {}", r.code, r.message));
    assert_eq!(effective, date(2022, 1, 1));
    assert!(
        warnings
            .iter()
            .any(|w| w.code == WarningCode::EuRule && w.lever == L::TaxVatRateStandard),
        "an EU warning: {warnings:?}"
    );
    let (_, at_floor) = validate(
        reg,
        &set(L::TaxVatRateStandard, None, V::Rate(1_500)),
        today,
        &[],
    )
    .unwrap_or_else(|r| panic!("15% is accepted: {}", r.message));
    assert!(
        !at_floor.iter().any(|w| w.code == WarningCode::EuRule),
        "no EU warning at 15%: {at_floor:?}"
    );

    let mut w = lever_world(1000);
    let accepted = give(&mut w, order);
    assert!(
        accepted
            .warnings
            .iter()
            .any(|x| x.code == WarningCode::EuRule && x.lever == L::TaxVatRateStandard),
        "the world's acceptance carries the warning: {accepted:?}"
    );
    assert!(
        w.pending().iter().any(|p| p.seq == accepted.seq),
        "the command is pending"
    );
    assert!(
        w.command_log().iter().any(|l| l.seq == Some(accepted.seq)),
        "the command is logged"
    );
    assert!(
        w.events().iter().any(|e| matches!(
            e,
            PolicyEvent::Warning { seq, warning, .. }
                if *seq == accepted.seq && warning.code == WarningCode::EuRule
        )),
        "a PolicyWarning event: {:?}",
        w.events()
    );
}
