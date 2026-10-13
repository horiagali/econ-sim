//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session (`ECON_TEST_AUTHORING=1`) edits these files.
mod accounting;

// AC-ACC-04 needs the ledger to reject unknown flow codes: compiled with the
// crate feature `flow-code-check`, which that change adds to `default`.
#[cfg(feature = "flow-code-check")]
mod flow_codes;
