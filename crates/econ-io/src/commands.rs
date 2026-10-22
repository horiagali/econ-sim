//! Commands files of headless play (spec `game/lever-system`, "Headless
//! play"): JSON lines in the form of the command log, `seq` and `effective`
//! optional, values as integers or in display units (`"21%"`, `"2,550 lei"`).
//!
//! **API skeleton only** (ADR-0014): implemented after the acceptance tests.
//! Behind the feature `levers`, off by default.

use std::path::Path;

use econ_core::levers::{LogLine, Registry, Rejection};

/// Read and validate a whole commands file before the first day runs.
///
/// # Errors
/// The first rejected line: its line number (from 1) and the rejection.
pub fn read_commands(path: &Path, registry: &Registry) -> Result<Vec<LogLine>, (usize, Rejection)> {
    let _ = (path, registry);
    unimplemented!("ADR-0014: implemented after the acceptance tests")
}
