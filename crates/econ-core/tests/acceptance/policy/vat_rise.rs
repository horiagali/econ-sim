//! S3 VAT rise (spec game/policy-scenarios, AC-POL-03):
//! `tax.vat.rate_standard` 1900 → 2400 (19% → 24%, the 2010 change),
//! effective T0. Commands: `commands/vat_rise.jsonl`.

use super::harness::{Check, Ind, Kind, Scenario, checked, watched};

const CHECKS: [Check; 4] = [
    checked(
        "the price level jumps once",
        Ind::PriceLevel,
        Kind::LevelPct,
        (2, 6),
        (100, 600),
    ),
    checked(
        "inflation returns to baseline after the base month",
        Ind::Inflation,
        Kind::Points,
        (14, 24),
        (-150, 150),
    ),
    checked(
        "VAT revenue rises, by less than in proportion",
        Ind::Flow("tax.vat"),
        Kind::FlowPct,
        (1, 12),
        (800, 2600),
    ),
    watched(
        "real consumption falls",
        Ind::RealConsumption,
        Kind::FlowPct,
        (1, 12),
        (-400, -10),
    ),
];

/// S3.
pub static S3: Scenario = Scenario {
    id: "S3",
    ac: "AC-POL-03",
    commands: "vat_rise.jsonl",
    checks: &CHECKS,
};

/// **AC-POL-03** `[sim]` VAT rise (S3): with standard VAT at 24% from T0, the
/// price level is 1–6% higher in months 2–6, twelve-month inflation is
/// within 1.5 points of the baseline in months 14–24, and VAT revenue rises
/// by 8–26% in the first year.
///
/// Readings: those of `policy/mod.rs`. "Within 1.5 points" is the points
/// kind (the mean monthly difference over months 14–24) in the band
/// [−1.5, +1.5], which contains zero and so skips the sign and 2ν rules. VAT
/// revenue is the receipts under `tax.vat`, flow kind, months 1–12. Watched:
/// real consumption (consumption over the CPI).
#[test]
fn ac_pol_03() {
    super::harness::assert_scenario(&S3, 1000);
}
