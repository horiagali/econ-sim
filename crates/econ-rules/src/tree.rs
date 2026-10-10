//! Contribution trees: the data behind the "why did this change?" panel.

/// A node explaining a change: `value` should equal the sum of `children`
/// (invariant I-6) unless the node is a leaf.
#[derive(Debug, Clone, PartialEq)]
pub struct ContributionTree {
    /// Label shown in the UI.
    pub label: String,
    /// Contribution of this node to its parent's change.
    pub value: f64,
    /// Breakdown (empty for a leaf).
    pub children: Vec<ContributionTree>,
}

impl ContributionTree {
    /// A leaf.
    #[must_use]
    pub fn leaf(label: &str, value: f64) -> Self {
        ContributionTree {
            label: label.to_string(),
            value,
            children: Vec::new(),
        }
    }

    /// An inner node.
    #[must_use]
    pub fn node(label: &str, value: f64, children: Vec<ContributionTree>) -> Self {
        ContributionTree {
            label: label.to_string(),
            value,
            children,
        }
    }

    /// Change the label.
    #[must_use]
    pub fn relabel(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Scale the whole subtree so its value equals `target` (used to attach a
    /// driver's own explanation, computed in different units, below it).
    #[must_use]
    pub fn rescaled_to(mut self, target: f64) -> Self {
        let f = if self.value == 0.0 {
            0.0
        } else {
            target / self.value
        };
        self.scale(f);
        self.value = target;
        self
    }

    fn scale(&mut self, f: f64) {
        self.value *= f;
        for c in &mut self.children {
            c.scale(f);
        }
    }

    /// Check that every inner node equals the sum of its children within a
    /// relative tolerance (I-6). Returns the first offending label.
    ///
    /// # Errors
    /// The label and the mismatch of the first failing node.
    pub fn check_sums(&self, rel_tol: f64) -> Result<(), (String, f64)> {
        if self.children.is_empty() {
            return Ok(());
        }
        let s: f64 = econ_num::det_sum(&self.children.iter().map(|c| c.value).collect::<Vec<_>>());
        let scale = self.value.abs().max(s.abs()).max(1e-300);
        if (s - self.value).abs() / scale > rel_tol {
            return Err((self.label.clone(), s - self.value));
        }
        for c in &self.children {
            c.check_sums(rel_tol)?;
        }
        Ok(())
    }

    /// The `k` largest children by absolute value, with the rest folded into
    /// "other" (what the UI shows).
    #[must_use]
    pub fn top_k(&self, k: usize) -> Vec<(String, f64)> {
        let mut v: Vec<(usize, &ContributionTree)> = self.children.iter().enumerate().collect();
        v.sort_by(|a, b| {
            b.1.value
                .abs()
                .total_cmp(&a.1.value.abs())
                .then(a.0.cmp(&b.0))
        });
        let mut out: Vec<(String, f64)> = v
            .iter()
            .take(k)
            .map(|(_, c)| (c.label.clone(), c.value))
            .collect();
        if v.len() > k {
            let rest: f64 = v.iter().skip(k).map(|(_, c)| c.value).sum();
            out.push(("other".to_string(), rest));
        }
        out
    }
}
