//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session (`ECON_TEST_AUTHORING=1`) edits these files.

// The `World` (game/world-loop) and the `[sim]` criteria of the four M2
// mechanics on it, compiled with the crate feature `world`, which the
// implementation adds to `default`.
#[cfg(feature = "world")]
mod households;
#[cfg(feature = "world")]
mod labour;
#[cfg(feature = "world")]
mod prices;
#[cfg(feature = "world")]
mod production;
#[cfg(feature = "world")]
mod support;
#[cfg(feature = "world")]
mod world;
#[cfg(feature = "world")]
mod world_golden;
