//! S2 Income-tax cut (spec game/policy-scenarios, AC-POL-02):
//! `tax.income.schedule` `[[0, 1000]]` → `[[0, 500]]` (flat 10% → 5%),
//! effective T0. Commands: `commands/income_tax_cut.jsonl`.

use super::harness::{Check, Ind, Kind, Scenario, checked, watched};

const CHECKS: [Check; 5] = [
    checked(
        "income-tax revenue falls, by less than the rate",
        Ind::Flow("tax.income"),
        Kind::FlowPct,
        (1, 12),
        (-6500, -2500),
    ),
    checked(
        "household consumption rises",
        Ind::Consumption,
        Kind::FlowPct,
        (2, 12),
        (20, 400),
    ),
    watched(
        "the deficit is larger",
        Ind::Deficit,
        Kind::PctOfGdp,
        (1, 12),
        (10, 120),
    ),
    watched(
        "prices are slightly higher",
        Ind::PriceLevel,
        Kind::LevelPct,
        (6, 18),
        (0, 200),
    ),
    watched(
        "unemployment is no higher",
        Ind::Unemployment,
        Kind::Points,
        (6, 18),
        (-100, 10),
    ),
];

/// S2.
pub static S2: Scenario = Scenario {
    id: "S2",
    ac: "AC-POL-02",
    commands: "income_tax_cut.jsonl",
    checks: &CHECKS,
};

/// **AC-POL-02** `[sim]` Income-tax cut (S2): with a flat 5% income tax from
/// T0, income-tax revenue falls by 25–65% and household consumption rises
/// by 0.2–4% over the first year, within the bands of S2.
///
/// Readings: those of `policy/mod.rs`. Income-tax revenue is the government's
/// receipts under `tax.income` (`FiscalMonth::revenue`), flow kind, months
/// 1–12; consumption is `MonthStats::consumption`, flow kind, months 2–12
/// (the table's window; "over the first year" in the criterion). Watched:
/// the deficit, the price level, unemployment.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the policy suite runs in release builds only: cargo test --release -p econ-core --test acceptance -- --test-threads=1 policy::"
)]
fn ac_pol_02() {
    super::harness::assert_scenario(&S2, 1000);
}
