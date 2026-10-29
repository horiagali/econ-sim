//! `econ-cli` — headless runner.
//!
//! ```text
//! econ-cli sim [--ticks N] [--hashes] [--check-golden FILE] [--mutate tax-sign|consume-gross]
//! econ-cli pc [--ticks N] [--rate-tick N] [--mutate interest-income|portfolio-sign]
//! econ-cli rng-raw [--pattern entity|tick|grid] [--entities N] [--bytes N] [--seed S]
//! econ-cli synth-population --margins FILE --sample-scale N [--seed S] --out DIR
//!                           [--attributes FILE [--jobs FILE [--wages FILE]] [--pensions FILE]]
//!                           [--housing FILE [--wealth FILE]] [--incomes FILE]
//!                           [--employers FILE]
//! econ-cli sim-population --margins FILE --sample-scale N [--seed S] [--ticks N]
//! econ-cli build-scenario --sample-scale N [--seed S] [--fixtures DIR] --out DIR
//! ```
//! Default output of `sim` is CSV: tick,Y,T,YD,C,G,H,GOV_DEFICIT,state_hash
//! (money in bani). `pc` runs Godley & Lavoie's model PC (bills and a central
//! bank; the rate rises from 2.5% to 3.5% at `--rate-tick`, default 100) and
//! prints tick,Y,T,YD,C,G,V,BH,HH,BS,BCB,IH,ICB,state_hash. `rng-raw` writes raw `fast_u64` output to stdout for
//! external test batteries (PractRand: `econ-cli rng-raw | RNG_test stdin64`).
//! `synth-population` is the data pipeline's `synth_population` stage
//! (ADR-0012): census margins in, `households.arrow`, `persons.arrow` and
//! `fit_report.json` out; each further margin file runs one more stage and adds
//! its columns (education and activity, jobs, locality size and tenure, wages,
//! pensions, wealth, declared and undeclared income, industry and employer).
//! `--incomes` needs the wages and housing stages; `--employers` (the firm-unit
//! file) needs the jobs stage.
//! `sim-population` generates the same population and
//! runs the scale world on it, printing one CSV line per month.
//! `build-scenario` is the scenario builder (spec `game/scenario-builder`): it
//! exits 0 if the report is consistent, 1 if a check fails (the scenario is
//! still written), 2 for bad arguments or an unreadable input, 3 if the
//! opening ledger fails its invariants (nothing written).
//! `run-world` builds the scenario at one scale and runs the `World` (spec
//! `game/world-loop`) under the placeholder law with no command, printing one
//! CSV line per year; it exits 1 if a check of the day fails or a year takes
//! longer than `--budget-ms` (roadmap M2: the three scales and the long run).

use std::io::Write;
use std::process::ExitCode;

use econ_core::sim::{Mutation, SimModel, SimParams};
use econ_rng::{KeyedRng, Stream};

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  econ-cli sim [--ticks N] [--hashes] [--check-golden FILE] [--mutate tax-sign|consume-gross]\n  econ-cli bench-scale [--scales 1000,100,10] [--ticks N]\n  econ-cli bench-save [--scales 100,10]\n  econ-cli bench-days [--scales 1000,100,10] [--years N]\n  econ-cli rng-raw [--pattern entity|tick|grid] [--entities N] [--bytes N] [--seed S]
  econ-cli synth-population --margins FILE --sample-scale N [--seed S] --out DIR [--attributes FILE [--jobs FILE [--wages FILE]] [--pensions FILE]] [--housing FILE [--wealth FILE]] [--incomes FILE] [--employers FILE]
  econ-cli sim-population --margins FILE --sample-scale N [--seed S] [--ticks N]
  econ-cli build-scenario --sample-scale N [--seed S] [--fixtures DIR] --out DIR
  econ-cli run-world --sample-scale N [--years N] [--seed S] [--fixtures DIR] [--out DIR] [--budget-ms MS]"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("sim") => run_sim(&args[1..]),
        Some("pc") => run_pc(&args[1..]),
        Some("bench-scale") => bench_scale(&args[1..]),
        Some("bench-days") => bench_days(&args[1..]),
        Some("bench-save") => bench_save(&args[1..]),
        Some("rng-raw") => rng_raw(&args[1..]),
        Some("synth-population") => synth_population(&args[1..]),
        Some("sim-population") => sim_population(&args[1..]),
        Some("build-scenario") => build_scenario(&args[1..]),
        Some("run-world") => run_world(&args[1..]),
        #[cfg(feature = "levers")]
        Some("play") => play(&args[1..]),
        _ => usage(),
    }
}

/// Headless play (spec `game/lever-system`, "Headless play"):
/// `econ-cli play --scenario DIR --commands FILE.jsonl --until DATE [--seed S]
/// [--out DIR] [--series a,b,…] [--io-table FILE]`, behind the feature
/// `levers`. The whole commands file is validated before the first day; any
/// rejection stops with its line number (exit 2). Writes `series_monthly.arrow`
/// and `.csv`, `commands_applied.jsonl`, `events.jsonl` and `run.json`; exits
/// 1 if a check of a day fails. `--daily` (the fast daily series) comes with
/// the statistics registry (M6) and is refused until then.
#[cfg(feature = "levers")]
fn play(args: &[String]) -> ExitCode {
    use std::path::PathBuf;

    let mut scenario: Option<PathBuf> = None;
    let mut commands: Option<PathBuf> = None;
    let mut until: Option<econ_types::Date> = None;
    let mut seed: Option<u64> = None;
    let mut out = PathBuf::from("play_out");
    let mut series: Option<Vec<String>> = None;
    let mut io_table = PathBuf::from("python/pipeline/fixtures/io_table2021_ro.json");
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--scenario" => scenario = it.next().map(PathBuf::from),
            "--commands" => commands = it.next().map(PathBuf::from),
            "--until" => until = it.next().and_then(|d| parse_date(d)),
            "--seed" => match it.next().and_then(|v| v.parse().ok()) {
                Some(s) => seed = Some(s),
                None => return usage(),
            },
            "--out" => match it.next() {
                Some(d) => out = PathBuf::from(d),
                None => return usage(),
            },
            "--series" => {
                series = it
                    .next()
                    .map(|v| v.split(',').map(str::to_string).collect())
            }
            "--io-table" => match it.next() {
                Some(f) => io_table = PathBuf::from(f),
                None => return usage(),
            },
            "--daily" => {
                eprintln!("--daily: the daily series come with the statistics registry (M6)");
                return ExitCode::from(2);
            }
            _ => return usage(),
        }
    }
    let (Some(dir), Some(commands), Some(until)) = (scenario, commands, until) else {
        return usage();
    };
    let fail = |what: String| {
        eprintln!("{what}");
        ExitCode::from(2)
    };

    let registry = econ_core::levers::Registry::committed();
    let law = match econ_io::commands::read_scenario_law(&dir, &registry) {
        Ok(l) => l,
        Err((line, r)) => return fail(format!("law.json/world.json line {line}: {}", r.message)),
    };
    let start = match econ_io::read_start_state(&dir, &io_table) {
        Ok(s) => s,
        Err(e) => return fail(format!("{}: {e:?}", dir.display())),
    };
    let seed = seed.unwrap_or(start.rng_seed);
    let params = econ_core::world::WorldParams::spec_defaults(seed);
    let mut world = match econ_core::world::World::with_law(start, law.registry, law.law, params) {
        Ok(w) => w,
        Err(e) => return fail(format!("the world does not build: {e:?}")),
    };
    let orders = match econ_io::commands::read_commands(&commands, world.registry()) {
        Ok(o) => o,
        Err((line, r)) => {
            return fail(format!("{} line {line}: {}", commands.display(), r.message));
        }
    };

    // Run, giving each order on its day.
    let mut next = 0;
    let mut month_ends: Vec<(econ_types::Date, u64)> = Vec::new();
    let mut failed = false;
    while world.date() <= until {
        let today = world.date();
        while next < orders.len() && orders[next].given <= today {
            if let Err(r) = world.give(orders[next].order.clone()) {
                return fail(format!(
                    "{} order {}: {}",
                    commands.display(),
                    next + 1,
                    r.message
                ));
            }
            next += 1;
        }
        let report = match world.step(&[]) {
            Ok(r) => r,
            Err(e) => return fail(format!("{today:?}: {e:?}")),
        };
        if !report.failed.is_empty() {
            eprintln!("{}: checks failed: {:?}", iso_date(today), report.failed);
            failed = true;
        }
        if today.is_month_end() {
            month_ends.push((today, report.state_hash));
        }
    }

    if let Err(e) = write_play(&world, &dir, &out, seed, &month_ends, series.as_deref()) {
        return fail(format!("{}: {e}", out.display()));
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// `2022-03-01`.
#[cfg(feature = "levers")]
fn parse_date(s: &str) -> Option<econ_types::Date> {
    let mut p = s.split('-');
    let (y, m, d) = (
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
    );
    if p.next().is_some() {
        return None;
    }
    econ_types::Date::new(y, m, d)
}

/// `2022-03-01`.
#[cfg(feature = "levers")]
fn iso_date(d: econ_types::Date) -> String {
    econ_io::commands::iso(d)
}

/// The outputs of `play` (spec, "Headless play").
#[cfg(feature = "levers")]
fn write_play(
    world: &econ_core::world::World,
    scenario: &std::path::Path,
    out: &std::path::Path,
    seed: u64,
    month_ends: &[(econ_types::Date, u64)],
    series: Option<&[String]>,
) -> Result<(), String> {
    use econ_core::levers::PolicyEvent;
    use econ_io::commands::{iso, log_line_json, value_json};
    use econ_io::table::{Column, NamedColumn, ipc_table};

    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let write = |name: &str, bytes: &[u8]| {
        std::fs::write(out.join(name), bytes).map_err(|e| format!("{name}: {e}"))
    };

    // The monthly series the `World` reports (until the statistics registry, M6).
    let months = world.months();
    let i64s = |f: &dyn Fn(&econ_core::world::MonthStats) -> i64| {
        months.iter().map(f).collect::<Vec<i64>>()
    };
    let all: Vec<(&'static str, &'static str, Vec<i64>)> = vec![
        ("cpi", "ppm", i64s(&|m| i64::from(m.cpi_ppm))),
        (
            "inflation",
            "ppm a year",
            i64s(&|m| i64::from(m.inflation_ppm)),
        ),
        (
            "expected_inflation",
            "ppm a year",
            i64s(&|m| i64::from(m.expected_inflation_ppm)),
        ),
        (
            "employed",
            "persons",
            i64s(&|m| i64::try_from(m.employed).unwrap_or(i64::MAX)),
        ),
        (
            "unemployed",
            "persons",
            i64s(&|m| i64::try_from(m.unemployed).unwrap_or(i64::MAX)),
        ),
        ("wage_bill", "bani", i64s(&|m| m.wage_bill.get())),
        ("consumption", "bani", i64s(&|m| m.consumption.get())),
        ("gdp_production", "bani", i64s(&|m| m.gdp_production.get())),
        (
            "gdp_expenditure",
            "bani",
            i64s(&|m| m.gdp_expenditure.get()),
        ),
        ("gdp_income", "bani", i64s(&|m| m.gdp_income.get())),
        (
            "hardship_households",
            "households",
            i64s(&|m| i64::try_from(m.hardship_households).unwrap_or(i64::MAX)),
        ),
    ];
    if let Some(bad) = series
        .unwrap_or(&[])
        .iter()
        .find(|w| !all.iter().any(|(n, _, _)| n == w))
    {
        return Err(format!("no series `{bad}`"));
    }
    let chosen: Vec<&(&'static str, &'static str, Vec<i64>)> = all
        .iter()
        .filter(|(n, _, _)| series.is_none_or(|w| w.iter().any(|x| x == n)))
        .collect();
    let month_ids: Vec<u32> = months.iter().map(|m| m.month.0).collect();
    let mut columns = vec![NamedColumn {
        name: "month",
        values: Column::U32(month_ids.clone()),
        unit: Some("months from the start"),
    }];
    for (name, unit, values) in &chosen {
        columns.push(NamedColumn {
            name,
            values: Column::I64(values.clone()),
            unit: Some(unit),
        });
    }
    write(
        "series_monthly.arrow",
        &ipc_table(columns).map_err(|e| format!("{e:?}"))?,
    )?;
    let mut csv = String::from("month");
    for (name, unit, _) in &chosen {
        csv.push_str(&format!(",{name} ({unit})"));
    }
    csv.push('\n');
    for (i, m) in month_ids.iter().enumerate() {
        csv.push_str(&m.to_string());
        for (_, _, values) in &chosen {
            csv.push_str(&format!(",{}", values[i]));
        }
        csv.push('\n');
    }
    write("series_monthly.csv", csv.as_bytes())?;

    // The applied log and the events.
    let registry = world.registry();
    let mut log = String::new();
    for line in world.command_log() {
        log.push_str(&log_line_json(line, registry));
        log.push('\n');
    }
    write("commands_applied.jsonl", log.as_bytes())?;
    let text = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let key = |lever: econ_core::levers::LeverId, k: Option<u32>| match k {
        None => String::new(),
        Some(k) => {
            let meta = registry.get(lever);
            let name = usize::try_from(k)
                .ok()
                .and_then(|k| meta.keys.get(k))
                .cloned()
                .unwrap_or_else(|| k.to_string());
            format!(",\"key\":{}", text(&name))
        }
    };
    let mut events = String::new();
    for e in world.events() {
        let line = match e {
            PolicyEvent::Given {
                seq,
                lever,
                key: k,
                given,
                effective,
            } => format!(
                "{{\"event\":\"PolicyGiven\",\"seq\":{seq},\"lever\":{}{},\"given\":\"{}\",\"effective\":\"{}\"}}",
                text(lever.as_str()),
                key(*lever, *k),
                iso(*given),
                iso(*effective)
            ),
            PolicyEvent::Changed(c) => {
                let meta = registry.get(c.lever);
                format!(
                    "{{\"event\":\"PolicyChanged\",\"seq\":{},\"lever\":{}{},\"old\":{},\"new\":{},\"given\":\"{}\",\"effective\":\"{}\"}}",
                    c.seq,
                    text(c.lever.as_str()),
                    key(c.lever, c.key),
                    value_json(meta, &c.old),
                    value_json(meta, &c.new),
                    iso(c.given),
                    iso(c.effective)
                )
            }
            PolicyEvent::Warning {
                seq,
                given,
                warning,
            } => format!(
                "{{\"event\":\"PolicyWarning\",\"seq\":{seq},\"lever\":{},\"code\":{},\"given\":\"{}\",\"message\":{}}}",
                text(warning.lever.as_str()),
                text(&format!("{:?}", warning.code)),
                iso(*given),
                text(&warning.message)
            ),
            PolicyEvent::BudgetOverrun {
                date,
                line,
                overrun,
            } => format!(
                "{{\"event\":\"BudgetOverrun\",\"date\":\"{}\",\"line\":{},\"overrun\":{}}}",
                iso(*date),
                text(line),
                overrun.get()
            ),
        };
        events.push_str(&line);
        events.push('\n');
    }
    write("events.jsonl", events.as_bytes())?;

    // run.json.
    let scenario_json =
        std::fs::read(scenario.join("scenario.json")).map_err(|e| format!("scenario.json: {e}"))?;
    let run = serde_json::json!({
        "scenario_hash": econ_scenario::sha256_hex(&scenario_json),
        "seed": seed,
        "engine_version": env!("CARGO_PKG_VERSION"),
        "last_day": month_ends.last().map(|_| iso(world.date())),
        "month_ends": month_ends
            .iter()
            .map(|(d, h)| serde_json::json!({ "date": iso(*d), "state_hash": format!("{h:016x}") }))
            .collect::<Vec<_>>(),
    });
    let mut bytes = serde_json::to_vec_pretty(&run).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    write("run.json", &bytes)
}

/// The scenario builder (spec `game/scenario-builder`): the world on the
/// start date at one `sample_scale`, as a folder, with its consistency report.
fn build_scenario(args: &[String]) -> ExitCode {
    let mut sample_scale: Option<u32> = None;
    let mut seed: u64 = 42;
    let mut fixtures = String::from("python/pipeline/fixtures");
    let mut out: Option<&String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--sample-scale" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) if n > 0 => sample_scale = Some(n),
                _ => return usage(),
            },
            "--seed" => match it.next().and_then(|v| v.parse().ok()) {
                Some(s) => seed = s,
                None => return usage(),
            },
            "--fixtures" => match it.next() {
                Some(d) => fixtures.clone_from(d),
                None => return usage(),
            },
            "--out" => out = it.next(),
            _ => return usage(),
        }
    }
    let (Some(sample_scale), Some(out)) = (sample_scale, out) else {
        return usage();
    };
    let params = econ_scenario::BuildParams {
        sample_scale,
        rng_seed: seed,
    };
    match econ_scenario::run(
        std::path::Path::new(&fixtures),
        &params,
        std::path::Path::new(out),
    ) {
        Ok(report) => {
            for c in &report.checks {
                let verdict = match (c.ok, c.known_gap) {
                    (true, _) => "ok",
                    (false, Some(_)) => "known gap",
                    (false, None) => "FAILED",
                };
                println!("{:<40} {verdict}", c.name);
            }
            let consistent = report.consistent();
            println!(
                "1:{sample_scale} seed {seed}: {} {out}",
                if consistent {
                    "consistent, wrote"
                } else {
                    "NOT consistent, wrote"
                }
            );
            if consistent {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("build-scenario: {e:?}; nothing written");
            ExitCode::from(e.exit_status())
        }
    }
}

/// The pipeline's `synth_population` stage (spec `society/population-generator`):
/// generate the starting population from a margin file and write its tables.
/// `--sample-scale` has no default: it comes from the scenario (ADR-0003).
fn synth_population(args: &[String]) -> ExitCode {
    use econ_io::popgen::{
        Stages, read_attribute_margins, read_firm_units, read_housing_margins, read_income_margins,
        read_job_margins, read_margins, read_pension_margins, read_wage_margins,
        read_wealth_margins, write_population,
    };
    use econ_popgen::{
        AttrParams, EmployerParams, GenParams, HousingParams, IncomeParams, JobParams,
        PensionParams, WageParams, WealthParams, assign_attributes, assign_employers,
        assign_housing, assign_jobs, assign_other_incomes, assign_pensions, assign_wages,
        assign_wealth, generate,
    };
    /// Read and parse one margin file, or say why not.
    fn load<T, E: std::fmt::Debug>(
        path: &str,
        parse: impl FnOnce(&str) -> Result<T, E>,
    ) -> Result<T, String> {
        std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|text| parse(&text).map_err(|e| format!("{e:?}")))
            .map_err(|e| format!("cannot read margin file {path}: {e}"))
    }
    let mut margins: Option<&String> = None;
    let mut out: Option<&String> = None;
    let mut attributes: Option<&String> = None;
    let mut jobs: Option<&String> = None;
    let mut housing: Option<&String> = None;
    let mut wages: Option<&String> = None;
    let mut pensions: Option<&String> = None;
    let mut wealth: Option<&String> = None;
    let mut incomes: Option<&String> = None;
    let mut employers: Option<&String> = None;
    let mut sample_scale: Option<u32> = None;
    let mut seed: u64 = 42;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--margins" => margins = it.next(),
            "--out" => out = it.next(),
            "--attributes" => attributes = it.next(),
            "--jobs" => jobs = it.next(),
            "--housing" => housing = it.next(),
            "--wages" => wages = it.next(),
            "--pensions" => pensions = it.next(),
            "--wealth" => wealth = it.next(),
            "--incomes" => incomes = it.next(),
            "--employers" => employers = it.next(),
            "--sample-scale" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) => sample_scale = Some(n),
                None => return usage(),
            },
            "--seed" => match it.next().and_then(|v| v.parse().ok()) {
                Some(s) => seed = s,
                None => return usage(),
            },
            _ => return usage(),
        }
    }
    let (Some(margins), Some(out), Some(sample_scale)) = (margins, out, sample_scale) else {
        return usage();
    };
    if jobs.is_some() && attributes.is_none() {
        eprintln!("--jobs needs --attributes: jobs are given to the employed of that stage");
        return ExitCode::from(2);
    }
    if wages.is_some() && jobs.is_none() {
        eprintln!("--wages needs --jobs: wages are given to the employees of that stage");
        return ExitCode::from(2);
    }
    if pensions.is_some() && attributes.is_none() {
        eprintln!("--pensions needs --attributes: pensions are given to the retired of that stage");
        return ExitCode::from(2);
    }
    if wealth.is_some() && housing.is_none() {
        eprintln!("--wealth needs --housing: dwellings are given to the owners of that stage");
        return ExitCode::from(2);
    }
    if incomes.is_some() && (wages.is_none() || housing.is_none()) {
        eprintln!(
            "--incomes needs --wages and --housing: it splits wages and farms in rural households"
        );
        return ExitCode::from(2);
    }
    if employers.is_some() && jobs.is_none() {
        eprintln!("--employers needs --jobs: units are given to the employed of that stage");
        return ExitCode::from(2);
    }
    let fail = |e: String| {
        eprintln!("{e}");
        ExitCode::FAILURE
    };
    let file = match load(margins, read_margins) {
        Ok(f) => f,
        Err(e) => return fail(e),
    };
    let params = GenParams::new(sample_scale, seed);
    let pop = match generate(&file.margins, &params) {
        Ok(p) => p,
        Err(e) => return fail(format!("margins rejected: {e:?}")),
    };
    // The stages after the fit: each needs its own margin file.
    let attributes = match attributes.map(|path| {
        let ea = load(path, |text| {
            read_attribute_margins(text, &file.margins.counties)
        })?;
        let attrs = assign_attributes(&pop, &ea.margins, &AttrParams::new(seed))
            .map_err(|e| format!("education-and-activity margins rejected: {e:?}"))?;
        Ok((ea, attrs))
    }) {
        Some(Err(e)) => return fail(e),
        Some(Ok(done)) => Some(done),
        None => None,
    };
    let jobs = match (jobs, &attributes) {
        (Some(path), Some((ea, attrs))) => {
            let done = load(path, |text| read_job_margins(text, &ea.margins)).and_then(|file| {
                let jobs = assign_jobs(&pop, attrs, &file.margins, &JobParams::new(seed))
                    .map_err(|e| format!("jobs margins rejected: {e:?}"))?;
                Ok((file, jobs))
            });
            match done {
                Ok(done) => Some(done),
                Err(e) => return fail(e),
            }
        }
        _ => None,
    };
    let housing = match housing.map(|path| {
        let file = load(path, read_housing_margins)?;
        let housing = assign_housing(&pop, &file.margins, &HousingParams::new(seed))
            .map_err(|e| format!("housing margins rejected: {e:?}"))?;
        Ok((file, housing))
    }) {
        Some(Err(e)) => return fail(e),
        Some(Ok(done)) => Some(done),
        None => None,
    };
    let wages = match (wages, &attributes, &jobs) {
        (Some(path), Some((_, attrs)), Some((_, jobs))) => {
            let done = load(path, read_wage_margins).and_then(|file| {
                let wages = assign_wages(&pop, attrs, jobs, &file.margins, &WageParams::new(seed))
                    .map_err(|e| format!("earnings margins rejected: {e:?}"))?;
                Ok((file, wages))
            });
            match done {
                Ok(done) => Some(done),
                Err(e) => return fail(e),
            }
        }
        _ => None,
    };
    let pensions = match (pensions, &attributes) {
        (Some(path), Some((_, attrs))) => {
            let done = load(path, read_pension_margins).and_then(|file| {
                let pensions =
                    assign_pensions(&pop, attrs, &file.margins, &PensionParams::new(seed))
                        .map_err(|e| format!("pension margins rejected: {e:?}"))?;
                Ok((file, pensions))
            });
            match done {
                Ok(done) => Some(done),
                Err(e) => return fail(e),
            }
        }
        _ => None,
    };
    let wealth = match (wealth, &housing) {
        (Some(path), Some((_, housing))) => {
            let done = load(path, read_wealth_margins).and_then(|file| {
                let wealth = assign_wealth(
                    &pop,
                    housing,
                    wages.as_ref().map(|(_, w)| w),
                    pensions.as_ref().map(|(_, p)| p),
                    &file.margins,
                    &WealthParams::new(seed),
                )
                .map_err(|e| format!("wealth margins rejected: {e:?}"))?;
                Ok((file, wealth))
            });
            match done {
                Ok(done) => Some(done),
                Err(e) => return fail(e),
            }
        }
        _ => None,
    };
    let incomes = match (incomes, &attributes, &jobs, &housing, &wages) {
        (Some(path), Some((_, attrs)), Some((_, jobs)), Some((_, housing)), Some((_, wages))) => {
            let done = load(path, read_income_margins).and_then(|file| {
                let incomes = assign_other_incomes(
                    &pop,
                    attrs,
                    jobs,
                    housing,
                    wages,
                    &file.margins,
                    &IncomeParams::new(seed),
                )
                .map_err(|e| format!("income margins rejected: {e:?}"))?;
                Ok((file, incomes))
            });
            match done {
                Ok(done) => Some(done),
                Err(e) => return fail(e),
            }
        }
        _ => None,
    };
    let employers = match (employers, &jobs) {
        (Some(path), Some((_, jobs))) => {
            let done = load(path, read_firm_units).and_then(|file| {
                let employers = assign_employers(
                    &pop,
                    jobs,
                    wages.as_ref().map(|(_, w)| w),
                    &file.margins,
                    &EmployerParams::new(seed),
                )
                .map_err(|e| format!("firm units rejected: {e:?}"))?;
                Ok((file, employers))
            });
            match done {
                Ok(done) => Some(done),
                Err(e) => return fail(e),
            }
        }
        _ => None,
    };
    let stages = Stages {
        attributes: attributes.as_ref().map(|(f, a)| (a, &f.provenance)),
        jobs: jobs.as_ref().map(|(f, j)| (j, &f.provenance)),
        housing: housing.as_ref().map(|(f, h)| (h, &f.provenance)),
        wages: wages.as_ref().map(|(f, w)| (w, &f.provenance)),
        pensions: pensions.as_ref().map(|(f, p)| (p, &f.provenance)),
        wealth: wealth.as_ref().map(|(f, w)| (w, &f.provenance)),
        incomes: incomes.as_ref().map(|(f, i)| (i, &f.provenance)),
        employers: employers.as_ref().map(|(f, e)| (e, &f.provenance)),
        inactivity: None,
        region_of_county: attributes
            .as_ref()
            .map(|(f, _)| f.margins.region_of_county.as_slice()),
    };
    if let Err(e) = write_population(std::path::Path::new(out), &file, &params, &pop, &stages) {
        return fail(format!("cannot write to {out}: {e:?}"));
    }
    let r = &pop.report;
    println!(
        "1:{sample_scale} seed {seed}: {} households, {} persons, state hash {:016x}",
        pop.households.hh_weight.len(),
        pop.persons.household_id.len(),
        pop.state_hash()
    );
    println!(
        "fit: {} sweeps, {}, {} ppm left on checked cells, {} county cells without a record",
        r.sweeps,
        if r.converged {
            "converged"
        } else {
            "sweep limit reached"
        },
        r.max_error_ppm,
        r.unfitted_cells
    );
    let hashes = [
        (
            "education and activity",
            stages.attributes.map(|(a, _)| a.state_hash()),
        ),
        ("jobs", stages.jobs.map(|(j, _)| j.state_hash())),
        (
            "locality size and tenure",
            stages.housing.map(|(h, _)| h.state_hash()),
        ),
        ("wages", stages.wages.map(|(w, _)| w.state_hash())),
        ("pensions", stages.pensions.map(|(p, _)| p.state_hash())),
        ("wealth", stages.wealth.map(|(w, _)| w.state_hash())),
        ("incomes", stages.incomes.map(|(i, _)| i.state_hash())),
        ("employers", stages.employers.map(|(e, _)| e.state_hash())),
    ];
    for (stage, hash) in hashes {
        if let Some(hash) = hash {
            println!("{stage}: hash {hash:016x}");
        }
    }
    ExitCode::SUCCESS
}

/// Run the scale world on a starting population generated from a margin file.
/// Households, weights, counties and ages are the census's; jobs, wages and
/// deposits are still invented (see `ScaleWorld::from_population`).
/// CSV on stdout: tick,employed,unemployed,wage_bill,consumption,vat,state_hash
/// (persons are real persons, money in bani).
fn sim_population(args: &[String]) -> ExitCode {
    use econ_core::scale_spike::ScaleWorld;
    use econ_io::popgen::{read_margins, seed_population};
    use econ_popgen::{GenParams, generate};
    let mut margins: Option<&String> = None;
    let mut sample_scale: Option<u32> = None;
    let mut seed: u64 = 42;
    let mut ticks: u32 = 12;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--margins" => margins = it.next(),
            "--sample-scale" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) => sample_scale = Some(n),
                None => return usage(),
            },
            "--seed" => match it.next().and_then(|v| v.parse().ok()) {
                Some(s) => seed = s,
                None => return usage(),
            },
            "--ticks" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) => ticks = n,
                None => return usage(),
            },
            _ => return usage(),
        }
    }
    let (Some(margins), Some(sample_scale)) = (margins, sample_scale) else {
        return usage();
    };
    let file = match std::fs::read_to_string(margins)
        .map_err(|e| format!("{e}"))
        .and_then(|text| read_margins(&text).map_err(|e| format!("{e:?}")))
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot read margin file {margins}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let pop = match generate(&file.margins, &GenParams::new(sample_scale, seed)) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("margins rejected: {e:?}");
            return ExitCode::FAILURE;
        }
    };
    let mut world = match ScaleWorld::from_population(sample_scale, seed, &seed_population(&pop)) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("population rejected: {e:?}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "1:{sample_scale} seed {seed}: {} households, {} persons standing for {} residents",
        world.n_households(),
        world.n_persons(),
        world.real_persons()
    );
    println!("tick,employed,unemployed,wage_bill,consumption,vat,state_hash");
    for tick in 0..ticks {
        let r = world.step();
        if !(r.ledger_ok && r.clearing_ok) {
            eprintln!("tick {tick}: accounting invariant failed");
            return ExitCode::FAILURE;
        }
        println!(
            "{tick},{},{},{},{},{},{:016x}",
            r.employed,
            r.unemployed,
            r.wage_bill.get(),
            r.consumption.get(),
            r.vat.get(),
            world.state_hash()
        );
    }
    ExitCode::SUCCESS
}

/// Raw little-endian `fast_u64` output on stdout, until `--bytes` is reached
/// or the reader closes the pipe. The pattern picks which key varies:
/// `entity` (one tick, entity = 0, 1, 2, …), `tick` (one entity, tick = 0, 1, …)
/// or `grid` (the simulation's order: entities 0..N for each tick in turn; N is
/// `--entities`, e.g. the person count of the scenario).
fn rng_raw(args: &[String]) -> ExitCode {
    let mut pattern = "entity";
    let mut entities: u64 = 0;
    let mut bytes: Option<u64> = None;
    let mut seed: u64 = 42;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--pattern" => match it.next().map(String::as_str) {
                Some(p @ ("entity" | "tick" | "grid")) => pattern = p,
                _ => return usage(),
            },
            "--entities" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) => entities = n,
                None => return usage(),
            },
            "--bytes" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) => bytes = Some(n),
                None => return usage(),
            },
            "--seed" => match it.next().and_then(|v| v.parse().ok()) {
                Some(s) => seed = s,
                None => return usage(),
            },
            _ => return usage(),
        }
    }
    if pattern == "grid" && entities == 0 {
        return usage();
    }
    let rng = KeyedRng::new(seed);
    let words = bytes.map_or(u64::MAX, |b| b.div_ceil(8));
    let mut out = std::io::BufWriter::with_capacity(1 << 16, std::io::stdout().lock());
    for i in 0..words {
        let v = match pattern {
            "tick" => rng.fast_u64(Stream::Labour, i as u32, 0, 0),
            "grid" => rng.fast_u64(Stream::Labour, (i / entities) as u32, i % entities, 0),
            _ => rng.fast_u64(Stream::Labour, 1, i, 0),
        };
        if out.write_all(&v.to_le_bytes()).is_err() {
            break; // the reader closed the pipe
        }
    }
    let _ = out.flush();
    ExitCode::SUCCESS
}

/// Model PC (the follow-up of Spike 2) as CSV, for `python/reference/diff_sim.py`.
fn run_pc(args: &[String]) -> ExitCode {
    use econ_core::pc::{PcModel, PcMutation, PcParams};
    let mut ticks: u32 = 200;
    let mut rate_tick: u32 = 100;
    let mut mutation = PcMutation::None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--ticks" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) => ticks = n,
                None => return usage(),
            },
            "--rate-tick" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) => rate_tick = n,
                None => return usage(),
            },
            "--mutate" => {
                mutation = match it.next().map(String::as_str) {
                    Some("interest-income") => PcMutation::InterestOutOfIncome,
                    Some("portfolio-sign") => PcMutation::PortfolioSign,
                    _ => return usage(),
                }
            }
            _ => return usage(),
        }
    }
    let mut model = PcModel::new(PcParams::default()).with_mutation(mutation);
    let mut failures = 0usize;
    println!("tick,Y,T,YD,C,G,V,BH,HH,BS,BCB,IH,ICB,state_hash");
    for tick in 0..ticks {
        if tick == rate_tick {
            model.set_rate(0.035);
        }
        let r = model.step();
        if !r.invariant_errors.is_empty() {
            failures = failures.saturating_add(1);
            eprintln!(
                "tick {}: invariant failures: {:?}",
                r.tick, r.invariant_errors
            );
        }
        let vals: Vec<String> = r
            .aggregates
            .iter()
            .map(|(_, v)| v.get().to_string())
            .collect();
        println!("{},{},{:016x}", r.tick, vals.join(","), r.state_hash);
    }
    if failures > 0 {
        eprintln!("{failures} ticks had invariant failures");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run_sim(args: &[String]) -> ExitCode {
    let mut ticks: u32 = 200;
    let mut hashes_only = false;
    let mut golden: Option<String> = None;
    let mut mutation = Mutation::None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--ticks" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) => ticks = n,
                None => return usage(),
            },
            "--hashes" => hashes_only = true,
            "--check-golden" => match it.next() {
                Some(f) => golden = Some(f.clone()),
                None => return usage(),
            },
            "--mutate" => {
                mutation = match it.next().map(String::as_str) {
                    Some("tax-sign") => Mutation::TaxSign,
                    Some("consume-gross") => Mutation::ConsumeOnGrossIncome,
                    _ => return usage(),
                }
            }
            _ => return usage(),
        }
    }

    let mut model = SimModel::new(SimParams::default()).with_mutation(mutation);
    let mut lines = Vec::new();
    let mut hash_lines = Vec::new();
    let mut failures = 0usize;
    for _ in 0..ticks {
        let r = model.step(&[]);
        if !r.invariant_errors.is_empty() {
            failures = failures.saturating_add(1);
            eprintln!(
                "tick {}: invariant failures: {:?}",
                r.tick, r.invariant_errors
            );
        }
        let vals: Vec<String> = r
            .aggregates
            .iter()
            .map(|(_, v)| v.get().to_string())
            .collect();
        lines.push(format!(
            "{},{},{:016x}",
            r.tick,
            vals.join(","),
            r.state_hash
        ));
        hash_lines.push(format!("{} {:016x}", r.tick, r.state_hash));
    }

    if let Some(path) = golden {
        let expected = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("cannot read golden file {path}: {e}");
                return ExitCode::FAILURE;
            }
        };
        let expected: Vec<&str> = expected
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        let got: Vec<&str> = hash_lines.iter().map(String::as_str).collect();
        if expected != got {
            let first = expected
                .iter()
                .zip(&got)
                .position(|(a, b)| a != b)
                .unwrap_or(expected.len().min(got.len()));
            eprintln!(
                "GOLDEN MISMATCH at line {first}: expected {:?}, got {:?}.\n\
                 If the change is intended, record the economic reason in CHANGELOG-sim.md \
                 and ask the owner to regenerate the golden file.",
                expected.get(first),
                got.get(first)
            );
            return ExitCode::FAILURE;
        }
        println!("golden OK ({} ticks)", got.len());
    } else if hashes_only {
        for l in &hash_lines {
            println!("{l}");
        }
    } else {
        println!("tick,Y,T,YD,C,G,H,GOV_DEFICIT,state_hash");
        for l in &lines {
            println!("{l}");
        }
    }
    if failures > 0 {
        eprintln!("{failures} ticks had invariant failures");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Spike 4: time a monthly tick at several population scales.
/// Wall-clock timing is allowed here: it never reaches simulation state.
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
fn bench_scale(args: &[String]) -> ExitCode {
    use econ_core::scale_spike::ScaleWorld;
    use std::time::Instant;
    let mut scales: Vec<u32> = vec![1000, 100, 10];
    let mut ticks: u32 = 12;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--scales" => match it.next() {
                Some(v) => scales = v.split(',').filter_map(|x| x.parse().ok()).collect(),
                None => return usage(),
            },
            "--ticks" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) => ticks = n,
                None => return usage(),
            },
            _ => return usage(),
        }
    }
    println!(
        "scale,persons,households,generate_ms,ms_per_tick,projected_600_ticks_s,employed,wage_bill_lei"
    );
    for s in scales {
        let t0 = Instant::now();
        let mut w = ScaleWorld::generate(s, 42);
        let gen_ms = t0.elapsed().as_secs_f64() * 1e3;
        let t1 = Instant::now();
        let mut last = None;
        for _ in 0..ticks {
            last = Some(w.step());
        }
        let per_tick = t1.elapsed().as_secs_f64() * 1e3 / f64::from(ticks.max(1));
        let r = last.expect("at least one tick");
        if !r.clearing_ok {
            eprintln!("clearing invariant failed at 1:{s}");
            return ExitCode::FAILURE;
        }
        println!(
            "1:{s},{},{},{gen_ms:.0},{per_tick:.1},{:.1},{},{}",
            w.n_persons(),
            w.n_households(),
            per_tick * 600.0 / 1e3,
            r.employed,
            r.wage_bill.get() / 100
        );
    }
    ExitCode::SUCCESS
}

/// Daily-tick spike (ADR-0017): what a simulated year costs when the clock
/// advances by the day and each part of the scale world runs at its own
/// period, against the same world stepped by the month.
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
fn bench_days(args: &[String]) -> ExitCode {
    use econ_core::scale_spike::ScaleWorld;
    use econ_types::{Calendar, Date, Day};
    use std::time::Instant;
    let mut scales: Vec<u32> = vec![1000, 100, 10];
    let mut years: u32 = 3;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--scales" => match it.next() {
                Some(v) => scales = v.split(',').filter_map(|x| x.parse().ok()).collect(),
                None => return usage(),
            },
            "--years" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) if n > 0 => years = n,
                _ => return usage(),
            },
            _ => return usage(),
        }
    }
    let calendar = Calendar::new(Date::new(2021, 12, 1).expect("a real date"));
    println!(
        "scale,persons,monthly_ms_per_year,daily_ms_per_year,ms_ordinary_day,ms_month_end_day,projected_50_years_daily_s,last_year_wage_bill_daily_over_monthly"
    );
    for s in scales {
        let mut monthly = ScaleWorld::generate(s, 42);
        let t0 = Instant::now();
        let mut monthly_wages = 0i64;
        for month in 0..years * 12 {
            let r = monthly.step();
            if month >= (years - 1) * 12 {
                monthly_wages += r.wage_bill.get();
            }
        }
        let monthly_ms = t0.elapsed().as_secs_f64() * 1e3 / f64::from(years);

        let mut daily = ScaleWorld::generate(s, 42);
        let mut schedule = daily.day_schedule();
        let (mut months, mut day) = (0, Day(0));
        let (mut ordinary_s, mut ordinary_n, mut closing_s, mut closing_n) = (0.0, 0u32, 0.0, 0u32);
        let mut daily_wages = 0i64;
        let t1 = Instant::now();
        while months < years * 12 {
            let t = Instant::now();
            let r = daily.step_day(&mut schedule, calendar.date(day));
            let took = t.elapsed().as_secs_f64();
            if !r.ledger_ok {
                eprintln!("ledger check failed on day {} at 1:{s}", day.0);
                return ExitCode::FAILURE;
            }
            if let Some(close) = r.month {
                if months >= (years - 1) * 12 {
                    daily_wages += close.wage_bill.get();
                }
                months += 1;
                closing_s += took;
                closing_n += 1;
            } else {
                ordinary_s += took;
                ordinary_n += 1;
            }
            day = day.next();
        }
        let daily_ms = t1.elapsed().as_secs_f64() * 1e3 / f64::from(years);
        #[allow(clippy::cast_precision_loss)]
        let ratio = daily_wages as f64 / monthly_wages as f64;
        println!(
            "1:{s},{},{monthly_ms:.1},{daily_ms:.1},{:.3},{:.3},{:.1},{ratio:.3}",
            daily.n_persons(),
            ordinary_s * 1e3 / f64::from(ordinary_n.max(1)),
            closing_s * 1e3 / f64::from(closing_n.max(1)),
            daily_ms * 50.0 / 1e3,
        );
    }
    ExitCode::SUCCESS
}

/// Spike 6: save size and save/load/verify time at several scales.
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
fn bench_save(args: &[String]) -> ExitCode {
    use econ_core::scale_spike::ScaleWorld;
    use econ_io::{LogEntry, LoggedCommand, load_from_bytes, run_with_log, save_to_bytes};
    use std::time::Instant;
    let mut scales: Vec<u32> = vec![100, 10];
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--scales" => match it.next() {
                Some(v) => scales = v.split(',').filter_map(|x| x.parse().ok()).collect(),
                None => return usage(),
            },
            _ => return usage(),
        }
    }
    let log = vec![LogEntry {
        tick: 3,
        cmd: LoggedCommand::SetSeparationRate(0.02),
    }];
    println!("scale,persons,save_mb,save_ms,load_and_verify_ms");
    for s in scales {
        let mut w = ScaleWorld::generate(s, 42);
        run_with_log(&mut w, &log, 6);
        let t0 = Instant::now();
        let bytes = match save_to_bytes(&w, &log) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("save failed: {e:?}");
                return ExitCode::FAILURE;
            }
        };
        let save_ms = t0.elapsed().as_secs_f64() * 1e3;
        let t1 = Instant::now();
        if let Err(e) = load_from_bytes(&bytes) {
            eprintln!("load failed: {e:?}");
            return ExitCode::FAILURE;
        }
        let load_ms = t1.elapsed().as_secs_f64() * 1e3;
        #[allow(clippy::cast_precision_loss)]
        let mb = bytes.len() as f64 / 1e6;
        println!("1:{s},{},{mb:.1},{save_ms:.0},{load_ms:.0}", w.n_persons());
    }
    ExitCode::SUCCESS
}

/// The `World` on the Romania scenario at one scale, for years, with no
/// command: one CSV line per year (wall time, the December figures).
/// Wall-clock timing is allowed here: it never reaches simulation state.
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
fn run_world(args: &[String]) -> ExitCode {
    use econ_core::world::{Law, World, WorldParams};
    let mut sample_scale: Option<u32> = None;
    let mut years: u32 = 1;
    let mut seed: u64 = 42;
    let mut fixtures = String::from("python/pipeline/fixtures");
    let mut out: Option<String> = None;
    let mut budget_ms: Option<u128> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--sample-scale" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) if n > 0 => sample_scale = Some(n),
                _ => return usage(),
            },
            "--years" => match it.next().and_then(|v| v.parse().ok()) {
                Some(n) if n > 0 => years = n,
                _ => return usage(),
            },
            "--seed" => match it.next().and_then(|v| v.parse().ok()) {
                Some(s) => seed = s,
                None => return usage(),
            },
            "--fixtures" => match it.next() {
                Some(d) => fixtures.clone_from(d),
                None => return usage(),
            },
            "--out" => match it.next() {
                Some(d) => out = Some(d.clone()),
                None => return usage(),
            },
            "--budget-ms" => match it.next().and_then(|v| v.parse().ok()) {
                Some(ms) => budget_ms = Some(ms),
                None => return usage(),
            },
            _ => return usage(),
        }
    }
    let Some(sample_scale) = sample_scale else {
        return usage();
    };
    let fixtures = std::path::Path::new(&fixtures);
    let out = out.map_or_else(
        || std::env::temp_dir().join(format!("econ-run-world-{sample_scale}")),
        std::path::PathBuf::from,
    );
    let started = std::time::Instant::now();
    let built = econ_scenario::read_inputs(fixtures).and_then(|inputs| {
        let params = econ_scenario::BuildParams {
            sample_scale,
            rng_seed: seed,
        };
        let scenario = econ_scenario::build(&inputs, &params)?;
        if out.exists() {
            std::fs::remove_dir_all(&out).map_err(|e| econ_scenario::BuildError::Input {
                file: out.display().to_string(),
                why: e.to_string(),
            })?;
        }
        econ_scenario::write(&scenario, &inputs, &out)
    });
    if let Err(e) = built {
        eprintln!("run-world: the scenario does not build: {e:?}");
        return ExitCode::from(2);
    }
    let start = match econ_io::read_start_state(&out, &fixtures.join("io_table2021_ro.json")) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("run-world: the scenario does not read: {e:?}");
            return ExitCode::from(2);
        }
    };
    let mut world = match World::new(
        start,
        Law::placeholder_2021(),
        WorldParams::spec_defaults(seed),
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("run-world: the world does not build: {e:?}");
            return ExitCode::from(2);
        }
    };
    eprintln!(
        "run-world: 1:{sample_scale} built in {} ms",
        started.elapsed().as_millis()
    );
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(
        stdout,
        "year,ms,unemployment_pct,inflation_pct,wage_share_pct,consumption_share_pct"
    );
    let mut failed = false;
    for year in 1..=years {
        let t0 = std::time::Instant::now();
        let mut closed = world.months().len();
        while world.months().len() < closed + 12 {
            match world.step(&[]) {
                Ok(r) if r.checks_ok => {}
                Ok(r) => {
                    eprintln!(
                        "run-world: day {} fails its checks: {:?}",
                        r.day.0, r.failed
                    );
                    return ExitCode::from(1);
                }
                Err(e) => {
                    eprintln!("run-world: day {} does not run: {e:?}", world.day().0);
                    return ExitCode::from(1);
                }
            }
        }
        closed += 12;
        let ms = t0.elapsed().as_millis();
        let m = &world.months()[closed - 1];
        let pct = |a: i128, b: i128| {
            if b == 0 {
                0.0
            } else {
                a as f64 * 100.0 / b as f64
            }
        };
        let gdp = i128::from(m.gdp_production.get());
        let _ = writeln!(
            stdout,
            "{year},{ms},{:.2},{:.2},{:.2},{:.2}",
            pct(
                i128::from(m.unemployed),
                i128::from(m.employed) + i128::from(m.unemployed)
            ),
            f64::from(m.inflation_ppm) / 10_000.0,
            pct(i128::from(m.wage_bill.get()), gdp),
            pct(i128::from(m.consumption.get()), gdp),
        );
        if budget_ms.is_some_and(|b| ms > b) {
            eprintln!("run-world: year {year} took {ms} ms, over the budget");
            failed = true;
        }
    }
    eprintln!(
        "run-world: {years} years in {} s",
        started.elapsed().as_secs()
    );
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
