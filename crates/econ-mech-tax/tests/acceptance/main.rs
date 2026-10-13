//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session (`ECON_TEST_AUTHORING=1`) edits these files.
mod vat;

// The second reduced rate: compiled with the crate feature `reduced-2`, which
// its implementation adds to `default`.
#[cfg(feature = "reduced-2")]
mod vat_reduced_2;
