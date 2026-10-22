//! Commands files of headless play (spec `game/lever-system`, "Headless
//! play"): JSON lines in the form of the command log, `seq` and `effective`
//! optional, values as integers or in display units (`"21%"`, `"2,550 lei"`).
//!
//! **API skeleton only** (ADR-0014): implemented after the acceptance tests.
//! Behind the feature `levers`, off by default.

use std::path::Path;

use econ_core::levers::{LawInForce, LogLine, Registry, Rejection};
use econ_core::world::World;

use crate::SaveError;

/// Read and validate a whole commands file before the first day runs.
///
/// # Errors
/// The first rejected line: its line number (from 1) and the rejection.
pub fn read_commands(path: &Path, registry: &Registry) -> Result<Vec<LogLine>, (usize, Rejection)> {
    let _ = (path, registry);
    unimplemented!("ADR-0014: implemented after the acceptance tests")
}

// --- Test writer's additions (ADR-0014) ----------------------------------------

/// The message of every body the test writer added.
const TEST_WRITER: &str = "test writer's addition (ADR-0014): the implementation fills it";

/// The law of a scenario folder.
#[derive(Debug, Clone)]
pub struct ScenarioLaw {
    /// The registry, its keyed levers' `keys` filled from the scenario's
    /// tables (industries, budget lines, public units, …).
    pub registry: Registry,
    /// The lines of `law.json` then `world.json`, in file order.
    pub lines: Vec<LogLine>,
    /// The law in force they set.
    pub law: LawInForce,
}

/// Read a scenario folder's `law.json` and `world.json` against the registry
/// (spec, "Default"; ADR-0018).
///
/// # Errors
/// A line rejected (its line number from 1 and the rejection), or an active
/// lever or key the files do not set (line 0, the rejection naming it).
pub fn read_scenario_law(
    dir: &Path,
    registry: &Registry,
) -> Result<ScenarioLaw, (usize, Rejection)> {
    let _ = (dir, registry);
    unimplemented!("{TEST_WRITER}")
}

/// A save of a `World` under the lever system (spec, "Logging and
/// deterministic replay"; ADR-0011): bytes holding what loading needs to go
/// on, the law in force, the pending list and the command log included.
///
/// # Errors
/// On encoding failures.
pub fn save_world(world: &World) -> Result<Vec<u8>, SaveError> {
    let _ = world;
    unimplemented!("{TEST_WRITER}")
}

/// Load a save of [`save_world`] made on a world of the scenario folder `dir`
/// (`io_table`: the input-output table, as for `read_start_state`).
///
/// # Errors
/// A damaged save, or one of another scenario or engine.
pub fn load_world(bytes: &[u8], dir: &Path, io_table: &Path) -> Result<World, SaveError> {
    let _ = (bytes, dir, io_table);
    unimplemented!("{TEST_WRITER}")
}
