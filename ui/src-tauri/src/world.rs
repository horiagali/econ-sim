//! The thin UI's window on the real `World` (roadmap M2): one world held by
//! the app, built from the Romania scenario, stepped by days, read as monthly
//! series. Commands run off the main thread (`async`), so a long step does
//! not freeze the window.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use econ_core::world::{Law, MonthStats, World, WorldParams};
use serde::Serialize;

/// The world the window shows, if one is open.
#[derive(Default)]
pub struct Held(Mutex<Option<World>>);

/// The fixtures of the scenario builder: `ECON_FIXTURES`, else the repo's.
fn fixtures() -> PathBuf {
    std::env::var_os("ECON_FIXTURES").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python/pipeline/fixtures"),
        PathBuf::from,
    )
}

/// Where the world is now.
#[derive(Serialize)]
pub struct Status {
    open: bool,
    sample_scale: u32,
    day: u32,
    date: String,
    months: u32,
    /// The checks of the latest day that failed, by name; empty when all hold.
    failed: Vec<String>,
}

fn status(w: Option<&World>, sample_scale: u32, failed: Vec<String>) -> Status {
    match w {
        None => Status {
            open: false,
            sample_scale,
            day: 0,
            date: String::new(),
            months: 0,
            failed,
        },
        Some(w) => {
            let date = w.date();
            Status {
                open: true,
                sample_scale,
                day: w.day().0,
                date: format!("{:04}-{:02}-{:02}", date.year, date.month, date.day),
                months: u32::try_from(w.months().len()).unwrap_or(u32::MAX),
                failed,
            }
        }
    }
}

/// Build the Romania scenario at `sample_scale` and open a world on it under
/// the placeholder law, at the spec defaults (seed 42).
#[tauri::command(async)]
pub fn world_open(state: tauri::State<'_, Held>, sample_scale: u32) -> Result<Status, String> {
    if sample_scale == 0 {
        return Err("the sample scale must be at least 1".into());
    }
    let fixtures = fixtures();
    let inputs = econ_scenario::read_inputs(&fixtures).map_err(|e| format!("inputs: {e:?}"))?;
    let params = econ_scenario::BuildParams {
        sample_scale,
        rng_seed: 42,
    };
    let scenario = econ_scenario::build(&inputs, &params).map_err(|e| format!("build: {e:?}"))?;
    let dir = std::env::temp_dir().join(format!("econ-sim-ui-{sample_scale}"));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    econ_scenario::write(&scenario, &inputs, &dir).map_err(|e| format!("write: {e:?}"))?;
    let start = econ_io::read_start_state(&dir, &fixtures.join("io_table2021_ro.json"))
        .map_err(|e| format!("read: {e:?}"))?;
    let world = World::new(
        start,
        Law::placeholder_2021(),
        WorldParams::spec_defaults(42),
    )
    .map_err(|e| format!("world: {e:?}"))?;
    let mut held = state.0.lock().map_err(|_| "the world's lock is poisoned")?;
    *held = Some(world);
    Ok(status(held.as_ref(), sample_scale, Vec::new()))
}

/// Run `days` days with no command; stops early at the first day whose checks
/// fail.
#[tauri::command(async)]
pub fn world_step(
    state: tauri::State<'_, Held>,
    days: u32,
    sample_scale: u32,
) -> Result<Status, String> {
    let mut held = state.0.lock().map_err(|_| "the world's lock is poisoned")?;
    let w = held.as_mut().ok_or("no world is open")?;
    for _ in 0..days {
        let r = w
            .step(&[])
            .map_err(|e| format!("day {}: {e:?}", w.day().0))?;
        if !r.checks_ok {
            let failed = r.failed.iter().map(|f| format!("{f:?}")).collect();
            return Ok(status(Some(w), sample_scale, failed));
        }
    }
    Ok(status(Some(w), sample_scale, Vec::new()))
}

/// The monthly series, as one name and one `f64` per closed month each.
pub const SERIES: [&str; 12] = [
    "Unemployment rate (%)",
    "Inflation, 12 months (%)",
    "Expected inflation (%)",
    "CPI (start = 100)",
    "Employed (thousands)",
    "GDP, production (bn lei)",
    "Household consumption (bn lei)",
    "Wage bill (bn lei)",
    "Wage share of GDP (%)",
    "Consumption share of GDP (%)",
    "Shortages (bn lei)",
    "Households in hardship (thousands)",
];

fn row(m: &MonthStats) -> [f64; 12] {
    let bn = |b: econ_types::Bani| b.get() as f64 / 1e11;
    let labour = (m.employed + m.unemployed).max(1) as f64;
    let gdp = m.gdp_production.get().max(1) as f64;
    [
        100.0 * m.unemployed as f64 / labour,
        f64::from(m.inflation_ppm) / 1e4,
        f64::from(m.expected_inflation_ppm) / 1e4,
        f64::from(m.cpi_ppm) / 1e4,
        m.employed as f64 / 1e3,
        bn(m.gdp_production),
        bn(m.consumption),
        bn(m.wage_bill),
        100.0 * m.wage_bill.get() as f64 / gdp,
        100.0 * m.consumption.get() as f64 / gdp,
        m.shortages.iter().map(|&s| s as f64).sum::<f64>() / 1e11,
        m.hardship_households as f64 / 1e3,
    ]
}

/// Every series over the closed months, series after series, as
/// little-endian `f64` bytes (binary IPC, as in Spike 8).
#[tauri::command(async)]
pub fn world_series(state: tauri::State<'_, Held>) -> Result<tauri::ipc::Response, String> {
    let held = state.0.lock().map_err(|_| "the world's lock is poisoned")?;
    let months = held.as_ref().map_or(&[][..], World::months);
    let rows: Vec<[f64; 12]> = months.iter().map(row).collect();
    let mut bytes = Vec::with_capacity(rows.len() * SERIES.len() * 8);
    for s in 0..SERIES.len() {
        for r in &rows {
            bytes.extend_from_slice(&r[s].to_le_bytes());
        }
    }
    Ok(tauri::ipc::Response::new(bytes))
}

/// The names of [`world_series`]'s series, in its order.
#[tauri::command]
pub fn world_series_names() -> Vec<&'static str> {
    SERIES.to_vec()
}
