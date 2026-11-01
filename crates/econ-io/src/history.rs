//! History files (spec `game/history-files`, roadmap M6, third slice): the
//! statistics registry's history and the watched groups on disk, as one
//! immutable Arrow IPC block (zstd-compressed) per closed calendar year,
//! `history/<year>.arrow`, and an `index.json` (year → file name, SHA-256,
//! rows) (ADR-0011). Behind the feature `history`.
//!
//! The API is the spec's sketch, written with the acceptance tests before the
//! code (ADR-0014).
//!
//! The encoding: one table per block (the registry's points and the watched
//! groups' fields as points named `group.<name>.<field>`), rows sorted by the
//! indicators' order, then breakdown, index, month and day; the metadata in
//! one schema key, `econ_sim_history`, a JSON object (one key, so the bytes do
//! not depend on a map's order). The same history writes the same bytes.
//!
//! The test writer's additions to the sketch, for the unit and bench
//! criteria (AC-HIST-03 to 05), which need histories that no run gives (other
//! format versions and scales, 50 years): a [`History`] carries its
//! metadata (format version, scenario id, engine hash, start date, each
//! indicator's unit and period), and [`write_history`] writes any `History`
//! as blocks. [`FORMAT_VERSION`] is the version written, and
//! [`READABLE_VERSIONS`] the versions [`read_history`] accepts.
//!
//! Choices the sketch left open, stated by the test writer:
//! - `dir` is the history folder itself: blocks are `dir/<year>.arrow` and
//!   the index `dir/index.json`.
//! - A point's year is the calendar year of its month (month 0 is the start
//!   date's month).
//! - Units and periods are the registry file's spellings (`bani`, `ppm`, …;
//!   `month`, `day`), breakdowns too (`region`, `age_band`, …).
//! - Every error names the file it is about in its message.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use arrow_array::{Array, ArrayRef, Int64Array, RecordBatch, StringArray, UInt32Array};
use arrow_schema::{DataType, Field, Schema};
use econ_core::groups::GroupSummary;
use econ_core::stats::indicators;
use econ_core::world::World;
use econ_types::{Bani, Calendar, Date, Day};
use serde_json::{Value, json};

use crate::{SaveError, ipc_bytes, read_batch};

/// The format version this engine writes.
pub const FORMAT_VERSION: u32 = 1;
/// The format versions [`read_history`] reads.
pub const READABLE_VERSIONS: &[u32] = &[1];

/// One block written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockInfo {
    /// Its calendar year.
    pub year: i32,
    /// Its file name in the folder (`2022.arrow`).
    pub file: String,
    /// The SHA-256 of its bytes, lowercase hex.
    pub sha256: String,
    /// Its rows.
    pub rows: u64,
}

/// An indicator's metadata as a block carries it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IndicatorInfo {
    /// The registry's dotted name.
    pub name: String,
    /// The unit's spelling.
    pub unit: String,
    /// The period's spelling.
    pub period: String,
}

/// One point of a series.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    /// The registry's dotted name.
    pub indicator: String,
    /// The breakdown's name, `None` for national.
    pub breakdown: Option<String>,
    /// The member's index (0 for national).
    pub index: u32,
    /// The month index from the start (for a daily point, the month the day
    /// is in).
    pub month: u32,
    /// For a daily point, the day index from the start.
    pub day: Option<u32>,
    /// The value, in the indicator's unit.
    pub value: i64,
}

/// A history read from (or to be written to) a folder of blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History {
    /// The format version of the blocks.
    pub format_version: u32,
    /// The scenario's `sample_scale`.
    pub sample_scale: u32,
    /// The scenario's id.
    pub scenario: String,
    /// The engine's git hash.
    pub engine: String,
    /// The date of month 0's first day.
    pub start: Date,
    /// Each indicator's unit and period, in the writer's registry order.
    pub indicators: Vec<IndicatorInfo>,
    /// Every point.
    pub points: Vec<Point>,
    /// The watched groups' summaries: (name, month, summary).
    pub watched: Vec<(String, u32, GroupSummary)>,
}

/// The metadata key of a block.
const META: &str = "econ_sim_history";
/// The prefix of a watched group's field points.
const GROUP: &str = "group.";

/// The fields of a summary, in order, as values (`None`: absent).
fn group_fields(g: &GroupSummary) -> Vec<(&'static str, Option<i64>)> {
    let u = |x: u64| Some(i64::try_from(x).unwrap_or(i64::MAX));
    vec![
        ("persons", u(g.persons)),
        ("households", u(g.households)),
        ("person_records", Some(i64::from(g.person_records))),
        ("household_records", Some(i64::from(g.household_records))),
        ("small_sample", Some(i64::from(g.small_sample))),
        ("employed", u(g.employed)),
        ("unemployed", u(g.unemployed)),
        (
            "unemployment_rate_ppm",
            Some(i64::from(g.unemployment_rate_ppm)),
        ),
        ("wages_gross", Some(g.wages_gross.get())),
        ("wages_net", Some(g.wages_net.get())),
        ("pensions", Some(g.pensions.get())),
        ("unemployment_benefits", Some(g.unemployment_benefits.get())),
        ("household_transfers", Some(g.household_transfers.get())),
        ("taxes_paid", Some(g.taxes_paid.get())),
        ("money", Some(g.money.get())),
        ("debt", Some(g.debt.get())),
        ("income_per_adult", g.income_per_adult.map(|b| b.get())),
        ("sol_milli", g.sol_milli.map(i64::from)),
        ("approval_milli", g.approval_milli.map(i64::from)),
    ]
}

/// A summary from its fields.
fn group_from(fields: &BTreeMap<String, i64>) -> GroupSummary {
    let get = |k: &str| fields.get(k).copied();
    let n = |k: &str| u64::try_from(get(k).unwrap_or(0)).unwrap_or(0);
    let b = |k: &str| Bani(get(k).unwrap_or(0));
    GroupSummary {
        persons: n("persons"),
        households: n("households"),
        person_records: u32::try_from(n("person_records")).unwrap_or(u32::MAX),
        household_records: u32::try_from(n("household_records")).unwrap_or(u32::MAX),
        small_sample: get("small_sample") == Some(1),
        employed: n("employed"),
        unemployed: n("unemployed"),
        unemployment_rate_ppm: u32::try_from(n("unemployment_rate_ppm")).unwrap_or(u32::MAX),
        wages_gross: b("wages_gross"),
        wages_net: b("wages_net"),
        pensions: b("pensions"),
        unemployment_benefits: b("unemployment_benefits"),
        household_transfers: b("household_transfers"),
        taxes_paid: b("taxes_paid"),
        money: b("money"),
        debt: b("debt"),
        income_per_adult: get("income_per_adult").map(Bani),
        sol_milli: get("sol_milli").and_then(|x| u32::try_from(x).ok()),
        approval_milli: get("approval_milli").and_then(|x| i32::try_from(x).ok()),
    }
}

/// The calendar year of month `m` (month 0 is the start date's month).
fn year_of(start: Date, m: u32) -> i32 {
    let months = i64::from(start.month) - 1 + i64::from(m);
    start.year + i32::try_from(months / 12).expect("a year")
}

/// The engine's identity: the git hash given at build time, else the crate's
/// version.
fn engine() -> String {
    option_env!("ECON_GIT_HASH").map_or_else(
        || concat!("econ-io ", env!("CARGO_PKG_VERSION")).to_string(),
        str::to_string,
    )
}

/// SHA-256 of `bytes`, lowercase hex (FIPS 180-4).
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let mut msg = bytes.to_vec();
    let bits = (bytes.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
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
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (x, y) in h.iter_mut().zip(v) {
            *x = x.wrapping_add(y);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// A world's whole history (its closed and open months).
fn history_of(world: &World) -> History {
    let start = world.start_date();
    let cal = Calendar::new(start);
    let stats = world.stats();
    let mut points = Vec::new();
    let mut infos = Vec::new();
    for m in indicators() {
        infos.push(IndicatorInfo {
            name: m.name.to_string(),
            unit: m.unit.name().to_string(),
            period: m.period.name().to_string(),
        });
        for &k in stats.keys(m.id) {
            let Some(s) = stats.series(m.id, k) else {
                continue;
            };
            let breakdown = k.breakdown.map(|b| b.name().to_string());
            let first = s.first_month.0;
            for (i, &v) in s.months.iter().enumerate() {
                points.push(Point {
                    indicator: m.name.to_string(),
                    breakdown: breakdown.clone(),
                    index: k.index,
                    month: first + u32::try_from(i).expect("a month"),
                    day: None,
                    value: v,
                });
            }
            if let Some(days) = s.days {
                let d0 = (0..).find(|&d| cal.month(Day(d)).0 >= first).unwrap_or(0);
                for (i, &v) in days.iter().enumerate() {
                    let d = d0 + u32::try_from(i).expect("a day");
                    points.push(Point {
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
    let closes = u32::try_from(world.months().len()).expect("months");
    let mut watched = Vec::new();
    for name in world.watched_names() {
        let s = world.watched(name).unwrap_or(&[]);
        let n = u32::try_from(s.len()).expect("summaries");
        for (i, g) in s.iter().enumerate() {
            watched.push((
                name.to_string(),
                closes - n + u32::try_from(i).expect("a month"),
                g.clone(),
            ));
        }
    }
    History {
        format_version: FORMAT_VERSION,
        sample_scale: world.sample_scale(),
        scenario: String::new(),
        engine: engine(),
        start,
        indicators: infos,
        points,
        watched,
    }
}

/// Every year of a history with a point, and its block's bytes.
fn blocks(h: &History) -> Result<BTreeMap<i32, (Vec<u8>, u64)>, SaveError> {
    let order: BTreeMap<&str, usize> = h
        .indicators
        .iter()
        .enumerate()
        .map(|(i, x)| (x.name.as_str(), i))
        .collect();
    // Rows: the points, then the watched groups' fields.
    let mut rows: Vec<(usize, Point)> = h
        .points
        .iter()
        .map(|p| {
            (
                order
                    .get(p.indicator.as_str())
                    .copied()
                    .unwrap_or(usize::MAX),
                p.clone(),
            )
        })
        .collect();
    for (name, month, g) in &h.watched {
        for (field, v) in group_fields(g) {
            if let Some(value) = v {
                rows.push((
                    usize::MAX,
                    Point {
                        indicator: format!("{GROUP}{name}.{field}"),
                        breakdown: None,
                        index: 0,
                        month: *month,
                        day: None,
                        value,
                    },
                ));
            }
        }
    }
    rows.sort_by(|a, b| {
        (
            a.0,
            &a.1.indicator,
            &a.1.breakdown,
            a.1.index,
            a.1.month,
            a.1.day,
        )
            .cmp(&(
                b.0,
                &b.1.indicator,
                &b.1.breakdown,
                b.1.index,
                b.1.month,
                b.1.day,
            ))
    });
    let mut by_year: BTreeMap<i32, Vec<Point>> = BTreeMap::new();
    for (_, p) in rows {
        by_year
            .entry(year_of(h.start, p.month))
            .or_default()
            .push(p);
    }
    let mut out = BTreeMap::new();
    for (year, pts) in by_year {
        let meta = json!({
            "format_version": h.format_version,
            "sample_scale": h.sample_scale,
            "scenario": h.scenario,
            "engine": h.engine,
            "year": year,
            "start": [h.start.year, h.start.month, h.start.day],
            "indicators": h.indicators.iter().map(|x| json!([x.name, x.unit, x.period])).collect::<Vec<_>>(),
        });
        let schema = Schema::new(vec![
            Field::new("indicator", DataType::Utf8, false),
            Field::new("breakdown", DataType::Utf8, false),
            Field::new("index", DataType::UInt32, false),
            Field::new("month", DataType::UInt32, false),
            Field::new("day", DataType::UInt32, true),
            Field::new("value", DataType::Int64, false),
        ])
        .with_metadata([(META.to_string(), meta.to_string())].into_iter().collect());
        let cols: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(
                pts.iter().map(|p| p.indicator.as_str()).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                pts.iter()
                    .map(|p| p.breakdown.as_deref().unwrap_or(""))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(UInt32Array::from(
                pts.iter().map(|p| p.index).collect::<Vec<_>>(),
            )),
            Arc::new(UInt32Array::from(
                pts.iter().map(|p| p.month).collect::<Vec<_>>(),
            )),
            Arc::new(UInt32Array::from(
                pts.iter().map(|p| p.day).collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(
                pts.iter().map(|p| p.value).collect::<Vec<_>>(),
            )),
        ];
        let batch = RecordBatch::try_new(Arc::new(schema), cols)
            .map_err(|e| SaveError::Arrow(format!("{year}.arrow: {e}")))?;
        let bytes = ipc_bytes(&batch)?;
        out.insert(year, (bytes, u64::try_from(pts.len()).expect("rows")));
    }
    Ok(out)
}

fn io(path: &Path, e: impl std::fmt::Display) -> SaveError {
    SaveError::Io(format!("{}: {e}", path.display()))
}

/// Write blocks and, when `index` is set, `index.json` listing them.
fn write_blocks(
    dir: &Path,
    h: &History,
    keep: impl Fn(i32) -> bool,
    index: bool,
) -> Result<Vec<BlockInfo>, SaveError> {
    std::fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
    let mut infos = Vec::new();
    for (year, (bytes, rows)) in blocks(h)? {
        if !keep(year) {
            continue;
        }
        let file = format!("{year}.arrow");
        let path = dir.join(&file);
        std::fs::write(&path, &bytes).map_err(|e| io(&path, e))?;
        infos.push(BlockInfo {
            year,
            file,
            sha256: sha256_hex(&bytes),
            rows,
        });
    }
    if index {
        let doc = json!({
            "format_version": h.format_version,
            "sample_scale": h.sample_scale,
            "blocks": infos.iter().map(|b| json!({"year": b.year, "file": b.file, "sha256": b.sha256, "rows": b.rows})).collect::<Vec<_>>(),
        });
        let path = dir.join("index.json");
        let text = serde_json::to_string_pretty(&doc).map_err(|e| io(&path, e))? + "\n";
        std::fs::write(&path, text).map_err(|e| io(&path, e))?;
    }
    Ok(infos)
}

/// Whether year `y`'s December has closed.
fn closed(world: &World, y: i32) -> bool {
    let start = world.start_date();
    let dec = (i64::from(y) - i64::from(start.year)) * 12 + 12 - i64::from(start.month);
    dec >= 0 && dec < i64::try_from(world.months().len()).expect("months")
}

/// Write a closed year's block of `world`'s history into `dir`.
///
/// # Errors
/// I/O and encoding failures, or a year not closed.
pub fn write_year(dir: &Path, world: &World, year: i32) -> Result<BlockInfo, SaveError> {
    if !closed(world, year) {
        return Err(SaveError::Parse(format!(
            "{year}.arrow: the year {year} is not closed"
        )));
    }
    let h = history_of(world);
    write_blocks(dir, &h, |y| y == year, false)?
        .pop()
        .ok_or_else(|| SaveError::Parse(format!("{year}.arrow: no point in {year}")))
}

/// Write every closed year's block of `world`'s history and `index.json`.
///
/// # Errors
/// I/O and encoding failures.
pub fn write_all(dir: &Path, world: &World) -> Result<Vec<BlockInfo>, SaveError> {
    let h = history_of(world);
    write_blocks(dir, &h, |y| closed(world, y), true)
}

/// Write a whole history, one block per calendar year with a point, and
/// `index.json`; the blocks carry `h.format_version`.
///
/// # Errors
/// I/O and encoding failures.
pub fn write_history(dir: &Path, h: &History) -> Result<Vec<BlockInfo>, SaveError> {
    write_blocks(dir, h, |_| true, true)
}

/// Read every block of `dir`, checked against `index.json`: each block's
/// hash, one `sample_scale`, no block missing, a readable format version.
///
/// # Errors
/// A check failed (naming the file), or I/O and decoding failures.
pub fn read_history(dir: &Path) -> Result<History, SaveError> {
    let ipath = dir.join("index.json");
    let text = std::fs::read_to_string(&ipath).map_err(|e| io(&ipath, e))?;
    let index: Value =
        serde_json::from_str(&text).map_err(|e| SaveError::Parse(format!("index.json: {e}")))?;
    let entries = index["blocks"]
        .as_array()
        .ok_or_else(|| SaveError::Parse("index.json: no blocks".into()))?;
    let mut out: Option<History> = None;
    for e in entries {
        let file = e["file"].as_str().unwrap_or_default().to_string();
        let path = dir.join(&file);
        let bytes = std::fs::read(&path).map_err(|err| io(&path, err))?;
        if Some(sha256_hex(&bytes).as_str()) != e["sha256"].as_str() {
            return Err(SaveError::Checksum(format!(
                "{file}: its bytes do not match index.json"
            )));
        }
        let batch =
            read_batch(bytes).map_err(|err| SaveError::Arrow(format!("{file}: {err:?}")))?;
        let meta_text = batch
            .schema()
            .metadata()
            .get(META)
            .cloned()
            .ok_or_else(|| SaveError::Parse(format!("{file}: no {META} metadata")))?;
        let meta: Value = serde_json::from_str(&meta_text)
            .map_err(|err| SaveError::Parse(format!("{file}: {err}")))?;
        let version = u32::try_from(meta["format_version"].as_u64().unwrap_or(0)).unwrap_or(0);
        if !READABLE_VERSIONS.contains(&version) {
            return Err(SaveError::Parse(format!(
                "{file}: format version {version} is not readable"
            )));
        }
        let scale = u32::try_from(meta["sample_scale"].as_u64().unwrap_or(0)).unwrap_or(0);
        let h = out.get_or_insert_with(|| {
            let s = &meta["start"];
            let n = |i: usize| s[i].as_i64().unwrap_or(1);
            History {
                format_version: version,
                sample_scale: scale,
                scenario: meta["scenario"].as_str().unwrap_or_default().to_string(),
                engine: meta["engine"].as_str().unwrap_or_default().to_string(),
                start: Date::new(
                    i32::try_from(n(0)).unwrap_or(2021),
                    u8::try_from(n(1)).unwrap_or(1),
                    u8::try_from(n(2)).unwrap_or(1),
                )
                .unwrap_or_else(|| Date::new(2021, 12, 1).expect("a date")),
                indicators: meta["indicators"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|x| IndicatorInfo {
                                name: x[0].as_str().unwrap_or_default().to_string(),
                                unit: x[1].as_str().unwrap_or_default().to_string(),
                                period: x[2].as_str().unwrap_or_default().to_string(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                points: Vec::new(),
                watched: Vec::new(),
            }
        });
        if h.sample_scale != scale {
            return Err(SaveError::Parse(format!(
                "{file}: sample_scale {scale}, not the other blocks' {}",
                h.sample_scale
            )));
        }
        let col_str = |name: &str| -> Result<Vec<String>, SaveError> {
            let a = batch
                .column_by_name(name)
                .and_then(|c| c.as_any().downcast_ref::<StringArray>().cloned())
                .ok_or_else(|| SaveError::Parse(format!("{file}: column {name}")))?;
            Ok((0..a.len()).map(|i| a.value(i).to_string()).collect())
        };
        let col_u32 = |name: &str| -> Result<UInt32Array, SaveError> {
            batch
                .column_by_name(name)
                .and_then(|c| c.as_any().downcast_ref::<UInt32Array>().cloned())
                .ok_or_else(|| SaveError::Parse(format!("{file}: column {name}")))
        };
        let ind = col_str("indicator")?;
        let br = col_str("breakdown")?;
        let (idx, month, day) = (col_u32("index")?, col_u32("month")?, col_u32("day")?);
        let value = batch
            .column_by_name("value")
            .and_then(|c| c.as_any().downcast_ref::<Int64Array>().cloned())
            .ok_or_else(|| SaveError::Parse(format!("{file}: column value")))?;
        let mut groups: BTreeMap<(String, u32), BTreeMap<String, i64>> = BTreeMap::new();
        for i in 0..ind.len() {
            if let Some(rest) = ind[i].strip_prefix(GROUP) {
                if let Some((name, field)) = rest.rsplit_once('.') {
                    groups
                        .entry((name.to_string(), month.value(i)))
                        .or_default()
                        .insert(field.to_string(), value.value(i));
                }
                continue;
            }
            h.points.push(Point {
                indicator: ind[i].clone(),
                breakdown: (!br[i].is_empty()).then(|| br[i].clone()),
                index: idx.value(i),
                month: month.value(i),
                day: (!day.is_null(i)).then(|| day.value(i)),
                value: value.value(i),
            });
        }
        for ((name, m), fields) in groups {
            h.watched.push((name, m, group_from(&fields)));
        }
    }
    out.ok_or_else(|| SaveError::Parse("index.json: no blocks".into()))
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }
}
