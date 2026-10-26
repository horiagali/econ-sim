//! Migration, the diaspora and remittances (spec
//! `economy/migration-remittances`, roadmap M5): the pull of abroad (step 1),
//! a person's emigration probability (step 2), whole-record alignment, the
//! remittance of a linked emigrant (step 3) and the destination shares of a
//! move between counties (step 6). Pure functions, no I/O; powers and
//! exponentials through `econ_num::math` (ADR-0006). The `World` walks its
//! tables and posts the ledger legs.
//!
//! Rates, ratios and indices are millionths; wages abroad are basis points of
//! the start (10,000 = 100%, the shape of the lever `world.wages_abroad`);
//! the exchange rate is lei per euro in millionths.
//!
//! API skeleton from the spec's API sketch, written with the acceptance tests
//! before the code (ADR-0014): the bodies come with the implementation
//! change.

use econ_types::Bani;

/// The tuning parameters of the spec's table, millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationParams {
    /// ε, `emigration_elasticity`.
    pub emigration_elasticity_ppm: u32,
    /// ε_r, `return_elasticity`.
    pub return_elasticity_ppm: u32,
    /// β_u, `emigration_unemployment`: the multiple per unit (1,000,000 ppm)
    /// of unemployment rate.
    pub unemployment_response_ppm: u32,
    /// β_net, `network_effect`.
    pub network_effect_ppm: u32,
    /// `unemployed_factor`.
    pub unemployed_factor_ppm: u32,
    /// p^max, `max_emigration_rate`, a month.
    pub max_emigration_ppm: u32,
    /// `household_share`.
    pub household_share_ppm: u32,
    /// `link_return`.
    pub link_return_ppm: u32,
    /// η, `remittance_elasticity`.
    pub remittance_elasticity_ppm: u32,
    /// δ, `remittance_decay`, a year.
    pub remittance_decay_ppm_year: u32,
}

impl MigrationParams {
    /// The spec's defaults: ε 1.5, ε_r 1.0, β_u 5, β_net 1.0, unemployed
    /// factor 2.0, p^max 2% a month, household share 0.3, link return 1.5,
    /// η 0.8, δ 5% a year.
    #[must_use]
    pub fn spec_defaults() -> MigrationParams {
        MigrationParams {
            emigration_elasticity_ppm: 1_500_000,
            return_elasticity_ppm: 1_000_000,
            unemployment_response_ppm: 5_000_000,
            network_effect_ppm: 1_000_000,
            unemployed_factor_ppm: 2_000_000,
            max_emigration_ppm: 20_000,
            household_share_ppm: 300_000,
            link_return_ppm: 1_500_000,
            remittance_elasticity_ppm: 800_000,
            remittance_decay_ppm_year: 50_000,
        }
    }
}

/// The pull of abroad, `R = ω / H` (step 1, AC-MIG-01), millionths: wages
/// abroad in basis points of the start over the home net wage index in euro
/// (millionths of the start).
pub fn pull_ppm(wages_abroad_bp: u32, home_net_wage_eur_index_ppm: u32) -> u32 {
    let _ = (wages_abroad_bp, home_net_wage_eur_index_ppm);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// A person's monthly emigration probability (step 2, AC-MIG-01),
/// millionths, inside the EU (`B` = 1): `min(p^max, p0 · R^ε · (1 + β_u u) /
/// (1 + β_u u0) · (1 + β_net · linked) · m^unemp)`. Rates `unemp_ppm` and
/// `unemp0_ppm` are the region's unemployment now and at the start.
pub fn emigration_ppm(
    base_ppm: u32,
    pull_ppm: u32,
    unemp_ppm: u32,
    unemp0_ppm: u32,
    linked: bool,
    job_seeker: bool,
    p: &MigrationParams,
) -> u32 {
    let _ = (base_ppm, pull_ppm, unemp_ppm, unemp0_ppm, linked, job_seeker, p);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// A record a cell may move: its index, its weight and its keyed priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    /// The record's index.
    pub record: u32,
    /// Its weight (real persons or households it stands for).
    pub weight: u32,
    /// Its keyed priority.
    pub priority: u64,
}

/// Whole-record alignment (AC-MIG-02): records taken in priority order, each
/// with its whole weight, until `target + carry` is met; `carry` becomes the
/// target less the weight taken. Returns the records taken, in the order
/// taken.
pub fn take_whole_records(target: i64, carry: &mut i64, candidates: &mut [Pick]) -> Vec<u32> {
    let _ = (target, carry, candidates);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// What one linked emigrant sends in a month (step 3, AC-MIG-03), euro
/// cents: `r̄ · ω^η · (1 − δ)^(months_abroad / 12)`.
pub fn remittance_eur_cents(
    base_cents: i64,
    wages_abroad_bp: u32,
    months_abroad: u16,
    p: &MigrationParams,
) -> i64 {
    let _ = (base_cents, wages_abroad_bp, months_abroad, p);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Euro cents in lei at a rate of lei per euro in millionths, to the ban
/// (AC-MIG-03).
pub fn eur_to_lei(eur_cents: i64, exchange_rate_ppm: u64) -> Bani {
    let _ = (eur_cents, exchange_rate_ppm);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The logit shares of the destinations of a move out of `origin` (step 6,
/// AC-MIG-04), millionths summing to 1,000,000, one per county; the origin's
/// is 0. `utility_ppm[d]` is `V_cd` in millionths.
pub fn destination_ppm(origin: u8, utility_ppm: &[i64]) -> Vec<u32> {
    let _ = (origin, utility_ppm);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}
