//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session (`ECON_TEST_AUTHORING=1`) edits these files.

// The opening balance sheet (economy/opening-balance-sheet) and the scenario
// builder (game/scenario-builder), compiled with the crate features `opening`
// and `builder`, which the implementation adds to `default`.
#[cfg(feature = "builder")]
mod builder;
// Undeclared sales and opening inventories of the firm units
// (economy/firm-units-undeclared-inventories), with the feature `firm-opening`.
#[cfg(feature = "firm-opening")]
mod firm_units;
#[cfg(feature = "opening")]
mod opening;
#[cfg(feature = "opening")]
mod support;
#[cfg(feature = "builder")]
mod support_builder;
#[cfg(feature = "firm-opening")]
mod support_fui;
