//! Tauri 2 host (Spike 8, ADR-0013). The simulation core is linked in-process
//! as an ordinary crate; commands return binary data where it is large. The
//! thin UI of roadmap M2 runs the real `World` (`world.rs`); the county map
//! and the "why" tree are still the spike's placeholders.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::BTreeMap;

use serde::Serialize;

mod world;

/// One value per county ISO code (placeholder values until the real model
/// produces county indicators).
#[tauri::command]
fn county_values(indicator: String) -> BTreeMap<String, f64> {
    const CODES: [&str; 42] = [
        "AB", "AR", "AG", "BC", "BH", "BN", "BT", "BV", "BR", "B", "BZ", "CS", "CL", "CJ", "CT",
        "CV", "DB", "DJ", "GL", "GR", "GJ", "HR", "HD", "IL", "IS", "IF", "MM", "MH", "MS", "NT",
        "OT", "PH", "SM", "SJ", "SB", "SV", "TR", "TM", "TL", "VS", "VL", "VN",
    ];
    let salt = indicator.len() as f64;
    CODES
        .iter()
        .enumerate()
        .map(|(i, c)| (format!("RO-{c}"), 3.0 + ((i as f64 * 7.3 + salt) % 9.0)))
        .collect()
}

#[derive(Serialize)]
struct Node {
    label: String,
    value: f64,
    children: Vec<Node>,
}

fn to_node(t: &econ_rules::ContributionTree) -> Node {
    Node {
        label: t.label.clone(),
        value: t.value,
        children: t.children.iter().map(to_node).collect(),
    }
}

econ_rules::behaviour_rule! {
    /// Target price p* = unit_cost · markup · tax (log-linear, explained by LMDI).
    pub rule TargetPrice: log_linear {
        unit_cost: "unit cost" [FromData],
        markup: "markup (demand pressure)" [Tuned],
        tax: "VAT and excise" [FromData],
    }
}

/// A real explanation tree from the `behaviour_rule!` machinery.
#[tauri::command]
fn explain(indicator: String) -> Node {
    let p = TargetPrice::Params {
        scale: 1.0,
        unit_cost: 1.0,
        markup: 1.0,
        tax: 1.0,
    };
    let a = TargetPrice::Inputs {
        unit_cost: 80.0,
        markup: 1.2,
        tax: 1.19,
    };
    let b = TargetPrice::Inputs {
        unit_cost: 88.0,
        markup: 1.15,
        tax: 1.21,
    };
    let mut n = to_node(&TargetPrice::explain_change(&p, &a, &b));
    n.label = format!("{indicator}: price change");
    n
}

/// `bytes` zero bytes with no computation: isolates the IPC round-trip cost
/// (Spike 8 exit criterion, measured for 1–5 MB payloads).
#[tauri::command]
fn ipc_probe(bytes: u32) -> tauri::ipc::Response {
    tauri::ipc::Response::new(vec![0u8; bytes as usize])
}

fn main() {
    tauri::Builder::default()
        .manage(world::Held::default())
        .invoke_handler(tauri::generate_handler![
            world::world_open,
            world::world_step,
            world::world_series,
            world::world_series_names,
            county_values,
            explain,
            ipc_probe
        ])
        .run(tauri::generate_context!())
        .expect("error while running the econ-sim app");
}
