//! The statistics registry (spec `game/statistics-registry`, roadmap M6): one
//! catalogue of every indicator the player can look at, and one history the
//! `World` fills at every close.
//!
//! **API skeleton only** (ADR-0014): the types and signatures of the spec's
//! "API sketch", written by the test writer with the acceptance tests
//! (AC-STAT-01 to 08) before the code; every body is `unimplemented!` until
//! the implementation fills it. Behind the feature `stats`, off by default.
//!
//! The registry's data is `crates/econ-core/indicators.toml`; the generator
//! `scripts/codegen_indicators.py` writes [`IndicatorId`] and the metadata
//! (spec, "The registry"). Neither exists yet: [`IndicatorId`] below is a
//! placeholder the generated enum replaces, and the tests name no variant of
//! it (they look indicators up by their dotted name in [`indicators`]).

use econ_types::Month;

use crate::world::World;

/// An indicator of the registry.
///
/// Placeholder (test writer's addition, ADR-0014): the generator writes the
/// real enum, one variant per entry of `indicators.toml` (`GdpNominal`,
/// `GdpReal`, …), and this one goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IndicatorId {
    /// Stands for the generated variants until they exist.
    Placeholder,
}

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
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// One point of a derived indicator at month `t`, by the spec's formulas in
/// integers rounded half away from zero (ADR-0006): `a` is the input's
/// monthly points from month 0, `b` the second input's (the denominator of
/// `ratio`, the CPI in millionths of `real`, the resident persons of
/// `per_capita`; unused by the growth rates). `None` where the formula has
/// no value (before month 12 for `growth_yoy`, at month 0 for `growth_mom`,
/// a zero denominator). Test writer's addition (ADR-0014).
#[must_use]
pub fn derive_point(op: DeriveOp, a: &[i64], b: &[i64], t: usize) -> Option<i64> {
    let _ = (op, a, b, t);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
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
pub struct StatsHistory {}

impl StatsHistory {
    /// The points of `id` at `key`; `None` when not computed.
    #[must_use]
    pub fn series(&self, id: IndicatorId, key: Key) -> Option<Series<'_>> {
        let _ = (id, key);
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The latest monthly point of `id` at `key`.
    #[must_use]
    pub fn latest(&self, id: IndicatorId, key: Key) -> Option<i64> {
        let _ = (id, key);
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The keys `id` has points for (national included).
    #[must_use]
    pub fn keys(&self, id: IndicatorId) -> &[Key] {
        let _ = id;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}

impl World {
    /// The statistics history.
    #[must_use]
    pub fn stats(&self) -> &StatsHistory {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}
