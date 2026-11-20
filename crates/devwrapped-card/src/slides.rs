use devwrapped_core::Stats;

use crate::draw::{Canvas, Scene, TextBox};
use crate::story::story;
use crate::theme::{Layout, Theme};
use crate::{Clock, afterglow, swiss, year_text, zine};

/// one 1080 x 1350 slide of the story, as a standalone svg
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slide {
    pub name: &'static str,
    pub svg: String,
}

pub fn slides(stats: &Stats, theme: Theme, clock: Clock) -> Vec<Slide> {
    laid_out(stats, theme, clock)
        .into_iter()
        .map(|(slide, _)| slide)
        .collect()
}

pub(crate) fn laid_out(stats: &Stats, theme: Theme, clock: Clock) -> Vec<(Slide, Vec<TextBox>)> {
    let pages = story(stats, clock);
    let year = year_text(stats);
    let palette = theme.palette();
    pages
        .iter()
        .enumerate()
        .map(|(index, page)| {
            let scene = Scene {
                theme,
                palette: &palette,
                page,
                number: index + 1,
                year: &year,
                total: pages.len(),
            };
            let mut canvas = Canvas::new();
            match theme.layout() {
                Layout::Afterglow => afterglow::draw(&scene, &mut canvas),
                Layout::Zine => zine::draw(&scene, &mut canvas),
                Layout::Swiss => swiss::draw(&scene, &mut canvas),
            }
            let boxes = canvas.boxes.clone();
            let slide = Slide {
                name: page.name,
                svg: canvas.finish(),
            };
            (slide, boxes)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use devwrapped_core::agents::Tokens;
    use devwrapped_core::{Activity, AgentStats, GitStats, ShellStats, Tally, sample};
    use jiff::civil::date;

    use super::*;
    use crate::draw::{HEIGHT, WIDTH};

    const THEMES: [Theme; 3] = [Theme::Night, Theme::Poster, Theme::Paper];
    const EDGE: f32 = 64.0;
    // a feed shows the slide about 390 px wide, where 28 px is the smallest text that reads
    const SMALLEST_TEXT: f32 = 28.0;

    fn assert_fits(stats: &Stats) {
        for (theme, clock) in THEMES
            .into_iter()
            .flat_map(|theme| [(theme, Clock::H24), (theme, Clock::H12)])
        {
            for (slide, boxes) in laid_out(stats, theme, clock) {
                let place = format!("{theme:?} {clock:?} {}", slide.name);
                for text in &boxes {
                    assert!(
                        text.size >= SMALLEST_TEXT,
                        "{place}: {} is {} px",
                        text.text,
                        text.size
                    );
                    assert!(
                        text.left >= EDGE,
                        "{place}: {} starts off the left",
                        text.text
                    );
                    assert!(
                        text.right <= WIDTH - EDGE,
                        "{place}: {} runs off the right",
                        text.text
                    );
                    assert!(text.top >= 60.0, "{place}: {} runs off the top", text.text);
                    assert!(
                        text.bottom <= HEIGHT - 40.0,
                        "{place}: {} runs off the bottom",
                        text.text
                    );
                }
                for (index, first) in boxes.iter().enumerate() {
                    for second in &boxes[index + 1..] {
                        let apart = first.right <= second.left + 2.0
                            || second.right <= first.left + 2.0
                            || first.bottom <= second.top + 2.0
                            || second.bottom <= first.top + 2.0;
                        assert!(apart, "{place}: {} runs into {}", first.text, second.text);
                    }
                }
            }
        }
    }

    fn tallies(name: &str, count: u64) -> Vec<Tally> {
        (0..5)
            .map(|index| Tally {
                name: format!("{index}{name}"),
                count,
            })
            .collect()
    }

    fn largest() -> Stats {
        Stats {
            since: date(2025, 12, 1),
            until: date(2026, 11, 30),
            git: Some(GitStats {
                commits: u64::MAX,
                active_days: u32::MAX,
                longest_streak: u32::MAX,
                top_languages: tallies("Objective-C++", u64::MAX),
                changed_files: u64::MAX,
                repositories: u32::MAX,
                top_repositories: tallies(&"r".repeat(80), u64::MAX),
                ..GitStats::default()
            }),
            shell: Some(ShellStats {
                commands: u64::MAX,
                undated_commands: u64::MAX,
                top_commands: tallies(&"c".repeat(31), u64::MAX),
                top_subcommands: tallies(&format!("kubectl {}", "s".repeat(31)), u64::MAX),
                most_common_typo: Some(Tally {
                    name: "t".repeat(32),
                    count: u64::MAX,
                }),
                typos: u64::MAX,
                git_commands: u64::MAX,
                git_status_commands: u64::MAX,
            }),
            agents: Some(AgentStats {
                sessions: tallies("Claude Code", u64::MAX),
                active_days: u32::MAX,
                tool_uses: tallies(&"x".repeat(64), u64::MAX),
                tool_use_total: u64::MAX,
                busiest_hour: Some(23),
                tokens: Tokens {
                    input: u64::MAX,
                    output: u64::MAX,
                    cache_read: u64::MAX,
                    cache_write: u64::MAX,
                },
                top_models: tallies(&"m".repeat(64), u64::MAX),
            }),
            activity: Activity {
                events: u64::MAX,
                by_hour: [u64::MAX; 24],
                by_weekday: [u64::MAX; 7],
            },
        }
    }

    #[test]
    fn sample_fits() {
        assert_fits(&sample());
    }

    #[test]
    fn largest_numbers_and_longest_names_fit() {
        assert_fits(&largest());
    }

    #[test]
    fn empty_sources_fit() {
        assert_fits(&Stats::default());
        assert_fits(&Stats {
            git: Some(GitStats::default()),
            shell: Some(ShellStats::default()),
            agents: Some(AgentStats::default()),
            ..Stats::default()
        });
    }

    #[test]
    fn every_single_source_fits() {
        let full = largest();
        for stats in [
            Stats {
                git: full.git.clone(),
                ..Stats::default()
            },
            Stats {
                shell: full.shell.clone(),
                ..Stats::default()
            },
            Stats {
                agents: full.agents.clone(),
                ..Stats::default()
            },
        ] {
            assert_fits(&stats);
        }
    }

    #[test]
    fn names_with_markup_stay_text() {
        let mut stats = sample();
        if let Some(agents) = &mut stats.agents {
            agents.top_models = vec![Tally {
                name: "<script>&\"'".to_string(),
                count: 3,
            }];
        }
        for theme in THEMES {
            let all: String = slides(&stats, theme, Clock::H24)
                .into_iter()
                .map(|slide| slide.svg)
                .collect();
            assert!(!all.contains("<script>"));
            assert!(all.contains("&lt;script&gt;&amp;&quot;&apos;"));
        }
    }

    #[test]
    fn every_slide_has_the_slide_size() {
        for theme in THEMES {
            for slide in slides(&sample(), theme, Clock::H24) {
                assert!(slide.svg.starts_with(
                    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1080\" height=\"1350\""
                ));
                assert!(slide.svg.ends_with("</svg>\n"));
            }
        }
    }
}
