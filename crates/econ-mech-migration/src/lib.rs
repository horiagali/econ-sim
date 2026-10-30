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

use econ_num::math;
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
    // R = ω / H: ω in basis points (10,000 = 1), H in millionths.
    let h = u64::from(home_net_wage_eur_index_ppm.max(1));
    let r = (u64::from(wages_abroad_bp) * 100 * 1_000_000 * 2 + h) / (2 * h);
    u32::try_from(r).unwrap_or(u32::MAX)
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
    let m = |ppm: u32| f64::from(ppm) / 1e6;
    let beta_u = m(p.unemployment_response_ppm);
    let mut x = f64::from(base_ppm)
        * math::powf(m(pull_ppm), m(p.emigration_elasticity_ppm))
        * (1.0 + beta_u * m(unemp_ppm))
        / (1.0 + beta_u * m(unemp0_ppm));
    if linked {
        x *= 1.0 + m(p.network_effect_ppm);
    }
    if job_seeker {
        x *= m(p.unemployed_factor_ppm);
    }
    let x = x.min(f64::from(p.max_emigration_ppm));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let v = (x + 0.5) as u32;
    v.min(p.max_emigration_ppm)
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
    candidates.sort_unstable_by_key(|c| (c.priority, c.record));
    let goal = i128::from(target) + i128::from(*carry);
    let mut taken = Vec::new();
    let mut weight = 0i128;
    for c in candidates.iter() {
        let w = i128::from(c.weight);
        if weight + w / 2 > goal {
            break;
        }
        weight += w;
        taken.push(c.record);
    }
    // When every candidate moved, the rest of the target is dropped.
    *carry = if taken.len() == candidates.len() && !candidates.is_empty() {
        0
    } else {
        i64::try_from(goal - weight).expect("a carry")
    };
    taken
}

/// What one linked emigrant sends in a month (step 3, AC-MIG-03), euro
/// cents: `r̄ · ω^η · (1 − δ)^(months_abroad / 12)`.
pub fn remittance_eur_cents(
    base_cents: i64,
    wages_abroad_bp: u32,
    months_abroad: u16,
    p: &MigrationParams,
) -> i64 {
    let omega = f64::from(wages_abroad_bp) / 10_000.0;
    let keep = 1.0 - f64::from(p.remittance_decay_ppm_year) / 1e6;
    #[allow(clippy::cast_precision_loss)]
    let x = base_cents as f64
        * math::powf(omega, f64::from(p.remittance_elasticity_ppm) / 1e6)
        * math::powf(keep, f64::from(months_abroad) / 12.0);
    #[allow(clippy::cast_possible_truncation)]
    let v = x.round() as i64;
    v
}

/// Euro cents in lei at a rate of lei per euro in millionths, to the ban
/// (AC-MIG-03).
pub fn eur_to_lei(eur_cents: i64, exchange_rate_ppm: u64) -> Bani {
    // A euro cent is e bani; to the nearest ban.
    let x = i128::from(eur_cents) * i128::from(exchange_rate_ppm);
    let v = if x >= 0 {
        (x + 500_000) / 1_000_000
    } else {
        -((-x + 500_000) / 1_000_000)
    };
    Bani(i64::try_from(v).expect("an amount"))
}

/// The logit shares of the destinations of a move out of `origin` (step 6,
/// AC-MIG-04), millionths summing to 1,000,000, one per county; the origin's
/// is 0. `utility_ppm[d]` is `V_cd` in millionths.
pub fn destination_ppm(origin: u8, utility_ppm: &[i64]) -> Vec<u32> {
    let n = utility_ppm.len();
    let o = usize::from(origin);
    if n < 2 {
        return vec![0; n];
    }
    // Softmax over the destinations, shifted by the largest utility.
    #[allow(clippy::cast_precision_loss)]
    let top = (0..n)
        .filter(|&d| d != o)
        .map(|d| utility_ppm[d])
        .max()
        .unwrap_or(0) as f64;
    #[allow(clippy::cast_precision_loss)]
    let e: Vec<f64> = (0..n)
        .map(|d| {
            if d == o {
                0.0
            } else {
                math::exp((utility_ppm[d] as f64 - top) / 1e6)
            }
        })
        .collect();
    let sum: f64 = e.iter().sum();
    let exact: Vec<f64> = e.iter().map(|&x| x / sum * 1e6).collect();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let mut shares: Vec<u32> = exact.iter().map(|&x| x.floor() as u32).collect();
    let left = 1_000_000 - shares.iter().sum::<u32>();
    // The rest to the largest remainders, ties to the lower index.
    let mut order: Vec<usize> = (0..n).filter(|&d| d != o).collect();
    order.sort_by(|&a, &b| {
        let (ra, rb) = (exact[a] - exact[a].floor(), exact[b] - exact[b].floor());
        rb.partial_cmp(&ra)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    for &d in order.iter().take(left as usize) {
        shares[d] += 1;
    }
    shares
}
