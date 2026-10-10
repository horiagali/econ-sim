//! Acceptance tests for AC-POPH-* (spec: society/population-housing).
//! Written in a test-authoring session from the spec, the API skeleton and the
//! Python reference, before the Rust stage existed (ADR-0014).
//!
//! "The fixtures" are the normalised Census 2021 margin files of Romania:
//! the county margins and the housing margins (locality size and tenure).

// Column tables are indexed in parallel; index loops are the clear way to read them.
#![allow(clippy::needless_range_loop)]

use std::borrow::Cow;
use std::sync::OnceLock;

use econ_popgen::{
    HouseholdHousing, HousingError, HousingMargins, HousingParams, NO_TENURE, Population,
    SIZE_GROUPS, TENURES, assign_housing,
};

use super::support::{
    DIFF_SCALES, HOUSING_GOLDEN, OTHER_SEED, SCALES, SEED, STATE_GOLDEN, Tolerances, assert_golden,
    assert_no_failures, assert_same_column, check_family, column, golden_hash, hex,
    housing_margins, margins, population, population_at, reference,
};

/// Tolerance in percent by expected synthetic records behind a cell.
const TOLERANCES: Tolerances = [(1000, 1), (100, 5), (30, 15)];
/// "Persons aged 65 or more" (AC-POPH-04).
const OLD_FROM_YEARS: u16 = 65;

/// Stage E on the fixture population at one of the three scales (run once).
fn housing(scale: u32) -> &'static HouseholdHousing {
    static HOUSING: [OnceLock<HouseholdHousing>; 3] =
        [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let i = SCALES
        .iter()
        .position(|&s| s == scale)
        .expect("one of the three scales");
    HOUSING[i].get_or_init(|| {
        assign_housing(
            population(scale),
            housing_margins(),
            &HousingParams::new(SEED),
        )
        .expect("fixture population gets housing")
    })
}

/// Stage E on the fixture population at any scale.
fn housing_at(scale: u32) -> Cow<'static, HouseholdHousing> {
    if SCALES.contains(&scale) {
        Cow::Borrowed(housing(scale))
    } else {
        Cow::Owned(
            assign_housing(
                &population_at(scale),
                housing_margins(),
                &HousingParams::new(SEED),
            )
            .expect("fixture population gets housing"),
        )
    }
}

/// Weighted totals in the shape of the two margin tables.
struct Totals {
    /// Persons by their own age, `[county][age_group][locality_class]`.
    persons: Vec<u64>,
    /// Private households, `[county][size_group][tenure]`.
    households: Vec<u64>,
}

fn totals(m: &HousingMargins, pop: &Population, housing: &HouseholdHousing) -> Totals {
    let (n_c, n_g, n_k) = (
        m.counties.len(),
        m.age_group_from.len(),
        usize::from(m.locality_classes),
    );
    let mut t = Totals {
        persons: vec![0; n_c * n_g * n_k],
        households: vec![0; n_c * SIZE_GROUPS * TENURES],
    };
    let (hh, p) = (&pop.households, &pop.persons);
    let mut members = vec![0u32; hh.hh_weight.len()];
    for i in 0..p.household_id.len() {
        let h = p.household_id[i] as usize;
        members[h] += 1;
        let years = p.age[i] / 12;
        let g = m
            .age_group_from
            .iter()
            .rposition(|&from| from <= years)
            .expect("age groups start at 0");
        let cell = usize::from(hh.hh_county[h]) * n_g + g;
        t.persons[cell * n_k + usize::from(housing.hh_locality_size[h])] +=
            u64::from(hh.hh_weight[h]);
    }
    for h in 0..hh.hh_weight.len() {
        if !hh.hh_collective[h] {
            let size_group = usize::from(members[h] != 1);
            let cell = usize::from(hh.hh_county[h]) * SIZE_GROUPS + size_group;
            t.households[cell * TENURES + usize::from(housing.hh_tenure[h])] +=
                u64::from(hh.hh_weight[h]);
        }
    }
    t
}

fn person_dims(m: &HousingMargins) -> [usize; 3] {
    [
        m.counties.len(),
        m.age_group_from.len(),
        usize::from(m.locality_classes),
    ]
}

fn household_dims(m: &HousingMargins) -> [usize; 3] {
    [m.counties.len(), SIZE_GROUPS, TENURES]
}

/// One family of margins of the persons table or of the households table.
fn check_table(
    m: &HousingMargins,
    scale: u32,
    t: &Totals,
    households: bool,
    family: &str,
    key: &dyn Fn(&[usize]) -> Vec<usize>,
    failures: &mut Vec<String>,
) {
    if households {
        check_family(
            &TOLERANCES,
            family,
            scale,
            &household_dims(m),
            &t.households,
            &m.households_by_tenure,
            key,
            failures,
        );
    } else {
        check_family(
            &TOLERANCES,
            family,
            scale,
            &person_dims(m),
            &t.persons,
            &m.persons_by_locality,
            key,
            failures,
        );
    }
}

/// The national totals: persons by locality class, households by tenure.
fn check_national(m: &HousingMargins, scale: u32, t: &Totals, failures: &mut Vec<String>) {
    check_table(
        m,
        scale,
        t,
        false,
        "national locality class",
        &|k| vec![k[2]],
        failures,
    );
    check_table(
        m,
        scale,
        t,
        true,
        "national tenure",
        &|k| vec![k[2]],
        failures,
    );
}

/// AC-POPH-01 [unit] Structure, at each of the three scales: one value of each column per household; `hh_locality_size` is 0 to 5; `hh_urban` is 1 exactly when `hh_locality_size` ≥ 3; a private household has `hh_tenure` 0 to 2 and any other record 255; no household is in a class its county has nobody in.
#[test]
fn ac_poph_01() {
    let m = housing_margins();
    let (n_g, n_k) = (m.age_group_from.len(), usize::from(m.locality_classes));
    assert_eq!((n_k, m.urban_from_class), (6, 3), "classes of the fixture");
    for scale in SCALES {
        let (hh, housing) = (&population(scale).households, housing(scale));
        let n = hh.hh_weight.len();
        assert_eq!(
            housing.hh_locality_size.len(),
            n,
            "1:{scale}: one locality size per household"
        );
        assert_eq!(
            housing.hh_urban.len(),
            n,
            "1:{scale}: one urban flag per household"
        );
        assert_eq!(
            housing.hh_tenure.len(),
            n,
            "1:{scale}: one tenure per household"
        );
        for h in 0..n {
            let (class, tenure) = (
                usize::from(housing.hh_locality_size[h]),
                housing.hh_tenure[h],
            );
            assert!(class < n_k, "1:{scale}: household {h} is in class {class}");
            assert_eq!(
                housing.hh_urban[h],
                class >= 3,
                "1:{scale}: household {h} in class {class}"
            );
            if hh.hh_collective[h] {
                assert_eq!(tenure, NO_TENURE, "1:{scale}: collective record {h}");
            } else {
                assert!(
                    usize::from(tenure) < TENURES,
                    "1:{scale}: private household {h} has tenure {tenure}"
                );
            }
            let county = usize::from(hh.hh_county[h]);
            let census: u64 = (0..n_g)
                .map(|g| m.persons_by_locality[(county * n_g + g) * n_k + class])
                .sum();
            assert!(
                census > 0,
                "1:{scale}: household {h} is in class {class}, which county {} does not have",
                m.counties[county]
            );
        }
    }
    assert_eq!(NO_TENURE, 255);
}

/// AC-POPH-02 [unit] The stage does not change the population: the state hash is that of AC-POP-06.
#[test]
fn ac_poph_02() {
    for scale in SCALES {
        let pop = population(scale).clone();
        let before = pop.clone();
        assign_housing(&pop, housing_margins(), &HousingParams::new(SEED))
            .expect("fixture population gets housing");
        assert_eq!(pop, before, "1:{scale}: population changed");
        assert_eq!(
            hex(pop.state_hash()),
            golden_hash(STATE_GOLDEN, scale),
            "state hash at 1:{scale}"
        );
    }
}

/// AC-POPH-03 [unit] At each of the three scales the weighted margins are within the tolerances of the table below.
#[test]
fn ac_poph_03() {
    let m = housing_margins();
    let urban = |class: usize| usize::from(class >= usize::from(m.urban_from_class));
    let mut failures = Vec::new();
    for scale in SCALES {
        let t = totals(m, population(scale), housing(scale));
        check_national(m, scale, &t, &mut failures);
        // Cells are [county, age group, locality class].
        let mut persons = |family: &str, key: &dyn Fn(&[usize]) -> Vec<usize>| {
            check_table(m, scale, &t, false, family, key, &mut failures);
        };
        persons("national urban", &|k| vec![urban(k[2])]);
        persons("county x locality class", &|k| vec![k[0], k[2]]);
        persons("county x urban", &|k| vec![k[0], urban(k[2])]);
        // Cells are [county, size group, tenure].
        let mut households = |family: &str, key: &dyn Fn(&[usize]) -> Vec<usize>| {
            check_table(m, scale, &t, true, family, key, &mut failures);
        };
        households("national size group x tenure", &|k| vec![k[1], k[2]]);
        households("county x tenure", &|k| vec![k[0], k[2]]);
        households("county x size group x tenure", &|k| k.to_vec());
    }
    assert_no_failures(&failures);
}

/// AC-POPH-04 [unit] At each of the three scales, nationally, the share of persons aged 65 or more is highest in the smallest locality class, and in every class it is within 4 percentage points of the census share.
#[test]
fn ac_poph_04() {
    let m = housing_margins();
    let (n_g, n_k) = (m.age_group_from.len(), usize::from(m.locality_classes));
    // National (persons of 65 or more, all persons) in each locality class.
    let old_and_all = |table: &[u64]| {
        let mut out = vec![(0u128, 0u128); n_k];
        for (flat, &v) in table.iter().enumerate() {
            let (g, k) = ((flat / n_k) % n_g, flat % n_k);
            if m.age_group_from[g] >= OLD_FROM_YEARS {
                out[k].0 += u128::from(v);
            }
            out[k].1 += u128::from(v);
        }
        out
    };
    let want = old_and_all(&m.persons_by_locality);
    for scale in SCALES {
        let t = totals(m, population(scale), housing(scale));
        let got = old_and_all(&t.persons);
        for k in 0..n_k {
            let ((g_old, g_all), (w_old, w_all)) = (got[k], want[k]);
            assert!(g_all > 0, "1:{scale}: nobody lives in class {k}");
            // |g_old / g_all − w_old / w_all| ≤ 4 / 100, without division.
            assert!(
                (g_old * w_all).abs_diff(w_old * g_all) * 100 <= 4 * g_all * w_all,
                "1:{scale} class {k}: {g_old} of {g_all} synthetic persons are 65 or more, {w_old} of {w_all} in the census"
            );
            // Highest in the smallest class.
            assert!(
                k == 0 || got[0].0 * g_all > g_old * got[0].1,
                "1:{scale}: the share of 65 or more in class {k} ({g_old} of {g_all}) is not below that of the smallest class ({} of {})",
                got[0].0,
                got[0].1
            );
        }
    }
}

/// AC-POPH-05 [unit] Same population, margins and seed give identical columns (equal housing hash). A different seed changes which households are where, and the national totals by locality class and by tenure stay within the tolerance of AC-POPH-03.
#[test]
fn ac_poph_05() {
    let (m, scale) = (housing_margins(), 1000);
    let (pop, first) = (population(scale), housing(scale));
    let again = assign_housing(pop, m, &HousingParams::new(SEED)).expect("housing");
    assert_eq!(&again, first);
    assert_eq!(again.state_hash(), first.state_hash());

    let other = assign_housing(pop, m, &HousingParams::new(OTHER_SEED)).expect("housing");
    assert_ne!(other.state_hash(), first.state_hash());
    assert_ne!(other.hh_locality_size, first.hh_locality_size);
    assert_ne!(other.hh_tenure, first.hh_tenure);
    let mut failures = Vec::new();
    check_national(m, scale, &totals(m, pop, &other), &mut failures);
    assert_no_failures(&failures);
}

/// AC-POPH-06 [golden] The housing hash of the fixture population at each of the three scales equals the committed golden value.
#[test]
fn ac_poph_06() {
    assert_golden(HOUSING_GOLDEN, "housing hash", |scale| {
        housing_at(scale).state_hash()
    });
}

/// AC-POPH-07 [diff] On the fixtures at 1:1000 and 1:200 the Rust stage and the independent Python reference produce identical columns, household for household.
#[test]
fn ac_poph_07() {
    for scale in DIFF_SCALES {
        let housing = housing_at(scale);
        let r = reference(scale);
        let wide = |column: &[u8]| -> Vec<u64> { column.iter().map(|&x| u64::from(x)).collect() };
        let urban: Vec<u64> = housing.hh_urban.iter().map(|&x| u64::from(x)).collect();
        for (name, got) in [
            ("hh_locality_size", wide(&housing.hh_locality_size)),
            ("hh_urban", urban),
            ("hh_tenure", wide(&housing.hh_tenure)),
        ] {
            assert_same_column(scale, name, &got, &column(r, name));
        }
        let want_hash = r["summary"]["housing_hash"]
            .as_str()
            .expect("reference housing hash");
        assert_eq!(
            hex(housing.state_hash()),
            want_hash,
            "1:{scale} housing hash"
        );
    }
}

/// AC-POPH-08 [unit] Margins for other counties than the population's, a table whose length does not match its dimensions, no locality classes, and a county whose census table is empty are each rejected with a named error and produce no output.
#[test]
fn ac_poph_08() {
    let m = housing_margins();
    let pop = population(1000);
    let params = HousingParams::new(SEED);
    let (n_c, n_g, n_k) = (
        m.counties.len(),
        m.age_group_from.len(),
        usize::from(m.locality_classes),
    );
    assert_eq!(m.counties, margins().counties, "counties of both fixtures");

    // Margins without the population's last county (the tables fit their own counties).
    let mut other = m.clone();
    other.counties.pop();
    other.persons_by_locality.truncate((n_c - 1) * n_g * n_k);
    other
        .households_by_tenure
        .truncate((n_c - 1) * SIZE_GROUPS * TENURES);
    assert_eq!(
        assign_housing(pop, &other, &params),
        Err(HousingError::ShapeMismatch)
    );

    let mut short = m.clone();
    short.persons_by_locality.pop();
    assert_eq!(
        assign_housing(pop, &short, &params),
        Err(HousingError::ShapeMismatch)
    );
    let mut short = m.clone();
    short.households_by_tenure.pop();
    assert_eq!(
        assign_housing(pop, &short, &params),
        Err(HousingError::ShapeMismatch)
    );

    let mut classless = m.clone();
    classless.locality_classes = 0;
    classless.persons_by_locality.clear();
    assert_eq!(
        assign_housing(pop, &classless, &params),
        Err(HousingError::EmptyTable)
    );

    // The census has nobody in the eighth county; the population has households there.
    let county = 7;
    let mut nobody = m.clone();
    nobody.persons_by_locality[county * n_g * n_k..(county + 1) * n_g * n_k].fill(0);
    assert_eq!(
        assign_housing(pop, &nobody, &params),
        Err(HousingError::NoMargin {
            county: m.counties[county].clone()
        })
    );

    // The same for a county with no tenure counts at all.
    let mut untenured = m.clone();
    let cells = SIZE_GROUPS * TENURES;
    untenured.households_by_tenure[county * cells..(county + 1) * cells].fill(0);
    assert_eq!(
        assign_housing(pop, &untenured, &params),
        Err(HousingError::NoMargin {
            county: m.counties[county].clone()
        })
    );
}
