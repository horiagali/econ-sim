//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session (`ECON_TEST_AUTHORING=1`) edits these files.

// The starting budget (economy/budget-2021), compiled with the crate feature
// `budget`, which the implementation adds to `default`.
#[cfg(feature = "budget")]
mod budget;
#[cfg(feature = "budget")]
mod support_budget;
