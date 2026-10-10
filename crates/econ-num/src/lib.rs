//! Deterministic numerics for the simulation core (ADR-0006).
//!
//! * [`math`]: transcendental functions via the pinned pure-Rust `libm`, so
//!   results are bit-identical on Windows and Linux.
//! * [`det_sum`]: float sums in a fixed chunked order, independent of how
//!   many threads computed the chunks.
//! * [`lu`]: a small partial-pivot LU solver for the input-output system.

pub mod lu;
pub mod math;
mod sum;

pub use sum::{CHUNK, det_sum, det_sum_chunks};

/// Assert that a value about to enter simulation state is finite
/// (ADR-0006 rule 2). Returns the value so it can be used inline.
///
/// # Panics
/// If `x` is NaN or infinite.
#[inline]
#[track_caller]
#[must_use]
pub fn finite(x: f64, what: &str) -> f64 {
    assert!(x.is_finite(), "non-finite value for {what}: {x}");
    x
}
