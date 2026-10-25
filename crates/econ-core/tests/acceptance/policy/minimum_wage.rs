//! S7 Minimum-wage rise (spec game/policy-scenarios, AC-POL-07):
//! `labour.minimum_wage` 230,000 → 300,000 bani (2,300 → 3,000 lei, +30%),
//! effective T0. Commands: `commands/minimum_wage.jsonl`.

use super::harness::{Check, Ind, Kind, Scenario, checked, watched};

const CHECKS: [Check; 4] = [
    checked(
        "the mean wage rises",
        Ind::MeanWage,
        Kind::LevelPct,
        (1, 6),
        (150, 1200),
    ),
    checked(
        "prices are higher",
        Ind::PriceLevel,
        Kind::LevelPct,
        (6, 18),
        (20, 500),
    ),
    watched(
        "employment falls a little, if at all",
        Ind::Employment,
        Kind::LevelPct,
        (6, 18),
        (-300, 50),
    ),
    watched(
        "fewer households in hardship",
        Ind::Hardship,
        Kind::LevelPct,
        (3, 12),
        (-3000, 0),
    ),
];

/// S7.
pub static S7: Scenario = Scenario {
    id: "S7",
    ac: "AC-POL-07",
    commands: "minimum_wage.jsonl",
    checks: &CHECKS,
};

/// **AC-POL-07** `[sim]` Minimum-wage rise (S7): with the minimum wage at
/// 3,000 lei from T0, the mean gross wage is 1.5–12% higher in months 1–6
/// and the price level 0.2–5% higher in months 6–18.
///
/// Readings: those of `policy/mod.rs`. The mean gross wage is
/// `MonthStats::wage_bill / employed`, level kind. Watched: employment
/// (`MonthStats::employed`), households in hardship
/// (`MonthStats::hardship_households`).
#[test]
fn ac_pol_07() {
    super::harness::assert_scenario(&S7, 1000);
}
