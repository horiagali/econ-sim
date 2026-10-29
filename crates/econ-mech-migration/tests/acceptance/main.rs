//! Acceptance tests generated from spec criteria (ADR-0014). Protected path:
//! only a test-writer session edits these files.

// Compiled with the crate feature `mechanic`, which the implementation adds to
// `default`.
#[cfg(feature = "mechanic")]
mod migration;
