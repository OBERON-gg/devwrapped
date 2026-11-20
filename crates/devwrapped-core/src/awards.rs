use crate::Stats;

pub const AWARDS_SHOWN: usize = 3;

const AGENT_WHISPERER_SESSIONS: u64 = 50;
const AGENT_WHISPERER_SESSIONS_PER_DAY: u64 = 5;
// the percentage awards mean little for a handful of events
const MIN_ACTIVITY_EVENTS: u64 = 100;
const NIGHT_OWL_PERCENT: u64 = 25;
const NIGHT_HOURS: std::ops::Range<usize> = 0..5;
const EARLY_BIRD_PERCENT: u64 = 20;
const EARLY_HOURS: std::ops::Range<usize> = 5..8;
const DAILY_DRIVER_DAYS: u32 = 200;
const STREAK_KEEPER_DAYS: u32 = 14;
const WEEKEND_WARRIOR_PERCENT: u64 = 35;
const WEEKEND_DAYS: std::ops::Range<usize> = 5..7;
const HANDS_OFF_TOOL_USES_PER_SESSION: u64 = 40;
const HANDS_OFF_SESSIONS: u64 = 10;
const AGENT_JUGGLER_PERCENT: u64 = 20;
const BIG_BANG_FILES_PER_COMMIT: u64 = 20;
const BIG_BANG_COMMITS: u64 = 10;
const POLYGLOT_LANGUAGES: usize = 5;
const POLYGLOT_PERCENT: u64 = 5;
// history files range from 500 lines to many thousands, so a count alone is unfair
const TYPO_CHAMPION_TYPOS: u64 = 25;
const TYPO_CHAMPION_PERCENT: u64 = 1;
const STATUS_CHECKER_USES: u64 = 100;
const STATUS_CHECKER_PERCENT: u64 = 25;
const SPECIALIST_PERCENT: u64 = 90;

/// in headline order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Award {
    AgentWhisperer,
    NightOwl,
    EarlyBird,
    HolidayShipper,
    DailyDriver,
    StreakKeeper,
    WeekendWarrior,
    HandsOff,
    AgentJuggler,
    BigBang,
    Polyglot,
    TypoChampion,
    StatusChecker,
    Specialist,
    SteadyShipper,
}

impl Award {
    pub fn name(self) -> &'static str {
        match self {
            Award::AgentWhisperer => "Agent Whisperer",
            Award::NightOwl => "Night Owl",
            Award::EarlyBird => "Early Bird",
            Award::HolidayShipper => "Holiday Shipper",
            Award::DailyDriver => "Daily Driver",
            Award::StreakKeeper => "Streak Keeper",
            Award::WeekendWarrior => "Weekend Warrior",
            Award::HandsOff => "Hands Off",
            Award::AgentJuggler => "Agent Juggler",
            Award::BigBang => "Big Bang",
            Award::Polyglot => "Polyglot",
            Award::TypoChampion => "Typo Champion",
            Award::StatusChecker => "Status Checker",
            Award::Specialist => "Specialist",
            Award::SteadyShipper => "Steady Shipper",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holiday {
    Christmas,
    NewYear,
}

/// the numbers the award rules look at, so a card can say why an award applies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AwardMeasures {
    pub sessions: u64,
    pub agent_days: u32,
    pub night_percent: u64,
    pub early_percent: u64,
    pub weekend_percent: u64,
    /// the holiday with a commit, Christmas first
    pub holiday: Option<Holiday>,
    pub commit_days: u32,
    pub longest_streak: u32,
    pub tool_uses_per_session: u64,
    /// the share of sessions of the less used agent, 0 with fewer than 2 agents
    pub second_agent_percent: u64,
    pub files_per_commit: u64,
    pub common_languages: usize,
    pub top_language_percent: u64,
    pub typos: u64,
    pub git_status_percent: u64,
}

pub fn award_measures(stats: &Stats) -> AwardMeasures {
    let activity = &stats.activity;
    let git = stats.git.as_ref();
    let shell = stats.shell.as_ref();
    let agents = stats.agents.as_ref();
    let sessions = agents.map_or(0, |agents| {
        agents
            .sessions
            .iter()
            .map(|tally| tally.count)
            .fold(0, u64::saturating_add)
    });
    let changed_files = git.map_or(0, |git| git.changed_files);
    AwardMeasures {
        sessions,
        agent_days: agents.map_or(0, |agents| agents.active_days),
        night_percent: percent(total(&activity.by_hour[NIGHT_HOURS]), activity.events),
        early_percent: percent(total(&activity.by_hour[EARLY_HOURS]), activity.events),
        weekend_percent: percent(total(&activity.by_weekday[WEEKEND_DAYS]), activity.events),
        holiday: git.and_then(|git| {
            if git.christmas_commits > 0 {
                Some(Holiday::Christmas)
            } else if git.new_year_commits > 0 {
                Some(Holiday::NewYear)
            } else {
                None
            }
        }),
        commit_days: git.map_or(0, |git| git.active_days),
        longest_streak: git.map_or(0, |git| git.longest_streak),
        tool_uses_per_session: ratio(agents.map_or(0, |agents| agents.tool_use_total), sessions),
        second_agent_percent: agents
            .filter(|agents| agents.sessions.len() >= 2)
            .and_then(|agents| agents.sessions.iter().map(|tally| tally.count).min())
            .map_or(0, |fewest| percent(fewest, sessions)),
        files_per_commit: ratio(changed_files, git.map_or(0, |git| git.commits)),
        common_languages: git.map_or(0, |git| {
            git.top_languages
                .iter()
                .filter(|language| at_least(language.count, changed_files, POLYGLOT_PERCENT))
                .count()
        }),
        top_language_percent: git
            .and_then(|git| git.top_languages.first())
            .map_or(0, |language| percent(language.count, changed_files)),
        typos: shell.map_or(0, |shell| shell.typos),
        git_status_percent: shell.map_or(0, |shell| {
            percent(shell.git_status_commands, shell.git_commands)
        }),
    }
}

/// returns every award that applies in headline order, so the first one is the headline
pub fn awards(stats: &Stats) -> Vec<Award> {
    let measures = award_measures(stats);
    let shares = stats.activity.events >= MIN_ACTIVITY_EVENTS;
    let commits = stats.git.as_ref().map_or(0, |git| git.commits);
    let shell = stats.shell.as_ref();

    let rules = [
        (
            Award::AgentWhisperer,
            measures.sessions >= AGENT_WHISPERER_SESSIONS
                && measures.sessions
                    >= u64::from(measures.agent_days) * AGENT_WHISPERER_SESSIONS_PER_DAY,
        ),
        (
            Award::NightOwl,
            shares && measures.night_percent >= NIGHT_OWL_PERCENT,
        ),
        (
            Award::EarlyBird,
            shares && measures.early_percent >= EARLY_BIRD_PERCENT,
        ),
        (Award::HolidayShipper, measures.holiday.is_some()),
        (
            Award::DailyDriver,
            measures.commit_days >= DAILY_DRIVER_DAYS,
        ),
        (
            Award::StreakKeeper,
            measures.longest_streak >= STREAK_KEEPER_DAYS,
        ),
        (
            Award::WeekendWarrior,
            shares && measures.weekend_percent >= WEEKEND_WARRIOR_PERCENT,
        ),
        (
            Award::HandsOff,
            measures.sessions >= HANDS_OFF_SESSIONS
                && measures.tool_uses_per_session >= HANDS_OFF_TOOL_USES_PER_SESSION,
        ),
        (
            Award::AgentJuggler,
            measures.second_agent_percent >= AGENT_JUGGLER_PERCENT,
        ),
        (
            Award::BigBang,
            commits >= BIG_BANG_COMMITS && measures.files_per_commit >= BIG_BANG_FILES_PER_COMMIT,
        ),
        (
            Award::Polyglot,
            measures.common_languages >= POLYGLOT_LANGUAGES,
        ),
        (
            Award::TypoChampion,
            shell.is_some_and(|shell| {
                shell.typos >= TYPO_CHAMPION_TYPOS
                    && at_least(shell.typos, shell.commands, TYPO_CHAMPION_PERCENT)
            }),
        ),
        (
            Award::StatusChecker,
            shell.is_some_and(|shell| {
                shell.git_status_commands >= STATUS_CHECKER_USES
                    && at_least(
                        shell.git_status_commands,
                        shell.git_commands,
                        STATUS_CHECKER_PERCENT,
                    )
            }),
        ),
        (
            Award::Specialist,
            stats.git.as_ref().is_some_and(|git| {
                git.top_languages.first().is_some_and(|language| {
                    at_least(language.count, git.changed_files, SPECIALIST_PERCENT)
                })
            }),
        ),
    ];
    let won: Vec<Award> = rules
        .into_iter()
        .filter(|&(_, applies)| applies)
        .map(|(award, _)| award)
        .collect();
    if won.is_empty() {
        vec![Award::SteadyShipper]
    } else {
        won
    }
}

fn total(counts: &[u64]) -> u64 {
    counts
        .iter()
        .fold(0, |sum, &count| sum.saturating_add(count))
}

fn percent(part: u64, whole: u64) -> u64 {
    if whole == 0 {
        0
    } else {
        (u128::from(part) * 100 / u128::from(whole)) as u64
    }
}

fn ratio(part: u64, whole: u64) -> u64 {
    part.checked_div(whole).unwrap_or(0)
}

fn at_least(part: u64, whole: u64, percent: u64) -> bool {
    whole > 0 && u128::from(part) * 100 >= u128::from(whole) * u128::from(percent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Activity, AgentStats, GitStats, ShellStats, Tally};

    #[test]
    fn measures_what_the_rules_look_at() {
        let measures = award_measures(&crate::sample());
        assert_eq!(measures.sessions, 179);
        assert_eq!(measures.agent_days, 30);
        assert_eq!(measures.longest_streak, 23);
        assert_eq!(measures.commit_days, 211);
        assert_eq!(measures.tool_uses_per_session, 14210 / 179);
        assert_eq!(measures.second_agent_percent, 37 * 100 / 179);
        assert_eq!(measures.files_per_commit, 4120 / 1284);
        assert_eq!(measures.top_language_percent, 1840 * 100 / 4120);
        assert_eq!(measures.typos, 512);
        assert_eq!(measures.git_status_percent, 4120 * 100 / 12904);
        assert_eq!(measures.night_percent, 225 * 100 / 19000);
        assert_eq!(measures.weekend_percent, 1200 * 100 / 19000);
        assert_eq!(measures.holiday, None);
        assert_eq!(award_measures(&Stats::default()).night_percent, 0);
    }

    fn tally(name: &str, count: u64) -> Tally {
        Tally {
            name: name.to_string(),
            count,
        }
    }

    // each slot is (hour, weekday from Monday, events)
    fn with_activity(slots: &[(usize, usize, u64)]) -> Stats {
        let mut activity = Activity::default();
        for &(hour, weekday, count) in slots {
            activity.events += count;
            activity.by_hour[hour] += count;
            activity.by_weekday[weekday] += count;
        }
        Stats {
            activity,
            ..Stats::default()
        }
    }

    fn with_agents(agents: AgentStats) -> Stats {
        Stats {
            agents: Some(agents),
            ..Stats::default()
        }
    }

    fn with_sessions(claude: u64, codex: u64, days: u32) -> Stats {
        with_agents(AgentStats {
            sessions: vec![tally("Claude Code", claude), tally("Codex CLI", codex)],
            active_days: days,
            ..AgentStats::default()
        })
    }

    fn with_git(git: GitStats) -> Stats {
        Stats {
            git: Some(git),
            ..Stats::default()
        }
    }

    fn with_shell(shell: ShellStats) -> Stats {
        Stats {
            shell: Some(shell),
            ..Stats::default()
        }
    }

    fn with_typos(typos: u64, commands: u64) -> Stats {
        with_shell(ShellStats {
            typos,
            commands,
            ..ShellStats::default()
        })
    }

    fn languages(counts: &[u64], changed_files: u64) -> Stats {
        let top_languages = counts
            .iter()
            .enumerate()
            .map(|(index, &count)| tally(&format!("language {index}"), count))
            .collect();
        with_git(GitStats {
            top_languages,
            changed_files,
            ..GitStats::default()
        })
    }

    fn wins(stats: &Stats, award: Award) -> bool {
        awards(stats).contains(&award)
    }

    #[test]
    fn agent_whisperer_needs_5_sessions_a_day_and_50_in_all() {
        assert!(wins(&with_sessions(40, 10, 10), Award::AgentWhisperer));
        assert!(!wins(&with_sessions(40, 10, 11), Award::AgentWhisperer));
        assert!(!wins(&with_sessions(40, 9, 1), Award::AgentWhisperer));
    }

    #[test]
    fn night_owl_needs_25_percent_before_5() {
        assert!(wins(
            &with_activity(&[(4, 0, 25), (12, 0, 75)]),
            Award::NightOwl
        ));
        assert!(!wins(
            &with_activity(&[(4, 0, 24), (12, 0, 76)]),
            Award::NightOwl
        ));
        assert!(!wins(
            &with_activity(&[(5, 0, 25), (12, 0, 75)]),
            Award::NightOwl
        ));
    }

    #[test]
    fn early_bird_needs_20_percent_from_5_to_8() {
        assert!(wins(
            &with_activity(&[(5, 0, 10), (7, 0, 10), (12, 0, 80)]),
            Award::EarlyBird
        ));
        assert!(!wins(
            &with_activity(&[(5, 0, 10), (7, 0, 9), (12, 0, 81)]),
            Award::EarlyBird
        ));
        assert!(!wins(
            &with_activity(&[(8, 0, 20), (12, 0, 80)]),
            Award::EarlyBird
        ));
    }

    #[test]
    fn holiday_shipper_needs_a_commit_on_christmas_or_new_years_day() {
        let holidays = |christmas, new_year| {
            award_measures(&with_git(GitStats {
                christmas_commits: christmas,
                new_year_commits: new_year,
                ..GitStats::default()
            }))
            .holiday
        };
        assert_eq!(holidays(1, 1), Some(Holiday::Christmas));
        assert_eq!(holidays(0, 1), Some(Holiday::NewYear));
        assert_eq!(holidays(0, 0), None);
        let stats = with_git(GitStats {
            new_year_commits: 1,
            ..GitStats::default()
        });
        assert!(wins(&stats, Award::HolidayShipper));
        assert!(!wins(&with_git(GitStats::default()), Award::HolidayShipper));
    }

    #[test]
    fn daily_driver_needs_200_days_with_a_commit() {
        let days = |active_days| {
            with_git(GitStats {
                active_days,
                ..GitStats::default()
            })
        };
        assert!(wins(&days(200), Award::DailyDriver));
        assert!(!wins(&days(199), Award::DailyDriver));
    }

    #[test]
    fn streak_keeper_needs_14_days() {
        let streak = |days| {
            with_git(GitStats {
                longest_streak: days,
                ..GitStats::default()
            })
        };
        assert!(wins(&streak(14), Award::StreakKeeper));
        assert!(!wins(&streak(13), Award::StreakKeeper));
    }

    #[test]
    fn weekend_warrior_needs_35_percent_on_the_weekend() {
        assert!(wins(
            &with_activity(&[(12, 5, 20), (12, 6, 15), (12, 2, 65)]),
            Award::WeekendWarrior
        ));
        assert!(!wins(
            &with_activity(&[(12, 5, 20), (12, 6, 14), (12, 2, 66)]),
            Award::WeekendWarrior
        ));
        assert!(!wins(
            &with_activity(&[(12, 4, 35), (12, 2, 65)]),
            Award::WeekendWarrior
        ));
    }

    #[test]
    fn percentage_awards_need_100_events() {
        let few = with_activity(&[(2, 6, 99)]);
        assert_eq!(awards(&few), [Award::SteadyShipper]);

        let enough = with_activity(&[(2, 6, 100)]);
        assert_eq!(awards(&enough), [Award::NightOwl, Award::WeekendWarrior]);
    }

    #[test]
    fn hands_off_needs_40_tool_uses_per_session_over_10_sessions() {
        let tools = |sessions, tool_use_total| {
            with_agents(AgentStats {
                sessions: vec![tally("Claude Code", sessions)],
                tool_use_total,
                ..AgentStats::default()
            })
        };
        assert!(wins(&tools(10, 400), Award::HandsOff));
        assert!(!wins(&tools(10, 399), Award::HandsOff));
        assert!(!wins(&tools(9, 900), Award::HandsOff));
    }

    #[test]
    fn agent_juggler_needs_20_percent_on_each_agent() {
        assert!(wins(&with_sessions(80, 20, 100), Award::AgentJuggler));
        assert!(!wins(&with_sessions(81, 19, 100), Award::AgentJuggler));
        let one = with_agents(AgentStats {
            sessions: vec![tally("Claude Code", 100)],
            ..AgentStats::default()
        });
        assert!(!wins(&one, Award::AgentJuggler));
    }

    #[test]
    fn big_bang_needs_20_files_per_commit_over_10_commits() {
        let commits = |commits, changed_files| {
            with_git(GitStats {
                commits,
                changed_files,
                ..GitStats::default()
            })
        };
        assert!(wins(&commits(10, 200), Award::BigBang));
        assert!(!wins(&commits(10, 199), Award::BigBang));
        assert!(!wins(&commits(9, 900), Award::BigBang));
    }

    #[test]
    fn polyglot_needs_5_languages_with_5_percent_each() {
        assert!(wins(&languages(&[60, 5, 5, 5, 5], 100), Award::Polyglot));
        assert!(!wins(&languages(&[60, 5, 5, 5, 4], 100), Award::Polyglot));
        assert!(!wins(&languages(&[60, 20, 20], 100), Award::Polyglot));
    }

    #[test]
    fn typo_champion_needs_25_typos_and_1_percent() {
        assert!(wins(&with_typos(25, 2500), Award::TypoChampion));
        assert!(!wins(&with_typos(25, 2501), Award::TypoChampion));
        assert!(!wins(&with_typos(24, 100), Award::TypoChampion));
    }

    #[test]
    fn status_checker_needs_100_uses_and_25_percent_of_git() {
        let status = |git_status_commands, git_commands| {
            with_shell(ShellStats {
                git_commands,
                git_status_commands,
                ..ShellStats::default()
            })
        };
        assert!(wins(&status(100, 400), Award::StatusChecker));
        assert!(!wins(&status(100, 401), Award::StatusChecker));
        assert!(!wins(&status(99, 100), Award::StatusChecker));
    }

    #[test]
    fn specialist_needs_90_percent_in_one_language() {
        assert!(wins(&languages(&[90, 5], 100), Award::Specialist));
        assert!(!wins(&languages(&[89, 5], 100), Award::Specialist));
        assert!(!wins(&languages(&[], 100), Award::Specialist));
    }

    #[test]
    fn steady_shipper_applies_only_when_nothing_else_does() {
        assert_eq!(awards(&Stats::default()), [Award::SteadyShipper]);
        assert!(!wins(&with_typos(25, 25), Award::SteadyShipper));
    }

    #[test]
    fn lists_awards_in_headline_order() {
        let mut stats = with_activity(&[(2, 5, 40), (6, 6, 40), (12, 2, 20)]);
        stats.agents = Some(AgentStats {
            sessions: vec![tally("Claude Code", 80), tally("Codex CLI", 20)],
            active_days: 20,
            tool_use_total: 4000,
            ..AgentStats::default()
        });
        stats.git = Some(GitStats {
            commits: 10,
            changed_files: 200,
            active_days: 200,
            longest_streak: 14,
            christmas_commits: 1,
            top_languages: vec![
                tally("Rust", 190),
                tally("a", 2),
                tally("b", 2),
                tally("c", 2),
            ],
            ..GitStats::default()
        });
        stats.shell = Some(ShellStats {
            commands: 100,
            typos: 25,
            git_commands: 100,
            git_status_commands: 100,
            ..ShellStats::default()
        });

        assert_eq!(
            awards(&stats),
            [
                Award::AgentWhisperer,
                Award::NightOwl,
                Award::EarlyBird,
                Award::HolidayShipper,
                Award::DailyDriver,
                Award::StreakKeeper,
                Award::WeekendWarrior,
                Award::HandsOff,
                Award::AgentJuggler,
                Award::BigBang,
                Award::TypoChampion,
                Award::StatusChecker,
                Award::Specialist,
            ]
        );
        stats.git = languages(&[20, 20, 20, 20, 20], 100).git;
        assert!(wins(&stats, Award::Polyglot));
    }

    #[test]
    fn sample_keeps_its_headline_and_the_shown_awards() {
        let won = awards(&crate::sample());

        assert_eq!(
            won[..AWARDS_SHOWN],
            [
                Award::AgentWhisperer,
                Award::DailyDriver,
                Award::StreakKeeper
            ]
        );
        assert!(won.contains(&Award::TypoChampion));
    }
}
