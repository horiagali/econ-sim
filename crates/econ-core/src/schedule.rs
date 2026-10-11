//! When each process runs (ADR-0017, section 2).
//!
//! A tick is one day. A process declares a [`Period`]; the [`Schedule`] says,
//! for a date, which processes are due and in which order. Staggered
//! processes run once a month for each household or firm unit, on that
//! record's own day of the month ([`stagger_day`], [`Cohorts`]).

use econ_rng::{KeyedRng, Stream};
use econ_types::Date;

/// Days of the month that staggered decisions are spread over: 1 to 28, so
/// that every record has its day in every month, February included.
pub const STAGGER_DAYS: u8 = 28;

/// How often a process runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    /// Every day.
    Daily,
    /// Once a month for each record, on the record's own day (1 to
    /// [`STAGGER_DAYS`]).
    Staggered,
    /// On a fixed day of each month; the last day if the month is shorter.
    Monthly(u8),
    /// On the last day of each month: pay day and the statistics close.
    MonthEnd,
    /// On the last day of March, June, September and December.
    Quarterly,
    /// On one date of each year; the last day of the month if that year's
    /// month is shorter (29 February).
    Yearly {
        /// Month, 1 to 12.
        month: u8,
        /// Day of the month, 1 to 31.
        day: u8,
    },
}

/// What a process has to do on a date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Due {
    /// Nothing.
    No,
    /// Run for everyone.
    All,
    /// Run for the records whose stagger day is this one (1 to
    /// [`STAGGER_DAYS`]).
    Cohort(u8),
}

impl Period {
    /// What a process of this period has to do on `date`.
    #[must_use]
    pub fn due(self, date: Date) -> Due {
        let on = |yes: bool| if yes { Due::All } else { Due::No };
        match self {
            Period::Daily => Due::All,
            Period::Staggered if date.day <= STAGGER_DAYS => Due::Cohort(date.day),
            Period::Staggered => Due::No,
            Period::Monthly(day) => on(date.day == day.clamp(1, date.days_in_month())),
            Period::MonthEnd => on(date.is_month_end()),
            Period::Quarterly => on(date.is_month_end() && date.month.is_multiple_of(3)),
            Period::Yearly { month, day } => {
                on(date.month == month && date.day == day.clamp(1, date.days_in_month()))
            }
        }
    }
}

/// Which table a stagger day is drawn for. Households and firm units have
/// separate id spaces, so the kind is part of the key of the draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Staggered {
    /// `hh_stagger_day`.
    Household = 0,
    /// `unit_stagger_day`.
    FirmUnit = 1,
}

/// The day of the month (1 to [`STAGGER_DAYS`]) on which a household or a
/// firm unit takes its monthly decisions. A pure function of the seed, the
/// kind and the id: fixed for the life of the record.
#[must_use]
pub fn stagger_day(seed: KeyedRng, kind: Staggered, id: u64) -> u8 {
    let draw = seed.fast_u64(Stream::Schedule, 0, id, kind as u32);
    // The remainder is below 28.
    #[allow(clippy::arithmetic_side_effects)]
    let day = draw % u64::from(STAGGER_DAYS) + 1;
    u8::try_from(day).expect("1 to 28")
}

/// The records of a table grouped by stagger day, each group in ascending
/// id, so that a staggered phase visits its day's records in a fixed order
/// without scanning the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cohorts {
    by_day: Vec<Vec<u32>>,
}

impl Cohorts {
    /// Group record ids `0..days.len()` by their stagger day.
    ///
    /// # Panics
    /// If a day is outside 1 to [`STAGGER_DAYS`], or there are more records
    /// than a `u32` counts.
    #[must_use]
    pub fn new(days: &[u8]) -> Self {
        let mut by_day = vec![Vec::new(); usize::from(STAGGER_DAYS)];
        for (id, &day) in days.iter().enumerate() {
            assert!(
                (1..=STAGGER_DAYS).contains(&day),
                "stagger day {day} of record {id} is outside 1 to {STAGGER_DAYS}"
            );
            by_day[usize::from(day.saturating_sub(1))].push(u32::try_from(id).expect("record id"));
        }
        Cohorts { by_day }
    }

    /// The records due for `due`: the day's cohort, nobody for [`Due::No`].
    /// [`Due::All`] has no cohort either: a process for everyone walks the
    /// table itself.
    #[must_use]
    pub fn due(&self, due: Due) -> &[u32] {
        match due {
            Due::Cohort(day) if (1..=STAGGER_DAYS).contains(&day) => {
                &self.by_day[usize::from(day.saturating_sub(1))]
            }
            _ => &[],
        }
    }

    /// Number of records over all days.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_day.iter().map(Vec::len).sum()
    }

    /// No records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_day.iter().all(Vec::is_empty)
    }
}

/// A process of the tick: its name and how often it runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phase {
    /// Stable name, used in reports and timings.
    pub name: &'static str,
    /// How often it runs.
    pub period: Period,
}

/// The processes of a tick in their fixed order (the tick order of the game
/// overview). The order of registration is the order of execution on every
/// day; a period only decides whether a process runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule {
    phases: Vec<Phase>,
}

impl Schedule {
    /// A schedule of `phases`, run in that order.
    ///
    /// # Panics
    /// If two phases have the same name.
    #[must_use]
    pub fn new(phases: Vec<Phase>) -> Self {
        for (i, phase) in phases.iter().enumerate() {
            assert!(
                phases[..i].iter().all(|p| p.name != phase.name),
                "phase {} is registered twice",
                phase.name
            );
        }
        Schedule { phases }
    }

    /// All phases, in order.
    #[must_use]
    pub fn phases(&self) -> &[Phase] {
        &self.phases
    }

    /// The phases that run on `date`, in order, each with what it has to do.
    pub fn due(&self, date: Date) -> impl Iterator<Item = (Phase, Due)> + '_ {
        self.phases.iter().filter_map(move |&phase| {
            let due = phase.period.due(date);
            (due != Due::No).then_some((phase, due))
        })
    }
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects)]
mod tests {
    use super::*;
    use econ_types::{Calendar, Day};

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap()
    }

    /// How often a period comes due over the days `0..days` from 1 December 2021.
    fn runs(period: Period, days: u32) -> Vec<(Date, Due)> {
        let calendar = Calendar::new(date(2021, 12, 1));
        (0..days)
            .map(|d| calendar.date(Day(d)))
            .map(|d| (d, period.due(d)))
            .filter(|(_, due)| *due != Due::No)
            .collect()
    }

    // 1 December 2021 to 30 November 2025: four years, one of them a leap year.
    const FOUR_YEARS: u32 = 1461;

    #[test]
    fn daily_runs_every_day() {
        assert_eq!(runs(Period::Daily, FOUR_YEARS).len(), 1461);
    }

    #[test]
    fn every_cohort_runs_exactly_once_in_every_month() {
        let all = runs(Period::Staggered, FOUR_YEARS);
        assert_eq!(all.len(), 48 * 28);
        for cohort in 1..=STAGGER_DAYS {
            let mine: Vec<Date> = all
                .iter()
                .filter(|(_, due)| *due == Due::Cohort(cohort))
                .map(|(d, _)| *d)
                .collect();
            assert_eq!(mine.len(), 48);
            assert!(mine.iter().all(|d| d.day == cohort));
        }
        assert_eq!(Period::Staggered.due(date(2022, 1, 29)), Due::No);
        assert_eq!(Period::Staggered.due(date(2022, 1, 31)), Due::No);
    }

    #[test]
    fn a_monthly_day_falls_back_to_the_last_day_of_a_short_month() {
        let all = runs(Period::Monthly(31), FOUR_YEARS);
        assert_eq!(all.len(), 48);
        assert!(all.iter().all(|(d, _)| d.is_month_end()));
        assert_eq!(Period::Monthly(30).due(date(2022, 2, 28)), Due::All);
        assert_eq!(Period::Monthly(30).due(date(2024, 2, 28)), Due::No);
        assert_eq!(Period::Monthly(30).due(date(2024, 2, 29)), Due::All);
        assert_eq!(Period::Monthly(30).due(date(2022, 3, 31)), Due::No);
        assert_eq!(runs(Period::Monthly(25), FOUR_YEARS).len(), 48);
    }

    #[test]
    fn month_end_quarter_and_year() {
        assert_eq!(runs(Period::MonthEnd, FOUR_YEARS).len(), 48);
        let quarters = runs(Period::Quarterly, FOUR_YEARS);
        assert_eq!(quarters.len(), 16);
        assert_eq!(quarters[0].0, date(2021, 12, 31));
        assert_eq!(quarters[1].0, date(2022, 3, 31));
        let januaries = runs(Period::Yearly { month: 1, day: 1 }, FOUR_YEARS);
        assert_eq!(januaries.len(), 4);
        assert_eq!(januaries[0].0, date(2022, 1, 1));
        let leap = runs(Period::Yearly { month: 2, day: 29 }, FOUR_YEARS);
        let days: Vec<u8> = leap.iter().map(|(d, _)| d.day).collect();
        assert_eq!(days, [28, 28, 29, 28]);
    }

    #[test]
    fn stagger_days_are_fixed_in_range_and_even() {
        let seed = KeyedRng::new(42);
        let n = 28_000u64;
        let mut count = [0u32; STAGGER_DAYS as usize];
        for id in 0..n {
            let day = stagger_day(seed, Staggered::Household, id);
            assert!((1..=STAGGER_DAYS).contains(&day));
            assert_eq!(day, stagger_day(seed, Staggered::Household, id));
            count[usize::from(day - 1)] += 1;
        }
        // 1,000 expected a day; the standard deviation is about 31.
        assert!(count.iter().all(|&c| (850..1150).contains(&c)), "{count:?}");
        // Households and firm units with the same id do not share their day.
        let same = (0..n)
            .filter(|&id| {
                stagger_day(seed, Staggered::Household, id)
                    == stagger_day(seed, Staggered::FirmUnit, id)
            })
            .count();
        assert!((700..1300).contains(&same), "{same}");
        // Another seed, another spread.
        let other = KeyedRng::new(43);
        assert!((0..n).any(|id| {
            stagger_day(seed, Staggered::Household, id)
                != stagger_day(other, Staggered::Household, id)
        }));
    }

    #[test]
    fn cohorts_hold_every_record_once_in_id_order() {
        let seed = KeyedRng::new(7);
        let days: Vec<u8> = (0..5_000)
            .map(|id| stagger_day(seed, Staggered::FirmUnit, id))
            .collect();
        let cohorts = Cohorts::new(&days);
        assert_eq!(cohorts.len(), 5_000);
        assert!(!cohorts.is_empty());
        let mut seen = vec![false; days.len()];
        for day in 1..=STAGGER_DAYS {
            let ids = cohorts.due(Due::Cohort(day));
            assert!(ids.windows(2).all(|w| w[0] < w[1]));
            for &id in ids {
                assert_eq!(days[id as usize], day);
                assert!(!seen[id as usize]);
                seen[id as usize] = true;
            }
        }
        assert!(seen.iter().all(|&s| s));
        assert!(cohorts.due(Due::No).is_empty());
        assert!(cohorts.due(Due::All).is_empty());
        assert!(Cohorts::new(&[]).is_empty());
    }

    #[test]
    #[should_panic(expected = "outside 1 to 28")]
    fn a_stagger_day_out_of_range_is_refused() {
        let _ = Cohorts::new(&[1, 29]);
    }

    #[test]
    fn the_schedule_keeps_its_order_and_skips_what_is_not_due() {
        let schedule = Schedule::new(vec![
            Phase {
                name: "production_plan",
                period: Period::Staggered,
            },
            Phase {
                name: "staffing",
                period: Period::Staggered,
            },
            Phase {
                name: "goods_market",
                period: Period::Daily,
            },
            Phase {
                name: "pay_day",
                period: Period::MonthEnd,
            },
            Phase {
                name: "wage_round",
                period: Period::Yearly { month: 1, day: 1 },
            },
            Phase {
                name: "statistics",
                period: Period::MonthEnd,
            },
        ]);
        let names = |d: Date| -> Vec<&str> { schedule.due(d).map(|(p, _)| p.name).collect() };
        assert_eq!(
            names(date(2022, 1, 1)),
            ["production_plan", "staffing", "goods_market", "wage_round"]
        );
        assert_eq!(names(date(2022, 1, 30)), ["goods_market"]);
        assert_eq!(
            names(date(2022, 1, 31)),
            ["goods_market", "pay_day", "statistics"]
        );
        let first: Vec<Due> = schedule
            .due(date(2022, 5, 17))
            .map(|(_, due)| due)
            .collect();
        assert_eq!(first, [Due::Cohort(17), Due::Cohort(17), Due::All]);
        assert_eq!(schedule.phases().len(), 6);
    }

    #[test]
    #[should_panic(expected = "registered twice")]
    fn a_phase_name_is_unique() {
        let _ = Schedule::new(vec![
            Phase {
                name: "a",
                period: Period::Daily,
            },
            Phase {
                name: "a",
                period: Period::MonthEnd,
            },
        ]);
    }
}
