//! Stage B — the fit (spec steps 7–11). Kept when the seed stage changes.
//!
//! Turns the seed's equal weights into integer weights that reproduce the
//! county's census totals exactly and its cells within tolerance.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use crate::arith::{Int, UNIT, apportion, count, int, rdiv};
use crate::seed::CountySeed;
use crate::{FitReport, GenError, GenParams};

/// Alternating passes of the two-way balancing of the person targets (step 7).
const TARGET_SWEEPS: u32 = 50;
/// Upper bound on single-unit moves when fixing a county's person total (step 10).
const MAX_PERSON_MOVES: u32 = 100_000;

/// Step 7: person targets per county and (sex, age band) cell, `[county][cell]`,
/// and the number of county cells that have people but no synthetic record.
///
/// A cell without a record cannot be fitted, so each national (sex, age band)
/// total is shared among the county cells that do have records, and the table
/// is balanced both ways: rows keep the national totals, columns keep each
/// county's own person total, which is what its households must hold.
/// Counties win: the last step makes every column exact.
pub(crate) fn person_targets(
    seeds: &[CountySeed],
    private: &[&[Int]],
    n_bands: usize,
) -> Result<(Vec<Vec<Int>>, u32), GenError> {
    let n_cells = 2 * n_bands;
    let has_record = |ci: usize, cell: usize| !seeds[ci].by_cell[cell].is_empty();
    let mut national: Vec<Int> = (0..n_cells)
        .map(|cell| private.iter().map(|p| p[cell]).sum())
        .collect();
    // t[county][cell], in millionths of a person.
    let mut t: Vec<Vec<Int>> = (0..seeds.len())
        .map(|ci| {
            (0..n_cells)
                .map(|cell| {
                    if has_record(ci, cell) {
                        private[ci][cell] * UNIT
                    } else {
                        0
                    }
                })
                .collect()
        })
        .collect();
    let covered = |t: &[Vec<Int>], cell: usize| t.iter().any(|col| col[cell] > 0);

    // A band with no record anywhere joins the nearest band of the same sex
    // that has one (the younger on a tie).
    for cell in 0..n_cells {
        if national[cell] > 0 && !covered(&t, cell) {
            let (sex, band) = (cell / n_bands, cell % n_bands);
            let nearest = (0..n_bands)
                .filter(|&b| covered(&t, sex * n_bands + b))
                .min_by_key(|&b| (b.abs_diff(band), b))
                .ok_or(GenError::EmptyTable)?;
            national[sex * n_bands + nearest] += national[cell];
            national[cell] = 0;
        }
    }

    let county_total: Vec<Int> = private.iter().map(|p| p.iter().sum()).collect();
    for _ in 0..TARGET_SWEEPS {
        for cell in 0..n_cells {
            let row: Int = t.iter().map(|col| col[cell]).sum();
            if row != 0 {
                for col in &mut t {
                    col[cell] = rdiv(col[cell] * national[cell] * UNIT, row);
                }
            }
        }
        for (col, &total) in t.iter_mut().zip(&county_total) {
            let sum: Int = col.iter().sum();
            if sum != 0 {
                for v in col.iter_mut() {
                    *v = rdiv(*v * total * UNIT, sum);
                }
            }
        }
    }
    let targets: Vec<Vec<Int>> = t
        .iter()
        .zip(&county_total)
        .map(|(col, &total)| apportion(total, col))
        .collect();

    let unfitted = (0..seeds.len())
        .flat_map(|ci| (0..n_cells).map(move |cell| (ci, cell)))
        .filter(|&(ci, cell)| private[ci][cell] > 0 && !has_record(ci, cell))
        .count();
    Ok((targets, u32::try_from(unfitted).unwrap_or(u32::MAX)))
}

/// Steps 8–10 for one county: integer weights of its private households.
pub(crate) fn private_weights(
    county: &str,
    seed: &CountySeed,
    person_targets: &[Int],
    h_total: Int,
    p_total: Int,
    params: &GenParams,
    report: &mut FitReport,
) -> Result<Vec<Int>, GenError> {
    let inconsistent = || GenError::InconsistentMargins {
        county: county.to_string(),
    };
    let raked = rake(seed, person_targets, params, report);
    let sizes: Vec<usize> = seed.members.iter().map(Vec::len).collect();
    let mut weights = whole_weights(&raked, h_total).ok_or_else(inconsistent)?;
    fix_person_total(&raked, &sizes, p_total, &mut weights).ok_or_else(inconsistent)?;
    Ok(weights)
}

/// Step 8: integer raking. Returns the weights in millionths of a household.
fn rake(
    seed: &CountySeed,
    person_targets: &[Int],
    params: &GenParams,
    report: &mut FitReport,
) -> Vec<Int> {
    let scale = Int::from(params.sample_scale);
    let n_hh = seed.members.len();
    let mut w = vec![scale * UNIT; n_hh];

    // Constraints, in order: each size class (a household counts 1), then each
    // (sex, age band) cell (a household counts its members in the cell).
    let mut constraints: Vec<(Int, Vec<(usize, Int)>)> = seed
        .class_targets
        .iter()
        .enumerate()
        .map(|(k, &target)| {
            let households = (0..n_hh).filter(|&h| seed.klass[h] == k);
            (target, households.map(|h| (h, 1)).collect())
        })
        .collect();
    constraints.extend(
        person_targets
            .iter()
            .zip(&seed.by_cell)
            .map(|(&target, cell)| (target, cell.clone())),
    );
    let total =
        |w: &[Int], who: &[(usize, Int)]| -> Int { who.iter().map(|&(h, n)| w[h] * n).sum() };
    let checked: Vec<usize> = (0..constraints.len())
        .filter(|&i| {
            let target = constraints[i].0;
            target > 0 && target >= Int::from(params.min_cell_records) * scale
        })
        .collect();

    let (mut sweeps, mut worst, mut converged) = (0, 0, false);
    for sweep in 1..=params.max_sweeps {
        sweeps = sweep;
        for (target, who) in &constraints {
            let current = total(&w, who);
            if *target == 0 || current == 0 {
                continue;
            }
            let wanted = target * UNIT;
            for &(h, _) in who {
                w[h] = rdiv(w[h] * wanted, current);
            }
        }
        worst = 0;
        for &i in &checked {
            let (target, who) = &constraints[i];
            let wanted = target * UNIT;
            worst = worst.max((total(&w, who) - wanted).abs() * 1_000_000 / wanted);
        }
        if worst <= Int::from(params.raking_tolerance_ppm) {
            converged = true;
            break;
        }
    }
    report.converged &= converged;
    report.sweeps = report.sweeps.max(sweeps);
    report.max_error_ppm = report
        .max_error_ppm
        .max(u32::try_from(worst).unwrap_or(u32::MAX));
    w
}

/// Step 9: whole households with the exact county total. `None` if the county
/// has fewer real households than synthetic ones.
fn whole_weights(raked: &[Int], h_total: Int) -> Option<Vec<Int>> {
    let n_hh = raked.len();
    let mut weights: Vec<Int> = raked.iter().map(|w| w / UNIT).collect();
    // Largest remainders first, lowest id on ties.
    let mut order: Vec<usize> = (0..n_hh).collect();
    order.sort_by_key(|&h| (Reverse(raked[h] % UNIT), h));
    let missing = h_total - weights.iter().sum::<Int>();
    // Units are handed out (or taken back) one at a time around `order`.
    let (rounds, extra) = (missing.abs() / int(n_hh), count(missing.abs() % int(n_hh)));
    if missing >= 0 {
        weights.iter_mut().for_each(|w| *w += rounds);
        order.iter().take(extra).for_each(|&h| weights[h] += 1);
    } else {
        weights.iter_mut().for_each(|w| *w -= rounds);
        order
            .iter()
            .rev()
            .take(extra)
            .for_each(|&h| weights[h] -= 1);
    }

    // A household whose weight would be 0 is given 1, taken from the largest
    // weight in the county (lowest id on ties). The heap holds stale entries;
    // one is valid if it still shows the household's current weight.
    if weights.iter().any(|&w| w < 1) {
        let mut largest: BinaryHeap<(Int, Reverse<usize>)> = weights
            .iter()
            .enumerate()
            .map(|(h, &w)| (w, Reverse(h)))
            .collect();
        for h in 0..n_hh {
            while weights[h] < 1 {
                let donor = loop {
                    let &(w, Reverse(d)) = largest.peek()?;
                    if weights[d] == w {
                        break d;
                    }
                    largest.pop();
                };
                if weights[donor] <= 1 {
                    return None;
                }
                weights[donor] -= 1;
                weights[h] += 1;
                largest.push((weights[donor], Reverse(donor)));
                largest.push((weights[h], Reverse(h)));
            }
        }
    }
    Some(weights)
}

/// Step 10: make the county's weighted person total exact by moving single
/// units of weight between households of different sizes. `None` if no move
/// can close the gap.
fn fix_person_total(
    raked: &[Int],
    sizes: &[usize],
    p_total: Int,
    weights: &mut [Int],
) -> Option<()> {
    let n_hh = raked.len();
    let mut gap = p_total - (0..n_hh).map(|h| weights[h] * int(sizes[h])).sum::<Int>();
    // What rounding did to each household: positive means rounded down.
    let mut favour: Vec<Int> = (0..n_hh).map(|h| raked[h] - weights[h] * UNIT).collect();
    let mut by_size: BTreeMap<Int, Vec<usize>> = BTreeMap::new();
    for (h, &size) in sizes.iter().enumerate() {
        by_size.entry(int(size)).or_default().push(h);
    }
    let present: Vec<Int> = by_size.keys().copied().collect();
    let descending: Vec<Int> = present.iter().rev().copied().collect();

    let mut moves = 0;
    while gap != 0 {
        moves += 1;
        if moves > MAX_PERSON_MOVES {
            return None;
        }
        let up = gap > 0;
        let mut moved = false;
        for &from in if up { &present } else { &descending } {
            // The donor is the household that rounding favoured most.
            let Some(donor) = by_size[&from]
                .iter()
                .copied()
                .filter(|&h| weights[h] >= 2)
                .min_by_key(|&h| (favour[h], h))
            else {
                continue;
            };
            // As large a size difference as fits in the gap.
            let to = if up {
                present
                    .iter()
                    .copied()
                    .rfind(|&s| from < s && s <= from + gap)
            } else {
                present
                    .iter()
                    .copied()
                    .find(|&s| from + gap <= s && s < from)
            };
            let Some(to) = to else {
                continue;
            };
            // The receiver is the household that rounding favoured least.
            let receiver = by_size[&to]
                .iter()
                .copied()
                .max_by_key(|&h| (favour[h], Reverse(h)))
                .expect("a size that is present has a household");
            weights[donor] -= 1;
            weights[receiver] += 1;
            favour[donor] += UNIT;
            favour[receiver] -= UNIT;
            gap -= to - from;
            moved = true;
            break;
        }
        if !moved {
            return None;
        }
    }
    Some(())
}

/// Step 11: weights of a county's collective records. Each cell's persons are
/// split evenly over the cell's records; persons of cells without a record go
/// to the county's other records in proportion to their weights.
pub(crate) fn collective_weights(cells: &[usize], census: &[Int]) -> Vec<Int> {
    let mut weights: Vec<Int> = vec![0; cells.len()];
    let mut records_of: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (j, &cell) in cells.iter().enumerate() {
        records_of.entry(cell).or_default().push(j);
    }
    for (&cell, records) in &records_of {
        let parts = apportion(census[cell], &vec![1; records.len()]);
        for (&j, part) in records.iter().zip(parts) {
            weights[j] = part;
        }
    }
    let leftover = census.iter().sum::<Int>() - weights.iter().sum::<Int>();
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by_key(|&j| (Reverse(weights[j]), j));
    let shares: Vec<Int> = order.iter().map(|&j| weights[j].max(1)).collect();
    for (&j, part) in order.iter().zip(apportion(leftover, &shares)) {
        weights[j] += part;
    }
    for j in 0..weights.len() {
        while weights[j] < 1 {
            let donor = (0..weights.len())
                .max_by_key(|&x| (weights[x], Reverse(x)))
                .expect("at least one record");
            if weights[donor] <= 1 {
                break; // fewer persons than records: cannot happen for sample_scale >= 1
            }
            weights[donor] -= 1;
            weights[j] += 1;
        }
    }
    weights
}
