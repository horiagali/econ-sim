//! S9 Oil-price shock (spec game/policy-scenarios, AC-POL-09):
//! `world.fuel_price[oil]` 32,400 → 48,600 bani a barrel (+50%), effective
//! T0, permanent. Commands: `commands/oil_shock.jsonl`.

use super::harness::{Check, Ind, Kind, Scenario, checked, watched};

const CHECKS: [Check; 4] = [
    checked(
        "prices are higher",
        Ind::PriceLevel,
        Kind::LevelPct,
        (3, 12),
        (20, 400),
    ),
    checked(
        "the current account worsens",
        Ind::CurrentAccount,
        Kind::PctOfGdp,
        (1, 12),
        (-200, -10),
    ),
    watched(
        "fuel is dearer",
        Ind::Price("refined_petroleum"),
        Kind::LevelPct,
        (1, 3),
        (500, 5000),
    ),
    watched(
        "real GDP is lower",
        Ind::RealGdp,
        Kind::FlowPct,
        (6, 18),
        (-200, 0),
    ),
];

/// S9.
pub static S9: Scenario = Scenario {
    id: "S9",
    ac: "AC-POL-09",
    commands: "oil_shock.jsonl",
    checks: &CHECKS,
};

/// **AC-POL-09** `[sim]` Oil shock (S9): with the world oil price 50% higher
/// from T0, the price level is 0.2–4% higher in months 3–12 and the current
/// account 0.1–2% of GDP lower over the first year.
///
/// Readings: those of `policy/mod.rs`. The current account is
/// `TradeMonth::current_account`, % of GDP kind over months 1–12 (−2 to
/// −0.1). Watched: the price of `refined_petroleum` (`World::industries()
/// .price_ppm` at the close), real GDP. Behind `lever_fuel_price`:
/// `world.fuel_price` is `planned` today, so the commands file is rejected
/// with `not_yet` until the lever is wired.
#[cfg(feature = "lever_fuel_price")]
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the policy suite runs in release builds only: cargo test --release -p econ-core --test acceptance -- --test-threads=1 policy::"
)]
fn ac_pol_09() {
    super::harness::assert_scenario(&S9, 1000);
}
