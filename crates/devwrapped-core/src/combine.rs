use std::collections::{BTreeMap, BTreeSet};

use jiff::tz::TimeZone;

use crate::agents::{AgentCollection, Tokens};
use crate::git::{self, CommitScan};
use crate::shell::{HistoryScan, typos};
use crate::stats::{Activity, busiest_hour, local_time, top_tallies};
use crate::{AgentStats, Range, ShellStats, Stats};

// a name used once or twice could be a password typed at the prompt by mistake
const MIN_NAME_USES: u64 = 3;

/// collector output for one run, where `None` means the source was skipped or not found
#[derive(Debug, Default)]
pub struct Sources {
    pub git: Option<CommitScan>,
    pub shell: Option<Vec<HistoryScan>>,
    pub agents: Option<Vec<AgentCollection>>,
}

/// builds the stats model, keeping dated records inside `range` and grouping them in `time_zone`
pub fn combine(sources: &Sources, range: Range, time_zone: &TimeZone) -> Stats {
    let mut activity = Activity::default();
    let git = sources.git.as_ref().map(|scan| {
        for commit in &scan.commits {
            activity.add(&local_time(commit.time, time_zone));
        }
        git::summarize(&scan.commits, scan.active_repositories, time_zone)
    });
    let shell = sources
        .shell
        .as_ref()
        .map(|scans| shell_stats(scans, range, time_zone, &mut activity));
    let agents = sources
        .agents
        .as_ref()
        .map(|collections| agent_stats(collections, range, time_zone, &mut activity));
    let (since, until) = range.days(time_zone);
    Stats {
        since,
        until,
        git,
        shell,
        agents,
        activity,
    }
}

fn shell_stats(
    scans: &[HistoryScan],
    range: Range,
    time_zone: &TimeZone,
    activity: &mut Activity,
) -> ShellStats {
    let mut stats = ShellStats::default();
    let mut names: BTreeMap<&str, u64> = BTreeMap::new();
    let mut subcommands: BTreeMap<String, u64> = BTreeMap::new();
    let mut typo_names: BTreeMap<String, u64> = BTreeMap::new();
    for command in scans.iter().flat_map(|scan| &scan.commands) {
        match command.time {
            Some(time) if !range.contains(time) => continue,
            Some(time) => activity.add(&local_time(time, time_zone)),
            None => stats.undated_commands += 1,
        }
        stats.commands += 1;
        *names.entry(&command.name).or_default() += 1;
        if let Some(subcommand) = &command.subcommand {
            *subcommands
                .entry(format!("{} {subcommand}", command.name))
                .or_default() += 1;
        }
        if command.name.eq_ignore_ascii_case("git") {
            stats.git_commands += 1;
            if command
                .subcommand
                .as_ref()
                .is_some_and(|subcommand| subcommand.eq_ignore_ascii_case("status"))
            {
                stats.git_status_commands += 1;
            }
        }
        if let Some(typo) = typos::find_typo(command) {
            stats.typos += 1;
            *typo_names.entry(typo).or_default() += 1;
        }
    }
    stats.top_commands = top_tallies(names, MIN_NAME_USES);
    stats.top_subcommands = top_tallies(subcommands, MIN_NAME_USES);
    stats.most_common_typo = top_tallies(typo_names, MIN_NAME_USES).into_iter().next();
    stats
}

fn agent_stats(
    collections: &[AgentCollection],
    range: Range,
    time_zone: &TimeZone,
    activity: &mut Activity,
) -> AgentStats {
    let mut sessions: BTreeMap<&str, u64> = BTreeMap::new();
    let mut days = BTreeSet::new();
    let mut hours = [0u64; 24];
    let mut tool_uses: BTreeMap<&str, u64> = BTreeMap::new();
    let mut models: BTreeMap<&str, u64> = BTreeMap::new();
    let mut tokens = Tokens::default();
    let in_range = collections
        .iter()
        .flat_map(|collection| &collection.sessions)
        .filter(|session| range.contains(session.start));
    for session in in_range {
        let local = local_time(session.start, time_zone);
        activity.add(&local);
        days.insert(local.date);
        hours[local.hour] += 1;
        *sessions.entry(session.agent.name()).or_default() += 1;
        for (name, count) in &session.tool_uses {
            *tool_uses.entry(name).or_default() += count;
        }
        for (name, count) in &session.models {
            *models.entry(name).or_default() += count;
        }
        tokens += session.tokens;
    }
    AgentStats {
        sessions: top_tallies(sessions, 1),
        active_days: days.len() as u32,
        tool_use_total: tool_uses
            .values()
            .fold(0, |sum, &count| sum.saturating_add(count)),
        tool_uses: top_tallies(tool_uses, 1),
        busiest_hour: busiest_hour(&hours),
        tokens,
        top_models: top_tallies(models, 1),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use jiff::Timestamp;

    use super::*;
    use crate::agents::{Agent, AgentSession};
    use crate::git::Commit;
    use crate::shell::ParsedCommand;

    fn at(time: &str) -> Timestamp {
        time.parse().unwrap()
    }

    fn range_2026() -> Range {
        Range {
            start: at("2026-01-01T00:00:00Z"),
            end: at("2027-01-01T00:00:00Z"),
        }
    }

    fn command(name: &str, subcommand: Option<&str>, time: Option<&str>) -> ParsedCommand {
        ParsedCommand {
            name: name.to_string(),
            subcommand: subcommand.map(str::to_string),
            time: time.map(at),
        }
    }

    fn names(tallies: &[crate::Tally]) -> Vec<(&str, u64)> {
        tallies
            .iter()
            .map(|tally| (tally.name.as_str(), tally.count))
            .collect()
    }

    #[test]
    fn leaves_out_sources_that_are_missing() {
        let stats = combine(&Sources::default(), range_2026(), &TimeZone::UTC);

        assert_eq!(
            stats,
            Stats {
                since: jiff::civil::date(2026, 1, 1),
                until: jiff::civil::date(2026, 12, 31),
                ..Stats::default()
            }
        );
    }

    #[test]
    fn counts_git_commits_and_their_activity() {
        let commit = |time| Commit {
            time: at(time),
            changed_files: 2,
            extensions: vec!["rs".to_string(), "rs".to_string()],
        };
        let sources = Sources {
            git: Some(CommitScan {
                commits: vec![
                    commit("2026-03-07T02:00:00Z"),
                    commit("2026-03-08T03:00:00Z"),
                ],
                active_repositories: 1,
                ..CommitScan::default()
            }),
            ..Sources::default()
        };

        let stats = combine(&sources, range_2026(), &TimeZone::UTC);

        let git = stats.git.unwrap();
        assert_eq!(
            (git.commits, git.changed_files, git.repositories),
            (2, 4, 1)
        );
        assert_eq!(stats.activity.events, 2);
        assert_eq!(stats.activity.by_hour[2] + stats.activity.by_hour[3], 2);
        assert_eq!(
            stats.activity.by_weekday[5] + stats.activity.by_weekday[6],
            2
        );
    }

    #[test]
    fn filters_dated_commands_and_keeps_undated_ones() {
        let mut commands = vec![
            command("git", Some("status"), Some("2025-12-31T23:00:00Z")),
            command("ls", None, None),
        ];
        for _ in 0..3 {
            commands.push(command("git", Some("status"), Some("2026-03-02T09:00:00Z")));
            commands.push(command("gti", None, Some("2026-03-02T09:00:00Z")));
        }
        commands.push(command("hunter2", None, Some("2026-03-02T09:00:00Z")));
        commands.push(command("sl", None, None));
        let sources = Sources {
            shell: Some(vec![HistoryScan {
                commands,
                skipped_lines: 0,
            }]),
            ..Sources::default()
        };

        let stats = combine(&sources, range_2026(), &TimeZone::UTC);

        let shell = stats.shell.unwrap();
        assert_eq!((shell.commands, shell.undated_commands), (9, 2));
        assert_eq!(names(&shell.top_commands), [("git", 3), ("gti", 3)]);
        assert_eq!(names(&shell.top_subcommands), [("git status", 3)]);
        assert_eq!(shell.most_common_typo.unwrap().name, "gti");
        assert_eq!(shell.typos, 4);
        assert_eq!((shell.git_commands, shell.git_status_commands), (3, 3));
        assert_eq!(stats.activity.events, 7);
    }

    #[test]
    fn merges_agent_sessions_in_range() {
        let session = |agent, start, tool: &str| AgentSession {
            agent,
            start: at(start),
            tool_uses: BTreeMap::from([(tool.to_string(), 2)]),
            models: BTreeMap::from([("model-a".to_string(), 1)]),
            tokens: Tokens {
                input: 10,
                output: 1,
                cache_read: 100,
                cache_write: 0,
            },
        };
        let sources = Sources {
            agents: Some(vec![
                AgentCollection {
                    found: true,
                    sessions: vec![
                        session(Agent::ClaudeCode, "2026-03-02T15:00:00Z", "Read"),
                        session(Agent::ClaudeCode, "2026-03-03T15:00:00Z", "Bash"),
                        session(Agent::ClaudeCode, "2025-06-01T15:00:00Z", "Read"),
                    ],
                    ..AgentCollection::default()
                },
                AgentCollection {
                    found: true,
                    sessions: vec![session(Agent::CodexCli, "2026-03-03T09:00:00Z", "Read")],
                    ..AgentCollection::default()
                },
            ]),
            ..Sources::default()
        };

        let stats = combine(&sources, range_2026(), &TimeZone::UTC);

        let agents = stats.agents.unwrap();
        assert_eq!(
            names(&agents.sessions),
            [("Claude Code", 2), ("Codex CLI", 1)]
        );
        assert_eq!(agents.active_days, 2);
        assert_eq!(names(&agents.tool_uses), [("Read", 4), ("Bash", 2)]);
        assert_eq!(agents.tool_use_total, 6);
        assert_eq!(agents.busiest_hour, Some(15));
        assert_eq!(agents.tokens.input, 30);
        assert_eq!(names(&agents.top_models), [("model-a", 3)]);
        assert_eq!(stats.activity.events, 3);
    }

    // clocks jump from 02:00 to 03:00 on 2026-03-08 and fall back from 02:00 to 01:00 on 2026-11-01
    fn new_york() -> TimeZone {
        TimeZone::posix("EST5EDT,M3.2.0,M11.1.0").unwrap()
    }

    #[test]
    fn groups_commits_by_local_time_across_the_spring_change() {
        let commit = |time| Commit {
            time: at(time),
            changed_files: 1,
            extensions: Vec::new(),
        };
        let sources = Sources {
            git: Some(CommitScan {
                commits: vec![
                    commit("2026-03-07T15:00:00Z"),
                    commit("2026-03-08T06:30:00Z"),
                    commit("2026-03-08T07:30:00Z"),
                    commit("2026-03-09T03:30:00Z"),
                    commit("2026-03-09T13:00:00Z"),
                ],
                active_repositories: 1,
                ..CommitScan::default()
            }),
            ..Sources::default()
        };

        let stats = combine(&sources, range_2026(), &new_york());

        let git = stats.git.unwrap();
        assert_eq!((git.active_days, git.longest_streak), (3, 3));
        let hours: Vec<usize> = (0..24)
            .filter(|&hour| stats.activity.by_hour[hour] > 0)
            .collect();
        assert_eq!(hours, [1, 3, 9, 10, 23]);
        assert_eq!(stats.activity.by_weekday, [1, 0, 0, 0, 0, 1, 3]);
    }

    #[test]
    fn groups_commands_and_sessions_by_local_time_across_the_fall_change() {
        let sources = Sources {
            shell: Some(vec![HistoryScan {
                commands: vec![
                    command("ls", None, Some("2026-11-01T05:30:00Z")),
                    command("ls", None, Some("2026-11-01T06:30:00Z")),
                ],
                skipped_lines: 0,
            }]),
            agents: Some(vec![AgentCollection {
                found: true,
                sessions: vec![AgentSession {
                    agent: Agent::ClaudeCode,
                    start: at("2026-11-01T06:59:00Z"),
                    tool_uses: BTreeMap::new(),
                    models: BTreeMap::new(),
                    tokens: Tokens::default(),
                }],
                ..AgentCollection::default()
            }]),
            ..Sources::default()
        };

        let stats = combine(&sources, range_2026(), &new_york());

        assert_eq!(stats.activity.events, 3);
        assert_eq!(stats.activity.by_hour[1], 3);
        assert_eq!(stats.activity.by_weekday[6], 3);
        let agents = stats.agents.unwrap();
        assert_eq!((agents.active_days, agents.busiest_hour), (1, Some(1)));
    }
}
