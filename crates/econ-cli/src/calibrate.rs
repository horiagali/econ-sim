//! `econ-cli calibrate` (spec `game/calibration`, "The report"): the quiet
//! baseline run, its targets judged by `econ_core::calibration`, and
//! `report.json`, `report.md` and the history files written.
//!
//! A configuration is a build: this binary is the `core` configuration, or
//! `people` when built with the feature `calibration-people` (econ-core's
//! `home_quality`, `education` and `approval`). `--config both` needs two
//! builds and is refused (`just calibrate` runs both).

use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use econ_core::calibration::{
    Report, RunConfig, RunInfo, Status, Stopped, Target, evaluate, read_targets,
};
use econ_core::stats::{Key, StatsHistory, Unit, indicators};
use econ_core::world::{StartEdit, World, WorldParams};
use econ_types::Day;

/// The seed of the quiet baseline.
const SEED: u64 = 42;

/// The configuration this binary runs.
const BUILD: RunConfig = if cfg!(feature = "calibration-people") {
    RunConfig::People
} else {
    RunConfig::Core
};

/// The parsed arguments.
struct Args {
    targets: PathBuf,
    sample_scale: u32,
    years: u32,
    config: RunConfig,
    out: PathBuf,
    fixtures: PathBuf,
    io_table: Option<PathBuf>,
    fail_on_day: Option<u32>,
}

fn bad(what: &str) -> ExitCode {
    eprintln!("calibrate: {what}");
    eprintln!(
        "usage: econ-cli calibrate --targets FILE --sample-scale N --years Y --config core|people \
         --out DIR [--fixtures DIR] [--io-table FILE] [--fail-on-day D]"
    );
    ExitCode::from(2)
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut targets = None;
    let mut sample_scale = None;
    let mut years = None;
    let mut config = None;
    let mut out = None;
    let mut fixtures = PathBuf::from("python/pipeline/fixtures");
    let mut io_table = None;
    let mut fail_on_day = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--targets" => targets = Some(PathBuf::from(value()?)),
            "--sample-scale" => sample_scale = value()?.parse().ok(),
            "--years" => years = value()?.parse().ok().filter(|&y: &u32| y > 0),
            "--config" => {
                config = Some(match value()?.as_str() {
                    "core" => RunConfig::Core,
                    "people" => RunConfig::People,
                    "both" => {
                        return Err("--config both needs two builds: run `just calibrate`".into());
                    }
                    c => return Err(format!("unknown configuration {c}")),
                });
            }
            "--out" => out = Some(PathBuf::from(value()?)),
            "--fixtures" => fixtures = PathBuf::from(value()?),
            "--io-table" => io_table = Some(PathBuf::from(value()?)),
            "--fail-on-day" => fail_on_day = value()?.parse().ok(),
            _ => return Err(format!("unknown argument {a}")),
        }
    }
    Ok(Args {
        targets: targets.ok_or("--targets is needed")?,
        sample_scale: sample_scale.ok_or("--sample-scale N is needed")?,
        years: years.ok_or("--years Y (a positive number) is needed")?,
        config: config.ok_or("--config is needed")?,
        out: out.ok_or("--out is needed")?,
        fixtures,
        io_table,
        fail_on_day,
    })
}

/// Run the report.
pub(crate) fn run(args: &[String]) -> ExitCode {
    #[allow(clippy::disallowed_types, clippy::disallowed_methods)]
    let started = std::time::Instant::now();
    let a = match parse(args) {
        Ok(a) => a,
        Err(e) => return bad(&e),
    };
    if a.config != BUILD {
        return bad(&format!(
            "this build is the {BUILD:?} configuration, not {:?} (build with the feature \
             calibration-people for people)",
            a.config
        ));
    }
    let targets = match read_targets(&a.targets) {
        Ok(t) => t,
        Err(e) => return bad(&e.message),
    };
    let run_dir = a.out.join(config_name(a.config));
    if run_dir.exists()
        && let Err(e) = std::fs::remove_dir_all(&run_dir)
    {
        return bad(&format!("{}: {e}", run_dir.display()));
    }
    let mut world = match baseline(&a, &run_dir.join("scenario")) {
        Ok(w) => w,
        Err(e) => return bad(&e),
    };
    #[allow(clippy::disallowed_types, clippy::disallowed_methods)]
    let built_ms = started.elapsed().as_millis();
    let stopped = run_baseline(&mut world, a.years);
    #[allow(clippy::disallowed_types, clippy::disallowed_methods)]
    let run_ms = started.elapsed().as_millis() - built_ms;

    let info = RunInfo {
        config: a.config,
        sample_scale: a.sample_scale,
        years: a.years,
        stopped,
    };
    let report = evaluate(&targets, world.stats(), &info);
    let history_dir = run_dir.join("history");
    let history = catch_unwind(AssertUnwindSafe(|| {
        econ_io::history::write_all(&history_dir, &world)
    }));
    if !matches!(history, Ok(Ok(_))) {
        eprintln!("calibrate: the history files were not written");
    }

    #[allow(clippy::disallowed_types, clippy::disallowed_methods)]
    let total_ms = started.elapsed().as_millis();
    let json = serde_json::json!({
        "runs": [&report],
        "timing": { "build_ms": built_ms, "run_ms": run_ms, "total_ms": total_ms },
    });
    let md = markdown(&report, &targets, world.stats());
    let written = std::fs::create_dir_all(&a.out)
        .and_then(|()| {
            std::fs::write(
                a.out.join("report.json"),
                serde_json::to_string_pretty(&json).expect("a report serialises") + "\n",
            )
        })
        .and_then(|()| std::fs::write(a.out.join("report.md"), md));
    if let Err(e) = written {
        return bad(&format!("{}: {e}", a.out.display()));
    }
    let out = report.targets.iter().filter(|t| !t.in_band).count();
    eprintln!(
        "calibrate: {} targets, {out} out{}; {} ms",
        report.targets.len(),
        report
            .stopped
            .as_ref()
            .map_or(String::new(), |s| format!(", stopped in month {}", s.month)),
        total_ms
    );
    if report.all_in_band() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn config_name(c: RunConfig) -> &'static str {
    match c {
        RunConfig::Core => "core",
        RunConfig::People => "people",
    }
}

/// The quiet baseline's world: the scenario built from the fixtures at 1:N
/// into `folder`, seed 42, under the law of its folder, the spec's defaults.
fn baseline(a: &Args, folder: &Path) -> Result<World, String> {
    let inputs = econ_scenario::read_inputs(&a.fixtures)
        .map_err(|e| format!("the fixtures do not read: {e:?}"))?;
    let scenario = econ_scenario::build(
        &inputs,
        &econ_scenario::BuildParams {
            sample_scale: a.sample_scale,
            rng_seed: SEED,
        },
    )
    .map_err(|e| format!("the scenario does not build: {e:?}"))?;
    econ_scenario::write(&scenario, &inputs, folder)
        .map_err(|e| format!("the scenario does not write: {e:?}"))?;
    let io_table = a
        .io_table
        .clone()
        .unwrap_or_else(|| a.fixtures.join("io_table2021_ro.json"));
    let mut start = econ_io::read_start_state(folder, &io_table)
        .map_err(|e| format!("the scenario does not read: {e:?}"))?;
    if let Some(d) = a.fail_on_day {
        start.apply(StartEdit::FailChecksOn(Day(d)));
    }
    let law =
        econ_io::commands::read_scenario_law(folder, &econ_core::levers::Registry::committed())
            .map_err(|(line, r)| format!("law.json/world.json line {line}: {}", r.message))?;
    World::with_law(
        start,
        law.registry,
        law.law,
        WorldParams::spec_defaults(SEED),
    )
    .map_err(|e| format!("the world does not build: {e:?}"))
}

/// Step the world with no command to the close of month `years × 12 − 1`,
/// stopping at the first day whose checks fail, an error or a panic.
fn run_baseline(world: &mut World, years: u32) -> Option<Stopped> {
    let months = usize::try_from(years).expect("years") * 12;
    let stop = |world: &World, reason: String| {
        Some(Stopped {
            month: u32::try_from(world.months().len()).expect("a month"),
            reason,
        })
    };
    while world.months().len() < months {
        if let Err(e) = routine_indexation(world) {
            return stop(world, format!("the routine indexation was rejected: {e}"));
        }
        match catch_unwind(AssertUnwindSafe(|| world.step(&[]))) {
            Ok(Ok(r)) if r.checks_ok && r.failed.is_empty() => {}
            Ok(Ok(r)) => {
                return stop(
                    world,
                    format!("day {} fails its checks: {:?}", r.day.0, r.failed),
                );
            }
            Ok(Err(e)) => return stop(world, format!("a day does not run: {e:?}")),
            Err(p) => {
                let why = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
                    .unwrap_or_else(|| "a panic".into());
                return stop(world, format!("panic: {why}"));
            }
        }
    }
    None
}

/// The levers a routine budget indexes (spec `game/calibration`, "The
/// quiet baseline"): the ministries' budgets, public pay, pensions, the
/// benefits' amounts and the minimum wage. Tax thresholds and brackets are
/// not indexed (Romania does not index them); excise follows its own law.
const ROUTINE: [&str; 10] = [
    "spend.ministry_budget",
    "spend.public_pay",
    "transfer.pension.point_value",
    "transfer.pension.minimum",
    "transfer.unemployment.isr",
    "transfer.child.under2",
    "transfer.child.to18",
    "transfer.min_income.floor",
    "transfer.min_income.per_extra",
    "labour.minimum_wage",
];

/// The routine government of the quiet baseline (owner, 2026-11-01: the
/// law's amounts stay in lei, and the baseline re-votes them as a
/// government does each year, with no other change): on the first day of
/// each November after the first, every amount of [`ROUTINE`] in force is
/// raised by the latest close's twelve-month inflation, by ordinary orders
/// taking effect on 1 January. A fall in prices lowers nothing.
fn routine_indexation(world: &mut World) -> Result<(), String> {
    use econ_core::levers::{Command, LeverValue, Order};
    let date = world.date();
    if date.month != 11 || date.day != 1 || world.months().len() < 12 {
        return Ok(());
    }
    let inflation = i128::from(world.months().last().map_or(0, |m| m.inflation_ppm).max(0));
    if inflation == 0 {
        return Ok(());
    }
    let effective = econ_types::Date::new(date.year + 1, 1, 1).ok_or("1 January does not exist")?;
    let up = |x: i128| x + (x * inflation + 500_000) / 1_000_000;
    let mut orders = Vec::new();
    for name in ROUTINE {
        let Some(meta) = world.registry().find(name).cloned() else {
            continue;
        };
        let keys: Vec<Option<u32>> = if meta.key.is_some() {
            (0..meta.keys.len())
                .map(|k| u32::try_from(k).ok())
                .collect()
        } else {
            vec![None]
        };
        for key in keys {
            let value = match world.law().get(meta.id, key) {
                Some(LeverValue::Money(b)) => LeverValue::Money(econ_types::Bani(
                    i64::try_from(up(i128::from(b.get()))).map_err(|e| e.to_string())?,
                )),
                Some(LeverValue::Rate(r)) => {
                    LeverValue::Rate(i32::try_from(up(i128::from(*r))).map_err(|e| e.to_string())?)
                }
                _ => continue,
            };
            let value = match (meta.range, &value) {
                (Some((_, hi)), LeverValue::Money(b)) if b.get() > hi => {
                    LeverValue::Money(econ_types::Bani(hi))
                }
                (Some((_, hi)), LeverValue::Rate(r)) if i64::from(*r) > hi => {
                    LeverValue::Rate(i32::try_from(hi).unwrap_or(i32::MAX))
                }
                _ => value,
            };
            orders.push(Order::Set(Command {
                lever: meta.id,
                key,
                value,
                effective: Some(effective),
            }));
        }
    }
    for o in orders {
        world.give(o).map_err(|r| r.message)?;
    }
    Ok(())
}

/// `report.md`: the targets (out ones first), a line a year of the headline
/// indicators, and where the run stopped.
fn markdown(r: &Report, targets: &[Target], h: &StatsHistory) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "# Calibration report: {}, 1:{}, {} years\n",
        config_name(r.config),
        r.sample_scale,
        r.years
    );
    let n_in = r.targets.iter().filter(|t| t.in_band).count();
    let _ = writeln!(s, "{n_in} of {} targets in band.\n", r.targets.len());
    if let Some(st) = &r.stopped {
        let _ = writeln!(
            s,
            "**The run stopped in month {}:** {}\n",
            st.month, st.reason
        );
    }
    let _ = writeln!(
        s,
        "| Target | Indicator | Metric | Window | Value | Band | Verdict | Out by |"
    );
    let _ = writeln!(s, "|---|---|---|---|---|---|---|---|");
    let mut order: Vec<_> = r.targets.iter().collect();
    order.sort_by_key(|t| t.in_band);
    for t in order {
        let Some(def) = targets.iter().find(|d| d.id == t.id) else {
            continue;
        };
        let unit = unit_of(&def.indicator);
        let verdict = match t.status {
            Status::InBand => "in band",
            Status::Out => "**out**",
            Status::NotReached => "**not reached**",
        };
        let _ = writeln!(
            s,
            "| {} | `{}` | {} | {}–{} | {} | {} to {} | {} | {} |",
            t.id,
            def.indicator,
            metric_name(&def.metric),
            def.window[0],
            def.window[1],
            t.value
                .map_or_else(|| "—".into(), |v| show(v, metric_unit(&def.metric, unit))),
            show(t.band[0], metric_unit(&def.metric, unit)),
            show(t.band[1], metric_unit(&def.metric, unit)),
            verdict,
            t.how_far_out
                .filter(|&f| f > 0.0)
                .map_or_else(String::new, |f| format!("{f:.3}")),
        );
    }

    let _ = writeln!(s, "\n## A line a year (the December close)\n");
    let cols = [
        ("Unemployment", "labour.unemployment_rate"),
        ("Inflation", "prices.inflation_yoy"),
        ("Lei per euro", "fx.eur_lei"),
        ("Policy rate", "money.policy_rate"),
        ("Real growth", "gdp.growth_yoy"),
        ("Debt / GDP", "debt.gdp_ratio"),
        ("Population", "people.population"),
    ];
    let _ = write!(s, "| Year |");
    for (title, _) in cols {
        let _ = write!(s, " {title} |");
    }
    let _ = writeln!(s);
    let _ = writeln!(s, "|---|{}", "---|".repeat(cols.len()));
    let months = r.years * 12;
    for y in 0..r.years {
        let t = y * 12;
        if t >= months {
            break;
        }
        let _ = write!(s, "| {y} |");
        for (_, name) in cols {
            let cell = point(h, name, t).map_or_else(|| "—".into(), |v| show(v, unit_of(name)));
            let _ = write!(s, " {cell} |");
        }
        let _ = writeln!(s);
    }
    let _ = writeln!(
        s,
        "\nYear y is the close of month 12·y (December). {}",
        match &r.stopped {
            Some(st) => format!("The run stopped in month {}.", st.month),
            None => "The run did not stop.".into(),
        }
    );
    s
}

/// The national point of `name` at month `t`.
fn point(h: &StatsHistory, name: &str, t: u32) -> Option<i64> {
    let m = indicators().iter().find(|m| m.name == name)?;
    let s = h.series(m.id, Key::NATIONAL)?;
    let k = usize::try_from(t.checked_sub(s.first_month.0)?).ok()?;
    s.months.get(k).copied()
}

/// The indicator's unit; lei per euro (millionths) shown as an index.
fn unit_of(name: &str) -> Option<Unit> {
    if name == "fx.eur_lei" {
        return Some(Unit::IndexPpm);
    }
    indicators().iter().find(|m| m.name == name).map(|m| m.unit)
}

/// The unit of a metric's value: millionths for `ratio` and `explained`,
/// nothing for `reached`, else the indicator's (a `trend` a year).
fn metric_unit(metric: &econ_core::calibration::Metric, unit: Option<Unit>) -> Option<Unit> {
    use econ_core::calibration::Metric;
    match metric {
        Metric::Ratio(_) | Metric::Explained(_) => Some(Unit::Ppm),
        Metric::Reached => None,
        _ => unit,
    }
}

fn metric_name(metric: &econ_core::calibration::Metric) -> String {
    use econ_core::calibration::Metric;
    match metric {
        Metric::Mean => "mean".into(),
        Metric::At => "at".into(),
        Metric::Min => "min".into(),
        Metric::Max => "max".into(),
        Metric::Drift => "drift".into(),
        Metric::Trend => "trend".into(),
        Metric::Reached => "reached".into(),
        Metric::Ratio(x) => format!("ratio({x})"),
        Metric::Explained(xs) => format!("explained({})", xs.join(", ")),
    }
}

/// A value in its unit, readably and with integers only (so the file is the
/// same on every machine).
fn show(v: i64, unit: Option<Unit>) -> String {
    let fixed = |v: i64, scale: i64, digits: usize, suffix: &str| {
        let sign = if v < 0 { "-" } else { "" };
        let a = v.unsigned_abs();
        let scale = scale.unsigned_abs();
        let frac = a % scale;
        let frac = format!("{frac:0w$}", w = scale.to_string().len() - 1);
        format!("{sign}{}.{}{suffix}", a / scale, &frac[..digits])
    };
    if v.unsigned_abs() >= 1 << 60 {
        return if v < 0 { "−∞".into() } else { "∞".into() };
    }
    match unit {
        Some(Unit::Ppm) => fixed(v, 10_000, 2, "%"),
        Some(Unit::Bani | Unit::RealBani) => fixed(v, 100_000_000_000, 2, " bn lei"),
        Some(Unit::Persons) => fixed(v, 1_000_000, 3, " m"),
        Some(Unit::IndexPpm) => fixed(v, 1_000_000, 3, ""),
        _ => v.to_string(),
    }
}
