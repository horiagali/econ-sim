//! Acceptance tests for AC-HIST-01 to AC-HIST-05 (spec: game/history-files,
//! draft of 2026-11-01), written by the test writer from the spec and its API
//! sketch before the code (ADR-0014); behind the feature `history`, against
//! the test writer's API skeleton `econ_io::history` (`write_year`,
//! `write_all`, `read_history`, `BlockInfo`, `History`, `Point`, and the
//! additions `write_history`, `IndicatorInfo`, `FORMAT_VERSION`,
//! `READABLE_VERSIONS`).
//!
//! "1:1000" is the Romania scenario at that scale, seed 42, under the law of
//! its folder (`law.json`: the 2021 law), every parameter at its spec
//! default. Month 0 is December 2021; month 12 December 2022.
//!
//! Readings shared by the tests:
//! - A history folder is the `dir` given: blocks `dir/<year>.arrow` (the
//!   `file` of each `BlockInfo`), and `dir/index.json`.
//! - The registry's history as points: for every indicator of the registry
//!   and every key `stats().keys` gives it, one point per monthly point
//!   (`month` = the series' first month + its position, `day` none) and,
//!   for a day indicator, one per daily point (`day` = the first day of the
//!   series' first month + its position, `month` the month that day is in).
//!   The breakdown is named by the registry's spelling (`region`,
//!   `age_band`, …), national `None`.
//! - The units and periods are the registry's spellings (`bani`, `ppm`, …;
//!   `month`, `day`).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use econ_core::groups::{Condition, Filter, GroupSummary};
use econ_core::stats::{Breakdown, Period, Unit, indicators};
use econ_core::world::{StartState, World, WorldParams};
use econ_io::commands::{ScenarioLaw, read_scenario_law};
use econ_io::history::{
    BlockInfo, FORMAT_VERSION, History, IndicatorInfo, Point, READABLE_VERSIONS, read_history,
    write_all, write_history, write_year,
};
use econ_io::read_start_state;
use econ_scenario::{BuildParams, Inputs, build, read_inputs, write};
use econ_types::{Bani, Calendar, Date, Day};

// --- the scenario and worlds ----------------------------------------------------

/// The seed of the scenario and of the worlds.
const SEED: u64 = 42;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixtures_dir() -> PathBuf {
    repo_root().join("python/pipeline/fixtures")
}

/// A folder under the tests' temporary directory, emptied and created.
fn fresh_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("econ-io-history")
        .join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("clean the test folder");
    }
    std::fs::create_dir_all(&dir).expect("create the test folder");
    dir
}

fn inputs() -> &'static Inputs {
    static I: OnceLock<Inputs> = OnceLock::new();
    I.get_or_init(|| read_inputs(&fixtures_dir()).expect("the fixtures read"))
}

/// The Romania scenario at 1:1000, written once.
fn scenario_dir() -> &'static Path {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let scenario = build(
            inputs(),
            &BuildParams {
                sample_scale: 1000,
                rng_seed: SEED,
            },
        )
        .expect("the fixtures build");
        let dir = fresh_dir("romania_1000").join("scenario");
        write(&scenario, inputs(), &dir).expect("the scenario writes");
        dir
    })
}

fn start_state() -> StartState {
    static S: OnceLock<StartState> = OnceLock::new();
    S.get_or_init(|| {
        read_start_state(scenario_dir(), &fixtures_dir().join("io_table2021_ro.json"))
            .expect("the scenario reads as a starting state")
    })
    .clone()
}

fn scenario_law() -> &'static ScenarioLaw {
    static L: OnceLock<ScenarioLaw> = OnceLock::new();
    L.get_or_init(|| {
        let registry = econ_core::levers::Registry::committed();
        match read_scenario_law(scenario_dir(), &registry) {
            Ok(law) => law,
            Err((line, r)) => panic!(
                "the scenario's law reads: line {line}: {:?} {}",
                r.code, r.message
            ),
        }
    })
}

/// A 1:1000 world under the scenario's law, with this seed of the draws.
fn world(seed: u64) -> World {
    let law = scenario_law();
    let p = WorldParams::spec_defaults(seed);
    World::with_law(start_state(), law.registry.clone(), law.law.clone(), p)
        .expect("the world builds under the scenario's law")
}

/// The groups the round trip watches from day 0.
fn watched_groups() -> Vec<(&'static str, Filter)> {
    vec![
        ("everyone", Filter::default()),
        (
            "young women in region 3",
            Filter {
                conditions: vec![
                    Condition::Sex(vec![econ_popgen::Sex::F]),
                    Condition::Age { from: 15, to: 30 },
                    Condition::Region(vec![3]),
                ],
            },
        ),
    ]
}

/// A world with the watched groups, run for `closes` closes, every day's
/// checks holding.
fn run(seed: u64, closes: usize) -> World {
    let mut w = world(seed);
    for (name, f) in watched_groups() {
        w.watch_group(name, f);
    }
    while w.months().len() < closes {
        let r = w.step(&[]).expect("a day runs");
        assert!(
            r.checks_ok,
            "day {}: checks failed: {:?}",
            r.day.0, r.failed
        );
    }
    w
}

// --- the registry's history as points ---------------------------------------------

fn breakdown_name(b: Breakdown) -> &'static str {
    match b {
        Breakdown::Region => "region",
        Breakdown::County => "county",
        Breakdown::Industry => "industry",
        Breakdown::Class => "class",
        Breakdown::Fifth => "fifth",
        Breakdown::AgeBand => "age_band",
        Breakdown::Sex => "sex",
        Breakdown::Edu => "edu",
        Breakdown::Activity => "activity",
        Breakdown::Level => "level",
        Breakdown::Flow => "flow",
        Breakdown::Line => "line",
        Breakdown::Good => "good",
    }
}

fn unit_name(u: Unit) -> &'static str {
    match u {
        Unit::Bani => "bani",
        Unit::RealBani => "real_bani",
        Unit::Ppm => "ppm",
        Unit::Persons => "persons",
        Unit::IndexPpm => "index_ppm",
        Unit::Milli => "milli",
        Unit::Count => "count",
    }
}

fn period_name(p: Period) -> &'static str {
    match p {
        Period::Month => "month",
        Period::Day => "day",
    }
}

fn start_date() -> Date {
    Date::new(2021, 12, 1).expect("a date")
}

/// The first day index of month `m`.
fn first_day(cal: Calendar, m: u32) -> u32 {
    (0..)
        .find(|&d| cal.month(Day(d)).0 == m)
        .expect("a day of the month")
}

/// Every point of the world's registry history, sorted.
fn points_of(w: &World) -> Vec<Point> {
    let cal = Calendar::new(start_date());
    let stats = w.stats();
    let mut out = Vec::new();
    for m in indicators() {
        for &k in stats.keys(m.id) {
            let s = stats.series(m.id, k).expect("a key's series");
            let breakdown = k.breakdown.map(|b| breakdown_name(b).to_string());
            let first = s.first_month.0;
            for (i, &v) in s.months.iter().enumerate() {
                out.push(Point {
                    indicator: m.name.to_string(),
                    breakdown: breakdown.clone(),
                    index: k.index,
                    month: first + u32::try_from(i).expect("a month"),
                    day: None,
                    value: v,
                });
            }
            if let Some(days) = s.days {
                let d0 = first_day(cal, first);
                for (i, &v) in days.iter().enumerate() {
                    let d = d0 + u32::try_from(i).expect("a day");
                    out.push(Point {
                        indicator: m.name.to_string(),
                        breakdown: breakdown.clone(),
                        index: k.index,
                        month: cal.month(Day(d)).0,
                        day: Some(d),
                        value: v,
                    });
                }
            }
        }
    }
    out.sort();
    out
}

/// Every watched summary of the world, as (name, month, summary), sorted by
/// name and month.
fn watched_of(w: &World) -> Vec<(String, u32, GroupSummary)> {
    let mut out = Vec::new();
    for (name, _) in watched_groups() {
        let s = w.watched(name).unwrap_or_else(|| panic!("{name}: watched"));
        for (t, g) in s.iter().enumerate() {
            out.push((
                name.to_string(),
                u32::try_from(t).expect("a month"),
                g.clone(),
            ));
        }
    }
    out.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    out
}

/// The sorted copy of a history's watched summaries.
fn sorted_watched(h: &History) -> Vec<(String, u32, GroupSummary)> {
    let mut v = h.watched.clone();
    v.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    v
}

// --- synthetic histories ----------------------------------------------------------

/// A small synthetic history of 25 months from December 2021 (2021, 2022,
/// 2023): a month indicator with a national value and two regions, a day
/// indicator with a national value, and one watched group. The indicator
/// names are not in the registry.
fn synthetic(sample_scale: u32, version: u32) -> History {
    let cal = Calendar::new(start_date());
    let indicators = vec![
        IndicatorInfo {
            name: "synthetic.households".to_string(),
            unit: "count".to_string(),
            period: "month".to_string(),
        },
        IndicatorInfo {
            name: "synthetic.rate".to_string(),
            unit: "ppm".to_string(),
            period: "day".to_string(),
        },
    ];
    let mut points = Vec::new();
    for t in 0..25u32 {
        for (b, i) in [(None, 0u32), (Some("region"), 0), (Some("region"), 1)] {
            points.push(Point {
                indicator: "synthetic.households".to_string(),
                breakdown: b.map(str::to_string),
                index: i,
                month: t,
                day: None,
                value: i64::from(t) * 1_000 - i64::from(i) * 7,
            });
        }
        points.push(Point {
            indicator: "synthetic.rate".to_string(),
            breakdown: None,
            index: 0,
            month: t,
            day: None,
            value: 17_500 + i64::from(t),
        });
    }
    let last = first_day(cal, 25);
    for d in 0..last {
        points.push(Point {
            indicator: "synthetic.rate".to_string(),
            breakdown: None,
            index: 0,
            month: cal.month(Day(d)).0,
            day: Some(d),
            value: 17_000 + i64::from(d % 97),
        });
    }
    let watched = (0..25u32)
        .map(|t| {
            (
                "a group".to_string(),
                t,
                GroupSummary {
                    persons: 1_000 + u64::from(t),
                    households: 400,
                    person_records: 40,
                    household_records: 16,
                    employed: 500,
                    unemployed: 25,
                    unemployment_rate_ppm: 47_619,
                    wages_gross: Bani(1_000_000 * i64::from(t)),
                    money: Bani(-5),
                    income_per_adult: Some(Bani(300_000)),
                    sol_milli: None,
                    approval_milli: Some(-1_500),
                    ..GroupSummary::default()
                },
            )
        })
        .collect();
    History {
        format_version: version,
        sample_scale,
        scenario: "synthetic-scenario-7f3a".to_string(),
        engine: "0123456789abcdef".to_string(),
        start: start_date(),
        indicators,
        points,
        watched,
    }
}

/// The block of `year` among `blocks`.
fn block(blocks: &[BlockInfo], year: i32) -> &BlockInfo {
    blocks
        .iter()
        .find(|b| b.year == year)
        .unwrap_or_else(|| panic!("no block of {year}: {blocks:?}"))
}

/// The error of reading `dir`, which must fail.
fn read_error(dir: &Path, what: &str) -> String {
    match read_history(dir) {
        Ok(_) => panic!("{what}: the history was read"),
        Err(e) => format!("{e:?}"),
    }
}

// --- AC-HIST-01 ---------------------------------------------------------------------

/// **AC-HIST-01** `[sim]` Round trip: at 1:1000, after 25 closes (two closed
/// calendar years, 2022 and 2023, and the December 2021 close), the blocks
/// written and read back give every point of the registry's history exactly
/// (every indicator, key, month and day point), and the watched groups'
/// summaries.
///
/// Readings: a 1:1000 world watching two groups from day 0 (everyone; women
/// of 15 to 29 in region 3), run for 25 closes. `write_all` writes the three
/// closed years 2021, 2022 and 2023 (one block each, named in the
/// `BlockInfo`s, each file present); `read_history` of the folder gives
/// `sample_scale` 1000, exactly the world's points (the file's readings; as
/// sorted lists, so every point is there once and nothing else) and exactly
/// the watched summaries (one per close and group). `write_year` of 2022
/// alone into another folder gives the same block bytes as `write_all`'s.
#[test]
fn ac_hist_01() {
    let w = run(SEED, 25);
    let dir = fresh_dir("round_trip");
    let blocks = write_all(&dir, &w).expect("the history writes");
    let years: Vec<i32> = blocks.iter().map(|b| b.year).collect();
    assert_eq!(years, vec![2021, 2022, 2023], "the closed years");
    for b in &blocks {
        assert!(dir.join(&b.file).is_file(), "{}: written", b.file);
        assert_eq!(b.sha256.len(), 64, "{}: a SHA-256 in hex", b.file);
    }
    assert!(dir.join("index.json").is_file(), "index.json written");
    let h = read_history(&dir).expect("the history reads back");
    assert_eq!(h.sample_scale, 1000);
    let mut got = h.points.clone();
    got.sort();
    let want = points_of(&w);
    assert!(!want.is_empty(), "the registry has a history");
    assert_eq!(got.len(), want.len(), "as many points");
    for (g, x) in got.iter().zip(&want) {
        assert_eq!(g, x, "a point read back");
    }
    assert_eq!(
        sorted_watched(&h),
        watched_of(&w),
        "the watched groups' summaries"
    );

    let one = fresh_dir("round_trip_2022");
    let b = write_year(&one, &w, 2022).expect("2022 writes");
    let all = block(&blocks, 2022);
    assert_eq!(
        b.sha256, all.sha256,
        "write_year and write_all agree on 2022"
    );
    assert_eq!(
        std::fs::read(one.join(&b.file)).expect("the block"),
        std::fs::read(dir.join(&all.file)).expect("the block"),
        "the same bytes"
    );
}

// --- AC-HIST-02 ---------------------------------------------------------------------

/// The bytes of every file of a folder, by name.
fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .expect("the folder")
        .map(|e| {
            let e = e.expect("an entry");
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).expect("a file"),
            )
        })
        .collect();
    v.sort();
    v
}

/// **AC-HIST-02** `[sim]` Determinism: two runs with the same seed write
/// byte-identical blocks and the same `index.json`; a different seed changes
/// at least one block's hash.
///
/// Readings: three 1:1000 runs of 13 closes (the closed years 2021 and 2022),
/// watching AC-HIST-01's groups: two with seed 42, one with seed 43 (the
/// world's draws; the same scenario). The two folders of seed 42 hold the
/// same files with the same bytes, `index.json` included, and the same
/// `BlockInfo`s; seed 43's blocks differ in at least one hash.
#[test]
fn ac_hist_02() {
    let a = fresh_dir("determinism_a");
    let b = fresh_dir("determinism_b");
    let c = fresh_dir("determinism_c");
    let ia = write_all(&a, &run(SEED, 13)).expect("writes");
    let ib = write_all(&b, &run(SEED, 13)).expect("writes");
    let ic = write_all(&c, &run(SEED + 1, 13)).expect("writes");
    assert_eq!(ia, ib, "the same blocks");
    let (fa, fb) = (files(&a), files(&b));
    assert!(fa.iter().any(|(n, _)| n == "index.json"), "index.json");
    assert_eq!(fa.len(), fb.len(), "the same files");
    for (x, y) in fa.iter().zip(&fb) {
        assert_eq!(x.0, y.0, "the same file names");
        assert!(x.1 == y.1, "{}: the bytes differ", x.0);
    }
    assert_eq!(ia.len(), ic.len(), "the same years");
    assert!(
        ia.iter().zip(&ic).any(|(x, y)| x.sha256 != y.sha256),
        "another seed changed no block"
    );
}

// --- AC-HIST-03 ---------------------------------------------------------------------

/// **AC-HIST-03** `[unit]` Integrity: a block whose bytes do not match its
/// hash in `index.json`, a block of another `sample_scale` than the others,
/// or a missing block is rejected with an error naming the file; a block
/// written by an older format version is read if the version is listed as
/// readable, else rejected.
///
/// Readings: synthetic histories (three years) written by `write_history`.
/// "Naming the file": the error's text (`Debug`) contains the block's file
/// name. A changed byte: one byte of the 2022 block flipped in place. Another
/// `sample_scale`: the 2023 block of the same history at 1:100 copied over
/// the 1:1000 one, and its hash in `index.json` replaced by its own (so
/// that only the scale differs). A missing block: the 2022 file deleted.
/// Versions: a history written with each version of `READABLE_VERSIONS`
/// (the current one among them) reads, with that version; one written with
/// a version not listed (0, older than any; and the one after the current)
/// is rejected.
#[test]
fn ac_hist_03() {
    // A sound folder reads.
    let dir = fresh_dir("integrity_sound");
    let blocks = write_history(&dir, &synthetic(1000, FORMAT_VERSION)).expect("writes");
    assert_eq!(blocks.len(), 3, "three years");
    read_history(&dir).expect("a sound history reads");

    // A changed byte.
    let dir = fresh_dir("integrity_byte");
    let blocks = write_history(&dir, &synthetic(1000, FORMAT_VERSION)).expect("writes");
    let b = block(&blocks, 2022);
    let path = dir.join(&b.file);
    let mut bytes = std::fs::read(&path).expect("the block");
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0x5a;
    std::fs::write(&path, bytes).expect("the block");
    let e = read_error(&dir, "a changed byte");
    assert!(
        e.contains(&b.file),
        "a changed byte: {e} does not name {}",
        b.file
    );

    // Another sample_scale.
    let dir = fresh_dir("integrity_scale");
    let blocks = write_history(&dir, &synthetic(1000, FORMAT_VERSION)).expect("writes");
    let other = fresh_dir("integrity_scale_100");
    let other_blocks = write_history(&other, &synthetic(100, FORMAT_VERSION)).expect("writes");
    let (mine, theirs) = (block(&blocks, 2023), block(&other_blocks, 2023));
    assert_eq!(mine.file, theirs.file, "the same file name");
    assert_ne!(mine.sha256, theirs.sha256, "the scale is in the block");
    std::fs::copy(other.join(&theirs.file), dir.join(&mine.file)).expect("copy the block");
    let index = dir.join("index.json");
    let text = std::fs::read_to_string(&index).expect("index.json");
    assert!(
        text.contains(&mine.sha256),
        "index.json holds the block's hash"
    );
    std::fs::write(&index, text.replace(&mine.sha256, &theirs.sha256)).expect("index.json");
    let e = read_error(&dir, "another sample_scale");
    assert!(
        e.contains(&mine.file),
        "another sample_scale: {e} does not name {}",
        mine.file
    );

    // A missing block.
    let dir = fresh_dir("integrity_missing");
    let blocks = write_history(&dir, &synthetic(1000, FORMAT_VERSION)).expect("writes");
    let b = block(&blocks, 2022);
    std::fs::remove_file(dir.join(&b.file)).expect("delete the block");
    let e = read_error(&dir, "a missing block");
    assert!(
        e.contains(&b.file),
        "a missing block: {e} does not name {}",
        b.file
    );

    // Format versions.
    assert!(
        READABLE_VERSIONS.contains(&FORMAT_VERSION),
        "the current version is readable"
    );
    for &v in READABLE_VERSIONS {
        let dir = fresh_dir(&format!("integrity_version_{v}"));
        write_history(&dir, &synthetic(1000, v)).expect("writes");
        let h = read_history(&dir).unwrap_or_else(|e| panic!("version {v} is readable: {e:?}"));
        assert_eq!(h.format_version, v, "read as version {v}");
    }
    for v in [0, FORMAT_VERSION + 1] {
        assert!(!READABLE_VERSIONS.contains(&v), "version {v} is not listed");
        let dir = fresh_dir(&format!("integrity_version_{v}"));
        write_history(&dir, &synthetic(1000, v)).expect("writes");
        let _ = read_error(&dir, &format!("version {v}"));
    }
}

// --- AC-HIST-04 ---------------------------------------------------------------------

/// The schema metadata of an Arrow IPC file, keys and values.
fn schema_metadata(path: &Path) -> Vec<(String, String)> {
    let f = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let r = arrow_ipc::reader::FileReader::try_new(f, None)
        .unwrap_or_else(|e| panic!("{}: an Arrow IPC file: {e}", path.display()));
    r.schema()
        .metadata()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// **AC-HIST-04** `[unit]` The schema metadata carries the format version,
/// `sample_scale`, the scenario id and each indicator's unit and period;
/// reading a block needs no registry (a reader given only the files gets the
/// units).
///
/// Readings: AC-HIST-03's synthetic history (1:1000, its own scenario id,
/// two indicators that are not in the registry, with their units and
/// periods). Each block is an Arrow IPC file whose schema metadata, keys and
/// values together, holds the format version, `sample_scale`, the scenario
/// id, each indicator's name, unit and period, and the block's year (the
/// encoding of the metadata is the implementation's). `read_history` of the
/// folder alone gives back the format version, `sample_scale`, scenario id,
/// engine hash, start date and the indicators' units and periods, and every
/// point and watched summary as written: the indicators are none of the
/// registry's, so nothing came from it.
#[test]
fn ac_hist_04() {
    let h = synthetic(1000, FORMAT_VERSION);
    for i in &h.indicators {
        assert!(
            indicators().iter().all(|m| m.name != i.name),
            "{}: not a registry indicator",
            i.name
        );
    }
    let dir = fresh_dir("metadata");
    let blocks = write_history(&dir, &h).expect("writes");
    for b in &blocks {
        let meta = schema_metadata(&dir.join(&b.file));
        let all: String = meta
            .iter()
            .map(|(k, v)| format!("{k}\u{1}{v}\u{1}"))
            .collect();
        for needle in [
            FORMAT_VERSION.to_string(),
            "1000".to_string(),
            h.scenario.clone(),
            b.year.to_string(),
        ] {
            assert!(
                all.contains(&needle),
                "{}: the metadata lacks {needle}: {meta:?}",
                b.file
            );
        }
        for i in &h.indicators {
            for needle in [&i.name, &i.unit, &i.period] {
                assert!(
                    all.contains(needle.as_str()),
                    "{}: the metadata lacks {needle}: {meta:?}",
                    b.file
                );
            }
        }
    }
    let back = read_history(&dir).expect("reads");
    assert_eq!(back.format_version, FORMAT_VERSION);
    assert_eq!(back.sample_scale, 1000);
    assert_eq!(back.scenario, h.scenario);
    assert_eq!(back.engine, h.engine);
    assert_eq!(back.start, h.start);
    let mut a = back.indicators.clone();
    let mut b = h.indicators.clone();
    a.sort();
    b.sort();
    assert_eq!(a, b, "the units and periods, from the files alone");
    let mut p = back.points.clone();
    let mut q = h.points.clone();
    p.sort();
    q.sort();
    assert_eq!(p, q, "the points");
    assert_eq!(
        sorted_watched(&back),
        sorted_watched(&h),
        "the watched summaries"
    );
}

// --- AC-HIST-05 ---------------------------------------------------------------------

/// A deterministic step of a sequence (no randomness in tests).
fn next(x: &mut u64) -> u64 {
    *x = x
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *x >> 33
}

/// A synthetic 50-year history at 1:1000 of the registry's shape: every
/// indicator with the keys a 1:1000 run has for it after one close, 600
/// monthly points per key and, for the day indicators, a point per day of
/// the 50 years; values a walk from the run's first point by up to ±0.5% a
/// step; one watched group with 600 summaries.
fn fifty_years() -> History {
    let mut w = world(SEED);
    while w.months().is_empty() {
        w.step(&[]).expect("a day runs");
    }
    let cal = Calendar::new(start_date());
    let days = first_day(cal, 600);
    let stats = w.stats();
    let mut points = Vec::new();
    let mut infos = Vec::new();
    let mut seed = 7u64;
    for m in indicators() {
        infos.push(IndicatorInfo {
            name: m.name.to_string(),
            unit: unit_name(m.unit).to_string(),
            period: period_name(m.period).to_string(),
        });
        for &k in stats.keys(m.id) {
            let start = stats.latest(m.id, k).unwrap_or(1_000_000);
            let breakdown = k.breakdown.map(|b| breakdown_name(b).to_string());
            let mut walk = |n: u32, daily: bool, points: &mut Vec<Point>| {
                let mut v = start;
                for i in 0..n {
                    let step = i64::try_from(next(&mut seed) % 1001).expect("small") - 500;
                    v += v / 100_000 * step;
                    points.push(Point {
                        indicator: m.name.to_string(),
                        breakdown: breakdown.clone(),
                        index: k.index,
                        month: if daily { cal.month(Day(i)).0 } else { i },
                        day: daily.then_some(i),
                        value: v,
                    });
                }
            };
            walk(600, false, &mut points);
            if m.period == Period::Day {
                walk(days, true, &mut points);
            }
        }
    }
    let watched = (0..600u32)
        .map(|t| {
            (
                "everyone".to_string(),
                t,
                GroupSummary {
                    persons: 19_000_000 + u64::from(t),
                    households: 7_000_000,
                    person_records: 19_000,
                    household_records: 7_000,
                    wages_gross: Bani(4_000_000_000_000 + i64::from(t)),
                    ..GroupSummary::default()
                },
            )
        })
        .collect();
    History {
        format_version: FORMAT_VERSION,
        sample_scale: 1000,
        scenario: "romania-2021".to_string(),
        engine: "0123456789abcdef".to_string(),
        start: start_date(),
        indicators: infos,
        points,
        watched,
    }
}

/// **AC-HIST-05** `[bench]` A 50-year history at 1:1000 (600 monthly frames
/// of every registry indicator with their breakdowns, and the day series; a
/// synthetic history of that shape, as long runs wait for M7's stability) is
/// at most 20 MB on disk and reads back in at most 2 seconds in a release
/// build.
///
/// Readings: the synthetic history of `fifty_years` (the registry's shape at
/// 1:1000, 50 calendar years from December 2021 and the day series), written
/// by `write_history`. "On disk" is the bytes of every file of the folder,
/// blocks and `index.json`; 20 MB is 20,000,000 bytes. The read is timed
/// three times, the fastest kept; it gives back every point. A time budget
/// means nothing in a debug build: the test runs in release builds only.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build budget: cargo test --release -p econ-io --features history --test acceptance history::"
)]
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
fn ac_hist_05() {
    let h = fifty_years();
    let dir = fresh_dir("fifty_years");
    let blocks = write_history(&dir, &h).expect("writes");
    assert!(blocks.len() >= 50, "a block per year: {}", blocks.len());
    let bytes: u64 = std::fs::read_dir(&dir)
        .expect("the folder")
        .map(|e| e.expect("an entry").metadata().expect("its size").len())
        .sum();
    assert!(
        bytes <= 20_000_000,
        "the 50-year history is {bytes} bytes on disk (more than 20 MB)"
    );
    let mut best = u128::MAX;
    let mut points = 0;
    for _ in 0..3 {
        let started = std::time::Instant::now();
        let back = read_history(&dir).expect("reads");
        best = best.min(started.elapsed().as_millis());
        points = back.points.len();
    }
    assert_eq!(points, h.points.len(), "every point read back");
    assert!(
        best <= 2_000,
        "the 50-year history read back in {best} ms (more than 2 s)"
    );
}
