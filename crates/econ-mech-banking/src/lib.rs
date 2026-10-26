//! Money and banking (spec `economy/money-banking`, "First version", roadmap
//! M3): the rates of loans and deposits, a month of interest, the room a firm
//! unit has on its credit line, the write-off of excess debt and the banks'
//! capital adequacy. Pure functions, no I/O; the daily credit draws, the
//! month-end interest and repayments and the reserve check live with the
//! `World`.
//!
//! Money is whole bani; rates and shares are millionths (a year for rates).
//! Every division rounds half away from zero, once.

use econ_types::Bani;

/// One, in millionths.
const ONE: i128 = 1_000_000;

/// Mortgages' risk weight in capital adequacy (spec, step 6), millionths.
const MORTGAGE_WEIGHT_PPM: i128 = 350_000;

/// `a · b / c` rounded half away from zero; `c` must be positive.
fn ratio(a: i128, b: i128, c: i128) -> i128 {
    debug_assert!(c > 0);
    let n = a * b;
    if n >= 0 {
        econ_types::div_i128(n + c / 2, c)
    } else {
        -econ_types::div_i128(-n + c / 2, c)
    }
}

fn to_u32(v: i128) -> u32 {
    u32::try_from(v.clamp(0, i128::from(u32::MAX))).expect("clamped")
}

fn to_bani(v: i128) -> Bani {
    Bani(i64::try_from(v).expect("Bani overflow in econ-mech-banking"))
}

/// A unit's credit limit, `credit_limit_months` of its monthly revenue; a
/// negative revenue counts as none.
fn credit_limit(revenue_month: Bani, p: &BankParams) -> i128 {
    i128::from(revenue_month.get().max(0)) * i128::from(p.credit_limit_months)
}

/// The banks' parameters (spec, "Tuning parameters"). Rates and shares in
/// millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankParams {
    /// `spread_firms`: firm loans over the policy rate.
    pub spread_firms_ppm: u32,
    /// `spread_mortgage`.
    pub spread_mortgage_ppm: u32,
    /// `spread_consumer`.
    pub spread_consumer_ppm: u32,
    /// `deposit_spread`: deposits under the policy rate, the rate not below 0.
    pub deposit_spread_ppm: u32,
    /// `reserve_requirement`: reserves banks must hold, of deposits.
    pub reserve_requirement_ppm: u32,
    /// `car_min`: capital adequacy below which new credit stops.
    pub car_min_ppm: u32,
    /// `credit_limit_months`: how many months of revenue a unit may owe.
    pub credit_limit_months: u32,
    /// `firm_repayment_share`, a month.
    pub firm_repayment_ppm: u32,
    /// `recovery_rate`: what the banks recover of a written-off loan.
    pub recovery_ppm: u32,
}

impl BankParams {
    /// The spec's defaults: spreads 3.0, 2.75 and 7.25 points, deposits 1
    /// point under the policy rate, reserves 8%, `car_min` 10%, 12 months of
    /// revenue, 1/60 a month, recovery 40%.
    #[must_use]
    pub fn spec_defaults() -> BankParams {
        BankParams {
            spread_firms_ppm: 30_000,
            spread_mortgage_ppm: 27_500,
            spread_consumer_ppm: 72_500,
            deposit_spread_ppm: 10_000,
            reserve_requirement_ppm: 80_000,
            car_min_ppm: 100_000,
            credit_limit_months: 12,
            firm_repayment_ppm: 16_667,
            recovery_ppm: 400_000,
        }
    }
}

/// The rates of the month, millionths a year.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rates {
    /// Loans to firm units.
    pub firms_ppm: u32,
    /// Mortgages.
    pub mortgage_ppm: u32,
    /// Consumer loans.
    pub consumer_ppm: u32,
    /// Deposits.
    pub deposit_ppm: u32,
}

/// The rates for a policy rate and the banks' capital adequacy (AC-BANK-05):
/// the policy rate plus each spread; deposits the policy rate less the
/// deposit spread, not below 0.
///
/// `car_ppm` does not move the rates in M3: the spec's capital surcharge
/// (`s_cap`) and risk term have no parameter in the first version. A loan
/// rate that a very negative policy rate would push below 0 is floored at 0
/// (a `u32`); in the spec's range the spreads keep loan rates positive.
#[must_use]
pub fn rates(policy_ppm: i32, car_ppm: u32, p: &BankParams) -> Rates {
    let _ = car_ppm;
    let i = i128::from(policy_ppm);
    Rates {
        firms_ppm: to_u32(i + i128::from(p.spread_firms_ppm)),
        mortgage_ppm: to_u32(i + i128::from(p.spread_mortgage_ppm)),
        consumer_ppm: to_u32(i + i128::from(p.spread_consumer_ppm)),
        deposit_ppm: to_u32(i - i128::from(p.deposit_spread_ppm)),
    }
}

/// A month of interest on a balance: a twelfth of a year at the rate,
/// rounded once (half away from zero): `balance · rate / 12`. A negative
/// balance gives the mirror amount.
pub fn month_interest(balance: Bani, rate_ppm: u32) -> Bani {
    to_bani(ratio(
        i128::from(balance.get()),
        i128::from(rate_ppm),
        12 * ONE,
    ))
}

/// What a unit may still draw on its credit line: up to
/// `credit_limit_months` of its revenue, nothing while the banks' capital
/// adequacy is below `car_min`.
///
/// At exactly `car_min` the line is open ("below which new credit stops").
/// A negative revenue counts as none; a unit already over its limit has no
/// room (never a negative amount).
pub fn credit_room(loans: Bani, revenue_month: Bani, car_ppm: u32, p: &BankParams) -> Bani {
    if car_ppm < p.car_min_ppm {
        return Bani::ZERO;
    }
    to_bani((credit_limit(revenue_month, p) - i128::from(loans.get())).max(0))
}

/// The write-off of a unit's loans above `credit_limit_months` of its
/// revenue: (written off, the banks' loss, `1 − recovery` of it).
///
/// "Written off" is the whole excess over the limit; the loss is
/// `(1 − recovery) · excess`, rounded once, and is what the unit's loans fall
/// by (it keeps owing the recovered part). A recovery above 100% counts as
/// 100%; a negative revenue counts as none.
pub fn write_off(loans: Bani, revenue_month: Bani, p: &BankParams) -> (Bani, Bani) {
    let excess = (i128::from(loans.get()) - credit_limit(revenue_month, p)).max(0);
    let lost = ONE - i128::from(p.recovery_ppm).min(ONE);
    (to_bani(excess), to_bani(ratio(excess, lost, ONE)))
}

/// Capital adequacy, millionths: equity over risk-weighted loans (firms and
/// consumer loans 100%, mortgages 35%).
///
/// Negative loan amounts count as none. Equity at or below 0 gives 0; positive
/// equity with no risk-weighted loans gives `u32::MAX` (unbounded adequacy),
/// as does any ratio above it.
#[must_use]
pub fn capital_ratio_ppm(equity: Bani, firm_loans: Bani, consumer: Bani, mortgages: Bani) -> u32 {
    let at_full = i128::from(firm_loans.get().max(0)) + i128::from(consumer.get().max(0));
    // Risk-weighted loans, in millionths of a ban.
    let rwa = at_full * ONE + i128::from(mortgages.get().max(0)) * MORTGAGE_WEIGHT_PPM;
    let equity = i128::from(equity.get());
    if equity <= 0 {
        return 0;
    }
    if rwa == 0 {
        return u32::MAX;
    }
    to_u32(ratio(equity * ONE, ONE, rwa))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_add_spreads_and_floor_deposits() {
        let p = BankParams::spec_defaults();
        let r = rates(17_500, 150_000, &p);
        assert_eq!(
            r,
            Rates {
                firms_ppm: 47_500,
                mortgage_ppm: 45_000,
                consumer_ppm: 90_000,
                deposit_ppm: 7_500,
            }
        );
        // Negative policy rate: deposits at 0, loans keep their spreads.
        let r = rates(-5_000, 150_000, &p);
        assert_eq!(r.deposit_ppm, 0);
        assert_eq!(r.firms_ppm, 25_000);
    }

    #[test]
    fn month_interest_rounds_once_half_away() {
        // 1_200 bani at 5%: 5 bani exactly.
        assert_eq!(month_interest(Bani(1_200), 50_000), Bani(5));
        // 6 bani at 100%: 0.5 rounds away from zero, both signs.
        assert_eq!(month_interest(Bani(6), 1_000_000), Bani(1));
        assert_eq!(month_interest(Bani(-6), 1_000_000), Bani(-1));
        assert_eq!(month_interest(Bani(5), 1_000_000), Bani(0));
        assert_eq!(month_interest(Bani(1_000), 0), Bani::ZERO);
    }

    #[test]
    fn credit_room_and_car_min() {
        let p = BankParams::spec_defaults();
        assert_eq!(credit_room(Bani(1_000), Bani(100), 100_000, &p), Bani(200));
        assert_eq!(credit_room(Bani(1_000), Bani(100), 99_999, &p), Bani::ZERO);
        assert_eq!(credit_room(Bani(5_000), Bani(100), 200_000, &p), Bani::ZERO);
        assert_eq!(credit_room(Bani(0), Bani(-100), 200_000, &p), Bani::ZERO);
    }

    #[test]
    fn write_off_excess_and_loss() {
        let p = BankParams::spec_defaults();
        assert_eq!(
            write_off(Bani(1_000), Bani(100), &p),
            (Bani::ZERO, Bani::ZERO)
        );
        // 3_600 against a limit of 1_200: 2_400 written off, 60% lost.
        assert_eq!(
            write_off(Bani(3_600), Bani(100), &p),
            (Bani(2_400), Bani(1_440))
        );
        // 0.6 · 1 ban rounds to 1.
        assert_eq!(write_off(Bani(1_201), Bani(100), &p), (Bani(1), Bani(1)));
    }

    #[test]
    fn capital_ratio_weights() {
        // 10 over 100 at full weight: 10%.
        assert_eq!(
            capital_ratio_ppm(Bani(10), Bani(60), Bani(40), Bani::ZERO),
            100_000
        );
        // Mortgages at 35%: 35 over 100 of mortgages is 100%.
        assert_eq!(
            capital_ratio_ppm(Bani(35), Bani::ZERO, Bani::ZERO, Bani(100)),
            1_000_000
        );
        assert_eq!(
            capital_ratio_ppm(Bani(-1), Bani(100), Bani::ZERO, Bani::ZERO),
            0
        );
        assert_eq!(
            capital_ratio_ppm(Bani(1), Bani::ZERO, Bani::ZERO, Bani::ZERO),
            u32::MAX
        );
    }
}
