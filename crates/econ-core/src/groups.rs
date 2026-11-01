//! Group queries (spec `game/group-queries`, roadmap M6, second slice): any
//! filter over the persons, its summary at the latest close, its breakdown by
//! an attribute, and watched groups whose summaries join the statistics
//! history at every close (feature `groups`, off by default). Read-only: a
//! query changes no state, and watching changes no state either (the watched
//! series are outputs, outside the state hash).
//!
//! The API is the spec's sketch, written with the acceptance tests before the
//! code (ADR-0014): a test writer's skeleton whose bodies are
//! `unimplemented!` until the code exists.
//!
//! Choices the sketch left open, stated by the test writer:
//! - `Condition::Age { from, to }` is the range of whole years `[from, to)`.
//! - `Condition::Industry` lists catalogue industry indices, `None` for a
//!   person without an employer unit; `Condition::Occupation` lists ISCO
//!   major groups 0 to 9 and `econ_popgen::NOT_EMPLOYED` for none.
//! - `Condition::Urban(true)` is urban, `false` rural.
//! - `Condition::Class` lists the standard-of-living classes of poverty
//!   (0 = at risk, 1 to 3 above it); without the feature `poverty` every
//!   household is in no class, so a `Class` condition gives an empty group.
//! - In a breakdown, each member's `u32` is the attribute's value: the index
//!   of the county, region, fifth, class, edu level, activity, enrolment
//!   level or ISCO group; for `Sex` 0 = F, 1 = M; for `Urban` 0 = rural, 1 =
//!   urban; for `AgeBand` the band's index (the registry's age bands); for
//!   `Industry` the catalogue index, and the industries' count for "none".
//! - A mean over no one (`income_per_adult`, `sol_milli`, `approval_milli`
//!   of an empty group) is `None`.

use econ_popgen::{Activity, EduLevel, Enrolled, Sex};
use econ_types::Bani;

use crate::world::World;

/// The fewest person records a summary is shown without a flag.
pub const MIN_RECORDS: u32 = 30;

/// One condition of a filter: the values of one attribute it accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// The household's county (scenario order).
    County(Vec<u8>),
    /// The household's development region (scenario order).
    Region(Vec<u8>),
    /// The household's locality type at load: `true` urban.
    Urban(bool),
    /// The person's age in whole years, `[from, to)`.
    Age {
        /// The first year included.
        from: u16,
        /// The first year excluded.
        to: u16,
    },
    /// The person's sex.
    Sex(Vec<Sex>),
    /// The person's highest completed education.
    Edu(Vec<EduLevel>),
    /// The person's refreshed activity.
    Activity(Vec<Activity>),
    /// ISCO major groups (or `econ_popgen::NOT_EMPLOYED`).
    Occupation(Vec<u8>),
    /// The industry of the person's employer unit, `None` for none.
    Industry(Vec<Option<u16>>),
    /// The household's income fifth (`hh_fifth`, 0 = poorest).
    Fifth(Vec<u8>),
    /// The household's standard-of-living class (feature `poverty`).
    Class(Vec<u8>),
    /// The person's level of enrolment.
    Enrolled(Vec<Enrolled>),
}

/// A conjunction of conditions; empty: every resident.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    /// The conditions, all of which a person meets.
    pub conditions: Vec<Condition>,
}

/// An attribute to break a group down by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "api", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum Attribute {
    County,
    Region,
    Urban,
    AgeBand,
    Sex,
    Edu,
    Activity,
    Occupation,
    Industry,
    Fifth,
    Class,
    Enrolled,
}

/// What a group earns, pays, receives and thinks, at the latest close
/// (spec, "The summary"). Money is national (weighted) bani; household
/// quantities are shared equally among the household's resident members.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupSummary {
    /// Weighted persons.
    pub persons: u64,
    /// Weighted households with at least one member in the group.
    pub households: u64,
    /// Person rows behind it.
    pub person_records: u32,
    /// Household rows behind it.
    pub household_records: u32,
    /// Fewer than [`MIN_RECORDS`] person rows.
    pub small_sample: bool,
    /// Weighted persons employed (refreshed activity).
    pub employed: u64,
    /// Weighted persons unemployed.
    pub unemployed: u64,
    /// `unemployed / (employed + unemployed)`, millionths (0 with no labour
    /// force).
    pub unemployment_rate_ppm: u32,
    /// The month's gross wages (pay slips), weighted.
    pub wages_gross: Bani,
    /// The month's net wages.
    pub wages_net: Bani,
    /// The month's gross pensions.
    pub pensions: Bani,
    /// The month's unemployment benefits.
    pub unemployment_benefits: Bani,
    /// Child allowance and minimum income of the month, shared per member.
    pub household_transfers: Bani,
    /// Income tax, pension and health contributions of the persons, and the
    /// household's self-employment tax shared per member.
    pub taxes_paid: Bani,
    /// Cash and deposits, shared per member.
    pub money: Bani,
    /// Mortgages and consumer debt, shared per member.
    pub debt: Bani,
    /// The mean equivalised recent real income of its households, weighted
    /// by the group's persons (feature `poverty`).
    pub income_per_adult: Option<Bani>,
    /// The mean SoL of its persons, thousandths (feature `poverty`).
    pub sol_milli: Option<u32>,
    /// The mean approval of its adults, thousandths (feature `approval`).
    pub approval_milli: Option<i32>,
}

impl World {
    /// The summary of a filter at the latest close.
    #[must_use]
    pub fn group(&self, f: &Filter) -> GroupSummary {
        let _ = f;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// One summary per value of `by` (with the filter's conditions), in the
    /// attribute's order.
    #[must_use]
    pub fn group_breakdown(&self, f: &Filter, by: Attribute) -> Vec<(u32, GroupSummary)> {
        let _ = (f, by);
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// Watch a filter under `name`: its summary is recorded from the next
    /// close on.
    pub fn watch_group(&mut self, name: &str, f: Filter) {
        let _ = (name, f);
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The summaries of a watched group, one per close since it was watched.
    #[must_use]
    pub fn watched(&self, name: &str) -> Option<&[GroupSummary]> {
        let _ = name;
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The names of the watched groups, in the order they were watched.
    #[must_use]
    pub fn watched_names(&self) -> Vec<&str> {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}
