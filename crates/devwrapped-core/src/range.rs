use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

/// `start` is inclusive and `end` is exclusive
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Timestamp,
    pub end: Timestamp,
}

impl Range {
    pub fn contains(&self, time: Timestamp) -> bool {
        self.start <= time && time < self.end
    }

    /// covers whole local days from `since` through `until`, or up to `now`, and is `None` when empty
    pub fn from_days(
        since: Date,
        until: Option<Date>,
        now: Timestamp,
        time_zone: &TimeZone,
    ) -> Option<Range> {
        let start = since.to_zoned(time_zone.clone()).ok()?.timestamp();
        let end = match until {
            Some(until) => until
                .tomorrow()
                .ok()?
                .to_zoned(time_zone.clone())
                .ok()?
                .timestamp(),
            None => now,
        };
        (start < end).then_some(Range { start, end })
    }

    /// returns the first and last local day, where the last day holds the moment before `end`
    pub fn days(&self, time_zone: &TimeZone) -> (Date, Date) {
        let last = self
            .end
            .checked_sub(SignedDuration::from_nanos(1))
            .unwrap_or(self.end);
        let date = |time: Timestamp| time.to_zoned(time_zone.clone()).date();
        (date(self.start), date(last.max(self.start)))
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    fn at(time: &str) -> Timestamp {
        time.parse().unwrap()
    }

    #[test]
    fn covers_whole_local_days() {
        let new_york = TimeZone::posix("EST5EDT,M3.2.0,M11.1.0").unwrap();
        let range = Range::from_days(
            date(2026, 3, 1),
            Some(date(2026, 3, 8)),
            at("2026-11-14T12:00:00Z"),
            &new_york,
        )
        .unwrap();

        assert_eq!(range.start, at("2026-03-01T05:00:00Z"));
        assert_eq!(range.end, at("2026-03-09T04:00:00Z"));
        assert!(range.contains(at("2026-03-09T03:59:59Z")));
        assert!(!range.contains(at("2026-03-09T04:00:00Z")));
    }

    #[test]
    fn ends_now_without_until() {
        let now = at("2026-11-14T12:00:00Z");
        let range = Range::from_days(date(2026, 1, 1), None, now, &TimeZone::UTC).unwrap();

        assert_eq!(range.start, at("2026-01-01T00:00:00Z"));
        assert_eq!(range.end, now);
    }

    #[test]
    fn finds_the_first_and_last_local_day() {
        let new_york = TimeZone::posix("EST5EDT,M3.2.0,M11.1.0").unwrap();
        let now = at("2026-11-14T03:00:00Z");
        let whole_days =
            Range::from_days(date(2026, 3, 1), Some(date(2026, 3, 8)), now, &new_york).unwrap();
        let to_now = Range::from_days(date(2026, 1, 1), None, now, &new_york).unwrap();

        assert_eq!(
            whole_days.days(&new_york),
            (date(2026, 3, 1), date(2026, 3, 8))
        );
        assert_eq!(
            to_now.days(&new_york),
            (date(2026, 1, 1), date(2026, 11, 13))
        );
    }

    #[test]
    fn rejects_empty_ranges() {
        let now = at("2026-11-14T12:00:00Z");
        let utc = TimeZone::UTC;

        assert!(Range::from_days(date(2026, 6, 2), Some(date(2026, 6, 1)), now, &utc).is_none());
        assert!(Range::from_days(date(2026, 12, 1), None, now, &utc).is_none());
        assert!(Range::from_days(date(2026, 6, 1), Some(date(2026, 6, 1)), now, &utc).is_some());
    }
}
