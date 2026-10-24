//! Relative age of a session's last activity (FR-SESS-03, A-10, F-13). Pure: the caller passes the times.
//! The text is built in `messages::activity_age`.

/// FR-SESS-03, A-10: the age of an activity, rounded down to whole units. There is no weeks unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Age {
    /// Under 60 seconds.
    JustNow,
    /// Under 60 minutes; the count of minutes.
    Minutes(i64),
    /// Under 24 hours; the count of hours.
    Hours(i64),
    /// Otherwise; the count of days.
    Days(i64),
}

/// FR-SESS-03, A-10: the age of `activity` at `now`, both Unix seconds. An activity in the future (clock skew)
/// is `JustNow`.
pub fn age(now: i64, activity: i64) -> Age {
    let seconds = now.saturating_sub(activity).max(0);
    match seconds {
        s if s < 60 => Age::JustNow,
        s if s < 3_600 => Age::Minutes(s / 60),
        s if s < 86_400 => Age::Hours(s / 3_600),
        s => Age::Days(s / 86_400),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::{age, Age};

    const NOW: i64 = 1_000_000;

    // FR-SESS-03, A-10: under 60 seconds is "just now", 59 s included
    #[test]
    fn fifty_nine_seconds_is_just_now() {
        assert_eq!(age(NOW, NOW - 59), Age::JustNow);
    }

    // FR-SESS-03, A-10: 60 seconds is one minute
    #[test]
    fn sixty_seconds_is_one_minute() {
        assert_eq!(age(NOW, NOW - 60), Age::Minutes(1));
    }

    // FR-SESS-03, A-10: 90 seconds rounds down to 1m
    #[test]
    fn ninety_seconds_rounds_down_to_one_minute() {
        assert_eq!(age(NOW, NOW - 90), Age::Minutes(1));
    }

    // FR-SESS-03, A-10: 59 minutes 59 seconds is still minutes
    #[test]
    fn just_under_an_hour_is_minutes() {
        assert_eq!(age(NOW, NOW - 3_599), Age::Minutes(59));
    }

    // FR-SESS-03, A-10: 60 minutes is one hour, not 60m
    #[test]
    fn sixty_minutes_is_one_hour() {
        assert_eq!(age(NOW, NOW - 3_600), Age::Hours(1));
    }

    // FR-SESS-03, A-10: just under 24 hours is hours
    #[test]
    fn just_under_a_day_is_hours() {
        assert_eq!(age(NOW, NOW - 86_399), Age::Hours(23));
    }

    // FR-SESS-03, A-10: 24 hours is one day
    #[test]
    fn twenty_four_hours_is_one_day() {
        assert_eq!(age(NOW, NOW - 86_400), Age::Days(1));
    }

    // FR-SESS-03, A-10: no weeks unit; 15 days is "15d"
    #[test]
    fn long_ages_stay_in_days() {
        assert_eq!(age(NOW, NOW - 15 * 86_400), Age::Days(15));
    }

    // FR-SESS-03: an activity in the future (clock skew) reads as just now
    #[test]
    fn future_activity_is_just_now() {
        assert_eq!(age(NOW, NOW + 500), Age::JustNow);
    }

    // FR-SESS-03: an activity of 0 (never active) is a large day count, not a panic
    #[test]
    fn zero_activity_does_not_overflow() {
        assert_eq!(age(NOW, 0), Age::Days(11));
    }
}
