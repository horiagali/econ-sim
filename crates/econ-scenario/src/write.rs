//! Step 5: the scenario folder, written into a temporary folder next to its
//! place and renamed, so that a failed write leaves no partial scenario.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use econ_io::popgen::{Stages, households_table, persons_table};
use econ_io::table::{Column, NamedColumn, ipc_table};
use econ_ledger::{Instrument, Sector};
use serde_json::{Map, Value, json};

use crate::sha256::sha256_hex;
use crate::{BuildError, FirmUnitTable, Inputs, START_DATE, Scenario};

fn io_err(e: impl std::fmt::Display) -> BuildError {
    BuildError::Io(e.to_string())
}

/// `tables/firm_units.arrow`.
fn firm_units_table(t: &FirmUnitTable) -> Result<Vec<u8>, BuildError> {
    let col = |name, values, unit| NamedColumn { name, values, unit };
    let mut columns = vec![
        col("unit_id", Column::Utf8(t.unit_id.clone()), None),
        col("industry", Column::U8(t.industry.clone()), None),
        col("industry_group", Column::U8(t.industry_group.clone()), None),
        col("size", Column::U8(t.size.clone()), None),
        col("kind", Column::U8(t.kind.clone()), None),
        col("controller", Column::U8(t.controller.clone()), None),
        col("firm_count", Column::U32(t.firm_count.clone()), None),
        col(
            "persons_employed",
            Column::U64(t.persons_employed.clone()),
            Some("persons, flow"),
        ),
        col("output", Column::U64(t.output.clone()), Some("bani, flow")),
        col(
            "value_added",
            Column::U64(t.value_added.clone()),
            Some("bani, flow"),
        ),
        col("wages", Column::U64(t.wages.clone()), Some("bani, flow")),
        col(
            "own_state",
            Column::U32(t.own_state.clone()),
            Some("millionths"),
        ),
        col(
            "own_domestic_private",
            Column::U32(t.own_domestic_private.clone()),
            Some("millionths"),
        ),
        col(
            "own_foreign",
            Column::U32(t.own_foreign.clone()),
            Some("millionths"),
        ),
    ];
    for (name, values) in [
        ("fixed_assets", &t.fixed_assets),
        ("currency", &t.currency),
        ("deposits", &t.deposits),
        ("debt_securities_held", &t.debt_securities_held),
        ("loans_granted", &t.loans_granted),
        ("equity_held", &t.equity_held),
        ("insurance", &t.insurance),
        ("other_receivable", &t.other_receivable),
        ("debt_securities_issued", &t.debt_securities_issued),
        ("loans", &t.loans),
        ("equity_issued", &t.equity_issued),
        ("other_payable", &t.other_payable),
    ] {
        columns.push(col(name, Column::U64(values.clone()), Some("bani, stock")));
    }
    columns.push(col(
        "employees",
        Column::U64(t.employees.clone()),
        Some("weighted persons"),
    ));
    ipc_table(columns).map_err(|e| io_err(format!("{e:?}")))
}

/// `balances.arrow`: one row per (instrument, sector), in matrix order.
fn balances_table(scenario: &Scenario) -> Result<Vec<u8>, BuildError> {
    let (mut instrument, mut sector, mut balance) = (Vec::new(), Vec::new(), Vec::new());
    for (i, row) in Instrument::ALL.into_iter().zip(&scenario.opening.balances) {
        for (s, b) in Sector::ALL.into_iter().zip(row) {
            instrument.push(i as u8);
            sector.push(s as u8);
            balance.push(b.get());
        }
    }
    ipc_table(vec![
        NamedColumn {
            name: "instrument",
            values: Column::U8(instrument),
            unit: None,
        },
        NamedColumn {
            name: "sector",
            values: Column::U8(sector),
            unit: None,
        },
        NamedColumn {
            name: "balance",
            values: Column::I64(balance),
            unit: Some("bani, stock"),
        },
    ])
    .map_err(|e| io_err(format!("{e:?}")))
}

fn report_json(scenario: &Scenario) -> Value {
    let checks: Vec<Value> = scenario
        .report
        .checks
        .iter()
        .map(|c| {
            let mut o = Map::new();
            o.insert("name".into(), json!(c.name));
            o.insert("ok".into(), json!(c.ok));
            if let Some(why) = c.known_gap {
                o.insert("known_gap".into(), json!(why));
            }
            o.insert("facts".into(), c.facts.clone());
            Value::Object(o)
        })
        .collect();
    json!({
        "sample_scale": scenario.params.sample_scale,
        "rng_seed": scenario.params.rng_seed,
        "consistent": scenario.report.consistent(),
        "checks": checks,
    })
}

fn pretty(v: &Value) -> Result<Vec<u8>, BuildError> {
    let mut bytes = serde_json::to_vec_pretty(v).map_err(io_err)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Every file of the folder but `scenario.json`, by its path in the folder.
fn files(scenario: &Scenario, inputs: &Inputs) -> Result<BTreeMap<String, Vec<u8>>, BuildError> {
    let s = &scenario.stages;
    let st = &inputs.stages;
    let stages = Stages {
        attributes: Some((&s.attributes, &st.attributes.provenance)),
        jobs: Some((&s.jobs, &st.jobs.provenance)),
        housing: Some((&s.housing, &st.housing.provenance)),
        wages: Some((&s.wages, &st.wages.provenance)),
        pensions: Some((&s.pensions, &st.pensions.provenance)),
        wealth: Some((&s.wealth, &st.wealth.provenance)),
        incomes: Some((&s.incomes, &st.incomes.provenance)),
        employers: Some((&s.employers, &st.firms.provenance)),
        region_of_county: Some(&st.attributes.margins.region_of_county),
    };
    let arrow = |r: Result<Vec<u8>, econ_io::SaveError>| r.map_err(|e| io_err(format!("{e:?}")));
    let mut out = BTreeMap::new();
    out.insert(
        "tables/households.arrow".to_string(),
        arrow(households_table(&scenario.population, &stages))?,
    );
    out.insert(
        "tables/persons.arrow".to_string(),
        arrow(persons_table(&scenario.population, &stages))?,
    );
    out.insert(
        "tables/firm_units.arrow".to_string(),
        firm_units_table(&scenario.firm_units)?,
    );
    out.insert("balances.arrow".to_string(), balances_table(scenario)?);
    // The starting budget, as the fixture is.
    if let Some((_, bytes)) = inputs.files.iter().find(|(n, _)| n == crate::build::BUDGET) {
        out.insert("budget.json".to_string(), bytes.clone());
    }
    out.insert("report.json".to_string(), pretty(&report_json(scenario))?);
    Ok(out)
}

fn scenario_json(scenario: &Scenario, inputs: &Inputs, files: &BTreeMap<String, Vec<u8>>) -> Value {
    let hex = |h: u64| format!("{h:016x}");
    let stage_hashes: Map<String, Value> = scenario
        .stage_hashes()
        .into_iter()
        .map(|(name, h)| (name.to_string(), json!(hex(h))))
        .collect();
    let sorted =
        |pairs: BTreeMap<String, Value>| -> Map<String, Value> { pairs.into_iter().collect() };
    let input_hashes = sorted(
        inputs
            .files
            .iter()
            .map(|(name, bytes)| (name.clone(), json!(sha256_hex(bytes))))
            .collect(),
    );
    let file_hashes = sorted(
        files
            .iter()
            .map(|(path, bytes)| (path.clone(), json!(sha256_hex(bytes))))
            .collect(),
    );
    let provenance = sorted(
        inputs
            .docs
            .iter()
            .map(|(name, doc)| ((*name).to_string(), doc["provenance"].clone()))
            .collect(),
    );
    json!({
        "format": 1,
        "name": "Romania, 1 December 2021",
        "start_date": START_DATE,
        "sample_scale": scenario.params.sample_scale,
        "rng_seed": scenario.params.rng_seed,
        "engine_version": env!("CARGO_PKG_VERSION"),
        "stage_hashes": stage_hashes,
        "opening_hash": hex(scenario.opening.state_hash()),
        // Government bonds by residual maturity (up to 1 year, 1–3, 3–5,
        // 5–10, over 10), bani: the shape of the opening debt (spec
        // economy/fiscal-policy, "Starting state").
        "gov_debt_by_maturity": inputs
            .details
            .government_debt
            .by_residual_maturity
            .iter()
            .map(|b| b.get())
            .collect::<Vec<_>>(),
        "inputs": input_hashes,
        "files": file_hashes,
        "law": Value::Null,
        "provenance": provenance,
        "consistent": scenario.report.consistent(),
    })
}

/// The temporary folder next to `dir`.
fn temporary(dir: &Path) -> Result<PathBuf, BuildError> {
    let name = dir
        .file_name()
        .ok_or_else(|| io_err(format!("{} has no folder name", dir.display())))?;
    let mut tmp = std::ffi::OsString::from(".");
    tmp.push(name);
    tmp.push(".building");
    Ok(dir.with_file_name(tmp))
}

pub(crate) fn write(scenario: &Scenario, inputs: &Inputs, dir: &Path) -> Result<(), BuildError> {
    let files = files(scenario, inputs)?;
    let scenario_file = pretty(&scenario_json(scenario, inputs, &files))?;
    let tmp = temporary(dir)?;
    let result = (|| -> Result<(), BuildError> {
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp).map_err(io_err)?;
        }
        std::fs::create_dir_all(tmp.join("tables")).map_err(io_err)?;
        for (path, bytes) in &files {
            std::fs::write(tmp.join(path), bytes).map_err(io_err)?;
        }
        std::fs::write(tmp.join("scenario.json"), &scenario_file).map_err(io_err)?;
        if dir.exists() {
            std::fs::remove_dir_all(dir).map_err(io_err)?;
        }
        std::fs::rename(&tmp, dir).map_err(io_err)
    })();
    if result.is_err() && tmp.exists() {
        // Best effort: the error that matters is the one returned.
        let _ = std::fs::remove_dir_all(&tmp);
    }
    result
}
