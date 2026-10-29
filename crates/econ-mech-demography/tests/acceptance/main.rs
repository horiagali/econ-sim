//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session edits these files.

// Compiled with the crate features `demography` and `lifecycle`, which the
// implementation adds to `default`.
#[cfg(feature = "demography")]
mod demography;
#[cfg(feature = "lifecycle")]
mod lifecycle;
