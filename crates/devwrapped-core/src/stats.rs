use jiff::Timestamp;
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;

use crate::agents::Tokens;

const TOP_COUNT: usize = 5;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    pub since: Date,
    /// the last local day of the range, inclusive
    pub until: Date,
    pub git: Option<GitStats>,
    pub shell: Option<ShellStats>,
    pub agents: Option<AgentStats>,
    pub activity: Activity,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitStats {
    pub commits: u64,
    pub active_days: u32,
    pub longest_streak: u32,
    pub busiest_hour: Option<u8>,
    pub busiest_weekday: Option<Weekday>,
    pub top_languages: Vec<Tally>,
    pub changed_files: u64,
    pub christmas_commits: u64,
    pub new_year_commits: u64,
    pub repositories: u32,
    /// stays empty unless the user asks to see repository names
    pub top_repositories: Vec<Tally>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShellStats {
    pub commands: u64,
    /// commands from history files without dates, which count no matter the range
    pub undated_commands: u64,
    pub top_commands: Vec<Tally>,
    pub top_subcommands: Vec<Tally>,
    pub most_common_typo: Option<Tally>,
    pub typos: u64,
    pub git_commands: u64,
    pub git_status_commands: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentStats {
    pub sessions: Vec<Tally>,
    pub active_days: u32,
    pub tool_uses: Vec<Tally>,
    /// every tool use, while `tool_uses` keeps only the top 5
    pub tool_use_total: u64,
    pub busiest_hour: Option<u8>,
    pub tokens: Tokens,
    pub top_models: Vec<Tally>,
}

/// commits, dated shell commands, and agent session starts by local hour and by weekday from Monday
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Activity {
    pub events: u64,
    pub by_hour: [u64; 24],
    pub by_weekday: [u64; 7],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tally {
    pub name: String,
    pub count: u64,
}

impl Activity {
    pub(crate) fn add(&mut self, local: &LocalTime) {
        self.events += 1;
        self.by_hour[local.hour] += 1;
        self.by_weekday[local.weekday] += 1;
    }
}

pub(crate) struct LocalTime {
    pub date: Date,
    pub hour: usize,
    pub weekday: usize,
}

pub(crate) fn local_time(time: Timestamp, time_zone: &TimeZone) -> LocalTime {
    let local = time.to_zoned(time_zone.clone());
    LocalTime {
        date: local.date(),
        hour: usize::from(local.hour().unsigned_abs()),
        weekday: usize::from(local.weekday().to_monday_zero_offset().unsigned_abs()),
    }
}

// ties go to the earliest slot so the result does not depend on order
pub(crate) fn busiest(counts: &[u64]) -> Option<usize> {
    let max = *counts.iter().max()?;
    if max == 0 {
        return None;
    }
    counts.iter().position(|&count| count == max)
}

pub(crate) fn busiest_hour(hours: &[u64; 24]) -> Option<u8> {
    busiest(hours).and_then(|hour| u8::try_from(hour).ok())
}

/// returns the 5 largest counts of at least `min_count`, with ties sorted by name
pub(crate) fn top_tallies<N: Into<String>>(
    counts: impl IntoIterator<Item = (N, u64)>,
    min_count: u64,
) -> Vec<Tally> {
    let mut tallies: Vec<Tally> = counts
        .into_iter()
        .filter(|(_, count)| *count >= min_count)
        .map(|(name, count)| Tally {
            name: name.into(),
            count,
        })
        .collect();
    tallies.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    tallies.truncate(TOP_COUNT);
    tallies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_top_5_with_a_minimum() {
        let tallies = top_tallies(
            [
                ("a", 1),
                ("b", 9),
                ("c", 3),
                ("d", 3),
                ("e", 4),
                ("f", 5),
                ("g", 6),
            ],
            3,
        );
        let names: Vec<&str> = tallies.iter().map(|tally| tally.name.as_str()).collect();
        assert_eq!(names, ["b", "g", "f", "e", "c"]);
    }

    #[test]
    fn finds_the_earliest_busiest_slot() {
        assert_eq!(busiest(&[0, 2, 5, 5]), Some(2));
        assert_eq!(busiest(&[0, 0]), None);
    }
}
