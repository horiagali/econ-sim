//! Acceptance tests for the `[sim]` criteria of society/households-lifecycle
//! (AC-HHL-05 to 10) on the `World`, behind the feature `lifecycle`.
//! Written in a test-authoring session from the spec before the code existed
//! (ADR-0014), against the test writer's additions of the feature
//! `demography` (see `demography.rs`).
//!
//! The readings shared with demography's tests hold here too (2021 law,
//! every check of the day, live records only, release-only long runs). "A
//! household with a head or partner at least 15 years older" is the spec's
//! parent proxy: a person whose role is child or other, in a live private
//! household whose live head or partner is at least 180 months older.

use std::collections::{BTreeMap, BTreeSet};

use econ_core::world::World;
use econ_popgen::{Role, Sex};
use econ_types::{Bani, Day};

use super::support::step;
use super::support_dem::{
    ADULT, YEAR, assert_moved_identity, budgets, dem_params, exit_run, household_index, in_couple,
    live_weights, members, people, person_index, ratio, real_households, report, step_checked,
    to_close, world_dem_default,
};

/// Weighted live persons by sex (F 0, M 1).
fn by_sex(w: &World) -> [u64; 2] {
    let people = people(w);
    let mut out = [0u64; 2];
    for (p, &wt) in live_weights(w).iter().enumerate() {
        out[people.sex[p] as usize] += u64::from(wt);
    }
    out
}

/// Σ weight × a household balance column over live households.
fn weighted_live(w: &World, column: &[Bani]) -> i128 {
    let h = w.households();
    let life = w.household_life();
    (0..h.hh_weight.len())
        .filter(|&i| life.hh_alive[i])
        .map(|i| i128::from(h.hh_weight[i]) * i128::from(column[i].get()))
        .sum()
}

/// Σ weight × (mortgage + consumer debt) over live households.
fn debts(w: &World) -> i128 {
    let life = w.household_life();
    weighted_live(w, &life.hh_mortgage) + weighted_live(w, &life.hh_consumer_debt)
}

/// **AC-HHL-05** `[sim]` Conservation, every month of 20 years at 1:1000: the
/// life cycle's operations change neither the weighted persons nor Σ weight ×
/// any balance column (only ledger flows and estates do); every live private
/// household has exactly one head; no live private household lacks a member
/// aged 18 or more after the month's deaths; AC-HH-03's identity holds with
/// `hh_moved_to_date`.
///
/// Readings, month by month:
/// - Weighted persons: the live population of each sex changes by exactly
///   that sex's births − deaths (the life cycle adds and removes none).
/// - Cash and deposits: equal to the ledger's every day (I-4, the checks of
///   the day), so only ledger flows move their totals.
/// - The dwelling: Σ weight × `hh_dwelling_value` changes by exactly the
///   estates' (what heirs received − the dwellings of the dead).
/// - Debts: Σ weight × (mortgage + consumer debt) falls by exactly the
///   estates' repaid and written-off debts (household debts do not change
///   otherwise in the World of M3 and M4).
/// - One live head and a live adult in every live private household, at the
///   month's close.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 20-year run: cargo test --release --features lifecycle"
)]
fn ac_hhl_05() {
    let mut w = world_dem_default(1000);
    let mut sexes = by_sex(&w);
    let mut dwelling = weighted_live(&w, &w.household_life().hh_dwelling_value);
    let mut debt = debts(&w);
    let mut before = budgets(&w);
    for _ in 0..240 {
        let r = to_close(&mut w);
        let when = format!("{:?}", r.date);
        let rep = report(&w);
        let now = by_sex(&w);
        for (s, sex) in [(0, Sex::F), (1, Sex::M)] {
            let born: u64 = rep
                .births
                .iter()
                .filter(|b| b.sex == sex)
                .map(|b| u64::from(b.weight))
                .sum();
            let died: u64 = rep
                .deaths
                .iter()
                .filter(|d| d.sex == sex)
                .map(|d| u64::from(d.weight))
                .sum();
            assert_eq!(
                i128::from(now[s]),
                i128::from(sexes[s]) + i128::from(born) - i128::from(died),
                "{when}: weighted {sex:?} persons"
            );
        }
        sexes = now;
        let estates_dwelling: i128 = rep
            .estates
            .iter()
            .map(|e| i128::from(e.to_heir_dwelling.get()) - i128::from(e.dwelling.get()))
            .sum();
        let estates_debt: i128 = rep
            .estates
            .iter()
            .map(|e| i128::from(e.repaid.get()) + i128::from(e.written_off.get()))
            .sum();
        let d = weighted_live(&w, &w.household_life().hh_dwelling_value);
        assert_eq!(
            d - dwelling,
            estates_dwelling,
            "{when}: Σ weight × dwelling value"
        );
        dwelling = d;
        let k = debts(&w);
        assert_eq!(k - debt, -estates_debt, "{when}: Σ weight × debts");
        debt = k;
        let people = people(&w);
        let life = w.household_life();
        for (hh, list) in members(&w).iter().enumerate() {
            if !life.hh_alive[hh] || life.hh_collective[hh] {
                continue;
            }
            let heads = list
                .iter()
                .filter(|&&p| people.role[p] == Role::Head)
                .count();
            assert_eq!(
                heads,
                1,
                "{when}: household {} has {heads} heads",
                w.people().hh_key[hh]
            );
            assert!(
                list.iter().any(|&p| people.age[p] >= ADULT),
                "{when}: household {} has no member aged 18 or more",
                w.people().hh_key[hh]
            );
        }
        let after = budgets(&w);
        assert_moved_identity(&before, &after, &when);
        before = after;
    }
}

/// The weighted share of live persons aged 25–34 living with a head or
/// partner at least 15 years older (the parent proxy).
fn with_parents_25_34(w: &World) -> f64 {
    let people = people(w);
    let life = w.household_life();
    let weights = live_weights(w);
    let list = members(w);
    let (mut all, mut with) = (0u64, 0u64);
    for (p, &wt) in weights.iter().enumerate() {
        if wt == 0 || !(25 * YEAR..35 * YEAR).contains(&people.age[p]) {
            continue;
        }
        all += u64::from(wt);
        let hh = w.persons().household_id[p] as usize;
        if life.hh_collective[hh] || !matches!(people.role[p], Role::Child | Role::Other) {
            continue;
        }
        let older = list[hh].iter().any(|&q| {
            matches!(people.role[q], Role::Head | Role::Partner)
                && people.age[q] >= people.age[p] + 15 * YEAR
        });
        if older {
            with += u64::from(wt);
        }
    }
    ratio(with, all)
}

/// **AC-HHL-06** `[sim]` Leaving home, years 1 to 10 at 1:100: the share of
/// persons aged 25–34 living in a household with a head or partner at least
/// 15 years older stays between 30% and 48%; the mean age of leavers is 23
/// to 28 for women and 27 to 33 for men.
///
/// Readings: "stays" is at every month end of the ten years; the mean age of
/// leavers (exact ages, months + ½ over 12, weighted) is checked in each of
/// the ten run years, each of which must have leavers of both sexes.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 10-year run at 1:100: cargo test --release --features lifecycle"
)]
fn ac_hhl_06() {
    let mut w = world_dem_default(100);
    for year in 1..=10 {
        let mut sum = [0.0f64; 2];
        let mut n = [0u64; 2];
        for _ in 0..12 {
            let r = to_close(&mut w);
            let share = with_parents_25_34(&w);
            assert!(
                (0.30..=0.48).contains(&share),
                "{:?}: 25–34-year-olds living with parents {share:.4}",
                r.date
            );
            for l in &report(&w).life.leavers {
                let s = l.sex as usize;
                sum[s] += f64::from(l.weight) * (f64::from(l.age) + 0.5) / 12.0;
                n[s] += u64::from(l.weight);
            }
        }
        for (s, sex, band) in [(0, Sex::F, (23.0, 28.0)), (1, Sex::M, (27.0, 33.0))] {
            assert!(n[s] > 0, "year {year}: no {sex:?} leaver");
            #[allow(clippy::cast_precision_loss)]
            let mean = sum[s] / n[s] as f64;
            assert!(
                (band.0..=band.1).contains(&mean),
                "year {year}: the mean age of {sex:?} leavers is {mean:.2}"
            );
        }
    }
}

/// The share of live persons aged 30–49 living with a partner, and the
/// average size of live private households.
fn couples_and_size(w: &World) -> (f64, f64) {
    let people = people(w);
    let life = w.household_life();
    let weights = live_weights(w);
    let couple = in_couple(w);
    let (mut all, mut paired, mut in_private) = (0u64, 0u64, 0u64);
    for (p, &wt) in weights.iter().enumerate() {
        if wt == 0 {
            continue;
        }
        if !life.hh_collective[w.persons().household_id[p] as usize] {
            in_private += u64::from(wt);
        }
        if (30 * YEAR..50 * YEAR).contains(&people.age[p]) {
            all += u64::from(wt);
            if couple[p] {
                paired += u64::from(wt);
            }
        }
    }
    (ratio(paired, all), ratio(in_private, real_households(w).1))
}

/// **AC-HHL-07** `[sim]` Couples and sizes, at 1:100: after 20 years the share
/// of persons aged 30–49 living with a partner is within 8 points of its
/// start value; the average size of private households stays between its
/// start value − 0.4 and + 0.1.
///
/// Readings: "living with a partner" is head or partner of a live private
/// household with a head and a partner of the two sexes; "stays" is at every
/// month end of the 20 years; the start is day 0.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 20-year run at 1:100: cargo test --release --features lifecycle"
)]
fn ac_hhl_07() {
    let mut w = world_dem_default(100);
    let (couples0, size0) = couples_and_size(&w);
    let mut couples = couples0;
    for _ in 0..240 {
        let r = to_close(&mut w);
        let (c, size) = couples_and_size(&w);
        assert!(
            size >= size0 - 0.4 && size <= size0 + 0.1,
            "{:?}: average private household size {size:.3}, {size0:.3} at the start",
            r.date
        );
        couples = c;
    }
    assert!(
        (couples - couples0).abs() <= 0.08,
        "after 20 years, 30–49-year-olds with a partner {couples:.4}, {couples0:.4} at the start"
    );
}

/// The weight of the households eligible to separate (a head and a partner
/// of the two sexes, the woman under 60), and each one's children by key.
fn eligible_couples(w: &World) -> (u64, BTreeMap<u64, BTreeSet<u64>>) {
    let people = people(w);
    let life = w.household_life();
    let mut weight = 0;
    let mut children = BTreeMap::new();
    for (hh, list) in members(w).iter().enumerate() {
        if !life.hh_alive[hh] || life.hh_collective[hh] {
            continue;
        }
        let head = list.iter().find(|&&p| people.role[p] == Role::Head);
        let partner = list.iter().find(|&&p| people.role[p] == Role::Partner);
        let (Some(&h), Some(&q)) = (head, partner) else {
            continue;
        };
        if people.sex[h] == people.sex[q] {
            continue;
        }
        let woman = if people.sex[h] == Sex::F { h } else { q };
        if people.age[woman] >= 60 * YEAR {
            continue;
        }
        weight += u64::from(w.households().hh_weight[hh]);
        children.insert(
            w.people().hh_key[hh],
            list.iter()
                .filter(|&&p| people.role[p] == Role::Child)
                .map(|&p| people.person_key[p])
                .collect(),
        );
    }
    (weight, children)
}

/// **AC-HHL-08** `[sim]` Separations: each year's weighted separations are
/// within 10% of the target; in each, the man's new household holds
/// `separation_share` of the original per-household cash and deposits
/// (rounded down) and the children stay with the woman.
///
/// Readings: at 1:100 over ten run years. The target of a month is
/// `separation_rate` ÷ 12 × the weight of the eligible households on the
/// month end's eve (deaths, unions and ageing of that day move it by far
/// less than the band). "Holds `separation_share`": the separation's moved
/// cash and deposits are ⌊0.5 × the original's per-household balance⌋, the
/// new household's `hh_moved_to_date` is their sum, and the man is its only
/// member and head. "The children stay with the woman": the original
/// household is live with the woman as head, and holds every child it had
/// on the eve that is still alive.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a 10-year run at 1:100: cargo test --release --features lifecycle"
)]
fn ac_hhl_08() {
    let mut w = world_dem_default(100);
    let p = dem_params().demography.lifecycle;
    let share = i128::from(p.separation_share_ppm);
    for year in 1..=10 {
        let (mut target_micro, mut done) = (0u128, 0u64);
        let mut closes = 0;
        while closes < 12 {
            if !w.date().is_month_end() {
                step_checked(&mut w);
                continue;
            }
            let (eligible, children) = eligible_couples(&w);
            target_micro += u128::from(eligible) * u128::from(p.separation_rate_ppm) / 12;
            let r = step_checked(&mut w);
            closes += 1;
            let when = r.date;
            let hidx = household_index(&w);
            let pidx = person_index(&w);
            let people = people(&w);
            let life = w.household_life();
            let list = members(&w);
            for s in &report(&w).life.separations {
                done += u64::from(s.weight);
                let floor = |b: Bani| {
                    Bani(
                        i64::try_from(i128::from(b.get()).max(0) * share / 1_000_000)
                            .expect("fits"),
                    )
                };
                assert_eq!(
                    s.moved_cash,
                    floor(s.cash_before),
                    "{when:?}: separation of {}: cash",
                    s.hh_key
                );
                assert_eq!(
                    s.moved_deposits,
                    floor(s.deposits_before),
                    "{when:?}: separation of {}: deposits",
                    s.hh_key
                );
                let n = hidx[&s.new_hh];
                let man = pidx[&s.man_key];
                assert_eq!(
                    list[n],
                    vec![man],
                    "{when:?}: the man alone in his new household"
                );
                assert_eq!(people.role[man], Role::Head);
                assert_eq!(
                    life.hh_moved_to_date[n],
                    s.moved_cash + s.moved_deposits,
                    "{when:?}: what he took"
                );
                let o = hidx[&s.hh_key];
                assert!(
                    life.hh_alive[o],
                    "{when:?}: the original household lives on"
                );
                let woman = pidx[&s.woman_key];
                assert_eq!(w.persons().household_id[woman] as usize, o);
                assert_eq!(
                    people.role[woman],
                    Role::Head,
                    "{when:?}: the woman heads it"
                );
                for c in children.get(&s.hh_key).into_iter().flatten() {
                    if let Some(&ci) = pidx.get(c)
                        && people.alive[ci]
                    {
                        assert_eq!(
                            w.persons().household_id[ci] as usize,
                            o,
                            "{when:?}: child {c} stays with the woman"
                        );
                    }
                }
            }
        }
        assert!(done > 0, "year {year}: no separation");
        let target = target_micro as f64 / 1e6;
        #[allow(clippy::cast_precision_loss)]
        let gap = (done as f64 - target).abs() / target;
        assert!(
            gap <= 0.10,
            "year {year}: separations {done} against a target of {target:.0}"
        );
    }
}

/// **AC-HHL-09** `[sim]` Records do not explode: in the 50-year exit run of
/// demographics at 1:1000, after each yearly compaction, synthetic persons
/// per real person and synthetic households per real household stay within
/// 25% of their start values.
///
/// Readings: synthetic persons and households are the records in the tables
/// after the December close (its compaction), real ones their live weights;
/// the start is day 0.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the 50-year exit run: cargo test --release --features lifecycle"
)]
fn ac_hhl_09() {
    let run = exit_run(1000);
    #[allow(clippy::cast_precision_loss)]
    let per = |records: usize, real: u64| records as f64 / real as f64;
    let p0 = per(run.start.person_records, run.start.population);
    let h0 = per(run.start.household_records, run.start.real_households);
    assert_eq!(run.year_end.len(), 50);
    for s in &run.year_end {
        let p = per(s.person_records, s.population) / p0;
        let h = per(s.household_records, s.real_households) / h0;
        assert!(
            (0.75..=1.25).contains(&p),
            "{:?}: person records per person {p:.3} of the start",
            s.date
        );
        assert!(
            (0.75..=1.25).contains(&h),
            "{:?}: household records per household {h:.3} of the start",
            s.date
        );
    }
}

/// **AC-HHL-10** `[sim]` A household formed in a month has a stagger day from
/// 1 to 28 and makes its first spending decision within the following 28
/// days; two runs with the same seed form the same households (state hashes
/// equal).
///
/// Readings: at 1:1000 over three years. "Formed" is the month's
/// `households_formed`; "within the following 28 days": a decision on a day
/// after the month end and at most 28 days after it. "The same households":
/// the same keys formed every month, and the same state hash every day.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "two 3-year runs: cargo test --release --features lifecycle"
)]
fn ac_hhl_10() {
    let mut a = world_dem_default(1000);
    let mut b = world_dem_default(1000);
    // Households formed and not yet seen deciding: key → the day formed.
    let mut pending: BTreeMap<u64, Day> = BTreeMap::new();
    let mut formed = 0usize;
    let mut closes = 0;
    while closes < 36 {
        let ra = step_checked(&mut a);
        let rb = step(&mut b);
        assert_eq!(
            ra.state_hash, rb.state_hash,
            "{:?}: the same seed, the same state",
            ra.date
        );
        let hidx = household_index(&a);
        let h = a.households();
        let life = a.household_life();
        pending.retain(|k, f| {
            // A household that ended before deciding is no longer followed.
            let Some(&i) = hidx.get(k) else { return false };
            if !life.hh_alive[i] {
                return false;
            }
            match h.last_decision[i] {
                Some(d) if d > *f => {
                    assert!(
                        d.0 - f.0 <= 28,
                        "household {k} formed on day {} decided on day {}",
                        f.0,
                        d.0
                    );
                    false
                }
                _ => {
                    assert!(
                        ra.day.0 - f.0 < 28,
                        "household {k} formed on day {} has not decided by day {}",
                        f.0,
                        ra.day.0
                    );
                    true
                }
            }
        });
        if ra.date.is_month_end() {
            closes += 1;
            let (x, y) = (
                &report(&a).life.households_formed,
                &report(&b).life.households_formed,
            );
            assert_eq!(x, y, "{:?}: the same households formed", ra.date);
            for k in x {
                let i = hidx[k];
                let s = h.hh_stagger_day[i];
                assert!(
                    (1..=28).contains(&s),
                    "{:?}: household {k}: stagger day {s}",
                    ra.date
                );
                pending.insert(*k, ra.day);
                formed += 1;
            }
        }
    }
    assert!(formed > 0, "no household formed in three years");
}
