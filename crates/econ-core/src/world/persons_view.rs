//! Person columns the M5 criteria read (features `migration`, `education`,
//! `approval`, `poverty`): each person's age and main activity, which the
//! `World` keeps but did not show.
//!
//! Test writer's addition (ADR-0014), written with those tests before the
//! code. The activity is Stage C's at load, then refreshed at each monthly
//! close: a job holder is employed, a job seeker unemployed, a pensioner
//! retired, anyone else keeps the starting activity (a newborn: a child).

use econ_popgen::Activity;

use super::World;

impl World {
    /// Each person's age in months, aligned with [`super::PersonColumns`].
    #[must_use]
    pub fn person_age_months(&self) -> &[u16] {
        &self.pp.age
    }

    /// Each person's main activity (Stage C's, kept by the `World`: a job
    /// seeker is `Unemployed`), aligned with [`super::PersonColumns`].
    #[must_use]
    pub fn activity(&self) -> &[Activity] {
        &self.act.now
    }

    /// Refresh every person's activity from jobs, job search and pensions.
    pub(super) fn refresh_activities(&mut self) {
        let n = self.pp.age.len();
        let mut now = Vec::with_capacity(n);
        for p in 0..n {
            let start = self.act.start.get(p).copied().unwrap_or(Activity::Child);
            let a = if self.persons.employer[p] != econ_popgen::NO_EMPLOYER
                || (2..=4).contains(&self.pp.status[p])
            {
                Activity::Employed
            } else if self.pp.seeking[p] {
                Activity::Unemployed
            } else if self.pp.pension[p] > econ_types::Bani::ZERO {
                Activity::Retired
            } else if matches!(start, Activity::Employed | Activity::Unemployed) {
                Activity::InactiveOther
            } else {
                start
            };
            now.push(a);
        }
        self.act.now = now;
    }

    /// Append the activity of a new person row (the last one): a copy of
    /// person `from`'s, or a child's.
    #[cfg(feature = "demography")]
    pub(super) fn act_push(&mut self, from: Option<usize>) {
        let n = self.pp.age.len();
        let a = |v: &[Activity]| {
            from.and_then(|p| v.get(p).copied())
                .unwrap_or(Activity::Child)
        };
        let (start, now) = (a(&self.act.start), a(&self.act.now));
        self.act.start.resize(n - 1, Activity::Child);
        self.act.now.resize(n - 1, Activity::Child);
        self.act.start.push(start);
        self.act.now.push(now);
    }

    /// Set the activity a person keeps when neither employed, seeking nor
    /// retired (education's leavers and pupils).
    #[cfg(feature = "education")]
    pub(super) fn set_base_activity(&mut self, p: usize, a: Activity) {
        if self.act.start.len() <= p {
            self.act.start.resize(p + 1, Activity::Child);
        }
        self.act.start[p] = a;
    }

    /// The yearly compaction of the People phase.
    #[cfg(feature = "demography")]
    pub(super) fn act_compact(&mut self, keep_p: &[bool]) {
        super::money::retain_by(&mut self.act.start, keep_p);
        super::money::retain_by(&mut self.act.now, keep_p);
    }
}

/// Each person's activity at load and now.
#[derive(Debug, Clone, Default)]
pub(crate) struct Activities {
    start: Vec<Activity>,
    now: Vec<Activity>,
}

impl Activities {
    /// From Stage C's codes.
    pub(crate) fn start(codes: &[u8]) -> Activities {
        let start: Vec<Activity> = codes
            .iter()
            .map(|&a| match a {
                0 => Activity::Child,
                1 => Activity::Pupil,
                2 => Activity::Student,
                3 => Activity::Employed,
                4 => Activity::Unemployed,
                5 => Activity::Retired,
                _ => Activity::InactiveOther,
            })
            .collect();
        Activities {
            now: start.clone(),
            start,
        }
    }
}
