//! S4 Deficit spending (spec game/policy-scenarios, AC-POL-04):
//! `spend.ministry_budget[education]` 3,242,470,000,000 → 3,842,470,000,000
//! bani and `spend.ministry_budget[health]` 5,768,260,000,000 →
//! 6,368,260,000,000 bani (+6 billion lei a year each), effective T0,
//! permanent. Commands: `commands/deficit_spending.jsonl`.

use super::harness::{Check, Ind, Kind, Scenario, checked, watched};

const CHECKS: [Check; 4] = [
    checked(
        "the deficit is larger, by less than the spending",
        Ind::Deficit,
        Kind::PctOfGdp,
        (1, 12),
        (30, 110),
    ),
    checked(
        "real GDP is higher",
        Ind::RealGdp,
        Kind::FlowPct,
        (3, 12),
        (10, 200),
    ),
    watched(
        "unemployment is lower",
        Ind::Unemployment,
        Kind::Points,
        (6, 18),
        (-100, -2),
    ),
    watched(
        "prices are slightly higher",
        Ind::PriceLevel,
        Kind::LevelPct,
        (6, 18),
        (0, 200),
    ),
];

/// S4.
pub static S4: Scenario = Scenario {
    id: "S4",
    ac: "AC-POL-04",
    commands: "deficit_spending.jsonl",
    checks: &CHECKS,
};

/// **AC-POL-04** `[sim]` Deficit spending (S4): with education and health
/// budgets each 6 billion lei a year higher from T0, the deficit is 0.3–1.1%
/// of GDP larger in the first year and real GDP 0.1–2% higher in months
/// 3–12.
///
/// Readings: those of `policy/mod.rs`. The deficit is `FiscalMonth::deficit`
/// (spending + interest − revenue), % of GDP kind over months 1–12 against
/// the baseline's nominal GDP; real GDP is `gdp_expenditure` over the CPI,
/// flow kind. Watched: unemployment, the price level.
#[test]
fn ac_pol_04() {
    super::harness::assert_scenario(&S4, 1000);
}
