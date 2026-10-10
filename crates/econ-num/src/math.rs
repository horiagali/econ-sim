//! Transcendental maths through the pinned pure-Rust `libm` (ADR-0006 rule 1).
//!
//! `f64::exp`, `ln`, `powf`, … call the platform C library, which differs
//! between Windows and Linux in the last bits. These wrappers do not.
//! Basic operations (+ − × ÷ sqrt) are IEEE-exact everywhere and need no
//! wrapper.

#[inline]
#[must_use]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}
#[inline]
#[must_use]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}
#[inline]
#[must_use]
pub fn powf(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}
/// Integer power by repeated multiplication (deterministic, unlike `powi`).
#[inline]
#[must_use]
pub fn powi(x: f64, n: i32) -> f64 {
    let mut base = if n < 0 { 1.0 / x } else { x };
    let mut e = n.unsigned_abs();
    let mut acc = 1.0;
    while e > 0 {
        if e & 1 == 1 {
            acc *= base;
        }
        base *= base;
        e >>= 1;
    }
    acc
}
#[inline]
#[must_use]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}
#[inline]
#[must_use]
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}
#[inline]
#[must_use]
pub fn sqrt(x: f64) -> f64 {
    // IEEE-exact; kept here so all maths goes through one module.
    x.sqrt()
}
/// Logistic function 1 / (1 + e^-x).
#[inline]
#[must_use]
pub fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + exp(-x))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// Known-answer vectors: the exact bit patterns from pinned libm 0.2.16.
    /// If this fails after a dependency bump, it is a re-golden event.
    #[test]
    fn known_answers() {
        let cases: [(f64, u64); 4] = [
            (exp(1.0), 0x4005_bf0a_8b14_576a),
            (ln(10.0), 0x4002_6bb1_bbb5_5516),
            (powf(1.5, 2.5), 0x4006_0b9f_d68a_4554),
            (sin(1.0), 0x3fea_ed54_8f09_0cee),
        ];
        for (v, bits) in cases {
            assert_eq!(v.to_bits(), bits, "value {v} bits {:#x}", v.to_bits());
        }
    }

    #[test]
    fn powi_matches() {
        assert_eq!(powi(2.0, 10), 1024.0);
        assert_eq!(powi(2.0, -2), 0.25);
        assert_eq!(powi(3.0, 0), 1.0);
    }
}
