//! Rates between periods (ADR-0017 §3): a rate given a year, converted once
//! to the period of the process that reads it.

use crate::math::powf;

/// The rate per period that compounds to `annual_ppm` (millionths a year)
/// over `periods` periods a year, millionths, rounded half away from zero.
/// A rate of −100% or below a year is taken as just above it.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn annual_to_period_ppm(annual_ppm: i64, periods: u32) -> i64 {
    if periods <= 1 {
        return annual_ppm;
    }
    let growth = (1.0 + annual_ppm as f64 / 1e6).max(1e-9);
    let per = powf(growth, 1.0 / f64::from(periods)) - 1.0;
    let v = per * 1e6;
    (if v < 0.0 { v - 0.5 } else { v + 0.5 }) as i64
}

#[cfg(test)]
mod tests {
    use super::annual_to_period_ppm;

    #[test]
    fn a_monthly_rate_compounds_to_the_annual_one() {
        assert_eq!(annual_to_period_ppm(0, 12), 0);
        // 2% a year: about 0.1652% a month.
        assert_eq!(annual_to_period_ppm(20_000, 12), 1_652);
        assert_eq!(annual_to_period_ppm(-20_000, 12), -1_682);
        assert_eq!(annual_to_period_ppm(50_000, 1), 50_000);
    }
}
