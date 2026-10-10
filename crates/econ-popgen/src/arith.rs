//! Integer helpers shared by the seed and fit stages.
//!
//! Everything is `i128`: raking multiplies weights in millionths by targets
//! in millionths, which does not fit 64 bits, and the Python reference uses
//! unbounded integers. Overflow panics in every build (workspace profile).

use std::cmp::Reverse;

/// The integer type of all count and weight arithmetic.
pub(crate) type Int = i128;

/// Weights are raked in millionths of a household.
pub(crate) const UNIT: Int = 1_000_000;

/// A count as `Int`.
pub(crate) fn int(n: usize) -> Int {
    Int::try_from(n).expect("count fits i128")
}

/// A non-negative `Int` as a count.
pub(crate) fn count(n: Int) -> usize {
    usize::try_from(n).expect("non-negative count that fits usize")
}

/// `p / d` rounded half away from zero (`p >= 0`, `d > 0`).
pub(crate) fn rdiv(p: Int, d: Int) -> Int {
    let (q, r) = (p / d, p % d);
    if 2 * r >= d { q + 1 } else { q }
}

/// Split `total` in proportion to `targets` by largest remainder, ties to the
/// lower index. With all-zero targets the first part takes everything.
pub(crate) fn apportion(total: Int, targets: &[Int]) -> Vec<Int> {
    let tsum: Int = targets.iter().sum();
    let mut parts = vec![0; targets.len()];
    if tsum == 0 {
        if let Some(first) = parts.first_mut() {
            *first = total;
        }
        return parts;
    }
    for (part, &t) in parts.iter_mut().zip(targets) {
        *part = total * t / tsum;
    }
    let missing = count(total - parts.iter().sum::<Int>());
    let mut order: Vec<usize> = (0..targets.len()).collect();
    order.sort_by_cached_key(|&i| (Reverse(total * targets[i] % tsum), i));
    for &i in order.iter().take(missing) {
        parts[i] += 1;
    }
    parts
}

/// `n` integer sizes in `[lo, hi]` around the average `num / den`: both
/// neighbours of a fractional average are present when `n >= 2`, the smaller
/// sizes first.
pub(crate) fn straddle(n: usize, num: Int, den: Int, lo: Int, hi: Int) -> Vec<Int> {
    if num <= lo * den {
        return vec![lo; n];
    }
    if num >= hi * den {
        return vec![hi; n];
    }
    let (f, frac) = (num / den, num % den);
    if frac == 0 {
        return vec![f; n];
    }
    if n == 1 {
        return vec![if 2 * frac >= den { f + 1 } else { f }];
    }
    let n_hi = count(rdiv(int(n) * frac, den)).clamp(1, n - 1);
    let mut sizes = vec![f; n - n_hi];
    sizes.resize(n, f + 1);
    sizes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rdiv_rounds_half_up() {
        assert_eq!(rdiv(5, 2), 3);
        assert_eq!(rdiv(4, 3), 1);
        assert_eq!(rdiv(5, 3), 2);
        assert_eq!(rdiv(0, 7), 0);
    }

    #[test]
    fn apportion_is_exact_and_breaks_ties_low() {
        assert_eq!(apportion(10, &[1, 1, 1]), vec![4, 3, 3]);
        assert_eq!(apportion(7, &[0, 5, 5]), vec![0, 4, 3]);
        assert_eq!(apportion(3, &[0, 0]), vec![3, 0]);
        assert_eq!(apportion(100, &[50, 30, 20]), vec![50, 30, 20]);
        assert!(apportion(5, &[]).is_empty());
    }

    #[test]
    fn straddle_brackets_the_average() {
        // Average 6.99 over 100 households: sizes 6 and 7, mostly 7.
        let s = straddle(100, 699, 100, 6, 10);
        assert_eq!(s.iter().filter(|&&x| x == 6).count(), 1);
        assert_eq!(s.iter().filter(|&&x| x == 7).count(), 99);
        assert_eq!(s[0], 6);
        // Two households always get one of each neighbour.
        assert_eq!(straddle(2, 601, 100, 6, 10), vec![6, 7]);
        assert_eq!(straddle(1, 649, 100, 6, 10), vec![6]);
        assert_eq!(straddle(1, 650, 100, 6, 10), vec![7]);
        assert_eq!(straddle(3, 7, 1, 6, 10), vec![7, 7, 7]);
        assert_eq!(straddle(3, -4, 1, 6, 10), vec![6, 6, 6]);
        assert_eq!(straddle(3, 99, 1, 6, 10), vec![10, 10, 10]);
    }
}
