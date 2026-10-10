//! Dealing the members of a group into categories: the "split", "deal" and
//! "balance" of the specs of stages C, D and E, and what else those stages share.
//!
//! Integer arithmetic only, as in `arith`. Each function mirrors the one of
//! the same name in `python/reference/popgen_reference.py`.

use crate::OPEN;
use crate::arith::{Int, UNIT, apportion, rdiv};

/// Category of each member of an ordered group, so that the weighted size of
/// every category is proportional to `targets` (stage C).
///
/// The group's weight is apportioned over the targets and the `carry` of
/// earlier groups is added: that is what each category is owed. The weight is
/// apportioned again over what is owed (a negative amount, or any amount for a
/// category with a zero target, counts as nothing), and a member belongs to the
/// category in whose share the midpoint of its own weight falls. `carry`
/// becomes what each category is still owed.
pub(crate) fn split(weights: &[Int], targets: &[Int], carry: &mut [Int]) -> Vec<usize> {
    let total: Int = weights.iter().sum();
    let want: Vec<Int> = apportion(total, targets)
        .iter()
        .zip(carry.iter())
        .map(|(t, c)| t + c)
        .collect();
    let wanted: Vec<Int> = want
        .iter()
        .zip(targets)
        .map(|(&v, &t)| if t > 0 { v.max(0) } else { 0 })
        .collect();
    let shares = if wanted.iter().any(|&v| v != 0) {
        apportion(total, &wanted)
    } else {
        apportion(total, targets)
    };
    let mut got: Vec<Int> = vec![0; shares.len()];
    let mut out = Vec::with_capacity(weights.len());
    let (mut k, mut done, mut bound) = (0, 0, shares.first().copied().unwrap_or(0));
    for &w in weights {
        while k + 1 < shares.len() && 2 * done + w >= 2 * bound {
            k += 1;
            bound += shares[k];
        }
        out.push(k);
        got[k] += w;
        done += w;
    }
    for ((c, v), g) in carry.iter_mut().zip(want).zip(got) {
        *c = v - g;
    }
    out
}

/// Category of each member of an ordered group, for groups that may hold a
/// single member (stages D and E).
///
/// What each category is owed is found as in [`split`]. Each member in turn
/// goes to the category that is still owed the most (the lowest index on a
/// tie), among those with a target above zero. `carry` becomes what is still
/// owed, positive or negative.
pub(crate) fn deal(weights: &[Int], targets: &[Int], carry: &mut [Int]) -> Vec<usize> {
    let total: Int = weights.iter().sum();
    let mut want: Vec<Int> = apportion(total, targets)
        .iter()
        .zip(carry.iter())
        .map(|(t, c)| t + c)
        .collect();
    let mut open_to: Vec<usize> = (0..targets.len()).filter(|&k| targets[k] > 0).collect();
    if open_to.is_empty() {
        open_to = (0..targets.len()).collect();
    }
    let mut out = Vec::with_capacity(weights.len());
    for &w in weights {
        let mut best = open_to[0];
        for &k in &open_to[1..] {
            if want[k] > want[best] {
                best = k;
            }
        }
        out.push(best);
        want[best] -= w;
    }
    carry.copy_from_slice(&want);
    out
}

/// A table with the proportions of `pattern`, whose rows sum exactly to
/// `row_weights` and whose columns follow `col_targets` (apportioned to the
/// total weight) as closely as the rows allow.
///
/// Two-way balancing in millionths: at most `passes` alternating passes
/// (columns, then rows), then each row is apportioned to its exact weight.
/// Rows with no weight are all zero. With `stop_ppm`, the passes stop after a
/// row pass once every column is within that many millionths of the total
/// weight of its target.
pub(crate) fn balance(
    pattern: &[Vec<Int>],
    row_weights: &[Int],
    col_targets: &[Int],
    passes: u32,
    stop_ppm: Option<u32>,
) -> Vec<Vec<Int>> {
    let n_k = col_targets.len();
    let mut t: Vec<Vec<Int>> = pattern
        .iter()
        .zip(row_weights)
        .map(|(row, &w)| {
            if w == 0 {
                vec![0; n_k]
            } else {
                row.iter().map(|v| v * UNIT).collect()
            }
        })
        .collect();
    let total: Int = row_weights.iter().sum();
    let cols = apportion(total, col_targets);
    let col_sum = |t: &[Vec<Int>], k: usize| -> Int { t.iter().map(|row| row[k]).sum() };
    for _ in 0..passes {
        for k in 0..n_k {
            let cs = col_sum(&t, k);
            if cs != 0 {
                for row in &mut t {
                    row[k] = rdiv(row[k] * cols[k] * UNIT, cs);
                }
            }
        }
        for (row, &w) in t.iter_mut().zip(row_weights) {
            let rs: Int = row.iter().sum();
            if rs != 0 {
                for v in row.iter_mut() {
                    *v = rdiv(*v * w * UNIT, rs);
                }
            }
        }
        if let Some(ppm) = stop_ppm {
            let slack = Int::from(ppm) * total;
            if (0..n_k).all(|k| (col_sum(&t, k) - cols[k] * UNIT).abs() <= slack) {
                break;
            }
        }
    }
    t.iter()
        .zip(row_weights)
        .map(|(row, &w)| apportion(w, row))
        .collect()
}

/// How far [`balance`] goes in stages D and E.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Balancing {
    /// Persons added to every pattern cell, so no combination is impossible.
    pub pattern_floor: u64,
    /// Upper bound on alternating passes.
    pub passes: u32,
    /// Passes stop once every column is within this many millionths of the
    /// total weight of its target.
    pub stop_ppm: u32,
}

/// [`balance`] a pattern against the weight of each row's members and the
/// column targets, then [`deal`] the members row by row over the columns, with
/// their row of the balanced table as targets. `put(member, column)` records
/// the result; the carry passes from row to row.
pub(crate) fn deal_table(
    rows: &[Vec<usize>],
    weight: impl Fn(usize) -> Int,
    pattern: impl Iterator<Item = Vec<Int>>,
    col_targets: &[Int],
    balancing: Balancing,
    carry: &mut [Int],
    mut put: impl FnMut(usize, usize),
) {
    let floor = Int::from(balancing.pattern_floor);
    let pattern: Vec<Vec<Int>> = pattern
        .map(|row| row.iter().map(|v| v + floor).collect())
        .collect();
    let weights: Vec<Vec<Int>> = rows
        .iter()
        .map(|row| row.iter().map(|&i| weight(i)).collect())
        .collect();
    let row_weights: Vec<Int> = weights.iter().map(|w| w.iter().sum()).collect();
    let table = balance(
        &pattern,
        &row_weights,
        col_targets,
        balancing.passes,
        Some(balancing.stop_ppm),
    );
    for ((row, weights), targets) in rows.iter().zip(&weights).zip(&table) {
        if row.is_empty() {
            continue;
        }
        for (&i, k) in row.iter().zip(deal(weights, targets, carry)) {
            put(i, k);
        }
    }
}

/// Index of the age band `[from, to)` that holds `years`; the last band may be
/// open (`to == OPEN`).
pub(crate) fn band_of(bands: &[(u16, u16)], years: u16) -> Option<usize> {
    bands
        .iter()
        .position(|&(from, to)| years >= from && (to == OPEN || years < to))
}

/// The region of every household. `Err` holds the index of the first county
/// without a region: one whose entry in `region_of_county` is not a region, or
/// a household's county that the map does not cover.
pub(crate) fn household_regions(
    hh_county: &[u8],
    region_of_county: &[u8],
    n_regions: usize,
) -> Result<Vec<usize>, u8> {
    if let Some(county) = region_of_county
        .iter()
        .position(|&r| usize::from(r) >= n_regions)
    {
        return Err(u8::try_from(county).unwrap_or(u8::MAX));
    }
    hh_county
        .iter()
        .map(|&c| {
            region_of_county
                .get(usize::from(c))
                .map(|&r| usize::from(r))
                .ok_or(c)
        })
        .collect()
}

/// A margin table as `Int`.
pub(crate) fn ints(table: &[u64]) -> Vec<Int> {
    table.iter().map(|&v| Int::from(v)).collect()
}

/// Sum of the rows of a row-major table that is `width` wide: one total per column.
pub(crate) fn column_sums(table: &[Int], width: usize) -> Vec<Int> {
    let mut out = vec![0; width];
    for row in table.chunks_exact(width) {
        for (sum, v) in out.iter_mut().zip(row) {
            *sum += v;
        }
    }
    out
}

/// FNV-1a 64 over a record count (`u32`, little-endian) and then the bytes of
/// the records: the hash of a stage's columns.
pub(crate) fn column_hash(records: usize, bytes: impl Iterator<Item = u8>) -> u64 {
    const OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01B3;
    let len = u32::try_from(records)
        .expect("table length fits u32")
        .to_le_bytes();
    len.into_iter()
        .chain(bytes)
        .fold(OFFSET, |hash, b| (hash ^ u64::from(b)).wrapping_mul(PRIME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_follows_the_targets_and_carries_the_rest() {
        // Ten members of weight 1, targets 3 : 1.
        let mut carry = [0, 0];
        let got = split(&[1; 10], &[3, 1], &mut carry);
        // 10 * 3/4 = 7.5 and 2.5: largest remainder gives 8 and 2 (tie to the lower index).
        assert_eq!(got.iter().filter(|&&k| k == 0).count(), 8);
        assert_eq!(carry, [0, 0]);
        // A category too small for a whole member waits in the carry.
        let mut carry = [0, 0];
        assert_eq!(split(&[10], &[9, 1], &mut carry), [0]);
        assert_eq!(carry, [-1, 1]);
        // A zero target never receives anyone, whatever it is owed.
        let mut carry = [-50, 50];
        assert_eq!(split(&[10, 10], &[1, 0], &mut carry), [0, 0]);
        assert_eq!(carry, [-50, 50]);
    }

    #[test]
    fn deal_gives_each_member_to_the_category_owed_most() {
        let mut carry = [0, 0, 0];
        assert_eq!(
            deal(&[1, 1, 1, 1], &[2, 0, 2], &mut carry),
            [0, 2, 0, 2],
            "ties to the lower index; a zero target gets nobody"
        );
        assert_eq!(carry, [0, 0, 0]);
        // One member of weight 10 against 6 : 4 leaves 4 owed to the second category.
        let mut carry = [0, 0];
        assert_eq!(deal(&[10], &[6, 4], &mut carry), [0]);
        assert_eq!(carry, [-4, 4]);
        assert_eq!(deal(&[10], &[6, 4], &mut carry), [1]);
        assert_eq!(carry, [2, -2]);
    }

    #[test]
    fn balance_keeps_rows_exact_and_columns_close() {
        let pattern = vec![vec![1, 1], vec![1, 3]];
        let table = balance(&pattern, &[100, 100], &[1, 1], 20, None);
        for row in &table {
            assert_eq!(row.iter().sum::<Int>(), 100);
        }
        let first: Int = table.iter().map(|row| row[0]).sum();
        assert!((first - 100).abs() <= 1, "column total {first}");
        assert!(table[0][0] > table[1][0], "the pattern's proportions stay");
        // A row without weight is all zero and does not hold the others back.
        let table = balance(&pattern, &[0, 100], &[1, 1], 200, Some(100));
        assert_eq!(table, [vec![0, 0], vec![50, 50]]);
    }

    #[test]
    fn band_of_handles_the_open_band() {
        let bands = [(0, 5), (5, 10), (10, OPEN)];
        assert_eq!(band_of(&bands, 4), Some(0));
        assert_eq!(band_of(&bands, 5), Some(1));
        assert_eq!(band_of(&bands, 300), Some(2));
        assert_eq!(band_of(&bands[..2], 10), None);
    }
}
