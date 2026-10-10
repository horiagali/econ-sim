//! Simulation time (ADR-0017): one tick is one day on the real calendar.
//!
//! [`Day`] counts ticks from the scenario's start date; [`Calendar`] turns it
//! into a [`Date`] with integer arithmetic only. No time zones and no wall
//! clock (ADR-0006 rule 10).

/// One simulation tick: a day, counted from the scenario's start date (day 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Day(pub u32);

impl Day {
    /// The following day.
    #[must_use]
    pub fn next(self) -> Self {
        Day(self.0.checked_add(1).expect("Day overflow"))
    }
}

/// A calendar month, counted from the month of the scenario's start date
/// (month 0). The index of a monthly statistic; not a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Month(pub u32);

impl Month {
    /// The following month.
    #[must_use]
    pub fn next(self) -> Self {
        Month(self.0.checked_add(1).expect("Month overflow"))
    }
}

/// A date of the Gregorian calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// Year, for example 2021.
    pub year: i32,
    /// Month, 1 to 12.
    pub month: u8,
    /// Day of the month, 1 to 31.
    pub day: u8,
}

/// Years the calendar accepts. Wide enough for any scenario, narrow enough
/// that day counts stay far inside 64 bits.
const YEARS: std::ops::RangeInclusive<i32> = 1600..=9999;

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn month_len(year: i32, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 31,
    }
}

impl Date {
    /// The date, or `None` if it does not exist (30 February, month 13, a
    /// year outside 1600 to 9999).
    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        let exists = YEARS.contains(&year)
            && (1..=12).contains(&month)
            && day >= 1
            && day <= month_len(year, month);
        exists.then_some(Date { year, month, day })
    }

    /// Number of days of this date's month.
    #[must_use]
    pub fn days_in_month(self) -> u8 {
        month_len(self.year, self.month)
    }

    /// The date is the last day of its month.
    #[must_use]
    pub fn is_month_end(self) -> bool {
        self.day == self.days_in_month()
    }

    /// Day of the week: 0 is Monday, 6 is Sunday.
    #[must_use]
    // An ordinal of a date in range is far from the ends of i64.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn weekday(self) -> u8 {
        // 1 January 1970 (ordinal 0) was a Thursday.
        u8::try_from((self.ordinal() + 3).rem_euclid(7)).expect("a remainder of 7")
    }

    /// Days since 1 January 1970 (negative before it).
    // The fields are bounded by `Date::new`, so nothing here can overflow i64.
    #[allow(clippy::arithmetic_side_effects)]
    fn ordinal(self) -> i64 {
        // Count from 1 March, so that the leap day is the last day of a year.
        let year = i64::from(self.year) - i64::from(self.month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year.rem_euclid(400);
        let month_from_march = (i64::from(self.month) + 9) % 12;
        let day_of_year = (153 * month_from_march + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// The date `ordinal` days after 1 January 1970, if its year is in range.
    // `ordinal` comes from a date in range plus a `u32`, so nothing here can overflow i64.
    #[allow(clippy::arithmetic_side_effects)]
    fn from_ordinal(ordinal: i64) -> Option<Self> {
        let z = ordinal + 719_468;
        let era = z.div_euclid(146_097);
        let day_of_era = z.rem_euclid(146_097);
        let year_of_era =
            (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_from_march = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
        let month = if month_from_march < 10 {
            month_from_march + 3
        } else {
            month_from_march - 9
        };
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Date::new(
            i32::try_from(year).ok()?,
            u8::try_from(month).ok()?,
            u8::try_from(day).ok()?,
        )
    }
}

/// The calendar of a scenario: its start date is day 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Calendar {
    start: Date,
}

impl Calendar {
    /// A calendar whose day 0 is `start`.
    #[must_use]
    pub fn new(start: Date) -> Self {
        Calendar { start }
    }

    /// The date of day 0.
    #[must_use]
    pub fn start(self) -> Date {
        self.start
    }

    /// The date of a day.
    ///
    /// # Panics
    /// If the date would be after the year 9999.
    #[must_use]
    pub fn date(self, day: Day) -> Date {
        // An ordinal of a date in range plus a u32 fits i64.
        #[allow(clippy::arithmetic_side_effects)]
        let ordinal = self.start.ordinal() + i64::from(day.0);
        Date::from_ordinal(ordinal).expect("date within the calendar's years")
    }

    /// The day of a date; `None` if it is before the start date or too far
    /// after it for a `u32`.
    #[must_use]
    pub fn day(self, date: Date) -> Option<Day> {
        // Both ordinals are of dates in range.
        #[allow(clippy::arithmetic_side_effects)]
        let days = date.ordinal() - self.start.ordinal();
        u32::try_from(days).ok().map(Day)
    }

    /// The calendar month a day falls in, counted from the start date's month.
    #[must_use]
    pub fn month(self, day: Day) -> Month {
        let date = self.date(day);
        // Years are bounded by `Date::new`.
        #[allow(clippy::arithmetic_side_effects)]
        let months = (i64::from(date.year) - i64::from(self.start.year)) * 12
            + i64::from(date.month)
            - i64::from(self.start.month);
        Month(u32::try_from(months).expect("a day is never before the start date"))
    }
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap()
    }

    #[test]
    fn dates_that_do_not_exist_are_refused() {
        assert!(Date::new(2021, 2, 29).is_none());
        assert!(Date::new(2024, 2, 29).is_some());
        assert!(Date::new(1900, 2, 29).is_none());
        assert!(Date::new(2000, 2, 29).is_some());
        assert!(Date::new(2021, 13, 1).is_none());
        assert!(Date::new(2021, 4, 31).is_none());
        assert!(Date::new(2021, 12, 0).is_none());
        assert!(Date::new(10_000, 1, 1).is_none());
    }

    #[test]
    fn the_romania_scenario_starts_on_a_wednesday() {
        // 1 December 2021, the census reference date.
        let cal = Calendar::new(date(2021, 12, 1));
        assert_eq!(cal.date(Day(0)), date(2021, 12, 1));
        assert_eq!(cal.date(Day(0)).weekday(), 2);
        assert_eq!(cal.date(Day(30)), date(2021, 12, 31));
        assert!(cal.date(Day(30)).is_month_end());
        assert_eq!(cal.date(Day(31)), date(2022, 1, 1));
        assert_eq!(cal.date(Day(31)).weekday(), 5); // a Saturday
        // 2024 is a leap year: 29 February exists, and 2024 has 366 days.
        assert_eq!(cal.day(date(2024, 2, 29)), Some(Day(820)));
        assert_eq!(cal.date(Day(820).next()), date(2024, 3, 1));
        // 50 years on: 18,262 days (twelve leap days: 2024, 2028, ..., 2068).
        assert_eq!(cal.day(date(2071, 12, 1)), Some(Day(18_262)));
        assert_eq!(cal.day(date(2021, 11, 30)), None);
    }

    #[test]
    fn months_count_from_the_start_month() {
        let cal = Calendar::new(date(2021, 12, 1));
        assert_eq!(cal.month(Day(0)), Month(0));
        assert_eq!(cal.month(Day(30)), Month(0));
        assert_eq!(cal.month(Day(31)), Month(1));
        assert_eq!(cal.month(cal.day(date(2022, 12, 15)).unwrap()), Month(12));
        assert_eq!(Month(12).next(), Month(13));
    }

    #[test]
    fn known_ordinals() {
        assert_eq!(date(1970, 1, 1).ordinal(), 0);
        assert_eq!(date(1970, 1, 1).weekday(), 3); // Thursday
        assert_eq!(date(2000, 3, 1).ordinal(), 11_017);
        assert_eq!(date(1969, 12, 31).ordinal(), -1);
    }

    proptest! {
        #[test]
        fn a_day_and_its_date_convert_both_ways(n in 0u32..200_000) {
            let cal = Calendar::new(date(2021, 12, 1));
            let d = cal.date(Day(n));
            prop_assert_eq!(cal.day(d), Some(Day(n)));
            prop_assert!(Date::new(d.year, d.month, d.day).is_some());
        }

        #[test]
        fn the_next_day_is_the_next_date(n in 0u32..200_000) {
            let cal = Calendar::new(date(1999, 12, 30));
            let (a, b) = (cal.date(Day(n)), cal.date(Day(n).next()));
            let rolls_over = a.is_month_end();
            prop_assert_eq!(b.day, if rolls_over { 1 } else { a.day + 1 });
            prop_assert_eq!(b.weekday(), (a.weekday() + 1) % 7);
            prop_assert!(b > a);
        }
    }
}
