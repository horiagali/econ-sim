//! `econ-cli` — headless runner.
//!
//! ```text
//! econ-cli sim [--ticks N] [--hashes] [--check-golden FILE] [--mutate tax-sign|consume-gross]
//! ```
//! Default output of `sim` is CSV: tick,Y,T,YD,C,G,H,GOV_DEFICIT,state_hash
//! (money in bani).

use std::process::ExitCode;

use econ_core::sim::{Mutation, SimModel, SimParams};

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  econ-cli sim [--ticks N] [--hashes] [--check-golden FILE] [--mutate tax-sign|consume-gross]\n  econ-cli bench-scale [--scales 1000,100,10] [--ticks N]\n  econ-cli bench-save [--scales 100,10]"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("sim") => run_sim(&args[1..]),
        Some("bench-scale") => bench_scale(&args[1..]),
        Some("bench-save") => bench_save(&args[1..]),
        _ => usage(),
    }
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
