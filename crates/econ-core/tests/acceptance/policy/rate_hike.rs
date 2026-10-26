//! S1 Rate hike (spec game/policy-scenarios, AC-POL-01): `money.policy_rate`
//! 175 → 375 basis points (1.75% → 3.75%), effective T0, held to the end.
//! Commands: `commands/rate_hike.jsonl`.

use super::harness::{Check, Ind, Kind, Scenario, checked, watched};

const CHECKS: [Check; 6] = [
    checked(
        "the leu is stronger within three months",
        Ind::ExchangeRate,
        Kind::LevelPct,
        (1, 3),
        (-1000, -10),
    ),
    checked(
        "prices are lower after a year",
        Ind::PriceLevel,
        Kind::LevelPct,
        (12, 24),
        (-600, -20),
    ),
    // Watched until M13 brings household credit (owner, 2026-10-25).
    watched(
        "unemployment is higher",
        Ind::Unemployment,
        Kind::Points,
        (6, 18),
        (5, 250),
    ),
    watched(
        "inflation is lower",
        Ind::Inflation,
        Kind::Points,
        (12, 24),
        (-400, -10),
    ),
    watched(
        "real GDP is lower",
        Ind::RealGdp,
        Kind::FlowPct,
        (6, 18),
        (-400, -5),
    ),
    watched(
        "the deficit is larger (dearer debt)",
        Ind::Deficit,
        Kind::PctOfGdp,
        (1, 24),
        (0, 100),
    ),
];

/// S1.
pub static S1: Scenario = Scenario {
    id: "S1",
    ac: "AC-POL-01",
    commands: "rate_hike.jsonl",
    checks: &CHECKS,
};

/// **AC-POL-01** `[sim]` Rate hike (S1): with the policy rate 2 points
/// higher from T0, the leu is stronger in months 1–3, the price level is
/// lower in months 12–24, within the bands of S1. (Unemployment higher in
/// months 6–18 is watched until M13 brings household credit, then checked.)
///
/// Readings: those of `policy/mod.rs`. "The leu is stronger" means
/// `exchange_rate` (lei per euro) lower than the baseline's: −10% to −0.1%,
/// level kind, months 1–3. The price level: −6% to −0.2%, months 12–24.
/// Watched and printed: unemployment, inflation, real GDP, the deficit.
/// Behind `lever_policy_rate`: `money.policy_rate` is `planned` today, so the
/// commands file is rejected with `not_yet` until the lever is wired.
#[cfg(feature = "lever_policy_rate")]
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the policy suite runs in release builds only: cargo test --release -p econ-core --test acceptance -- --test-threads=1 policy::"
)]
fn ac_pol_01() {
    super::harness::assert_scenario(&S1, 1000);
}
