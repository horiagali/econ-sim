//! Acceptance tests for the `[unit]` criteria of economy/demographics:
//! AC-DEM-01 (the mortality table), 02 (alignment), 10 (a new pension) and
//! 17 (fertility's multipliers). Written in a test-authoring session from the
//! spec and its API sketch, before the code existed (ADR-0014).
//!
//! Probabilities are parts per billion, multipliers and shares millionths,
//! pension points millionths of a point, alignment targets millionths of a
//! person.

use econ_mech_demography::{
    Candidate, ChildSupport, FertileWoman, FertilityResponse, MORTALITY_AGES, MortalityTable, PPB,
    PensionLaw, align, benefit_gap, job_normalisation, m_cyc, m_job, m_pol, monthly_from_yearly,
    new_pension_points, table_for_year,
};
use econ_mech_transfers::{PensionRule, pension};
use econ_num::math;
use econ_rng::{KeyedRng, Stream};
use econ_types::{Bani, Day};

/// The oldest age of the mortality table, years.
const MAX_AGE: usize = MORTALITY_AGES - 1;

fn lei(x: i64) -> Bani {
    Bani::from_lei(x)
}

/// `n / d` rounded half away from zero, for non-negative `n` and positive `d`.
fn round_half_away(n: u128, d: u128) -> u128 {
    (2 * n + d) / (2 * d)
}

/// The pipeline's rule in floating point: `1 − (1 − q)^(1/12)`, nearest ppb.
fn reference_monthly(q_ppb: u32) -> u32 {
    let q = f64::from(q_ppb) / 1e9;
    let m = 1.0 - math::powf(1.0 - q, 1.0 / 12.0);
    let scaled = m * 1e9;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let v = (scaled + 0.5) as u32;
    v
}

/// A base table (2018) whose values differ by sex and age, one at 110 (its
/// yearly column, which `table_for_year` does not read, is a copy).
fn base_table() -> MortalityTable {
    let mut monthly = [[0u32; MAX_AGE + 1]; 2];
    for (s, row) in monthly.iter_mut().enumerate() {
        for (a, v) in row.iter_mut().enumerate() {
            let a = u32::try_from(a).expect("an age");
            let s = u32::try_from(s).expect("a sex");
            // From about 300 ppb a month in childhood to 30 million at 100.
            *v = 100 + 37 * a + (1 + s) * a * a * a * 29;
        }
        row[MAX_AGE] = PPB;
    }
    // A tie for the half-away rule: 100 × 0.985 = 98.5 → 99.
    monthly[0][10] = 100;
    MortalityTable {
        base_year: 2018,
        yearly: monthly,
        monthly,
    }
}

/// **AC-DEM-01** `[unit]` The mortality table: a yearly probability turns into
/// the monthly one of the pipeline's rule; the table of year $y$ is the base ×
/// (1 − improvement)^(y − 2018), rounded half away from zero, the same on
/// every platform; at 110 the monthly probability is one.
///
/// Readings:
/// - "The pipeline's rule" is $m = 1 - (1 - q)^{1/12}$ (spec, step 3),
///   stored as the nearest part per billion; the test compares with the rule
///   computed in floating point (`econ_num::math`), which is accurate far
///   below half a ppb, so the two agree exactly.
/// - The spec computes the table "by repeated integer multiplication, half
///   away from zero": a year after the base it is exactly round(base ×
///   (1 − improvement)); after $n$ years a rounding at each year may differ
///   from one rounding of the exact power by at most $n/2$ ppb, which the
///   test allows (both readings of "repeated" pass). Where the power is
///   exact (an improvement of 50% on powers of two, or no improvement) the
///   table must be exact: these known answers are the "same on every
///   platform" check, the arithmetic being integer.
#[test]
fn ac_dem_01() {
    // The monthly probability of a yearly one.
    for q in [
        0u32,
        1,
        10,
        1_000,
        123_456,
        1_000_000,
        5_000_000,
        10_000_000,
        50_000_000,
        100_000_000,
        300_000_000,
        500_000_000,
        900_000_000,
        999_999_999,
        PPB,
    ] {
        assert_eq!(
            monthly_from_yearly(q),
            reference_monthly(q),
            "the monthly probability of a yearly {q} ppb"
        );
    }
    assert_eq!(monthly_from_yearly(PPB), PPB, "certain death stays certain");

    let base = base_table();
    let imp = 15_000; // 1.5% a year, the default
    // The base year is the base.
    let t2018 = table_for_year(&base, 2018, imp);
    assert_eq!(t2018.base_year, 2018);
    assert_eq!(t2018.monthly, base.monthly, "the table of 2018 is the base");
    // One year on: exactly round(base × 0.985), half away from zero.
    let t2019 = table_for_year(&base, 2019, imp);
    assert_eq!(t2019.base_year, 2019);
    for s in 0..2 {
        for a in 0..MAX_AGE {
            let b = u128::from(base.monthly[s][a]);
            let want = u32::try_from(round_half_away(b * 985_000, 1_000_000)).expect("fits");
            assert_eq!(t2019.monthly[s][a], want, "2019, sex {s}, age {a}");
        }
    }
    assert_eq!(t2019.monthly[0][10], 99, "98.5 rounds away from zero");
    // Later years: the base × 0.985^n, within n/2 ppb of rounding.
    for n in 2..=60u16 {
        let t = table_for_year(&base, 2018 + n, imp);
        assert_eq!(t.base_year, 2018 + n);
        let factor = math::powi(0.985, i32::from(n));
        let tol = f64::from(n) / 2.0;
        for s in 0..2 {
            for a in 0..MAX_AGE {
                let exact = f64::from(base.monthly[s][a]) * factor;
                let got = f64::from(t.monthly[s][a]);
                assert!(
                    (got - exact).abs() <= tol,
                    "year {}, sex {s}, age {a}: {got} against {exact:.3}",
                    2018 + n
                );
            }
            assert_eq!(t.monthly[s][MAX_AGE], PPB, "at 110, year {}", 2018 + n);
        }
    }
    // Known answers, exact under any reading.
    let mut pow2 = base.clone();
    for row in &mut pow2.monthly {
        for v in row.iter_mut().take(MAX_AGE) {
            *v = 1_024;
        }
    }
    for n in 0..=10u16 {
        let t = table_for_year(&pow2, 2018 + n, 500_000);
        for s in 0..2 {
            for a in 0..MAX_AGE {
                assert_eq!(t.monthly[s][a], 1_024 >> n, "halved {n} times");
            }
            assert_eq!(t.monthly[s][MAX_AGE], PPB);
        }
    }
    let flat = table_for_year(&base, 2070, 0);
    assert_eq!(flat.monthly, base.monthly, "no improvement, no change");
    // At 110 the probability is one whatever the base says.
    let mut low = base.clone();
    low.monthly[0][MAX_AGE] = 500_000_000;
    low.monthly[1][MAX_AGE] = 0;
    for y in [2018u16, 2022, 2050] {
        let t = table_for_year(&low, y, imp);
        assert_eq!(t.monthly[0][MAX_AGE], PPB, "women at 110 in {y}");
        assert_eq!(t.monthly[1][MAX_AGE], PPB, "men at 110 in {y}");
    }
    // A pure function of its inputs.
    assert_eq!(
        table_for_year(&base, 2045, imp),
        table_for_year(&base, 2045, imp)
    );
}

/// A stratum of `n` records with weights from 1 to 5,000 and monthly
/// probabilities up to 3%, whole ppm (so that Σ w·p is a whole number of
/// millionths of a person under any order of summation).
fn stratum(n: u64) -> Vec<Candidate> {
    let rng = KeyedRng::new(7);
    (0..n)
        .map(|i| {
            let mut d = rng.draw(Stream::Test, 0, i);
            let weight = u32::try_from(1 + d.below(5_000)).expect("fits");
            let p = u32::try_from(1_000 * (1 + d.below(30_000))).expect("fits");
            Candidate {
                person_key: 10_000 + 3 * i,
                weight,
                p,
                rank_p: p,
            }
        })
        .collect()
}

/// Σ w·p of a stratum, millionths of a person.
fn expected_micro(c: &[Candidate]) -> i64 {
    let ppb: u128 = c
        .iter()
        .map(|x| u128::from(x.weight) * u128::from(x.p))
        .sum();
    assert_eq!(ppb % 1_000, 0, "whole millionths");
    i64::try_from(ppb / 1_000).expect("fits")
}

/// The weight of the chosen keys, persons.
fn chosen_weight(c: &[Candidate], chosen: &[u64]) -> i64 {
    chosen
        .iter()
        .map(|k| {
            let x = c
                .iter()
                .find(|x| x.person_key == *k)
                .unwrap_or_else(|| panic!("chosen key {k} is not a candidate"));
            i64::from(x.weight)
        })
        .sum()
}

/// **AC-DEM-02** `[unit]` Alignment: for a stratum of records with weights
/// between 1 and 5,000, the weight selected each month is within half the
/// largest weight of target + carry; over 120 months the cumulative selected
/// weight is within the largest weight of the cumulative expected; the result
/// does not depend on the order of the input; a person with twice the
/// probability is selected 1.8 to 2.2 times as often over 10,000 trials.
///
/// Readings:
/// - "Target + carry" is $T_s = \sum w_i p_i + \text{carry}_s$ in millionths
///   of a person; the carry returned is $T_s - S$ exactly; no key is chosen
///   twice and every chosen key is a candidate.
/// - "Does not depend on the order": the same set of keys and the same carry
///   for the input reversed and rotated, every month.
/// - "A person with twice the probability": ten persons with $p$ and ten with
///   $2p$, all of weight 1,000, aligned for 10,000 months with the carry
///   kept; the mean count of selections of the second ten over that of the
///   first ten is 1.8 to 2.2 (per-person counts of a few hundred would be
///   too noisy for the band).
#[test]
fn ac_dem_02() {
    let rng = KeyedRng::new(42);
    let purpose = 3;
    let c = stratum(300);
    let largest = i64::from(c.iter().map(|x| x.weight).max().expect("a stratum"));
    assert!(largest > 4_000, "the stratum spans weights up to 5,000");
    let e = expected_micro(&c);
    let mut reversed = c.clone();
    reversed.reverse();
    let mut rotated = c.clone();
    rotated.rotate_left(117);
    let (mut carry, mut cum_selected, mut cum_expected) = (0i64, 0i64, 0i64);
    for month in 0..120u32 {
        let day = Day(30 * month + 29);
        let s = align(&c, carry, &rng, day, purpose);
        let mut keys = s.chosen.clone();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(
            keys.len(),
            s.chosen.len(),
            "month {month}: a key chosen twice"
        );
        let selected = chosen_weight(&c, &s.chosen) * 1_000_000;
        let target = e + carry;
        assert!(
            (selected - target).abs() <= largest * 1_000_000 / 2,
            "month {month}: selected {selected} against target {target}"
        );
        assert_eq!(s.carry_micro, target - selected, "month {month}: the carry");
        for (name, input) in [("reversed", &reversed), ("rotated", &rotated)] {
            let o = align(input, carry, &rng, day, purpose);
            let mut ok = o.chosen.clone();
            ok.sort_unstable();
            assert_eq!(ok, keys, "month {month}: {name} input, other persons");
            assert_eq!(o.carry_micro, s.carry_micro, "month {month}: {name} input");
        }
        cum_selected += selected;
        cum_expected += e;
        assert!(
            (cum_selected - cum_expected).abs() <= largest * 1_000_000,
            "after {} months: {cum_selected} against {cum_expected}",
            month + 1
        );
        carry = s.carry_micro;
    }

    // Twice the probability, twice as often.
    let pair: Vec<Candidate> = (0..20u64)
        .map(|i| {
            let p = if i < 10 { 20_000_000 } else { 40_000_000 };
            Candidate {
                person_key: 500 + i,
                weight: 1_000,
                p,
                rank_p: p,
            }
        })
        .collect();
    let mut count = [0u64; 20];
    let mut carry = 0i64;
    for trial in 0..10_000u32 {
        let s = align(&pair, carry, &rng, Day(trial), purpose);
        for k in &s.chosen {
            count[usize::try_from(k - 500).expect("a key of the pair")] += 1;
        }
        carry = s.carry_micro;
    }
    let low: u64 = count[..10].iter().sum();
    let high: u64 = count[10..].iter().sum();
    assert!(low > 0, "the persons of probability p were selected");
    #[allow(clippy::cast_precision_loss)]
    let ratio = high as f64 / low as f64;
    assert!(
        (1.8..=2.2).contains(&ratio),
        "twice the probability selected {ratio:.3} times as often ({high} against {low})"
    );
}

/// The pension law of 2021: 35 and 15 years, 0.75% a month early, survivors
/// 50%, point 1,442 lei, minimum pension 800 lei.
fn law_2021() -> PensionLaw {
    PensionLaw {
        full_years: 35,
        min_years: 15,
        early_reduction_ppm_per_month: 7_500,
        survivor_share_ppm: 500_000,
        point_value: lei(1_442),
        minimum: lei(800),
    }
}

/// **AC-DEM-10** `[unit]` A new pension: points = max(career points ÷ full
/// contribution years × (1 − 0.75% × months early, up to 60, only below
/// full + 8 years), survivor points); no pension below 15 years of contribution;
/// the pension paid is points × point value, not below the minimum pension.
///
/// Readings: "only below full + 8 years" is fewer than 43 years (516 months)
/// of contribution; "no pension below 15 years" is `None` for fewer than 180
/// months, survivor points or not. The pension paid is social transfers'
/// rule (`econ_mech_transfers::pension`), which pays new and starting
/// pensioners alike. Where the exact points are not a whole millionth the
/// test allows one millionth of rounding.
#[test]
fn ac_dem_10() {
    let law = law_2021();
    // 35 points over 35 years: one point; 40 years of contribution.
    let full = 35_000_000;
    assert_eq!(new_pension_points(full, 0, 0, 480, &law), Some(1_000_000));
    // 12 months early: 9% less.
    assert_eq!(new_pension_points(full, 0, 12, 480, &law), Some(910_000));
    // 60 months early: 45% less; 80 months early is capped at 60.
    assert_eq!(new_pension_points(full, 0, 60, 480, &law), Some(550_000));
    assert_eq!(new_pension_points(full, 0, 80, 480, &law), Some(550_000));
    // From 43 years of contribution: no reduction.
    assert_eq!(new_pension_points(full, 0, 24, 515, &law), Some(820_000));
    assert_eq!(new_pension_points(full, 0, 24, 516, &law), Some(1_000_000));
    assert_eq!(new_pension_points(full, 0, 24, 600, &law), Some(1_000_000));
    // The survivor's points when larger.
    assert_eq!(
        new_pension_points(full, 1_200_000, 12, 480, &law),
        Some(1_200_000)
    );
    assert_eq!(
        new_pension_points(full, 900_000, 12, 480, &law),
        Some(910_000)
    );
    assert_eq!(new_pension_points(0, 700_000, 0, 200, &law), Some(700_000));
    // Not a whole millionth: 10 points over 35 years, 7 months early.
    let got = new_pension_points(10_000_000, 0, 7, 300, &law).expect("15 years and more");
    let exact = 10_000_000.0 / 35.0 * (1.0 - 0.0075 * 7.0);
    #[allow(clippy::cast_precision_loss)]
    let diff = (got as f64 - exact).abs();
    assert!(diff <= 1.0, "{got} against {exact:.3}");
    // Fewer than 15 years: no pension.
    assert_eq!(new_pension_points(full, 0, 0, 179, &law), None);
    assert_eq!(new_pension_points(full, 800_000, 0, 179, &law), None);
    assert_eq!(new_pension_points(full, 0, 0, 0, &law), None);
    assert_eq!(
        new_pension_points(full, 0, 0, 180, &law).map(|p| p > 0),
        Some(true)
    );

    // The pension paid: points × point value, not below the minimum.
    let rule = PensionRule {
        point_value: law.point_value,
        minimum: law.minimum,
    };
    let one = new_pension_points(full, 0, 0, 480, &law).expect("a pension");
    assert_eq!(pension(one, &rule), lei(1_442));
    let reduced = new_pension_points(full, 0, 12, 480, &law).expect("a pension");
    assert_eq!(
        pension(reduced, &rule),
        Bani(131_222),
        "0.91 × 1,442 = 1,312.22 lei"
    );
    let early = new_pension_points(full, 0, 60, 480, &law).expect("a pension");
    assert_eq!(
        pension(early, &rule),
        lei(800),
        "0.55 × 1,442 = 793.10 lei is below the minimum"
    );
    let small = new_pension_points(10_000_000, 0, 0, 200, &law).expect("a pension");
    assert_eq!(
        pension(small, &rule),
        lei(800),
        "412 lei is below the minimum"
    );
}

/// The child support of the 2021 law: 427 and 214 lei, no child tax credit.
fn support_2021() -> ChildSupport {
    ChildSupport {
        under2: lei(427),
        to18: lei(214),
        tax_credit: Bani::ZERO,
    }
}

/// **AC-DEM-17** `[unit]` Fertility multipliers: each is exactly 1 under the
/// 2021 law at the reference state (no benefit change, nobody unemployed in
/// the stratum's normalisation, income growth at normal); $M^{pol}$ rises with
/// ΔB and falls with income, and every multiplier stays within its bounds for
/// any input (including zero income and negative ΔB); the normalisation $k_s$
/// gives the base-rate target on the starting population.
///
/// Readings:
/// - The spec's worked values are checked as well: ΔB = (2 Δunder2 + 16
///   Δto18) / 18 + Δcredit; $M^{pol}$ = 1 + 0.1 ΔB / Y with Y not below 25%
///   of the median; $M^{job}$ 0.85; $M^{cyc}$ = 1 + 0.2 (g − 2%). Values that
///   are not whole millionths (or bani) may be one unit off.
/// - "Rises with ΔB and falls with income": never falls along a sweep of ΔB
///   and never rises along a sweep of income, and moves strictly inside the
///   bounds.
/// - "Any input": a grid that includes zero and negative incomes, a median
///   of one ban, ΔB of ±10 million lei, and growth rates up to `i32::MIN` and
///   `i32::MAX`.
/// - "Gives the base-rate target": $\sum w f k M^{job} = \sum w f$ to within
///   the rounding of $k_s$ to a millionth (half a millionth of the total).
#[test]
fn ac_dem_17() {
    let r = FertilityResponse::spec_defaults();
    assert_eq!(r.benefit_ppm, 100_000);
    assert_eq!(r.unemployed_ratio_ppm, 850_000);
    assert_eq!(r.cycle_ppm, 200_000);
    assert_eq!(r.normal_growth_ppm, 20_000);
    assert_eq!(r.lag_months, 9);
    assert_eq!(r.income_floor_ppm, 250_000);
    const ONE: u32 = 1_000_000;

    // Exactly one at the reference state.
    let s21 = support_2021();
    assert_eq!(benefit_gap(&s21, &s21), Bani::ZERO);
    for (income, median) in [
        (lei(0), lei(2_000)),
        (lei(1_500), lei(2_000)),
        (lei(90_000), lei(2_000)),
    ] {
        assert_eq!(
            m_pol(Bani::ZERO, income, median, &r),
            ONE,
            "no benefit change, income {income:?}"
        );
    }
    assert_eq!(m_job(false, &r), ONE);
    assert_eq!(m_cyc(Some(r.normal_growth_ppm), &r), ONE);
    assert_eq!(m_cyc(None, &r), ONE, "no income history yet");
    let nobody: Vec<FertileWoman> = (0..50u32)
        .map(|i| FertileWoman {
            weight: 990 + i,
            fertility_ppb: 1_000_000 * (1 + i),
            m_job_ppm: ONE,
        })
        .collect();
    assert_eq!(job_normalisation(&nobody), ONE, "nobody unemployed");

    // The spec's values.
    let then = ChildSupport {
        under2: lei(427 + 90),
        to18: lei(214 + 9),
        tax_credit: lei(500),
    };
    assert_eq!(
        benefit_gap(&then, &s21),
        lei(518),
        "(180 + 144) / 18 + 500 lei"
    );
    let doubled = ChildSupport {
        under2: lei(854),
        to18: lei(428),
        tax_credit: Bani::ZERO,
    };
    let gap = benefit_gap(&doubled, &s21).get();
    assert!(
        (gap - 23_767).abs() <= 1,
        "doubling both allowances: {gap} bani, about 238 lei"
    );
    let halved = ChildSupport {
        under2: Bani(21_350),
        to18: lei(107),
        tax_credit: Bani::ZERO,
    };
    assert!(
        benefit_gap(&halved, &s21) < Bani::ZERO,
        "a cut is a negative gap"
    );
    let close = |got: u32, want: u32| got.abs_diff(want) <= 1;
    let median = lei(2_000);
    assert!(
        close(m_pol(lei(100), lei(1_000), median, &r), 1_010_000),
        "10% of income: +1%"
    );
    assert!(
        close(m_pol(lei(100), lei(0), median, &r), 1_020_000),
        "no income: at the floor"
    );
    assert!(
        close(m_pol(lei(100), lei(300), median, &r), 1_020_000),
        "below the floor"
    );
    assert!(
        m_pol(lei(100), lei(600), median, &r) > m_pol(lei(100), lei(3_000), median, &r),
        "the poorer respond more to the same lei"
    );
    assert_eq!(m_job(true, &r), 850_000);
    assert!(
        close(m_cyc(Some(-30_000), &r), 990_000),
        "5 points below normal: −1%"
    );
    assert!(
        close(m_cyc(Some(70_000), &r), 1_010_000),
        "5 points above normal: +1%"
    );

    // Rises with ΔB, falls with income.
    let mut last = 0;
    let mut moved = false;
    for k in -100..=100i64 {
        let v = m_pol(lei(50 * k), lei(1_200), median, &r);
        assert!(v >= last, "M_pol fell as ΔB rose to {} lei", 50 * k);
        moved |= v > last && last > 800_000 && v < 1_300_000;
        last = v;
    }
    assert!(moved, "M_pol moves with ΔB inside its bounds");
    let mut last = u32::MAX;
    let mut moved = false;
    for k in 0..=200i64 {
        let v = m_pol(lei(300), lei(100 * k), median, &r);
        assert!(v <= last, "M_pol rose as income rose to {} lei", 100 * k);
        moved |= v < last && last < 1_300_000 && v > 800_000;
        last = v;
    }
    assert!(moved, "M_pol moves with income inside its bounds");

    // Within the bounds for any input.
    let gaps = [
        Bani(-1_000_000_000),
        lei(-5_000),
        lei(-238),
        Bani(-1),
        Bani::ZERO,
        Bani(1),
        lei(238),
        lei(5_000),
        Bani(1_000_000_000),
    ];
    let incomes = [
        lei(-1_000),
        Bani::ZERO,
        Bani(1),
        lei(100),
        lei(2_000),
        lei(1_000_000),
    ];
    for &d in &gaps {
        for &y in &incomes {
            for m in [Bani(1), lei(2_000), lei(100_000)] {
                let v = m_pol(d, y, m, &r);
                assert!(
                    (800_000..=1_300_000).contains(&v),
                    "M_pol({d:?}, {y:?}, {m:?}) = {v}"
                );
            }
        }
    }
    for g in [
        i32::MIN,
        -1_000_000,
        -200_000,
        -30_000,
        0,
        20_000,
        70_000,
        1_000_000,
        i32::MAX,
    ] {
        let v = m_cyc(Some(g), &r);
        assert!((970_000..=1_030_000).contains(&v), "M_cyc({g}) = {v}");
    }
    for u in [false, true] {
        let v = m_job(u, &r);
        assert!(v > 0 && v <= ONE, "M_job({u}) = {v}");
    }

    // The normalisation gives the base-rate target.
    let rng = KeyedRng::new(11);
    let women: Vec<FertileWoman> = (0..400u64)
        .map(|i| {
            let mut d = rng.draw(Stream::Test, 1, i);
            FertileWoman {
                weight: u32::try_from(900 + d.below(200)).expect("fits"),
                fertility_ppb: u32::try_from(5_000_000 + d.below(150_000_000)).expect("fits"),
                m_job_ppm: if d.below(10) == 0 {
                    r.unemployed_ratio_ppm
                } else {
                    ONE
                },
            }
        })
        .collect();
    let k = job_normalisation(&women);
    assert!(
        k > ONE,
        "unemployed women in the stratum: k above one ({k})"
    );
    let base: u128 = women
        .iter()
        .map(|x| u128::from(x.weight) * u128::from(x.fertility_ppb))
        .sum();
    let with_job: u128 = women
        .iter()
        .map(|x| u128::from(x.weight) * u128::from(x.fertility_ppb) * u128::from(x.m_job_ppm))
        .sum();
    // Σ w f M k, in units of 10^-12 of Σ w f; half a millionth of k is the
    // rounding allowed.
    let normalised = with_job * u128::from(k);
    let target = base * 1_000_000_000_000;
    let tolerance = with_job / 2 + 1;
    assert!(
        normalised.abs_diff(target) <= tolerance,
        "k = {k}: Σ w f M k = {normalised} against Σ w f = {target}"
    );
}
