//! Fixed-order floating-point sums (ADR-0006 rule 3).

/// Chunk size for [`det_sum`]. Fixed forever: changing it changes results.
pub const CHUNK: usize = 4096;

/// Sum `xs` in a fixed order: sequential sums of consecutive `CHUNK`-sized
/// chunks, then a sequential sum of the chunk partials. A parallel
/// implementation that computes each chunk's partial independently and
/// combines them in index order gives bit-identical results.
#[must_use]
pub fn det_sum(xs: &[f64]) -> f64 {
    det_sum_chunks(&chunk_partials(xs))
}

/// Combine pre-computed chunk partials in index order.
#[must_use]
pub fn det_sum_chunks(partials: &[f64]) -> f64 {
    let mut acc = 0.0;
    for &p in partials {
        acc += p;
    }
    acc
}

fn chunk_partials(xs: &[f64]) -> Vec<f64> {
    xs.chunks(CHUNK)
        .map(|c| {
            let mut s = 0.0;
            for &x in c {
                s += x;
            }
            s
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunked_equals_manual_parallel_combination() {
        let xs: Vec<f64> = (0..50_000u32).map(|i| f64::from(i) * 0.1 + 1e-7).collect();
        // Simulate "threads": compute partials out of order, combine in order.
        let mut partials: Vec<(usize, f64)> = xs
            .chunks(CHUNK)
            .enumerate()
            .rev()
            .map(|(i, c)| (i, c.iter().fold(0.0, |a, b| a + b)))
            .collect();
        partials.sort_by_key(|p| p.0);
        let combined = det_sum_chunks(&partials.iter().map(|p| p.1).collect::<Vec<_>>());
        assert_eq!(det_sum(&xs).to_bits(), combined.to_bits());
    }
}
