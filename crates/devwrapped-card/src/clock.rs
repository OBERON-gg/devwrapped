#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Clock {
    #[default]
    H24,
    H12,
}

impl Clock {
    /// formats the start of an hour, such as `15:00` or `3 p.m.`
    pub fn hour(self, hour: u8) -> String {
        let hour = hour % 24;
        match self {
            Clock::H24 => format!("{hour:02}:00"),
            Clock::H12 => {
                let half = if hour < 12 { "a.m." } else { "p.m." };
                let shown = match hour % 12 {
                    0 => 12,
                    rest => rest,
                };
                format!("{shown} {half}")
            }
        }
    }

    /// labels for the hours 0, 6, 12, and 18 under the hour bars
    pub(crate) fn axis(self) -> [&'static str; 4] {
        match self {
            Clock::H24 => ["00", "06", "12", "18"],
            Clock::H12 => ["midnight", "6 a.m.", "noon", "6 p.m."],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_hours_on_both_clocks() {
        let hours = |clock: Clock| [0, 5, 12, 15, 23].map(|hour| clock.hour(hour));
        assert_eq!(
            hours(Clock::H24),
            ["00:00", "05:00", "12:00", "15:00", "23:00"]
        );
        assert_eq!(
            hours(Clock::H12),
            ["12 a.m.", "5 a.m.", "12 p.m.", "3 p.m.", "11 p.m."]
        );
    }
}
