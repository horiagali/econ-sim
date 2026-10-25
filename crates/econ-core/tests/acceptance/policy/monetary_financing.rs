//! S5 Monetary financing (spec game/policy-scenarios, AC-POL-05):
//! `money.monetary_financing` 0 → 800,000,000,000 bani a month, effective
//! T0, back to 0 on 1 March 2024 (after 24 months). Spending is unchanged.
//! Commands: `commands/monetary_financing.jsonl`.

use super::harness::{Check, Ind, Kind, Scenario, checked, watched};

const CHECKS: [Check; 5] = [
    checked(
        "expected inflation rises",
        Ind::ExpectedInflation,
        Kind::Points,
        (3, 12),
        (50, 1200),
    ),
    checked(
        "credibility falls",
        Ind::Credibility,
        Kind::Points,
        (6, 24),
        (-7000, -200),
    ),
    checked(
        "prices are higher",
        Ind::PriceLevel,
        Kind::LevelPct,
        (12, 24),
        (50, 2500),
    ),
    watched(
        "inflation is higher",
        Ind::Inflation,
        Kind::Points,
        (13, 30),
        (50, 2000),
    ),
    watched(
        "the leu is weaker",
        Ind::ExchangeRate,
        Kind::LevelPct,
        (6, 24),
        (10, 3000),
    ),
];

/// S5.
pub static S5: Scenario = Scenario {
    id: "S5",
    ac: "AC-POL-05",
    commands: "monetary_financing.jsonl",
    checks: &CHECKS,
};

/// **AC-POL-05** `[sim]` Monetary financing (S5): with the central bank
/// funding 8 billion lei a month for 24 months, expected inflation is higher
/// in months 3–12, credibility lower in months 6–24 and the price level
/// higher in months 12–24, within the bands of S5.
///
/// Readings: those of `policy/mod.rs`. Expected inflation is
/// `MonthStats::expected_inflation_ppm` in points (+0.5 to +12); credibility
/// is `World::central_bank().credibility_ppm` at each close, ×100, in points
/// (−70 to −2); the price level +0.5% to +25%. Watched: inflation (months
/// 13–30), the exchange rate. Behind `lever_monetary_financing`:
/// `money.monetary_financing` is `planned` today, so the commands file is
/// rejected with `not_yet` until the lever is wired.
#[cfg(feature = "lever_monetary_financing")]
#[test]
fn ac_pol_05() {
    super::harness::assert_scenario(&S5, 1000);
}
