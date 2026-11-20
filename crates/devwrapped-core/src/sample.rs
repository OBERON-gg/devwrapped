use jiff::civil::{Weekday, date};

use crate::agents::Tokens;
use crate::{Activity, AgentStats, GitStats, ShellStats, Stats, Tally};

pub fn sample() -> Stats {
    Stats {
        since: date(2026, 1, 1),
        until: date(2026, 12, 31),
        git: Some(GitStats {
            commits: 1284,
            active_days: 211,
            longest_streak: 23,
            busiest_hour: Some(22),
            busiest_weekday: Some(Weekday::Tuesday),
            top_languages: tallies(&[
                ("Rust", 1840),
                ("TypeScript", 960),
                ("Python", 410),
                ("Go", 255),
                ("Shell", 120),
            ]),
            changed_files: 4120,
            christmas_commits: 0,
            new_year_commits: 0,
            repositories: 14,
            top_repositories: tallies(&[
                ("devwrapped", 412),
                ("dotfiles", 233),
                ("homelab", 198),
                ("api-server", 151),
                ("blog", 64),
            ]),
        }),
        shell: Some(ShellStats {
            commands: 48213,
            undated_commands: 0,
            top_commands: tallies(&[
                ("git", 12904),
                ("cargo", 8311),
                ("cd", 7650),
                ("ls", 5122),
                ("nvim", 3018),
            ]),
            top_subcommands: tallies(&[
                ("git status", 4120),
                ("cargo test", 3307),
                ("git commit", 2290),
                ("cargo build", 1988),
                ("git push", 1204),
            ]),
            most_common_typo: Some(Tally {
                name: "gti".to_string(),
                count: 31,
            }),
            typos: 512,
            git_commands: 12904,
            git_status_commands: 4120,
        }),
        agents: Some(AgentStats {
            sessions: tallies(&[("Claude Code", 142), ("Codex CLI", 37)]),
            active_days: 30,
            tool_uses: tallies(&[
                ("Read", 5210),
                ("Edit", 3388),
                ("Bash", 2954),
                ("Grep", 1470),
                ("Write", 610),
            ]),
            tool_use_total: 14210,
            busiest_hour: Some(15),
            tokens: Tokens {
                input: 3_400_000,
                output: 9_800_000,
                cache_read: 812_000_000,
                cache_write: 41_000_000,
            },
            top_models: tallies(&[("claude-opus-5-5", 9120), ("gpt-5.5-codex", 2210)]),
        }),
        activity: Activity {
            events: 19000,
            by_hour: [
                120, 60, 30, 10, 5, 5, 10, 60, 300, 900, 1300, 1400, 1100, 1300, 1500, 1600, 1500,
                1300, 1100, 1000, 1100, 1300, 1400, 600,
            ],
            by_weekday: [3600, 3900, 3700, 3500, 3100, 1100, 100],
        },
    }
}

fn tallies(items: &[(&str, u64)]) -> Vec<Tally> {
    items
        .iter()
        .map(|&(name, count)| Tally {
            name: name.to_string(),
            count,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_ranked(list: &[Tally]) {
        assert!(!list.is_empty() && list.len() <= 5);
        assert!(list.windows(2).all(|pair| pair[0].count >= pair[1].count));
    }

    #[test]
    fn sample_fills_every_source() {
        let stats = sample();
        let git = stats.git.as_ref().unwrap();
        let shell = stats.shell.as_ref().unwrap();
        let agents = stats.agents.as_ref().unwrap();

        assert_ranked(&git.top_languages);
        assert_ranked(&shell.top_commands);
        assert_ranked(&shell.top_subcommands);
        assert_ranked(&agents.sessions);
        assert_ranked(&agents.tool_uses);
        assert!(git.busiest_hour.is_some_and(|hour| hour < 24));
        assert!(agents.busiest_hour.is_some_and(|hour| hour < 24));
        assert!(git.longest_streak <= git.active_days);
    }
}
