//! Fiscal policy (spec `economy/fiscal-policy`, "First version", roadmap M3):
//! the government's debt book by year of maturity, its coupons, new issues
//! and the yield of new bonds. Pure functions, no I/O; the month-end
//! postings live with the `World`.
//!
//! Money is whole bani; rates and shares are millionths (a year for rates).

use econ_types::Bani;

const ONE: i128 = 1_000_000;

/// The debt's parameters, millionths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebtParams {
    /// `new_issue_years`: maturity of new bonds.
    pub new_issue_years: u8,
    /// `term_premium`: the new issues' yield over the policy rate, a year.
    pub term_premium_ppm: u32,
    /// `risk_premium` risk_0: yield per unit of (debt-to-GDP above the
    /// threshold) squared, a year.
    pub risk_ppm: u32,
    /// `debt_threshold`: debt over GDP where the risk premium starts.
    pub threshold_ppm: u32,
}

impl DebtParams {
    /// The spec's defaults: 7 years, 1.5 points, 20 points, 60% of GDP.
    #[must_use]
    pub fn spec_defaults() -> DebtParams {
        DebtParams {
            new_issue_years: 7,
            term_premium_ppm: 15_000,
            risk_ppm: 200_000,
            threshold_ppm: 600_000,
        }
    }
}

/// `a · b / c` rounded half away from zero; 0 when `c` is 0.
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

fn saturate(v: i128) -> i32 {
    i32::try_from(v.clamp(i128::from(i32::MIN), i128::from(i32::MAX))).expect("clamped")
}

/// The yield of new bonds (step 4, AC-FIS-04): `i + term + risk_0 ·
/// max(0, B/Y − threshold)²`, with Y the last twelve months' GDP. Debt over
/// GDP is taken in whole millionths, rounded down, so the yield never falls as
/// debt rises; with no GDP there is no ratio and no risk premium.
#[must_use]
pub fn bond_yield_ppm(policy_ppm: i32, debt: Bani, gdp_year: Bani, p: &DebtParams) -> i32 {
    let base = i128::from(policy_ppm) + i128::from(p.term_premium_ppm);
    let gdp = i128::from(gdp_year.get());
    if gdp <= 0 {
        return saturate(base);
    }
    let debt_ratio = i128::from(debt.get().max(0)) * ONE / gdp;
    let over = (debt_ratio - i128::from(p.threshold_ppm)).max(0);
    // risk_0 · over², all three in millionths: over 10^12.
    let risk = ratio(i128::from(p.risk_ppm) * over, over, ONE * ONE);
    saturate(base + risk)
}

/// `total` split over `weights` by largest remainder, ties to the lower
/// index; zeros when no weight is positive.
fn largest_remainder(total: i128, weights: &[i128]) -> Vec<i128> {
    let sum: i128 = weights.iter().map(|&w| w.max(0)).sum();
    if sum == 0 {
        return vec![0; weights.len()];
    }
    let (sign, total) = (total.signum(), total.abs());
    let mut parts = Vec::with_capacity(weights.len());
    let mut rems = Vec::with_capacity(weights.len());
    for &w in weights {
        let x = total * w.max(0);
        parts.push(x / sum);
        rems.push(x % sum);
    }
    let left = usize::try_from(total - parts.iter().sum::<i128>()).expect("fewer than the parts");
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(rems[i]), i));
    for &i in order.iter().take(left) {
        parts[i] += 1;
    }
    parts.into_iter().map(|v| v * sign).collect()
}

fn bani(v: i128) -> Bani {
    Bani(i64::try_from(v).expect("an amount that fits i64 bani"))
}

/// Bonds outstanding by year of maturity and the stock's average coupon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebtBook {
    /// Maturing in year k from now (k = 0 is the coming twelve months).
    pub by_year: Vec<Bani>,
    /// The stock's average coupon, millionths a year.
    pub coupon_ppm: u32,
}

impl DebtBook {
    /// The book at the start: `total` spread over years of maturity in the
    /// proportions of the profile's buckets (from year, to year, amount),
    /// each bucket spread evenly over its years.
    ///
    /// A bucket covers the years `from..to` (from inclusive, to exclusive;
    /// "up to one year" is `(0, 1)`, "one to three years" `(1, 3)`); a bucket
    /// with `to <= from` is the single year `from`. Both splits are by
    /// largest remainder, so the book adds up to `total` exactly.
    #[must_use]
    pub fn from_profile(total: Bani, buckets: &[(u8, u8, Bani)], coupon_ppm: u32) -> DebtBook {
        let weights: Vec<i128> = buckets.iter().map(|b| i128::from(b.2.get())).collect();
        let per_bucket = largest_remainder(i128::from(total.get()), &weights);
        let years = buckets
            .iter()
            .map(|&(from, to, _)| usize::from(to.max(from + 1)))
            .max()
            .unwrap_or(1);
        let mut by_year = vec![Bani::ZERO; years];
        for (&(from, to, _), &amount) in buckets.iter().zip(&per_bucket) {
            let span = usize::from(to.max(from + 1) - from);
            for (k, part) in largest_remainder(amount, &vec![1; span])
                .into_iter()
                .enumerate()
            {
                by_year[usize::from(from) + k] += bani(part);
            }
        }
        DebtBook {
            by_year,
            coupon_ppm,
        }
    }

    /// The bonds outstanding.
    pub fn total(&self) -> Bani {
        self.by_year.iter().copied().fold(Bani::ZERO, |a, b| a + b)
    }

    /// A month of coupon on the stock: a twelfth of the coupon, rounded once.
    pub fn month_coupon(&self) -> Bani {
        bani(ratio(
            i128::from(self.total().get()),
            i128::from(self.coupon_ppm),
            12 * ONE,
        ))
    }

    /// What falls due this month, taken off the book. `months_elapsed` is the
    /// number of month ends before this one since the book was made: the
    /// coming year's bonds fall due evenly over its twelve months (the rest of
    /// the year's bucket over the months left), and after the twelfth the
    /// book moves a year closer.
    pub fn mature(&mut self, months_elapsed: u32) -> Bani {
        if self.by_year.is_empty() {
            return Bani::ZERO;
        }
        let left_in_year = 12 - months_elapsed % 12;
        let due = self.by_year[0].get() / i64::from(left_in_year);
        let due = Bani(due);
        self.by_year[0] -= due;
        if left_in_year == 1 {
            let rest = self.by_year.remove(0);
            debug_assert_eq!(rest, Bani::ZERO);
        }
        due
    }

    /// Issue new bonds maturing in `new_issue_years` at the yield (a
    /// negative amount redeems early); the coupon becomes the weighted mean.
    ///
    /// New bonds go to year `new_issue_years − 1` (the seventh year for 7).
    /// An early redemption takes the nearest maturities first and leaves the
    /// coupon as it is; it never takes more than the book holds.
    pub fn issue(&mut self, amount: Bani, yield_ppm: i32, p: &DebtParams) {
        if amount > Bani::ZERO {
            let stock = i128::from(self.total().get());
            let add = i128::from(amount.get());
            let paid = i128::from(self.coupon_ppm) * stock + i128::from(yield_ppm.max(0)) * add;
            self.coupon_ppm = u32::try_from(ratio(paid, 1, stock + add)).expect("a coupon");
            let k = usize::from(p.new_issue_years.max(1) - 1);
            if self.by_year.len() <= k {
                self.by_year.resize(k + 1, Bani::ZERO);
            }
            self.by_year[k] += amount;
        } else {
            let mut left = Bani::ZERO - amount;
            for year in &mut self.by_year {
                let take = left.min(*year);
                *year -= take;
                left -= take;
                if left == Bani::ZERO {
                    break;
                }
            }
        }
    }
}

/// An amount split in proportion to holdings, by largest remainder: the
/// parts add up to the amount exactly. A negative holding counts as none;
/// with no holding at all every part is zero.
#[must_use]
pub fn split_by_holdings(amount: Bani, holdings: &[Bani]) -> Vec<Bani> {
    let weights: Vec<i128> = holdings.iter().map(|h| i128::from(h.get())).collect();
    largest_remainder(i128::from(amount.get()), &weights)
        .into_iter()
        .map(bani)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bn(lei: i64) -> Bani {
        Bani::from_lei(lei * 1_000_000_000)
    }

    #[test]
    fn the_profile_spreads_the_total_exactly() {
        let buckets = [
            (0, 1, bn(29)),
            (1, 3, bn(120)),
            (3, 5, bn(110)),
            (5, 10, bn(177)),
            (10, 15, bn(140)),
        ];
        let book = DebtBook::from_profile(Bani(47_860_000_000_001), &buckets, 45_000);
        assert_eq!(book.by_year.len(), 15);
        assert_eq!(book.total(), Bani(47_860_000_000_001));
        // A bucket spread evenly: its years differ by a ban at most.
        assert!((book.by_year[1] - book.by_year[2]).abs() <= Bani(1));
        assert!(book.by_year[0] < book.by_year[1]);
    }

    #[test]
    fn a_year_falls_due_over_twelve_months() {
        let mut book = DebtBook {
            by_year: vec![Bani(1_200_007), Bani(500)],
            coupon_ppm: 40_000,
        };
        let mut paid = Bani::ZERO;
        for m in 0..12 {
            paid += book.mature(m);
        }
        assert_eq!(paid, Bani(1_200_007));
        assert_eq!(book.by_year, vec![Bani(500)]);
        // A twelfth of 4% on the stock.
        let book = DebtBook {
            by_year: vec![bn(1)],
            coupon_ppm: 45_000,
        };
        assert_eq!(book.month_coupon(), Bani::from_lei(3_750_000));
    }

    #[test]
    fn issues_move_the_coupon_and_redemptions_take_the_nearest() {
        let p = DebtParams::spec_defaults();
        let mut book = DebtBook {
            by_year: vec![bn(1), bn(1)],
            coupon_ppm: 40_000,
        };
        book.issue(bn(2), 60_000, &p);
        assert_eq!(book.coupon_ppm, 50_000);
        assert_eq!(book.by_year.len(), 7);
        assert_eq!(book.by_year[6], bn(2));
        book.issue(Bani::ZERO - bn(1) - Bani(5), 0, &p);
        assert_eq!(book.by_year[0], Bani::ZERO);
        assert_eq!(book.by_year[1], bn(1) - Bani(5));
        assert_eq!(book.coupon_ppm, 50_000);
    }

    #[test]
    fn holders_share_exactly() {
        let parts = split_by_holdings(Bani(10), &[Bani(1), Bani(1), Bani(1), Bani(-4)]);
        assert_eq!(parts, vec![Bani(4), Bani(3), Bani(3), Bani(0)]);
        assert_eq!(
            split_by_holdings(Bani(-10), &[Bani(1), Bani(1), Bani(1)]),
            vec![Bani(-4), Bani(-3), Bani(-3)]
        );
        assert_eq!(split_by_holdings(Bani(7), &[Bani::ZERO]), vec![Bani::ZERO]);
    }
}
