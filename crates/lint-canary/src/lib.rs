//! This crate must FAIL clippy. If it passes, the determinism deny-lists
//! in clippy.toml are not being applied (ADR-0006).
#![deny(clippy::disallowed_types, clippy::disallowed_methods)]

use std::collections::HashMap;

pub fn bad(x: f64) -> f64 {
    let mut m: HashMap<u32, f64> = HashMap::new();
    m.insert(1, x.exp());
    m[&1]
}
