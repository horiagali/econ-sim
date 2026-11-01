//! Explanations (spec `game/explanations`, roadmap M6, fourth slice): "why
//! did this change?" for the headline indicators. A month's change of an
//! indicator is a tree whose children sum to it exactly, in integers of the
//! indicator's unit (feature `explain`, off by default; needs `stats`).
//! Read-only: computing a tree changes no state.
//!
//! The API is the spec's sketch, written with the acceptance tests before the
//! code (ADR-0014): a test writer's skeleton whose bodies are
//! `unimplemented!` until the code exists.
//!
//! Choices the sketch left open, stated by the test writer:
//! - `explain(id, key, from, to)` explains the change from the point of
//!   month `from` to the point of month `to` (`from < to`): the monthly trees
//!   of months `from + 1` to `to`, added child by child. For the
//!   year-on-year rates (`gdp.growth_yoy`, `prices.inflation_yoy`), whose
//!   value is itself a change over twelve months, the window `(to − 12, to)`
//!   explains the value at `to`: the root is that value.
//! - A child that is a registry indicator names it and its key in
//!   `indicator`; a good's child of the CPI names `prices.cpi_contribution`
//!   with the good's key (`Breakdown::Good`, the catalogue index).
//! - `lmdi(y1, y0, parts)`: `y = Π x_k` with `parts[k] = (x_k at 1, x_k at
//!   0)`, all positive; the contribution of part `k` is
//!   `L(y1, y0) · ln(x_k1 / x_k0)`, `L` the logarithmic mean (`L(a, a) = a`).
//! - `round_to_sum(values, total)` rounds by largest remainders: each value
//!   to its floor or ceiling, so that the results sum to `total`.

use econ_types::Month;

use crate::stats::{IndicatorId, Key};
use crate::world::World;

/// One node of a tree: a part of the change of its parent, in the root
/// indicator's unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// What the part is (an English label; the UI's string table comes with
    /// M8).
    pub label: String,
    /// Its value: the root's is the indicator's change.
    pub value: i64,
    /// The registry indicator and key this part is, for drill-down.
    pub indicator: Option<(IndicatorId, Key)>,
    /// Its parts, summing to `value` exactly (none for a leaf).
    pub children: Vec<Node>,
}

impl World {
    /// The tree of the change of `id` at `key` from month `from` to month
    /// `to`; `None` when the indicator has no tree, or no points there.
    #[must_use]
    pub fn explain(&self, id: IndicatorId, key: Key, from: Month, to: Month) -> Option<Node> {
        let _ = (id, key, from, to);
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The indicators that have a tree in this build.
    #[must_use]
    pub fn explained(&self) -> &'static [IndicatorId] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}

/// ADR-0008's log-mean Divisia helper: the contributions of the factors of
/// `y = Π x_k` to `y1 − y0`, summing to it within 1e-9 relative.
#[must_use]
pub fn lmdi(y1: f64, y0: f64, parts: &[(f64, f64)]) -> Vec<f64> {
    let _ = (y1, y0, parts);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Floats rounded to integers summing to `total`, by largest remainders.
#[must_use]
pub fn round_to_sum(values: &[f64], total: i64) -> Vec<i64> {
    let _ = (values, total);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}
