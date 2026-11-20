use std::ffi::OsStr;

use devwrapped_card::{Clock, thousands};
use devwrapped_core::{AWARDS_SHOWN, Stats, Tally, awards};
use jiff::civil::Weekday;

use crate::Source;

const LABEL_WIDTH: usize = 19;

const BOLD: &str = "\x1b[1m";
const HEADLINE: &str = "\x1b[1;33m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

pub struct Report {
    pub stats: Stats,
    /// `None` for sample data
    pub run: Option<Run>,
}

pub struct Run {
    pub sources: Vec<(Source, Status)>,
    pub skipped_records: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Found,
    NotFound,
    Skipped,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Found => "found",
            Status::NotFound => "not found",
            Status::Skipped => "skipped",
        }
    }
}

/// follows the convention at no-color.org, where an empty value leaves color on
pub fn use_color(no_color: Option<&OsStr>, is_terminal: bool) -> bool {
    is_terminal && no_color.is_none_or(OsStr::is_empty)
}

pub fn render(report: &Report, color: bool, clock: Clock) -> String {
    let paint = |style: &str, text: &str| {
        if color {
            format!("{style}{text}{RESET}")
        } else {
            text.to_string()
        }
    };
    let heading = |name: &str| paint(BOLD, name) + "\n";
    let stats = &report.stats;
    let mut out = String::new();

    match &report.run {
        Some(_) => out += &format!("devwrapped  {} to {}\n\n", stats.since, stats.until),
        None => out += "devwrapped  sample data, no source was read\n\n",
    }
    if let Some(run) = &report.run {
        out += &heading("sources");
        for (source, status) in &run.sources {
            out += &row(source.name(), status.label());
        }
        out += "\n";
    }

    if let Some(git) = &stats.git {
        out += &heading("git");
        out += &row("commits", &thousands(git.commits));
        out += &row("active days", &thousands(git.active_days.into()));
        out += &row("longest streak", &days(git.longest_streak));
        if let Some(hour) = git.busiest_hour {
            out += &row("busiest hour", &clock.hour(hour));
        }
        if let Some(weekday) = git.busiest_weekday {
            out += &row("busiest weekday", weekday_name(weekday));
        }
        out += &row("repositories", &thousands(git.repositories.into()));
        if !git.top_repositories.is_empty() {
            out += &list("top repositories", &git.top_repositories);
        }
        out += &list("top languages", &git.top_languages);
        out += "\n";
    }

    if let Some(shell) = &stats.shell {
        out += &heading("shell");
        out += &row("commands", &thousands(shell.commands));
        out += &list("top commands", &shell.top_commands);
        out += &list("top subcommands", &shell.top_subcommands);
        if let Some(typo) = &shell.most_common_typo {
            out += &list("most common typo", std::slice::from_ref(typo));
        }
        out += &row("typos", &thousands(shell.typos));
        out += "\n";
    }

    if let Some(agents) = &stats.agents {
        out += &heading("agents");
        out += &list("sessions", &agents.sessions);
        out += &row("active days", &thousands(agents.active_days.into()));
        if let Some(hour) = agents.busiest_hour {
            out += &row("busiest hour", &clock.hour(hour));
        }
        out += &list("tool uses", &agents.tool_uses);
        out += &list("top models", &agents.top_models);
        out += &row("input tokens", &thousands(agents.tokens.input));
        out += &row("output tokens", &thousands(agents.tokens.output));
        out += &row("cache reads", &thousands(agents.tokens.cache_read));
        out += &row("cache writes", &thousands(agents.tokens.cache_write));
        out += &row("total tokens", &thousands(agents.tokens.total()));
        out += "\n";
    }

    out += &heading("awards");
    for (index, award) in awards(stats).into_iter().take(AWARDS_SHOWN).enumerate() {
        let name = if index == 0 {
            paint(HEADLINE, award.name())
        } else {
            award.name().to_string()
        };
        out += &format!("  {name}\n");
    }

    let notes = notes(report);
    if !notes.is_empty() {
        out += "\n";
        for note in notes {
            out += &format!("{}\n", paint(DIM, &note));
        }
    }
    out
}

fn notes(report: &Report) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(shell) = report
        .stats
        .shell
        .as_ref()
        .filter(|s| s.undated_commands > 0)
    {
        let undated = match shell.undated_commands {
            1 => "1 shell command has no date. It counts".to_string(),
            n => format!("{} shell commands have no date. They count", thousands(n)),
        };
        notes.push(format!(
            "note: {undated} for any range, but not for hours, weekdays, or awards."
        ));
    }
    let Some(run) = &report.run else {
        return notes;
    };
    if run.sources.contains(&(Source::Claude, Status::Found)) {
        notes.push(
            "note: Claude Code deletes logs after about 30 days \
             unless cleanupPeriodDays is raised in its settings."
                .to_string(),
        );
    }
    if run.skipped_records > 0 {
        notes.push(match run.skipped_records {
            1 => "note: 1 unreadable record was skipped.".to_string(),
            n => format!(
                "note: {} unreadable records were skipped.",
                thousands(n as u64)
            ),
        });
    }
    notes
}

fn row(label: &str, value: &str) -> String {
    format!("  {label:LABEL_WIDTH$}{value}\n")
}

fn list(label: &str, tallies: &[Tally]) -> String {
    if tallies.is_empty() {
        return row(label, "none");
    }
    // a crafted log name could otherwise send escape codes to the terminal
    let names: Vec<String> = tallies
        .iter()
        .map(|t| t.name.chars().filter(|c| !c.is_control()).collect())
        .collect();
    let name_width = names.iter().map(|name| name.chars().count()).max();
    let counts: Vec<String> = tallies.iter().map(|t| thousands(t.count)).collect();
    let count_width = counts.iter().map(String::len).max();
    let (name_width, count_width) = (name_width.unwrap_or(0), count_width.unwrap_or(0));
    let mut out = String::new();
    for (index, (name, count)) in names.iter().zip(&counts).enumerate() {
        let label = if index == 0 { label } else { "" };
        let value = format!("{name:name_width$}  {count:>count_width$}");
        out += &row(label, &value);
    }
    out
}

fn days(count: u32) -> String {
    match count {
        1 => "1 day".to_string(),
        count => format!("{} days", thousands(count.into())),
    }
}

fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
        Weekday::Sunday => "Sunday",
    }
}

#[cfg(test)]
mod tests {
    use devwrapped_core::{ShellStats, sample};

    use super::*;

    #[test]
    fn drops_control_characters_from_names() {
        let tallies = [Tally {
            name: "x\u{1b}]0;pwned\u{7}\u{1b}[31mRED".to_string(),
            count: 3,
        }];
        let out = list("tools", &tallies);
        assert!(!out.trim_end().chars().any(char::is_control), "{out:?}");
        assert!(out.contains("x]0;pwned[31mRED"), "{out:?}");
    }

    fn run(sources: Vec<(Source, Status)>) -> Run {
        Run {
            sources,
            skipped_records: 0,
        }
    }

    fn report_with(stats: Stats, run: Run) -> Report {
        Report {
            stats,
            run: Some(run),
        }
    }

    #[test]
    fn color_needs_a_terminal_and_no_no_color() {
        assert!(use_color(None, true));
        assert!(use_color(Some(OsStr::new("")), true));
        assert!(!use_color(Some(OsStr::new("1")), true));
        assert!(!use_color(None, false));
    }

    #[test]
    fn plain_output_has_no_escape_codes() {
        let report = Report {
            stats: sample(),
            run: None,
        };
        assert!(!render(&report, false, Clock::H24).contains('\x1b'));
        assert!(render(&report, true, Clock::H24).contains(HEADLINE));
    }

    #[test]
    fn lists_each_source_with_its_status() {
        let report = report_with(
            Stats::default(),
            run(vec![
                (Source::Git, Status::Found),
                (Source::Shell, Status::NotFound),
                (Source::Claude, Status::Skipped),
                (Source::Codex, Status::Found),
            ]),
        );
        let out = render(&report, false, Clock::H24);
        assert!(
            out.contains(
                "sources\n  git                found\n  shell              not found\n  \
                 claude             skipped\n  codex              found\n"
            ),
            "{out}"
        );
    }

    #[test]
    fn shows_hours_on_the_chosen_clock() {
        let report = Report {
            stats: sample(),
            run: None,
        };
        let twelve = render(&report, false, Clock::H12);
        assert!(
            twelve.contains("  busiest hour       10 p.m.\n"),
            "{twelve}"
        );
        assert!(twelve.contains("  busiest hour       3 p.m.\n"), "{twelve}");
        let twenty_four = render(&report, false, Clock::H24);
        assert!(
            twenty_four.contains("  busiest hour       22:00\n"),
            "{twenty_four}"
        );
    }

    #[test]
    fn aligns_ranked_lists() {
        let out = render(
            &Report {
                stats: sample(),
                run: None,
            },
            false,
            Clock::H24,
        );
        assert!(
            out.contains(
                "  sessions           Claude Code  142\n                     Codex CLI     37\n"
            ),
            "{out}"
        );
    }

    #[test]
    fn skips_sections_of_sources_without_data() {
        let out = render(
            &report_with(Stats::default(), run(Vec::new())),
            false,
            Clock::H24,
        );
        assert!(!out.contains("git\n"), "{out}");
        assert!(!out.contains("shell\n"), "{out}");
        assert!(!out.contains("agents\n"), "{out}");
        assert!(out.contains("awards\n  Steady Shipper\n"), "{out}");
    }

    #[test]
    fn notes_undated_shell_commands() {
        let stats = Stats {
            shell: Some(ShellStats {
                commands: 1500,
                undated_commands: 1200,
                ..ShellStats::default()
            }),
            ..Stats::default()
        };
        let out = render(&report_with(stats, run(Vec::new())), false, Clock::H24);
        assert!(
            out.contains("note: 1,200 shell commands have no date."),
            "{out}"
        );
        let one = Stats {
            shell: Some(ShellStats {
                commands: 1,
                undated_commands: 1,
                ..ShellStats::default()
            }),
            ..Stats::default()
        };
        let out = render(&report_with(one, run(Vec::new())), false, Clock::H24);
        assert!(
            out.contains("note: 1 shell command has no date. It counts for any range"),
            "{out}"
        );
    }

    #[test]
    fn notes_log_retention_only_when_claude_code_logs_are_found() {
        let note = "cleanupPeriodDays";
        let found = run(vec![(Source::Claude, Status::Found)]);
        let missing = run(vec![(Source::Claude, Status::NotFound)]);
        assert!(render(&report_with(Stats::default(), found), false, Clock::H24).contains(note));
        assert!(!render(&report_with(Stats::default(), missing), false, Clock::H24).contains(note));
    }

    #[test]
    fn notes_skipped_records() {
        let mut with_skips = run(Vec::new());
        with_skips.skipped_records = 1234;
        let out = render(
            &report_with(Stats::default(), with_skips),
            false,
            Clock::H24,
        );
        assert!(out.contains("note: 1,234 unreadable records"), "{out}");
        let mut one_skip = run(Vec::new());
        one_skip.skipped_records = 1;
        let out = render(&report_with(Stats::default(), one_skip), false, Clock::H24);
        assert!(
            out.contains("note: 1 unreadable record was skipped."),
            "{out}"
        );
    }
}
