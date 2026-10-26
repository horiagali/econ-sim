//! Investment and capital (spec `economy/investment-capital`, "First
//! version", roadmap M3): the capital book with its depreciation and work in
//! progress, the investment budget, and dividends with their split by owner.
//! Pure functions, no I/O; profit, the postings and the monthly close live
//! with the `World`.
//!
//! Money is whole bani; real capital is in real bani (bani at starting
//! prices); rates and shares are millionths (a year for rates).
//!
//! Written before its acceptance tests (owner's decision, 2026-10-21: the
//! dividends first, one test session after); the tests come from the spec.

use econ_types::Bani;

const ONE: i128 = 1_000_000;

/// `a · b / c`, rounded half away from zero; 0 when `c` is 0.
fn ratio(a: i128, b: i128, c: i128) -> i128 {
    if c == 0 {
        return 0;
    }
    let n = a * b;
    let (n, c) = if c < 0 { (-n, -c) } else { (n, c) };
    if n >= 0 {
        econ_types::div_i128(n + c / 2, c)
    } else {
        -econ_types::div_i128(-n + c / 2, c)
    }
}

fn bani(v: i128) -> Bani {
    Bani(i64::try_from(v).expect("an amount that fits i64 bani"))
}

/// The spec's tuning parameters, millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvestParams {
    /// `depreciation_rate` δ, a year (6.5%).
    pub depreciation_ppm: u32,
    /// `gestation_months` (3).
    pub gestation_months: u8,
    /// cu*, the utilisation at which the accelerator is neutral (0.8).
    pub cu_target_ppm: u32,
    /// `beta_cu`, a year per unit of utilisation gap (0.5).
    pub beta_cu_ppm: u32,
    /// `beta_r`, a year per unit of real rate (1.0).
    pub beta_r_ppm: u32,
    /// `growth_adjustment` λ, a month (1/24).
    pub growth_adjust_ppm: u32,
    /// `growth_bound`, a year (±10%).
    pub growth_bound_ppm: u32,
    /// `payout_domestic` (0.3).
    pub payout_domestic_ppm: u32,
    /// `payout_foreign` (0.45).
    pub payout_foreign_ppm: u32,
    /// `payout_state` (0.5).
    pub payout_state_ppm: u32,
    /// `cash_sweep` σ, a month (0.1).
    pub cash_sweep_ppm: u32,
}

impl InvestParams {
    /// The spec's defaults.
    #[must_use]
    pub fn spec_defaults() -> InvestParams {
        InvestParams {
            depreciation_ppm: 65_000,
            gestation_months: 3,
            cu_target_ppm: 800_000,
            beta_cu_ppm: 500_000,
            beta_r_ppm: 1_000_000,
            growth_adjust_ppm: 41_667,
            growth_bound_ppm: 100_000,
            payout_domestic_ppm: 300_000,
            payout_foreign_ppm: 450_000,
            payout_state_ppm: 500_000,
            cash_sweep_ppm: 100_000,
        }
    }
}

/// Expected growth at the start (spec, "Starting state"): the starting
/// investment over the capital, less depreciation, a year.
#[must_use]
pub fn starting_growth_ppm(start_investment_year: Bani, capital: i64, p: &InvestParams) -> i32 {
    if capital <= 0 {
        return 0;
    }
    let rate = ratio(
        i128::from(start_investment_year.get()),
        ONE,
        i128::from(capital),
    );
    i32::try_from(rate - i128::from(p.depreciation_ppm)).unwrap_or(0)
}

/// The month's desired investment (step 1, AC-INV-01): `p^I K / 12 · max(0,
/// δ + g^e + β_cu (cu − cu*) − β_r (ρ − ρ0))`, rounded once.
pub fn desired_investment(
    capital: i64,
    price_index_ppm: u32,
    utilisation_ppm: u32,
    real_rate_gap_ppm: i32,
    expected_growth_ppm: i32,
    p: &InvestParams,
) -> Bani {
    if capital <= 0 {
        return Bani::ZERO;
    }
    // The bracket, a year, in millionths of millionths.
    let bracket = (i128::from(p.depreciation_ppm) + i128::from(expected_growth_ppm)) * ONE
        + i128::from(p.beta_cu_ppm) * (i128::from(utilisation_ppm) - i128::from(p.cu_target_ppm))
        - i128::from(p.beta_r_ppm) * i128::from(real_rate_gap_ppm);
    if bracket <= 0 {
        return Bani::ZERO;
    }
    let value = i128::from(capital) * i128::from(price_index_ppm);
    bani(ratio(value, bracket, 12 * ONE * ONE * ONE))
}

/// What a unit can finance: its desired investment, at most its spare money
/// and credit room (step 1).
pub fn financed(desired: Bani, spare: Bani, credit_room: Bani) -> Bani {
    desired
        .min(spare.max(Bani::ZERO) + credit_room.max(Bani::ZERO))
        .max(Bani::ZERO)
}

/// Expected growth after a monthly close (step 7): `g + λ (ĝ − g)`, with the
/// realised growth bounded by `growth_bound`.
#[must_use]
pub fn next_expected_growth_ppm(current_ppm: i32, realised_ppm: i32, p: &InvestParams) -> i32 {
    let bound = i32::try_from(p.growth_bound_ppm).unwrap_or(i32::MAX);
    let realised = realised_ppm.clamp(-bound, bound);
    let step = ratio(
        i128::from(realised) - i128::from(current_ppm),
        i128::from(p.growth_adjust_ppm),
        ONE,
    );
    i32::try_from(i128::from(current_ppm) + step).unwrap_or(current_ppm)
}

/// A unit's capital in place and its projects in progress, real bani.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapitalBook {
    /// Capital in place.
    pub capital: i64,
    /// Work in progress, oldest first, `gestation_months` long.
    pub in_progress: Vec<i64>,
}

impl CapitalBook {
    /// The book at the start: each month of work in progress holds a month
    /// of the starting investment.
    #[must_use]
    pub fn new(capital: i64, start_month_investment: i64, p: &InvestParams) -> CapitalBook {
        CapitalBook {
            capital,
            in_progress: vec![start_month_investment; usize::from(p.gestation_months.max(1))],
        }
    }

    /// A month of depreciation: `round(K δ / 12)`.
    #[must_use]
    pub fn depreciation(&self, p: &InvestParams) -> i64 {
        i64::try_from(ratio(
            i128::from(self.capital),
            i128::from(p.depreciation_ppm),
            12 * ONE,
        ))
        .expect("smaller than the capital")
    }

    /// The monthly close (step 6, AC-INV-02): `K' = K − round(K δ / 12) +
    /// the oldest work in progress`; the month's spending enters the newest
    /// slot. Returns (depreciated, completed).
    pub fn close_month(&mut self, spent_real: i64, p: &InvestParams) -> (i64, i64) {
        let depreciated = self.depreciation(p);
        let completed = if self.in_progress.is_empty() {
            spent_real
        } else {
            let oldest = self.in_progress.remove(0);
            self.in_progress.push(spent_real);
            oldest
        };
        self.capital = self.capital - depreciated + completed;
        (depreciated, completed)
    }
}

/// Who controls a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Controller {
    /// Domestic private owners.
    Domestic,
    /// Foreign owners.
    Foreign,
    /// The state.
    State,
}

/// A unit's dividend (step 5, AC-INV-03): `min(spare, θ_c max(0, Π) +
/// σ max(0, M − m0 C))`, never negative; `cash_target` is `m0 C`.
pub fn dividend(
    profit_after_tax: Bani,
    deposits: Bani,
    cash_target: Bani,
    spare: Bani,
    controller: Controller,
    p: &InvestParams,
) -> Bani {
    let payout = match controller {
        Controller::Domestic => p.payout_domestic_ppm,
        Controller::Foreign => p.payout_foreign_ppm,
        Controller::State => p.payout_state_ppm,
    };
    let from_profit = ratio(
        i128::from(profit_after_tax.get().max(0)),
        i128::from(payout),
        ONE,
    );
    let excess = i128::from((deposits - cash_target).get().max(0));
    let sweep = ratio(excess, i128::from(p.cash_sweep_ppm), ONE);
    bani(from_profit + sweep).min(spare).max(Bani::ZERO)
}

/// An amount split by ownership shares (state, domestic, foreign), by
/// largest remainder, ties to the first: the parts add up to the amount.
/// With no share at all, everything is domestic.
pub fn split_by_ownership(
    amount: Bani,
    state_ppm: u32,
    domestic_ppm: u32,
    foreign_ppm: u32,
) -> [Bani; 3] {
    let w = [
        i128::from(state_ppm),
        i128::from(domestic_ppm),
        i128::from(foreign_ppm),
    ];
    let sum: i128 = w.iter().sum();
    let total = i128::from(amount.get());
    if sum == 0 {
        return [Bani::ZERO, amount, Bani::ZERO];
    }
    let mut parts = [0i128; 3];
    let mut rems = [(0i128, 0usize); 3];
    for k in 0..3 {
        let x = total * w[k];
        parts[k] = x / sum;
        rems[k] = (x % sum, k);
    }
    let mut left = total - parts.iter().sum::<i128>();
    rems.sort_by_key(|&(r, k)| (std::cmp::Reverse(r), k));
    for (_, k) in rems {
        if left <= 0 {
            break;
        }
        parts[k] += 1;
        left -= 1;
    }
    [bani(parts[0]), bani(parts[1]), bani(parts[2])]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_budget_reproduces_the_start_and_reacts() {
        let p = InvestParams::spec_defaults();
        let k = 1_200_000_000;
        let g = starting_growth_ppm(Bani(120_000_000), k, &p);
        assert_eq!(g, 35_000);
        let base = desired_investment(k, 1_000_000, 800_000, 0, g, &p);
        assert_eq!(base, Bani(10_000_000), "a month of the starting investment");
        assert!(desired_investment(k, 1_000_000, 800_000, 20_000, g, &p) < base);
        assert!(desired_investment(k, 1_000_000, 900_000, 0, g, &p) > base);
        assert_eq!(
            desired_investment(k, 1_000_000, 0, 500_000, g, &p),
            Bani::ZERO
        );
        assert_eq!(financed(base, Bani(3), Bani(4)), Bani(7));
    }

    #[test]
    fn capital_completes_after_the_gestation() {
        let p = InvestParams::spec_defaults();
        let mut b = CapitalBook::new(1_200_000, 0, &p);
        let (d, c) = b.close_month(500, &p);
        assert_eq!((d, c), (6_500, 0));
        b.close_month(0, &p);
        let (_, c) = b.close_month(0, &p);
        assert_eq!(c, 0, "not yet at the second close after");
        let (_, c) = b.close_month(0, &p);
        assert_eq!(c, 500, "three closes later");
    }

    #[test]
    fn dividends_and_their_split() {
        let p = InvestParams::spec_defaults();
        let d = dividend(
            Bani(1_000),
            Bani(5_000),
            Bani(4_000),
            Bani(10_000),
            Controller::Foreign,
            &p,
        );
        assert_eq!(d, Bani(450 + 100));
        assert_eq!(
            dividend(
                Bani(1_000),
                Bani(0),
                Bani(0),
                Bani(200),
                Controller::State,
                &p
            ),
            Bani(200),
            "capped by spare money"
        );
        assert_eq!(
            dividend(
                Bani(-5),
                Bani(0),
                Bani(0),
                Bani(9),
                Controller::Domestic,
                &p
            ),
            Bani::ZERO
        );
        let s = split_by_ownership(Bani(10), 1, 1, 1);
        assert_eq!(s, [Bani(4), Bani(3), Bani(3)]);
        assert_eq!(
            split_by_ownership(Bani(7), 0, 0, 0),
            [Bani(0), Bani(7), Bani(0)]
        );
    }

    #[test]
    fn expected_growth_moves_slowly_and_bounded() {
        let p = InvestParams::spec_defaults();
        let g = next_expected_growth_ppm(35_000, 500_000, &p);
        assert_eq!(g, 35_000 + ratio(100_000 - 35_000, 41_667, ONE) as i32);
    }
}
