//! Save games, migrations and replay (ADR-0011) — Spike 6.
//!
//! A save is a ZIP archive (entries stored, not deflated: tables are already
//! zstd-compressed) containing:
//!
//! | Entry | Content |
//! |---|---|
//! | `manifest.json` | format version, engine version, seed, `sample_scale`, tick, state hash, applied migrations, per-entry checksums |
//! | `persons.arrow`, `households.arrow`, `io.arrow`, `shares.arrow` | Arrow IPC files, zstd-compressed |
//! | `commands.jsonl` | the player command log since the scenario start |
//!
//! Loading verifies checksums, runs named migrations in order, rebuilds the
//! world and checks its state hash against the manifest. Because the core is
//! deterministic, any past state can be rebuilt from the scenario start plus
//! the command log (replay).
//!
//! [`popgen`] reads the population generator's margin file and writes its
//! output tables (ADR-0012).

use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use arrow_array::{
    Array, ArrayRef, Float64Array, Int64Array, RecordBatch, UInt8Array, UInt16Array, UInt32Array,
};
use arrow_ipc::CompressionType;
use arrow_ipc::reader::FileReader;
use arrow_ipc::writer::{FileWriter, IpcWriteOptions};
use arrow_schema::{DataType, Field, Schema};
use econ_core::scale_spike::{ScaleColumns, ScaleCommand, ScaleParams, ScaleWorld};
use serde::{Deserialize, Serialize};

/// Current save format version.
pub const SAVE_FORMAT_VERSION: u32 = 3;
/// Engine version written into saves.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A command as stored in the log.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LoggedCommand {
    /// See [`ScaleCommand::SetSeparationRate`].
    SetSeparationRate(f64),
}

impl From<LoggedCommand> for ScaleCommand {
    fn from(c: LoggedCommand) -> Self {
        match c {
            LoggedCommand::SetSeparationRate(r) => ScaleCommand::SetSeparationRate(r),
        }
    }
}

/// One entry of the command log: applied at the start of `tick`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    /// Tick at whose start the command applies.
    pub tick: u32,
    /// The command.
    pub cmd: LoggedCommand,
}

/// Save manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Save format version.
    pub save_format_version: u32,
    /// Engine version that wrote the save.
    pub engine_version: String,
    /// Scenario id.
    pub scenario: String,
    /// Real people per synthetic person.
    pub sample_scale: u32,
    /// Master seed.
    pub seed: u64,
    /// Next tick to run.
    pub tick: u32,
    /// Monthly separation rate (added in format v2; see migration `v1_to_v2`).
    #[serde(default)]
    pub separation_rate: Option<f64>,
    /// Behaviour parameters `[matching_efficiency, vacancy_ratio, mpc_income, mpc_wealth]`
    /// (absent in older saves → defaults).
    #[serde(default)]
    pub params: Option<[f64; 4]>,
    /// VAT collected by government so far, bani (added in format v3; see
    /// migration `v2_to_v3`).
    #[serde(default)]
    pub gov_vat: Option<i64>,
    /// State hash of the saved world (hex).
    pub state_hash: String,
    /// Names of migrations applied on load, in order.
    #[serde(default)]
    pub migrations_applied: Vec<String>,
    /// FNV-1a 64 checksum (hex) of every other entry.
    pub checksums: Vec<(String, String)>,
}

/// Errors while saving or loading.
#[derive(Debug)]
pub enum SaveError {
    /// I/O or archive problem.
    Io(String),
    /// Arrow encoding problem.
    Arrow(String),
    /// Manifest or log could not be parsed.
    Parse(String),
    /// A checksum did not match.
    Checksum(String),
    /// The rebuilt world's hash differs from the manifest.
    StateHash {
        /// In the manifest.
        expected: String,
        /// Recomputed.
        got: String,
    },
    /// Unsupported format version.
    Version(u32),
}

fn io_err(e: impl std::fmt::Display) -> SaveError {
    SaveError::Io(e.to_string())
}
fn arrow_err(e: impl std::fmt::Display) -> SaveError {
    SaveError::Arrow(e.to_string())
}

fn fnv64(bytes: &[u8]) -> String {
    let mut h = econ_core::StateHasher::default();
    h.write(bytes);
    format!("{:016x}", h.finish())
}

fn ipc_bytes(batch: &RecordBatch) -> Result<Vec<u8>, SaveError> {
    let opts = IpcWriteOptions::default()
        .try_with_compression(Some(CompressionType::ZSTD))
        .map_err(arrow_err)?;
    let mut buf = Vec::new();
    {
        let mut w =
            FileWriter::try_new_with_options(&mut buf, &batch.schema(), opts).map_err(arrow_err)?;
        w.write(batch).map_err(arrow_err)?;
        w.finish().map_err(arrow_err)?;
    }
    Ok(buf)
}

fn read_batch(bytes: Vec<u8>) -> Result<RecordBatch, SaveError> {
    let mut r = FileReader::try_new(Cursor::new(bytes), None).map_err(arrow_err)?;
    match r.next() {
        Some(b) => b.map_err(arrow_err),
        None => Err(SaveError::Arrow("empty table".into())),
    }
}

fn batch(fields: Vec<(&str, DataType, ArrayRef)>) -> Result<RecordBatch, SaveError> {
    let schema = Schema::new(
        fields
            .iter()
            .map(|(n, t, _)| Field::new(*n, t.clone(), false))
            .collect::<Vec<_>>(),
    );
    RecordBatch::try_new(Arc::new(schema), fields.into_iter().map(|f| f.2).collect())
        .map_err(arrow_err)
}

macro_rules! col {
    ($b:expr, $name:literal, $ty:ty) => {{
        let a = $b
            .column_by_name($name)
            .ok_or_else(|| SaveError::Parse(format!("missing column {}", $name)))?;
        a.as_any()
            .downcast_ref::<$ty>()
            .ok_or_else(|| SaveError::Parse(format!("bad type for {}", $name)))?
            .values()
            .to_vec()
    }};
}

pub mod popgen;

/// Serialise a world + command log into save bytes (a ZIP archive).
///
/// # Errors
/// On encoding failures.
pub fn save_to_bytes(world: &ScaleWorld, log: &[LogEntry]) -> Result<Vec<u8>, SaveError> {
    let c = world.to_columns();
    let persons = batch(vec![
        (
            "age",
            DataType::UInt8,
            Arc::new(UInt8Array::from(c.age.clone())) as ArrayRef,
        ),
        (
            "county",
            DataType::UInt8,
            Arc::new(UInt8Array::from(c.county.clone())),
        ),
        (
            "edu",
            DataType::UInt8,
            Arc::new(UInt8Array::from(c.edu.clone())),
        ),
        (
            "status",
            DataType::UInt8,
            Arc::new(UInt8Array::from(c.status.clone())),
        ),
        (
            "industry",
            DataType::UInt16,
            Arc::new(UInt16Array::from(c.industry.clone())),
        ),
        (
            "wage",
            DataType::Int64,
            Arc::new(Int64Array::from(c.wage.clone())),
        ),
        (
            "household",
            DataType::UInt32,
            Arc::new(UInt32Array::from(c.household.clone())),
        ),
    ])?;
    let households = batch(vec![
        (
            "hh_weight",
            DataType::UInt32,
            Arc::new(UInt32Array::from(c.hh_weight.clone())) as ArrayRef,
        ),
        (
            "hh_deposits",
            DataType::Int64,
            Arc::new(Int64Array::from(c.hh_deposits.clone())),
        ),
        (
            "hh_income",
            DataType::Int64,
            Arc::new(Int64Array::from(c.hh_income.clone())),
        ),
    ])?;
    let io = batch(vec![(
        "a",
        DataType::Float64,
        Arc::new(Float64Array::from(c.io.clone())) as ArrayRef,
    )])?;
    let shares = batch(vec![(
        "share",
        DataType::Float64,
        Arc::new(Float64Array::from(c.consumption_shares.clone())) as ArrayRef,
    )])?;
    let mut entries: Vec<(String, Vec<u8>)> = vec![
        ("persons.arrow".into(), ipc_bytes(&persons)?),
        ("households.arrow".into(), ipc_bytes(&households)?),
        ("io.arrow".into(), ipc_bytes(&io)?),
        ("shares.arrow".into(), ipc_bytes(&shares)?),
    ];
    let mut log_txt = String::new();
    for e in log {
        log_txt.push_str(&serde_json::to_string(e).map_err(|e| SaveError::Parse(e.to_string()))?);
        log_txt.push('\n');
    }
    entries.push(("commands.jsonl".into(), log_txt.into_bytes()));
    let manifest = Manifest {
        save_format_version: SAVE_FORMAT_VERSION,
        engine_version: ENGINE_VERSION.into(),
        scenario: "scale-spike".into(),
        sample_scale: c.sample_scale,
        seed: c.seed,
        tick: c.tick,
        separation_rate: Some(c.separation_rate),
        params: Some([
            c.params.matching_efficiency,
            c.params.vacancy_ratio,
            c.params.mpc_income,
            c.params.mpc_wealth,
        ]),
        gov_vat: Some(c.gov_vat),
        state_hash: format!("{:016x}", world.state_hash()),
        migrations_applied: vec![],
        checksums: entries.iter().map(|(n, b)| (n.clone(), fnv64(b))).collect(),
    };
    let manifest_json =
        serde_json::to_vec_pretty(&manifest).map_err(|e| SaveError::Parse(e.to_string()))?;
    write_zip(&manifest_json, &entries)
}

fn write_zip(manifest: &[u8], entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, SaveError> {
    let mut out = Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut out);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .last_modified_time(zip::DateTime::default()); // fixed timestamp → byte-reproducible saves
        z.start_file("manifest.json", opts).map_err(io_err)?;
        z.write_all(manifest).map_err(io_err)?;
        for (name, bytes) in entries {
            z.start_file(name.as_str(), opts).map_err(io_err)?;
            z.write_all(bytes).map_err(io_err)?;
        }
        z.finish().map_err(io_err)?;
    }
    Ok(out.into_inner())
}

/// A loaded save.
#[derive(Debug)]
pub struct Loaded {
    /// The rebuilt world.
    pub world: ScaleWorld,
    /// The command log.
    pub log: Vec<LogEntry>,
    /// The manifest after migrations.
    pub manifest: Manifest,
}

/// Named migrations, applied in order to manifests older than the current
/// format. Each takes the manifest and returns it upgraded by one version.
/// One migration step: (from version, name, function).
type Migration = (u32, &'static str, fn(Manifest) -> Manifest);

const MIGRATIONS: &[Migration] = &[
    (1, "v1_to_v2_add_separation_rate", v1_to_v2),
    (2, "v2_to_v3_add_gov_vat", v2_to_v3),
];

fn v1_to_v2(mut m: Manifest) -> Manifest {
    // v1 saves predate the adjustable separation rate; it was fixed at 1.5%.
    if m.separation_rate.is_none() {
        m.separation_rate = Some(0.015);
    }
    m.save_format_version = 2;
    m
}

fn v2_to_v3(mut m: Manifest) -> Manifest {
    // v2 saves predate VAT in the scale world: government had collected nothing.
    if m.gov_vat.is_none() {
        m.gov_vat = Some(0);
    }
    m.save_format_version = 3;
    m
}

/// Load save bytes: verify checksums, migrate, rebuild, verify state hash.
///
/// # Errors
/// On any corruption, unsupported version or hash mismatch.
pub fn load_from_bytes(bytes: &[u8]) -> Result<Loaded, SaveError> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).map_err(io_err)?;
    let mut read = |name: &str| -> Result<Vec<u8>, SaveError> {
        let mut f = z.by_name(name).map_err(io_err)?;
        let mut v = Vec::new();
        f.read_to_end(&mut v).map_err(io_err)?;
        Ok(v)
    };
    let mut manifest: Manifest = serde_json::from_slice(&read("manifest.json")?)
        .map_err(|e| SaveError::Parse(e.to_string()))?;
    let mut files = std::collections::BTreeMap::new();
    for (name, sum) in &manifest.checksums {
        let b = read(name)?;
        if &fnv64(&b) != sum {
            return Err(SaveError::Checksum(name.clone()));
        }
        files.insert(name.clone(), b);
    }
    if manifest.save_format_version > SAVE_FORMAT_VERSION {
        return Err(SaveError::Version(manifest.save_format_version));
    }
    for &(from, name, f) in MIGRATIONS {
        if manifest.save_format_version == from {
            manifest = f(manifest);
            manifest.migrations_applied.push(name.to_string());
        }
    }
    if manifest.save_format_version != SAVE_FORMAT_VERSION {
        return Err(SaveError::Version(manifest.save_format_version));
    }
    let mut take = |n: &str| {
        files
            .remove(n)
            .ok_or_else(|| SaveError::Parse(format!("missing {n}")))
    };
    let p = read_batch(take("persons.arrow")?)?;
    let h = read_batch(take("households.arrow")?)?;
    let io = read_batch(take("io.arrow")?)?;
    let sh = read_batch(take("shares.arrow")?)?;
    let log_txt =
        String::from_utf8(take("commands.jsonl")?).map_err(|e| SaveError::Parse(e.to_string()))?;
    let log = log_txt
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).map_err(|e| SaveError::Parse(e.to_string())))
        .collect::<Result<Vec<LogEntry>, _>>()?;
    let cols = ScaleColumns {
        sample_scale: manifest.sample_scale,
        seed: manifest.seed,
        tick: manifest.tick,
        separation_rate: manifest.separation_rate.unwrap_or(0.015),
        params: manifest
            .params
            .map_or_else(ScaleParams::default, |p| ScaleParams {
                matching_efficiency: p[0],
                vacancy_ratio: p[1],
                mpc_income: p[2],
                mpc_wealth: p[3],
            }),
        age: col!(p, "age", UInt8Array),
        county: col!(p, "county", UInt8Array),
        edu: col!(p, "edu", UInt8Array),
        status: col!(p, "status", UInt8Array),
        industry: col!(p, "industry", UInt16Array),
        wage: col!(p, "wage", Int64Array),
        household: col!(p, "household", UInt32Array),
        hh_weight: col!(h, "hh_weight", UInt32Array),
        hh_deposits: col!(h, "hh_deposits", Int64Array),
        hh_income: col!(h, "hh_income", Int64Array),
        gov_vat: manifest.gov_vat.unwrap_or(0),
        io: col!(io, "a", Float64Array),
        consumption_shares: col!(sh, "share", Float64Array),
    };
    let world = ScaleWorld::from_columns(cols);
    let got = format!("{:016x}", world.state_hash());
    if got != manifest.state_hash {
        return Err(SaveError::StateHash {
            expected: manifest.state_hash.clone(),
            got,
        });
    }
    Ok(Loaded {
        world,
        log,
        manifest,
    })
}

/// Run `world` from its current tick up to (not including) `until`, applying
/// logged commands at the start of their tick.
pub fn run_with_log(world: &mut ScaleWorld, log: &[LogEntry], until: u32) {
    while world.tick() < until {
        let t = world.tick();
        let cmds: Vec<ScaleCommand> = log
            .iter()
            .filter(|e| e.tick == t)
            .map(|e| e.cmd.into())
            .collect();
        world.apply(&cmds);
        let _ = world.step();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log() -> Vec<LogEntry> {
        vec![
            LogEntry {
                tick: 2,
                cmd: LoggedCommand::SetSeparationRate(0.03),
            },
            LogEntry {
                tick: 7,
                cmd: LoggedCommand::SetSeparationRate(0.01),
            },
        ]
    }

    #[test]
    fn save_load_replay_round_trip() {
        let scale = 2000;
        // Uninterrupted run to tick 10.
        let mut live = ScaleWorld::generate(scale, 9);
        run_with_log(&mut live, &log(), 10);
        // Run to tick 5, save, load, continue to 10.
        let mut w = ScaleWorld::generate(scale, 9);
        run_with_log(&mut w, &log(), 5);
        let bytes = save_to_bytes(&w, &log()).unwrap();
        let mut loaded = load_from_bytes(&bytes).unwrap();
        assert_eq!(loaded.world.state_hash(), w.state_hash());
        run_with_log(&mut loaded.world, &loaded.log, 10);
        assert_eq!(loaded.world.state_hash(), live.state_hash());
        // Full replay from the scenario start reproduces the same state.
        let mut replay = ScaleWorld::generate(scale, 9);
        run_with_log(&mut replay, &loaded.log, 10);
        assert_eq!(replay.state_hash(), live.state_hash());
        // The commands must actually matter (guards against a hollow test).
        let mut no_cmds = ScaleWorld::generate(scale, 9);
        run_with_log(&mut no_cmds, &[], 10);
        assert_ne!(no_cmds.state_hash(), live.state_hash());
    }

    #[test]
    fn saves_are_byte_reproducible() {
        let w = ScaleWorld::generate(5000, 1);
        assert_eq!(
            save_to_bytes(&w, &log()).unwrap(),
            save_to_bytes(&w, &log()).unwrap()
        );
    }

    #[test]
    fn corruption_is_detected() {
        let w = ScaleWorld::generate(5000, 1);
        let mut bytes = save_to_bytes(&w, &[]).unwrap();
        // Flip a byte deep inside the archive (inside a table entry).
        let i = bytes.len() / 2;
        bytes[i] ^= 0xFF;
        assert!(load_from_bytes(&bytes).is_err());
    }

    #[test]
    fn v1_save_is_migrated() {
        let w = ScaleWorld::generate(5000, 1);
        let bytes = save_to_bytes(&w, &[]).unwrap();
        // Rewrite the manifest as a v1 save (no separation_rate field).
        let mut z = zip::ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
        let mut m: Manifest = {
            let mut s = String::new();
            z.by_name("manifest.json")
                .unwrap()
                .read_to_string(&mut s)
                .unwrap();
            serde_json::from_str(&s).unwrap()
        };
        m.save_format_version = 1;
        m.separation_rate = None;
        m.gov_vat = None;
        let mut entries = Vec::new();
        for (name, _) in m.checksums.clone() {
            let mut v = Vec::new();
            z.by_name(&name).unwrap().read_to_end(&mut v).unwrap();
            entries.push((name, v));
        }
        let v1 = write_zip(&serde_json::to_vec(&m).unwrap(), &entries).unwrap();
        let loaded = load_from_bytes(&v1).unwrap();
        assert_eq!(
            loaded.manifest.migrations_applied,
            vec![
                "v1_to_v2_add_separation_rate".to_string(),
                "v2_to_v3_add_gov_vat".to_string()
            ]
        );
        assert_eq!(loaded.manifest.save_format_version, 3);
        assert_eq!(loaded.manifest.gov_vat, Some(0));
    }
}
