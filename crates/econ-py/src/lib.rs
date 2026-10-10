//! Python bindings for calibration and analysis (ADR-0009, Spike 5).
//!
//! Runs whole simulations inside Rust and returns plain lists, so a
//! calibration loop pays the Python overhead once per run, not per tick.

use econ_core::scale_spike::{ScaleParams, ScaleWorld};
use econ_core::sim::{SimModel, SimParams};
use econ_types::Bani;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Run the Spike-4 scale world and return monthly series.
///
/// params: matching_efficiency, vacancy_ratio, mpc_income, mpc_wealth,
/// separation_rate (all optional; defaults otherwise).
#[pyfunction]
#[pyo3(signature = (sample_scale, seed, ticks, params=None))]
fn run_scale<'py>(
    py: Python<'py>,
    sample_scale: u32,
    seed: u64,
    ticks: u32,
    params: Option<&Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyDict>> {
    let mut p = ScaleParams::default();
    let mut sep = 0.015;
    if let Some(d) = params {
        let get = |k: &str, dflt: f64| -> PyResult<f64> {
            Ok(match d.get_item(k)? {
                Some(v) => v.extract()?,
                None => dflt,
            })
        };
        p.matching_efficiency = get("matching_efficiency", p.matching_efficiency)?;
        p.vacancy_ratio = get("vacancy_ratio", p.vacancy_ratio)?;
        p.mpc_income = get("mpc_income", p.mpc_income)?;
        p.mpc_wealth = get("mpc_wealth", p.mpc_wealth)?;
        sep = get("separation_rate", sep)?;
    }
    // The simulation itself runs without the GIL.
    let (u, wb, c, vat) = py.detach(|| {
        let mut w = ScaleWorld::generate(sample_scale, seed);
        w.set_params(p, sep);
        let mut u = Vec::with_capacity(ticks as usize);
        let mut wb = Vec::with_capacity(ticks as usize);
        let mut c = Vec::with_capacity(ticks as usize);
        let mut vat = Vec::with_capacity(ticks as usize);
        for _ in 0..ticks {
            let r = w.step();
            #[allow(clippy::cast_precision_loss)]
            u.push(r.unemployed as f64 / (r.employed + r.unemployed).max(1) as f64);
            wb.push(r.wage_bill.get());
            c.push(r.consumption.get());
            vat.push(r.vat.get());
        }
        (u, wb, c, vat)
    });
    let out = PyDict::new(py);
    out.set_item("unemployment_rate", u)?;
    out.set_item("wage_bill_bani", wb)?;
    out.set_item("consumption_bani", c)?;
    out.set_item("vat_bani", vat)?;
    Ok(out)
}

/// Run the Godley–Lavoie SIM model; returns national income Y per tick (bani).
#[pyfunction]
#[pyo3(signature = (ticks, g_lei=20_000_000, theta=0.2, alpha1=0.6, alpha2=0.4))]
fn run_sim(ticks: u32, g_lei: i64, theta: f64, alpha1: f64, alpha2: f64) -> Vec<i64> {
    let mut m = SimModel::new(SimParams {
        g: Bani::from_lei(g_lei),
        theta,
        alpha1,
        alpha2,
    });
    (0..ticks)
        .map(|_| m.step(&[]).get("Y").map_or(0, Bani::get))
        .collect()
}

#[pymodule]
fn econ_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run_scale, m)?)?;
    m.add_function(wrap_pyfunction!(run_sim, m)?)?;
    Ok(())
}
