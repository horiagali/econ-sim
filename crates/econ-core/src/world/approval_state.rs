//! Approval on the `World` (feature `approval`; spec `society/opinion-approval`,
//! first version): each adult's approval from own finances, updated at each
//! monthly close after the poverty indicators, and the group means. Outside
//! the state hash: display only.

use econ_popgen::{Activity, Sex};
use econ_types::{Bani, Month};

use super::World;
use crate::approval::{ApprovalStats, GroupApproval, PersonInputs, next_reference, step, targets};

/// Approval's state, aligned with the persons and the households.
#[derive(Debug, Clone, Default)]
pub(crate) struct AppState {
    /// Per person: the smoothed parts 1 to 4 (`None` under 18), the five
    /// parts and `A_i`.
    smoothed: Vec<Option<[i32; 4]>>,
    parts: Vec<[i32; 5]>,
    approval: Vec<i32>,
    /// Per household: the reference real income (`None` before its first).
    reference: Vec<Option<Bani>>,
    stats: Vec<ApprovalStats>,
}

/// Age bands of the groups: 18–24, 25–34, …, 55–64, 65 or more.
fn age_band(age_months: u16) -> usize {
    match age_months / 12 {
        0..=24 => 0,
        25..=34 => 1,
        35..=44 => 2,
        45..=54 => 3,
        55..=64 => 4,
        _ => 5,
    }
}

/// A group's mean over (weight, parts) of its adults: the five parts' means
/// and `A` = 50 + their sum exactly.
pub(super) fn group_mean(members: &[(u64, [i32; 5])]) -> Option<GroupApproval> {
    let total: i128 = members.iter().map(|&(w, _)| i128::from(w)).sum();
    if total == 0 {
        return None;
    }
    let mut parts = [0i32; 5];
    for (k, part) in parts.iter_mut().enumerate() {
        let s: i128 = members
            .iter()
            .map(|&(w, p)| i128::from(w) * i128::from(p[k]))
            .sum();
        let m = if s >= 0 {
            (2 * s + total) / (2 * total)
        } else {
            -((-2 * s + total) / (2 * total))
        };
        *part = i32::try_from(m).expect("millipoints");
    }
    Some(GroupApproval {
        adults: u64::try_from(total).expect("adults"),
        mean_milli: 50_000 + parts.iter().sum::<i32>(),
        parts_milli: parts,
        // The mean is 50 plus the parts' means exactly, so the change is the
        // parts' changes and nothing is left for composition.
        composition_milli: 0,
    })
}

impl World {
    /// A person's approval and its five parts; `None` under 18.
    pub(crate) fn approval_of(&self, person: u32) -> Option<(i32, [i32; 5])> {
        let p = person as usize;
        self.app.smoothed.get(p).copied().flatten()?;
        Some((self.app.approval[p], self.app.parts[p]))
    }

    pub(crate) fn app_stats(&self) -> &[ApprovalStats] {
        &self.app.stats
    }

    /// The yearly compaction of the People phase.
    #[cfg(feature = "demography")]
    pub(super) fn app_compact(&mut self, keep_p: &[bool], keep_h: &[bool]) {
        use super::money::retain_by;
        let a = &mut self.app;
        retain_by(&mut a.smoothed, keep_p);
        retain_by(&mut a.parts, keep_p);
        retain_by(&mut a.approval, keep_p);
        retain_by(&mut a.reference, keep_h);
    }

    /// The monthly close, after the poverty indicators.
    #[allow(clippy::too_many_lines)]
    pub(super) fn approval_close(&mut self) {
        let Some(p) = self.params.approval else {
            return;
        };
        let n_p = self.pp.age.len();
        let n_hh = self.households.hh_weight.len();
        self.app.smoothed.resize(n_p, None);
        self.app.parts.resize(n_p, [0; 5]);
        self.app.approval.resize(n_p, 0);
        self.app.reference.resize(n_hh, None);
        let t = self.months.len() - 1;
        let inflation = self.months.last().map_or(0, |m| m.inflation_ppm);
        let recent: Vec<Bani> = self.hh_real_income_recent().to_vec();
        // Job seekers per household.
        let mut seekers = vec![0u8; n_hh];
        for q in 0..n_p {
            if self.person_resident(q) && self.pp.seeking[q] {
                let h = self.persons.household_id[q] as usize;
                seekers[h] = seekers[h].saturating_add(1);
            }
        }
        for q in 0..n_p {
            let h = self.persons.household_id[q] as usize;
            if !self.person_resident(q) || self.pp.age[q] < 18 * 12 || !self.household_alive(h) {
                self.app.smoothed[q] = None;
                continue;
            }
            let y = recent.get(h).copied().unwrap_or(Bani::ZERO);
            let r = self.app.reference[h].unwrap_or(y);
            let seeking = self.pp.seeking[q];
            let inputs = PersonInputs {
                recent: y,
                reference: r,
                seeking,
                others_seeking: seekers[h].saturating_sub(u8::from(seeking)),
                inflation_ppm: inflation,
                hardship: self.households.hh_hardship[h],
            };
            let (parts, a) = step(self.app.smoothed[q], targets(&inputs, &p), &p);
            self.app.smoothed[q] = Some([parts[0], parts[1], parts[2], parts[3]]);
            self.app.parts[q] = parts;
            self.app.approval[q] = a;
        }
        // The references move after the close (a new household starts at its
        // recent income).
        for h in 0..n_hh {
            let y = recent.get(h).copied().unwrap_or(Bani::ZERO);
            self.app.reference[h] = Some(match self.app.reference[h] {
                None => y,
                Some(r) => next_reference(r, y, &p),
            });
        }
        // Groups.
        let n_act = 7;
        let n_reg = (0..n_p)
            .map(|q| usize::from(self.pp.region[q]) + 1)
            .max()
            .unwrap_or(0);
        #[cfg(feature = "poverty")]
        let county_of = |w: &World, h: usize| usize::from(w.pov_county(h));
        let n_cty = (0..n_hh).map(|h| county_of(self, h) + 1).max().unwrap_or(0);
        let mut nat = Vec::new();
        let mut act = vec![Vec::new(); n_act];
        let mut fifth = vec![Vec::new(); 5];
        let mut reg = vec![Vec::new(); n_reg];
        let mut cty = vec![Vec::new(); n_cty];
        let mut age = vec![Vec::new(); 6];
        let mut edu = vec![Vec::new(); 5];
        let mut sex = vec![Vec::new(); 2];
        let activity = self.activity().to_vec();
        for q in 0..n_p {
            if self.app.smoothed[q].is_none() {
                continue;
            }
            let h = self.persons.household_id[q] as usize;
            let e = (u64::from(self.households.hh_weight[h]), self.app.parts[q]);
            nat.push(e);
            act[activity.get(q).copied().unwrap_or(Activity::InactiveOther) as usize].push(e);
            fifth[usize::from(self.households.hh_fifth[h]).min(4)].push(e);
            reg[usize::from(self.pp.region[q])].push(e);
            cty[county_of(self, h)].push(e);
            age[age_band(self.pp.age[q])].push(e);
            edu[(self.persons.edu_level[q] as usize).min(4)].push(e);
            sex[usize::from(self.people.sex[q] != Sex::F)].push(e);
        }
        let arr5 = |v: &[Vec<(u64, [i32; 5])>]| -> [Option<GroupApproval>; 5] {
            [
                group_mean(&v[0]),
                group_mean(&v[1]),
                group_mean(&v[2]),
                group_mean(&v[3]),
                group_mean(&v[4]),
            ]
        };
        self.app.stats.push(ApprovalStats {
            month: Month(u32::try_from(t).expect("a month")),
            national: group_mean(&nat),
            by_activity: act.iter().map(|v| group_mean(v)).collect(),
            by_fifth: arr5(&fifth),
            by_region: reg.iter().map(|v| group_mean(v)).collect(),
            by_county: cty.iter().map(|v| group_mean(v)).collect(),
            by_age: age.iter().map(|v| group_mean(v)).collect(),
            by_edu: arr5(&edu),
            by_sex: [group_mean(&sex[0]), group_mean(&sex[1])],
        });
    }
}
