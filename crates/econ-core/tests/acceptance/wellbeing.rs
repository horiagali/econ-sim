//! Acceptance tests for AC-WB-01 to AC-WB-04 (spec: society/social-outcomes,
//! "Wellbeing index", first version of 2026-10-31), written by the test
//! writer from the spec and its API sketch before the code (ADR-0014);
//! behind the feature `wellbeing`, against the test writer's API skeleton
//! `econ_core::wellbeing` (`life_expectancy_milli`, `wellbeing_ppm`,
//! `World::wellbeing_stats`, `WorldParams::wellbeing`).
//!
//! "1:1000" is the Romania scenario at that scale, seed 42, under the law of
//! its folder (`law.json`: the 2021 law). Month 0 is December 2021. Index
//! values are millionths; years and SoL points thousandths.
//!
//! Readings shared by the tests:
//! - Residents are the persons of records of positive weight (with
//!   migration, not `abroad`), weighted by their household's weight, in
//!   private and collective households alike (the spec says "residents").
//! - Ages in whole years are the age in months over 12, rounded down.

use econ_core::wellbeing::{life_expectancy_milli, wellbeing_ppm};
use econ_core::world::World;
use econ_mech_demography::PPB;
use econ_popgen::{EduLevel, Enrolled, Sex};

use super::support::params;
use super::support_m5::{
    residents_mask, run_closes, step_checked, weights_of_persons, world_2021, world_2021_default,
};

/// The four dimension values of a `Wellbeing`, checked.
fn assert_dims(
    got: econ_core::wellbeing::Wellbeing,
    index: u32,
    health: u32,
    education: u32,
    living: u32,
    what: &str,
) {
    assert_eq!(got.health_ppm, health, "{what}: health");
    assert_eq!(got.education_ppm, education, "{what}: education");
    assert_eq!(got.living_ppm, living, "{what}: living standard");
    assert_eq!(got.index_ppm, index, "{what}: index");
}

/// **AC-WB-01** `[unit]` The index is the geometric mean of the three
/// dimension indices, each clamped to [0, 1], in millionths rounded to the
/// nearest; with any dimension at 0 the index is 0; with all at 1 it is 1;
/// the goalposts give 0 and 1 at life expectancy 20 and 85, mean years 0
/// and 15, expected years 0 and 18, SoL 10 and 60.
///
/// Readings: the inputs are thousandths (years, SoL points). The education
/// index is the mean of the two schooling ratios; a case where one ratio is
/// above 1 and the other below is not tested, since the spec clamps "each
/// dimension index" and does not say whether the two ratios are clamped
/// before their mean. A worked case: life expectancy 75.6 (health
/// 55.6 / 65 = 0.855385), mean years 11.3 and expected years 14.2
/// (education (0.753333 + 0.788889) / 2 = 0.771111), SoL 30 (living 0.4):
/// the index is (0.855385 · 0.771111 · 0.4)^(1/3) = 0.641376, to within one
/// millionth (the spec does not say whether the mean takes the rounded
/// dimensions or the exact ones).
#[test]
fn ac_wb_01() {
    // All at the upper goalposts, and beyond: 1.
    assert_dims(
        wellbeing_ppm(85_000, 15_000, 18_000, 60_000),
        1_000_000,
        1_000_000,
        1_000_000,
        1_000_000,
        "the upper goalposts",
    );
    assert_dims(
        wellbeing_ppm(95_000, 20_000, 22_000, 90_000),
        1_000_000,
        1_000_000,
        1_000_000,
        1_000_000,
        "beyond the upper goalposts",
    );
    // The lower goalposts, one at a time: that dimension is 0, so the index.
    let w = wellbeing_ppm(20_000, 15_000, 18_000, 60_000);
    assert_eq!((w.health_ppm, w.index_ppm), (0, 0), "life expectancy 20");
    let w = wellbeing_ppm(15_000, 15_000, 18_000, 60_000);
    assert_eq!((w.health_ppm, w.index_ppm), (0, 0), "life expectancy 15");
    let w = wellbeing_ppm(85_000, 0, 0, 60_000);
    assert_eq!((w.education_ppm, w.index_ppm), (0, 0), "no schooling");
    let w = wellbeing_ppm(85_000, 15_000, 18_000, 10_000);
    assert_eq!((w.living_ppm, w.index_ppm), (0, 0), "SoL 10");
    let w = wellbeing_ppm(85_000, 15_000, 18_000, 4_000);
    assert_eq!((w.living_ppm, w.index_ppm), (0, 0), "SoL below 10");
    let w = wellbeing_ppm(20_000, 0, 0, 10_000);
    assert_eq!(w.index_ppm, 0, "everything at 0");
    // Midpoints: every dimension 0.5, so the index.
    assert_dims(
        wellbeing_ppm(52_500, 7_500, 9_000, 35_000),
        500_000,
        500_000,
        500_000,
        500_000,
        "the midpoints",
    );
    // The two schooling ratios are averaged: 15 years and none expected.
    let w = wellbeing_ppm(85_000, 15_000, 0, 60_000);
    assert_eq!(w.education_ppm, 500_000, "mean years at 1, expected at 0");
    let w = wellbeing_ppm(85_000, 0, 18_000, 60_000);
    assert_eq!(w.education_ppm, 500_000, "mean years at 0, expected at 1");
    // Geometric, not arithmetic: 1, 1 and 0.125 give 0.5.
    let w = wellbeing_ppm(85_000, 15_000, 18_000, 16_250);
    assert_eq!(w.living_ppm, 125_000);
    assert_eq!(w.index_ppm, 500_000, "the cube root of 0.125");
    // The worked case.
    let w = wellbeing_ppm(75_600, 11_300, 14_200, 30_000);
    assert_eq!(w.health_ppm, 855_385, "55.6 / 65");
    assert_eq!(w.education_ppm, 771_111, "(11.3 / 15 + 14.2 / 18) / 2");
    assert_eq!(w.living_ppm, 400_000, "20 / 50");
    assert!(
        w.index_ppm.abs_diff(641_376) <= 1,
        "the geometric mean {} against 641,376",
        w.index_ppm
    );
}

/// A table of yearly probabilities by sex and age 0 to 100 (100 the last
/// age: everyone alive at 100 dies in its year).
fn table(women: impl Fn(usize) -> u32, men: impl Fn(usize) -> u32) -> [[u32; 101]; 2] {
    [std::array::from_fn(women), std::array::from_fn(men)]
}

/// The life table's closed form for a constant yearly probability `q` below
/// 100 (all die at 100), deaths at mid-year: `Σ_{a<100} (l_a + l_{a+1}) / 2 +
/// l_100 / 2` with `l_a = (1 − q)^a`, i.e. `(1 − p^100) / q − 1/2 + p^100`,
/// in years.
fn closed_form(q: f64) -> f64 {
    let p = 1.0 - q;
    let mut p100 = 1.0;
    for _ in 0..100 {
        p100 *= p;
    }
    (1.0 - p100) / q - 0.5 + p100
}

/// Thousandths of a year to the nearest, from years.
fn milli(years: f64) -> u32 {
    let x = (years * 1_000.0).round();
    assert!((0.0..4.0e9).contains(&x));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let v = x as u32;
    v
}

/// **AC-WB-02** `[unit]` Life expectancy from a mortality table: a table in
/// which nobody dies before 100 and everyone dies at 100 gives 100.5 years
/// (deaths at mid-year); a constant yearly probability q below 100 gives the
/// life table's closed form; from the 2019 table of the Romania scenario
/// (pre-pandemic), 74.5 to 76.5 years (2019: 75.6).
///
/// Readings: tables are yearly probabilities in parts per billion, ages 0 to
/// 100, 100 the last age (the test gives it probability one); "the life
/// table's closed form" is `(1 − p^100) / q − 1/2 + p^100` with `p = 1 − q`
/// (`closed_form`), to within two thousandths of a year (the function works
/// in integers or floats; the spec fixes no rounding inside it). "Both sexes
/// weighted by the residents' sex shares" is the mean of the two sexes' life
/// expectancies weighted by the share of men (`men_share_ppm`), to within
/// one thousandth. "The 2019 table of the Romania scenario" is the
/// scenario's pre-pandemic table (`demography.json`, the 2017–2019 average
/// the scenario carries, read from `World::people().demography`), ages 0 to
/// 100 of its 0 to 110, with the share of men among the residents at load at
/// 1:1000.
#[test]
fn ac_wb_02() {
    // Nobody dies before 100; everyone at 100.
    let all_at_100 = table(
        |a| if a == 100 { PPB } else { 0 },
        |a| if a == 100 { PPB } else { 0 },
    );
    for share in [0, 480_000, 1_000_000] {
        assert_eq!(
            life_expectancy_milli(&all_at_100, share),
            100_500,
            "nobody dies before 100 (men {share})"
        );
    }
    // Constant probabilities below 100.
    for q_ppb in [1_000_000u32, 10_000_000, 50_000_000, 200_000_000] {
        let t = table(
            |a| if a == 100 { PPB } else { q_ppb },
            |a| if a == 100 { PPB } else { q_ppb },
        );
        let want = milli(closed_form(f64::from(q_ppb) / 1e9));
        let got = life_expectancy_milli(&t, 500_000);
        assert!(
            got.abs_diff(want) <= 2,
            "q = {q_ppb} ppb: {got} against the closed form {want}"
        );
    }
    // Two sexes, weighted by the share of men.
    let t = table(
        |a| if a == 100 { PPB } else { 10_000_000 },
        |a| if a == 100 { PPB } else { 50_000_000 },
    );
    let women = life_expectancy_milli(&t, 0);
    let men = life_expectancy_milli(&t, 1_000_000);
    assert!(
        women.abs_diff(milli(closed_form(0.01))) <= 2,
        "women {women}"
    );
    assert!(men.abs_diff(milli(closed_form(0.05))) <= 2, "men {men}");
    for share in [250_000u32, 500_000, 480_000] {
        let want = (u64::from(women) * u64::from(1_000_000 - share)
            + u64::from(men) * u64::from(share)
            + 500_000)
            / 1_000_000;
        let got = u64::from(life_expectancy_milli(&t, share));
        assert!(
            got.abs_diff(want) <= 1,
            "men {share}: {got} against the weighted mean {want}"
        );
    }

    // The scenario's table.
    let w = world_2021_default(1000);
    let full = w
        .people()
        .demography
        .as_ref()
        .expect("the scenario has demographic tables")
        .mortality
        .yearly;
    let mut t = [[0u32; 101]; 2];
    for s in 0..2 {
        t[s].copy_from_slice(&full[s][..101]);
    }
    let weights = weights_of_persons(&w);
    let res = residents_mask(&w);
    let (mut all, mut men) = (0u64, 0u64);
    for p in 0..res.len() {
        if res[p] {
            all += u64::from(weights[p]);
            if w.people().sex[p] == Sex::M {
                men += u64::from(weights[p]);
            }
        }
    }
    let share = u32::try_from((men * 1_000_000 + all / 2) / all).expect("a share");
    let e0 = life_expectancy_milli(&t, share);
    assert!(
        (74_500..=76_500).contains(&e0),
        "life expectancy of the scenario's table {e0} (men {share})"
    );
}

/// Years of schooling of an `edu_level`, thousandths (the spec's
/// `schooling_years`).
fn schooling_milli(e: EduLevel) -> u64 {
    match e {
        EduLevel::Isced0To2 => 8_000,
        EduLevel::Isced3 => 12_000,
        EduLevel::Isced4 => 13_000,
        EduLevel::Isced5To6 => 15_500,
        EduLevel::Isced7To8 => 17_500,
    }
}

/// The nation's mean years of schooling of residents 25 and over and its
/// expected years of schooling (ages 5 to 29), thousandths, from the
/// persons table.
fn schooling_from_persons(w: &World) -> (u64, u64) {
    let weights = weights_of_persons(w);
    let res = residents_mask(w);
    let ages = w.person_age_months();
    let edu = &w.persons().edu_level;
    let enrolled = &w.people().enrolled;
    let (mut years, mut adults) = (0u64, 0u64);
    let mut by_age = [(0u64, 0u64); 30];
    for p in 0..res.len() {
        if !res[p] {
            continue;
        }
        let wt = u64::from(weights[p]);
        let a = usize::from(ages[p] / 12);
        if a >= 25 {
            years += wt * schooling_milli(edu[p]);
            adults += wt;
        }
        if (5..=29).contains(&a) {
            by_age[a].1 += wt;
            if enrolled[p] != Enrolled::None {
                by_age[a].0 += wt;
            }
        }
    }
    let mys = (years + adults / 2) / adults;
    let eys: f64 = (5..=29)
        .filter(|&a| by_age[a].1 > 0)
        .map(|a| {
            #[allow(clippy::cast_precision_loss)]
            let s = by_age[a].0 as f64 / by_age[a].1 as f64;
            s
        })
        .sum();
    (mys, u64::from(milli(eys)))
}

/// **AC-WB-03** `[sim]` At 1:1000, at the close of month 0: mean years of
/// schooling of residents 25 and over between 10.0 and 12.5 (UNDP 2021:
/// 11.3), expected years of schooling between 12.5 and 16.0 (14.2), and
/// every region's index between 0 and 1 with the national index between the
/// regions' lowest and highest.
///
/// Readings: the bands are the nation's (the last element of
/// `mean_years_milli` and `expected_years_milli`), inclusive. They are also
/// computed from the persons table (residents, `edu_level` and `enrolled`
/// at the close; the spec's `schooling_years`): the mean years to within one
/// thousandth, the expected years to within 13 thousandths (the sum of 25
/// shares, each of which the code may round to a thousandth). "Every
/// region" is the eight development regions (`by_region`, with
/// `mean_years_milli` and `expected_years_milli` one longer, the nation
/// last); "between" is inclusive.
#[test]
fn ac_wb_03() {
    let mut w = world_2021_default(1000);
    run_closes(&mut w, 1);
    let s = &w.wellbeing_stats()[0];
    assert_eq!(s.month.0, 0);
    let regions = s.by_region.len();
    assert_eq!(regions, 8, "Romania's eight development regions");
    assert_eq!(
        s.mean_years_milli.len(),
        regions + 1,
        "by region, the nation last"
    );
    assert_eq!(
        s.expected_years_milli.len(),
        regions + 1,
        "by region, the nation last"
    );
    let mys = s.mean_years_milli[regions];
    let eys = s.expected_years_milli[regions];
    assert!(
        (10_000..=12_500).contains(&mys),
        "mean years of schooling {mys}"
    );
    assert!(
        (12_500..=16_000).contains(&eys),
        "expected years of schooling {eys}"
    );
    let (mys_p, eys_p) = schooling_from_persons(&w);
    assert!(
        u64::from(mys).abs_diff(mys_p) <= 1,
        "mean years {mys} against {mys_p} from the persons table"
    );
    assert!(
        u64::from(eys).abs_diff(eys_p) <= 13,
        "expected years {eys} against {eys_p} from the persons table"
    );
    for (r, x) in s.by_region.iter().enumerate() {
        assert!(
            x.index_ppm <= 1_000_000,
            "region {r}: index {}",
            x.index_ppm
        );
    }
    let lo = s
        .by_region
        .iter()
        .map(|x| x.index_ppm)
        .min()
        .expect("regions");
    let hi = s
        .by_region
        .iter()
        .map(|x| x.index_ppm)
        .max()
        .expect("regions");
    assert!(
        (lo..=hi).contains(&s.national.index_ppm),
        "the national index {} outside the regions' {lo} to {hi}",
        s.national.index_ppm
    );
}

/// **AC-WB-04** `[sim]` Isolation: at 1:1000, the state hash after 24 months
/// is the same with the index computed and not computed (it is read by
/// migration only through step 5b, behind its own feature).
///
/// Readings: "not computed" is `WorldParams::wellbeing` `None`. Step 5b's
/// own feature is `home_quality`; with it compiled in, step 5b's three
/// elasticities are 0 in both runs, so that its multipliers are 1 whatever
/// the index. The test compares the state hash of every day, which includes
/// the one after 24 months; the run with the index has 24 months of it, the
/// other none.
#[test]
fn ac_wb_04() {
    let on = params();
    #[cfg(feature = "home_quality")]
    let on = {
        let mut on = on;
        on.migration.quality_return_elasticity_ppm = 0;
        on.migration.quality_emigration_elasticity_ppm = 0;
        on.migration.quality_immigration_elasticity_ppm = 0;
        on
    };
    let mut off = on;
    off.wellbeing = None;
    assert!(on.wellbeing.is_some());
    let mut a = world_2021(1000, on, Vec::new());
    let mut b = world_2021(1000, off, Vec::new());
    while a.months().len() < 24 {
        let ra = step_checked(&mut a);
        let rb = step_checked(&mut b);
        assert_eq!(
            ra.state_hash, rb.state_hash,
            "{:?}: the state hashes differ",
            ra.date
        );
    }
    assert_eq!(b.months().len(), 24);
    assert_eq!(a.state_hash(), b.state_hash(), "after 24 months");
    assert_eq!(a.wellbeing_stats().len(), 24, "the index was computed");
    assert!(b.wellbeing_stats().is_empty(), "the index was not computed");
}
