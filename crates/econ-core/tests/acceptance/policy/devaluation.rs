//! S6 Devaluation (spec game/policy-scenarios, AC-POL-06): the leu devalued
//! by 10% against the euro on T0, the managed float continuing from the new
//! level, through the `market` lever `fx.devaluation` (a signed rate, a
//! one-off step of `exchange_rate` on its effective day; owner's decision of
//! 2026-10-25) set to 1000 basis points. Commands:
//! `commands/devaluation.jsonl`.

use super::harness::{Check, Ind, Kind, Scenario, checked, watched};

const CHECKS: [Check; 5] = [
    checked(
        "the leu is weaker",
        Ind::ExchangeRate,
        Kind::LevelPct,
        (1, 3),
        (500, 1100),
    ),
    checked(
        "prices are higher, by much less than 10%",
        Ind::PriceLevel,
        Kind::LevelPct,
        (6, 12),
        (50, 600),
    ),
    checked(
        "exports rise",
        Ind::ExportVolume,
        Kind::FlowPct,
        (7, 18),
        (100, 1200),
    ),
    watched(
        "import prices in lei rise",
        Ind::ImportPriceIndex,
        Kind::LevelPct,
        (1, 3),
        (500, 1100),
    ),
    watched(
        "the current account improves after the J-curve",
        Ind::CurrentAccount,
        Kind::PctOfGdp,
        (13, 24),
        (0, 300),
    ),
];

/// S6.
pub static S6: Scenario = Scenario {
    id: "S6",
    ac: "AC-POL-06",
    commands: "devaluation.jsonl",
    checks: &CHECKS,
};

/// **AC-POL-06** `[sim]` Devaluation (S6): with the leu devalued by 10% on
/// T0, the exchange rate is 5–11% weaker in months 1–3, the price level
/// 0.5–6% higher in months 6–12 and export volumes 1–12% higher in months
/// 7–18.
///
/// Readings: those of `policy/mod.rs`. "Devalued by 10%" is the command
/// `fx.devaluation` = 1000 (+10%), effective 1 March 2022, as the spec's
/// Open questions decided; "weaker" is `exchange_rate` (lei per euro)
/// higher than the baseline's. Exports are `TradeMonth::exports_volume`.
/// Watched: the import price index (plain mean over industries, see
/// `Ind::ImportPriceIndex`), the current account. Behind
/// `lever_fx_devaluation`: no lever `fx.devaluation` is registered today, so
/// the commands file is rejected with `unknown_lever` until it is.
#[cfg(feature = "lever_fx_devaluation")]
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the policy suite runs in release builds only: cargo test --release -p econ-core --test acceptance -- --test-threads=1 policy::"
)]
fn ac_pol_06() {
    super::harness::assert_scenario(&S6, 1000);
}
