//! S7 Minimum-wage rise (spec game/policy-scenarios, AC-POL-07):
//! `labour.minimum_wage` 230,000 → 300,000 bani (2,300 → 3,000 lei, +30%),
//! effective T0. Commands: `commands/minimum_wage.jsonl`.

use super::harness::{Check, Ind, Kind, Scenario, checked, watched};

const CHECKS: [Check; 4] = [
    checked(
        "the mean wage of employees rises",
        Ind::MeanWage,
        Kind::LevelPct,
        (1, 6),
        (100, 1200),
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
/// 3,000 lei from T0, the mean gross wage of employees is 1–12% higher in
/// months 1–6 and the price level 0.2–5% higher in months 6–18.
///
/// Restated 2026-10-25 (owner decision): the mean gross wage is over
/// employees only, not the wage bill over all the employed, which counts the
/// self-employed (who earn no wage) and diluted the rise; and the band starts
/// at +1%, not +1.5%, as 21% of employees earn below 3,000 lei in the
/// starting data. It stays checked: a fall or no change fails.
///
/// Readings: those of `policy/mod.rs`. The mean gross wage of employees is
/// the weighted mean of `PersonColumns::wage` over persons whose `wage` is
/// above zero at the close (`Monthly::employee_wages`), level kind. Watched: employment
/// (`MonthStats::employed`), households in hardship
/// (`MonthStats::hardship_households`).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the policy suite runs in release builds only: cargo test --release -p econ-core --test acceptance -- --test-threads=1 policy::"
)]
fn ac_pol_07() {
    super::harness::assert_scenario(&S7, 1000);
}
