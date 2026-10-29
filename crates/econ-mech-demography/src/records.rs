//! The operations on weighted records (spec `society/households-lifecycle`,
//! "Record operations"; AC-HHL-01 to 04): move out, split, merge, unite
//! (split then merge), make collective, and the leaver's money rule. Adding
//! and removing persons, ending a record and the compaction come with the
//! implementation. Every operation keeps Σ weight × balance of
//! each balance column and the weighted persons by every attribute exact,
//! except where the spec says otherwise.
//!
//! Balances are per real household (before the weight). Tenure: owner 0,
//! tenant 1, other 2 (Stage E's codes).
//!
//! API skeleton from the spec's API sketch, written with the acceptance
//! tests before the code (ADR-0014): the bodies come with the implementation
//! change.

use econ_popgen::{Activity, EduLevel, Role, Sex};
use econ_rng::KeyedRng;
use econ_types::{Bani, Day};

/// The balance column of `hh_cash`.
pub const CASH: usize = 0;
/// The balance column of `hh_deposits`.
pub const DEPOSITS: usize = 1;
/// The balance column of `hh_mortgage`.
pub const MORTGAGE: usize = 2;
/// The balance column of `hh_consumer_debt`.
pub const CONSUMER_DEBT: usize = 3;
/// The balance column of `hh_dwelling_value`.
pub const DWELLING: usize = 4;

/// Stage E's tenure code of an owner.
pub const OWNER: u8 = 0;
/// Stage E's tenure code of a tenant.
pub const TENANT: u8 = 1;

/// The households, one record per synthetic household (ended records stay
/// as tombstones until the yearly compaction).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HouseholdTable {
    /// `hh_key`: stable identity, never reused.
    pub hh_key: Vec<u64>,
    /// `hh_weight`: real households the record stands for.
    pub weight: Vec<u32>,
    /// `hh_county`.
    pub county: Vec<u8>,
    /// The development region of the county.
    pub region: Vec<u8>,
    /// Stage E's locality size class.
    pub locality: Vec<u8>,
    /// `hh_collective`.
    pub collective: Vec<bool>,
    /// Stage E's tenure.
    pub tenure: Vec<u8>,
    /// `hh_alive`: the record counts.
    pub alive: Vec<bool>,
    /// `hh_stagger_day`, 1 to 28.
    pub stagger_day: Vec<u8>,
    /// The day of the last spending decision; `None` before the first.
    pub last_decision: Vec<Option<Day>>,
    /// Everything received since day 0 (AC-HH-03).
    pub income_to_date: Vec<Bani>,
    /// Everything spent since day 0 (AC-HH-03).
    pub spending_to_date: Vec<Bani>,
    /// `hh_moved_to_date`: money moved into the household by these
    /// operations since day 0, net.
    pub moved_to_date: Vec<Bani>,
    /// `[column][record]`: the balance columns, [`CASH`] to [`DWELLING`] and
    /// any added later, per real household.
    pub balances: Vec<Vec<Bani>>,
}

/// The persons, one record per synthetic person (dead persons stay as
/// tombstones until the yearly compaction).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersonTable {
    /// `person_key`: stable identity, never reused.
    pub person_key: Vec<u64>,
    /// Index into the households.
    pub household: Vec<u32>,
    /// `alive`: the record counts.
    pub alive: Vec<bool>,
    /// `age`, months.
    pub age: Vec<u16>,
    /// `sex`.
    pub sex: Vec<Sex>,
    /// `role`.
    pub role: Vec<Role>,
    /// Main activity.
    pub activity: Vec<Activity>,
    /// Highest completed education.
    pub edu_level: Vec<EduLevel>,
    /// Firm unit, or `econ_popgen::NO_EMPLOYER`.
    pub employer: Vec<u16>,
    /// `origin_household`: the `hh_key` the person left when leaving home.
    pub origin_household: Vec<Option<u64>>,
}

/// The people state the operations change.
#[derive(Debug, Clone)]
pub struct Tables {
    /// Households.
    pub households: HouseholdTable,
    /// Persons.
    pub persons: PersonTable,
    /// The next `hh_key` to give.
    pub next_hh_key: u64,
    /// The next `person_key` to give.
    pub next_person_key: u64,
    /// The keyed draws (stagger days of new households on
    /// `Stream::Schedule`).
    pub rng: KeyedRng,
}

/// Why an operation was refused; nothing changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeCycleError {
    /// Merge of records of different weights.
    WeightsDiffer,
    /// The record does not hold exactly one live person.
    NotOnePerson(u32),
    /// The record is collective.
    Collective(u32),
    /// The record is a tombstone.
    Tombstone(u32),
}

/// Move out (leaving home, separation; AC-HHL-03): person `p` goes to a new
/// household of the same weight, county, region and locality, as its head,
/// tenant, with no dwelling and no debt; `cash` and `deposits` (per
/// household) move with it; `origin_household` is the old record's key.
/// Returns the new household's index.
pub fn move_out(t: &mut Tables, p: u32, cash: Bani, deposits: Bani, day: Day) -> u32 {
    let _ = (t, p, cash, deposits, day);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Split (AC-HHL-01): record `hh` of weight `w` keeps `w − k` and a copy of
/// weight `k` is appended, every member copied with new keys and the same
/// attributes and employer, every balance copied per household. Returns the
/// copy's index.
pub fn split(t: &mut Tables, hh: u32, k: u32, day: Day) -> u32 {
    let _ = (t, hh, k, day);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Merge two one-person private records of the same weight (AC-HHL-02):
/// `other`'s person moves into `keep`, the balances add per household and
/// `other` ends.
///
/// # Errors
/// [`LifeCycleError`]; nothing changes.
pub fn merge_singles(t: &mut Tables, keep: u32, other: u32) -> Result<(), LifeCycleError> {
    let _ = (t, keep, other);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The merge of a union (AC-HHL-02): if the weights of the two one-person
/// records differ, the one with the larger weight is first split so that a
/// part has the smaller weight (the leftover keeps its person and
/// balances); then the two equal records merge into `keep`'s (or into its
/// part of the smaller weight). Returns the merged record's index.
///
/// # Errors
/// [`LifeCycleError`]; nothing changes.
pub fn unite(t: &mut Tables, keep: u32, other: u32, day: Day) -> Result<u32, LifeCycleError> {
    let _ = (t, keep, other, day);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Make collective (orphans; AC-HHL-04): each live member of `hh` becomes a
/// one-member collective record of the same weight; cash and deposits are
/// split equally per member (floor; the remainder to the first member by
/// `person_key`), the dwelling goes to the first member. Returns the
/// members' records.
pub fn make_collective(t: &mut Tables, hh: u32, day: Day) -> Vec<u32> {
    let _ = (t, hh, day);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// What a leaver takes of one money balance (AC-HHL-03): `leave_home_share`
/// × the household's per-household balance ÷ its members aged 18 or more,
/// rounded down; nothing when the balance is zero or negative.
pub fn leave_home_amount(per_household: Bani, adults: u32, share_ppm: u32) -> Bani {
    let _ = (per_household, adults, share_ppm);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The life cycle's tuning parameters (the spec's table), millionths a year
/// unless said otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifeCycleParams {
    /// `leave_home_rate[sex, band]`: `[F, M][18–19, 20–24, 25–29, 30–34]`.
    pub leave_home_rate_ppm: [[u32; 4]; 2],
    /// `leave_home_job_ratio`.
    pub leave_home_job_ratio_ppm: u32,
    /// `leave_home_share`.
    pub leave_home_share_ppm: u32,
    /// `union_rate[band]`: the woman's 18–24, 25–29, 30–34, 35–39, 40–49.
    pub union_rate_ppm: [u32; 5],
    /// `union_age_gap`, months.
    pub union_age_gap_months: u16,
    /// `separation_rate`.
    pub separation_rate_ppm: u32,
    /// `separation_share`.
    pub separation_share_ppm: u32,
}

impl LifeCycleParams {
    /// The spec's defaults.
    #[must_use]
    pub fn spec_defaults() -> LifeCycleParams {
        LifeCycleParams {
            leave_home_rate_ppm: [
                [30_000, 120_000, 150_000, 120_000],
                [20_000, 50_000, 100_000, 100_000],
            ],
            leave_home_job_ratio_ppm: 2_000_000,
            leave_home_share_ppm: 250_000,
            union_rate_ppm: [150_000, 200_000, 150_000, 100_000, 50_000],
            union_age_gap_months: 24,
            separation_rate_ppm: 9_000,
            separation_share_ppm: 500_000,
        }
    }
}
