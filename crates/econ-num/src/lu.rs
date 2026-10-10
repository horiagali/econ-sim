#![allow(clippy::needless_range_loop)] // explicit index loops keep the summation order obvious

//! Dense LU decomposition with partial pivoting (ADR-0006).
//!
//! Hand-written, scalar and in a fixed loop order so results are
//! bit-identical across platforms. Sized for the ~90×90 input-output
//! system; no BLAS, no runtime CPU dispatch.

/// An LU factorisation `P·A = L·U` of a square matrix (row-major).
#[derive(Debug, Clone)]
pub struct Lu {
    n: usize,
    lu: Vec<f64>,
    perm: Vec<usize>,
}

/// Errors from factorisation.
#[derive(Debug, Clone, PartialEq)]
pub enum LuError {
    /// The matrix is not square or the data length is wrong.
    BadShape,
    /// A pivot was (near) zero: the matrix is singular.
    Singular { column: usize },
}

impl Lu {
    /// Factorise an `n×n` row-major matrix.
    ///
    /// # Errors
    /// [`LuError::BadShape`] or [`LuError::Singular`].
    pub fn factor(n: usize, a: &[f64]) -> Result<Lu, LuError> {
        if a.len() != n * n {
            return Err(LuError::BadShape);
        }
        let mut lu = a.to_vec();
        let mut perm: Vec<usize> = (0..n).collect();
        for k in 0..n {
            // Pivot: largest |value| in column k at or below row k; ties → lowest row.
            let mut p = k;
            let mut best = lu[k * n + k].abs();
            for i in (k + 1)..n {
                let v = lu[i * n + k].abs();
                if v > best {
                    best = v;
                    p = i;
                }
            }
            if best < 1e-300 {
                return Err(LuError::Singular { column: k });
            }
            if p != k {
                for j in 0..n {
                    lu.swap(k * n + j, p * n + j);
                }
                perm.swap(k, p);
            }
            let pivot = lu[k * n + k];
            for i in (k + 1)..n {
                let f = lu[i * n + k] / pivot;
                lu[i * n + k] = f;
                for j in (k + 1)..n {
                    lu[i * n + j] -= f * lu[k * n + j];
                }
            }
        }
        Ok(Lu { n, lu, perm })
    }

    /// Solve `A·x = b`.
    ///
    /// # Panics
    /// If `b.len() != n`.
    #[must_use]
    pub fn solve(&self, b: &[f64]) -> Vec<f64> {
        let n = self.n;
        assert_eq!(b.len(), n, "rhs length");
        let mut x: Vec<f64> = self.perm.iter().map(|&p| b[p]).collect();
        // Forward substitution (unit lower triangle).
        for i in 0..n {
            let mut s = x[i];
            for j in 0..i {
                s -= self.lu[i * n + j] * x[j];
            }
            x[i] = s;
        }
        // Back substitution.
        for i in (0..n).rev() {
            let mut s = x[i];
            for j in (i + 1)..n {
                s -= self.lu[i * n + j] * x[j];
            }
            x[i] = s / self.lu[i * n + i];
        }
        x
    }
}

/// Solve the Leontief system `(I − A)·x = d` for gross output `x` given final
/// demand `d`, where `a` is the `n×n` row-major technical-coefficient matrix
/// (`a[i*n+j]` = units of input `i` per unit of output `j`).
///
/// # Errors
/// If `I − A` is singular.
pub fn leontief_output(n: usize, a: &[f64], d: &[f64]) -> Result<Vec<f64>, LuError> {
    if a.len() != n * n {
        return Err(LuError::BadShape);
    }
    let mut m = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            let id = if i == j { 1.0 } else { 0.0 };
            m[i * n + j] = id - a[i * n + j];
        }
    }
    Ok(Lu::factor(n, &m)?.solve(d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_small_system() {
        // 2x + y = 3 ; x + 3y = 5  → x = 0.8, y = 1.4
        let lu = Lu::factor(2, &[2.0, 1.0, 1.0, 3.0]).unwrap();
        let x = lu.solve(&[3.0, 5.0]);
        assert!((x[0] - 0.8).abs() < 1e-12 && (x[1] - 1.4).abs() < 1e-12);
    }

    #[test]
    fn singular_detected() {
        assert_eq!(
            Lu::factor(2, &[1.0, 2.0, 2.0, 4.0]).unwrap_err(),
            LuError::Singular { column: 1 }
        );
    }

    #[test]
    fn leontief_90_residual_small_and_reproducible() {
        let n = 90;
        // Deterministic pseudo-random productive matrix (column sums < 0.6).
        let mut a = vec![0.0; n * n];
        let mut s: u64 = 12345;
        for v in &mut a {
            s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            #[allow(clippy::cast_precision_loss)]
            let u = (s >> 11) as f64 / (1u64 << 53) as f64;
            *v = u * 0.6 / 90.0;
        }
        let d: Vec<f64> = (0..n)
            .map(|i| 100.0 + f64::from(u32::try_from(i).unwrap()))
            .collect();
        let x = leontief_output(n, &a, &d).unwrap();
        // Residual check: (I - A) x ≈ d
        for i in 0..n {
            let mut r = x[i];
            for j in 0..n {
                r -= a[i * n + j] * x[j];
            }
            assert!((r - d[i]).abs() < 1e-9, "row {i} residual {}", r - d[i]);
        }
        // Same inputs → same bits.
        let x2 = leontief_output(n, &a, &d).unwrap();
        assert!(x.iter().zip(&x2).all(|(p, q)| p.to_bits() == q.to_bits()));
    }
}
