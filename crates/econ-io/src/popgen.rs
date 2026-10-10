//! Population generator I/O (spec `society/population-generator`, ADR-0012):
//! the margin file in, Arrow tables and the fit report out. `econ-popgen`
//! itself does no I/O.
//!
//! | File | Content |
//! |---|---|
//! | `households.arrow` | `hh_weight` (u32), `hh_county` (u8, index into `counties`), `hh_collective` (bool) |
//! | `persons.arrow` | `household_id` (u32), `age` (u16, months), `sex` (u8: F 0, M 1), `role` (u8: head 0, partner 1, child 2, other 3) |
//! | `fit_report.json` | parameters, table sizes, weighted totals, state hash, fit report, county codes, provenance of the margins |

use std::path::Path;
use std::sync::Arc;

use arrow_array::{ArrayRef, BooleanArray, UInt8Array, UInt16Array, UInt32Array};
use arrow_schema::DataType;
use econ_core::scale_spike::SeedPopulation;
use econ_popgen::{GenParams, Margins, OPEN, Population};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{SaveError, batch, io_err, ipc_bytes};

/// A margin file: the generator's input tables plus where they came from.
#[derive(Debug, Clone, PartialEq)]
pub struct MarginFile {
    /// The tables.
    pub margins: Margins,
    /// The file's `provenance` block, passed through to the fit report.
    pub provenance: Value,
}

/// The JSON layout written by `python/pipeline/normalise_census.py`.
#[derive(Deserialize)]
struct RawMargins {
    counties: Vec<String>,
    sexes: Vec<String>,
    age_bands: Vec<(u16, Option<u16>)>,
    size_classes: Vec<(u16, Option<u16>)>,
    persons_private: Vec<Vec<Vec<u64>>>,
    persons_collective: Vec<Vec<Vec<u64>>>,
    households: Vec<Vec<u64>>,
    #[serde(default)]
    provenance: Value,
}

/// Parse a margin file. Ragged tables are rejected here, so a wrong row
/// cannot hide behind a flat table of the right total length.
///
/// # Errors
/// `SaveError::Parse` if the JSON is not a margin file.
pub fn read_margins(json: &str) -> Result<MarginFile, SaveError> {
    let parse_err = |what: &str| SaveError::Parse(format!("margin file: {what}"));
    let raw: RawMargins = serde_json::from_str(json).map_err(|e| parse_err(&e.to_string()))?;
    if raw.sexes != ["F", "M"] {
        return Err(parse_err("sexes must be [\"F\", \"M\"]"));
    }
    let (n_c, n_b, n_k) = (
        raw.counties.len(),
        raw.age_bands.len(),
        raw.size_classes.len(),
    );
    let persons = |table: Vec<Vec<Vec<u64>>>, name: &str| {
        let regular = table.len() == n_c
            && table
                .iter()
                .all(|c| c.len() == 2 && c.iter().all(|s| s.len() == n_b));
        if regular {
            Ok(table.into_iter().flatten().flatten().collect::<Vec<u64>>())
        } else {
            Err(parse_err(&format!("{name} is not [county][sex][age_band]")))
        }
    };
    let persons_private = persons(raw.persons_private, "persons_private")?;
    let persons_collective = persons(raw.persons_collective, "persons_collective")?;
    if raw.households.len() != n_c || raw.households.iter().any(|c| c.len() != n_k) {
        return Err(parse_err("households is not [county][size_class]"));
    }
    let open = |pairs: Vec<(u16, Option<u16>)>| -> Vec<(u16, u16)> {
        pairs
            .into_iter()
            .map(|(lo, hi)| (lo, hi.unwrap_or(OPEN)))
            .collect()
    };
    Ok(MarginFile {
        margins: Margins {
            counties: raw.counties,
            age_bands: open(raw.age_bands),
            size_classes: open(raw.size_classes),
            persons_private,
            persons_collective,
            households: raw.households.into_iter().flatten().collect(),
        },
        provenance: raw.provenance,
    })
}

/// The columns of a generated population that the simulation starts from.
#[must_use]
pub fn seed_population(pop: &Population) -> SeedPopulation<'_> {
    SeedPopulation {
        hh_weight: &pop.households.hh_weight,
        hh_county: &pop.households.hh_county,
        household_id: &pop.persons.household_id,
        age_months: &pop.persons.age,
    }
}

/// The households table as Arrow IPC bytes.
///
/// # Errors
/// On Arrow encoding failures.
pub fn households_table(pop: &Population) -> Result<Vec<u8>, SaveError> {
    let hh = &pop.households;
    ipc_bytes(&batch(vec![
        (
            "hh_weight",
            DataType::UInt32,
            Arc::new(UInt32Array::from(hh.hh_weight.clone())) as ArrayRef,
        ),
        (
            "hh_county",
            DataType::UInt8,
            Arc::new(UInt8Array::from(hh.hh_county.clone())),
        ),
        (
            "hh_collective",
            DataType::Boolean,
            Arc::new(BooleanArray::from(hh.hh_collective.clone())),
        ),
    ])?)
}

/// The persons table as Arrow IPC bytes.
///
/// # Errors
/// On Arrow encoding failures.
pub fn persons_table(pop: &Population) -> Result<Vec<u8>, SaveError> {
    let p = &pop.persons;
    let codes = |it: &mut dyn Iterator<Item = u8>| Arc::new(UInt8Array::from_iter_values(it));
    ipc_bytes(&batch(vec![
        (
            "household_id",
            DataType::UInt32,
            Arc::new(UInt32Array::from(p.household_id.clone())) as ArrayRef,
        ),
        (
            "age",
            DataType::UInt16,
            Arc::new(UInt16Array::from(p.age.clone())),
        ),
        (
            "sex",
            DataType::UInt8,
            codes(&mut p.sex.iter().map(|&s| s as u8)),
        ),
        (
            "role",
            DataType::UInt8,
            codes(&mut p.role.iter().map(|&r| r as u8)),
        ),
    ])?)
}

/// The fit report: what was generated, from what, and how well it fits.
#[must_use]
pub fn fit_report(file: &MarginFile, params: &GenParams, pop: &Population) -> Value {
    let hh = &pop.households;
    let weight = |h: usize| u64::from(hh.hh_weight[h]);
    let weighted_households: u64 = (0..hh.hh_weight.len())
        .filter(|&h| !hh.hh_collective[h])
        .map(weight)
        .sum();
    let weighted_persons: u64 = pop
        .persons
        .household_id
        .iter()
        .map(|&h| weight(h as usize))
        .sum();
    json!({
        "sample_scale": params.sample_scale,
        "rng_seed": params.rng_seed,
        "max_household_size": params.max_household_size,
        "raking_tolerance_ppm": params.raking_tolerance_ppm,
        "max_sweeps": params.max_sweeps,
        "min_cell_records": params.min_cell_records,
        "households": hh.hh_weight.len(),
        "persons": pop.persons.household_id.len(),
        "weighted_households": weighted_households,
        "weighted_persons": weighted_persons,
        "state_hash": format!("{:016x}", pop.state_hash()),
        "report": {
            "sweeps": pop.report.sweeps,
            "converged": pop.report.converged,
            "max_error_ppm": pop.report.max_error_ppm,
            "unfitted_cells": pop.report.unfitted_cells,
        },
        "counties": file.margins.counties,
        "provenance": file.provenance,
    })
}

/// Write `households.arrow`, `persons.arrow` and `fit_report.json` into `dir`
/// (created if missing).
///
/// # Errors
/// On encoding or file-system failures.
pub fn write_population(
    dir: &Path,
    file: &MarginFile,
    params: &GenParams,
    pop: &Population,
) -> Result<(), SaveError> {
    let report = serde_json::to_vec_pretty(&fit_report(file, params, pop))
        .map_err(|e| SaveError::Parse(e.to_string()))?;
    std::fs::create_dir_all(dir).map_err(io_err)?;
    std::fs::write(dir.join("households.arrow"), households_table(pop)?).map_err(io_err)?;
    std::fs::write(dir.join("persons.arrow"), persons_table(pop)?).map_err(io_err)?;
    std::fs::write(dir.join("fit_report.json"), report).map_err(io_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_batch;
    use arrow_array::Array;

    /// Two counties, three age bands, two size classes.
    const TINY: &str = r#"{
        "counties": ["A", "B"], "sexes": ["F", "M"],
        "age_bands": [[0, 20], [20, 65], [65, null]],
        "size_classes": [[1, 1], [2, null]],
        "persons_private": [[[30, 90, 30], [30, 90, 30]], [[20, 60, 20], [20, 60, 20]]],
        "persons_collective": [[[0, 4, 6], [0, 5, 5]], [[0, 0, 0], [0, 0, 0]]],
        "households": [[60, 100], [40, 60]],
        "provenance": {"note": "test"}
    }"#;

    /// The scale world on the Romanian starting population (Census 2021
    /// fixture, 1:1000, seed 42), 12 ticks. A change here is a re-golden
    /// event (DETERMINISM.md): explain it in CHANGELOG-sim.md.
    #[test]
    fn golden_romania_12_ticks_at_1_in_1000() {
        use econ_core::scale_spike::ScaleWorld;
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../python/pipeline/fixtures/census2021_margins_ro.json"
        );
        let file = read_margins(&std::fs::read_to_string(path).unwrap()).unwrap();
        let pop = econ_popgen::generate(&file.margins, &GenParams::new(1000, 42)).unwrap();
        let mut w = ScaleWorld::from_population(1000, 42, &seed_population(&pop)).unwrap();
        // Every resident of the census is in the world, at any scale.
        assert_eq!(w.real_persons(), 19_053_815);
        assert_eq!(w.n_households(), pop.households.hh_weight.len());
        let mut last = None;
        for _ in 0..12 {
            let r = w.step();
            assert!(r.ledger_ok && r.clearing_ok);
            last = Some(r);
        }
        let r = last.unwrap();
        let got = (w.state_hash(), r.employed, r.unemployed, r.vat.get());
        assert_eq!(
            got, GOLDEN_ROMANIA_12,
            "Romanian scale-world golden changed: {got:?}"
        );
    }

    /// (state hash, employed, unemployed, VAT in tick 12).
    const GOLDEN_ROMANIA_12: (u64, u64, u64, i64) =
        (0xc050_2b54_1f4a_bd0b, 10_577_737, 641_162, 979_111_507_848);

    #[test]
    fn margin_file_is_parsed_row_major() {
        let f = read_margins(TINY).unwrap();
        let m = &f.margins;
        assert_eq!(m.counties, ["A", "B"]);
        assert_eq!(m.age_bands, [(0, 20), (20, 65), (65, OPEN)]);
        assert_eq!(m.size_classes, [(1, 1), (2, OPEN)]);
        assert_eq!(m.persons_private[..6], [30, 90, 30, 30, 90, 30]);
        assert_eq!(m.persons_collective[..6], [0, 4, 6, 0, 5, 5]);
        assert_eq!(m.households, [60, 100, 40, 60]);
        assert_eq!(f.provenance["note"], "test");
    }

    #[test]
    fn bad_margin_files_are_rejected() {
        let ragged = TINY.replace(
            "[[20, 60, 20], [20, 60, 20]]",
            "[[20, 60], [20, 60, 20, 20]]",
        );
        assert!(matches!(read_margins(&ragged), Err(SaveError::Parse(_))));
        let sexes = TINY.replace(r#"["F", "M"]"#, r#"["M", "F"]"#);
        assert!(matches!(read_margins(&sexes), Err(SaveError::Parse(_))));
        assert!(matches!(read_margins("{}"), Err(SaveError::Parse(_))));
    }

    #[test]
    fn tables_round_trip_and_the_report_is_exact() -> Result<(), SaveError> {
        let file = read_margins(TINY).unwrap();
        let params = GenParams::new(10, 3);
        let pop = econ_popgen::generate(&file.margins, &params).unwrap();

        let h = read_batch(households_table(&pop).unwrap()).unwrap();
        let p = read_batch(persons_table(&pop).unwrap()).unwrap();
        assert_eq!(col!(h, "hh_weight", UInt32Array), pop.households.hh_weight);
        assert_eq!(col!(h, "hh_county", UInt8Array), pop.households.hh_county);
        let collective = h.column_by_name("hh_collective").unwrap();
        let collective = collective.as_any().downcast_ref::<BooleanArray>().unwrap();
        assert_eq!(
            (0..collective.len())
                .map(|i| collective.value(i))
                .collect::<Vec<_>>(),
            pop.households.hh_collective
        );
        assert_eq!(
            col!(p, "household_id", UInt32Array),
            pop.persons.household_id
        );
        assert_eq!(col!(p, "age", UInt16Array), pop.persons.age);
        let sex: Vec<u8> = pop.persons.sex.iter().map(|&s| s as u8).collect();
        let role: Vec<u8> = pop.persons.role.iter().map(|&r| r as u8).collect();
        assert_eq!(col!(p, "sex", UInt8Array), sex);
        assert_eq!(col!(p, "role", UInt8Array), role);

        // The report carries the census totals whatever the scale.
        let r = fit_report(&file, &params, &pop);
        assert_eq!(r["weighted_households"], 260);
        assert_eq!(r["weighted_persons"], 520);
        assert_eq!(r["sample_scale"], 10);
        assert_eq!(r["state_hash"], format!("{:016x}", pop.state_hash()));
        assert_eq!(r["provenance"]["note"], "test");
        Ok(())
    }
}
