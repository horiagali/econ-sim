//! Trade and the exchange rate on the `World` (feature `trade`; spec
//! `economy/trade-fx`, "First version", roadmap M3): its parameters and
//! read-only state for the `[sim]` criteria AC-TFX-03 to 05.
//!
//! API skeleton written by the test writer with those tests, before the code
//! (ADR-0014). The bodies come with the implementation change.

use econ_mech_trade::FxParams;
use econ_types::{Bani, Month};

use super::World;

/// Trade's parameters (spec, "Tuning parameters"), millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeParams {
    /// The exchange rate's step.
    pub fx: FxParams,
    /// `armington_sigma` σ (1.5).
    pub sigma_ppm: u32,
    /// `share_adjustment`, a month (0.1).
    pub share_adjustment_ppm: u32,
    /// `export_elasticity` ε (1.0).
    pub export_elasticity_ppm: u32,
    /// `reserve_floor`, in months of imports (3).
    pub reserve_floor_months: u32,
}

impl TradeParams {
    /// The spec's defaults.
    #[must_use]
    pub fn spec_defaults() -> TradeParams {
        TradeParams {
            fx: FxParams::spec_defaults(),
            sigma_ppm: 1_500_000,
            share_adjustment_ppm: 100_000,
            export_elasticity_ppm: 1_000_000,
            reserve_floor_months: 3,
        }
    }
}

/// The outside world and the exchange rate after the latest day.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Foreign {
    /// `exchange_rate`: lei per euro, millionths.
    pub exchange_rate_ppm: u32,
    /// Its value at the start (e0).
    pub start_rate_ppm: u32,
    /// The lei price of each catalogue industry's imports, millionths of its
    /// starting price (the euro price times e / e0).
    pub import_price_ppm: Vec<u32>,
    /// The central bank's floor of foreign assets: `reserve_floor` months of
    /// imports, at the latest monthly close.
    pub reserves_floor: Bani,
}

/// The rest of the world in a closed month, national bani.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeMonth {
    /// The month.
    pub month: Month,
    /// Exports, real units (bani at starting prices).
    pub exports_volume: i64,
    /// The current account: exports less imports, less primary income paid
    /// abroad.
    pub current_account: Bani,
    /// The central bank's intervention: foreign assets sold (> 0) or bought
    /// (< 0) against the rest of the world's lei deposits.
    pub intervention: Bani,
}

impl World {
    /// The outside world and the exchange rate.
    #[must_use]
    pub fn foreign(&self) -> &Foreign {
        unimplemented!("econ-core trade: written in the implementation change")
    }

    /// The rest of the world in every closed month, in order.
    #[must_use]
    pub fn trade_months(&self) -> &[TradeMonth] {
        unimplemented!("econ-core trade: written in the implementation change")
    }
}
