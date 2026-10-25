//! Trade and the exchange rate on the `World` (feature `trade`; spec
//! `economy/trade-fx`, "First version", roadmap M3): its parameters and
//! read-only state for the `[sim]` criteria AC-TFX-03 to 05, and the steps
//! that move them.
//!
//! Trade acts under a law with the income taxes of 2021, as the treasury and
//! the banks do (the placeholder law keeps M2's imports at the starting price
//! and exports at fixed real levels, and its goldens).
//! - **Each day:** import prices in lei are each good's euro price times
//!   e / e0; imported inputs, the imported parts of government purchases and
//!   of investment pay them, and producers' unit costs carry them. Export
//!   demand on each good is `export_demand` at the day's prices.
//! - **Monthly close:** households' import shares move toward their Armington
//!   shares; the exchange rate takes its step, and the central bank offsets
//!   part of it against the rest of the world's lei deposits while its
//!   foreign assets stay above the floor.
//! - **After the day's last phase:** the rest of the world's flows into the
//!   month, and on a month end the month's report.
//!
//! > **Simplification (M3):** imported inputs change in price, not in
//! > quantity (their coefficients by using industry hold no goods to
//! > substitute between); the fixed government purchases and investment keep
//! > their imported parts. Only the households' basket substitutes.

use std::collections::BTreeMap;

use econ_ledger::{FlowCode, Instrument, Sector};
use econ_mech_trade::{FxParams, armington_share_ppm, export_demand, fx_step};
use econ_types::{Bani, Month};

use super::World;
use super::money::{Postings, bani};

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
    /// The capital account inside `current_account` (a memo): the EU's
    /// capital grants (spec economy/eu-funds, AC-TFX-03 restated).
    pub capital_account: Bani,
}

/// EUR/RON on 31 December 2021 (BNR), millionths.
const START_RATE_PPM: u32 = 4_948_100;
/// The outside world's constants (decision D8): the euro area's policy rate
/// and expected inflation, a year, millionths.
const FOREIGN_RATE_PPM: i32 = -5_000;
const FOREIGN_EXPECTED_INFLATION_PPM: i32 = 20_000;
/// What the `World` keeps for trade.
#[derive(Debug, Clone, Default)]
pub(super) struct ForeignState {
    active: bool,
    view: Foreign,
    months: Vec<TradeMonth>,
    /// Each good's euro price, millionths of its starting price.
    world_price_ppm: Vec<u32>,
    /// Foreign demand, millionths of the start (`world.foreign_demand_growth`).
    demand_index_ppm: u32,
    /// The oil goods (`crude_oil`, `refined_petroleum`), whose euro price
    /// `world.fuel_price[oil]` scales, and each industry's oil share of its
    /// imported inputs, millionths.
    oil_goods: Vec<usize>,
    oil_import_ppm: Vec<u32>,
    /// The starting export quantities a day and households' import shares.
    exports0: Vec<i64>,
    basket_import0: Vec<u32>,
    /// The rest of the world's flows of the month so far by code, received
    /// positive, financing excluded; exports served this month, real.
    month_flows: BTreeMap<FlowCode, i128>,
    exports_month: i64,
    /// A month closed today: its intervention, reported after the last phase.
    closing: Option<(Month, Bani)>,
}

fn is_financing(c: FlowCode) -> bool {
    c.name().starts_with("finance.")
}

impl World {
    /// The outside world and the exchange rate.
    #[must_use]
    pub fn foreign(&self) -> &Foreign {
        &self.fx.view
    }

    /// The rest of the world in every closed month, in order.
    #[must_use]
    pub fn trade_months(&self) -> &[TradeMonth] {
        &self.fx.months
    }

    /// Trade at load: the outside world's prices (times a world-constants
    /// edit's factor), the starting rate (times a depreciation edit's factor,
    /// with e0 unchanged), exports and import shares.
    pub(super) fn start_trade(
        &mut self,
        world_factor_ppm: u32,
        depreciation_ppm: u32,
        oil_import_ppm: &[u32],
    ) {
        let active = self.law.income.is_some();
        let n_i = self.industries.id.len();
        self.fx = ForeignState {
            active,
            ..ForeignState::default()
        };
        if !active {
            return;
        }
        let oil_goods: Vec<usize> = self
            .industries
            .id
            .iter()
            .enumerate()
            .filter(|(_, id)| *id == "crude_oil" || *id == "refined_petroleum")
            .map(|(g, _)| g)
            .collect();
        let f = &mut self.fx;
        f.oil_goods = oil_goods;
        f.oil_import_ppm = oil_import_ppm.to_vec();
        f.world_price_ppm = vec![world_factor_ppm; n_i];
        f.exports0 = self.ii.exp_q.clone();
        f.basket_import0 = self.ii.basket_import_ppm.clone();
        f.view.exchange_rate_ppm =
            u32::try_from(u64::from(START_RATE_PPM) * u64::from(depreciation_ppm) / 1_000_000)
                .unwrap_or(u32::MAX);
        f.view.start_rate_ppm = START_RATE_PPM;
        f.demand_index_ppm = 1_000_000;
        self.trade_prices();
        self.ledger
            .allow_negative(Sector::RestOfWorld, Instrument::Deposits);
    }

    /// e / e0, millionths rounded to the nearest, for other modules
    /// (1,000,000 when trade does not act).
    #[cfg_attr(not(feature = "revenue"), allow(dead_code))]
    pub(super) fn trade_rate_index_ppm(&self) -> u32 {
        if self.fx.active {
            let v = &self.fx.view;
            let e0 = u64::from(v.start_rate_ppm.max(1));
            u32::try_from((u64::from(v.exchange_rate_ppm) * 1_000_000 + e0 / 2) / e0)
                .unwrap_or(u32::MAX)
        } else {
            1_000_000
        }
    }

    /// e / e0, millionths.
    fn rate_index_ppm(&self) -> u32 {
        let v = &self.fx.view;
        u32::try_from(
            u64::from(v.exchange_rate_ppm) * 1_000_000 / u64::from(v.start_rate_ppm.max(1)),
        )
        .unwrap_or(u32::MAX)
    }

    /// The day's import prices.
    fn trade_prices(&mut self) {
        let e = u64::from(self.rate_index_ppm());
        self.fx.view.import_price_ppm = self
            .fx
            .world_price_ppm
            .iter()
            .map(|&p| u32::try_from(u64::from(p) * e / 1_000_000).unwrap_or(u32::MAX))
            .collect();
    }

    /// The import price index of the day, millionths: the mean of the import
    /// prices of the goods other than oil (1,000,000 when trade does not
    /// act). Oil has its own price ([`World::oil_import_price_ppm`]); while
    /// it moves with the rest, this is the mean of all goods.
    pub(super) fn import_index_ppm(&self) -> u32 {
        let p = &self.fx.view.import_price_ppm;
        if !self.fx.active || p.is_empty() {
            return 1_000_000;
        }
        let oil = &self.fx.oil_goods;
        let (sum, n) = p
            .iter()
            .enumerate()
            .filter(|(g, _)| !oil.contains(g))
            .fold((0u64, 0u64), |(s, n), (_, &x)| (s + u64::from(x), n + 1));
        if n == 0 {
            return 1_000_000;
        }
        u32::try_from(sum / n).unwrap_or(u32::MAX)
    }

    /// The day's import price of oil, millionths: that of `refined_petroleum`
    /// (crude moves with it), or the index when the catalogue has no oil.
    fn oil_import_price_ppm(&self) -> u32 {
        self.fx
            .oil_goods
            .last()
            .and_then(|&g| self.fx.view.import_price_ppm.get(g).copied())
            .unwrap_or_else(|| self.import_index_ppm())
    }

    /// Industry `j`'s imported inputs worth `v` at the starting prices, at
    /// the day's: the oil share at oil's import price, the rest at the
    /// index. While oil moves with the rest, the index alone.
    pub(super) fn trade_inputs_import_value(&self, j: usize, v: i128) -> i128 {
        let idx = i128::from(self.import_index_ppm());
        let oil = i128::from(self.oil_import_price_ppm());
        let o = i128::from(self.fx.oil_import_ppm.get(j).copied().unwrap_or(0));
        if !self.fx.active || oil == idx || o == 0 {
            return if idx == 1_000_000 {
                v
            } else {
                super::money::ratio(v, idx, 1_000_000)
            };
        }
        let price = super::money::ratio((1_000_000 - o) * idx + o * oil, 1, 1_000_000);
        super::money::ratio(v, price, 1_000_000)
    }

    /// The start of a day: import prices and the day's export demand.
    pub(super) fn trade_begin_day(&mut self) {
        if !self.fx.active {
            return;
        }
        self.trade_prices();
        let e = self.rate_index_ppm();
        let eps = self.params.trade.export_elasticity_ppm;
        for g in 0..self.ii.exp_q.len() {
            self.ii.exp_q[g] = export_demand(
                self.fx.exports0[g],
                self.industries.price_ppm[g],
                e,
                self.fx.world_price_ppm[g],
                self.fx.demand_index_ppm,
                eps,
            )
            .max(0);
        }
    }

    /// Exports served today, real (from the goods market).
    pub(super) fn trade_exported(&mut self, quantity: i64) {
        if self.fx.active {
            self.fx.exports_month += quantity;
        }
    }

    /// The foreign interest rate, millionths a year: `world.foreign_rate` in
    /// a world under `law.json`, else the constant of M3.
    fn foreign_rate_ppm(&self) -> i32 {
        #[cfg(feature = "levers")]
        if let Some(crate::levers::LeverValue::Rate(bp)) = self
            .lever_state()
            .and_then(|s| s.law.get(crate::levers::LeverId::WorldForeignRate, None))
        {
            return bp * 100;
        }
        FOREIGN_RATE_PPM
    }

    /// A month of the world settings' growth (spec game/lever-system, "World
    /// settings"): each good's euro price by `world.foreign_price_growth`
    /// and foreign demand by `world.foreign_demand_growth`, each the mean of
    /// the two blocs' rates, converted from a year to a month.
    #[cfg(feature = "levers")]
    fn world_settings_month(&mut self) {
        use crate::levers::{LeverId, LeverValue};
        let (price_bp, demand_bp) = {
            let Some(st) = self.lever_state() else {
                return;
            };
            let mean_bp = |l: LeverId| -> Option<i64> {
                let keys = st.registry.get(l).keys.len();
                let rates: Vec<i64> = (0..keys)
                    .filter_map(|k| match st.law.get(l, u32::try_from(k).ok()) {
                        Some(LeverValue::Rate(v)) => Some(i64::from(*v)),
                        _ => None,
                    })
                    .collect();
                let n = i64::try_from(rates.len()).ok()?;
                (n > 0).then(|| rates.iter().sum::<i64>() / n)
            };
            (
                mean_bp(LeverId::WorldForeignPriceGrowth),
                mean_bp(LeverId::WorldForeignDemandGrowth),
            )
        };
        let month = |bp: i64| econ_num::rates::annual_to_period_ppm(bp * 100, 12);
        let grow = |v: u32, g: i64| {
            u32::try_from((i64::from(v) * (1_000_000 + g) + 500_000) / 1_000_000)
                .unwrap_or(u32::MAX)
        };
        if let Some(bp) = price_bp {
            let g = month(bp);
            for p in &mut self.fx.world_price_ppm {
                *p = grow(*p, g);
            }
        }
        if let Some(bp) = demand_bp {
            self.fx.demand_index_ppm = grow(self.fx.demand_index_ppm, month(bp));
        }
    }

    /// `fx.devaluation` on its effective day (spec economy/trade-fx,
    /// amendment of 2026-10-25): the exchange rate steps once by `bp`
    /// (1000 = the leu 10% weaker; negative revalues), and the managed float
    /// continues from the new level. Import prices and export demand follow
    /// the same day.
    #[cfg(feature = "levers")]
    pub(crate) fn fx_devalue(&mut self, bp: i32) {
        if !self.fx.active || bp == 0 {
            return;
        }
        let factor = (10_000 + i64::from(bp)).max(1);
        let e = i64::from(self.fx.view.exchange_rate_ppm);
        self.fx.view.exchange_rate_ppm =
            u32::try_from(((e * factor + 5_000) / 10_000).max(1)).unwrap_or(u32::MAX);
        self.trade_begin_day();
    }

    /// `world.fuel_price[oil]` on its effective day (owner's decision of
    /// 2026-10-25): the euro prices of `crude_oil` and `refined_petroleum`
    /// scale by the new oil price over the old, so each is its growth path
    /// times the oil price over its starting value. Import prices follow the
    /// same day.
    #[cfg(feature = "levers")]
    pub(crate) fn fx_oil_price(&mut self, old: i64, new: i64) {
        if !self.fx.active || old <= 0 || old == new {
            return;
        }
        for &g in &self.fx.oil_goods {
            let p = &mut self.fx.world_price_ppm[g];
            *p = u32::try_from(
                (i128::from(*p) * i128::from(new.max(0)) + i128::from(old) / 2) / i128::from(old),
            )
            .unwrap_or(u32::MAX);
        }
        self.trade_begin_day();
    }

    /// The policy rate the exchange rate sees.
    fn trade_policy_rate(&self) -> i32 {
        #[cfg(feature = "banking")]
        if self.banks_active() {
            return self.bank_policy_rate();
        }
        17_500
    }

    /// The month's flow of the rest of the world so far, today's included.
    fn row_month(&self, keep: impl Fn(FlowCode) -> bool) -> i128 {
        FlowCode::ALL
            .into_iter()
            .filter(|&c| !is_financing(c) && keep(c))
            .map(|c| {
                self.fx.month_flows.get(&c).copied().unwrap_or(0)
                    + i128::from(self.ledger.flows().get(c, Sector::RestOfWorld).get())
            })
            .sum()
    }

    /// Monthly close, last: the households' import shares, the exchange rate
    /// and the central bank's intervention.
    pub(super) fn trade_close(&mut self) {
        if !self.fx.active {
            return;
        }
        let month = Month(u32::try_from(self.months.len().saturating_sub(1)).expect("months"));
        // The world settings' growth of the month (world prices, foreign demand).
        #[cfg(feature = "levers")]
        self.world_settings_month();
        // Import shares toward Armington's (step 2).
        let p = self.params.trade;
        for g in 0..self.ii.basket_import_ppm.len() {
            let target = armington_share_ppm(
                self.fx.basket_import0[g],
                self.fx.view.import_price_ppm[g],
                self.industries.price_ppm[g],
                p.sigma_ppm,
            );
            let s = i64::from(self.ii.basket_import_ppm[g]);
            let moved = s + (i64::from(target) - s) * i64::from(p.share_adjustment_ppm) / 1_000_000;
            self.ii.basket_import_ppm[g] =
                u32::try_from(moved.clamp(0, 1_000_000)).expect("a share");
        }
        // The exchange rate (step 4): the current account so far this month
        // (received by the rest of the world is paid by residents).
        let ca = -self.row_month(|_| true);
        let gdp = self.months.last().map_or(Bani::ZERO, |m| m.gdp_production);
        let ca_ppm = if gdp > Bani::ZERO {
            i32::try_from(ca * 1_000_000 / i128::from(gdp.get())).unwrap_or(0)
        } else {
            0
        };
        let imports = self.row_month(|c| c == FlowCode::IMPORTS);
        let floor = bani(imports * i128::from(p.reserve_floor_months));
        let reserves = self
            .ledger
            .balance(Sector::CentralBank, Instrument::ForeignAssets);
        let gap = (self.trade_policy_rate() - self.mac.expected_inflation_ppm)
            - (self.foreign_rate_ppm() - FOREIGN_EXPECTED_INFLATION_PPM);
        let s = fx_step(
            self.fx.view.exchange_rate_ppm,
            gap,
            ca_ppm,
            reserves > floor,
            gdp,
            &p.fx,
        );
        self.fx.view.exchange_rate_ppm = s.rate_ppm;
        self.fx.view.reserves_floor = floor;
        // Never sell more foreign assets than lie above the floor, nor more
        // than the banks' reserves can pay for (an extreme lever, such as an
        // excise duty at the top of its range, ends in a bad economy, not a
        // crash).
        let bank_reserves = self
            .ledger
            .balance(Sector::Banks, Instrument::Reserves)
            .max(Bani::ZERO);
        let intervention = s.intervention.min((reserves - floor).max(Bani::ZERO));
        // `fx.intervention`: the player's amount on top of the rule's.
        #[cfg(feature = "levers")]
        let intervention = self.fx_with_player(intervention, reserves - floor);
        let intervention = intervention.min(bank_reserves);
        let mut post = Postings::default();
        let code = FlowCode::FX_INTERVENTION;
        if intervention > Bani::ZERO {
            // Sells foreign assets: the rest of the world pays for them in lei.
            post.leg(
                Sector::RestOfWorld,
                Sector::Banks,
                Instrument::Deposits,
                intervention,
                code,
            );
            post.leg(
                Sector::Banks,
                Sector::CentralBank,
                Instrument::Reserves,
                intervention,
                code,
            );
            post.leg(
                Sector::CentralBank,
                Sector::RestOfWorld,
                Instrument::ForeignAssets,
                intervention,
                code,
            );
        } else if intervention < Bani::ZERO {
            let x = Bani::ZERO - intervention;
            post.leg(
                Sector::RestOfWorld,
                Sector::CentralBank,
                Instrument::ForeignAssets,
                x,
                code,
            );
            post.leg(
                Sector::CentralBank,
                Sector::Banks,
                Instrument::Reserves,
                x,
                code,
            );
            post.leg(
                Sector::Banks,
                Sector::RestOfWorld,
                Instrument::Deposits,
                x,
                code,
            );
        }
        post.commit(&mut self.ledger);
        self.fx.closing = Some((month, intervention));
        self.trade_prices();
    }

    /// After the day's last phase: the rest of the world's flows of the day
    /// into the month, and on a month end the month's report.
    pub(super) fn trade_day_end(&mut self) {
        if !self.fx.active {
            return;
        }
        for c in FlowCode::ALL {
            if is_financing(c) {
                continue;
            }
            let v = i128::from(self.ledger.flows().get(c, Sector::RestOfWorld).get());
            if v != 0 {
                *self.fx.month_flows.entry(c).or_insert(0) += v;
            }
        }
        let Some((month, intervention)) = self.fx.closing.take() else {
            return;
        };
        let flows = std::mem::take(&mut self.fx.month_flows);
        let ca: i128 = -flows.values().sum::<i128>();
        let capital = -flows
            .get(&FlowCode::TRANSFER_EU_GRANTS_CAPITAL)
            .copied()
            .unwrap_or(0);
        let volume = std::mem::take(&mut self.fx.exports_month);
        self.fx.months.push(TradeMonth {
            month,
            exports_volume: volume,
            current_account: bani(ca),
            intervention,
            capital_account: bani(capital),
        });
    }

    /// Trade's state for the state hash.
    pub(super) fn trade_hash(&self, write: &mut impl FnMut(u64)) {
        if !self.fx.active {
            return;
        }
        write(u64::from(self.fx.view.exchange_rate_ppm));
        for &s in &self.ii.basket_import_ppm {
            write(u64::from(s));
        }
        #[allow(clippy::cast_sign_loss)]
        write(self.fx.exports_month as u64);
    }
}
