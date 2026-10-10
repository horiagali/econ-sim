//! Money as integer bani (ADR-0007).

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// An amount of money in bani (1 RON = 100 bani).
///
/// Arithmetic is checked and panics on overflow in every build. Amounts can be
/// negative (e.g. a liability balance). Products with rates go through
/// [`Bani::mul_ratio`] (exact, `i128`) or [`Bani::mul_rate`] (one documented
/// rounding rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[must_use]
pub struct Bani(pub i64);

impl Bani {
    /// Zero bani.
    pub const ZERO: Bani = Bani(0);

    /// Construct from whole lei.
    pub fn from_lei(lei: i64) -> Bani {
        Bani(lei.checked_mul(100).expect("Bani overflow in from_lei"))
    }

    /// The raw number of bani.
    #[must_use]
    pub fn get(self) -> i64 {
        self.0
    }

    /// Is this exactly zero?
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Absolute value.
    pub fn abs(self) -> Bani {
        Bani(self.0.checked_abs().expect("Bani overflow in abs"))
    }

    /// Multiply by an integer (e.g. a record weight), checked.
    pub fn times(self, k: i64) -> Bani {
        Bani(self.0.checked_mul(k).expect("Bani overflow in times"))
    }

    /// Exact `self * num / den`, rounded half away from zero, via `i128`.
    ///
    /// When the product fits in `i64` the same result is computed in 64-bit
    /// arithmetic (128-bit division is several times slower, which shows in
    /// per-household loops).
    ///
    /// # Panics
    /// If `den == 0` or the result does not fit in `i64`.
    #[inline]
    #[allow(clippy::arithmetic_side_effects)] // i64×i64 fits in i128; final conversion is checked
    pub fn mul_ratio(self, num: i64, den: i64) -> Bani {
        assert!(den != 0, "mul_ratio: zero denominator");
        if den > 0
            && let Some(p) = self.0.checked_mul(num)
        {
            let q = p / den;
            let r = (p % den).abs();
            // r < den, so `den - r` cannot overflow; r >= den - r  ⇔  2r >= den
            return Bani(if r >= den - r { q + p.signum() } else { q });
        }
        let p = i128::from(self.0) * i128::from(num);
        let d = i128::from(den);
        Bani(round_div_i128(p, d))
    }

    /// `self * rate` rounded half away from zero. **The single rounding rule
    /// for money × float rate** (ADR-0007).
    ///
    /// Exact as long as `|self| < 2^53` bani (≈ 90 trillion lei), which every
    /// per-record and sector amount satisfies; larger amounts panic rather
    /// than silently lose precision.
    pub fn mul_rate(self, rate: f64) -> Bani {
        Bani::round_f64(self.to_f64_exact() * rate)
    }

    /// Convert an `f64` amount of bani to [`Bani`], rounding half away from
    /// zero. This is the only float→money conversion.
    ///
    /// # Panics
    /// If `x` is not finite or out of `i64` range.
    pub fn round_f64(x: f64) -> Bani {
        assert!(x.is_finite(), "Bani::round_f64 got non-finite {x}");
        // f64::round is IEEE-exact (round half away from zero) on every platform.
        let r = x.round();
        assert!(
            (-9.2e18..9.2e18).contains(&r),
            "Bani::round_f64 out of range: {x}"
        );
        #[allow(clippy::cast_possible_truncation)]
        Bani(r as i64)
    }

    /// The value as `f64`, panicking if it is not exactly representable.
    #[must_use]
    pub fn to_f64_exact(self) -> f64 {
        const LIM: i64 = 1 << 53;
        assert!(
            self.0 > -LIM && self.0 < LIM,
            "Bani too large for exact f64: {}",
            self.0
        );
        #[allow(clippy::cast_precision_loss)]
        let v = self.0 as f64;
        v
    }
}

/// Integer division of `p / d` rounded half away from zero.
#[allow(clippy::arithmetic_side_effects)] // operands come from i64×i64; result conversion is checked
fn round_div_i128(p: i128, d: i128) -> i64 {
    let (p, d) = if d < 0 { (-p, -d) } else { (p, d) };
    let q = p / d;
    let r = p % d;
    // |r| * 2 >= d  → round away from zero
    let q = if r.abs() * 2 >= d { q + p.signum() } else { q };
    i64::try_from(q).expect("Bani overflow in mul_ratio")
}

impl Add for Bani {
    type Output = Bani;
    fn add(self, rhs: Bani) -> Bani {
        Bani(self.0.checked_add(rhs.0).expect("Bani overflow in add"))
    }
}
impl Sub for Bani {
    type Output = Bani;
    fn sub(self, rhs: Bani) -> Bani {
        Bani(self.0.checked_sub(rhs.0).expect("Bani overflow in sub"))
    }
}
impl Neg for Bani {
    type Output = Bani;
    fn neg(self) -> Bani {
        Bani(self.0.checked_neg().expect("Bani overflow in neg"))
    }
}
impl AddAssign for Bani {
    fn add_assign(&mut self, rhs: Bani) {
        *self = Add::add(*self, rhs);
    }
}
impl SubAssign for Bani {
    fn sub_assign(&mut self, rhs: Bani) {
        *self = Sub::sub(*self, rhs);
    }
}
impl Sum for Bani {
    fn sum<I: Iterator<Item = Bani>>(iter: I) -> Bani {
        iter.fold(Bani::ZERO, Add::add)
    }
}
impl<'a> Sum<&'a Bani> for Bani {
    fn sum<I: Iterator<Item = &'a Bani>>(iter: I) -> Bani {
        iter.fold(Bani::ZERO, |a, b| Add::add(a, *b))
    }
}

impl fmt::Display for Bani {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let a = self.0.unsigned_abs();
        write!(f, "{sign}{}.{:02} RON", a / 100, a % 100)
    }
}

/// Split `total` into parts proportional to `weights` so the parts sum to
/// `total` **exactly** (largest-remainder method, ADR-0007).
///
/// Ties in the remainder are broken by the lower index. All-zero weights
/// put everything on the first part. Negative totals are split by splitting
/// the absolute value and negating.
///
/// # Panics
/// If `weights` is empty.
#[must_use]
#[allow(clippy::arithmetic_side_effects)] // u128 of i64-sized values: no overflow possible
pub fn split_largest_remainder(total: Bani, weights: &[u64]) -> Vec<Bani> {
    assert!(!weights.is_empty(), "split_largest_remainder: no weights");
    if total.0 < 0 {
        return split_largest_remainder(-total, weights)
            .into_iter()
            .map(|b| -b)
            .collect();
    }
    let w_sum: u128 = weights.iter().map(|&w| u128::from(w)).sum();
    let t = u128::from(total.0.unsigned_abs());
    if w_sum == 0 {
        let mut v = vec![Bani::ZERO; weights.len()];
        v[0] = total;
        return v;
    }
    let mut parts: Vec<u128> = Vec::with_capacity(weights.len());
    let mut rems: Vec<(u128, usize)> = Vec::with_capacity(weights.len());
    let mut assigned: u128 = 0;
    for (i, &w) in weights.iter().enumerate() {
        let num = t * u128::from(w);
        let q = num / w_sum;
        parts.push(q);
        rems.push((num % w_sum, i));
        assigned += q;
    }
    let mut left = t - assigned; // < weights.len()
    // Largest remainder first; ties → lower index first.
    rems.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for &(_, i) in &rems {
        if left == 0 {
            break;
        }
        parts[i] += 1;
        left -= 1;
    }
    parts
        .into_iter()
        .map(|p| Bani(i64::try_from(p).expect("split overflow")))
        .collect()
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn display() {
        assert_eq!(Bani(12345).to_string(), "123.45 RON");
        assert_eq!(Bani(-5).to_string(), "-0.05 RON");
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn add_overflow_panics() {
        let _ = Bani(i64::MAX) + Bani(1);
    }

    #[test]
    fn rounding_half_away_from_zero() {
        assert_eq!(Bani::round_f64(2.5), Bani(3));
        assert_eq!(Bani::round_f64(-2.5), Bani(-3));
        assert_eq!(Bani::round_f64(2.4999), Bani(2));
        assert_eq!(Bani(5).mul_ratio(1, 2), Bani(3));
        assert_eq!(Bani(-5).mul_ratio(1, 2), Bani(-3));
        assert_eq!(Bani(7).mul_ratio(1, -2), Bani(-4));
        assert_eq!(Bani(1000).mul_rate(0.19), Bani(190));
    }

    #[test]
    fn split_examples() {
        assert_eq!(
            split_largest_remainder(Bani(100), &[1, 1, 1]),
            vec![Bani(34), Bani(33), Bani(33)]
        );
        assert_eq!(
            split_largest_remainder(Bani(-100), &[1, 1, 1]),
            vec![Bani(-34), Bani(-33), Bani(-33)]
        );
        assert_eq!(
            split_largest_remainder(Bani(10), &[0, 0]),
            vec![Bani(10), Bani(0)]
        );
    }

    proptest! {
        #[test]
        fn split_sums_exactly(total in -1_000_000_000_000i64..1_000_000_000_000,
                              weights in prop::collection::vec(0u64..1_000_000, 1..50)) {
            let parts = split_largest_remainder(Bani(total), &weights);
            prop_assert_eq!(parts.len(), weights.len());
            prop_assert_eq!(parts.iter().sum::<Bani>(), Bani(total));
        }

        #[test]
        /// The 64-bit fast path and the `i128` path give the same result,
        /// including where the product is close to overflowing `i64`.
        fn mul_ratio_fast_path_matches_i128(
            a in prop_oneof![any::<i64>(), -4_000_000_000i64..4_000_000_000],
            n in prop_oneof![any::<i64>(), -4_000_000_000i64..4_000_000_000, -20_000i64..20_000],
            d in prop_oneof![1i64..=i64::MAX, 1i64..20_000, 9_999i64..12_101],
        ) {
            let p = i128::from(a) * i128::from(n);
            let wide = {
                let q = p / i128::from(d);
                let r = (p % i128::from(d)).abs();
                if r * 2 >= i128::from(d) { q + p.signum() } else { q }
            };
            prop_assume!(i64::try_from(wide).is_ok());
            prop_assert_eq!(i128::from(Bani(a).mul_ratio(n, d).0), wide);
            prop_assert_eq!(i64::try_from(wide).unwrap(), round_div_i128(p, i128::from(d)));
        }

        #[test]
        fn mul_ratio_matches_reference(a in -1_000_000_000i64..1_000_000_000, n in -1000i64..1000, d in 1i64..1000) {
            // Reference: exact rational rounding with i128.
            let p = i128::from(a) * i128::from(n);
            let fl = p.div_euclid(i128::from(d));
            let r = p.rem_euclid(i128::from(d));
            // candidates fl and fl+1; pick nearest, ties away from zero
            let twice = 2 * r;
            let dd = i128::from(d);
            let expect = if twice > dd { fl + 1 } else if twice < dd { fl } else if p >= 0 { fl + 1 } else { fl };
            prop_assert_eq!(Bani(a).mul_ratio(n, d).0 as i128, expect);
        }
    }
}
