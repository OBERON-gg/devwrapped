use devwrapped_core::agents::Tokens;
use devwrapped_core::shell::typos;
use devwrapped_core::{
    AWARDS_SHOWN, Award, AwardMeasures, Holiday, Stats, Tally, award_measures, awards,
};

use crate::{Clock, compact, thousands, year_text};

pub(crate) const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const SUMMARY_ROWS: usize = 6;
const WORDS_PER_TOKEN: u64 = 3;
const TOKENS_PER_WORDS: u64 = 4;
const WORDS_PER_NOVEL: u64 = 90_000;

pub(crate) struct Page {
    pub name: &'static str,
    pub section: &'static str,
    pub content: Content,
}

pub(crate) struct Week {
    pub counts: [u64; 7],
    pub busiest: usize,
    pub busiest_line: String,
    pub quietest_line: String,
}

pub(crate) enum Content {
    Intro {
        year: String,
        lead: String,
        sections: Vec<(&'static str, &'static str)>,
    },
    Commits {
        commits: String,
        rate: String,
        line: &'static str,
        week: Option<Week>,
    },
    Streak {
        days: String,
        unit: &'static str,
        line: &'static str,
        length: u32,
    },
    Languages {
        headline: String,
        line: String,
        bars: Vec<(String, u64)>,
    },
    PeakHour {
        hour: String,
        axis: [&'static str; 4],
        line: &'static str,
        by_hour: [u64; 24],
        peak: usize,
    },
    Shell {
        commands: String,
        favorite: Option<String>,
        extra: Option<String>,
        rows: Vec<(String, String)>,
    },
    Typo {
        typo: String,
        note: String,
        footnote: String,
    },
    Agents {
        sessions: String,
        line: &'static str,
        split: String,
        model: Option<String>,
        tools: Vec<(String, String)>,
        models: Vec<(String, String)>,
    },
    Tokens {
        total: String,
        cache: String,
        novels: String,
        bars: Vec<(&'static str, String, u64)>,
    },
    Award {
        label: String,
        name: &'static str,
        reason: String,
        also: Option<String>,
    },
    Repositories {
        line: String,
        rows: Vec<(String, String)>,
    },
    Summary {
        year: String,
        award: &'static str,
        rows: Vec<(&'static str, String)>,
    },
}

pub(crate) fn story(stats: &Stats, clock: Clock) -> Vec<Page> {
    let year = year_text(stats);
    let mut pages = vec![intro(stats, &year)];
    if let Some(git) = stats.git.as_ref().filter(|git| git.commits > 0) {
        pages.push(page("commits", "git", commits(stats, git.commits)));
        pages.push(page("streak", "git", streak(git.longest_streak)));
        if let Some(top) = git.top_languages.first() {
            pages.push(page(
                "languages",
                "git",
                languages(top, &git.top_languages, git.changed_files),
            ));
        }
    }
    if stats.activity.events > 0 {
        pages.push(page(
            "peak-hour",
            "when",
            peak_hour(&stats.activity.by_hour, clock),
        ));
    }
    if let Some(shell) = stats.shell.as_ref().filter(|shell| shell.commands > 0) {
        pages.push(page("shell", "shell", shell_page(shell)));
        if let Some(typo) = &shell.most_common_typo {
            pages.push(page("typo", "shell", typo_page(typo, shell.typos)));
        }
    }
    let measures = award_measures(stats);
    if let Some(agents) = stats.agents.as_ref().filter(|_| measures.sessions > 0) {
        pages.push(page(
            "agents",
            "agents",
            agents_page(agents, measures.sessions),
        ));
        if agents.tokens.total() > 0 {
            pages.push(page("tokens", "agents", tokens_page(&agents.tokens)));
        }
    }
    let won = awards(stats);
    pages.push(page(
        "award",
        "you",
        award_page(stats, &won, &measures, &year),
    ));
    if let Some(git) = stats
        .git
        .as_ref()
        .filter(|git| !git.top_repositories.is_empty())
    {
        pages.push(page(
            "repositories",
            "git",
            repositories(&git.top_repositories, git.commits),
        ));
    }
    pages.push(page(
        "summary",
        "the summary",
        summary(stats, &won, year, clock),
    ));
    pages
}

fn page(name: &'static str, section: &'static str, content: Content) -> Page {
    Page {
        name,
        section,
        content,
    }
}

fn intro(stats: &Stats, year: &str) -> Page {
    let mut read = Vec::new();
    let mut sections = Vec::new();
    if let Some(git) = &stats.git {
        read.push("git log");
        if git.commits > 0 {
            sections.push(("git", "commits, streaks, languages"));
        }
    }
    if let Some(shell) = &stats.shell {
        read.push("shell history");
        if shell.commands > 0 {
            sections.push(("shell", "commands and typos"));
        }
    }
    if stats.agents.is_some() {
        read.push("agent logs");
        if award_measures(stats).sessions > 0 {
            sections.push(("agents", "sessions and tokens"));
        }
    }
    sections.push(("you", "the award"));
    let lead = if read.is_empty() {
        "We found no git, shell, or agent history to read.".to_string()
    } else {
        format!(
            "We read your {}.\nNone of it left your machine. Relax.",
            join_words(&read)
        )
    };
    page(
        "intro",
        "the year in review",
        Content::Intro {
            year: year.to_string(),
            lead,
            sections,
        },
    )
}

fn join_words(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [one] => one.to_string(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

fn commits(stats: &Stats, commits: u64) -> Content {
    let days = range_days(stats).max(1);
    let tenths_per_day = u128::from(commits) * 10 / u128::from(days);
    let rate = if tenths_per_day >= 10 {
        format!("{} a day", one_decimal(tenths_per_day))
    } else {
        let tenths_per_week = u128::from(commits) * 70 / u128::from(days);
        format!("{} a week", one_decimal(tenths_per_week))
    };
    let line = match tenths_per_day {
        0..5 => "Quality over quantity.\nThat’s the story we’re going with.",
        5..=30 => "Small commits. Big energy.",
        31..=100 => "Your keyboard has filed a complaint.",
        _ => "Do you commit in your sleep? Blink twice.",
    };
    Content::Commits {
        commits: compact(commits),
        rate,
        line,
        week: week(&stats.activity.by_weekday),
    }
}

fn range_days(stats: &Stats) -> u64 {
    stats
        .until
        .since(stats.since)
        .map_or(1, |span| span.get_days().unsigned_abs().into())
        + 1
}

fn one_decimal(tenths: u128) -> String {
    if tenths >= 100 {
        compact(u64::try_from(tenths / 10).unwrap_or(u64::MAX))
    } else {
        format!("{}.{}", tenths / 10, tenths % 10)
    }
}

fn week(counts: &[u64; 7]) -> Option<Week> {
    let busiest = position(counts, counts.iter().max())?;
    let quietest = position(counts, counts.iter().min())?;
    if counts[busiest] == 0 || busiest == quietest {
        return None;
    }
    Some(Week {
        counts: *counts,
        busiest,
        busiest_line: format!("{}s carried the team.", WEEKDAYS[busiest]),
        quietest_line: format!("{}s, you touched grass.", WEEKDAYS[quietest]),
    })
}

fn position(counts: &[u64], value: Option<&u64>) -> Option<usize> {
    let value = value?;
    counts.iter().position(|count| count == value)
}

fn streak(days: u32) -> Content {
    let line = match days {
        0..=2 => "Commitment is a strong word.",
        3..=6 => "A solid little run. Then life happened.",
        7..=13 => "Over a week straight.\nYour contribution graph is blushing.",
        _ => "The streak is your personality now.",
    };
    Content::Streak {
        days: compact(days.into()),
        unit: if days == 1 {
            "day in a row."
        } else {
            "days in a row."
        },
        line,
        length: days,
    }
}

fn percent(part: u64, whole: u64) -> u64 {
    if whole == 0 {
        0
    } else {
        (u128::from(part) * 100 / u128::from(whole)).min(100) as u64
    }
}

fn languages(top: &Tally, all: &[Tally], changed_files: u64) -> Content {
    let line = if all.len() == 1 {
        format!("Just you and {}. Cute.", top.name)
    } else {
        format!(
            "{}% of everything you touched.\nThe other languages are jealous.",
            percent(top.count, changed_files)
        )
    };
    Content::Languages {
        headline: format!("{} ran your year.", top.name),
        line,
        bars: all
            .iter()
            .take(5)
            .map(|tally| (tally.name.clone(), percent(tally.count, changed_files)))
            .collect(),
    }
}

fn peak_hour(by_hour: &[u64; 24], clock: Clock) -> Content {
    let peak = position(by_hour, by_hour.iter().max()).unwrap_or(0);
    let line = match peak {
        0..=4 => "The bugs come out at night.\nSo, apparently, do you.",
        5..=7 => "Up before the coffee.\nImpressive. Slightly concerning.",
        8..=11 => "A morning person.\nWe don’t trust you.",
        12..=16 => "Peak brain, right after lunch.\nThe sandwich is working.",
        17..=20 => "Clocked out?\nYour commits say otherwise.",
        _ => "Prime time is code time.",
    };
    Content::PeakHour {
        hour: clock.hour(peak as u8),
        axis: clock.axis(),
        line,
        by_hour: *by_hour,
        peak,
    }
}

fn shell_page(shell: &devwrapped_core::ShellStats) -> Content {
    let top = shell.top_commands.first();
    let favorite = top.map(|top| {
        format!(
            "{}, {} times. Your ride or die.",
            top.name,
            thousands(top.count)
        )
    });
    let extra = top.map(|top| {
        let status = shell
            .top_subcommands
            .first()
            .filter(|sub| sub.name == "git status");
        match (top.name.to_ascii_lowercase().as_str(), status) {
            ("git", Some(status)) => format!(
                "And git status {} times.\nIt didn’t change. We checked.",
                thousands(status.count)
            ),
            ("cargo", _) => "The borrow checker sends its regards.".to_string(),
            ("cd", _) => "You really get around.".to_string(),
            ("ls", _) => "Still looking for that file?".to_string(),
            ("vim" | "nvim", _) => "And you found the exit every time.".to_string(),
            ("npm" | "pnpm" | "yarn", _) => "node_modules grew another inch.".to_string(),
            ("docker" | "podman", _) => "It works on your machine. Literally.".to_string(),
            ("kubectl", _) => "YAML is your love language.".to_string(),
            ("python" | "python3", _) => "Indentation is a lifestyle.".to_string(),
            _ => "A classic.".to_string(),
        }
    });
    Content::Shell {
        commands: compact(shell.commands),
        favorite,
        extra,
        rows: rows(&shell.top_subcommands),
    }
}

fn rows(tallies: &[Tally]) -> Vec<(String, String)> {
    tallies
        .iter()
        .take(5)
        .map(|tally| (tally.name.clone(), compact(tally.count)))
        .collect()
}

fn typo_page(typo: &Tally, typos: u64) -> Content {
    let times = format!("{} times.", thousands(typo.count));
    let note = match typos::correction(&typo.name) {
        Some(fix) => format!("you meant {fix}. {times}"),
        None => times,
    };
    Content::Typo {
        typo: typo.name.clone(),
        note,
        footnote: format!(
            "{} typos in all, and your shell never said a word.\nA saint.",
            thousands(typos)
        ),
    }
}

fn agents_page(agents: &devwrapped_core::AgentStats, sessions: u64) -> Content {
    let line = match sessions {
        0..10 => "Just testing the waters.",
        10..100 => "You two are getting close.",
        _ => "At this point it should come to standup.",
    };
    let mut split: Vec<String> = agents
        .sessions
        .iter()
        .map(|tally| format!("{} {}", tally.name, thousands(tally.count)))
        .collect();
    match agents.active_days {
        0 => {}
        1 => split.push("1 day together".to_string()),
        days => split.push(format!("{} days together", thousands(days.into()))),
    }
    Content::Agents {
        sessions: compact(sessions),
        line,
        split: split.join(" · "),
        model: agents
            .top_models
            .first()
            .map(|model| format!("Mostly {}. You have a type.", model.name)),
        tools: rows(&agents.tool_uses),
        models: rows(&agents.top_models).into_iter().take(3).collect(),
    }
}

fn tokens_page(tokens: &Tokens) -> Content {
    let total = tokens.total();
    let cache = if tokens.cache_read > 0 {
        format!(
            "{} were cache reads.\nYour AI has a great memory.",
            compact(tokens.cache_read)
        )
    } else {
        "All fresh tokens. No reruns.".to_string()
    };
    let novels = match total / TOKENS_PER_WORDS * WORDS_PER_TOKEN / WORDS_PER_NOVEL {
        0 => "Not quite a novel yet.".to_string(),
        1 => "About 1 novel’s worth, with no plot.".to_string(),
        count => format!(
            "About {} novels’ worth, none with a plot.",
            thousands(count)
        ),
    };
    let bars = [
        ("input", tokens.input),
        ("output", tokens.output),
        ("cache reads", tokens.cache_read),
        ("cache writes", tokens.cache_write),
    ]
    .into_iter()
    .filter(|&(_, count)| count > 0)
    .map(|(label, count)| (label, compact(count), percent(count, total)))
    .collect();
    Content::Tokens {
        total: compact(total),
        cache,
        novels,
        bars,
    }
}

fn award_page(stats: &Stats, won: &[Award], measures: &AwardMeasures, year: &str) -> Content {
    let headline = won.first().copied().unwrap_or(Award::SteadyShipper);
    let typo = stats
        .shell
        .as_ref()
        .and_then(|shell| shell.most_common_typo.as_ref());
    let reason = match headline {
        Award::AgentWhisperer => format!(
            "{} AI sessions.\nYour AI knows your coding style better than your coworkers do.",
            thousands(measures.sessions)
        ),
        Award::NightOwl => format!(
            "{}% of your activity between midnight and 5\u{a0}a.m.\nSleep is a suggestion.",
            measures.night_percent
        ),
        Award::EarlyBird => format!(
            "{}% of your activity between 5 and 8\u{a0}a.m.\nThe sun asks you for tips.",
            measures.early_percent
        ),
        Award::HolidayShipper => match measures.holiday {
            Some(Holiday::NewYear) => {
                "You committed on New Year's Day.\nNew year, same repo.".to_string()
            }
            _ => "You committed on December 25.\nSanta saw.".to_string(),
        },
        Award::DailyDriver => format!(
            "{} days with a commit.\nDays off are a rumor.",
            thousands(measures.commit_days.into())
        ),
        Award::HandsOff => format!(
            "{} tool calls per session.\nYou mostly watch.",
            thousands(measures.tool_uses_per_session)
        ),
        Award::AgentJuggler => "Claude Code and Codex CLI.\nYou don't have to choose.".to_string(),
        Award::BigBang => format!(
            "{} files per commit.\nAtomic commits are for other people.",
            thousands(measures.files_per_commit)
        ),
        Award::StatusChecker => format!(
            "{}% of your git commands were git status.\nStill nothing changed.",
            measures.git_status_percent
        ),
        Award::Specialist => format!(
            "{}% {}.\nYou found the one.",
            measures.top_language_percent,
            stats
                .git
                .as_ref()
                .and_then(|git| git.top_languages.first())
                .map_or("one language", |language| language.name.as_str())
        ),
        Award::WeekendWarrior => format!(
            "{}% of your activity on weekends.\nYour weekend has a backlog.",
            measures.weekend_percent
        ),
        Award::StreakKeeper => format!(
            "{} days in a row.\nA wall of green.",
            thousands(measures.longest_streak.into())
        ),
        Award::Polyglot => format!(
            "{} languages, all used for real.\nYour resume is just a list.",
            measures.common_languages
        ),
        Award::TypoChampion => match typo {
            Some(typo) => format!(
                "{} typos.\n{} is basically a real command now.",
                thousands(measures.typos),
                typo.name
            ),
            None => format!(
                "{} typos.\nYour shell is very patient.",
                thousands(measures.typos)
            ),
        },
        Award::SteadyShipper => {
            "No chaos, no all-nighters, just shipping.\nSuspiciously healthy.".to_string()
        }
    };
    let others: Vec<&str> = won
        .iter()
        .skip(1)
        .take(AWARDS_SHOWN - 1)
        .map(|award| award.name())
        .collect();
    Content::Award {
        label: format!("Your {year} title"),
        name: headline.name(),
        reason,
        also: (!others.is_empty()).then(|| format!("Also: {}.", others.join(", "))),
    }
}

fn repositories(top: &[Tally], commits: u64) -> Content {
    let line = top.first().map_or_else(String::new, |first| {
        format!(
            "{} got {}% of your commits.\nThe others noticed.",
            first.name,
            percent(first.count, commits)
        )
    });
    Content::Repositories {
        line,
        rows: rows(top),
    }
}

fn summary(stats: &Stats, won: &[Award], year: String, clock: Clock) -> Content {
    let mut rows: Vec<(&'static str, String)> = Vec::new();
    if let Some(git) = stats.git.as_ref().filter(|git| git.commits > 0) {
        rows.push(("commits", compact(git.commits)));
        let unit = if git.longest_streak == 1 {
            "day"
        } else {
            "days"
        };
        rows.push((
            "longest streak",
            format!("{} {unit}", compact(git.longest_streak.into())),
        ));
        if let Some(top) = git.top_languages.first() {
            rows.push(("top language", top.name.clone()));
        }
    }
    if let Some(shell) = stats.shell.as_ref().filter(|shell| shell.commands > 0) {
        rows.push(("shell commands", compact(shell.commands)));
    }
    let sessions = award_measures(stats).sessions;
    if sessions > 0 {
        rows.push(("AI sessions", compact(sessions)));
    }
    let tokens = stats
        .agents
        .as_ref()
        .map_or(0, |agents| agents.tokens.total());
    if tokens > 0 {
        rows.push(("tokens", compact(tokens)));
    }
    if stats.activity.events > 0 {
        let by_hour = &stats.activity.by_hour;
        let peak = position(by_hour, by_hour.iter().max()).unwrap_or(0);
        rows.push(("peak hour", clock.hour(peak as u8)));
    }
    if let Some(shell) = stats.shell.as_ref().filter(|shell| shell.typos > 0) {
        rows.push(("typos", compact(shell.typos)));
    }
    rows.truncate(SUMMARY_ROWS);
    Content::Summary {
        year,
        award: won.first().copied().unwrap_or(Award::SteadyShipper).name(),
        rows,
    }
}

#[cfg(test)]
mod tests {
    use devwrapped_core::{AgentStats, GitStats, ShellStats, sample};

    use super::*;

    fn names(stats: &Stats) -> Vec<&'static str> {
        story(stats, Clock::H24)
            .iter()
            .map(|page| page.name)
            .collect()
    }

    #[test]
    fn tells_the_whole_story_for_the_sample() {
        assert_eq!(
            names(&sample()),
            [
                "intro",
                "commits",
                "streak",
                "languages",
                "peak-hour",
                "shell",
                "typo",
                "agents",
                "tokens",
                "award",
                "repositories",
                "summary"
            ]
        );
    }

    #[test]
    fn skips_pages_without_data() {
        assert_eq!(names(&Stats::default()), ["intro", "award", "summary"]);
        let empty_sources = Stats {
            git: Some(GitStats::default()),
            shell: Some(ShellStats::default()),
            agents: Some(AgentStats::default()),
            ..Stats::default()
        };
        assert_eq!(names(&empty_sources), ["intro", "award", "summary"]);
    }

    #[test]
    fn picks_lines_from_the_numbers() {
        let pages = story(&sample(), Clock::H24);
        let Content::Commits {
            rate, line, week, ..
        } = &pages[1].content
        else {
            panic!("not the commits page");
        };
        assert_eq!(rate, "3.5 a day");
        assert_eq!(*line, "Your keyboard has filed a complaint.");
        let week = week.as_ref().unwrap();
        assert_eq!(week.busiest_line, "Tuesdays carried the team.");
        assert_eq!(week.quietest_line, "Sundays, you touched grass.");

        let Content::Typo { note, .. } = &pages[6].content else {
            panic!("not the typo page");
        };
        assert_eq!(note, "you meant git. 31 times.");

        let Content::Shell { extra, .. } = &pages[5].content else {
            panic!("not the shell page");
        };
        assert_eq!(
            extra.as_deref(),
            Some("And git status 4,120 times.\nIt didn’t change. We checked.")
        );

        let Content::Agents {
            split,
            tools,
            models,
            ..
        } = &pages[7].content
        else {
            panic!("not the agents page");
        };
        assert_eq!(split, "Claude Code 142 · Codex CLI 37 · 30 days together");
        assert_eq!(tools.len(), 5);
        assert_eq!(
            models[0],
            ("claude-opus-5-5".to_string(), "9,120".to_string())
        );

        let Content::Tokens {
            total,
            cache,
            novels,
            bars,
        } = &pages[8].content
        else {
            panic!("not the tokens page");
        };
        assert_eq!(total, "866M");
        assert_eq!(cache, "812M were cache reads.\nYour AI has a great memory.");
        assert_eq!(novels, "About 7,218 novels’ worth, none with a plot.");
        assert_eq!(bars.len(), 4);
        assert_eq!(bars[2], ("cache reads", "812M".to_string(), 93));
    }

    #[test]
    fn counts_slow_years_per_week() {
        let stats = Stats {
            since: jiff::civil::date(2026, 1, 1),
            until: jiff::civil::date(2026, 12, 31),
            git: Some(GitStats {
                commits: 40,
                longest_streak: 1,
                ..GitStats::default()
            }),
            ..Stats::default()
        };
        let pages = story(&stats, Clock::H24);
        let Content::Commits { rate, line, .. } = &pages[1].content else {
            panic!("not the commits page");
        };
        assert_eq!(rate, "0.7 a week");
        assert_eq!(
            *line,
            "Quality over quantity.\nThat’s the story we’re going with."
        );
        let Content::Streak { unit, .. } = &pages[2].content else {
            panic!("not the streak page");
        };
        assert_eq!(*unit, "day in a row.");
    }

    #[test]
    fn lists_only_the_sections_that_have_slides() {
        let empty_sources = Stats {
            git: Some(GitStats::default()),
            shell: Some(ShellStats::default()),
            agents: Some(AgentStats::default()),
            ..Stats::default()
        };
        let Content::Intro { lead, sections, .. } = &story(&empty_sources, Clock::H24)[0].content
        else {
            panic!("not the intro page");
        };
        assert_eq!(
            lead,
            "We read your git log, shell history, and agent logs.\nNone of it left your machine. Relax."
        );
        assert_eq!(sections, &[("you", "the award")]);
        let Content::Intro { sections, .. } = &story(&sample(), Clock::H24)[0].content else {
            panic!("not the intro page");
        };
        assert_eq!(sections.len(), 4);
    }

    #[test]
    fn joins_the_sources_it_read() {
        assert_eq!(join_words(&["a"]), "a");
        assert_eq!(join_words(&["a", "b"]), "a and b");
        assert_eq!(join_words(&["a", "b", "c"]), "a, b, and c");
    }

    #[test]
    fn summarizes_at_most_6_rows() {
        let pages = story(&sample(), Clock::H24);
        let Some(Content::Summary { rows, award, .. }) = pages.last().map(|page| &page.content)
        else {
            panic!("no summary");
        };
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0], ("commits", "1,284".to_string()));
        assert_eq!(*award, "Agent Whisperer");
    }
}
