//! The statistics registry (spec `game/statistics-registry`, roadmap M6): one
//! catalogue of every indicator the player can look at, and one history the
//! `World` fills at every close (feature `stats`, off by default). The API is
//! the spec's sketch, written with the acceptance tests before the code
//! (ADR-0014).
//!
//! The registry's data is `crates/econ-core/indicators.toml`; the generator
//! `scripts/codegen_indicators.py` writes [`IndicatorId`] and the metadata.
//! The history is outputs only: no rule reads it, and it is outside the state
//! hash.

use std::collections::BTreeMap;

use econ_types::Month;

use crate::world::World;

pub use crate::indicators_gen::IndicatorId;

/// An indicator's unit (spec, "The registry", `unit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Unit {
    /// Money, national, current prices.
    Bani,
    /// Money at day 0's prices.
    RealBani,
    /// Rates and shares, millionths.
    Ppm,
    /// Persons.
    Persons,
    /// An index, 1,000,000 at the start.
    IndexPpm,
    /// Thousandths (years, points).
    Milli,
    /// A count.
    Count,
}

/// How often an indicator gets a point (spec, `period`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Period {
    /// A point at each monthly close.
    Month,
    /// Also a point each day (the fast series of ADR-0017).
    Day,
}

/// A breakdown of an indicator (spec, `breakdowns`); national is always
/// there and is not one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs)]
pub enum Breakdown {
    Region,
    County,
    Industry,
    Class,
    Fifth,
    AgeBand,
    Sex,
    Edu,
    Activity,
    Level,
    Flow,
    /// A budget line.
    Line,
    Good,
}

/// How an indicator's breakdowns relate to its national value (spec,
/// `aggregate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Aggregate {
    /// The parts add up to the national value exactly (counts and money).
    Sum,
    /// The national value is the parts' mean weighted by the breakdown's
    /// population, to within rounding.
    Mean,
    /// No relation (rates over different bases, ratios).
    None,
}

/// How a derived indicator is computed from others (spec, `derive`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Derive {
    /// `a / b`, millionths.
    Ratio(IndicatorId, IndicatorId),
    /// `a` deflated by the CPI: value × 1,000,000 / CPI.
    Real(IndicatorId),
    /// value / value 12 months before − 1, millionths; none before month 12.
    GrowthYoy(IndicatorId),
    /// value / value a month before − 1, millionths; none at month 0.
    GrowthMom(IndicatorId),
    /// `a` over resident persons.
    PerCapita(IndicatorId),
}

/// The formula of a [`Derive`], without its inputs (test writer's addition,
/// ADR-0014, for AC-STAT-04's given inputs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs)]
pub enum DeriveOp {
    Ratio,
    Real,
    GrowthYoy,
    GrowthMom,
    PerCapita,
}

/// A view of spec `game/information` an indicator appears in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs)]
pub enum View {
    Overview,
    Budget,
    Prices,
    Finance,
    Trade,
    People,
    Informal,
    Map,
}

/// An indicator's status, as levers'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    /// Computed.
    Active,
    /// Its mechanic comes later.
    Planned,
}

/// One entry of the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndicatorMeta {
    /// Its id.
    pub id: IndicatorId,
    /// Its dotted, stable name (`gdp.nominal`).
    pub name: &'static str,
    /// The glossary `code_name` it shows.
    pub glossary: &'static str,
    /// Its unit.
    pub unit: Unit,
    /// Its period.
    pub period: Period,
    /// Its breakdowns (national is always there).
    pub breakdowns: &'static [Breakdown],
    /// How the breakdowns relate to the national value.
    pub aggregate: Aggregate,
    /// For a derived indicator, how.
    pub derive: Option<Derive>,
    /// The cargo feature that computes its source, if any.
    pub feature: Option<&'static str>,
    /// The views it appears in.
    pub views: &'static [View],
    /// An overview indicator (a "why did this change" tree in M6's fourth
    /// slice).
    pub headline: bool,
    /// Active or planned.
    pub status: Status,
}

/// Every indicator of the registry, in its order.
#[must_use]
pub fn indicators() -> &'static [IndicatorMeta] {
    &crate::indicators_gen::INDICATORS
}

/// One point of a derived indicator at month `t`, by the spec's formulas in
/// integers rounded half away from zero (ADR-0006): `a` is the input's
/// monthly points from month 0, `b` the second input's (the denominator of
/// `ratio`, the CPI in millionths of `real`, the resident persons of
/// `per_capita`; unused by the growth rates). `None` where the formula has
/// no value (before month 12 for `growth_yoy`, at month 0 for `growth_mom`,
/// a zero denominator).
#[must_use]
pub fn derive_point(op: DeriveOp, a: &[i64], b: &[i64], t: usize) -> Option<i64> {
    let at = |v: &[i64], k: usize| v.get(k).copied();
    let (n, d) = match op {
        DeriveOp::Ratio => (i128::from(at(a, t)?) * 1_000_000, i128::from(at(b, t)?)),
        DeriveOp::Real => (i128::from(at(a, t)?) * 1_000_000, i128::from(at(b, t)?)),
        DeriveOp::PerCapita => (i128::from(at(a, t)?), i128::from(at(b, t)?)),
        DeriveOp::GrowthYoy => {
            let before = i128::from(at(a, t.checked_sub(12)?)?);
            ((i128::from(at(a, t)?) - before) * 1_000_000, before)
        }
        DeriveOp::GrowthMom => {
            let before = i128::from(at(a, t.checked_sub(1)?)?);
            ((i128::from(at(a, t)?) - before) * 1_000_000, before)
        }
    };
    if d == 0 {
        return None;
    }
    i64::try_from(round_div(n, d)).ok()
}

/// A breakdown member of an indicator: `breakdown` `None` is the national
/// value (`index` 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key {
    /// The breakdown, `None` for national.
    pub breakdown: Option<Breakdown>,
    /// The member of the breakdown (region, county, … in the order of its
    /// source).
    pub index: u32,
}

/// The points of one indicator and key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Series<'a> {
    /// One point per monthly close from `first_month`.
    pub months: &'a [i64],
    /// For a day indicator, one point per day stepped.
    pub days: Option<&'a [i64]>,
    /// The month of `months[0]`.
    pub first_month: Month,
}

/// The history of every indicator (spec, "The history"): outputs only,
/// outside the state hash.
#[derive(Debug, Clone, Default)]
pub struct StatsHistory {
    /// Each (indicator, key)'s column: its first month, monthly points and
    /// (day indicators) daily points.
    cols: BTreeMap<(IndicatorId, Key), Column>,
    /// Each indicator's keys, national first, in the order first recorded.
    keys: BTreeMap<IndicatorId, Vec<Key>>,
}

/// One series's points.
#[derive(Debug, Clone, Default)]
struct Column {
    first_month: u32,
    months: Vec<i64>,
    days: Vec<i64>,
}

/// `n / d` rounded half away from zero (`d` ≠ 0).
fn round_div(n: i128, d: i128) -> i128 {
    let (an, ad) = (n.abs(), d.abs());
    let q = (2 * an + ad) / (2 * ad);
    if (n < 0) == (d < 0) { q } else { -q }
}

impl StatsHistory {
    /// The points of `id` at `key`; `None` when not computed.
    #[must_use]
    pub fn series(&self, id: IndicatorId, key: Key) -> Option<Series<'_>> {
        let c = self.cols.get(&(id, key))?;
        let day = crate::indicators_gen::INDICATORS
            .iter()
            .any(|m| m.id == id && m.period == Period::Day);
        Some(Series {
            months: &c.months,
            days: day.then_some(c.days.as_slice()),
            first_month: Month(c.first_month),
        })
    }

    /// The latest monthly point of `id` at `key`.
    #[must_use]
    pub fn latest(&self, id: IndicatorId, key: Key) -> Option<i64> {
        self.cols
            .get(&(id, key))
            .and_then(|c| c.months.last().copied())
    }

    /// The keys `id` has points for (national included).
    #[must_use]
    pub fn keys(&self, id: IndicatorId) -> &[Key] {
        self.keys.get(&id).map_or(&[], Vec::as_slice)
    }
}

impl StatsHistory {
    fn column(&mut self, id: IndicatorId, key: Key, month: u32) -> &mut Column {
        if !self.cols.contains_key(&(id, key)) {
            let keys = self.keys.entry(id).or_default();
            if key.breakdown.is_none() {
                keys.insert(0, key);
            } else {
                keys.push(key);
            }
        }
        self.cols.entry((id, key)).or_insert_with(|| Column {
            first_month: month,
            ..Column::default()
        })
    }

    /// A monthly point of month `t`.
    pub(crate) fn push_month(&mut self, id: IndicatorId, key: Key, t: u32, value: i64) {
        self.column(id, key, t).months.push(value);
    }

    /// The monthly points of `id` at `key` from `first_month`, replacing any
    /// it had (a built history, for the calibration's tests).
    #[cfg(feature = "calibration")]
    pub(crate) fn set_months(
        &mut self,
        id: IndicatorId,
        key: Key,
        first_month: u32,
        points: &[i64],
    ) {
        self.cols.remove(&(id, key));
        let c = self.column(id, key, first_month);
        c.months = points.to_vec();
    }

    /// A daily point (the month `t` the day is in, for a new column).
    pub(crate) fn push_day(&mut self, id: IndicatorId, key: Key, t: u32, value: i64) {
        self.column(id, key, t).days.push(value);
    }

    /// The national monthly points of an indicator from month 0, `None`
    /// where it has none.
    pub(crate) fn national_from_zero(&self, id: IndicatorId, t: u32) -> Vec<i64> {
        let Some(c) = self.cols.get(&(id, Key::NATIONAL)) else {
            return Vec::new();
        };
        if c.first_month != 0 {
            return Vec::new();
        }
        c.months
            .iter()
            .copied()
            .take(usize::try_from(t + 1).expect("a month"))
            .collect()
    }
}

impl Breakdown {
    /// The registry file's spelling.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Breakdown::Region => "region",
            Breakdown::County => "county",
            Breakdown::Industry => "industry",
            Breakdown::Class => "class",
            Breakdown::Fifth => "fifth",
            Breakdown::AgeBand => "age_band",
            Breakdown::Sex => "sex",
            Breakdown::Edu => "edu",
            Breakdown::Activity => "activity",
            Breakdown::Level => "level",
            Breakdown::Flow => "flow",
            Breakdown::Line => "line",
            Breakdown::Good => "good",
        }
    }
}

impl Unit {
    /// The registry file's spelling.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Unit::Bani => "bani",
            Unit::RealBani => "real_bani",
            Unit::Ppm => "ppm",
            Unit::Persons => "persons",
            Unit::IndexPpm => "index_ppm",
            Unit::Milli => "milli",
            Unit::Count => "count",
        }
    }
}

impl Period {
    /// The registry file's spelling.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Period::Month => "month",
            Period::Day => "day",
        }
    }
}

impl Key {
    /// The national value's key.
    pub const NATIONAL: Key = Key {
        breakdown: None,
        index: 0,
    };

    /// A breakdown member's key.
    #[must_use]
    pub fn of(breakdown: Breakdown, index: usize) -> Key {
        Key {
            breakdown: Some(breakdown),
            index: u32::try_from(index).expect("a member"),
        }
    }
}

impl World {
    /// The statistics history.
    #[must_use]
    pub fn stats(&self) -> &StatsHistory {
        self.stats_history()
    }
}

/// Floats rounded to integers summing to `total` by largest remainders (spec
/// game/explanations): each to its floor or ceiling when the floats sum to
/// `total`; otherwise the difference is first spread over the values in
/// proportion to their size. Ties go to the earlier value.
#[must_use]
pub(crate) fn round_to_sum(values: &[f64], total: i64) -> Vec<i64> {
    if values.is_empty() {
        return Vec::new();
    }
    #[allow(clippy::cast_precision_loss)]
    let gap = total as f64 - values.iter().sum::<f64>();
    let size: f64 = values.iter().map(|v| v.abs()).sum();
    let fitted: Vec<f64> = if gap.abs() < 1e-6 {
        values.to_vec()
    } else if size > 0.0 {
        values.iter().map(|v| v + gap * v.abs() / size).collect()
    } else {
        let mut v = values.to_vec();
        v[0] += gap;
        v
    };
    #[allow(clippy::cast_possible_truncation)]
    let mut out: Vec<i64> = fitted.iter().map(|v| v.floor() as i64).collect();
    let mut rest = total - out.iter().sum::<i64>();
    let mut order: Vec<usize> = (0..fitted.len()).collect();
    #[allow(clippy::cast_precision_loss)]
    let frac = |k: usize| fitted[k] - out[k] as f64;
    let fr: Vec<f64> = (0..fitted.len()).map(frac).collect();
    order.sort_by(|&a, &b| fr[b].total_cmp(&fr[a]).then(a.cmp(&b)));
    // `rest` is within [0, n] up to float error; any excess goes round
    // the values again.
    let n = i64::try_from(order.len()).expect("a count");
    while rest != 0 {
        let step = rest.signum();
        let k = usize::try_from(rest.abs().min(n)).expect("a count");
        let pick: Vec<usize> = if step > 0 {
            order[..k].to_vec()
        } else {
            order[order.len() - k..].to_vec()
        };
        for i in pick {
            out[i] += step;
            rest -= step;
        }
    }
    out
}
