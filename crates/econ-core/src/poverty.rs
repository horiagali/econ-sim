//! Poverty and inequality indicators from incomes (spec
//! `society/poverty-inequality`, feature `poverty`): the measures of a
//! weighted income distribution, the standard-of-living classes, the
//! equivalised size and the recent real income (pure functions), and the
//! month's indicators on the `World`. Outputs only: no behaviour reads them.
//!
//! All in integers, ratios in millionths, sums in i128. API skeleton from the
//! spec's API sketch, written with the acceptance tests before the code
//! (ADR-0014); every body is `unimplemented!` until the implementation fills
//! it.

use econ_types::{Bani, Month};

use crate::world::World;

/// A weighted distribution: each household's equivalised income `x_h`
/// (bani × 1,000) and its persons `v_h = n_h w_h`.
#[derive(Debug, Clone, Copy)]
pub struct Distribution<'a> {
    /// `x_h`, thousandths of a ban.
    pub income_milli: &'a [i128],
    /// `v_h`, persons.
    pub weight: &'a [u64],
}

/// The measures of step 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncomeMeasures {
    /// The weighted median `m`, in the unit of the incomes.
    pub median_milli: i128,
    /// Persons strictly below the line over all persons, millionths.
    pub poverty_rate_ppm: u32,
    /// Gini coefficient, millionths.
    pub gini_ppm: u32,
    /// S80/S20, millionths; `None` when the poorest fifth has no income.
    pub s80_s20_ppm: Option<u32>,
    /// `V`, persons.
    pub population: u64,
}

/// The measures of a distribution with the poverty line at `poverty_line_ppm`
/// of the median (AC-POV-02, 03).
pub fn measures(d: &Distribution, poverty_line_ppm: u32) -> IncomeMeasures {
    let _ = (d, poverty_line_ppm);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The standard-of-living class of an equivalised real income against the
/// anchor and the class lines (millionths of the anchor): 0 at risk, 1
/// lower-middle, 2 middle, 3 affluent.
pub fn class_of(x_milli: i128, anchor_milli: i128, lines_ppm: [u32; 3]) -> u8 {
    let _ = (x_milli, anchor_milli, lines_ppm);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The modified OECD scale (AC-POV-01), thousandths: 1 for the first member,
/// 0.5 for each further member of 14 or more, 0.3 for each under 14.
pub fn eq_adults_milli(member_ages_months: &[u16]) -> u32 {
    let _ = member_ages_months;
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Step 2 (AC-POV-09): `ỹ + λ (Y / P − ỹ)`, whole bani, with the CPI `P` in
/// millionths of day 0's and the speed `λ` in millionths.
pub fn recent_income(prev: Bani, month_income: Bani, cpi_ppm: u32, speed_ppm: u32) -> Bani {
    let _ = (prev, month_income, cpi_ppm, speed_ppm);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// The tuning parameters of the spec's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PovertyParams {
    /// `poverty_line_share`, millionths (fixed: 0.60).
    pub poverty_line_ppm: u32,
    /// `class_lines`, millionths of the anchor (fixed: 0.60, 0.75, 2.00).
    pub class_lines_ppm: [u32; 3],
    /// `income_recent_speed` a month, millionths (1/3).
    pub income_recent_speed_ppm: u32,
}

impl PovertyParams {
    /// The spec's defaults.
    #[must_use]
    pub fn spec_defaults() -> PovertyParams {
        PovertyParams {
            poverty_line_ppm: 600_000,
            class_lines_ppm: [600_000, 750_000, 2_000_000],
            income_recent_speed_ppm: 333_333,
        }
    }
}

/// The indicators of a closed month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PovertyStats {
    /// The month.
    pub month: Month,
    /// The measures on equivalised recent real income, national.
    pub national: IncomeMeasures,
    /// Persons in a household in hardship over all persons, millionths.
    pub hardship_rate_ppm: u32,
    /// Persons in each class over all persons, millionths.
    pub class_share_ppm: [u32; 4],
    /// Persons in each class.
    pub class_persons: [u64; 4],
    /// Persons whose household entered each class since the last close.
    pub class_entered: [u64; 4],
    /// Persons whose household left each class since the last close.
    pub class_left: [u64; 4],
    /// Persons added to each class by a new household or member (births,
    /// arrivals).
    pub class_added: [u64; 4],
    /// Persons removed from each class with a household or member that
    /// disappeared (deaths, departures).
    pub class_removed: [u64; 4],
    /// By county (national lines).
    pub by_county: Vec<IncomeMeasures>,
    /// By development region (national lines).
    pub by_region: Vec<IncomeMeasures>,
    /// By age group: 0–17, 18–64, 65 or more.
    pub by_age: [IncomeMeasures; 3],
    /// The annual indicators, on the calendar year's income: December
    /// closes only.
    pub annual: Option<IncomeMeasures>,
}

impl World {
    /// The indicators of every closed month, in order.
    #[must_use]
    pub fn poverty_stats(&self) -> &[PovertyStats] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// `poverty_anchor`: the median equivalised recent real income of month
    /// 0, December 2021 lei a month.
    pub fn poverty_anchor(&self) -> Bani {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// `hh_real_income_recent`: each household's recent real income, December
    /// 2021 lei a month, aligned with [`crate::world::HouseholdColumns`].
    pub fn hh_real_income_recent(&self) -> &[Bani] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}
