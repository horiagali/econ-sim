//! Roadmap M5's starting data on the Romania scenario (1:1000, seed 42):
//! what the scenario builder writes and the `World` loads, before any
//! mechanic reads it. Not acceptance tests (those are the test writer's,
//! ADR-0014): checks of the loading, the counts and the tables.

use econ_core::world::{Law, World, WorldParams};
use econ_io::read_start_state;
use econ_popgen::{Enrolled, Role, Sex};
use econ_scenario::{BuildParams, build, read_inputs, write};
use econ_types::{Bani, Day};

const SEED: u64 = 42;

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_people_data_loads_and_changes_nothing_else() {
    let fixtures = repo_root().join("python/pipeline/fixtures");
    let inputs = read_inputs(&fixtures).expect("the fixtures read");
    let scenario = build(
        &inputs,
        &BuildParams {
            sample_scale: 1000,
            rng_seed: SEED,
        },
    )
    .expect("the fixtures build");
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("econ-core-people")
        .join("scenario");
    if let Some(parent) = dir.parent() {
        if parent.exists() {
            std::fs::remove_dir_all(parent).expect("clean the test folder");
        }
        std::fs::create_dir_all(parent).expect("create the test folder");
    }
    write(&scenario, &inputs, &dir).expect("the scenario writes");
    let start =
        read_start_state(&dir, &fixtures.join("io_table2021_ro.json")).expect("the scenario reads");
    let w = World::new(
        start.clone(),
        Law::placeholder_2021(),
        WorldParams::spec_defaults(SEED),
    )
    .expect("the world builds");
    let p = w.people();
    let (n_p, n_hh) = (
        w.persons().household_id.len(),
        w.households().hh_weight.len(),
    );
    let hh_weight = &w.households().hh_weight;
    let weight = |i: usize| u64::from(hh_weight[w.persons().household_id[i] as usize]);

    // Stable identities: the start's indices, then the linked emigrants'.
    assert_eq!(p.person_key, (0..n_p as u64).collect::<Vec<_>>());
    assert_eq!(p.hh_key, (0..n_hh as u64).collect::<Vec<_>>());
    let le = &p.linked_emigrants;
    let n_le = le.household_id.len() as u64;
    assert_eq!(
        le.person_key,
        (n_p as u64..n_p as u64 + n_le).collect::<Vec<_>>()
    );
    assert_eq!(p.next_person_key, n_p as u64 + n_le);
    assert_eq!(p.next_hh_key, n_hh as u64);

    // Sex and role as the generator made them; every household has one head.
    let pop = &scenario.population.persons;
    assert_eq!(p.sex, pop.sex);
    assert_eq!(p.role, pop.role);
    assert_eq!(w.person_ages(), pop.age.as_slice());
    let mut heads = vec![0u32; n_hh];
    for i in 0..n_p {
        if p.role[i] == Role::Head {
            heads[w.persons().household_id[i] as usize] += 1;
        }
    }
    assert!(heads.iter().all(|&h| h == 1), "one head per household");

    // Linked emigrants: 1.2 million real households within half a record, in
    // private households, of working age at departure; the diaspora total is
    // 4 million.
    let linked: u64 = le
        .household_id
        .iter()
        .map(|&h| u64::from(hh_weight[h as usize]))
        .sum();
    let largest = u64::from(*hh_weight.iter().max().expect("households"));
    assert!(linked.abs_diff(1_200_000) <= largest / 2, "{linked}");
    let stock: u64 = p.diaspora_stock.iter().map(|c| c.persons).sum();
    assert_eq!(linked + stock, 4_000_000);
    for k in 0..le.household_id.len() {
        assert!(!start.hh_collective[le.household_id[k] as usize]);
        let at_departure = le.age[k] - le.months_abroad[k];
        assert!((18 * 12..70 * 12).contains(&at_departure));
        assert!(le.months_abroad[k] < 20 * 12);
    }
    let men = le.sex.iter().filter(|&&s| s == Sex::M).count();
    assert!(men * 2 > le.sex.len(), "most emigrants are men");

    // The demographic tables: pre-pandemic, as the fixture has them.
    let d = p.demography.as_ref().expect("demography.json");
    assert_eq!(d.mortality.base_year, 2018);
    assert_eq!(d.fertility.year, 2019);
    let start_date = w.date();
    assert_eq!(
        d.retirement_age_schedule.age_in_force(0, start_date),
        Some(741)
    );
    assert_eq!(
        d.retirement_age_schedule.age_in_force(1, start_date),
        Some(780)
    );
    // A year of pre-pandemic deaths on the starting population: 240,000 to
    // 280,000 (AC-DEM-04's band; 2021 had 335,527).
    let deaths: u64 = (0..n_p)
        .map(|i| {
            let age = usize::from(w.person_ages()[i] / 12).min(110);
            weight(i) * u64::from(d.mortality.yearly[p.sex[i] as usize][age])
        })
        .sum::<u64>()
        / 1_000_000_000;
    assert!((240_000..280_000).contains(&deaths), "{deaths}");
    // And a year of births at the 2019 rates: 175,000 to 205,000.
    let births: u64 = (0..n_p)
        .filter(|&i| p.sex[i] == Sex::F)
        .map(|i| weight(i) * u64::from(d.fertility.rate_ppm(w.person_ages()[i] / 12)))
        .sum::<u64>()
        / 1_000_000;
    assert!((175_000..205_000).contains(&births), "{births}");

    // School places: stage K's enrolment, weighted.
    let mut places = [0u64; 8];
    for i in 0..n_p {
        places[p.enrolled[i] as usize] += weight(i);
    }
    places[Enrolled::None as usize] = 0;
    assert_eq!(p.edu_capacity, places);
    assert!(p.edu_capacity[Enrolled::Primary as usize] > 700_000);

    // Careers: none under 18; an employee of 50 has more than 20 years.
    for i in 0..n_p {
        if w.person_ages()[i] < 18 * 12 {
            assert_eq!(p.contribution_months[i], 0);
        }
    }
    assert!(p.wage_nominal > Bani::from_lei(4_000) && p.wage_nominal < Bani::from_lei(7_000));
    let mid: Vec<usize> = (0..n_p)
        .filter(|&i| w.persons().wage[i].get() > 0 && w.person_ages()[i] / 12 == 50)
        .collect();
    assert!(!mid.is_empty());
    assert!(mid.iter().all(|&i| p.contribution_months[i] > 20 * 12));

    // Loading it changes nothing else: a world without the people data
    // stands on the same state hash after a few days.
    let mut bare = start;
    bare.people = Default::default();
    let mut other = World::new(
        bare,
        Law::placeholder_2021(),
        WorldParams::spec_defaults(SEED),
    )
    .expect("the world builds");
    let mut w = w;
    while w.day() < Day(3) {
        w.step(&[]).expect("a day");
        other.step(&[]).expect("a day");
    }
    assert_eq!(w.state_hash(), other.state_hash());
}
