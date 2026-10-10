//! Aggregating log-linear explanations across agents (settles the open
//! question in ADR-0008, Spike 3).
//!
//! **Decision:** for indicators that are *sums of levels* (total output,
//! consumption, wage bill), aggregate per-agent LMDI contributions in levels:
//! `Cᵢ = Σₖ L(yₖ₁, yₖ₀)·βᵢ·Δln xᵢₖ`. This sums exactly to the change in the
//! total. For *indices and averages* (CPI, average wage growth), use
//! weight-averaged log contributions `Σₖ wₖ·βᵢ·Δln xᵢₖ`, which sum to the
//! change in the (weighted) log index. Weighted-mean-log contributions do
//! **not** sum to level changes, so they must never be used for totals.

/// Per-agent LMDI contributions for a rule with `n_terms` terms, summed in
/// agent order. `deltas_ln[k][i]` = βᵢ·Δln xᵢ for agent k; `y0[k]`, `y1[k]`
/// the agent's before/after levels (> 0); `weights[k]` the record weight.
/// Returns (Δ total, contribution per term).
#[must_use]
pub fn lmdi_levels(
    y0: &[f64],
    y1: &[f64],
    weights: &[f64],
    deltas_ln: &[Vec<f64>],
    n_terms: usize,
) -> (f64, Vec<f64>) {
    let mut contrib = vec![0.0; n_terms];
    let mut total = 0.0;
    for k in 0..y0.len() {
        let l = crate::log_mean(y1[k], y0[k]) * weights[k];
        total += (y1[k] - y0[k]) * weights[k];
        for (i, c) in contrib.iter_mut().enumerate() {
            *c += l * deltas_ln[k][i];
        }
    }
    (total, contrib)
}

#[cfg(test)]
mod tests {
    use super::*;
    use econ_rng::{KeyedRng, Stream};

    /// 1,000 weighted agents with y = x1^0.7 · x2^0.3 and random changes:
    /// level LMDI sums to the change in the weighted total.
    #[test]
    fn lmdi_aggregates_exactly_across_agents() {
        let r = KeyedRng::new(3);
        let (b1, b2) = (0.7, 0.3);
        let (mut y0, mut y1, mut w, mut d) = (vec![], vec![], vec![], vec![]);
        for k in 0..1000u64 {
            let mut g = r.draw(Stream::Test, 0, k);
            let x1a = 50.0 + 100.0 * g.uniform();
            let x2a = 10.0 + 20.0 * g.uniform();
            let x1b = x1a * (0.8 + 0.4 * g.uniform());
            let x2b = x2a * (0.8 + 0.4 * g.uniform());
            let f = |a: f64, b: f64| econ_num::math::powf(a, b1) * econ_num::math::powf(b, b2);
            y0.push(f(x1a, x2a));
            y1.push(f(x1b, x2b));
            w.push(1.0 + (k % 7) as f64);
            let ln = econ_num::math::ln;
            d.push(vec![b1 * (ln(x1b) - ln(x1a)), b2 * (ln(x2b) - ln(x2a))]);
        }
        let (total, c) = lmdi_levels(&y0, &y1, &w, &d, 2);
        let s = c[0] + c[1];
        assert!(
            ((s - total) / total.abs().max(1.0)).abs() < 1e-9,
            "sum {s} vs total {total}"
        );
    }
}
