//! Explanations (spec `game/explanations`, roadmap M6, fourth slice): "why
//! did this change?" for the headline indicators. A month's change of an
//! indicator is a tree whose children sum to it exactly, in integers of the
//! indicator's unit (feature `explain`, off by default; needs `stats`).
//! Read-only: computing a tree changes no state.
//!
//! The API is the spec's sketch, written with the acceptance tests before the
//! code (ADR-0014). The trees read the registry's history, the mechanics'
//! month records and a few details the `World` keeps at each close
//! (`world/explain_state.rs`): GDP's expenditure parts, each good's CPI term,
//! employment by region, wages and staff by industry, and whether the
//! policy rate at the close is the player's.
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

use crate::stats::{Breakdown, IndicatorId as I, Key, Status, indicators};
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
    pub indicator: Option<(I, Key)>,
    /// Its parts, summing to `value` exactly (none for a leaf).
    pub children: Vec<Node>,
}

/// A part before rounding: a float value (exact for integers), and its own
/// parts.
struct Part {
    label: String,
    value: f64,
    indicator: Option<(I, Key)>,
    children: Vec<Part>,
}

fn part(label: impl Into<String>, value: f64) -> Part {
    Part {
        label: label.into(),
        value,
        indicator: None,
        children: Vec::new(),
    }
}

fn part_of(label: impl Into<String>, value: f64, id: I, key: Key) -> Part {
    Part {
        indicator: Some((id, key)),
        ..part(label, value)
    }
}

fn group(label: impl Into<String>, children: Vec<Part>) -> Part {
    Part {
        value: children.iter().map(|c| c.value).sum(),
        children,
        ..part(label, 0.0)
    }
}

/// Parts rounded to integers summing to `total`, at every depth.
fn settle(parts: Vec<Part>, total: i64) -> Vec<Node> {
    let values: Vec<f64> = parts.iter().map(|p| p.value).collect();
    let rounded = round_to_sum(&values, total);
    parts
        .into_iter()
        .zip(rounded)
        .map(|(p, v)| Node {
            label: p.label,
            value: v,
            indicator: p.indicator,
            children: settle(p.children, v),
        })
        .collect()
}

/// `b` added to `a` child by child (the same tree's shape).
fn add(a: &mut Node, b: &Node) {
    a.value += b.value;
    for (x, y) in a.children.iter_mut().zip(&b.children) {
        add(x, y);
    }
}

#[allow(clippy::cast_precision_loss)]
fn f(x: i64) -> f64 {
    x as f64
}

#[allow(clippy::cast_precision_loss)]
fn f128(x: i128) -> f64 {
    x as f64
}

/// The indicators with a tree, in the spec's order.
const TREES: [I; 19] = [
    I::GdpNominal,
    I::GdpReal,
    I::WagesMeanReal,
    I::GdpPerCapita,
    I::GdpGrowthYoy,
    I::PricesCpi,
    I::PricesInflationYoy,
    I::LabourUnemploymentRate,
    I::LabourEmployed,
    I::WagesMeanNominal,
    I::BudgetBalance,
    I::DebtGdpRatio,
    I::MoneyPolicyRate,
    I::FxEurLei,
    I::ApprovalNational,
    I::PeoplePopulation,
    I::PeopleSol,
    I::PeoplePovertyRateAnchored,
    I::PeopleWellbeing,
];

/// Whether econ-core's feature `name` is compiled (a `matches!` only when
/// every feature is on).
#[allow(clippy::match_like_matches_macro)]
fn feature_on(name: &str) -> bool {
    match name {
        "fiscal" => cfg!(feature = "fiscal"),
        "monetary" => cfg!(feature = "monetary"),
        "trade" => cfg!(feature = "trade"),
        "approval" => cfg!(feature = "approval"),
        "poverty" => cfg!(feature = "poverty"),
        "wellbeing" => cfg!(feature = "wellbeing"),
        "migration" => cfg!(feature = "migration"),
        "banking" => cfg!(feature = "banking"),
        "investment" => cfg!(feature = "investment"),
        "revenue" => cfg!(feature = "revenue"),
        "levers" => cfg!(feature = "levers"),
        "education" => cfg!(feature = "education"),
        _ => false,
    }
}

/// The year-on-year rates, whose window of twelve months explains their
/// value.
fn is_yoy(id: I) -> bool {
    matches!(id, I::GdpGrowthYoy | I::PricesInflationYoy)
}

const GDP_PARTS: [&str; 6] = [
    "household consumption",
    "government purchases",
    "investment",
    "exports",
    "inventories",
    "imports",
];

impl World {
    /// The tree of the change of `id` at `key` from month `from` to month
    /// `to`; `None` when the indicator has no tree, or no points there.
    #[must_use]
    pub fn explain(&self, id: I, key: Key, from: Month, to: Month) -> Option<Node> {
        if key != Key::NATIONAL || !self.explained().contains(&id) {
            return None;
        }
        let (from, to) = (usize::try_from(from.0).ok()?, usize::try_from(to.0).ok()?);
        if from >= to || to >= self.xp_frames().len() {
            return None;
        }
        if is_yoy(id) {
            return self.yoy_tree(id, from, to);
        }
        let root = self.point(id, to)? - self.point(id, from)?;
        let mut tree: Option<Node> = None;
        for t in from + 1..=to {
            let change = self.point(id, t)? - self.point(id, t - 1)?;
            let parts = self.month_parts(id, t)?;
            let n = Node {
                label: label_of(id).to_owned(),
                value: change,
                indicator: Some((id, Key::NATIONAL)),
                children: settle(parts, change),
            };
            match &mut tree {
                None => tree = Some(n),
                Some(a) => add(a, &n),
            }
        }
        tree.inspect(|n| debug_assert_eq!(n.value, root))
    }

    /// The indicators that have a tree in this build.
    #[must_use]
    pub fn explained(&self) -> &'static [I] {
        static LIST: std::sync::OnceLock<Vec<I>> = std::sync::OnceLock::new();
        LIST.get_or_init(|| {
            TREES
                .iter()
                .copied()
                .filter(|&id| {
                    indicators().iter().any(|m| {
                        m.id == id && m.status == Status::Active && m.feature.is_none_or(feature_on)
                    })
                })
                .collect()
        })
    }

    /// The national point of `id` at month `t`.
    fn point(&self, id: I, t: usize) -> Option<i64> {
        self.point_at(id, Key::NATIONAL, t)
    }

    fn point_at(&self, id: I, key: Key, t: usize) -> Option<i64> {
        let s = self.stats().series(id, key)?;
        let i = t.checked_sub(usize::try_from(s.first_month.0).ok()?)?;
        s.months.get(i).copied()
    }

    /// A point as a float (0 when there is none).
    fn pf(&self, id: I, t: usize) -> f64 {
        f(self.point(id, t).unwrap_or(0))
    }

    /// The parts of the month `t`'s change (from `t − 1`).
    fn month_parts(&self, id: I, t: usize) -> Option<Vec<Part>> {
        let fr = self.xp_frames();
        let (a, b) = (&fr[t - 1], &fr[t]);
        Some(match id {
            I::GdpNominal => GDP_PARTS
                .iter()
                .enumerate()
                .map(|(k, l)| {
                    let d = f(b.gdp[k] - a.gdp[k]);
                    part(*l, if k == 5 { -d } else { d })
                })
                .collect(),
            I::GdpReal | I::WagesMeanReal => {
                let (nominal, label) = if id == I::GdpReal {
                    (I::GdpNominal, "nominal GDP")
                } else {
                    (I::WagesMeanNominal, "the nominal mean wage")
                };
                let (n1, n0) = (self.pf(nominal, t), self.pf(nominal, t - 1));
                let (p1, p0) = (self.pf(I::PricesCpi, t), self.pf(I::PricesCpi, t - 1));
                let c = lmdi_or_first(&[(n1, n0), (1e6 / p1, 1e6 / p0)]);
                vec![
                    part_of(label, c[0], nominal, Key::NATIONAL),
                    part_of("consumer prices", c[1], I::PricesCpi, Key::NATIONAL),
                ]
            }
            I::GdpPerCapita => {
                let (n1, n0) = (self.pf(I::GdpNominal, t), self.pf(I::GdpNominal, t - 1));
                let (p1, p0) = (
                    self.pf(I::PeoplePopulation, t),
                    self.pf(I::PeoplePopulation, t - 1),
                );
                let c = lmdi_or_first(&[(n1, n0), (1.0 / p1, 1.0 / p0)]);
                vec![
                    part_of("nominal GDP", c[0], I::GdpNominal, Key::NATIONAL),
                    part_of("population", c[1], I::PeoplePopulation, Key::NATIONAL),
                ]
            }
            I::PricesCpi => (0..b.terms.len())
                .map(|j| {
                    let key = Key::of(Breakdown::Good, j);
                    let v = self.point_at(I::PricesCpiContribution, key, t).unwrap_or(0);
                    part_of(self.good_name(j), f(v), I::PricesCpiContribution, key)
                })
                .collect(),
            I::LabourUnemploymentRate => {
                let n = a.employed.len().max(b.employed.len());
                let get = |v: &[i64], r: usize| f(v.get(r).copied().unwrap_or(0));
                let side = |fr: &crate::world::XpFrame| -> (Vec<f64>, Vec<f64>) {
                    let lf: Vec<f64> = (0..n)
                        .map(|r| get(&fr.employed, r) + get(&fr.unemployed, r))
                        .collect();
                    let total: f64 = lf.iter().sum();
                    let rate = (0..n)
                        .map(|r| {
                            if lf[r] > 0.0 {
                                get(&fr.unemployed, r) / lf[r]
                            } else {
                                0.0
                            }
                        })
                        .collect();
                    let share = (0..n)
                        .map(|r| if total > 0.0 { lf[r] / total } else { 0.0 })
                        .collect();
                    (rate, share)
                };
                let (u0, s0) = side(a);
                let (u1, s1) = side(b);
                let within = (0..n)
                    .map(|r| {
                        part(
                            region_label(r),
                            1e6 * (s0[r] + s1[r]) / 2.0 * (u1[r] - u0[r]),
                        )
                    })
                    .collect();
                let shares = (0..n)
                    .map(|r| {
                        part(
                            region_label(r),
                            1e6 * (u0[r] + u1[r]) / 2.0 * (s1[r] - s0[r]),
                        )
                    })
                    .collect();
                vec![
                    group("unemployment within the regions", within),
                    group("the regions' shares of the labour force", shares),
                ]
            }
            I::LabourEmployed => {
                let n = a.employed.len().max(b.employed.len());
                (0..n)
                    .map(|r| {
                        let g = |v: &[i64]| v.get(r).copied().unwrap_or(0);
                        part(region_label(r), f(g(&b.employed) - g(&a.employed)))
                    })
                    .collect()
            }
            I::WagesMeanNominal => {
                // Shares of all employed persons: those outside the firm
                // units' staff (the self-employed, …) earn no wage of the
                // bill, so the mean wage is Σ share × the industry's wage.
                let n = b.staff.len();
                let employed = |t: usize| {
                    #[allow(clippy::cast_precision_loss)]
                    let e = self.months()[t].employed as f64;
                    e
                };
                let side = |fr: &crate::world::XpFrame, total: f64| -> (Vec<f64>, Vec<f64>) {
                    let wage = (0..n)
                        .map(|j| {
                            let s = f(fr.staff.get(j).copied().unwrap_or(0));
                            if s > 0.0 {
                                f(fr.wages.get(j).copied().unwrap_or(0)) / s
                            } else {
                                0.0
                            }
                        })
                        .collect();
                    let share = (0..n)
                        .map(|j| {
                            if total > 0.0 {
                                f(fr.staff.get(j).copied().unwrap_or(0)) / total
                            } else {
                                0.0
                            }
                        })
                        .collect();
                    (wage, share)
                };
                let (w0, e0) = side(a, employed(t - 1));
                let (w1, e1) = side(b, employed(t));
                let within = (0..n)
                    .map(|j| part(self.good_name(j), (e0[j] + e1[j]) / 2.0 * (w1[j] - w0[j])))
                    .collect();
                let shares = (0..n)
                    .map(|j| part(self.good_name(j), (w0[j] + w1[j]) / 2.0 * (e1[j] - e0[j])))
                    .collect();
                vec![
                    group("wages within the industries", within),
                    group("the industries' shares of employment", shares),
                ]
            }
            I::BudgetBalance => self.budget_parts(t),
            I::DebtGdpRatio => {
                let annual = |t: usize| -> f64 {
                    let from = t.saturating_sub(11);
                    let sum: f64 = (from..=t).map(|m| self.pf(I::GdpNominal, m)).sum();
                    #[allow(clippy::cast_precision_loss)]
                    let n = (t - from + 1) as f64;
                    sum * 12.0 / n
                };
                let (d1, d0) = (self.pf(I::DebtStock, t), self.pf(I::DebtStock, t - 1));
                let c = lmdi_or_first(&[(d1, d0), (1e6 / annual(t), 1e6 / annual(t - 1))]);
                vec![
                    part_of("the debt", c[0], I::DebtStock, Key::NATIONAL),
                    part_of(
                        "the GDP of the last twelve months",
                        c[1],
                        I::GdpNominal,
                        Key::NATIONAL,
                    ),
                ]
            }
            I::MoneyPolicyRate => {
                let change = self.pf(I::MoneyPolicyRate, t) - self.pf(I::MoneyPolicyRate, t - 1);
                // Who set the rate at the close: once an order took
                // effect, the rule no longer moves it.
                let orders = if b.rate_by_orders { change } else { 0.0 };
                vec![
                    part("the player's orders", orders),
                    part("the advisory rule", change - orders),
                ]
            }
            I::FxEurLei => vec![part(
                "the exchange-rate step",
                self.pf(I::FxEurLei, t) - self.pf(I::FxEurLei, t - 1),
            )],
            #[cfg(feature = "approval")]
            I::ApprovalNational => {
                let parts = |t: usize| -> [i32; 5] {
                    self.approval_stats()
                        .iter()
                        .find(|s| s.month.0 as usize == t)
                        .and_then(|s| s.national.as_ref())
                        .map_or([0; 5], |n| n.parts_milli)
                };
                let (p0, p1) = (parts(t - 1), parts(t));
                ["income", "unemployment", "inflation", "hardship", "bounds"]
                    .iter()
                    .enumerate()
                    .map(|(k, l)| part(*l, f(i64::from(p1[k]) - i64::from(p0[k]))))
                    .collect()
            }
            I::PeoplePopulation => self.population_parts(t),
            #[cfg(feature = "poverty")]
            I::PeopleSol => {
                let s = self
                    .poverty_stats()
                    .iter()
                    .find(|s| s.month.0 as usize == t)?;
                [
                    "purchases",
                    "home-grown food",
                    "imputed rent",
                    "health",
                    "education",
                ]
                .iter()
                .zip(s.sol.parts_change_milli)
                .map(|(l, v)| part(*l, f(v)))
                .collect()
            }
            #[cfg(feature = "poverty")]
            I::PeoplePovertyRateAnchored => {
                let st = self.poverty_stats();
                let s1 = st.iter().find(|s| s.month.0 as usize == t)?;
                let s0 = st.iter().find(|s| s.month.0 as usize == t - 1)?;
                #[allow(clippy::cast_precision_loss)]
                let (p1, p0) = (
                    s1.national.population.max(1) as f64,
                    s0.national.population.max(1) as f64,
                );
                #[allow(clippy::cast_precision_loss)]
                let g = |x: u64| x as f64;
                let a0 = g(s0.class_persons[0]);
                vec![
                    part(
                        "households entering the class at risk",
                        1e6 * g(s1.class_entered[0]) / p1,
                    ),
                    part("households leaving it", -1e6 * g(s1.class_left[0]) / p1),
                    part("persons added to it", 1e6 * g(s1.class_added[0]) / p1),
                    part(
                        "persons removed from it",
                        -1e6 * g(s1.class_removed[0]) / p1,
                    ),
                    part("the persons counted", 1e6 * a0 * (1.0 / p1 - 1.0 / p0)),
                ]
            }
            #[cfg(feature = "wellbeing")]
            I::PeopleWellbeing => {
                let w = |t: usize| {
                    self.wellbeing_stats()
                        .iter()
                        .find(|s| s.month.0 as usize == t)
                        .map(|s| s.national)
                };
                let (w0, w1) = (w(t - 1)?, w(t)?);
                let cube = |x: u32| econ_num::math::powf(f64::from(x.max(1)), 1.0 / 3.0);
                let c = lmdi_or_first(&[
                    (cube(w1.health_ppm), cube(w0.health_ppm)),
                    (cube(w1.education_ppm), cube(w0.education_ppm)),
                    (cube(w1.living_ppm), cube(w0.living_ppm)),
                ]);
                vec![
                    part("health", c[0]),
                    part("education", c[1]),
                    part("living standard", c[2]),
                ]
            }
            _ => return None,
        })
    }

    /// The budget balance's parts: each revenue flow code's change, less
    /// each spending flow code's and the interest's. The codes are those
    /// with a point other than zero in the history.
    fn budget_parts(&self, t: usize) -> Vec<Part> {
        let mut parts = Vec::new();
        for (id, sign, what) in [
            (I::BudgetRevenue, 1.0, "revenue"),
            (I::BudgetSpending, -1.0, "spending"),
        ] {
            for code in econ_ledger::FlowCode::ALL {
                let key = Key::of(Breakdown::Flow, usize::from(code.0));
                let Some(s) = self.stats().series(id, key) else {
                    continue;
                };
                if s.months.iter().all(|&x| x == 0) {
                    continue;
                }
                let d = f(self.point_at(id, key, t).unwrap_or(0))
                    - f(self.point_at(id, key, t - 1).unwrap_or(0));
                parts.push(part_of(
                    format!("{what}: {}", code.name()),
                    sign * d,
                    id,
                    key,
                ));
            }
        }
        if self.point(I::BudgetInterest, t).is_some() {
            parts.push(part_of(
                "interest",
                -(self.pf(I::BudgetInterest, t) - self.pf(I::BudgetInterest, t - 1)),
                I::BudgetInterest,
                Key::NATIONAL,
            ));
        }
        parts
    }

    /// The population's parts: the month's flows of persons (with
    /// migration), else each region's change.
    fn population_parts(&self, t: usize) -> Vec<Part> {
        #[cfg(feature = "migration")]
        if let Some(m) = self
            .migration_months()
            .iter()
            .find(|m| m.month.0 as usize == t)
        {
            #[allow(clippy::cast_precision_loss)]
            let g = |x: u64| x as f64;
            return vec![
                part_of("births", g(m.births), I::PeopleBirths, Key::NATIONAL),
                part("resident deaths", -g(m.deaths_resident)),
                part_of(
                    "emigrants",
                    -g(m.emigrants),
                    I::PeopleEmigrants,
                    Key::NATIONAL,
                ),
                part_of("returns", g(m.returns), I::PeopleReturns, Key::NATIONAL),
                part_of(
                    "immigrants",
                    g(m.immigrants),
                    I::PeopleImmigrants,
                    Key::NATIONAL,
                ),
            ];
        }
        self.stats()
            .keys(I::PeoplePopulation)
            .iter()
            .filter(|k| k.breakdown == Some(Breakdown::Region))
            .map(|&k| {
                let d = f(self.point_at(I::PeoplePopulation, k, t).unwrap_or(0))
                    - f(self.point_at(I::PeoplePopulation, k, t - 1).unwrap_or(0));
                part_of(region_label(k.index as usize), d, I::PeoplePopulation, k)
            })
            .collect()
    }

    /// The tree of a year-on-year rate: its value at `to` when the window is
    /// the year to `to`, else its change from `from` (both values' trees,
    /// child by child).
    fn yoy_tree(&self, id: I, from: usize, to: usize) -> Option<Node> {
        let (value, parts) = if to - from == 12 {
            (self.point(id, to)?, self.yoy_parts(id, to)?)
        } else {
            if from < 12 {
                return None;
            }
            let (a, b) = (self.yoy_parts(id, from)?, self.yoy_parts(id, to)?);
            let parts = b
                .into_iter()
                .zip(a)
                .map(|(mut x, y)| {
                    x.value -= y.value;
                    x
                })
                .collect();
            (self.point(id, to)? - self.point(id, from)?, parts)
        };
        Some(Node {
            label: label_of(id).to_owned(),
            value,
            indicator: Some((id, Key::NATIONAL)),
            children: settle(parts, value),
        })
    }

    /// The parts of a year-on-year rate's value at month `t`.
    fn yoy_parts(&self, id: I, t: usize) -> Option<Vec<Part>> {
        let fr = self.xp_frames();
        if id == I::GdpGrowthYoy {
            let a = fr.get(t.checked_sub(12)?)?;
            let b = fr.get(t)?;
            let (p1, p0) = (self.pf(I::PricesCpi, t), self.pf(I::PricesCpi, t - 12));
            let real0: f64 = self.pf(I::GdpNominal, t - 12) / p0;
            return Some(
                GDP_PARTS
                    .iter()
                    .enumerate()
                    .map(|(k, l)| {
                        let sign = if k == 5 { -1.0 } else { 1.0 };
                        let d = f(b.gdp[k]) / p1 - f(a.gdp[k]) / p0;
                        part(*l, sign * 1e6 * d / real0)
                    })
                    .collect(),
            );
        }
        // Inflation: each good's term over the CPI a year before (in the
        // first year, its share of the change since day 0, annualised by
        // the rate itself).
        let b = fr.get(t)?;
        let (base, cpi0) = if t >= 12 {
            (fr[t - 12].terms.clone(), self.pf(I::PricesCpi, t - 12))
        } else {
            (self.xp_base_terms().to_vec(), 1e6)
        };
        let raw: Vec<f64> = (0..b.terms.len())
            .map(|j| f128(b.terms[j] - base.get(j).copied().unwrap_or(0)) / cpi0)
            .collect();
        let sum: f64 = raw.iter().sum();
        let value = f(self.point(id, t)?);
        let scale = if t >= 12 || sum == 0.0 {
            1.0
        } else {
            value / sum
        };
        Some(
            raw.iter()
                .enumerate()
                .map(|(j, &x)| part(self.good_name(j), x * scale))
                .collect(),
        )
    }

    fn good_name(&self, j: usize) -> String {
        self.industries().id.get(j).cloned().unwrap_or_default()
    }
}

fn region_label(r: usize) -> String {
    format!("region {r}")
}

fn label_of(id: I) -> &'static str {
    indicators()
        .iter()
        .find(|m| m.id == id)
        .map_or("", |m| m.name)
}

/// LMDI when every part is positive, else the whole change to the first
/// part.
fn lmdi_or_first(parts: &[(f64, f64)]) -> Vec<f64> {
    let y1: f64 = parts.iter().map(|p| p.0).product();
    let y0: f64 = parts.iter().map(|p| p.1).product();
    if parts
        .iter()
        .all(|p| p.0 > 0.0 && p.1 > 0.0 && p.0.is_finite() && p.1.is_finite())
    {
        lmdi(y1, y0, parts)
    } else {
        let mut v = vec![0.0; parts.len()];
        if let Some(x) = v.first_mut() {
            *x = y1 - y0;
        }
        v
    }
}

/// The logarithmic mean of two positive numbers.
fn log_mean(a: f64, b: f64) -> f64 {
    if (a - b).abs() <= 1e-12 * a.abs().max(b.abs()) {
        a
    } else {
        (a - b) / (econ_num::math::ln(a) - econ_num::math::ln(b))
    }
}

/// ADR-0008's log-mean Divisia helper: the contributions of the factors of
/// `y = Π x_k` to `y1 − y0`, summing to it within 1e-9 relative.
#[must_use]
pub fn lmdi(y1: f64, y0: f64, parts: &[(f64, f64)]) -> Vec<f64> {
    let l = log_mean(y1, y0);
    parts
        .iter()
        .map(|&(x1, x0)| l * econ_num::math::ln(x1 / x0))
        .collect()
}

/// Floats rounded to integers summing to `total`, by largest remainders.
#[must_use]
pub fn round_to_sum(values: &[f64], total: i64) -> Vec<i64> {
    crate::stats::round_to_sum(values, total)
}
