//! Self-explaining behaviour rules (ADR-0008).
//!
//! Every behaviour rule in the simulation is declared with
//! [`behaviour_rule!`]. One declaration generates:
//!
//! * `Inputs` and `Params` structs with one named field per term;
//! * `eval` — the fast path;
//! * `eval_explained` — the value plus one contribution per term;
//! * `explain_change(before, after)` — a [`ContributionTree`] whose children
//!   sum to the change in the rule's output;
//! * `META` — name, shape, terms and parameter provenance (feeds docs,
//!   calibration and the stability check).
//!
//! Shapes:
//! * `additive`: `y = c + Σ βᵢ·xᵢ`. Contributions to Δy are `βᵢ·Δxᵢ`
//!   (exact).
//! * `log_linear`: `y = A · Π xᵢ^βᵢ`. Contributions to Δy use the
//!   logarithmic-mean Divisia index (LMDI): `L(y₁, y₀)·βᵢ·Δln xᵢ`, which sums
//!   exactly to Δy and **aggregates exactly across agents** (see
//!   [`aggregate`]).
//!
//! Partial adjustment towards a target (`x ← x + λ(x* − x)`) is a separate
//! helper, [`partial_adjust`], with its stability multiplier
//! [`stability_multiplier`].

/// Re-exported so `behaviour_rule!` works in crates that don't depend on econ-num.
pub use econ_num;

pub mod aggregate;
mod tree;

pub use tree::ContributionTree;

/// Shape of a behaviour rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `y = c + Σ βᵢ·xᵢ`
    Additive,
    /// `y = A · Π xᵢ^βᵢ` (all inputs > 0)
    LogLinear,
}

/// Where a parameter's value comes from (ADR-0009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Set directly from statistics.
    FromData,
    /// Estimated / fitted against targets.
    Fitted,
    /// Hand-tuned for plausible behaviour.
    Tuned,
}

/// Metadata of one term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermMeta {
    /// Field name.
    pub name: &'static str,
    /// Human-readable label for the "why" panel.
    pub label: &'static str,
    /// Provenance of its coefficient.
    pub provenance: Provenance,
}

/// Metadata of a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleMeta {
    /// Rule name.
    pub name: &'static str,
    /// Shape.
    pub shape: Shape,
    /// Terms in declaration order.
    pub terms: &'static [TermMeta],
}

/// Logarithmic mean `L(a, b) = (a − b) / (ln a − ln b)`, with `L(a, a) = a`.
/// Inputs must be positive.
#[must_use]
pub fn log_mean(a: f64, b: f64) -> f64 {
    assert!(
        a > 0.0 && b > 0.0,
        "log_mean needs positive values: {a}, {b}"
    );
    let d = econ_num::math::ln(a) - econ_num::math::ln(b);
    if d.abs() < 1e-12 {
        // Second-order expansion around a == b.
        return 0.5 * (a + b);
    }
    (a - b) / d
}

/// One partial-adjustment step `x + λ·(target − x)`.
#[must_use]
pub fn partial_adjust(x: f64, target: f64, lambda: f64) -> f64 {
    x + lambda * (target - x)
}

/// Local multiplier μ = 1 + λ·(f′ − 1) of the map `x ← x + λ(f(x) − x)`.
/// The rule is locally stable iff |μ| < 1 (ADR-0009 per-rule stability test).
#[must_use]
pub fn stability_multiplier(lambda: f64, f_prime: f64) -> f64 {
    1.0 + lambda * (f_prime - 1.0)
}

/// Declare a behaviour rule. See the crate docs.
///
/// ```
/// use econ_rules::{behaviour_rule, Provenance::*};
/// behaviour_rule! {
///     /// Household consumption: C = c_y·YD + c_w·W
///     pub rule Consumption: additive {
///         yd: "disposable income" [FromData],
///         wealth: "liquid wealth" [Tuned],
///     }
/// }
/// let p = Consumption::Params { constant: 0.0, yd: 0.8, wealth: 0.05 };
/// let x = Consumption::Inputs { yd: 1000.0, wealth: 2000.0 };
/// assert_eq!(Consumption::eval(&p, &x), 900.0);
/// ```
#[macro_export]
macro_rules! behaviour_rule {
    (
        $(#[$doc:meta])*
        pub rule $name:ident : additive {
            $( $term:ident : $label:literal [ $prov:ident ] ),+ $(,)?
        }
    ) => {
        $(#[$doc])*
        #[allow(non_snake_case, dead_code)]
        pub mod $name {
            #![allow(clippy::float_arithmetic)]
            use $crate::{ContributionTree, RuleMeta, Shape, TermMeta, Provenance};

            /// Inputs (one per term).
            #[derive(Debug, Clone, Copy, PartialEq)]
            pub struct Inputs { $( pub $term: f64, )+ }

            /// Coefficients (one per term) plus a constant.
            #[derive(Debug, Clone, Copy, PartialEq)]
            pub struct Params { pub constant: f64, $( pub $term: f64, )+ }

            /// Rule metadata.
            pub const META: RuleMeta = RuleMeta {
                name: stringify!($name),
                shape: Shape::Additive,
                terms: &[ $( TermMeta { name: stringify!($term), label: $label, provenance: Provenance::$prov }, )+ ],
            };

            /// Fast evaluation.
            #[must_use]
            pub fn eval(p: &Params, x: &Inputs) -> f64 {
                let mut y = p.constant;
                $( y += p.$term * x.$term; )+
                y
            }

            /// Value plus per-term contributions to the level (terms in declaration order;
            /// the constant is reported as "constant").
            #[must_use]
            pub fn eval_explained(p: &Params, x: &Inputs) -> (f64, Vec<(&'static str, f64)>) {
                let mut parts = vec![("constant", p.constant)];
                $( parts.push((stringify!($term), p.$term * x.$term)); )+
                (eval(p, x), parts)
            }

            /// Decompose `eval(after) − eval(before)` into per-term contributions.
            #[must_use]
            pub fn explain_change(p: &Params, before: &Inputs, after: &Inputs) -> ContributionTree {
                let total = eval(p, after) - eval(p, before);
                let children = vec![
                    $( ContributionTree::leaf($label, p.$term * (after.$term - before.$term)), )+
                ];
                ContributionTree::node(stringify!($name), total, children)
            }
        }
    };
    (
        $(#[$doc:meta])*
        pub rule $name:ident : log_linear {
            $( $term:ident : $label:literal [ $prov:ident ] ),+ $(,)?
        }
    ) => {
        $(#[$doc])*
        #[allow(non_snake_case, dead_code)]
        pub mod $name {
            use $crate::{ContributionTree, RuleMeta, Shape, TermMeta, Provenance};

            /// Inputs (one per term; all must be > 0).
            #[derive(Debug, Clone, Copy, PartialEq)]
            pub struct Inputs { $( pub $term: f64, )+ }

            /// Elasticities (one per term) plus a scale factor `scale` (A).
            #[derive(Debug, Clone, Copy, PartialEq)]
            pub struct Params { pub scale: f64, $( pub $term: f64, )+ }

            /// Rule metadata.
            pub const META: RuleMeta = RuleMeta {
                name: stringify!($name),
                shape: Shape::LogLinear,
                terms: &[ $( TermMeta { name: stringify!($term), label: $label, provenance: Provenance::$prov }, )+ ],
            };

            /// Fast evaluation: `A · Π xᵢ^βᵢ`.
            #[must_use]
            pub fn eval(p: &Params, x: &Inputs) -> f64 {
                let mut y = p.scale;
                $( y *= $crate::econ_num::math::powf(x.$term, p.$term); )+
                y
            }

            /// Value plus per-term contributions to ln y.
            #[must_use]
            pub fn eval_explained(p: &Params, x: &Inputs) -> (f64, Vec<(&'static str, f64)>) {
                let mut parts = vec![("scale", $crate::econ_num::math::ln(p.scale))];
                $( parts.push((stringify!($term), p.$term * $crate::econ_num::math::ln(x.$term))); )+
                (eval(p, x), parts)
            }

            /// Per-term LMDI contributions to Δy (sum exactly to Δy up to float
            /// rounding). Returned in declaration order.
            #[must_use]
            pub fn lmdi(p: &Params, before: &Inputs, after: &Inputs) -> (f64, Vec<f64>) {
                let y0 = eval(p, before);
                let y1 = eval(p, after);
                let l = $crate::log_mean(y1, y0);
                let parts = vec![
                    $( l * p.$term * ($crate::econ_num::math::ln(after.$term) - $crate::econ_num::math::ln(before.$term)), )+
                ];
                (y1 - y0, parts)
            }

            /// Decompose `eval(after) − eval(before)` into per-term LMDI contributions.
            #[must_use]
            pub fn explain_change(p: &Params, before: &Inputs, after: &Inputs) -> ContributionTree {
                let (total, parts) = lmdi(p, before, after);
                let labels = [ $( $label, )+ ];
                let children = labels.iter().zip(parts).map(|(l, v)| ContributionTree::leaf(l, v)).collect();
                ContributionTree::node(stringify!($name), total, children)
            }
        }
    };
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    behaviour_rule! {
        /// Target price: p* = A · unit_cost^1 · markup^1 · tax^1
        pub rule TargetPrice: log_linear {
            unit_cost: "unit cost" [FromData],
            markup: "markup (demand pressure)" [Tuned],
            tax: "VAT and excise" [FromData],
        }
    }

    behaviour_rule! {
        /// Nominal wage growth: π^e + productivity growth + β·(tightness − tightness*)
        pub rule WageGrowth: additive {
            expected_inflation: "expected inflation" [Fitted],
            productivity: "productivity growth" [FromData],
            tightness_gap: "labour market tightness" [Fitted],
        }
    }

    behaviour_rule! {
        /// Household consumption C = c_y·YD + c_w·W
        pub rule Consumption: additive {
            yd: "disposable income" [FromData],
            wealth: "liquid wealth" [Tuned],
        }
    }

    #[test]
    fn additive_change_sums_exactly() {
        let p = Consumption::Params {
            constant: 10.0,
            yd: 0.8,
            wealth: 0.05,
        };
        let a = Consumption::Inputs {
            yd: 1000.0,
            wealth: 2000.0,
        };
        let b = Consumption::Inputs {
            yd: 1100.0,
            wealth: 1500.0,
        };
        let t = Consumption::explain_change(&p, &a, &b);
        assert!(t.check_sums(1e-12).is_ok(), "{t:?}");
        assert_eq!(t.children.len(), 2);
        assert!((t.children[0].value - 80.0).abs() < 1e-9);
        assert!((t.children[1].value + 25.0).abs() < 1e-9);
    }

    #[test]
    fn log_linear_change_sums_exactly() {
        let p = TargetPrice::Params {
            scale: 1.0,
            unit_cost: 1.0,
            markup: 1.0,
            tax: 1.0,
        };
        let a = TargetPrice::Inputs {
            unit_cost: 80.0,
            markup: 1.2,
            tax: 1.19,
        };
        let b = TargetPrice::Inputs {
            unit_cost: 88.0,
            markup: 1.15,
            tax: 1.21,
        };
        let t = TargetPrice::explain_change(&p, &a, &b);
        assert!(t.check_sums(1e-9).is_ok(), "{t:?}");
        // Unit cost +10% is the biggest driver.
        assert!(t.children[0].value > t.children[2].value);
        assert!(t.children[1].value < 0.0);
    }

    #[test]
    fn nested_tree_sums() {
        // Wage growth feeds unit cost which feeds price: a 2-level tree.
        let wp = WageGrowth::Params {
            constant: 0.0,
            expected_inflation: 1.0,
            productivity: 1.0,
            tightness_gap: 0.5,
        };
        let w0 = WageGrowth::Inputs {
            expected_inflation: 0.03,
            productivity: 0.01,
            tightness_gap: 0.0,
        };
        let w1 = WageGrowth::Inputs {
            expected_inflation: 0.05,
            productivity: 0.01,
            tightness_gap: 0.02,
        };
        let wage_tree = WageGrowth::explain_change(&wp, &w0, &w1);
        let pp = TargetPrice::Params {
            scale: 1.0,
            unit_cost: 1.0,
            markup: 1.0,
            tax: 1.0,
        };
        let p0 = TargetPrice::Inputs {
            unit_cost: 100.0,
            markup: 1.2,
            tax: 1.19,
        };
        let p1 = TargetPrice::Inputs {
            unit_cost: 103.0,
            markup: 1.2,
            tax: 1.19,
        };
        let mut price_tree = TargetPrice::explain_change(&pp, &p0, &p1);
        // Attach the wage explanation as the breakdown of the unit-cost driver,
        // rescaled so it explains that driver's contribution.
        let uc = price_tree.children[0].value;
        price_tree.children[0] = wage_tree.rescaled_to(uc).relabel("unit cost");
        assert!(price_tree.check_sums(1e-9).is_ok(), "{price_tree:?}");
    }

    #[test]
    fn stability_multiplier_rule() {
        // f'(x) = 0 (target independent of current value): stable for 0 < λ ≤ 1.
        assert!(stability_multiplier(0.3, 0.0).abs() < 1.0);
        // Over-reaction λ = 2.5 with f' = 0 → μ = −1.5: unstable (oscillates).
        assert!(stability_multiplier(2.5, 0.0).abs() > 1.0);
        // Self-reinforcing target f' = 1.2 with λ = 0.5 → μ = 1.1: unstable (explodes).
        assert!(stability_multiplier(0.5, 1.2) > 1.0);
        assert_eq!(partial_adjust(10.0, 20.0, 0.25), 12.5);
    }

    #[test]
    fn eval_explained_parts_match_value() {
        let p = Consumption::Params {
            constant: 10.0,
            yd: 0.8,
            wealth: 0.05,
        };
        let x = Consumption::Inputs {
            yd: 1000.0,
            wealth: 2000.0,
        };
        let (v, parts) = Consumption::eval_explained(&p, &x);
        assert!((parts.iter().map(|p| p.1).sum::<f64>() - v).abs() < 1e-9);
        let pp = TargetPrice::Params {
            scale: 1.0,
            unit_cost: 1.0,
            markup: 1.0,
            tax: 1.0,
        };
        let px = TargetPrice::Inputs {
            unit_cost: 80.0,
            markup: 1.2,
            tax: 1.19,
        };
        let (pv, lparts) = TargetPrice::eval_explained(&pp, &px);
        let lnsum: f64 = lparts.iter().map(|p| p.1).sum();
        assert!((econ_num::math::exp(lnsum) - pv).abs() < 1e-9);
    }

    #[test]
    fn meta_lists_terms_and_provenance() {
        assert_eq!(TargetPrice::META.terms.len(), 3);
        assert_eq!(TargetPrice::META.shape, Shape::LogLinear);
        assert_eq!(WageGrowth::META.terms[0].provenance, Provenance::Fitted);
    }
}
