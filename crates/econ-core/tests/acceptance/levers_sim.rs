//! Acceptance tests of the lever system's `[sim]` criteria (spec
//! game/lever-system): commands taking effect (AC-LEV-05), replay
//! (AC-LEV-06), saves (AC-LEV-07), headless play (AC-LEV-08), the starting
//! values as commands (AC-LEV-09), the ministry budgets (AC-LEV-11 to 13),
//! every alpha lever at the ends of its range (AC-LEV-14), the "why" of a
//! VAT change (AC-LEV-15) and a world setting (AC-LEV-16). Written from the
//! spec against the API skeleton, before the code (ADR-0014); behind the
//! feature `levers`. Worlds are the Romania scenario at 1:1000, seed 42,
//! under the law of its `law.json` and `world.json`; day 0 is 1 December
//! 2021.

use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::sync::OnceLock;

use econ_core::levers::{
    Group, LeverId, LeverMeta, LeverValue, Order, PolicyEvent, RejectCode, Shape, attribute_groups,
};
use econ_core::world::World;
use econ_io::commands::{load_world, save_world};
use econ_ledger::{FlowCode, Sector};
use econ_popgen::NO_EMPLOYER;
use econ_types::{Bani, Date};
use serde_json::Value;

use super::support::{
    assert_day_checks, fresh_dir, industry, io_table_path, repo_root, scenario_dir, step,
};
use super::support_lev::{
    Plan, budget_month, date, first_difference, give, is_active, key_of, keys_of, lever_world,
    option_of, public_unit, replay, run_checked_through, run_plan, set, set_on, step_to,
};

/// The value of a money lever in force.
fn money_in_force(w: &World, lever: LeverId, key: Option<u32>) -> Bani {
    match w.law().get(lever, key) {
        Some(LeverValue::Money(b)) => *b,
        other => panic!("{} {key:?}: money in force, got {other:?}", lever.as_str()),
    }
}

/// The value of a rate lever in force, basis points.
fn rate_in_force(w: &World, lever: LeverId, key: Option<u32>) -> i32 {
    match w.law().get(lever, key) {
        Some(LeverValue::Rate(r)) => *r,
        other => panic!("{} {key:?}: a rate in force, got {other:?}", lever.as_str()),
    }
}

/// `|a − b| ≤ tol`.
fn near(a: Bani, b: Bani, tol: Bani) -> bool {
    (a.get() - b.get()).abs() <= tol.get()
}

/// `amount × num / den`, rounded down.
fn part(amount: Bani, num: i64, den: i64) -> Bani {
    let v = i128::from(amount.get()) * i128::from(num) / i128::from(den);
    Bani(i64::try_from(v).expect("fits"))
}

/// The flow code the ledger names so.
fn code(name: &str) -> FlowCode {
    FlowCode::ALL
        .iter()
        .copied()
        .find(|c| c.name() == name)
        .unwrap_or_else(|| panic!("no flow code {name}"))
}

/// **AC-LEV-05** `[sim]` A command changes the law in force on its
/// effective day and not before: until the day before, the daily state
/// hashes equal those of a run without it.
///
/// Readings: the command is the VAT standard rate from 19% to 21%, given on
/// 10 December 2021, so it takes effect on 1 January 2022 (day 31). "Changes
/// the law in force on its effective day": the law in force reads 19% after
/// 31 December has run and 21% after 1 January has run, and 1 January's
/// state hash differs from the run without it (prices read VAT daily).
#[test]
fn ac_lev_05() {
    let last = date(2022, 1, 31);
    let mut base = lever_world(1000);
    let base_hashes = run_checked_through(&mut base, last);

    let mut w = lever_world(1000);
    let mut hashes: Vec<u64> = Vec::new();
    while w.date() < date(2021, 12, 10) {
        hashes.push(step(&mut w).state_hash);
    }
    let accepted = give(
        &mut w,
        set(LeverId::TaxVatRateStandard, None, LeverValue::Rate(2_100)),
    );
    assert_eq!(accepted.effective, date(2022, 1, 1));
    while w.date() <= last {
        let today = w.date();
        let before = rate_in_force(&w, LeverId::TaxVatRateStandard, None);
        let r = step(&mut w);
        assert_day_checks(&w, &r);
        let after = rate_in_force(&w, LeverId::TaxVatRateStandard, None);
        if today < date(2022, 1, 1) {
            assert_eq!((before, after), (1_900, 1_900), "{today:?}: 19% in force");
        } else if today == date(2022, 1, 1) {
            assert_eq!((before, after), (1_900, 2_100), "the change on 1 January");
        } else {
            assert_eq!(after, 2_100, "{today:?}: 21% in force");
        }
        hashes.push(r.state_hash);
    }
    assert_eq!(hashes.len(), base_hashes.len());
    assert_eq!(
        first_difference(&hashes, &base_hashes),
        Some(31),
        "the hashes equal the run without the command until 31 December and differ on 1 January"
    );
}

/// The twenty orders of AC-LEV-06 on a world at its start.
fn year_of_orders(w: &World) -> Vec<(Date, Plan)> {
    use LeverId as L;
    use LeverValue as V;
    let lei = Bani::from_lei;
    let category = w.registry().get(L::TaxVatCategory);
    let standard = option_of(category, "standard");
    let g = |o: Order| Plan::Give(o);
    vec![
        (
            date(2021, 12, 10),
            g(set(L::TaxVatRateStandard, None, V::Rate(2_000))),
        ),
        (
            date(2021, 12, 10),
            g(set(L::TaxVatRateStandard, None, V::Rate(2_100))),
        ),
        (
            date(2021, 12, 20),
            g(set(L::LabourMinimumWage, None, V::Money(lei(2_550)))),
        ),
        (
            date(2022, 1, 15),
            g(set(L::TaxContribCass, None, V::Rate(1_100))),
        ),
        (
            date(2022, 1, 15),
            g(set(L::TaxProfitRate, None, V::Rate(1_500))),
        ),
        (
            date(2022, 2, 3),
            g(set(L::TransferChildTo18, None, V::Money(lei(250)))),
        ),
        (
            date(2022, 2, 3),
            g(set(
                L::TransferPensionPointValue,
                None,
                V::Money(lei(1_586)),
            )),
        ),
        (
            date(2022, 2, 20),
            g(set(L::TaxVatRateReduced, None, V::Rate(800))),
        ),
        (date(2022, 2, 20), Plan::CancelNth(7)),
        (
            date(2022, 3, 5),
            g(set(
                L::TaxIncomeSchedule,
                None,
                V::Schedule(vec![(Bani::ZERO, 1_000), (lei(5_000), 1_600)]),
            )),
        ),
        (
            date(2022, 3, 5),
            g(set(L::TransferUnemploymentIsr, None, V::Money(lei(550)))),
        ),
        (
            date(2022, 4, 10),
            g(set(L::TaxContribCas, None, V::Rate(2_400))),
        ),
        (
            date(2022, 4, 10),
            g(set(L::TransferChildUnder2, None, V::Money(lei(500)))),
        ),
        (
            date(2022, 5, 2),
            g(set(
                L::TaxVatCategory,
                Some(key_of(w, L::TaxVatCategory, "restaurants")),
                V::Choice(standard),
            )),
        ),
        (
            date(2022, 5, 2),
            g(set(
                L::TransferMinIncomeFloor,
                Some(key_of(w, L::TransferMinIncomeFloor, "1")),
                V::Money(lei(200)),
            )),
        ),
        (
            date(2022, 6, 1),
            g(set(
                L::TaxIncomeDeductionBase,
                Some(key_of(w, L::TaxIncomeDeductionBase, "2")),
                V::Money(lei(900)),
            )),
        ),
        (
            date(2022, 6, 1),
            g(set(L::TaxProfitMicroRateStaff, None, V::Rate(200))),
        ),
        (
            date(2022, 7, 7),
            g(set(L::LabourMinimumWage, None, V::Money(lei(2_700)))),
        ),
        (
            date(2022, 8, 8),
            g(set(L::TransferPensionMinimum, None, V::Money(lei(1_000)))),
        ),
        (
            date(2022, 9, 9),
            g(set_on(
                L::TaxVatRateStandard,
                None,
                V::Rate(1_900),
                date(2022, 11, 1),
            )),
        ),
    ]
}

/// **AC-LEV-06** `[sim]` Replay: a year at 1:1000 with twenty commands (a
/// cancel and two same-day changes of one lever included), replayed from the
/// scenario, seed and log, gives the same state hash every day.
///
/// Readings: "a year" is 1 December 2021 to 30 November 2022; the twenty
/// orders are of active levers, one is the cancel of a pending change of the
/// reduced VAT rate, and two set the standard rate on the same day for the
/// same effective day. Replay is a fresh world of the same scenario and
/// seed, given each line of the first run's `command_log` on its `given`
/// day with its logged `effective` day; it must get the same `seq`s, the
/// same log and the same hash every day. The law in force also shows the
/// later of the two same-day changes and the cancel (spec, "Order").
#[test]
fn ac_lev_06() {
    let last = date(2022, 11, 30);
    let mut w = lever_world(1000);
    let plan = year_of_orders(&w);
    assert_eq!(plan.len(), 20, "twenty commands");
    let (hashes, accepted) = run_plan(&mut w, &plan, last);
    assert_eq!(accepted.len(), 20);
    assert_eq!(
        accepted[0].effective, accepted[1].effective,
        "same-day changes"
    );
    assert_eq!(w.command_log().len(), 20, "every order is logged");
    assert!(
        w.command_log()
            .iter()
            .all(|l| l.seq.is_some() && l.effective.is_some())
    );
    // Seen in the law: the later same-day change won until November, the
    // cancelled change never took effect, the November change did.
    assert_eq!(rate_in_force(&w, LeverId::TaxVatRateReduced, None), 900);
    assert_eq!(rate_in_force(&w, LeverId::TaxVatRateStandard, None), 1_900);
    assert!(
        w.events().iter().any(|e| matches!(
            e,
            PolicyEvent::Changed(c) if c.lever == LeverId::TaxVatRateStandard
                && c.new == LeverValue::Rate(2_100)
                && c.effective == date(2022, 1, 1)
        )),
        "21% took effect on 1 January"
    );

    let (again, replayed) = replay(1000, w.command_log(), last);
    assert_eq!(again.command_log(), w.command_log(), "the replay's log");
    assert_eq!(replayed.len(), hashes.len());
    assert_eq!(
        first_difference(&replayed, &hashes),
        None,
        "the replay gives the same hash every day"
    );
}

/// **AC-LEV-07** `[sim]` Saving with commands pending, loading and running
/// on gives the same daily hashes as the uninterrupted run.
///
/// Readings: the save is made on 25 December 2021 with three commands
/// pending (two law changes effective 1 January and one effective 1
/// March), by `econ_io::commands::save_world`, and loaded by `load_world`
/// with the scenario folder; both worlds then run to 31 March 2022. The
/// loaded world has the saved law in force, pending list and log.
#[test]
fn ac_lev_07() {
    use LeverId as L;
    use LeverValue as V;
    let mut w = lever_world(1000);
    let plan = vec![
        (
            date(2021, 12, 10),
            Plan::Give(set(L::TaxVatRateStandard, None, V::Rate(2_100))),
        ),
        (
            date(2021, 12, 20),
            Plan::Give(set(
                L::LabourMinimumWage,
                None,
                V::Money(Bani::from_lei(2_550)),
            )),
        ),
        (
            date(2021, 12, 24),
            Plan::Give(set_on(
                L::TaxContribCass,
                None,
                V::Rate(1_100),
                date(2022, 3, 1),
            )),
        ),
    ];
    let _ = run_plan(&mut w, &plan, date(2021, 12, 24));
    assert_eq!(w.date(), date(2021, 12, 25));
    assert_eq!(w.pending().len(), 3, "three commands pending");

    let bytes = save_world(&w).expect("the world saves");
    let mut loaded =
        load_world(&bytes, scenario_dir(1000), &io_table_path()).expect("the save loads");
    assert_eq!(loaded.date(), w.date());
    assert_eq!(loaded.state_hash(), w.state_hash());
    assert_eq!(loaded.law(), w.law());
    assert_eq!(loaded.pending(), w.pending());
    assert_eq!(loaded.command_log(), w.command_log());

    let last = date(2022, 3, 31);
    let a = run_checked_through(&mut w, last);
    let b = run_checked_through(&mut loaded, last);
    assert_eq!(
        first_difference(&a, &b),
        None,
        "the loaded world gives the same daily hashes"
    );
    assert!(loaded.pending().is_empty() && w.pending().is_empty());
}

// --- headless play -----------------------------------------------------------

/// The `econ-cli` binary of this build profile with the feature `levers`,
/// built once.
fn econ_cli() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // The test binary is target/<profile>/deps/acceptance-<hash>.
        let exe = std::env::current_exe().expect("the test binary");
        let profile = exe
            .parent()
            .and_then(Path::parent)
            .expect("target/<profile>")
            .to_path_buf();
        let mut cmd = Process::new(option_env!("CARGO").unwrap_or("cargo"));
        cmd.args(["build", "-q", "-p", "econ-cli", "--features", "levers"])
            .current_dir(repo_root());
        if profile.file_name().is_some_and(|n| n == "release") {
            cmd.arg("--release");
        }
        let status = cmd.status().expect("cargo runs");
        assert!(status.success(), "econ-cli builds with the feature levers");
        profile.join(format!("econ-cli{}", std::env::consts::EXE_SUFFIX))
    })
}

/// Run `econ-cli play` on the 1:1000 scenario; its output folder.
fn play(commands: &Path, out: &Path) {
    let status = Process::new(econ_cli())
        .arg("play")
        .arg("--scenario")
        .arg(scenario_dir(1000))
        .arg("--commands")
        .arg(commands)
        .args(["--until", "2022-02-28", "--seed", "42", "--out"])
        .arg(out)
        .current_dir(repo_root())
        .status()
        .expect("econ-cli runs");
    assert_eq!(status.code(), Some(0), "econ-cli play exits 0");
}

/// Every file of a folder by name, with its bytes.
fn files_of(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .expect("the output folder")
        .map(|e| {
            let e = e.expect("an entry");
            let name = e.file_name().to_string_lossy().into_owned();
            let bytes = std::fs::read(e.path()).expect("a file");
            (name, bytes)
        })
        .collect();
    files.sort();
    files
}

/// **AC-LEV-08** `[sim]` `econ-cli play` writes the series, the applied log,
/// the events and `run.json`; feeding the applied log back gives
/// byte-identical outputs.
///
/// Readings: the run is the 1:1000 scenario, seed 42, to 28 February 2022,
/// from a commands file of two lines (one without `effective`); the outputs
/// are those the spec lists without `--daily`: `series_monthly.arrow`,
/// `series_monthly.csv`, `commands_applied.jsonl` (every line with `seq` and
/// `effective`), `events.jsonl` and `run.json` (with the seed). The second
/// run is fed the first run's `commands_applied.jsonl`; every file of the
/// two output folders is byte for byte the same.
#[test]
fn ac_lev_08() {
    let dir = fresh_dir("lev08_play");
    std::fs::create_dir_all(&dir).expect("a folder");
    let commands = dir.join("commands.jsonl");
    std::fs::write(
        &commands,
        "{\"given\":\"2021-12-10\",\"lever\":\"tax.vat.rate_standard\",\"value\":2100}\n\
         {\"given\":\"2021-12-20\",\"lever\":\"labour.minimum_wage\",\"value\":255000,\"effective\":\"2022-01-01\"}\n",
    )
    .expect("the commands file writes");
    let first = dir.join("first");
    play(&commands, &first);
    for f in [
        "series_monthly.arrow",
        "series_monthly.csv",
        "commands_applied.jsonl",
        "events.jsonl",
        "run.json",
    ] {
        assert!(first.join(f).is_file(), "play writes {f}");
    }
    let applied = std::fs::read_to_string(first.join("commands_applied.jsonl")).expect("the log");
    let lines: Vec<Value> = applied
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("a JSON line"))
        .collect();
    assert_eq!(lines.len(), 2, "both commands applied");
    for l in &lines {
        assert!(l["seq"].is_u64(), "seq filled: {l}");
        assert!(l["effective"].is_string(), "effective filled: {l}");
    }
    assert_eq!(lines[0]["effective"], Value::from("2022-01-01"));
    assert_eq!(lines[0]["value"], Value::from(2_100));
    let run: Value =
        serde_json::from_str(&std::fs::read_to_string(first.join("run.json")).expect("run.json"))
            .expect("run.json is JSON");
    assert_eq!(run["seed"], Value::from(42), "run.json has the seed: {run}");

    let second = dir.join("second");
    play(&first.join("commands_applied.jsonl"), &second);
    let (a, b) = (files_of(&first), files_of(&second));
    let names = |v: &[(String, Vec<u8>)]| v.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>();
    assert_eq!(names(&a), names(&b), "the same files");
    for ((name, x), (_, y)) in a.iter().zip(&b) {
        assert!(x == y, "{name} is byte-identical");
    }
}

/// **AC-LEV-09** `[sim]` Setting every active lever to its starting value on
/// the second day gives the same state hashes as no commands.
///
/// Readings: on 2 December 2021 (day 1) every active lever, and every key
/// of a keyed one, is given its value in force (that of `law.json` or
/// `world.json`); both runs go to 28 February 2022, past the law changes'
/// effective day of 1 January, and every daily hash is equal.
#[test]
fn ac_lev_09() {
    let last = date(2022, 2, 28);
    let mut base = lever_world(1000);
    let base_hashes = run_checked_through(&mut base, last);

    let mut w = lever_world(1000);
    let first = step(&mut w).state_hash;
    let metas: Vec<LeverMeta> = w
        .registry()
        .levers
        .iter()
        .filter(|m| is_active(m))
        .cloned()
        .collect();
    assert!(!metas.is_empty());
    let mut given = 0;
    for meta in &metas {
        for key in keys_of(meta) {
            let value = w
                .law()
                .get(meta.id, key)
                .unwrap_or_else(|| panic!("{} {key:?} is in force", meta.id.as_str()))
                .clone();
            give(&mut w, set(meta.id, key, value));
            given += 1;
        }
    }
    assert_eq!(w.pending().len(), given);
    let mut hashes = vec![first];
    hashes.extend(run_checked_through(&mut w, last));
    assert_eq!(
        first_difference(&hashes, &base_hashes),
        None,
        "the starting values as commands change nothing"
    );
}

// --- ministry budgets --------------------------------------------------------

/// **AC-LEV-11** `[sim]` Raising the education budget with pay unchanged
/// raises education's input purchases and leaves its employees' pay
/// unchanged; raising education's pay with the budget unchanged raises its
/// wage bill and lowers its input purchases by the same amount, until inputs
/// reach zero.
///
/// Readings: four worlds, each given one order on 1 December 2021 that takes
/// effect on 1 January 2022: none; `spend.ministry_budget[education]` × 1.2;
/// `spend.public_pay[education_services.public]` at 120%; the same at 300%
/// (the top of its range). January 2022's `BudgetLineMonth` of the line
/// `education` is compared. "Employees' pay unchanged": every person
/// employed by the education public unit in both worlds at the end of
/// January since day 0 has the same wage in both. "By the same amount":
/// wages plus inputs are the same with the pay raise as without (the
/// running allowance does not change). "Until inputs reach zero": at 300%
/// the line buys no inputs.
#[test]
fn ac_lev_11() {
    use LeverId as L;
    use LeverValue as V;
    let last = date(2022, 1, 31);
    let base_w = lever_world(1000);
    let line = key_of(&base_w, L::SpendMinistryBudget, "education");
    let unit_key = key_of(&base_w, L::SpendPublicPay, "education_services.public");
    let budget = money_in_force(&base_w, L::SpendMinistryBudget, Some(line));
    assert_eq!(
        rate_in_force(&base_w, L::SpendPublicPay, Some(unit_key)),
        10_000
    );
    let run = |order: Option<Order>| {
        let mut w = base_w.clone();
        if let Some(o) = order {
            let a = give(&mut w, o);
            assert_eq!(a.effective, date(2022, 1, 1));
        }
        run_checked_through(&mut w, last);
        w
    };
    let base = run(None);
    let richer = run(Some(set(
        L::SpendMinistryBudget,
        Some(line),
        V::Money(part(budget, 6, 5)),
    )));
    let paid_more = run(Some(set(
        L::SpendPublicPay,
        Some(unit_key),
        V::Rate(12_000),
    )));
    let paid_most = run(Some(set(
        L::SpendPublicPay,
        Some(unit_key),
        V::Rate(30_000),
    )));
    let jan = |w: &World| budget_month(w, "education", 1).clone();
    let (b, r, p, q) = (jan(&base), jan(&richer), jan(&paid_more), jan(&paid_most));

    // A larger budget: more inputs, the same pay.
    assert!(
        r.inputs > b.inputs,
        "inputs {:?} > {:?}",
        r.inputs,
        b.inputs
    );
    let u = public_unit(1000, "education_services");
    let mut compared = 0;
    for pp in 0..base.persons().employer.len() {
        let here = |w: &World| {
            usize::from(w.persons().employer[pp]) == u && w.persons().hired_on[pp].is_none()
        };
        if base.persons().employer[pp] != NO_EMPLOYER && here(&base) && here(&richer) {
            assert_eq!(
                richer.persons().wage[pp],
                base.persons().wage[pp],
                "person {pp}: pay unchanged by the budget"
            );
            compared += 1;
        }
    }
    assert!(compared > 0, "education's employees compared");

    // A pay rise: a larger wage bill, fewer inputs, by the same amount.
    assert!(p.wages > b.wages, "wage bill {:?} > {:?}", p.wages, b.wages);
    assert!(
        p.inputs < b.inputs,
        "inputs {:?} < {:?}",
        p.inputs,
        b.inputs
    );
    assert_eq!(
        p.wages + p.inputs,
        b.wages + b.inputs,
        "inputs fall by the wage bill's rise"
    );
    // Until inputs reach zero.
    assert!(q.wages > p.wages);
    assert_eq!(q.inputs, Bani::ZERO, "at 300% pay no inputs are bought");
}

/// **AC-LEV-12** `[sim]` With an education budget above what the unit can
/// spend, the year's government spending on education equals what was
/// paid; no posting moves the unspent part, and on 1 January the allowance
/// restarts from the new year's twelfths.
///
/// Readings: the education budget is doubled from 1 January 2022 (given on
/// day 0) and the world runs to 31 January 2023. For each month of 2022 the
/// line's running allowance to date is that many twelfths of the year's
/// running part, `A (1 − s)`, within a leu a month of rounding; what the
/// running part paid to date is the sum of the months' wages and inputs and
/// does not exceed the allowance (the unspent part is the difference). "The
/// year's government spending on education equals what was paid" and "no
/// posting moves the unspent part": over 2022 the government's outflows
/// under `gov.purchases` and `gov.investment` in the ledger equal the sum,
/// over every budget line and month, of wages, inputs and investment paid
/// (so nothing beyond what was paid is posted). In January 2023 the
/// allowance to date is one twelfth again and what was paid to date is
/// January's alone.
#[test]
fn ac_lev_12() {
    use LeverId as L;
    use LeverValue as V;
    let mut w = lever_world(1000);
    let line = key_of(&w, L::SpendMinistryBudget, "education");
    let budget = part(money_in_force(&w, L::SpendMinistryBudget, Some(line)), 2, 1);
    let share_line = key_of(&w, L::SpendInvestmentShare, "education");
    give(
        &mut w,
        set(L::SpendMinistryBudget, Some(line), V::Money(budget)),
    );
    step_to(&mut w, date(2022, 1, 1));
    let share = rate_in_force(&w, L::SpendInvestmentShare, Some(share_line));
    let running_year = part(budget, i64::from(10_000 - share), 10_000);

    let (purchases, investment) = (code("gov.purchases"), code("gov.investment"));
    let mut posted = Bani::ZERO;
    while w.date() <= date(2022, 12, 31) {
        let r = step(&mut w);
        assert_day_checks(&w, &r);
        let f = w.ledger().flows();
        posted -= f.get(purchases, Sector::Government) + f.get(investment, Sector::Government);
    }
    run_checked_through(&mut w, date(2023, 1, 31));

    let mut paid = Bani::ZERO;
    for m in 1..=12u32 {
        let b = budget_month(&w, "education", m);
        paid += b.wages + b.inputs;
        let due = part(running_year, i64::from(m), 12);
        assert!(
            near(
                b.running_allowance_to_date,
                due,
                Bani::from_lei(i64::from(m))
            ),
            "month {m}: allowance {:?}, twelfths {due:?}",
            b.running_allowance_to_date
        );
        assert_eq!(b.running_paid_to_date, paid, "month {m}: paid to date");
        if b.overrun == Bani::ZERO {
            assert!(
                b.running_paid_to_date <= b.running_allowance_to_date,
                "month {m}: paid within the allowance"
            );
        }
    }
    let spent: Bani = w
        .budget_months()
        .iter()
        .filter(|b| (1..=12).contains(&b.month.0))
        .map(|b| b.wages + b.inputs + b.investment)
        .fold(Bani::ZERO, |a, x| a + x);
    assert_eq!(
        posted, spent,
        "the ledger's purchases and investment of 2022 are what the lines paid"
    );

    let jan = budget_month(&w, "education", 13);
    assert!(
        near(
            jan.running_allowance_to_date,
            part(running_year, 1, 12),
            Bani::from_lei(1)
        ),
        "January 2023 restarts from one twelfth: {:?}",
        jan.running_allowance_to_date
    );
    assert_eq!(
        jan.running_paid_to_date,
        jan.wages + jan.inputs,
        "paid to date restarts"
    );
}

/// **AC-LEV-13** `[sim]` An investment share of 0 puts the whole health
/// budget into running; a share of 100% sends all of it to investment
/// purchases, wages are still paid and an overrun warning is emitted.
///
/// Readings: `spend.investment_share[health]` given on day 0, effective 1
/// January 2022; January's `BudgetLineMonth` of `health`. At 0: the running
/// allowance is a twelfth of the whole budget and no investment is bought.
/// At 100%: no running allowance, a twelfth of the budget bought as
/// investment, no inputs, wages paid (all of it overrun, paid by the
/// treasury) and a `BudgetOverrun` event naming `health` in January.
#[test]
fn ac_lev_13() {
    use LeverId as L;
    use LeverValue as V;
    let last = date(2022, 1, 31);
    let base_w = lever_world(1000);
    let line = key_of(&base_w, L::SpendMinistryBudget, "health");
    let share_line = key_of(&base_w, L::SpendInvestmentShare, "health");
    let budget = money_in_force(&base_w, L::SpendMinistryBudget, Some(line));
    let twelfth = part(budget, 1, 12);
    let run = |bp: i32| {
        let mut w = base_w.clone();
        give(
            &mut w,
            set(L::SpendInvestmentShare, Some(share_line), V::Rate(bp)),
        );
        run_checked_through(&mut w, last);
        w
    };

    let all_running = run(0);
    let m = budget_month(&all_running, "health", 1);
    assert!(
        near(m.running_allowance_to_date, twelfth, Bani::from_lei(1)),
        "the whole twelfth is running: {:?} against {twelfth:?}",
        m.running_allowance_to_date
    );
    assert_eq!(m.investment, Bani::ZERO, "no investment at a share of 0");

    let all_investment = run(10_000);
    let m = budget_month(&all_investment, "health", 1);
    assert_eq!(
        m.running_allowance_to_date,
        Bani::ZERO,
        "no running allowance"
    );
    assert!(
        near(m.investment, twelfth, Bani::from_lei(1)),
        "the twelfth goes to investment: {:?} against {twelfth:?}",
        m.investment
    );
    assert_eq!(m.inputs, Bani::ZERO, "no inputs");
    assert!(m.wages > Bani::ZERO, "wages are still paid");
    assert_eq!(m.overrun, m.wages, "the whole wage bill is overrun");
    assert!(
        all_investment.events().iter().any(|e| matches!(
            e,
            PolicyEvent::BudgetOverrun { date: d, line, overrun }
                if line == "health" && d.year == 2022 && d.month == 1 && *overrun > Bani::ZERO
        )),
        "an overrun warning names health in January"
    );
}

// --- every alpha lever -------------------------------------------------------

/// The bottom or the top of a lever's range in its shape: the first or the
/// last option of a choice, off or on for a flag, and for a schedule one row
/// from 0 at the lowest or the highest rate.
fn end_value(meta: &LeverMeta, top: bool) -> LeverValue {
    let bound = || {
        let (lo, hi) = meta
            .range
            .unwrap_or_else(|| panic!("{} has a range", meta.id.as_str()));
        if top { hi } else { lo }
    };
    let rate = |v: i64| i32::try_from(v).expect("a rate fits i32");
    match meta.shape {
        Shape::Rate => LeverValue::Rate(rate(bound())),
        Shape::Money => LeverValue::Money(Bani(bound())),
        Shape::Number => LeverValue::Number(bound()),
        Shape::Choice => {
            assert!(!meta.options.is_empty(), "{} has options", meta.id.as_str());
            let last = u16::try_from(meta.options.len() - 1).expect("fits");
            LeverValue::Choice(if top { last } else { 0 })
        }
        Shape::Flag => LeverValue::Flag(top),
        Shape::Schedule => LeverValue::Schedule(vec![(Bani::ZERO, rate(bound()))]),
    }
}

/// The alpha levers the owner deferred beyond M4 (roadmap M4, owner,
/// 2026-10-24), registered `planned`, and the milestone that wires each
/// (spec, "The alpha levers", deferred). `world.fuel_price` left the list on
/// 2026-10-26: its oil key is wired (owner, 2026-10-25); gas and coal are
/// planned keys of the active lever.
const DEFERRED: [(&str, &str); 5] = [
    ("transfer.pension.retirement_age", "M5"),
    ("world.wages_abroad", "M5"),
    ("informal.inspection_budget", "M15"),
    ("informal.audit_budget", "M15"),
    ("informal.penalty_multiple", "M15"),
];

/// The condition under which a lever's ends bind, for a lever whose end
/// cannot bind in the starting state (spec, "The alpha levers", ends that
/// bind under a condition): orders given on 1 December 2021 to the baseline
/// and to the end's run alike, and the day the end takes effect if not the
/// earliest. `None` for a lever that binds in the starting state.
struct Condition {
    orders: Vec<Order>,
    end_effective: Option<Date>,
}

fn condition(w: &World, lever: LeverId) -> Option<Condition> {
    use LeverId as L;
    let lei = |v: i64| LeverValue::Money(Bani::from_lei(v));
    let law = |orders: Vec<Order>| {
        Some(Condition {
            orders,
            end_effective: None,
        })
    };
    match lever {
        // A ceiling below some wages (6,000 lei a month, about the mean
        // wage): the ceiling's top (above every wage) removes it.
        L::TaxContribCeiling => law(vec![set(L::TaxContribCeiling, None, lei(6_000))]),
        // A wealth tax of 1% a year: a threshold above every net worth
        // (the top) takes it away.
        L::TaxWealthThreshold => law(vec![set(L::TaxWealthRate, None, LeverValue::Rate(100))]),
        // A minimum pension of 1,000 lei, above the 800 lei at which stage
        // G puts the lowest pensions: the bottom drops the lifted ones back.
        L::TransferPensionMinimum => law(vec![set(L::TransferPensionMinimum, None, lei(1_000))]),
        // Households of six or more on the minimum income: the floor of a
        // five-person household at the top of its range (10,000 lei), so
        // every one of them that takes the benefit up qualifies.
        L::TransferMinIncomePerExtra => law(vec![set(
            L::TransferMinIncomeFloor,
            Some(key_of(w, L::TransferMinIncomeFloor, "5")),
            lei(10_000),
        )]),
        // A minimum wage that lifts wages: 2,550 lei, the law of January
        // 2022, from the wage round on.
        L::LabourMinimumWage => law(vec![set(L::LabourMinimumWage, None, lei(2_550))]),
        // Bonds held by the central bank: 1 billion lei bought at the end of
        // December, nothing in force from 1 January, when the end takes
        // effect (so a sale has something to sell).
        L::MoneyQe => Some(Condition {
            orders: vec![
                set_on(L::MoneyQe, None, lei(1_000_000_000), date(2021, 12, 2)),
                set_on(L::MoneyQe, None, lei(0), date(2022, 1, 1)),
            ],
            end_effective: Some(date(2022, 1, 1)),
        }),
        _ => None,
    }
}

/// A run of the baseline: the world as the end's run starts from it, its
/// daily hashes and the world after `last`.
struct Baseline {
    start: World,
    hashes: Vec<u64>,
    end: World,
}

/// Run a world through `last`, every day checked; what went wrong, if
/// anything.
fn run_through(w: &mut World, last: Date, what: &str) -> Result<Vec<u64>, String> {
    let mut hashes = Vec::new();
    while w.date() <= last {
        let r = w
            .step(&[])
            .map_err(|e| format!("{what}: a day fails on {:?}: {e:?}", w.date()))?;
        if !r.checks_ok || !r.failed.is_empty() || w.ledger().check_invariants().is_err() {
            return Err(format!(
                "{what}: checks fail on {:?}: {:?}",
                r.date, r.failed
            ));
        }
        hashes.push(r.state_hash);
    }
    Ok(hashes)
}

/// The baseline of a lever under its condition; `None` for a lever without
/// one (its baseline is the shared one).
fn baseline_of(
    shared: &Baseline,
    meta: &LeverMeta,
    last: Date,
) -> Result<Option<Baseline>, String> {
    let id = meta.id.as_str();
    let Some(cond) = condition(&shared.start, meta.id) else {
        return Ok(None);
    };
    let mut start = shared.start.clone();
    for order in cond.orders {
        if let Err(r) = start.give(order) {
            return Err(format!(
                "{id}: its condition is rejected {:?}: {}",
                r.code, r.message
            ));
        }
    }
    let mut end = start.clone();
    let hashes = run_through(
        &mut end,
        last,
        &format!("{id}: the baseline under its condition"),
    )?;
    Ok(Some(Baseline { start, hashes, end }))
}

/// Whether a key of a lever's commands is one of its planned keys
/// (`planned_keys`, read from the registry at run time): registered, not
/// wired yet.
fn is_planned_key(meta: &LeverMeta, key: Option<u32>) -> bool {
    key.is_some_and(|k| {
        let name = &meta.keys[usize::try_from(k).expect("fits")];
        meta.planned_keys.contains(name)
    })
}

/// One end of one active lever for three months against its baseline;
/// what went wrong, if anything. Its active keys are moved to the end
/// together; a command at the end on each of its planned keys must be
/// rejected with `not_yet`.
fn try_end(base: &Baseline, meta: &LeverMeta, top: bool, last: Date) -> Vec<String> {
    let id = meta.id.as_str();
    let end = if top { "top" } else { "bottom" };
    let value = end_value(meta, top);
    let effective = condition(&base.start, meta.id).and_then(|c| c.end_effective);
    let mut out = Vec::new();
    let (planned_keys, active_keys): (Vec<_>, Vec<_>) = keys_of(meta)
        .into_iter()
        .partition(|k| is_planned_key(meta, *k));
    for key in planned_keys {
        let mut probe = base.start.clone();
        match probe.give(set(meta.id, key, value.clone())) {
            Err(r) if r.code == RejectCode::NotYet => {}
            Err(r) => out.push(format!(
                "{id} planned key {key:?} at its {end}: rejected {:?}, not not_yet: {}",
                r.code, r.message
            )),
            Ok(a) => out.push(format!(
                "{id} planned key {key:?} at its {end}: accepted {a:?}"
            )),
        }
    }
    let mut w = base.start.clone();
    let mut moves = false;
    for key in active_keys {
        if base.end.law().get(meta.id, key) != Some(&value) {
            moves = true;
        }
        let order = match effective {
            Some(d) => set_on(meta.id, key, value.clone(), d),
            None => set(meta.id, key, value.clone()),
        };
        if let Err(r) = w.give(order) {
            out.push(format!(
                "{id} at its {end}: rejected {:?}: {}",
                r.code, r.message
            ));
            return out;
        }
    }
    let hashes = match run_through(&mut w, last, &format!("{id} at its {end}")) {
        Ok(h) => h,
        Err(e) => {
            out.push(e);
            return out;
        }
    };
    if moves && hashes == base.hashes {
        out.push(format!(
            "{id} at its {end}: the state hash never moves (not read)"
        ));
    }
    out
}

/// Both ends of a planned lever are rejected with `not_yet`; what went
/// wrong, if anything.
fn try_planned(start: &World, meta: &LeverMeta) -> Vec<String> {
    let id = meta.id.as_str();
    let mut out = Vec::new();
    for top in [false, true] {
        let end = if top { "top" } else { "bottom" };
        let mut w = start.clone();
        for key in keys_of(meta) {
            match w.give(set(meta.id, key, end_value(meta, top))) {
                Err(r) if r.code == RejectCode::NotYet => {}
                Err(r) => out.push(format!(
                    "{id} (planned) at its {end}: rejected {:?}, not not_yet: {}",
                    r.code, r.message
                )),
                Ok(a) => out.push(format!("{id} (planned) at its {end}: accepted {a:?}")),
            }
        }
    }
    out
}

/// **AC-LEV-14** `[sim]` For every alpha lever with `status = "active"`:
/// moving it to the bottom and the top of its range for three months changes
/// the state hash against the baseline (it is read) and breaks no invariant
/// I-1 to I-8; an end that cannot bind in the starting state is moved
/// against a baseline that holds the condition under which it binds (the
/// spec's list). For a keyed lever with planned keys, its active keys are
/// moved to each end, and a command changing a planned key is rejected with
/// `not_yet`. Every alpha lever still `planned` is one the owner deferred
/// (the spec's list), and a command at either end of its range is rejected
/// with `not_yet`.
///
/// *Restated 2026-10-25:* the criterion covered every alpha lever, planned
/// ones included; the owner deferred six of them beyond M4 (2026-10-24), and
/// six ends of active levers cannot bind in the starting state (the spec
/// names the condition of each).
///
/// *Restated 2026-10-26:* `world.fuel_price` is active with only its oil key
/// wired (owner, 2026-10-25); gas and coal are `planned_keys` until M12, so
/// moving every key of it was rejected `not_yet` and counted as a failure.
///
/// Readings: the alpha levers are the registry entries with `milestone =
/// "alpha"`, their status and planned keys read at run time; every active
/// key of a keyed lever is moved together, and each planned key is tried
/// alone at the same end on the starting world; the bottom and top are those of `end_value` (a choice's
/// first and last option; a schedule one row from 0 at the lowest or highest
/// rate). Given on 1 December 2021, taking effect at the earliest (1 January
/// for a law; for `money.qe` under its condition, 1 January) and run to 31
/// March 2022. The baseline is the run without the end, under the lever's
/// condition where it has one (`condition`, the spec's list; the condition's
/// orders are given before the end, so for a condition on the lever itself
/// the end, given later for the same day, wins). "Changes the state hash":
/// some day's hash differs from the baseline's; an end equal to the value
/// the baseline has in force is not expected to. "No invariant": every
/// day's checks of phase 13 and the ledger's invariants hold, in the
/// baselines too. All failures are gathered and reported together.
#[test]
fn ac_lev_14() {
    let last = date(2022, 3, 31);
    let start = lever_world(1000);
    let shared = {
        let mut end = start.clone();
        let hashes = run_checked_through(&mut end, last);
        Baseline {
            start: start.clone(),
            hashes,
            end,
        }
    };
    let alpha: Vec<LeverMeta> = start
        .registry()
        .levers
        .iter()
        .filter(|m| m.milestone == "alpha")
        .cloned()
        .collect();
    let (active, planned): (Vec<LeverMeta>, Vec<LeverMeta>) =
        alpha.into_iter().partition(is_active);
    assert!(
        active.len() >= 50,
        "the active alpha levers: {}",
        active.len()
    );
    for meta in &planned {
        assert!(
            DEFERRED.iter().any(|(id, _)| *id == meta.id.as_str()),
            "{} is planned but not among the levers the owner deferred",
            meta.id.as_str()
        );
    }
    let mut failures: Vec<String> = planned
        .iter()
        .flat_map(|m| try_planned(&start, m))
        .collect();
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get().min(4));
    let chunk = active.len().div_ceil(threads);
    failures.extend(std::thread::scope(|s| {
        let handles: Vec<_> = active
            .chunks(chunk)
            .map(|part| {
                let shared = &shared;
                s.spawn(move || {
                    part.iter()
                        .flat_map(|m| {
                            let own = match baseline_of(shared, m, last) {
                                Err(e) => return vec![e],
                                Ok(own) => own,
                            };
                            let base = own.as_ref().unwrap_or(shared);
                            [false, true]
                                .into_iter()
                                .flat_map(|top| try_end(base, m, top, last))
                                .collect()
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("a lever thread"))
            .collect::<Vec<_>>()
    }));
    assert!(
        failures.is_empty(),
        "{} failures over {} active alpha levers' ends and {} planned levers:\n{}",
        failures.len(),
        active.len() * 2,
        planned.len(),
        failures.join("\n")
    );
}

/// **AC-LEV-15** `[sim]` After a VAT rise, `PolicyChanged` is in the events
/// with old, new, given and effective, and the price "why" tree of a
/// standard-rated good in the month of effect has a term tagged
/// `tax.vat.rate_standard` with a non-zero contribution.
///
/// Readings: 19% to 21% given on 10 December 2021, in effect on 1 January
/// 2022; the good is `cars` (standard-rated); the month of effect is January
/// 2022 (month 1), its "why" tree the terms of `World::price_why`.
#[test]
fn ac_lev_15() {
    let mut w = lever_world(1000);
    step_to(&mut w, date(2021, 12, 10));
    let accepted = give(
        &mut w,
        set(LeverId::TaxVatRateStandard, None, LeverValue::Rate(2_100)),
    );
    run_checked_through(&mut w, date(2022, 1, 31));
    let changed: Vec<_> = w
        .events()
        .iter()
        .filter_map(|e| match e {
            PolicyEvent::Changed(c) if c.lever == LeverId::TaxVatRateStandard => Some(c),
            _ => None,
        })
        .collect();
    assert_eq!(
        changed.len(),
        1,
        "one PolicyChanged of the VAT rate: {changed:?}"
    );
    let c = changed[0];
    assert_eq!(c.old, LeverValue::Rate(1_900));
    assert_eq!(c.new, LeverValue::Rate(2_100));
    assert_eq!(c.given, date(2021, 12, 10));
    assert_eq!(c.effective, date(2022, 1, 1));
    assert_eq!(c.seq, accepted.seq);
    assert_eq!(c.key, None);

    let cars = industry(&w, "cars");
    let terms = w.price_why(cars, econ_types::Month(1));
    assert!(
        terms
            .iter()
            .any(|t| t.lever == Some(LeverId::TaxVatRateStandard)
                && t.contribution != 0.0
                && t.contribution.is_finite()),
        "a VAT term in the price of cars: {terms:?}"
    );
}

/// **AC-LEV-16** `[sim]` A change of `world.foreign_rate` is validated,
/// logged and replayed like a law command, and in a two-group
/// counterfactual (taxes, world) its effect is attributed to `world`.
///
/// Readings: the foreign rate goes from its start to 1% (100 basis points),
/// given on 10 December 2021; as a `world` setting it takes effect the next
/// day. It is logged with `seq` and `effective`, and the replay of the log
/// gives the same daily hashes to 28 February 2022. The counterfactual
/// (`attribute_groups`) forks the world on 10 December with the groups
/// taxes and world; the indicator is the exchange rate (lei per euro,
/// millionths) at the end of 28 February. "Attributed to world": the world
/// group's effect equals the whole effect of the command (the run with it
/// less the run without it), which is not zero, and the taxes group's
/// effect is zero.
#[cfg(feature = "trade")]
#[test]
fn ac_lev_16() {
    let last = date(2022, 2, 28);
    let mut w = lever_world(1000);
    let mut hashes: Vec<u64> = Vec::new();
    while w.date() < date(2021, 12, 10) {
        hashes.push(step(&mut w).state_hash);
    }
    let fork = w.clone();
    let accepted = give(
        &mut w,
        set(LeverId::WorldForeignRate, None, LeverValue::Rate(100)),
    );
    assert_eq!(accepted.effective, date(2021, 12, 11));
    assert!(accepted.warnings.is_empty());
    let log = w.command_log().to_vec();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].seq, Some(accepted.seq));
    assert_eq!(log[0].given, date(2021, 12, 10));
    assert_eq!(log[0].effective, Some(date(2021, 12, 11)));
    hashes.extend(run_checked_through(&mut w, last));
    let (_, replayed) = replay(1000, &log, last);
    assert_eq!(first_difference(&replayed, &hashes), None, "the replay");

    let rate = |w: &World| i64::from(w.foreign().exchange_rate_ppm);
    let without = {
        let mut f = fork.clone();
        run_checked_through(&mut f, last);
        rate(&f)
    };
    let whole = rate(&w) - without;
    assert_ne!(whole, 0, "the foreign rate moves the exchange rate");
    let effects = attribute_groups(&fork, &log, &[Group::Taxes, Group::World], last, rate)
        .unwrap_or_else(|r| panic!("the counterfactual runs: {:?} {}", r.code, r.message));
    let of = |g: Group| {
        effects
            .iter()
            .find(|(x, _)| *x == g)
            .map(|(_, e)| *e)
            .unwrap_or_else(|| panic!("an effect of {g:?}: {effects:?}"))
    };
    assert_eq!(of(Group::World), whole, "attributed to world");
    assert_eq!(of(Group::Taxes), 0, "nothing to taxes");
}
