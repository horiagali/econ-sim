//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session (`ECON_TEST_AUTHORING=1`) edits these files.
mod popgen;

// Stages C, D and E: each stage's tests are compiled with its crate feature,
// which the stage's implementation change adds to `default`.
#[cfg(feature = "attributes")]
mod attributes;
#[cfg(feature = "housing")]
mod housing;
#[cfg(feature = "jobs")]
mod jobs;
#[cfg(any(feature = "attributes", feature = "jobs", feature = "housing"))]
mod support;
