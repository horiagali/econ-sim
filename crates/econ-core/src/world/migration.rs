//! Migration, the diaspora and remittances on the `World` (spec
//! `economy/migration-remittances`, feature `migration`): the month's flows
//! and the diaspora, read-only, for the `[sim]` criteria AC-MIG-05 to 17.
//!
//! Test writer's addition (ADR-0014), written with those tests before the
//! code from the spec's API sketch; every body is `unimplemented!` until the
//! implementation fills it.

use econ_types::{Bani, Month};

use super::World;

/// The age bands of the month's emigrants, `[from, to)` in years, the last
/// open: those of the research note's table (Eurostat `migr_emi2`).
pub const MIGRATION_AGE_BANDS: [(u16, u16); 7] =
    [(0, 15), (15, 25), (25, 35), (35, 45), (45, 55), (55, 65), (65, u16::MAX)];

/// A month of migration, written at the monthly close. Persons are weighted
/// (real persons); money is national (weighted) bani.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationMonth {
    /// The month.
    pub month: Month,
    /// Emigrants: persons who left Romania at this close, alone (linked) or
    /// with their whole household.
    pub emigrants: u64,
    /// The same by [`MIGRATION_AGE_BANDS`].
    pub emigrants_by_age: [u64; 7],
    /// Returns of Romanian citizens: linked emigrants who rejoined their
    /// household and persons back from the diaspora stock.
    pub returns: u64,
    /// Foreign immigrants.
    pub immigrants: u64,
    /// Births of the month (the demographics core's), for the balance of
    /// persons (AC-MIG-08).
    pub births: u64,
    /// Deaths of resident persons in the month (the demographics core's).
    pub deaths_resident: u64,
    /// Deaths abroad: linked emigrants and the diaspora stock.
    pub deaths_abroad: u64,
    /// Persons who moved into each county from another county, by county
    /// index (the scenario's order).
    pub moves_in: Vec<u64>,
    /// Persons who moved out of each county to another county.
    pub moves_out: Vec<u64>,
    /// Remittances paid to linked households this month, lei.
    pub remittances: Bani,
    /// The same in euro cents.
    pub remittances_eur_cents: i64,
    /// Each household record that emigrated whole at this close: its index,
    /// its weight before it left, and the net deposits per record it took to
    /// the rest of the world under `transfer.migrant_capital` (after its cash
    /// was deposited and its loans repaid or written off).
    pub emigrated_households: Vec<(u32, u32, Bani)>,
}

/// One cell of the diaspora stock: unlinked emigrants as counts of real
/// persons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StockCell {
    /// Origin development region.
    pub region: u8,
    /// Sex (`econ_popgen::Sex` as its index).
    pub sex: u8,
    /// Year of birth.
    pub birth_year: u16,
    /// Skill tier: low 0, mid 1, high 2.
    pub tier: u8,
    /// Real persons.
    pub persons: u64,
}

/// The diaspora now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiasporaView {
    /// Linked emigrants, weighted (persons with `abroad` in records of
    /// positive weight).
    pub linked: u64,
    /// The stock, by cell.
    pub stock: Vec<StockCell>,
}

impl World {
    /// Every closed month's migration, in order (one per monthly close).
    #[must_use]
    pub fn migration_months(&self) -> &[MigrationMonth] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// The diaspora now: linked emigrants and the stock.
    #[must_use]
    pub fn diaspora(&self) -> DiasporaView {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// Each person's `abroad` flag (a linked emigrant), aligned with
    /// [`super::PersonColumns`].
    #[must_use]
    pub fn abroad(&self) -> &[bool] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}
