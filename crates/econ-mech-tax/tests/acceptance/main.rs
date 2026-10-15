//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session (`ECON_TEST_AUTHORING=1`) edits these files.
mod vat;

// The second reduced rate: compiled with the crate feature `reduced-2`, which
// its implementation adds to `default`.
#[cfg(feature = "reduced-2")]
mod vat_reduced_2;

// The taxes on income and wages (AC-TAX-01..07): compiled with the crate
// feature `taxation`, which their implementation adds to `default`.
#[cfg(feature = "taxation")]
mod taxation;
