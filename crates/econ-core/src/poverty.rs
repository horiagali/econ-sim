//! Poverty and inequality indicators from incomes (spec
//! `society/poverty-inequality`, feature `poverty`): the measures of a
//! weighted income distribution, the standard-of-living classes, the
//! equivalised size and the recent real income (pure functions), and the
//! month's indicators on the `World`. Outputs only: no behaviour reads them.
//!
//! All in integers, ratios in millionths, sums in i128. API skeleton from the
//! spec's API sketch, written with the acceptance tests before the code
//! (ADR-0014); the `World` side is in `world::poverty_state`.

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
    let n = d.income_milli.len();
    let mut order: Vec<usize> = (0..n).filter(|&h| d.weight[h] > 0).collect();
    order.sort_by_key(|&h| (d.income_milli[h], h));
    let v_all: i128 = order.iter().map(|&h| i128::from(d.weight[h])).sum();
    let population = u64::try_from(v_all).unwrap_or(u64::MAX);
    if v_all == 0 {
        return IncomeMeasures {
            median_milli: 0,
            poverty_rate_ppm: 0,
            gini_ppm: 0,
            s80_s20_ppm: None,
            population: 0,
        };
    }
    let x = |h: usize| d.income_milli[h].max(0);
    // The median: the smallest x with V_h ≥ V/2.
    let mut cum = 0i128;
    let mut median = 0i128;
    for &h in &order {
        cum += i128::from(d.weight[h]);
        if 2 * cum >= v_all {
            median = d.income_milli[h];
            break;
        }
    }
    let line = i128::from(poverty_line_ppm);
    let poor: i128 = order
        .iter()
        .filter(|&&h| d.income_milli[h] * 1_000_000 < line * median)
        .map(|&h| i128::from(d.weight[h]))
        .sum();
    // Gini: Σ v x (2V_h − v) / (V X) − 1.
    let total_x: i128 = order.iter().map(|&h| i128::from(d.weight[h]) * x(h)).sum();
    let gini = if total_x > 0 {
        let mut cum = 0i128;
        let mut num = 0i128;
        for &h in &order {
            let v = i128::from(d.weight[h]);
            cum += v;
            num += v * x(h) * (2 * cum - v);
        }
        let g = num * 1_000_000 / (v_all * total_x) - 1_000_000;
        u32::try_from(g.max(0)).unwrap_or(0)
    } else {
        0
    };
    // S80/S20: the income of the richest V/5 persons over the poorest V/5,
    // splitting the households on the boundaries (persons counted × 5).
    let fifth_income = |iter: &mut dyn Iterator<Item = usize>| -> i128 {
        let mut left = v_all; // V/5 persons, × 5
        let mut sum = 0i128;
        for h in iter {
            if left <= 0 {
                break;
            }
            let v5 = 5 * i128::from(d.weight[h]);
            let take = v5.min(left);
            sum += take * x(h);
            left -= take;
        }
        sum
    };
    let bottom = fifth_income(&mut order.iter().copied());
    let top = fifth_income(&mut order.iter().rev().copied());
    let s80_s20 = (bottom > 0).then(|| u32::try_from(top * 1_000_000 / bottom).unwrap_or(u32::MAX));
    IncomeMeasures {
        median_milli: median,
        poverty_rate_ppm: u32::try_from(poor * 1_000_000 / v_all).expect("a share"),
        gini_ppm: gini,
        s80_s20_ppm: s80_s20,
        population,
    }
}

/// The standard-of-living class of an equivalised real income against the
/// anchor and the class lines (millionths of the anchor): 0 at risk, 1
/// lower-middle, 2 middle, 3 affluent.
pub fn class_of(x_milli: i128, anchor_milli: i128, lines_ppm: [u32; 3]) -> u8 {
    let x = x_milli * 1_000_000;
    let line = |k: usize| i128::from(lines_ppm[k]) * anchor_milli;
    if x < line(0) {
        0
    } else if x < line(1) {
        1
    } else if x <= line(2) {
        2
    } else {
        3
    }
}

/// The modified OECD scale (AC-POV-01), thousandths: 1 for the first member,
/// 0.5 for each further member of 14 or more, 0.3 for each under 14.
pub fn eq_adults_milli(member_ages_months: &[u16]) -> u32 {
    let n = u32::try_from(member_ages_months.len()).expect("members");
    if n == 0 {
        return 0;
    }
    let older = u32::try_from(member_ages_months.iter().filter(|&&a| a >= 14 * 12).count())
        .expect("members");
    let younger = n - older;
    if older > 0 {
        1_000 + 500 * (older - 1) + 300 * younger
    } else {
        1_000 + 300 * (younger - 1)
    }
}

/// Step 2 (AC-POV-09): `ỹ + λ (Y / P − ỹ)`, whole bani, with the CPI `P` in
/// millionths of day 0's and the speed `λ` in millionths.
pub fn recent_income(prev: Bani, month_income: Bani, cpi_ppm: u32, speed_ppm: u32) -> Bani {
    // ỹ + λ (Y / P − ỹ), exactly, rounded half away from zero once.
    let p = i128::from(cpi_ppm.max(1));
    let (prev, y, l) = (
        i128::from(prev.get()),
        i128::from(month_income.get()),
        i128::from(speed_ppm),
    );
    let num = prev * 1_000_000 * p + l * (y * 1_000_000 - prev * p);
    let den = 1_000_000 * p;
    let v = if num >= 0 {
        (2 * num + den) / (2 * den)
    } else {
        -((-2 * num + den) / (2 * den))
    };
    Bani(i64::try_from(v).expect("an amount"))
}

/// Step 9 (AC-POV-10): the standard of living of a household, thousandths of
/// a point, `clamp(10 + 10 log2(c / s), 0, 100)` rounded to the nearest, from
/// its real equivalised consumption `c` (bani a month per adult, day 0's
/// prices) and `sol_subsistence` `s`.
///
/// Test writer's addition (ADR-0014): the implementation fills it.
#[must_use]
pub fn sol_milli(consumption_real_eq: Bani, subsistence: Bani) -> u32 {
    let _ = (consumption_real_eq, subsistence);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Step 9 (AC-POV-10): a month's public health (or education) spending
/// shared out over household records by `record weight × person weight`
/// (the person weight being the sum of the age or level weights of its
/// members). Each element is the record's whole amount (its weight
/// included), whole bani; the elements sum to `total` exactly.
///
/// Test writer's addition (ADR-0014): the implementation fills it.
#[must_use]
pub fn share_by_weight(total: Bani, weights: &[(u32, u32)]) -> Vec<Bani> {
    let _ = (total, weights);
    unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
}

/// Step 9's outputs of a month: weighted means of SoL over persons,
/// thousandths of a point.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SolStats {
    /// The national mean.
    pub national_milli: u32,
    /// By county.
    pub by_county: Vec<u32>,
    /// By development region.
    pub by_region: Vec<u32>,
    /// By standard-of-living class (step 6): at risk, lower-middle, middle,
    /// affluent.
    pub by_class: [u32; 4],
    /// By income fifth of the households (`hh_fifth`), poorest first.
    pub by_fifth: [u32; 5],
    /// By age group: 0–17, 18–64, 65 or more.
    pub by_age: [u32; 3],
    /// The national mean's change since the last close by part: purchases,
    /// home-grown food, imputed rent, health, education; they sum to the
    /// change.
    pub parts_change_milli: [i64; 5],
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
    /// `sol_subsistence`: consumption at SoL 10, per adult a month at day 0's
    /// prices (700 lei). Test writer's addition (ADR-0014).
    pub sol_subsistence: Bani,
    /// `sol_health_age_weights`, millionths: under 5, 5–64, 65 and over.
    /// Test writer's addition (ADR-0014).
    pub sol_health_age_weights_ppm: [u32; 3],
    /// `sol_education_level_weights`, millionths: pre-school, primary, lower
    /// secondary, upper secondary, post-secondary, tertiary, master or
    /// doctorate. Test writer's addition (ADR-0014).
    pub sol_education_level_weights_ppm: [u32; 7],
}

impl PovertyParams {
    /// The spec's defaults.
    #[must_use]
    pub fn spec_defaults() -> PovertyParams {
        PovertyParams {
            poverty_line_ppm: 600_000,
            class_lines_ppm: [600_000, 750_000, 2_000_000],
            income_recent_speed_ppm: 333_333,
            sol_subsistence: Bani::from_lei(700),
            sol_health_age_weights_ppm: [1_500_000, 1_000_000, 3_000_000],
            sol_education_level_weights_ppm: [
                1_000_000, 1_000_000, 1_200_000, 1_300_000, 1_300_000, 2_000_000, 2_500_000,
            ],
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
    /// The standard of living (step 9). Test writer's addition (ADR-0014):
    /// left at its default until the implementation computes it.
    pub sol: SolStats,
}

impl World {
    /// The indicators of every closed month, in order.
    #[must_use]
    pub fn poverty_stats(&self) -> &[PovertyStats] {
        self.pov().stats.as_slice()
    }

    /// `poverty_anchor`: the median equivalised recent real income of month
    /// 0, December 2021 lei a month.
    pub fn poverty_anchor(&self) -> Bani {
        self.pov().anchor
    }

    /// `hh_real_income_recent`: each household's recent real income, December
    /// 2021 lei a month, aligned with [`crate::world::HouseholdColumns`].
    pub fn hh_real_income_recent(&self) -> &[Bani] {
        self.pov().recent.as_slice()
    }

    /// `hh_income_year`: each household's income received in the calendar
    /// year so far (zero after a December close), aligned with
    /// [`crate::world::HouseholdColumns`].
    pub fn hh_income_year(&self) -> &[Bani] {
        self.pov().year.as_slice()
    }

    /// `hh_disposable_income` as the indicators take it: each household's
    /// income `Y_h` of the latest closed month (step 1; month 0's from after
    /// November's pay on day 0, imputed rent left out, as restated on
    /// 2026-10-31), aligned with [`crate::world::HouseholdColumns`].
    ///
    /// Test writer's addition (ADR-0014): the implementation fills it.
    pub fn hh_income_month(&self) -> &[Bani] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }

    /// Each household's standard of living (step 9) at the latest close,
    /// thousandths of a point, aligned with
    /// [`crate::world::HouseholdColumns`].
    ///
    /// Test writer's addition (ADR-0014): the implementation fills it.
    #[must_use]
    pub fn hh_sol_milli(&self) -> &[u32] {
        unimplemented!("test writer's addition (ADR-0014): the implementation fills it")
    }
}
