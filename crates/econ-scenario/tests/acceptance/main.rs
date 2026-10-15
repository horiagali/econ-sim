//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session (`ECON_TEST_AUTHORING=1`) edits these files.

// The opening balance sheet (economy/opening-balance-sheet) and the scenario
// builder (game/scenario-builder), compiled with the crate features `opening`
// and `builder`, which the implementation adds to `default`.
#[cfg(feature = "builder")]
mod builder;
#[cfg(feature = "opening")]
mod opening;
#[cfg(feature = "opening")]
mod support;
#[cfg(feature = "builder")]
mod support_builder;
