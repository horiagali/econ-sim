//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session (`ECON_TEST_AUTHORING=1`) edits these files.

// The starting budget (economy/budget-2021), compiled with the crate feature
// `budget`, which the implementation adds to `default`.
#[cfg(feature = "budget")]
mod budget;
#[cfg(feature = "budget")]
mod support_budget;

// M6: the history files (game/history-files, AC-HIST-01 to 05), written
// before the code against the test writer's API skeleton
// `econ_io::history`, behind the feature `history`, which the
// implementation adds to `default` once they pass. AC-HIST-05 is
// release-only (ignored under debug_assertions).
#[cfg(feature = "history")]
mod history;
