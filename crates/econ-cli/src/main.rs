//! `econ-cli` — headless runner.
//!
//! ```text
//! econ-cli sim [--ticks N] [--hashes] [--check-golden FILE] [--mutate tax-sign|consume-gross]
//! econ-cli rng-raw [--pattern entity|tick|grid] [--entities N] [--bytes N] [--seed S]
//! econ-cli synth-population --margins FILE --sample-scale N [--seed S] --out DIR
//!                           [--attributes FILE [--jobs FILE]] [--housing FILE]
//! econ-cli sim-population --margins FILE --sample-scale N [--seed S] [--ticks N]
//! ```
//! Default output of `sim` is CSV: tick,Y,T,YD,C,G,H,GOV_DEFICIT,state_hash
//! (money in bani). `rng-raw` writes raw `fast_u64` output to stdout for
//! external test batteries (PractRand: `econ-cli rng-raw | RNG_test stdin64`).
//! `synth-population` is the data pipeline's `synth_population` stage
//! (ADR-0012): census margins in, `households.arrow`, `persons.arrow` and
//! `fit_report.json` out; each further margin file runs one more stage and adds
//! its columns (education and activity, jobs, locality size and tenure).
//! `sim-population` generates the same population and
//! runs the scale world on it, printing one CSV line per month.

use std::io::Write;
use std::process::ExitCode;

use econ_core::sim::{Mutation, SimModel, SimParams};
use econ_rng::{KeyedRng, Stream};

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  econ-cli sim [--ticks N] [--hashes] [--check-golden FILE] [--mutate tax-sign|consume-gross]\n  econ-cli bench-scale [--scales 1000,100,10] [--ticks N]\n  econ-cli bench-save [--scales 100,10]\n  econ-cli rng-raw [--pattern entity|tick|grid] [--entities N] [--bytes N] [--seed S]
  econ-cli synth-population --margins FILE --sample-scale N [--seed S] --out DIR [--attributes FILE [--jobs FILE]] [--housing FILE]
  econ-cli sim-population --margins FILE --sample-scale N [--seed S] [--ticks N]"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("sim") => run_sim(&args[1..]),
        Some("bench-scale") => bench_scale(&args[1..]),
        Some("bench-save") => bench_save(&args[1..]),
        Some("rng-raw") => rng_raw(&args[1..]),
        Some("synth-population") => synth_population(&args[1..]),
        Some("sim-population") => sim_population(&args[1..]),
        _ => usage(),
    }
}

/// The pipeline's `synth_population` stage (spec `society/population-generator`):
/// generate the starting population from a margin file and write its tables.
/// `--sample-scale` has no default: it comes from the scenario (ADR-0003).
fn synth_population(args: &[String]) -> ExitCode {
    use econ_io::popgen::{
        Stages, read_attribute_margins, read_housing_margins, read_job_margins, read_margins,
        write_population,
    };
    use econ_popgen::{
        AttrParams, GenParams, HousingParams, JobParams, assign_attributes, assign_housing,
        assign_jobs, generate,
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
    let stages = Stages {
        attributes: attributes.as_ref().map(|(f, a)| (a, &f.provenance)),
        jobs: jobs.as_ref().map(|(f, j)| (j, &f.provenance)),
        housing: housing.as_ref().map(|(f, h)| (h, &f.provenance)),
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
