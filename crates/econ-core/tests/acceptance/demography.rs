//! Acceptance tests for the `[sim]` criteria of economy/demographics
//! (AC-DEM-03 to 09, 11 to 16, 18 to 20) on the `World`, behind the feature
//! `demography`. Written in a test-authoring session from the spec before
//! the code existed (ADR-0014), against the test writer's additions:
//! `World::people`, `household_life`, `people_report`,
//! `retirement_age_in_force`, `mortality_table`, `WorldParams::demography`,
//! `PhaseId::People` and `StartEdit::ScaleLabourRequirementFrom`.
//!
//! Readings shared by every test:
//! - Worlds run under the 2021 law of the scenario's folder (`law.json`)
//!   with every default feature (M3 and M4) on and migration off; every
//!   check of the day holds on every day stepped.
//! - "The weighted population" counts live persons of live households,
//!   with their household's weight; tombstones count nowhere.
//! - "The first year" is the first 12 month ends (December 2021 to November
//!   2022). A yearly count by calendar year sums the month ends of that year.
//! - The pay-day record (`World::pay_slips`) is read only on month ends
//!   that are not in December, because the December close compacts the
//!   tables after the pay day, so that its indices may no longer be the
//!   tables'.
//! - Runs of years are release-only (`cfg_attr(debug_assertions, ignore)`,
//!   the form of AC-WLD-10): debug builds are too slow for them.

use std::collections::{BTreeMap, BTreeSet};

use econ_core::levers::{LeverId, LeverValue};
use econ_core::world::{Birth, StartEdit, World, WorldParams};
use econ_ledger::{FlowCode, Instrument, Sector};
use econ_mech_transfers::{PensionRule, pension};
use econ_popgen::{Activity, NO_EMPLOYER, Role, Sex};
use econ_types::{Bani, Date};

use super::support::step;
use super::support_dem::{
    YEAR, assert_moved_identity, budgets, dem_params, e0_of_table, exit_run, household_index,
    in_couple, in_labour_force, live_weights, members, person_index, population, ratio, report,
    run_until, snapshot, step_checked, to_close, without_economic_terms, world_dem,
    world_dem_default,
};
use super::support_lev::{date, give, key_of, set};
use super::support_m3::{code, flow, legs_of_day, legs_sum};

/// The law in force's amount of a money lever.
fn law_money(w: &World, lever: LeverId) -> Bani {
    match w.law().get(lever, None) {
        Some(LeverValue::Money(b)) => *b,
        other => panic!("{} is not money in force: {other:?}", lever.as_str()),
    }
}

/// Σ weight of a month's births or deaths, and the boys or men among them.
fn weight_of<T>(v: &[T], weight: impl Fn(&T) -> u32) -> u64 {
    v.iter().map(|x| u64::from(weight(x))).sum()
}

/// Each unit's weighted staff counted over live persons only.
fn live_staff(w: &World) -> Vec<u64> {
    let weights = live_weights(w);
    let mut staff = vec![0u64; w.firm_units().industry.len()];
    for (p, &e) in w.persons().employer.iter().enumerate() {
        if e != NO_EMPLOYER {
            staff[usize::from(e)] += u64::from(weights[p]);
        }
    }
    staff
}

/// **AC-DEM-03** `[sim]` Person accounting, every month of 20 years at 1:1000:
/// the weighted population changes by exactly births − deaths (+ immigrants
/// − emigrants when migration is on); every live person is in exactly one
/// live household; no live household is empty; every live person's age rose
/// by one month; tombstones count nowhere.
///
/// Readings: migration is off here (it is its own spec's and feature's).
/// "Exactly one live household": a live person's household is live (the
/// persons table gives each person one household). "Rose by one month": a
/// live person of last month is one month older, a newborn of the month is
/// 0, and no other live person appears. "Tombstones count nowhere": a dead
/// person is on no firm unit's staff (each unit's staff is the weight of its
/// live employees), and, from the month after the death, is paid no wage,
/// pension or benefit.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 20-year run: cargo test --release --features demography"
)]
fn ac_dem_03() {
    let mut w = world_dem_default(1000);
    let mut pop = population(&w);
    let ages = |w: &World| -> BTreeMap<u64, u16> {
        let people = w.people();
        live_weights(w)
            .iter()
            .enumerate()
            .filter(|&(_, &wt)| wt > 0)
            .map(|(p, _)| (people.person_key[p], people.age[p]))
            .collect()
    };
    let mut last_ages = ages(&w);
    for _ in 0..240 {
        let r = to_close(&mut w);
        let rep = report(&w);
        let births = weight_of(&rep.births, |b| b.weight);
        let deaths = weight_of(&rep.deaths, |d| d.weight);
        let now = population(&w);
        assert_eq!(
            i128::from(now),
            i128::from(pop) + i128::from(births) - i128::from(deaths),
            "{:?}: population {pop} + births {births} − deaths {deaths}",
            r.date
        );
        let people = w.people();
        let life = w.household_life();
        for (p, &hh) in w.persons().household_id.iter().enumerate() {
            if people.alive[p] {
                assert!(
                    life.hh_alive[hh as usize],
                    "{:?}: live person {} in ended household {hh}",
                    r.date, people.person_key[p]
                );
            }
        }
        for (hh, list) in members(&w).iter().enumerate() {
            if life.hh_alive[hh] {
                assert!(
                    !list.is_empty(),
                    "{:?}: live household {hh} is empty",
                    r.date
                );
            }
        }
        let born: BTreeSet<u64> = rep.births.iter().map(|b| b.child_key).collect();
        let died: BTreeSet<u64> = rep.deaths.iter().map(|d| d.person_key).collect();
        let now_ages = ages(&w);
        for (k, &a) in &now_ages {
            if born.contains(k) {
                assert_eq!(a, 0, "{:?}: newborn {k}", r.date);
            } else {
                let before = last_ages
                    .get(k)
                    .unwrap_or_else(|| panic!("{:?}: live person {k} from nowhere", r.date));
                assert_eq!(a, before + 1, "{:?}: person {k} aged by one month", r.date);
            }
        }
        assert_eq!(
            live_staff(&w),
            w.firm_units().staff,
            "{:?}: staff of live persons",
            r.date
        );
        if r.date.month != 12 {
            let slips = w.pay_slips();
            for p in 0..slips.pension.len().min(people.alive.len()) {
                if !people.alive[p] && !died.contains(&people.person_key[p]) {
                    assert_eq!(
                        (
                            slips.gross_wage[p],
                            slips.pension[p],
                            slips.unemployment_benefit[p]
                        ),
                        (Bani::ZERO, Bani::ZERO, Bani::ZERO),
                        "{:?}: tombstone {} was paid",
                        r.date,
                        people.person_key[p]
                    );
                }
            }
        }
        pop = now;
        last_ages = now_ages;
    }
}

/// **AC-DEM-04** `[sim]` The first year at 1:100 (pre-COVID): deaths between
/// 240,000 and 280,000 (not 2021's 335,527), with men 50% to 55% of them;
/// births between 175,000 and 205,000; boys 50.5% to 52.3% of births over
/// the first 10 years.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 10-year run: cargo test --release --features demography"
)]
fn ac_dem_04() {
    let mut w = world_dem_default(100);
    let (mut deaths, mut men, mut births) = (0u64, 0u64, 0u64);
    let (mut all_births, mut boys) = (0u64, 0u64);
    for m in 0..120 {
        to_close(&mut w);
        let rep = report(&w);
        if m < 12 {
            deaths += weight_of(&rep.deaths, |d| d.weight);
            men += weight_of(&rep.deaths, |d| if d.sex == Sex::M { d.weight } else { 0 });
            births += weight_of(&rep.births, |b| b.weight);
        }
        all_births += weight_of(&rep.births, |b| b.weight);
        boys += weight_of(&rep.births, |b| if b.sex == Sex::M { b.weight } else { 0 });
        if m == 11 {
            assert!(
                (240_000..=280_000).contains(&deaths),
                "deaths in the first year: {deaths}"
            );
            let share = ratio(men, deaths);
            assert!(
                (0.50..=0.55).contains(&share),
                "men's share of deaths: {share:.4}"
            );
            assert!(
                (175_000..=205_000).contains(&births),
                "births in the first year: {births}"
            );
        }
    }
    let share = ratio(boys, all_births);
    assert!(
        (0.505..=0.523).contains(&share),
        "boys' share of births over 10 years: {share:.4}"
    );
}

/// **AC-DEM-05** `[sim]` Life expectancy at birth from the run's first-year
/// death rates is within 0.6 years of the one computed directly from the
/// year's mortality table (the 2017–2019 average, 71.7 for men and 79.1 for
/// women, improved to 2022: about 72.5 and 79.9) and rises between 0.1 and
/// 0.3 years a year over 50 years.
///
/// Readings: the run is the exit run at 1:100. "From the run's death rates":
/// the year's weighted deaths by sex and single year of age over the mean of
/// the 12 month ends' population, central rates turned into probabilities,
/// deaths in mid-year. "The year's mortality table" is the World's table of
/// 2022 (`World::mortality_table`), whose own life expectancy must be
/// "about" 72.5 and 79.9: within 0.5 years (this test's reading of
/// "about"). "Rises between 0.1 and 0.3 years a year over 50 years": from
/// the first run year to the fiftieth, (e₀ of year 50 − e₀ of year 1) / 49.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the 50-year exit run: cargo test --release --features demography"
)]
fn ac_dem_05() {
    let run = exit_run(100);
    for (sex, table_e0) in [(Sex::M, 72.5), (Sex::F, 79.9)] {
        let from_table = e0_of_table(&run.table_2022, sex);
        assert!(
            (from_table - table_e0).abs() <= 0.5,
            "{sex:?}: life expectancy of the 2022 table {from_table:.2}, about {table_e0}"
        );
        let first = run.years[0].e0(sex);
        assert!(
            (first - from_table).abs() <= 0.6,
            "{sex:?}: the first year's {first:.2} against the table's {from_table:.2}"
        );
        let last = run.years[49].e0(sex);
        let rise = (last - first) / 49.0;
        assert!(
            (0.1..=0.3).contains(&rise),
            "{sex:?}: life expectancy rose {rise:.3} years a year ({first:.2} → {last:.2})"
        );
    }
}

/// **AC-DEM-06** `[sim]` Fertility in the first year: the total fertility rate
/// from the run's births is within 0.05 of the input's (1.77), the mothers'
/// mean age within 0.5 years of 28.1, and births to women in a couple are a
/// larger share than their share of women aged 15–49.
///
/// Readings: at 1:100. The rate is Σ over ages 15 to 49 of the year's births
/// to mothers of that age (in whole years at the birth) over the mean number
/// of women of that age at the 12 month ends. The mean age of mothers uses
/// exact ages (months + ½ over 12). "In a couple" is the birth's
/// `mother_in_couple`; the women's share is the mean over the 12 month ends
/// of the weighted share of live women aged 15–49 who live in a couple.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a year at 1:100: cargo test --release --features demography"
)]
fn ac_dem_06() {
    let mut w = world_dem_default(100);
    let mut births_at = vec![0u64; 50];
    let mut women_at = vec![0u64; 50];
    let (mut births, mut in_couple_births, mut age_sum) = (0u64, 0u64, 0.0f64);
    let (mut women, mut women_in_couple) = (0u64, 0u64);
    for _ in 0..12 {
        to_close(&mut w);
        for b in &report(&w).births {
            let a = usize::from(b.mother_age / YEAR);
            assert!((15..=49).contains(&a), "a mother of {a}");
            births_at[a] += u64::from(b.weight);
            births += u64::from(b.weight);
            if b.mother_in_couple {
                in_couple_births += u64::from(b.weight);
            }
            age_sum += f64::from(b.weight) * (f64::from(b.mother_age) + 0.5) / 12.0;
        }
        let people = w.people();
        let couple = in_couple(&w);
        for (p, &wt) in live_weights(&w).iter().enumerate() {
            let a = usize::from(people.age[p] / YEAR);
            if wt > 0 && people.sex[p] == Sex::F && (15..=49).contains(&a) {
                women_at[a] += u64::from(wt);
                women += u64::from(wt);
                if couple[p] {
                    women_in_couple += u64::from(wt);
                }
            }
        }
    }
    let tfr: f64 = (15..=49)
        .map(|a| ratio(births_at[a], women_at[a]) * 12.0)
        .sum();
    assert!((tfr - 1.77).abs() <= 0.05, "total fertility rate {tfr:.3}");
    #[allow(clippy::cast_precision_loss)]
    let mean_age = age_sum / births as f64;
    assert!(
        (mean_age - 28.1).abs() <= 0.5,
        "mothers' mean age {mean_age:.2}"
    );
    let couple_births = ratio(in_couple_births, births);
    let couple_women = ratio(women_in_couple, women);
    assert!(
        couple_births > couple_women,
        "births to women in a couple {couple_births:.3}, their share of women {couple_women:.3}"
    );
}

/// **AC-DEM-07** `[sim]` Estates conserve money: on every month of 20 years at
/// 1:1000 the ledger's invariants and I-4 hold; at each estate, the
/// households' money falls by exactly the debts repaid plus the amount paid
/// to the government under 4109, and the banks' loans fall by the debts
/// repaid plus written off; AC-HH-03's identity holds for every household
/// with `hh_moved_to_date`.
///
/// Readings:
/// - The invariants and I-4 are the checks of every day (`to_close`).
/// - At each estate (the month's `EstateRecord`s, national amounts): money −
///   what the heir received = repaid + paid to the government; repaid +
///   written off = the debts, repaid = min(money, debts); without an heir
///   nothing goes to one. The month end's ledger agrees: the government
///   receives Σ paid to it under `transfer.estate` (4109) and households pay
///   it; the banks' loans to households fall (legs from banks to households
///   on loans) by Σ repaid under `loan` (7003) and Σ written off under
///   `write_off` (7004). The month end's legs between banks and households
///   on loans come from estates only (household debts do not change
///   otherwise in the World of M3 and M4).
/// - AC-HH-03's identity is checked month by month for every live
///   household.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 20-year run: cargo test --release --features demography"
)]
fn ac_dem_07() {
    let estate_code = code("transfer.estate");
    assert_eq!(estate_code, FlowCode(4109), "transfer.estate is 4109");
    let mut w = world_dem_default(1000);
    let mut before = budgets(&w);
    let mut estates = 0usize;
    for _ in 0..240 {
        let r = to_close(&mut w);
        let when = format!("{:?}", r.date);
        let rep = report(&w);
        let (mut to_gov, mut repaid, mut written) = (Bani::ZERO, Bani::ZERO, Bani::ZERO);
        for e in &rep.estates {
            assert_eq!(
                e.money - e.to_heir,
                e.repaid + e.to_government,
                "{when}: estate {}: the households' money falls by repaid + to the government",
                e.hh_key
            );
            assert_eq!(
                e.repaid + e.written_off,
                e.debts,
                "{when}: estate {}: the debts",
                e.hh_key
            );
            assert_eq!(
                e.repaid,
                e.money.min(e.debts),
                "{when}: estate {}: repaid",
                e.hh_key
            );
            assert!(e.to_government >= Bani::ZERO && e.to_heir >= Bani::ZERO);
            if e.heir.is_none() {
                assert_eq!(
                    e.to_heir,
                    Bani::ZERO,
                    "{when}: estate {}: no heir",
                    e.hh_key
                );
                assert_eq!(e.heir_weight, 0);
            }
            to_gov += e.to_government;
            repaid += e.repaid;
            written += e.written_off;
        }
        estates += rep.estates.len();
        assert_eq!(
            flow(&w, estate_code, Sector::Government),
            to_gov,
            "{when}: 4109 received"
        );
        assert_eq!(
            flow(&w, estate_code, Sector::Households),
            Bani::ZERO - to_gov,
            "{when}: 4109 paid"
        );
        let legs = legs_of_day(&w, &r);
        let net = |c: FlowCode| {
            legs_sum(
                &legs,
                c,
                Instrument::Loans,
                Sector::Banks,
                Sector::Households,
            ) - legs_sum(
                &legs,
                c,
                Instrument::Loans,
                Sector::Households,
                Sector::Banks,
            )
        };
        assert_eq!(
            net(FlowCode::LOAN),
            repaid,
            "{when}: the banks' loans fall by the debts repaid"
        );
        assert_eq!(
            net(FlowCode::WRITE_OFF),
            written,
            "{when}: and by the debts written off"
        );
        let after = budgets(&w);
        assert_moved_identity(&before, &after, &when);
        before = after;
    }
    assert!(estates > 0, "no estate in 20 years");
}

/// **AC-DEM-08** `[sim]` When a retired person dies leaving a retired partner
/// whose pension is below `survivor_share` of the deceased's, the partner's
/// pension at the next pay day is half the deceased's points × the point
/// value (to the ban); a partner not yet retired gets at least that at
/// retirement.
///
/// Readings: at 1:100 over four years. "Whose pension is below": the
/// partner's pension points are below half the deceased's. "Half the
/// deceased's points × the point value (to the ban)": social transfers'
/// rounding of points × the point value in force at that pay day, within one
/// ban of half the points, for cases where the amount is above the minimum
/// pension (below it the minimum is paid, by social transfers' rule). Deaths
/// in October to December are not followed to the next pay day (see the
/// readings at the top). "At least that at retirement": the partner's new
/// pension points are at least half the deceased's (less one millionth of
/// rounding). Each part needs at least one case.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 4-year run at 1:100: cargo test --release --features demography"
)]
fn ac_dem_08() {
    let mut w = world_dem_default(100);
    // Survivors to check at the next pay day, and at their retirement.
    let mut next_pay: Vec<(u64, u64)> = Vec::new();
    let mut at_retirement: BTreeMap<u64, u64> = BTreeMap::new();
    let (mut paid_cases, mut retired_cases) = (0, 0);
    for _ in 0..48 {
        let r = to_close(&mut w);
        let idx = person_index(&w);
        let people = w.people();
        let pv = law_money(&w, LeverId::TransferPensionPointValue);
        let min = law_money(&w, LeverId::TransferPensionMinimum);
        let rule = PensionRule {
            point_value: pv,
            minimum: min,
        };
        // Last month's survivors, at this pay day.
        for (q, deceased_points) in std::mem::take(&mut next_pay) {
            let Some(&i) = idx.get(&q) else { continue };
            if r.date.month == 12 || !people.alive[i] {
                continue;
            }
            let survivor = deceased_points / 2;
            let want = pension(survivor, &rule);
            if want <= min {
                continue;
            }
            let got = w.pay_slips().pension[i];
            assert!(
                (got - want).abs() <= Bani(1),
                "{:?}: survivor {q}: pension {got:?}, half the deceased's points pay {want:?}",
                r.date
            );
            paid_cases += 1;
        }
        let rep = report(&w);
        for t in &rep.retirements {
            if let Some(required) = at_retirement.remove(&t.person_key) {
                assert!(
                    t.pension_points + 1 >= required,
                    "{:?}: survivor {} retired on {} points, at least {required}",
                    r.date,
                    t.person_key,
                    t.pension_points
                );
                retired_cases += 1;
            }
        }
        for d in &rep.deaths {
            let (true, Some(q)) = (d.retired, d.partner_key) else {
                continue;
            };
            let Some(&i) = idx.get(&q) else { continue };
            if !people.alive[i] {
                continue;
            }
            let half = d.pension_points / 2;
            if people.activity[i] == Activity::Retired {
                if people.pension_points[i] < half {
                    next_pay.push((q, d.pension_points));
                }
            } else {
                at_retirement.insert(q, half);
            }
        }
    }
    assert!(
        paid_cases > 0,
        "no retired survivor followed to the next pay day"
    );
    assert!(retired_cases > 0, "no survivor retired in four years");
}

/// **AC-DEM-09** `[sim]` Standard retirement: at every monthly close no live
/// non-retired person has both an age at or above the age in force and the
/// minimum contribution; women's age in force is 741 months in December 2021
/// and 756 in January 2030, men's 780 throughout; an employee who retires is
/// off the employer's staff from that month and paid no wage after it.
///
/// Readings: at 1:1000 from December 2021 to the close of January 2030. The
/// minimum contribution is 15 years (180 months). "Off the employer's staff
/// from that month": at the close of the month of retirement the person has
/// no employer and every unit's staff is the weight of its live employees.
/// "Paid no wage after it": a zero gross wage on every later pay day read
/// (not December's, see the readings at the top).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "an 8-year run: cargo test --release --features demography"
)]
fn ac_dem_09() {
    let mut w = world_dem_default(1000);
    let mut retired_employees: BTreeSet<u64> = BTreeSet::new();
    let (mut saw_dec_2021, mut saw_jan_2030) = (false, false);
    loop {
        let r = to_close(&mut w);
        let when = r.date;
        let f = w.retirement_age_in_force(Sex::F);
        let m = w.retirement_age_in_force(Sex::M);
        assert_eq!(m, 780, "{when:?}: men's age in force");
        if (when.year, when.month) == (2021, 12) {
            assert_eq!(f, 741, "women's age in force in December 2021");
            saw_dec_2021 = true;
        }
        if (when.year, when.month) == (2030, 1) {
            assert_eq!(f, 756, "women's age in force in January 2030");
            saw_jan_2030 = true;
        }
        let people = w.people();
        for (p, &wt) in live_weights(&w).iter().enumerate() {
            if wt == 0 || people.activity[p] == Activity::Retired {
                continue;
            }
            let age_in_force = if people.sex[p] == Sex::F { f } else { m };
            assert!(
                !(people.age[p] >= age_in_force && people.contribution_months[p] >= 180),
                "{when:?}: person {} of {} months with {} months of contributions is not retired",
                people.person_key[p],
                people.age[p],
                people.contribution_months[p]
            );
        }
        let idx = person_index(&w);
        for t in &report(&w).retirements {
            if t.employer_before != NO_EMPLOYER {
                let i = idx[&t.person_key];
                assert_eq!(
                    w.persons().employer[i],
                    NO_EMPLOYER,
                    "{when:?}: retiree {} still employed",
                    t.person_key
                );
                retired_employees.insert(t.person_key);
            }
        }
        assert_eq!(
            live_staff(&w),
            w.firm_units().staff,
            "{when:?}: staff of live employees"
        );
        if when.month != 12 {
            let slips = w.pay_slips();
            for k in &retired_employees {
                if let Some(&i) = idx.get(k)
                    && i < slips.gross_wage.len()
                    && !report(&w).retirements.iter().any(|t| t.person_key == *k)
                {
                    assert_eq!(
                        slips.gross_wage[i],
                        Bani::ZERO,
                        "{when:?}: retiree {k} paid a wage"
                    );
                }
            }
        }
        if saw_jan_2030 {
            break;
        }
    }
    assert!(saw_dec_2021 && saw_jan_2030);
    assert!(
        !retired_employees.is_empty(),
        "no employee retired in eight years"
    );
}

/// **AC-DEM-11** `[sim]` A newborn is in the mother's household with age 0,
/// role child and the household's weight; the household's child allowance at
/// the next pay day rises by the under-2 amount; a woman gives birth at most
/// once in 12 months.
///
/// Readings: at 1:100 over three years. The allowance is compared between
/// the pay day of the birth's month (before the birth) and the next one, for
/// households with one birth that month, the same other live members at both
/// closes and no other child turning 2 or 18 in between (otherwise their
/// change adds to the newborn's); births of November and December are not
/// followed (see the readings at the top); there must be such cases. "At
/// most once in 12 months": a mother's births are at least 12 month ends
/// apart, and at her birth's close her `months_since_birth` is 0.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 3-year run at 1:100: cargo test --release --features demography"
)]
fn ac_dem_11() {
    let mut w = world_dem_default(100);
    let mut last_birth: BTreeMap<u64, u32> = BTreeMap::new();
    // Households to follow: key → (allowance before, other members' keys).
    let mut follow: BTreeMap<u64, (Bani, BTreeSet<u64>, u64)> = BTreeMap::new();
    let mut checked = 0;
    for m in 0..36u32 {
        let r = to_close(&mut w);
        let when = r.date;
        let hidx = household_index(&w);
        let pidx = person_index(&w);
        let people = w.people();
        let list = members(&w);
        let under2 = law_money(&w, LeverId::TransferChildUnder2);
        // Last month's births, at this pay day.
        for (hk, (before, others, child)) in std::mem::take(&mut follow) {
            if when.month == 12 {
                continue;
            }
            let Some(&h) = hidx.get(&hk) else { continue };
            let now: BTreeSet<u64> = list[h].iter().map(|&p| people.person_key[p]).collect();
            let mut want = others.clone();
            want.insert(child);
            if now != want {
                continue;
            }
            let after = w.pay_slips().child_allowance[h];
            assert_eq!(
                after - before,
                under2,
                "{when:?}: household {hk}: a newborn's allowance"
            );
            checked += 1;
        }
        let births = &report(&w).births;
        let mut per_household: BTreeMap<u64, u32> = BTreeMap::new();
        for b in births {
            *per_household.entry(b.hh_key).or_insert(0) += 1;
        }
        for b in births {
            let c = pidx[&b.child_key];
            let mo = pidx[&b.mother_key];
            let h = w.persons().household_id[c] as usize;
            assert_eq!(
                h,
                w.persons().household_id[mo] as usize,
                "{when:?}: in the mother's household"
            );
            assert_eq!(w.household_life().hh_key[h], b.hh_key);
            assert_eq!(people.age[c], 0, "{when:?}: age 0");
            assert_eq!(people.role[c], Role::Child, "{when:?}: role child");
            assert_eq!(
                b.weight,
                w.households().hh_weight[h],
                "{when:?}: the household's weight"
            );
            assert_eq!(
                people.months_since_birth[mo],
                Some(0),
                "{when:?}: the mother's clock"
            );
            if let Some(&prev) = last_birth.get(&b.mother_key) {
                assert!(
                    m - prev >= 12,
                    "{when:?}: mother {} gave birth {} months apart",
                    b.mother_key,
                    m - prev
                );
            }
            last_birth.insert(b.mother_key, m);
            if per_household[&b.hh_key] == 1 && !matches!(when.month, 11 | 12) {
                let others: BTreeSet<u64> = list[h]
                    .iter()
                    .filter(|&&p| p != c)
                    .map(|&p| people.person_key[p])
                    .collect();
                let turning = list[h]
                    .iter()
                    .any(|&p| p != c && (people.age[p] == 24 || people.age[p] == 216));
                if !turning {
                    let before = w.pay_slips().child_allowance[h];
                    follow.insert(b.hh_key, (before, others, b.child_key));
                }
            }
        }
    }
    assert!(
        checked > 0,
        "no newborn's household followed to the next pay day"
    );
}

/// **AC-DEM-12** `[sim]` **The exit run**, at 1:1000 and 1:100: the population
/// falls in every decade; on 1 January 2070 it is within 7% of EUROPOP2023's
/// no-migration figure (15.75 million); the share aged 65+ rises from 19.5%
/// to between 25% and 31%, the share under 15 falls, the median age rises by
/// at least three years, and the old-age dependency ratio rises.
///
/// Readings: "falls in every decade": the population on 1 January of 2030,
/// 2040, 2050, 2060 and 2070 is each below the one before, the first below
/// the start (1 December 2021). "From 19.5%": the start's share is within
/// half a point of 19.5% (this test's reading). Every comparison is between
/// the start and 1 January 2070. The old-age dependency ratio is 65+ over
/// 15–64.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the 50-year exit run: cargo test --release --features demography"
)]
fn ac_dem_12() {
    for scale in [1000, 100] {
        let run = exit_run(scale);
        let s = &run.start;
        let mut last = s.population;
        for y in [2029, 2039, 2049, 2059, 2069] {
            let p = run.end_of(y).population;
            assert!(
                p < last,
                "1:{scale}: the population on 1 January {} ({p}) is not below {last}",
                y + 1
            );
            last = p;
        }
        let e = run.end_of(2069);
        let gap = ratio(e.population, 15_750_000) - 1.0;
        assert!(
            gap.abs() <= 0.07,
            "1:{scale}: 1 January 2070: {} against 15.75 million",
            e.population
        );
        let old0 = ratio(s.over65, s.population);
        let old1 = ratio(e.over65, e.population);
        assert!(
            (old0 - 0.195).abs() <= 0.005,
            "1:{scale}: 65+ at the start {old0:.4}"
        );
        assert!(
            (0.25..=0.31).contains(&old1),
            "1:{scale}: 65+ in 2070 {old1:.4}"
        );
        assert!(
            ratio(e.under15, e.population) < ratio(s.under15, s.population),
            "1:{scale}: the share under 15 falls"
        );
        assert!(
            e.median_age >= s.median_age + 36,
            "1:{scale}: median age {} → {} months",
            s.median_age,
            e.median_age
        );
        assert!(
            ratio(e.over65, e.aged15_64) > ratio(s.over65, s.aged15_64),
            "1:{scale}: the old-age dependency ratio rises"
        );
    }
}

/// **AC-DEM-13** `[sim]` Pension spending responds, in the exit run: the
/// weighted number of pensioners and the sum of their pension points in 2045
/// both exceed their December 2021 values by at least 5%.
///
/// Readings: at both scales of the exit run; "pensioners" are live persons
/// with pension points; the values are those after the December closes of
/// 2021 and 2045.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the 50-year exit run: cargo test --release --features demography"
)]
fn ac_dem_13() {
    for scale in [1000, 100] {
        let run = exit_run(scale);
        let (a, b) = (run.end_of(2021), run.end_of(2045));
        assert!(
            b.pensioners * 100 >= a.pensioners * 105,
            "1:{scale}: pensioners {} → {}",
            a.pensioners,
            b.pensioners
        );
        assert!(
            b.pension_points * 100 >= a.pension_points * 105,
            "1:{scale}: pension points {} → {}",
            a.pension_points,
            b.pension_points
        );
    }
}

/// Weighted live persons aged 60 to 67 in the labour force.
fn aged_60_67_active(w: &World) -> u64 {
    let people = w.people();
    live_weights(w)
        .iter()
        .enumerate()
        .filter(|&(p, &wt)| {
            wt > 0
                && (60 * YEAR..68 * YEAR).contains(&people.age[p])
                && in_labour_force(people.activity[p])
        })
        .map(|(_, &wt)| u64::from(wt))
        .sum()
}

/// **AC-DEM-14** `[sim]` The retirement-age lever: of two worlds, one with both
/// ages raised by 24 months from January 2023, after five years the raised
/// world has fewer pensioners (by at least 3% of the pensioners), a lower
/// pension bill, and more persons aged 60 to 67 employed or unemployed; the
/// value in force rose one month of age a month.
///
/// Readings: at 1:1000. The commands are given on 15 December 2022 on
/// `transfer.pension.retirement_age` for F and M (effective 1 January 2023),
/// each the age in force that day + 24 months. "After five years": at the
/// close of December 2027; the pension bill is 2027's (households' receipts
/// under `benefit.pension` on the year's pay days). "Rose one month of age
/// a month": from January 2023 each month's value is the last + 1 until it
/// reaches the commanded one, which it then keeps.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 6-year comparison: cargo test --release --features demography"
)]
fn ac_dem_14() {
    let lever = LeverId::TransferPensionRetirementAge;
    let mut base = world_dem_default(1000);
    let mut raised = world_dem_default(1000);
    let given = date(2022, 12, 15);
    run_until(&mut base, given);
    run_until(&mut raised, given);
    let mut target = [0u16; 2];
    for (s, sex, key) in [(0, Sex::F, "F"), (1, Sex::M, "M")] {
        let now = raised.retirement_age_in_force(sex);
        target[s] = now + 24;
        let k = key_of(&raised, lever, key);
        give(
            &mut raised,
            set(lever, Some(k), LeverValue::Number(i64::from(target[s]))),
        );
    }
    let mut last = [
        raised.retirement_age_in_force(Sex::F),
        raised.retirement_age_in_force(Sex::M),
    ];
    let (mut bill_base, mut bill_raised) = (Bani::ZERO, Bani::ZERO);
    let end = date(2028, 1, 1);
    while raised.date() < end {
        let r = to_close(&mut raised);
        to_close(&mut base);
        for (s, sex) in [(0, Sex::F), (1, Sex::M)] {
            let v = raised.retirement_age_in_force(sex);
            if r.date.year >= 2023 {
                let want = if last[s] < target[s] {
                    last[s] + 1
                } else {
                    target[s]
                };
                assert_eq!(
                    v, want,
                    "{:?}: {sex:?}'s age in force moves one month a month",
                    r.date
                );
            }
            last[s] = v;
        }
        if r.date.year == 2027 {
            bill_base += flow(&base, FlowCode::BENEFIT_PENSION, Sector::Households);
            bill_raised += flow(&raised, FlowCode::BENEFIT_PENSION, Sector::Households);
        }
    }
    assert_eq!(last, target, "the commanded ages are in force");
    let (b, r) = (snapshot(&base), snapshot(&raised));
    assert!(
        r.pensioners < b.pensioners && (b.pensioners - r.pensioners) * 100 >= b.pensioners * 3,
        "pensioners: raised {} against {}",
        r.pensioners,
        b.pensioners
    );
    assert!(
        bill_raised < bill_base,
        "the pension bill of 2027: {bill_raised:?} against {bill_base:?}"
    );
    let (ab, ar) = (aged_60_67_active(&base), aged_60_67_active(&raised));
    assert!(
        ar > ab,
        "persons aged 60 to 67 employed or unemployed: {ar} against {ab}"
    );
}

/// A month's statistics a compaction must not change.
fn month_numbers(w: &World) -> (u64, u64, u64) {
    let rep = report(w);
    (
        population(w),
        weight_of(&rep.births, |b| b.weight),
        weight_of(&rep.deaths, |d| d.weight),
    )
}

/// **AC-DEM-15** `[sim]` Determinism and scale: two 10-year runs with the same
/// seed have equal state hashes every month; a run compacted every month and
/// one compacted yearly have equal statistics; yearly births and deaths at
/// 1:1000 and 1:100 differ by less than 5% in each of the first 10 years.
///
/// Readings: at 1:1000 for the first two parts; "every month" is every
/// month end's state hash; "equal statistics" are every closed month's
/// `MonthStats` and the month's population, births and deaths (weighted);
/// "yearly" is run years of 12 month ends; "differ by less than 5%" is
/// |a − b| < 5% of the 1:100 count.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "four 10-year runs: cargo test --release --features demography"
)]
fn ac_dem_15() {
    let mut a = world_dem_default(1000);
    let mut b = world_dem_default(1000);
    let mut monthly = dem_params();
    monthly.demography.compact_monthly = true;
    let mut c = world_dem(1000, monthly, Vec::new());
    let mut d = world_dem_default(100);
    let mut years = [[0u64; 4]; 10];
    for m in 0..120usize {
        let ra = to_close(&mut a);
        let rb = to_close(&mut b);
        assert_eq!(
            ra.state_hash, rb.state_hash,
            "{:?}: the same seed, the same state",
            ra.date
        );
        to_close(&mut c);
        assert_eq!(
            month_numbers(&c),
            month_numbers(&a),
            "{:?}: compacted monthly",
            ra.date
        );
        to_close(&mut d);
        let ya = month_numbers(&a);
        let yd = month_numbers(&d);
        let y = &mut years[m / 12];
        y[0] += ya.1;
        y[1] += ya.2;
        y[2] += yd.1;
        y[3] += yd.2;
    }
    assert_eq!(
        a.months(),
        c.months(),
        "every month's statistics, compacted monthly"
    );
    for (k, y) in years.iter().enumerate() {
        for (what, x, z) in [("births", y[0], y[2]), ("deaths", y[1], y[3])] {
            assert!(
                x.abs_diff(z) * 100 < z * 5,
                "run year {}: {what} {x} at 1:1000 against {z} at 1:100",
                k + 1
            );
        }
    }
}

/// **AC-DEM-16** `[sim]` Performance: at 1:100 the People phase adds at most
/// 15% to the time of a simulated year (AC-WLD-10's budget), and the exit run
/// at 1:100 stays within the 50-year time budget of Spike 4.
///
/// Readings (the stricter of each, flagged for the owner):
/// - A year at 1:100 with the People phase takes at most 1.15 × the same year
///   without it (`DemographyParams::people_phase` off), and at most 2.3 s
///   (115% of AC-WLD-10's 2 s).
/// - Spike 4's budget is a 50-year run at 1:100 in about 30 s (roadmap,
///   spike table; ADR-0004). The roadmap's M2 item reads "about 70 s at 1.3 s
///   a year"; this test takes the spike's 30 s.
/// - Only stepping is timed, in a release build, as AC-WLD-10.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build time budget: cargo test --release --features demography"
)]
#[allow(clippy::disallowed_types)]
fn ac_dem_16() {
    let year = |p: WorldParams| {
        let mut w = world_dem(100, p, Vec::new());
        let started = std::time::Instant::now();
        for _ in 0..365 {
            step(&mut w);
        }
        let took = started.elapsed();
        assert_eq!(w.months().len(), 12, "every phase ran");
        took
    };
    let mut off = dem_params();
    off.demography.people_phase = false;
    let without = year(off);
    let with = year(dem_params());
    assert!(
        with.as_secs_f64() <= 1.15 * without.as_secs_f64(),
        "a year at 1:100: {with:?} with the People phase, {without:?} without"
    );
    assert!(
        with.as_millis() <= 2_300,
        "a year at 1:100 with the People phase: {with:?}"
    );
    let mut w = world_dem_default(100);
    let started = std::time::Instant::now();
    while w.months().len() < 600 {
        step(&mut w);
    }
    let took = started.elapsed();
    assert!(
        took.as_secs_f64() <= 30.0,
        "the 50-year exit run at 1:100 took {took:?}"
    );
}

/// Step, checking every day, until the next day to run is `last`; the
/// births of each month end on the way.
fn births_until(w: &mut World, last: Date) -> Vec<(Date, Vec<Birth>)> {
    let mut out = Vec::new();
    while w.date() < last {
        let r = step_checked(w);
        if r.date.is_month_end() {
            out.push((r.date, report(w).births.clone()));
        }
    }
    out
}

/// Weighted births of the month ends of some calendar years.
fn births_in(v: &[(Date, Vec<Birth>)], years: std::ops::RangeInclusive<i32>) -> u64 {
    v.iter()
        .filter(|(d, _)| years.contains(&d.year))
        .map(|(_, b)| weight_of(b, |x| x.weight))
        .sum()
}

/// Give the child-benefit rise of AC-DEM-18 on 15 December 2022 (effective
/// 1 January 2023): both allowances doubled, a 500-lei child tax credit.
fn raise_child_benefits(w: &mut World) {
    run_until(w, date(2022, 12, 15));
    let u2 = law_money(w, LeverId::TransferChildUnder2);
    let to18 = law_money(w, LeverId::TransferChildTo18);
    give(
        w,
        set(
            LeverId::TransferChildUnder2,
            None,
            LeverValue::Money(u2.times(2)),
        ),
    );
    give(
        w,
        set(
            LeverId::TransferChildTo18,
            None,
            LeverValue::Money(to18.times(2)),
        ),
    );
    give(
        w,
        set(
            LeverId::TaxIncomeCreditPerChild,
            None,
            LeverValue::Money(Bani::from_lei(500)),
        ),
    );
}

/// **AC-DEM-18** `[sim]` A large child-benefit rise: of two worlds at 1:100, one
/// with both child allowances doubled and a 500-lei child tax credit from 1
/// January 2023, and the economic terms switched off in both (so only the
/// benefit term differs), births before October 2023 are equal person for
/// person (the lag), and births in 2024–2025 are 1% to 6% higher in the
/// raised world; with every term on, they are still higher, by 0.5% to 7%.
///
/// Readings: "equal person for person": every month end through September
/// 2023 has the same births (child, mother, household, weight) in both
/// worlds. The rise is given on 15 December 2022 as three commands taking
/// effect on 1 January 2023. "Every term on" is a second pair of worlds at
/// the defaults.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "four 4-year runs at 1:100: cargo test --release --features demography"
)]
fn ac_dem_18() {
    let end = date(2026, 1, 1);
    for (terms_on, band) in [(false, (0.01, 0.06)), (true, (0.005, 0.07))] {
        let p = if terms_on {
            dem_params()
        } else {
            without_economic_terms(dem_params())
        };
        let mut base = world_dem(100, p, Vec::new());
        let mut raised = world_dem(100, p, Vec::new());
        let mut b = births_until(&mut base, date(2022, 12, 15));
        let mut r = births_until(&mut raised, date(2022, 12, 15));
        raise_child_benefits(&mut raised);
        b.extend(births_until(&mut base, end));
        r.extend(births_until(&mut raised, end));
        assert_eq!(b.len(), r.len());
        if !terms_on {
            for ((d, x), (_, y)) in b.iter().zip(&r) {
                if *d < date(2023, 10, 1) {
                    assert_eq!(
                        x, y,
                        "{d:?}: births before October 2023 are the same persons"
                    );
                }
            }
        }
        let (nb, nr) = (births_in(&b, 2024..=2025), births_in(&r, 2024..=2025));
        let rise = ratio(nr, nb) - 1.0;
        assert!(
            (band.0..=band.1).contains(&rise),
            "economic terms {}: births 2024–2025 rose {rise:.4} ({nr} against {nb})",
            if terms_on { "on" } else { "off" }
        );
    }
}

/// The unemployment rate of live persons aged 20 to 44.
fn unemployment_20_44(w: &World) -> f64 {
    let people = w.people();
    let (mut unemployed, mut force) = (0u64, 0u64);
    for (p, &wt) in live_weights(w).iter().enumerate() {
        if wt > 0
            && (20 * YEAR..45 * YEAR).contains(&people.age[p])
            && in_labour_force(people.activity[p])
        {
            force += u64::from(wt);
            if people.activity[p] == Activity::Unemployed {
                unemployed += u64::from(wt);
            }
        }
    }
    ratio(unemployed, force)
}

/// **AC-DEM-19** `[sim]` A recession lowers births slightly: of two worlds at
/// 1:100, one with a lasting layoff that raises the unemployment of persons
/// aged 20–44 by at least 4 points from January 2023, births in 2024 are
/// 0.2% to 3% lower in the recession world.
///
/// Readings: the lasting layoff is a 10% cut of every unit's labour
/// requirement from 1 January 2023 (`StartEdit::ScaleLabourRequirementFrom`),
/// which the labour market carries out by layoffs; the test checks that it
/// raises the unemployment rate of persons aged 20–44 (by activity) by at
/// least 4 points at every month end from January 2023 to December 2024.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "two 3-year runs at 1:100: cargo test --release --features demography"
)]
fn ac_dem_19() {
    let mut base = world_dem_default(100);
    let mut recession = world_dem(
        100,
        dem_params(),
        vec![StartEdit::ScaleLabourRequirementFrom {
            from: date(2023, 1, 1),
            factor_ppm: 900_000,
        }],
    );
    let (mut nb, mut nr) = (0u64, 0u64);
    while base.date() < date(2025, 1, 1) {
        let r = to_close(&mut base);
        to_close(&mut recession);
        if r.date.year >= 2023 {
            let (ub, ur) = (unemployment_20_44(&base), unemployment_20_44(&recession));
            assert!(
                ur - ub >= 0.04,
                "{:?}: unemployment at 20–44 {ur:.4} in the recession, {ub:.4} without",
                r.date
            );
        }
        if r.date.year == 2024 {
            nb += weight_of(&report(&base).births, |b| b.weight);
            nr += weight_of(&report(&recession).births, |b| b.weight);
        }
    }
    let fall = 1.0 - ratio(nr, nb);
    assert!(
        (0.002..=0.03).contains(&fall),
        "births in 2024 fell {fall:.4} ({nr} against {nb})"
    );
}

/// **AC-DEM-20** `[sim]` With the law at its 2021 values and the economic terms
/// switched off (`unemployed_fertility_ratio` 1, `fertility_cycle_response`
/// 0), the births of every month are those of the base rates exactly: the
/// same targets and the same mothers as a run whose three multipliers are
/// fixed at 1.
///
/// Readings: at 1:1000 over five years; "the same targets" are the births'
/// alignment targets of every band (`birth_targets_micro`), "the same
/// mothers" the same births (child, mother, household, weight), at every
/// month end.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "two 5-year runs: cargo test --release --features demography"
)]
fn ac_dem_20() {
    let p = without_economic_terms(dem_params());
    let mut fixed = p;
    fixed.demography.fixed_fertility_multipliers = true;
    let mut a = world_dem(1000, p, Vec::new());
    let mut b = world_dem(1000, fixed, Vec::new());
    let mut births = 0usize;
    for _ in 0..60 {
        let r = to_close(&mut a);
        to_close(&mut b);
        let (x, y) = (report(&a), report(&b));
        assert_eq!(
            x.birth_targets_micro, y.birth_targets_micro,
            "{:?}: the same targets",
            r.date
        );
        assert_eq!(x.births, y.births, "{:?}: the same mothers", r.date);
        births += x.births.len();
    }
    assert!(births > 0, "births happened");
}
