//! S8 Pension rise (spec game/policy-scenarios, AC-POL-08):
//! `transfer.pension.point_value` 144,200 → 158,600 bani (1,442 → 1,586 lei,
//! +10%), effective T0. Commands: `commands/pension_rise.jsonl`.

use super::harness::{Check, Ind, Kind, Scenario, checked, watched};

const CHECKS: [Check; 4] = [
    checked(
        "state pension spending rises, by at most 10%",
        Ind::Flow("benefit.pension"),
        Kind::FlowPct,
        (1, 12),
        (600, 1050),
    ),
    checked(
        "household consumption rises",
        Ind::Consumption,
        Kind::FlowPct,
        (1, 12),
        (30, 300),
    ),
    watched(
        "the deficit is larger",
        Ind::Deficit,
        Kind::PctOfGdp,
        (1, 12),
        (30, 120),
    ),
    watched(
        "prices are slightly higher",
        Ind::PriceLevel,
        Kind::LevelPct,
        (6, 18),
        (0, 150),
    ),
];

/// S8.
pub static S8: Scenario = Scenario {
    id: "S8",
    ac: "AC-POL-08",
    commands: "pension_rise.jsonl",
    checks: &CHECKS,
};

/// **AC-POL-08** `[sim]` Pension rise (S8): with the pension point 10% higher
/// from T0, state pension spending is 6–10.5% higher and household
/// consumption 0.3–3% higher over the first year.
///
/// Readings: those of `policy/mod.rs`. State pension spending is the
/// government's payments under `benefit.pension` (`FiscalMonth::spending`),
/// flow kind, months 1–12; consumption is `MonthStats::consumption`, months
/// 1–12. Watched: the deficit, the price level.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the policy suite runs in release builds only: cargo test --release -p econ-core --test acceptance -- --test-threads=1 policy::"
)]
fn ac_pol_08() {
    super::harness::assert_scenario(&S8, 1000);
}
